use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use kv_domain::{EmbeddingModel, segment_passages};

use crate::{
    Clock, CollectionIndexUpdate, EmbeddingGenerator, FileRecord, FileRetrievalStore, FileStore,
    FileSystem, PreparedSemanticPassage, SemanticIndexStore, UpdateCollectionError, UpdateOutcome,
};

/// Groups the ports used to stage and atomically commit collection updates.
pub struct SemanticUpdatePorts<FS, S, G, C> {
    filesystem: FS,
    store: S,
    generator: G,
    clock: C,
}

impl<FS, S, G, C> SemanticUpdatePorts<FS, S, G, C> {
    /// Creates the collaborator context for source-aware updates.
    #[must_use]
    pub const fn new(filesystem: FS, store: S, generator: G, clock: C) -> Self {
        Self {
            filesystem,
            store,
            generator,
            clock,
        }
    }
}

/// Stages semantic vectors and atomically commits all configured collection indexes.
pub struct SemanticUpdateCoordinator<FS, S, G, C> {
    ports: SemanticUpdatePorts<FS, S, G, C>,
}

impl<FS, S, G, C> SemanticUpdateCoordinator<FS, S, G, C>
where
    FS: FileSystem,
    S: FileStore + FileRetrievalStore + SemanticIndexStore,
    G: EmbeddingGenerator,
    C: Clock,
{
    /// Creates a coordinator with filesystem, storage, model, and clock ports.
    #[must_use]
    pub const fn new(ports: SemanticUpdatePorts<FS, S, G, C>) -> Self {
        Self { ports }
    }

    /// Reconciles one collection from its registered Markdown files.
    ///
    /// Unreadable files are skipped when `skip_unreadable` is enabled; stored
    /// content is retained and included in semantic staging when available.
    ///
    /// # Errors
    ///
    /// Returns an error when discovery, model preparation, or the atomic
    /// collection transaction fails.
    pub fn execute(
        &mut self,
        collection: &kv_domain::CollectionName,
        paths: &[PathBuf],
        skip_unreadable: bool,
        download: bool,
    ) -> Result<UpdateOutcome, UpdateCollectionError> {
        let stored = self.ports.store.list_files(collection)?;
        let stored_by_path = stored
            .iter()
            .map(|file| (file.path().to_owned(), file.content_hash().clone()))
            .collect::<HashMap<_, _>>();
        let mut unique = HashSet::new();
        let mut files = Vec::new();
        let mut skipped = 0;
        for path in paths {
            if !unique.insert(path.clone()) {
                continue;
            }
            match self.ports.filesystem.read(path) {
                Ok(content) => files.push(FileRecord::new(path.clone(), content)),
                Err(_) if skip_unreadable => {
                    if let Some(previous) = self.ports.store.get_by_path(collection, path)? {
                        files.push(FileRecord::new(
                            previous.path().to_owned(),
                            previous.content().to_vec(),
                        ));
                    }
                    skipped += 1;
                }
                Err(error) => return Err(error.into()),
            }
        }

        let current_paths = files
            .iter()
            .map(|file| file.path().to_owned())
            .collect::<HashSet<_>>();
        let delete = stored
            .iter()
            .filter(|file| !current_paths.contains(file.path()))
            .map(|file| file.path().to_owned())
            .collect::<Vec<_>>();
        let added = files
            .iter()
            .filter(|file| !stored_by_path.contains_key(file.path()))
            .count();
        let modified = files
            .iter()
            .filter(|file| {
                stored_by_path
                    .get(file.path())
                    .is_some_and(|hash| hash != file.content_hash())
            })
            .count();
        let at = self.ports.clock.now()?;
        let semantic_enabled = self.ports.store.semantic_enabled(collection)?;
        let model = self.ports.store.global_model()?.unwrap_or(default_model()?);
        let (semantic, malformed) = if semantic_enabled {
            stage_semantic(&self.ports.generator, &model, &files, download)?
        } else {
            (Vec::new(), malformed_frontmatter_count(&files))
        };
        self.ports.store.reconcile_collection(
            &CollectionIndexUpdate {
                collection: collection.clone(),
                upsert: files,
                delete: delete.clone(),
                semantic,
                model,
            },
            at,
        )?;

        Ok(UpdateOutcome::new(
            added,
            modified,
            delete.len(),
            skipped,
            malformed,
        ))
    }
}

fn malformed_frontmatter_count(files: &[FileRecord]) -> usize {
    files
        .iter()
        .filter(|file| segment_passages(file.content()).1.is_some())
        .count()
}

fn stage_semantic<G: EmbeddingGenerator>(
    generator: &G,
    model: &EmbeddingModel,
    files: &[FileRecord],
    download: bool,
) -> Result<(Vec<PreparedSemanticPassage>, usize), UpdateCollectionError> {
    let mut staged = Vec::new();
    let mut malformed = 0;
    if files
        .iter()
        .any(|file| !segment_passages(file.content()).0.is_empty())
    {
        generator.ensure_available(model, download)?;
    }
    for file in files {
        let (passages, issue) = segment_passages(file.content());
        if issue.is_some() {
            malformed += 1;
        }
        let texts = passages
            .iter()
            .map(kv_domain::Passage::text)
            .collect::<Vec<_>>();
        if texts.is_empty() {
            continue;
        }
        let embeddings = generator.embed(model, &texts)?;
        if embeddings.len() != passages.len() {
            return Err(
                crate::EmbeddingError::Storage(Box::new(std::io::Error::other(
                    "embedding count does not match passage count",
                )))
                .into(),
            );
        }
        for (position, (passage, embedding)) in passages.into_iter().zip(embeddings).enumerate() {
            staged.push(PreparedSemanticPassage::new(
                file.path().to_owned(),
                passage.kind(),
                position,
                embedding,
            ));
        }
    }
    Ok((staged, malformed))
}

fn default_model() -> Result<EmbeddingModel, UpdateCollectionError> {
    EmbeddingModel::try_new("all-MiniLM-L6-v2")
        .map_err(|error| crate::EmbeddingError::Storage(Box::new(error)).into())
}
