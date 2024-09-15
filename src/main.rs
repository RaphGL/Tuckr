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

mod dotfiles;
mod fileops;
mod hooks;
mod secrets;
mod symlinks;

use clap::Parser;
use std::process::ExitCode;

#[derive(Parser)]
#[command(about, author, version, propagate_version = true)]
struct Cli {
    #[arg(short, long)]
    /// Choose which dotfile profile to use
    profile: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Parser)]
enum Command {
    #[command(alias = "s")]
    /// Get dotfiles' symlinking status (alias: s)
    Status {
        #[arg(value_name = "group")]
        groups: Option<Vec<String>>,
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

    /// Copy files into groups
    Push {
        group: String,
        #[arg(required = true)]
        files: Vec<String>,
    },

    /// Remove groups from dotfiles/Configs
    #[command(arg_required_else_help = true)]
    Pop { groups: Vec<String> },

    /// List available hooks
    LsHooks,

    /// List stored secrets
    LsSecrets,

    /// Initialize dotfile directory
    ///
    /// Creates files necessary to use Tuckr
    Init,

    /// Convert a GNU Stow repo into Tuckr
    FromStow,

    /// Returns the group the files belongs to
    #[command(name = "groupis", arg_required_else_help = true)]
    GroupIs { files: Vec<String> },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Command::Set {
            groups,
            exclude,
            force,
            adopt,
        } => hooks::set_cmd(cli.profile, &groups, &exclude, force, adopt),

        Command::Add {
            groups,
            exclude,
            force,
            adopt,
        } => symlinks::add_cmd(cli.profile, &groups, &exclude, force, adopt),

        Command::Rm { groups, exclude } => symlinks::remove_cmd(cli.profile, &groups, &exclude),
        Command::Status { groups } => symlinks::status_cmd(cli.profile, groups),
        Command::Encrypt { group, dotfiles } => {
            secrets::encrypt_cmd(cli.profile, &group, &dotfiles)
        }
        Command::Decrypt { groups, exclude } => {
            secrets::decrypt_cmd(cli.profile, &groups, &exclude)
        }
        Command::FromStow => fileops::from_stow_cmd(),
        Command::Init => fileops::init_cmd(),
        Command::LsHooks => fileops::ls_hooks_cmd(cli.profile),
        Command::LsSecrets => fileops::ls_secrets_cmd(cli.profile),
        Command::Push { group, files } => fileops::push_cmd(cli.profile, group, &files),
        Command::Pop { groups } => fileops::pop_cmd(cli.profile, &groups),
        Command::GroupIs { files } => fileops::groupis_cmd(cli.profile, &files),
    };

    match exit_code {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => e,
    }
}
