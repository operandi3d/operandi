//! Missing tools group 3 (specifications/missing-tools-g3): Topology Mode
//! (MT7), the Topology tab's Offset, Foreground Opacity and Selection Hit
//! Size fields (MT8-MT10), and its Instance Mirror and Merge Instance
//! buttons (MT11, MT12).
use super::tests::draw;
use super::toolbox_form_tests::button_by_line;
use super::*;
use std::collections::BTreeSet;

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1)
            .unwrap()
            .into();
    a
}

#[test]
fn mt7_topology_mode_toggles_per_view_and_is_saved_with_it() {
    let mut a = app();
    assert!(!a.topology_mode(), "off by default");
    assert!(!a.line_state("view.topologyMode").unwrap().on);
    a.execute("view.topologyMode");
    assert!(a.topology_mode(), "{}", a.status);
    assert!(a.line_state("view.topologyMode").unwrap().on, "lit when on");
    a.execute("view.topologyMode off");
    assert!(!a.topology_mode());
    a.execute("view.topologyMode on");
    // Saved with the view's settings (MT7-04).
    let saved = serde_json::to_string(&a.display.settings).unwrap();
    let back: crate::view_display::Settings = serde_json::from_str(&saved).unwrap();
    assert!(back.drawing.topology);
    // Display only: the selection and the scene stay as they were.
    let scene = a.editor.scene.clone();
    a.execute("view.topologyMode off");
    assert!(a.editor.scene == scene);
}

#[test]
fn mt7_02_the_active_look_is_see_through_flat_and_on_top_in_the_topo_colours() {
    use crate::theme::{ColorId, color};
    use crate::view_display::{Overlay, TransparencyMode};
    let base = crate::gpu::Look::of(crate::view_display::ViewDisplay::default().settings.active);
    let look = crate::topology_mode::topology_look(base, 50);
    let face = color(ColorId::TopoFace).to_array();
    let t = look.transparency;
    assert_eq!(t.mode, TransparencyMode::Ghost);
    assert!(t.overlay && t.flat, "over everything, flat");
    assert_eq!(t.front_color, [face[0], face[1], face[2]]);
    assert_eq!((t.front_opacity, t.back_opacity), (50, 50));
    assert_eq!(look.overlay, Overlay::Colored);
    assert_eq!(look.colored, color(ColorId::TopoEdge));
    assert_eq!(look.plain_wire, Some(color(ColorId::TopoBackEdge)));
    assert!(look.see_through());
    // The spec's default colours.
    assert_eq!(color(ColorId::TopoFace), Color32::from_rgb(51, 51, 51));
    assert_eq!(color(ColorId::TopoEdge), Color32::from_rgb(28, 64, 105));
    assert_eq!(color(ColorId::TopoBackEdge), Color32::from_rgb(28, 64, 105));
    assert_eq!(
        color(ColorId::TopoVertices),
        Color32::from_rgb(97, 158, 235)
    );
}

#[test]
fn mt7_03_points_take_topo_vertices_and_back_facing_ones_hide() {
    let item = app().editor.scene.active().unwrap().clone();
    let mut resident = crate::gpu_scene::Resident::default();
    let topo = |hide_back: bool| crate::gpu_scene::TopologyDots {
        color: Color32::from_rgb(97, 158, 235),
        // An eye on +Z, looking at the cube.
        hide_back: hide_back.then_some((DVec3::new(0., 0., 5.), true)),
    };
    let mut dots = |t| {
        resident
            .dots(
                &item,
                glam::DMat4::IDENTITY,
                true,
                true,
                true,
                &[],
                &BTreeSet::new(),
                [3., 4.],
                false,
                &|| None,
                t,
            )
            .0
    };
    let all = dots(Some(topo(false)));
    assert_eq!(all.len(), 8 * 6, "every point, six corners each");
    assert!(
        all.iter()
            .all(|d| d.color == Color32::from_rgb(97, 158, 235))
    );
    // The cube's four far points face away from an eye on +Z.
    let front = dots(Some(topo(true)));
    assert_eq!(front.len(), 4 * 6);
    assert!(front.iter().all(|d| d.center[2] > 0.));
}

#[test]
fn mt9_10_the_topology_tab_fields_set_the_preferences() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = app();
    a.category = toolbox::Category::Topology;
    draw(&mut a, &ctx, vec2(1440., 2200.), vec![]);
    let rects = crate::ui_inventory::take_rects();
    for label in ["Foreground Opacity", "Selection Hit Size"] {
        assert!(rects.contains_key(label), "{label} on the Topology tab");
    }
    // MT8 Offset was dropped (Jason, 2026-11-21).
    assert!(!rects.contains_key("Offset"));
    // Defaults (MT9 50 %, MT10 12 px, unchanged).
    assert_eq!(a.viewport_preferences.topology_opacity, 50);
    assert_eq!(a.selection_preferences.hit_size, 12);
    // The lines the fields run.
    a.execute("pref.value display.topology_opacity 0.75");
    a.execute("pref.value selection.hit_size 20");
    assert_eq!(a.viewport_preferences.topology_opacity, 75);
    assert_eq!(a.selection_preferences.hit_size, 20);
    // The tab's buttons run real commands now.
    for line in ["view.topologyMode", "Instance Mirror", "Merge Instance"] {
        button_by_line(&a, line);
    }
}

#[test]
fn mt12_merge_instance_from_the_command_line() {
    let mut a = app();
    let source = a.editor.scene.active().unwrap().id;
    a.execute("Instance");
    a.run(Command::SelectItem(Some(source)));
    a.execute("Merge Instance");
    assert_eq!(a.editor.scene.items.len(), 1, "{}", a.status);
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 12);
    a.execute("Merge Instance");
    assert!(
        a.status.contains("Select a mesh that has an instance."),
        "{}",
        a.status
    );
}

/// MT8 dropped (Jason, 2026-11-21): a display preferences file saved
/// with Topology Offset still loads, and it isn't written again.
#[test]
fn mt8_a_saved_topology_offset_still_loads_and_goes() {
    let d = crate::viewport_preferences::ViewportPreferences::default();
    let mut json = serde_json::to_value(d).unwrap();
    assert!(json.get("topology_offset").is_none(), "not written");
    json["topology_offset"] = serde_json::json!(0.5);
    let back: crate::viewport_preferences::ViewportPreferences =
        serde_json::from_value(json).expect("old files load");
    assert!(
        serde_json::to_value(back)
            .unwrap()
            .get("topology_offset")
            .is_none()
    );
}
