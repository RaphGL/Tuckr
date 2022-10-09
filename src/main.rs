mod fileops;
mod symlinks;
mod utils;
mod hooks;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(about, author, version)]
enum Cli {
    /// Setup the given program and run its hooks
    Set {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,
    },

    /// Deploy dotfiles for the given program
    Add {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,
    },

    /// Remove configuration for the given program
    Rm {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,
    },

    /// Print a status message for all dotfiles
    Status {
        /// Get dotfiles' symlinks
        #[arg(short, long)]
        all: bool,
    },

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
        Cli::Set { programs } => hooks::set_cmd(&programs),
        Cli::Add { programs } => symlinks::add_cmd(&programs),
        Cli::Rm { programs } => symlinks::remove_cmd(&programs),
        Cli::Status { all } => symlinks::status_cmd(),
        Cli::Init => fileops::init_tuckr_dir(),
        Cli::FromStow => fileops::convert_to_tuckr(),
    }
}
