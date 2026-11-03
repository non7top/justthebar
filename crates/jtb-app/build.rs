use std::env;
use std::path::PathBuf;
use std::process::Command;

// Without a supportedOS manifest Windows reports version 6.2 to the process, and Open-Shell's
// library, which runs inside it, then skips its per-monitor DPI handling.
fn main() {
    println!("cargo:rerun-if-changed=jtb-app.rc");
    println!("cargo:rerun-if-changed=jtb-app.manifest");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let object = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("jtb-app-manifest.o");
    let status = Command::new("x86_64-w64-mingw32-windres")
        .current_dir(&dir)
        .args(["jtb-app.rc", "-O", "coff", "-o"])
        .arg(&object)
        .status()
        .expect("x86_64-w64-mingw32-windres is part of binutils-mingw-w64-x86-64");
    assert!(status.success(), "windres failed");
    println!("cargo:rustc-link-arg-bins={}", object.display());
}
