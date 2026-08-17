//! The indexing-assigned identifier of a stored file.

use thiserror::Error;

/// A positive indexing-assigned file ID.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FileId(u64);

/// Describes why a file ID is invalid.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum FileIdError {
    /// The value is zero; file IDs are positive.
    #[error("file ID must be positive")]
    Zero,
}

impl FileId {
    /// Creates a file ID from a positive value.
    ///
    /// # Errors
    ///
    /// Returns a zero error when the value is zero.
    pub fn try_new(value: u64) -> Result<Self, FileIdError> {
        if value == 0 {
            Err(FileIdError::Zero)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the file ID as its underlying value.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for FileId {
    type Error = FileIdError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::FileId;
    use super::FileIdError;

    /// Covers: the positive-value rule — valid IDs are accepted.
    #[test]
    fn accepts_positive_values() -> Result<(), FileIdError> {
        let id = FileId::try_new(42)?;

        assert_eq!(id.as_u64(), 42);

        Ok(())
    }

    /// Covers: the positive-value rule — zero is rejected.
    #[rstest]
    #[case(0)]
    fn rejects_zero(#[case] value: u64) {
        assert_eq!(FileId::try_new(value), Err(FileIdError::Zero));
        assert_eq!(FileId::try_from(value), Err(FileIdError::Zero));
    }
}
