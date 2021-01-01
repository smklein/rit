use anyhow::{anyhow, Result};
use std::fs::Metadata;
use std::path::{Path, PathBuf};

/// A file path, relative to the workspace origin.
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct WorkspacePath {
    path: PathBuf,
}

impl WorkspacePath {
    /// Creates a new WorkspacePath, relative to some unspecified
    /// workspace root.
    ///
    /// The provided path must be a relative path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        // This is to help prevent user error; we want the incoming
        // paths to all be relative to the workspace root.
        //
        // Absolute paths would contain non-repository information,
        // which would cause issues when paths are embedded within
        // the object store.
        if path.as_ref().is_absolute() {
            return Err(anyhow!("Absolute paths disallowed"));
        }
        Ok(WorkspacePath {
            path: PathBuf::from(path.as_ref()),
        })
    }

    /// Returns the partial path (within some workspace) of the file.
    pub fn as_partial_path(&self) -> &Path {
        &self.path.as_path()
    }
}

pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Workspace {
            root: PathBuf::from(path.as_ref()),
        }
    }

    pub fn full_path(&self, sub_path: &WorkspacePath) -> PathBuf {
        self.root.join(sub_path.as_partial_path())
    }

    /// Read the entirety of a file within the workspace.
    pub fn read_file(&self, path: &WorkspacePath) -> Result<Vec<u8>> {
        let real_path = self.root.join(path.as_partial_path());
        std::fs::read(real_path).map_err(|e| anyhow!(e))
    }

    pub fn metadata(&self, path: &WorkspacePath) -> Result<Metadata> {
        std::fs::metadata(self.root.join(path.as_partial_path())).map_err(|e| anyhow!(e))
    }

    /// Returns a list of files within the workspace, all
    /// relative to the workspace root.
    pub fn list_files(&self) -> Result<Vec<WorkspacePath>> {
        let entries = std::fs::read_dir(self.root.as_path())?
            .map(|entry| entry.map(|entry| WorkspacePath::new(entry.file_name())))
            .flatten()
            .collect::<Result<Vec<WorkspacePath>>>()?
            .into_iter()
            .filter(|entry| !Workspace::ignored(&entry))
            .collect::<Vec<_>>();
        Ok(entries)
    }

    fn ignored(path: &WorkspacePath) -> bool {
        if let Some(file) = path.as_partial_path().file_name() {
            if let Some(file) = file.to_str() {
                return matches!(file, "." | ".." | ".git");
            }
        }
        // If we can't unwrap the name, ignore whatever this thing is.
        true
    }
}
