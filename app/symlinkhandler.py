import os
import colorama

class SymlinkHandler():
    def __init__(self):
        self.files = os.listdir()

    def check_symlinks(self):
        '''
        WIP. Gives information about current symlinks.
        Missing suggested commands.
        '''
        not_linked = []
        linked = []
        for file in self.files:
            if not os.path.islink(file):
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
            os.symlink(file, f"{file}x")
        print(f"{colorama.Fore.GREEN}symlinks created")
    
    def remove_symlinks(self):
        '''
        WIP. Checks files in the current directory and remove their symlinks from .config and $HOME dirs
        '''
        for file in self.files:
            if os.path.islink(file):
                os.remove(file)
        print(f"{colorama.Fore.GREEN}symlinks removed")

    def add_symlink(self, link: str):
        '''
        Adds a single symlink to .config or $HOME
        '''
        print(f"{colorama.Fore.GREEN}{link} symlink created")