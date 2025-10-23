# Dotfile commands

## Status

When status is run with no arguments:
 ```sh
$ tuckr status
```
 
it checks the status of all programs and displays it, returning either 0 or 1 depending on whether there were any remaining unsymlinked dotfiles.  
If one wishes to check the status of individual or multiple programs that is achieved by running: 
```sh
tuckr status program...
```
When returning tuckr returns an error code pertaining to the specified programs.

## Add 

The add command symlinks unsymlinked groups, returning an error code in case of failure symlinking.
By default add ignores all conflicting files, conflicts should be handled by checking the status command and deciding whether to adopt or override conflicts.

Say there's a group foo in conflict, to adopt the system's dotfile:
```sh
$ tuckr add -a foo
```
To override conflicts:
```sh
$ tuckr add -f foo
```

## Rm

The remove command removes dotfiles marked as symlinked, it only returns an error if there was a failure to remove files.
It accepts either a single program or multiple
```
$ tuckr rm foo
$ tuckr rm foo bar
```



