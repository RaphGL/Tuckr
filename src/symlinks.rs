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

use crate::utils;
use owo_colors::OwoColorize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use tabled::{Table, Tabled};

#[cfg(target_family = "unix")]
fn symlink_file(f: fs::DirEntry) {
    let target_path = utils::to_home_path(f.path().to_str().unwrap());

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
    symlinked: HashSet<PathBuf>,              // path to symlinked groups in Dotfiles/Configs
    not_symlinked: HashSet<PathBuf>,          // path to groups that aren't symlinked to $HOME
    not_owned: HashMap<String, Vec<PathBuf>>, // key: group the file belongs to, value: list of conflicting files from that group
}

impl SymlinkHandler {
    /// Initializes SymlinkHandler and fills it dotfiles' status information
    fn try_new() -> Result<Self, ExitCode> {
        let dotfiles_dir = match utils::get_dotfiles_path() {
            Some(dir) => dir,
            None => {
                eprintln!(
                    "{}",
                    "Could not find dotfiles, make sure it's in the right path".red()
                );
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
        let dir = match fs::read_dir(self.dotfiles_dir.join("Configs")) {
            Ok(dir) => dir,
            Err(_) => {
                eprintln!("{}", "There's no Configs folder set up".red());
                return Err(ExitCode::from(utils::NO_SETUP_FOLDER));
            }
        };

        for file in dir {
            let group_dir = file.unwrap();
            // Ignores all regular files since Configs should only care about group folders
            if group_dir.path().is_file() {
                continue;
            }

            // Checks for the files in each of the groups' dirs
            utils::group_dir_map(group_dir.path(), |f| {
                let config_file = utils::to_home_path(f.path().to_str().unwrap());

                match fs::read_link(&config_file) {
                    Ok(f) => {
                        let dotfiles_configs_path = PathBuf::from("dotfiles").join("Configs");
                        let dotfiles_configs_path = dotfiles_configs_path.to_str().unwrap();

                        // group_dir can only be in one set at a time
                        // this makes it so one would get a not symlinked status
                        // if at least one of the files is not symlinked
                        if f.to_str().unwrap().contains(dotfiles_configs_path) {
                            self.symlinked.insert(group_dir.path());
                            self.not_symlinked.remove(&group_dir.path());
                        } else {
                            self.not_symlinked.insert(group_dir.path());
                            self.symlinked.remove(&group_dir.path());
                        }
                    }

                    // file is in conflict with dotfiles and is added to not_owned
                    Err(_) => {
                        self.not_symlinked.insert(group_dir.path());
                        self.symlinked.remove(&group_dir.path());
                        if PathBuf::from(&config_file).exists() {
                            let group_dir = group_dir.file_name().to_str().unwrap().to_string();
                            if let Some(group) = self.not_owned.get_mut(&group_dir) {
                                group.push(config_file);
                            } else {
                                self.not_owned.insert(group_dir, vec![config_file]);
                            }
                        }
                    }
                }
            });
        }

        Ok(self)
    }

    /// Symlinks all the files of a group to the user's $HOME
    fn add(&self, group: &str) {
        let group_dir = self.dotfiles_dir.join("Configs").join(group);
        if group_dir.exists() {
            // iterate through all the files in group_dir
            utils::group_dir_map(group_dir, symlink_file);
        } else {
            eprintln!("{} {}", "There's no dotfiles for".red(), group.red());
        }
    }

    /// Deletes symlinks from $HOME if they're owned by dotfiles dir
    fn remove(&self, group: &str) {
        let remove_symlink = |file: fs::DirEntry| {
            let dotfile = utils::to_home_path(file.path().to_str().unwrap());
            if let Ok(linked) = fs::read_link(&dotfile) {
                let dotfiles_configs_path = PathBuf::from("dotfiles").join("Configs");
                let dotfiles_configs_path = dotfiles_configs_path.to_str().unwrap();
                if linked.to_str().unwrap().contains(dotfiles_configs_path) {
                    fs::remove_file(dotfile).unwrap();
                }
            }
        };

        let group_dir = self.dotfiles_dir.join("Configs").join(group);
        if group_dir.exists() {
            // iterate through all the files in group_dir
            utils::group_dir_map(group_dir, remove_symlink);
        } else {
            eprintln!("{} {}", "There's no group called".red(), group.red());
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
        let symgroup = if symlinked {
            &sym.not_symlinked
        } else {
            &sym.symlinked
        };

        for p in symgroup {
            // Takes the name of the group to be passed the function
            let group_name = utils::to_group_name(p.to_str().unwrap()).unwrap();
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
        // add all groups if wildcard
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
    if force {
        let mut answer = String::new();
        print!("Are you sure you want to override conflicts? (N/y) ");
        std::io::stdout()
            .flush()
            .expect("Could not print to stdout");
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
                for files in sym.not_owned.values() {
                    for file in files {
                        // removing everything from sym.not_owned makes so sym.add() doesn't ignore those
                        // files thus forcing them to be symlinked
                        let group_dir = sym.dotfiles_dir.join("Configs").join(group);
                        utils::group_dir_map(group_dir, |group_file| {
                            if &utils::to_home_path(group_file.path().to_str().unwrap()) == file {
                                if file.is_dir() {
                                    _ = fs::remove_dir_all(file);
                                } else {
                                    _ = fs::remove_file(file);
                                }
                            }
                        });
                    }
                }
            }

            if adopt {
                // Discard dotfile and adopt the conflicting dotfile
                let group_dir = utils::get_dotfiles_path()
                    .unwrap()
                    .join("Configs")
                    .join(group);

                for files in sym.not_owned.values() {
                    for file in files {
                        utils::group_dir_map(group_dir.clone(), |f| {
                            let group_path = f.path();
                            // only adopts dotfile if it matches requested group
                            if utils::to_home_path(group_path.to_str().unwrap()) == file.clone() {
                                if group_path.is_dir() {
                                    _ = fs::remove_dir(&group_path);
                                } else {
                                    _ = fs::remove_file(&group_path);
                                }
                                _ = fs::rename(file, group_path);
                            }
                        });
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

fn handle_global_status(sym: &SymlinkHandler) -> Result<(), ExitCode> {
    #[derive(Tabled)]
    struct SymlinkRow<'a> {
        #[tabled(display_with = "display_option")]
        #[tabled(rename = "Symlinked")]
        symlinked: Option<&'a str>,

        #[tabled(display_with = "display_option")]
        #[tabled(rename = "Not Symlinked")]
        not_symlinked: Option<&'a str>,
    }

    // used on SymlinkRow so that table rows are empty if it's None
    fn display_option<'a>(o: &Option<&'a str>) -> &'a str {
        match o {
            Some(s) => s,
            None => "",
        }
    }

    // Generates a Vec<SymlinkRow> for symlinked and not symlinked files
    let mut symlinked_status: Vec<SymlinkRow> = Vec::new();
    for sym in &sym.symlinked {
        let symlinked_group = utils::to_group_name(sym.to_str().unwrap()).unwrap();
        symlinked_status.push(SymlinkRow {
            symlinked: Some(symlinked_group),
            not_symlinked: None,
        });
    }

    let mut notsym_status: Vec<SymlinkRow> = Vec::new();
    for nsym in &sym.not_symlinked {
        let notsym_group = utils::to_group_name(nsym.to_str().unwrap()).unwrap();
        notsym_status.push(SymlinkRow {
            symlinked: None,
            not_symlinked: Some(notsym_group),
        });
    }

    // Merges symlinked_status and notsym_status into a single Vec<SymlinkRow>
    let mut status: Vec<SymlinkRow> = Vec::new();
    // loops over the biggest vector so the resulting vector can encompass all values
    for i in 0..if symlinked_status.len() > notsym_status.len() {
        symlinked_status.len()
    } else {
        notsym_status.len()
    } {
        let sym = symlinked_status.get(i).unwrap_or(&SymlinkRow {
            symlinked: None,
            not_symlinked: None,
        });

        let nsym = notsym_status.get(i).unwrap_or(&SymlinkRow {
            symlinked: None,
            not_symlinked: None,
        });

        let new_sym = SymlinkRow {
            symlinked: if sym.symlinked.is_none() && nsym.symlinked.is_some() {
                nsym.symlinked
            } else {
                sym.symlinked
            },

            not_symlinked: if sym.not_symlinked.is_none() && nsym.not_symlinked.is_some() {
                nsym.not_symlinked
            } else {
                sym.not_symlinked
            },
        };

        status.push(new_sym);
    }

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

    let mut conflict_table = Table::builder(sym.not_owned.keys())
        .set_columns(["Conflicting Dotfiles".yellow().to_string()])
        .clone()
        .build();
    conflict_table
        .with(Style::empty())
        .with(Alignment::center());

    // Creates a table with sym_table and conflict_table
    let mut final_table = col![sym_table];

    if !sym.not_owned.is_empty() {
        final_table = col![sym_table, conflict_table];
    }

    final_table
        .with(Style::empty())
        .with(Margin::new(4, 4, 1, 1))
        .with(Alignment::center());
    println!("{final_table}");

    if !sym.not_owned.is_empty() {
        println!("To learn more about conflicting dotfiles run: `tuckr status <group...>`\n")
    }

    // Determines exit code for the command
    if !sym.symlinked.is_empty() && sym.not_symlinked.is_empty() && sym.not_owned.is_empty() {
        Ok(())
    } else {
        Err(ExitCode::FAILURE)
    }
}

fn print_groups_status(sym: &SymlinkHandler, groups: Vec<String>) -> Result<(), ExitCode> {
    'next_group: for group in &groups {
        for item in &sym.symlinked {
            if item.file_name().unwrap().to_str().unwrap() == group {
                println!("{}", (group.to_owned() + " is already symlinked.").green());
                continue 'next_group;
            }
        }

        if let Some(files) = sym.not_owned.get(group) {
            println!("The following {group} files are in conflict:");
            for file in files {
                println!("\t{}", file.to_str().unwrap().red());
            }
            println!(
                "\n{}\n",
                "Check `tuckr help add` to learn how to resolve them.".yellow()
            );
            continue;
        }

        if sym.not_symlinked.is_empty() {
            println!("{}", (group.to_owned() + " does not exist.").yellow());
            continue;
        }

        for item in &sym.not_symlinked {
            if item.file_name().unwrap().to_str().unwrap() == group {
                println!("{}", (group.to_owned() + " is not yet symlinked.").red());
            } else {
                println!("{}", (group.to_owned() + " does not exist.").yellow());
            }
        }
    }

    let symlinked_groups: Vec<_> = sym.symlinked.iter().collect();
    let symlinked_groups: Vec<_> = symlinked_groups
        .iter()
        .map(|f| f.file_name().unwrap().to_str().unwrap())
        .collect();
    for group in groups {
        if !symlinked_groups.contains(&group.as_str()) {
            return Err(ExitCode::FAILURE);
        }
    }

    Ok(())
}

/// Prints symlinking status
pub fn status_cmd(groups: Option<Vec<String>>) -> Result<(), ExitCode> {
    let sym = SymlinkHandler::try_new()?;
    match groups {
        Some(groups) => print_groups_status(&sym, groups)?,
        None => handle_global_status(&sym)?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::utils;
    use std::fs::{self, File};
    use std::path;

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
    fn init_symlink_test() -> (super::SymlinkHandler, path::PathBuf) {
        let sym = super::SymlinkHandler::try_new().unwrap();
        let group_dir = sym.dotfiles_dir.clone().join("Configs").join("group");
        if let Err(_) = fs::create_dir_all(group_dir.clone().join(".config")) {
            panic!("Could not create required folders");
        }

        File::create(group_dir.clone().join("group.test")).unwrap();
        File::create(group_dir.clone().join(".config").join("group.test")).unwrap();

        let sym = sym.validate().unwrap();

        (sym, group_dir)
    }

    #[test]
    fn add_symlink() {
        let init = init_symlink_test();
        let sym = init.0;
        let group_dir = init.1;
        let group_name = group_dir.file_name().unwrap().to_str().unwrap();

        sym.add(group_name);

        let file = group_dir.clone().join("group.test");
        let config_file = group_dir.clone().join(".config").join("group.test");
        assert_eq!(
            fs::read_link(utils::to_home_path(file.to_str().unwrap())).unwrap(),
            file
        );
        assert_eq!(
            fs::read_link(utils::to_home_path(config_file.to_str().unwrap())).unwrap(),
            config_file
        );

        sym.remove(group_name);
    }

    #[test]
    fn remove_symlink() {
        let init = init_symlink_test();
        let sym = init.0;
        let group_dir = init.1;

        sym.add("group");
        sym.remove("group");

        let file = group_dir.join("group.test");
        let config_file = group_dir.join(".config").join("group.test");
        assert!(
            match fs::read_link(utils::to_home_path(file.to_str().unwrap())) {
                Err(_) => true,
                Ok(link) => link != file,
            }
        );

        assert!(
            match fs::read_link(utils::to_home_path(config_file.to_str().unwrap())) {
                Err(_) => true,
                Ok(link) => link != file,
            }
        );
        let _ = fs::remove_dir_all(group_dir);
    }
}
