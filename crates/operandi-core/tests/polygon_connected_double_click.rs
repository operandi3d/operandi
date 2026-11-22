//! Polygon-mode double-click: `Command::SelectConnectedPolygons` selects
//! every polygon connected to the clicked one (Shift adds the piece), one
//! "Select Connected" undo step.
use glam::DVec3;
use operandi_core::*;

fn quad(a: u32) -> Face {
    Face {
        vertices: vec![a, a + 1, a + 4, a + 3],
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// Two pieces in one mesh, each a strip of two quads (points 3 wide, 2
/// tall). Piece A is faces 0, 1; piece B is faces 2, 3. Piece
/// B's first points sit exactly on piece A's last ones, so connectivity
/// must come from shared points, not positions.
fn two_pieces() -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    for base in [0., 2.] {
        for y in 0..2 {
            for x in 0..3 {
                mesh.vertices
                    .push(DVec3::new(base + x as f64, y as f64, 0.));
            }
        }
    }
    mesh.faces = vec![quad(0), quad(1), quad(6), quad(7)].into();
    e.scene.selection.mode = Mode::Polygon;
    let item = e.scene.selection.item.unwrap();
    (e, item)
}

#[test]
fn double_click_selects_the_piece_and_replaces_the_selection() {
    let (mut e, item) = two_pieces();
    e.scene.selection.faces = vec![3];
    let history = e.undo.len();
    e.dispatch(Command::SelectConnectedPolygons {
        item,
        face: 1,
        add: false,
    })
    .unwrap();
    assert_eq!(e.scene.selection.faces, vec![1, 0], "clicked polygon first");
    assert_eq!(e.undo.len(), history + 1, "one Select Connected step");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection.faces, vec![3]);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.selection.faces, vec![1, 0]);
}

#[test]
fn shift_double_click_adds_only_the_clicked_piece() {
    let (mut e, item) = two_pieces();
    // Piece A partly selected; the first Shift press already added face 2.
    e.scene.selection.faces = vec![0, 2];
    assert_eq!(e.scene.select_connected_polygons(item, 2, true).unwrap(), 2);
    assert_eq!(
        e.scene.selection.faces,
        vec![0, 2, 3],
        "piece A is not grown, piece B added after the kept selection"
    );
    // Adding a piece that's already selected changes nothing.
    let before = e.scene.selection.clone();
    e.scene.select_connected_polygons(item, 3, true).unwrap();
    assert_eq!(e.scene.selection, before);
}

#[test]
fn a_lone_polygon_selects_just_itself() {
    let (mut e, item) = two_pieces();
    e.scene.active_mut().unwrap().mesh.faces.truncate(1);
    assert_eq!(
        e.scene.select_connected_polygons(item, 0, false).unwrap(),
        1
    );
    assert_eq!(e.scene.selection.faces, vec![0]);
}

#[test]
fn invalid_polygons_and_other_modes_are_refused_without_changes() {
    let (mut e, item) = two_pieces();
    e.scene.selection.faces = vec![3];
    let before = e.scene.clone();
    let history = e.undo.len();
    assert!(
        e.dispatch(Command::SelectConnectedPolygons {
            item,
            face: 99,
            add: false,
        })
        .is_err()
    );
    assert!(
        e.dispatch(Command::SelectConnectedPolygons {
            item: 9_999,
            face: 0,
            add: false,
        })
        .is_err()
    );
    e.scene.selection.mode = Mode::Edge;
    assert!(
        e.dispatch(Command::SelectConnectedPolygons {
            item,
            face: 0,
            add: false,
        })
        .is_err()
    );
    e.scene.selection.mode = Mode::Polygon;
    assert_eq!(e.scene, before);
    assert_eq!(e.undo.len(), history);
}

#[test]
fn survives_save_and_reopen() {
    let (mut e, item) = two_pieces();
    e.dispatch(Command::SelectConnectedPolygons {
        item,
        face: 3,
        add: false,
    })
    .unwrap();
    let mut faces = e.scene.selection.faces.clone();
    faces.sort();
    assert_eq!(faces, vec![2, 3]);
    let path =
        std::env::temp_dir().join(format!("operandi-connected-{}.opscene", std::process::id()));
    persistence::save(&path, &e.scene).unwrap();
    let reopened = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).ok();
    let mut faces = reopened.selection.faces.clone();
    faces.sort();
    assert_eq!(faces, vec![2, 3]);
}
