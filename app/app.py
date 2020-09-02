import click
import colorama # type: ignore
from symlinkhandler import SymlinkHandler
import dotfilehandler


@click.group()
def main():
    pass

@main.command()
def setup():
    '''
    runs all the scripts and installs all the packages
    '''
    pass

@main.command()
def status():
    current.check_symlinks()

@main.command()
@click.argument('file', type=click.Path(exists=True))
def add(file):
    current.add_symlink(click.format_filename(file))

@main.command()
def reset():
    current.remove_symlinks()

@main.command()
@click.argument('file', type=click.Path(exists=True))
def rm(file):
    current.rm_symlink(click.format_filename(file))
    
if __name__ == '__main__':
    current = SymlinkHandler()
    colorama.init()
    main()
