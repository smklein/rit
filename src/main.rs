use anyhow::Result;
use clap::{Arg, ArgMatches, App, SubCommand};

fn init(args: &ArgMatches) -> Result<()> {
    // Either acquire the user-supplied path or pick a default.
    let path = match args.value_of("path") {
        Some(path) => std::path::PathBuf::from(path),
        None => std::env::current_dir()?,
    };
    // Reference the path as a "Path", not "PathBuf", for ease-of-use.
    let path = path.as_path();

    println!("path: {}", path.display());

    Ok(())
}

fn main() -> Result<()> {
    let args = App::new("Rusty git (rit)")
        .version("1.0")
        .author("Sean Klein")
        .subcommands(vec![
            SubCommand::with_name("init")
                .about("Initializes a git repo")
                .arg(Arg::with_name("path")
                    .default_value(".")
                    .takes_value(true)
                    .help("Path to git repo which should be initialized"))
        ])
        .get_matches();

    match args.subcommand() {
        ("init", Some(args)) => init(args)?,
        _ => {
            eprintln!("Command not found");
        },
    }

    Ok(())
}
