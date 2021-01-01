use crate::database::ObjectID;
use crate::workspace::WorkspacePath;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    path: WorkspacePath,
    oid: ObjectID,
    mode: Mode,
}

impl Entry {
    pub fn new(path: WorkspacePath, oid: ObjectID, mode: Mode) -> Self {
        Entry {
            path: path,
            oid,
            mode,
        }
    }

    pub fn path_bytes(&self) -> &[u8] {
        self.path.as_partial_path().as_os_str().as_bytes()
    }

    pub fn oid(&self) -> &ObjectID {
        &self.oid
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }
}
