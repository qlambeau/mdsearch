//! Source roots associated with a collection.

use std::path::{Path, PathBuf};

/// Describes whether a registered collection source is a file or directory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceKind {
    /// A single Markdown file.
    File,
    /// A directory recursively containing Markdown files.
    Directory,
}

/// A canonical filesystem path registered as a collection source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionSource {
    path: PathBuf,
    kind: SourceKind,
}

impl CollectionSource {
    /// Creates a source from a canonical absolute path and its filesystem kind.
    #[must_use]
    pub fn new(path: PathBuf, kind: SourceKind) -> Self {
        Self { path, kind }
    }

    /// Returns the canonical source path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns whether the source is a file or directory.
    #[must_use]
    pub const fn kind(&self) -> SourceKind {
        self.kind
    }
}
