use std::fs;

/// Converts a path string from pointing to their config in the dotfiles to where they should be
/// deployed on $HOME
pub fn to_home_path(path: &str) -> String {
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

pub fn to_program_name(path: &str) -> Option<&str> {
    let dir: &str;
    if path.contains("Configs") {
        dir = "Configs"
    } else if path.contains("Hooks") {
        dir = "Hooks"
    } else {
        return None;
    }

    Some(
        path.split_once(format!("dotfiles/{}/", dir).to_string().as_str())
            .unwrap()
            .1,
    )
}

/// Goes through each file in the program_dir and applies the function
pub fn file_or_xdgdir_map<F: FnMut(fs::DirEntry)>(file: fs::DirEntry, mut func: F) {
    match file.file_name().to_str().unwrap() {
        ".config" | "Pictures" | "Documents" | "Desktop" | "Downloads" | "Public" | "Templates"
        | "Videos" => {
            for file in fs::read_dir(file.path()).unwrap() {
                func(file.unwrap());
            }
        }

        _ => {
            func(file);
        }
    }
}
