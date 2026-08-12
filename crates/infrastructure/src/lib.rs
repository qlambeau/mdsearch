#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Cross-cutting infrastructure for the `kv` application.

use std::time::{SystemTime, UNIX_EPOCH};

use kv_application::{Clock, ClockError};
use kv_domain::Timestamp;

/// Reads the current time from the operating system.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Result<Timestamp, ClockError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| ClockError::Unavailable(Box::new(error)))?;

        Ok(Timestamp::from_unix_seconds(duration.as_secs()))
    }
}
