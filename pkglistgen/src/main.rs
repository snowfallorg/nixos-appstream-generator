use clap::{ArgGroup, Parser};
use owo_colors::{OwoColorize, Stream::Stdout};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

/// Generate Appstream data for a given package
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(group(
    ArgGroup::new("license")
        .args(&["free", "unfree"]),
))]
struct Args {
    /// List only packages with unfree licenses
    #[clap(short, long)]
    unfree: bool,

    /// List only packages with free licenses
    #[clap(short, long)]
    free: bool,

    /// List only packages with selected architecture
    #[clap(short, long)]
    arch: Option<String>,

    /// Output package list to file
    #[clap(short, long)]
    output: Option<String>,

    /// Include ALL packages (including kernels, drivers, etc.) NOT RECOMMENDED
    #[clap(short, long)]
    everything: bool,

    /// packages.json file location
    #[clap(short, long)]
    packages: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PackageBase {
    packages: HashMap<String, Package>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Package {
    system: String,
    meta: Meta,
}
#[derive(Serialize, Deserialize, Debug)]
struct Meta {
    broken: Option<bool>,
    insecure: Option<bool>,
    unsupported: Option<bool>,
    unfree: Option<bool>,
}

fn main() {
    let args = Args::parse();
    let inputdata = if let Some(path) = args.packages {
        if Path::new(&path).is_file() {
            fs::read_to_string(path).expect("Failed to read json file")
        } else {
            panic!("{} {} is not a file", "error:".if_supports_color(Stdout, |x| x.red()), path.purple())
        }
    } else if Path::new("packages.json").is_file() {
        fs::read_to_string("packages.json").expect("Failed to read json file")
    } else {
        eprintln!("{} No packages.json file specified", "error:".if_supports_color(Stdout, |x| x.red()));
        std::process::exit(1);
    };
    let data: PackageBase = serde_json::from_str(&inputdata).expect("Failed to parse json file");

    let arch = if let Some(a) = args.arch {
        a
    } else {
        "x86_64-linux".to_string()
    };

    let mut pkgdata = data.packages;

    // Filter by license
    if args.free {
        pkgdata = pkgdata
            .into_iter()
            .filter(|(_, p)| p.meta.unfree != Some(true))
            .collect::<HashMap<_, _>>();
    } else if args.unfree {
        pkgdata = pkgdata
            .into_iter()
            .filter(|(_, p)| p.meta.unfree == Some(true))
            .collect::<HashMap<_, _>>();
    }

    // Filter by architecture
    pkgdata = pkgdata
        .into_iter()
        .filter(|(_, p)| p.system == arch)
        .collect::<HashMap<_, _>>();

    // Remove broken packages
    pkgdata = pkgdata
        .into_iter()
        .filter(|(_, p)| p.meta.broken != Some(true))
        .collect::<HashMap<_, _>>();

    // Remove insecure packages
    pkgdata = pkgdata
        .into_iter()
        .filter(|(_, p)| p.meta.insecure != Some(true))
        .collect::<HashMap<_, _>>();

    // Remove unsupported packages
    pkgdata = pkgdata
        .into_iter()
        .filter(|(_, p)| p.meta.unsupported != Some(true))
        .collect::<HashMap<_, _>>();

    // Remove large package sets
    if !args.everything {
        let filteredstart = vec![
            "linuxKernel",

            "androidStudioPackages",
            "apacheHttpdPackages",
            "arcanPackages",
            "beetsPackages",
            "chickenPackages",
            "coqPackages",
            "cudaPackages",
            "dhallPackages",
            "dotnetCorePackages",
            "dotnetPackages",
            "dwarf-fortress-packages",
            "elmPackages",
            "emacs28Packages",
            "emscriptenPackages",
            "fdbPackages",
            "gnuradio3_8Packages",
            "haskellPackages",
            "haxePackages",
            "idrisPackages",
            "javaPackages",
            "kodiPackages",
            "lispPackages",
            "llvmPackages",
            "lua51Packages",
            "lua52Packages",
            "lua53Packages",
            "luajitPackages",
            "nimPackages",
            "nodePackages",
            "ocamlPackages",
            "octavePackages",
            "openraPackages",
            "perl532Packages",
            "perl534Packages",
            "php80Packages",
            "php81Packages",
            "postgresql11Packages",
            "postgresql12Packages",
            "postgresql13Packages",
            "postgresql14Packages",
            "python310Packages",
            "python39Packages",
            "quicklispPackagesClisp",
            "rPackages",
            "rubyPackages",
            "ue4demos",
            "wine64Packages",
            "winePackages",
            "wineWowPackages",

            "elasticsearchPlugins",
            "fishPlugins",
            "graylogPlugins",
            "gsignondPlugins",
            "kakounePlugins",
            "tmuxPlugins",
            "vdrPlugins",
            "vimPlugins",

            "gnomeExtensions",
            "passExtensions",
            "php80Extensions",
            "php81Extensions",
            "vscode-extensions",

            "adoptopenjdk",
            "alephone",
            "aspellDicts",
            "bitcoind",
            "CuboCore",
            "dictdDBs",
            "ethminer",
            "gawkextlib",
            "haskell.",
            "home-assistant-component-tests",
            "hunspellDicts",
            "libretro.",
            "lohit-fonts",
            "minecraftServers",
            "mpvScripts",
            "openjdk",
            "optifinePackages",
            "pythonDocs",
            "terraform-providers",
            "texlive",
            "tree-sitter-grammars",
            "weechatScripts",
            "zncModules",

        ];

        let filteredend = vec![
            "WithCuda",
            "withCuda",
            "CudaMpi",
        ];

        let filteredpkgs = vec![
            "cntk",
            "gpu-burn",
            "mathematica-cuda"
        ];

        pkgdata = pkgdata
            .into_iter()
            .filter(|(x, _)| !filteredstart.iter().any(|y| x.starts_with(y)))
            .filter(|(x, _)| !filteredend.iter().any(|y| x.ends_with(y)))
            .filter(|(x, _)| !filteredpkgs.iter().any(|y| x == y))
            .collect::<HashMap<_, _>>();
    }

    let mut p = pkgdata.into_iter().map(|(x, _)| x).collect::<Vec<_>>();
    p.sort();
    if let Some(outpath) = args.output {
        fs::write(outpath, p.join("\n")).expect("Failed to write to file");
    } else {
        for pkg in p {
            println!("{}", pkg);
        }
    }
    
}
