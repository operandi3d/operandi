//! Edge Bevel/Chamfer/Extrude and Vertex Bevel/Extrude carry UV maps
//! (provisional rule, Jason 2026-11-01): new corners interpolate inside each
//! polygon's own UVs, untouched polygons keep theirs, nothing is flipped.
use glam::{DVec2, DVec3};
use operandi_core::*;

/// A 2x2x2-segment cube with the primitive "Texture" UVs.
fn cube() -> Mesh {
    let (position, size) = (DVec3::ZERO, DVec3::splat(2.));
    let mut mesh = primitive::segmented_cube(position, size, [2; 3]).unwrap();
    add_cube_uvs(&mut mesh, position, size).unwrap();
    mesh
}

fn area(uvs: &[Option<DVec2>]) -> f64 {
    let p: Vec<DVec2> = uvs.iter().map(|u| u.expect("UV")).collect();
    (0..p.len())
        .map(|i| p[i].perp_dot(p[(i + 1) % p.len()]))
        .sum::<f64>()
        / 2.
}

/// One interior edge and one interior point of the cube (shared by four
/// polygons on a single side).
fn interior(mesh: &Mesh) -> ((u32, u32), u32) {
    let point = (0..mesh.vertices.len() as u32)
        .find(|&v| {
            mesh.faces
                .iter()
                .filter(|f| f.vertices.contains(&v))
                .count()
                == 4
                && mesh.vertices[v as usize].abs().max_element() > 0.99
                && mesh.vertices[v as usize].abs().min_element() < 0.01
        })
        .expect("face-center point");
    let edge = mesh
        .edges()
        .into_iter()
        .find(|&(a, b)| a == point || b == point)
        .unwrap();
    (edge, point)
}

fn check(name: &str, before: &Mesh, after: &Mesh) {
    after.validate().unwrap();
    let old = &before.uv_maps[PRIMITIVE_UV_MAP];
    let new = &after.uv_maps[PRIMITIVE_UV_MAP];
    assert_eq!(new.len(), after.faces.len(), "{name}");
    for (fi, (face, uvs)) in after.faces.iter().zip(new).enumerate() {
        assert_eq!(uvs.len(), face.vertices.len(), "{name} face {fi}");
        assert!(
            uvs.iter().all(Option::is_some),
            "{name} face {fi} lost a UV"
        );
        for uv in uvs.iter().flatten() {
            assert!(
                (0. ..=1.).contains(&uv.x) && (0. ..=1.).contains(&uv.y),
                "{name}: {uv}"
            );
        }
        if let Some(src) = before.faces.get(fi) {
            if src.vertices == face.vertices {
                assert_eq!(uvs, &old[fi], "{name} face {fi}: untouched UVs kept");
            } else {
                // A reshaped neighbour shrinks inside its old UV area.
                let (a, b) = (area(uvs), area(&old[fi]));
                assert!(a > 0. && a <= b + 1e-12, "{name} face {fi}: {a} vs {b}");
            }
        }
    }
}

#[test]
fn edge_tools_carry_uvs() {
    let mesh = cube();
    let (edge, _) = interior(&mesh);
    // An edge ending on the cube's edge keeps its end points (N08); it
    // carries UVs too.
    let (out, _) = edge_bevel_geometry(&mesh, &[edge], 0.2, 1, 100., false).unwrap();
    check("Edge Bevel ending at a fold", &mesh, &out);
    // The bevels also run around the middle ring.
    let ring: Vec<(u32, u32)> = mesh
        .edges()
        .into_iter()
        .filter(|&(a, b)| {
            mesh.vertices[a as usize].z.abs() < 1e-9 && mesh.vertices[b as usize].z.abs() < 1e-9
        })
        .collect();
    assert_eq!(ring.len(), 8);
    for (name, parallel, rows) in [
        ("Edge Bevel", false, 1),
        ("Edge Bevel rounded", false, 3),
        ("Edge Chamfer", true, 2),
    ] {
        let (out, _) = edge_bevel_geometry(&mesh, &ring, 0.2, rows, 100., parallel).unwrap();
        check(name, &mesh, &out);
    }
    let (out, _) = edge_bevel_geometry(&mesh, &[edge], 0.2, 2, 100., true).unwrap();
    check("Edge Chamfer to an edge", &mesh, &out);
    let (out, _) = edge_extrude_geometry(&mesh, &[edge], 0.3, 0.2).unwrap();
    check("Edge Extrude", &mesh, &out);
}

#[test]
fn vertex_tools_carry_uvs() {
    let mesh = cube();
    let (_, point) = interior(&mesh);
    let (out, _) = vertex_bevel_geometry(&mesh, &[point], 0.25).unwrap();
    check("Vertex Bevel", &mesh, &out);
    // VX-B02: fillet and cap points interpolate inside the polygons too.
    for level in [1, 2] {
        let (out, _) = vertex_bevel_round(&mesh, &[point], 0.25, level, None).unwrap();
        check(&format!("Vertex Bevel L{level}"), &mesh, &out);
    }
    // A cube corner sits on UV seams: each cap quad reads its UVs off one
    // fan polygon (UVN-T-04).
    let corner = (0..mesh.vertices.len() as u32)
        .find(|&v| mesh.vertices[v as usize].abs().min_element() > 0.99)
        .unwrap();
    let (out, _) = vertex_bevel_round(&mesh, &[corner], 0.25, 2, None).unwrap();
    check("Vertex Bevel L2 corner", &mesh, &out);
    let (out, _) = vertex_extrude_geometry(&mesh, &[point], 0.3, 0.25).unwrap();
    check("Vertex Extrude", &mesh, &out);
}

#[test]
fn a_cut_point_sits_where_it_is_cut_in_uv_space() {
    // Vertex Bevel by a quarter of a 1 m edge: in each neighbour polygon the
    // new corner's UV is a quarter of the way along that polygon's UV edge.
    let mesh = cube();
    let (_, point) = interior(&mesh);
    let (out, new) = vertex_bevel_geometry(&mesh, &[point], 0.25).unwrap();
    let uvs = &out.uv_maps[PRIMITIVE_UV_MAP];
    let old = &mesh.uv_maps[PRIMITIVE_UV_MAP];
    let at = |m: &Mesh, rows: &Vec<Vec<Option<DVec2>>>, f: usize, v: u32| {
        let c = m.faces[f].vertices.iter().position(|&x| x == v)?;
        rows[f][c]
    };
    let mut checked = 0;
    for (fi, face) in out.faces.iter().enumerate().take(mesh.faces.len()) {
        for &q in face.vertices.iter().filter(|v| new.contains(v)) {
            // The neighbour this cut point lies toward, in the source polygon.
            let p = out.vertices[q as usize];
            let toward = mesh.faces[fi]
                .vertices
                .iter()
                .copied()
                .find(|&w| {
                    w != point
                        && mesh.edges().contains(&(w.min(point), w.max(point)))
                        && (mesh.vertices[w as usize] - p).length() < 0.76
                })
                .unwrap();
            let expect = at(&mesh, old, fi, point)
                .unwrap()
                .lerp(at(&mesh, old, fi, toward).unwrap(), 0.25);
            let got = at(&out, uvs, fi, q).unwrap();
            assert!(
                (got - expect).length() < 1e-12,
                "face {fi}: {got} vs {expect}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 8, "two cut corners in each of four polygons");
}

#[test]
fn uv_records_stay_valid_on_a_cone() {
    let mesh = cone_geometry(&ConeParameters {
        sides: 8,
        segments: 3,
        ..Default::default()
    })
    .unwrap();
    assert!(
        !mesh.attributes.uv_records.is_empty(),
        "Cone carries records"
    );
    let ring_point = (0..mesh.vertices.len() as u32)
        .find(|&v| {
            mesh.faces
                .iter()
                .filter(|f| f.vertices.contains(&v))
                .count()
                == 4
        })
        .expect("interior ring point");
    let (out, _) = vertex_bevel_geometry(&mesh, &[ring_point], 0.05).unwrap();
    out.validate().unwrap();
    let edge = mesh
        .edges()
        .into_iter()
        .find(|&(a, b)| a == ring_point || b == ring_point)
        .unwrap();
    // Edge Bevel may refuse to end on the bent ring (EBC-Q11); whatever
    // either tool builds is valid.
    for parallel in [false, true] {
        if let Ok((out, _)) = edge_bevel_geometry(&mesh, &[edge], 0.02, 1, 100., parallel) {
            out.validate().unwrap();
        }
    }
    assert!(edge_bevel_geometry(&mesh, &[edge], 0.02, 1, 100., true).is_ok());
    edge_extrude_geometry(&mesh, &[edge], 0.05, 0.02)
        .unwrap()
        .0
        .validate()
        .unwrap();
}

#[test]
fn other_corner_data_is_still_refused() {
    let mut mesh = cube();
    let rows = mesh
        .faces
        .iter()
        .map(|f| vec![Some(0.5); f.vertices.len()])
        .collect();
    mesh.attributes.corner_weights.insert("w".into(), rows);
    let (edge, point) = interior(&mesh);
    assert!(edge_bevel_geometry(&mesh, &[edge], 0.2, 1, 100., false).is_err());
    assert!(edge_extrude_geometry(&mesh, &[edge], 0.3, 0.2).is_err());
    assert!(vertex_bevel_geometry(&mesh, &[point], 0.25).is_err());
}

/// A 3x3x3-segment cube with the primitive UVs, so each side has a center cell.
fn cube3() -> Mesh {
    let (position, size) = (DVec3::ZERO, DVec3::splat(2.));
    let mut mesh = primitive::segmented_cube(position, size, [3; 3]).unwrap();
    add_cube_uvs(&mut mesh, position, size).unwrap();
    mesh
}

/// `cube3` with the center polygon of each listed side
/// (`(axis, sign)`) removed, leaving square holes.
fn holed(holes: &[(usize, f64)]) -> Mesh {
    let mut m = cube3();
    let gone: Vec<usize> = holes
        .iter()
        .map(|&(axis, sign)| {
            (0..m.faces.len())
                .find(|&f| {
                    let c = m.faces[f]
                        .vertices
                        .iter()
                        .map(|&v| m.vertices[v as usize])
                        .sum::<DVec3>()
                        / 4.;
                    (c[axis] - sign).abs() < 1e-9
                        && c[(axis + 1) % 3].abs() < 1e-9
                        && c[(axis + 2) % 3].abs() < 1e-9
                })
                .unwrap()
        })
        .collect();
    let keep: Vec<usize> = (0..m.faces.len()).filter(|f| !gone.contains(f)).collect();
    m.faces = keep.iter().map(|&f| m.faces[f].clone()).collect();
    for map in m.uv_maps.values_mut() {
        *map = keep.iter().map(|&f| map[f].clone()).collect();
    }
    m.validate().unwrap();
    m
}

/// Edges used by exactly one polygon.
fn border(m: &Mesh) -> Vec<(u32, u32)> {
    let mut count = std::collections::BTreeMap::new();
    for f in &m.faces {
        let n = f.vertices.len();
        for i in 0..n {
            let (a, b) = (f.vertices[i], f.vertices[(i + 1) % n]);
            *count.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    count
        .into_iter()
        .filter(|&(_, k)| k == 1)
        .map(|(e, _)| e)
        .collect()
}

/// Every corner has a UV and polygons kept from `before` keep theirs.
fn carried(name: &str, before: &Mesh, after: &Mesh) {
    after.validate().unwrap();
    let old = &before.uv_maps[PRIMITIVE_UV_MAP];
    let new = &after.uv_maps[PRIMITIVE_UV_MAP];
    for (fi, (face, uvs)) in after.faces.iter().zip(new).enumerate() {
        assert!(
            uvs.iter().all(Option::is_some),
            "{name} face {fi} lost a UV"
        );
        if let Some(src) = before.faces.get(fi)
            && src.vertices == face.vertices
        {
            assert_eq!(uvs, &old[fi], "{name} face {fi}: untouched UVs kept");
        }
    }
}

#[test]
fn grow_and_fill_quads_carry_uvs() {
    let mesh = holed(&[(2, 1.)]);
    let edges = border(&mesh);
    let (grown, _) = edge_command_geometry(&mesh, &edges, EdgeCommand::GrowQuads).unwrap();
    carried("Grow Quads", &mesh, &grown);
    let (filled, _) = edge_command_geometry(&mesh, &edges, EdgeCommand::FillQuads).unwrap();
    carried("Fill Quads", &mesh, &filled);
    // The hole lies inside one tile, so the fill matches the removed polygon.
    let new = &filled.uv_maps[PRIMITIVE_UV_MAP][mesh.faces.len()];
    assert!(area(new) > 0., "Fill Quads polygon is not flipped");
}

#[test]
fn edge_bridge_carries_uvs() {
    let mesh = holed(&[(2, 1.), (2, -1.)]);
    let (out, _) = edge_bridge_geometry(&mesh, &border(&mesh), 2, 0).unwrap();
    carried("Edge Bridge", &mesh, &out);
}

#[test]
fn chamfer_edit_carries_uvs() {
    let mut cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    add_cube_uvs(&mut cube, DVec3::ZERO, DVec3::ONE).unwrap();
    let top = (0..cube.faces.len())
        .find(|&f| cube.normal(f).y > 0.9)
        .unwrap() as u32;
    let r = bevel_geometry(
        &cube,
        &[top],
        0.2,
        0.2,
        false,
        &BevelOptions::default(),
        None,
    )
    .unwrap();
    let mesh = r.mesh;
    let mut outer: Vec<u32> = (0..mesh.faces.len() as u32)
        .filter(|&f| mesh.normal(f as usize).y.abs() < 1e-9)
        .collect();
    outer.push(r.selected[0]);
    let p = ChamferParameters {
        offset: 0.05,
        segments: 3,
        source_segments: 1,
        depth: 100.,
        ..Default::default()
    };
    let out = chamfer_geometry(&mesh, &outer, p).unwrap();
    carried("Chamfer Edit", &mesh, &out);
}

#[test]
fn fill_quads_at_a_cube_edge_uses_the_hole_side_uvs() {
    // A hole in a corner cell of +Z: its border points are shared with the
    // neighbouring sides, which use other UV tiles.
    let mut m = cube3();
    let corner = (0..m.faces.len())
        .find(|&f| {
            let c = m.faces[f]
                .vertices
                .iter()
                .map(|&v| m.vertices[v as usize])
                .sum::<DVec3>()
                / 4.;
            (c.z - 1.).abs() < 1e-9 && c.x > 0.5 && c.y > 0.5
        })
        .unwrap();
    let removed = m.uv_maps[PRIMITIVE_UV_MAP][corner].clone();
    m.faces.remove(corner);
    m.uv_maps.get_mut(PRIMITIVE_UV_MAP).unwrap().remove(corner);
    let (out, _) = edge_command_geometry(&m, &border(&m), EdgeCommand::FillQuads).unwrap();
    carried("Fill Quads at an edge", &m, &out);
    let key = |u: &Option<DVec2>| {
        let u = u.unwrap();
        ((u.x * 1e9).round() as i64, (u.y * 1e9).round() as i64)
    };
    let mut got: Vec<_> = out.uv_maps[PRIMITIVE_UV_MAP][m.faces.len()]
        .iter()
        .map(key)
        .collect();
    let mut want: Vec<_> = removed.iter().map(key).collect();
    got.sort();
    want.sort();
    assert_eq!(
        got, want,
        "the fill takes the removed polygon's +Z tile UVs"
    );
}
