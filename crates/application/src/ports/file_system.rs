use std::path::{Path, PathBuf};

use crate::FileSystemError;
use kv_domain::CollectionSource;

/// Discovers and reads markdown files from the local filesystem.
pub trait FileSystem {
    /// Resolves an existing Markdown file or directory to a canonical source.
    ///
    /// # Errors
    ///
    /// Returns an error when the path is absent, inaccessible, or is not a Markdown file or directory.
    fn resolve_source(&self, path: &Path) -> Result<CollectionSource, FileSystemError>;
    /// Expands a single input path into zero or more canonical `.md` file paths.
    ///
    /// A directory is walked recursively; a `.md` file yields itself; a
    /// non-`.md` file yields nothing.
    ///
    /// # Errors
    ///
    /// Returns an error when the path does not exist or cannot be read.
    fn expand(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError>;

    /// Reads the bytes of the file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read.
    fn read(&self, path: &Path) -> Result<Vec<u8>, FileSystemError>;

    /// Returns whether the path exists, treating only a missing path as absent.
    ///
    /// # Errors
    ///
    /// Returns an error when the path cannot be inspected for a reason other
    /// than absence.
    fn exists(&self, path: &Path) -> Result<bool, FileSystemError>;
}
