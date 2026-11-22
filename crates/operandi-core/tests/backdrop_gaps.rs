//! Backdrop gaps (BD-60..62): Auto Fit to the active meshes, a picture
//! dropped into a view, and the Pen's Backdrop Item UVs.
use glam::{DVec2, DVec3};
use operandi_core::backdrop::*;
use operandi_core::*;

/// A mesh item holding a box from `lo` to `hi`; returns its id.
fn add_box(e: &mut Editor, lo: DVec3, hi: DVec3) -> u64 {
    e.dispatch(Command::NewMesh).unwrap();
    let id = e.scene.selection.item.unwrap();
    e.scene.items.last_mut().unwrap().mesh =
        Mesh::box_mesh((lo + hi) / 2., hi - lo, 0).unwrap().into();
    id
}

/// A Front Backdrop with Use Uniform Size on (Auto Fit needs it).
fn fit_backdrop(e: &mut Editor) -> u64 {
    e.dispatch(Command::NewBackdrop).unwrap();
    let id = e.scene.selection.item.unwrap();
    e.dispatch(Command::EditBackdrop {
        item: id,
        edit: BackdropEdit::UniformSize(true),
    })
    .unwrap();
    id
}

fn transform(e: &Editor, id: u64) -> scene::Transform {
    e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .transform
        .clone()
}

// ---- BD-61 Auto Fit uses the active mesh items ----

#[test]
fn bd_61_auto_fit_uses_the_combined_box_of_the_selected_meshes_hidden_included() {
    let mut e = Editor::default();
    let backdrop = fit_backdrop(&mut e);
    let a = add_box(&mut e, DVec3::new(0., 0., 0.), DVec3::new(1., 1., 1.));
    let b = add_box(&mut e, DVec3::new(2., 1., -1.), DVec3::new(3., 4., 0.));
    // Not active: a visible mesh outside the selection is ignored.
    let _far = add_box(&mut e, DVec3::splat(50.), DVec3::splat(60.));
    // Hide b; it stays selected, so it still counts (BD-Q04).
    e.dispatch(Command::Visibility(b)).unwrap();
    assert!(!e.scene.items.iter().find(|i| i.id == b).unwrap().visible);
    e.dispatch(Command::SelectItems(vec![backdrop, a, b]))
        .unwrap();
    assert_eq!(e.scene.auto_fit_meshes(), vec![a, b]);
    e.dispatch(Command::BackdropAutoFit(backdrop)).unwrap();
    let t = transform(&e, backdrop);
    // Box X 0..3, Y 0..4, Z -1..1: centre (1.5, 2), the Front face Z 1.
    assert_eq!(t.translation, DVec3::new(1.5, 2., 1.));
    assert_eq!(t.scale, DVec3::new(4., 4., 1.));
}

#[test]
fn bd_61_a_backdrop_selected_alone_fits_the_mesh_that_stays_active() {
    let mut e = Editor::default();
    let backdrop = fit_backdrop(&mut e);
    let _other = add_box(&mut e, DVec3::splat(20.), DVec3::splat(21.));
    let mesh = add_box(&mut e, DVec3::ZERO, DVec3::new(2., 1., 1.));
    e.dispatch(Command::SelectItem(Some(backdrop))).unwrap();
    assert_eq!(e.scene.auto_fit_meshes(), vec![mesh]);
    e.dispatch(Command::BackdropAutoFit(backdrop)).unwrap();
    let t = transform(&e, backdrop);
    assert_eq!(t.translation, DVec3::new(1., 0.5, 1.));
    assert_eq!(t.scale.y, 1.);
}

#[test]
fn bd_61_no_active_mesh_with_size_refuses_and_changes_nothing() {
    // No mesh at all.
    let mut e = Editor::default();
    let backdrop = fit_backdrop(&mut e);
    let before = e.scene.clone();
    assert_eq!(
        e.dispatch(Command::BackdropAutoFit(backdrop)).unwrap_err(),
        AUTO_FIT_NO_MESH
    );
    assert_eq!(e.scene, before);
    // A selected mesh with no geometry, beside a visible one that isn't
    // active: refused, where the old stand-in fitted to the visible one.
    let _shown = add_box(&mut e, DVec3::ZERO, DVec3::ONE);
    e.dispatch(Command::NewMesh).unwrap();
    let empty = e.scene.selection.item.unwrap();
    e.dispatch(Command::SelectItems(vec![backdrop, empty]))
        .unwrap();
    let before = e.scene.clone();
    assert_eq!(
        e.dispatch(Command::BackdropAutoFit(backdrop)).unwrap_err(),
        AUTO_FIT_NO_MESH
    );
    assert_eq!(e.scene, before);
    // A flat active mesh with no height in the Backdrop's plane.
    let mut e = Editor::default();
    let backdrop = fit_backdrop(&mut e);
    let flat = add_box(&mut e, DVec3::ZERO, DVec3::ONE);
    // One square lying flat at Y 0.
    let square = [(0., 0.), (1., 0.), (1., -1.), (0., -1.)].map(|(x, z)| DVec3::new(x, 0., z));
    e.scene.items.last_mut().unwrap().mesh = pen_geometry(&square, &PenOptions::default())
        .unwrap()
        .into();
    e.dispatch(Command::SelectItems(vec![backdrop, flat]))
        .unwrap();
    let before = e.scene.clone();
    assert!(e.dispatch(Command::BackdropAutoFit(backdrop)).is_err());
    assert_eq!(e.scene, before);
}

// ---- BD-60 a picture dropped into a 3D view ----

#[test]
fn bd_60_add_backdrop_with_an_image_is_one_undo_step() {
    let mut e = Editor::default();
    let mesh = add_box(&mut e, DVec3::ZERO, DVec3::ONE);
    let empty = e.scene.clone();
    e.dispatch(Command::AddBackdrop {
        name: None,
        image: Some(("/refs/side view.png".into(), 300, 200)),
        projection: Projection::Left,
    })
    .unwrap();
    let item = e.scene.items.last().unwrap().clone();
    // BD-01: named, at the origin, the only selected item.
    assert_eq!(item.name, "Backdrop Item");
    assert_eq!(item.transform, Default::default());
    assert_eq!(item.parent, None);
    assert_eq!(e.scene.selection.item, Some(item.id));
    assert!(e.scene.selection.also.is_empty());
    // BD-02 defaults with the view's projection; BD-03 the image entry.
    let b = item.backdrop.unwrap();
    assert_eq!(
        b,
        Backdrop {
            image: b.image,
            projection: Projection::Left,
            ..Backdrop::new_default()
        }
    );
    let image = e.scene.image(b.image.unwrap()).unwrap();
    assert_eq!(
        (image.name.as_str(), image.width, image.height),
        ("side view", 300, 200)
    );
    // One step: Undo takes away both the Backdrop and the image entry.
    let after = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, empty);
    assert_eq!(e.scene.selection.item, Some(mesh));
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
    // The same file again shares its entry (BD-03), and the name counts on.
    e.dispatch(Command::AddBackdrop {
        name: None,
        image: Some(("/refs/side view.png".into(), 300, 200)),
        projection: Projection::Top,
    })
    .unwrap();
    assert_eq!(e.scene.images.len(), 1);
    assert_eq!(e.scene.items.last().unwrap().name, "Backdrop Item (2)");
    // A name, and no picture: a named Backdrop with no image.
    e.dispatch(Command::AddBackdrop {
        name: Some("Side plan".into()),
        image: None,
        projection: Projection::Right,
    })
    .unwrap();
    let item = e.scene.items.last().unwrap();
    assert_eq!(item.name, "Side plan");
    let b = item.backdrop.as_ref().unwrap();
    assert_eq!((b.image, b.projection), (None, Projection::Right));
    let before = e.scene.clone();
    assert!(
        e.dispatch(Command::AddBackdrop {
            name: Some(" ".into()),
            image: None,
            projection: Projection::Front,
        })
        .is_err()
    );
    assert_eq!(e.scene, before);
}

#[test]
fn bd_60_load_image_adds_only_an_image_entry() {
    let mut e = Editor::default();
    let before = e.scene.clone();
    e.dispatch(Command::LoadImage {
        path: "/refs/plan.jpg".into(),
        width: 64,
        height: 32,
    })
    .unwrap();
    assert!(e.scene.items.is_empty());
    assert_eq!(e.scene.images.len(), 1);
    assert_eq!(e.scene.images[0].name, "plan");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    // A picture with no pixels is refused, nothing changes.
    assert!(
        e.dispatch(Command::AddBackdrop {
            name: None,
            image: Some(("/refs/empty.png".into(), 0, 10)),
            projection: Projection::Front,
        })
        .is_err()
    );
    assert_eq!(e.scene, before);
}

// ---- BD-62 Pen Backdrop Item UVs ----

/// A Backdrop showing a 200 × 100 px picture at 10 mm per pixel: 2 m × 1 m
/// centred on `at`, facing `projection`.
fn picture(scene: &mut Scene, projection: Projection, at: DVec3, flip: bool) -> u64 {
    let selection = scene.selection.clone();
    let image = scene
        .image_for_path(&format!("/refs/{projection:?}.png"), 200, 100)
        .unwrap();
    let id = scene
        .new_backdrop(Backdrop {
            image: Some(image),
            projection,
            flip,
            ..Backdrop::default()
        })
        .unwrap();
    scene.items.last_mut().unwrap().transform.translation = at;
    scene.selection = selection;
    id
}

fn close(a: DVec2, b: DVec2) -> bool {
    (a - b).abs().max_element() < 1e-9
}

#[test]
fn bd_62_picture_frame_uvs_at_corners_outside_and_flipped() {
    let mut scene = Scene::default();
    let id = picture(&mut scene, Projection::Front, DVec3::ZERO, false);
    let f = picture_frame(&scene, id).unwrap();
    // Left -1, right 1, bottom -0.5, top 0.5; depth doesn't matter.
    assert!(close(f.uv(DVec3::new(-1., -0.5, 0.)), DVec2::new(0., 0.)));
    assert!(close(f.uv(DVec3::new(1., -0.5, 3.)), DVec2::new(1., 0.)));
    assert!(close(f.uv(DVec3::new(1., 0.5, 0.)), DVec2::new(1., 1.)));
    assert!(close(f.uv(DVec3::new(-1., 0.5, 0.)), DVec2::new(0., 1.)));
    assert!(close(f.uv(DVec3::ZERO), DVec2::splat(0.5)));
    // Outside: below 0 or above 1, never clamped.
    assert!(close(f.uv(DVec3::new(3., -1.5, 0.)), DVec2::new(2., -1.)));
    // Flip mirrors U (BD-21).
    let flipped = picture(&mut scene, Projection::Front, DVec3::ZERO, true);
    let f = picture_frame(&scene, flipped).unwrap();
    assert!(close(f.uv(DVec3::new(-1., -0.5, 0.)), DVec2::new(1., 0.)));
    assert!(close(f.uv(DVec3::new(-0.5, 0.5, 0.)), DVec2::new(0.75, 1.)));
    // Top view: picture up is -Z.
    let top = picture(&mut scene, Projection::Top, DVec3::ZERO, false);
    let f = picture_frame(&scene, top).unwrap();
    assert!(close(f.uv(DVec3::new(-1., 7., 0.5)), DVec2::new(0., 0.)));
    assert!(close(f.uv(DVec3::new(1., 0., -0.5)), DVec2::new(1., 1.)));
}

#[test]
fn bd_62_the_topmost_backdrop_of_the_view_is_used() {
    let mut scene = Scene::default();
    assert_eq!(
        topmost_backdrop_in(&scene, ViewKind::Axis(Projection::Front)),
        None
    );
    let first = picture(&mut scene, Projection::Front, DVec3::ZERO, false);
    let _second = picture(&mut scene, Projection::Front, DVec3::X, false);
    let top = picture(&mut scene, Projection::Top, DVec3::ZERO, false);
    let front = ViewKind::Axis(Projection::Front);
    // BD-11b: the first created is drawn on top.
    assert_eq!(topmost_backdrop_in(&scene, front), Some(first));
    assert_eq!(
        topmost_backdrop_in(&scene, ViewKind::Axis(Projection::Top)),
        Some(top)
    );
    assert_eq!(
        topmost_backdrop_in(&scene, ViewKind::Axis(Projection::Back)),
        None
    );
    // The free views keep world X/Y (BD-Q22).
    assert_eq!(topmost_backdrop_in(&scene, ViewKind::Free), None);
    // A hidden Backdrop isn't drawn, so the next one is used.
    scene
        .items
        .iter_mut()
        .find(|i| i.id == first)
        .unwrap()
        .visible = false;
    assert_eq!(topmost_backdrop_in(&scene, front), Some(_second));
    // A Backdrop with no picture has no picture point to map to.
    scene
        .items
        .iter_mut()
        .find(|i| i.id == first)
        .unwrap()
        .visible = true;
    scene
        .items
        .iter_mut()
        .find(|i| i.id == first)
        .unwrap()
        .backdrop
        .as_mut()
        .unwrap()
        .image = None;
    assert_eq!(topmost_backdrop_in(&scene, front), Some(_second));
}

fn pen_editor() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.selection.mode = Mode::Polygon;
    e.synchronize_identities().unwrap();
    e
}

/// Draw a square from (-1,-0.5) to (3,0.5) in the XY plane with Make UVs
/// on, drawn in `view`; the Texture UVs of the new polygon.
fn pen_uvs(e: &mut Editor, view: Option<ViewKind>) -> Vec<DVec2> {
    e.dispatch(Command::PenStart).unwrap();
    e.dispatch(Command::PenOptions(PenOptions {
        make_uvs: true,
        uv_view: view,
        plane_normal: DVec3::Z,
        ..Default::default()
    }))
    .unwrap();
    for p in [
        DVec3::new(-1., -0.5, 0.),
        DVec3::new(3., -0.5, 0.),
        DVec3::new(3., 0.5, 0.),
        DVec3::new(-1., 0.5, 0.),
    ] {
        e.dispatch(Command::PenCapture(p)).unwrap();
    }
    e.dispatch(Command::Drop).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    mesh.uv_maps["Texture"]
        .last()
        .unwrap()
        .iter()
        .map(|uv| uv.unwrap())
        .collect()
}

#[test]
fn bd_62_pen_points_get_the_picture_uvs_under_them() {
    let mut e = pen_editor();
    picture(&mut e.scene, Projection::Front, DVec3::ZERO, false);
    let uvs = pen_uvs(&mut e, Some(ViewKind::Axis(Projection::Front)));
    let want = [(0., 0.), (2., 0.), (2., 1.), (0., 1.)].map(|(u, v)| DVec2::new(u, v));
    assert!(uvs.iter().zip(want).all(|(&a, b)| close(a, b)), "{uvs:?}");
    // The mesh item's own transform doesn't change where the picture is.
    let mut e = pen_editor();
    e.scene.active_mut().unwrap().transform.translation = DVec3::new(5., 5., 5.);
    picture(&mut e.scene, Projection::Front, DVec3::ZERO, true);
    let uvs = pen_uvs(&mut e, Some(ViewKind::Axis(Projection::Front)));
    let want = [(1., 0.), (-1., 0.), (-1., 1.), (1., 1.)].map(|(u, v)| DVec2::new(u, v));
    assert!(
        uvs.iter().zip(want).all(|(&a, b)| close(a, b)),
        "flipped {uvs:?}"
    );
}

#[test]
fn bd_62_without_a_backdrop_in_the_view_uvs_stay_world_x_and_y() {
    let world = [(-1., -0.5), (3., -0.5), (3., 0.5), (-1., 0.5)].map(|(u, v)| DVec2::new(u, v));
    for (backdrop, view) in [
        // No Backdrop at all.
        (None, Some(ViewKind::Axis(Projection::Front))),
        // One drawn in another view.
        (
            Some(Projection::Top),
            Some(ViewKind::Axis(Projection::Front)),
        ),
        // Drawn in a free view (BD-Q22) or no view given (numeric route).
        (Some(Projection::Front), Some(ViewKind::Free)),
        (Some(Projection::Front), None),
    ] {
        let mut e = pen_editor();
        if let Some(p) = backdrop {
            picture(&mut e.scene, p, DVec3::new(9., 9., 9.), false);
        }
        let uvs = pen_uvs(&mut e, view);
        assert!(
            uvs.iter().zip(world).all(|(&a, b)| close(a, b)),
            "{backdrop:?} {view:?}: {uvs:?}"
        );
    }
}
