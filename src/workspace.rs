use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct Workspace<'a> {
    root: &'a Path,
}

impl<'a> Workspace<'a> {
    // TODO: AsRef path here would be better, right?
    pub fn new(path: &'a Path) -> Self {
        Workspace { root: path }
    }

    pub fn list_files(&self) -> Result<Vec<PathBuf>> {
        let entries = std::fs::read_dir(self.root)?
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
                return match file.as_ref() {
                    "." | ".." | ".git" => true,
                    _ => false,
                };
            }
        }
        // If we can't unwrap the name, ignore whatever this thing is.
        true
    }
}
