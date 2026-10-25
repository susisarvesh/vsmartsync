fn main() {
    // The Dock icon is compiled into the binary. Cargo must rebuild when it changes.
    println!("cargo:rerun-if-changed=icons/icon.icns");
    println!("cargo:rerun-if-changed=icons/icon.png");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    tauri_build::build()
}
