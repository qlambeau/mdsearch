//! Test fixtures for legacy stored states and report decoding.

use std::{ffi::OsString, path::Path};

pub(crate) fn run<I, T>(args: I, home: &Path) -> Result<String, kv_app::AppError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    String::from_utf8(kv_app::run(args, home)?.into_bytes())
        .map_err(|_| kv_app::AppError::NonUtf8Content)
}
