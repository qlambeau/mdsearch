use std::path::{Path, PathBuf};

/// Resolves the model cache directory for a run.
///
/// The resolution order is `HF_HOME`, then `FASTEMBED_CACHE_DIR`, then the
/// product default `home_directory/.mdsearch/models` (ADR-012, REQ-017 FR-001).
#[must_use]
pub(crate) fn model_cache_dir(home_directory: &Path) -> PathBuf {
    resolve_cache_dir(
        home_directory,
        std::env::var_os("HF_HOME").map(PathBuf::from),
        std::env::var_os("FASTEMBED_CACHE_DIR").map(PathBuf::from),
    )
}

/// Pure resolution of the cache directory from the environment inputs.
fn resolve_cache_dir(
    home_directory: &Path,
    hf_home: Option<PathBuf>,
    fastembed_cache_dir: Option<PathBuf>,
) -> PathBuf {
    if let Some(hf_home) = hf_home {
        return hf_home;
    }
    if let Some(cache_dir) = fastembed_cache_dir {
        return cache_dir;
    }
    home_directory.join(".mdsearch").join("models")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::tempdir;

    use super::resolve_cache_dir;

    /// Covers: REQ-017 FR-008 — `HF_HOME` wins over `FASTEMBED_CACHE_DIR`.
    #[test]
    fn hf_home_wins_over_fastembed_cache_dir() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempdir()?;

        let resolved = resolve_cache_dir(
            home.path(),
            Some(Path::new("/cache/hf").to_owned()),
            Some(Path::new("/cache/fastembed").to_owned()),
        );

        assert_eq!(resolved, Path::new("/cache/hf"));

        Ok(())
    }

    /// Covers: REQ-017 FR-008 — `FASTEMBED_CACHE_DIR` wins over the default.
    #[test]
    fn fastembed_cache_dir_wins_over_default() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempdir()?;

        let resolved = resolve_cache_dir(
            home.path(),
            None,
            Some(Path::new("/cache/fastembed").to_owned()),
        );

        assert_eq!(resolved, Path::new("/cache/fastembed"));

        Ok(())
    }

    /// Covers: REQ-017 FR-001/FR-002 — no environment override resolves to the
    /// product default under the home directory.
    #[test]
    fn home_default_is_used_without_environment_overrides() -> Result<(), Box<dyn std::error::Error>>
    {
        let home = tempdir()?;

        let resolved = resolve_cache_dir(home.path(), None, None);

        assert_eq!(resolved, home.path().join(".mdsearch").join("models"));

        Ok(())
    }
}
