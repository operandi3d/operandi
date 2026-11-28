use super::tests::{draw, pointer};
use super::*;
use operandi_core::{Mesh, scene::Transform};

fn fresh() -> Operandi {
    let mut app = Operandi::empty();
    app.initialize_new_document();
    app
}

fn assert_baseline(app: &Operandi) {
    let scene = &app.editor.scene;
    // CL-1: the empty mesh, then one camera and one directional light;
    // ENV §1: and the Environment.
    assert_eq!(scene.items.len(), 4);
    assert_eq!(scene.items[1].name, "Camera");
    assert!(scene.items[1].camera.is_some());
    assert_eq!(scene.items[2].name, "Directional Light");
    assert!(scene.items[2].light.is_some());
    assert_eq!(scene.render_camera(), Some(scene.items[1].id));
    let item = scene.active().expect("initial mesh is active");
    assert_eq!(item.id, 1);
    assert_eq!(item.name, "Mesh");
    assert_eq!(item.transform, Transform::default());
    assert_eq!(*item.mesh, Mesh::default());
    assert_eq!(scene.selection.item, Some(item.id));
    // New scene spec NS01: Item selection mode.
    assert_eq!(scene.selection.mode, Mode::Item);
    assert_eq!(scene, &app.saved);
    assert!(!app.dirty());
    assert!(app.path.is_none());
    assert!(app.editor.session.live.is_none());
    assert!(app.editor.history.undo.is_empty());
    assert!(app.editor.history.redo.is_empty());
}

fn assert_title(app: &mut Operandi, ctx: &Context, document: &str, dirty: bool) {
    let _ = draw(app, ctx, vec2(1440., 900.), vec![]);
    app.sync_window_title(ctx);
    assert_eq!(app.dirty(), dirty);
    let marker = if dirty { " *" } else { "" };
    assert_eq!(
        app.window_title,
        format!("{} — {document}{marker}", app.environment.title())
    );
}

#[test]
fn fresh_scene_title_tracks_edits_and_save() {
    check_new_scene_title(false);
}

#[test]
fn fresh_scene_from_native_template_with_import_provenance_starts_clean() {
    check_new_scene_title(true);
}

fn check_new_scene_title(use_template: bool) {
    let mut app = Operandi::empty();
    let ctx = Context::default();
    theme::install(&ctx);
    let root = app.environment.paths().data_root().to_owned();
    if use_template {
        let mut scene = fresh().editor.scene.clone();
        scene.import_source = Some(operandi_core::ImportSource {
            path: root.join("Original.lxo").display().to_string(),
            producer: "Test fixture".into(),
            device: None,
            inode: None,
        });
        let path = root.join("Template.opscene");
        persistence::save(&path, &scene).unwrap();
        app.defaults_preferences.default_scene = path.display().to_string();
    }
    app.initialize_new_document();
    app.apply_launch_selection();
    app.start_layouts();

    assert_eq!(app.editor.scene.import_source.is_some(), use_template);
    assert_title(&mut app, &ctx, "Untitled", false);
    app.run(Command::NewMesh);
    assert_title(&mut app, &ctx, "Untitled", true);

    app.menu_action(&ctx, app_menu::line_action("scene.new"));
    assert_eq!(app.scenes.tabs.len(), 2);
    assert_eq!(app.editor.scene.import_source.is_some(), use_template);
    assert_title(&mut app, &ctx, "Untitled", false);
    app.run(Command::NewMesh);
    assert_title(&mut app, &ctx, "Untitled", true);

    assert!(app.save(root.join("Saved.opscene")));
    assert_title(&mut app, &ctx, "Saved.opscene", false);
}

#[test]
fn fresh_baseline_and_primitive_in_existing_mesh() {
    let mut app = fresh();
    assert_baseline(&app);
    app.activate(Tool::Box);
    app.run(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        ..Default::default()
    }));
    app.run(Command::Apply);
    app.run(Command::Drop);
    assert_eq!(app.editor.scene.items.len(), 4);
    assert!(!app.editor.scene.active().unwrap().mesh.faces.is_empty());
}

#[test]
fn new_adds_a_scene_and_keeps_the_old_one() {
    let mut app = fresh();
    app.run(Command::NewMesh);
    let original = app.editor.scene.clone();
    app.request_file(FileAction::New);
    assert!(app.pending.is_none(), "New asks nothing");
    assert_baseline(&app);
    app.request_file(FileAction::New);
    assert_baseline(&app);
    assert_eq!(app.scenes.tabs.len(), 3);
    assert!(app.switch_scene(0));
    assert_eq!(app.editor.scene, original);
}

#[test]
fn opening_and_recovery_read_preserve_stored_scenes() {
    let mut app = fresh();
    let root = app.environment.paths().data_root().to_owned();
    let empty = Scene::default();
    let mut populated = Scene::default();
    populated.new_mesh().unwrap();
    populated.new_mesh().unwrap();
    for (name, scene) in [("empty", &empty), ("populated", &populated)] {
        let path = root.join(format!("{name}.opscene"));
        persistence::save(&path, scene).unwrap();
        app.open(path.clone());
        assert_eq!(&app.editor.scene, scene);
        assert_eq!(&app.saved, scene);
        assert_eq!(app.path.as_ref(), Some(&path));
        persistence::save(&app.environment.paths().recovery(), scene).unwrap();
        app.read_recovery();
        assert_eq!(app.recovery.as_ref(), Some(scene));
    }
}

#[test]
fn recover_button_restores_stored_scenes_without_injecting_a_mesh() {
    for scene in [Scene::default(), {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.new_mesh().unwrap();
        scene
    }] {
        let mut app = fresh();
        persistence::save(&app.environment.paths().recovery(), &scene).unwrap();
        app.read_recovery();
        let ctx = Context::default();
        ctx.enable_accesskit();
        theme::install(&ctx);
        let size = vec2(1440., 900.);
        let _ = draw(&mut app, &ctx, size, vec![]);
        let out = draw(&mut app, &ctx, size, vec![]);
        let node = out
            .platform_output
            .accesskit_update
            .unwrap()
            .nodes
            .into_iter()
            .map(|(_, node)| node)
            .find(|node| node.label() == Some("OK"))
            .expect("Recover button");
        let bounds = node.bounds().expect("Recover bounds");
        let at = pos2(
            ((bounds.x0 + bounds.x1) / 2.) as f32,
            ((bounds.y0 + bounds.y1) / 2.) as f32,
        );
        let _ = draw(&mut app, &ctx, size, vec![Event::PointerMoved(at)]);
        let _ = draw(
            &mut app,
            &ctx,
            size,
            vec![pointer(at, true, Modifiers::NONE)],
        );
        let _ = draw(
            &mut app,
            &ctx,
            size,
            vec![pointer(at, false, Modifiers::NONE)],
        );
        assert!(app.recovery.is_none());
        assert_eq!(app.editor.scene, scene);
        assert!(app.path.is_none());
    }
}
