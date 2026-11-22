//! Nodes N1 in the app: Import Node Material and its commands.
use crate::tests::box_app;
use operandi_core::shader_tree::NodeKind;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/node-graphs/noise_ramp.json"
);

fn node_materials(app: &crate::Operandi) -> Vec<u64> {
    app.editor
        .scene
        .shader_tree
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::NodeMaterial(_)))
        .map(|n| n.id)
        .collect()
}

#[test]
fn import_command_adds_a_selected_layer_in_one_undo_step() {
    let mut app = box_app();
    app.execute(&format!("operandi.nodeMaterial.import filename:{FIXTURE}"));
    let ids = node_materials(&app);
    assert_eq!(ids.len(), 1, "{}", app.status);
    assert_eq!(app.shading.selected, ids);
    // Bare showGraph opens the selected layer's graph.
    app.execute("operandi.nodeMaterial.showGraph");
    assert!(app.graph_window.graph.is_some());
    // setDefault, and ? reads it back.
    let graph = app.editor.scene.shader_tree.node_graphs.graphs[0].id;
    app.execute(&format!(
        "operandi.nodeMaterial.setDefault graph:{graph} input:Roughness value:0.25"
    ));
    app.execute(&format!(
        "operandi.nodeMaterial.setDefault graph:{graph} input:Roughness value:?"
    ));
    assert_eq!(app.status, "0.25");
    app.execute("undo");
    app.execute("undo");
    assert!(node_materials(&app).is_empty());
}

#[test]
fn bad_files_leave_the_scene_and_say_why() {
    let mut app = box_app();
    let before = app.editor.scene.clone();
    let dir = std::env::temp_dir().join(format!("nm-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.json");
    std::fs::write(&bad, "{ not json").unwrap();
    app.execute(&format!(
        "operandi.nodeMaterial.import filename:{}",
        bad.display()
    ));
    assert!(
        app.status.contains("Not a node graph file"),
        "{}",
        app.status
    );
    let text = std::fs::read_to_string(FIXTURE)
        .unwrap()
        .replace("ShaderNodeTexNoise", "ShaderNodeTexBrick");
    let unsupported = dir.join("brick.json");
    std::fs::write(&unsupported, text).unwrap();
    app.execute(&format!(
        "operandi.nodeMaterial.import filename:{}",
        unsupported.display()
    ));
    assert!(
        app.status.contains("Noise Texture") && app.status.contains("ShaderNodeTexBrick"),
        "{}",
        app.status
    );
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(app.editor.scene, before);
}

#[test]
fn graph_view_lays_out_nodes_from_the_file() {
    let g = operandi_core::node_graph::import(&std::fs::read_to_string(FIXTURE).unwrap(), None)
        .unwrap()
        .remove(0);
    let places = crate::node_material_ui::layout(&g);
    assert_eq!(places.len(), g.material.nodes.len());
    // Left to right as in the file.
    assert!(places.windows(2).all(|w| w[0].x < w[1].x));
}

/// Node editor slice 1: a Material Group shows as its MaterialX graph,
/// made again after an edit, and closing forgets it.
#[test]
fn a_material_group_shows_as_nodes_and_follows_edits() {
    use operandi_core::Command;
    use operandi_core::shader_tree::{NewLayer, ShaderTreeEdit};
    let mut app = box_app();
    app.editor
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
            layer: NewLayer::Group,
            selected: None,
        }))
        .unwrap();
    let group = app
        .editor
        .scene
        .shader_tree
        .nodes
        .iter()
        .rfind(|n| matches!(n.kind, NodeKind::Group(_)))
        .expect("a material group")
        .id;
    app.execute(&format!("operandi.nodeMaterial.showGraph layer:{group}"));
    assert_eq!(app.graph_window.group, Some(group), "{}", app.status);
    assert_eq!(app.graph_window.graph, None);
    let view = app.group_graph(group).unwrap().unwrap();
    let cats: Vec<&str> = view
        .material
        .nodes
        .iter()
        .filter_map(|n| operandi_core::materialx::view::category(&n.idname))
        .collect();
    assert!(cats.contains(&"operandi_surface"), "{cats:?}");
    assert!(cats.contains(&"surfacematerial"), "{cats:?}");
    // An edit (a new revision) makes the view again.
    let revision = app.editor.revision;
    app.editor.revision = revision + 1;
    assert!(app.group_graph(group).unwrap().is_ok());
    assert_eq!(
        app.graph_window.group_view.as_ref().map(|v| v.0),
        Some(revision + 1)
    );
    // A group that's gone closes the view.
    app.execute("undo");
    assert!(app.group_graph(group).is_none());
}

/// A MaterialX graph layer now has a view: its nodes, by category.
#[test]
fn a_materialx_graph_layer_shows_its_nodes() {
    let mut app = box_app();
    let dir = std::env::temp_dir().join(format!("nm-mx-view-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("grain.mtlx");
    std::fs::write(
        &file,
        r#"<?xml version="1.0"?>
<materialx version="1.39">
  <noise3d name="grain" type="float"><input name="amplitude" type="float" value="0.2" /></noise3d>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="specular_roughness" type="float" nodename="grain" />
  </open_pbr_surface>
  <surfacematerial name="Grain" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="s" />
  </surfacematerial>
</materialx>"#,
    )
    .unwrap();
    let doc = operandi_core::materialx::read(&file).unwrap();
    let (preset, _) =
        operandi_core::materialx::import::preset(&doc, None, &dir, &|_| Some((2, 2))).unwrap();
    let graph = &preset.scene.shader_tree.node_graphs.graphs[0];
    let view = super::node_material_ui::materialx_view_for_tests(graph)
        .expect("a MaterialX graph")
        .unwrap();
    let names: Vec<&str> = view
        .material
        .nodes
        .iter()
        .map(|n| n.name.as_str())
        .collect();
    assert_eq!(names, ["grain", "s", "Grain"]);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = &mut app;
}
