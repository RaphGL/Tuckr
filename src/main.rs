use clap::{arg, Command};
mod fileops;
mod symlinks;
mod utils;
mod hooks;

fn main() {
    let matches = Command::new("Tuckr")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand_required(true)
        .subcommand(
            Command::new("set")
                .about("Setup a program and run their hooks hooks")
                .arg(arg!(<PROGRAM>...))
        )
        .subcommand(
            Command::new("add")
                .about("Deploy dotfiles for PROGRAM")
                .arg(arg!(<PROGRAM>...))
        )
        .subcommand(
            Command::new("rm")
                .about("Remove configuration for a program on the system")
                .arg(arg!(<PROGRAM>...))
        )
        .subcommand(
            Command::new("status")
                .about("Check symlink status")
                .long_about("Prints a status message for all dotfiles")
                .arg(arg!(-a --all).help("Get dotfiles' symlinks"))
        )
        .subcommand(
            Command::new("init")
                .about("Initialize a dotfile folder")
                .long_about("Creates necessary files to use Tuckr")
        )
        .subcommand(
            Command::new("from-stow")
                .about("Converts a stow repo into a tuckr one")
                .long_about("Converts a GNU Stow repo into a Tuckr one")
        )
        .get_matches();

    match matches.subcommand() {
        Some(("set", submatches)) => {
            let programs = submatches.get_many::<String>("PROGRAM").unwrap();
            hooks::set_cmd(programs);
        }
        Some(("add", submatches)) => {
            let programs = submatches.get_many::<String>("PROGRAM").unwrap();
            symlinks::add_cmd(programs);
        }
        Some(("rm", submatches)) => {
            let programs = submatches.get_many::<String>("PROGRAM").unwrap();
            symlinks::remove_cmd(programs);
        }
        Some(("status", _)) => symlinks::status_cmd(),
        Some(("init", _)) => fileops::init_tuckr_dir(),
        Some(("from-stow", _)) => fileops::convert_to_tuckr(),
        Some((_, _)) => unreachable!(),
        None => return,
    }
}
