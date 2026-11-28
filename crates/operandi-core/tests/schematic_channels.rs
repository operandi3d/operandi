//! #54: channels dragged into the Schematic: their item comes onto the
//! page (where it isn't yet) with those channels as ports, saved with the
//! workspace, one undo step.
use operandi_core::schematic_layout::Metrics;
use operandi_core::schematic_pages::{self, PageId};
use operandi_core::*;

fn line(e: &mut Editor, text: &str) -> Result<Option<String>> {
    command_line::run_line(e, text).expect("a headless command line")
}
fn ok(e: &mut Editor, text: &str) {
    line(e, text).unwrap_or_else(|err| panic!("{text}: {err}"));
}

fn scene() -> (Editor, u64, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mesh = e.scene.active().unwrap().id;
    ok(&mut e, "operandi.rig.workspaceAdd name:Arms");
    let ws = e.scene.animation.rig.workspaces.last().unwrap().id;
    (e, mesh, ws)
}

#[test]
fn dropped_channels_put_their_item_on_the_page_with_ports() {
    let (mut e, mesh, ws) = scene();
    let undo = e.history.undo.len();
    ok(
        &mut e,
        &format!(
            "operandi.rig.schematicChannels workspace:{ws} channels:{mesh}:pos.X,{mesh}:rot.Y at:40/60"
        ),
    );
    assert_eq!(e.history.undo.len(), undo + 1, "one undo step");
    let view = schematic_pages::page(&e.scene, PageId::Saved(ws), &Metrics::default());
    let key = format!("item:{mesh}");
    let node = view
        .schematic
        .nodes
        .iter()
        .find(|n| n.key == key)
        .expect("the item is on the page");
    for port in ["pos.X", "rot.Y"] {
        assert!(
            node.outputs.iter().any(|p| p == port),
            "{port}: {:?}",
            node.outputs
        );
        assert!(
            node.inputs.iter().any(|p| p == port),
            "{port}: {:?}",
            node.inputs
        );
    }
    assert_eq!(view.positions[&key], [40., 60.]);
    // Again with one more: the item stays put and gains the port.
    ok(
        &mut e,
        &format!("operandi.rig.schematicChannels workspace:{ws} channels:{mesh}:scl.Z at:300/300"),
    );
    let view = schematic_pages::page(&e.scene, PageId::Saved(ws), &Metrics::default());
    assert_eq!(view.positions[&key], [40., 60.], "already there: not moved");
    let node = view.schematic.nodes.iter().find(|n| n.key == key).unwrap();
    assert!(node.outputs.iter().any(|p| p == "scl.Z"));
    // Undo takes back one drop at a time; saved with the scene.
    e.dispatch(Command::Undo).unwrap();
    let w = &e.scene.animation.rig.workspaces.last().unwrap().nodes[0];
    assert_eq!(w.channels, vec!["pos.X".to_owned(), "rot.Y".to_owned()]);
    let text = serde_json::to_string(&e.scene.animation.rig.workspaces).unwrap();
    assert!(text.contains("rot.Y"), "{text}");
}

#[test]
fn bad_channels_are_refused() {
    let (mut e, mesh, ws) = scene();
    let err = line(
        &mut e,
        &format!("operandi.rig.schematicChannels workspace:{ws} channels:{mesh}:nothing.here"),
    )
    .unwrap_err();
    assert!(err.contains("nothing.here"), "{err}");
    let err = line(
        &mut e,
        &format!("operandi.rig.schematicChannels workspace:{ws} channels:999:pos.X"),
    )
    .unwrap_err();
    assert!(!err.is_empty());
}
