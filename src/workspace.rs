use anyhow::{anyhow, Result};
use std::fs::{File, Metadata};
use std::io::Read;
use std::path::{Path, PathBuf};

/// A file path, relative to the workspace origin.
#[derive(Debug)]
pub struct WorkspacePath {
    path: PathBuf,
}

impl AsRef<Path> for WorkspacePath {
    #[inline]
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl WorkspacePath {
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

    /// Read the entirety of a file within the workspace.
    pub fn read_file(&self, path: &WorkspacePath) -> Result<Vec<u8>> {
        let real_path = self.root.join(path);
        let mut f = File::open(real_path)?;
        let mut result = Vec::new();
        f.read_to_end(&mut result).map_err(|e| anyhow!(e))?;
        Ok(result)
    }

    pub fn metadata(&self, path: &WorkspacePath) -> Result<Metadata> {
        std::fs::metadata(self.root.join(path)).map_err(|e| anyhow!(e))
    }

    /// Returns a list of files within the workspace, all
    /// relative to the workspace root.
    pub fn list_files(&self) -> Result<Vec<WorkspacePath>> {
        let entries = std::fs::read_dir(self.root.as_path())?
            .map(|entry| entry.map(|entry| WorkspacePath::new(entry.file_name())))
            .flatten()
            .collect::<Result<Vec<WorkspacePath>>>()?
            .into_iter()
            .filter(|entry| !Workspace::ignored(entry))
            .collect::<Vec<_>>();
        Ok(entries)
    }

    fn ignored<P: AsRef<Path>>(s: P) -> bool {
        if let Some(file) = s.as_ref().file_name() {
            if let Some(file) = file.to_str() {
                return matches!(file, "." | ".." | ".git");
            }
        }
        // If we can't unwrap the name, ignore whatever this thing is.
        true
    }
}
