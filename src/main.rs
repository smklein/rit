mod database;
mod entry;
mod tree;
mod workspace;

use crate::database::{Blob, Database, Storable};
use crate::entry::Entry;
use crate::tree::Tree;
use crate::workspace::Workspace;
use anyhow::Result;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::path::PathBuf;

// TODO: Split commands into separate modules.

fn init(args: &ArgMatches) -> Result<()> {
    // Either acquire the user-supplied path or pick a default.
    let mut path = match args.value_of("path") {
        Some(path) => PathBuf::from(path),
        None => std::env::current_dir()?,
    };

    {
        path.push(".git");
        let dirs = ["objects", "refs"];
        for dir in dirs.iter() {
            path.push(dir);
            std::fs::create_dir_all(&path)?;
            path.pop();
        }
        path.pop(); // Remove ".git" path component
    }

    println!(
        "Initialized empty Rit repository in {}",
        path.as_path().display()
    );
    Ok(())
}

fn commit(_args: &ArgMatches) -> Result<()> {
    let root_path = std::env::current_dir()?;
    let git_path = root_path.as_path().join(".git");
    let db_path = git_path.as_path().join("objects");

    let workspace = Workspace::new(&root_path);
    let database = Database::new(db_path);

    let files = workspace.list_files()?;
    println!("Files in workspace: {:#?}", files);

    let mut entries = Vec::new();
    for file in files {
        let data = workspace.read_file(&file)?;
        let blob = Blob::new(data);
        database.store(&blob)?;

        entries.push(Entry::new(file, blob.oid()));
    }

    let tree = Tree::new(entries);
    database.store(&tree)?;

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
            SubCommand::with_name("commit").about("Record changes to the repository"),
        ])
        .get_matches();

    match args.subcommand() {
        ("init", Some(args)) => init(args)?,
        ("commit", Some(args)) => commit(args)?,
        _ => eprintln!("Unknown command, try 'rit help'"),
    }

    Ok(())
}
