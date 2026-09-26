mod app_version {
    include!("src/app_version.rs");
}

fn main() {
    println!("cargo:rerun-if-changed=VERSION.txt");
    println!("cargo:rerun-if-changed=src/app_version.rs");
    println!("cargo:rerun-if-changed=assets/icons/app.rc");
    println!("cargo:rerun-if-changed=assets/icons/icon.ico");
    println!("cargo:rerun-if-changed=assets/icons/app.manifest");
    let version = app_version::APP_VERSION;
    assert_eq!(include_str!("VERSION.txt").trim(), version, "release versions must match");
    let parts: Vec<u16> = version.split('.').map(|part| {
        part.parse().expect("version components must be integers in 0..=65535")
    }).collect();
    assert!((3..=4).contains(&parts.len()), "expected three or four version components");
    assert_eq!(format!("{}.{}.{}", parts[0], parts[1], parts[2]),
        std::env::var("CARGO_PKG_VERSION").unwrap(), "Cargo package version must match the release");
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let header = format!(
            "#define ZSCLIP_VERSION_NUM {},{},{},{}\n#define ZSCLIP_VERSION_STR \"{}\"\n",
            parts[0], parts[1], parts[2], parts.get(3).copied().unwrap_or(0), version);
        let header_path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("app-version.h");
        std::fs::write(header_path, header).expect("write version resource header");
        embed_resource::compile("assets/icons/app.rc", embed_resource::NONE);
    }
}
