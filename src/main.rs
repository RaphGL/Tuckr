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
mod filetree;
mod hooks;
mod secrets;
mod symlinks;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

rust_i18n::i18n!("locales", minify_key = true, fallback = "en");

/// style similar to cargo's
const fn tuckr_color_styles() -> clap::builder::Styles {
    use clap::builder::styling::{AnsiColor, Styles};
    let styles = Styles::styled()
        .header(AnsiColor::Green.on_default())
        .usage(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::BrightCyan.on_default())
        .literal(AnsiColor::BrightCyan.on_default());
    styles
}

#[derive(Parser)]
#[command(about, author, version, propagate_version = true, styles = tuckr_color_styles())]
struct Cli {
    /// Choose which dotfile profile to use
    #[arg(short, long)]
    profile: Option<String>,

    /// No filesystem operations. Only print what would happen
    #[arg(short = 'n', long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Get dotfiles' symlinking status (alias: s)
    #[command(alias = "s")]
    Status {
        #[arg(value_name = "group")]
        groups: Option<Vec<String>>,
    },

    /// Deploy dotfiles for the supplied groups (alias: a)
    #[command(alias = "a")]
    Add {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,

        /// Exclude certain groups from being added
        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        exclude: Vec<String>,

        /// Override conflicting dotfiles
        #[arg(short, long)]
        force: bool,

        /// Adopt conflicting dotfiles
        #[arg(short, long)]
        adopt: bool,

        /// Automatically answer yes on every prompt
        #[arg(short = 'y', long)]
        assume_yes: bool,

        /// Only add files and ignore directories
        #[arg(long)]
        only_files: bool,
    },

    /// Remove dotfiles for the supplied groups
    Rm {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,

        /// Exclude certain groups from being removed
        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        exclude: Vec<String>,
    },

    /// Setup groups and run their hooks
    Set {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,

        /// Exclude certain groups from being added and hooked
        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        exclude: Vec<String>,

        /// Override conflicting dotfiles
        #[arg(short, long)]
        force: bool,

        /// Adopt conflicting dotfiles
        #[arg(short, long)]
        adopt: bool,

        /// Automatically answer yes to stdin prompts
        #[arg(short = 'y', long)]
        assume_yes: bool,

        /// Only add files and ignore directories
        #[arg(long)]
        only_files: bool,
    },

    /// Remove groups and run their cleanup hooks
    Unset {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,

        /// Exclude certain groups from being removed
        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        exclude: Vec<String>,
    },

    /// Encrypt files and move them to dotfiles/Secrets (alias: e)
    #[command(alias = "e")]
    Encrypt {
        group: String,
        #[arg(required = true, value_name = "FILE")]
        dotfiles: Vec<String>,
    },

    /// Decrypt files (alias: d)
    #[command(alias = "d")]
    Decrypt {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,
        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        exclude: Vec<String>,
    },

    /// Copy files into groups
    Push {
        group: String,
        #[arg(short = 'y', long)]
        assume_yes: bool,
        #[arg(required = true)]
        files: Vec<String>,
    },

    /// Remove groups from dotfiles/Configs
    #[command(arg_required_else_help = true)]
    Pop {
        groups: Vec<String>,
        #[arg(short = 'y', long)]
        assume_yes: bool,
    },

    /// List dotfiles hooks, secrets, profiles
    #[command(subcommand, arg_required_else_help = true)]
    Ls(ListType),

    /// Initialize dotfile directory
    ///
    /// Creates the files that are necessary to use Tuckr
    Init,

    /// Return the group files belongs to
    #[command(name = "groupis", arg_required_else_help = true)]
    GroupIs { files: Vec<String> },
}

#[derive(Debug, Subcommand)]
enum ListType {
    /// Lists dotfiles directories with a suffix _<profile> (alias: p)
    #[command(alias = "p")]
    Profiles,
    #[command(alias = "s")]
    /// Lists encrypted files (alias: s)
    Secrets,
    /// Lists which hooks exists for each group (alias: h)
    #[command(alias = "h")]
    Hooks,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    rust_i18n::set_locale(sys_locale::get_locale().unwrap_or_default().as_str());

    let exit_code = match cli.command {
        Command::Set {
            groups,
            exclude,
            force,
            adopt,
            assume_yes,
            only_files,
        } => hooks::set_cmd(
            cli.profile,
            cli.dry_run,
            only_files,
            &groups,
            &exclude,
            force,
            adopt,
            assume_yes,
        ),

        Command::Unset { groups, exclude } => {
            hooks::unset_cmd(cli.profile, cli.dry_run, &groups, &exclude)
        }

        Command::Add {
            groups,
            exclude,
            force,
            adopt,
            assume_yes,
            only_files,
        } => symlinks::add_cmd(
            cli.profile,
            cli.dry_run,
            only_files,
            &groups,
            &exclude,
            force,
            adopt,
            assume_yes,
        ),

        Command::Rm { groups, exclude } => {
            symlinks::remove_cmd(cli.profile, cli.dry_run, &groups, &exclude)
        }
        Command::Status { groups } => symlinks::status_cmd(cli.profile, groups),
        Command::Encrypt { group, dotfiles } => {
            secrets::encrypt_cmd(cli.profile, cli.dry_run, &group, &dotfiles)
        }
        Command::Decrypt { groups, exclude } => {
            secrets::decrypt_cmd(cli.profile, cli.dry_run, &groups, &exclude)
        }
        Command::Init => fileops::init_cmd(cli.profile, cli.dry_run),

        Command::Ls(ls_type) => match ls_type {
            ListType::Profiles => fileops::ls_profiles_cmd(),
            ListType::Secrets => fileops::ls_secrets_cmd(cli.profile),
            ListType::Hooks => fileops::ls_hooks_cmd(cli.profile),
        },

        Command::Push {
            group,
            files,
            assume_yes,
        } => fileops::push_cmd(cli.profile, cli.dry_run, group, &files, assume_yes),
        Command::Pop { groups, assume_yes } => {
            fileops::pop_cmd(cli.profile, cli.dry_run, &groups, assume_yes)
        }
        Command::GroupIs { files } => fileops::groupis_cmd(cli.profile, &files),
    };

    match exit_code {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => e,
    }
}
