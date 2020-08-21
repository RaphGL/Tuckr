import click
import colorama # type: ignore
from symlinkhandler import SymlinkHandler
import dotfilehandler

current = SymlinkHandler()
    
if __name__ == '__main__':
    colorama.init()
    current.check_symlinks()