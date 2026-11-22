//! Window ▸ Node View and its key (node_view_open.rs).
use crate::node_view_open::NOTHING_TO_SHOW;
use crate::tests::box_app;
use operandi_core::Command;
use operandi_core::shader_tree::{NewLayer, NodeKind, ShaderTreeEdit};

fn add(app: &mut crate::Operandi, layer: NewLayer, selected: Option<u64>) -> u64 {
    app.editor
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Add { layer, selected }))
        .unwrap();
    app.editor.scene.shader_tree.next_id - 1
}

#[test]
fn node_view_opens_the_selected_group_or_the_group_a_layer_is_in() {
    let mut app = box_app();
    let group = add(&mut app, NewLayer::Group, None);
    assert!(matches!(
        app.editor.scene.shader_tree.get(group).map(|n| &n.kind),
        Some(NodeKind::Group(_))
    ));
    // A layer inside the group opens its parent group.
    let inner = add(&mut app, NewLayer::Constant, Some(group));
    assert_eq!(
        app.editor
            .scene
            .shader_tree
            .get(inner)
            .and_then(|n| n.parent),
        Some(group),
        "the layer sits in the group"
    );
    app.shading.selected = vec![inner];
    app.execute("operandi.window.nodeView");
    assert_eq!(app.graph_window.group, Some(group), "{}", app.status);

    // Closed, with nothing selected, the key opens the first group.
    app.graph_window.group = None;
    app.shading.selected.clear();
    app.execute("Node View");
    let first = app
        .editor
        .scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Group(_)))
        .map(|n| n.id);
    assert_eq!(app.graph_window.group, first, "{}", app.status);

    // Selecting the group itself opens it too.
    app.graph_window.group = None;
    app.shading.selected = vec![group];
    app.execute("Node View");
    assert_eq!(app.graph_window.group, Some(group));
}

#[test]
fn node_view_with_no_material_group_says_so() {
    let mut app = box_app();
    app.editor
        .scene
        .shader_tree
        .nodes
        .retain(|n| !matches!(n.kind, NodeKind::Group(_)) && n.parent.is_none());
    app.shading.selected.clear();
    app.graph_window.group = None;
    app.execute("operandi.window.nodeView");
    assert_eq!(app.graph_window.group, None);
    assert_eq!(app.status, NOTHING_TO_SHOW);
}

#[test]
fn node_view_has_its_key_and_window_menu_entries() {
    assert_eq!(
        crate::input::default_bindings()
            .get("Node View")
            .map(String::as_str),
        Some("Cmd+Shift+N")
    );
    // No other default action shares the chord.
    let taken: Vec<_> = crate::input::default_bindings()
        .into_iter()
        .filter(|(a, c)| a != "Node View" && c.eq_ignore_ascii_case("Cmd+Shift+N"))
        .collect();
    assert!(taken.is_empty(), "{taken:?}");
    // Window ▸ Panels ▸ Node View runs the command (the only Window menu
    // entry for it); Layout ▸ Windows/Palettes lists it too.
    assert!(!include_str!("../assets/forms/menu.window.json").contains("operandi.window.nodeView"));
    assert!(
        include_str!("../assets/forms/menu.layout.palettes.json")
            .contains("\"operandi.window.nodeView\"")
    );
    assert!(include_str!("menu_forms.rs").contains("\"operandi.window.nodeView\".into()"));
}
