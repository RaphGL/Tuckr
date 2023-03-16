//! A set of helper functions that reduce boilerplate

use std::env;
use std::fs;
use std::path;

// Exit codes
/// Couldn't find the dotfiles directory
pub const COULDNT_FIND_DOTFILES: u8 = 2;
/// No Configs/Hooks/Secrets folder setup
pub const NO_SETUP_FOLDER: u8 = 3;
/// Referenced file does not exist in the current directory
pub const NO_SUCH_FILE_OR_DIR: u8 = 4;
/// Failed to encrypt referenced file
pub const ENCRYPTION_FAILED: u8 = 5;
/// Failed to decrypt referenced file
pub const DECRYPTION_FAILED: u8 = 6;

pub fn has_valid_target(group: &str) -> bool {
    let target_os = format!("_{}", env::consts::OS);
    let target_family = format!("_{}", env::consts::FAMILY);

    group.ends_with(&target_os) || group.ends_with(&target_family) || !group.contains("_")
}

/// Returns an Option<String> with the path to of the tuckr dotfiles directory
pub fn get_dotfiles_path() -> Option<path::PathBuf> {
    let home_dotfiles = dirs::home_dir().unwrap().join(".dotfiles");
    let config_dotfiles = dirs::config_dir().unwrap().join("dotfiles");

    if home_dotfiles.exists() {
        Some(home_dotfiles)
    } else if config_dotfiles.exists() {
        Some(config_dotfiles)
    } else if cfg!(test) {
        Some(
            std::env::temp_dir()
                .join(format!("tuckr-{}", std::process::id()))
                .join("dotfiles"),
        )
    } else {
        None
    }
}

/// Converts a path string from dotfiles/Configs to where they should be
/// deployed on $HOME
pub fn to_home_path(path: &str) -> path::PathBuf {
    // uses join("") so that the path appends / or \ depending on platform
    let dotfiles_configs_path = get_dotfiles_path().unwrap().join("Configs").join("");
    let dotfiles_configs_path = dotfiles_configs_path.to_str().unwrap();

    dirs::home_dir().unwrap().join(
        path.strip_prefix(dotfiles_configs_path)
            .unwrap()
            .split_once(path::MAIN_SEPARATOR)
            .unwrap()
            .1,
    )
}

/// Extracts group name from tuckr directories
pub fn to_group_name(path: &str) -> Option<&str> {
    let dir = if path.contains("Configs") {
        "Configs"
    } else if path.contains("Hooks") {
        "Hooks"
    } else if path.contains("Secrets") {
        "Secrets"
    } else {
        return None;
    };

    // uses join("") so that the path appends / or \ depending on platform
    let config_path = path::PathBuf::from("dotfiles").join(dir).join("");

    Some(path.split_once(config_path.to_str().unwrap()).unwrap().1)
}

/// Goes through every file in Configs/<group_dir> and applies the function
pub fn group_dir_map<F: FnMut(fs::DirEntry)>(dir: path::PathBuf, mut func: F) {
    let group_dir = match fs::read_dir(&dir) {
        Ok(f) => f,
        Err(_) => panic!("{} does not exist", dir.to_str().unwrap()),
    };

    for file in group_dir {
        let file = file.unwrap();
        match file.file_name().to_str().unwrap() {
            // Special folders that should not be handled directly ("owned" by the system)
            // everything inside of it should be handled instead
            ".config" | "Pictures" | "Documents" | "Desktop" | "Downloads" | "Public"
            | "Templates" | "Videos" => {
                for file in fs::read_dir(file.path()).unwrap() {
                    func(file.unwrap());
                }
            }

            _ => {
                func(file);
            }
        }
    }
}

/// Prints a single row info box with title on the left
/// and content on the right
pub fn print_info_box(title: &str, content: &str) {
    let mut hook_box = tabled::builder::Builder::default()
        .set_columns([title])
        .add_record([content])
        .to_owned()
        .build();
    hook_box
        .with(tabled::Rotate::Left)
        .with(tabled::Style::rounded().off_vertical());
    println!("{hook_box}");
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
            path if path == home_dotfiles || path == config_dotfiles => true,
            _ => false,
        });
    }

    #[test]
    fn to_home_path() {
        assert_eq!(
            // /home/$USER/.config/dotfiles/Configs/zsh/.zshrc
            super::to_home_path(
                dirs::config_dir()
                    .unwrap()
                    .join("dotfiles")
                    .join("Configs")
                    .join("zsh")
                    .join(".zshrc")
                    .to_str()
                    .unwrap()
            ),
            // /home/$USER/.zshrc
            dirs::home_dir().unwrap().join(".zshrc")
        );

        let config_path = if cfg!(target_os = "windows") {
            dirs::config_dir()
                .unwrap()
                .join("dotfiles")
                .join("Configs")
                .join("zsh")
                .join("AppData")
                .join("Roaming")
                .join("group")
        } else {
            dirs::config_dir()
                .unwrap()
                .join("dotfiles")
                .join("Configs")
                .join("zsh")
                .join(".config")
                .join("group")
        };

        assert_eq!(
            // /home/$USER/.config/dotfiles/Configs/zsh/.config/$group
            super::to_home_path(config_path.to_str().unwrap()),
            // /home/$USER/.config/$group
            dirs::config_dir().unwrap().join("group")
        );
    }

    #[test]
    fn to_group_name() {
        assert_eq!(
            super::to_group_name(
                // /home/$USER/.config/dotfiles/Configs/zsh
                dirs::config_dir()
                    .unwrap()
                    .join("dotfiles")
                    .join("Configs")
                    .join("zsh")
                    .to_str()
                    .unwrap()
            )
            .unwrap(),
            "zsh"
        );
        assert_eq!(
            super::to_group_name(
                // /home/$USER/.config/dotfiles/Hooks/zsh
                dirs::config_dir()
                    .unwrap()
                    .join("dotfiles")
                    .join("Hooks")
                    .join("zsh")
                    .to_str()
                    .unwrap()
            )
            .unwrap(),
            "zsh"
        );
    }
}
