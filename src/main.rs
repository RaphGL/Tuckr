use clap::{arg, Command};
mod symlink;

fn main() {
    let matches = Command::new("Tuckr")
        .version("0.1")
        .author("RaphGL")
        .about("Super powered GNU Stow replacement")
        .subcommand_required(true)
        .subcommand(
            Command::new("add")
                .about("Add symlinks to the dotfile folder")
                .arg(arg!(<PROGRAM>))
                .arg(arg!([FILE]...)),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove symlinks from the dotfile folder")
                .arg(arg!(<PROGRAM>))
                .arg(arg!([FILE]...)),
        )
        .subcommand(Command::new("status").about("Check symlink status"))
        .subcommand(Command::new("set").about("Run hooks").arg(arg!([HOOK])))
        .subcommand(Command::new("init").about("Initialize a dotfile folder"))
        .subcommand(Command::new("from-stow").about("Converts a stow repo into a tuckr one"))
        .get_matches();

    match matches.subcommand() {
        Some(("status", _)) => symlink::get_status(),
        Some(("add", submatches)) => {
            symlink::add(
                submatches.values_of("PROGRAM").unwrap(),
                submatches.values_of("FILE").unwrap(),
            );
        }
        Some(("remove", submatches)) => {
            symlink::remove(
                submatches.values_of("PROGRAM").unwrap(),
                submatches.values_of("FILE").unwrap(),
            );
        }
        _ => unreachable!(),
    }
}
