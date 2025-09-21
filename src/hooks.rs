//! Manages script running
//!
//! Hooks are run in a state machine.
//! Hooking steps:
//! 1. Setup scripts are run
//! 2. Dotfiles are symlinked
//! 3. Post setup scripts are run

use crate::Context;
use crate::dotfiles::{self, ReturnCode};
use crate::symlinks;
use core::slice;
use owo_colors::OwoColorize;
use rust_i18n::t;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use tabled::{Table, Tabled};

/// Prints a single row info box with title on the left
/// and content on the right
fn print_info_box(title: &str, content: &str) {
    let mut box_builder = tabled::builder::Builder::new();
    box_builder.push_column([title]);
    box_builder.push_record([content]);

    let mut box_table = box_builder.build();
    box_table
        .with(tabled::settings::Rotate::Left)
        .with(tabled::settings::Style::modern_rounded().remove_vertical());

    println!("{box_table}");
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
fn run_set_hook(ctx: &Context, group: &str, hook_type: DeployStep) -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path(ctx.profile.clone()) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("{e}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    let group_dir = dotfiles_dir.join("Hooks").join(group);
    // a hook might just be a `tuckr add` meaning, so a corresponding hooks group dir might just not exist at all
    if !group_dir.exists() {
        return Ok(());
    }
    std::env::set_current_dir(&group_dir).unwrap();

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

        if ctx.dry_run {
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

fn all_hooks_are_nonexistent(profile: Option<String>, groups: &[String]) -> bool {
    let Some(nonexistent_groups) =
        dotfiles::get_nonexistent_groups(profile.clone(), dotfiles::DotfileType::Hooks, groups)
    else {
        return false;
    };

    nonexistent_groups.len() == groups.len()
}

/// Runs hooks for specified groups and symlinks them
#[allow(clippy::too_many_arguments)]
pub fn set_cmd(
    ctx: &Context,
    only_files: bool,
    groups: &[String],
    exclude: &[String],
    force: bool,
    adopt: bool,
    assume_yes: bool,
) -> Result<(), ExitCode> {
    if all_hooks_are_nonexistent(ctx.profile.clone(), groups) {
        println!(
            "{}",
            t!("info.no_hooks_exist_running_cmd", cmd = "tuckr add").yellow()
        );
        return symlinks::add_cmd(ctx, only_files, groups, exclude, force, adopt, assume_yes);
    }

    let hooks_dir = match dotfiles::get_dotfiles_path(ctx.profile.clone()) {
        Ok(dir) => dir.join("Hooks"),
        Err(err) => {
            eprintln!("{}", err.red());
            return Err(ReturnCode::NoSetupFolder.into());
        }
    };

    let run_deploy_steps = |stages: DeployStages, group: String| -> Result<(), ExitCode> {
        if !dotfiles::group_is_valid_target(&group, &ctx.custom_targets) || exclude.contains(&group)
        {
            return Ok(());
        }

        for step in stages {
            match step {
                DeployStep::Initialize => return Ok(()),

                DeployStep::PreHook => {
                    run_set_hook(ctx, &group, step)?;
                }

                DeployStep::Symlink => {
                    if !dotfiles::dotfile_contains(
                        ctx.profile.clone(),
                        dotfiles::DotfileType::Configs,
                        &group,
                    ) {
                        continue;
                    }

                    print_info_box(
                        &t!("info.symlinking_group"),
                        group.yellow().to_string().as_str(),
                    );
                    symlinks::add_cmd(
                        ctx,
                        only_files,
                        slice::from_ref(&group),
                        exclude,
                        force,
                        adopt,
                        assume_yes,
                    )?;
                }

                DeployStep::PostHook => run_set_hook(ctx, &group, step)?,
            }
        }

        Ok(())
    };

    let mut groups = if groups.contains(&'*'.to_string()) {
        let mut groups = Vec::new();
        let mut add_group_dotfiles = |dir: PathBuf| -> Result<(), ExitCode> {
            for folder in fs::read_dir(dir).unwrap() {
                let folder = folder.unwrap();
                groups.push(folder.file_name().into_string().unwrap());
            }

            Ok(())
        };

        add_group_dotfiles(hooks_dir)?;

        let configs_dir = dotfiles::get_dotfiles_path(ctx.profile.clone())
            .unwrap()
            .join("Configs");

        if configs_dir.exists() {
            add_group_dotfiles(configs_dir)?;
        }
        groups
    } else {
        // groups with their related conditional groups added
        let mut expanded_groups = groups.to_vec();

        for file in hooks_dir.read_dir().unwrap() {
            let filename = file.unwrap().file_name().into_string().unwrap();
            let base_group = dotfiles::group_without_target(&filename);

            if expanded_groups
                .iter()
                .any(|group| group == base_group && *group != filename)
            {
                expanded_groups.push(filename);
            }
        }

        expanded_groups
    };
    // sorting is necessary to ensure that the conditional groups are run right after their base group
    groups.sort();
    groups.dedup();
    // trick to restore immutability
    let groups = groups;

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
    for group in &groups {
        hooks_summary.push(RunStatus {
            succeeded: get_symbol(run_deploy_steps(DeployStages::new(), group.clone()).is_ok()),
            group: group.clone(),
        })
    }

    if groups.len() > 1 {
        use tabled::settings::{Alignment, Margin, Modify, Style, object::Segment};

        let mut hooks_list = Table::new(hooks_summary);
        hooks_list
            .with(Style::rounded())
            .with(Margin::new(2, 4, 1, 1))
            .with(Modify::new(Segment::new(1.., 1..)).with(Alignment::center()));

        println!("\n\n{}", t!("info.hooks_finished").green());
        println!("{hooks_list}");
    }

    Ok(())
}

/// Runs cleanup hooks for groups and then removes all their symlinks
pub fn unset_cmd(ctx: &Context, groups: &[String], exclude: &[String]) -> Result<(), ExitCode> {
    if all_hooks_are_nonexistent(ctx.profile.clone(), groups) {
        println!(
            "{}",
            t!("info.no_hooks_exist_running_cmd", cmd = "tuckr rm").yellow()
        );
        return symlinks::remove_cmd(ctx, groups, exclude);
    }

    let hooks_dir = match dotfiles::get_dotfiles_path(ctx.profile.clone()) {
        Ok(dir) => dir.join("Hooks"),
        Err(err) => {
            eprintln!("{}", err.red());
            return Err(ReturnCode::NoSetupFolder.into());
        }
    };

    let wildcard = String::from("*");
    if groups.contains(&wildcard) {
        return symlinks::remove_cmd(ctx, &[wildcard], exclude);
    }

    for group in groups {
        let group_dir = hooks_dir.join(group);
        std::env::set_current_dir(&group_dir).unwrap();

        for file in group_dir.read_dir().unwrap() {
            let file = file.unwrap().path();
            let filename = file.file_name().unwrap().to_str().unwrap();

            if filename.starts_with("rm") {
                print_info_box("Running cleanup hook", group.yellow().to_string().as_str());

                if ctx.dry_run {
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

        symlinks::remove_cmd(ctx, &[group.to_owned()], exclude)?;
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
