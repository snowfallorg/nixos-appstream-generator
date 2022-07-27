use nixos_appstream_generator::find::{dlmeta, findmeta, PkgData};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
    process::{exit, Command},
};
use sysinfo::{self, DiskExt, System, SystemExt};
use clap::{self, ArgGroup, Parser};
use owo_colors::{OwoColorize, Stream::Stdout};

/// Generate Appstream data for a given package
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(group(
    ArgGroup::new("inputs")
        .args(&["package", "list"]),
))]
#[clap(arg_required_else_help = true)]
struct Args {
    /// Package to generate Appstream data for
    #[clap(short, long)]
    package: Option<String>,

    /// Path to text file with a list of packages to check
    #[clap(short, long)]
    list: Option<String>,

    /// Customization json file
    #[clap(short, long)]
    data: Option<String>,

    /// Weather to clean nix-store periodically
    #[clap(short, long)]
    clean: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct CustomPackages {
    packages: HashMap<String, CustomPackage>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CustomPackage {
    metainfo: Option<String>,
    icon: Option<String>,
    id: Option<String>,
    output: Option<CustomPackageOutput>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CustomPackageOutput {
    metainfo: Option<String>,
    icon: Option<String>,
}

fn main() {
    let args = Args::parse();
    let mut sys = System::new_all();
    let disk = sys.disks_mut().iter_mut().find(|x| x.mount_point() == Path::new("/")).unwrap();
    let cleanspace = &disk.available_space() - 53687091200; // 50GB

    let data: HashMap<String, CustomPackage> = if let Some(custompath) = args.data {
        let inputdata = fs::read_to_string(custompath).expect("Failed to read json file");
        serde_json::from_str(&inputdata).expect("Failed to parse json file")
    } else if Path::new("custom.json").is_file() {
        let inputdata = fs::read_to_string("custom.json").expect("Failed to read json file");
        serde_json::from_str(&inputdata).expect("Failed to parse json file")
    } else {
        HashMap::new()
    };

    if !Path::new("tmp").exists() {
        fs::create_dir("tmp").unwrap();
    }

    if let Some(pkg) = args.package {
        if let Some(pkgdat) = data.get(&pkg) {
            let pkgdata = PkgData {
                id: pkgdat.id.clone(),
                icon: pkgdat.icon.clone(),
                outputicon: if let Some(out) = &pkgdat.output {
                    out.icon.clone()
                } else {
                    None
                },
                outputmetainfo: if let Some(out) = &pkgdat.output {
                    out.metainfo.clone()
                } else {
                    None
                },
            };
            gendata(&pkg, false, pkgdat.metainfo.clone(), pkgdata);
        } else {
            gendata(&pkg, false, None, PkgData::default());
        }
    } else if let Some(listfile) = args.list {


        if let Ok(file) = File::open(&listfile) {
            let reader = BufReader::new(file);
            for pkg in reader.lines().flatten() {
                disk.refresh();
                let clean = disk.available_space() < cleanspace;
                eprintln!("AVAILABLE SPACE: {}", disk.available_space());
                eprintln!("CLEAN AFTER: {}", cleanspace);
                if let Some(pkgdat) = data.get(&pkg) {
                    let pkgdata = PkgData {
                        id: pkgdat.id.clone(),
                        icon: pkgdat.icon.clone(),
                        outputicon: if let Some(out) = &pkgdat.output {
                            out.icon.clone()
                        } else {
                            None
                        },
                        outputmetainfo: if let Some(out) = &pkgdat.output {
                            out.metainfo.clone()
                        } else {
                            None
                        },
                    };
                    gendata(&pkg, clean && args.clean, pkgdat.metainfo.clone(), pkgdata);
                } else {
                    gendata(&pkg, clean && args.clean, None, PkgData::default());
                }
            }
        } else {
            println!("Could not open file {}", listfile);
            exit(1);
        }
    } else {
        println!("No package or package list specified");
        std::process::exit(1);
    }

    if args.clean {
        println!("{}", "Cleaning nix store...".if_supports_color(Stdout, |x| x.purple()));
        match Command::new("nix-store").arg("--gc").output() {
            Ok(_) => (),
            Err(_) => println!("{}", "Could not run nix-store --gc".if_supports_color(Stdout, |x| x.red())),
        }
    }

    if Path::new("tmp").exists() {
        fs::remove_dir_all("tmp").unwrap();
    }
}

fn gendata(pkg: &str, clean: bool, metaoverride: Option<String>, pkgdata: PkgData) {
    let out = Command::new("nix-build")
        .arg("--no-out-link")
        .arg("<nixpkgs>")
        .arg("-A")
        .arg(pkg)
        .output();

    if let Ok(o) = out {
        if o.status.success() {
            if !Path::new("output/icons/128x128").exists() {
                fs::create_dir_all("output/icons/128x128").unwrap();
            }
            if !Path::new("output/icons/64x64").exists() {
                fs::create_dir_all("output/icons/64x64").unwrap();
            }
            if !Path::new("output/metadata").exists() {
                fs::create_dir_all("output/metadata").unwrap();
            }

            let path = std::str::from_utf8(&o.stdout).unwrap().replace('\n', "");

            if let Some(metaurl) = metaoverride {
                dlmeta(path, metaurl, pkg, pkgdata);
            } else if Path::exists(Path::new(&format!("{}/share/metainfo", path))) {
                findmeta(path, "metainfo".to_string(), pkg, pkgdata);
            } else if Path::exists(Path::new(&format!("{}/share/appdata", path))) {
                findmeta(path, "appdata".to_string(), pkg, pkgdata);
            } else {
                println!("{pkg}: {}", "No metadata found".if_supports_color(Stdout, |x| x.red()));
            }
        } else {
            println!("{} failed to build {}", "error:".if_supports_color(Stdout, |x| x.red()), pkg);
        }
    }

    if clean {
        println!("{}", "Cleaning nix store...".if_supports_color(Stdout, |x| x.purple()));
        match Command::new("nix-store").arg("--gc").output() {
            Ok(_) => (),
            Err(_) => println!("{}", "Could not run nix-store --gc".if_supports_color(Stdout, |x| x.red())),
        }
    }
}
