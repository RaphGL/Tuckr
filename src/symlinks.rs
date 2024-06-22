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

use crate::utils::{self, Dotfile, DotfileType, ReturnCode};
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use tabled::{Table, Tabled};

fn symlink_file(f: PathBuf) {
    let src_path = f;
    match Dotfile::from(src_path.clone()) {
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
    dotfiles_dir: PathBuf,                        // path to the dotfiles directory
    symlinked: HashMap<String, Vec<Dotfile>>,     // path to symlinked groups in Dotfiles/Configs
    not_symlinked: HashMap<String, Vec<Dotfile>>, // path to groups that aren't symlinked to $HOME
    not_owned: HashMap<String, Vec<Dotfile>>, // key: group the file belongs to, value: list of conflicting files from that group
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
            symlinked: HashMap::new(),
            not_symlinked: HashMap::new(),
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
        let configs_dir = Dotfile::from(self.dotfiles_dir.join("Configs")).unwrap();

        let mut symlinked: HashMap<String, Vec<Dotfile>> = HashMap::new();
        let mut not_symlinked: HashMap<String, Vec<Dotfile>> = HashMap::new();
        let mut not_owned: HashMap<String, Vec<Dotfile>> = HashMap::new();

        configs_dir.map(|f| {
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
                    group.push(f);
                } else {
                    not_owned.entry(f.group_name.clone()).or_default();

                    let group = not_owned.get_mut(&f.group_name).unwrap();
                    group.push(f);
                }
            } else {
                not_symlinked.entry(f.group_name.clone()).or_default();

                let group = not_symlinked.get_mut(&f.group_name).unwrap();
                group.push(f);
            }
        });

        let remove_empty_groups = move |mut group_type: HashMap<String, Vec<Dotfile>>| {
            group_type
                .iter_mut()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .filter(|(_, v)| !v.is_empty())
                .collect()
        };

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
        .filter(|(group, files)| group.starts_with(target_group) && files[0].is_valid_target())
        .map(|(group, _)| group.clone())
        .collect()
    }

    /// Symlinks all the files of a group to the user's $HOME
    fn add(&self, group: &str) {
        let groups = self.get_related_conditional_groups(group, false);

        for group in groups {
            let group = Dotfile::from(self.dotfiles_dir.join("Configs").join(&group)).unwrap();
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
        let remove_symlink = |file: PathBuf| {
            let dotfile = Dotfile::from(file).unwrap().to_target_path();
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
            let group = Dotfile::from(self.dotfiles_dir.join("Configs").join(&group)).unwrap();
            if group.path.exists() {
                // iterate through all the files in group_dir
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

        for (group, _) in symgroups {
            // Takes the name of the group to be passed the function
            // Ignore groups in the excludes array
            if exclude.contains(&group) {
                continue;
            }
            // do something with the group name
            // passing the sym context
            func(&sym, &group);
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
                        let target_file = file.to_target_path();
                        if target_file.is_dir() {
                            fs::remove_dir_all(&file.path).unwrap();
                        } else if target_file.is_file() {
                            fs::remove_file(&file.path).unwrap();
                        }

                        fs::rename(target_file, &file.path).unwrap();
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

fn print_global_status(sym: &SymlinkHandler) -> Result<(), ExitCode> {
    #[derive(Tabled, Debug)]
    struct SymlinkRow<'a> {
        #[tabled(rename = "Symlinked")]
        symlinked: &'a str,

        #[tabled(rename = "Not Symlinked")]
        not_symlinked: &'a str,
    }

    let not_symlinked: Vec<_> = sym.not_symlinked.keys().collect();
    let symlinked: Vec<_> = sym
        .symlinked
        .keys()
        .filter(|group| !not_symlinked.contains(group))
        .collect();

    println!(
        "symlinked: {}\nnot_symlinked: {}",
        symlinked.len(),
        not_symlinked.len()
    );

    println!("{:?}", sym.symlinked.get("alacritty").unwrap());

    let empty_str = String::from("");
    let status: Vec<SymlinkRow> = {
        use std::cmp::Ordering;
        let (longest, shortest, symlinked_is_longest) = {
            match symlinked.len().cmp(&not_symlinked.len()) {
                Ordering::Greater | Ordering::Equal => (&symlinked, &not_symlinked, true),
                Ordering::Less => (&not_symlinked, &symlinked, false),
            }
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

    let not_owned: Vec<&String> = sym.not_owned.keys().collect();

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
    if !symlinked.is_empty() && not_symlinked.is_empty() && not_owned.is_empty() {
        Ok(())
    } else {
        Err(ExitCode::FAILURE)
    }
}

fn print_groups_status(sym: &SymlinkHandler, groups: Vec<String>) -> Result<(), ExitCode> {
    macro_rules! add_related_groups {
        ($symgroup:ident, $symlinked:literal) => {
            for i in 0..$symgroup.len() {
                let group = &$symgroup[i];
                for group in sym.get_related_conditional_groups(group.as_str(), $symlinked) {
                    $symgroup.push(group);
                }
            }

            $symgroup.sort();
            $symgroup.dedup();
        };
    }

    let not_symlinked = {
        let not_symlinked: Vec<_> = sym.not_symlinked.keys().collect();

        let mut not_symlinked: Vec<_> = not_symlinked
            .iter()
            .map(|group| group.to_string())
            .filter(|group| groups.contains(&group.to_string()))
            .collect();

        add_related_groups!(not_symlinked, false);

        not_symlinked
    };

    let symlinked = {
        let symlinked: Vec<_> = sym.symlinked.keys().collect();

        let mut symlinked = symlinked
            .iter()
            .map(|group| group.to_string())
            .filter(|group| !not_symlinked.contains(group) && groups.contains(&group.to_string()))
            .collect::<Vec<_>>();

        add_related_groups!(symlinked, true);

        symlinked
    };

    let unsupported = {
        let mut unsupported = groups
            .iter()
            .map(|group| Dotfile::from(sym.dotfiles_dir.join("Configs").join(group)).unwrap())
            .filter(|group| !group.is_valid_target())
            .map(|group| group.group_name)
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
                    let conflict = conflict.to_target_path();
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
        println!();
    }

    if !not_symlinked.is_empty() {
        println!("Check `tuckr help add` to learn how to fix symlinks.");
    }

    if let None = invalid_groups {
        return Err(ReturnCode::NoSetupFolder.into());
    }

    Ok(())
}

/// Prints symlinking status
/// todo: make status work with new symlink traversal method
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
    use utils::Dotfile;

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
    fn init_symlink_test() -> (super::SymlinkHandler, Dotfile) {
        let sym = super::SymlinkHandler::try_new().unwrap();
        let group_dir = sym.dotfiles_dir.join("Configs").join("group");
        if let Err(_) = fs::create_dir_all(group_dir.clone().join(".config")) {
            panic!("Could not create required folders");
        }

        File::create(group_dir.join("group.test")).unwrap();
        File::create(group_dir.join(".config").join("group.test")).unwrap();

        let sym = sym.validate().unwrap();

        (sym, Dotfile::from(group_dir).unwrap())
    }

    #[test]
    fn add_symlink() {
        let init = init_symlink_test();
        let sym = init.0;
        let group = init.1;

        sym.add(group.group_name.as_str());

        let file = Dotfile::from(group.path.join("group.test")).unwrap();
        assert!(match fs::read_link(file.to_target_path()) {
            Ok(link) => link == file.path,
            Err(_) => false,
        });

        let config_file =
            Dotfile::from(group.path.clone().join(".config").join("group.test")).unwrap();
        assert!(match fs::read_link(config_file.to_target_path()) {
            Ok(link) => link == config_file.path,
            Err(_) => false,
        });

        sym.remove(group.group_name.as_str());
    }

    #[test]
    fn remove_symlink() {
        let init = init_symlink_test();
        let sym = init.0;
        let group = init.1;

        sym.add(group.group_name.as_str());
        sym.remove(group.group_name.as_str());

        let file = Dotfile::from(group.path.join("group.test")).unwrap();
        assert!(match fs::read_link(file.to_target_path()) {
            Err(_) => true,
            Ok(link) => link != file.path,
        });

        let config_file =
            Dotfile::from(group.path.clone().join(".config").join("group.test")).unwrap();
        assert!(match fs::read_link(config_file.to_target_path()) {
            Err(_) => true,
            Ok(link) => link != config_file.path,
        });

        _ = fs::remove_dir(group.path.as_path());
    }
}
