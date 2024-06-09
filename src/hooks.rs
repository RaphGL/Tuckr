//! Manages script running
//!
//! Hooks are run in a state machine.
//! Hooking steps:
//! 1. Setup scripts are run
//! 2. Dotfiles are symlinked
//! 3. Post setup scripts are run

use crate::symlinks;
use crate::utils::{self, DotfileGroup, ReturnCode};
use owo_colors::OwoColorize;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

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
fn run_hook(group: &str, hook_type: DeployStep) -> Result<(), ExitCode> {
    utils::print_info_box(
        match hook_type {
            DeployStep::PreHook => "Running Prehook",
            DeployStep::PostHook => "Running Posthook",
            _ => panic!("{:?} is not a valid step.", hook_type),
        },
        group.yellow().to_string().as_str(),
    );

    let dotfiles_dir = match utils::get_dotfiles_path() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("{e}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    let group_dir = PathBuf::from(&dotfiles_dir).join("Hooks").join(group);
    let Ok(group_dir) = fs::read_dir(group_dir) else {
        eprintln!("{}", "Could not read Hooks, folder may not exist or does not have the appropriate permissions".red());
        return Err(ReturnCode::NoSetupFolder.into());
    };

    for file in group_dir {
        let file = file.unwrap().path();
        let filename = file.file_name().unwrap().to_str().unwrap();
        let file = file.to_str().unwrap();
        // make sure it will only run for their specific hooks
        match hook_type {
            DeployStep::PreHook => {
                if !filename.starts_with("pre") {
                    continue;
                }
            }
            DeployStep::PostHook => {
                if !filename.starts_with("post") {
                    continue;
                }
            }
            _ => (),
        }

        let mut output = match Command::new(file).spawn() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("{e}");
                return Err(ExitCode::FAILURE);
            }
        };

        if output.wait().unwrap().success() {
            println!(
                "{}",
                format!("Hooked {group} {filename} successfully")
                    .green()
                    .to_string()
                    .as_str()
            );
        } else {
            utils::print_info_box(
                "Failed to hook".red().to_string().as_str(),
                format!("{group} {filename}").as_str(),
            );
        }
    }

    Ok(())
}

/// Runs hooks for specified groups
pub fn set_cmd(
    groups: &[String],
    exclude: &[String],
    force: bool,
    adopt: bool,
) -> Result<(), ExitCode> {
    if let Some(invalid_groups) = utils::check_invalid_groups(utils::DotfileType::Hooks, groups) {
        for group in invalid_groups {
            eprintln!("{}", format!("{group} does not exist.").red());
        }

        return Err(ReturnCode::NoSuchFileOrDir.into());
    }

    let run_deploy_steps = |step: DeployStages, group: DotfileGroup| -> Result<(), ExitCode> {
        if !group.is_valid_target() {
            return Ok(());
        }

        for i in step {
            match i {
                DeployStep::Initialize => return Ok(()),

                DeployStep::PreHook => {
                    run_hook(&group.name, DeployStep::PreHook)?;
                }

                DeployStep::Symlink => {
                    utils::print_info_box(
                        "Symlinking group",
                        group.name.yellow().to_string().as_str(),
                    );
                    symlinks::add_cmd(groups, exclude, force, adopt)?;
                }

                DeployStep::PostHook => run_hook(&group.name, DeployStep::PostHook)?,
            }
        }

        Ok(())
    };

    let hooks_dir = match utils::get_dotfiles_path() {
        Ok(dir) => dir.join("Hooks"),
        Err(e) => {
            eprintln!("{e}",);
            return Err(ReturnCode::NoSetupFolder.into());
        }
    };

    if groups.contains(&'*'.to_string()) {
        for folder in fs::read_dir(hooks_dir).unwrap() {
            let Some(group) = DotfileGroup::from(folder.unwrap().path()) else {
                eprintln!("Received an invalid group path.");
                return Err(ExitCode::FAILURE);
            };
            run_deploy_steps(DeployStages::new(), group)?;
        }

        return Ok(());
    }

    for group in groups {
        let Some(group) = DotfileGroup::from(hooks_dir.join(group)) else {
            eprintln!("Received an invalid group path.");
            return Err(ExitCode::FAILURE);
        };
        run_deploy_steps(DeployStages::new(), group)?;
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
