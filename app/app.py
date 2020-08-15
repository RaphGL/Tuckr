import os
import colorama
import click
from symlinkhandler import SymlinkHandler
import dotfilehandler

if __name__ == '__main__':
    colorama.init()
    SymlinkHandler().check_symlinks()
