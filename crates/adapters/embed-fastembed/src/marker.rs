use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Returns the completion-marker path for a model in a cache directory.
pub(crate) fn marker_path(cache_dir: &Path, model: &str) -> PathBuf {
    cache_dir.join(format!("{model}.completed"))
}

/// Returns whether the model's completion marker exists in the cache directory.
pub(crate) fn marker_exists(cache_dir: &Path, model: &str) -> bool {
    marker_path(cache_dir, model).is_file()
}

/// Writes the model's completion marker, creating the cache directory on
/// demand.
///
/// The marker is written to a temporary sibling file and renamed into place so
/// a partially written marker is never observed (ADR-012).
pub(crate) fn write_marker(cache_dir: &Path, model: &str) -> io::Result<()> {
    fs::create_dir_all(cache_dir)?;
    let target = marker_path(cache_dir, model);
    let temp = target.with_extension("tmp");
    fs::write(&temp, b"ok")?;
    fs::rename(&temp, target)?;
    Ok(())
}
