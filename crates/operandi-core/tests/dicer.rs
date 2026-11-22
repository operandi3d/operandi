//! Dicer (EDIT.md v0, provisional): JD2 seeded cuts; JD3 one undo step
//! and refusals leave the mesh as it was.
use glam::DVec3;
use operandi_core::dicer::{self, DicerOptions};
use operandi_core::*;

fn cube() -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap()
}

/// Closed pieces: every edge used exactly twice.
fn closed(m: &Mesh) -> bool {
    m.edge_counts().values().all(|&c| c == 2)
}

/// Points of each connected piece.
fn pieces(m: &Mesh) -> usize {
    let n = m.vertices.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        p[x] = r;
        r
    }
    for f in m.faces.iter() {
        for w in f.vertices.windows(2) {
            let (a, b) = (
                find(&mut parent, w[0] as usize),
                find(&mut parent, w[1] as usize),
            );
            parent[a] = b;
        }
    }
    let used = m.used_points();
    let mut roots: Vec<usize> = (0..n)
        .filter(|&v| used[v])
        .map(|v| find(&mut parent, v))
        .collect();
    roots.sort_unstable();
    roots.dedup();
    roots.len()
}

#[test]
fn jd2_dicer_is_seeded() {
    let o = DicerOptions {
        planes: 3,
        ..Default::default()
    };
    let a = dicer::dicer(&cube(), &o).unwrap();
    let b = dicer::dicer(&cube(), &o).unwrap();
    assert_eq!(a, b);
    assert!(pieces(&a) >= 2);
    assert!(closed(&a));
    let c = dicer::dicer(&cube(), &DicerOptions { seed: 7, ..o }).unwrap();
    assert_ne!(a, c);
}

#[test]
fn jd3_one_undo_step_and_refusals() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = cube().into();
    let steps = e.undo.len();
    e.dispatch(Command::Dicer(DicerOptions {
        planes: 2,
        ..Default::default()
    }))
    .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert!(pieces(&e.scene.active().unwrap().mesh) >= 2);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, cube());
    // An open mesh: refused, nothing changes.
    let mut open = cube();
    open.faces.pop();
    e.scene.active_mut().unwrap().mesh = open.clone().into();
    assert!(e.dispatch(Command::Dicer(DicerOptions::default())).is_err());
    assert_eq!(e.scene.active().unwrap().mesh, open);
}
