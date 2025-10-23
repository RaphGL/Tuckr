# Path from root

If you're managing a global configuration from Tuckr, you need to tell if that the configuration is global. Otherwise it will attempt to deploy it to `$HOME`.
The way to tell if is just by using `^`, it will then expand your dotfile path starting from that path into root.

If you have a dotfile:
```
xorg
└── ^etc
    └── X11
```
This will be expanded to `/etc/X11`. 

Bear in mind that this can be anywhere inside your dotfile group, wherever it is the expansion will always be the same. 
You can even put one inside the other and the last one to occur is the one that takes precedence. 
For example if you have:
```
xorg
└── ^etc
    └── %PROGRAM_PATH
        └── some_file
```
Then `some_file` will end up in whatever path was in this variable and not in root.
If it were the other way around (`xorg/%PROGRAM_PATH/^etc/some_file`) then it would be in root but not in the path in the environment variable.

