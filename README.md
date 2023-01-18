<!-- PROJECT LOGO -->
<br />
<p align="center">
  <a href="https://github.com/RaphGL/Tuckr">
    <!-- <img src="logo.png" alt="Logo" height="80"> -->
  </a>

  <h3 align="center">A super powered replacement for GNU Stow</h3>
  <p align="center">
    <br />
    <a href="https://github.com/RaphGL/Tuckr"><strong>Explore the docs »</strong></a>
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
        <li><a href="#exit-codes">Using secrets</a></li>
      </ul>
    </li>
    <li><a href="#license">License</a></li>
  </ol>
</details>

<!-- ABOUT THE PROJECT -->

Tuckr is a dotfile manager inspired by Stow and Git. Tuckr aims to make dotfile management less painful. It follows the same model as Stow, symlinking files onto $HOME. It works on all the major OSes (Linux, Windows, BSDs and MacOS).  

Managing dotfiles is something that's done every once in a while and should not require one to go and study complex tooling just to be able to deploy dotfiles.

To achieve that goal Tuckr tries to only cover what is directly needed to manage dotfiles and nothing more. We won't wrap git, rm, cp or reimplement the functionality of a perfectly functioning separate utility unless it greatly impacts usability.

**What makes tuckr different?**

- No configuration required
- Tuckr always knows where your dotfiles are, you don't have to cd into them
- Symlinks are tracked and validated, you're informed of conflicts and can easily handle (or not handle them)
- Hooks, write scripts to handle additional configuration
- Encrypt your sensitive configuration and files

<!-- GETTING STARTED -->

## Getting Started
The following paths should be used for your dotfiles:  

Dotfile Path in each OS:
| Platform       | Config Path                                       | Home Path               |
|----------------|---------------------------------------------------|-------------------------|
| Linux/BSDs/etc | /home/user/.config/dotfiles                      | /home/user/.dotfiles    |
| MacOS          | /Users/User/Library/Application Support/dotfiles | /Users/User/.dotfiles   |
| Windows        | C:\Users\User\AppData\Roaming/dotfiles           | C:\Users\Alice/.dotfiles |

Create the required directories:
```
tuckr init
```

#### Stow users
Tuckr is interchangeable with Stow. To migrate:  
1. Open your dotfiles repo and remove the symlinks with `stow -t $HOME --delete *`
2. Run `tuckr from-stow`
3. Move your repo to `$HOME/.dotfiles` or `$HOME/.config/dotfiles`
4. Resymlink your dotfiles with `tuckr add \*`

#### Windows users
You need to enable developer mode for the symlinking to work.  


### Installation  

**Install from source:**
```sh
cargo install --git https://github.com/RaphGL/Tuckr.git
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
$ tuckr add neovim zsh # adds the neovim and zsh dotfiles only
$ tuckr set \* # adds all the dotfiles and runs their hooks (scripts)
$ tuckr rm \* # removes all dotfiles from your system
```

```
Super powered GNU Stow replacement

Usage: tuckr <COMMAND>

Commands:
  set        Setup the program and run its hooks
  add        Deploy dotfiles for the given program (alias: a)
  rm         Remove dotfiles for a program
  status     Print a status message for all dotfiles (alias: s)
  encrypt    Encrypts files and moves it to dotfiles/Secrets
  decrypt    Decrypts files
  init       Initialize dotfile directory
  from-stow  Converts a GNU Stow repo into a Tuckr one
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### How it works  
Tuckr works with no configuration, this is achieved by making some assumptions about the structure of your dotfiles directory.
Every Tuckr dotfiles directory should have the following structure:  
```sh
dotfiles
├── Configs # Dotfiles go here
├── Secrets # Encrypted files go here
└── Hooks # Setup scripts go here
```

These directories contain directories that separate the dotfiles by program name (or whatever you want to separate them by)
```
dotfiles
├── Configs
│   ├── tmux
│   └── zsh 
└── Hooks
    ├── tmux 
    └── zsh 
```

Inside of these program directories the structure is exactly the same as what your $HOME looks like.
```
Configs
├── tmux
│   └── .config
│       └── tmux
│           └── tmux.conf
└── zsh
    ├── .zshenv
    └── .zshrc
```

The program directories' names are used to reference them in commands

### Using Hooks
Hooks are run before and after adding every program, if they're coupled with a program they should their directory should have the same name in Hooks as in Configs.  
Hooks that run before symlinking the program are prefixed with `pre`, scripts that run afterwards are prefixed with `post`, as long as this is true you can name the file whatever you want.

```
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

### Exit codes
For scripting purposes Tuckr has the following exit codes:  
- `2` Could not find Dotfiles directory
- `3` No Configs/Hooks/Secrets directory setup
- `4` No such file or directory exists
- `5` Encryption failed
- `6` Decryption failed

<!-- LICENSE -->

## License

Distributed under GPLv3 License. See [`LICENSE`](https://github.com/RaphGL/Tuckr/blob/main/LICENSE) for more information.
