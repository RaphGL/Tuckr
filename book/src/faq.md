### How are Tuckr and Stow different?

Tuckr is sort of a spiritual successor to Stow. It adopts some of the functionality and the idioms people have used with Stow 
and attempts to make it more opinionated in hopes of making it easier to validate that your dotfiles are properly deployed.

|                                                       **Tuckr**                                                      |                                           **Stow**                                           |
|:--------------------------------------------------------------------------------------------------------------------:|:--------------------------------------------------------------------------------------------:|
| command based CLI                                                                                                    | flag based CLI                                                                               |
| has a set lookup path which can be overwritten, this means tuckr can be called from anywhere                         | symlinks everything from the current directory to the parent or a specified directory        |
| checks every single file and gives feedback on whether they're symlinked, not symlinked or is a symlink to elsewhere | doesn't provide much checking, instead preferring to be a simple tool                        |
| more complicated but made with dotfiles and scripting in mind, supports running hooks                                | very simple, compromises on error messages and wasn't initially built for dotfiles           |
| is multiplatform and supports conditional deployment based on the OS and OS family (including detecting WSL2)        | is mostly meant for Unix-like systems and it doesn't concern itself with detecting platforms |

Here's a noncomprehensive list of commands in both programs:

|                 **Action**                 |           **Tuckr**           |    **Stow**   |
|:------------------------------------------:|:-----------------------------:|:-------------:|
| Symlink a set of dotfiles                  | tuckr add <group>             | stow <dir>    |
| Remove dotfile symlinks                    | tuckr rm <group>              | stow -D <dir> |
| Dry run symlink files                      | tuckr -n add <group>          | stow -n <dir> |
| Check symlink status                       | tuckr status [group]          | *None*        |
| Run setup scripts and symlink files        | tuckr set [group]             | *None*        |
| Remove files from repo and back into $HOME | tuckr pop <group>             | *None*        |
| Add files to dotfiles repo                 | tuckr push <group> <files...> | *None*        |

### How do I share code between hooks?

Tuckr changes the current directory to the directory of the hook group that is currently being run.
So you can use relative paths to import code from the parent directory or anywhere else.
