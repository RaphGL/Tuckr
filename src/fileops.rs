//! Creates basic file structure for tuckr
//!
//! Contains functions to create the base directories and to convert users from stow to tuckr

use crate::dotfiles::{self, ReturnCode};
use crate::fileops;
use owo_colors::OwoColorize;
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{fs, path};
use tabled::object::Segment;
use tabled::{Alignment, Modify, Table, Tabled};

pub struct DirWalk {
    queue: Vec<path::PathBuf>,
}

impl DirWalk {
    pub fn new(dir_path: impl AsRef<Path>) -> Self {
        let dir_path = dir_path.as_ref();
        let dir = match fs::read_dir(dir_path) {
            Ok(f) => f,
            Err(_) => panic!("{} does not exist", dir_path.to_str().unwrap()),
        };

        Self {
            queue: dir.map(|f| f.unwrap().path()).collect(),
        }
    }
}

impl Iterator for DirWalk {
    type Item = path::PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        let curr_file = self.queue.pop()?;

        if curr_file.is_dir() {
            for file in fs::read_dir(&curr_file).unwrap() {
                let file = file.unwrap();
                self.queue.push(file.path());
            }
        }

        Some(curr_file)
    }
}

/// Converts a stow directory into a tuckr directory
pub fn from_stow_cmd(profile: Option<String>) -> Result<(), ExitCode> {
    // assume that from_stow is always run from a no profile dotfiles dir
    let dotfiles_dir = match dotfiles::get_dotfiles_path(profile) {
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
pub fn init_cmd(profile: Option<String>) -> Result<(), ExitCode> {
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
        dotfiles::get_dotfiles_path(None).unwrap()
    } else {
        let dotfiles_dir_name = match profile {
            Some(profile) => "dotfiles_".to_string() + profile.as_str(),
            None => "dotfiles".to_string(),
        };
        dirs::config_dir().unwrap().join(dotfiles_dir_name)
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

pub fn push_cmd(profile: Option<String>, group: String, files: &[String]) -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path(profile) {
        Ok(dir) => dir.join("Configs").join(group),
        Err(e) => {
            eprintln!("{e}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    let mut any_file_failed = false;
    for file in files {
        let file = PathBuf::from(file);
        if !file.exists() {
            eprintln!("{}", format!("{} does not exist.", file.display()).yellow());
            any_file_failed = true;
            continue;
        }

        let file = path::absolute(file).unwrap();
        let target_file = dotfiles_dir.join(dotfiles::get_target_basepath(&file));
        let target_dir = target_file.parent().unwrap();

        if !target_file.exists() {
            fs::create_dir_all(target_dir).unwrap();

            if cfg!(target_family = "unix") || file.is_file() {
                fs::copy(file, target_file).unwrap();
            } else {
                for f in fileops::DirWalk::new(file) {
                    let file = path::absolute(f).unwrap();
                    let target_file = dotfiles_dir.join(dotfiles::get_target_basepath(&file));
                    fs::create_dir_all(target_file.parent().unwrap()).unwrap();
                    fs::copy(file, target_file).unwrap();
                }
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

pub fn pop_cmd(profile: Option<String>, groups: &[String]) -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path(profile) {
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

pub fn ls_hooks_cmd(profile: Option<String>) -> Result<(), ExitCode> {
    let dir = match dotfiles::get_dotfiles_path(profile) {
        Ok(dir) => dir.join("Hooks"),
        Err(err) => {
            eprintln!("{err}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    if !dir.exists() {
        eprintln!(
            "{}",
            "There's no directory setup for Hooks".to_string().red()
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
        println!("{}", "No hooks have been set up yet.".to_string().yellow());
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

// todo: make ls-secrets command prettier
pub fn ls_secrets_cmd(profile: Option<String>) -> Result<(), ExitCode> {
    let secrets_dir = dotfiles::get_dotfiles_path(profile)
        .unwrap()
        .join("Secrets");

    let Ok(secrets) = secrets_dir.read_dir() else {
        return Err(ReturnCode::NoSetupFolder.into());
    };

    for secret in secrets {
        let secret = secret.unwrap();
        println!("{}", secret.file_name().to_str().unwrap());
    }
    Ok(())
}

pub fn ls_profiles_cmd() -> Result<(), ExitCode> {
    let home_dir = dirs::home_dir().unwrap();
    let config_dir = dirs::config_dir().unwrap();

    fn get_profiles_from_dir(dir: PathBuf) -> HashSet<String> {
        let mut available_profiles = HashSet::new();

        for file in dir.read_dir().unwrap() {
            let file = file.unwrap();

            let Some(profile) = dotfiles::get_dotfile_profile_from_path(file.path()) else {
                continue;
            };

            available_profiles.insert(profile);
        }

        available_profiles
    }

    let home_profiles = get_profiles_from_dir(home_dir);
    let config_profiles = get_profiles_from_dir(config_dir);

    let profiles: HashSet<_> = home_profiles.union(&config_profiles).collect();
    if profiles.is_empty() {
        println!("{}", "No profiles have been set up yet.".yellow());
        return Ok(());
    }

    println!("Profiles available:");
    for profile in profiles {
        println!("\t{profile}");
    }

    Ok(())
}

pub fn groupis_cmd(profile: Option<String>, files: &[String]) -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path(profile) {
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

        if let Ok(dotfile) = dotfiles::Dotfile::try_from(file_path.clone()) {
            println!("{}", dotfile.group_name);
            continue;
        }

        while !file_path.is_symlink() {
            if !file_path.pop() {
                eprintln!("{}", format!("`{file}` is not a tuckr dotfile.").red());
                break 'next_file;
            }
        }

        let basepath = dotfiles::get_target_basepath(&file_path);

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
