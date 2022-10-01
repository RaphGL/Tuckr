use crate::fileops;
use std::fs;
use std::path::PathBuf;
use colored;

fn to_home_path(path: &str) -> String {
    format!(
        "{}/{}",
        dirs::home_dir().unwrap().to_str().unwrap(),
        path.split_once("dotfiles/Configs/")
            .unwrap()
            .1
            .split_once("/")
            .unwrap()
            .1
    )
}

#[derive(Debug)]
struct SymlinkHandler {
    dotfiles_dir: String,
    symlinked: Vec<PathBuf>,
    not_symlinked: Vec<PathBuf>,
}

impl SymlinkHandler {
    fn new() -> SymlinkHandler {
        let symlinker = SymlinkHandler {
            dotfiles_dir: fileops::get_dotfiles_path().unwrap(),
            symlinked: Vec::new(),
            not_symlinked: Vec::new(),
        };

        symlinker.validate_symlinks()
    }

    /// TODO make function work ad hoc, it will only work for the programs mentioned
    /// THIS FUNCTION SHOULD NOT BE USED DIRECTLY
    /// Checks which dotfiles are or are not symlinked and registers their Configs/$PROGRAM path
    /// into the struct
    /// Returns a copy of self with all the fields set accordingly
    fn validate_symlinks(mut self: Self) -> Self {
        let home_dir = dirs::home_dir().unwrap();

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

    fn add(self: Self, program: &str) {
        let program_dir = fs::read_dir(self.dotfiles_dir.clone() + "/Configs/" + &program).unwrap();
        for file in program_dir {
            let f = file.unwrap();
            _ = std::os::unix::fs::symlink(f.path(), to_home_path(f.path().to_str().unwrap()));
        }
    }

    fn remove(self: Self, program: &str) {
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

pub fn add_cmd() {
    let sym = SymlinkHandler::new();
    sym.add("FirstTest");
}
pub fn remove_cmd() {
    let sym = SymlinkHandler::new();
    sym.remove("FirstTest");
}

pub fn status_cmd() {

}
