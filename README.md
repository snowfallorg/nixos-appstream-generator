# NixOS Appstream Generator

A small program that iterates over a list of nix packages and extracts appstream data.

```
Generate Appstream data for a given package

USAGE:
    nixos-appstream-generator [OPTIONS]

OPTIONS:
    -c, --clean                Weather to clean nix-store periodically
    -d, --data <DATA>          Customization json file
    -h, --help                 Print help information
    -l, --list <LIST>          Path to text file with a list of packages to check
    -p, --package <PACKAGE>    Package to generate Appstream data for
    -V, --version              Print version information
```