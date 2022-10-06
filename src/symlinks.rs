use crate::fileops;
use crate::utils;
use colored::Colorize;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Handles generic symlinking and symlink status
struct SymlinkHandler {
    dotfiles_dir: String,
    symlinked: Vec<PathBuf>,
    not_symlinked: Vec<PathBuf>,
}

impl SymlinkHandler {
    /// Initializes SymlinkHandler and fills it with information about all the dotfiles
    fn new() -> SymlinkHandler {
        let symlinker = SymlinkHandler {
            dotfiles_dir: fileops::get_dotfiles_path().unwrap(),
            symlinked: Vec::new(),
            not_symlinked: Vec::new(),
        };

        symlinker.validate_symlinks()
    }

    /// THIS FUNCTION SHOULD NOT BE USED DIRECTLY
    /// Checks which dotfiles are or are not symlinked and registers their Configs/$PROGRAM path
    /// into the struct
    /// Returns a copy of self with all the fields set accordingly
    fn validate_symlinks(mut self: Self) -> Self {
        // Opens and loops through each of Dotfiles/Configs' dotfiles
        let dir = fs::read_dir(self.dotfiles_dir.clone() + "/Configs")
            .expect("There's no Configs folder set up");
        for file in dir {
            let program_dir = file.unwrap();
            if program_dir.file_type().unwrap().is_file() {
                continue;
            }

            // Checks for the files in each of the programs' dirs
            for f in fs::read_dir(program_dir.path()).unwrap() {
                let file = f.unwrap();

                let check_symlink = |f: fs::DirEntry| {
                    let config_file = utils::to_home_path(f.path().to_str().unwrap());
                    if let Ok(f) = fs::read_link(&config_file) {
                        if f.to_str().unwrap().contains("dotfiles/Configs") {
                            self.symlinked.push(program_dir.path());
                        } else {
                            self.not_symlinked.push(program_dir.path());
                        }
                    } else {
                        self.not_symlinked.push(program_dir.path());
                    }
                };

                utils::file_or_xdgdir_map(file, check_symlink);
            }
        }

        self
    }

    /// Symlinks all the files of a program to the user's $HOME
    fn add(self: &Self, program: &str) {
        let program_dir = fs::read_dir(self.dotfiles_dir.clone() + "/Configs/" + &program);
        if let Ok(dir) = program_dir {
            for file in dir {
                let file = file.unwrap();

                let symlink_file = |f: fs::DirEntry| {
                    let _ = std::os::unix::fs::symlink(
                        f.path(),
                        utils::to_home_path(f.path().to_str().unwrap()),
                    );
                };

                utils::file_or_xdgdir_map(file, symlink_file);
            }
        } else {
            println!(
                "{} {}",
                "Error: There's no program called".red(),
                program.red()
            );
        }
    }

    /// Deletes symlinks from $HOME if their links are pointing to the dotfiles directory
    fn remove(self: &Self, program: &str) {
        let program_dir = fs::read_dir(self.dotfiles_dir.clone() + "/Configs/" + &program);
        if let Ok(dir) = program_dir {
            for file in dir {
                let file = file.unwrap();

                let remove_symlink = |file: fs::DirEntry| {
                    let dotfile = utils::to_home_path(file.path().to_str().unwrap());
                    if let Ok(linked) = fs::read_link(&dotfile) {
                        if linked.to_str().unwrap().contains("dotfiles/Configs") {
                            fs::remove_file(dotfile).unwrap();
                        }
                    }
                };

                utils::file_or_xdgdir_map(file, remove_symlink);
            }
        } else {
            println!(
                "{} {}",
                "Error: There's no program called".red(),
                program.red()
            );
        }
    }
}

pub fn add_cmd(programs: clap::parser::ValuesRef<String>) {
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

pub fn remove_cmd(programs: clap::parser::ValuesRef<String>) {
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
        print!("\t(use \"tuckr add <program>\" to resymlink program)\n");
        print!("\t(use \"tuckr rm <program>\" to remove the symlink)\n");
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
        print!("\t(use \"tuckr add <program>\" to symlink it)\n");
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
    print!("\n");
    std::io::stdout().flush().unwrap();
}
