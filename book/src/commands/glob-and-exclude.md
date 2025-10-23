# Globbing and excluding groups

## Globbing

Globbing permits you to select every group at once.
Using `*` is the same as selecting every group that match the following criteria:
- Is not currently symlinked
- Does not contain conflicting files
- Is supported by the current platform (for example `group_windows` would only work on windows, `group_linux` only on linux, `group` everywhere) 

If you try to use `tuckr add -f \*` to try to override every conflict, this won't work. You need to manually name every group you want to override like so `tuckr add -f group1 group2`. It's recommended that you don't blindly just override everything, check `tuckr status <groups>` to see what files are causing conflict and if you want to delete or adopt certain files or if you you're 100% sure that is the case for all the files you can then use the `-a` or `-f` flags, otherwise you could always just manually resolve them, depending on your needs.

```
$ tuckr add \*
```

**Note:** on some systems the glob symbol `*` is not part of the shell's operators and thus do not need to be escaped.

## Exclusion

Exclusion permits you to exclude specified groups
```
$ tuckr a firefox nvim hyprland -e firefox
$ tuckr a \* -e firefox,nvim,hyperland
```
Exclusion is most useful when globbing, it allows you to add and remove groups en masse. 
