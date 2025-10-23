# Path from environment variable

Certain programs depend on environment variables to know where to put things in.
Sometimes you might also just want to script the deployment of your dotfiles. For both cases you can expand your path through an enviroment variables.

You can tell Tuckr to expand an environment variable by using `%`. 

For example if you set an environment variable `PROGRAM_PATH` to `/home/user/Documents/program` and you have a dotfile with this file structure:
```
program
└── %PROGRAM_PATH
    └── config.txt
```
Tuckr will expand it to `/home/user/Documents/program/config.txt`.

