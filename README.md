<!-- PROJECT LOGO -->
<br />
<p align="center">
  <a href="https://github.com/RaphGL/Tuckr">
    <img src="logo.png" alt="Logo" height="80">
  </a>

  <h3 align="center">A super powered replacement for GNU Stow</h3>
  <p align="center">
    <br />
    <a href="https://github.com/RaphGL/Tuckr"><strong>Explore the docs »</strong></a>
    <br />
    <br />
    ·
    <a href="https://github.com/RaphGL/Tuckr/issues">Report Bug</a>
    ·
    <a href="https://github.com/RaphGL/Tuckr/issues">Request Feature</a>
  </p>
</p>

<!-- TABLE OF CONTENTS -->
<details open="open">
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#license">License</a></li>
  </ol>
</details>

<!-- ABOUT THE PROJECT -->

Tuckr is a dotfile manager inspired by Stow and Git. Tuckr aims to make dotfile management slightly less painful. It follows the same model and stow, symlinking files onto $HOME. 

**What makes tuckr different?**

- You can manage your files from any directory
- Symlinking is tracked and you can check which programs are deployed or not
- Hooks can be used to do additional configuration to programs that need it upon symlinking
- Encryption can be used without worrying about decrypting and setting it up on the host system


### Built With

- [Rust](https://www.rust-lang.org/)
- [Clap](https://github.com/clap-rs/clap)

<!-- GETTING STARTED -->

## Getting Started

### Installation

TODO

<!-- USAGE EXAMPLES -->

## Usage

```sh
USAGE:
    tuckr <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    add          Deploy configuration for PROGRAM
    from-stow    Converts a stow repo into a tuckr one
    help         Print this message or the help of the given subcommand(s)
    init         Initialize a dotfile folder
    rm           Remove configuration for PROGRAM from the system
    set          Setup program with hooks
    status       Check symlink status

```

<!-- LICENSE -->

## License

Distributed under GPLv3 License. See [`LICENSE`](https://github.com/RaphGL/Tuckr/blob/main/LICENSE) for more information.

<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->

[contributors-shield]: https://img.shields.io/github/contributors/othneildrew/Best-README-Template.svg?style=for-the-badge
[contributors-url]: https://github.com/othneildrew/Best-README-Template/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/othneildrew/Best-README-Template.svg?style=for-the-badge
[forks-url]: https://github.com/othneildrew/Best-README-Template/network/members
[stars-shield]: https://img.shields.io/github/stars/othneildrew/Best-README-Template.svg?style=for-the-badge
[stars-url]: https://github.com/othneildrew/Best-README-Template/stargazers
[issues-shield]: https://img.shields.io/github/issues/othneildrew/Best-README-Template.svg?style=for-the-badge
[issues-url]: https://github.com/othneildrew/Best-README-Template/issues
[license-shield]: https://img.shields.io/github/license/othneildrew/Best-README-Template.svg?style=for-the-badge
[license-url]: https://github.com/othneildrew/Best-README-Template/blob/master/LICENSE.txt
[linkedin-shield]: https://img.shields.io/badge/-LinkedIn-black.svg?style=for-the-badge&logo=linkedin&colorB=555
[linkedin-url]: https://linkedin.com/in/othneildrew
[product-screenshot]: images/screenshot.png