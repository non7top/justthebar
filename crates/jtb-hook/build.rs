#[path = "../build-support/versioninfo.rs"]
mod versioninfo;

fn main() {
    println!("cargo:rerun-if-changed=../build-support/versioninfo.rs");
    versioninfo::rerun_on_env_change();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let script = versioninfo::resource_script("JustTheBar flyout hook");
    let object = versioninfo::compile(&script, "jtb-hook-resources");
    println!("cargo:rustc-link-arg-cdylib={}", object.display());
}
