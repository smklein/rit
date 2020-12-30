use crate::author::Author;
use crate::commit::Commit;
use crate::database::{Blob, Database, Storable};
use crate::entry::{Entry, Mode};
use crate::refs::Refs;
use crate::tree::Tree;
use crate::workspace::Workspace;
use anyhow::{anyhow, Result};
use clap::ArgMatches;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

// TODO: Continue pulling out envs / args

pub struct InitArgs<'a> {
    pub path: Option<&'a str>,
}

pub fn init(args: InitArgs<'_>) -> Result<()> {
    // Either acquire the user-supplied path or pick a default.
    let mut path = match args.path {
        Some(path) => PathBuf::from(path),
        None => env::current_dir()?,
    };

    path.push(".git");
    let dirs = ["objects", "refs"];
    for dir in dirs.iter() {
        path.push(dir);
        std::fs::create_dir_all(&path)?;
        path.pop();
    }

    //    std::fs::write(path.join("HEAD"), "ref: refs/heads/master")?;
    std::fs::write(path.join("HEAD"), "ref: refs/heads/master\n")?;

    println!(
        "Initialized empty Rit repository in {}",
        std::fs::canonicalize(&path)?.as_path().display()
    );
    Ok(())
}

pub fn commit(args: &ArgMatches) -> Result<()> {
    let root_path = env::current_dir()?;
    let git_path = root_path.as_path().join(".git");
    let db_path = git_path.as_path().join("objects");

    let workspace = Workspace::new(&root_path);
    let database = Database::new(db_path);
    let refs = Refs::new(&git_path);

    let files = workspace.list_files()?;

    let mut entries = Vec::new();
    for file in files {
        let data = workspace.read_file(&file)?;

        // Calculate the OID, and ensuure the entry exists in the object
        // store if it does not already exist there.
        let blob = Blob::new(data);
        database.store(&blob)?;

        // Identify if the entry is executable or not.
        let metadata = workspace.metadata(&file)?;
        let mode = if metadata.permissions().mode() & 0b111 != 0 {
            Mode::ReadWriteExecute
        } else {
            Mode::ReadWrite
        };

        entries.push(Entry::new(file, blob.oid(), mode));
    }

    let tree = Tree::new(entries);
    database.store(&tree)?;

    let parent = refs.read_head().ok();
    let name = env::var("GIT_AUTHOR_NAME")?;
    let email = env::var("GIT_AUTHOR_EMAIL")?;

    let author = Author::new(name, email, std::time::SystemTime::now());
    let message = args
        .value_of("message")
        .ok_or_else(|| anyhow!("No commit message"))?
        .to_string();

    let commit = Commit::new(&parent, &tree.oid(), author, message);
    database.store(&commit)?;
    refs.update_head(&commit.oid())?;

    let head_path = git_path.join("HEAD");
    let mut head = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&head_path)?;
    head.write_all(commit.oid().as_str().as_bytes())?;

    let root_msg = if parent.is_some() {
        "(root-commit) "
    } else {
        ""
    };

    println!(
        "[{}{}] {}",
        root_msg,
        commit.oid().as_str(),
        commit
            .message()
            .lines()
            .next()
            .unwrap_or("<No commit message>"),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Result};
    use directory_compare::directory_compare;
    use std::process::Command;
    use tempdir::TempDir;

    #[test]
    fn test_init() -> Result<()> {
        let golden_dir = TempDir::new("git-golden")?;
        let test_dir = TempDir::new("git-under-test")?;

        // Invoke golden version.
        Command::new("git")
            .args(&["init", &golden_dir.path().as_os_str().to_string_lossy()])
            .output()
            .expect("failed to execute git init");

        // Invoke manual version.
        init(InitArgs {
            path: Some(test_dir.path().as_os_str().to_str().unwrap()),
        })?;

        // Compare the outputs for known paths.
        directory_compare(
            &mut vec![".git/HEAD", ".git/objects", ".git/refs"].into_iter(),
            golden_dir.path(),
            test_dir.path(),
        )
        .map_err(|e| anyhow!(e))?;

        Ok(())
    }
}
