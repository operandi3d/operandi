//! Adding a layer with a selected id the tree doesn't have (a closed
//! scene's Shader Tree selection) puts it at the top level, never under
//! itself (the "Add Color Texture hangs after closing a scene" report).
use operandi_core::shader_tree::*;

#[test]
fn a_stale_selected_id_adds_at_the_top_level() {
    let mut tree = ShaderTree::default();
    let stale = tree.next_id;
    let id = tree.add(NewLayer::ImageMap, Some(stale)).unwrap();
    assert_eq!(id, stale, "the new layer takes the stale id");
    assert_ne!(
        tree.get(id).unwrap().parent,
        Some(id),
        "never its own parent"
    );
    tree.validate().unwrap();
}
