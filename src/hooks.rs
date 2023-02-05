//! Manages script running
//!
//! Hooks are run in a state machine. 
//! Hooking steps:
//! 1. Setup scripts are run
//! 2. Dotfiles are symlinked
//! 3. Post setup scripts are run

use crate::symlinks;
use crate::utils;
use owo_colors::OwoColorize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(PartialEq)]
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
fn run_hook(program: &str, hook_type: DeployStep) {
    utils::print_info_box(
        match hook_type {
            DeployStep::PreHook => "Running Prehook",
            DeployStep::PostHook => "Running Posthook",
            _ => unreachable!(),
        },
        program.yellow().to_string().as_str(),
    );

    let dotfiles_dir = utils::get_dotfiles_path().unwrap_or_else(|| {
        eprintln!("{}", "Could not find dotfiles directory".red());
        std::process::exit(utils::COULDNT_FIND_DOTFILES);
    });

    let program_dir = PathBuf::from(&dotfiles_dir).join("Hooks").join(program);
    let program_dir = fs::read_dir(program_dir).unwrap_or_else(|_| {
        eprintln!("{}", "Could not read Hooks, folder may not exist or does not have the appropriate permissions".red());
        std::process::exit(utils::NO_SETUP_FOLDER);
    });

    for file in program_dir {
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

        let mut output = Command::new("sh")
            .arg("-c")
            .arg(file)
            .spawn()
            .expect("Failed to run hook");

        if output.wait().unwrap().success() {
            println!(
                "{}",
                format!("Hooked {program} {filename} successfully")
                    .green()
                    .to_string()
                    .as_str()
            );
        } else {
            utils::print_info_box(
                "Failed to hook".red().to_string().as_str(),
                format!("{program} {filename}").as_str(),
            );
        }
    }
}

/// Runs hooks for specified programs/groups
pub fn set_cmd(programs: &[String], exclude: &[String], force: bool, adopt: bool) {
    let run_deploy_steps = |step: DeployStages, program: &str| {
        for i in step {
            match i {
                DeployStep::Initialize => return,

                DeployStep::PreHook => {
                    run_hook(program, DeployStep::PreHook);
                }

                DeployStep::Symlink => {
                    utils::print_info_box(
                        "Symlinking program",
                        program.yellow().to_string().as_str(),
                    );
                    symlinks::add_cmd(programs, exclude, force, adopt);
                }

                DeployStep::PostHook => run_hook(program, DeployStep::PostHook),
            }
        }
    };

    for program in programs {
        if program == "*" {
            let dotfiles_dir = utils::get_dotfiles_path()
                .unwrap_or_else(|| {
                    eprintln!(
                        "{}",
                        "Could not find the Hooks directory in your dotfiles".red()
                    );
                    std::process::exit(utils::NO_SETUP_FOLDER);
                })
                .join("Hooks");

            for folder in fs::read_dir(dotfiles_dir).unwrap() {
                let folder = folder.unwrap();
                run_deploy_steps(
                    DeployStages::new(),
                    utils::to_program_name(folder.path().to_str().unwrap()).unwrap(),
                );
            }
        } else {
            run_deploy_steps(DeployStages::new(), program);
        }
    }
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
