//! Manages encrypted files
//!
//! Encrypts files into dotfiles/Secrets using the chacha20poly1305 algorithm

use crate::utils;
use chacha20poly1305::{aead::Aead, AeadCore, KeyInit, XChaCha20Poly1305};
use owo_colors::OwoColorize;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zeroize::Zeroize;

struct SecretsHandler {
    dotfiles_dir: PathBuf,
    key: chacha20poly1305::Key,
    nonce: chacha20poly1305::XNonce,
}

impl SecretsHandler {
    fn new() -> Self {
        // makes a hash of the password so that it can fit on the 256 bit buffer used by the
        // algorithm
        let input_key = rpassword::prompt_password("Password: ").unwrap();
        let mut input_key = input_key.trim().as_bytes().to_vec();
        let mut hasher = Sha256::new();

        hasher.update(&input_key);
        let input_hash = hasher.finalize();

        // zeroes sensitive information from memory
        input_key.zeroize();

        SecretsHandler {
            dotfiles_dir: utils::get_dotfiles_path().unwrap_or_else(|| {
                eprintln!("{}", "Couldn't find dotfiles directory".red());
                std::process::exit(utils::COULDNT_FIND_DOTFILES);
            }),
            key: input_hash,
            nonce: XChaCha20Poly1305::generate_nonce(&mut OsRng),
        }
    }

    /// takes a path to a file and returns its encrypted content
    fn encrypt(&self, dotfile: &str) -> Vec<u8> {
        let cipher = XChaCha20Poly1305::new(&self.key);
        let dotfile = match fs::read(dotfile) {
            Ok(f) => f,
            Err(_) => {
                eprintln!(
                    "{}",
                    format!("{} {}", "No such file or directory: ", dotfile).red()
                );
                std::process::exit(utils::NO_SUCH_FILE_OR_DIR);
            }
        };

        match cipher.encrypt(&self.nonce, dotfile.as_slice()) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{}", e.red());
                std::process::exit(utils::ENCRYPTION_FAILED);
            }
        }
    }

    /// takes a path to a file and returns its decrypted content
    fn decrypt(&self, dotfile: &str) -> Vec<u8> {
        let cipher = XChaCha20Poly1305::new(&self.key);
        let dotfile = fs::read(dotfile).expect("Couldn't read dotfile");

        // extracts the nonce from the first 24 bytes in the file
        let (nonce, contents) = dotfile.split_at(24);

        match cipher.decrypt(nonce.into(), contents) {
            Ok(f) => f,
            Err(_) => {
                eprintln!("{}", "Wrong password.".red());
                std::process::exit(utils::DECRYPTION_FAILED);
            }
        }
    }
}

/// Encrypts secrets
pub fn encrypt_cmd(group: &str, dotfiles: &[String]) {
    let handler = SecretsHandler::new();
    let dest_dir = handler.dotfiles_dir.join("Secrets").join(group);
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).unwrap();
    }

    let home_dir = dirs::home_dir().unwrap();

    for dotfile in dotfiles {
        let mut encrypted = handler.encrypt(dotfile);
        let mut encrypted_file = handler.nonce.to_vec();

        let target_file = Path::new(dotfile).canonicalize().unwrap();
        let target_file = target_file.strip_prefix(&home_dir).unwrap();

        let mut dir_path = target_file.to_path_buf();
        dir_path.pop();

        // makes sure all parent directories of the dotfile are created
        fs::create_dir_all(dest_dir.join(dir_path)).unwrap();

        // appends a 24 byte nonce to the beginning of the file
        encrypted_file.append(&mut encrypted);
        fs::write(dest_dir.join(target_file), encrypted_file).unwrap();
    }
}

/// Decrypts secrets
pub fn decrypt_cmd(groups: &[String], exclude: &[String]) {
    let handler = SecretsHandler::new();
    let dest_dir = std::env::current_dir().unwrap();

    let decrypt_group = |group: &String| {
        let groupname = &utils::to_program_name(group).unwrap().to_string();
        if exclude.contains(groupname) {
            return;
        }

        let group_dir = handler.dotfiles_dir.join("Secrets").join(group);
        for secret in WalkDir::new(group_dir) {
            let secret = secret.unwrap();
            if secret.file_type().is_dir() {
                continue;
            }

            let decrypted = handler.decrypt(secret.path().to_str().unwrap());

            fs::write(dest_dir.join(secret.file_name()), decrypted).unwrap();
        }
    };

    if groups.contains(&"*".to_string()) {
        let groups_dir = handler.dotfiles_dir.join("Secrets");
        for group in fs::read_dir(groups_dir).unwrap() {
            let group = group.unwrap().path().to_str().unwrap().to_string();
            decrypt_group(&group);
        }

        return;
    }

    for group in groups {
        decrypt_group(group);
    }
}
