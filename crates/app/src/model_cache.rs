use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub(crate) struct ModelCacheEnvironment {
    hf_home: Option<PathBuf>,
    fastembed_cache_dir: Option<PathBuf>,
}

impl ModelCacheEnvironment {
    pub(crate) fn from_process() -> Self {
        Self {
            hf_home: std::env::var_os("HF_HOME").map(PathBuf::from),
            fastembed_cache_dir: std::env::var_os("FASTEMBED_CACHE_DIR").map(PathBuf::from),
        }
    }

    pub(crate) const fn needs_home(&self) -> bool {
        self.hf_home.is_none() && self.fastembed_cache_dir.is_none()
    }
}

/// Resolves the model cache directory for a run.
///
/// The resolution order is `HF_HOME`, then `FASTEMBED_CACHE_DIR`, then the
/// product default `home_directory/.mdsearch/models` (ADR-012, REQ-017 FR-001).
#[must_use]
pub(crate) fn model_cache_dir(
    home_directory: Option<&Path>,
    environment: &ModelCacheEnvironment,
) -> Option<PathBuf> {
    resolve_cache_dir(
        home_directory,
        environment.hf_home.clone(),
        environment.fastembed_cache_dir.clone(),
    )
}

/// Pure resolution of the cache directory from the environment inputs.
fn resolve_cache_dir(
    home_directory: Option<&Path>,
    hf_home: Option<PathBuf>,
    fastembed_cache_dir: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(hf_home) = hf_home {
        return Some(hf_home);
    }
    if let Some(cache_dir) = fastembed_cache_dir {
        return Some(cache_dir);
    }
    home_directory.map(|path| path.join(".mdsearch").join("models"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::resolve_cache_dir;

    /// Covers: REQ-017 FR-008 — `HF_HOME` wins over `FASTEMBED_CACHE_DIR`.
    #[test]
    fn hf_home_wins_over_fastembed_cache_dir() {
        let resolved = resolve_cache_dir(
            Some(Path::new("/home/user")),
            Some(Path::new("/cache/hf").to_owned()),
            Some(Path::new("/cache/fastembed").to_owned()),
        );

        assert_eq!(resolved, Some(Path::new("/cache/hf").to_owned()));
    }

    /// Covers: REQ-017 FR-008 — `FASTEMBED_CACHE_DIR` wins over the default.
    #[test]
    fn fastembed_cache_dir_wins_over_default() {
        let resolved = resolve_cache_dir(
            Some(Path::new("/home/user")),
            None,
            Some(Path::new("/cache/fastembed").to_owned()),
        );

        assert_eq!(resolved, Some(Path::new("/cache/fastembed").to_owned()));
    }

    /// Covers: REQ-017 FR-001/FR-002 — no environment override resolves to the
    /// product default under the home directory.
    #[test]
    fn home_default_is_used_without_environment_overrides() {
        let resolved = resolve_cache_dir(Some(Path::new("/home/user")), None, None);

        assert_eq!(
            resolved,
            Some(Path::new("/home/user/.mdsearch/models").to_owned())
        );
    }

    /// Covers: REQ-020 FR-005 — explicit model-cache settings do not need HOME.
    #[test]
    fn explicit_cache_override_is_available_without_home() {
        let resolved = resolve_cache_dir(None, Some(Path::new("/cache/hf").to_owned()), None);

        assert_eq!(resolved, Some(Path::new("/cache/hf").to_owned()));
    }
}
