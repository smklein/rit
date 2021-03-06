use crate::database::ObjectID;
use crate::lockfile::LockFile;
use anyhow::Result;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Shorthand names for git object IDs.
pub struct Refs {
    path: PathBuf,
}

impl Refs {
    /// Takes a path to the directory containing references
    /// as input.
    ///
    /// As an example, to access ".git/refs/HEAD", the path
    /// to ".git/refs" would be supplied to this constructor.
    pub fn new<P: AsRef<Path>>(path: P) -> Refs {
        Refs {
            path: PathBuf::from(path.as_ref()),
        }
    }

    /// Updates the HEAD file, returning an error if it is already in use.
    pub fn update_head(&self, oid: &ObjectID) -> Result<()> {
        let mut head = LockFile::new(self.head_path())?;
        head.writer().write_all(oid.as_str().as_bytes())?;
        head.commit()
    }

    pub fn read_head(&self) -> Result<ObjectID> {
        let contents = std::fs::read(self.head_path())?;
        let hex_oid: String = std::str::from_utf8(&contents)?.split_whitespace().collect();
        ObjectID::from_str(hex_oid)
    }

    fn head_path(&self) -> PathBuf {
        self.path.join("HEAD")
    }
}
