//! Creates basic file structure for tuckr
//!
//! Contains functions to create the base directories and to convert users from stow to tuckr

use crate::utils::{self, ReturnCode};
use owo_colors::OwoColorize;
use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;
use tabled::TableIteratorExt;

/// Converts a stow directory into a tuckr directory
pub fn from_stow_cmd() -> Result<(), ExitCode> {
    // --- Getting user confirmation ---
    print!(
        "Are you sure you want to convert your dotfiles to tuckr?\n\
        All files starting with a dot will be ignored (y/N) "
    );
    io::stdout().flush().unwrap();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).unwrap();
    let answer = answer.trim().to_lowercase();

    match answer.as_str() {
        "yes" | "y" => (),
        _ => return Ok(()),
    }

    // --- initializing required directory ---
    let dotfiles_dir = match utils::get_dotfiles_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{e}");
            return Err(ExitCode::from(ReturnCode::NoSetupFolder));
        }
    };

    let configs_path = dotfiles_dir.join("Configs");
    fs::create_dir_all(&configs_path).expect("Could not create required directory.");

    // --- Moving dotfiles to Configs/ ---
    let cwd = fs::read_dir(&dotfiles_dir).expect("Could not open current directory");
    const IGNORED_FILES: &[&str] = &["COPYING", "LICENSE", "README.md"];

    for dir in cwd {
        let dir = dir.unwrap();
        let dirname = dir.file_name().to_str().unwrap().to_owned();
        if dirname.starts_with('.') || IGNORED_FILES.contains(&dirname.as_str()) {
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

    let dotfiles_dir = dirs::config_dir().unwrap().join("dotfiles");

    create_dirs!(
        dotfiles_dir.join("Configs"),
        dotfiles_dir.join("Hooks"),
        dotfiles_dir.join("Secrets")
    );

    println!(
        "A dotfiles directory has been created on {}.",
        dotfiles_dir.to_str().unwrap()
    );

    Ok(())
}

fn list_tuckr_dir(dirname: &str) -> Result<(), ExitCode> {
    let dir = match utils::get_dotfiles_path() {
        Ok(dir) => dir.join(dirname),
        Err(_) => return Err(ExitCode::from(ReturnCode::CouldntFindDotfiles)),
    };

    let dirs = match fs::read_dir(dir) {
        Ok(dir) => dir
            .into_iter()
            .map(|dir| dir.unwrap().file_name().to_str().unwrap().to_string()),

        Err(_) => return Err(ExitCode::from(ReturnCode::NoSetupFolder)),
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
