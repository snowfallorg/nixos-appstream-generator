# Pkglistgen

Simple program that outputs a list of nixpkgs given a specified `packages.json` file. Large package sets and packages that do not contain metadata are removed so that generating appstream data does not require building these.

```
Generate Appstream data for a given package

USAGE:
    pkglistgen [OPTIONS]

OPTIONS:
    -a, --arch <ARCH>            List only packages with selected architecture
    -e, --everything             Include ALL packages (including kernels, drivers, etc.) NOT RECOMMENDED
    -f, --free                   List only packages with free licenses
    -h, --help                   Print help information
    -o, --output <OUTPUT>        Output package list to file
    -p, --packages <PACKAGES>    packages.json file location
    -u, --unfree                 List only packages with unfree licenses
    -V, --version                Print version information
```