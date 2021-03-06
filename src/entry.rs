use crate::database::ObjectID;
use crate::workspace::{Workspace, WorkspacePath};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Mode {
    ReadWriteExecute,
    ReadWrite,
    Directory,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Mode::ReadWriteExecute => "100755",
            Mode::ReadWrite => "100644",
            Mode::Directory => "040000",
        }
    }
}

/// An Entry contains the information necessary to represent
/// a line within a tree.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    path: WorkspacePath,
    oid: ObjectID,
    mode: Mode,
}

impl Entry {
    pub fn new(path: WorkspacePath, oid: ObjectID, mode: Mode) -> Self {
        Entry { path, oid, mode }
    }

    pub fn path(&self) -> &Path {
        self.path.as_partial_path().as_ref()
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
