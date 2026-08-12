use kv_domain::Timestamp;

use crate::ClockError;

/// Supplies the current time to application use cases.
pub trait Clock {
    /// Returns the current UTC timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error when the time source is unavailable.
    fn now(&self) -> Result<Timestamp, ClockError>;
}
