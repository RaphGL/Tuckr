//! Contains utilities to handle dotfiles

use crate::dotfiles;
use crate::fileops;
use owo_colors::OwoColorize;
use rust_i18n::t;
use std::env;
use std::path::{Path, PathBuf};
use std::{
    path::{self, Component},
    process,
};

// these targets come from the rust target_os and target_family cfg! values
pub const VALID_TARGETS: &[&str] = &[
    // special cases not available through cfg!
    "_wsl",
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

/// Returns the priority number for the group
/// A higher number means a higher priority
pub fn get_group_priority<T: AsRef<str>>(group: T) -> usize {
    let group = group.as_ref();
    let target = group.split('_').next_back().unwrap_or(group);

    // priority is in order of specificity
    // the more os specific target has higher priority
    match target {
        "unix" | "windows" => 1,
        "wsl" => 3,
        _ if !group_ends_with_target_name(group) => 0,
        _ if target.starts_with('#') => 4,
        _ => 2,
    }
}

/// Returns the index of the group with the highest priority in the `targets`
pub fn get_highest_priority_target_idx(targets: &[impl AsRef<str>]) -> Option<usize> {
    if targets.is_empty() {
        return None;
    }

    let mut highest_priority = 0;
    let mut highest_idx = 0;

    for (idx, target) in targets.iter().enumerate() {
        let target_priority = get_group_priority(target);

        if target_priority >= highest_priority {
            highest_priority = target_priority;
            highest_idx = idx;
        }
    }

    Some(highest_idx)
}

/// Exit codes
pub enum ReturnCode {
    /// Couldn't find the dotfiles directory
    CouldntFindDotfiles = 2,
    /// No Configs/Hooks/Secrets folder setup
    NoSetupFolder = 3,
    /// Referenced file does not exist in the current directory
    NoSuchFileOrDir = 4,
    /// Failed to encrypt referenced file
    EncryptionFailed = 5,
    /// Failed to decrypt referenced file
    DecryptionFailed = 6,
    /// Failed to read encrypted referenced file
    EncryptedReadFailed = 7,
}

impl From<ReturnCode> for process::ExitCode {
    fn from(value: ReturnCode) -> Self {
        Self::from(value as u8)
    }
}

pub fn get_dotfile_profile_from_path<T: AsRef<Path>>(file: T) -> Option<String> {
    let file: &Path = file.as_ref();
    let file = file.to_str().unwrap();

    const DIRNAME: &str = "dotfiles_";
    let dotfiles_start = file.find(DIRNAME)?;

    let file = &file[dotfiles_start + DIRNAME.len()..];
    let dotfiles_end = file.find(std::path::MAIN_SEPARATOR);

    Some(
        match dotfiles_end {
            Some(end) => &file[..end],
            None => file,
        }
        .into(),
    )
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Dotfile {
    pub path: path::PathBuf,
    pub group_path: path::PathBuf,
    pub group_name: String,
}

impl TryFrom<path::PathBuf> for Dotfile {
    type Error = String;

    /// Returns Ok if the path is pointing to a group within $TUCKR_HOME
    fn try_from(value: path::PathBuf) -> Result<Self, Self::Error> {
        let dotfiles_dir = get_dotfiles_path(get_dotfile_profile_from_path(&value))?;
        let configs_dir = dotfiles_dir.join("Configs");
        let hooks_dir = dotfiles_dir.join("Hooks");
        let secrets_dir = dotfiles_dir.join("Secrets");

        let dotfile_root_dir = if value.starts_with(&configs_dir) {
            configs_dir
        } else if value.starts_with(&hooks_dir) {
            hooks_dir
        } else if value.starts_with(&secrets_dir) {
            secrets_dir
        } else {
            return Err(t!("errors.path_is_not_dotfiles", path = value.display()).into_owned());
        };

        let group_path = if *value != dotfile_root_dir {
            let Component::Normal(group_relpath) = value
                .strip_prefix(&dotfile_root_dir)
                .unwrap()
                .components()
                .next()
                .unwrap()
            else {
                return Err(t!("errors.failed_to_get_group_relative_to_dotfiles_dir").into_owned());
            };

            dotfile_root_dir.join(group_relpath)
        } else {
            return Err(t!(
                "errors.path_is_not_dotfiles",
                path = dotfile_root_dir.display()
            )
            .into());
        };

        Ok(Dotfile {
            group_name: group_path.file_name().unwrap().to_str().unwrap().into(),
            path: value,
            group_path,
        })
    }
}

// returns true if group ends with a valid target platform suffix
pub fn group_ends_with_target_name(group: &str) -> bool {
    let Some(potential_target) = group.split('_').next_back() else {
        return false;
    };

    // a custom target would look like my_group_#target
    potential_target.starts_with('#') || VALID_TARGETS.iter().any(|target| group.ends_with(target))
}

pub fn group_without_target(group: &str) -> &str {
    for target in VALID_TARGETS {
        if let Some(base_group) = group.strip_suffix(target) {
            return base_group;
        }
    }

    group
}

fn platform_is_under_wsl() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }

    for file in &["/proc/sys/kernel/osrelease", "/proc/version"] {
        if let Ok(content) = std::fs::read(file) {
            let Ok(content) = std::str::from_utf8(&content) else {
                return false;
            };

            if content.contains("microsoft") || content.contains("WSL") {
                return true;
            }
        }
    }

    let Ok(wsl_interop_exists) = std::fs::exists("/proc/sys/fs/binfmt_misc/WSLInterop") else {
        return false;
    };

    wsl_interop_exists
}

/// Returns true if a group with specified name can be used by current platform.
/// Checks if a group should be linked on current platform. For unconditional
/// groups, this function returns true; for conditional groups, this function
/// returns true when group suffix matches current target_os or target_family.
pub fn group_is_valid_target(group: &str, custom_targets: &[impl AsRef<str>]) -> bool {
    // Gets the current OS and OS family
    let current_target_os = format!("_{}", env::consts::OS);
    let current_target_family = format!("_{}", env::consts::FAMILY);

    // returns true if a group has no suffix or its suffix matches the current OS
    if group_ends_with_target_name(group) {
        let target = group
            .split('_')
            .next_back()
            .expect("a group with target name should always have a `_`");

        // custom target syntax: group_#target
        if let Some(target) = target.strip_prefix(target)
            && custom_targets.iter().any(|t| t.as_ref() == target)
        {
            return true;
        }

        group.ends_with(&current_target_os)
            || group.ends_with(&current_target_family)
            || (group.ends_with("_wsl") && platform_is_under_wsl())
    } else {
        true
    }
}

impl Dotfile {
    /// Returns true if the target can be used by the current platform
    pub fn is_valid_target(&self, custom_targets: &[impl AsRef<str>]) -> bool {
        group_is_valid_target(self.group_name.as_str(), custom_targets)
    }

    /// Checks whether the current groups is needs root privileges to modify the target path
    // TODO: check if the directory has permission for the user, if so, this just returns false
    // as root access would not be needed to be able to deploy to it
    pub fn needs_root_privilege(&self) -> bool {
        let Ok(target) = self.to_target_path() else {
            return false;
        };

        let home_dir = dirs::home_dir().unwrap();
        !target.starts_with(home_dir)
    }

    /// Converts a path string from dotfiles/Configs to where they should be
    /// deployed on $TUCKR_TARGET
    pub fn to_target_path(&self) -> Result<PathBuf, String> {
        let dotfiles_configs_path =
            get_dotfiles_path(get_dotfile_profile_from_path(&self.path))?.join("Configs");

        let group_path = {
            let dotfile_path = self.path.strip_prefix(dotfiles_configs_path).unwrap();
            let mut dotfile_path_components = dotfile_path.components();

            match dotfile_path_components.next() {
                Some(_) => dotfile_path_components.as_path(),
                None => dotfile_path,
            }
        };

        // If we have a path component that starts with `^` it means that it will start from
        // the root, if it starts with `%` then it is expanded from an environment variable.
        // The last occurrence of one of these prefix is the one that is used.
        let mut target_path = PathBuf::new();
        let mut group_path_components = group_path.components();
        while let Some(comp) = group_path_components.next() {
            let comp = comp.as_os_str().to_str().unwrap();

            if comp.len() < 2 {
                continue;
            }

            let prefix = comp.chars().next().unwrap();
            let comp = &comp[1..];

            if prefix == '%' {
                let Ok(comp) = env::var(comp) else {
                    return Err(format!(
                        "Failed to read environment variable `{comp}` defined at `{}`",
                        self.path.display()
                    ));
                };
                target_path = PathBuf::from(comp).join(group_path_components.as_path());
            }

            if prefix == '^' {
                target_path = PathBuf::from(path::MAIN_SEPARATOR_STR)
                    .join(comp)
                    .join(group_path_components.as_path());
            }
        }

        if target_path.components().count() == 0 {
            target_path = get_dotfiles_target_dir_path()?.join(group_path);
        }

        Ok(target_path)
    }

    /// Creates an iterator that walks the directory
    /// Returns none if the Dotfile is not a directory, since it would not be walkable
    pub fn try_iter(&self) -> Result<DotfileIter, String> {
        if !self.path.is_dir() {
            Err(t!("errors.not_a_dir", directory = self.path.display()).into_owned())
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
    let dotfiles_dir = match profile {
        Some(ref profile) => format!("dotfiles_{profile}"),
        None => "dotfiles".into(),
    };

    if let Ok(dir) = std::env::var("TUCKR_HOME")
        && !dir.is_empty()
    {
        return Ok(PathBuf::from(dir).join(dotfiles_dir));
    }

    let (home_dotfiles, config_dotfiles) = {
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
            "{}\n{}",
            t!("errors.couldnt_find_dotfiles_dir").yellow(),
            t!(
                "errors.make_sure_dir_exists_or_run",
                dir = config_dotfiles.display(),
                cmd = init_cmd
            )
        ))
    }
}

/// removes the $HOME from path
pub fn get_target_basepath(target: &path::Path) -> Option<PathBuf> {
    let target_dir = get_dotfiles_target_dir_path().ok()?;
    Some(target.strip_prefix(target_dir).ok()?.into())
}

pub fn get_dotfiles_target_dir_path() -> Result<PathBuf, String> {
    #[cfg(test)]
    {
        unsafe { std::env::remove_var("TUCKR_TARGET") };
    }

    if let Ok(dir) = std::env::var("TUCKR_TARGET")
        && !dir.is_empty()
    {
        return Ok(dir.into());
    }

    dirs::home_dir().ok_or("No destination directory was found.".into())
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
pub fn get_nonexistent_groups(
    profile: Option<String>,
    dtype: DotfileType,
    groups: &[impl AsRef<str>],
) -> Option<Vec<String>> {
    let mut nonexistent_groups = Vec::new();
    for group in groups {
        let group = group.as_ref();
        if !dotfiles::dotfile_contains(profile.clone(), dtype, group) && group != "*" {
            nonexistent_groups.push(group.into());
        }
    }

    if nonexistent_groups.is_empty() {
        return None;
    }

    Some(nonexistent_groups)
}

/// Returns true if the group's name is valid on all platforms
///
/// For more information check: https://stackoverflow.com/questions/1976007/what-characters-are-forbidden-in-windows-and-linux-directory-names
/// This avoids further headaches for the end user and also allows tuckr to be able to detect invalid groups instead of just panicking
pub fn is_valid_groupname(group: impl AsRef<str>) -> Result<(), String> {
    let group = group.as_ref();

    let last_char = group.chars().next_back().unwrap();
    if group.len() > 1 && (last_char.is_whitespace() || last_char == '.') {
        return Err(t!(
            "errors.group_contains_invalid_char",
            group = group,
            char = last_char
        )
        .into_owned());
    }

    for char in group.chars() {
        if matches!(
            char,
            '/' | '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*' | '\0'
        ) {
            return Err(t!(
                "errors.group_contains_invalid_char",
                group = group,
                char = char
            )
            .into_owned());
        }

        if char.is_control() {
            return Err(t!("errors.group_contains_control_chars", group = group).into_owned());
        }
    }

    match group {
        // Windows invalid file names
        "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6"
        | "COM7" | "COM8" | "COM9" | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6"
        | "LPT7" | "LPT8" | "LPT9" => {
            Err(t!("errors.group_name_is_invalid", group = group).into_owned())
        }

        // Unix invalid file names
        "." | ".." => Err(t!("errors.group_name_is_invalid", group = group).into_owned()),

        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::dotfiles::{Dotfile, get_dotfiles_path};

    #[test]
    fn dotfile_to_target_path() {
        let group = get_dotfiles_path(None)
            .unwrap()
            .join("Configs")
            .join("zsh")
            .join(".zshrc");

        assert_eq!(
            Dotfile::try_from(group).unwrap().to_target_path().unwrap(),
            dirs::home_dir().unwrap().join(".zshrc")
        );
    }

    #[test]
    fn dotfile_targets_root() {
        let dotfiles_dir = super::get_dotfiles_path(None).unwrap().join("Configs");

        let root_dotfile =
            super::Dotfile::try_from(dotfiles_dir.join("Root").join("^bin")).unwrap();
        assert!(root_dotfile.needs_root_privilege());

        let nonroot_dotfile = super::Dotfile::try_from(dotfiles_dir.join("Zsh")).unwrap();
        assert!(!nonroot_dotfile.needs_root_privilege());
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
            (new_group("custom_target_group_#set_target"), true),
            (new_group("custom_target_group_#unset_target"), true),
        ];

        for (dotfile, expected) in target_tests {
            assert_eq!(dotfile.is_valid_target(&["set_target"]), expected);
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
        let work_profile_dir = dirs::config_dir().unwrap().join("dotfiles_work");
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

    #[test]
    fn group_priority() {
        let groups = [
            ("first_group_windows", 1),
            ("second_linux", 2),
            ("anotherone_here_macos", 2),
            ("another_unix", 1),
            ("priority_is_zero", 0),
            ("no_priority", 0),
        ];

        for (group, expected_priority) in groups {
            assert_eq!(super::get_group_priority(group), expected_priority);
        }
    }
}
