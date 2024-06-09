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

use crate::utils::{self, DotfileGroup, DotfileType, ReturnCode};
use owo_colors::OwoColorize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use tabled::{Table, Tabled};

fn symlink_file(f: PathBuf) {
    let src_path = f;
    match DotfileGroup::from(src_path.clone()) {
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
                return Err(ReturnCode::CouldntFindDotfiles.into());
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
            return Err(ReturnCode::NoSetupFolder.into());
        };

        for file in dir {
            let group = DotfileGroup::from(file.unwrap().path()).unwrap();
            // Ignores all regular files since Configs should only care about group folders
            if group.path.is_file() {
                continue;
            }

            group.map(|f| {
                let config_file_group = DotfileGroup::from(f).unwrap();
                let home_config_file = config_file_group.to_target_path();

                match fs::read_link(&home_config_file) {
                    Ok(symlink_target) => {
                        // group_dir can only be in one set at a time
                        // this makes it so one would get a not symlinked status
                        // if at least one of the files is not symlinked
                        if symlink_target == config_file_group.path {
                            self.not_symlinked.remove(&group);
                            self.symlinked.insert(group.clone());
                        } else {
                            self.symlinked.remove(&group);
                            self.not_symlinked.insert(group.clone());
                        }
                    }

                    // add file to not_owned if it's in conflict with dotfiles
                    Err(_) => {
                        self.symlinked.remove(&group);
                        self.not_symlinked.insert(group.clone());

                        if home_config_file.exists() {
                            if let Some(group) = self.not_owned.get_mut(&group.name) {
                                group.push(config_file_group.path);
                            } else {
                                self.not_owned
                                    .insert(group.name.clone(), vec![config_file_group.path]);
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
    fn get_related_conditional_groups(
        &self,
        target_group: &str,
        symlinked: bool,
    ) -> Vec<&DotfileGroup> {
        if symlinked {
            self.symlinked
                .iter()
                .filter(|group| {
                    let filename = group.path.file_name().unwrap().to_str().unwrap();
                    filename.starts_with(target_group) && group.is_valid_target()
                })
                .collect()
        } else {
            self.not_symlinked
                .iter()
                .filter(|group| {
                    let filename = group.path.file_name().unwrap().to_str().unwrap();
                    filename.starts_with(target_group) && group.is_valid_target()
                })
                .collect()
        }
    }

    /// Symlinks all the files of a group to the user's $HOME
    fn add(&self, group: &str) {
        let groups = self.get_related_conditional_groups(group, false);

        for group in groups {
            let group =
                DotfileGroup::from(self.dotfiles_dir.join("Configs").join(&group.name)).unwrap();
            if group.path.exists() {
                // iterate through all the files in group_dir
                group.map(symlink_file);
            } else {
                eprintln!("{} {}", "There's no dotfiles for".red(), group.name.red());
            }
        }
    }

    /// Deletes symlinks from $HOME if they're owned by dotfiles dir
    fn remove(&self, group: &str) {
        let remove_symlink = |file: PathBuf| {
            let dotfile = DotfileGroup::from(file).unwrap().to_target_path();
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
            let group =
                DotfileGroup::from(self.dotfiles_dir.join("Configs").join(&group.name)).unwrap();
            if group.path.exists() {
                // iterate through all the files in group_dir
                group.map(remove_symlink);
            } else {
                eprintln!("{} {}", "There's no group called".red(), group.name.red());
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
    if let Some(invalid_groups) = utils::check_invalid_groups(DotfileType::Configs, groups) {
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

        for group in symgroups {
            // Takes the name of the group to be passed the function
            // Ignore groups in the excludes array
            if exclude.contains(&group.name) {
                continue;
            }
            // do something with the group name
            // passing the sym context
            func(&sym, &group.name);
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

    foreach_group(groups, exclude, true, |sym, group| {
        if !sym.not_owned.is_empty() {
            // Symlink dotfile by force
            if force {
                for (group, group_files) in &sym.not_owned {
                    if !groups.contains(group) {
                        continue;
                    }

                    for file in group_files {
                        let file = DotfileGroup::from(file.clone()).unwrap();
                        let target_file = file.to_target_path();
                        if target_file.is_dir() {
                            fs::remove_dir_all(target_file).unwrap();
                        } else if target_file.is_file() {
                            fs::remove_file(target_file).unwrap();
                        }
                    }
                }
            }

            // Discard dotfile and adopt the conflicting dotfile
            if adopt {
                for (group, group_files) in &sym.not_owned {
                    if !groups.contains(group) {
                        continue;
                    }

                    for file in group_files {
                        let file = DotfileGroup::from(file.clone()).unwrap();
                        let target_file = file.to_target_path();
                        if target_file.is_dir() {
                            fs::remove_dir_all(&file.path).unwrap();
                        } else if target_file.is_file() {
                            fs::remove_file(&file.path).unwrap();
                        }

                        fs::rename(target_file, file.path).unwrap();
                    }
                }
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
                let group = group.path.file_name().unwrap().to_str().unwrap();
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
                    $symgroup.push(&group.name);
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
            .map(|group| DotfileGroup::from(sym.dotfiles_dir.join("Configs").join(group)).unwrap())
            .filter(|group| !group.is_valid_target())
            .map(|group| group.name.to_owned())
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
                    let conflict = DotfileGroup::from(conflict.to_owned())
                        .unwrap()
                        .to_target_path();
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

    let invalid_groups = utils::check_invalid_groups(DotfileType::Configs, &groups);
    if let Some(invalid_groups) = &invalid_groups {
        eprintln!("{}", "Following groups do not exist:".red());
        for group in invalid_groups {
            eprintln!("\t{}", group.red());
        }
    }

    if !not_symlinked.is_empty() {
        println!("\nCheck `tuckr help add` to learn how to fix symlinks.");
    }

    if let None = invalid_groups {
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

        (sym, DotfileGroup::from(group_dir).unwrap())
    }

    #[test]
    fn add_symlink() {
        let init = init_symlink_test();
        let sym = init.0;
        let group = init.1;

        sym.add(group.name.as_str());

        let file = DotfileGroup::from(group.path.join("group.test")).unwrap();
        assert!(match fs::read_link(file.to_target_path()) {
            Ok(link) => link == file.path,
            Err(_) => false,
        });

        let config_file =
            DotfileGroup::from(group.path.clone().join(".config").join("group.test")).unwrap();
        assert!(match fs::read_link(config_file.to_target_path()) {
            Ok(link) => link == config_file.path,
            Err(_) => false,
        });

        sym.remove(group.name.as_str());
    }

    #[test]
    fn remove_symlink() {
        let init = init_symlink_test();
        let sym = init.0;
        let group = init.1;

        sym.add(group.name.as_str());
        sym.remove(group.name.as_str());

        let file = DotfileGroup::from(group.path.join("group.test")).unwrap();
        assert!(match fs::read_link(file.to_target_path()) {
            Err(_) => true,
            Ok(link) => link != file.path,
        });

        let config_file =
            DotfileGroup::from(group.path.clone().join(".config").join("group.test")).unwrap();
        assert!(match fs::read_link(config_file.to_target_path()) {
            Err(_) => true,
            Ok(link) => link != config_file.path,
        });

        _ = fs::remove_dir(group.path.as_path());
    }
}
