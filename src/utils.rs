use std::fs;
use std::path::PathBuf;

pub fn wildcard_matches<'a>(pattern: &str, string: &'a str) -> Option<&'a str> {
    let s = pattern.clone().split_once("*");
    if pattern == "*" || pattern == string {
        return Some(string);
    }
    if let Some(s) = s {
        if pattern.ends_with("*") {
            let s = s.0;
            if string.starts_with(s) {
                return Some(string);
            }
        } else if pattern.starts_with("*") {
            let s = s.1;
            if string.ends_with(s) {
                return Some(string);
            }
        }
    }
    None
}

pub fn get_dotfiles_path() -> Option<PathBuf> {
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
}

pub fn strip_away_program_path<'a>(fpath: &'a PathBuf) -> &'a str {
    let mut newstr = fpath.to_str().unwrap().split_once("Configs").unwrap();
    newstr = newstr.1.split_once("/").unwrap();
    newstr.1
}

// retrieves a vector with only the names of the programs
pub fn get_unique_config(paths: Vec<PathBuf>) -> Vec<String> {
    let mut programs: Vec<String> = Vec::new();

    for p in paths {
        let program_path = strip_away_program_path(&p)
            .split_once("/")
            .unwrap()
            .0
            .to_owned();
        if !programs.contains(&program_path) {
            programs.push(program_path);
        }
    }

    programs
}
