use clap::{arg, Command};
use std::fs;
use std::path::PathBuf;
mod symlink;

fn get_dotfiles_path() -> Option<PathBuf> {
    let home = dirs::home_dir().unwrap();
    for f in fs::read_dir(home).unwrap() {
        let file = f.unwrap();
        let filepath = file.path();
        let filename = filepath.to_str().unwrap();
        if filename.contains("Dotfiles")
            || filename.contains("dotfiles")
            || filename.contains(".dotfiles")
        {
            return Some(filepath);
        }
    }
    None
}

fn main() {
    let matches = Command::new("Tuckr")
        .version("0.1")
        .author("RaphGL")
        .about("Super powered GNU Stow replacement")
        .subcommand_required(true)
        .subcommand(
            Command::new("set")
                .about("Setup program with hooks")
                .arg(arg!(<PROGRAM>...)),
        )
        .subcommand(
            Command::new("add")
                .about("Symlink program to $HOME")
                .arg(arg!(<PROGRAM>...)),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove symlinks from $HOME")
                .arg(arg!(<PROGRAM>...)),
        )
        .subcommand(Command::new("status").about("Check symlink status"))
        .subcommand(Command::new("init").about("Initialize a dotfile folder"))
        .subcommand(Command::new("from-stow").about("Converts a stow repo into a tuckr one"))
        .get_matches();

    match matches.subcommand() {
        Some(("status", _)) => symlink::get_status(),
        Some(("add", submatches)) => {
            symlink::add(submatches.values_of("PROGRAM").unwrap());
        }
        Some(("remove", submatches)) => {
            symlink::remove(submatches.values_of("PROGRAM").unwrap());
        }
        _ => unreachable!(),
    }
}
