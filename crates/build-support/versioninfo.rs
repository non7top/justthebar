//! Shared by the build scripts of the shipped binaries through `#[path]`; keep it dependency-free.
use std::env;

/// The release version when the release workflow sets it, else the crate version.
pub fn version() -> String {
    env::var("RELEASE_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION"))
}

/// The `owner/repo` a release build claims to come from; unset for local and PR builds.
fn provenance_repo() -> Option<String> {
    let repo = env::var("PROVENANCE_REPO").ok().filter(|r| !r.is_empty())?;
    let valid = repo.matches('/').count() == 1
        && repo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.-/".contains(c));
    assert!(valid, "PROVENANCE_REPO must be owner/repo, got {repo:?}");
    Some(repo)
}

pub fn rerun_on_env_change() {
    println!("cargo:rerun-if-env-changed=RELEASE_VERSION");
    println!("cargo:rerun-if-env-changed=PROVENANCE_REPO");
}

/// Resource script text with a VERSIONINFO block, carrying the `ProvenanceRepo` claim on release builds.
pub fn resource_script(description: &str) -> String {
    let version = version();
    let mut parts = version.split(|c: char| !c.is_ascii_digit());
    let mut next = || {
        parts
            .next()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(0)
    };
    let numeric = format!("{},{},{},0", next(), next(), next());
    let claim = provenance_repo()
        .map(|r| format!("      VALUE \"ProvenanceRepo\", \"{r}\"\n"))
        .unwrap_or_default();
    format!(
        "1 VERSIONINFO\nFILEVERSION {numeric}\nPRODUCTVERSION {numeric}\nBEGIN\n  BLOCK \"StringFileInfo\"\n  BEGIN\n    BLOCK \"040904b0\"\n    BEGIN\n      VALUE \"ProductName\", \"JustTheBar\"\n      VALUE \"FileDescription\", \"{description}\"\n      VALUE \"FileVersion\", \"{version}\"\n      VALUE \"ProductVersion\", \"{version}\"\n{claim}    END\n  END\n  BLOCK \"VarFileInfo\"\n  BEGIN\n    VALUE \"Translation\", 0x409, 1200\n  END\nEND\n"
    )
}

/// Compiles `script` with windres into `name` in OUT_DIR and returns the object's path.
pub fn compile(script: &str, name: &str) -> std::path::PathBuf {
    let out = std::path::PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let rc = out.join(format!("{name}.rc"));
    std::fs::write(&rc, script).expect("write resource script");
    let object = out.join(format!("{name}.o"));
    let status = std::process::Command::new("x86_64-w64-mingw32-windres")
        .current_dir(&out)
        .arg(&rc)
        .args(["-O", "coff", "-o"])
        .arg(&object)
        .status()
        .expect("x86_64-w64-mingw32-windres is part of binutils-mingw-w64-x86-64");
    assert!(status.success(), "windres failed");
    object
}
