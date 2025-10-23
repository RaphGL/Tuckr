# Conditional groups

Conditional deployment is used when a dotfile should only be deployed on a specific platform.
All you have to do is add a suffix for the platform where the dotfile is valid on and it will be ignored on every other platform by tuckr.

Example:
```
Configs
├── config
├── config_unix          // deployed on any unix system
├── config_linux         // only on linux
├── config_macos         // only on macos
├── config_windows       // only on windows
└── config_wsl           // only on Windows Subsystem for Linux (WSL)
```
The conditional groups are treated as if they were files in the regular base group.
So if you were on windows the `config` and `config_windows` groups are treated as a single group, if you had a `config_linux`, it would be as if the files in this group didn't exist at all.

If use only the conditional group explictly, you can do that as well, for example: `tuckr rm config_linux` (this won't remove `config`)

## Dotfile fallback
A nice thing about conditional dotfiles is that they can fallback.
You can have the same config file multiple times in different conditional groups and tuckr won't complain. It will just choose the most specific one for your platform.

This is possible because the suffixes have a priority order.
The order is defined like this:
1. Custom targets
2. Windows Subsystem for Linux
3. OS name (e.g: linux, freebsd, macos)
4. OS family: unix, windows 
5. Non conditional groups (groups without a conditional suffix)   

> Any of the [options available](https://doc.rust-lang.org/reference/conditional-compilation.html#target_os) on Rust's `target_family` and `target_os` are valid targets.


