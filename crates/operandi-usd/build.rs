//! Links liboperandi_usd when the `usd` feature is on.
//!
//! The library comes from scripts/build-usd.sh, which installs it (with
//! OpenUSD and the libraries it loads) under development/usd/install/lib.
//! OPERANDI_USD_LIB_DIR overrides that.
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=OPERANDI_USD_LIB_DIR");
    if std::env::var_os("CARGO_FEATURE_USD").is_none() {
        return;
    }
    let dir = std::env::var_os("OPERANDI_USD_LIB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../development/usd/install/lib")
        });
    let dir = dir.canonicalize().unwrap_or_else(|_| {
        panic!(
            "USD library folder {} not found: run scripts/build-usd.sh \
             or set OPERANDI_USD_LIB_DIR",
            dir.display()
        )
    });
    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=dylib=operandi_usd");
    // Lets this crate's own tests find the library at run time;
    // operandi-app reads `root` (DEP_OPERANDI_USD_ROOT) to do the same.
    if !cfg!(windows) {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dir.display());
    }
    println!("cargo:root={}", dir.display());
}
