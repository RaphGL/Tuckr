from typing import List
import os
from pathlib import Path
import colorama # type: ignore

def in_dotfiles_dir(func):
    files: List[str] = [str(file) for file in Path.cwd().iterdir()]
    def check_cwd(self, *args):
        if f"{Path.cwd()}/tuckr.conf" in files:
            func(self, *args)
        else:
            print(f"{colorama.Fore.RED}Error: You need to be in a dotfile directory{colorama.Fore.RESET}")
    return check_cwd

class SymlinkHandler():
    def __init__(self):
        self.files: List = [Path(file) for file in Path.cwd().iterdir()]

    def find_symlink_path(self):
        '''
        Looks for symlinks on .config and $HOME and returns a list with them
        '''
        pass

    @in_dotfiles_dir
    def check_symlinks(self) -> None:
        '''
        WIP. Gives information about current symlinks.
        Missing suggested commands.
        '''
        # Needs full rewrite with find_symlink_path

    @in_dotfiles_dir
    def create_symlinks(self) -> None:
        '''
        WIP. Checks files in the current directory and symlinks them to the .config and $HOME dirs 
        if they aren't already symlinked. 
        '''
        for file in self.files:
            file = str(file).split("/")[-1]
            dest = Path(f"~/{file}").expanduser().resolve()
            try:
                if "tuckr.conf" in str(file):
                    continue
                dest.symlink_to(Path(file).resolve())
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
        dest = Path(f"~/{file}").expanduser().resolve()
        if "tuckr.conf" in file:
            print(f'{colorama.Fore.RED}Warning: tuckr.conf should not be symlinked{colorama.Fore.RESET}')
            return
        Path(dest).symlink_to(Path(file).resolve())
        print(f"{colorama.Fore.GREEN}{file} symlinked to {dest}{colorama.Fore.RESET}")

    @in_dotfiles_dir
    def rm_symlink(self, link: str) -> None:
        '''
        WIP. Removes a single symlink from .config or $HOME
        '''
        path = Path(link).resolve()
        if path.is_symlink():
            path.unlink()
            print(f"{colorama.Fore.GREEN}{path}{path}symlink removed{colorama.Fore.RESET}")
