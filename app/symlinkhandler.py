from typing import List
import os
from pathlib import Path
import colorama # type: ignore

class SymlinkHandler():
    def __init__(self):
        self.files: List = [Path(file) for file in Path.cwd().iterdir()]

    def check_symlinks(self) -> None:
        '''
        WIP. Gives information about current symlinks.
        Missing suggested commands.
        '''
        not_linked:List[str] = []
        linked: List[str] = []
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

    def create_symlinks(self) -> None:
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
    
    def remove_symlinks(self) -> None:
        '''
        WIP. Checks files in the current directory and remove their symlinks from .config and $HOME dirs
        '''
        for file in self.files:
            if Path.is_symlink(file):
                Path.unlink(file)
        print(f"{colorama.Fore.GREEN}done{colorama.Fore.RESET}")

    def add_symlink(self, link: str, target: str) -> None:
        '''
        Adds a single symlink to .config or $HOME
        '''
        Path(link).symlink_to(Path(target))
        print(f"{colorama.Fore.GREEN}{link}{link}symlinked to {target}{colorama.Fore.RESET}")

    def rm_symlink(self, link: str) -> None:
        '''
        Removes a single symlink from .config or $HOME
        '''
        path = Path(link)
        if path.is_symlink():
            Path.unlink(path)
            print(f"{colorama.Fore.GREEN}{path}{path}symlink removed {colorama.Fore.RESET}")