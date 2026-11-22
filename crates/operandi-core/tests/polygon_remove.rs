//! MP-C10 REM-001..004 core acceptance for basic Polygon Remove.
//!
//! Positions, faces and the selected face come from the admitted original
//! fixture `ORIGINAL_FIXTURE.json`. The regression scene adds independently
//! authored asymmetric sparse payload for every face/corner/edge/point class,
//! one pre-existing isolated point (index 6) and an unrelated tagged item.
//!
//! Expected scenes are authored directly: each face's payload is one literal
//! row, a mesh is built from an explicit list of rows in storage order, and
//! every face-indexed set, crease and remembered selection is written out per
//! case. No expectation is produced by calling the operation under test.
use glam::DVec3;
use operandi_core::*;
use std::collections::BTreeMap;

fn remove(e: &mut Editor) {
    e.dispatch(Command::RemovePolygons).unwrap();
}

/// Three faces share edge 0-1 (an admitted exact exceptional incidence), and
/// face 3 is an explicitly recorded zero-area cage face.
fn exceptional(order: &[usize], ledger: &[[u32; 3]], degenerate: &[u32]) -> Mesh {
    let cycles = [vec![0, 1, 2], vec![1, 0, 3], vec![0, 1, 4], vec![5, 6, 7]];
    Mesh {
        vertices: [
            (0., 0., 0.),
            (1., 0., 0.),
            (0.5, 0., 1.),
            (0.5, 1., 0.),
            (0.5, -1., 0.),
            (2., 0., 0.),
            (3., 0., 0.),
            (4., 0., 0.),
        ]
        .map(|(x, y, z)| DVec3::new(x, y, z))
        .to_vec(),
        faces: order
            .iter()
            .map(|&r| Face {
                vertices: cycles[r].clone(),
                uv: None,
                material: format!("m{r}"),
                smooth: false,
                part: Some(format!("row{r}")),
                material_label: None,
            })
            .collect(),
        attributes: (MeshAttributes {
            extrude_edge_uses: if ledger.is_empty() {
                BTreeMap::new()
            } else {
                BTreeMap::from([("0:1".into(), ledger.to_vec())])
            },
            degenerate_faces: degenerate.iter().copied().collect(),
            ..Default::default()
        })
        .into(),
        ..Default::default()
    }
}

fn exceptional_editor(mesh: Mesh, selected: &[u32]) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = selected.to_vec();
    e.scene.validate().unwrap();
    e
}

/// Selected slots, expected surviving rows, expected ledger uses of edge
/// 0-1, expected degenerate slots.
type LedgerCase = (
    &'static [u32],
    &'static [usize],
    &'static [[u32; 3]],
    &'static [u32],
);

#[test]
fn rem_002_exact_incidence_and_degenerate_bookkeeping_follow_the_same_map() {
    // Source: degenerate row 3 stored first, then the three edge-sharing rows.
    let source = exceptional(&[3, 0, 1, 2], &[[1, 0, 1], [2, 1, 0], [3, 0, 1]], &[0]);
    let cases: [LedgerCase; 4] = [
        // Removing the degenerate face renumbers every ledger use.
        (&[0], &[0, 1, 2], &[[0, 0, 1], [1, 1, 0], [2, 0, 1]], &[]),
        // Two uses remain with the same direction: still exceptional.
        (&[2], &[3, 0, 2], &[[1, 0, 1], [2, 0, 1]], &[0]),
        // Two opposite uses remain: an ordinary manifold edge again.
        (&[3], &[3, 0, 1], &[], &[0]),
        // One use remains: an ordinary boundary edge.
        (&[1, 2], &[3, 2], &[], &[0]),
    ];
    for (selected, rows, ledger, degenerate) in cases {
        let mut e = exceptional_editor(source.clone(), selected);
        let before = e.scene.clone();
        remove(&mut e);
        let mesh = exceptional(rows, ledger, degenerate);
        let mut want = before.clone();
        want.active_mut().unwrap().mesh = mesh.into();
        want.selection.faces.clear();
        assert_eq!(e.scene, want, "selected {selected:?}");
    }
}
