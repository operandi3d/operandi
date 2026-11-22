//! Compiles the vendored Alembic (Ogawa core only, no HDF5) and Imath
//! sources plus Operandi's C shim into one static library with the `cc`
//! crate, so no CMake or system Alembic is needed on any platform.
use std::path::{Path, PathBuf};

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "cpp"))
        .collect();
    entries.sort();
    out.extend(entries);
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let vendor = root.join("vendor");
    let mut files = Vec::new();
    for lib in [
        "Util",
        "AbcCoreAbstract",
        "Ogawa",
        "AbcCoreOgawa",
        "Abc",
        "AbcGeom",
    ] {
        sources(&vendor.join("alembic/Alembic").join(lib), &mut files);
    }
    sources(&vendor.join("imath/src"), &mut files);
    files.push(root.join("shim/operandi_alembic.cpp"));
    println!("cargo:rerun-if-changed=shim");
    println!("cargo:rerun-if-changed=vendor");
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .files(&files)
        .include(vendor.join("alembic"))
        .include(vendor.join("imath"))
        // Alembic includes <Imath/...>; Imath's own sources include bare names.
        .include(vendor.join("imath/Imath"))
        .include(root.join("shim"))
        .define("IMATH_HALF_NO_LOOKUP_TABLE", None)
        .warnings(false);
    if build.get_compiler().is_like_msvc() {
        build.flag("/EHsc");
    }
    build.compile("operandi_alembic");
}
