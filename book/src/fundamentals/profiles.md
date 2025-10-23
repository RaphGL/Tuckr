# Profiles

A profile is a way to keep dotfiles in different repos in the same machine. 
Say you have dotfiles that are personal and others that should only be for work and is on a private repo. 

You could attempt to deploy it with tuckr, but you would either have to:
1. make the private work repo a submodule and have tuckr dump every dotfile as one group
2. make your work dotfiles part of your personal one (which might not be feasible)

Profiles fix this by allowing you to be able to work with multiple repos at once.

## How it works

Profiles are essentially just a dotfiles directory with a suffix.

It follows this structure: `dotfiles_<profile>` 
If you have a `dotfiles_work` directory in the lookup path, you can use work as a profile:
```sh
tuckr -p work status
```
And it will give you the status for dotfiles in this repo

If you're using the unsuffixed `dotfiles`, no profile flag is needed. So you can consider that one your `default` profile. 

## Listing profiles
This command lists every profiles that are available in the machine
```sh
$ tuckr ls profiles
```
