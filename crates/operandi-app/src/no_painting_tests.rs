//! A build without the `painting` feature (`--no-default-features`;
//! painting is on by default since 2026-11-19) carries no painting: no
//! painting commands in the registry and no painting menu entries.
use super::*;

#[test]
fn default_build_has_no_painting_commands_or_menus() {
    let a = Operandi::empty();
    let painting: Vec<&str> = a
        .commands
        .iter()
        .map(|c| c.entry.name)
        .filter(|n| n.starts_with("operandi.image.") || n.starts_with("operandi.paint."))
        .collect();
    assert!(
        painting.is_empty(),
        "painting commands in a default build: {painting:?}"
    );
    let ctx = egui::Context::default();
    for menu in a.menu_model(&ctx) {
        for item in menu.leaves() {
            for word in [
                "New Image",
                "Add Color Texture",
                "Save Image",
                "Flatten Layers",
            ] {
                assert!(
                    !item.title.contains(word),
                    "{} in {}",
                    item.title,
                    menu.title
                );
            }
        }
    }
}
