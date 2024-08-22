//! Creates basic file structure for tuckr
//!
//! Contains functions to create the base directories and to convert users from stow to tuckr

use crate::dotfiles::{self, ReturnCode};
use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{fs, path};
use tabled::object::Segment;
use tabled::{Alignment, Modify, Table, Tabled};

pub fn dir_map<F>(dir_path: impl AsRef<Path>, mut func: F)
where
    F: FnMut(&Path),
{
    let dir_path = dir_path.as_ref();
    let dir = match fs::read_dir(dir_path) {
        Ok(f) => f,
        Err(_) => panic!("{} does not exist", dir_path.to_str().unwrap()),
    };

    let mut queue: Vec<path::PathBuf> = dir.map(|f| f.unwrap().path()).collect();

    while let Some(curr_file) = queue.pop() {
        func(&curr_file);

        if curr_file.is_dir() {
            for dir in fs::read_dir(curr_file).unwrap() {
                let dir = dir.unwrap();
                queue.push(dir.path());
            }
        }
    }
}

/// Converts a stow directory into a tuckr directory
pub fn from_stow_cmd() -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{e}");
            return Err(ReturnCode::NoSetupFolder.into());
        }
    };

    // --- Getting user confirmation ---
    println!(
        "{}",
        format!(
            "The dotfiles at `{}` will be converted into Tuckr.",
            dotfiles_dir.display()
        )
        .yellow()
    );
    print!("Are you sure you want to convert your dotfiles to tuckr? (y/N)");
    io::stdout().flush().unwrap();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).unwrap();
    if !matches!(answer.trim().to_lowercase().as_str(), "yes" | "y") {
        return Ok(());
    }

    // --- initializing required directory ---
    let configs_path = dotfiles_dir.join("Configs");
    fs::create_dir_all(&configs_path).expect("Could not create required directory.");

    // --- Moving dotfiles to Configs/ ---
    let cwd = fs::read_dir(&dotfiles_dir).expect("Could not open current directory");

    for file in cwd {
        let dir = file.unwrap();
        if !dir.metadata().unwrap().is_dir() {
            continue;
        }

        let dirname = dir.file_name().to_str().unwrap().to_owned();
        if dirname.starts_with('.') {
            continue;
        }

        let path = configs_path.join(&dirname);

        if !dirname.ends_with("Configs")
            && !dirname.ends_with("Hooks")
            && !dirname.ends_with("Secrets")
        {
            fs::rename(dir.path(), path).expect("Could not move files");
        }
    }

    Ok(())
}

/// Creates the necessary files and folders for a tuckr directory if they don't exist
pub fn init_cmd() -> Result<(), ExitCode> {
    macro_rules! create_dirs {
        ($($dirname: expr),+) => {
            $(
            if let Err(e) = fs::create_dir_all($dirname) {
                eprintln!("{}", e.red());
                return Err(ExitCode::FAILURE);
            })+
        };
    }

    let dotfiles_dir = if cfg!(test) {
        dotfiles::get_dotfiles_path().unwrap()
    } else {
        dirs::config_dir().unwrap().join("dotfiles")
    };

    create_dirs!(
        dotfiles_dir.join("Configs"),
        dotfiles_dir.join("Hooks"),
        dotfiles_dir.join("Secrets")
    );

    println!(
        "{}",
        format!(
            "A dotfiles directory has been created on `{}`.",
            dotfiles_dir.to_str().unwrap()
        )
        .green()
    );

    Ok(())
}

pub fn push_cmd(group: String, files: &[String]) -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path() {
        Ok(dir) => dir.join("Configs").join(&group),
        Err(e) => {
            eprintln!("{e}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    let home_dir = dirs::home_dir().unwrap();

    let mut any_file_failed = false;
    for file in files {
        let file = PathBuf::from(file);
        if !file.exists() {
            eprintln!("{}", format!("{} does not exist.", file.display()).yellow());
            any_file_failed = true;
            continue;
        }

        let file = path::absolute(file).unwrap();
        let target_file = dotfiles_dir.join(file.strip_prefix(&home_dir).unwrap());
        let target_dir = target_file.parent().unwrap();

        if !target_file.exists() {
            fs::create_dir_all(target_dir).unwrap();

            if cfg!(target_family = "unix") || file.is_file() {
                fs::copy(file, target_file).unwrap();
            } else {
                dir_map(file, |f| {
                    let file = path::absolute(f).unwrap();
                    let target_file = dotfiles_dir.join(file.strip_prefix(&home_dir).unwrap());
                    fs::create_dir_all(target_file.parent().unwrap()).unwrap();
                    fs::copy(file, target_file).unwrap();
                });
            }
        } else {
            print!(
                "{} already exists. Do you want to override it? (y/N) ",
                target_file.to_str().unwrap()
            );
            std::io::stdout().flush().unwrap();
            let mut confirmation = String::new();
            std::io::stdin().read_line(&mut confirmation).unwrap();

            let confirmed = matches!(confirmation.trim().to_lowercase().as_str(), "y" | "yes");

            if confirmed {
                fs::create_dir_all(target_dir).unwrap();
                fs::copy(file, target_file).unwrap();
            }
        }
    }

    if any_file_failed {
        Err(ReturnCode::NoSuchFileOrDir.into())
    } else {
        Ok(())
    }
}

pub fn pop_cmd(groups: &[String]) -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path() {
        Ok(dir) => dir.join("Configs"),
        Err(e) => {
            eprintln!("{e}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    let mut valid_groups = Vec::new();
    let mut invalid_groups = Vec::new();
    for group in groups {
        let group_dir = dotfiles_dir.join(group);
        if !group_dir.is_dir() {
            invalid_groups.push(group);
            continue;
        }

        if !group_dir.exists() {
            invalid_groups.push(group);
        } else {
            valid_groups.push(group_dir);
        }
    }

    if !invalid_groups.is_empty() {
        for group in invalid_groups {
            eprintln!("{}", format!("{} does not exist.", group).yellow());
        }

        return Err(ReturnCode::NoSuchFileOrDir.into());
    }

    println!("The following groups will be removed:");
    for group in groups {
        println!("\t{}", group.yellow());
    }
    print!("\nDo you want to proceed? (y/N) ");
    std::io::stdout().flush().unwrap();
    let mut confirmation = String::new();
    std::io::stdin().read_line(&mut confirmation).unwrap();

    let confirmed = matches!(confirmation.trim().to_lowercase().as_str(), "y" | "yes");

    if !confirmed {
        return Ok(());
    }

    for group_path in valid_groups {
        fs::remove_dir_all(group_path).unwrap();
    }

    Ok(())
}

fn list_tuckr_dir(dirname: &str) -> Result<(), ExitCode> {
    let dir = match dotfiles::get_dotfiles_path() {
        Ok(dir) => dir.join(dirname),
        Err(err) => {
            eprintln!("{err}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    if !dir.exists() {
        eprintln!(
            "{}",
            format!("There's no directory setup for {dirname}").red()
        );
        return Err(ReturnCode::NoSetupFolder.into());
    }

    #[derive(Tabled)]
    struct ListRow<'a> {
        #[tabled(rename = "Group")]
        group: String,
        #[tabled(rename = "Prehook")]
        prehook: &'a str,
        #[tabled(rename = "Posthook")]
        posthook: &'a str,
    }

    let dir = fs::read_dir(dir).unwrap();
    let mut rows = Vec::new();

    let true_symbol = "✓".green().to_string();
    let false_symbol = "✗".red().to_string();

    for hook in dir {
        let hook_dir = hook.unwrap();
        let hook_name = hook_dir.file_name();
        let group = hook_name.to_str().unwrap().to_string();

        let mut hook_entry = ListRow {
            group,
            prehook: &false_symbol,
            posthook: &false_symbol,
        };

        for hook in fs::read_dir(hook_dir.path()).unwrap() {
            let hook = hook.unwrap().file_name();
            let hook = hook.to_str().unwrap();
            if hook.starts_with("pre") {
                hook_entry.prehook = &true_symbol;
            } else if hook.starts_with("post") {
                hook_entry.posthook = &true_symbol;
            }
        }

        rows.push(hook_entry);
    }

    if rows.is_empty() {
        println!(
            "{}",
            format!("No {} have been set up yet.", dirname.to_lowercase(),).yellow()
        );
        return Ok(());
    }

    use tabled::{Margin, Style};

    let mut hooks_list = Table::new(rows);
    hooks_list
        .with(Style::rounded())
        .with(Margin::new(4, 4, 1, 1))
        .with(Modify::new(Segment::new(1.., 1..)).with(Alignment::center()));
    println!("{hooks_list}");

    Ok(())
}

pub fn ls_hooks_cmd() -> Result<(), ExitCode> {
    list_tuckr_dir("Hooks")?;
    Ok(())
}

pub fn ls_secrets_cmd() -> Result<(), ExitCode> {
    list_tuckr_dir("Secrets")?;
    Ok(())
}

pub fn groupis_cmd(files: &[String]) -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{e}");
            return Err(ReturnCode::NoSetupFolder.into());
        }
    }
    .join("Configs");

    let groups: Vec<_> = dotfiles_dir
        .read_dir()
        .unwrap()
        .filter_map(|f| {
            let f = f.unwrap();
            if f.file_type().unwrap().is_dir() {
                Some(f.file_name().into_string().unwrap())
            } else {
                None
            }
        })
        .collect();

    'next_file: for file in files {
        let mut file_path = PathBuf::from(file);
        if !file_path.exists() {
            eprintln!("{}", format!("`{file} does not exist.`").red());
            continue;
        }

        while !file_path.is_symlink() {
            if !file_path.pop() {
                eprintln!("{}", format!("`{file}` is not a tuckr dotfile.").red());
                break 'next_file;
            }
        }

        let basepath = dotfiles::get_target_basepath(file_path);

        for group in &groups {
            let dotfile_path = dotfiles_dir.join(group).join(&basepath);

            if !dotfile_path.exists() {
                continue;
            }

            let dotfile = match dotfiles::Dotfile::try_from(dotfile_path) {
                Ok(dotfile) => dotfile,
                Err(err) => {
                    eprintln!("{err}");
                    continue;
                }
            };

            println!("{}", dotfile.group_name);
        }
    }

    Ok(())
}
