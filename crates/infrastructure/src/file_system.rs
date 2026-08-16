use std::fs;
use std::path::{Path, PathBuf};

use kv_application::{FileSystem, FileSystemError};

/// Reads markdown files from the local filesystem.
pub struct SystemFileSystem;

impl FileSystem for SystemFileSystem {
    fn expand(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        let metadata = fs::metadata(path).map_err(|source| unreadable(path, source))?;

        let mut files = Vec::new();
        if metadata.is_dir() {
            collect_markdown_files(path, &mut files)?;
        } else if metadata.is_file() && is_markdown(path) {
            let canonical = fs::canonicalize(path).map_err(|source| unreadable(path, source))?;
            files.push(canonical);
        }

        Ok(files)
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FileSystemError> {
        fs::read(path).map_err(|source| unreadable(path, source))
    }

    fn exists(&self, path: &Path) -> Result<bool, FileSystemError> {
        match fs::metadata(path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(FileSystemError::Unreadable {
                path: path.to_owned(),
                source: error,
            }),
        }
    }
}

fn unreadable(path: &Path, source: std::io::Error) -> FileSystemError {
    FileSystemError::Unreadable {
        path: path.to_owned(),
        source,
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), FileSystemError> {
    let entries = fs::read_dir(dir).map_err(|source| unreadable(dir, source))?;

    for entry in entries {
        let entry = entry.map_err(|source| unreadable(dir, source))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| unreadable(&path, source))?;

        if file_type.is_dir() {
            collect_markdown_files(&path, files)?;
        } else if file_type.is_file() && is_markdown(&path) {
            let canonical = fs::canonicalize(&path).map_err(|source| unreadable(&path, source))?;
            files.push(canonical);
        }
    }

    Ok(())
}
