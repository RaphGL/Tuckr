use std::path::PathBuf;
use std::fs;

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
