// SPDX-License-Identifier: GPL-2.0-or-later
//! Builds the vendored QuadriFlow (with LEMON and Eigen) into one static
//! library.

fn main() {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let vendor = root.join("vendor");
    let qf = vendor.join("quadriflow");
    let lemon = qf.join("3rd/lemon-1.3.1");

    let mut b = cc::Build::new();
    b.cpp(true)
        .std("c++17")
        .warnings(false)
        // Only Eigen's MPL-2.0 parts may be used.
        .define("EIGEN_MPL2_ONLY", None)
        .define("NDEBUG", None)
        .include(root.join("cpp"))
        .include(vendor.join("eigen"))
        .include(qf.join("src"))
        .include(qf.join("3rd/pcg32"))
        .include(qf.join("3rd/pss"))
        .include(&lemon);
    // Remeshing in a debug build would take minutes; always optimise the
    // C++ side.
    b.opt_level(2).debug(false);
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // LEMON tests `WIN32`, which MSVC doesn't define (only `_WIN32`);
        // without it LEMON includes POSIX headers like unistd.h.
        b.define("WIN32", None);
        b.define("_USE_MATH_DEFINES", None);
        b.define("NOMINMAX", None);
    }

    let qf_src = [
        "adjacent-matrix.cpp",
        "dedge.cpp",
        "hierarchy.cpp",
        "loader.cpp",
        "localsat.cpp",
        "merge-vertex.cpp",
        "optimizer.cpp",
        "parametrizer.cpp",
        "parametrizer-flip.cpp",
        "parametrizer-int.cpp",
        "parametrizer-mesh.cpp",
        "parametrizer-scale.cpp",
        "parametrizer-sing.cpp",
        "subdivide.cpp",
    ];
    for f in qf_src {
        b.file(qf.join("src").join(f));
    }
    for f in [
        "arg_parser.cc",
        "base.cc",
        "color.cc",
        "lp_base.cc",
        "lp_skeleton.cc",
        "random.cc",
        "bits/windows.cc",
    ] {
        b.file(lemon.join("lemon").join(f));
    }
    b.file(root.join("cpp/quadriflow_capi.cpp"));
    b.compile("operandi_remesh");

    println!("cargo:rerun-if-changed=cpp");
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=build.rs");
}
