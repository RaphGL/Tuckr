//! Contains utilities to handle dotfiles

use crate::utils;
use std::env;
use std::fs;
use std::{
    path::{self, Component},
    process,
};

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

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Dotfile {
    pub path: path::PathBuf,
    pub group_path: path::PathBuf,
    pub group_name: String,
}

impl Dotfile {
    pub fn try_from(file_path: path::PathBuf) -> Option<Dotfile> {
        /// Extracts group name from tuckr directories
        pub fn to_group_path(group_path: &path::PathBuf) -> Option<path::PathBuf> {
            let dotfiles_dir = get_dotfiles_path().unwrap();
            let configs_dir = dotfiles_dir.join("Configs");
            let hooks_dir = dotfiles_dir.join("Hooks");
            let secrets_dir = dotfiles_dir.join("Secrets");

            let dir = if group_path.starts_with(&configs_dir) {
                configs_dir
            } else if group_path.starts_with(&hooks_dir) {
                hooks_dir
            } else if group_path.starts_with(&secrets_dir) {
                secrets_dir
            } else {
                return None;
            };

            let group = if *group_path == dir {
                Some(dir)
            } else {
                let Component::Normal(groupname) = group_path
                    .strip_prefix(&dir)
                    .unwrap()
                    .components()
                    .next()
                    .unwrap()
                else {
                    return None;
                };

                Some(dir.join(groupname))
            };

            group
        }

        let group_path = to_group_path(&file_path).unwrap();

        Some(Dotfile {
            group_name: group_path.file_name().unwrap().to_str().unwrap().into(),
            path: file_path,
            group_path,
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
        let root_dir = get_dotfiles_path().unwrap().join("Configs").join("Root");
        self.group_path.starts_with(root_dir)
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
    pub fn map<F>(&self, mut func: F)
    where
        F: FnMut(Dotfile),
    {
        let dir = match fs::read_dir(&self.path) {
            Ok(f) => f,
            Err(_) => panic!("{} does not exist", self.path.to_str().unwrap()),
        };

        let mut queue: Vec<path::PathBuf> = dir.map(|f| f.unwrap().path()).collect();

        while let Some(curr_file) = queue.pop() {
            func(Dotfile::try_from(curr_file.clone()).unwrap());

            if curr_file.is_dir() {
                for dir in fs::read_dir(curr_file).unwrap() {
                    let dir = dir.unwrap();
                    queue.push(dir.path());
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
        if !utils::dotfile_contains(dtype, group) && group != "*" {
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
    use crate::utils::Dotfile;

    #[test]
    fn get_dotfiles_path() {
        // /home/$USER/.dotfiles
        let home_dotfiles = dirs::home_dir().unwrap().join(".dotfiles");
        // /home/$USER/.config/dotfiles
        let config_dotfiles = dirs::config_dir().unwrap().join("dotfiles");

        let path_found = super::get_dotfiles_path().unwrap();
        assert!(path_found == home_dotfiles || path_found == config_dotfiles);
    }

    #[test]
    fn dotfile_to_target_path() {
        let group = dirs::config_dir()
            .unwrap()
            .join("dotfiles")
            .join("Configs")
            .join("zsh")
            .join(".zshrc");

        assert_eq!(
            // /home/$USER/.config/dotfiles/Configs/zsh/.zshrc
            Dotfile::try_from(group).unwrap().to_target_path(),
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
            Dotfile::try_from(config_path).unwrap().to_target_path(),
            // /home/$USER/.config/$group
            dirs::config_dir().unwrap().join("group")
        );
    }

    #[test]
    fn dotfile_targets_root() {
        let dotfiles_dir = super::get_dotfiles_path().unwrap().join("Configs");

        let root_dotfile = super::Dotfile::try_from(dotfiles_dir.join("Root")).unwrap();
        assert!(root_dotfile.targets_root());

        let nonroot_dotfile = super::Dotfile::try_from(dotfiles_dir.join("Zsh")).unwrap();
        assert!(!nonroot_dotfile.targets_root());
    }
}
