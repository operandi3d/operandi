//! The shared drag layer (loop-slice-2026-11-15/BEHAVIOR-DRAG-LAYER.md):
//! the table lookup, fall-through (DL-09), the axis lock (DL-04), whole
//! steps (DL-05) and the sign (DL-06). Original test data.
use operandi_core::Tool;
use operandi_core::haul::{self, Axis, Button, Pair, Rate, Tracker, feel};

fn pair(sideways: Option<&str>, up: Option<&str>) -> Pair {
    Pair {
        sideways: sideways.map(Into::into),
        up: up.map(Into::into),
    }
}

#[test]
fn factory_table_parses_and_has_loop_slice() {
    let row = haul::row(Tool::LoopSlice).expect("Loop Slice row");
    assert_eq!(row.left.get(Axis::Sideways), Some("loop_slice_position"));
    assert_eq!(row.right.get(Axis::Sideways), Some("loop_slice_count"));
    assert_eq!(row.middle.get(Axis::Sideways), Some("loop_slice_current"));
    assert_eq!(row.middle.get(Axis::Up), Some("loop_slice_inset"));
    assert_eq!(row.right.get(Axis::Up), None);
}

#[test]
fn a_button_with_no_entry_falls_through() {
    // DL-09: Bevel lists only the right button; left keeps its own drag.
    assert!(haul::claims(Tool::Bevel, Button::Right));
    assert!(!haul::claims(Tool::Bevel, Button::Left));
    assert!(!haul::claims(Tool::Bevel, Button::Middle));
    // A tool with no row claims nothing.
    assert!(!haul::claims(Tool::Move, Button::Right));
}

#[test]
fn table_refuses_unknown_tools_duplicates_and_unknown_keys() {
    assert!(haul::parse("[[tool]]\ntool = \"NoSuchTool\"\n").is_err());
    assert!(haul::parse("[[tool]]\ntool = \"Bevel\"\n[[tool]]\ntool = \"Bevel\"\n").is_err());
    assert!(haul::parse("[[tool]]\ntool = \"Bevel\"\nright = { across = \"x\" }\n").is_err());
    let rows = haul::parse("[[tool]]\ntool = \"Bevel\"\nright = { up = \"x\" }\n").unwrap();
    assert_eq!(rows[0].0, Tool::Bevel);
    assert_eq!(rows[0].1.right.get(Axis::Up), Some("x"));
}

#[test]
fn first_clear_direction_locks_the_axis() {
    let p = pair(Some("a"), Some("b"));
    let mut t = Tracker::new([100., 100.], &p);
    // Inside the dead zone: nothing yet.
    assert_eq!(t.update([102., 99.]), None);
    assert!(!t.moved([102., 99.]));
    // Mostly sideways past the lock distance: sideways, for good.
    assert_eq!(t.update([106., 98.]), Some((Axis::Sideways, 6.)));
    // Going up a long way afterwards still reads sideways only.
    assert_eq!(t.update([110., 40.]), Some((Axis::Sideways, 10.)));
    assert_eq!(t.axis, Some(Axis::Sideways));

    let mut t = Tracker::new([0., 0.], &p);
    // Screen y grows down: moving up is positive.
    assert_eq!(t.update([1., -feel::AXIS_LOCK_PX]), Some((Axis::Up, 4.)));
    assert_eq!(t.update([50., -6.]), Some((Axis::Up, 6.)));
}

#[test]
fn a_single_value_button_takes_any_motion() {
    // Provisional DL-B01: Count is the right button's only value, so a
    // drag that starts upward still changes it, by its sideways part.
    let mut t = Tracker::new([0., 0.], &pair(Some("count"), None));
    assert_eq!(t.axis, Some(Axis::Sideways));
    // Still a click inside the dead zone: nothing changes.
    assert_eq!(t.update([2., 1.]), None);
    assert_eq!(t.update([3., -20.]), Some((Axis::Sideways, 3.)));
    // Once dragging, coming back near the start still counts.
    assert_eq!(t.update([1., 0.]), Some((Axis::Sideways, 1.)));
}

#[test]
fn the_axis_with_no_value_does_nothing() {
    // Two-value lock with only an up value missing can't happen in the
    // tracker (it locks at once); a pair with neither drives nothing.
    let mut t = Tracker::new([0., 0.], &pair(None, None));
    assert_eq!(t.update([40., 0.]), None);
}

#[test]
fn whole_numbers_step_and_smooth_values_scale() {
    let step = feel::WHOLE_STEP_PX;
    assert_eq!(haul::change(step - 1., Rate::Whole), 0.);
    assert_eq!(haul::change(step, Rate::Whole), 1.);
    assert_eq!(haul::change(2.5 * step, Rate::Whole), 2.);
    // Left is fewer (DL-06), counted toward zero.
    assert_eq!(haul::change(-step - 1., Rate::Whole), -1.);
    assert_eq!(haul::change(10., Rate::PerPixel(0.5)), 5.);
    assert_eq!(haul::change(-10., Rate::PerPixel(0.5)), -5.);
}

#[test]
fn every_table_tool_is_live_editable() {
    // Each row names a tool the editor can run.
    for (tool, row) in haul::table() {
        assert!(
            row.claims(Button::Left) || row.claims(Button::Middle) || row.claims(Button::Right),
            "{tool:?} has a row with no values"
        );
    }
}

/// A strip of three unit quads in XZ (x 0..3, z 0..1), facing +Y.
fn strip() -> operandi_core::Mesh {
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=1u32 {
        for i in 0..=3u32 {
            mesh.vertices.push(glam::DVec3::new(i as f64, 0., j as f64));
        }
    }
    for i in 0..3u32 {
        mesh.faces.push(operandi_core::Face {
            vertices: vec![i, i + 4, i + 5, i + 1],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
    }
    mesh
}

#[test]
fn loop_slice_inset_digs_the_cut_in_along_the_normal() {
    // LS-41, provisional LS-Q10: with no profile every new point is pushed
    // the same distance against the surface normal; negative pushes out.
    let mesh = strip();
    let mut o = operandi_core::LoopSlice {
        rings: vec![(0, 4)],
        ..Default::default()
    };
    let (flat, _, _) = operandi_core::slice::loop_slice_geometry(&mesh, &o, &[]).unwrap();
    let new_points = |m: &operandi_core::Mesh| m.vertices[mesh.vertices.len()..].to_vec();
    assert!(new_points(&flat).iter().all(|v| v.y == 0. && v.z == 0.5));
    o.inset = 0.2;
    let (dug, _, _) = operandi_core::slice::loop_slice_geometry(&mesh, &o, &[]).unwrap();
    let points = new_points(&dug);
    assert_eq!(points.len(), 4);
    assert!(
        points
            .iter()
            .all(|v| (v.y + 0.2).abs() < 1e-12 && v.z == 0.5),
        "{points:?}"
    );
    o.inset = -0.1;
    let (raised, _, _) = operandi_core::slice::loop_slice_geometry(&mesh, &o, &[]).unwrap();
    assert!(
        new_points(&raised)
            .iter()
            .all(|v| (v.y - 0.1).abs() < 1e-12)
    );
    o.inset = f64::NAN;
    assert!(operandi_core::slice::loop_slice_geometry(&mesh, &o, &[]).is_err());
}
