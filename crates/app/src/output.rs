use std::io::{self, Write};

/// The output of a command, preserving exact stored bytes (REQ-023 FR-017).
#[derive(Debug, Eq, PartialEq)]
pub enum CommandOutput {
    /// A human or JSON report, terminated by the executable with a newline.
    Report(String),
    /// Exact file bytes, written without conversion or an added newline.
    Content(Vec<u8>),
}

impl CommandOutput {
    /// Writes output with the command's specified newline policy.
    ///
    /// # Errors
    /// Returns the output stream's I/O error.
    ///
    /// # Examples
    /// ```
    /// let mut bytes = Vec::new();
    /// kv_app::CommandOutput::Content(b"alpha".to_vec()).write_to(&mut bytes)?;
    /// assert_eq!(bytes, b"alpha");
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn write_to(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            Self::Report(text) if text.is_empty() => Ok(()),
            Self::Report(text) => writeln!(writer, "{text}"),
            Self::Content(bytes) => writer.write_all(bytes),
        }
    }

    /// Returns the payload bytes without the report's executable newline.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Report(text) => text.into_bytes(),
            Self::Content(bytes) => bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Covers: REQ-023 FR-017/025 — report and content streams follow their newline policies.
    #[test]
    fn output_policy_preserves_payload_and_propagates_writer_errors() -> io::Result<()> {
        for (output, expected) in [
            (
                CommandOutput::Report("report".to_owned()),
                b"report\n".to_vec(),
            ),
            (CommandOutput::Report(String::new()), Vec::new()),
            (CommandOutput::Content(vec![0xff, 0]), vec![0xff, 0]),
        ] {
            let mut actual = Vec::new();
            output.write_to(&mut actual)?;
            assert_eq!(actual, expected);
        }
        let mut full_buffer = [].as_mut_slice();
        let failure = CommandOutput::Content(vec![1]).write_to(&mut full_buffer);
        assert!(failure.is_err_and(|error| error.kind() == io::ErrorKind::WriteZero));
        Ok(())
    }
}
