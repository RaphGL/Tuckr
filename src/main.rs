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
    Styles::styled()
        .header(AnsiColor::Green.on_default())
        .usage(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::BrightCyan.on_default())
        .literal(AnsiColor::BrightCyan.on_default())
}

#[derive(Parser)]
pub struct Context {
    /// Choose which dotfile profile to use.
    ///
    /// A profile is a separate dotfiles repository (eg `dotfiles_work`),
    /// allowing you to (for example) keep work-related configuration in a
    /// separate repository to your personal configuration.
    #[arg(short, long)]
    pub profile: Option<String>,

    /// No filesystem operations. Only print what would happen.
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Select custom targets to use when deploying dotfiles.
    ///
    /// When a custom target is selected, config files, hooks and secrets from
    /// groups matching that target will be preferred over less-specific
    /// groups.
    #[arg(short = 't', long = "targets", use_value_delimiter = true)]
    pub custom_targets: Vec<String>,
}

// we should never even be creating our own context since this is user provided context
// but this is useful for running unit tests so...
#[cfg(test)]
impl Default for Context {
    fn default() -> Self {
        Self {
            profile: None,
            dry_run: false,
            custom_targets: vec!["custom".into(), "laptop".into()],
        }
    }
}

#[derive(Parser)]
#[command(about, author, version, propagate_version = true, styles = tuckr_color_styles())]
struct Cli {
    #[command(flatten)]
    ctx: Context,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Get dotfiles' symlinking status (alias: s).
    ///
    /// If groups are supplied, only the status of those groups will be shown.
    #[command(alias = "s")]
    Status {
        #[arg(value_name = "group")]
        groups: Option<Vec<String>>,

        /// Output status in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Deploy dotfiles for the supplied groups (alias: a).
    ///
    /// Each file within the dotfiles groups will be linked to its
    /// corresponding location on the system, effectively "installing" those
    /// groups' files.
    #[command(alias = "a")]
    Add {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,

        /// Exclude certain groups from being added
        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        exclude: Vec<String>,

        /// Override conflicting dotfiles.
        ///
        /// If a file exists on the system and in the dotfiles repository, the
        /// version from the dotfiles repository will be used to overwrite the
        /// version on the system.
        #[arg(short, long)]
        force: bool,

        /// Adopt conflicting dotfiles.
        ///
        /// If a file exists on the system and in the dotfiles repository, the
        /// version from the system will be used to overwrite the version
        /// in the dotfiles repository.
        #[arg(short, long)]
        adopt: bool,

        /// Automatically answer yes on every prompt.
        #[arg(short = 'y', long)]
        assume_yes: bool,

        /// Recursively create symbolic links to files, instead of to the first
        /// un-created directory.
        ///
        /// This allows you to have different groups place files within the
        /// same location, even if that location does not already exist on the
        /// system.
        ///
        /// For example, if group `a` contains `bin/foo` and group `b` contains
        /// `bin/bar`, and `~/bin` does not already exist on the system, then
        /// failure to specify this option would cause a conflict as both
        /// groups would try to create the `~/bin` directory as a symbolic
        /// link.
        #[arg(long)]
        only_files: bool,
    },

    /// Remove dotfiles for the supplied groups.
    ///
    /// This removes the symbolic links on the system which point to files
    /// within the given groups.
    Rm {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,

        /// Exclude certain groups from being removed
        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        exclude: Vec<String>,
    },

    /// Add groups and run their setup hooks.
    Set {
        #[arg(required = true, value_name = "group")]
        groups: Vec<String>,

        /// Exclude certain groups from being added and hooked.
        #[arg(short, long, value_name = "group", use_value_delimiter = true)]
        exclude: Vec<String>,

        /// Override conflicting dotfiles.
        ///
        /// If a file exists on the system and in the dotfiles repository, the
        /// version from the dotfiles repository will be used to overwrite the
        /// version on the system.
        #[arg(short, long)]
        force: bool,

        /// Adopt conflicting dotfiles.
        ///
        /// If a file exists on the system and in the dotfiles repository, the
        /// version from the system will be used to overwrite the version
        /// in the dotfiles repository.
        #[arg(short, long)]
        adopt: bool,

        /// Automatically answer yes to stdin prompts.
        #[arg(short = 'y', long)]
        assume_yes: bool,

        /// Recursively create symbolic links to files, instead of to the first
        /// un-created directory.
        ///
        /// This allows you to have different groups place files within the
        /// same location, even if that location does not already exist on the
        /// system.
        ///
        /// For example, if group `a` contains `bin/foo` and group `b` contains
        /// `bin/bar`, and `~/bin` does not already exist on the system, then
        /// failure to specify this option would cause a conflict as both
        /// groups would try to create the `~/bin` directory as a symbolic
        /// link.
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
        #[arg(required = true)]
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

    /// Copy the given files into the given group, creating the group if it
    /// doesn't already exist.
    ///
    /// For each file given as a parameter, copy it to the given group within
    /// the dotfiles repository.
    Push {
        /// The group to move files into
        group: String,

        /// Files to move into the group
        #[arg(required = true)]
        files: Vec<String>,

        /// Automatically answer yes on every prompt. In the case where a file
        /// already exists in the dotfiles, this will overwrite it.
        #[arg(short = 'y', long)]
        assume_yes: bool,

        /// Symlink flags after pushing them. This is the equivalent to running
        /// `tuckr add` after pushing the files.
        #[arg(short = 'a', long)]
        add: bool,

        /// Recursively create symbolic links to files, instead of to the first
        /// un-created directory when adding dotfiles.
        ///
        /// This option only takes effect when the `--add` flag is also used.
        ///
        /// This allows you to have different groups place files within the
        /// same location, even if that location does not already exist on the
        /// system.
        ///
        /// For example, if group `a` contains `bin/foo` and group `b` contains
        /// `bin/bar`, and `~/bin` does not already exist on the system, then
        /// failure to specify this option would cause a conflict as both
        /// groups would try to create the `~/bin` directory as a symbolic
        /// link.
        #[arg(long)]
        only_files: bool,
    },

    /// Delete the given groups from the dotfiles, cleaning up any left-over
    /// symbolic links.
    ///
    /// Note that this will remove the files on the system, rather than
    /// restoring them to their original non-symlinked state.
    #[command(arg_required_else_help = true)]
    Pop {
        groups: Vec<String>,

        /// Automatically answer yes on every prompt.
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

    /// Convert a GNU Stow dotfiles repository into a Tuckr repository.
    #[command(name = "from-stow")]
    FromStow { stow_path: Option<String> },
}

#[derive(Debug, Subcommand)]
enum ListType {
    /// Lists dotfiles directories with a suffix _<profile> (alias: p)
    #[command(alias = "p")]
    Profiles,
    /// Lists encrypted files (alias: s)
    #[command(alias = "s")]
    Secrets,
    /// Lists which hooks exists for each group (alias: h)
    #[command(alias = "h")]
    Hooks,
}

fn main() -> ExitCode {
    let cli = {
        // custom targets can be set permanently through env vars or set temporarily through the cli
        // so we need to append env var targets before running
        let mut cli = Cli::parse();
        if let Ok(custom_targets) = std::env::var("TUCKR_CUSTOM_TARGETS") {
            let mut custom_targets: Vec<_> =
                custom_targets.split(',').map(|t| t.to_string()).collect();
            cli.ctx.custom_targets.append(&mut custom_targets);
            cli.ctx.custom_targets.sort();
            cli.ctx.custom_targets.dedup();
        }
        cli
    };

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
            &cli.ctx, only_files, &groups, &exclude, force, adopt, assume_yes,
        ),

        Command::Unset { groups, exclude } => hooks::unset_cmd(&cli.ctx, &groups, &exclude),

        Command::Add {
            groups,
            exclude,
            force,
            adopt,
            assume_yes,
            only_files,
        } => symlinks::add_cmd(
            &cli.ctx, only_files, &groups, &exclude, force, adopt, assume_yes,
        ),

        Command::Rm { groups, exclude } => symlinks::remove_cmd(&cli.ctx, &groups, &exclude),

        Command::Status { groups, json } => symlinks::status_cmd(&cli.ctx, groups, json),

        Command::Encrypt { group, dotfiles } => secrets::encrypt_cmd(&cli.ctx, &group, &dotfiles),

        Command::Decrypt { groups, exclude } => secrets::decrypt_cmd(&cli.ctx, &groups, &exclude),

        Command::Init => fileops::init_cmd(&cli.ctx),

        Command::Ls(ls_type) => match ls_type {
            ListType::Profiles => fileops::ls_profiles_cmd(),
            ListType::Secrets => fileops::ls_secrets_cmd(&cli.ctx),
            ListType::Hooks => fileops::ls_hooks_cmd(&cli.ctx),
        },

        Command::Push {
            group,
            files,
            assume_yes,
            only_files,
            add,
        } => fileops::push_cmd(&cli.ctx, group, &files, add, only_files, assume_yes),

        Command::Pop { groups, assume_yes } => fileops::pop_cmd(&cli.ctx, &groups, assume_yes),

        Command::GroupIs { files } => fileops::groupis_cmd(&cli.ctx, &files),

        Command::FromStow { stow_path } => fileops::from_stow_cmd(&cli.ctx, stow_path),
    };

    match exit_code {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => e,
    }
}
