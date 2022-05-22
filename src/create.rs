use std::{path::Path, fs::{self, File}, process::Command, io::Write};

use serde_json::Value;

use crate::find::findfiles;

pub fn createmeta(path: String, pkg: &str, output: &str) {
    let desktops = match findfiles(Path::new(&format!("{}/share/applications", path))) {
        Ok(x) => x,
        Err(e) => {
            println!("No desktop files found");
            std::process::exit(1);
        }
    };
    let desktop = desktops.get(0).unwrap().to_string();

    let desktopdata = fs::read_to_string(&desktop).unwrap();

    let out = Command::new("nix-instantiate")
        .arg("--eval")
        .arg("-E")
        .arg(format!(
            "with import <nixpkgs/nixos> {{}}; pkgs.{}.meta",
            pkg
        ))
        .arg("--strict")
        .arg("--json")
        .output()
        .unwrap()
        .stdout;
    let pname = if desktopdata.contains("\nName=") {
        desktopdata
            .lines()
            .find(|x| x.split("=").nth(0).unwrap_or("") == "Name")
            .unwrap()
            .split("=")
            .nth(1)
            .unwrap()
            .trim()
            .to_string()
    } else {
        let name = Command::new("nix-instantiate")
            .arg("--eval")
            .arg("-E")
            .arg(format!(
                "with import <nixpkgs/nixos> {{}}; pkgs.{}.pname",
                pkg
            ))
            .arg("--strict")
            .arg("--json")
            .output()
            .unwrap()
            .stdout;
        std::str::from_utf8(&name).unwrap().to_string()
    };

    let outstr = std::str::from_utf8(&out).unwrap();
    let x: Value = serde_json::from_str(outstr).unwrap();

    let mut homeurl = String::new();
    if (x["homepage"].is_string()) {
        homeurl = x["homepage"].as_str().unwrap().to_string();
    }

    let mut license = String::new();
    if (x["license"].is_array()) {
        let arr = x["license"].as_array().unwrap();
        license = arr
            .iter()
            .filter(|x| x["spdxId"].is_string())
            .map(|x| x["spdxId"].as_str().unwrap().to_string())
            .collect::<Vec<_>>()
            .join(" and ");
    } else if x["license"]["spdxId"].is_string() {
        license = x["license"]["spdxId"].to_string();
    }

    let id = format!("\n  <id>{}</id>", desktop.split("/").last().unwrap());
    let name = format!("\n  <name>{}</name>", pname);
    let summary = format!(
        "\n  <summary>{}</summary>",
        x["description"].as_str().unwrap()
    );
    let homepage = format!("\n  <url type=\"homepage\">{homeurl}</url>");
    let project_license = format!("\n  <project_license>{}</project_license>", license);
    let pkgdata = format!("\n  <pkgname>{}</pkgname>", pkg);
    let launchdata = format!(
        "\n  <launchable type=\"desktop-id\">{}</launchable>",
        desktop.split("/").last().unwrap()
    );

    let xml = "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<component type=\"desktop-application\">"
        .to_string()
        + &id
        + &name
        + &summary
        + if !homeurl.is_empty() { &homepage } else { "" }
        + if !license.is_empty() { &project_license } else { "" }
        + &pkgdata
        + &launchdata
        + "\n</component>";

    let mut file = File::create(output).unwrap();
    file.write_all(xml.as_bytes()).unwrap();
}