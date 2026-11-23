//! With the `python` feature: records where the Python this build links
//! against keeps its standard library (`sys.base_prefix`), so a
//! development build finds it without a bundled copy. The app bundle's
//! own copy (Contents/Resources/python) wins at run time (embed.rs).
fn main() {
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");
    println!("cargo:rerun-if-env-changed=OPERANDI_PYTHON_BUILD_HOME");
    if std::env::var_os("CARGO_FEATURE_PYTHON").is_none() {
        return;
    }
    if let Ok(home) = std::env::var("OPERANDI_PYTHON_BUILD_HOME") {
        println!("cargo:rustc-env=OPERANDI_PYTHON_BUILD_HOME={home}");
        return;
    }
    let python = std::env::var("PYO3_PYTHON").unwrap_or_else(|_| "python3".into());
    let out = std::process::Command::new(&python)
        .args(["-c", "import sys; print(sys.base_prefix)"])
        .output();
    if let Ok(out) = out
        && out.status.success()
    {
        let home = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        println!("cargo:rustc-env=OPERANDI_PYTHON_BUILD_HOME={home}");
    }
}
