use serde_json::{self, to_string, Value};
use std::{
    fmt::Debug,
    fs::{self, File},
    io::{Read, Write},
    path::{self, Path},
    process::{self, Command},
};
use xmltree;
use xmltree::Element;
use nixos_appstream::{find::*, create::*};
use clap::Parser;

/// Generate Appstream data for a given package
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Package to generate Appstream data for
    #[clap(short, long)]
    package: String,

    /// Were to output the Appstream data
    #[clap(short, long, default_value = "result.xml")]
    output: String,

    /// File to write combined Appstream data to
    #[clap(short, long)]
    combine: Option<String>,
}

fn main() {
    let args = Args::parse();
    let pkg = args.package;
    let out = Command::new("nix-shell")
        .arg("-p")
        .arg(&pkg)
        .arg("--run")
        .arg(format!("nix-build '<nixpkgs>' -A {pkg}"))
        .output();
    let path = std::str::from_utf8(&out.unwrap().stdout)
        .unwrap()
        .replace("\"", "")
        .replace("\n", "");
    if Path::exists(Path::new(&format!("{}/share/metainfo", path))) {
        findmeta(path, "metainfo".to_string(), &pkg, &args.output);
    } else if Path::exists(Path::new(&format!("{}/share/appdata", path))) {
        findmeta(path, "appdata".to_string(), &pkg, &args.output);
    } else {
        createmeta(path, &pkg, &args.output);
    }
    if let Some(c) = args.combine {
        if !Path::is_file(Path::new(&c)) {
            let mut file = fs::File::create(c.as_str()).unwrap();
            let f = "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<components version=\"0.1\" origin=\"nix-appstream\">
</components>";
            file.write_all(f.as_bytes()).unwrap();
        }
        let out = Command::new("sed")
            .arg("-i")
            .arg("-e")
            .arg(format!("/<\\/components>/e echo \"$(tail -n +2 {})\"", args.output))
            .arg(c)
            .output();
    }
}
