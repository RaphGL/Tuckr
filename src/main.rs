//! Tuckr is a set of tools to manage dotfiles
//!
//! Tuckr separates dotfiles into 3 different directories:
//! - dotfiles/Configs - stores config files
//! - dotfiles/Hooks - stores scripts that configure the environment
//! - dotfiles/Secrets - stores encrypted/sensitive files
//!
//! Each of these of these directories contain directories with the name of the groups or logical
//! groups which contains all user scripts, configs and scripts, these are used to label them on tuckr
//! so you can add or remove them anytime

mod fileops;
mod hooks;
mod secrets;
mod symlinks;
mod utils;

use clap::Parser;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(about, author, version, propagate_version = true)]
enum Cli {
    /// Setup groups and run their hooks
    Set {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,

        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        /// Exclude certain groups from being added and hooked
        exclude: Vec<String>,

        #[arg(short, long)]
        /// Override conflicting dotfiles
        force: bool,

        #[arg(short, long)]
        /// Adopt conflicting dotfiles
        adopt: bool,
    },

    #[command(alias = "a")]
    /// Deploy dotfiles for the supplied groups (alias: a)
    Add {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,

        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        /// Exclude certain groups from being added
        exclude: Vec<String>,

        #[arg(short, long)]
        /// Override conflicting dotfiles
        force: bool,

        #[arg(short, long)]
        /// Adopt conflicting dotfiles
        adopt: bool,
    },

    /// Remove dotfiles for the supplied groups
    Rm {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,

        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        /// Exclude certain groups from being removed
        exclude: Vec<String>,
    },

    #[command(alias = "s")]
    /// Get dotfiles' symlinking status (alias: s)
    Status {
        #[arg(value_name = "group")]
        groups: Option<Vec<String>>,
    },

    #[command(alias = "e")]
    /// Encrypt files and move them to dotfiles/Secrets (alias: e)
    Encrypt {
        group: String,
        #[arg(required = true, value_name = "FILE")]
        dotfiles: Vec<String>,
    },

    #[command(alias = "d")]
    /// Decrypt files (alias: d)
    Decrypt {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,
        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        exclude: Vec<String>,
    },

    /// Initialize dotfile directory
    ///
    /// Creates files necessary to use Tuckr
    Init,

    /// Convert a GNU Stow repo into Tuckr
    FromStow,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli {
        Cli::Set {
            groups,
            exclude,
            force,
            adopt,
        } => hooks::set_cmd(&groups, &exclude, force, adopt),

        Cli::Add {
            groups,
            exclude,
            force,
            adopt,
        } => symlinks::add_cmd(&groups, &exclude, force, adopt),

        Cli::Rm { groups, exclude } => symlinks::remove_cmd(&groups, &exclude),
        Cli::Status { groups } => symlinks::status_cmd(groups),
        Cli::Encrypt { group, dotfiles } => secrets::encrypt_cmd(&group, &dotfiles),
        Cli::Decrypt { groups, exclude } => secrets::decrypt_cmd(&groups, &exclude),
        Cli::Init => fileops::init_tuckr_dir(),
        Cli::FromStow => fileops::convert_to_tuckr(),
    }

    ExitCode::SUCCESS
}
