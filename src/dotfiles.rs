//! Contains utilities to handle dotfiles

use owo_colors::OwoColorize;

use crate::dotfiles;
use crate::fileops;
use std::env;
use std::path::PathBuf;
use std::{
    path::{self, Component},
    process,
};

pub const VALID_TARGETS: &[&str] = &[
    // default target_os values
    "_windows",
    "_macos",
    "_ios",
    "_linux",
    "_android",
    "_freebsd",
    "_dragonfly",
    "_openbsd",
    "_netbsd",
    "_none",
    // default target_family values
    "_unix",
    "_windows",
];

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

pub fn get_dotfile_profile_from_path<T: AsRef<path::Path>>(file: T) -> Option<String> {
    let file = file.as_ref();

    let dotfiles_path = {
        let home_path = dirs::home_dir().unwrap();
        let configs_path = dirs::config_dir().unwrap();

        if file.starts_with(&configs_path) {
            configs_path
        } else if file.starts_with(&home_path) {
            home_path
        } else {
            return None;
        }
    };

    let dotfiles_path = file.strip_prefix(dotfiles_path).unwrap();

    let dotfiles_dirname = dotfiles_path
        .components()
        .next()?
        .as_os_str()
        .to_str()
        .unwrap();

    let (dirname, profile_name) = dotfiles_dirname.split_once('_')?;

    if dirname != "dotfiles" {
        return None;
    }

    Some(profile_name.into())
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Dotfile {
    pub path: path::PathBuf,
    pub group_path: path::PathBuf,
    pub group_name: String,
}

impl TryFrom<path::PathBuf> for Dotfile {
    type Error = String;

    fn try_from(value: path::PathBuf) -> Result<Self, Self::Error> {
        /// returns the path for the group the file belongs to.
        /// an error is returns if the file does not belong to dotfiles
        pub fn to_group_path(file_path: &path::PathBuf) -> Result<path::PathBuf, String> {
            let dotfiles_dir = get_dotfiles_path(get_dotfile_profile_from_path(file_path))?;
            let configs_dir = dotfiles_dir.join("Configs");
            let hooks_dir = dotfiles_dir.join("Hooks");
            let secrets_dir = dotfiles_dir.join("Secrets");

            let dotfile_root_dir = if file_path.starts_with(&configs_dir) {
                configs_dir
            } else if file_path.starts_with(&hooks_dir) {
                hooks_dir
            } else if file_path.starts_with(&secrets_dir) {
                secrets_dir
            } else {
                return Err("path does not belong to dotfiles.".into());
            };

            let group = if *file_path == dotfile_root_dir {
                Ok(dotfile_root_dir)
            } else {
                let Component::Normal(group_relpath) = file_path
                    .strip_prefix(&dotfile_root_dir)
                    .unwrap()
                    .components()
                    .next()
                    .unwrap()
                else {
                    return Err("failed to get group path relative to dotfile dir.".into());
                };

                Ok(dotfile_root_dir.join(group_relpath))
            };

            group
        }

        let group_path = to_group_path(&value)?;

        Ok(Dotfile {
            group_name: group_path.file_name().unwrap().to_str().unwrap().into(),
            path: value,
            group_path,
        })
    }
}

pub fn group_ends_with_target_name(group: &str) -> bool {
    VALID_TARGETS.iter().any(|target| group.ends_with(target))
}

pub fn group_without_target(group: &str) -> &str {
    for target in VALID_TARGETS {
        if let Some(base_group) = group.strip_suffix(target) {
            return base_group;
        }
    }

    group
}

/// Returns true if a group with specified name can be used by current platform.
/// Checks if a group should be linked on current platform. For unconditional
/// groups, this function returns true; for conditional groups, this function
/// returns true when group suffix matches current target_os or target_family.
pub fn group_is_valid_target(group: &str) -> bool {
    // Gets the current OS and OS family
    let current_target_os = format!("_{}", env::consts::OS);
    let current_target_family = format!("_{}", env::consts::FAMILY);

    // returns true if a group has no suffix or its suffix matches the current OS
    if group_ends_with_target_name(group) {
        group.ends_with(&current_target_os) || group.ends_with(&current_target_family)
    } else {
        true
    }
}

impl Dotfile {
    /// Returns true if the target can be used by the current platform
    pub fn is_valid_target(&self) -> bool {
        let group = self.group_name.as_str();
        group_is_valid_target(group)
    }

    /// Checks whether the current groups is targetting the root path aka `/`
    pub fn targets_root(&self) -> bool {
        let root_dir = get_dotfiles_path(get_dotfile_profile_from_path(&self.group_path))
            .unwrap()
            .join("Configs")
            .join("Root");
        self.group_path.starts_with(root_dir)
    }

    /// Converts a path string from dotfiles/Configs to where they should be
    /// deployed on $HOME
    pub fn to_target_path(&self) -> path::PathBuf {
        // uses join("") so that the path appends / or \ depending on platform
        let dotfiles_configs_path = get_dotfiles_path(get_dotfile_profile_from_path(&self.path))
            .unwrap()
            .join("Configs")
            .join("");
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

    /// Creates an iterator that walks the directory
    /// Returns none if the Dotfile is not a directory, since it would not be walkable
    pub fn try_iter(&self) -> Result<DotfileIter, String> {
        if !self.path.is_dir() {
            Err(format!("{} is not a directory", self.path.display()))
        } else {
            Ok(DotfileIter(fileops::DirWalk::new(self.path.clone())))
        }
    }
}

pub struct DotfileIter(fileops::DirWalk);

impl Iterator for DotfileIter {
    type Item = Dotfile;

    fn next(&mut self) -> Option<Self::Item> {
        let curr_file = self.0.next()?;
        let dotfile = Dotfile::try_from(curr_file).unwrap();
        Some(dotfile)
    }
}

/// Returns an Option<String> with the path to of the tuckr dotfiles directory
///
/// When run on a unit test it returns a temporary directory for testing purposes.
/// this testing directory is unique to the thread it's running on,
/// so different unit tests cannot interact with the other's dotfiles directory
pub fn get_dotfiles_path(profile: Option<String>) -> Result<path::PathBuf, String> {
    let (home_dotfiles, config_dotfiles) = {
        let dotfiles_dir = match profile {
            Some(ref profile) => format!("dotfiles_{profile}"),
            None => "dotfiles".into(),
        };

        let home_dotfiles = dirs::home_dir().unwrap();
        let config_dotfiles = dirs::config_dir().unwrap();

        (
            home_dotfiles.join(format!(".{dotfiles_dir}")),
            config_dotfiles.join(dotfiles_dir),
        )
    };

    if cfg!(test) {
        // using the thread's name is necessary for tests
        // since unit tests run in parallel, each test needs a uniquely identifying name.
        // cargo-test names each threads with the name of the unit test that is running on it.
        Ok(std::env::temp_dir()
            .join(format!("tuckr-{}", std::thread::current().name().unwrap()))
            .join("dotfiles"))
    } else if config_dotfiles.exists() {
        Ok(config_dotfiles)
    } else if home_dotfiles.exists() {
        Ok(home_dotfiles)
    } else {
        let init_cmd = match profile {
            Some(profile) => format!("tuckr -p {profile} init"),
            None => "tuckr init".into(),
        };
        Err(format!(
            "{}\n\n\
            Make sure a `{}` directory exists.\n\
            Or use `{init_cmd}`.",
            "Couldn't find dotfiles directory.".yellow(),
            config_dotfiles.display(),
        ))
    }
}

/// removes the $HOME from path
pub fn get_target_basepath(target: &path::Path) -> Option<PathBuf> {
    let home_dir = dirs::home_dir().unwrap();
    match target.strip_prefix(home_dir) {
        Ok(basepath) => Some(basepath.into()),
        Err(_) => None,
    }
}

#[derive(Copy, Clone)]
pub enum DotfileType {
    Configs,
    Secrets,
    Hooks,
}

/// Returns if a config has been setup for <group> on <dtype>
pub fn dotfile_contains(profile: Option<String>, dtype: DotfileType, group: &str) -> bool {
    let target_dir = match dtype {
        DotfileType::Configs => "Configs",
        DotfileType::Secrets => "Secrets",
        DotfileType::Hooks => "Hooks",
    };

    let Ok(dotfiles_dir) = get_dotfiles_path(profile) else {
        return false;
    };
    let group_src = dotfiles_dir.join(target_dir).join(group);
    group_src.exists()
}

/// Returns all groups in the slice that don't have a corresponding directory in dotfiles/{Configs,Hooks,Secrets}
pub fn check_invalid_groups(
    profile: Option<String>,
    dtype: DotfileType,
    groups: &[String],
) -> Option<Vec<String>> {
    let mut invalid_groups = Vec::new();
    for group in groups {
        if !dotfiles::dotfile_contains(profile.clone(), dtype, group) && group != "*" {
            invalid_groups.push(group.clone());
        }
    }

    if invalid_groups.is_empty() {
        return None;
    }

    Some(invalid_groups)
}

#[cfg(test)]
mod tests {
    use crate::dotfiles::{get_dotfiles_path, Dotfile};

    #[test]
    fn dotfile_to_target_path() {
        let group = get_dotfiles_path(None)
            .unwrap()
            .join("Configs")
            .join("zsh")
            .join(".zshrc");

        assert_eq!(
            Dotfile::try_from(group).unwrap().to_target_path(),
            dirs::home_dir().unwrap().join(".zshrc")
        );
    }

    #[test]
    fn dotfile_targets_root() {
        let dotfiles_dir = super::get_dotfiles_path(None).unwrap().join("Configs");

        let root_dotfile = super::Dotfile::try_from(dotfiles_dir.join("Root")).unwrap();
        assert!(root_dotfile.targets_root());

        let nonroot_dotfile = super::Dotfile::try_from(dotfiles_dir.join("Zsh")).unwrap();
        assert!(!nonroot_dotfile.targets_root());
    }

    #[test]
    fn detect_valid_targets() {
        fn new_group(name: &str) -> Dotfile {
            Dotfile {
                group_name: name.to_string(),
                path: Default::default(),
                group_path: Default::default(),
            }
        }

        let target_tests = [
            (
                new_group("group_windows"),
                std::env::consts::FAMILY == "windows",
            ),
            (new_group("group_linux"), std::env::consts::OS == "linux"),
            (new_group("group_unix"), std::env::consts::FAMILY == "unix"),
            (new_group("group_something"), true),
            (new_group("some_random_group"), true),
        ];

        for (dotfile, expected) in target_tests {
            assert_eq!(dotfile.is_valid_target(), expected);
        }
    }

    #[test]
    fn get_profile_name_from_dotfile_path() {
        let no_profile_dir = dirs::config_dir().unwrap();
        let invalid_dir = dirs::config_dir()
            .unwrap()
            .join("somethingelse_work")
            .join("Configs")
            .join("Vim");
        let work_profile_dir = dirs::config_dir()
            .unwrap()
            .join("dotfiles_work")
            .join("Configs")
            .join("Vim");
        let home_profile_dir = dirs::config_dir()
            .unwrap()
            .join("dotfiles_my_home")
            .join("Configs")
            .join("Somethign")
            .join("test.cfg");

        assert_eq!(
            super::get_dotfile_profile_from_path(work_profile_dir),
            Some("work".into())
        );
        assert_eq!(
            super::get_dotfile_profile_from_path(home_profile_dir),
            Some("my_home".into())
        );
        assert_eq!(super::get_dotfile_profile_from_path(no_profile_dir), None,);
        assert_eq!(super::get_dotfile_profile_from_path(invalid_dir), None,);
    }
}
