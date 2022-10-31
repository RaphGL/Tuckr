use crate::fileops;
use crate::symlinks;
use crate::utils;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

enum DeployStep {
    Initialize, // only used so prehook can be returned
    PreHook,
    Symlink,
    PostHook,
}

/// State machine for the dotfile deployment
struct DeployStages {
    stage: DeployStep,
}

impl DeployStages {
    fn new() -> DeployStages {
        DeployStages {
            stage: DeployStep::Initialize,
        }
    }
}

impl Iterator for DeployStages {
    type Item = DeployStep;

    fn next(&mut self) -> Option<DeployStep> {
        match self.stage {
            DeployStep::Initialize => {
                self.stage = DeployStep::PreHook;
                Some(DeployStep::PreHook)
            }
            DeployStep::PreHook => {
                self.stage = DeployStep::Symlink;
                Some(DeployStep::Symlink)
            }
            DeployStep::Symlink => {
                self.stage = DeployStep::PostHook;
                Some(DeployStep::PostHook)
            }
            DeployStep::PostHook => None,
        }
    }
}

/// Get's either PreHook or PostHook as hook_type
/// this allows it to choose which script to run
fn run_hook(program: &str, hook_type: DeployStep) {
    let dotfiles_dir = fileops::get_dotfiles_path().expect("Could not find dotfiles directory");
    let program_dir = PathBuf::from(&dotfiles_dir).join("Hooks").join(program);
    let program_dir = fs::read_dir(program_dir)
        .expect("Could not read Hooks, folder may not exist or have the appropriate permissions");

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
                format!("{}'s {} hooked", program, filename).green().bold()
            );
        } else {
            println!(
                "{}",
                format!("{}'s {} failed to hook", program, filename)
                    .red()
                    .bold()
            )
        }
    }
}

pub fn set_cmd(programs: &[String], exclude: &[String], force: bool) {
    let run_deploy_steps = |step: DeployStages, program: &str| {
        for i in step {
            match i {
                DeployStep::PreHook => {
                    run_hook(program, DeployStep::PreHook);
                }

                DeployStep::Symlink => symlinks::add_cmd(programs, exclude, force),

                DeployStep::PostHook => run_hook(program, DeployStep::PostHook),

                _ => unreachable!(),
            }
        }
    };

    for program in programs {
        if program == "*" {
            let dotfiles_dir = PathBuf::from(fileops::get_dotfiles_path().unwrap_or_else(|| {
                eprintln!("Could not find the Hooks directory in your dotfiles");
                std::process::exit(2);
            }))
            .join("Hooks");

            for folder in fs::read_dir(dotfiles_dir).unwrap() {
                let folder = folder.unwrap();
                run_deploy_steps(
                    DeployStages::new(),
                    utils::to_program_name(folder.path().to_str().unwrap()).unwrap(),
                );
            }
            break;
        } else {
            run_deploy_steps(DeployStages::new(), program);
        }
    }
}
