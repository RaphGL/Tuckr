//! Manages dotfile symlinking
//!
//! Dotfiles are managed by the SymlinkHandler, its fields contain the following runtime information:
//! - dotfiles_dir: the location of the dotfiles directory
//! - symlinked: all files that have been symlinked
//! - not_symlinked: files that haven't been symlinked yet
//! - not_owned: files that have the same name on dotfiles/Configs but that do not belong to us,
//! therefore they are in conflict
//!
//! This information is retrieved by walking through dotfiles/Configs and checking whether their
//! $HOME equivalents are pointing to them and categorizing them accordingly.

use crate::utils::{self, DotfileGroup};
use owo_colors::OwoColorize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use tabled::{Table, Tabled};

fn symlink_file(f: fs::DirEntry) {
    let target_path = DotfileGroup::from(f.path()).to_home_path();

    #[cfg(target_family = "unix")]
    {
        _ = std::os::unix::fs::symlink(f.path(), target_path);
    }

    #[cfg(target_family = "windows")]
    {
        _ = std::os::windows::fs::symlink_file(f.path(), target_path);
    }
}

/// Handles dotfile symlinking and their current status
struct SymlinkHandler {
    dotfiles_dir: PathBuf,                    // path to the dotfiles directory
    symlinked: HashSet<DotfileGroup>,         // path to symlinked groups in Dotfiles/Configs
    not_symlinked: HashSet<DotfileGroup>,     // path to groups that aren't symlinked to $HOME
    not_owned: HashMap<String, Vec<PathBuf>>, // key: group the file belongs to, value: list of conflicting files from that group
}

impl SymlinkHandler {
    /// Initializes SymlinkHandler and fills it dotfiles' status information
    fn try_new() -> Result<Self, ExitCode> {
        let dotfiles_dir = match utils::get_dotfiles_path() {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("{e}");
                return Err(ExitCode::from(utils::COULDNT_FIND_DOTFILES));
            }
        };

        let symlinker = SymlinkHandler {
            dotfiles_dir,
            symlinked: HashSet::new(),
            not_symlinked: HashSet::new(),
            not_owned: HashMap::new(),
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
        // Opens and loops through each of Dotfiles/Configs' dotfiles
        let Ok(dir) = fs::read_dir(self.dotfiles_dir.join("Configs")) else {
                eprintln!("{}", "There's no Configs folder set up".red());
                return Err(ExitCode::from(utils::NO_SETUP_FOLDER));
        };

        for file in dir {
            let group_dir = DotfileGroup::from(file.unwrap().path());
            // Ignores all regular files since Configs should only care about group folders
            if group_dir.is_file() {
                continue;
            }

            // Checks for the files in each of the groups' dirs
            group_dir.map(|f| {
                let config_file = DotfileGroup::from(f.path());
                let home_config_file = config_file.to_home_path();

                match fs::read_link(&home_config_file) {
                    Ok(f) => {
                        let dotfiles_configs_path = PathBuf::from("dotfiles").join("Configs");
                        let dotfiles_configs_path = DotfileGroup::from(dotfiles_configs_path);

                        // group_dir can only be in one set at a time
                        // this makes it so one would get a not symlinked status
                        // if at least one of the files is not symlinked
                        if f.to_str()
                            .unwrap()
                            .contains(dotfiles_configs_path.to_str().unwrap())
                        {
                            self.symlinked.insert(group_dir.clone());
                            self.not_symlinked.remove(&group_dir);
                        } else {
                            self.not_symlinked.insert(group_dir.clone());
                            self.symlinked.remove(&group_dir);
                        }
                    }

                    // file is in conflict with dotfiles and is added to not_owned
                    Err(_) => {
                        self.not_symlinked.insert(group_dir.clone());
                        self.symlinked.remove(&group_dir);

                        if home_config_file.exists() {
                            let group_dir =
                                group_dir.file_name().unwrap().to_str().unwrap().to_string();

                            if let Some(group) = self.not_owned.get_mut(&group_dir) {
                                group.push(config_file.to_path_buf());
                            } else {
                                self.not_owned
                                    .insert(group_dir, vec![config_file.to_path_buf()]);
                            }
                        }
                    }
                }
            });
        }

        Ok(self)
    }

    /// Returns all conditional groups with the same name that satify the current user's platform
    ///
    /// symlinked: gets symlinked conditional groupsif true, otherwise gets not symlinked ones
    fn get_related_conditional_groups(&self, group: &str, symlinked: bool) -> Vec<&DotfileGroup> {
        if symlinked {
            self.symlinked
                .iter()
                .filter(|f| {
                    let filename = f.file_name().unwrap().to_str().unwrap();
                    filename.starts_with(group) && f.is_valid_target()
                })
                .collect()
        } else {
            self.not_symlinked
                .iter()
                .filter(|f| {
                    let filename = f.file_name().unwrap().to_str().unwrap();
                    filename.starts_with(group) && f.is_valid_target()
                })
                .collect()
        }
    }

    /// Symlinks all the files of a group to the user's $HOME
    fn add(&self, group: &str) {
        let groups = self.get_related_conditional_groups(group, false);

        for group in groups {
            let group_name = group.to_group_name().unwrap();
            let group_dir = DotfileGroup::from(self.dotfiles_dir.join("Configs").join(group_name));
            if group_dir.exists() {
                // iterate through all the files in group_dir
                group_dir.map(symlink_file);
            } else {
                eprintln!("{} {}", "There's no dotfiles for".red(), group_name.red());
            }
        }
    }

    /// Deletes symlinks from $HOME if they're owned by dotfiles dir
    fn remove(&self, group: &str) {
        let remove_symlink = |file: fs::DirEntry| {
            let dotfile = DotfileGroup::from(file.path()).to_home_path();
            if let Ok(linked) = fs::read_link(&dotfile) {
                let dotfiles_configs_path = PathBuf::from("dotfiles").join("Configs");
                let dotfiles_configs_path = dotfiles_configs_path.to_str().unwrap();
                if linked.to_str().unwrap().contains(dotfiles_configs_path) {
                    fs::remove_file(dotfile).unwrap();
                }
            }
        };

        let groups = self.get_related_conditional_groups(group, true);

        for group in groups {
            let group_name = group.to_group_name().unwrap();
            let group_dir = DotfileGroup::from(self.dotfiles_dir.join("Configs").join(group_name));
            if group_dir.exists() {
                // iterate through all the files in group_dir
                group_dir.map(remove_symlink);
            } else {
                eprintln!("{} {}", "There's no group called".red(), group_name.red());
            }
        }
    }
}

/// groups: the groups will be applied to
///
/// exclude: the groups that will be ignored
///
/// symlinked: whether it should be applied to symlinked or non symlinked groups
/// iterates over each group in the dotfiles and calls a function F giving it the SymlinkHandler
/// instance and the name of the group that's being handled
///
/// This abstracts this recurrent loop allowing handle groups just by their names
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

    // handles wildcard
    if groups.contains(&"*".to_string()) {
        let symgroups = if symlinked {
            &sym.not_symlinked
        } else {
            &sym.symlinked
        };

        for group_path in symgroups {
            // Takes the name of the group to be passed the function
            let group_name = group_path.to_group_name().unwrap();
            // Ignore groups in the excludes array
            if exclude.contains(&group_name.to_string()) {
                continue;
            }
            // do something with the group name
            // passing the sym context
            func(&sym, &group_name.to_string());
        }

        return Ok(());
    }

    for group in groups {
        if exclude.contains(group) {
            continue;
        } else {
            func(&sym, group);
        }
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
    if force || adopt {
        if force {
            print!("Are you sure you want to override conflicts? (N/y) ");
        } else if adopt {
            print!("Are you sure you want to adopt conflicts? (N/y) ");
        }

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

    /// Iterates through every file in group and lets the consumer compare it to the not owned file
    /// and decide how to handle it
    fn handle_conflicting_files(
        sym: &SymlinkHandler,
        group: &str,
        handle_conflict: impl Fn(fs::DirEntry, &PathBuf),
    ) {
        for files in sym.not_owned.values() {
            for file in files {
                // removing everything from sym.not_owned makes so sym.add() doesn't ignore those
                // files thus forcing them to be symlinked
                for group in sym.get_related_conditional_groups(group, false) {
                    let group_name = group.to_group_name().unwrap();
                    let group_path =
                        DotfileGroup::from(sym.dotfiles_dir.join("Configs").join(group_name));
                    group_path.map(|group_file| handle_conflict(group_file, file));
                }
            }
        }
    }

    foreach_group(groups, exclude, true, |sym, group| {
        if !sym.not_owned.is_empty() {
            // Symlink dotfile by force
            if force {
                handle_conflicting_files(sym, group, |group_file, file| {
                    let group_file = DotfileGroup::from(group_file.path());
                    if &group_file.to_home_path() == file {
                        if file.is_dir() {
                            _ = fs::remove_dir_all(file);
                        } else {
                            _ = fs::remove_file(file);
                        }
                    }
                })
            }

            // Discard dotfile and adopt the conflicting dotfile
            if adopt {
                handle_conflicting_files(sym, group, |group_file, file| {
                    // only adopts dotfile if it matches requested group
                    let group_file = DotfileGroup::from(group_file.path());
                    if &group_file.to_home_path() == file {
                        if group_file.is_dir() {
                            _ = fs::remove_dir(group_file.as_path());
                        } else {
                            _ = fs::remove_file(group_file.as_path());
                        }
                        _ = fs::rename(file, group_file.as_path());
                    }
                })
            }
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

macro_rules! get_valid_groups {
    ($symlinks:expr) => {{
        let mut group = $symlinks
            .iter()
            .filter(|group| group.is_valid_target())
            .map(|group| {
                let group = group.file_name().unwrap().to_str().unwrap();
                // strips _windows, _linux, etc from the group name
                match group.split_once('_') {
                    Some(no_cond_group) => no_cond_group.0,
                    None => group,
                }
            })
            .collect::<Vec<_>>();
        group.sort();
        group
    }};
}

fn print_global_status(sym: &SymlinkHandler) -> Result<(), ExitCode> {
    #[derive(Tabled)]
    struct SymlinkRow<'a> {
        #[tabled(rename = "Symlinked")]
        symlinked: &'a str,

        #[tabled(rename = "Not Symlinked")]
        not_symlinked: &'a str,
    }

    let not_symlinked = {
        let mut not_symlinked = get_valid_groups!(sym.not_symlinked);
        not_symlinked.dedup();
        not_symlinked
    };

    let symlinked = {
        let mut symlinked = get_valid_groups!(sym.symlinked);
        symlinked = symlinked
            .iter()
            .filter(|group| !not_symlinked.contains(group))
            .map(|group| group.to_owned())
            .collect();
        symlinked.dedup();
        symlinked
    };

    let not_owned = {
        let mut not_owned = sym
            .not_owned
            .keys()
            .map(|group| {
                // strips _windows, _linux, etc from the group name
                match group.split_once('_') {
                    Some(no_cond_group) => no_cond_group.0,
                    None => group,
                }
            })
            .collect::<Vec<_>>();
        not_owned.sort();
        not_owned.dedup();
        not_owned
    };

    // pads the smallest vector and zips it into a vec with (Symlinked, NotSymlinked) values
    let status = if symlinked.len() > not_symlinked.len() {
        symlinked
            .iter()
            .zip(not_symlinked.iter().chain(std::iter::repeat(&"")))
            .map(|group| SymlinkRow {
                symlinked: group.0,
                not_symlinked: group.1,
            })
            .collect::<Vec<_>>()
    } else {
        symlinked
            .iter()
            .chain(std::iter::repeat(&""))
            .zip(not_symlinked.iter())
            .map(|group| SymlinkRow {
                symlinked: group.0,
                not_symlinked: group.1,
            })
            .collect::<Vec<_>>()
    };

    // --- Creates all the tables and prints them ---
    use tabled::{
        col, format::Format, object::Columns, object::Rows, Alignment, Margin, Modify, Style,
    };

    let mut sym_table = Table::new(status);
    sym_table
        .with(Style::rounded())
        .with(Margin::new(4, 4, 1, 1))
        .with(Modify::new(Rows::first()).with(Format::new(|s| s.default_color().to_string())))
        .with(Modify::new(Columns::single(0)).with(Format::new(|s| s.green().to_string())))
        .with(Modify::new(Columns::single(1)).with(Format::new(|s| s.red().to_string())));

    let mut conflict_table = Table::builder(&not_owned)
        .set_columns(["Conflicting Dotfiles".yellow().to_string()])
        .clone()
        .build();
    conflict_table
        .with(Style::empty())
        .with(Alignment::center());

    // Creates a table with sym_table and conflict_table
    let mut final_table = if not_owned.is_empty() {
        col![sym_table]
    } else {
        col![sym_table, conflict_table]
    };

    final_table.with(Style::empty()).with(Alignment::center());
    println!("{final_table}");

    if !not_owned.is_empty() {
        println!("\nTo learn more about conflicting dotfiles run: `tuckr status <group...>`\n")
    }

    // Determines exit code for the command based on the dotfiles' status
    if !sym.symlinked.is_empty() && sym.not_symlinked.is_empty() && not_owned.is_empty() {
        Ok(())
    } else {
        Err(ExitCode::FAILURE)
    }
}

fn print_groups_status(sym: &SymlinkHandler, groups: Vec<String>) -> Result<(), ExitCode> {
    macro_rules! add_related_groups {
        ($symgroup:ident, $symlinked:literal) => {
            for i in 0..$symgroup.len() {
                let group = $symgroup[i];
                for group in sym.get_related_conditional_groups(group, $symlinked) {
                    $symgroup.push(group.to_group_name().unwrap());
                }
            }

            $symgroup.sort();
            $symgroup.dedup();
        };
    }

    let not_symlinked = {
        let not_symlinked = get_valid_groups!(sym.not_symlinked);

        let mut not_symlinked = not_symlinked
            .iter()
            .map(|group| group.to_owned())
            .filter(|group| groups.contains(&group.to_string()))
            .collect::<Vec<_>>();

        add_related_groups!(not_symlinked, false);

        not_symlinked
    };

    let symlinked = {
        let symlinked = get_valid_groups!(sym.symlinked);

        let mut symlinked = symlinked
            .iter()
            .map(|group| group.to_owned())
            .filter(|group| !not_symlinked.contains(group) && groups.contains(&group.to_string()))
            .collect::<Vec<_>>();

        add_related_groups!(symlinked, true);

        symlinked
    };

    let unsupported = {
        let mut unsupported = groups
            .iter()
            .map(|group| DotfileGroup::from(sym.dotfiles_dir.join("Configs").join(group)))
            .filter(|group| !group.is_valid_target())
            .map(|group| group.to_group_name().unwrap().to_owned())
            .collect::<Vec<_>>();

        unsupported.sort();
        unsupported.dedup();
        unsupported
    };

    if !not_symlinked.is_empty() {
        println!("Not Symlinked:");
        for group in &not_symlinked {
            println!("\t{}", group.red());
        }
        println!();

        for group in &not_symlinked {
            if let Some(conflicts) = sym.not_owned.get(&group.to_string()) {
                println!("Conflicting files:");
                for conflict in conflicts {
                    let conflict = DotfileGroup::from(conflict.to_owned()).to_home_path();
                    println!("\t{} -> {}", group.yellow(), conflict.to_str().unwrap());
                }
                println!();
            }
        }
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

    if !not_symlinked.is_empty() {
        println!("Check `tuckr help add` to learn how to resolve them.");
        return Err(ExitCode::FAILURE);
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
    use crate::utils;
    use std::fs::{self, File};
    use std::path;
    use utils::DotfileGroup;

    /// makes sure that symlink status is loaded on startup
    #[test]
    fn new_symlink_handler() {
        let dotfiles_dir = path::PathBuf::from(utils::get_dotfiles_path().unwrap());
        let dirs = fs::read_dir(dotfiles_dir.join("Configs"));

        if dirs.is_err() {
            panic!("{:#?}", dirs);
        } else {
            let sym = super::SymlinkHandler::try_new().unwrap();
            assert!(
                if !sym.symlinked.is_empty() || !sym.not_symlinked.is_empty() {
                    true
                } else {
                    false
                }
            );
        }
    }

    /// Initializes symlink test by creating a SymlinkHandler and a mockup dotfiles directory
    fn init_symlink_test() -> (super::SymlinkHandler, DotfileGroup) {
        let sym = super::SymlinkHandler::try_new().unwrap();
        let group_dir = sym.dotfiles_dir.join("Configs").join("group");
        if let Err(_) = fs::create_dir_all(group_dir.clone().join(".config")) {
            panic!("Could not create required folders");
        }

        File::create(group_dir.join("group.test")).unwrap();
        File::create(group_dir.join(".config").join("group.test")).unwrap();

        let sym = sym.validate().unwrap();

        (sym, DotfileGroup::from(group_dir))
    }

    #[test]
    fn add_symlink() {
        let init = init_symlink_test();
        let sym = init.0;
        let group_dir = init.1;
        let group_name = group_dir.to_group_name().unwrap();

        sym.add(group_name);

        let file = DotfileGroup::from(group_dir.join("group.test"));
        assert!(match fs::read_link(file.to_home_path()) {
            Ok(link) => link == *file,
            Err(_) => false,
        });

        let config_file = DotfileGroup::from(group_dir.clone().join(".config").join("group.test"));
        assert!(match fs::read_link(config_file.to_home_path()) {
            Ok(link) => link == *config_file,
            Err(_) => false,
        });

        sym.remove(group_name);
    }

    #[test]
    fn remove_symlink() {
        let init = init_symlink_test();
        let sym = init.0;
        let group_dir = init.1;

        let group_name = group_dir.to_group_name().unwrap();
        sym.add(group_name);
        sym.remove(group_name);

        let file = DotfileGroup::from(group_dir.join("group.test"));
        assert!(match fs::read_link(file.to_home_path()) {
            Err(_) => true,
            Ok(link) => link != *file,
        });

        let config_file = DotfileGroup::from(group_dir.clone().join(".config").join("group.test"));
        assert!(match fs::read_link(config_file.to_home_path()) {
            Err(_) => true,
            Ok(link) => link != *config_file,
        });
    }
}
