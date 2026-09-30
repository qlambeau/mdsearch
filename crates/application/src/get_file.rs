use std::path::PathBuf;

use kv_domain::{CollectionName, FileId};

use crate::{FileRetrievalStore, GetFileError, RetrievedFile};

/// An explicit stored-file selector (REQ-023 FR-016).
pub enum FileSelector<'a> {
    /// An exact path or unique basename, including numeric names.
    Name(&'a str),
    /// A validated stored file ID.
    Id(FileId),
}

impl<'a> From<&'a str> for FileSelector<'a> {
    fn from(name: &'a str) -> Self {
        Self::Name(name)
    }
}

impl From<FileId> for FileSelector<'_> {
    fn from(id: FileId) -> Self {
        Self::Id(id)
    }
}

/// Retrieves a complete stored file by name or ID.
pub struct GetFile<S> {
    store: S,
}

impl<S> GetFile<S>
where
    S: FileRetrievalStore,
{
    /// Creates a get-file use case with its store port.
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    /// Returns the stored file addressed by an explicit selector in `collection`.
    ///
    /// Names resolve by exact path and then unique basename; only typed IDs
    /// select the stored ID lookup.
    ///
    /// # Errors
    ///
    /// Returns a not-found, ambiguous-basename, or store error when the file
    /// cannot be retrieved.
    pub fn execute<'a>(
        &self,
        collection: &CollectionName,
        selector: impl Into<FileSelector<'a>>,
    ) -> Result<RetrievedFile, GetFileError> {
        let name_or_id = match selector.into() {
            FileSelector::Id(id) => {
                return self
                    .store
                    .get_by_id(collection, id)?
                    .ok_or(GetFileError::FileNotFound);
            }
            FileSelector::Name(name) => name,
        };

        let path = PathBuf::from(name_or_id);
        if let Some(file) = self.store.get_by_path(collection, &path)? {
            return Ok(file);
        }

        let basename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(name_or_id);
        let candidates = self.store.list_by_basename(collection, basename)?;

        match candidates.len() {
            0 => Err(GetFileError::FileNotFound),
            1 => candidates
                .into_iter()
                .next()
                .ok_or(GetFileError::FileNotFound),
            _ => Err(GetFileError::Ambiguous(
                candidates
                    .into_iter()
                    .map(|file| file.path().to_owned())
                    .collect(),
            )),
        }
    }
}
