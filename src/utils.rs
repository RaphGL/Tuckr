use std::fs;
use std::path;

/// Converts a path string from pointing to their config in the dotfiles to where they should be
/// deployed on $HOME
pub fn to_home_path(path: &str) -> String {
    // uses join("") so that the path appends / or \ depending on platform
    let dotfiles_configs_path = path::PathBuf::from("dotfiles").join("Configs").join("");
    let dotfiles_configs_path = dotfiles_configs_path.to_str().unwrap();

    dirs::home_dir()
        .unwrap()
        .join(
            path.split_once(dotfiles_configs_path)
                .unwrap()
                .1
                .split_once(path::MAIN_SEPARATOR)
                .unwrap()
                .1,
        )
        .to_str()
        .unwrap()
        .to_string()
}

/// Converts paths from dotfiles/Hooks and Configs to their target destination at $HOME
pub fn to_program_name(path: &str) -> Option<&str> {
    let dir = if path.contains("Configs") {
        "Configs"
    } else if path.contains("Hooks") {
        "Hooks"
    } else {
        return None;
    };

    // uses join("") so that the path appends / or \ depending on platform
    let dotfiles_configs_path = path::PathBuf::from("dotfiles").join(dir).join("");

    Some(
        path.split_once(dotfiles_configs_path.to_str().unwrap())
            .unwrap()
            .1,
    )
}

/// Goes through each file in the program_dir and applies the function
pub fn file_or_xdgdir_map<F: FnMut(fs::DirEntry)>(file: fs::DirEntry, mut func: F) {
    match file.file_name().to_str().unwrap() {
        // Special folders that should not be handled directly ("owned" by the system)
        // Instead everything inside of it should be handled instead
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

/// Prints a single row info box with title on the left
/// and content on the right
pub fn print_info_box(title: &str, content: &str) {
    let mut hook_box = tabled::builder::Builder::default()
        .set_columns([title])
        .add_record([content])
        .to_owned()
        .build();
    hook_box
        .with(tabled::Rotate::Left)
        .with(tabled::Style::rounded().off_vertical());
    println!("{}", hook_box);
}

#[cfg(test)]
mod tests {
    #[test]
    fn to_home_path() {
        assert_eq!(
            // /home/$USER/.config/dotfiles/Configs/zsh/.zshrc
            super::to_home_path(
                dirs::config_dir()
                    .unwrap()
                    .join("dotfiles")
                    .join("Configs")
                    .join("zsh")
                    .join(".zshrc")
                    .to_str()
                    .unwrap()
            ),
            // /home/$USER/.zshrc
            dirs::home_dir().unwrap().join(".zshrc").to_str().unwrap()
        );
        assert_eq!(
            // /home/$USER/.config/dotfiles/Configs/zsh/.config/$PROGRAM
            super::to_home_path(
                dirs::config_dir()
                    .unwrap()
                    .join("dotfiles")
                    .join("Configs")
                    .join("zsh")
                    .join(".config")
                    .join("program")
                    .to_str()
                    .unwrap()
            ),
            // /home/$USER/.config/$PROGRAM
            dirs::config_dir()
                .unwrap()
                .join("program")
                .to_str()
                .unwrap()
        );
    }

    #[test]
    fn to_program_name() {
        assert_eq!(
            super::to_program_name(
                // /home/$USER/.config/dotfiles/Configs/zsh
                dirs::config_dir()
                    .unwrap()
                    .join("dotfiles")
                    .join("Configs")
                    .join("zsh")
                    .to_str()
                    .unwrap()
            )
            .unwrap(),
            "zsh"
        );
        assert_eq!(
            super::to_program_name(
                // /home/$USER/.config/dotfiles/Hooks/zsh
                dirs::config_dir()
                    .unwrap()
                    .join("dotfiles")
                    .join("Hooks")
                    .join("zsh")
                    .to_str()
                    .unwrap()
            )
            .unwrap(),
            "zsh"
        );
    }
}
