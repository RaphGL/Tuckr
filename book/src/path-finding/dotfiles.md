## Dotfiles paths

By default tuckr will use your `$HOME` to symlink things and your dotfiles have to be on the standard config directory for your platform.
This is generally the most desirable behavior. But if you don't like this and/or need to use other directories, you can set the `TUCKR_HOME` and `TUCKR_TARGET` environment variables to override the default behavior. 

- `TUCKR_HOME`: 
parent directory for your dotfiles. It assumes that your dotfiles directory is named as `dotfiles`.
- `TUCKR_TARGET`:
the base directory from which all dotfiles will be deployed (with the exception of root and environment variable paths). By default this is `$HOME` on UNIX-like systems and `%USERPROFILE%` on Windows.

Another use case for overriding these is if you're managing dotfiles for many users, you can then set these variables to point to those users' directories.

