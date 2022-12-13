mod fileops;
mod hooks;
mod symlinks;
mod utils;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(about, author, version, propagate_version = true)]
enum Cli {
    /// Setup the program and run its hooks
    Set {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,

        #[arg(short, long, value_name = "PROGRAM", use_value_delimiter = true)]
        /// Exclude certain programs from being removed
        exclude: Vec<String>,

        #[arg(short, long)]
        /// Replace dotfiles even if there are conflicts
        force: bool,

        #[arg(short, long)]
        /// Adopt the conflicting dotfile instead
        adopt: bool,
    },

    #[command(alias = "a")]
    /// Deploy dotfiles for the given program (alias: a)
    Add {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,

        #[arg(short, long, value_name = "PROGRAM", use_value_delimiter = true)]
        /// Exclude certain programs from being added
        exclude: Vec<String>,

        #[arg(short, long)]
        /// Replace dotfiles even if there are conflicts
        force: bool,

        #[arg(short, long)]
        /// Adopt the conflicting dotfile instead
        adopt: bool,
    },

    /// Remove dotfiles for a program
    Rm {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,

        #[arg(short, long, value_name = "PROGRAM", use_value_delimiter = true)]
        /// Exclude certain programs from being removed
        exclude: Vec<String>,
    },

    #[command(alias = "s")]
    /// Print a status message for all dotfiles (alias: s)
    Status,

    /// Initialize dotfile directory
    ///
    /// Creates files necessary to use Tuckr
    Init,

    /// Converts a GNU Stow repo into a Tuckr one
    FromStow,
}

fn main() {
    let cli = Cli::parse();

    match cli {
        Cli::Set {
            programs,
            exclude,
            force,
            adopt,
        } => hooks::set_cmd(&programs, &exclude, force, adopt),

        Cli::Add {
            programs,
            exclude,
            force,
            adopt,
        } => symlinks::add_cmd(&programs, &exclude, force, adopt),

        Cli::Rm { programs, exclude } => symlinks::remove_cmd(&programs, &exclude),
        Cli::Status => symlinks::status_cmd(),
        Cli::Init => fileops::init_tuckr_dir(),
        Cli::FromStow => fileops::convert_to_tuckr(),
    }
}
