//! Vertex Map Tools (spec vertex-map-tools-2026-11-11, VMT-01..19).
use glam::DVec3;
use operandi_core::sculpt::{Dab, Stroke, StrokeSetup};
use operandi_core::*;
use std::collections::BTreeMap;

const N: u32 = 20;

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

fn at(i: u32, j: u32) -> u32 {
    j * (N + 1) + i
}

fn editor() -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = grid().into();
    let id = e.scene.active().unwrap().id;
    e.dispatch(Command::NewWeightMap {
        name: "Mask".into(),
        value: None,
    })
    .unwrap();
    (e, id)
}

fn setup(e: &Editor, brush: SculptBrush) -> StrokeSetup {
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
        weight_map: e.selected_weight_map.clone(),
    }
}

fn dab(x: f64) -> Dab {
    Dab {
        center: DVec3::new(x, 0., 0.),
        radius: 0.2,
        view: DVec3::Z,
        normal: DVec3::Z,
        pressure: 1.,
    }
}

/// One stroke of `dabs` committed through `Command::WeightStroke`.
fn paint(e: &mut Editor, id: u64, setup: StrokeSetup, dabs: usize) -> BTreeMap<u32, f64> {
    let mut s = Stroke::begin(&e.scene, &[id], setup).unwrap().unwrap();
    for _ in 0..dabs {
        s.dab(dab(0.));
    }
    let edit = s.weight_edit().expect("a weight stroke");
    s.restore(&mut e.scene);
    e.dispatch(Command::WeightStroke(Box::new(edit))).unwrap();
    map(e, id)
}

fn map(e: &Editor, id: u64) -> BTreeMap<u32, f64> {
    let item = e.scene.items.iter().find(|i| i.id == id).unwrap();
    item.mesh.attributes.point_weights["Mask"].clone()
}

#[test]
fn new_weight_map_is_selected_and_init_value_fills() {
    let (mut e, id) = editor();
    assert_eq!(e.selected_weight_map.as_deref(), Some("Mask"));
    assert!(map(&e, id).is_empty(), "no value anywhere (VM-03)");
    assert!(
        e.dispatch(Command::NewWeightMap {
            name: "Mask".into(),
            value: None
        })
        .is_err()
    );
    e.dispatch(Command::NewWeightMap {
        name: "Full".into(),
        value: Some(0.5),
    })
    .unwrap();
    let item = e.scene.items.iter().find(|i| i.id == id).unwrap();
    assert_eq!(
        item.mesh.attributes.point_weights["Full"].len(),
        grid().vertices.len()
    );
    assert_eq!(e.selected_weight_map.as_deref(), Some("Full"));
}

#[test]
fn n32_paint_strength_builds_between_strokes_and_ctrl_lowers() {
    let (mut e, id) = editor();
    let mut st = setup(&e, SculptBrush::VmPaint);
    st.settings.offset_amount = 0.2;
    let first = paint(&mut e, id, st.clone(), 20);
    assert!((first[&at(10, 10)] - 0.2).abs() < 1e-9);
    let second = paint(&mut e, id, st.clone(), 20);
    assert!((second[&at(10, 10)] - 0.36).abs() < 1e-9);
    assert!(!second.contains_key(&at(0, 0)), "far points untouched");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(map(&e, id), first);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(map(&e, id), second);
    let saved = persistence::encode(&e.scene).unwrap();
    let reopened = persistence::decode(&saved).unwrap();
    let weights = &reopened.item(id).unwrap().mesh.attributes.point_weights["Mask"];
    assert!((weights[&at(10, 10)] - 0.36).abs() < 1e-9);

    st.ctrl = true;
    let m = paint(&mut e, id, st, 20);
    assert!((m[&at(10, 10)] - 0.288).abs() < 1e-9);
    assert!(m.values().all(|w| *w <= 1. + 1e-12));
    // Points don't move.
    assert!(
        e.scene
            .active()
            .unwrap()
            .mesh
            .vertices
            .iter()
            .all(|p| p.z == 0.)
    );
}

#[test]
fn n32_paint_preview_uses_peak_coverage_and_cancel_restores_the_map() {
    let (mut e, id) = editor();
    let mut st = setup(&e, SculptBrush::VmPaint);
    st.settings.offset_amount = 0.2;
    let mut stroke = Stroke::begin(&e.scene, &[id], st).unwrap().unwrap();
    for (pressure, want) in [(0.25, 0.05), (0.75, 0.15), (0.5, 0.15)] {
        stroke.dab(Dab {
            pressure,
            ..dab(0.)
        });
        assert!(stroke.flush(&mut e.scene));
        assert!((map(&e, id)[&at(10, 10)] - want).abs() < 1e-9);
    }
    stroke.restore(&mut e.scene);
    assert!(map(&e, id).is_empty());
}

#[test]
fn set_value_writes_exactly_with_no_fade_and_ctrl_erases() {
    let (mut e, id) = editor();
    let mut s = setup(&e, SculptBrush::VmSetValue);
    assert_eq!(s.settings.offset_amount, 0.);
    s.settings.offset_amount = 0.7;
    let m = paint(&mut e, id, s.clone(), 1);
    // Centre and near the rim (0.15 m out) both get exactly 0.7.
    assert_eq!(m[&at(10, 10)], 0.7);
    assert_eq!(m[&at(13, 10)], 0.7);
    let mut c = s;
    c.ctrl = true;
    let m = paint(&mut e, id, c, 1);
    assert!(!m.contains_key(&at(10, 10)));
}

#[test]
fn erase_removes_values_and_scale_down_pulls_toward_zero() {
    let (mut e, id) = editor();
    e.dispatch(Command::NewWeightMap {
        name: "Mask2".into(),
        value: Some(1.),
    })
    .unwrap();
    e.dispatch(Command::SelectWeightMap(Some("Mask2".into())))
        .unwrap();
    let get = |e: &Editor| {
        e.scene
            .items
            .iter()
            .find(|i| i.id == id)
            .unwrap()
            .mesh
            .attributes
            .point_weights["Mask2"]
            .clone()
    };
    let mut s = Stroke::begin(&e.scene, &[id], setup(&e, SculptBrush::VmScaleDown))
        .unwrap()
        .unwrap();
    s.dab(dab(0.));
    let edit = s.weight_edit().unwrap();
    s.restore(&mut e.scene);
    e.dispatch(Command::WeightStroke(Box::new(edit))).unwrap();
    let m = get(&e);
    let w = m[&at(10, 10)];
    assert!(w < 1. && w > 0.9, "one dab nudges: {w}");
    let mut s = Stroke::begin(&e.scene, &[id], setup(&e, SculptBrush::VmErase))
        .unwrap()
        .unwrap();
    s.dab(dab(0.));
    let edit = s.weight_edit().unwrap();
    s.restore(&mut e.scene);
    e.dispatch(Command::WeightStroke(Box::new(edit))).unwrap();
    let m = get(&e);
    assert!(!m.contains_key(&at(10, 10)), "erased, not 0 (VMT-14)");
    assert!(m.contains_key(&at(0, 0)));
}

#[test]
fn smooth_blurs_and_shift_smooths_the_map() {
    let (mut e, id) = editor();
    // A hard edge: 1 for x < 0.
    let item = e.scene.active_mut().unwrap();
    let edge: BTreeMap<u32, f64> = (0..=N)
        .flat_map(|j| (0..=N).map(move |i| (i, j)))
        .filter(|&(i, _)| i < 10)
        .map(|(i, j)| (at(i, j), 1.))
        .collect();
    item.mesh
        .attributes
        .point_weights
        .insert("Mask".into(), edge);
    let m = {
        let st = setup(&e, SculptBrush::VmSmooth);
        paint(&mut e, id, st, 3)
    };
    let (l, r) = (m[&at(9, 10)], m.get(&at(10, 10)).copied().unwrap_or(0.));
    assert!(l < 1. && r > 0., "blurred: {l} {r}");
    // Shift with Paint smooths too, and is labelled Smooth.
    let mut s = setup(&e, SculptBrush::VmPaint);
    s.shift = true;
    let stroke = Stroke::begin(&e.scene, &[id], s).unwrap().unwrap();
    assert_eq!(stroke.label(), "Vertex Map Smooth Stroke");
}

#[test]
fn normalize_lowers_the_other_maps() {
    // VMT-11 example: A at 100 %, paint B to 50 %: A ends at 50 %.
    let (mut e, id) = editor();
    e.dispatch(Command::NewWeightMap {
        name: "A".into(),
        value: Some(1.),
    })
    .unwrap();
    e.dispatch(Command::SelectWeightMap(Some("Mask".into())))
        .unwrap();
    let mut s = setup(&e, SculptBrush::VmSetValue);
    s.settings.offset_amount = 0.5;
    paint(&mut e, id, s.clone(), 1);
    let a = &e.scene.active().unwrap().mesh.attributes.point_weights["A"];
    assert!((a[&at(10, 10)] - 0.5).abs() < 1e-9);
    assert_eq!(a[&at(0, 0)], 1.);
    // Off: the other map stays.
    let (mut e, id) = editor();
    e.dispatch(Command::NewWeightMap {
        name: "A".into(),
        value: Some(1.),
    })
    .unwrap();
    e.dispatch(Command::SelectWeightMap(Some("Mask".into())))
        .unwrap();
    s.settings.normalize_weights = false;
    s.weight_map = Some("Mask".into());
    paint(&mut e, id, s, 1);
    let a = &e.scene.active().unwrap().mesh.attributes.point_weights["A"];
    assert_eq!(a[&at(10, 10)], 1.);
}

#[test]
fn a_stroke_is_one_undo_step_and_needs_a_map() {
    let (mut e, id) = editor();
    let steps = e.undo.len();
    {
        let st = setup(&e, SculptBrush::VmPaint);
        paint(&mut e, id, st, 5)
    };
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(e.undo.last().unwrap().label, "Vertex Map Paint Stroke");
    e.dispatch(Command::Undo).unwrap();
    assert!(map(&e, id).is_empty());
    let mut s = setup(&e, SculptBrush::VmPaint);
    s.weight_map = None;
    assert!(Stroke::begin(&e.scene, &[id], s).is_err());
}

#[test]
fn every_brush_command_round_trips() {
    for b in SculptBrush::every() {
        assert_eq!(SculptBrush::from_command(b.command()), Some(b));
    }
    assert_eq!(SculptBrush::VERTEX_MAP.len(), 5);
    let s = SculptSettings::default();
    assert_eq!(s.vm_paint.offset_amount, 0.5);
    assert_eq!(s.vm_set_value.offset_amount, 0.);
    assert!(s.vm_smooth.normalize_weights);
}
