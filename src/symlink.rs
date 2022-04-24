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

        // pushes file to their specific struct
        let mut push_to_struct = |fpath: PathBuf| {
            // ignore .git folder 
            if fpath.to_str().unwrap().contains(".git") {
                return;
            }
            // check if file is symlinked or not
            if is_valid_symlink(fpath.clone()) {
                self.symlinked.push(fpath);
            } else {
                self.notsymlink.push(fpath);
            }
        };

        // read the config of each program
        for program in
            fs::read_dir(format!("{}/{}", dotfiles.to_str().unwrap(), "configs")).unwrap()
        {
            let p = program.unwrap();
            // if file is a program dir, read it
            if let Ok(config) = fs::read_dir(p.path()) {
                for conf in config {
                    let c = conf.unwrap();
                    // reads subdirs from program such as .config
                    if let Ok(dir) = fs::read_dir(c.path()) {
                        for file in dir {
                            push_to_struct(file.unwrap().path());
                        }
                    } else {
                        push_to_struct(c.path());
                    }
                }
            }
        }
    }

    fn strip_away_program_path<'a>(&self, fpath: &'a PathBuf) -> &'a str {
        let mut newstr = fpath.to_str().unwrap().split_once("configs").unwrap();
        newstr = newstr.1.split_once("/").unwrap();
        newstr.1
    }

    // Retrieve symlinked filenames
    fn print_symlinked(&self) {
        if self.symlinked.len() > 0 {
            println!("Symlinked files:");
            for f in &self.symlinked {
                let f = self.strip_away_program_path(f);
                println!("\t{}", f.green());
            }
        }
    }
    // Retrieve non symlinked filenames
    fn print_notsymlinked(self) {
        if self.notsymlink.len() > 0 {
            println!("Not symlinked files:");
            for f in &self.notsymlink {
                let f = self.strip_away_program_path(f);
                println!("\t{}", f.red());
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
