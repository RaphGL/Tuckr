use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::Context;
use crate::dotfiles::{self, ReturnCode};
use crate::symlinks;
use owo_colors::OwoColorize;
use rust_i18n::t;
use std::io::Write;
use std::process::ExitCode;
use std::{fs, path};
use tabled::{Table, Tabled};

fn is_ignored_file(file: impl AsRef<Path>) -> bool {
    let file = file.as_ref().file_name().unwrap().to_str().unwrap();

    fn is_ignored_file(ignored_files: &[&str], file: &str) -> bool {
        ignored_files.contains(&file)
    }

    fn is_ignored_extension(ignored_extensions: &[&str], file: &str) -> bool {
        ignored_extensions.iter().any(|e| file.ends_with(e))
    }

    fn is_ignored_prefix(ignored_prefixes: &[&str], file: &str) -> bool {
        ignored_prefixes.iter().any(|e| file.starts_with(e))
    }

    if cfg!(target_os = "macos") {
        static IGNORED_FILES_MACOS: &[&str] = &[
            // General
            ".DS_Store",
            ".AppleDouble",
            ".LSOverride",
            // Custom icons
            "Icon",
            // Root volume
            ".DocumentRevisions-V100",
            ".fseventsd",
            ".Spotlight-V100",
            ".TemporaryItems",
            ".Trashes",
            ".VolumeIcon.icns",
            ".com.apple.timemachine.donotpresent",
            // Files potentially created on remote AFP share
            ".AppleDB",
            ".AppleDesktop",
            "Network Trash Folder",
            "Temporary Items",
            ".apdisk",
        ];

        return is_ignored_file(IGNORED_FILES_MACOS, file);
    }

    if cfg!(target_family = "windows") {
        static IGNORED_FILES_WINDOWS: &[&str] = &[
            // Windows thumbnail cache files
            "Thumbs.db",
            "Thumbs.db:encryptable",
            "ehthumbs.db",
            "ehthumbs_vista.db",
            // Recycle bin used on file shares
            "$RECYCLE.BIN",
        ];

        static IGNORED_EXTENSIONS_WINDOWS: &[&str] = &[
            // Dump file
            ".stackdump",
        ];

        return is_ignored_file(IGNORED_FILES_WINDOWS, file)
            || is_ignored_extension(IGNORED_EXTENSIONS_WINDOWS, file);
    }

    if cfg!(target_family = "unix") {
        static IGNORED_FILES_UNIX: &[&str] = &[
            // KDE directory preferences
            ".directory",
        ];

        static IGNORED_PREFIXES_UNIX: &[&str] = &[
            // Temporary files which can be created if a process still has a handle open of a deleted file
            ".fuse_hidden",
            // Linux trash folder which might appear on any partition or disk
            ".Trash-",
            // .nfs files are created when an open file is removed but is still being accessed
            ".nfs",
        ];

        static IGNORED_EXTENSIONS_UNIX: &[&str] = &["~"];

        return is_ignored_file(IGNORED_FILES_UNIX, file)
            || is_ignored_prefix(IGNORED_PREFIXES_UNIX, file)
            || is_ignored_extension(IGNORED_EXTENSIONS_UNIX, file);
    }

    false
}

pub struct DirWalk {
    queue: Vec<path::PathBuf>,
}

impl DirWalk {
    pub fn new(dir_path: impl AsRef<Path>) -> Self {
        let dir_path = dir_path.as_ref();
        let dir = fs::read_dir(dir_path).unwrap_or_else(|_| {
            panic!(
                "{}",
                t!("errors.x_doesnt_exist", x = dir_path.to_str().unwrap())
            )
        });

        Self {
            queue: dir.map(|f| f.unwrap().path()).collect(),
        }
    }
}

impl Iterator for DirWalk {
    type Item = path::PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        let curr_file = self.queue.pop()?;

        if is_ignored_file(&curr_file) {
            return self.next();
        }

        if curr_file.is_dir() {
            for file in fs::read_dir(&curr_file).unwrap() {
                let file = file.unwrap();
                self.queue.push(file.path());
            }
        }

        Some(curr_file)
    }
}

/// Creates the necessary files and folders for a tuckr directory if they don't exist
pub fn init_cmd(ctx: &Context) -> Result<(), ExitCode> {
    let dotfiles_dir = dotfiles::get_dotfiles_path(if cfg!(test) {
        None
    } else {
        ctx.profile.clone()
    })
    .map_err(|err| {
        eprintln!("{err}");
        ExitCode::from(ReturnCode::CouldntFindDotfiles)
    })?;

    for dir in [
        dotfiles_dir.join("Configs"),
        dotfiles_dir.join("Hooks"),
        dotfiles_dir.join("Secrets"),
    ] {
        if ctx.dry_run {
            eprintln!(
                "{}",
                t!("dry-run.creating_dir", dir = dir.display()).green(),
            )
        } else if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("{}", e.red());
            return Err(ExitCode::FAILURE);
        }
    }

    println!(
        "{}",
        t!(
            "info.dotfiles_created_at",
            location = dotfiles_dir.to_str().unwrap()
        )
        .green()
    );

    Ok(())
}

/// Push the given files into the given group, essentially adopting them into
/// the dotfiles.
pub fn push_cmd(
    ctx: &Context,
    group: String,
    files: &[String],
    add: bool,
    only_files: bool,
    assume_yes: bool,
) -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path(ctx.profile.clone()) {
        Ok(dir) => dir.join("Configs").join(&group),
        Err(e) => {
            eprintln!("{e}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    let mut any_file_failed = false;
    let mut created_dirs = HashSet::new();

    for file in files {
        let file = PathBuf::from(file);
        if !file.exists() {
            eprintln!("{}", t!("errors.x_doesnt_exist", x = file.display()).red());
            any_file_failed = true;
            continue;
        }

        let file = path::absolute(file).unwrap();
        let target_file = dotfiles_dir.join(dotfiles::get_target_basepath(&file).unwrap());

        if target_file.exists() && !assume_yes {
            let confirmation_msg = format!(
                "`{}` {}.",
                target_file.to_str().unwrap(),
                t!("errors.already_exists"),
            );
            if !get_user_confirmation(&confirmation_msg) {
                continue;
            }
        }

        #[inline]
        fn push_file(
            dry_run: bool,
            target_parent_dir: &Path,
            target_file: &Path,
            file: &Path,
            created_dirs: &mut HashSet<PathBuf>,
        ) {
            if dry_run {
                if !target_parent_dir.exists() && !created_dirs.contains(target_parent_dir) {
                    eprintln!(
                        "{} parent directory `{}`",
                        "creating".yellow(),
                        target_parent_dir.display()
                    );
                    created_dirs.insert(target_parent_dir.to_path_buf());
                }
                eprintln!(
                    "{}",
                    t!(
                        "dry-run.copying_x_to_y",
                        x = file.display(),
                        y = target_file.display()
                    )
                    .green()
                );
            } else {
                if !target_parent_dir.exists() {
                    fs::create_dir_all(target_parent_dir).unwrap();
                }
                fs::copy(file, target_file).unwrap();
            }
        }

        if file.is_file() {
            let target_dir = target_file.parent().unwrap();
            push_file(
                ctx.dry_run,
                target_dir,
                &target_file,
                &file,
                &mut created_dirs,
            );
            continue;
        }

        for f in DirWalk::new(file) {
            if f.is_dir() {
                continue;
            }

            if !f.exists() {
                eprintln!("{}", t!("errors.x_doesnt_exist", x = f.display()).red());
                any_file_failed = true;
                continue;
            }

            let file = path::absolute(f).unwrap();

            let target_file = dotfiles_dir.join(dotfiles::get_target_basepath(&file).unwrap());
            let target_parent_file = target_file.parent().unwrap();

            push_file(
                ctx.dry_run,
                target_parent_file,
                target_file.as_path(),
                file.as_path(),
                &mut created_dirs,
            );
        }
    }

    if any_file_failed {
        return Err(ReturnCode::NoSuchFileOrDir.into());
    }

    if add {
        return symlinks::add_cmd(ctx, only_files, &[group], &[], false, false, assume_yes);
    }

    Ok(())
}

/// Remove the given group from the dotfiles, and clean up any symlinks that
/// point to that group.
pub fn pop_cmd(ctx: &Context, groups: &[String], assume_yes: bool) -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path(ctx.profile.clone()) {
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
            eprintln!("{}", t!("errors.x_doesnt_exist", x = group).red());
        }

        return Err(ReturnCode::NoSuchFileOrDir.into());
    }

    if !assume_yes {
        println!("{}:", t!("info.groups_will_be_removed"));
        for group in groups {
            println!("\t{}", group.yellow());
        }

        if !get_user_confirmation(t!("warn.want_to_proceed").to_string().as_str()) {
            return Err(ExitCode::FAILURE);
        }
    }

    for group_path in valid_groups {
        if ctx.dry_run {
            eprintln!(
                "{}",
                t!("dry-run.removing_x", x = group_path.display()).red()
            );
            continue;
        }

        let dotfile = dotfiles::Dotfile::try_from(group_path.clone()).unwrap();
        symlinks::remove_cmd(ctx, &[dotfile.group_name], &[])?;
        fs::remove_dir_all(&group_path).unwrap();
    }

    Ok(())
}

pub fn ls_hooks_cmd(ctx: &Context) -> Result<(), ExitCode> {
    let dir = match dotfiles::get_dotfiles_path(ctx.profile.clone()) {
        Ok(dir) => dir.join("Hooks"),
        Err(err) => {
            eprintln!("{err}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    if !dir.exists() {
        eprintln!("{}", t!("errors.no_dir_setup_for_x", x = "Hooks").red());
        return Err(ReturnCode::NoSetupFolder.into());
    }

    if !dir.is_dir() {
        eprintln!(
            "{}",
            t!("errors.not_a_dir", directory = dir.display()).red()
        );
        return Err(ReturnCode::NoSetupFolder.into());
    }

    #[derive(Tabled)]
    struct ListRow<'a> {
        #[tabled(rename = "Group")]
        group: String,
        #[tabled(rename = "Prehook")]
        pre_hook: &'a str,
        #[tabled(rename = "Posthook")]
        post_hook: &'a str,
        #[tabled(rename = "Remove")]
        rm_hook: &'a str,
    }

    let dir = fs::read_dir(dir).unwrap();
    let mut rows = Vec::new();

    let true_symbol = "✓".green().to_string();
    let false_symbol = "✗".red().to_string();

    for hook in dir {
        let hook_dir = hook.unwrap().path();

        if !hook_dir.is_dir() {
            continue;
        }

        let hook_name = hook_dir.file_name();
        let group = hook_name.unwrap().to_str().unwrap();

        let mut hook_entry = ListRow {
            group: group.to_owned(),
            pre_hook: &false_symbol,
            post_hook: &false_symbol,
            rm_hook: &false_symbol,
        };

        for hook in fs::read_dir(hook_dir).unwrap() {
            let hook = hook.unwrap().file_name();
            let hook = hook.to_str().unwrap();
            if hook.starts_with("pre") {
                hook_entry.pre_hook = &true_symbol;
            } else if hook.starts_with("post") {
                hook_entry.post_hook = &true_symbol;
            } else if hook.starts_with("rm") {
                hook_entry.rm_hook = &true_symbol;
            }
        }

        rows.push(hook_entry);
    }

    if rows.is_empty() {
        println!("{}", t!("errors.no_x_setup_yet", x = "hooks").yellow());
        return Ok(());
    }

    let mut hooks_list = Table::new(rows);
    hooks_list.with(tabled::settings::Alignment::center());
    hooks_list.with(tabled::settings::Style::rounded());
    hooks_list.with(tabled::settings::Margin::new(4, 4, 1, 1));

    println!("{hooks_list}");

    Ok(())
}

pub fn ls_secrets_cmd(ctx: &Context) -> Result<(), ExitCode> {
    let secrets_dir = dotfiles::get_dotfiles_path(ctx.profile.clone())
        .unwrap()
        .join("Secrets");

    let Ok(secrets) = secrets_dir.read_dir() else {
        eprintln!("{}", t!("errors.no_dir_setup_for_x", x = "Secrets").red());
        return Err(ReturnCode::NoSetupFolder.into());
    };

    let secrets: Vec<_> = secrets.collect();

    if secrets.is_empty() {
        eprintln!("{}", t!("errors.no_x_setup_yet", x = "secrets").yellow());
        return Err(ExitCode::FAILURE);
    }

    println!("{}:", t!("info.x_available", x = "Secrets"));
    for secret in secrets {
        let secret = secret.unwrap();
        println!("\t{}", secret.file_name().to_str().unwrap());
    }

    Ok(())
}

pub fn ls_profiles_cmd() -> Result<(), ExitCode> {
    let home_dir = dirs::home_dir().unwrap();
    let config_dir = dirs::config_dir().unwrap();
    let custom_dotfiles_dir = std::env::var("TUCKR_HOME");

    let profiles = {
        let mut available_profiles = HashSet::new();

        let dirs = match custom_dotfiles_dir {
            Ok(target) => &[PathBuf::from(target)],
            Err(_) => &[home_dir, config_dir][..],
        };

        for dir in dirs {
            for file in dir.read_dir().unwrap() {
                let file = file.unwrap();

                let Some(profile) = dotfiles::get_dotfile_profile_from_path(file.path()) else {
                    continue;
                };

                available_profiles.insert(profile);
            }
        }

        available_profiles
    };

    if profiles.is_empty() {
        println!("{}", t!("errors.no_x_setup_yet", x = "profiles").yellow());
        return Ok(());
    }

    println!("{}:", t!("info.x_available", x = "Profiles"));
    for profile in profiles {
        println!("\t{profile}");
    }

    Ok(())
}

pub fn groupis_cmd(ctx: &Context, files: &[String]) -> Result<(), ExitCode> {
    let dotfiles_dir = match dotfiles::get_dotfiles_path(ctx.profile.clone()) {
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
        let mut file_path = match PathBuf::from(file).canonicalize() {
            Ok(fp) => fp,
            Err(err) => {
                eprintln!("{err}");
                continue;
            }
        };

        if !file_path.exists() {
            eprintln!("{}", t!("errors.x_doesnt_exist", x = file).red());
            continue;
        }

        if let Ok(dotfile) = dotfiles::Dotfile::try_from(file_path.clone()) {
            println!("{}", dotfile.group_name);
            continue;
        }

        while !file_path.is_symlink() {
            // continuously go up a directory trying to find where the symlink is
            if !file_path.pop() {
                eprintln!("{}", t!("errors.not_a_tuckr_dotfile", file = file).red());
                continue 'next_file;
            }
        }

        let basepath = dotfiles::get_target_basepath(&file_path).unwrap();

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

pub fn get_user_confirmation(msg: &str) -> bool {
    let mut answer = String::new();
    print!("{msg} (y/N) ");
    _ = std::io::stdout().flush();
    std::io::stdin().read_line(&mut answer).unwrap();
    let answer = answer.trim().to_lowercase();

    let yes = t!("confirmation.yes").to_string().to_lowercase();

    let Some(yes_first_char) = yes.chars().next() else {
        return false;
    };

    if answer == yes {
        true
    } else if answer.len() == 1 {
        let Some(answer_char) = answer.chars().next() else {
            return false;
        };
        yes_first_char == answer_char
    } else {
        false
    }
}

// TODO: translate messages
// TODO: add colors to the dry run messages
pub fn from_stow_cmd(ctx: &Context, stow_path: Option<String>) -> Result<(), ExitCode> {
    println!("By running this command a copy of the stow dotfiles repo is created and converted to tuckr.
        This operation is non-destructive, if you have a previous config it will be kept with a `_old` suffix.

        Before continuing, here's the differences between Stow and Tuckr you ought to know:
            - Stow is unopinionated, Tuckr expects your dotfiles to have a certain file structure
            - `--dotfiles` is not supported, but this command will properly convert `dot-`
            - Tuckr validates that dotfiles are properly symlinked, so use groups to be able to get the most out of it

        Also note that this command only be so smart, so you will likely have to create the groups yourself to be able
        to start using tuckr unless you already were using stow with different programs' dotfiles in different directories.

        To check more in depth what Tuckr can do and how to use it, please check the wiki: https://github.com/RaphGL/Tuckr/wiki
    ");

    let used_dot_prefix = get_user_confirmation("Did you use `--dotfiles` with Stow?");

    if !get_user_confirmation(t!("warn.want_to_proceed").into_owned().as_ref()) {
        return Err(ExitCode::FAILURE);
    }

    let temp_profile = Some(match ctx.profile.as_ref() {
        Some(profile) => format!("{profile}_incomplete_conversion"),
        None => "incomplete_conversion".into(),
    });

    init_cmd(ctx)?;

    let stow_path = match stow_path {
        Some(path) => PathBuf::from(path).canonicalize().unwrap(),
        None => std::env::current_dir().unwrap(),
    };

    let dotfiles_dir = match dotfiles::get_dotfiles_path(ctx.profile.clone()) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("{err}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    // we want to preserve the user's original configs just in case we screw up the conversion
    // or they want to go back and they didn't have version control enabled
    // so we work on a new copy and then swap to the new one if it's converted successfully
    let new_dotfiles_dir = match dotfiles::get_dotfiles_path(temp_profile.clone()) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("{err}");
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }
    };

    let new_configs_dir = new_dotfiles_dir.join("Configs");

    for file in DirWalk::new(&stow_path) {
        if ctx.dry_run && file.as_os_str().to_str().unwrap().contains("dot-") {
            eprintln!("Converting `dot-` to `.` in file path `{}`", file.display());
        }

        let final_stem = file
            .strip_prefix(&stow_path)
            .unwrap()
            .components()
            .map(|component| {
                let component = component.as_os_str().to_str().unwrap();
                if used_dot_prefix && component.starts_with("dot-") {
                    component.replacen("dot-", ".", 1)
                } else {
                    component.into()
                }
            })
            .collect::<PathBuf>();

        let tuckr_path = new_configs_dir.join(final_stem);

        if file.is_dir() {
            if ctx.dry_run {
                eprintln!("Creating directory `{}`", tuckr_path.display());
            }
            fs::create_dir_all(tuckr_path).unwrap();
            continue;
        }

        if ctx.dry_run {
            eprintln!(
                "Copying file `{}` to `{}`",
                file.display(),
                tuckr_path.display()
            );
        } else {
            fs::copy(file, tuckr_path).unwrap();
        }
    }

    if dotfiles_dir.exists() {
        let old_dirname = dotfiles_dir.file_name().unwrap().to_str().unwrap();
        let mut old_dotfiles = dotfiles_dir.clone();
        old_dotfiles.set_file_name(format!("{old_dirname}_old"));

        if ctx.dry_run {
            eprintln!(
                "Moving previous dofiles (`{}`) to `{}`",
                dotfiles_dir.display(),
                old_dotfiles.display()
            );
        } else {
            fs::rename(&dotfiles_dir, old_dotfiles).unwrap();
        }

        if ctx.dry_run {
            eprintln!("Moving converted dotfiles to `{}`", dotfiles_dir.display());
        } else {
            fs::rename(new_dotfiles_dir, dotfiles_dir).unwrap();
        }
    }

    println!("{}", "\nYour dotfiles have been converted to Tuckr".green());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[must_use = "must be used before every test is conducted"]
    struct FileopsTest {
        dotfiles_dir: PathBuf,
        target_dir: PathBuf,
    }

    impl FileopsTest {
        fn start() -> Self {
            let dotfiles_dir = dotfiles::get_dotfiles_path(None).unwrap();
            fs::create_dir_all(dotfiles_dir.join("Configs")).unwrap();

            let target_dir = dirs::home_dir().unwrap().join(format!(
                "tuckr_test-{}",
                std::thread::current().name().unwrap()
            ));
            fs::create_dir_all(&target_dir).unwrap();

            Self {
                dotfiles_dir,
                target_dir,
            }
        }
    }

    impl Drop for FileopsTest {
        fn drop(&mut self) {
            if self.dotfiles_dir.exists() {
                fs::remove_dir_all(&self.dotfiles_dir).unwrap();
            }

            if self.target_dir.exists() {
                fs::remove_dir_all(&self.target_dir).unwrap();
            }
        }
    }

    #[test]
    fn push_files() {
        let ft = FileopsTest::start();

        let file_path = ft.target_dir.join("test");
        let mut file = fs::File::create(&file_path).unwrap();
        file.write("this is a test".as_bytes()).unwrap();

        let pushed_file = ft
            .dotfiles_dir
            .join("Configs")
            .join("test")
            .join(dotfiles::get_target_basepath(&file_path).unwrap());

        assert!(!pushed_file.exists());

        super::push_cmd(
            &Context::default(),
            "test".into(),
            &[file_path.to_str().unwrap().to_string()],
            false,
            false,
            true,
        )
        .unwrap();

        let first_pushed_content = fs::read_to_string(&pushed_file).unwrap();
        assert!(
            pushed_file.exists() && first_pushed_content == fs::read_to_string(&file_path).unwrap()
        );

        file.write("something something".as_bytes()).unwrap();
        super::push_cmd(
            &Context::default(),
            "test".into(),
            &[file_path.to_str().unwrap().to_string()],
            false,
            false,
            true,
        )
        .unwrap();

        assert!(
            pushed_file.exists()
                && first_pushed_content != fs::read_to_string(pushed_file).unwrap()
        );
    }

    #[test]
    fn push_directories() {
        let ft = FileopsTest::start();

        let dir1 = ft.target_dir.join("dir1");
        fs::create_dir_all(&dir1).unwrap();
        let mut file1 = fs::File::create(dir1.join("file")).unwrap();
        file1.write("test".as_bytes()).unwrap();

        let dir2 = ft.target_dir.join("dir2");
        fs::create_dir_all(&dir2).unwrap();
        let mut file2 = fs::File::create(dir2.join("file")).unwrap();
        file2.write("test".as_bytes()).unwrap();

        // never used because it is empty
        fs::create_dir_all(ft.target_dir.join("dir3")).unwrap();

        let group_dir = ft
            .dotfiles_dir
            .join("Configs")
            .join("test")
            .join(dotfiles::get_target_basepath(&ft.target_dir).unwrap());

        assert!(!group_dir.exists());

        super::push_cmd(
            &Context::default(),
            "test".into(),
            &[ft.target_dir.to_str().unwrap().to_owned()],
            false,
            false,
            true,
        )
        .unwrap();

        // pushing ignores empty directories so the count = 2 and not 3
        assert!(group_dir.exists() && fs::read_dir(group_dir).unwrap().count() == 2);
    }

    #[test]
    fn pop_groups() {
        let ft = FileopsTest::start();

        fs::create_dir_all(&ft.target_dir).unwrap();
        let mut file = fs::File::create(ft.target_dir.join("file")).unwrap();
        file.write("test".as_bytes()).unwrap();

        let group_dir = ft
            .dotfiles_dir
            .join("Configs")
            .join("test")
            .join(dotfiles::get_target_basepath(&ft.target_dir).unwrap());

        super::push_cmd(
            &Context::default(),
            "test".into(),
            &[ft.target_dir.to_str().unwrap().to_owned()],
            false,
            false,
            true,
        )
        .unwrap();

        assert!(group_dir.exists());
        super::pop_cmd(&Context::default(), &["test".into()], true).unwrap();
        assert!(!group_dir.exists());
    }

    #[test]
    fn ignore_garbage_files() {
        assert!(is_ignored_file("asdfadsfaf") == false);

        if cfg!(target_os = "macos") {
            assert!(is_ignored_file(".DS_Store"));
        }

        if cfg!(target_family = "windows") {
            assert!(is_ignored_file("Thumbs.db"));
            assert!(is_ignored_file("test.stackdump"));
        }

        if cfg!(target_family = "unix") {
            assert!(is_ignored_file(".Trash-tuckr"));
            assert!(is_ignored_file("tuckr-backup~"));
        }
    }
}
