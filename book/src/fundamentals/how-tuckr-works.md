# How Tuckr works

Tuckr mandates that all dotfiles are under a `dotfiles` directory. This directory should be on your systems standard location for configuration files.
On Unix-like OSes `$HOME/.dotfiles` is also permitted, but is discouraged.

Tuckr looks up the following paths for dotfiles:

| Platform       | Config Path                                  | Home Path               |
| -------------- | ------------------------------------------   | --------------------    |
| Linux/BSDs/etc | `$HOME/.config/dotfiles`                     | `$HOME/.dotfiles`       |
| MacOS          | `$HOME/Library/Application Support/dotfiles` | `$HOME/.dotfiles`       |
| Windows        | `%HomePath%\AppData\Roaming\dotfiles`        | `%HomePath%\\.dotfiles` |

The dotfiles directory itself has the following structure:
```sh
dotfiles
├── Configs # Stores all configuration files
├── Secrets # Stores encrypted files
└── Hooks # Stores scripts that configure the system
```

## Dotfile validation
All dotfiles are stored inside of the `Configs` directory. Inside this directory you will find a directory for each program or group of dotfiles (I recommend making each group the dotfiles for a specific program, but you can group them however you want).
Tuckr will use these groups to same files that are already in your `$HOME` directory:
- if it points to the dotfile in the repo it's deemed as `symlinked`
- if it's not symlinked it's deemed as `unsymlinked`
- if it's a symlink but points somewhere else it's put on the `not_owned` bucket to indicate that it's in conflict

All these checks are done right after the program's execution starts. Once the entire repo is mapped to either one of those states. The actual command execution starts.

Some commands work only on dotfiles with certain statuses, such as:
- `add`: works on `not_symlinked` and `not_owned` if conflicts are being resolved
- `rm`: works on `symlinked`

## Hooks
Hooks are scripts used to configure and clean up your dotfiles.
You can run hooks before and after adding a dotfile, you can also run a hook when removing a dotfile.
These hooks are only called if using the `set` and `unset` commands.

The hooks are stored in `dotfiles/Hooks` and when they should be run is determined by their suffixes:
- `pre_`: run before adding dotfiles
- `post_`: run after adding dotfiles
- `rm_`: run when removing dotfiles

Most scripts will likely not matter if they're run before or after having the dotfiles symlinked, but sometimes this is useful. 

## Secrets

Secrets use is discouraged right now, they're the least maintained part of the program as I've spent most of the time making sure the other parts of the program are correct and useful.

Secrets might end up being removed later on or improved enough that I might start recommending people to use it.
