//! Action Center Selection and Local on two islands in a moved, turned item
//! (behaviour check 2026-11-18: AC-rotate-Selection, AC-scale-Local,
//! AC-rotate-Local). Original fixture: a two-quad strip tilted 45° and a
//! flat quad, in an item at (2, 0, 1) turned 30° about Y.
use glam::{DMat3, DMat4, DQuat, DVec3, EulerRot};
use operandi_core::*;

fn quad(v: [u32; 4]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// Points 0-5: the strip (y = z, normal along (0, 1, -1)); 6-9: the flat
/// quad (normal +Y). Their average height is 0.3, their box's middle 0.5.
fn fixture(center: CenterMode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let item = e.scene.active_mut().unwrap();
    item.mesh = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(0., 1., 1.),
            DVec3::new(1., 1., 1.),
            DVec3::new(2., 1., 1.),
            DVec3::new(4., 0., 0.),
            DVec3::new(5., 0., 0.),
            DVec3::new(5., 0., 1.),
            DVec3::new(4., 0., 1.),
        ],
        faces: vec![quad([0, 3, 4, 1]), quad([1, 4, 5, 2]), quad([6, 9, 8, 7])].into(),
        ..Default::default()
    }
    .into();
    item.transform.translation = DVec3::new(2., 0., 1.);
    item.transform.rotation = DVec3::new(0., 30f64.to_radians(), 0.);
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0, 1, 2];
    e.dispatch(Command::ActionContext(ActionContext {
        center,
        // Local by name is Local's centre with Local's axes.
        axis: if center == CenterMode::Local {
            AxisOrientation::Local
        } else {
            AxisOrientation::World
        },
    }))
    .unwrap();
    e
}

fn world(e: &Editor) -> DMat4 {
    let item = e.scene.active().unwrap();
    e.scene.world_matrix(item.id).unwrap()
}

fn close(a: DVec3, b: DVec3, what: &str) {
    assert!((a - b).length() < 1e-9, "{what}: {a:?} != {b:?}");
}

/// Runs `tool` with `delta` and returns the world points after.
fn run(e: &mut Editor, tool: Tool, delta: DVec3) -> Vec<DVec3> {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = delta;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let w = world(e);
    let item = e.scene.active().unwrap();
    item.mesh
        .vertices
        .iter()
        .map(|&v| w.transform_point3(v))
        .collect()
}

#[test]
fn selection_centres_on_the_item_space_box() {
    let mut e = fixture(CenterMode::Selection);
    let w = world(&e);
    let want = w.transform_point3(DVec3::new(2.5, 0.5, 0.5));
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    close(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        want,
        "frame centre",
    );
    e.dispatch(Command::Cancel).unwrap();
    // Scale halves every point's distance to that centre.
    let item = e.scene.active().unwrap();
    let before: Vec<DVec3> = item
        .mesh
        .vertices
        .iter()
        .map(|&v| w.transform_point3(v))
        .collect();
    let after = run(&mut e, Tool::Scale, DVec3::splat(0.5));
    for (&p, &q) in before.iter().zip(&after) {
        close(q, want + (p - want) * 0.5, "scaled point");
    }
    // Selection Center Auto Axis shares the centre.
    let mut e = fixture(CenterMode::SelectionAutoAxis);
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    close(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        want,
        "auto axis centre",
    );
}

/// The frame a Local piece should use (A-LOCAL): its item-space box
/// middle, Z along its world normal and X along the item's X (both fixture
/// islands' principal direction nearest item X, or a tie), Y = Z × X.
fn piece_frame(w: DMat4, lo: DVec3, hi: DVec3, normal: DVec3) -> (DVec3, DMat3) {
    let z = (DMat3::from_mat4(w) * normal).normalize();
    let y = z.cross(DMat3::from_mat4(w) * DVec3::X).normalize();
    (
        w.transform_point3((lo + hi) / 2.),
        DMat3::from_cols(y.cross(z), y, z),
    )
}

#[test]
fn local_turns_and_scales_each_island_in_its_own_frame() {
    for (tool, delta) in [
        (Tool::Rotate, DVec3::new(0., 40., 0.)),
        (Tool::Rotate, DVec3::new(25., 0., 10.)),
        (Tool::Scale, DVec3::new(2., 1., 0.5)),
    ] {
        let mut e = fixture(CenterMode::Local);
        let w = world(&e);
        let before: Vec<DVec3> = e
            .scene
            .active()
            .unwrap()
            .mesh
            .vertices
            .iter()
            .map(|&v| w.transform_point3(v))
            .collect();
        let strip = piece_frame(
            w,
            DVec3::ZERO,
            DVec3::new(2., 1., 1.),
            DVec3::new(0., 1., -1.),
        );
        let flat = piece_frame(w, DVec3::new(4., 0., 0.), DVec3::new(5., 0., 1.), DVec3::Y);
        let after = run(&mut e, tool, delta);
        for (i, (&p, &q)) in before.iter().zip(&after).enumerate() {
            let (c, b) = if i < 6 { strip } else { flat };
            let local = b.transpose() * (p - c);
            let moved = match tool {
                Tool::Rotate => {
                    let r = delta.map(f64::to_radians);
                    DQuat::from_euler(EulerRot::XYZ, r.x, r.y, r.z) * local
                }
                _ => local * delta,
            };
            close(q, c + b * moved, &format!("{tool:?} {delta:?} point {i}"));
        }
    }
}

/// With Symmetry X on, a mirrored pair of islands turns as mirror images:
/// each point ends where its partner's result, mirrored, ends.
#[test]
fn local_with_symmetry_mirrors_each_islands_frame() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let strip = [
        DVec3::new(1., 0., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(3., 0., 0.),
        DVec3::new(1., 1., 1.),
        DVec3::new(2., 1., 1.),
        DVec3::new(3., 1., 1.),
    ];
    let mirror = |p: DVec3| DVec3::new(-p.x, p.y, p.z);
    let mut vertices: Vec<DVec3> = strip.to_vec();
    vertices.extend(strip.iter().map(|&p| mirror(p)));
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices,
        faces: vec![
            quad([0, 3, 4, 1]),
            quad([1, 4, 5, 2]),
            quad([6, 7, 10, 9]),
            quad([7, 8, 11, 10]),
        ]
        .into(),
        ..Default::default()
    }
    .into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0, 1, 2, 3];
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Local,
        axis: AxisOrientation::Local,
    }))
    .unwrap();
    e.symmetry.axis = Some(0);
    let after = run(&mut e, Tool::Rotate, DVec3::new(20., 35., 0.));
    for i in 0..6 {
        close(after[i + 6], mirror(after[i]), &format!("point {i}"));
    }
}
