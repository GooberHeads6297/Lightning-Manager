fn main() {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let ico_data = include_bytes!("Icons/Icon.ico");

    let rc_path = out_dir.join("resource.rc");
    let ico_path = out_dir.join("Icon.ico");
    let obj_path = out_dir.join("resource.o");

    std::fs::write(&rc_path, b"1 ICON \"Icon.ico\"\n").unwrap();
    std::fs::write(&ico_path, ico_data).unwrap();

    let status = std::process::Command::new("windres")
        .arg(&rc_path)
        .arg("-o")
        .arg(&obj_path)
        .status()
        .expect("windres failed – is MinGW installed and on PATH?");

    if !status.success() {
        panic!("windres exited with non-zero status");
    }

    println!("cargo:rustc-link-arg={}", obj_path.display());
}
