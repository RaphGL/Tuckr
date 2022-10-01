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
