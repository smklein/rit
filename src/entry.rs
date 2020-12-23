use crate::database::ObjectID;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

#[derive(PartialEq, PartialOrd, Eq, Ord)]
pub struct Entry {
    path: PathBuf,
    oid: ObjectID,
}

impl Entry {
    pub fn new<P: AsRef<Path>>(path: P, oid: ObjectID) -> Self {
        Entry {
            path: PathBuf::from(path.as_ref()),
            oid,
        }
    }

    pub fn path_bytes(&self) -> &[u8] {
        self.path.as_path().as_os_str().as_bytes()
    }

    pub fn oid(&self) -> &ObjectID {
        &self.oid
    }
}
