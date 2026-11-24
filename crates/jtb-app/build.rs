#[path = "../build-support/versioninfo.rs"]
mod versioninfo;

use std::env;
use std::path::PathBuf;

// Without a supportedOS manifest Windows reports version 6.2 to the process, and Open-Shell's
// library, which runs inside it, then skips its per-monitor DPI handling.
fn main() {
    println!("cargo:rerun-if-changed=jtb-app.manifest");
    println!("cargo:rerun-if-changed=../build-support/versioninfo.rs");
    versioninfo::rerun_on_env_change();
    println!("cargo:rustc-env=JTB_VERSION={}", versioninfo::version());
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .join("jtb-app.manifest");
    let script = format!(
        "1 24 \"{}\"\n{}",
        manifest.display().to_string().replace('\\', "/"),
        versioninfo::resource_script("JustTheBar taskbar replacement")
    );
    let object = versioninfo::compile(&script, "jtb-app-resources");
    println!("cargo:rustc-link-arg-bins={}", object.display());
}
