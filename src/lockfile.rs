use anyhow::{anyhow, Result};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

/// Defines access to a cooperative filesystem lock
/// object. Creates a lockfile by extending the provided path with a ".lock"
/// suffix, and renaming over the previously existing file on completion.
pub struct LockFile {
    // Original path, without the ".lock" suffix.
    path: PathBuf,
    // Connection to the currently open, ".lock" variant.
    file: File,
}

impl LockFile {
    // Given "/path/to/foo.bar", return "/path/to/foo.bar.lock".
    fn lock_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let file = path
            .as_ref()
            .file_name()
            .ok_or_else(|| anyhow!("No file name"))?;
        let lock_file = file
            .to_str()
            .ok_or_else(|| anyhow!("Invalid unicode"))?
            .to_string()
            + ".lock";

        // Same as the orignal path, but swap the file name with a new suffix.
        let mut lock_buf = PathBuf::from(path.as_ref());
        lock_buf.set_file_name(lock_file);
        Ok(lock_buf)
    }

    /// Wraps the `path` argument in a lockfile variant.
    ///
    /// Does not mutate the object behind `path` until commit is invoked.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(LockFile::lock_path(path.as_ref())?)?;
        Ok(LockFile {
            path: path.as_ref().into(),
            file,
        })
    }

    /// Provide access to the writer interface of the file.
    pub fn writer(&mut self) -> &mut impl std::io::Write {
        &mut self.file
    }

    /// Consumes the lockfile object, atomically moving the written
    /// contents of the LockFile to the final path location.
    pub fn commit(self) -> Result<()> {
        Ok(std::fs::rename(
            LockFile::lock_path(&self.path)?,
            self.path,
        )?)
    }
}
