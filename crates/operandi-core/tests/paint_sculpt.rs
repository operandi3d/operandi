//! Painting stage 7: the shared painting parts on sculpt brushes and the
//! sculpt mask (PNT-243, 246, 260), acceptance A1–A3 at the core level.
//! Painting builds only.
#![cfg(feature = "painting")]
use glam::{DVec2, DVec3};
use operandi_core::paint_inks::{Ink, InkRun, Inks, Parameter, ParametricInk};
use operandi_core::paint_sculpt::SculptPaint;
use operandi_core::paint_stroke::PaintView;
use operandi_core::paint_tips::{ImageTip, TipExtra};
use operandi_core::sculpt::{Dab, Stroke, StrokeSetup};
use operandi_core::*;
use operandi_paint::{Depth, Format, Layout, PaintImage};
use std::sync::Arc;

const N: u32 = 40;

fn grid() -> Mesh {
    let mut mesh = Mesh::default();
    for j in 0..=N {
        for i in 0..=N {
            mesh.vertices.push(DVec3::new(
                i as f64 / N as f64 - 0.5,
                j as f64 / N as f64 - 0.5,
                0.,
            ));
        }
    }
    for j in 0..N {
        for i in 0..N {
            let a = j * (N + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + N + 2, a + N + 1],
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

fn at(i: u32, j: u32) -> usize {
    (j * (N + 1) + i) as usize
}

fn editor(mesh: Mesh) -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    let id = e.scene.active().unwrap().id;
    (e, id)
}

fn setup(brush: SculptBrush, radius: f64) -> StrokeSetup {
    StrokeSetup {
        brush,
        settings: brush.defaults(),
        ctrl: false,
        shift: false,
        symmetry: Symmetry::OFF,
        falloff: None,
        lock_borders: false,
        lock_corners: false,
        radius,
        weight_map: None,
    }
}

fn top_view() -> PaintView {
    PaintView::looking(DVec3::new(0., 0., 3.), DVec3::ZERO, DVec2::new(400., 400.))
}

/// One stroke of `n` dabs at `dab`, then the points.
fn sculpt(
    e: &mut Editor,
    id: u64,
    s: StrokeSetup,
    paint: SculptPaint,
    dab: Dab,
    n: usize,
) -> Vec<DVec3> {
    let mut stroke = Stroke::begin(&e.scene, &[id], s).unwrap().unwrap();
    stroke.set_paint(Arc::new(paint));
    for _ in 0..n {
        stroke.dab(dab);
    }
    stroke.flush(&mut e.scene);
    e.scene.item(id).unwrap().mesh.vertices.clone()
}

fn flat_dab(radius: f64) -> Dab {
    Dab {
        center: DVec3::ZERO,
        radius,
        view: DVec3::Z,
        normal: DVec3::Z,
        pressure: 1.,
    }
}

/// White disc on a mid-grey square, float pixels.
fn disc_picture() -> Arc<PaintImage> {
    let mut p = PaintImage::new(Layout {
        width: 64,
        height: 64,
        format: Format::Rgba,
        depth: Depth::F32,
    })
    .unwrap();
    for y in 0..64 {
        for x in 0..64 {
            let d = ((x as f64 - 31.5).powi(2) + (y as f64 - 31.5).powi(2)).sqrt();
            let v = if d < 14. { 1. } else { 0.5 };
            p.set(y * 64 + x, [v, v, v, 1.]);
        }
    }
    Arc::new(p)
}

#[test]
fn a1_a_texture_tip_at_offset_50_pushes_the_pattern_and_leaves_no_step() {
    let (mut e, id) = editor(grid());
    let mut paint = SculptPaint::new(top_view());
    paint.extra = Some(TipExtra::Image {
        px: disc_picture(),
        tip: ImageTip {
            use_as_mask: true,
            offset: 0.5,
            ..ImageTip::default()
        },
    });
    paint.signed_offset = Some(0.5);
    let v = sculpt(
        &mut e,
        id,
        setup(SculptBrush::Push, 0.4),
        paint,
        flat_dab(0.4),
        4,
    );
    // The white disc (centre) is pushed out; the grey border (0.3 out,
    // still under the square picture) doesn't move: no step.
    assert!(v[at(20, 20)].z > 0.01, "{:?}", v[at(20, 20)]);
    assert!(v[at(32, 20)].z.abs() < 1e-6, "{:?}", v[at(32, 20)]);
    assert!(v[at(20, 31)].z.abs() < 1e-6);
    // Without the zero point the grey would push too.
    let (mut e2, id2) = editor(grid());
    let mut plain = SculptPaint::new(top_view());
    plain.extra = Some(TipExtra::Image {
        px: disc_picture(),
        tip: ImageTip {
            use_as_mask: true,
            ..ImageTip::default()
        },
    });
    let w = sculpt(
        &mut e2,
        id2,
        setup(SculptBrush::Push, 0.4),
        plain,
        flat_dab(0.4),
        4,
    );
    assert!(w[at(32, 20)].z > 1e-4);
}

#[test]
fn a2_parametric_slope_lets_only_up_facing_areas_change() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: UnitDestination::ExistingMesh,
    })
    .unwrap();
    let id = e.scene.active().unwrap().id;
    let r = e
        .scene
        .item(id)
        .unwrap()
        .mesh
        .vertices
        .iter()
        .map(|p| p.length())
        .fold(0., f64::max);
    let slope = || {
        let mut p = SculptPaint::new(PaintView::looking(
            DVec3::new(0., 4., 0.),
            DVec3::ZERO,
            DVec2::new(400., 400.),
        ));
        p.inks = InkRun {
            inks: Inks(vec![Ink::Parametric(ParametricInk {
                parameter: Parameter::Slope,
                use_as_mask: true,
                ..ParametricInk::default()
            })]),
            ..InkRun::default()
        };
        p
    };
    let before = e.scene.item(id).unwrap().mesh.vertices.clone();
    // On top (facing up): it moves.
    let top = Dab {
        center: DVec3::new(0., r, 0.),
        radius: 0.25 * r,
        view: DVec3::Y,
        normal: DVec3::Y,
        pressure: 1.,
    };
    let after = sculpt(
        &mut e,
        id,
        setup(SculptBrush::Clay, 0.25 * r),
        slope(),
        top,
        6,
    );
    let moved_top = before
        .iter()
        .zip(&after)
        .filter(|(a, b)| a.distance(**b) > 1e-6)
        .count();
    assert!(moved_top > 0);
    // On the side (facing sideways): nothing changes.
    let mut e3 = Editor::default();
    e3.dispatch(Command::NewMesh).unwrap();
    e3.dispatch(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: UnitDestination::ExistingMesh,
    })
    .unwrap();
    let id3 = e3.scene.active().unwrap().id;
    let before = e3.scene.item(id3).unwrap().mesh.vertices.clone();
    let side = Dab {
        center: DVec3::new(r, 0., 0.),
        radius: 0.25 * r,
        view: DVec3::X,
        normal: DVec3::X,
        pressure: 1.,
    };
    let after = sculpt(
        &mut e3,
        id3,
        setup(SculptBrush::Clay, 0.25 * r),
        slope(),
        side,
        6,
    );
    let moved = before
        .iter()
        .zip(&after)
        .map(|(a, b)| a.distance(*b))
        .fold(0., f64::max);
    assert!(moved < 1e-3 * r, "the side barely moves: {moved}");
}

#[test]
fn a3_the_mask_holds_points_partial_mask_moves_partly_and_invert_flips() {
    // Left half fully masked, a band at x ≈ +0.1 half masked.
    let mut mask = vec![0f32; ((N + 1) * (N + 1)) as usize];
    for j in 0..=N {
        for i in 0..=N {
            let x = i as f64 / N as f64 - 0.5;
            mask[at(i, j)] = if x < -0.01 {
                1.
            } else if (0.07..0.13).contains(&x) {
                0.5
            } else {
                0.
            };
        }
    }
    let mask = Arc::new(mask);
    let run = |invert: bool| {
        let (mut e, id) = editor(grid());
        let mut p = SculptPaint::new(top_view());
        p.masks.insert(id, mask.clone());
        p.invert_mask = invert;
        sculpt(
            &mut e,
            id,
            setup(SculptBrush::Push, 0.4),
            p,
            flat_dab(0.4),
            4,
        )
    };
    let free = {
        let (mut e, id) = editor(grid());
        sculpt(
            &mut e,
            id,
            setup(SculptBrush::Push, 0.4),
            SculptPaint::new(top_view()),
            flat_dab(0.4),
            4,
        )
    };
    let v = run(false);
    assert_eq!(v[at(16, 20)].z, 0., "masked: held");
    assert!(v[at(24, 20)].z > 0.01, "free: moves");
    // x = +0.1 (i = 24) is half masked: about half the free move.
    let (half, full) = (v[at(24, 20)].z, free[at(24, 20)].z);
    assert!((half / full - 0.5).abs() < 0.05, "{half} vs {full}");
    // Invert: the masked side moves, the free side is held.
    let w = run(true);
    assert!(w[at(16, 20)].z > 0.01);
    assert_eq!(w[at(26, 20)].z, 0.);
}

#[test]
fn a3_the_mask_lives_in_the_scene_undoes_and_isnt_saved() {
    use operandi_core::paint::PaintEdit;
    let (mut e, id) = editor(grid());
    let n = e.scene.item(id).unwrap().mesh.vertices.len();
    e.dispatch(Command::Paint(PaintEdit::Mask {
        item: id,
        values: Some(Arc::new(vec![1.; n])),
        label: "Mask Paint".into(),
    }))
    .unwrap();
    assert!(e.scene.paint.masks.contains_key(&id));
    e.dispatch(Command::Undo).unwrap();
    assert!(!e.scene.paint.masks.contains_key(&id), "one undo step");
    e.dispatch(Command::Redo).unwrap();
    assert!(e.scene.paint.masks.contains_key(&id));
    // Saving and reading back leaves no mask.
    let dir = std::env::temp_dir().join(format!("operandi-mask-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mask.opscene");
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    assert!(back.paint.masks.is_empty());
    let _ = std::fs::remove_dir_all(dir);
    // A wrong-sized mask is refused.
    assert!(
        e.dispatch(Command::Paint(PaintEdit::Mask {
            item: id,
            values: Some(Arc::new(vec![1.; 3])),
            label: "Mask Paint".into(),
        }))
        .is_err()
    );
}

/// A 40 × 40 grid with a round bump of height 0.2 in the middle.
fn bump() -> Mesh {
    let mut m = grid();
    for p in &mut m.vertices {
        let r2 = p.x * p.x + p.y * p.y;
        p.z = 0.2 * (-r2 / 0.02).exp();
    }
    m
}

#[test]
fn a5_geometry_to_brush_makes_a_tip_that_stamps_the_same_bump() {
    use operandi_core::paint_geometry_brush::*;
    let px = geometry_to_brush(
        &bump(),
        &[],
        &GeometryBrush {
            resolution: 64,
            iterations: 200,
            ..GeometryBrush::default()
        },
    )
    .unwrap();
    // The bump is bright in the middle; the edge sits at the zero point.
    assert!(px.get(32 * 64 + 32)[0] > 0.9, "{:?}", px.get(32 * 64 + 32));
    assert!((px.get(2 * 64 + 2)[0] - 0.5).abs() < 0.02);
    // Stamped on a plane with a Texture tip at Offset 50 %: a bump.
    let (mut e, id) = editor(grid());
    let mut paint = SculptPaint::new(top_view());
    paint.extra = Some(TipExtra::Image {
        px: Arc::new(px),
        tip: ImageTip {
            use_as_mask: true,
            offset: 0.5,
            ..ImageTip::default()
        },
    });
    paint.signed_offset = Some(0.5);
    let v = sculpt(
        &mut e,
        id,
        setup(SculptBrush::Push, 0.5),
        paint,
        flat_dab(0.5),
        3,
    );
    let centre = v[at(20, 20)].z;
    let side = v[at(20 + 8, 20)].z;
    let edge = v[at(20 + 18, 20)].z;
    assert!(centre > 0.01, "{centre}");
    assert!(
        side < centre * 0.6,
        "falls off like the bump: {side} vs {centre}"
    );
    assert!(edge.abs() < centre * 0.05, "no step at the rim: {edge}");
}

#[test]
fn geometry_to_brush_refuses_a_closed_mesh_and_keeps_undercuts_in_vector_mode() {
    use operandi_core::paint_geometry_brush::*;
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::InsertUnit {
        tool: Tool::Box,
        destination: UnitDestination::ExistingMesh,
    })
    .unwrap();
    let cube = e.scene.active().unwrap().mesh.to_owned_value();
    assert_eq!(
        geometry_to_brush(&cube, &[], &GeometryBrush::default()).unwrap_err(),
        CLOSED_PATCH
    );
    let px = geometry_to_brush(
        &bump(),
        &[],
        &GeometryBrush {
            mode: GeometryBrushMode::VectorDisplacement,
            resolution: 32,
            iterations: 100,
            ..GeometryBrush::default()
        },
    )
    .unwrap();
    assert_eq!(px.format, Format::Rgb);
    // The middle pushes out along the axis (blue above mid).
    assert!(px.get(16 * 32 + 16)[2] > 0.9);
}

#[test]
fn constrain_to_background_keeps_sculpted_points_on_the_background() {
    use operandi_core::background_constraint::*;
    // The sculpted grid, and a background plane 0.05 above it.
    let (mut e, id) = editor(grid());
    let mut plane = grid();
    for p in &mut plane.vertices {
        p.z = 0.05;
    }
    let bg = Background::of_mesh(&plane, glam::DMat4::IDENTITY).unwrap();
    let mut paint = SculptPaint::new(top_view());
    paint.background = Some((
        Arc::new(bg),
        BackgroundConstraint {
            double_sided: true,
            ..BackgroundConstraint::default()
        },
        ConstraintView {
            forward: -DVec3::Z,
            eye: None,
        },
    ));
    let v = sculpt(
        &mut e,
        id,
        setup(SculptBrush::Push, 0.4),
        paint,
        flat_dab(0.4),
        6,
    );
    let moved: Vec<f64> = v.iter().map(|p| p.z).filter(|z| *z != 0.).collect();
    assert!(!moved.is_empty());
    assert!(
        moved.iter().all(|z| (z - 0.05).abs() < 1e-6),
        "on the background: {moved:?}"
    );
}
