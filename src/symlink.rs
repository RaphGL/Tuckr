use colored::Colorize;
use core::panic;
use dirs;
use std::fmt::Result;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

struct SymlinkStatus {
    symlinked: Vec<PathBuf>,
    notsymlink: Vec<PathBuf>,
}

impl SymlinkStatus {
    // Initializes struct
    fn new() -> SymlinkStatus {
        SymlinkStatus {
            symlinked: Vec::new(),
            notsymlink: Vec::new(),
        }
    }

    // Populates SymlinkStatus with info
    fn retrieve_info(&mut self) {
        // find the dotfile's path
        let dotfiles = (|| {
            let home = dirs::home_dir().unwrap();
            for f in fs::read_dir(home).unwrap() {
                let file = f.unwrap();
                let filepath = file.path();
                let filename = filepath.to_str().unwrap();
                if filename.contains("Dotfiles")
                    || filename.contains("dotfiles")
                    || filename.contains(".dotfiles")
                {
                    return Some(filepath);
                }
            }
            None
        })()
        .unwrap();

        // read the config of each program
        for program in
            fs::read_dir(format!("{}/{}", dotfiles.to_str().unwrap(), "configs")).unwrap()
        {
            let p = program.unwrap();
            if let Ok(config) = fs::read_dir(p.path()) {
                for conf in config {
                    let c = conf.unwrap();
                    if is_valid_symlink(c.path().clone()) {
                        self.symlinked.push(c.path());
                    } else {
                        self.notsymlink.push(c.path());
                    }
                }
            }
        }
    }

    // Retrieve symlinked filenames
    fn print_symlinked(&self) {
        if self.symlinked.len() > 0 {
            println!("Symlinked files:");
            for f in &self.symlinked {
                println!("\t{}", f.to_str().unwrap().green());
            }
        }
    }
    // Retrieve non symlinked filenames
    fn print_notsymlinked(self) {
        if self.notsymlink.len() > 0 {
            println!("Not symlinked files:");
            for f in &self.notsymlink {
                println!("\t{}", f.to_str().unwrap().red());
            }
        }
    }
}

// Returns true if Dotfiles file matches file in $HOME
fn is_valid_symlink(file: PathBuf) -> bool {
    let fpath = file.to_str().unwrap();

    // strips away $HOME/Dotfiles/program from string
    let new_path: (&str, &str);
    if fpath.contains("Dotfiles") {
        new_path = fpath.split_once("Dotfiles/").unwrap();
    } else if fpath.contains("dotfiles") {
        new_path = fpath.split_once("dotfiles/").unwrap();
    } else if fpath.contains(".dotfiles") {
        new_path = fpath.split_once(".dotfiles/").unwrap();
    } else {
        return false;
    }
    let mut new_path = new_path.1.split_once("/").unwrap();
    new_path = new_path.1.split_once("/").unwrap();

    // appends home to new_path
    let path_with_home = dirs::home_dir().unwrap();
    let path_with_home = path_with_home.to_str().unwrap().to_owned() + "/" + new_path.1;
    let f = Path::new(&path_with_home);
    // returns true if file is a symlink and is on Dotfiles
    if f.exists() && f.read_link().is_ok() {
        true
    } else {
        false
    }
}

pub fn get_status() {
    let mut symstruct = SymlinkStatus::new();
    symstruct.retrieve_info();
    symstruct.print_symlinked();
    symstruct.print_notsymlinked();
}

// Symlink files
pub fn add(program_name: clap::Values, files: clap::Values) {
    // TODO
}

// Remove symlink from files
pub fn remove(program_name: clap::Values, files: clap::Values) {
    // TODO
}
