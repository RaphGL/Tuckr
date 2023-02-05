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
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use tabled::{Table, Tabled};

#[cfg(target_family = "windows")]
fn symlink_file(f: fs::DirEntry) {
    let target_path = utils::to_home_path(f.path().to_str().unwrap());
    _ = std::os::windows::fs::symlink_file(f.path(), target_path);
}

#[cfg(target_family = "unix")]
fn symlink_file(f: fs::DirEntry) {
    let target_path = utils::to_home_path(f.path().to_str().unwrap());
    _ = std::os::unix::fs::symlink(f.path(), target_path);
}

/// Handles dotfile symlinking and their current status
struct SymlinkHandler {
    dotfiles_dir: PathBuf,                    // path to the dotfiles directory
    symlinked: HashSet<PathBuf>,              // path to symlinked programs in Dotfiles/Configs
    not_symlinked: HashSet<PathBuf>,          // path to programs that aren't symlinked to $HOME
    not_owned: HashMap<String, Vec<PathBuf>>, // key: group the file belongs to, value: list of conflicting files from that group
}

impl SymlinkHandler {
    /// Initializes SymlinkHandler and fills it dotfiles' status information
    fn new() -> Self {
        let symlinker = SymlinkHandler {
            dotfiles_dir: utils::get_dotfiles_path().unwrap_or_else(|| {
                eprintln!(
                    "{}",
                    "Could not find dotfiles, make sure it's in the right path".red()
                );
                process::exit(utils::COULDNT_FIND_DOTFILES);
            }),
            symlinked: HashSet::new(),
            not_symlinked: HashSet::new(),
            not_owned: HashMap::new(),
        };

        // this fills the symlinker with dotfile status information
        symlinker.validate()
    }

    /// **This function should not be used outside this scope**
    ///
    /// Checks which dotfiles are or are not symlinked and registers their Configs/$PROGRAM path
    /// into the struct
    ///
    /// Returns a copy of self with all the fields set accordingly
    fn validate(mut self) -> Self {
        // Opens and loops through each of Dotfiles/Configs' dotfiles
        let dir = fs::read_dir(self.dotfiles_dir.join("Configs")).unwrap_or_else(|_| {
            eprintln!("{}", "There's no Configs folder set up".red());
            process::exit(utils::NO_SETUP_FOLDER);
        });

        for file in dir {
            let program_dir = file.unwrap();
            // Ignores all regular files since Configs should only care about group folders
            if program_dir.path().is_file() {
                continue;
            }

            // Checks for the files in each of the programs' dirs
            utils::program_dir_map(program_dir.path(), |f| {
                let config_file = utils::to_home_path(f.path().to_str().unwrap());

                match fs::read_link(&config_file) {
                    Ok(f) => {
                        let dotfiles_configs_path = PathBuf::from("dotfiles").join("Configs");
                        let dotfiles_configs_path = dotfiles_configs_path.to_str().unwrap();

                        // program_dir can only be in one set at a time
                        // this makes it so one would get a not symlinked status
                        // if at least one of the files is not symlinked
                        if f.to_str().unwrap().contains(dotfiles_configs_path) {
                            self.symlinked.insert(program_dir.path());
                            self.not_symlinked.remove(&program_dir.path());
                        } else {
                            self.not_symlinked.insert(program_dir.path());
                            self.symlinked.remove(&program_dir.path());
                        }
                    }

                    // file is in conflict with dotfiles and is added to not_owned
                    Err(_) => {
                        self.not_symlinked.insert(program_dir.path());
                        self.symlinked.remove(&program_dir.path());
                        if PathBuf::from(&config_file).exists() {
                            let program_dir = program_dir.file_name().to_str().unwrap().to_string();
                            if let Some(group) = self.not_owned.get_mut(&program_dir) {
                                group.push(config_file);
                            } else {
                                self.not_owned.insert(program_dir, vec![config_file]);
                            }
                        }
                    }
                }
            });
        }

        self
    }

    /// Symlinks all the files of a program to the user's $HOME
    fn add(&self, program: &str) {
        let program_dir = self.dotfiles_dir.join("Configs").join(program);
        if program_dir.exists() {
            // iterate through all the files in program_dir
            utils::program_dir_map(program_dir, symlink_file);
        } else {
            eprintln!(
                "{} {}",
                "Error: There's no dotfiles for".red(),
                program.red()
            );
        }
    }

    /// Deletes symlinks from $HOME if they're owned by dotfiles dir
    fn remove(&self, program: &str) {
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

        let program_dir = self.dotfiles_dir.join("Configs").join(program);
        if program_dir.exists() {
            // iterate through all the files in program_dir
            utils::program_dir_map(program_dir, remove_symlink);
        } else {
            eprintln!(
                "{} {}",
                "Error: There's no program called".red(),
                program.red()
            );
        }
    }
}

/// programs: the programs will be applied to
///
/// exclude: the programs that will be ignored
///
/// symlinked: whether it should be applied to symlinked or non symlinked programs
/// iterates over each program in the dotfiles and calls a function F giving it the SymlinkHandler
/// instance and the name of the program that's being handled
///
/// This abstracts this recurrent loop allowing handle programs just by their names
fn foreach_program<F>(programs: &[String], exclude: &[String], symlinked: bool, func: F)
where
    F: Fn(&SymlinkHandler, &String),
{
    // loads the runtime information needed to carry out actions
    let sym = SymlinkHandler::new();

    // handles wildcard
    if programs.contains(&"*".to_string()) {
        let symgroup = if symlinked {
            &sym.not_symlinked
        } else {
            &sym.symlinked
        };

        for p in symgroup {
            // Takes the name of the program to be passed the function
            let program_name = utils::to_program_name(p.to_str().unwrap()).unwrap();
            // Ignore programs in the excludes array
            if exclude.contains(&program_name.to_string()) {
                continue;
            }
            // do something with the program name
            // passing the sym context
            func(&sym, &program_name.to_string());
        }
        return;
    }

    for program in programs {
        // add all programs if wildcard
        if exclude.contains(program) {
            continue;
        } else {
            func(&sym, program);
        }
    }
}

/// Adds symlinks
pub fn add_cmd(programs: &[String], exclude: &[String], force: bool, adopt: bool) {
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
            _ => return,
        }
    }

    foreach_program(programs, exclude, true, |sym, program| {
        if !sym.not_owned.is_empty() {
            // Symlink dotfile by force
            if force {
                for files in sym.not_owned.values() {
                    for file in files {
                        // removing everything from sym.not_owned makes so sym.add() doesn't ignore those
                        // files thus forcing them to be symlinked
                        let program_dir = sym.dotfiles_dir.join("Configs").join(program);
                        utils::program_dir_map(program_dir, |program_file| {
                            if &utils::to_home_path(program_file.path().to_str().unwrap()) == file {
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
                let program_dir = utils::get_dotfiles_path()
                    .unwrap()
                    .join("Configs")
                    .join(program);

                for files in sym.not_owned.values() {
                    for file in files {
                        utils::program_dir_map(program_dir.clone(), |f| {
                            let program_path = f.path();
                            // only adopts dotfile if it matches requested program
                            if utils::to_home_path(program_path.to_str().unwrap()) == file.clone() {
                                if program_path.is_dir() {
                                    _ = fs::remove_dir(&program_path);
                                } else {
                                    _ = fs::remove_file(&program_path);
                                }
                                _ = fs::rename(file, program_path);
                            }
                        });
                    }
                }
            }
        }

        sym.add(program)
    });
}

/// Removes symlinks
pub fn remove_cmd(programs: &[String], exclude: &[String]) {
    foreach_program(programs, exclude, false, |sym, p| sym.remove(p));
}

fn print_global_status(sym: SymlinkHandler) {
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
        let symlinked_program = utils::to_program_name(sym.to_str().unwrap()).unwrap();
        symlinked_status.push(SymlinkRow {
            symlinked: Some(symlinked_program),
            not_symlinked: None,
        });
    }

    let mut notsym_status: Vec<SymlinkRow> = Vec::new();
    for nsym in &sym.not_symlinked {
        let notsym_program = utils::to_program_name(nsym.to_str().unwrap()).unwrap();
        notsym_status.push(SymlinkRow {
            symlinked: None,
            not_symlinked: Some(notsym_program),
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
}

fn print_programs_status(sym: SymlinkHandler, programs: Vec<String>) {
    for program in &programs {
        for item in &sym.symlinked {
            if item.file_name().unwrap().to_str().unwrap() == program {
                println!(
                    "{}",
                    (program.to_owned() + " is already symlinked.").green()
                );
                continue;
            }
        }

        if let Some(files) = sym.not_owned.get(program) {
            println!("The following {program} files are in conflict:");
            for file in files {
                println!("\t{}", file.to_str().unwrap().red());
            }
            println!(
                "\n{}\n",
                "Check `tuckr help add` to learn how to resolve them.".yellow()
            );
            continue;
        }

        for item in &sym.not_symlinked {
            if item.file_name().unwrap().to_str().unwrap() == program {
                println!("{}", (program.to_owned() + " is not yet symlinked.").red());
            }
        }
    }
}

/// Prints symlinking status
pub fn status_cmd(programs: Option<Vec<String>>) {
    let sym = SymlinkHandler::new();
    match programs {
        Some(programs) => print_programs_status(sym, programs),
        None => print_global_status(sym),
    }
}

#[cfg(test)]
mod tests {
    use crate::utils;
    use std::path;
    use std::{
        collections::{HashMap, HashSet},
        fs::{self, File},
    };

    /// makes sure that symlink status is loaded on startup
    #[test]
    fn new_symlink_handler() {
        let dotfiles_dir = path::PathBuf::from(utils::get_dotfiles_path().unwrap());
        let dirs = fs::read_dir(dotfiles_dir.join("Configs"));

        if dirs.is_err() {
            panic!("{:#?}", dirs);
        } else {
            let sym = super::SymlinkHandler::new();
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
        let sym = super::SymlinkHandler {
            dotfiles_dir: path::PathBuf::from(std::env::temp_dir())
                .join(format!("tuckr-{}", std::process::id()))
                .join("dotfiles"),
            symlinked: HashSet::new(),
            not_symlinked: HashSet::new(),
            not_owned: HashMap::new(),
        };
        let program_dir = sym.dotfiles_dir.clone().join("Configs").join("program");
        if let Err(_) = fs::create_dir_all(program_dir.clone().join(".config")) {
            panic!("Could not create required folders");
        }

        File::create(program_dir.clone().join("program.test")).unwrap();
        File::create(program_dir.clone().join(".config").join("program.test")).unwrap();

        let sym = sym.validate();

        (sym, program_dir)
    }

    #[test]
    fn add_symlink() {
        let init = init_symlink_test();
        let sym = init.0;
        let program_dir = init.1;

        sym.add("program");

        let file = program_dir.clone().join("program.test");
        let config_file = program_dir.clone().join(".config").join("program.test");
        assert_eq!(
            fs::read_link(utils::to_home_path(file.to_str().unwrap())).unwrap(),
            file
        );
        assert_eq!(
            fs::read_link(utils::to_home_path(config_file.to_str().unwrap())).unwrap(),
            config_file
        );
    }

    #[test]
    fn add_force_symlink() {
        //let init = init_symlink_test();
        //let sym = init.0;
        //let program_dir = init.1;
    }

    #[test]
    fn remove_symlink() {
        let init = init_symlink_test();
        let sym = init.0;
        let program_dir = init.1;

        sym.add("program");
        sym.remove("program");

        let file = program_dir.join("program.test");
        let config_file = program_dir.join(".config").join("program.test");
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
        let _ = fs::remove_dir_all(program_dir);
    }
}
