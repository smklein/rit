use anyhow::{anyhow, Result};
use std::fs::Metadata;
use std::path::{Path, PathBuf};

/// A file path, relative to the workspace origin.
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
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

    /// Access the full path of a file within the workspace.
    pub fn full_path(&self, sub_path: &WorkspacePath) -> PathBuf {
        self.root.join(sub_path.as_partial_path())
    }

    /// Read the entirety of a file within the workspace.
    pub fn read_file(&self, path: &WorkspacePath) -> Result<Vec<u8>> {
        let real_path = self.full_path(path);
        std::fs::read(real_path).map_err(|e| anyhow!(e))
    }

    /// Read a file's metadata within the workspace.
    pub fn metadata(&self, path: &WorkspacePath) -> Result<Metadata> {
        let real_path = self.full_path(path);
        std::fs::metadata(real_path).map_err(|e| anyhow!(e))
    }

    /// Returns a list of files within the workspace, all relative to the
    /// provided path.
    ///
    /// The files are not necessarily returned in sorted order.
    pub fn list_files(&self) -> Result<Vec<WorkspacePath>> {
        self.list_files_r(None)
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::fs::{create_dir, File};
    use tempdir::TempDir;

    enum TestPath<P> {
        File(P),
        Dir(P),
    }

    struct TestDir(TempDir);

    impl TestDir {
        fn new(prefix: &str) -> Result<Self> {
            Ok(TestDir(TempDir::new(prefix)?))
        }

        fn create<P: AsRef<Path>>(&self, path: TestPath<P>) -> Result<()> {
            match path {
                TestPath::File(path) => {
                    let _ = File::create(self.0.path().join(path.as_ref()))?;
                }
                TestPath::Dir(path) => {
                    let _ = create_dir(self.0.path().join(path.as_ref()))?;
                }
            };
            Ok(())
        }
    }

    #[test]
    fn test_full_path() {
        let dir = TestDir::new("test_full_path").unwrap();
        dir.create(TestPath::File("file.txt")).unwrap();

        let workspace = Workspace::new(dir.0.path());
        assert_eq!(
            workspace.full_path(&WorkspacePath::new("").unwrap()),
            dir.0.path()
        );
        assert_eq!(
            workspace.full_path(&WorkspacePath::new("file.txt").unwrap()),
            dir.0.path().join("file.txt"),
        );
    }

    #[test]
    fn test_list_files() {
        let dir = TestDir::new("test_list_files").unwrap();
        dir.create(TestPath::Dir("dir")).unwrap();
        dir.create(TestPath::File("dir/file.txt")).unwrap();
        dir.create(TestPath::Dir("dir/subdir")).unwrap();
        dir.create(TestPath::File("dir/subdir/file.txt")).unwrap();
        dir.create(TestPath::File("file.txt")).unwrap();

        let workspace = Workspace::new(dir.0.path());
        let mut files = workspace.list_files().unwrap();
        files.sort();

        assert_eq!(
            vec![
                WorkspacePath::new("dir").unwrap(),
                WorkspacePath::new("dir/file.txt").unwrap(),
                WorkspacePath::new("dir/subdir").unwrap(),
                WorkspacePath::new("dir/subdir/file.txt").unwrap(),
                WorkspacePath::new("file.txt").unwrap(),
            ],
            files
        );
    }

    #[test]
    fn test_ignore_git() {
        let dir = TestDir::new("test_ignore_git").unwrap();
        dir.create(TestPath::Dir(".git")).unwrap();
        dir.create(TestPath::File(".git/objects")).unwrap();
        dir.create(TestPath::File("not-git-file")).unwrap();
        dir.create(TestPath::Dir("not-git-dir")).unwrap();

        let workspace = Workspace::new(dir.0.path());
        let mut files = workspace.list_files().unwrap();
        files.sort();

        assert_eq!(
            vec![
                WorkspacePath::new("not-git-dir").unwrap(),
                WorkspacePath::new("not-git-file").unwrap(),
            ],
            files
        );
    }
}
