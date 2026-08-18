//! Value types for cross-encoder re-ranking.

use thiserror::Error;

/// A local cross-encoder re-ranker model identifier.
///
/// The name is the canonical identifier of a model supported by the re-ranker
/// adapter; the adapter validates supportedness, so this type only enforces
/// that the name is non-empty.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RerankerModel(String);

/// Describes why a re-ranker model name is invalid.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum RerankerModelError {
    /// The name is empty or whitespace-only.
    #[error("re-ranker model name must not be empty")]
    Empty,
}

impl RerankerModel {
    /// Creates a re-ranker model from a non-empty name.
    ///
    /// # Errors
    ///
    /// Returns an empty error when the name is empty or whitespace-only.
    pub fn try_new(name: &str) -> Result<Self, RerankerModelError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            Err(RerankerModelError::Empty)
        } else {
            Ok(Self(trimmed.to_owned()))
        }
    }

    /// Returns the model name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for RerankerModel {
    type Error = RerankerModelError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        Self::try_new(name)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::RerankerModel;
    use super::RerankerModelError;

    /// Covers: REQ-011 — a non-empty model name is accepted.
    #[rstest]
    #[case("bge-reranker-base")]
    #[case("  bge-reranker-base  ")]
    fn accepts_non_empty_model_names(#[case] name: &str) -> Result<(), RerankerModelError> {
        let model = RerankerModel::try_new(name)?;

        assert_eq!(model.as_str(), "bge-reranker-base");

        Ok(())
    }

    /// Covers: REQ-011 — empty model names are rejected.
    #[rstest]
    #[case("")]
    #[case("   ")]
    fn rejects_empty_model_names(#[case] name: &str) {
        assert_eq!(RerankerModel::try_new(name), Err(RerankerModelError::Empty));
        assert_eq!(
            RerankerModel::try_from(name),
            Err(RerankerModelError::Empty)
        );
    }
}
