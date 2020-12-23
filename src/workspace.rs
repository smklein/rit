use anyhow::{anyhow, Result};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Workspace {
            root: PathBuf::from(path.as_ref()),
        }
    }

    pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
        let mut f = File::open(path)?;
        let mut result = Vec::new();
        f.read_to_end(&mut result).map_err(|e| anyhow!(e))?;
        Ok(result)
    }

    pub fn list_files(&self) -> Result<Vec<PathBuf>> {
        let entries = std::fs::read_dir(self.root.as_path())?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()?
            .into_iter()
            .filter(|entry| !Workspace::ignored(entry))
            .collect::<Vec<_>>();
        Ok(entries)
    }

    fn ignored(s: &Path) -> bool {
        if let Some(file) = s.file_name() {
            if let Some(file) = file.to_str() {
                return matches!(file, "." | ".." | ".git");
            }
        }
        // If we can't unwrap the name, ignore whatever this thing is.
        true
    }
}
