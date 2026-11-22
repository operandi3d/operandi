//! Material editing item 6: which shader tree layers have an image (the
//! row's double-click shows it in the Image Viewer).
use operandi_core::shader_tree::{NewLayer, NodeKind, ShaderTree, TextureSource};

#[test]
fn only_image_maps_with_an_image_have_one() {
    let mut tree = ShaderTree::default();
    let map = tree.add(NewLayer::ImageMap, None).unwrap();
    let group = tree.add(NewLayer::Group, None).unwrap();
    let material = tree.add(NewLayer::Material, None).unwrap();
    // No image chosen yet.
    assert_eq!(tree.layer_image(map), None);
    let i = tree.nodes.iter().position(|n| n.id == map).unwrap();
    if let NodeKind::Texture(t) = &mut tree.nodes[i].kind
        && let TextureSource::ImageMap { image, .. } = &mut t.source
    {
        *image = Some(7);
    }
    assert_eq!(tree.layer_image(map), Some(7));
    assert_eq!(tree.layer_image(group), None);
    assert_eq!(tree.layer_image(material), None);
    assert_eq!(tree.layer_image(9999), None);
}
