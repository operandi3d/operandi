//! Spec `solid-sketch` v0.3 section 11, core tests 1–11c.
use super::*;
use std::collections::BTreeMap;

fn none() -> SketchOptions {
    SketchOptions {
        subdiv: SubdivMode::None,
        ..SketchOptions::default()
    }
}

/// Every edge used once each way: closed, consistently wound.
fn closed(mesh: &Mesh) -> bool {
    let mut uses: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    for f in &mesh.faces {
        for i in 0..f.vertices.len() {
            let (a, b) = (f.vertices[i], f.vertices[(i + 1) % f.vertices.len()]);
            *uses.entry((a, b)).or_default() += 1;
        }
    }
    !uses.is_empty()
        && uses
            .iter()
            .all(|(&(a, b), &n)| n == 1 && uses.get(&(b, a)) == Some(&1))
}

/// Signed volume: positive when normals face outward.
fn volume(mesh: &Mesh) -> f64 {
    mesh.faces
        .iter()
        .map(|f| {
            let p: Vec<DVec3> = f
                .vertices
                .iter()
                .map(|&v| mesh.vertices[v as usize])
                .collect();
            (1..p.len() - 1)
                .map(|k| p[0].dot(p[k].cross(p[k + 1])) / 6.)
                .sum::<f64>()
        })
        .sum()
}

/// Connected pieces (by shared points).
fn shells(mesh: &Mesh) -> usize {
    let mut parent: Vec<usize> = (0..mesh.vertices.len()).collect();
    fn find(p: &mut [usize], mut v: usize) -> usize {
        while p[v] != v {
            p[v] = p[p[v]];
            v = p[v];
        }
        v
    }
    for f in &mesh.faces {
        for w in f.vertices.windows(2) {
            let (a, b) = (
                find(&mut parent, w[0] as usize),
                find(&mut parent, w[1] as usize),
            );
            parent[a] = b;
        }
    }
    (0..mesh.vertices.len())
        .filter(|&v| find(&mut parent, v) == v)
        .count()
}

fn chain(n: usize, spacing: f64) -> SolidSketch {
    let mut s = SolidSketch::new(DVec3::Y);
    for i in 0..n {
        s.add(DVec3::new(i as f64 * spacing, 0., 0.), CLICK_SIZE, None)
            .unwrap();
    }
    s
}

#[test]
fn one_node_is_a_box_of_six_quads() {
    let mut s = SolidSketch::new(DVec3::Y);
    s.add(DVec3::new(1., 2., 3.), CLICK_SIZE, None).unwrap();
    let m = s.cage(&none()).unwrap();
    assert_eq!(m.faces.len(), 6);
    assert!(m.faces.iter().all(|f| f.vertices.len() == 4));
    assert!(closed(&m));
    assert!((volume(&m) - 0.008).abs() < 1e-9, "{}", volume(&m));
    for p in &m.vertices {
        let d = (*p - DVec3::new(1., 2., 3.)).abs();
        assert!((d - DVec3::splat(0.1)).abs().max_element() < 1e-12);
    }
    assert!(m.attributes.representations.is_empty());
}

#[test]
fn quad_counts_match_the_loop_model() {
    for (n, quads) in [(1, 6), (2, 14), (3, 22), (4, 30), (5, 38)] {
        let m = chain(n, 1.).cage(&none()).unwrap();
        assert_eq!(m.faces.len(), quads, "{n} nodes");
        assert!(closed(&m) && volume(&m) > 0. && shells(&m) == 1);
    }
    // A branch off the middle of a 3-chain adds 8 (spec 6).
    let mut s = chain(3, 1.);
    s.add(DVec3::new(1., 0., 1.), CLICK_SIZE, Some(1)).unwrap();
    let m = s.cage(&none()).unwrap();
    assert_eq!(m.faces.len(), 30);
    assert!(closed(&m) && shells(&m) == 1 && volume(&m) > 0.);
}

#[test]
fn two_linked_nodes_make_one_closed_shell() {
    let m = chain(2, 1.).cage(&none()).unwrap();
    assert_eq!(m.faces.len(), 14);
    assert!(closed(&m));
    assert_eq!(shells(&m), 1);
    m.validate().unwrap();
}

#[test]
fn root_with_three_children_bridges_each_link() {
    let mut s = SolidSketch::new(DVec3::Y);
    s.add(DVec3::ZERO, CLICK_SIZE, None).unwrap();
    for to in [DVec3::X, DVec3::Z, -DVec3::Z] {
        s.add(to, CLICK_SIZE, Some(0)).unwrap();
    }
    let m = s.cage(&none()).unwrap();
    assert_eq!(m.faces.len(), 6 + 3 * 8);
    assert!(closed(&m) && shells(&m) == 1 && volume(&m) > 0.);
}

#[test]
fn subdiv_level1_marks_the_same_cage() {
    let s = chain(3, 1.);
    let plain = s.cage(&none()).unwrap();
    let m = s.cage(&level(SubdivMode::Level1)).unwrap();
    assert_eq!(m.vertices, plain.vertices);
    assert_eq!(m.faces, plain.faces);
    assert!(all_subdivs(&m));
    // Back to none: plain cage polygons again.
    let back = s.cage(&none()).unwrap();
    assert_eq!(back, plain);
    assert!(
        back.attributes
            .representations
            .iter()
            .all(|r| *r != PolygonRepresentation::Subdivs)
    );
}

#[test]
fn subdiv_level2_bakes_one_smoothing_step() {
    // Spec test 5: 1 node gives 24 quads, 2 linked nodes 56.
    for (n, quads) in [(1, 24), (2, 56), (3, 4 * (6 + 2 * 8))] {
        let s = chain(n, 1.);
        let l1 = s.cage(&level(SubdivMode::Level1)).unwrap();
        let l2 = s.cage(&level(SubdivMode::Level2)).unwrap();
        assert_eq!(l2.faces.len(), 4 * l1.faces.len());
        assert_eq!(l2.faces.len(), quads, "{n} nodes");
        assert!(l2.faces.iter().all(|f| f.vertices.len() == 4));
        assert!(all_subdivs(&l2));
        assert!(l2.validate().is_ok());
        assert!(closed(&l2) && shells(&l2) == 1 && volume(&l2) > 0.);
        // Smoothed, not a straight cut: rounder, never bigger than Level 1.
        let (lo1, hi1) = bounds(&l1);
        let (lo2, hi2) = bounds(&l2);
        assert!(lo2.cmpge(lo1 - 1e-9).all() && hi2.cmple(hi1 + 1e-9).all());
        assert!(volume(&l2) < volume(&l1));
    }
}

fn level(subdiv: SubdivMode) -> SketchOptions {
    SketchOptions {
        subdiv,
        ..SketchOptions::default()
    }
}

fn all_subdivs(m: &Mesh) -> bool {
    m.attributes.representations.len() == m.faces.len()
        && m.attributes
            .representations
            .iter()
            .all(|r| *r == PolygonRepresentation::Subdivs)
}

fn bounds(m: &Mesh) -> (DVec3, DVec3) {
    m.vertices.iter().fold(
        (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
        |(lo, hi), v| (lo.min(*v), hi.max(*v)),
    )
}

#[test]
fn twist_turns_the_cross_section_only() {
    let mut s = chain(3, 1.);
    let centres: Vec<_> = s.nodes.iter().map(|n| n.center).collect();
    let before = s.frames(false)[1];
    s.nodes[1].twist = std::f64::consts::FRAC_PI_2;
    let after = s.frames(false)[1];
    assert!(after.x.distance(before.x) < 1e-12);
    assert!(after.y.distance(before.z) < 1e-12);
    assert_eq!(
        s.nodes.iter().map(|n| n.center).collect::<Vec<_>>(),
        centres
    );
    assert!(closed(&s.cage(&none()).unwrap()));
}

#[test]
fn rotate_swings_the_branch_about_the_node() {
    let mut s = chain(4, 1.);
    let turn = DQuat::from_rotation_y(std::f64::consts::FRAC_PI_2);
    s.rotate(1, turn).unwrap();
    assert!(s.nodes[1].center.distance(DVec3::X) < 1e-12);
    assert!(s.nodes[0].center.distance(DVec3::ZERO) < 1e-12);
    // Child and grandchild turn by the same angle about node 1.
    assert!(s.nodes[2].center.distance(DVec3::new(1., 0., -1.)) < 1e-12);
    assert!(s.nodes[3].center.distance(DVec3::new(1., 0., -2.)) < 1e-12);
    let m = s.cage(&none()).unwrap();
    assert!(closed(&m) && shells(&m) == 1);
}

#[test]
fn deleting_a_middle_node_reconnects_its_child() {
    let mut s = chain(3, 1.);
    assert_eq!(s.delete(1).unwrap(), Some(0));
    assert_eq!(s.len(), 2);
    assert_eq!(s.nodes[1].parent, Some(0));
    let m = s.cage(&none()).unwrap();
    assert!(closed(&m) && shells(&m) == 1 && m.faces.len() == 14);
    // Deleting the root: its first child becomes the root, the others
    // hang from it.
    let mut s = SolidSketch::new(DVec3::Y);
    s.add(DVec3::ZERO, CLICK_SIZE, None).unwrap();
    s.add(DVec3::X, CLICK_SIZE, Some(0)).unwrap();
    s.add(DVec3::Z, CLICK_SIZE, Some(0)).unwrap();
    assert_eq!(s.delete(0).unwrap(), Some(0));
    assert_eq!(s.nodes[0].parent, None);
    assert_eq!(s.nodes[1].parent, Some(0));
    s.validate().unwrap();
    // The last node: nothing left, nothing current.
    let mut s = chain(1, 1.);
    assert_eq!(s.delete(0).unwrap(), None);
    assert!(s.cage(&none()).unwrap().faces.is_empty());
}

#[test]
fn deleting_a_branch_removes_its_descendants() {
    let mut s = chain(4, 1.);
    s.add(DVec3::new(0., 0., 1.), CLICK_SIZE, Some(0)).unwrap();
    assert_eq!(s.delete_branch(1).unwrap(), Some(0));
    assert_eq!(s.len(), 2);
    assert_eq!(s.nodes[1].center, DVec3::new(0., 0., 1.));
    assert_eq!(s.nodes[1].parent, Some(0));
}

#[test]
fn coincident_nodes_never_panic_or_nan() {
    for merge in [false, true] {
        for subdiv in SubdivMode::ALL {
            let mut s = SolidSketch::new(DVec3::Y);
            s.add(DVec3::ZERO, CLICK_SIZE, None).unwrap();
            s.add(DVec3::ZERO, CLICK_SIZE, None).unwrap();
            s.add(DVec3::ZERO, CLICK_SIZE, None).unwrap();
            let o = SketchOptions {
                subdiv,
                auto_merge: merge,
                ..SketchOptions::default()
            };
            let m = s.cage(&o).unwrap();
            assert!(m.vertices.iter().all(|p| p.is_finite()));
            m.validate().unwrap();
        }
    }
}

#[test]
fn random_tree_of_200_nodes_stays_closed_and_fast() {
    // Small deterministic generator, no dependency.
    let mut seed = 0x1234_5678_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % 10_000) as f64 / 10_000.
    };
    let mut s = SolidSketch::new(DVec3::Y);
    s.add(DVec3::ZERO, CLICK_SIZE, None).unwrap();
    for i in 1..200 {
        let parent = ((next() * i as f64) as usize).min(i - 1);
        let from = s.nodes[parent].center;
        let offset = DVec3::new(next() - 0.5, next() - 0.5, next() - 0.5) * 2.;
        s.add(from + offset, 0.05 + next() * 0.1, Some(parent))
            .unwrap();
    }
    for merge in [false, true] {
        let o = SketchOptions {
            auto_merge: merge,
            auto_align: merge,
            ..none()
        };
        let start = std::time::Instant::now();
        let m = s.cage(&o).unwrap();
        let took = start.elapsed();
        assert!(closed(&m));
        assert!(m.vertices.iter().all(|p| p.is_finite()));
        // Generous for debug builds; release is far below a frame.
        assert!(took.as_millis() < 500, "{took:?}");
    }
}

#[test]
fn ctrl_insert_renumbers_later_nodes() {
    let mut s = chain(3, 1.);
    let at = s.link_point(2, DVec3::new(1.5, 0.3, 0.)).unwrap();
    assert!(at.distance(DVec3::new(1.5, 0., 0.)) < 1e-12);
    assert_eq!(s.insert(2, at, CLICK_SIZE).unwrap(), 2);
    assert_eq!(s.len(), 4);
    assert_eq!(s.nodes[2].parent, Some(1));
    assert_eq!(s.nodes[3].parent, Some(2));
    assert_eq!(s.nodes[3].center, DVec3::new(2., 0., 0.));
    assert_eq!(s.cage(&none()).unwrap().faces.len(), 30);
}

#[test]
fn auto_merge_drops_a_loop_where_boxes_overlap() {
    // Three big nodes close together: 22 quads apart, 18 merged (spec 6).
    let mut s = SolidSketch::new(DVec3::Y);
    s.add(DVec3::ZERO, 0.3, None).unwrap();
    s.add(DVec3::new(0.5, 0., 0.), 0.3, None).unwrap();
    s.add(DVec3::new(1.5, 0., 0.), 0.3, None).unwrap();
    assert_eq!(s.cage(&none()).unwrap().faces.len(), 22);
    let merged = s
        .cage(&SketchOptions {
            auto_merge: true,
            ..none()
        })
        .unwrap();
    assert_eq!(merged.faces.len(), 18);
    assert!(closed(&merged) && shells(&merged) == 1 && volume(&merged) > 0.);
    // Well spaced: nothing to merge.
    let spaced = chain(4, 1.).cage(&SketchOptions {
        auto_merge: true,
        ..none()
    });
    assert_eq!(spaced.unwrap().faces.len(), 30);
}

#[test]
fn sharp_u_turn_does_not_turn_inside_out() {
    let mut s = SolidSketch::new(DVec3::Y);
    s.add(DVec3::ZERO, CLICK_SIZE, None).unwrap();
    s.add(DVec3::X, CLICK_SIZE, None).unwrap();
    s.add(DVec3::new(0.1, 0., 0.6), CLICK_SIZE, None).unwrap();
    let m = s.cage(&none()).unwrap();
    assert!(closed(&m) && volume(&m) > 0. && shells(&m) == 1);
}

#[test]
fn more_links_than_faces_gives_separate_boxes() {
    let mut s = SolidSketch::new(DVec3::Y);
    s.add(DVec3::ZERO, CLICK_SIZE, None).unwrap();
    for k in 0..8 {
        let a = k as f64 * std::f64::consts::TAU / 8.;
        s.add(DVec3::new(a.cos(), 0., a.sin()), CLICK_SIZE, Some(0))
            .unwrap();
    }
    let m = s.cage(&none()).unwrap();
    assert!(closed(&m));
    m.validate().unwrap();
}

#[test]
fn scale_branch_grows_sizes_and_spacing() {
    let mut s = chain(3, 1.);
    s.scale(1, 2., true).unwrap();
    assert_eq!(s.nodes[0].size, DVec3::splat(0.1));
    assert_eq!(s.nodes[1].size, DVec3::splat(0.2));
    assert!(s.nodes[2].center.distance(DVec3::new(3., 0., 0.)) < 1e-12);
    s.scale(2, 0.5, false).unwrap();
    assert!(s.nodes[2].center.distance(DVec3::new(3., 0., 0.)) < 1e-12);
    assert_eq!(s.nodes[2].size, DVec3::splat(0.1));
}

#[test]
fn branch_move_carries_descendants() {
    let mut s = chain(3, 1.);
    s.translate(1, DVec3::Z, true).unwrap();
    assert_eq!(s.nodes[0].center, DVec3::ZERO);
    assert_eq!(s.nodes[2].center, DVec3::new(2., 0., 1.));
    s.translate(2, DVec3::Z, false).unwrap();
    assert_eq!(s.nodes[1].center, DVec3::new(1., 0., 1.));
}
