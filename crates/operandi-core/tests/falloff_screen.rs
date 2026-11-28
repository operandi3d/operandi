//! Falloff batch 4 (falloff spec 2026-11-15): Screen (FO-43), Lasso
//! (FO-52) and Airbrush (FO-44) weights from a view, and the stamps that
//! carry them to the tools (FO-Q08). Original test data.
use glam::{DMat4, DVec2, DVec3};
use operandi_core::falloff_screen::{self as fs, StampView};
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// An 11 × 11 grid of points 0.1 apart in the XY plane (a 10 × 10 quad
/// grid facing +Z). With `flip`, the polygons face −Z.
fn grid(flip: bool) -> Mesh {
    let mut vertices = Vec::new();
    for y in 0..=10 {
        for x in 0..=10 {
            vertices.push(DVec3::new(x as f64 / 10., y as f64 / 10., 0.));
        }
    }
    let mut faces = Vec::new();
    for y in 0..10u32 {
        for x in 0..10u32 {
            let a = y * 11 + x;
            let mut q = vec![a, a + 1, a + 12, a + 11];
            if flip {
                q.reverse();
            }
            faces.push(face(q));
        }
    }
    Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
}

/// A front view: 1 m = 100 px, x right, y up, looking down −Z.
fn front_view<R>(f: impl FnOnce(&StampView<'_>) -> R) -> R {
    let project = |p: DVec3| Some(DVec2::new(p.x * 100., -p.y * 100.));
    let facing = |_: DVec3, n: DVec3| n.z > 0.;
    f(&StampView {
        project: &project,
        facing: &facing,
    })
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// FO-43: 1 − d / Size around Center (linear, hands-on 2026-11-17), 0
/// beyond; measured defaults;
/// facing away weighs 0 unless Transparent.
#[test]
fn screen_weights_fall_off_around_the_centre() {
    let s = ScreenFalloff::default();
    assert_eq!((s.size, s.transparent), (20., false));
    let mesh = grid(false);
    let f = ScreenFalloff {
        center: DVec2::new(50., -50.),
        size: 30.,
        transparent: false,
    };
    let w = front_view(|v| fs::screen_weights(&mesh, DMat4::IDENTITY, v, f));
    assert_eq!(w[5 * 11 + 5], 1.);
    // 10 px away: 1 − 1/3.
    assert!(close(w[5 * 11 + 6], 2. / 3.));
    assert_eq!(w[5 * 11 + 9], 0.);
    let back = grid(true);
    let hidden = front_view(|v| fs::screen_weights(&back, DMat4::IDENTITY, v, f));
    assert!(hidden.iter().all(|&w| w == 0.));
    let through = front_view(|v| {
        fs::screen_weights(
            &back,
            DMat4::IDENTITY,
            v,
            ScreenFalloff {
                transparent: true,
                ..f
            },
        )
    });
    assert_eq!(through[5 * 11 + 5], 1.);
}

/// FO-52: inside the outline 1, fading over Soft Border outside, 0 beyond;
/// hidden points count (it projects straight through).
#[test]
fn lasso_weights_inside_and_soft_border() {
    assert_eq!(LassoFalloff::default().style, LassoStyle::Rectangle);
    assert!(LassoFalloff::default().world);
    let mesh = grid(true);
    let rect = fs::lasso_outline(
        LassoStyle::Rectangle,
        DVec2::new(15., -15.),
        DVec2::new(45., -45.),
        &[],
    );
    let w = front_view(|v| fs::lasso_weights(&mesh, DMat4::IDENTITY, v, &rect, 0.));
    for y in 0..=10 {
        for x in 0..=10 {
            let want = if (2..=4).contains(&x) && (2..=4).contains(&y) {
                1.
            } else {
                0.
            };
            assert_eq!(w[y * 11 + x], want, "({x}, {y})");
        }
    }
    let soft = front_view(|v| fs::lasso_weights(&mesh, DMat4::IDENTITY, v, &rect, 10.));
    // x = 5 sits 5 px outside the right side.
    assert!(close(soft[3 * 11 + 5], FalloffShape::Smooth.weight(0.5)));
    assert_eq!(soft[3 * 11 + 6], 0.);
    // Circle: centred on the press, through the release point.
    let circle = fs::lasso_outline(LassoStyle::Circle, DVec2::ZERO, DVec2::new(10., 0.), &[]);
    assert!(circle.iter().all(|p| close(p.length(), 10.)));
}

/// FO-44: each pass adds Strength × Smooth(d / Size), clamped to 1.
#[test]
fn airbrush_builds_up_and_clamps() {
    let a = AirbrushFalloff::default();
    assert_eq!((a.size, a.strength, a.transparent), (20., 0.1, false));
    let mesh = grid(false);
    let mut w = vec![0.; mesh.vertices.len()];
    let at = DVec2::new(50., -50.);
    front_view(|v| fs::airbrush_dab(&mut w, &mesh, DMat4::IDENTITY, v, a, at));
    assert!(close(w[5 * 11 + 5], 0.1));
    for _ in 0..20 {
        front_view(|v| fs::airbrush_dab(&mut w, &mesh, DMat4::IDENTITY, v, a, at));
    }
    assert_eq!(w[5 * 11 + 5], 1.);
    assert_eq!(w[0], 0.);
    assert!(AirbrushFalloff { strength: 2., ..a }.validate().is_err());
}

fn added_airbrush(size: f64) -> (Falloff, MixMode) {
    (
        Falloff::Airbrush(AirbrushFalloff {
            size,
            ..Default::default()
        }),
        MixMode::Multiply,
    )
}

fn assert_airbrush_stamp_remap(
    before: Vec<(Falloff, MixMode)>,
    after: Vec<(Falloff, MixMode)>,
    moved: &[(usize, Option<usize>)],
) {
    let mut editor = Editor::default();
    editor
        .dispatch(Command::SpatialFalloff(Some(Falloff::Airbrush(
            AirbrushFalloff::default(),
        ))))
        .unwrap();
    editor.dispatch(Command::AddedFalloffs(before)).unwrap();
    let mut stamps = std::collections::BTreeMap::from([((0, 1), vec![0.25, 0.])]);
    let mut expected = stamps.clone();
    for (paint, &(old, new)) in moved.iter().enumerate() {
        for (offset, item) in [(0, 1), (0, 2), (operandi_core::uv_pipe::UV_STAMP_LAYER, 1)] {
            let weights = vec![(paint + 1) as f64 * 0.125, item as f64 * 0.25];
            stamps.insert((old + offset, item), weights.clone());
            if let Some(new) = new {
                expected.insert((new + offset, item), weights);
            }
        }
    }
    editor.dispatch(Command::FalloffStamps(stamps)).unwrap();
    editor.dispatch(Command::AddedFalloffs(after)).unwrap();
    let actual = editor
        .falloff_stamps
        .iter()
        .map(|(&key, weights)| (key, weights.as_ref().clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(actual, expected);
}

#[test]
fn airbrush_stamps_follow_removal_before_the_layer() {
    let brush = added_airbrush(25.);
    let invert = (Falloff::Invert, MixMode::Multiply);
    assert_airbrush_stamp_remap(vec![invert, brush], vec![brush], &[(2, Some(1))]);
}

#[test]
fn airbrush_stamps_follow_insertion_before_the_layer() {
    let brush = added_airbrush(25.);
    let unpainted = added_airbrush(30.);
    assert_airbrush_stamp_remap(vec![brush], vec![unpainted, brush], &[(1, Some(2))]);
}

#[test]
fn airbrush_stamps_follow_reordered_layers() {
    let first = added_airbrush(25.);
    let second = added_airbrush(30.);
    assert_airbrush_stamp_remap(
        vec![first, second],
        vec![second, first],
        &[(1, Some(2)), (2, Some(1))],
    );
}

#[test]
fn airbrush_stamps_drop_with_their_layer() {
    let first = added_airbrush(25.);
    let second = added_airbrush(30.);
    assert_airbrush_stamp_remap(
        vec![first, second],
        vec![second],
        &[(1, None), (2, Some(1))],
    );
}

#[test]
fn airbrush_stamps_survive_settings_edits() {
    let first = added_airbrush(25.);
    let second = added_airbrush(30.);
    let edited = (added_airbrush(35.).0, MixMode::Add);
    assert_airbrush_stamp_remap(
        vec![first, second],
        vec![edited, second],
        &[(1, Some(1)), (2, Some(2))],
    );
}

/// The stamps carry the weights to a Move; nothing moves before one is
/// drawn; picking another falloff type drops them.
#[test]
fn stamps_drive_a_move() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = grid(false).into();
    e.scene.selection.mode = Mode::Vertex;
    let id = e.scene.active().unwrap().id;
    let before = e.scene.active().unwrap().mesh.vertices.clone();
    e.dispatch(Command::SpatialFalloff(Some(Falloff::Screen(
        ScreenFalloff::default(),
    ))))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let p = e.session.live.as_ref().unwrap().params.clone();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::Z,
        ..p.clone()
    }))
    .unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices, before);
    let stamp: Vec<f64> = (0..before.len()).map(|v| (v % 2) as f64 * 0.5).collect();
    e.dispatch(Command::FalloffStamps([((0, id), stamp.clone())].into()))
        .unwrap();
    let after = e.scene.active().unwrap().mesh.vertices.clone();
    for (v, (a, b)) in after.iter().zip(&before).enumerate() {
        assert!(close(a.z - b.z, stamp[v]), "point {v}");
    }
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices, before);
    e.dispatch(Command::SpatialFalloff(Some(Falloff::Lasso(
        LassoFalloff::default(),
    ))))
    .unwrap();
    assert!(e.falloff_stamps.is_empty());
}

#[test]
fn commands_take_their_options() {
    let parsed = |line: &str| match command_line::parse_sculpt(line) {
        Some(command_line::SculptLine::Falloff { falloff, .. }) => falloff,
        other => panic!("{line}: {other:?}"),
    };
    assert_eq!(
        parsed("falloff.airbrush size:40 strength:0.5"),
        Falloff::Airbrush(AirbrushFalloff {
            size: 40.,
            strength: 0.5,
            ..Default::default()
        })
    );
    assert_eq!(
        parsed("falloff.lasso style:ellipse softBorder:8"),
        Falloff::Lasso(LassoFalloff {
            style: LassoStyle::Ellipse,
            soft_border: 8.,
            ..Default::default()
        })
    );
    assert_eq!(
        parsed("falloff.screen transparent:true"),
        Falloff::Screen(ScreenFalloff {
            transparent: true,
            ..Default::default()
        })
    );
}
