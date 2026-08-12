use thiserror::Error;

/// A validated collection name with its display and comparison forms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionName {
    display_name: String,
    name_key: String,
}

/// Describes why a collection name is invalid.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CollectionNameError {
    /// The name is empty after trimming.
    #[error("collection name is empty")]
    Empty,
    /// The name contains a path separator.
    #[error("collection name contains a path separator")]
    PathSeparator,
    /// The name contains a control character.
    #[error("collection name contains a control character")]
    ControlCharacter,
}

impl CollectionName {
    /// Creates a validated collection name from raw input.
    ///
    /// # Errors
    ///
    /// Returns an error when the input is empty, contains a path separator, or
    /// contains a control character.
    pub fn try_new(raw_name: &str) -> Result<Self, CollectionNameError> {
        let display_name = raw_name.trim();

        if display_name.is_empty() {
            return Err(CollectionNameError::Empty);
        }

        if display_name.contains('/') || display_name.contains('\\') {
            return Err(CollectionNameError::PathSeparator);
        }

        if display_name.chars().any(char::is_control) {
            return Err(CollectionNameError::ControlCharacter);
        }

        Ok(Self {
            display_name: display_name.to_owned(),
            name_key: display_name.to_lowercase(),
        })
    }

    /// Returns the trimmed spelling retained for display.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Returns the canonical key used for case-insensitive uniqueness.
    #[must_use]
    pub fn name_key(&self) -> &str {
        &self.name_key
    }
}

impl TryFrom<&str> for CollectionName {
    type Error = CollectionNameError;

    fn try_from(raw_name: &str) -> Result<Self, Self::Error> {
        Self::try_new(raw_name)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::CollectionName;
    use super::CollectionNameError;

    /// Covers: FR-004 — surrounding whitespace is trimmed before storage.
    #[test]
    fn trims_surrounding_whitespace_before_storage() -> Result<(), CollectionNameError> {
        let name = CollectionName::try_from("  Notes  ")?;

        assert_eq!(name.display_name(), "Notes");

        Ok(())
    }

    /// Covers: FR-005 — invalid collection names are rejected.
    #[rstest]
    #[case("", CollectionNameError::Empty)]
    #[case("   ", CollectionNameError::Empty)]
    #[case("Notes/2026", CollectionNameError::PathSeparator)]
    #[case("Notes\\2026", CollectionNameError::PathSeparator)]
    #[case("Notes\n2026", CollectionNameError::ControlCharacter)]
    fn rejects_invalid_collection_names(
        #[case] raw_name: &str,
        #[case] expected_error: CollectionNameError,
    ) {
        let result = CollectionName::try_from(raw_name);

        assert_eq!(result, Err(expected_error));
    }

    /// Covers: FR-006 — names use a case-insensitive comparison key.
    #[test]
    fn retains_display_spelling_and_normalizes_comparison_key() -> Result<(), CollectionNameError> {
        let name = CollectionName::try_from(" Notes ")?;

        assert_eq!(name.display_name(), "Notes");
        assert_eq!(name.name_key(), "notes");

        Ok(())
    }

    /// Covers: the story's no-product-defined-maximum-length rule.
    #[test]
    fn accepts_a_long_collection_name() {
        let raw_name = "n".repeat(10_000);

        let result = CollectionName::try_from(raw_name.as_str());

        assert!(result.is_ok());
    }

    // Covers: FR-006 — comparison-key generation is deterministic for valid names.
    proptest! {
        #[test]
        fn generates_the_expected_comparison_key(raw_name in "[A-Za-z]{1,32}") {
            let result = CollectionName::try_from(raw_name.as_str())
                .map(|name| name.name_key().to_owned());

            prop_assert_eq!(result, Ok(raw_name.to_lowercase()));
        }
    }
}
