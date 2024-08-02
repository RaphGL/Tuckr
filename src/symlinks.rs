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
//! $HOME equivalents are pointing to them and categorizing them accordingly.

use crate::dotfiles::{self, Dotfile, DotfileType, ReturnCode};
use owo_colors::OwoColorize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use tabled::{Table, Tabled};

fn symlink_file(f: PathBuf) {
    let src_path = f;
    match Dotfile::try_from(src_path.clone()) {
        Some(group) => {
            let target_path = group.to_target_path();

            #[cfg(target_family = "unix")]
            {
                _ = std::os::unix::fs::symlink(src_path, target_path);
            }

            #[cfg(target_family = "windows")]
            {
                _ = std::os::windows::fs::symlink_file(src_path, target_path);
            }
        }

        None => eprintln!("Failed to link {}", src_path.to_str().unwrap()),
    }
}

type HashCache = HashMap<String, HashSet<Dotfile>>;

/// Handles dotfile symlinking and their current status
struct SymlinkHandler {
    dotfiles_dir: PathBuf,    // path to the dotfiles directory
    symlinked: HashCache,     // dotfiles that have been symlinked from Dotfiles/Configs
    not_symlinked: HashCache, // dotfiles that haven't been symlinked to $HOME yet
    not_owned: HashCache, // dotfiles that are symlinks but points somewhere outside of their respective Dotfiles/Configs's group dir
}

impl SymlinkHandler {
    /// Initializes SymlinkHandler and fills it dotfiles' status information
    fn try_new() -> Result<Self, ExitCode> {
        let dotfiles_dir = match dotfiles::get_dotfiles_path() {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("{e}");
                return Err(ReturnCode::CouldntFindDotfiles.into());
            }
        };

        let symlinker = SymlinkHandler {
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
        let configs_dir = Dotfile::try_from(self.dotfiles_dir.join("Configs")).unwrap();

        let mut symlinked = HashCache::new();
        let mut not_symlinked = HashCache::new();
        let mut not_owned = HashCache::new();

        // iterates over every file inside dotfiles/Config and determines their symlink status
        configs_dir.map(|f| {
            // skip group directories otherwise it would try to link dotfiles/Configs/Groups to the users home
            if f.path == f.group_path {
                return;
            }

            let target = f.to_target_path();

            if target.is_symlink() {
                let link = match fs::read_link(target) {
                    Ok(link) => link,
                    Err(err) => {
                        eprintln!("{err}");
                        return;
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
                    return;
                }

                not_symlinked.entry(f.group_name.clone()).or_default();

                let group = not_symlinked.get_mut(&f.group_name).unwrap();
                group.insert(f);
            }
        });

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

    /// Returns all conditional groups with the same name that satify the current user's platform
    ///
    /// symlinked: gets symlinked conditional groupsif true, otherwise gets not symlinked ones
    fn get_related_conditional_groups(&self, target_group: &str, symlinked: bool) -> Vec<String> {
        if symlinked {
            &self.symlinked
        } else {
            &self.not_symlinked
        }
        .iter()
        .filter(|(group, files)| {
            group.starts_with(target_group) &&
                // any file in this group is in the same target so just pick any file to check
                 files.iter().next().unwrap().is_valid_target()
        })
        .map(|(group, _)| group.clone())
        .collect()
    }

    /// Symlinks all the files of a group to the user's $HOME
    fn add(&self, group: &str) {
        let groups = self.get_related_conditional_groups(group, false);

        for group in groups {
            let group = Dotfile::try_from(self.dotfiles_dir.join("Configs").join(&group)).unwrap();
            if group.path.exists() {
                // iterate through all the files in group_dir
                group.map(|f| symlink_file(f.path));
            } else {
                eprintln!(
                    "{} {}",
                    "There's no dotfiles for".red(),
                    group.group_name.red()
                );
            }
        }
    }

    /// Deletes symlinks from $HOME if they're owned by dotfiles dir
    fn remove(&self, group: &str) {
        fn remove_symlink(file: PathBuf) {
            let dotfile = Dotfile::try_from(file).unwrap().to_target_path();
            if let Ok(linked) = fs::read_link(&dotfile) {
                let dotfiles_configs_path = PathBuf::from("dotfiles").join("Configs");
                let dotfiles_configs_path = dotfiles_configs_path.to_str().unwrap();
                if linked.to_str().unwrap().contains(dotfiles_configs_path) {
                    fs::remove_file(dotfile).unwrap();
                }
            }
        }

        let groups = self.get_related_conditional_groups(group, true);

        for group in groups {
            let group = Dotfile::try_from(self.dotfiles_dir.join("Configs").join(&group)).unwrap();
            if group.path.exists() {
                group.map(|f| remove_symlink(f.path));
            } else {
                eprintln!(
                    "{} {}",
                    "There's no group called".red(),
                    group.group_name.red()
                );
            }
        }
    }
}

/// groups: the groups that will be iterated
///
/// exclude: the groups that will be ignored
///
/// symlinked: whether it should be applied to symlinked or non symlinked groups
/// iterates over each group in the dotfiles and calls a function F giving it the SymlinkHandler
/// instance and the name of the group that's being handled
fn foreach_group<F>(
    groups: &[String],
    exclude: &[String],
    symlinked: bool,
    func: F,
) -> Result<(), ExitCode>
where
    F: Fn(&SymlinkHandler, &String),
{
    // loads the runtime information needed to carry out actions
    let sym = SymlinkHandler::try_new()?;

    // detect if user provided an invalid group
    if let Some(invalid_groups) = dotfiles::check_invalid_groups(DotfileType::Configs, groups) {
        for group in invalid_groups {
            eprintln!("{}", format!("{group} doesn't exist.").red());
        }
        return Err(ReturnCode::NoSetupFolder.into());
    }

    // handles wildcard
    if groups.contains(&"*".to_string()) {
        let symgroups = if symlinked {
            &sym.not_symlinked
        } else {
            &sym.symlinked
        };

        for group in symgroups.keys() {
            // Takes the name of the group to be passed the function
            // Ignore groups in the excludes array
            if exclude.contains(group) {
                continue;
            }
            // do something with the group name
            // passing the sym context
            func(&sym, group);
        }

        return Ok(());
    }

    for group in groups {
        if exclude.contains(group) {
            continue;
        }
        func(&sym, group);
    }

    Ok(())
}

/// Adds symlinks
pub fn add_cmd(
    groups: &[String],
    exclude: &[String],
    force: bool,
    adopt: bool,
) -> Result<(), ExitCode> {
    if force {
        print!("Are you sure you want to override conflicts? (N/y) ");
    } else if adopt {
        print!("Are you sure you want to adopt conflicts? (N/y) ");
    }

    if force || adopt {
        std::io::stdout()
            .flush()
            .expect("Could not print to stdout");

        let mut answer = String::new();
        std::io::stdin()
            .read_line(&mut answer)
            .expect("Could not read from stdin");

        match answer.trim().to_lowercase().as_str() {
            "y" | "yes" => (),
            _ => return Ok(()),
        }
    }

    foreach_group(groups, exclude, true, |sym, group| {
        // Symlink dotfile by force
        if force {
            let remove_overlapping_files = |status_group: &HashCache| {
                for (group, group_files) in status_group {
                    if !groups.contains(group) {
                        continue;
                    }

                    for file in group_files {
                        let target_file = file.to_target_path();
                        if target_file.is_dir() {
                            fs::remove_dir_all(target_file).unwrap();
                        } else if target_file.is_file() {
                            fs::remove_file(target_file).unwrap();
                        }
                    }
                }
            };

            remove_overlapping_files(&sym.not_owned);
            remove_overlapping_files(&sym.not_symlinked);
        }

        // Discard dotfile and adopt the conflicting dotfile
        if adopt {
            let adopt_overlapping_files = |status_group: &HashCache| {
                for (group, group_files) in status_group {
                    if !groups.contains(group) {
                        continue;
                    }

                    for file in group_files {
                        let target_file = file.to_target_path();
                        if target_file.is_dir() {
                            fs::remove_dir_all(&file.path).unwrap();
                        } else if target_file.is_file() {
                            fs::remove_file(&file.path).unwrap();
                        }

                        fs::rename(target_file, &file.path).unwrap();
                    }
                }
            };

            adopt_overlapping_files(&sym.not_owned);
            adopt_overlapping_files(&sym.not_symlinked);
        }

        sym.add(group)
    })?;

    Ok(())
}

/// Removes symlinks
pub fn remove_cmd(groups: &[String], exclude: &[String]) -> Result<(), ExitCode> {
    foreach_group(groups, exclude, false, |sym, p| sym.remove(p))?;
    Ok(())
}

/// returns a cache with files in dotfiles that already exist in $HOME
fn get_conflicts_in_cache(cache: &HashCache) -> HashCache {
    let mut conflicts = HashCache::new();

    // mark group as conflicting if at least one value already exists in $HOME
    for files in cache.values() {
        for file in files {
            if !file.to_target_path().exists() {
                continue;
            }

            conflicts.entry(file.group_name.clone()).or_default();
            let entry = conflicts.get_mut(&file.group_name).unwrap();
            entry.insert(file.clone());
        }
    }

    conflicts
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
        let mut not_symlinked: Vec<_> = sym.not_symlinked.keys().collect();

        let mut symlinked: Vec<_> = sym
            .symlinked
            .keys()
            .filter(|group| {
                if sym.get_related_conditional_groups(group, false).is_empty()
                    // ignore conditional groups
                    && !group.contains('_')
                {
                    true
                } else {
                    not_symlinked.push(group);
                    false
                }
            })
            .collect();

        symlinked.sort();

        let mut not_symlinked: Vec<_> = not_symlinked
            .into_iter()
            .filter(|group| !group.contains('_'))
            .collect();
        not_symlinked.sort();
        (symlinked, not_symlinked)
    };

    let empty_str = String::from("");
    let status_rows: Vec<SymlinkRow> = {
        let (longest, shortest, symlinked_is_longest) = if symlinked.len() >= not_symlinked.len() {
            (&symlinked, &not_symlinked, true)
        } else {
            (&not_symlinked, &symlinked, false)
        };

        longest
            .iter()
            .zip(shortest.iter().chain(std::iter::repeat(&&empty_str)))
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
    let conflicts = get_conflicts_in_cache(&sym.not_symlinked);
    // whether a conflict is a symlink or a pre-existing file does not matter for global status
    // so we just add them together
    let conflicts: HashSet<_> = conflicts.keys().chain(sym.not_owned.keys()).collect();

    // --- Creates all the tables and prints them ---
    use tabled::{
        col, format::Format, object::Columns, object::Rows, Alignment, Margin, Modify, Style,
    };

    let mut sym_table = Table::new(status_rows);
    sym_table
        .with(Style::rounded())
        .with(Margin::new(4, 4, 1, 1))
        .with(Modify::new(Rows::first()).with(Format::new(|s| s.default_color().to_string())))
        .with(Modify::new(Columns::single(0)).with(Format::new(|s| s.green().to_string())))
        .with(Modify::new(Columns::single(1)).with(Format::new(|s| s.red().to_string())));

    let mut conflict_table = Table::builder(&conflicts)
        .set_columns(["Conflicting Dotfiles".yellow().to_string()])
        .clone()
        .build();
    conflict_table
        .with(Style::empty())
        .with(Alignment::center());

    // Creates a table with sym_table and conflict_table
    let mut final_table = if conflicts.is_empty() {
        col![sym_table]
    } else {
        col![sym_table, conflict_table]
    };

    final_table.with(Style::empty()).with(Alignment::center());
    println!("{final_table}");

    if !conflicts.is_empty() {
        println!("\nTo learn more about conflicting dotfiles run: `tuckr status <group...>`")
    }

    // Determines exit code for the command based on the dotfiles' status
    if !symlinked.is_empty() && not_symlinked.is_empty() && conflicts.is_empty() {
        Ok(())
    } else {
        Err(ExitCode::FAILURE)
    }
}

fn print_groups_status(sym: &SymlinkHandler, groups: Vec<String>) -> Result<(), ExitCode> {
    let get_related_groups =
        |sym: &SymlinkHandler, not_symlinked_groups: Option<&Vec<String>>| -> Vec<String> {
            let mut related_groups = Vec::new();

            let symlinked = not_symlinked_groups.is_some();

            // merges conditional groups into their base group
            // eg: `dotfile_unix` gets merged into the `dotfile` group
            for base_group in &groups {
                let related_cond_groups = sym.get_related_conditional_groups(base_group, symlinked);

                for group in related_cond_groups {
                    match not_symlinked_groups {
                        Some(not_symlinked) => {
                            if not_symlinked.contains(&group) {
                                continue;
                            }
                        }

                        None => {
                            if !sym.not_symlinked.contains_key(&group) {
                                continue;
                            }
                        }
                    }

                    related_groups.push(group);
                }
            }

            related_groups.sort();
            related_groups.dedup();
            related_groups
        };

    let not_symlinked = get_related_groups(sym, None);
    let symlinked = get_related_groups(sym, Some(&not_symlinked));

    let not_owned: HashCache = sym
        .not_owned
        .clone()
        .into_iter()
        .filter(|(group, _)| groups.contains(group))
        .collect();

    let unsupported = {
        let mut unsupported = groups
            .iter()
            .map(|group| Dotfile::try_from(sym.dotfiles_dir.join("Configs").join(group)).unwrap())
            .filter(|group| !group.is_valid_target())
            .map(|group| group.group_name)
            .collect::<Vec<_>>();

        unsupported.sort();
        unsupported.dedup();
        unsupported
    };

    if !not_symlinked.is_empty() || !not_owned.is_empty() {
        let print_conflicts = |conflicts_cache: &HashCache, group: &str, msg: &str| {
            let Some(conflicts) = conflicts_cache.get(group) else {
                return;
            };

            for file in conflicts {
                if file.group_name != group {
                    continue;
                }

                let conflict = file.to_target_path();
                println!("\t\t-> {} ({})", conflict.display(), msg,);
            }
        };

        let file_conflicts = get_conflicts_in_cache(&sym.not_symlinked);

        println!("Not Symlinked:");
        for group in &not_symlinked {
            println!("\t{}", group.red());
            print_conflicts(&file_conflicts, group, "already exists");
            print_conflicts(&sym.not_owned, group, "symlinks elsewhere");
        }

        println!();
    }

    if !symlinked.is_empty() {
        println!("Symlinked:");
        for group in symlinked {
            println!("\t{}", group.green());
        }
        println!();
    }

    if !unsupported.is_empty() {
        println!("Not supported on this platform:");
        for group in unsupported {
            println!("\t{}", group.yellow());
        }
        println!();
    }

    let invalid_groups = dotfiles::check_invalid_groups(DotfileType::Configs, &groups);
    if let Some(invalid_groups) = &invalid_groups {
        eprintln!("Following groups do not exist:");
        for group in invalid_groups {
            eprintln!("\t{}", group.red());
        }
        println!();
    }

    if !not_symlinked.is_empty() {
        println!("Check `tuckr help add` to learn how to fix symlinks.");
    }

    if invalid_groups.is_none() {
        return Err(ReturnCode::NoSetupFolder.into());
    }

    Ok(())
}

/// Prints symlinking status
pub fn status_cmd(groups: Option<Vec<String>>) -> Result<(), ExitCode> {
    let sym = SymlinkHandler::try_new()?;
    match groups {
        Some(groups) => print_groups_status(&sym, groups)?,
        None => print_global_status(&sym)?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::dotfiles;
    use dotfiles::Dotfile;
    use std::fs::{self, File};
    use std::path;

    fn start_symlink_cache() -> (super::SymlinkHandler, Dotfile) {
        todo!()
    }

    fn cleanup_symlink_cache() {
        todo!()
    }

    #[test]
    fn add_symlink() {
        todo!()
    }

    #[test]
    fn remove_symlink() {
        todo!()
    }
}
