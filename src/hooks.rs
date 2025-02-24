//! Manages script running
//!
//! Hooks are run in a state machine.
//! Hooking steps:
//! 1. Setup scripts are run
//! 2. Dotfiles are symlinked
//! 3. Post setup scripts are run

use crate::dotfiles::{self, Dotfile, ReturnCode};
use crate::symlinks;
use owo_colors::OwoColorize;
use rust_i18n::t;
use std::fs;
use std::process::{Command, ExitCode};
use tabled::{Table, Tabled};

/// Prints a single row info box with title on the left
/// and content on the right
fn print_info_box(title: &str, content: &str) {
    let mut hook_box = tabled::builder::Builder::default()
        .set_columns([title])
        .add_record([content])
        .to_owned()
        .build();
    hook_box
        .with(tabled::Rotate::Left)
        .with(tabled::Style::rounded().off_vertical());
    println!("{hook_box}");
}

#[derive(Debug, PartialEq)]
enum DeployStep {
    Initialize, // Default value before starting deployment
    PreHook,
    Symlink,
    PostHook,
}

/// State machine for running hooks
struct DeployStages(DeployStep);

impl DeployStages {
    fn new() -> DeployStages {
        DeployStages(DeployStep::Initialize)
    }
}

impl Iterator for DeployStages {
    type Item = DeployStep;

    fn next(&mut self) -> Option<DeployStep> {
        match self.0 {
            DeployStep::Initialize => {
                self.0 = DeployStep::PreHook;
                Some(DeployStep::PreHook)
            }
            DeployStep::PreHook => {
                self.0 = DeployStep::Symlink;
                Some(DeployStep::Symlink)
            }
            DeployStep::Symlink => {
                self.0 = DeployStep::PostHook;
                Some(DeployStep::PostHook)
            }
            DeployStep::PostHook => None,
        }
    }
}

/// Runs hooks of type PreHook or PostHook
fn run_set_hook(
    profile: Option<String>,
    dry_run: bool,
    group: &str,
    hook_type: DeployStep,
) -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path(profile) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("{e}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    let group_dir = dotfiles_dir.join("Hooks").join(group);
    let Ok(group_dir) = fs::read_dir(group_dir) else {
        eprintln!("{}", t!("errors.could_not_read_hooks").red());
        return Err(ReturnCode::NoSetupFolder.into());
    };

    for file in group_dir {
        let file = file.unwrap().path();
        let filename = file.file_name().unwrap().to_str().unwrap();

        // make sure it will only run for their specific hooks
        match hook_type {
            DeployStep::PreHook => {
                if !filename.starts_with("pre") {
                    continue;
                }
                print_info_box(
                    &t!("info.running_prehook"),
                    group.yellow().to_string().as_str(),
                );
            }

            DeployStep::PostHook => {
                if !filename.starts_with("post") {
                    continue;
                }
                print_info_box(
                    &t!("info.running_posthook"),
                    group.yellow().to_string().as_str(),
                );
            }
            _ => (),
        }

        if dry_run {
            continue;
        }

        let mut output = match Command::new(&file).spawn() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("{e}");
                return Err(ExitCode::FAILURE);
            }
        };

        if !output.wait().unwrap().success() {
            print_info_box(
                t!("errors.failed_to_hook").red().to_string().as_str(),
                format!("{group} {filename}").as_str(),
            );
            return Err(ExitCode::FAILURE);
        }
    }

    Ok(())
}

macro_rules! get_hooks_dir_if_exists_or_run_cmd {
    ($profile:ident, $groups:ident, $cmd:expr) => {{
        if let Some(invalid_groups) =
            dotfiles::check_invalid_groups($profile.clone(), dotfiles::DotfileType::Hooks, $groups)
        {
            if dotfiles::check_invalid_groups(
                $profile.clone(),
                dotfiles::DotfileType::Configs,
                $groups,
            )
            .is_some()
            {
                for group in invalid_groups {
                    println!("{}", t!("errors.x_doesnt_exist", x = group).red());
                }

                return Err(ReturnCode::NoSuchFileOrDir.into());
            } else {
                return $cmd;
            }
        }

        match dotfiles::get_dotfiles_path($profile.clone()) {
            Ok(dir) => dir.join("Hooks"),
            Err(err) => {
                eprintln!("{}", err.red());
                return Err(ReturnCode::NoSetupFolder.into());
            }
        }
    }};
}

/// Runs hooks for specified groups and symlinks them
#[allow(clippy::too_many_arguments)]
pub fn set_cmd(
    profile: Option<String>,
    dry_run: bool,
    only_files: bool,
    groups: &[String],
    exclude: &[String],
    force: bool,
    adopt: bool,
    assume_yes: bool,
) -> Result<(), ExitCode> {
    let hooks_dir = get_hooks_dir_if_exists_or_run_cmd!(profile, groups, {
        println!("{}", "No hooks exist. Running `tuckr rm <args>`".yellow());
        symlinks::add_cmd(
            profile, dry_run, only_files, groups, exclude, force, adopt, assume_yes,
        )
    });

    let run_deploy_steps = |stages: DeployStages, group: &Dotfile| -> Result<(), ExitCode> {
        if !group.is_valid_target() {
            return Ok(());
        }

        for step in stages {
            match step {
                DeployStep::Initialize => return Ok(()),

                DeployStep::PreHook => {
                    run_set_hook(profile.clone(), dry_run, &group.group_name, step)?;
                }

                DeployStep::Symlink => {
                    if dotfiles::check_invalid_groups(
                        profile.clone(),
                        dotfiles::DotfileType::Configs,
                        &[&group.group_name],
                    )
                    .is_some()
                    {
                        continue;
                    }

                    print_info_box(
                        &t!("info.symlinking_group"),
                        group.group_name.yellow().to_string().as_str(),
                    );
                    symlinks::add_cmd(
                        profile.clone(),
                        dry_run,
                        only_files,
                        groups,
                        exclude,
                        force,
                        adopt,
                        assume_yes,
                    )?;
                }

                DeployStep::PostHook => {
                    run_set_hook(profile.clone(), dry_run, &group.group_name, step)?
                }
            }
        }

        Ok(())
    };

    #[derive(Tabled)]
    struct RunStatus<'a> {
        #[tabled(rename = "Hook")]
        group: String,
        #[tabled(rename = "Success")]
        succeeded: &'a str,
    }

    let true_symbol = "✓".green().to_string();
    let false_symbol = "✗".red().to_string();

    let get_symbol = |success: bool| -> &str { if success { &true_symbol } else { &false_symbol } };

    let mut hooks_summary: Vec<RunStatus> = Vec::new();

    if groups.contains(&'*'.to_string()) {
        for folder in fs::read_dir(hooks_dir).unwrap() {
            let folder = folder.unwrap().path();
            let Ok(group) = Dotfile::try_from(folder.clone()) else {
                eprintln!(
                    "{}",
                    format!("Got an invalid group: {}", folder.display()).red()
                );
                return Err(ExitCode::FAILURE);
            };

            hooks_summary.push(RunStatus {
                succeeded: get_symbol(run_deploy_steps(DeployStages::new(), &group).is_ok()),
                group: group.group_name,
            })
        }
    } else {
        // groups with their related conditional groups added
        let groups = {
            let mut groups = groups.to_vec();

            for file in hooks_dir.read_dir().unwrap() {
                let filename = file.unwrap().file_name();
                let filename = filename.into_string().unwrap();
                let base_group = dotfiles::group_without_target(&filename);

                if groups.iter().any(|g| g == base_group && *g != filename) {
                    groups.push(filename);
                }
            }

            // sorting is necessary to ensure that the conditional groups are run right after their base group
            groups.sort();
            groups
        };

        for group in groups {
            let hook_path = hooks_dir.join(group);
            let Ok(group) = Dotfile::try_from(hook_path.clone()) else {
                eprintln!(
                    "{}",
                    t!("errors.got_invalid_group", group = hook_path.display()).red()
                );
                return Err(ExitCode::FAILURE);
            };

            hooks_summary.push(RunStatus {
                succeeded: get_symbol(run_deploy_steps(DeployStages::new(), &group).is_ok()),
                group: group.group_name,
            })
        }
    }

    if groups.len() > 1 {
        use tabled::{Alignment, Margin, Modify, Style, object::Segment};

        let mut hooks_list = Table::new(hooks_summary);
        hooks_list
            .with(Style::rounded())
            .with(Margin::new(2, 4, 1, 1))
            .with(Modify::new(Segment::new(1.., 1..)).with(Alignment::center()));

        println!("\n\n Hooks have finished running. Here's a summary:");
        println!("{hooks_list}");
    }

    Ok(())
}

/// Runs cleanup hooks for groups and then removes all their symlinks
pub fn unset_cmd(
    profile: Option<String>,
    dry_run: bool,
    groups: &[String],
    exclude: &[String],
) -> Result<(), ExitCode> {
    let hooks_dir = get_hooks_dir_if_exists_or_run_cmd!(profile, groups, {
        println!("{}", "No hooks exist. Running `tuckr rm <args>`".yellow());
        symlinks::remove_cmd(profile, dry_run, groups, exclude)
    });

    for group in groups {
        let group_dir = hooks_dir.join(group);

        for file in group_dir.read_dir().unwrap() {
            let file = file.unwrap().path();
            let filename = file.file_name().unwrap().to_str().unwrap();

            if filename.starts_with("rm") {
                print_info_box("Running cleanup hook", group.yellow().to_string().as_str());

                if dry_run {
                    continue;
                }

                let hook = Command::new(&file).spawn();

                let mut output = match hook {
                    Ok(out) => out,
                    Err(err) => {
                        eprintln!("{err}");
                        return Err(ReturnCode::NoSuchFileOrDir.into());
                    }
                };

                if !output.wait().unwrap().success() {
                    print_info_box(
                        t!("errors.failed_to_hook").red().to_string().as_str(),
                        format!("{group} {filename}").as_str(),
                    );
                    return Err(ExitCode::FAILURE);
                }
            }
        }

        print_info_box(
            "Removing symlinked group",
            group.yellow().to_string().as_str(),
        );

        symlinks::remove_cmd(profile.clone(), dry_run, &[group.to_owned()], exclude)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_deploy_steps() {
        let mut steps = DeployStages::new();
        assert!(steps.0 == DeployStep::Initialize);
        steps.next();
        assert!(steps.0 == DeployStep::PreHook);
        steps.next();
        assert!(steps.0 == DeployStep::Symlink);
        steps.next();
        assert!(steps.0 == DeployStep::PostHook);
    }
}
