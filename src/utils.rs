//! A set of helper functions that reduce boilerplate

use crate::utils;
use std::env;
use std::fs;
use std::{path, process};

// Exit codes
/// Couldn't find the dotfiles directory
pub enum ReturnCode {
    CouldntFindDotfiles = 2,
    /// No Configs/Hooks/Secrets folder setup
    NoSetupFolder = 3,
    /// Referenced file does not exist in the current directory
    NoSuchFileOrDir = 4,
    /// Failed to encrypt referenced file
    EncryptionFailed = 5,
    /// Failed to decrypt referenced file
    DecryptionFailed = 6,
}

impl From<ReturnCode> for process::ExitCode {
    fn from(value: ReturnCode) -> Self {
        Self::from(value as u8)
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct DotfileGroup {
    pub path: path::PathBuf,
    pub name: String,
}

impl DotfileGroup {
    pub fn from(group_path: path::PathBuf) -> Option<DotfileGroup> {
        /// Extracts group name from tuckr directories
        pub fn to_group_name(group_path: &path::PathBuf) -> Option<String> {
            let path = group_path.to_str().unwrap();
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
            let path = path.split_once(config_path.to_str().unwrap())?.1;

            if let Some(path) = path.split_once(path::MAIN_SEPARATOR) {
                return Some(path.0.to_string());
            }

            Some(path.to_string())
        }

        let group_str = group_path.to_str().unwrap();
        Some(DotfileGroup {
            name: if !group_str.contains(path::MAIN_SEPARATOR) {
                group_str.to_string()
            } else {
                to_group_name(&group_path)?
            },
            path: group_path,
        })
    }

    /// Returns true if the target can be used by the current platform
    pub fn is_valid_target(&self) -> bool {
        // Gets the current OS and OS family
        let target_os = format!("_{}", env::consts::OS);
        let target_family = format!("_{}", env::consts::FAMILY);
        let group = self.path.file_name().unwrap().to_str().unwrap();

        // returns true if a group has no suffix or its suffix matches the current OS
        group.ends_with(&target_os) || group.ends_with(&target_family) || !group.contains('_')
    }

    /// Checks whether the current groups is targetting the root path aka `/`
    pub fn targets_root(&self) -> bool {
        let group = self.path.clone();
        let dotfiles_dir = get_dotfiles_path().unwrap().join("Configs").join("Root");
        let dotfiles_dir = dotfiles_dir.to_str().unwrap();

        if group.to_str().unwrap().contains(dotfiles_dir) {
            return true;
        }
        false
    }

    /// Converts a path string from dotfiles/Configs to where they should be
    /// deployed on $HOME
    pub fn to_target_path(&self) -> path::PathBuf {
        // uses join("") so that the path appends / or \ depending on platform
        let dotfiles_configs_path = get_dotfiles_path().unwrap().join("Configs").join("");
        let dotfiles_configs_path = dotfiles_configs_path.to_str().unwrap();
        let group_path = self.path.clone();
        let group_path = {
            let group_path = group_path.to_str().unwrap();
            let group_path = group_path.strip_prefix(dotfiles_configs_path).unwrap();
            match group_path.split_once(path::MAIN_SEPARATOR) {
                Some(path) => path.1,
                None => group_path,
            }
        };

        if self.targets_root() {
            path::PathBuf::from(path::MAIN_SEPARATOR_STR).join(group_path)
        } else {
            dirs::home_dir().unwrap().join(group_path)
        }
    }

    /// Goes through every file in Configs/<group_dir> and applies the function
    pub fn map<F: FnMut(path::PathBuf)>(&self, mut func: F) {
        let dir = self.path.clone();
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
                        func(file.unwrap().path());
                    }
                }

                _ => {
                    func(file.path());
                }
            }
        }
    }
}

/// Returns an Option<String> with the path to of the tuckr dotfiles directory
pub fn get_dotfiles_path() -> Result<path::PathBuf, String> {
    let home_dotfiles = dirs::home_dir().unwrap().join(".dotfiles");
    let config_dotfiles = dirs::config_dir().unwrap().join("dotfiles");

    if home_dotfiles.exists() {
        Ok(home_dotfiles)
    } else if config_dotfiles.exists() {
        Ok(config_dotfiles)
    } else if cfg!(test) {
        Ok(std::env::temp_dir()
            .join(format!("tuckr-{}", std::process::id()))
            .join("dotfiles"))
    } else {
        Err(format!("Couldn't find dotfiles directory.\n\
            Initialize your dotfiles with `tuckr init` or make sure the dotfiles are set on either {} or {}.", 
            config_dotfiles.to_str().unwrap(), home_dotfiles.to_str().unwrap()))
    }
}

#[derive(Copy, Clone)]
pub enum DotfileType {
    Configs,
    Secrets,
    Hooks,
}

/// Returns if a config has been setup for <group> on <dtype>
pub fn dotfile_contains(dtype: DotfileType, group: &str) -> bool {
    let target_dir = match dtype {
        DotfileType::Configs => "Configs",
        DotfileType::Secrets => "Secrets",
        DotfileType::Hooks => "Hooks",
    };

    let Ok(dotfiles_dir) = get_dotfiles_path() else {
        return false;
    };
    let group_src = dotfiles_dir.join(target_dir).join(group);
    group_src.exists()
}

pub fn check_invalid_groups(dtype: DotfileType, groups: &[String]) -> Option<Vec<String>> {
    let mut invalid_groups = Vec::new();
    for group in groups {
        if !utils::dotfile_contains(dtype, group) {
            invalid_groups.push(group.clone());
        }
    }

    if !invalid_groups.is_empty() {
        return Some(invalid_groups);
    }

    None
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
    use crate::utils::DotfileGroup;

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
    fn group_to_home_path() {
        let group = dirs::config_dir()
            .unwrap()
            .join("dotfiles")
            .join("Configs")
            .join("zsh")
            .join(".zshrc");

        assert_eq!(
            // /home/$USER/.config/dotfiles/Configs/zsh/.zshrc
            DotfileGroup::from(group).unwrap().to_target_path(),
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
            DotfileGroup::from(config_path).unwrap().to_target_path(),
            // /home/$USER/.config/$group
            dirs::config_dir().unwrap().join("group")
        );
    }
}
