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
    pub time: chrono::DateTime<chrono::FixedOffset>,
}

pub fn commit(args: CommitArgs) -> Result<()> {
    let root_path = args.cwd;
    let git_path = root_path.as_path().join(".git");
    let db_path = git_path.as_path().join("objects");

    let workspace = Workspace::new(&root_path);
    let database = Database::new(db_path);
    let refs = Refs::new(&git_path);

    let files = workspace.list_files()?;

    println!("COMMIT: file list: {:#?}", files);

    // XXX wrong invocation
    Tree::build(&workspace, files.clone()).unwrap();

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
        let mode = if metadata.permissions().mode() & 0o111 != 0 {
            Mode::ReadWriteExecute
        } else {
            Mode::ReadWrite
        };

        entries.push(Entry::new(file, blob.oid(), mode));
    }

    let tree = Tree::new(entries);
    database.store(&tree)?;

    let parent = refs.read_head().ok();

    let author = Author::new(args.name, args.email, args.time);
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
    use std::ffi::OsStr;
    use std::fs::{create_dir_all, write};
    use std::process::Command;
    use tempdir::TempDir;
    use test_process_executor::Executor;

    const AUTHOR_NAME: &str = "Sean";
    const AUTHOR_EMAIL: &str = "sean@zombo.com";
    const DATE: &str = "2021-01-01T01:01:01+00:00";
    const MESSAGE: &str = "message";

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

    // TODO: We have utilities to make test file generation easiser; see
    // workspace tests
    fn create_test_files(dir: &TempDir) {
        write(dir.path().join("file.txt"), "file contents").unwrap();
        create_dir_all(dir.path().join("subdir")).unwrap();
        create_dir_all(dir.path().join("subdir/nested")).unwrap();
        write(dir.path().join("subdir/file.txt"), "hi").unwrap();
        write(dir.path().join("subdir/nested/file.txt"), "hello").unwrap();
    }

    // Helper structure representing the "pristine" git impl for comparison.
    struct GoldenGit {
        dir: TempDir,
        executor: Executor<String, String>,
    }

    impl GoldenGit {
        fn git_dir(dir: &TempDir) -> String {
            dir.path()
                .join(".git")
                .to_str()
                .ok_or_else(|| anyhow!("Cannot convert path to str"))
                .unwrap()
                .to_string()
        }

        fn git_worktree(dir: &TempDir) -> String {
            dir.path()
                .as_os_str()
                .to_str()
                .ok_or_else(|| anyhow!("Cannot convert path to str"))
                .unwrap()
                .to_string()
        }

        fn new() -> Self {
            let dir = TempDir::new("git-golden").unwrap();
            init_golden(&dir);

            let executor = Executor::new(vec![
                ("GIT_AUTHOR_NAME".to_string(), AUTHOR_NAME.to_string()),
                ("GIT_AUTHOR_EMAIL".to_string(), AUTHOR_EMAIL.to_string()),
                ("GIT_AUTHOR_DATE".to_string(), DATE.to_string()),
                ("GIT_COMMITTER_NAME".to_string(), AUTHOR_NAME.to_string()),
                ("GIT_COMMITTER_EMAIL".to_string(), AUTHOR_EMAIL.to_string()),
                ("GIT_COMMITTER_DATE".to_string(), DATE.to_string()),
                ("GIT_WORK_TREE".to_string(), GoldenGit::git_worktree(&dir)),
                ("GIT_DIR".to_string(), GoldenGit::git_dir(&dir)),
            ]);

            GoldenGit { dir, executor }
        }

        fn path(&self) -> &std::path::Path {
            self.dir.path()
        }

        fn populate_test_files(&self) {
            create_test_files(&self.dir);
        }

        fn run<I, S>(&self, args: I)
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>,
        {
            self.executor.run(args)
        }
    }

    #[test]
    fn test_init() -> Result<()> {
        let git_env = GoldenGit::new();
        git_env.run(vec!["git", "init"]);

        let test_dir = TempDir::new("git-under-test")?;
        init_manually(&test_dir);

        // Compare the outputs for known paths.
        directory_compare(
            &mut vec![".git/HEAD", ".git/objects", ".git/refs"].into_iter(),
            git_env.path(),
            test_dir.path(),
        )
        .map_err(|e| anyhow!(e))?;

        Ok(())
    }

    /*
    #[test]
    fn test_commit() -> Result<()> {
        let git_env = GoldenGit::new();
        git_env.run(vec!["git", "init"]);
        git_env.populate_test_files();
        git_env.run(vec!["git", "add", "file.txt"].into_iter());
        git_env.run(vec!["git", "commit", "-m", MESSAGE].into_iter());

        let test_dir = TempDir::new("git-under-test")?;
        init_manually(&test_dir);
        create_test_files(&test_dir);

        commit(CommitArgs {
            cwd: test_dir.path().to_path_buf(),
            message: Some(MESSAGE),
            name: AUTHOR_NAME.to_string(),
            email: AUTHOR_EMAIL.to_string(),
            time: chrono::DateTime::parse_from_rfc3339(DATE)?,
        })
        .unwrap();

        directory_compare(
            &mut vec![
                // 75..53: "file.txt"
                ".git/objects/75/4bb844fb01df2613c0c1fe26eaa701ce46e853",
                // d5..e7: Tree of files (only file.txt).
                //                ".git/objects/d5/fc9eda85155890f7d5424130ab8684251b65e7",
                // 6f..10: Commit object (message, timestamp, tree).
                // TODO: Time?
                //                ".git/objects/6f/7f57f6e269b85accf32468d9b6a31bfe206510",
                ".git/refs",
            ]
            .into_iter(),
            git_env.path(),
            test_dir.path(),
        )?;

        Ok(())
    }
    */

    #[test]
    fn test_commit_nested() -> Result<()> {
        let test_dir = TempDir::new("git-under-test")?;
        init_manually(&test_dir);
        create_test_files(&test_dir);

        commit(CommitArgs {
            cwd: test_dir.path().to_path_buf(),
            message: Some(MESSAGE),
            name: AUTHOR_NAME.to_string(),
            email: AUTHOR_EMAIL.to_string(),
            time: chrono::DateTime::parse_from_rfc3339(DATE)?,
        })
        .unwrap();

        // TODO: Actual test here

        Ok(())
    }
}
