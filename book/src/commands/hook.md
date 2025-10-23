# Hook commands

Hooks are scripts that are run to set up your environment.
A hook is essentially directory in the `dotfiles/Hooks` directory. To create a hook foo, one creates a directory `dotfiles/Hooks/foo` and put scripts there.
Those scripts have to be prefixed with either `pre_`, `post_` or `rm_`, if they're not prefixed they're ignored by tuckr.

Hooks are run in 3 step: pre-hooking -> symlinking -> post-hooking.
They are associated with Configs groups of the same name. So if one has a `Configs/foo` and a `Hooks/foo`. So the `Configs/foo` group is symlinked after a successful run of the pre-hook.

## Running setup hooks
Hooks are run by using the following command:
```sh
$ tuckr set foo
```

All setup hooks are run if the following command is used:
```sh
$ tuckr set \*
```

A few flags can be used to change the behavior of the hooking and symlinking steps, check `tuckr help set` to know more.

### Running cleanup hooks
Just like setup hooks they follow these 2 steps: run-cleanup -> remove-symlinks

```sh
$ tuckr unset \*
```

## A practical example
If one has a neovim config, where one needs to download a plugin manager (packer), the npm and pip neovim libraries, and after installing them you need to download all of your plugins and LSPs, this could be achieved with hooks:
1. Prehook: install the neovim libraries for npm and pip and download packer
2. Symlink: symlink neovim configs (if previous step was successful)
3. Posthook: run command to install plugins with packer and install all LSPs with mason

## Listing hooks
If you want to know which hooks are available run:
```sh
$ tuckr ls hooks
```
This will print a table informing you about which hooks are available and you'll either have a tick or an X.
- A tick means that there's a pre-hook or post-hook available
- An X means that there's no pre-hook or post-hook set up
```
                                      
    ╭───────┬─────────┬──────────╮    
    │ Group │ Prehook │ Posthook │    
    ├───────┼─────────┼──────────┤    
    │ nvim  │    ✗    │    ✓     │    
    │ tmux  │    ✓    │    ✗     │    
    │ zsh   │    ✓    │    ✗     │    
    ╰───────┴─────────┴──────────╯    
                                      
```
