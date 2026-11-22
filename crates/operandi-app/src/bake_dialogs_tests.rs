use super::*;
use glam::DVec3;

/// An app with one box mesh item that has its own UVs.
fn app_with_box() -> Operandi {
    let mut a = Operandi::empty();
    a.editor.scene.new_mesh().unwrap();
    let id = a.editor.scene.items.last().unwrap().id;
    a.editor.scene.items.last_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1)
            .unwrap()
            .into();
    a.editor.scene.select_item(Some(id));
    a
}

#[test]
fn bare_commands_open_their_dialogs_and_typed_lines_run() {
    let mut a = app_with_box();
    a.execute("operandi.uv.atlas");
    assert!(matches!(a.bake_dialog, Some(BakeDialog::Atlas(_))));
    a.bake_dialog = None;
    a.execute("operandi.bake.curvature");
    assert!(matches!(
        a.bake_dialog,
        Some(BakeDialog::Bake(_, BakeMap::Curvature))
    ));
    a.bake_dialog = None;
    a.execute("operandi.bake.padImage");
    assert!(matches!(a.bake_dialog, Some(BakeDialog::Pad(_))));
    a.bake_dialog = None;

    // Typed with arguments: Atlas is one undo step, then a bake writes.
    let undo = a.editor.undo.len();
    a.execute("operandi.uv.atlas angle:0.7");
    assert_eq!(a.editor.undo.len(), undo + 1, "{}", a.status);
    assert!(
        a.editor
            .scene
            .active()
            .unwrap()
            .mesh
            .uv_maps
            .contains_key("Texture")
    );
    let dir = std::env::temp_dir().join(format!("operandi-bake-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.png");
    a.execute(&format!(
        "operandi.bake.thickness size:64 rays:4 filename:{}",
        file.display()
    ));
    assert!(file.is_file(), "{}", a.status);
    assert!(
        a.status.starts_with("Baked Thickness 64×64"),
        "{}",
        a.status
    );
    // The bake changed nothing in the scene.
    assert_eq!(a.editor.undo.len(), undo + 1);
}

#[test]
fn the_save_panel_answer_runs_the_pending_line() {
    let mut a = app_with_box();
    a.execute("operandi.uv.atlas gaps:2");
    let dir = std::env::temp_dir().join(format!("operandi-bake-ui2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    a.bake_pending = Some("operandi.bake.curvature size:32 rays:4".into());
    a.bake_to(&dir.join("box curvature"));
    assert!(dir.join("box curvature.png").is_file(), "{}", a.status);
    assert!(a.bake_pending.is_none());
    // Pad writes a copy beside the picture.
    a.bake_pending = Some("operandi.bake.padImage padding:2".into());
    a.pad_texture(&dir.join("box curvature.png"));
    assert!(
        dir.join("box curvature_padded.png").is_file(),
        "{}",
        a.status
    );
}

#[test]
fn dialogs_need_a_mesh() {
    let mut a = Operandi::empty();
    a.execute("operandi.bake.thickness");
    assert!(a.bake_dialog.is_none());
    assert!(a.status.contains("select a mesh"), "{}", a.status);
}
