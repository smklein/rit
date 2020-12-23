mod author;
mod commit;
mod database;
mod entry;
mod tree;
mod workspace;

use crate::author::Author;
use crate::commit::Commit;
use crate::database::{Blob, Database, Storable};
use crate::entry::Entry;
use crate::tree::Tree;
use crate::workspace::Workspace;
use anyhow::{anyhow, Result};
use clap::{App, Arg, ArgMatches, SubCommand};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

// TODO: Split commands into separate modules.

fn init(args: &ArgMatches) -> Result<()> {
    // Either acquire the user-supplied path or pick a default.
    let mut path = match args.value_of("path") {
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

    println!(
        "Initialized empty Rit repository in {}",
        std::fs::canonicalize(&path)?.as_path().display()
    );
    Ok(())
}

fn commit(args: &ArgMatches) -> Result<()> {
    let root_path = env::current_dir()?;
    let git_path = root_path.as_path().join(".git");
    let db_path = git_path.as_path().join("objects");

    let workspace = Workspace::new(&root_path);
    let database = Database::new(db_path);

    let files = workspace.list_files()?;

    let mut entries = Vec::new();
    for file in files {
        let data = workspace.read_file(&file)?;
        let blob = Blob::new(data);
        database.store(&blob)?;

        entries.push(Entry::new(file, blob.oid()));
    }

    let tree = Tree::new(entries);
    database.store(&tree)?;

    let name = env::var("GIT_AUTHOR_NAME")?;
    let email = env::var("GIT_AUTHOR_EMAIL")?;

    let author = Author::new(name, email, std::time::SystemTime::now());
    let message = args
        .value_of("message")
        .ok_or_else(|| anyhow!("No commit message"))?
        .to_string();

    let commit = Commit::new(tree.oid(), author, message);
    database.store(&commit)?;

    let head_path = git_path.join("HEAD");
    let mut head = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&head_path)?;
    head.write_all(commit.oid().as_str().as_bytes())?;

    println!(
        "[(root-commit) {} {}",
        commit.oid().as_str(),
        commit
            .message()
            .lines()
            .next()
            .unwrap_or("<No commit message>"),
    );

    Ok(())
}

fn main() -> Result<()> {
    let args = App::new("Rusty git (rit)")
        .version("1.0")
        .author("Sean Klein")
        .subcommands(vec![
            SubCommand::with_name("init")
                .about("Initializes a git repo")
                .arg(
                    Arg::with_name("path")
                        .default_value(".")
                        .takes_value(true)
                        .help("Path to git repo which should be initialized"),
                ),
            SubCommand::with_name("commit")
                .about("Record changes to the repository")
                .arg(
                    Arg::with_name("message")
                        .short("m")
                        .long("message")
                        .takes_value(true)
                        .help("Uses the provided argument as a commit message"),
                ),
        ])
        .get_matches();

    match args.subcommand() {
        ("init", Some(args)) => init(args)?,
        ("commit", Some(args)) => commit(args)?,
        _ => eprintln!("Unknown command, try 'rit help'"),
    }

    Ok(())
}
