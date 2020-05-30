import yaml
import os

with open('test/conf.yml') as cfg:
    conf = yaml.safe_load(cfg)

class PackageHandler:
    def __init__(self):
        self.packages = conf['packages']
        self.installers = conf['package_installer']

    def find_installers(self):
        installer = dict()
        for items in self.installers:
            installer.update(items)
        return installer

    def install_packages(self):
        installers = self.find_installers()
        for pacman in installers: 
            if 'nosudo' in installers[pacman][1]:
                os.system(f'{installers[pacman][0]} {self.packages[pacman]}')
            if 'nosudo' not in installers[pacman][1]:
                os.system(f'sudo {installers[pacman]} {self.packages[pacman]}')

class SetupHandler:
    def __init__(self):
        self.setup = conf['setup']
        self.scripts = conf['run_script']

    def create(self):
        print(self.setup['create'])

    def move(self):
        print(self.setup['move'])

    def clone_repos(self):
        for item in self.setup['git']:
            item = item.split()
            os.system(f'git clone {item[0]} {item[1]}')

    def run_custom_scripts(self):
        for item in self.scripts:
            os.system(f'sh -c {item}')

class DotfileHandler:
    def __init__(self):
        self.dotfiles_dir = conf['dotfiles_dir']

if __name__ == "__main__":
    test = SetupHandler()
    test.run_custom_scripts()
