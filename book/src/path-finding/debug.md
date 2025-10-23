# Debugging

If you're trying to figure out what will happen or where things will go. You can enable dry-running commands.

Dry running means that Tuckr will tell only you what actions it would take without carrying them out.
If you do dry run and nothing is printed. That just means that there was nothing to do.

You can enable dry running by using the `-n` or `--dry-run` flags after tuckr. Examples:
```sh
$ tuckr -n add \*
$ tuckr -n add -f zsh
$ tuckr -n set neovim
```

