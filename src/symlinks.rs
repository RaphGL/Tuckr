use crate::fileops;
use crate::utils;
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process;

#[cfg(target_os = "windows")]
fn symlink_file(f: fs::DirEntry) {
    let target_path = utils::to_home_path(f.path().to_str().unwrap());
    let _ = std::os::windows::fs::symlink_file(f.path(), target_path);
}

#[cfg(target_os = "linux")]
fn symlink_file(f: fs::DirEntry) {
    let target_path = utils::to_home_path(f.path().to_str().unwrap());
    let _ = std::os::unix::fs::symlink(f.path(), target_path);
}

/// Handles generic symlinking and symlink status
struct SymlinkHandler {
    dotfiles_dir: PathBuf,           // path to the dotfiles directory
    symlinked: HashSet<PathBuf>,     // path to symlinked programs in Dotfiles/Configs
    not_symlinked: HashSet<PathBuf>, // path to programs that aren't symlinked to $HOME
    not_owned: HashSet<PathBuf>,     // Path to files in $HOME that don't link to dotfiles_dir
}

impl SymlinkHandler {
    /// Initializes SymlinkHandler and fills it with information about all the dotfiles
    fn new() -> SymlinkHandler {
        let symlinker = SymlinkHandler {
            dotfiles_dir: PathBuf::from(fileops::get_dotfiles_path().unwrap_or_else(|| {
                eprintln!("Error: Could not find dotfiles, make sure it's in the right path");
                process::exit(1);
            })),
            symlinked: HashSet::new(),
            not_symlinked: HashSet::new(),
            not_owned: HashSet::new(),
        };

        // this will fill symlinker with all the information it needs to be useful
        symlinker.validate_symlinks()
    }

    /// THIS FUNCTION SHOULD NOT BE USED DIRECTLY
    /// Checks which dotfiles are or are not symlinked and registers their Configs/$PROGRAM path
    /// into the struct
    /// Returns a copy of self with all the fields set accordingly
    fn validate_symlinks(mut self: Self) -> Self {
        // Opens and loops through each of Dotfiles/Configs' dotfiles
        let dir = fs::read_dir(self.dotfiles_dir.clone().join("Configs")).unwrap_or_else(|_| {
            eprintln!("There's no Configs folder set up");
            process::exit(1);
        });
        for file in dir {
            let program_dir = file.unwrap();
            if program_dir.file_type().unwrap().is_file() {
                continue;
            }

            // Checks for the files in each of the programs' dirs
            for f in fs::read_dir(program_dir.path()).unwrap() {
                let file = f.unwrap();

                // a closure that takes a file and determines if it's a symlink or not
                let check_symlink = |f: fs::DirEntry| {
                    let config_file = utils::to_home_path(f.path().to_str().unwrap());
                    if let Ok(f) = fs::read_link(&config_file) {
                        // program_dir can only be in one set at a time
                        // this makes it so one would get an not symlinked status
                        // if at least one of the files is not symlinked
                        let dotfiles_configs_path = PathBuf::from("dotfiles").join("Configs");
                        let dotfiles_configs_path = dotfiles_configs_path.to_str().unwrap();
                        if f.to_str().unwrap().contains(dotfiles_configs_path) {
                            self.symlinked.insert(program_dir.path());
                            self.not_symlinked.remove(&program_dir.path());
                        } else {
                            self.not_symlinked.insert(program_dir.path());
                            self.symlinked.remove(&program_dir.path());
                        }
                    } else {
                        self.not_symlinked.insert(program_dir.path());
                        self.symlinked.remove(&program_dir.path());
                        self.not_owned.insert(PathBuf::from(config_file));
                    }
                };

                // iterate through all the files in program_dir
                utils::file_or_xdgdir_map(file, check_symlink);
            }
        }

        self
    }

    /// Symlinks all the files of a program to the user's $HOME
    fn add(self: &Self, program: &str) {
        let program_dir = fs::read_dir(self.dotfiles_dir.clone().join("Configs").join(&program));
        if let Ok(dir) = program_dir {
            for file in dir {
                let file = file.unwrap();

                // iterate through all the files in program_dir
                utils::file_or_xdgdir_map(file, symlink_file);
            }
        } else {
            eprintln!(
                "{} {}",
                "Error: There's no program called".red(),
                program.red()
            );
        }
    }

    /// Deletes symlinks from $HOME if their links are pointing to the dotfiles directory
    fn remove(self: &Self, program: &str) {
        let program_dir = fs::read_dir(self.dotfiles_dir.clone().join("Configs").join(&program));
        if let Ok(dir) = program_dir {
            for file in dir {
                let file = file.unwrap();

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

                // iterate through all the files in program_dir
                utils::file_or_xdgdir_map(file, remove_symlink);
            }
        } else {
            eprintln!(
                "{} {}",
                "Error: There's no program called".red(),
                program.red()
            );
        }
    }
}

pub fn add_cmd(programs: &[String]) {
    let sym = SymlinkHandler::new();

    for program in programs {
        // add all programs if wildcard
        if program == "*" {
            for p in &sym.not_symlinked {
                // Takes the name of the program and passes to the function
                let p = utils::to_program_name(p.to_str().unwrap()).unwrap();
                sym.add(p);
            }
            break;
        } else {
            sym.add(program);
        }
    }
}

pub fn remove_cmd(programs: &[String]) {
    let sym = SymlinkHandler::new();
    for program in programs {
        // remove all programs if wildcard
        if program == "*" {
            for p in &sym.symlinked {
                // Takes the name of the program and passes to the function
                let p = utils::to_program_name(p.to_str().unwrap()).unwrap();
                sym.remove(p);
            }
            break;
        } else {
            sym.remove(program);
        }
    }
}

/// Prints symlinking status
pub fn status_cmd() {
    let sym = SymlinkHandler::new();
    if !sym.symlinked.is_empty() {
        print!("Symlinked programs:\n");
        for program in sym.symlinked {
            print!(
                "\t\t{}\n",
                utils::to_program_name(program.to_str().unwrap())
                    .unwrap()
                    .green()
            );
        }
    }

    if !sym.not_symlinked.is_empty() {
        print!("Programs that aren't symlinked:\n");
        for program in sym.not_symlinked {
            print!(
                "\t\t{}\n",
                utils::to_program_name(program.to_str().unwrap())
                    .unwrap()
                    .red()
            );
        }
    } else {
        print!("{}", "\nAll programs are already symlinked.\n".yellow());
    }

    if !sym.not_owned.is_empty() {
        print!("\nThe following files are in conflict with your dotfiles:\n");
        for file in sym.not_owned {
            print!("\t{}\n", file.to_str().unwrap().yellow());
        }
    }

    print!("\n");
    std::io::stdout().flush().unwrap();
}

#[cfg(test)]
mod tests {
    use crate::utils;
    use std::path;
    use std::{
        collections::HashSet,
        fs::{self, File},
    };

    // makes sure that symlink status is loaded on startup
    #[test]
    fn new_symlink_handler() {
        let sym = super::SymlinkHandler::new();
        assert!(
            if !sym.symlinked.is_empty() || !sym.not_symlinked.is_empty() {
                true
            } else {
                false
            }
        );
    }

    fn init_symlink_test() -> (super::SymlinkHandler, path::PathBuf) {
        let sym = super::SymlinkHandler {
            dotfiles_dir: path::PathBuf::from(std::env::temp_dir())
                .join(format!("tuckr-{}", std::process::id()))
                .join("dotfiles"),
            symlinked: HashSet::new(),
            not_symlinked: HashSet::new(),
            not_owned: HashSet::new(), // TODO not yet tested
        };
        let program_dir = sym.dotfiles_dir.clone().join("Configs").join("program");
        if fs::create_dir_all(program_dir.clone().join(".config")).is_err() {
            panic!("Could not create required folders");
        }

        File::create(program_dir.clone().join("program.test")).unwrap();
        File::create(program_dir.clone().join(".config").join("program.test")).unwrap();

        let sym = sym.validate_symlinks();

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
    fn remove_symlink() {
        let init = init_symlink_test();
        let sym = init.0;
        let program_dir = init.1;

        sym.add("program");
        sym.remove("program");

        let file = program_dir.clone().join("program.test");
        let config_file = program_dir.clone().join(".config").join("program.test");
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
