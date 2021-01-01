use crate::author::Author;
use crate::commit::Commit;
use crate::database::{Blob, Database, Storable};
use crate::entry::{Entry, Mode};
use crate::refs::Refs;
use crate::tree::Tree;
use crate::workspace::Workspace;
use anyhow::{anyhow, Result};
use std::fs::{canonicalize, create_dir_all, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

pub struct InitArgs<'a> {
    pub path: Option<&'a str>,
    pub cwd: PathBuf,
}

pub fn init(args: InitArgs) -> Result<()> {
    // Either acquire the user-supplied path or pick a default.
    let mut path = match args.path {
        Some(path) => PathBuf::from(path),
        None => args.cwd,
    };

    path.push(".git");
    let dirs = ["objects", "refs"];
    for dir in dirs.iter() {
        path.push(dir);
        create_dir_all(&path)?;
        path.pop();
    }

    //    std::fs::write(path.join("HEAD"), "ref: refs/heads/master")?;
    std::fs::write(path.join("HEAD"), "ref: refs/heads/master\n")?;

    println!(
        "Initialized empty Rit repository in {}",
        canonicalize(&path)?.as_path().display()
    );
    Ok(())
}

pub struct CommitArgs<'a> {
    pub cwd: PathBuf,
    pub message: Option<&'a str>,
    pub name: String,
    pub email: String,
}

pub fn commit(args: CommitArgs) -> Result<()> {
    println!("COMMIT");
    let root_path = args.cwd;
    let git_path = root_path.as_path().join(".git");
    let db_path = git_path.as_path().join("objects");

    let workspace = Workspace::new(&root_path);
    let database = Database::new(db_path);
    let refs = Refs::new(&git_path);

    let files = workspace.list_files()?;

    println!("COMMIT: file list: {:#?}", files);

    let mut entries = Vec::new();
    for file in files {
        if workspace.full_path(&file).is_dir() {
            println!("Ignoring {:#?}", file);
            // XXX: Ignoring directories
            continue;
        }
        println!("Reading data for: {:#?}", file);
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

    println!("COMMIT: entries list: {:#?}", entries);
    let tree = Tree::new(entries);
    database.store(&tree)?;

    let parent = refs.read_head().ok();

    let author = Author::new(args.name, args.email, std::time::SystemTime::now());
    let message = args
        .message
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
    use std::env;
    use std::fs::{create_dir_all, write};
    use std::process::Command;
    use tempdir::TempDir;
    use test_process_executor::Executor;

    const AUTHOR_NAME: &str = "Sean";
    const AUTHOR_EMAIL: &str = "sean@zombo.com";

    fn init_golden(dir: &TempDir) {
        Command::new("git")
            .args(&["init", &dir.path().as_os_str().to_string_lossy()])
            .output()
            .expect("failed to execute git init");
    }

    fn init_manually(dir: &TempDir) {
        init(InitArgs {
            path: Some(dir.path().as_os_str().to_str().unwrap()),
            cwd: env::current_dir().unwrap(),
        })
        .unwrap();
    }

    fn create_test_files(dir: &TempDir) {
        write(dir.path().join("file.txt"), "file contents").unwrap();
        create_dir_all(dir.path().join("subdir")).unwrap();
        write(dir.path().join("subdir/file.txt"), "nested file contents").unwrap();
    }

    #[test]
    fn test_init() -> Result<()> {
        let golden_dir = TempDir::new("git-golden")?;
        let test_dir = TempDir::new("git-under-test")?;

        init_golden(&golden_dir);
        init_manually(&test_dir);

        // Compare the outputs for known paths.
        directory_compare(
            &mut vec![".git/HEAD", ".git/objects", ".git/refs"].into_iter(),
            golden_dir.path(),
            test_dir.path(),
        )
        .map_err(|e| anyhow!(e))?;

        Ok(())
    }

    #[test]
    fn test_commit() -> Result<()> {
        let golden_dir = TempDir::new("git-golden")?;
        init_golden(&golden_dir);
        create_test_files(&golden_dir);

        let golden_worktree = golden_dir
            .path()
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("Cannot convert path to str"))?;
        let golden_git_path = golden_dir.path().join(".git");
        let golden_git = golden_git_path
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("Cannot convert path to str"))?;

        let executor = Executor::new(vec![
            ("GIT_AUTHOR_NAME", AUTHOR_NAME),
            ("GIT_AUTHOR_EMAIL", AUTHOR_EMAIL),
            ("GIT_WORK_TREE", &golden_worktree),
            ("GIT_DIR", &golden_git),
        ]);
        executor.run(vec!["git", "add", "file.txt"].into_iter());
        executor.run(vec!["git", "commit", "-m", "message"].into_iter());

        let test_dir = TempDir::new("git-under-test")?;
        init_manually(&test_dir);
        create_test_files(&test_dir);

        commit(CommitArgs {
            cwd: test_dir.path().to_path_buf(),
            message: Some("message"),
            name: AUTHOR_NAME.to_string(),
            email: AUTHOR_EMAIL.to_string(),
        })
        .unwrap();

        directory_compare(
            &mut vec![
                // 75..53: "file.txt"
                ".git/objects/75/4bb844fb01df2613c0c1fe26eaa701ce46e853",
                ".git/objects",
                ".git/refs",
            ]
            .into_iter(),
            golden_dir.path(),
            test_dir.path(),
        )
        .unwrap();

        Ok(())
    }
}
