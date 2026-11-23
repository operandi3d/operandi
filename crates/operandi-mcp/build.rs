//! Gives `operandi-mcp`'s main thread 8 MB of stack on Windows, as macOS and
//! Linux do (Windows' default is 1 MB). The headless session runs on the
//! main thread and overflowed 1 MB answering its first requests. The same
//! setting is in `operandi-app/build.rs`.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
        Ok("msvc") => println!("cargo:rustc-link-arg-bins=/STACK:8388608"),
        Ok("gnu") => println!("cargo:rustc-link-arg-bins=-Wl,--stack,8388608"),
        _ => {}
    }
}
