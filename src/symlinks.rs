use colored::Colorize;
use dirs;
use std::fs::{self, DirEntry};
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
        let dotfiles = crate::get_dotfiles_path().unwrap();
        println!("{}", dotfiles.to_str().unwrap());

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
            fs::read_dir(format!("{}/{}", dotfiles.to_str().unwrap(), "Configs")).unwrap()
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
        let mut newstr = fpath.to_str().unwrap().split_once("Configs").unwrap();
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
    // TODO: use crate::get_dotfiles_path for this instead
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
pub fn add(program_name: clap::Values) {
    let home_dir = dirs::home_dir().unwrap();

    // create symlinks from conf to home_conf
    let create_symlink = |home_conf: String, conf: DirEntry| {
        if let Err(_) = std::os::unix::fs::symlink(conf.path(), &home_conf) {
            if is_valid_symlink(PathBuf::from(&home_conf)) {
                return;
            } else {
                fs::remove_file(PathBuf::from(&home_conf)).unwrap();
                std::os::unix::fs::symlink(conf.path(), home_conf).unwrap();
            }
        }
    };

    for p in program_name {
        // make $HOME/Dotfiles/Configs/program string
        let program_dir = format!(
            "{}/{}/{}",
            crate::get_dotfiles_path().unwrap().to_str().unwrap(),
            "Configs",
            p
        );
        let program_dir = fs::read_dir(program_dir);
        if let Ok(dir) = program_dir {
            // read all the configs for said program
            for f in dir {
                let cfg = f.unwrap();
                // if program is a dir get all the files inside of it
                if cfg.file_name() == ".config" {
                    let cfg_dir = fs::read_dir(cfg.path()).unwrap();
                    for c in cfg_dir {
                        let conf = c.unwrap();
                        // create $HOME/.config/program string
                        let to_home_path = format!(
                            "{}/{}/{}",
                            home_dir.to_str().unwrap(),
                            ".config",
                            conf.file_name().to_str().unwrap()
                        );
                        create_symlink(to_home_path, conf);
                    }
                } else {
                    // create $HOME/config string
                    let to_home_path = format!(
                        "{}/{}",
                        home_dir.to_str().unwrap(),
                        cfg.file_name().to_str().unwrap()
                    );
                    create_symlink(to_home_path, cfg);
                }
            }
        }
    }
}

// Remove symlink from files
pub fn remove(program_name: clap::Values) {
    let home_dir = dirs::home_dir().unwrap();
    for p in program_name {
        // get /home/Dotfiles/Configs/program
        let program_path = format!(
            "{}/{}/{}",
            crate::get_dotfiles_path().unwrap().to_str().unwrap(),
            "Configs",
            p
        );
        for f in fs::read_dir(program_path).unwrap() {
            let file = f.unwrap();
            // generate symlink path
            let home_path = format!("{}/{}", home_dir.to_str().unwrap(), file.file_name().to_str().unwrap());
            // if Configs/program file has a symlink, delete it
            if is_valid_symlink(file.path()) {
                if file.file_type().unwrap().is_dir() {
                    fs::remove_dir_all(home_path).unwrap();
                } else {
                    fs::remove_file(home_path).unwrap();
                }
            }
        }
    }
}
