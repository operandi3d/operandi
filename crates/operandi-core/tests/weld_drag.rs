//! Weld drag (WD-01..05): merge at the destination, move on an empty
//! release, degenerate polygon removal and one undo step per drag.
use glam::DVec3;
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// 3×3 quads in z = 0 facing +z; point `r * 4 + c` sits at (c, r, 0).
fn grid() -> Mesh {
    let mut m = Mesh {
        vertices: (0..16)
            .map(|i| DVec3::new((i % 4) as f64, (i / 4) as f64, 0.))
            .collect(),
        ..Default::default()
    };
    for r in 0..3 {
        for c in 0..3 {
            let i = r * 4 + c;
            m.faces.push(face(vec![i, i + 1, i + 5, i + 4]));
        }
    }
    m.validate().unwrap();
    m
}

fn near(a: DVec3, b: DVec3) -> bool {
    a.distance(b) < 1e-9
}

fn editor(m: Mesh, points: Vec<u32>) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = points;
    e
}

#[test]
fn merge_lands_on_the_destination_and_the_dragged_point_disappears() {
    // WD-03: drag point 5 (1,1) onto point 6 (2,1).
    let (out, remap) = weld_drag_geometry(&grid(), 5, WeldDrop::Point(6)).unwrap();
    assert_eq!(out.vertices.len(), 15);
    assert_eq!(remap[5], None, "the dragged point is gone");
    let kept = remap[6].unwrap();
    assert!(near(out.vertices[kept as usize], DVec3::new(2., 1., 0.)));
    assert!(
        !out.vertices
            .iter()
            .any(|&p| near(p, DVec3::new(1., 1., 0.))),
        "nothing remains at the dragged point's position"
    );
    // The two quads sharing edge 5–6 lose it and become triangles; the
    // other polygons that used 5 now use the destination.
    assert_eq!(out.faces.len(), 9);
    let triangles = out.faces.iter().filter(|f| f.vertices.len() == 3).count();
    assert_eq!(triangles, 2);
    let using_kept = out
        .faces
        .iter()
        .filter(|f| f.vertices.contains(&kept))
        .count();
    assert_eq!(using_kept, 6, "four of 6's polygons plus two of 5's");
    assert!(!out.edges().contains(&(kept, kept)));
    out.validate().unwrap();
}

#[test]
fn merging_keeps_the_destination_point_attributes() {
    let mut m = grid();
    m.attributes
        .point_weights
        .entry("W".into())
        .or_default()
        .insert(6, 0.75);
    m.attributes
        .point_weights
        .get_mut("W")
        .unwrap()
        .insert(5, 0.25);
    let (out, remap) = weld_drag_geometry(&m, 5, WeldDrop::Point(6)).unwrap();
    let kept = remap[6].unwrap();
    assert_eq!(out.attributes.point_weights["W"].get(&kept), Some(&0.75));
}

#[test]
fn polygons_left_with_fewer_than_three_corners_are_removed() {
    // A triangle 0-1-2 beside a quad 1-3-4-2: welding 0 onto 1 leaves the
    // triangle with two corners, so it goes; the quad stays.
    let p = |x: f64, y: f64| DVec3::new(x, y, 0.);
    let m = Mesh {
        vertices: vec![p(0., 0.), p(1., 0.), p(1., 1.), p(2., 0.), p(2., 1.)],
        faces: vec![face(vec![0, 1, 2]), face(vec![1, 3, 4, 2])].into(),
        ..Default::default()
    };
    m.validate().unwrap();
    let (out, remap) = weld_drag_geometry(&m, 0, WeldDrop::Point(1)).unwrap();
    assert_eq!(out.faces.len(), 1);
    assert_eq!(out.faces[0].vertices.len(), 4);
    assert_eq!(out.vertices.len(), 4);
    assert!(near(out.vertices[remap[1].unwrap() as usize], p(1., 0.)));
}

#[test]
fn an_empty_release_only_moves_the_point() {
    // WD-05.
    let to = DVec3::new(1.4, 1.3, 0.);
    let (out, remap) = weld_drag_geometry(&grid(), 5, WeldDrop::Position(to)).unwrap();
    assert_eq!(out.vertices.len(), 16);
    assert!(near(out.vertices[5], to));
    assert_eq!(out.faces, grid().faces);
    assert!(remap.iter().enumerate().all(|(i, &r)| r == Some(i as u32)));
}

#[test]
fn bad_drops_are_refused() {
    assert!(weld_drag_geometry(&grid(), 5, WeldDrop::Point(5)).is_err());
    assert!(weld_drag_geometry(&grid(), 5, WeldDrop::Point(99)).is_err());
    assert!(weld_drag_geometry(&grid(), 99, WeldDrop::Point(5)).is_err());
    assert!(weld_drag_geometry(&grid(), 5, WeldDrop::Position(DVec3::NAN)).is_err());
}

#[test]
fn each_drag_is_one_undo_step_and_keeps_the_selection() {
    // WD-04: two drags, two steps; undo/redo walk them one at a time.
    let mut e = editor(grid(), vec![0, 5, 15]);
    let start = e.scene.clone();
    e.dispatch(Command::WeldDrag {
        source: 5,
        drop: WeldDrop::Point(6),
    })
    .unwrap();
    assert_eq!(e.undo.len(), 1);
    assert_eq!(e.undo.last().unwrap().label, "Weld");
    // The merged-away point leaves the selection; the others are remapped.
    assert_eq!(e.scene.selection.vertices, vec![0, 14]);
    let merged = e.scene.clone();
    e.dispatch(Command::WeldDrag {
        source: 0,
        drop: WeldDrop::Position(DVec3::new(-0.5, 0., 0.)),
    })
    .unwrap();
    assert_eq!(e.undo.len(), 2);
    assert_eq!(
        e.scene.selection.vertices,
        vec![0, 14],
        "selection unchanged"
    );
    assert!(near(
        e.scene.active().unwrap().mesh.vertices[0],
        DVec3::new(-0.5, 0., 0.)
    ));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh,
        merged.active().unwrap().mesh
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, start.active().unwrap().mesh);
    assert_eq!(e.scene.selection.vertices, vec![0, 5, 15]);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh,
        merged.active().unwrap().mesh
    );
}

#[test]
fn a_refused_drag_changes_nothing() {
    let mut e = editor(grid(), vec![5]);
    let before = e.scene.clone();
    assert!(
        e.dispatch(Command::WeldDrag {
            source: 5,
            drop: WeldDrop::Point(5),
        })
        .is_err()
    );
    assert_eq!(e.scene, before);
    assert!(e.undo.is_empty());
}

#[test]
fn an_empty_release_takes_a_world_position_through_the_item_transform() {
    let mut e = editor(grid(), vec![]);
    e.scene.active_mut().unwrap().transform.translation = DVec3::new(10., 0., 0.);
    e.dispatch(Command::WeldDrag {
        source: 5,
        drop: WeldDrop::Position(DVec3::new(11.5, 1., 0.)),
    })
    .unwrap();
    assert!(near(
        e.scene.active().unwrap().mesh.vertices[5],
        DVec3::new(1.5, 1., 0.)
    ));
}

#[test]
fn a_merged_scene_saves_and_reopens() {
    let mut e = editor(grid(), vec![]);
    e.dispatch(Command::WeldDrag {
        source: 5,
        drop: WeldDrop::Point(6),
    })
    .unwrap();
    let mesh = e.scene.active().unwrap().mesh.clone();
    let path = std::env::temp_dir().join(format!("weld-drag-{}.opscene", std::process::id()));
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(back.active().unwrap().mesh, mesh);
}
