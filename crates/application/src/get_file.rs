use std::path::PathBuf;

use kv_domain::{CollectionName, FileId};

use crate::{FileRetrievalStore, GetFileError, RetrievedFile};

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

    /// Returns the stored file addressed by `name_or_id` in `collection`.
    ///
    /// An all-digit positive argument is treated as a file ID; otherwise it is
    /// a name resolved by exact canonical path and then by unique basename.
    ///
    /// # Errors
    ///
    /// Returns a not-found, ambiguous-basename, or store error when the file
    /// cannot be retrieved.
    pub fn execute(
        &self,
        collection: &CollectionName,
        name_or_id: &str,
    ) -> Result<RetrievedFile, GetFileError> {
        if let Ok(value) = name_or_id.parse::<u64>()
            && let Ok(id) = FileId::try_new(value)
        {
            return self
                .store
                .get_by_id(collection, id)?
                .ok_or(GetFileError::FileNotFound);
        }

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
