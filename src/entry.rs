use crate::database::ObjectID;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

#[derive(PartialEq, PartialOrd, Eq, Ord)]
pub enum Mode {
    ReadWriteExecute,
    ReadWrite,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Mode::ReadWriteExecute => "100755",
            Mode::ReadWrite => "100644",
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    path: PathBuf,
    oid: ObjectID,
    mode: Mode,
}

impl Entry {
    pub fn new<P: AsRef<Path>>(path: P, oid: ObjectID, mode: Mode) -> Self {
        Entry {
            path: PathBuf::from(path.as_ref()),
            oid,
            mode,
        }
    }

    pub fn path_bytes(&self) -> &[u8] {
        self.path.as_path().as_os_str().as_bytes()
    }

    pub fn oid(&self) -> &ObjectID {
        &self.oid
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }
}
