//! Links liboperandi_cycles when the `cycles` feature is on.
//!
//! The library comes from scripts/build-cycles.sh, which installs it under
//! development/cycles/install/lib. OPERANDI_CYCLES_LIB_DIR overrides that.
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=OPERANDI_CYCLES_LIB_DIR");
    if std::env::var_os("CARGO_FEATURE_CYCLES").is_none() {
        return;
    }
    let dir = std::env::var_os("OPERANDI_CYCLES_LIB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../development/cycles/install/lib")
        });
    let dir = dir.canonicalize().unwrap_or_else(|_| {
        panic!(
            "Cycles library folder {} not found: run scripts/build-cycles.sh \
             or set OPERANDI_CYCLES_LIB_DIR",
            dir.display()
        )
    });
    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=dylib=operandi_cycles");
    // Lets this crate's own examples and tests find the library at run time;
    // operandi-app reads `root` (DEP_OPERANDI_CYCLES_ROOT) to do the same.
    if !cfg!(windows) {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dir.display());
    }
    println!("cargo:root={}", dir.display());
    // Kernel sources for GPU rendering, installed next to the library.
    let resources = dir.join("../share/cycles");
    if let Ok(resources) = resources.canonicalize() {
        println!(
            "cargo:rustc-env=OPERANDI_CYCLES_DEV_RESOURCES={}",
            resources.display()
        );
    }
}
