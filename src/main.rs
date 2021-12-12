pub mod symlink {
    use colored::*;
    use std::os::unix::fs::symlink;
    use std::{env, path, fs};
    use dirs;

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

    pub fn create(fpath: &str) {
        let curpath = format!("{}/{}", env::current_dir().ok().unwrap().display(), fpath);
        if let Some(x) = dirs::home_dir() {
            let home = format!("{}/{}", x.display(), fpath);
            let _ = symlink(curpath, home); // ignore if it fails
        }
    }

    pub fn remove(fpath: &str) {
        if let Some(x) = dirs::home_dir() {
            let home = format!("{}/{}", x.display(), fpath);
            fs::remove_file(home).ok();
        }
    }
}

fn main() {
    symlink::remove("src");
}
