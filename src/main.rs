mod fileops;
mod hooks;
mod symlinks;
mod utils;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(about, author, version)]
enum Cli {
    /// Setup the given program and run its hooks
    Set {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,

        #[arg(short, long, value_name = "PROGRAM", use_value_delimiter = true)]
        /// Exclude certain programs from being removed
        exclude: Vec<String>,
    },

    /// Deploy dotfiles for the given program
    Add {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,

        #[arg(short, long, value_name = "PROGRAM", use_value_delimiter = true)]
        /// Exclude certain programs from being added
        exclude: Vec<String>,
    },

    /// Remove configuration for the given program
    Rm {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,

        #[arg(short, long, value_name = "PROGRAM", use_value_delimiter = true)]
        /// Exclude certain programs from being removed
        exclude: Vec<String>,
    },

    /// Print a status message for all dotfiles
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
        Cli::Set { programs, exclude } => hooks::set_cmd(&programs, &exclude),
        Cli::Add { programs, exclude } => symlinks::add_cmd(&programs, &exclude),
        Cli::Rm { programs, exclude } => symlinks::remove_cmd(&programs, &exclude),
        Cli::Status => symlinks::status_cmd(),
        Cli::Init => fileops::init_tuckr_dir(),
        Cli::FromStow => fileops::convert_to_tuckr(),
    }
}
