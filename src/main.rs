mod author;
mod commands;
mod commit;
mod database;
mod entry;
mod lockfile;
mod refs;
mod tree;
mod workspace;

use crate::commands::{commit, init, InitArgs};
use anyhow::Result;
use clap::{App, Arg, SubCommand};

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
        ("init", Some(args)) => {
            let args = InitArgs {
                path: args.value_of("path"),
            };
            init(args)?;
        }
        ("commit", Some(args)) => commit(args)?,
        _ => eprintln!("Unknown command, try 'rit help'"),
    }

    Ok(())
}
