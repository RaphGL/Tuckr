//! Manages encrypted files
//!
//! Encrypts files into dotfiles/Secrets using the age library

use crate::Context;
use crate::dotfiles::{self, Dotfile, ReturnCode};
use crate::fileops::DirWalk;
use owo_colors::OwoColorize;
use rust_i18n::t;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use age::secrecy::SecretString;

fn get_dotfiles_dir(ctx: &Context) -> Result<PathBuf, ExitCode> {
        let dotfiles_dir = match dotfiles::get_dotfiles_path(ctx.profile.clone()) {
            Ok(path) => path,
            Err(e) => {
                eprintln!("{e}");
                return Err(ReturnCode::CouldntFindDotfiles.into());
            }
        };
        Ok(dotfiles_dir)
}

fn read_passphrase()-> Result<SecretString, ExitCode>{
        let input_key = rpassword::prompt_password(format!("{}: ", t!("info.password"))).unwrap();
        Ok(SecretString::from(input_key))
}
fn encrypt(recipient: &age::scrypt::Recipient, dotfile: &Path) -> Result<Vec<u8>, ExitCode> {
    let Ok(dotfile) = fs::read(dotfile) else {
        eprintln!(
            "{}",
            t!("errors.x_doesnt_exist", x = dotfile.display()).red()
        );
        return Err(ReturnCode::NoSuchFileOrDir.into());
    };

    age::encrypt(recipient, &dotfile).map_err(|e|{
        eprintln!("{}", e.red());
        ReturnCode::EncryptionFailed.into()
    })
}

fn decrypt(identity: &age::scrypt::Identity, dotfile: &Path) -> Result<Vec<u8>, ExitCode> {
    let Ok(dotfile) = fs::read(dotfile) else {
            eprintln!("{}", t!("errors.could_not_read_enc", path = dotfile.display()).red());
            return Err(ReturnCode::EncryptedReadFailed.into())
    };

    age::decrypt(identity, &dotfile).map_err(|_|{
        eprintln!("{}", t!("errors.wrong_password").red());
        ReturnCode::DecryptionFailed.into()
    })
}

/// Encrypts secrets
pub fn encrypt_cmd(ctx: &Context, group: &str, dotfiles: &[String]) -> Result<(), ExitCode> {
    {
        let mut invalid_dotfiles = false;
        for dotfile in dotfiles {
            if !Path::new(dotfile).exists() {
                eprintln!("{}", t!("errors.x_doesnt_exist", x = dotfile).red());
                invalid_dotfiles = true;
            }
        }

        if invalid_dotfiles {
            return Err(ExitCode::FAILURE);
        }
    }

    //let handler = SecretsHandler::try_new(profile)?;
    let passphrase = read_passphrase()?;
    let recipient = age::scrypt::Recipient::new(passphrase);

    let dest_dir = get_dotfiles_dir(ctx)?.join("Secrets").join(group);
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).unwrap();
    }

    let target_dir = match dotfiles::get_dotfiles_target_dir_path() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("{}", err.red());
            return Err(ReturnCode::NoSuchFileOrDir.into());
        }
    };

    let encrypt_file = |dotfile: &Path| -> Result<(), ExitCode> {
        let target_file = dotfile.strip_prefix(&target_dir).unwrap();

        let encrypted_file_path = dest_dir.join(target_file);
        if ctx.dry_run {
            eprintln!(
                "{}",
                t!(
                    "dry-run.encrypting_x_into_y",
                    x = dotfile.display(),
                    y = encrypted_file_path.display()
                )
                .green()
            );
            return Ok(());
        }

        let dir_path = {
            let mut tf = target_file.to_path_buf();
            tf.pop();
            tf
        };

        let encrypted_file = encrypt(&recipient, dotfile)?;

        // makes sure all parent directories of the dotfile are created
        fs::create_dir_all(dest_dir.join(dir_path)).unwrap();
        fs::write(encrypted_file_path, encrypted_file).unwrap();

        Ok(())
    };

    for dotfile in dotfiles {
        let dotfile = Path::new(dotfile).canonicalize().unwrap();

        if dotfile.is_dir() {
            let Ok(dir) = dotfile.read_dir() else {
                eprintln!("{}", t!("errors.x_doesnt_exist", x = dotfile.display()));
                return Err(ExitCode::FAILURE);
            };

            for file in dir {
                let file = file.unwrap().path();
                encrypt_file(&file)?;
            }
        } else if dotfile.is_file() {
            encrypt_file(&dotfile)?;
        }
    }

    Ok(())
}

/// Decrypts secrets
pub fn decrypt_cmd(
    ctx: &Context,
    groups: &[String],
    exclude: &[String],
) -> Result<(), ExitCode> {
    let passphrase = read_passphrase()?;
    let identity = age::scrypt::Identity::new(passphrase);
    let dotfiles_dir = get_dotfiles_dir(ctx)?;

    if let Some(nonexistent_groups) = dotfiles::get_nonexistent_groups(
        ctx.profile.clone(),
        dotfiles::DotfileType::Secrets,
        groups,
    ) {
        for group in nonexistent_groups {
            eprintln!("{}", t!("errors.no_group", group = group).red());
        }
        return Err(ReturnCode::DecryptionFailed.into());
    }

    let target_dir = match dotfiles::get_dotfiles_target_dir_path() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("{}", err.red());
            return Err(ReturnCode::NoSuchFileOrDir.into());
        }
    };

    let decrypt_group = |group: Dotfile| -> Result<(), ExitCode> {
        if exclude.contains(&group.group_name) || !group.is_valid_target(&ctx.custom_targets) {
            return Ok(());
        }

        let group_dir = dotfiles_dir.join("Secrets").join(&group.group_path);
        for secret in DirWalk::new(&group_dir) {
            if secret.is_dir() {
                continue;
            }

            let base_secret_path = secret.strip_prefix(&group_dir).unwrap();
            let decrypted_dest = target_dir.join(base_secret_path);

            if ctx.dry_run {
                eprintln!(
                    "{}",
                    t!(
                        "dry-run.decrypting_x_into_y",
                        x = secret.display(),
                        y = decrypted_dest.display()
                    )
                    .green()
                );
                continue;
            }

            let decrypted_parent_dir = decrypted_dest.parent().unwrap();
            fs::create_dir_all(decrypted_parent_dir).unwrap();

            let decrypted = decrypt(&identity, &secret)?;
            fs::write(decrypted_dest, decrypted).unwrap();
        }

        Ok(())
    };

    if groups.contains(&"*".to_string()) {
        let groups_dir = dotfiles_dir.join("Secrets");
        for group in fs::read_dir(groups_dir).unwrap() {
            let Ok(group) = Dotfile::try_from(group.unwrap().path()) else {
                eprintln!("{}", t!("errors.got_invalid_group").red());
                return Err(ExitCode::FAILURE);
            };
            decrypt_group(group)?;
        }

        return Ok(());
    }

    for group in groups {
        let group = dotfiles_dir.join("Secrets").join(group);
        let Ok(group) = Dotfile::try_from(group) else {
            eprintln!("{}", t!("errors.got_invalid_group").red());
            return Err(ExitCode::FAILURE);
        };
        decrypt_group(group)?;
    }

    Ok(())
}
