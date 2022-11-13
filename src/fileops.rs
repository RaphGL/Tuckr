use owo_colors::OwoColorize;
use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::path;

/// Converts a stow directory into a tuckr directory
pub fn convert_to_tuckr() {
    print!("{}", "Are you sure you want to convert this repo to Tuckr?\nFiles that start with a dot will be ignored (y/N) ".yellow());
    io::stdout().flush().unwrap();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).unwrap();
    let answer = answer.to_lowercase().trim().to_owned();

    if answer == "y" {
        // don't do anything if directory already exists
        init_tuckr_dir();

        let cwd = env::current_dir().unwrap();
        let curr_path = cwd.to_str().unwrap();
        let cwd = fs::read_dir(&cwd).expect("Could not open current directory");
        const IGNORED_FILES: &[&str] = &["COPYING", "LICENSE", "README.md"];

        for dir in cwd {
            let dir = dir.unwrap();
            let dirname = dir.file_name().to_str().unwrap().to_owned();
            if dirname.starts_with('.') || IGNORED_FILES.contains(&dirname.as_ref()) {
                continue;
            }

            let path = path::PathBuf::from(curr_path)
                .join("Configs")
                .join(&dirname);

            if !dirname.ends_with("Configs")
                && !dirname.ends_with("Hooks")
                && !dirname.ends_with("Encrypts")
            {
                fs::rename(dir.path().to_str().unwrap(), path).expect("Could not move files");
            }
        }
    }
}

/// Creates the necessary files and folders for a tuckr directory if they don't exist
pub fn init_tuckr_dir() {
    _ = fs::create_dir("Configs");
    _ = fs::create_dir("Hooks");
    _ = fs::create_dir("Encrypts");
}

/// Returns an Option<String> with the path to of the tuckr dotfiles directory
pub fn get_dotfiles_path() -> Option<path::PathBuf> {
    let home_dotfiles = dirs::home_dir().unwrap().join(".dotfiles");
    let config_dotfiles = dirs::config_dir().unwrap().join("dotfiles");

    if home_dotfiles.exists() {
        Some(home_dotfiles)
    } else if config_dotfiles.exists() {
        Some(config_dotfiles)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn get_dotfiles_path() {
        // /home/$USER/.dotfiles
        let home_dotfiles = dirs::home_dir().unwrap().join(".dotfiles");
        // /home/$USER/.config/dotfiles
        let config_dotfiles = dirs::config_dir().unwrap().join("dotfiles");

        assert!(match super::get_dotfiles_path().unwrap() {
            path if path == home_dotfiles
                || path == config_dotfiles =>
                true,
            _ => false,
        });
    }
}
