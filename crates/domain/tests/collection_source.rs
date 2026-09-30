//! Source value tests.

use std::path::PathBuf;

use kv_domain::{CollectionSource, SourceKind};

/// Covers: REQ-021 FR-001 — source values retain canonical paths and kinds.
#[test]
fn retains_source_path_and_kind() {
    let source = CollectionSource::new(PathBuf::from("/vault"), SourceKind::Directory);

    assert_eq!(source.path(), std::path::Path::new("/vault"));
    assert_eq!(source.kind(), SourceKind::Directory);
}
