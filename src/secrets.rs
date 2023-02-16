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
use std::process::ExitCode;

struct SecretsHandler {
    dotfiles_dir: PathBuf,
    key: chacha20poly1305::Key,
    nonce: chacha20poly1305::XNonce,
}

impl SecretsHandler {
    fn try_new() -> Result<Self, ExitCode> {
        // makes a hash of the password so that it can fit on the 256 bit buffer used by the
        // algorithm
        let input_key = rpassword::prompt_password("Password: ").unwrap();
        let mut input_key = input_key.trim().as_bytes().to_vec();
        let mut hasher = Sha256::new();

        hasher.update(&input_key);
        let input_hash = hasher.finalize();

        // zeroes sensitive information from memory
        input_key.zeroize();

        let dotfiles_dir = match utils::get_dotfiles_path() {
            Some(dir) => dir,
            None => {
                eprintln!("{}", "Couldn't find dotfiles directory".red());
                return Err(ExitCode::from(utils::COULDNT_FIND_DOTFILES));
            }
        };

        Ok(SecretsHandler {
            dotfiles_dir,
            key: input_hash,
            nonce: XChaCha20Poly1305::generate_nonce(&mut OsRng),
        })
    }

    /// takes a path to a file and returns its encrypted content
    fn encrypt(&self, dotfile: &str) -> Result<Vec<u8>, ExitCode> {
        let cipher = XChaCha20Poly1305::new(&self.key);
        let dotfile = match fs::read(dotfile) {
            Ok(f) => f,
            Err(_) => {
                eprintln!(
                    "{}",
                    format!("{} {}", "No such file or directory: ", dotfile).red()
                );
                return Err(ExitCode::from(utils::NO_SUCH_FILE_OR_DIR));
            }
        };

        match cipher.encrypt(&self.nonce, dotfile.as_slice()) {
            Ok(f) => Ok(f),
            Err(e) => {
                eprintln!("{}", e.red());
                Err(ExitCode::from(utils::ENCRYPTION_FAILED))
            }
        }
    }

    /// takes a path to a file and returns its decrypted content
    fn decrypt(&self, dotfile: &str) -> Result<Vec<u8>, ExitCode> {
        let cipher = XChaCha20Poly1305::new(&self.key);
        let dotfile = fs::read(dotfile).expect("Couldn't read dotfile");

        // extracts the nonce from the first 24 bytes in the file
        let (nonce, contents) = dotfile.split_at(24);

        match cipher.decrypt(nonce.into(), contents) {
            Ok(f) => Ok(f),
            Err(_) => {
                eprintln!("{}", "Wrong password.".red());
                Err(ExitCode::from(utils::DECRYPTION_FAILED))
            }
        }
    }
}

/// Encrypts secrets
pub fn encrypt_cmd(group: &str, dotfiles: &[String]) -> Result<(), ExitCode> {
    let handler = SecretsHandler::try_new()?;

    let dest_dir = handler.dotfiles_dir.join("Secrets").join(group);
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).unwrap();
    }

    let home_dir = dirs::home_dir().unwrap();

    for dotfile in dotfiles {
        let mut encrypted = handler.encrypt(dotfile)?;

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

    Ok(())
}

/// Decrypts secrets
pub fn decrypt_cmd(groups: &[String], exclude: &[String]) -> Result<(), ExitCode> {
    let handler = SecretsHandler::try_new()?;

    let dest_dir = std::env::current_dir().unwrap();

    let decrypt_group = |group: &String| -> Result<(), ExitCode> {
        if exclude.contains(group) {
            return Ok(());
        }

        let group_dir = handler.dotfiles_dir.join("Secrets").join(group);
        for secret in WalkDir::new(group_dir) {
            let secret = match secret {
                Ok(secret) => secret,
                Err(_) => {
                    eprintln!("{}", (group.to_owned() + " does not exist.").red());
                    return Err(ExitCode::from(utils::NO_SETUP_FOLDER));
                }
            };

            if secret.file_type().is_dir() {
                continue;
            }

            let decrypted = handler.decrypt(secret.path().to_str().unwrap())?;

            fs::write(dest_dir.join(secret.file_name()), decrypted).unwrap();
        }

        Ok(())
    };

    if groups.contains(&"*".to_string()) {
        let groups_dir = handler.dotfiles_dir.join("Secrets");
        for group in fs::read_dir(groups_dir).unwrap() {
            let group = group.unwrap().file_name();
            decrypt_group(&group.to_str().unwrap().to_string())?;
        }

    }

    for group in groups {
        decrypt_group(group)?;
    }

    Ok(())
}
