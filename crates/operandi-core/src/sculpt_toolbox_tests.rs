//! The toolbox brushes (spec `sculpt-toolbox`, SCT-01..55): one test per
//! acceptance check that the core can answer without a window.
use super::*;
use crate::{Command, Editor, Face};

/// `n` × `n` quads on the XY plane, 1 m wide, centred, facing +Z.
fn grid(n: u32) -> Mesh {
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(
                i as f64 / n as f64 - 0.5,
                j as f64 / n as f64 - 0.5,
                0.,
            ));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    mesh
}

const N: u32 = 40;

/// Point index at grid cell (i, j).
fn at(i: u32, j: u32) -> usize {
    (j * (N + 1) + i) as usize
}

fn scene_with(mesh: Mesh) -> (Scene, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let id = e.scene.active().unwrap().id;
    e.scene.active_mut().unwrap().mesh = mesh.into();
    (e.scene, id)
}

fn setup(brush: SculptBrush) -> StrokeSetup {
    StrokeSetup {
        brush,
        settings: brush.defaults(),
        ctrl: false,
        shift: false,
        symmetry: Symmetry::OFF,
        falloff: None,
        lock_borders: false,
        lock_corners: true,
        radius: 0.2,
        weight_map: None,
    }
}

fn dab(x: f64, y: f64, z: f64) -> Dab {
    Dab {
        center: DVec3::new(x, y, z),
        radius: 0.2,
        view: DVec3::Z,
        normal: DVec3::Z,
        pressure: 1.,
    }
}

fn points(scene: &Scene, id: u64) -> Vec<DVec3> {
    scene.item(id).unwrap().mesh.vertices.clone()
}

/// Runs one stroke of `dabs` on `scene` and commits it into the scene.
fn stroke(scene: &mut Scene, id: u64, setup: StrokeSetup, dabs: &[Dab]) -> Vec<DVec3> {
    let mut s = Stroke::begin(scene, &[id], setup).unwrap().unwrap();
    for d in dabs {
        s.dab(*d);
    }
    assert!(s.flush(scene));
    points(scene, id)
}

/// A short stroke along +x through the centre.
fn sweep() -> Vec<Dab> {
    (0..20)
        .map(|k| dab(-0.02 + 0.002 * k as f64, 0., 0.))
        .collect()
}

#[test]
fn toolbox_order_names_and_starting_values() {
    // SCT-01: two per row, pinch pair in one slot.
    let labels: Vec<&str> = SculptBrush::ALL.iter().map(|b| b.label()).collect();
    assert_eq!(
        labels,
        [
            "Push",
            "Inflate",
            "Fold",
            "Clay",
            "Smooth",
            "Smudge",
            "Carve",
            "Scar",
            "Flatten",
            "Tangent Pinch",
            "Center Pinch",
            "Emboss",
            "Attenuate",
            "Spin",
            "Move"
        ]
    );
    for b in SculptBrush::ALL {
        assert_eq!(SculptBrush::from_command(b.command()), Some(b));
        b.defaults().validate().unwrap();
        // SCT-09: 5 % step everywhere.
        assert_eq!(b.defaults().step, 0.05, "{b:?}");
    }
    assert_eq!(SculptBrush::Attenuate.command(), "sculpt.shrink");
    let d = |b: SculptBrush| b.defaults();
    assert_eq!(d(SculptBrush::Fold).offset_amount, 0.25);
    assert_eq!(d(SculptBrush::Clay).offset_amount, 0.5);
    assert_eq!(d(SculptBrush::Clay).level, 0.1);
    assert_eq!(d(SculptBrush::Clay).density_mode, DensityMode::Attenuate);
    assert_eq!(d(SculptBrush::Carve).density_mode, DensityMode::Attenuate);
    assert_eq!(d(SculptBrush::Flatten).normal_mode, NormalMode::BrushCenter);
    assert!(!d(SculptBrush::Flatten).inverted_flatten);
    assert_eq!(d(SculptBrush::Spin).normal_mode, NormalMode::BrushCenter);
    assert_eq!(d(SculptBrush::Move).move_mode, MoveMode::Normal);
    assert_eq!(
        SculptSettings::default().pinch_face,
        SculptBrush::TangentPinch
    );
}

#[test]
fn fold_dents_the_centre_and_raises_a_rim() {
    // Acceptance 3 (on a grid): dent ringed by a raised rim; Ctrl flips it.
    for (ctrl, sign) in [(false, 1.), (true, -1.)] {
        let (mut scene, id) = scene_with(grid(N));
        let s = StrokeSetup {
            ctrl,
            ..setup(SculptBrush::Fold)
        };
        let p = stroke(&mut scene, id, s, &[dab(0., 0., 0.)]);
        let centre = p[at(20, 20)].z;
        let rim = p[at(20, 26)].z; // 0.15 m out: past the crossover
        assert!(centre * sign < -1e-4, "ctrl {ctrl}: centre {centre}");
        assert!(rim * sign > 1e-5, "ctrl {ctrl}: rim {rim}");
        assert_eq!(p[at(0, 0)].z, 0.);
    }
}

#[test]
fn clay_builds_to_its_level_and_no_higher() {
    // Acceptance 4: Level 10 % of a 0.2 m radius stops at 0.02 m.
    let (mut scene, id) = scene_with(grid(N));
    let many: Vec<Dab> = (0..400)
        .map(|k| dab(0.0001 * (k % 2) as f64, 0., 0.))
        .collect();
    let p = stroke(&mut scene, id, setup(SculptBrush::Clay), &many);
    let z = p[at(20, 20)].z;
    assert!((z - 0.02).abs() < 1e-3, "centre {z}");
    assert!(p.iter().all(|q| q.z <= 0.02 + 1e-9));
    // Level 0 % changes nothing on a flat grid.
    let (mut scene, id) = scene_with(grid(N));
    let mut s0 = setup(SculptBrush::Clay);
    s0.settings.level = 0.;
    let p = stroke(&mut scene, id, s0, &many);
    assert!(p.iter().all(|q| q.z == 0.));
    // Ctrl cuts down to a plane Level below.
    let (mut scene, id) = scene_with(grid(N));
    let p = stroke(
        &mut scene,
        id,
        StrokeSetup {
            ctrl: true,
            ..setup(SculptBrush::Clay)
        },
        &many,
    );
    assert!(p[at(20, 20)].z < -0.01);
}

#[test]
fn carve_caps_depth_per_stroke_and_a_second_stroke_goes_deeper() {
    // Acceptance 5.
    let back_and_forth: Vec<Dab> = (0..60)
        .map(|k| {
            let u = (k % 20) as f64 * 0.002;
            let x = if (k / 20) % 2 == 0 { u } else { 0.04 - u };
            dab(x - 0.02, 0., 0.)
        })
        .collect();
    let (mut a, id) = scene_with(grid(N));
    let once = stroke(&mut a, id, setup(SculptBrush::Carve), &sweep());
    let (mut b, id_b) = scene_with(grid(N));
    let repeated = stroke(&mut b, id_b, setup(SculptBrush::Carve), &back_and_forth);
    let (z1, z3) = (once[at(20, 20)].z, repeated[at(20, 20)].z);
    assert!(z1 < -1e-3, "carve digs: {z1}");
    // The cap is the full offset (50 % of the radius) × weight.
    assert!(z3 >= -0.1 - 1e-9 && z3 < z1 + 1e-9, "{z1} {z3}");
    let again = stroke(&mut b, id_b, setup(SculptBrush::Carve), &back_and_forth);
    assert!(again[at(20, 20)].z < z3 - 1e-4);
    // Ctrl raises a ridge.
    let (mut c, id_c) = scene_with(grid(N));
    let up = stroke(
        &mut c,
        id_c,
        StrokeSetup {
            ctrl: true,
            ..setup(SculptBrush::Carve)
        },
        &sweep(),
    );
    assert!(up[at(20, 20)].z > 1e-3);
}

/// A grid with a bump in the middle and a dip to the side.
fn bumpy() -> Mesh {
    let mut m = grid(N);
    for p in &mut m.vertices {
        let d = (p.x * p.x + p.y * p.y).sqrt();
        p.z = 0.05 * (1. - d / 0.15).max(0.);
    }
    m
}

#[test]
fn flatten_lowers_peaks_and_inverted_fills_dips() {
    // Acceptance 6: Mouse Down flattens to the press plane.
    let (mut scene, id) = scene_with(bumpy());
    let mut s = setup(SculptBrush::Flatten);
    s.settings.normal_mode = NormalMode::MouseDown;
    let many: Vec<Dab> = (0..100)
        .map(|k| dab(0.0001 * (k % 2) as f64, 0., 0.02))
        .collect();
    let p = stroke(&mut scene, id, s, &many);
    // Points above z 0.02 come down to it; points below stay.
    let top = p[at(20, 20)].z;
    assert!((top - 0.02).abs() < 2e-3, "peak {top}");
    assert!(p.iter().all(|q| q.z <= 0.05 + 1e-9));
    // Inverted Flatten raises what's below the plane instead.
    let (mut scene, id) = scene_with(bumpy());
    let mut s = setup(SculptBrush::Flatten);
    s.settings.normal_mode = NormalMode::MouseDown;
    s.settings.inverted_flatten = true;
    let p = stroke(&mut scene, id, s, &many);
    assert!((p[at(20, 20)].z - 0.05).abs() < 1e-9, "peak stays");
    // On the bump's foot (z ≈ 0.008), under the brush: raised toward 0.02.
    let foot = at(20, 25);
    let before = bumpy().vertices[foot].z;
    assert!(
        p[foot].z > before + 1e-3 && p[foot].z <= 0.02 + 1e-9,
        "{}",
        p[foot].z
    );
}

#[test]
fn center_pinch_gathers_toward_the_centre_on_the_surface() {
    // Acceptance 7.
    let (mut scene, id) = scene_with(grid(N));
    let p = stroke(&mut scene, id, setup(SculptBrush::CenterPinch), &sweep());
    let base = grid(N).vertices;
    let v = at(24, 20);
    assert!(p[v].x < base[v].x - 1e-4, "pulled in: {}", p[v].x);
    assert!(p.iter().all(|q| q.z.abs() < 1e-12), "stays on the plane");
    // Ctrl spreads.
    let (mut scene, id) = scene_with(grid(N));
    let s = StrokeSetup {
        ctrl: true,
        ..setup(SculptBrush::CenterPinch)
    };
    let p = stroke(&mut scene, id, s, &sweep());
    assert!(p[v].x > base[v].x + 1e-4);
}

#[test]
fn tangent_pinch_draws_toward_the_stroke_line() {
    let (mut scene, id) = scene_with(grid(N));
    let p = stroke(&mut scene, id, setup(SculptBrush::TangentPinch), &sweep());
    let base = grid(N).vertices;
    // The stroke runs along x: points beside it move in y toward it, not
    // along x.
    let v = at(20, 24);
    assert!(p[v].y < base[v].y - 1e-4);
    assert!((p[v].x - base[v].x).abs() < 1e-9);
    assert!(p.iter().all(|q| q.z.abs() < 1e-12));
}

#[test]
fn spin_swirls_and_ctrl_turns_the_other_way() {
    // Acceptance 8: clockwise looking down +Z (toward the viewer).
    let v = at(24, 20); // on +x of the centre
    let base = grid(N).vertices;
    let (mut scene, id) = scene_with(grid(N));
    let p = stroke(&mut scene, id, setup(SculptBrush::Spin), &sweep());
    assert!(p[v].y < base[v].y - 1e-4, "clockwise: {}", p[v].y);
    let (mut scene, id) = scene_with(grid(N));
    let s = StrokeSetup {
        ctrl: true,
        ..setup(SculptBrush::Spin)
    };
    let p = stroke(&mut scene, id, s, &sweep());
    assert!(p[v].y > base[v].y + 1e-4);
}

#[test]
fn smudge_drags_points_along_the_stroke() {
    let (mut scene, id) = scene_with(grid(N));
    let p = stroke(&mut scene, id, setup(SculptBrush::Smudge), &sweep());
    let base = grid(N).vertices;
    let v = at(20, 20);
    assert!(p[v].x > base[v].x + 1e-4);
    assert!(p[v].z.abs() < 1e-12, "along the surface");
    let (mut scene, id) = scene_with(grid(N));
    let s = StrokeSetup {
        ctrl: true,
        ..setup(SculptBrush::Smudge)
    };
    let p = stroke(&mut scene, id, s, &sweep());
    assert!(p[v].x < base[v].x - 1e-4);
}

#[test]
fn scar_cuts_in_and_pinches_toward_the_line() {
    let (mut scene, id) = scene_with(grid(N));
    let p = stroke(&mut scene, id, setup(SculptBrush::Scar), &sweep());
    let base = grid(N).vertices;
    assert!(p[at(20, 20)].z < -1e-4);
    let v = at(20, 23);
    assert!(p[v].y < base[v].y - 1e-5, "pinched: {}", p[v].y);
    let (mut scene, id) = scene_with(grid(N));
    let s = StrokeSetup {
        ctrl: true,
        ..setup(SculptBrush::Scar)
    };
    let p = stroke(&mut scene, id, s, &sweep());
    assert!(p[at(20, 20)].z > 1e-4, "welt");
}

#[test]
fn emboss_moves_the_pressed_patch_along_its_normals() {
    // Acceptance 9: right raises, left lowers; the patch stays put.
    for (dist, sign) in [(0.1, 1.), (-0.1, -1.)] {
        let (mut scene, id) = scene_with(grid(N));
        let mut s = Stroke::begin(&scene, &[id], setup(SculptBrush::Emboss))
            .unwrap()
            .unwrap();
        assert!(s.is_grab());
        s.dab(dab(0., 0., 0.));
        s.dab(dab(0.3, 0., 0.)); // later dabs don't capture more
        s.emboss_to(dist);
        s.flush(&mut scene);
        let p = points(&scene, id);
        // Offset Amount 25 % × 0.1 m × the centre's weight 1.
        assert!((p[at(20, 20)].z - 0.025 * sign).abs() < 1e-9);
        assert_eq!(p[at(32, 20)].z, 0.);
        assert_eq!(s.label(), "Emboss Stroke");
    }
}

#[test]
fn attenuate_is_unavailable_on_plain_meshes() {
    // SCT-45 / SCT-Q12 recommendation.
    let (scene, id) = scene_with(grid(N));
    if ATTENUATE_ON_PLAIN_MESHES {
        return;
    }
    let err = Stroke::begin(&scene, &[id], setup(SculptBrush::Attenuate)).err();
    assert!(err.is_some());
    assert!(SculptBrush::Attenuate.unavailable().is_some());
    // Shift still smooths with it.
    let s = StrokeSetup {
        shift: true,
        ..setup(SculptBrush::Attenuate)
    };
    assert!(Stroke::begin(&scene, &[id], s).unwrap().is_some());
}

#[test]
fn move_slide_keeps_points_on_the_surface() {
    // SCT-49 (provisional SCT-Q14).
    let (mut scene, id) = scene_with(grid(N));
    let mut st = setup(SculptBrush::Move);
    st.settings.move_mode = MoveMode::Slide;
    let mut s = Stroke::begin(&scene, &[id], st).unwrap().unwrap();
    s.dab(dab(0., 0., 0.));
    s.grab_to(DVec3::new(0.05, 0., 0.1));
    s.flush(&mut scene);
    let p = points(&scene, id);
    assert!((p[at(20, 20)].x - 0.05).abs() < 1e-9);
    assert!(p.iter().all(|q| q.z.abs() < 1e-12));
}

#[test]
fn every_brush_respects_shift_locks_and_symmetry() {
    // Acceptance 10 (core half): Shift smooths ("Smooth Stroke"); locked
    // corners never move; symmetry mirrors.
    for b in SculptBrush::ALL {
        if b.kind() == BrushKind::Grab {
            continue;
        }
        let (scene, id) = scene_with(grid(N));
        let s = Stroke::begin(
            &scene,
            &[id],
            StrokeSetup {
                shift: true,
                ..setup(b)
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(s.label(), "Smooth Stroke", "{b:?}");
        if b.unavailable().is_some() {
            continue;
        }
        // A sweep over a corner: Lock Corners holds it.
        let (mut scene, id) = scene_with(grid(N));
        let corner: Vec<Dab> = (0..10)
            .map(|k| dab(-0.5 + 0.002 * k as f64, -0.5, 0.))
            .collect();
        let p = stroke(&mut scene, id, setup(b), &corner);
        assert_eq!(p[at(0, 0)], grid(N).vertices[at(0, 0)], "{b:?} corner");
        // Symmetry X: a stroke at +x moves its mirror at −x the same way.
        let (mut scene, id) = scene_with(grid(N));
        let mut st = setup(b);
        st.symmetry = Symmetry {
            axis: Some(0),
            ..Symmetry::OFF
        };
        let side: Vec<Dab> = (0..20)
            .map(|k| dab(0.2, -0.02 + 0.002 * k as f64, 0.))
            .collect();
        let p = stroke(&mut scene, id, st, &side);
        let (a, m) = (p[at(28, 20)], p[at(12, 20)]);
        assert!((a.z - m.z).abs() < 1e-9, "{b:?} mirror z {} {}", a.z, m.z);
        assert!((a.x + m.x).abs() < 1e-9, "{b:?} mirror x {} {}", a.x, m.x);
    }
}

#[test]
fn old_settings_files_load_with_the_new_brushes_at_their_defaults() {
    // A settings file from before the toolbox brushes.
    let old = r#"{"push":{"offset_amount":0.4},"lock_borders":true}"#;
    let s: SculptSettings = serde_json::from_str(old).unwrap();
    let s = s.sanitized();
    assert_eq!(s.push.offset_amount, 0.4);
    assert!(s.lock_borders);
    assert_eq!(s.clay, SculptBrush::Clay.defaults());
    assert_eq!(s.pinch_face, SculptBrush::TangentPinch);
}
