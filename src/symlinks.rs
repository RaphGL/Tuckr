use crate::fileops;
use crate::utils::to_home_path;
use colored::Colorize;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

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
            let file = file.unwrap();
            if file.file_type().unwrap().is_file() {
                continue;
            }

            // Checks for the files in each of the programs' dirs
            for f in fs::read_dir(file.path()).unwrap() {
                let f = f.unwrap();
                let config_file = to_home_path(f.path().to_str().unwrap());

                if let Ok(f) = fs::read_link(&config_file) {
                    if f.to_str().unwrap().contains("dotfiles/Configs") {
                        self.symlinked.push(file.path())
                    } else {
                        self.not_symlinked.push(file.path())
                    }
                } else {
                    self.not_symlinked.push(file.path())
                }
            }
        }

        self
    }

    /// Symlinks all the files of a program to the user's $HOME
    fn add(self: &Self, program: &str) {
        let program_dir = fs::read_dir(self.dotfiles_dir.clone() + "/Configs/" + &program).unwrap();
        for file in program_dir {
            let f = file.unwrap();
            _ = std::os::unix::fs::symlink(f.path(), to_home_path(f.path().to_str().unwrap()));
        }
    }

    /// Deletes symlinks from $HOME if their links are pointing to the dotfiles directory
    fn remove(self: &Self, program: &str) {
        let program_dir = fs::read_dir(self.dotfiles_dir.clone() + "/Configs/" + &program).unwrap();
        for file in program_dir {
            let file = file.unwrap();
            let dotfile = to_home_path(file.path().to_str().unwrap());
            if let Ok(linked) = fs::read_link(&dotfile) {
                if linked.to_str().unwrap().contains("dotfiles/Configs") {
                    fs::remove_file(dotfile).unwrap();
                }
            }
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
                let p = p.to_owned().into_os_string().into_string().unwrap();
                let p = p.as_str().split_once("dotfiles/Configs/").unwrap().1;
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
                let p = p.to_owned().into_os_string().into_string().unwrap();
                let p = p.as_str().split_once("dotfiles/Configs/").unwrap().1;
                sym.remove(p);
            }
            break;
        } else {
            sym.remove(program);
        }
    }
}

pub fn status_cmd() {
    let sym = SymlinkHandler::new();
    if !sym.symlinked.is_empty() {
        print!("Symlinked programs:\n");
        print!("\t(use \"tuckr add <program>\" to resymlink program)\n");
        print!("\t(use \"tuckr rm <program>\" to remove the symlink)\n");
        for program in sym.symlinked {
            print!(
                "\t\t{}\n",
                program
                    .to_str()
                    .unwrap()
                    .split_once("dotfiles/Configs/")
                    .unwrap()
                    .1
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
                program
                    .to_str()
                    .unwrap()
                    .split_once("dotfiles/Configs/")
                    .unwrap()
                    .1
                    .red()
            );
        }
    } else {
        print!("{}", "\nAll programs are already symlinked.\n".yellow());
    }
    print!("\n");
    std::io::stdout().flush().unwrap();
}
