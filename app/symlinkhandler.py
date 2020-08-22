from typing import List
import os
from pathlib import Path
import colorama # type: ignore

def in_dotfiles_dir(func):
    files: List[str] = [str(file) for file in Path.cwd().iterdir()]
    def check_cwd(a):
        if f"{Path.cwd()}/tuckr.conf" in files:
            func(a)
        else:
            print(f"{colorama.Fore.RED}Error: You need to be in a dotfile directory{colorama.Fore.RESET}")
    return check_cwd

class SymlinkHandler():
    def __init__(self):
        self.files: List = [Path(file) for file in Path.cwd().iterdir()]

    def find_symlink_path(self):
        pass

    @in_dotfiles_dir
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

    @in_dotfiles_dir
    def create_symlinks(self) -> None:
        '''
        WIP. Checks files in the current directory and symlinks them to the .config and $HOME dirs 
        if they aren't already symlinked. 
        '''
        for file in self.files:
            file = str(file).split("/")[-1]
            dest = Path(f"~/{file}").expanduser()
            try:
                if "tuckr.conf" in str(file):
                    continue
                dest.symlink_to(file)
            except FileExistsError:
                print(f'{colorama.Fore.RED}Skipping: {file} is already a symlink{colorama.Fore.RESET}')
        print(f"{colorama.Fore.GREEN}done{colorama.Fore.RESET}")
    
    def remove_symlinks(self) -> None:
        '''
        WIP. Checks files in the current directory and remove their symlinks from .config and $HOME dirs
        '''
        for file in self.files:
            if file.is_symlink():
                file.unlink()
        print(f"{colorama.Fore.GREEN}done{colorama.Fore.RESET}")

    @in_dotfiles_dir
    def add_symlink(self, file: str) -> None:
        '''
        WIP. Adds a single symlink to .config or $HOME
        '''
        dest = Path(f"~/{file}").expanduser()
        if "tuckr.conf" in file:
            print(f'{colorama.Fore.RED}Warning: tuckr.conf should not be symlinked{colorama.Fore.RESET}')
            return
        Path(file).symlink_to(dest)
        print(f"{colorama.Fore.GREEN}{file} symlinked to {dest}{colorama.Fore.RESET}")

    @in_dotfiles_dir
    def rm_symlink(self, link: str) -> None:
        '''
        WIP. Removes a single symlink from .config or $HOME
        '''
        path = Path(link)
        if path.is_symlink():
            path.unlink()
            print(f"{colorama.Fore.GREEN}{path}{path}symlink removed{colorama.Fore.RESET}")
