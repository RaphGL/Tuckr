use clap::{arg, Command};
mod fileops;
mod symlinks;
mod utils;

fn main() {
    let matches = Command::new("Tuckr")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand_required(true)
        .subcommand(
            Command::new("set")
                .about("Setup program with hooks")
                .arg(arg!(<PROGRAM>...))
        )
        .subcommand(
            Command::new("add")
                .about("Deploy dotfiles for PROGRAM")
                .arg(arg!(<PROGRAM>...))
        )
        .subcommand(
            Command::new("rm")
                .about("Remove configuration for PROGRAM from the system")
                .arg(arg!(<PROGRAM>...))
        )
        .subcommand(
            Command::new("status")
                .about("Check symlink status")
                .long_about("Prints a status message showing which symlinks have been and have not been symlinked.")
        )
        .subcommand(
            Command::new("init")
                .about("Initialize a dotfile folder")
                .long_about("Creates the files necessary for using Tuckr if the current working directory is empty")
        )
        .subcommand(
            Command::new("from-stow")
                .about("Converts a stow repo into a tuckr one")
                .long_about("Converts the current working directory's stow repo into a tuckr one putting all the files in their respective folders")
        )
        .get_matches();

    match matches.subcommand() {
        Some(("status", _)) => symlinks::get_status(),
        Some(("add", submatches)) => {
            symlinks::add_cmd(submatches.values_of("PROGRAM").unwrap());
        }
        Some(("rm", submatches)) => {
            symlinks::rm_cmd(submatches.values_of("PROGRAM").unwrap());
        }
        Some(("init", _)) => fileops::init_tuckr_dir(),
        Some(("from-stow", _)) => fileops::from_stow(),
        _ => unreachable!(),
    }

}
