use std::fs;
use std::env;

pub fn from_stow() {}

pub fn init_tuckr_dir() {
    let curr_dir = env::current_dir().unwrap();
    let mut curr_dir = fs::read_dir(curr_dir).unwrap();
    if !curr_dir.next().is_none() {
        println!("Current directory is not empty. Please empty it before initializing tuckr.");
        return;
    } else {
        fs::create_dir("Configs").unwrap();
        fs::create_dir("Hooks").unwrap();
        fs::create_dir("Encrypts").unwrap();
    }
}