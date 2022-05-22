use std::{path::Path, fs::{self, File}};

use xmltree::Element;

pub fn findfiles(path: &Path) -> Result<Vec<String>, String> {
    let mut files: Vec<String> = Vec::new();
    match fs::read_dir(path) {
        Ok(x) => {
            for entry in x {
                let entry = entry.unwrap();
                let p = entry.path();
                if p.is_file() {
                    files.push(p.to_string_lossy().to_string());
                }
            }
            return Ok(files);
        },
        Err(_) => Err("No file in directory".to_string()),
    }
    
}

pub fn findmeta(path: String, meta: String, pkg: &str, out: &str) {
    let meta = match findfiles(Path::new(&format!("{}/share/{meta}", path))) {
        Ok(x) => x,
        Err(e) => {
            println!("No metadata files foud");
            std::process::exit(1);
        }
    };
    let desktops = match findfiles(Path::new(&format!("{}/share/applications", path))) {
        Ok(x) => x,
        Err(e) => {
            println!("No desktop files found");
            std::process::exit(1);
        }
    };
    println!("META: {:?}", meta);
    println!("APPS: {:?}", desktops);
    if meta.len() > 0 && desktops.len() > 0 {
        xmlparse(
            meta.get(0).unwrap().to_string(),
            desktops.get(0).unwrap().to_string(),
            pkg,
            out,
        );
    }
}

pub fn xmlparse(meta: String, desktop: String, pkg: &str, out: &str) {
    let pkgdata = format!("<pkgname>{}</pkgname>", pkg);
    let launchdata = format!(
        "<launchable type=\"desktop-id\">{}</launchable>",
        desktop.split("/").last().unwrap()
    );

    let f = fs::read_to_string(&meta).unwrap();
    let mut x = Element::parse(f.as_bytes()).unwrap();
    let p = Element::parse(pkgdata.as_bytes()).unwrap();
    let l = Element::parse(launchdata.as_bytes()).unwrap();
    let len = x.children.len();

    if x.get_child("pkgname").is_none() {
        x.children.insert(len, xmltree::XMLNode::Element(p));
    } else {
        x.take_child("pkgname").unwrap();
        x.children.insert(len, xmltree::XMLNode::Element(p));
    }

    if x.get_child("launchable").is_none() {
        x.children.insert(len + 1, xmltree::XMLNode::Element(l));
    }

    let writer = xmltree::EmitterConfig::new().perform_indent(true);
    x.write_with_config(File::create(out).unwrap(), writer);
}