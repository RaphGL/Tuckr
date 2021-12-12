use colored::*;
use dirs;
use std::os::unix::fs::symlink;
use std::{env, fs, path};

//  Returns tuple with symlinked files(0) and unsymlinnked files (1)
pub fn get(path: &str) -> (Vec<path::PathBuf>, Vec<path::PathBuf>) {
    let files = fs::read_dir(path).unwrap();
    let mut dots_sym = Vec::new();
    let mut dots_nsym = Vec::new();
    for i in files {
        let path = i.ok().unwrap().path();
        match fs::read_link(&path) {
            Ok(_) => dots_sym.push(path),
            Err(_) => dots_nsym.push(path),
        }
    }

    (dots_sym, dots_nsym)
}

// Prints the status of the symlinks
pub fn print() {
    let dots = get(".");
    println!("Symlinked files:");
    for f in dots.0 {
        println!("\t{}", f.to_str().unwrap().green());
    }
    println!("Not symlinked files:");
    for f in dots.1 {
        println!("\t{}", f.to_str().unwrap().red());
    }
}

// checks if a file is a valid dotfile entry before doing actions
fn check_validity() -> bool {true}

// finds all files in a dir and then symlinks them to home
pub fn create(fpath: &str) {
    let cur_path = env::current_dir().ok().unwrap();
    let cur_dir = fs::read_dir(fpath).ok();
    let home = dirs::home_dir().unwrap();
    if let Some(dir) = cur_dir {
        for d in dir {
            let dir = d.unwrap();
            let og_file = format!("{}/{}", cur_path.display(), dir.path().display());
            let file_path = dir.path();
            let filename = file_path.to_str().unwrap().split("/").last().unwrap();
            let dest_file = format!("{}/{}", home.display(), filename);
            let _ = symlink(og_file, dest_file);
        }
    }
}

pub fn remove(fpath: &str) {
    if let Some(home_dir)= dirs::home_dir() {
        // TODO make to only read dotfile dir | checks default locations 
        let dir = fs::read_dir(fpath).ok().unwrap();
        for d in dir {
            let file_path = d.ok().unwrap().path();
            let filename = file_path.to_str().unwrap().split("/").last().unwrap();
            let target_file = format!("{}/{}", home_dir.display(), filename);
            let _ = fs::remove_file(target_file);
        }
    }
}
