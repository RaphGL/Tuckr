use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use colored::Colorize;

/// Converts a stow directory into a tuckr directory
pub fn convert_to_tuckr() {
    print!("{}", "Are you sure you want to convert this repo to Tuckr?\nFiles that start with a dot will be ignored (y/N) ".yellow());
    io::stdout().flush().unwrap();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).unwrap();
    let answer = answer.to_lowercase().trim().to_owned();

    if answer == "y" {
        _ = fs::create_dir("Configs");
        _ = fs::create_dir("Hooks");
        _ = fs::create_dir("Encrypts");

        let cwd = env::current_dir().unwrap();
        let curr_path = cwd.to_str().unwrap();
        let cwd = fs::read_dir(&cwd).expect("Could not open current directory");

        for dir in cwd {
            let dir = dir.unwrap();
            let dirname = dir.file_name().clone().to_str().unwrap().to_owned();
            if dirname.starts_with(".") {
                continue;
            }

            let path = format!("{}/{}/{}", curr_path, "Configs", dirname);

            if !dirname.ends_with("Configs")
                && !dirname.ends_with("Hooks")
                && !dirname.ends_with("Encrypts")
            {
                fs::rename(dir.path().to_str().unwrap(), path).expect("Could not move files");
            }
        }
    }
}

/// Creates the necessary files and folders for a tuckr directory
pub fn init_tuckr_dir() {
    _ = fs::create_dir("Configs");
    _ = fs::create_dir("Hooks");
    _ = fs::create_dir("Encrypts");
}

/// Returns a Option<String> with the path to of the tuckr dotfiles directory
pub fn get_dotfiles_path() -> Option<String> {
    let home_dir = env::var("HOME").unwrap();
    let home_dotfiles = format!("{}/{}", home_dir, ".dotfiles");
    let config_dotfiles = format!("{}/{}", home_dir, ".config/dotfiles");

    if Path::new(&home_dotfiles).exists() {
        Some(home_dotfiles)
    } else if Path::new(&config_dotfiles).exists() {
        Some(config_dotfiles)
    } else {
        None
    }
}
