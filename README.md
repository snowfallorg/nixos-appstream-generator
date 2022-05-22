# NixOS-Appstream

A proof of concept appstream data generator for NixOS. Currently extremely inefficient and lacking a lot of features.

```
Generate Appstream data for a given package

USAGE:
    nixos-appstream [OPTIONS] --package <PACKAGE>

OPTIONS:
    -c, --combine <COMBINE>    File to write combined Appstream data to
    -h, --help                 Print help information
    -o, --output <OUTPUT>      Were to output the Appstream data [default: result.xml]
    -p, --package <PACKAGE>    Package to generate Appstream data for
    -V, --version              Print version information
```