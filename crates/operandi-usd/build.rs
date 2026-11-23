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
    // With Python scripting (`python`): the bridge carries the static
    // Python OpenUSD needs and exports its symbols, so the app's own
    // libpython must come first on the link line. Otherwise the
    // interpreter's calls bind to the bridge's copy (another build, with
    // other built-in modules), and OpenUSD and extension modules, which
    // look Python up by name, find it first too. One interpreter: ours.
    if std::env::var_os("CARGO_FEATURE_PYTHON").is_some()
        && let Some((lib, name)) = python_lib()
    {
        println!("cargo:rustc-link-search=native={}", lib.display());
        println!("cargo:rustc-link-lib=dylib={name}");
    }
    println!("cargo:rustc-link-lib=dylib=operandi_usd");
    // Lets this crate's own tests find the library at run time;
    // operandi-app reads `root` (DEP_OPERANDI_USD_ROOT) to do the same.
    if !cfg!(windows) {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dir.display());
    }
    println!("cargo:root={}", dir.display());
}

/// The bundled libpython's folder and name, from PyO3's config file
/// (`lib_dir=`, `lib_name=`), as scripts/bundle-dev.sh and merge-check.sh
/// set it.
fn python_lib() -> Option<(PathBuf, String)> {
    println!("cargo:rerun-if-env-changed=PYO3_CONFIG_FILE");
    let config = std::fs::read_to_string(std::env::var_os("PYO3_CONFIG_FILE")?).ok()?;
    let field = |k: &str| {
        config
            .lines()
            .find_map(|l| l.strip_prefix(k))
            .map(|v| v.trim().to_owned())
    };
    let dir = PathBuf::from(field("lib_dir=")?);
    let name = field("lib_name=")?;
    dir.join(format!("lib{name}.dylib"))
        .is_file()
        .then_some((dir, name))
}
