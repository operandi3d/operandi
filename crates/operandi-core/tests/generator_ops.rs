//! Generators and on-hold ops (SCATTER-UVRECT-TRANSFER-CONSTRAINT v1,
//! GO-xx): acceptance criteria from GENERATORS_OPS_ACCEPTANCE.md.
//!
//! SC1–SC5 Scatter Clone's generator; SO1–SO4 the Scatter op; CS1–CS4
//! Surface Constraint; UR1–UR5 UV Rectangle; TV2, TV4 Transfer Vertex Map;
//! RO1 UnSubdivide and UV Fit ops.
use glam::{DMat4, DVec2, DVec3};
use operandi_core::generator_ops::{
    self, RectangleOptions, ScatterBox, TransferSampling, TransferType,
};
use operandi_core::meshop_types::{Linked, OpContext, Source, op_def};
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "M".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn run(id: &str, input: &Mesh, values: serde_json::Value, links: &[Linked]) -> Mesh {
    let def = op_def(id).unwrap_or_else(|| panic!("{id}"));
    let mut v = def.defaults();
    if let (Some(v), Some(over)) = (v.as_object_mut(), values.as_object()) {
        for (k, x) in over {
            v.insert(k.clone(), x.clone());
        }
    }
    let ctx = OpContext {
        input,
        op: 7,
        selection: None,
        element: Default::default(),
        links,
        time: Default::default(),
        notes: Default::default(),
        falloff: None,
    };
    def.run(&ctx, &v).unwrap()
}

fn cube(size: f64) -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::splat(size), 0).unwrap()
}

fn centres(m: &Mesh) -> Vec<DVec3> {
    m.vertices
        .chunks(8)
        .map(|c| c.iter().sum::<DVec3>() / 8.)
        .collect()
}

#[test]
fn sc1_sc2_sc3_scatter_box_counts_the_source_and_stays_in_range() {
    let d = ScatterBox::default();
    assert_eq!(d.count, 8);
    assert_eq!(d.range, DVec3::ONE);
    let s = ScatterBox {
        count: 5,
        range: DVec3::new(1., 0., 1.),
        ..Default::default()
    };
    let t = s.transforms(DVec3::ZERO, false).unwrap();
    assert_eq!(t.len(), 4, "the original plus 4 copies");
    for m in &t {
        let c = m.transform_point3(DVec3::ZERO);
        assert_eq!(c.y, 0.);
        assert!(c.x.abs() <= 1. && c.z.abs() <= 1.);
    }
    let moved = ScatterBox {
        centre: DVec3::new(2., 0., 0.),
        ..s
    };
    for m in moved.transforms(DVec3::ZERO, false).unwrap() {
        let c = m.transform_point3(DVec3::ZERO);
        assert!((1. ..=3.).contains(&c.x), "{c}");
    }
}

#[test]
fn sc4_sc5_seed_and_replace() {
    let s = ScatterBox::default();
    assert_eq!(
        s.transforms(DVec3::ZERO, false).unwrap(),
        s.transforms(DVec3::ZERO, false).unwrap()
    );
    let other = ScatterBox { seed: 3, ..s };
    assert_ne!(
        s.transforms(DVec3::ZERO, false).unwrap(),
        other.transforms(DVec3::ZERO, false).unwrap()
    );
    let replaced = s.transforms(DVec3::new(5., 0., 0.), true).unwrap();
    assert_eq!(replaced.len(), 8);
    assert!(
        replaced[0]
            .transform_point3(DVec3::new(5., 0., 0.))
            .distance(DVec3::ZERO)
            < 1e-12
    );
}

#[test]
fn so1_so2_scatter_op_defaults_and_region() {
    let def = op_def("scatter.clone.item").unwrap();
    let d = def.defaults();
    assert_eq!(d["number"], 1);
    assert_eq!(d["type"], 0);
    assert_eq!(d["axis"], 1);
    assert_eq!(d["sizeX"], 1.0);
    // One copy at default changes nothing.
    assert_eq!(
        run("scatter.clone.item", &cube(0.1), serde_json::json!({}), &[]),
        cube(0.1)
    );
    let out = run(
        "scatter.clone.item",
        &cube(0.1),
        serde_json::json!({ "number": 9 }),
        &[],
    );
    let c = centres(&out);
    assert_eq!(c.len(), 9);
    for p in &c[1..] {
        assert!(
            p.y.abs() < 1e-12 && (0. ..=1.).contains(&p.x) && (0. ..=1.).contains(&p.z),
            "{p}"
        );
    }
    let out = run(
        "scatter.clone.item",
        &cube(0.1),
        serde_json::json!({ "number": 9, "type": 1 }),
        &[],
    );
    for p in &centres(&out)[1..] {
        assert!(p.min_element() >= 0. && p.max_element() <= 1., "{p}");
    }
}

#[test]
fn so3_so4_even_spread_deterministic_and_replace() {
    let pts = generator_ops::even_points(9, true, 0);
    for (i, a) in pts.iter().enumerate() {
        for b in &pts[i + 1..] {
            assert!(a.distance(*b) > 0.15, "{a} {b}");
        }
    }
    assert_eq!(pts, generator_ops::even_points(9, true, 0));
    let out = run(
        "scatter.clone.item",
        &cube(0.1),
        serde_json::json!({ "number": 3, "replace": true }),
        &[],
    );
    let c = centres(&out);
    assert_eq!(c.len(), 3);
    assert!(
        c.iter()
            .any(|p| p.distance(DVec3::new(0.5, 0., 0.5)) < 1e-9),
        "{c:?}"
    );
    assert!(!c.iter().any(|p| p.length() < 1e-9), "the source moved");
}

/// A 1 × 1 grid of points standing in XY at y −0.5, 0, 0.5 (a 2 × 2 quad
/// strip), and a big floor quad at `y`.
fn wall() -> Mesh {
    let mut m = Mesh::default();
    for y in [-0.5, 0., 0.5] {
        for x in [-0.5, 0.5] {
            m.vertices.push(DVec3::new(x, y, 0.));
        }
    }
    m.faces.push(face(vec![0, 1, 3, 2]));
    m.faces.push(face(vec![2, 3, 5, 4]));
    m.validate().unwrap();
    m
}
fn floor(y: f64) -> Mesh {
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(-5., y, -5.),
            DVec3::new(-5., y, 5.),
            DVec3::new(5., y, 5.),
            DVec3::new(5., y, -5.),
        ],
        ..Default::default()
    };
    m.faces.push(face(vec![0, 1, 2, 3]));
    m
}
fn link(m: Mesh) -> Linked {
    Linked {
        item: 2,
        source: Ok(Source {
            item: 2,
            name: "Floor".into(),
            mesh: Shared::new(m),
            to_local: DMat4::IDENTITY,
            world: DMat4::IDENTITY,
            rest: DMat4::IDENTITY,
        }),
    }
}
fn ys(m: &Mesh) -> Vec<f64> {
    m.vertices
        .iter()
        .map(|v| (v.y * 1e9).round() / 1e9)
        .collect()
}

#[test]
fn cs1_cs2_directional_invert_and_closest() {
    let def = op_def("surfaceConstraint.meshop.item").unwrap();
    let d = def.defaults();
    assert_eq!(
        (d["mode"].clone(), d["axis"].clone(), d["strength"].clone()),
        (0.into(), 1.into(), 1.0.into())
    );
    let l = [link(floor(-1.))];
    let same = run(
        "surfaceConstraint.meshop.item",
        &wall(),
        serde_json::json!({}),
        &l,
    );
    assert_eq!(same, wall(), "the floor is below: nothing moves");
    let down = run(
        "surfaceConstraint.meshop.item",
        &wall(),
        serde_json::json!({ "invert": true }),
        &l,
    );
    assert!(ys(&down).iter().all(|&y| y == -1.));
    let near = run(
        "surfaceConstraint.meshop.item",
        &wall(),
        serde_json::json!({ "mode": 2 }),
        &l,
    );
    assert!(ys(&near).iter().all(|&y| y == -1.));
    // No link: unchanged.
    assert_eq!(
        run(
            "surfaceConstraint.meshop.item",
            &wall(),
            serde_json::json!({ "invert": true }),
            &[]
        ),
        wall()
    );
}

#[test]
fn cs3_offset_strength_and_max_distance() {
    let l = [link(floor(-1.))];
    let off = run(
        "surfaceConstraint.meshop.item",
        &wall(),
        serde_json::json!({ "invert": true, "offset": 0.1 }),
        &l,
    );
    assert!(
        ys(&off).iter().all(|&y| (y + 0.9).abs() < 1e-9),
        "{:?}",
        ys(&off)
    );
    let half = run(
        "surfaceConstraint.meshop.item",
        &wall(),
        serde_json::json!({ "invert": true, "strength": 0.5 }),
        &l,
    );
    assert_eq!(ys(&half), vec![-0.75, -0.75, -0.5, -0.5, -0.25, -0.25]);
    let near = run(
        "surfaceConstraint.meshop.item",
        &wall(),
        serde_json::json!({ "invert": true, "maxDistance": 0.8 }),
        &l,
    );
    assert_eq!(ys(&near), vec![-1., -1., 0., 0., 0.5, 0.5]);
}

/// A `cols` × `rows` quad grid of 1 m cells in XZ with wobbly UVs.
fn uv_grid(cols: u32, rows: u32) -> Mesh {
    let mut m = Mesh::default();
    for j in 0..=rows {
        for i in 0..=cols {
            m.vertices.push(DVec3::new(i as f64, 0., j as f64));
        }
    }
    let at = |i: u32, j: u32| j * (cols + 1) + i;
    let mut uv = Vec::new();
    for j in 0..rows {
        for i in 0..cols {
            m.faces.push(face(vec![
                at(i, j),
                at(i + 1, j),
                at(i + 1, j + 1),
                at(i, j + 1),
            ]));
            let w = |i: u32, j: u32| {
                let k = (i * 7 + j * 13) as f64;
                Some(DVec2::new(
                    i as f64 * 0.1 + 0.01 * k.sin(),
                    j as f64 * 0.1 + 0.05 + 0.01 * k.cos(),
                ))
            };
            uv.push(vec![w(i, j), w(i + 1, j), w(i + 1, j + 1), w(i, j + 1)]);
        }
    }
    m.uv_maps.insert("Texture".into(), uv);
    m.validate().unwrap();
    m
}

fn cell_sizes(m: &Mesh) -> Vec<(f64, f64)> {
    let rows = &m.uv_maps["Texture"];
    rows.iter()
        .map(|r| {
            let p: Vec<DVec2> = r.iter().map(|c| c.unwrap()).collect();
            (p[0].distance(p[1]), p[1].distance(p[2]))
        })
        .collect()
}

#[test]
fn ur1_a_wobbly_grid_becomes_equal_cells_keeping_shape_corner_and_area() {
    let m = uv_grid(3, 2);
    let r = generator_ops::uv_rectangle(&m, &[], &RectangleOptions::default()).unwrap();
    assert_eq!((r.straightened, r.skipped), (1, 0));
    let sizes = cell_sizes(&r.mesh);
    let s0 = sizes[0].0;
    for (w, h) in &sizes {
        assert!((w - s0).abs() < 1e-9 && (h - s0).abs() < 1e-9, "{sizes:?}");
    }
    // The first corner stays.
    assert_eq!(r.mesh.uv_maps["Texture"][0][0], m.uv_maps["Texture"][0][0]);
    // Every cell is a right-angled rectangle.
    for row in &r.mesh.uv_maps["Texture"] {
        let p: Vec<DVec2> = row.iter().map(|c| c.unwrap()).collect();
        assert!((p[1] - p[0]).dot(p[2] - p[1]).abs() < 1e-9);
    }
}

#[test]
fn ur2_spans_follow_3d_lengths_unless_uniform() {
    let mut m = uv_grid(2, 1);
    // Make the second column twice as wide in 3D.
    for v in &mut m.vertices {
        if v.x > 1.5 {
            v.x = 3.;
        }
    }
    let r = generator_ops::uv_rectangle(&m, &[], &RectangleOptions::default()).unwrap();
    let s = cell_sizes(&r.mesh);
    assert!((s[1].0 / s[0].0 - 2.).abs() < 1e-9, "{s:?}");
    let u = generator_ops::uv_rectangle(
        &m,
        &[],
        &RectangleOptions {
            uniform_x: true,
            ..Default::default()
        },
    )
    .unwrap();
    let s = cell_sizes(&u.mesh);
    assert!((s[1].0 - s[0].0).abs() < 1e-9, "{s:?}");
}

#[test]
fn ur3_pack_fits_the_unit_square() {
    let m = uv_grid(3, 2);
    let r = generator_ops::uv_rectangle(
        &m,
        &[],
        &RectangleOptions {
            pack: true,
            ..Default::default()
        },
    )
    .unwrap();
    for row in &r.mesh.uv_maps["Texture"] {
        for c in row {
            let p = c.unwrap();
            assert!(
                p.min_element() >= -1e-9 && p.max_element() <= 1. + 1e-9,
                "{p}"
            );
        }
    }
}

#[test]
fn ur4_ur5_non_grids_stay_and_area_is_kept() {
    let mut m = uv_grid(2, 1);
    // A triangle island apart from the grid.
    let n = m.vertices.len() as u32;
    m.vertices.extend([
        DVec3::new(9., 0., 0.),
        DVec3::new(10., 0., 0.),
        DVec3::new(9., 0., 1.),
    ]);
    m.faces.push(face(vec![n, n + 1, n + 2]));
    let tri_uv = vec![
        Some(DVec2::new(0.8, 0.8)),
        Some(DVec2::new(0.9, 0.8)),
        Some(DVec2::new(0.8, 0.9)),
    ];
    m.uv_maps.get_mut("Texture").unwrap().push(tri_uv.clone());
    m.validate().unwrap();
    let r = generator_ops::uv_rectangle(&m, &[], &RectangleOptions::default()).unwrap();
    assert_eq!((r.straightened, r.skipped), (1, 1));
    assert_eq!(r.mesh.uv_maps["Texture"][2], tri_uv);
    let area = |rows: &[Vec<Option<DVec2>>]| -> f64 {
        rows[..2]
            .iter()
            .map(|r| {
                let p: Vec<DVec2> = r.iter().map(|c| c.unwrap()).collect();
                (0..4)
                    .map(|i| p[i].perp_dot(p[(i + 1) % 4]))
                    .sum::<f64>()
                    .abs()
                    / 2.
            })
            .sum()
    };
    assert!((area(&r.mesh.uv_maps["Texture"]) - area(&m.uv_maps["Texture"])).abs() < 1e-9);
    let d = op_def("uvrect.meshop.item").unwrap().defaults();
    assert_eq!(d["name"], "Texture");
    assert_eq!(d["gaps"], 0.2);
    assert_eq!(d["pack"], false);
}

#[test]
fn tv4_identical_meshes_copy_values_exactly() {
    let mut source = uv_grid(2, 2);
    source.attributes.point_weights.insert(
        "Weight".into(),
        (0..9).map(|v| (v, v as f64 / 8.)).collect(),
    );
    let mut target = uv_grid(2, 2);
    let points: Vec<u32> = (0..9).collect();
    let n = generator_ops::transfer_vertex_map(
        &mut target,
        &points,
        &source,
        TransferType::Weight,
        "Weight",
        TransferSampling::Distance,
    )
    .unwrap();
    assert_eq!(n, 9);
    for v in 0..9u32 {
        let got = target.attributes.point_weights["Weight"][&v];
        assert!((got - v as f64 / 8.).abs() < 1e-6);
    }
    // Position: points move onto the source's surface.
    let mut lifted = uv_grid(2, 2);
    for v in &mut lifted.vertices {
        v.y += 0.3;
    }
    generator_ops::transfer_vertex_map(
        &mut lifted,
        &points,
        &source,
        TransferType::Position,
        "",
        TransferSampling::Distance,
    )
    .unwrap();
    assert!(lifted.vertices.iter().all(|v| v.y.abs() < 1e-9));
    // A missing source map is refused.
    assert!(
        generator_ops::transfer_vertex_map(
            &mut lifted,
            &points,
            &source,
            TransferType::Weight,
            "Nope",
            TransferSampling::Distance,
        )
        .is_err()
    );
}

#[test]
fn ro1_the_uv_fit_op_matches_its_command() {
    let m = uv_grid(2, 2);
    let fitted = run("uvfit.meshop.item", &m, serde_json::json!({}), &[]);
    for row in &fitted.uv_maps["Texture"] {
        for c in row {
            let p = c.unwrap();
            assert!(p.min_element() >= -1e-9 && p.max_element() <= 1. + 1e-9);
        }
    }
    // The fitted block touches both ends of the tile on its longer side.
    let all: Vec<DVec2> = fitted.uv_maps["Texture"]
        .iter()
        .flatten()
        .map(|c| c.unwrap())
        .collect();
    let hi = all.iter().fold(DVec2::splat(f64::MIN), |a, p| a.max(*p));
    assert!((hi.max_element() - 1.).abs() < 1e-9, "{hi}");
}

fn editor(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}

#[test]
fn sc_the_scatter_tool_makes_its_copies_in_one_undo_step() {
    let mut e = editor(cube(0.1), Mode::Polygon);
    let steps = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::ScatterClone)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    assert_eq!(p.duplicate.scatter.generator.count, 8);
    p.duplicate.placed = true;
    p.duplicate.scatter.generator.count = 5;
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
    e.dispatch(Command::Drop).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(m.vertices.len(), 5 * 8);
    for c in centres(m) {
        assert!(c.abs().max_element() <= 1. + 1e-9, "{c}");
    }
    assert_eq!(e.history.undo.len(), steps + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices.len(), 8);
}

#[test]
fn ur_the_uv_rectangle_command_is_one_step_and_refuses_non_grids() {
    let mut e = editor(uv_grid(3, 2), Mode::Polygon);
    e.dispatch(Command::UvRectangle(RectangleOptions::default()))
        .unwrap();
    let sizes = cell_sizes(&e.scene.active().unwrap().mesh);
    assert!(sizes.iter().all(|s| (s.0 - sizes[0].0).abs() < 1e-9));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, uv_grid(3, 2));
    let mut tri = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X, DVec3::Z],
        ..Default::default()
    };
    tri.faces.push(face(vec![0, 1, 2]));
    tri.uv_maps
        .insert("Texture".into(), vec![vec![Some(DVec2::ZERO); 3]]);
    let mut e = editor(tri, Mode::Polygon);
    assert!(
        e.dispatch(Command::UvRectangle(RectangleOptions::default()))
            .is_err()
    );
}

#[test]
fn tv_the_transfer_command_reads_the_background_mesh() {
    let mut e = editor(uv_grid(2, 2), Mode::Vertex);
    // A second, lower copy as the background (inactive) mesh.
    e.dispatch(Command::NewMesh).unwrap();
    let mut floor = uv_grid(2, 2);
    for v in &mut floor.vertices {
        v.y -= 0.5;
    }
    e.scene.active_mut().unwrap().mesh = floor.into();
    let target = e.scene.items[0].id;
    e.dispatch(Command::SelectItem(Some(target))).unwrap();
    assert_eq!(e.scene.active().map(|i| i.id), Some(target));
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.dispatch(Command::TransferVertexMap {
        map: String::new(),
        kind: TransferType::Position,
        world: true,
        sampling: TransferSampling::Distance,
    })
    .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert!(m.vertices.iter().all(|v| (v.y + 0.5).abs() < 1e-9));
}

/// GO-32..34 (probe 2026-11-17): a 1 m source quad with W = x; target
/// points at x = 0.05, 0.5, 0.95 and one past the edge at 1.2.
fn go_source() -> Mesh {
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 0., 1.),
            DVec3::new(0., 0., 1.),
        ],
        ..Default::default()
    };
    m.faces.push(face(vec![0, 3, 2, 1]));
    m.attributes.point_weights.insert(
        "W".into(),
        [(0, 0.), (1, 1.), (2, 1.), (3, 0.)].into_iter().collect(),
    );
    m
}
fn go_target() -> Mesh {
    // A thin strip through the four sample points, so each has a normal.
    let xs = [0.05, 0.5, 0.95, 1.2];
    let mut m = Mesh::default();
    for &x in &xs {
        m.vertices.push(DVec3::new(x, 0., 0.5));
        m.vertices.push(DVec3::new(x, 0., 0.6));
    }
    for i in 0..3u32 {
        m.faces
            .push(face(vec![2 * i, 2 * i + 1, 2 * i + 3, 2 * i + 2]));
    }
    m.attributes
        .point_weights
        .insert("W".into(), Default::default());
    m
}

#[test]
fn go32_go34_values_interpolate_and_raycast_misses_stay_unset() {
    let source = go_source();
    let points: Vec<u32> = (0..8).step_by(2).collect();
    for sampling in [TransferSampling::Distance, TransferSampling::Raycast] {
        let mut t = go_target();
        generator_ops::transfer_vertex_map(
            &mut t,
            &points,
            &source,
            TransferType::Weight,
            "W",
            sampling,
        )
        .unwrap();
        let w = &t.attributes.point_weights["W"];
        for (v, want) in [(0u32, 0.05), (2, 0.5), (4, 0.95)] {
            assert!((w[&v] - want).abs() < 1e-9, "{sampling:?} point {v}");
        }
        // Past the source's edge: Distance takes the edge's value,
        // Raycast leaves it unset.
        match sampling {
            TransferSampling::Distance => assert!((w[&6] - 1.).abs() < 1e-9),
            TransferSampling::Raycast => assert!(!w.contains_key(&6)),
        }
    }
}

#[test]
fn go33_world_space_uses_placements_and_local_ignores_them() {
    for (world, want) in [(true, [0.25, 0.7, 1.0]), (false, [0.05, 0.5, 0.95])] {
        let mut e = editor(go_source(), Mode::Vertex);
        e.dispatch(Command::NewMesh).unwrap();
        e.scene.active_mut().unwrap().mesh = go_target().into();
        // The target moved +0.2 m in X and 0.3 m up.
        e.scene.active_mut().unwrap().transform.translation = DVec3::new(0.2, 0.3, 0.);
        e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
        e.scene.selection.vertices = vec![0, 2, 4];
        e.dispatch(Command::TransferVertexMap {
            map: "W".into(),
            kind: TransferType::Weight,
            world,
            sampling: TransferSampling::Distance,
        })
        .unwrap();
        let w = &e.scene.active().unwrap().mesh.attributes.point_weights["W"];
        for (v, want) in [0u32, 2, 4].into_iter().zip(want) {
            assert!(
                (w[&v] - want).abs() < 1e-9,
                "world {world} point {v}: {}",
                w[&v]
            );
        }
    }
}

#[test]
fn go31_the_op_lists_its_options_in_the_reference_order() {
    let d = op_def("vertexmaptransfer").unwrap();
    let keys: Vec<&str> = d.fields().iter().map(|f| f.key.as_ref()).collect();
    assert_eq!(
        keys,
        [
            "item",
            "source",
            "type",
            "space",
            "method",
            "flip",
            "completion"
        ]
    );
    assert_eq!(TransferType::NAMES[1], "Weight");
    assert_eq!(TransferType::NAMES[8], "Vertex Normal");
}
