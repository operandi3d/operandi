//! R4 acceptance constructions and admitted sparse observations. No reference
//! application or full reference mesh is used by these tests.
use glam::{DVec2, DVec3};
use operandi_core::*;
use std::collections::{BTreeMap, BTreeSet};
fn parameters(n: u32) -> ConeParameters {
    ConeParameters {
        position: DVec3::new(1., 2., 3.),
        size: DVec3::new(0.37, 0.81, 1.23),
        sides: n,
        segments: 3,
        bottom_cap: ConeCap::QuadGrid,
        make_uvs: false,
        ..Default::default()
    }
}
fn near(a: DVec3, b: DVec3) {
    assert!(
        (a - b).abs().max_element() < 2e-6,
        "{a:?} differs from {b:?}"
    );
}
fn canonical(cycle: &[u32]) -> Vec<u32> {
    (0..cycle.len())
        .map(|i| {
            cycle[i..]
                .iter()
                .chain(&cycle[..i])
                .copied()
                .collect::<Vec<_>>()
        })
        .min()
        .unwrap()
}
fn cap_cycles(mesh: &Mesh, start: usize) -> BTreeSet<Vec<u32>> {
    mesh.faces[start..]
        .iter()
        .map(|f| canonical(&f.vertices))
        .collect()
}
fn counts(n: u32, m: u32) -> (usize, usize, usize) {
    if n <= 4 {
        let v = n * m + 1;
        return (v as usize, (2 * n * m) as usize, (n * m + 1) as usize);
    }
    let p = n / 4;
    let q = p + u32::from(n % 4 == 2);
    let v = n * m + 1 + n + (p - 1) * (q - 1);
    let f = n * m + n + p * q;
    (v as usize, (v + f - 2) as usize, f as usize)
}
fn inspect(p: &ConeParameters) {
    let mesh = cone_geometry(p).unwrap();
    let n = p.sides as usize;
    let wall = (p.sides * p.segments) as usize;
    assert_eq!(
        (mesh.vertices.len(), mesh.edges().len(), mesh.faces.len()),
        counts(p.sides, p.segments)
    );
    assert!(mesh.is_closed());
    assert_eq!(
        mesh.faces[..wall]
            .iter()
            .filter(|f| f.vertices.len() == 3)
            .count(),
        n
    );
    assert!(
        mesh.faces[wall..]
            .iter()
            .all(|f| f.vertices.len() == if n == 3 { 3 } else { 4 })
    );
    let cap_edges = mesh.faces[wall..]
        .iter()
        .flat_map(|f| {
            (0..f.vertices.len()).map(|i| {
                let a = f.vertices[i];
                let b = f.vertices[(i + 1) % f.vertices.len()];
                (a.min(b), a.max(b))
            })
        })
        .fold(BTreeMap::new(), |mut map, edge| {
            *map.entry(edge).or_insert(0) += 1;
            map
        });
    let boundary: BTreeSet<_> = cap_edges
        .into_iter()
        .filter(|(_, count)| *count == 1)
        .map(|(edge, _)| edge)
        .collect();
    let expected: BTreeSet<_> = (0..p.sides)
        .map(|i| (i.min((i + 1) % p.sides), i.max((i + 1) % p.sides)))
        .collect();
    assert_eq!(boundary, expected);
    let base = p.position[p.axis] - p.size[p.axis];
    let normal_sign = if p.size[(p.axis + 1) % 3] * p.size[(p.axis + 2) % 3] > 0. {
        -1.
    } else {
        1.
    };
    let normal_sign = normal_sign * p.size[p.axis].signum();
    for fi in wall..mesh.faces.len() {
        assert!((mesh.normal(fi)[p.axis] - normal_sign).abs() < 1e-9);
        for &v in &mesh.faces[fi].vertices {
            assert!((mesh.vertices[v as usize][p.axis] - base).abs() < 2e-6);
        }
    }
    assert!(
        mesh.attributes
            .representations
            .iter()
            .all(|&r| r == p.polygon)
    );
    assert_eq!(
        mesh.attributes.creases.len(),
        if p.polygon == PolygonRepresentation::Face {
            0
        } else {
            n
        }
    );
    for crease in &mesh.attributes.creases {
        assert!(crease.a < p.sides && crease.b < p.sides);
        assert_eq!(crease.weight, p.edge_weight / 100.);
    }
}
#[test]
fn ca_gt02_gt03_complete_eight_side_oracle() {
    let p = ConeParameters {
        position: DVec3::ZERO,
        size: DVec3::new(1., 1.5, 1.),
        segments: 3,
        ..parameters(8)
    };
    let mesh = cone_geometry(&p).unwrap();
    let start = 25u32;
    for i in 0..8 {
        let mut expected = mesh.vertices[(i + 1) % 8] * 0.5;
        expected.y = -1.5;
        near(mesh.vertices[start as usize + i], expected);
    }
    near(mesh.vertices[33], DVec3::new(0., -1.5, 0.));
    let b = |i: u32| i;
    let j = |i: u32| start + i;
    let cycles = [
        [b(1), j(0), j(1), b(2)],
        [b(2), j(1), j(2), b(3)],
        [b(3), j(2), j(3), b(4)],
        [b(4), j(3), j(4), b(5)],
        [b(5), j(4), j(5), b(6)],
        [b(6), j(5), j(6), b(7)],
        [b(7), j(6), j(7), b(0)],
        [b(0), j(7), j(0), b(1)],
        [j(7), j(8), j(1), j(0)],
        [j(8), j(3), j(2), j(1)],
        [j(6), j(5), j(8), j(7)],
        [j(5), j(4), j(3), j(8)],
    ];
    assert_eq!(
        cap_cycles(&mesh, 24),
        cycles.iter().map(|f| canonical(f)).collect()
    );
}
#[test]
fn ca_gt02_even_count_axis_type_matrices_topology_and_planarity() {
    for n in (6..=32).step_by(2).chain([48, 64]) {
        inspect(&ConeParameters {
            position: DVec3::ZERO,
            size: DVec3::new(1., 1.5, 1.),
            segments: 2,
            ..parameters(n)
        });
    }
    for n in [34, 38, 44, 66, 96, 128] {
        for axis in 0..3 {
            inspect(&ConeParameters {
                axis,
                ..parameters(n)
            });
        }
    }
    for n in [10, 14] {
        for axis in 0..3 {
            for polygon in [
                PolygonRepresentation::Subdivs,
                PolygonRepresentation::CatmullClark,
            ] {
                inspect(&ConeParameters {
                    axis,
                    segments: 4,
                    polygon,
                    edge_weight: 50.,
                    ..parameters(n)
                });
            }
        }
    }
    for n in [3, 4] {
        for axis in 0..3 {
            inspect(&ConeParameters {
                axis,
                ..parameters(n)
            });
        }
    }
}
#[test]
fn ca_gt05_every_uv_corner_and_assignment_identity_in_54_supported_cases() {
    // Chart constants evaluated from admitted CA-CON-021 at the R4 unequal size.
    // These are acceptance constructions, not additional runtime observations.
    let compensated = [
        (
            [0.27360171735993327, 0.620827021786768],
            [0.20360171735993327, 0.309172978213232],
            [0.7386017173599333, 0.6393581633984171],
            [0.19139828264006675, 0.2906418366015828],
        ),
        (
            [0.3019843236727236, 0.8602160977569855],
            [0.2319843236727236, 0.06978390224301441],
            [0.7669843236727236, 0.8809627640316322],
            [0.1630156763272764, 0.0490372359683677],
        ),
        (
            [0.26641975308641974, 0.5],
            [0.19641975308641973, 0.43],
            [0.6177896240520069, 0.7440281203185795],
            [0.08495011787916738, 0.18597187968142048],
        ),
    ];
    for n in [3, 4, 6, 8, 10, 12, 24, 34, 66] {
        for (axis, &chart) in compensated.iter().enumerate() {
            for compensation in [false, true] {
                let p = ConeParameters {
                    axis,
                    segments: 2,
                    make_uvs: true,
                    compensation,
                    gaps: if compensation { 7. } else { 0. },
                    selected_uv_map: Some("SparseUV".into()),
                    ..parameters(n)
                };
                let mesh = cone_geometry(&p).unwrap();
                let wall = (n * 2) as usize;
                let map = &mesh.uv_maps["SparseUV"];
                let ids = &mesh.attributes.uv_records["SparseUV"];
                let (sc, sr, cc, cr) = if compensation {
                    chart
                } else {
                    ([0.25, 0.75], [0.25, 0.25], [0.75, 0.75], [0.25, 0.25])
                };
                let mut side_ids = BTreeMap::<u32, u32>::new();
                let mut cap_ids = BTreeSet::new();
                let mut cap_corners = 0;
                for (fi, face) in mesh.faces.iter().enumerate() {
                    for (ci, &vertex) in face.vertices.iter().enumerate() {
                        let id = ids[fi][ci].unwrap();
                        let phi = std::f64::consts::TAU * (vertex % n) as f64 / n as f64
                            + std::f64::consts::PI / n as f64;
                        let expected = if fi < wall {
                            if let Some(previous) = side_ids.insert(vertex, id) {
                                assert_eq!(previous, id);
                            }
                            let t = if vertex == n * 2 {
                                1.
                            } else {
                                (vertex / n) as f64 / 2.
                            };
                            DVec2::from(sc)
                                + (1. - t) * DVec2::from(sr) * DVec2::new(phi.cos(), phi.sin())
                        } else {
                            assert!(cap_ids.insert(id), "cap corners merged by equal coordinate");
                            cap_corners += 1;
                            if n <= 4 {
                                DVec2::from(cc)
                                    + DVec2::from(cr) * DVec2::new(phi.cos(), -phi.sin())
                            } else {
                                let mut base = p.position;
                                base[axis] -= p.size[axis];
                                let d = mesh.vertices[vertex as usize] - base;
                                let normalized = match axis {
                                    0 => DVec2::new(d.z / p.size.z, d.y / p.size.y),
                                    1 => DVec2::new(d.x / p.size.x, d.z / p.size.z),
                                    _ => DVec2::new(d.x / p.size.x, d.y / p.size.y),
                                };
                                DVec2::from(cc) + DVec2::from(cr) * normalized
                            }
                        };
                        assert!((map[fi][ci].unwrap() - expected).abs().max_element() < 2e-6);
                    }
                }
                assert_eq!(side_ids.len(), (n * 2 + 1) as usize);
                assert_eq!(cap_ids.len(), cap_corners);
                assert!(side_ids.values().all(|id| !cap_ids.contains(id)));
                let expected_caps = if n <= 4 {
                    n as usize
                } else {
                    let p = n / 4;
                    let q = p + u32::from(n % 4 == 2);
                    (4 * (n + p * q)) as usize
                };
                assert_eq!(cap_corners, expected_caps);
                if n >= 6 {
                    inspect(&p);
                }
            }
        }
    }
}

#[test]
fn ca_gt02_all_grid_coordinates_and_oriented_cycles_at_six_resolutions() {
    // Independent analytic boundary oracle: evaluate J directly from angle and
    // the output contract, never from the generator's allocated grid vertices.
    for n in [8u32, 10, 14, 24, 34, 128] {
        let p = parameters(n);
        let mesh = cone_geometry(&p).unwrap();
        let cols = n / 4;
        let rows = cols + u32::from(n % 4 == 2);
        let offset = n / 8;
        let rho = (1. - 1.8 * std::f64::consts::PI / n as f64).max(0.5);
        let boundary = |index: u32| {
            let theta = std::f64::consts::TAU * ((offset + index) % n) as f64 / n as f64;
            p.position
                + DVec3::new(
                    -p.size.x * rho * theta.sin(),
                    -p.size.y,
                    -p.size.z * rho * theta.cos(),
                )
        };
        let point = |i: u32, j: u32| {
            if i == 0 {
                return boundary(j);
            }
            if j == rows {
                return boundary(rows + i);
            }
            if i == cols {
                return boundary(2 * rows + cols - j);
            }
            if j == 0 {
                return boundary(n - i);
            }
            let s = i as f64 / cols as f64;
            let t = j as f64 / rows as f64;
            let u = (1. + 18. * s * s - 12. * s * s * s) / 8.;
            let v = (1. + 18. * t * t - 12. * t * t * t) / 8.;
            // Expanded blending coefficients from CA-CON-045, evaluated independently.
            boundary(j) * (1. - u)
                + boundary(2 * rows + cols - j) * u
                + boundary(n - i) * (1. - v)
                + boundary(rows + i) * v
                + boundary(0) * (-1. + u + v - u * v)
                + boundary(n - cols) * (-u + u * v)
                + boundary(rows) * (-v + u * v)
                - boundary(rows + cols) * (u * v)
        };
        let find = |position: DVec3| -> u32 {
            mesh.vertices
                .iter()
                .position(|&v| (v - position).abs().max_element() < 2e-6)
                .expect("missing semantic cap point") as u32
        };
        let mut expected = BTreeSet::new();
        let mut all_grid = BTreeSet::new();
        for j in 0..=rows {
            for i in 0..=cols {
                all_grid.insert(find(point(i, j)));
            }
        }
        assert_eq!(all_grid.len(), ((cols + 1) * (rows + 1)) as usize);
        for j in 0..rows {
            for i in 0..cols {
                expected.insert(canonical(&[
                    find(point(i + 1, j)),
                    find(point(i + 1, j + 1)),
                    find(point(i, j + 1)),
                    find(point(i, j)),
                ]));
            }
        }
        for i in 0..n {
            expected.insert(canonical(&[
                (offset + i) % n,
                find(boundary(i)),
                find(boundary((i + 1) % n)),
                (offset + i + 1) % n,
            ]));
        }
        assert_eq!(
            cap_cycles(&mesh, (n * p.segments) as usize),
            expected,
            "N{n}: wrong oriented cap cycles"
        );
    }
}

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.history.undo.clear();
    e
}
#[test]
fn ca_gt05_gt06_sparse_identity_preservation_apply_history_and_disk_roundtrip() {
    let mut e = editor();
    let source = ConeParameters {
        bottom_cap: ConeCap::Single,
        make_uvs: true,
        selected_uv_map: Some("SparseUV".into()),
        polygon: PolygonRepresentation::Subdivs,
        edge_weight: 37.5,
        ..parameters(8)
    };
    let mut mesh = cone_geometry(&source).unwrap();
    // A genuinely absent source UV/identity pair, while other corners stay assigned.
    mesh.uv_maps.get_mut("SparseUV").unwrap()[0][0] = None;
    mesh.attributes.uv_records.get_mut("SparseUV").unwrap()[0][0] = None;
    mesh.attributes.corner_normals.insert(
        "OriginalNormals".into(),
        mesh.faces
            .iter()
            .map(|f| vec![Some(DVec3::X); f.vertices.len()])
            .collect(),
    );
    mesh.attributes
        .point_weights
        .entry("OriginalWeight".into())
        .or_default()
        .insert(4, 0.7);
    mesh.faces[0].part = Some("PartOriginal".into());
    mesh.faces[0].material = "OriginalMaterial".into();
    mesh.validate().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.clone().into();
    let target = e.scene.active().unwrap().id;
    e.scene.new_mesh().unwrap();
    let unrelated = e.scene.active().unwrap().clone();
    e.scene.select_item(Some(target));
    let before = e.scene.clone();
    e.dispatch(Command::ArmCone).unwrap();
    let p = ConeParameters {
        polygon: PolygonRepresentation::CatmullClark,
        edge_weight: 50.,
        make_uvs: true,
        compensation: true,
        gaps: 7.,
        selected_uv_map: Some("SparseUV".into()),
        ..parameters(14)
    };
    e.dispatch(Command::ConfigureCone(p.clone())).unwrap();
    let valid = e.session.cone.as_ref().unwrap().preview.clone();
    e.dispatch(Command::ConfigureCone(ConeParameters {
        gaps: 50.,
        ..p.clone()
    }))
    .unwrap_err();
    assert_eq!(e.session.cone.as_ref().unwrap().preview, valid);
    assert!(e.dispatch(Command::Apply).is_err());
    assert_eq!(e.scene, before);
    e.dispatch(Command::ConfigureCone(p)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    let committed = e.scene.clone();
    let out = &committed.active().unwrap().mesh;
    assert_eq!(&out.vertices[..mesh.vertices.len()], &mesh.vertices);
    assert_eq!(&out.faces[..mesh.faces.len()], &mesh.faces);
    assert_eq!(
        &out.uv_maps["SparseUV"][..mesh.faces.len()],
        &mesh.uv_maps["SparseUV"]
    );
    assert_eq!(
        &out.attributes.uv_records["SparseUV"][..mesh.faces.len()],
        &mesh.attributes.uv_records["SparseUV"]
    );
    let old: BTreeSet<_> = mesh.attributes.uv_records["SparseUV"]
        .iter()
        .flatten()
        .flatten()
        .copied()
        .collect();
    let new: BTreeSet<_> = out.attributes.uv_records["SparseUV"][mesh.faces.len()..]
        .iter()
        .flatten()
        .flatten()
        .copied()
        .collect();
    assert!(old.is_disjoint(&new));
    assert_eq!(
        &out.attributes.corner_normals["OriginalNormals"][..mesh.faces.len()],
        &mesh.attributes.corner_normals["OriginalNormals"]
    );
    assert_eq!(out.attributes.point_weights, mesh.attributes.point_weights);
    assert_eq!(committed.items[1], unrelated);
    // Drop after Apply shows no preview, so it records nothing.
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.cone.is_none());
    assert_eq!(e.scene, committed);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, committed);
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(persistence::decode(&bytes).unwrap(), committed);
    let path = std::env::temp_dir().join(format!("operandi-grid-{}.opscene", std::process::id()));
    e.save_committed(&path).unwrap();
    assert_eq!(persistence::load(&path).unwrap(), committed);
    std::fs::remove_file(path).unwrap();
    // Legacy v3 coordinates lack ownership metadata: do not invent record sharing.
    let mut legacy: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    legacy["version"] = 3.into();
    for item in legacy["scene"]["items"].as_array_mut().unwrap() {
        item["mesh"]["attributes"]
            .as_object_mut()
            .unwrap()
            .remove("uv_records");
    }
    let loaded = persistence::decode(&serde_json::to_vec(&legacy).unwrap()).unwrap();
    assert_eq!(
        loaded.active().unwrap().mesh.uv_maps,
        committed.active().unwrap().mesh.uv_maps
    );
    assert!(
        loaded
            .active()
            .unwrap()
            .mesh
            .attributes
            .uv_records
            .is_empty()
    );
}
#[test]
fn ca_gt05_record_validation_and_atomic_overflow_never_merge_equal_uvs() {
    let p = ConeParameters {
        make_uvs: true,
        ..parameters(8)
    };
    let mesh = cone_geometry(&p).unwrap();
    let mut bad = mesh.clone();
    let side_id = bad.attributes.uv_records["Texture"][0][0];
    bad.attributes.uv_records.get_mut("Texture").unwrap()[0][1] = side_id;
    assert!(bad.validate().is_err());
    let mut bad = mesh.clone();
    bad.uv_maps.get_mut("Texture").unwrap()[0][0] = None;
    assert!(bad.validate().is_err());
    let mut target = Mesh::default();
    assert!(append_mesh(&mut target, bad).is_err());
    assert_eq!(target, Mesh::default());
    let mut legacy = mesh.clone();
    legacy.attributes.uv_records.clear();
    let prefix = legacy.uv_maps.clone();
    append_mesh(&mut legacy, mesh.clone()).unwrap();
    assert_eq!(
        &legacy.uv_maps["Texture"][..mesh.faces.len()],
        &prefix["Texture"]
    );
    assert!(
        legacy.attributes.uv_records["Texture"][..mesh.faces.len()]
            .iter()
            .flatten()
            .all(Option::is_none)
    );
    let mut source = mesh.clone();
    for id in source
        .attributes
        .uv_records
        .get_mut("Texture")
        .unwrap()
        .iter_mut()
        .flatten()
        .flatten()
    {
        if *id == 0 {
            *id = u32::MAX;
        }
    }
    source.validate().unwrap();
    let before = source.clone();
    assert!(append_mesh(&mut source, mesh).is_err());
    assert_eq!(source, before);
}
#[test]
fn ca_gt04_unsupported_domains_and_nonfinite_endpoint_preserve_last_preview() {
    let mut e = editor();
    e.dispatch(Command::ArmCone).unwrap();
    let p = ConeParameters {
        make_uvs: true,
        ..parameters(8)
    };
    e.dispatch(Command::ConfigureCone(p.clone())).unwrap();
    let valid = e.session.cone.as_ref().unwrap().preview.clone();
    // Odd sides are a supported cap since PRIM-K-02.
    for request in [
        ConeParameters {
            sides: 4096,
            ..p.clone()
        },
        ConeParameters {
            gaps: 50.,
            ..p.clone()
        },
        ConeParameters {
            gaps: 40.,
            ..p.clone()
        },
        ConeParameters {
            size: DVec3::new(0., 1., 1.),
            ..p.clone()
        },
        ConeParameters {
            size: DVec3::new(-1., 1., 1.),
            ..p.clone()
        },
    ] {
        assert!(e.dispatch(Command::ConfigureCone(request)).is_err());
        assert_eq!(e.session.cone.as_ref().unwrap().preview, valid);
        assert!(e.dispatch(Command::Apply).is_err());
        assert!(e.scene.active().unwrap().mesh.vertices.is_empty());
    }
    e.dispatch(Command::ConfigureCone(ConeParameters {
        gaps: 50.,
        make_uvs: false,
        ..p
    }))
    .unwrap();
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 36);
}
