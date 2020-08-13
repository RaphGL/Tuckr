import os
import configparser
import click

class EnvInterpolation(configparser.BasicInterpolation):
    """Interpolation which expands environment variables in values."""

    def before_get(self, parser, section, option, value, defaults):
        value = super().before_get(parser, section, option, value, defaults)
        return os.path.expandvars(value)

config = configparser.ConfigParser(interpolation=EnvInterpolation())
try:
    try: 
        config.read('./tuckr.conf')
    except:
        config.read(f"{os.environ['HOME']}/.config/tuckr.conf")
    config_dir = config['GENERAL']['config_dir']
except KeyError:
    print('Error: No config file was found')


def clone_dotfiles():
    try:
        if ('dotfiles_repo' in config['GENERAL']):
            if ('dotfiles_dest' in config['GENERAL']):
                os.system(f"git clone {config['GENERAL']['dotfiles_repo']} {config['GENERAL']['dotfiles_dest']}")
            else:
                os.system(f"git clone {config['GENERAL']['dotfiles_repo']} $HOME/dotfiles")
        else:
            os.system(config['GENERAL']['clone_dotfiles_cmd'])
    except KeyError:
        print('Error: No dotfile repo was specified. Make sure you set it up in your tuckr.ini file.')

def install_packages():
    if ('pkg_install_cmd' in config['PACKAGES']):
        with open(config['PACKAGES']['pkg_list'], 'r') as pkgs:
            pkgs = pkgs.read().replace('\n', ' ')
            os.system(f"{config['PACKAGES']['pkg_install_cmd']} {pkgs}")

def run_scripts():
    for script in config['SCRIPTS']:
        print(config['SCRIPTS'][script])
        os.system(f"sh -c {config['SCRIPTS'][script]}")

def install_from_list():
    install_cmd = {
        'pip': 'install --user',
        'npm': 'install -g',
        'yarn': 'global add'
    }
    for list_ in config['PACKAGES']:
        if('_list' in list_):
            cmd = list_.split('_')[0]
            try: 
                with open(config['PACKAGES'][list_], 'r') as list_file:
                    list_file = list_file.read().replace('\n', ' ')
                    os.system(f"{cmd} {install_cmd[cmd]} {list_file}")
            except KeyError:
                pass

if __name__ == '__main__':
    clone_dotfiles()
