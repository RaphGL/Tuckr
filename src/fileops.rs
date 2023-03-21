//! Creates basic file structure for tuckr
//!
//! Contains functions to create the base directories and to convert users from stow to tuckr

use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::process::ExitCode;
use std::{env, fs, path};
use tabled::TableIteratorExt;

use crate::utils;

/// Converts a stow directory into a tuckr directory
pub fn from_stow_cmd() -> Result<(), ExitCode> {
    print!("{}", "Are you sure you want to convert the current directory to tuckr?\nAll files starting with a dot will be ignored (y/N) ");
    io::stdout().flush().unwrap();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).unwrap();
    let answer = answer.to_lowercase().trim().to_owned();

    match answer.as_str() {
        "yes" | "y" => (),
        _ => return Ok(()),
    }

    init_cmd()?;

    let cwd = env::current_dir().unwrap();
    let curr_path = cwd.to_str().unwrap();
    let cwd = fs::read_dir(&cwd).expect("Could not open current directory");
    const IGNORED_FILES: &[&str] = &["COPYING", "LICENSE", "README.md"];

    for dir in cwd {
        let dir = dir.unwrap();
        let dirname = dir.file_name().to_str().unwrap().to_owned();
        if dirname.starts_with('.') || IGNORED_FILES.contains(&dirname.as_str()) {
            continue;
        }

        let path = path::PathBuf::from(curr_path)
            .join("Configs")
            .join(&dirname);

        if !dirname.ends_with("Configs")
            && !dirname.ends_with("Hooks")
            && !dirname.ends_with("Secrets")
        {
            fs::rename(dir.path().to_str().unwrap(), path).expect("Could not move files");
        }
    }

    Ok(())
}

/// Creates the necessary files and folders for a tuckr directory if they don't exist
pub fn init_cmd() -> Result<(), ExitCode> {
    macro_rules! create_dirs {
        ($($dirname: literal),+) => {
            $(
            if let Err(e) = fs::create_dir($dirname) {
                eprintln!("{}", e.red());
                return Err(ExitCode::FAILURE);
            })+
        };
    }

    create_dirs!("Configs", "Hooks", "Secrets");

    Ok(())
}

fn list_tuckr_dir(dirname: &str) -> Result<(), ExitCode> {
    let dir = {
        let dotfiles_dir = if let Some(dir) = utils::get_dotfiles_path() {
            dir
        } else {
            return Err(ExitCode::from(utils::COULDNT_FIND_DOTFILES));
        };

        dotfiles_dir.join(dirname)
    };

    let dirs = if let Ok(dir) = fs::read_dir(dir) {
        dir.into_iter()
            .map(|dir| dir.unwrap().file_name().to_str().unwrap().to_string())
    } else {
        return Err(ExitCode::from(utils::NO_SETUP_FOLDER));
    };

    let mut dirs_table = dirs.table();
    dirs_table
        .with(tabled::Style::empty())
        .with(tabled::Disable::row(tabled::object::FirstRow));
    // TODO: add back once tabled::Split is available
    //.with(tabled::Rotate::Left)
    //.with(tabled::Disable::column(tabled::object::FirstColumn));

    println!("{dirs_table}");

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
