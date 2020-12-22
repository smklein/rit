use anyhow::Result;
use clap::{App, Arg, ArgMatches, SubCommand};

fn init(args: &ArgMatches) -> Result<()> {
    // Either acquire the user-supplied path or pick a default.
    let mut path = match args.value_of("path") {
        Some(path) => std::path::PathBuf::from(path),
        None => std::env::current_dir()?,
    };
    path.push(".git");

    let dirs = ["objects", "refs"];

    for dir in dirs.iter() {
        path.push(dir);
        std::fs::create_dir_all(&path)?;
        path.pop();
    }

    path.pop(); // Remove .git

    println!(
        "Initialized empty Rit repository in {}",
        path.as_path().display()
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = App::new("Rusty git (rit)")
        .version("1.0")
        .author("Sean Klein")
        .subcommands(vec![SubCommand::with_name("init")
            .about("Initializes a git repo")
            .arg(
                Arg::with_name("path")
                    .default_value(".")
                    .takes_value(true)
                    .help("Path to git repo which should be initialized"),
            )])
        .get_matches();

    match args.subcommand() {
        ("init", Some(args)) => init(args)?,
        _ => {
            eprintln!("Command not found");
        }
    }

    Ok(())
}
