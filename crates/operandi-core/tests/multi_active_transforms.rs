//! Multiple active meshes, stage 2 (multi_transform.rs): Move, Rotate,
//! Scale and Transform act on every active mesh's selected components, and
//! in Item mode on every selected item, about one shared centre.
use glam::DVec3;
use operandi_core::*;

/// Adds a 2 m cube as a new mesh centred at `x`; returns its id.
fn add_cube(e: &mut Editor, x: f64) -> u64 {
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        position: DVec3::new(x, 0., 0.),
        size: DVec3::splat(2.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.scene.active().unwrap().id
}
/// Cubes at x = 0 (`a`, primary) and x = 6 (`b`), both selected, Polygon
/// mode, nothing selected.
fn two() -> (Editor, u64, u64) {
    let mut e = Editor::default();
    let a = add_cube(&mut e, 0.);
    let b = add_cube(&mut e, 6.);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::SelectItems(vec![a, b])).unwrap();
    e.dispatch(Command::ClearSelection).unwrap();
    (e, a, b)
}
fn pick(e: &mut Editor, item: u64, hit: Component) {
    e.dispatch(Command::Select {
        item,
        hit,
        replace: false,
        remove: false,
    })
    .unwrap();
}
fn mesh(e: &Editor, id: u64) -> Mesh {
    (*e.scene.items.iter().find(|i| i.id == id).unwrap().mesh).clone()
}
fn item(e: &Editor, id: u64) -> Item {
    e.scene.items.iter().find(|i| i.id == id).unwrap().clone()
}
/// The polygon of `id` facing +Y (the top).
fn top(e: &Editor, id: u64) -> u32 {
    let m = mesh(e, id);
    (0..m.faces.len()).find(|&f| m.normal(f).y > 0.9).unwrap() as u32
}
fn drag(e: &mut Editor, tool: Tool, delta: DVec3) {
    e.dispatch(Command::Activate(tool)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta,
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
}
fn moved(before: &Mesh, after: &Mesh) -> usize {
    before
        .vertices
        .iter()
        .zip(&after.vertices)
        .filter(|(a, b)| a.distance(**b) > 1e-9)
        .count()
}

#[test]
fn move_moves_the_selection_on_every_mesh_in_one_step() {
    let (mut e, a, b) = two();
    let (ta, tb) = (top(&e, a), top(&e, b));
    pick(&mut e, a, Component::Polygon(ta));
    pick(&mut e, b, Component::Polygon(tb));
    let (a0, b0) = (mesh(&e, a), mesh(&e, b));
    let steps = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    // One handle at the centre of both tops: x = 3, y = 1.
    let frame = e.session.live.as_ref().unwrap().action_frame.unwrap();
    assert!(
        frame.center.distance(DVec3::new(3., 1., 0.)) < 1e-9,
        "{:?}",
        frame.center
    );
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::Y,
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(moved(&a0, &mesh(&e, a)), 4);
    assert_eq!(moved(&b0, &mesh(&e, b)), 4);
    assert_eq!(e.history.undo.len(), steps + 1, "one undo step");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e, a), a0);
    assert_eq!(mesh(&e, b), b0);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(moved(&b0, &mesh(&e, b)), 4);
    // The selections are unchanged.
    assert_eq!(e.scene.selection.faces_of(b), [tb]);
    e.scene.validate().unwrap();
}

#[test]
fn a_mesh_with_nothing_selected_stays_put() {
    let (mut e, a, b) = two();
    {
        let f = top(&e, b);
        pick(&mut e, b, Component::Polygon(f));
    }
    let (a0, b0) = (mesh(&e, a), mesh(&e, b));
    drag(&mut e, Tool::Move, DVec3::Y);
    assert_eq!(mesh(&e, a), a0, "nothing selected on the primary");
    assert_eq!(moved(&b0, &mesh(&e, b)), 4);
}

#[test]
fn nothing_selected_anywhere_moves_every_active_mesh() {
    let (mut e, a, b) = two();
    let (a0, b0) = (mesh(&e, a), mesh(&e, b));
    drag(&mut e, Tool::Move, DVec3::Y);
    assert_eq!(moved(&a0, &mesh(&e, a)), 8);
    assert_eq!(moved(&b0, &mesh(&e, b)), 8);
}

#[test]
fn rotate_and_scale_turn_about_the_shared_centre() {
    let (mut e, a, b) = two();
    {
        let f = top(&e, a);
        pick(&mut e, a, Component::Polygon(f));
    }
    {
        let f = top(&e, b);
        pick(&mut e, b, Component::Polygon(f));
    }
    // Scale ×2 about x = 3: a's top corners (x = ±1) go to -5 and -1.
    drag(&mut e, Tool::Scale, DVec3::splat(2.));
    let xs: Vec<f64> = mesh(&e, a)
        .vertices
        .iter()
        .filter(|v| v.y > 0.)
        .map(|v| v.x)
        .collect();
    assert!(
        xs.iter()
            .all(|x| (x + 5.).abs() < 1e-9 || (x + 1.).abs() < 1e-9),
        "{xs:?}"
    );
    // Rotate 180° about Y through the shared centre (x = 3, both whole
    // cubes with nothing selected) swaps the cubes' places.
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::ClearSelection).unwrap();
    drag(&mut e, Tool::Rotate, DVec3::new(0., 180., 0.));
    let centre = |id| mesh(&e, id).vertices.iter().sum::<DVec3>() / 8.;
    assert!(
        centre(a).distance(DVec3::new(6., 0., 0.)) < 1e-9,
        "{:?}",
        centre(a)
    );
    assert!(centre(b).distance(DVec3::ZERO) < 1e-9, "{:?}", centre(b));
}

#[test]
fn falloff_weights_are_read_in_world_space() {
    let (mut e, a, b) = two();
    e.dispatch(Command::SelectItems(vec![a, b])).unwrap();
    // A linear falloff from x = 6 (full) to x = 0 (none).
    e.spatial_falloff = Some(Falloff::Linear(LinearFalloff {
        start: DVec3::new(6., 0., 0.),
        end: DVec3::ZERO,
        ..Default::default()
    }));
    let (a0, b0) = (mesh(&e, a), mesh(&e, b));
    drag(&mut e, Tool::Move, DVec3::Y);
    let lift = |m0: &Mesh, m: &Mesh| {
        m.vertices
            .iter()
            .zip(&m0.vertices)
            .map(|(v, v0)| v.y - v0.y)
            .sum::<f64>()
            / 8.
    };
    let (la, lb) = (lift(&a0, &mesh(&e, a)), lift(&b0, &mesh(&e, b)));
    assert!(lb > la + 0.5, "the far cube moves more: {la} vs {lb}");
}

#[test]
fn moved_items_move_their_points_in_world_space() {
    let (mut e, a, b) = two();
    // b moved up 5 as an item; its top polygon still lands where dragged.
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::SelectItem(Some(b))).unwrap();
    drag(&mut e, Tool::Move, DVec3::new(0., 5., 0.));
    e.dispatch(Command::SelectItems(vec![a, b])).unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    {
        let f = top(&e, a);
        pick(&mut e, a, Component::Polygon(f));
    }
    {
        let f = top(&e, b);
        pick(&mut e, b, Component::Polygon(f));
    }
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    // Tops at world y = 1 and y = 6: centre y = 3.5.
    let frame = e.session.live.as_ref().unwrap().action_frame.unwrap();
    assert!((frame.center.y - 3.5).abs() < 1e-9, "{:?}", frame.center);
    e.dispatch(Command::Cancel).unwrap();
}

#[test]
fn item_mode_moves_rotates_and_scales_every_selected_item() {
    let (mut e, a, b) = two();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    // Item positions: both at the origin (the cubes' points are offset), so
    // give b its own place first.
    e.dispatch(Command::SelectItem(Some(b))).unwrap();
    drag(&mut e, Tool::Move, DVec3::new(4., 0., 0.));
    e.dispatch(Command::SelectItems(vec![a, b])).unwrap();
    let steps = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    // The handle at the shared centre of the items' positions.
    let frame = e.session.live.as_ref().unwrap().action_frame.unwrap();
    assert!(frame.center.distance(DVec3::new(2., 0., 0.)) < 1e-9);
    e.dispatch(Command::Cancel).unwrap();
    drag(&mut e, Tool::Move, DVec3::Y);
    assert_eq!(item(&e, a).transform.translation, DVec3::Y);
    assert_eq!(item(&e, b).transform.translation, DVec3::new(4., 1., 0.));
    assert_eq!(e.history.undo.len(), steps + 1, "one undo step");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(item(&e, a).transform.translation, DVec3::ZERO);
    assert_eq!(item(&e, b).transform.translation, DVec3::new(4., 0., 0.));
    // Scale ×2 about x = 2: a goes to -2, b to 6, both scaled.
    drag(&mut e, Tool::Scale, DVec3::splat(2.));
    assert!(
        item(&e, a)
            .transform
            .translation
            .distance(DVec3::new(-2., 0., 0.))
            < 1e-9
    );
    assert!(
        item(&e, b)
            .transform
            .translation
            .distance(DVec3::new(6., 0., 0.))
            < 1e-9
    );
    assert_eq!(item(&e, b).transform.scale, DVec3::splat(2.));
    e.dispatch(Command::Undo).unwrap();
    // Rotate 180° about Y through x = 2: they swap places, both turned.
    drag(&mut e, Tool::Rotate, DVec3::new(0., 180., 0.));
    assert!(
        item(&e, a)
            .transform
            .translation
            .distance(DVec3::new(4., 0., 0.))
            < 1e-9
    );
    assert!(item(&e, b).transform.translation.distance(DVec3::ZERO) < 1e-9);
    assert!((item(&e, a).transform.rotation.y - std::f64::consts::PI).abs() < 1e-9);
}

#[test]
fn item_mode_skips_locked_items_and_children_of_selected_parents() {
    let (mut e, a, b) = two();
    let c = add_cube(&mut e, 12.);
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    // c is b's child; c and b both selected: c follows b, moved once.
    e.scene.set_parent(c, Some(b), true).unwrap();
    e.dispatch(Command::SelectItems(vec![a, b, c])).unwrap();
    drag(&mut e, Tool::Move, DVec3::Y);
    assert_eq!(item(&e, b).transform.translation, DVec3::Y);
    assert_eq!(
        item(&e, c).transform.translation,
        DVec3::ZERO,
        "moved with b, not twice"
    );
    assert_eq!(e.scene.world_matrix(c).unwrap().w_axis.truncate(), DVec3::Y);
    // A locked item stays; the others still move.
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == a)
        .unwrap()
        .common
        .assembly
        .lock = item_common::Choice::Yes;
    drag(&mut e, Tool::Move, DVec3::Y);
    assert_eq!(item(&e, a).transform.translation, DVec3::Y);
    assert_eq!(item(&e, b).transform.translation, DVec3::new(0., 2., 0.));
    // All locked: refused.
    for id in [b, c] {
        e.scene
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .unwrap()
            .common
            .assembly
            .lock = item_common::Choice::Yes;
    }
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert!(
        e.dispatch(Command::Preview(Parameters {
            delta: DVec3::Y,
            ..e.session.live.as_ref().unwrap().params.clone()
        }))
        .is_err()
    );
}

#[test]
fn one_item_or_one_mesh_works_as_before() {
    let mut e = Editor::default();
    let a = add_cube(&mut e, 0.);
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    drag(&mut e, Tool::Rotate, DVec3::new(0., 90., 0.));
    assert_eq!(item(&e, a).transform.translation, DVec3::ZERO);
    assert!((item(&e, a).transform.rotation.y - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    assert_eq!(e.scene.component_transform_meshes(), None);
    assert_eq!(e.scene.item_transform_targets(), None);
}
