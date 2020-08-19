import os
from pathlib import Path
import colorama

class SymlinkHandler():
    def __init__(self):
        self.files = [Path(file) for file in Path.cwd().iterdir()]

    def check_symlinks(self):
        '''
        WIP. Gives information about current symlinks.
        Missing suggested commands.
        '''
        not_linked = []
        linked = []
        for file in self.files:
            if not Path.is_symlink(Path(file)):
                not_linked.append(f'{colorama.Fore.RED}{file}{colorama.Fore.RESET}')
            else: 
                linked.append(f'{colorama.Fore.GREEN}{file}{colorama.Fore.RESET}')
        if linked:
            print('\nSymlinked files:')
            for link in linked:
                print('\t'+link)
        if not_linked:
            print('\nUnsymlinked files:')
            for link in not_linked:
                print('\t'+link)

    def create_symlinks(self):
        '''
        WIP. Checks files in the current directory and symlinks them to the .config and $HOME dirs 
        if they're aren't already symlinked. 
        '''
        for file in self.files:
            try:
                Path(f"{file}sdfasdf").symlink_to(file)
            except FileExistsError:
                print(colorama.Fore.RED + f'Error: {file} is already a symlink' + colorama.Fore.RESET)
        print(f"{colorama.Fore.GREEN}done{colorama.Fore.RESET}")
    
    def remove_symlinks(self):
        '''
        WIP. Checks files in the current directory and remove their symlinks from .config and $HOME dirs
        '''
        for file in self.files:
            if Path.is_symlink(file):
                Path.unlink(file)
        print(f"{colorama.Fore.GREEN}done{colorama.Fore.RESET}")

    def add_symlink(self, link: str, target: str):
        '''
        Adds a single symlink to .config or $HOME
        '''
        Path(link).symlink_to(Path(target))
        print(f"{colorama.Fore.GREEN}{link}{link}symlinked to {target}{colorama.Fore.RESET}")

    def rm_symlink(self, link: str):
        '''
        Removes a single symlink from .config or $HOME
        '''
        if Path(link).is_symlink():
            Path.unlink(link)
            print(f"{colorama.Fore.GREEN}{link}{link}symlink removed {colorama.Fore.RESET}")

