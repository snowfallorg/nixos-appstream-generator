use curl::easy::Easy;
use image;
use owo_colors::{OwoColorize, Stream::Stdout};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    os::unix::prelude::PermissionsExt,
    path::Path,
    process::Command,
};
use xmltree::Element;

#[derive(Default, Debug)]
pub struct PkgData {
    pub id: Option<String>,
    pub icon: Option<String>,
    pub outputicon: Option<String>,
    pub outputmetainfo: Option<String>,
}

fn findfiles(path: &Path, ext: &str) -> Result<Vec<String>, String> {
    let mut files: Vec<String> = Vec::new();
    match fs::read_dir(path) {
        Ok(x) => {
            for entry in x {
                let entry = entry.unwrap();
                let p = entry.path();
                if p.is_file() {
                    if let Some(x) = p.as_path().extension() {
                        if x.to_str() == Some(ext) {
                            files.push(p.to_string_lossy().to_string());
                        }
                    }
                }
            }
            Ok(files)
        }
        Err(_) => Err("No file in directory".to_string()),
    }
}

fn dl(url: &str, path: &str) {
    let mut dst = Vec::new();
    let mut easy = Easy::new();
    easy.url(url).unwrap();

    {
        let mut transfer = easy.transfer();
        transfer
            .write_function(|data| {
                dst.extend_from_slice(data);
                Ok(data.len())
            })
            .unwrap();
        transfer.perform().unwrap();
    }

    let mut file = File::create(path).expect("Failed to create file");
    if let Err(e) = file.write_all(&dst) {
        panic!("{}", e)
    }
}

pub fn findmeta(path: String, meta: String, pkg: &str, pkgdata: PkgData) {
    let meta = match findfiles(Path::new(&format!("{}/share/{meta}", path)), "xml") {
        Ok(x) => x,
        Err(_) => {
            println!("No metadata files foud");
            std::process::exit(1);
        }
    };
    genmeta(path, meta, pkg, pkgdata)
}

pub fn dlmeta(path: String, metaurl: String, pkg: &str, pkgdata: PkgData) {
    let name = metaurl.split('/').last().unwrap();
    dl(&metaurl, &format!("tmp/{}", &name));
    let meta = vec![format!("tmp/{}", &name)];
    genmeta(path, meta, pkg, pkgdata)
}

fn genmeta(path: String, meta: Vec<String>, pkg: &str, pkgdata: PkgData) {
    let desktops = match findfiles(
        Path::new(&format!("{}/share/applications", path)),
        "desktop",
    ) {
        Ok(x) => x,
        Err(_) => {
            println!(
                "{pkg}: {}",
                "No desktop files found".if_supports_color(Stdout, |x| x.yellow())
            );
            for m in meta {
                xmlparse_nondesktop(m, pkg);
            }
            return;
        }
    };

    if meta.len() == 1 && desktops.len() == 1 {
        xmlparse(
            &path,
            meta[0].to_string(),
            desktops[0].to_string(),
            pkg,
            &pkgdata,
        );
    } else {
        let mut metapairs: Vec<(String, String)> = Vec::new();
        for m in &meta {
            if let Ok(f) = fs::read_to_string(&m) {
                if let Ok(x) = Element::parse(f.as_bytes()) {
                    if x.attributes.get("type") == Some(&String::from("desktop"))
                        || x.attributes.get("type") == Some(&String::from("desktop-application"))
                        || {
                            if let Some(y) = x.get_child("id") {
                                y.attributes.get("type") == Some(&String::from("desktop"))
                            } else {
                                false
                            }
                        }
                        || {
                            if let Some(y) = x.get_child("id") {
                                y.attributes.get("type")
                                    == Some(&String::from("desktop-application"))
                            } else {
                                false
                            }
                        }
                    {
                        let id = m
                            .split('/')
                            .last()
                            .unwrap()
                            .replace(".appdata", "")
                            .replace(".metainfo", "")
                            .replace(".xml", "");

                        let deskpfx = format!("{}/share/applications/", path);
                        let mut desktopfile = String::new();
                        if let Some(idattr) = x.get_child("id") {
                            let d = idattr.children.get(0).unwrap().as_text().unwrap();
                            if d.contains(".desktop")
                                && desktops.contains(&format!("{deskpfx}{}", d))
                            {
                                desktopfile = d.to_string();
                            }
                        }

                        if desktopfile.is_empty()
                            && desktops.contains(&format!("{deskpfx}{}.desktop", id))
                        {
                            desktopfile = format!("{}.desktop", id);
                        }

                        if desktopfile.is_empty()
                            && desktops.contains(&format!("{deskpfx}{}.desktop", pkg))
                        {
                            desktopfile = format!("{}.desktop", pkg);
                        }

                        if desktopfile.is_empty() {
                            let mut filtered = desktops
                                .iter()
                                .filter(|x| x.to_lowercase().contains(&id.to_lowercase()))
                                .collect::<Vec<_>>();
                            if filtered.len() == 1 {
                                desktopfile = filtered[0].to_string();
                            } else {
                                filtered = desktops
                                    .iter()
                                    .filter(|x| x.contains("org") || x.contains("com"))
                                    .collect::<Vec<_>>();
                                if filtered.len() == 1 {
                                    desktopfile = filtered[0].to_string();
                                }
                            }
                        }

                        if desktopfile.is_empty() {
                            continue;
                        } else {
                            metapairs.push((m.to_string(), format!("{deskpfx}{desktopfile}")));
                        }
                    }
                }
            }
        }

        if metapairs.is_empty() {
            println!(
                "{} No metapair found for package {pkg}",
                "error:".if_supports_color(Stdout, |x| x.red())
            );
            //std::process::exit(1);
        } else {
            if metapairs.len() == 1 {
                xmlparse(
                    &path,
                    metapairs[0].0.to_string(),
                    metapairs[0].1.to_string(),
                    pkg,
                    &pkgdata,
                );
            } else {
                for (m, d) in &metapairs {
                    xmlparse(
                        &path,
                        m.to_string(),
                        d.to_string(),
                        pkg,
                        &PkgData::default(),
                    );
                }
            }
            for m in meta {
                if !metapairs.iter().any(|(x, _)| x == &m) {
                    xmlparse_nondesktop(m, pkg);
                }
            }
        }
    }
}

pub fn xmlparse(path: &str, meta: String, desktop: String, pkg: &str, pkgdata: &PkgData) {
    let f = fs::read_to_string(&meta).unwrap();
    let mut x = match Element::parse(f.as_bytes()) {
        Ok(x) => x,
        Err(_) => {
            println!(
                "{pkg}: {}: {meta}",
                "FAILED TO PARSE XML".if_supports_color(Stdout, |x| x.bright_red())
            );
            return;
        }
    };

    if x.name == "application" {
        x.name = "component".to_string();
        x.attributes
            .insert("type".to_string(), "desktop-application".to_string());
    } else if x.name != "component" {
        println!(
            "{pkg}: {}: {meta}",
            "Not a component or application".if_supports_color(Stdout, |x| x.red())
        );
        return;
    }

    let mut icondata: Vec<String> = vec![];
    if let Some(i) = &pkgdata.icon {
        let ipath = format!("tmp/{}", i.split('/').last().unwrap());
        dl(i, &ipath);

        let iout = if let Some(customiconpath) = &pkgdata.outputicon {
            customiconpath.to_string()
        } else {
            format!(
                "{}.png",
                meta.split('/')
                    .last()
                    .unwrap()
                    .replace(".appdata", "")
                    .replace(".metainfo", "")
                    .replace(".xml", "")
            )

            //format!("{}.png", i.split('/').last().unwrap().replace(".png", "").replace(".jpg", "").replace(".svg", ""))
        };

        let mut dlicon = |size: u32| {
            match Command::new("convert")
                .arg("-size")
                .arg(format!("{}x{}", size, size))
                .arg("xc:none")
                .arg("-background")
                .arg("none")
                .arg(&ipath)
                .arg("-gravity")
                .arg("center")
                .arg("-composite")
                .arg(format!("output/icons/{size}x{size}/{iout}"))
                .output()
            {
                Ok(x) => {
                    if !x.status.success() {
                        println!("{}", String::from_utf8_lossy(&x.stderr))
                    }
                }
                Err(e) => println!("{}", e),
            }
            icondata.push(format!(
                "<icon type=\"cached\" width=\"{size}\" height=\"{size}\">{iout}</icon>"
            ));
        };

        dlicon(64);
        dlicon(128);
    } else {
        let mut icon = None;
        if let Ok(file) = File::open(&desktop) {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                if line.get(..5) == Some("Icon=") {
                    if let Some(x) = line.split('=').nth(1) {
                        icon = Some(x.to_string());
                    }
                    break;
                }
            }
        }

        if let Some(i) = icon {
            let pathscalable = format!("{}/share/icons/hicolor/scalable/apps/{}.svg", path, i);
            let iout = if let Some(customiconpath) = &pkgdata.outputicon {
                customiconpath.to_string()
            } else {
                format!("{i}.png")
            };
            let mut addicon = |size: u32| {
                let iconpath = format!("{}/share/icons/hicolor/{size}x{size}/apps/{}", path, iout);
                if Path::new(&iconpath).exists()
                    && fs::copy(&iconpath, format!("output/icons/{size}x{size}/{iout}")).is_ok()
                {
                    fs::set_permissions(
                        &format!("output/icons/{size}x{size}/{iout}"),
                        fs::Permissions::from_mode(0o644),
                    )
                    .unwrap();
                    icondata.push(format!(
                        "<icon type=\"cached\" width=\"{size}\" height=\"{size}\">{iout}</icon>"
                    ));
                } else if Path::new(&iconpath.replace(".png", ".svg")).exists() {
                    match Command::new("convert")
                        .arg("-size")
                        .arg(format!("{}x{}", size, size))
                        .arg("xc:none")
                        .arg("-background")
                        .arg("none")
                        .arg(&iconpath.replace(".png", ".svg"))
                        .arg("-gravity")
                        .arg("center")
                        .arg("-composite")
                        .arg(format!("output/icons/{size}x{size}/{iout}"))
                        .output()
                    {
                        Ok(x) => {
                            if !x.status.success() {
                                println!("{}", String::from_utf8_lossy(&x.stderr))
                            }
                        }
                        Err(e) => println!("{}", e),
                    }
                    icondata.push(format!(
                        "<icon type=\"cached\" width=\"{size}\" height=\"{size}\">{iout}</icon>"
                    ));
                } else if Path::new(&pathscalable).exists() {
                    match Command::new("convert")
                        .arg("-size")
                        .arg(format!("{}x{}", size, size))
                        .arg("xc:none")
                        .arg("-background")
                        .arg("none")
                        .arg(&pathscalable)
                        .arg("-gravity")
                        .arg("center")
                        .arg("-composite")
                        .arg(format!("output/icons/{size}x{size}/{iout}"))
                        .output()
                    {
                        Ok(x) => {
                            if !x.status.success() {
                                println!("{}", String::from_utf8_lossy(&x.stderr))
                            }
                        }
                        Err(e) => println!("{}", e),
                    }
                    icondata.push(format!(
                        "<icon type=\"cached\" width=\"{size}\" height=\"{size}\">{iout}</icon>"
                    ));
                } else {
                    let sizes = vec![64, 72, 96, 128, 192, 256, 512, 1024]
                        .into_iter()
                        .filter(|x| *x >= size)
                        .collect::<Vec<u32>>();
                    let mut scalepath = String::new();
                    for s in &sizes {
                        if Path::new(&format!(
                            "{}/share/icons/hicolor/{s}x{s}/apps/{}.png",
                            path, i
                        ))
                        .exists()
                        {
                            //scalesize = s;
                            scalepath =
                                format!("{}/share/icons/hicolor/{s}x{s}/apps/{}.png", path, i);
                            break;
                        }
                    }

                    if !scalepath.is_empty() {
                        let img = image::open(&scalepath).unwrap();
                        let newimg = img.resize(size, size, image::imageops::Lanczos3);
                        newimg
                            .save(format!("output/icons/{size}x{size}/{iout}"))
                            .unwrap();
                        icondata.push(format!(
                            "<icon type=\"cached\" width=\"{size}\" height=\"{size}\">{iout}</icon>"
                        ));
                    } else {
                        for s in &sizes {
                            if Path::new(&format!(
                                "{}/share/icons/hicolor/{s}x{s}/apps/{}.svg",
                                path, i
                            ))
                            .exists()
                            {
                                // scalesize = s;
                                scalepath =
                                    format!("{}/share/icons/hicolor/{s}x{s}/apps/{}.svg", path, i);
                                break;
                            }
                        }

                        if !scalepath.is_empty() {
                            match Command::new("convert")
                                .arg("-size")
                                .arg(format!("{}x{}", size, size))
                                .arg("xc:none")
                                .arg("-background")
                                .arg("none")
                                .arg(&scalepath)
                                .arg("-gravity")
                                .arg("center")
                                .arg("-composite")
                                .arg(format!("output/icons/{size}x{size}/{iout}"))
                                .output()
                            {
                                Ok(x) => {
                                    if !x.status.success() {
                                        println!("{}", String::from_utf8_lossy(&x.stderr))
                                    }
                                }
                                Err(e) => println!("{}", e),
                            }
                            icondata.push(format!(
                                "<icon type=\"cached\" width=\"{size}\" height=\"{size}\">{iout}</icon>"
                            ));
                        }
                    }
                }
            };
            addicon(128);
            addicon(64);
        }
    }

    let pkgname = format!("<pkgname>{}</pkgname>", pkg);
    let launchdata = format!(
        "<launchable type=\"desktop-id\">{}</launchable>",
        desktop.split('/').last().unwrap()
    );

    let p = Element::parse(pkgname.as_bytes()).unwrap();
    let l = Element::parse(launchdata.as_bytes()).unwrap();

    while x.get_child("icon").is_some() {
        x.take_child("icon").unwrap();
    }

    if icondata.is_empty() {
        println!(
            "{pkg}: {}",
            "no desktop icons found".if_supports_color(Stdout, |x| x.bright_purple())
        );
    } else if icondata.len() == 1 {
        println!(
            "{pkg}: {}",
            "some desktop icons missing".if_supports_color(Stdout, |x| x.bright_purple())
        );
    }

    for data in icondata {
        let d = Element::parse(data.as_bytes()).unwrap();
        x.children.insert(0, xmltree::XMLNode::Element(d));
    }

    if x.get_child("launchable").is_none() {
        x.children.insert(0, xmltree::XMLNode::Element(l));
    }

    if let Some(customid) = &pkgdata.id {
        x.take_child("id").unwrap();
        x.children.insert(
            0,
            xmltree::XMLNode::Element(
                Element::parse(format!("<id>{customid}</id>").as_bytes()).unwrap(),
            ),
        );
    }

    if x.get_child("pkgname").is_none() {
        x.children.insert(0, xmltree::XMLNode::Element(p));
    } else {
        x.take_child("pkgname").unwrap();
        x.children.insert(0, xmltree::XMLNode::Element(p));
    }

    // Fix description field translations
    let mut desc_children = x
        .children
        .iter()
        .filter(|x| {
            if let Some(y) = x.as_element() {
                y.name == "description"
            } else {
                false
            }
        })
        .collect::<Vec<_>>();
    if desc_children.len() == 1 {
        if let Some(d) = desc_children.get_mut(0) {
            if let Some(d) = d.clone().as_mut_element() {
                fixtranslation(d, &mut x);
            }
        }
    }

    let writer = xmltree::EmitterConfig::new().perform_indent(true);
    let id = if let Some(customidout) = &pkgdata.outputmetainfo {
        customidout.replace(".xml", "")
    } else {
        meta.split('/')
            .last()
            .unwrap()
            .replace(".appdata", "")
            .replace(".metainfo", "")
            .replace(".xml", "")
    };

    match x.write_with_config(
        File::create(format!("output/metadata/{pkg}::{id}.xml")).unwrap(),
        writer,
    ) {
        Ok(_) => {
            match Command::new("sed")
                .arg("-i")
                .arg(r#"s/\xe2\x80\x8b//g"#)
                .arg(&format!("output/metadata/{pkg}::{id}.xml"))
                .output()
            {
                Ok(_) => {
                    println!(
                        "{pkg}: {}: {}",
                        id.if_supports_color(Stdout, |x| x.cyan()),
                        "Success!".if_supports_color(Stdout, |x| x.green())
                    );
                }
                Err(e) => {
                    println!("{pkg}: {}", e.if_supports_color(Stdout, |x| x.red()));
                }
            }
        }
        Err(e) => println!("{pkg}: {}", e.if_supports_color(Stdout, |x| x.red())),
    }
}

pub fn xmlparse_nondesktop(meta: String, pkg: &str) {
    let f = fs::read_to_string(&meta).unwrap();
    let mut x = match Element::parse(f.as_bytes()) {
        Ok(x) => x,
        Err(_) => {
            println!(
                "{pkg}: {}: {meta}",
                "FAILED TO PARSE XML".if_supports_color(Stdout, |x| x.bright_green())
            );
            return;
        }
    };

    if x.name != "component" {
        println!(
            "{pkg}: {}: {meta}",
            "Not a component".if_supports_color(Stdout, |x| x.red())
        );
        return;
    }

    let pkgdata = format!("<pkgname>{}</pkgname>", pkg);

    let p = Element::parse(pkgdata.as_bytes()).unwrap();

    if x.get_child("pkgname").is_none() {
        x.children.insert(0, xmltree::XMLNode::Element(p));
    } else {
        x.take_child("pkgname").unwrap();
        x.children.insert(0, xmltree::XMLNode::Element(p));
    }

    // Fix description field translations
    let mut desc_children = x
        .children
        .iter()
        .filter(|x| {
            if let Some(y) = x.as_element() {
                y.name == "description"
            } else {
                false
            }
        })
        .collect::<Vec<_>>();
    if desc_children.len() == 1 {
        if let Some(d) = desc_children.get_mut(0) {
            if let Some(d) = d.clone().as_mut_element() {
                fixtranslation(d, &mut x);
            }
        }
    }

    let writer = xmltree::EmitterConfig::new().perform_indent(true);
    let id = meta
        .split('/')
        .last()
        .unwrap()
        .replace(".appdata", "")
        .replace(".metainfo", "")
        .replace(".xml", "");
    match x.write_with_config(
        File::create(format!("output/metadata/{pkg}::{id}.xml")).unwrap(),
        writer,
    ) {
        Ok(_) => {
            match Command::new("sed")
                .arg("-i")
                .arg(r#"s/\xe2\x80\x8b//g"#)
                .arg(&format!("output/metadata/{pkg}::{id}.xml"))
                .output()
            {
                Ok(_) => {
                    println!(
                        "{pkg}: {}: {}",
                        id.if_supports_color(Stdout, |x| x.cyan()),
                        "Addon success!".if_supports_color(Stdout, |x| x.green())
                    );
                }
                Err(e) => {
                    println!("{pkg}: {}", e.if_supports_color(Stdout, |x| x.red()));
                }
            }
        }
        Err(e) => println!("{pkg}: {}", e.if_supports_color(Stdout, |x| x.red())),
    }
}

fn fixtranslation(d: &mut Element, x: &mut Element) {
    let n = &d.name.to_string();
    let mut map: HashMap<String, Vec<xmltree::Element>> = HashMap::new();
    for p in d.clone().children {
        if let Some(p) = p.as_element() {
            if p.children.iter().any(|x| x.as_element().is_some()) {
                let mut p = p.clone();
                fixtranslation(&mut p, d);
            }
        }
    }
    for p in &d.children {
        if let Some(p) = p.as_element() {
            let mut p2 = p.clone();
            p2.attributes.clear();
            if let Some(l) = p.attributes.get("lang") {
                if let Some(v) = map.get_mut(l) {
                    v.push(p2.clone());
                } else {
                    map.insert(l.clone(), vec![p2.clone()]);
                }
            } else if let Some(v) = map.get_mut("") {
                v.push(p2.clone());
            } else {
                map.insert("".to_string(), vec![p2.clone()]);
            }
        }
    }
    let i = x.children.iter().position(|x| if let Some(y) = x.as_element() { y.eq(d) } else { false }).unwrap_or_default();
    x.take_child(n.as_str()).unwrap();
    let mut mapvec = map.into_iter().collect::<Vec<_>>();
    // Reverse order
    mapvec.sort_by(|(x, _), (y, _)| y.cmp(x));
    for (k, v) in mapvec {
        let mut d = Element::parse(format!("<{n}></{n}>").as_bytes()).unwrap();
        if !k.is_empty() {
            d.attributes.insert("lang".to_string(), k);
        }
        for p in v.into_iter().rev() {
            d.children.insert(0, xmltree::XMLNode::Element(p));
        }
        x.children.insert(i, xmltree::XMLNode::Element(d));
    }
}