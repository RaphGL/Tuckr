<!-- PROJECT LOGO -->
<br />
<p align="center">
  <a href="https://github.com/RaphGL/Tuckr">
  </a>

  <h1 align="center">Tuckr</h1>
  <h3 align="center">A super powered replacement for GNU Stow</h3>
  <p align="center">
    <br />
    <a href="https://github.com/RaphGL/Tuckr/wiki"><strong>Explore the docs »</strong></a>
    <br />
    <br />
    ·
    <a href="https://github.com/RaphGL/Tuckr/issues">Report Bug</a>
    ·
    <a href="https://github.com/RaphGL/Tuckr/issues">Request Feature</a>
  </p>
</p>

<!-- TABLE OF CONTENTS -->
<details open="open">
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a>
      <ul>
        <li><a href="#how-it-works">How it works</a></li>
        <li><a href="#using-hooks">Using hooks</a></li>
        <li><a href="#using-secrets">Using secrets</a></li>
        <li><a href="#conditional-deployment">Conditional Deployment</a></li>
        <li><a href="#environment-variables">Environment variables</a></li>
        <li><a href="#exit-codes">Exit codes</a></li>
      </ul>
    </li>
    <li><a href="#license">License</a></li>
  </ol>
</details>

<!-- ABOUT THE PROJECT -->

Tuckr is a dotfile manager inspired by Stow and Git. Tuckr aims to make dotfile management less painful. It follows the same model as Stow, symlinking files onto $HOME. It works on all the major OSes (Linux, Windows, BSDs and MacOS).

Tuckr aims to bring the simplicity of Stow to a dotfile manager with a very small learning curve.
To achieve that goal Tuckr tries to only cover what is directly needed to manage dotfiles and nothing else. We won't wrap git, rm, cp or reimplement the functionality that are perfeclty covered by other utilities in the system unless it greatly impacts usability.

### Goals:

- No configuration
- Commands can be run from anywhere
- Symlinks are tracked and validated
- Configuration files are grouped and handled as a logical unit 
- Provide ability to run hooks (scripts) that facilitate deployment and uninstallation
- Easily encrypt and deploy sensitive configuration files (WIP: please do not secrets for production just yet)

<!-- GETTING STARTED -->

## Getting Started

Tuckr uses a `dotfiles` directory to store all your configs. Please check the table below to know where
to put it according to your operating system.

You can choose either the config or the home path.

| Platform       | Config Path                                | Home Path            |
| -------------- | ------------------------------------------ | -------------------- |
| Linux/BSDs/etc | $HOME/.config/dotfiles                     | $HOME/.dotfiles      |
| MacOS          | $HOME/Library/Application Support/dotfiles | $HOME/.dotfiles      |
| Windows        | %HomePath%\AppData\Roaming\dotfiles        | %HomePath%\\.dotfiles |

To learn how to set up your dotfiles, check the `How it works` sections.

#### Stow users

Tuckr is interchangeable with Stow. To migrate:

1. Go to your dotfiles directory remove all symlinks with

```
stow -t $HOME --delete *
```

2. Move your dotfiles directory to one of the valid paths and move all your directories there:

```
$ mkdir -p <CONFIG_PATH>/Configs
$ mv -t <CONFIG_PATH> * 
```

3. Resymlink your dotfiles with:

```
tuckr add \*
```

4. You can confirm that your dotfiles have been deployed:

```
tuckr status
```

#### Windows users

You need to enable developer mode for symlinking to work, this is a restriction imposed by the OS.

### Installation

**Install from source:**

```sh
cargo install --git https://github.com/RaphGL/Tuckr.git
```

**Install from crates.io:**
```sh
cargo install tuckr
```

Note: The binary will be installed to `$HOME/.cargo/bin` either move it to somewhere in your $PATH or add it to path.

**Install from the AUR:**

```sh
paru -S tuckr-git
```

<!-- USAGE EXAMPLES -->

## Usage

```sh
$ tuckr add \* # adds all dotfiles to the system
$ tuckr add \* -e neovim # adds all dotfiles except neovim
$ tuckr add neovim zsh # adds only the neovim and zsh dotfiles
$ tuckr set \* # adds all the dotfiles and runs their hooks (scripts)
$ tuckr rm \* # removes all dotfiles from your system
```

```
Super powered GNU Stow replacement

Usage: tuckr [OPTIONS] <COMMAND>

Commands:
  status   Get dotfiles' symlinking status (alias: s)
  add      Deploy dotfiles for the supplied groups (alias: a)
  rm       Remove dotfiles for the supplied groups
  set      Setup groups and run their hooks
  unset    Remove groups and run their cleanup hooks
  encrypt  Encrypt files and move them to dotfiles/Secrets (alias: e)
  decrypt  Decrypt files (alias: d)
  push     Copy files into groups
  pop      Remove groups from dotfiles/Configs
  ls       List dotfiles hooks, secrets, profiles
  init     Initialize dotfile directory
  groupis  Return the group files belongs to
  help     Print this message or the help of the given subcommand(s)

Options:
  -p, --profile <PROFILE>  Choose which dotfile profile to use
  -h, --help               Print help
  -V, --version            Print version
```

Note: for additional information also check [the wiki](https://github.com/RaphGL/Tuckr/wiki)

### How it works

Tuckr works with no configuration, this is achieved by making some assumptions about the structure of your dotfiles directory.
Every Tuckr dotfiles directory should have the following structure:

```sh
dotfiles
├── Configs # Dotfiles go here
├── Secrets # Encrypted files go here
└── Hooks # Setup scripts go here
```

These directories contain directories that separate the dotfiles by program name (or whatever criteria you want to group them by)

```sh
dotfiles
├── Configs
│   ├── tmux
│   └── zsh
└── Hooks
    ├── tmux
    └── zsh
```

Inside of these group directories the structure is exactly the same as what your $HOME looks like.

```sh
Configs
├── tmux
│   └── .config
│       └── tmux
│           └── tmux.conf
└── zsh
    ├── .zshenv
    └── .zshrc
```

The group directories' names are used to reference them on tuckr.

### Using Hooks

Hooks are run before and after adding every program, if they're coupled with a program they should their directory should have the same name in Hooks as in Configs.  
Hooks that run before symlinking the program are prefixed with `pre`, scripts that run afterwards are prefixed with `post`, as long as this is true you can name the file whatever you want.

```sh
Hooks
├── tmux
│   ├── post.sh
│   └── pre.sh
└── zsh
    ├── post.sh
    └── pre.sh
```

To run scripts for a program run `tuckr set <program_name>` or alternatively use a wildcard like so: `tuckr set \*` to run all hooks.

### Using Secrets
Please not that secrets are still WIP and their security is really not guaranteed. So it's best to avoid it in production. If you want to deploy secrets with tuckr,
then consider create a hook that deploys secrets for you using some of the reputable encryption tools out there like veracrypt, gpg, etc.

#### Encrypting files

Encrypt a file and put it in <group_name>

```
tuckr encrypt <group_name> <file_name...>
```

This will create an appropriate file in the `Secrets` directory pointing to the path where it originally came from

#### Decrypting files

Decrypt files from the groups <group_name...> and put them on their appropriate paths

```
tuckr decrypt <group_name...>
```

### Conditional deployment
Conditional deployment is used when a dotfile should only be deployed on a specific platform. This is done by creating a separate group with the same name suffixed with the desired platform.

Conditional groups are entirely ignored on unsupported systems.

Example:

```
Configs
├── config
├── config_unix # deployed on any unix system
├── config_linux # only on linux
├── config_macos # only on macos
├── config_windows # only on windows
└── config_wsl # only on Windows Subsystem for Linux (WSL)
```

The groups that are supported on the target system will be treated as being a part of the original `config` group. One only needs to reference it to have all of the valid ones included as well.

Any of the [options available](https://doc.rust-lang.org/reference/conditional-compilation.html#target_os) on Rust's `target_family` and `target_os` are valid targets.

### Environment variables
You might want to dynamically decide where to deploy a dotfile, for example a program might allow changing where the dotfiles will be or you might want to choose the location on a per machine basis.
Tuckr allows you to use environment variables to decide where to place things.
Anything prefixed with a `%` is considered an environment variable.

For example:
```
program
└── %PROGRAM_PATH
    └── config.txt
```

The `%PROGRAM_PATH` will be expanded to whatever was in the environment variable. So if you had `%PROGRAM_PATH/config.txt` and the environment variable was set as `/home/user/Documents`, the Tuckr will attempt to symlink it to `/home/user/Documents/config.txt`.

### Root targeting
Some dotfiles are stored in some directory that outside of your home directory, by default tuckr doesn't support those, but by using the `^` it tells tuckr that it should target the root.
So say you wanted to manage your crontab settings with tuckr, you would need to somehow symlink `/etc/crontab`. The file structure to do that would be:

```
crontab
└── ^etc
    └── crontab
```

The `^etc/crontab` would be expanded to `/etc/crontab`.


### Exit codes

For scripting purposes Tuckr has the following exit codes:

- `2` Could not find Dotfiles directory
- `3` No Configs/Hooks/Secrets directory setup
- `4` No such file or directory exists
- `5` Encryption failed
- `6` Decryption failed

On success Tuckr returns whatever is the default success return code for the platform (0 on unix-like systems).

<!-- LICENSE -->

## License

Distributed under GPLv3 License. See [`LICENSE`](https://github.com/RaphGL/Tuckr/blob/main/LICENSE) for more information.
