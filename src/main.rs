mod fileops;
mod hooks;
mod symlinks;
mod utils;
mod secrets;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(about, author, version, propagate_version = true)]
enum Cli {
    /// Setup programs and run their hooks
    Set {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,

        #[arg(short, long, value_name = "PROGRAM", use_value_delimiter = true)]
        /// Exclude certain programs from being added and hooked 
        exclude: Vec<String>,

        #[arg(short, long)]
        /// Override conflicting dotfiles
        force: bool,

        #[arg(short, long)]
        /// Adopt conflicting dotfiles
        adopt: bool,
    },

    #[command(alias = "a")]
    /// Deploy dotfiles for the supplied programs (alias: a)
    Add {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,

        #[arg(short, long, value_name = "PROGRAM", use_value_delimiter = true)]
        /// Exclude certain programs from being added
        exclude: Vec<String>,

        #[arg(short, long)]
        /// Override conflicting dotfiles
        force: bool,

        #[arg(short, long)]
        /// Adopt conflicting dotfiles
        adopt: bool,
    },

    /// Remove dotfiles for the supplied programs
    Rm {
        #[arg(required = true, value_name = "PROGRAM")]
        programs: Vec<String>,

        #[arg(short, long, value_name = "PROGRAM", use_value_delimiter = true)]
        /// Exclude certain programs from being removed
        exclude: Vec<String>,
    },

    #[command(alias = "s")]
    /// Get dotfiles' symlinking status (alias: s)
    Status,

    #[command(alias = "e")]
    /// Encrypt files and move them to dotfiles/Secrets (alias: e)
    Encrypt {
        group: String,
        #[arg(required = true, value_name = "dotfiles")]
        dotfiles: Vec<String>,
    },

    #[command(alias = "d")]
    /// Decrypt files (alias: d)
    Decrypt {
        group: String,
    },

    /// Initialize dotfile directory
    ///
    /// Creates files necessary to use Tuckr
    Init,

    /// Convert a GNU Stow repo into a Tuckr one
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
        Cli::Encrypt {group, dotfiles} => secrets::encrypt_cmd(&group, &dotfiles),
        Cli::Decrypt {group}=> secrets::decrypt_cmd(&group),
        Cli::Init => fileops::init_tuckr_dir(),
        Cli::FromStow => fileops::convert_to_tuckr(),
    }
}
