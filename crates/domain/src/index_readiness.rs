use crate::Timestamp;

/// Whether an index is configured (REQ-023 FR-022).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexEnablement {
    /// Updates maintain this index.
    Enabled,
    /// Updates remove this index's stored state.
    Disabled,
}

/// A built index's relationship to current stored content (REQ-023 FR-023).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexFreshness {
    /// Indexed content matches stored content.
    Current,
    /// Indexed content differs from stored content.
    Stale,
}

/// Readiness of one configured index (REQ-023 FR-022).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexReadiness {
    /// Index maintenance is disabled.
    Disabled,
    /// No successful build exists.
    NotBuilt,
    /// A successful build matches stored content and prerequisites.
    Ready,
    /// A successful build differs from stored content or model configuration.
    Stale,
}

impl IndexReadiness {
    /// Derives readiness from policy, build metadata, and compatibility.
    #[must_use]
    pub const fn for_index(
        enablement: IndexEnablement,
        built_at: Option<Timestamp>,
        freshness: IndexFreshness,
    ) -> Self {
        match (enablement, built_at, freshness) {
            (IndexEnablement::Disabled, _, _) => Self::Disabled,
            (_, None, _) => Self::NotBuilt,
            (_, Some(_), IndexFreshness::Current) => Self::Ready,
            (_, Some(_), IndexFreshness::Stale) => Self::Stale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Covers: REQ-023 FR-022/023 — policy and build state determine readiness.
    #[test]
    fn readiness_preserves_disabled_unbuilt_current_and_stale() {
        let at = Some(Timestamp::from_unix_seconds(42));

        assert_eq!(
            IndexReadiness::for_index(IndexEnablement::Disabled, at, IndexFreshness::Current),
            IndexReadiness::Disabled
        );
        assert_eq!(
            IndexReadiness::for_index(IndexEnablement::Enabled, None, IndexFreshness::Stale),
            IndexReadiness::NotBuilt
        );
        assert_eq!(
            IndexReadiness::for_index(IndexEnablement::Enabled, at, IndexFreshness::Current),
            IndexReadiness::Ready
        );
        assert_eq!(
            IndexReadiness::for_index(IndexEnablement::Enabled, at, IndexFreshness::Stale),
            IndexReadiness::Stale
        );
    }
}
