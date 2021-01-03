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

    /// Returns a sorted list of files within the workspace, all
    /// relative to the provided path.
    pub fn list_files(&self) -> Result<Vec<WorkspacePath>> {
        let mut list = self.list_files_r(None)?;
        list.sort();
        Ok(list)
    }

    // Recursive helper for list_files.
    //
    // If no path is provided, returns `WorkspacePath` objects within the
    // workspace root.
    fn list_files_r(&self, path: Option<&WorkspacePath>) -> Result<Vec<WorkspacePath>> {
        // Absolute path to directory in which we'll be searching.
        let dir = path
            .map(|workspace_path| self.full_path(workspace_path))
            .unwrap_or_else(|| self.root.clone());

        let base = path
            .map(|workspace_path| workspace_path.as_partial_path())
            .unwrap_or_else(|| Path::new(""));

        let entries: Vec<WorkspacePath> = std::fs::read_dir(dir)?
            .flat_map(|entry| {
                entry.map(|entry| {
                    // The entry_path represents the full portion of the path
                    // relative to the workspace root.
                    let entry_path = WorkspacePath::new(base.join(entry.file_name()))?;
                    let file_type = match entry.file_type() {
                        Ok(file_type) => file_type,
                        Err(e) => return Err(anyhow!(e)),
                    };

                    if Workspace::ignored(&entry_path) {
                        Ok(vec![])
                    } else if file_type.is_dir() {
                        let mut nested_entries = self.list_files_r(Some(&entry_path))?;
                        nested_entries.push(entry_path);
                        Ok(nested_entries)
                    } else {
                        Ok(vec![entry_path])
                    }
                })
            })
            .flatten()
            .flatten()
            .collect::<Vec<WorkspacePath>>();
        Ok(entries)
    }

    fn ignored(path: &WorkspacePath) -> bool {
        if let Some(file) = path.as_partial_path().file_name() {
            if let Some(file) = file.to_str() {
                return matches!(file, "." | ".." | ".git");
            }
        }
        false
    }
}
