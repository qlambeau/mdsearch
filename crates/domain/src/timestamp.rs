/// A UTC timestamp represented as Unix seconds.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Creates a timestamp from Unix seconds.
    #[must_use]
    pub const fn from_unix_seconds(seconds: u64) -> Self {
        Self(seconds)
    }

    /// Returns the timestamp as Unix seconds.
    #[must_use]
    pub const fn as_unix_seconds(self) -> u64 {
        self.0
    }
}
