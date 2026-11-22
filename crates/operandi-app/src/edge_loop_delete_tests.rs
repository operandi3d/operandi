//! Delete / Backspace over a viewport delete the selected elements with
//! the points left unused, in every component mode (Jason, 2026-11-21);
//! Shift+Backspace removes edges and polygons but keeps their points
//! (Edge Remove, Remove). Item mode deletes items, asking first before the
//! scene's only mesh item goes.
use super::tests::draw;
use super::*;
use operandi_core::{EdgeCommand, Mesh, RadialParameters, edge_command_geometry, globe_geometry};

const SIZE: Vec2 = vec2(1440., 900.);
const SIDES: u32 = 12;

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx
}

fn globe() -> Mesh {
    globe_geometry(
        DVec3::ZERO,
        RadialParameters {
            radius: DVec3::ONE,
            sides: SIDES,
            segments: 8,
        },
    )
    .unwrap()
}

/// The equator loop, stored low point first.
fn equator() -> Vec<(u32, u32)> {
    let first = 1 + 3 * SIDES;
    (0..SIDES)
        .map(|i| {
            let (a, b) = (first + i, first + (i + 1) % SIDES);
            (a.min(b), a.max(b))
        })
        .collect()
}

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.editor.scene.new_mesh().unwrap();
    a.editor.scene.active_mut().unwrap().mesh = globe().into();
    a.editor.synchronize_identities().unwrap();
    a.editor.scene.selection.mode = Mode::Edge;
    a.editor.scene.selection.edges = equator();
    a.saved = a.editor.scene.clone();
    a.camera.view = View::Perspective;
    a.frame(false);
    a
}

/// Press and release `key` with the pointer at `at`.
fn press(a: &mut Operandi, ctx: &Context, key: Key, at: Pos2) {
    chord_at(a, ctx, key, Modifiers::NONE, at);
}

/// Press and release `key` with `modifiers` held and the pointer at `at`.
fn chord_at(a: &mut Operandi, ctx: &Context, key: Key, modifiers: Modifiers, at: Pos2) {
    draw(a, ctx, SIZE, vec![]);
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    for pressed in [true, false] {
        draw(
            a,
            ctx,
            SIZE,
            vec![Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            }],
        );
    }
}

/// Both Delete keys (the Mac's delete key is Backspace) take the loop's
/// points; Shift+Backspace (Edge Remove) keeps them.
#[test]
fn delete_and_backspace_take_the_loops_points_and_shift_backspace_keeps_them() {
    let cleaned = operandi_core::edge_delete_geometry(&globe(), &equator()).unwrap();
    let (kept, _) = edge_command_geometry(&globe(), &equator(), EdgeCommand::Remove).unwrap();
    assert!(cleaned.vertices.len() < kept.vertices.len());
    for (key, modifiers, expected, label) in [
        (Key::Delete, Modifiers::NONE, &cleaned, "Delete"),
        (Key::Backspace, Modifiers::NONE, &cleaned, "Delete"),
        (Key::Backspace, Modifiers::SHIFT, &kept, "Remove"),
    ] {
        let ctx = context();
        let mut a = app();
        draw(&mut a, &ctx, SIZE, vec![]);
        let at = a.viewport_rect.center();
        chord_at(&mut a, &ctx, key, modifiers, at);
        let mesh = &a.editor.scene.active().unwrap().mesh;
        assert_eq!(**mesh, *expected, "{key:?} {modifiers:?}");
        assert_eq!(a.editor.scene.items.len(), 1, "{key:?} kept the item");
        assert_eq!(a.editor.undo.last().unwrap().label, label);
    }
}

/// Bug 2026-11-21 (Model the Mug): with the Shader Tree tab in front,
/// Delete over the view did nothing in any mode.
#[test]
fn delete_works_with_the_shader_tree_showing() {
    let cleaned = operandi_core::edge_delete_geometry(&globe(), &equator()).unwrap();
    for key in [Key::Backspace, Key::Delete] {
        let ctx = context();
        let mut a = app();
        a.upper_frame.add(panel_registry::PanelKind::ShaderTree);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(a.shading_tab());
        let at = a.viewport_rect.center();
        press(&mut a, &ctx, key, at);
        assert_eq!(*a.editor.scene.active().unwrap().mesh, cleaned, "{key:?}");
    }
}

#[test]
fn delete_away_from_the_viewport_changes_nothing() {
    let ctx = context();
    let mut a = app();
    draw(&mut a, &ctx, SIZE, vec![]);
    let before = a.editor.scene.clone();
    let off = pos2(
        a.viewport_rect.right() + 40.,
        a.viewport_rect.bottom() - 20.,
    );
    press(&mut a, &ctx, Key::Backspace, off);
    assert_eq!(a.editor.scene, before);
}

/// Points (with their polygons) and polygons go through `delete` with
/// either key; Shift+Backspace removes polygons and keeps their points.
#[test]
fn delete_removes_the_selection_of_every_mode() {
    let ctx = context();
    let run = |mode: Mode, key: Key, modifiers: Modifiers| {
        let mut a = app();
        let s = &mut a.editor.scene.selection;
        s.mode = mode;
        s.edges.clear();
        s.vertices = vec![37, 40];
        s.faces = vec![3, 4];
        draw(&mut a, &ctx, SIZE, vec![]);
        let at = a.viewport_rect.center();
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
        for pressed in [true, false] {
            draw(
                &mut a,
                &ctx,
                SIZE,
                vec![Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                }],
            );
        }
        a.editor.scene
    };
    // Points and polygons run `delete`: the same scene as the command.
    let typed = |mode: Mode| {
        let mut e = app().editor;
        let s = &mut e.scene.selection;
        s.mode = mode;
        s.edges.clear();
        s.vertices = vec![37, 40];
        s.faces = vec![3, 4];
        operandi_core::command_exec::run(&mut e, "delete", &Default::default()).unwrap();
        e.scene
    };
    // Every component mode: both keys run `delete` (orphaned points go);
    // Shift+Backspace on polygons is Remove (they stay).
    // (Edges: delete_and_backspace_take_the_loops_points_and_….)
    for mode in [Mode::Vertex, Mode::Polygon] {
        for key in [Key::Backspace, Key::Delete] {
            let deleted = run(mode, key, Modifiers::NONE);
            assert_eq!(deleted, typed(mode), "{mode:?} {key:?}");
            assert_eq!(deleted.items.len(), 1);
        }
    }
    let removed = run(Mode::Polygon, Key::Backspace, Modifiers::SHIFT);
    let faces = |s: &Scene| s.active().unwrap().mesh.faces.len();
    assert_eq!(faces(&removed), globe().faces.len() - 2);
    let count = |s: &Scene| s.active().unwrap().mesh.vertices.len();
    // (Faces 3 and 4 share every point with neighbours, so Remove keeps
    // every point; rem_001 covers orphaned points.)
    assert_eq!(count(&removed), globe().vertices.len());
}

/// Item mode: Delete deletes the selected items. The scene's only mesh
/// item asks first (Jason, 2026-11-21): Yes (Return) deletes it, No
/// (Escape) keeps it, Cancel (Shift+Escape) changes nothing.
#[test]
fn item_mode_delete_asks_before_the_last_mesh() {
    let ctx = context();
    let item_app = || {
        let mut a = app();
        a.editor.scene.selection.mode = Mode::Item;
        a
    };
    // A second mesh: no question, the selected one goes.
    let mut a = item_app();
    a.editor.scene.new_mesh().unwrap();
    a.editor.synchronize_identities().unwrap();
    draw(&mut a, &ctx, SIZE, vec![]);
    let at = a.viewport_rect.center();
    press(&mut a, &ctx, Key::Backspace, at);
    assert_eq!(a.editor.scene.items.len(), 1);
    assert!(a.scenes.confirm_last_mesh.is_none());
    // The only mesh: the question, then each answer.
    for (key, modifiers, left) in [
        (Key::Enter, Modifiers::NONE, 0),
        (Key::Escape, Modifiers::NONE, 1),
        (Key::Escape, Modifiers::SHIFT, 1),
    ] {
        let mut a = item_app();
        draw(&mut a, &ctx, SIZE, vec![]);
        let at = a.viewport_rect.center();
        press(&mut a, &ctx, Key::Delete, at);
        assert_eq!(a.editor.scene.items.len(), 1, "asks before deleting");
        assert!(a.scenes.confirm_last_mesh.is_some());
        assert_eq!(a.open_dialog(), Some("last mesh item question"));
        chord_at(&mut a, &ctx, key, modifiers, at);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(a.scenes.confirm_last_mesh.is_none(), "{key:?} closes it");
        assert_eq!(a.editor.scene.items.len(), left, "{key:?} {modifiers:?}");
        // Always Ask stays the preference unless the dropdown changed.
        assert_eq!(
            a.defaults_preferences.last_mesh_delete,
            crate::defaults_preferences::InTheFuture::Ask
        );
    }
}

/// Deleting every mesh of a scene with several asks too, with its own
/// question (Jason, 2026-11-21): Yes deletes them all, No keeps them all.
#[test]
fn deleting_every_mesh_asks_too() {
    let ctx = context();
    let meshes = |a: &Operandi| a.editor.scene.items.iter().filter(|i| i.is_mesh()).count();
    for (key, left) in [(Key::Enter, 0), (Key::Escape, 2)] {
        let mut a = app();
        let first = a.editor.scene.selection.item.unwrap();
        a.editor.scene.new_mesh().unwrap();
        a.editor.synchronize_identities().unwrap();
        a.editor.scene.selection.mode = Mode::Item;
        a.editor.scene.toggle_item(first);
        assert_eq!(a.editor.scene.selected_items().len(), 2);
        draw(&mut a, &ctx, SIZE, vec![]);
        let at = a.viewport_rect.center();
        press(&mut a, &ctx, Key::Delete, at);
        assert_eq!(meshes(&a), 2, "asks before deleting");
        let (dialog, _) = a.scenes.confirm_last_mesh.as_ref().unwrap();
        assert_eq!(
            dialog.rows[0].label,
            operandi_core::item_delete::ALL_MESHES_QUESTION
        );
        chord_at(&mut a, &ctx, key, Modifiers::NONE, at);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(a.scenes.confirm_last_mesh.is_none(), "{key:?} closes it");
        assert_eq!(meshes(&a), left, "{key:?}");
    }
}

/// The In the Future choice answers the question from then on.
#[test]
fn in_the_future_answers_without_asking() {
    use crate::defaults_preferences::InTheFuture;
    for (future, left) in [(InTheFuture::AlwaysYes, 0), (InTheFuture::AlwaysNo, 1)] {
        let mut a = app();
        a.editor.scene.selection.mode = Mode::Item;
        a.defaults_preferences.last_mesh_delete = future;
        a.request_delete();
        assert!(
            a.scenes.confirm_last_mesh.is_none(),
            "{future:?} doesn't ask"
        );
        assert_eq!(a.editor.scene.items.len(), left, "{future:?}");
    }
    // Picking Always Yes in the dialog and answering Yes keeps it.
    let ctx = context();
    let mut a = app();
    a.editor.scene.selection.mode = Mode::Item;
    a.request_delete();
    let (dialog, _) = a.scenes.confirm_last_mesh.as_mut().unwrap();
    dialog.values.set(
        "future",
        crate::form::Value::Choice(InTheFuture::AlwaysYes.index()),
    );
    draw(&mut a, &ctx, SIZE, vec![]);
    let at = a.viewport_rect.center();
    chord_at(&mut a, &ctx, Key::Enter, Modifiers::NONE, at);
    assert!(a.editor.scene.items.is_empty());
    assert_eq!(
        a.defaults_preferences.last_mesh_delete,
        InTheFuture::AlwaysYes
    );
}

/// No keeps the last mesh but deletes the other selected items.
#[test]
fn no_keeps_the_mesh_and_deletes_the_rest() {
    let mut a = app();
    let mesh = a.editor.scene.selection.item.unwrap();
    let light = a
        .editor
        .scene
        .add_light(Default::default(), Default::default())
        .unwrap();
    a.editor.scene.selection.mode = Mode::Item;
    a.editor.scene.select_item(Some(mesh));
    a.editor.scene.toggle_item(light);
    a.defaults_preferences.last_mesh_delete = crate::defaults_preferences::InTheFuture::AlwaysNo;
    a.request_delete();
    let left: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    assert_eq!(left, vec![mesh]);
}
