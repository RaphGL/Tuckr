//! Manages dotfile symlinking
//!
//! Dotfiles are managed by the SymlinkHandler, its fields contain the following runtime information:
//! - dotfiles_dir: the location of the dotfiles directory
//! - symlinked: all files that have been symlinked
//! - not_symlinked: files that haven't been symlinked yet
//! - not_owned: files that have the same name on dotfiles/Configs but that do not belong to us,
//!   therefore they are in conflict
//!
//! This information is retrieved by walking through dotfiles/Configs and checking whether their
//! $TUCKR_TARGET equivalents are pointing to them and categorizing them accordingly.

use crate::Context;
use crate::dotfiles::{self, Dotfile, DotfileType, ReturnCode};
use crate::fileops;
use enumflags2::BitFlags;
use owo_colors::OwoColorize;
use rust_i18n::t;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use tabled::{Table, Tabled};

fn symlink_file(dry_run: bool, f: PathBuf) {
    match Dotfile::try_from(f.clone()) {
        Ok(group) => {
            let target_path = match group.to_target_path() {
                Ok(t) => t,
                Err(err) => {
                    eprintln!("{err}");
                    return;
                }
            };

            if target_path.exists() {
                if dry_run {
                    eprintln!(
                        "{}",
                        t!(
                            "dry-run.ignoring_x_already_exists",
                            x = target_path.display()
                        )
                        .yellow()
                    );
                }
                return;
            }

            if dry_run {
                eprintln!(
                    "{}",
                    t!(
                        "dry-run.symlinking_x_to_y",
                        x = f.display(),
                        y = target_path.display()
                    )
                );
                return;
            }

            let result = {
                #[cfg(target_family = "unix")]
                {
                    std::os::unix::fs::symlink(f, target_path)
                }

                #[cfg(target_family = "windows")]
                {
                    if f.is_dir() {
                        std::os::windows::fs::symlink_dir(f, target_path)
                    } else {
                        std::os::windows::fs::symlink_file(f, target_path)
                    }
                }
            };

            if let Err(err) = result {
                eprintln!(
                    "{}",
                    t!(
                        "errors.failed_to_symlink_x",
                        groupname = group.group_name,
                        err_msg = err.red()
                    )
                );
            }
        }

        Err(err) => {
            eprintln!("{err}");
            eprintln!(
                "{}",
                t!("errors.failed_to_link_file", file = f.to_str().unwrap())
            );
        }
    }
}

#[enumflags2::bitflags]
#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Debug)]
enum SymlinkType {
    Symlinked = 0b001,
    NotSymlinked = 0b010,
    NotOwned = 0b100,
}

type HashCache = HashMap<String, HashSet<Dotfile>>;

/// Handles dotfile symlinking and their current status
struct SymlinkHandler<'a> {
    ctx: &'a Context,
    dotfiles_dir: PathBuf,    // path to the dotfiles directory
    symlinked: HashCache,     // dotfiles that have been symlinked from Dotfiles/Configs
    not_symlinked: HashCache, // dotfiles that haven't been symlinked to $TUCKR_TARGET yet
    not_owned: HashCache, // dotfiles that are symlinks but points somewhere outside of their respective Dotfiles/Configs's group dir
}

impl<'a> SymlinkHandler<'a> {
    /// Initializes SymlinkHandler and fills it dotfiles' status information
    fn try_new(ctx: &'a Context) -> Result<Self, ExitCode> {
        let dotfiles_dir = match dotfiles::get_dotfiles_path(ctx.profile.clone()) {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("{e}");
                return Err(ReturnCode::CouldntFindDotfiles.into());
            }
        };

        if let Err(err) = dotfiles::get_dotfiles_target_dir_path() {
            eprintln!("{err}");
            return Err(ReturnCode::NoSuchFileOrDir.into());
        }

        let symlinker = SymlinkHandler {
            ctx,
            dotfiles_dir,
            symlinked: HashCache::new(),
            not_symlinked: HashCache::new(),
            not_owned: HashCache::new(),
        };

        // this fills the symlinker with dotfile status information
        symlinker.validate()
    }

    /// **This function should not be used outside this scope**
    ///
    /// Checks which dotfiles are or are not symlinked and registers their Configs/$group path
    /// into the struct
    ///
    /// Returns a copy of self with all the fields set accordingly
    fn validate(mut self) -> Result<Self, ExitCode> {
        let configs_dir = self.dotfiles_dir.join("Configs");

        if !configs_dir.exists() && !configs_dir.is_dir() {
            eprintln!(
                "{}",
                t!(
                    "errors.no_configs_dir_in_dotfiles",
                    dotfiles = configs_dir.display()
                )
            );
            return Err(ReturnCode::CouldntFindDotfiles.into());
        }

        let mut symlinked = HashCache::new();
        let mut not_symlinked = HashCache::new();
        let mut not_owned = HashCache::new();

        // iterates over every file inside dotfiles/Config and determines their symlink status

        for f in fileops::DirWalk::new(configs_dir) {
            // skip group directories otherwise it would try to link dotfiles/Configs/Groups to the users home
            let f = Dotfile::try_from(f).unwrap();
            if f.path == f.group_path {
                continue;
            }

            let target = match f.to_target_path() {
                Ok(target) => target,
                Err(err) => {
                    eprintln!("{}", err.red());
                    continue;
                }
            };

            if target.is_symlink() {
                let link = match fs::read_link(target) {
                    Ok(link) => link,
                    Err(err) => {
                        eprintln!("{err}");
                        continue;
                    }
                };

                if link == f.path {
                    symlinked.entry(f.group_name.clone()).or_default();

                    let group = symlinked.get_mut(&f.group_name).unwrap();
                    group.insert(f);
                } else {
                    not_owned.entry(f.group_name.clone()).or_default();

                    let group = not_owned.get_mut(&f.group_name).unwrap();
                    group.insert(f);
                }
            } else {
                if target.is_dir() {
                    continue;
                }

                not_symlinked.entry(f.group_name.clone()).or_default();

                let group = not_symlinked.get_mut(&f.group_name).unwrap();
                group.insert(f);
            }
        }

        fn remove_empty_groups(group_type: HashCache) -> HashCache {
            group_type
                .iter()
                .filter(|(_, v)| !v.is_empty())
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect()
        }

        // removes entries for paths that are subpaths of another entry (canonicalization).
        // this procedure makes so that symlinks are shallow.
        //
        // shallow symlinking: only symlinking files/directories that don't exist already
        fn canonicalize_groups(groups: &mut HashCache) {
            for files in groups.values_mut() {
                let files_copy = files.clone();

                for file in &files_copy {
                    for file2 in &files_copy {
                        if file2.path != file.path && file2.path.starts_with(&file.path) {
                            files.remove(file2);
                        }
                    }
                }
            }
        }

        // canonicalizes not_symlinked based on symlinked cache
        //
        // this is necessary because if a directory is canonicalized and symlinked,
        // files inside it won't be symlinked and thus marked as `not_symlinked` wrongly.
        for (group, files) in &symlinked {
            let Some(unsymlinked_group) = not_symlinked.get_mut(group) else {
                continue;
            };

            let unsymlinked_group_copy = unsymlinked_group.clone();

            for file1 in files {
                for file2 in unsymlinked_group_copy.iter() {
                    if file2.path.starts_with(&file1.path) {
                        unsymlinked_group.remove(file2);
                    }
                }
            }
        }

        canonicalize_groups(&mut symlinked);
        canonicalize_groups(&mut not_symlinked);
        canonicalize_groups(&mut not_owned);

        self.symlinked = remove_empty_groups(symlinked);
        self.not_symlinked = remove_empty_groups(not_symlinked);
        self.not_owned = remove_empty_groups(not_owned);

        Ok(self)
    }

    fn is_empty(&self) -> bool {
        self.symlinked.is_empty() && self.not_symlinked.is_empty() && self.not_owned.is_empty()
    }

    /// only meant for internal use
    fn get_related_cond_groups(
        &self,
        target_group: &str,
        cache: &HashCache,
    ) -> Option<Vec<String>> {
        if dotfiles::group_ends_with_target_name(target_group) {
            return match cache.contains_key(target_group) {
                true => Some(vec![target_group.to_string()]),
                false => None,
            };
        }

        let cond_groups: Vec<String> = cache
            .iter()
            .filter(|(group, _)| {
                dotfiles::group_without_target(group) == target_group
                    && dotfiles::group_is_valid_target(group, &self.ctx.custom_targets)
            })
            .map(|(group, _)| group.clone())
            .collect();

        if cond_groups.is_empty() {
            return None;
        }

        Some(cond_groups)
    }

    /// Returns target_group and all of its conditional groups that are valid in the current platform
    ///
    /// symlinked: gets symlinked conditional groups if true, otherwise gets not symlinked ones
    fn get_related_conditional_groups(
        &self,
        target_group: &str,
        symtype: BitFlags<SymlinkType>,
    ) -> Option<Vec<String>> {
        let symlinked = if symtype.contains(SymlinkType::Symlinked) {
            self.get_related_cond_groups(target_group, &self.symlinked)
        } else {
            None
        };

        let not_symlinked = if symtype.contains(SymlinkType::NotSymlinked) {
            self.get_related_cond_groups(target_group, &self.not_symlinked)
        } else {
            None
        };

        let not_owned = if symtype.contains(SymlinkType::NotOwned) {
            self.get_related_cond_groups(target_group, &self.not_owned)
        } else {
            None
        };

        let cond_groups: Vec<_> = symlinked
            .iter()
            .chain(not_symlinked.iter())
            .chain(not_owned.iter())
            .flatten()
            .map(|group| group.to_owned())
            .collect();

        if cond_groups.is_empty() {
            None
        } else {
            Some(cond_groups)
        }
    }

    /// returns a cache with files in dotfiles that already exist in $TUCKR_TARGET
    fn get_conflicts_in_cache(&self) -> HashCache {
        let mut conflicts = HashCache::new();

        // mark group as conflicting if at least one value already exists in $TUCKR_TARGET
        for files in self.not_symlinked.values() {
            for file in files {
                if file.to_target_path().unwrap().exists()
                    && file.is_valid_target(&self.ctx.custom_targets)
                {
                    conflicts.entry(file.group_name.clone()).or_default();
                    let curr_entry = conflicts.get_mut(&file.group_name).unwrap();
                    curr_entry.insert(file.clone());
                }
            }
        }

        // doesn't mark not owned dotfiles as conflicts if a higher priority dotfile
        // with the same file is already symlinked. this allows dotfile fallbacks to
        // work properly instead of falsely flagged as conflicts
        for files in self.not_owned.values() {
            for file in files {
                conflicts.entry(file.group_name.clone()).or_default();
                let curr_entry = conflicts.get_mut(&file.group_name).unwrap();

                let dotfile_source = file.to_target_path().unwrap().read_link().unwrap();
                let Ok(dotfile) = Dotfile::try_from(dotfile_source) else {
                    curr_entry.insert(file.clone());
                    continue;
                };

                let target_has_higher_priority = dotfiles::get_group_priority(&dotfile.group_name)
                    > dotfiles::get_group_priority(&file.group_name);
                let not_same_base_group = dotfiles::group_without_target(&file.group_name)
                    != dotfiles::group_without_target(&dotfile.group_name);

                if dotfile.path != file.path && target_has_higher_priority && not_same_base_group {
                    curr_entry.insert(file.clone());
                }
            }
        }

        conflicts.into_iter().filter(|g| !g.1.is_empty()).collect()
    }

    /// Symlinks all the files of a group to the user's $TUCKR_TARGET
    fn add(&self, dry_run: bool, only_files: bool, group: &str) {
        let Some(mut groups) =
            self.get_related_conditional_groups(group, SymlinkType::NotSymlinked.into())
        else {
            return;
        };

        loop {
            let Some(idx) = dotfiles::get_highest_priority_target_idx(&groups) else {
                break;
            };

            let group = &groups[idx];
            let group = Dotfile::try_from(self.dotfiles_dir.join("Configs").join(group)).unwrap();
            if group.path.exists() {
                for f in group.try_iter().unwrap() {
                    if only_files {
                        if f.path.is_dir() {
                            continue;
                        }

                        // we need to ensure that the target dotfile's parent exists otherwise symlink will fail
                        let f_target = f.to_target_path().unwrap();
                        let target_parent = f_target.parent().unwrap();

                        if !target_parent.exists() {
                            fs::create_dir_all(target_parent).unwrap();
                        }
                    }

                    symlink_file(dry_run, f.path);
                }
            } else {
                eprintln!(
                    "{}",
                    t!("errors.no_dotfiles_for_group", group = group.group_name).red()
                );
            }

            groups.remove(idx);
        }
    }

    /// Deletes symlinks from $TUCKR_TARGET if they're owned by dotfiles dir
    fn remove(&self, dry_run: bool, group: &str) {
        fn remove_symlink(dry_run: bool, file: PathBuf) {
            let dotfile = Dotfile::try_from(file).unwrap();
            let target_dotfile = dotfile.to_target_path().unwrap();
            let Ok(linked) = fs::read_link(&target_dotfile) else {
                return;
            };

            if dotfile.path != linked {
                return;
            }

            if dry_run {
                eprintln!(
                    "{}",
                    t!("dry-run.removing_x", x = target_dotfile.display()).red()
                );
                return;
            }

            if target_dotfile.is_dir() {
                fs::remove_dir_all(&target_dotfile).unwrap();
            } else {
                fs::remove_file(&target_dotfile)
                    .map_err(|err| format!("error with path `{}`: {err}", target_dotfile.display()))
                    .unwrap();
            }
        }

        let Some(groups) =
            self.get_related_conditional_groups(group, SymlinkType::Symlinked.into())
        else {
            return;
        };

        for group in groups {
            let group = Dotfile::try_from(self.dotfiles_dir.join("Configs").join(&group)).unwrap();

            if !group.path.exists() {
                eprintln!("{}", t!("errors.no_group", group = group.group_name).red());
                continue;
            }

            group
                .try_iter()
                .unwrap()
                .for_each(|f| remove_symlink(dry_run, f.path));
        }
    }
}

/// Adds symlinks
#[allow(clippy::too_many_arguments)]
pub fn add_cmd(
    ctx: &Context,
    only_files: bool,
    groups: &[String],
    exclude: &[String],
    force: bool,
    adopt: bool,
    assume_yes: bool,
) -> Result<(), ExitCode> {
    if !assume_yes && (force || adopt) {
        let confirmed = fileops::get_user_confirmation(if force {
            "Are you sure you want to override conflicts?"
        } else {
            "Are you sure you want to adopt conflicts?"
        });

        if !confirmed {
            return Ok(());
        }
    }

    let sym = SymlinkHandler::try_new(ctx)?;

    if let Some(invalid_groups) =
        dotfiles::check_invalid_groups(ctx.profile.clone(), DotfileType::Configs, groups)
    {
        for group in invalid_groups {
            eprintln!("{}", t!("errors.x_doesnt_exist", x = group).red());
        }

        return Err(ReturnCode::NoSetupFolder.into());
    };

    let add_group = |group: &String| {
        if exclude.contains(group) {
            return;
        }

        fn remove_files_and_decide_if_adopt(
            group: &str,
            status_group: &HashCache,
            adopt: bool,
            dry_run: bool,
        ) {
            let group = status_group.get(group);
            let Some(group_files) = group else { return };

            for file in group_files {
                let target_file = file.to_target_path().unwrap();

                let deleted_file = if adopt { &file.path } else { &target_file };

                if dry_run {
                    eprintln!(
                        "{}",
                        t!("dry-run.removing_x", x = deleted_file.display()).red()
                    );
                } else if target_file.is_dir() {
                    fs::remove_dir_all(deleted_file).unwrap();
                } else if target_file.is_file() {
                    fs::remove_file(deleted_file).unwrap();
                }

                if adopt {
                    if dry_run {
                        eprintln!(
                            "{}",
                            t!(
                                "dry-run.moving_x_to_y",
                                x = target_file.display(),
                                y = file.path.display()
                            )
                            .yellow()
                        );
                    } else {
                        fs::rename(target_file, &file.path).unwrap();
                    }
                }
            }
        }

        // Symlink dotfile by force
        if force {
            remove_files_and_decide_if_adopt(group, &sym.not_owned, false, ctx.dry_run);
            remove_files_and_decide_if_adopt(group, &sym.not_symlinked, false, ctx.dry_run);
        }

        // Discard dotfile and adopt the conflicting dotfile
        if adopt {
            remove_files_and_decide_if_adopt(group, &sym.not_owned, true, ctx.dry_run);
            remove_files_and_decide_if_adopt(group, &sym.not_symlinked, true, ctx.dry_run);
        }

        sym.add(ctx.dry_run, only_files, group)
    };

    if groups.contains(&"*".to_string()) {
        for group in sym.not_symlinked.keys() {
            if dotfiles::group_is_valid_target(group, &ctx.custom_targets) {
                add_group(group);
            }
        }
    } else {
        let groups = {
            let mut related_groups = Vec::new();

            for group in groups {
                let Some(mut related) = sym.get_related_conditional_groups(
                    group,
                    SymlinkType::NotSymlinked | SymlinkType::NotOwned,
                ) else {
                    continue;
                };

                related_groups.append(&mut related);
            }

            related_groups
        };

        groups.iter().for_each(add_group);
    }

    let post_add_sym = SymlinkHandler::try_new(ctx)?;
    let potential_conflicts = post_add_sym.get_conflicts_in_cache();

    if !potential_conflicts.is_empty() {
        if groups.iter().any(|g| g == "*") {
            println!(
                "{} {}",
                t!("info.conflicts_detected").yellow(),
                t!("info.learn_more_about_conflicts", cmd = "tuckr status").yellow(),
            );
        }

        if groups.iter().any(|g| potential_conflicts.contains_key(g)) {
            println!(
                "{} {}",
                t!("info.conflicts_detected").yellow(),
                t!(
                    "info.conflicting_groups_not_added_until_resolved",
                    cmd = "tuckr status"
                )
                .yellow(),
            );
            return print_groups_status(ctx, &post_add_sym, groups.into());
        }
    }
    Ok(())
}

/// Removes symlinks
pub fn remove_cmd(ctx: &Context, groups: &[String], exclude: &[String]) -> Result<(), ExitCode> {
    let sym = SymlinkHandler::try_new(ctx)?;

    // remove command should not care about whether a group is a valid target
    // if it's been added, removing it should always be possible
    if groups.contains(&"*".to_string()) {
        for group in sym.symlinked.keys() {
            if exclude.contains(group) {
                continue;
            }
            sym.remove(ctx.dry_run, group);
        }
        return Ok(());
    }

    if let Some(invalid_groups) =
        dotfiles::check_invalid_groups(ctx.profile.clone(), DotfileType::Configs, groups)
    {
        for group in invalid_groups {
            eprintln!("{}", t!("errors.x_doesnt_exist", x = group).red());
        }

        return Err(ReturnCode::NoSetupFolder.into());
    };

    let related_groups: Vec<_> = sym
        .symlinked
        .keys()
        .filter(|g| {
            groups.contains(g)
                || groups
                    .iter()
                    .any(|group| dotfiles::group_without_target(g) == group)
        })
        .collect();

    for group in related_groups {
        if exclude.contains(group) {
            continue;
        }
        sym.remove(ctx.dry_run, group);
    }

    Ok(())
}

fn print_global_status(sym: &SymlinkHandler) -> Result<(), ExitCode> {
    #[derive(Tabled, Debug)]
    struct SymlinkRow<'a> {
        #[tabled(rename = "Symlinked")]
        symlinked: &'a str,

        #[tabled(rename = "Not Symlinked")]
        not_symlinked: &'a str,
    }

    // --- process status from symlink ---
    // groups that are both in symlinked and not_symlinked
    // will be marked as not_symlinked only

    let (symlinked, not_symlinked) = {
        let mut not_symlinked: Vec<_> = sym
            .not_symlinked
            .keys()
            .map(|group| group.as_str())
            .collect();

        let mut symlinked: Vec<_> = sym
            .symlinked
            .keys()
            .filter_map(|group| {
                if sym
                    .get_related_conditional_groups(group, SymlinkType::NotSymlinked.into())
                    .is_none()
                {
                    Some(dotfiles::group_without_target(group))
                } else {
                    not_symlinked.push(group);
                    None
                }
            })
            .collect();

        let mut not_symlinked: Vec<_> = not_symlinked
            .iter()
            .filter_map(|group| {
                if dotfiles::group_is_valid_target(group, &sym.ctx.custom_targets) {
                    Some(dotfiles::group_without_target(group))
                } else {
                    None
                }
            })
            .collect();

        symlinked.sort();
        symlinked.dedup();

        not_symlinked.sort();
        not_symlinked.dedup();

        (symlinked, not_symlinked)
    };

    let status_rows: Vec<SymlinkRow> = {
        let (longest, shortest, symlinked_is_longest) = if symlinked.len() >= not_symlinked.len() {
            (&symlinked, &not_symlinked, true)
        } else {
            (&not_symlinked, &symlinked, false)
        };

        longest
            .iter()
            .zip(shortest.iter().chain(std::iter::repeat(&"")))
            .map(|(longest, shortest)| SymlinkRow {
                symlinked: if symlinked_is_longest {
                    longest
                } else {
                    shortest
                },

                not_symlinked: if symlinked_is_longest {
                    shortest
                } else {
                    longest
                },
            })
            .collect()
    };

    // --- detect conflicts ---
    let conflicts = sym.get_conflicts_in_cache();
    let conflicts: HashSet<_> = conflicts.keys().collect();

    // --- Creates all the tables and prints them ---
    use tabled::col;
    use tabled::settings::{
        Alignment, Margin, Modify, Style, format::Format, object::Columns, object::Rows,
    };

    let mut sym_table = Table::new(status_rows);
    sym_table
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Format::content(|s| s.default_color().to_string())))
        .with(Modify::new(Columns::single(0)).with(Format::content(|s| s.green().to_string())))
        .with(Modify::new(Columns::single(1)).with(Format::content(|s| s.red().to_string())));

    let conflict_builder = tabled::Table::builder(&conflicts)
        .index()
        .column(0)
        .name(Some("Conflicting Dotfiles".yellow().to_string()));
    let mut conflict_table = conflict_builder.build();
    conflict_table
        .with(Style::empty())
        .with(Alignment::center());

    // Creates a table with sym_table and conflict_table
    let mut final_table = if conflicts.is_empty() {
        col![sym_table]
    } else {
        col![sym_table, conflict_table]
    };

    final_table
        .with(Style::empty())
        .with(Alignment::center())
        .with(Margin::new(4, 4, 1, 1));
    println!("{final_table}");

    if !conflicts.is_empty() {
        println!(
            "{}",
            t!(
                "info.learn_more_about_conflicts",
                cmd = "tuckr status <group...>"
            )
        );
    }

    // Determines exit code for the command based on the dotfiles' status
    if !symlinked.is_empty() && not_symlinked.is_empty() && conflicts.is_empty() {
        Ok(())
    } else {
        Err(ExitCode::FAILURE)
    }
}

fn print_groups_status(
    ctx: &Context,
    sym: &SymlinkHandler,
    groups: Vec<String>,
) -> Result<(), ExitCode> {
    fn get_related_groups(
        sym: &SymlinkHandler,
        symlink_types: BitFlags<SymlinkType>,
        groups: &[String],
    ) -> Vec<String> {
        let mut related_groups = Vec::new();
        for group in groups {
            let Some(mut cond_groups) = sym.get_related_conditional_groups(group, symlink_types)
            else {
                continue;
            };

            related_groups.append(&mut cond_groups);
        }

        related_groups
    }

    let not_symlinked = get_related_groups(
        sym,
        enumflags2::make_bitflags!(SymlinkType::{NotSymlinked | NotOwned}),
        &groups,
    );
    let symlinked: Vec<_> = get_related_groups(sym, SymlinkType::Symlinked.into(), &groups);

    let unsupported = {
        let mut unsupported = groups
            .iter()
            .map(|group| Dotfile::try_from(sym.dotfiles_dir.join("Configs").join(group)).unwrap())
            .filter(|group| !group.is_valid_target(&ctx.custom_targets))
            .map(|group| group.group_name)
            .collect::<Vec<_>>();

        unsupported.sort();
        unsupported.dedup();
        unsupported
    };

    let file_conflicts: HashCache = sym
        .get_conflicts_in_cache()
        .into_iter()
        .filter(|(g, _)| {
            groups.contains(g) || groups.contains(&dotfiles::group_without_target(g).to_string())
        })
        .collect();

    if !file_conflicts.is_empty() || !not_symlinked.is_empty() {
        fn print_conflicts(conflicts_cache: &HashCache, group: &str) {
            let Some(conflicts) = conflicts_cache.get(group) else {
                return;
            };

            for file in conflicts {
                let conflict = file.to_target_path().unwrap();
                let msg = if !conflict.is_symlink() {
                    t!("errors.already_exists")
                } else {
                    let conflict_dotfile = Dotfile::try_from(conflict.read_link().unwrap());

                    match conflict_dotfile {
                        Ok(conflict) => {
                            if file.path != conflict.path {
                                t!("errors.already_exists")
                            } else {
                                unreachable!();
                            }
                        }
                        Err(_) => t!("errors.symlinks_elsewhere"),
                    }
                };

                println!("\t -> {} ({})", conflict.display(), msg,);
            }
        }

        println!("{}:", t!("table-column.not_symlinked"));
        for group in not_symlinked {
            if file_conflicts.contains_key(&group) {
                continue;
            }
            println!("\t{}", group.red());
        }

        for group in file_conflicts.keys() {
            print!("\t{}", group.red());
            print_conflicts(&file_conflicts, group);
        }

        println!();
    }

    if !symlinked.is_empty() {
        println!("{}:", t!("table-column.symlinked"));
        for group in symlinked {
            println!("\t{}", group.green());
        }
        println!();
    }

    if !unsupported.is_empty() {
        println!("{}:", t!("errors.not_supported_on_this_platform"));
        for group in unsupported {
            println!("\t{}", group.yellow());
        }
        println!();
    }

    let invalid_groups =
        dotfiles::check_invalid_groups(ctx.profile.clone(), DotfileType::Configs, &groups);
    if let Some(invalid_groups) = &invalid_groups {
        eprintln!("{}:", t!("errors.following_groups_dont_exist"));
        for group in invalid_groups {
            eprintln!("\t{}", group.red());
        }
        println!();
    }

    if !file_conflicts.is_empty() {
        println!(
            "{}",
            t!("info.learn_how_to_fix_symlinks", cmd = "tuckr help add")
        );
    }

    if invalid_groups.is_none() {
        return Err(ReturnCode::NoSetupFolder.into());
    }

    Ok(())
}

/// Prints symlinking status
pub fn status_cmd(ctx: &Context, groups: Option<Vec<String>>) -> Result<(), ExitCode> {
    let sym = SymlinkHandler::try_new(ctx)?;

    if sym.is_empty() {
        println!("{}", t!("errors.no_x_setup_yet", x = "dotfiles").yellow());
        println!(
            "{}",
            t!(
                "info.how_to_get_started",
                dotfiles_config_dir = sym.dotfiles_dir.join("Configs").display()
            )
        );
        return Err(ReturnCode::NoSetupFolder.into());
    }

    match groups {
        Some(groups) => {
            let mut invalid_group_errs = Vec::new();

            let groups: Vec<_> = groups
                .into_iter()
                .filter_map(|g| match dotfiles::is_valid_groupname(&g) {
                    Ok(()) => Some(g),
                    Err(err) => {
                        invalid_group_errs.push(err);
                        None
                    }
                })
                .collect();

            let ret = print_groups_status(ctx, &sym, groups);

            if !invalid_group_errs.is_empty() {
                for err in invalid_group_errs {
                    eprintln!("{}", err.red());
                }
            }

            return ret;
        }

        None => print_global_status(&sym)?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
        path,
    };

    use owo_colors::OwoColorize;

    use super::SymlinkHandler;
    use crate::Context;
    use crate::dotfiles::{self, Dotfile};

    /// this ensures that the tests never
    // use crate::Context; fail with weird random panics
    #[must_use = "must be initialized before every test"]
    struct Test {
        files_used: Vec<path::PathBuf>,
    }

    impl Test {
        fn start() -> Self {
            crate::fileops::init_cmd(&Context::default()).unwrap();
            let dotfiles_dir = dotfiles::get_dotfiles_path(None).unwrap();
            let group_dir = dotfiles_dir.join("Configs").join("Group1");
            let new_config_dir = group_dir.join(".config");

            fs::create_dir_all(&new_config_dir).unwrap();

            let filepaths = [
                new_config_dir.join("group_file"),
                group_dir.join("group_file_0"),
            ];

            for filepath in &filepaths {
                let mut file = File::create(filepath).unwrap();
                _ = file
                    .write("Some random content on file".as_bytes())
                    .unwrap();
            }

            Self {
                files_used: filepaths.to_vec(),
            }
        }
    }

    impl Drop for Test {
        fn drop(&mut self) {
            let Ok(dotfiles_dir) = dotfiles::get_dotfiles_path(None) else {
                eprintln!("{}", "Failed to clean up test.".red());
                return;
            };

            for file in &self.files_used {
                // delete everything to ensure everything starts from a blank slate
                if file.exists() {
                    let dotfile_path = Dotfile::try_from(file.clone()).unwrap();
                    _ = fs::remove_file(dotfile_path.to_target_path().unwrap());
                }
            }

            if dotfiles_dir.exists() {
                _ = super::remove_cmd(&Context::default(), &["*".to_string()], &[]);
                fs::remove_dir_all(dotfiles_dir).unwrap();
            }
        }
    }

    fn test_adding_symlink() {
        let _test = Test::start();

        let mut ctx = Context::default();
        ctx.profile = None;

        let sym = SymlinkHandler::try_new(&ctx).unwrap();
        assert!(
            !sym.not_symlinked.is_empty() || !sym.symlinked.is_empty() || !sym.not_owned.is_empty()
        );

        assert!(!sym.symlinked.contains_key("Group1"));
        super::add_cmd(
            &Context::default(),
            false,
            &["Group1".to_string()],
            &[],
            false,
            false,
            false,
        )
        .unwrap();

        let sym = SymlinkHandler::try_new(&ctx).unwrap();
        assert!(sym.symlinked.contains_key("Group1"));
    }

    fn test_removing_symlink() {
        let _test = Test::start();

        super::add_cmd(
            &Context::default(),
            false,
            &["Group1".to_string()],
            &[],
            false,
            false,
            false,
        )
        .unwrap();

        let ctx = Context::default();

        let sym = SymlinkHandler::try_new(&ctx).unwrap();
        assert!(
            !sym.not_symlinked.is_empty() || !sym.symlinked.is_empty() || !sym.not_owned.is_empty()
        );

        assert!(!sym.not_symlinked.contains_key("Group1"));

        super::remove_cmd(&Context::default(), &["Group1".to_string()], &[]).unwrap();
        let sym = SymlinkHandler::try_new(&ctx).unwrap();
        assert!(sym.not_symlinked.contains_key("Group1"));
    }

    #[test]
    fn add_and_remove_symlink() {
        test_adding_symlink();
        test_removing_symlink();
    }
}
