use std::env;
use std::fs;
use std::io;
use std::io::Write;

// converts a stow repo into a tuckr one
pub fn from_stow() {
    // get user confirmation
    print!("Tuckr: Would you like to convert the repo? [y/N]: ");
    io::stdout().flush().unwrap();

    let mut ans = String::new();
    io::stdin()
        .read_line(&mut ans)
        .expect("Could not read input");
    ans = ans.to_lowercase();

    if ans.trim() == "y" {
        let mut path: String;
        let curr_path = env::current_dir().unwrap().to_str().unwrap().to_owned();
        let curr_dir = fs::read_dir(&curr_path).expect("Could not read current directory");

        let _ = fs::create_dir(format!("{}/{}", curr_path, "Configs"));
        let _ = fs::create_dir(format!("{}/{}", curr_path, "Hooks"));
        let _ = fs::create_dir(format!("{}/{}", curr_path, "Encrypts"));

        for dir in curr_dir {
            let d = dir.unwrap();
            if d.file_name().to_str().unwrap().starts_with('.') {
                continue;
            }

            // new path for the file
            path = format!(
                "{}/{}/{}",
                curr_path,
                "Configs",
                d.file_name().to_str().unwrap()
            );

            let dname = d.file_name().to_str().unwrap().to_owned();
            if !dname.ends_with("Configs") && !dname.ends_with("Hooks") && !dname.ends_with("Encrypts") {
                fs::rename(d.path().to_str().unwrap(), path).expect("Could not move files");
            }
        }
    }
}

// creates all the folder and files necessary for Tuckr
pub fn init_tuckr_dir() {
    let curr_dir = env::current_dir().unwrap();
    let mut curr_dir = fs::read_dir(curr_dir).unwrap();

    if !curr_dir.next().is_none() {
        println!("Current directory is not empty. Please empty it before initializing tuckr.");
        return;
    } else {
        let _ = fs::create_dir("Configs");
        let _ = fs::create_dir("Hooks");
        let _ = fs::create_dir("Encrypts");
    }
}
