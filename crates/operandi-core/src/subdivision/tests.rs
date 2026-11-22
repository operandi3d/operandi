//! Oracles from specifications/subdivision-2026-11-03/ORACLES.json.
use super::*;
use crate::MeshCrease;
use crate::topology::edge_key;
use serde_json::Value;

pub(super) fn oracles() -> BTreeMap<String, Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specifications/subdivision-2026-11-03/ORACLES.json");
    let all: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    all["oracles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| (o["id"].as_str().unwrap().to_owned(), o.clone()))
        .collect()
}

fn vec3(v: &Value) -> DVec3 {
    DVec3::new(
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    )
}

/// The oracle's cage, with every polygon of the given kind and its weights.
pub(super) fn cage(input: &Value) -> (Mesh, SubdivisionSettings) {
    let rep_of = |name: &str| match name {
        "catmull-clark" | "catmull-clark (Shift+Tab)" => PolygonRepresentation::CatmullClark,
        "face" => PolygonRepresentation::Face,
        _ => PolygonRepresentation::Subdivs,
    };
    let mut mesh = Mesh {
        vertices: input["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(vec3)
            .collect(),
        faces: input["polygons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| Face {
                vertices: p
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect(),
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    };
    mesh.attributes.representations = match input["polygon_types"].as_array() {
        Some(types) => types.iter().map(|t| rep_of(t.as_str().unwrap())).collect(),
        None => vec![rep_of(input["all_polygons_type"].as_str().unwrap()); mesh.faces.len()],
    };
    for w in input["edge_weights"].as_array().into_iter().flatten() {
        let (a, b) = (
            w["edge"][0].as_u64().unwrap() as u32,
            w["edge"][1].as_u64().unwrap() as u32,
        );
        mesh.attributes.creases.push(MeshCrease {
            a: a.min(b),
            b: a.max(b),
            weight: w["weight"].as_f64().unwrap(),
        });
    }
    let level = input["display_level"].as_u64().unwrap() as u8;
    let rule = match input["boundary_rule"].as_str().unwrap_or("crease") {
        "smooth" => BoundaryRule::Smooth,
        "creaseEdges" => BoundaryRule::CreaseEdges,
        _ => BoundaryRule::Crease,
    };
    let settings = SubdivisionSettings {
        subdivision_level: level,
        catmull_clark_level: level,
        boundary_rule: rule,
        ..Default::default()
    };
    (mesh, settings)
}

/// Compare the frozen point set and counts, to the oracle's six decimals.
fn check(id: &str, o: &Value) -> std::result::Result<(), String> {
    let (mesh, settings) = cage(&o["input"]);
    let out = subdivide(&mesh, &settings)?;
    let e = &o["expected"];
    if out.vertices.len() as u64 != e["n_points"].as_u64().unwrap()
        || out.faces.len() as u64 != e["n_polygons"].as_u64().unwrap()
    {
        return Err(format!(
            "{id}: {} points, {} polygons",
            out.vertices.len(),
            out.faces.len()
        ));
    }
    let tolerance = o["tolerance"].as_f64().unwrap_or(1e-5).max(1e-5);
    let mut unmatched: Vec<DVec3> = out.vertices.clone();
    for p in e["frozen_points_as_a_set"]
        .as_array()
        .unwrap()
        .iter()
        .map(vec3)
    {
        let Some(i) = unmatched
            .iter()
            .position(|q| (*q - p).abs().max_element() <= tolerance)
        else {
            return Err(format!("{id}: no point near {p}"));
        };
        unmatched.swap_remove(i);
    }
    Ok(())
}

#[test]
fn catmull_clark_matches_every_oracle() {
    let failures: Vec<String> = oracles()
        .iter()
        .filter(|(id, _)| id.starts_with("SDV-O-PSUB-"))
        .filter_map(|(id, o)| check(id, o).err())
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn modo_subdivision_matches_every_oracle_without_edge_weights() {
    // SDV-09 v2 at every level, closed and open. Edge weights on Subdivision
    // polygons are SDV-Q02 (open), so those oracles are left out.
    let failures: Vec<String> = oracles()
        .iter()
        .filter(|(id, _)| id.starts_with("SDV-O-SUBD-") && !id.contains("edgew"))
        .filter_map(|(id, o)| check(id, o).err())
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn toggles_follow_the_oracle_sequence() {
    let o = &oracles()["SDV-O-toggle"]["expected"];
    let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    let names = |m: &Mesh| -> Vec<&'static str> {
        (0..m.faces.len())
            .map(|f| match kind(m, f) {
                PolygonRepresentation::Subdivs => "subdiv",
                PolygonRepresentation::CatmullClark => "psubdiv",
                _ => "face",
            })
            .collect()
    };
    let expect = |m: &Mesh, key: &str| {
        let want: Vec<&str> = o[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(names(m), want, "{key}");
    };
    let tab = |m: &Mesh, sel: &[u32]| toggle(m, sel, Toggle::Subdivision).unwrap();
    let stab = |m: &Mesh, sel: &[u32]| toggle(m, sel, Toggle::CatmullClark).unwrap();
    expect(&tab(&cube, &[]), "face_tab");
    expect(&tab(&tab(&cube, &[]), &[]), "face_tab_tab");
    expect(&stab(&cube, &[]), "face_stab");
    expect(&stab(&stab(&cube, &[]), &[]), "face_stab_stab");
    expect(&tab(&stab(&cube, &[]), &[]), "psub_tab");
    expect(&stab(&tab(&cube, &[]), &[]), "subd_stab");
    expect(&tab(&cube, &[0, 1]), "sel01_tab");
    // The steps run in sequence on the same cube.
    let mixed = tab(&tab(&cube, &[0, 1]), &[1, 2]);
    expect(&mixed, "mixed_sel12_tab");
    expect(&tab(&mixed, &[]), "mixed_nosel_tab");
    let three = tab(&stab(&cube, &[0]), &[1]);
    expect(&three, "three_kinds_before");
    expect(&tab(&three, &[]), "three_kinds_nosel_tab");
    expect(&stab(&three, &[]), "three_kinds_nosel_stab");
}

#[test]
fn freeze_replaces_smooth_polygons_with_quads() {
    let cube = toggle(
        &Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap(),
        &[],
        Toggle::CatmullClark,
    )
    .unwrap();
    let frozen = freeze(&cube, &SubdivisionSettings::default()).unwrap();
    // SDV-04: level 2 on a cube gives 98 points and 96 quads.
    assert_eq!((frozen.vertices.len(), frozen.faces.len()), (98, 96));
    assert!(!has_smooth(&frozen));
    assert!(frozen.is_closed());
    assert!(freeze(&frozen, &SubdivisionSettings::default()).is_err());
}

#[test]
fn face_polygons_cut_off_the_smooth_region() {
    // SDV-13.
    let o = oracles();
    let failures: Vec<String> = ["SDV-O-MIX-psub5-face1-L1", "SDV-O-MIX-subd5-face1-L1"]
        .iter()
        .filter_map(|id| check(id, &o[*id]).err())
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn freeze_keeps_uvs_as_a_linear_subdivision() {
    // SDV-15: one UV island per face comes out as a straight subdivision.
    let mut cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    let square = [(0., 0.), (1., 0.), (1., 1.), (0., 1.)];
    let uvs = cube
        .faces
        .iter()
        .map(|_| {
            square
                .iter()
                .map(|&(u, v)| Some(glam::DVec2::new(u, v)))
                .collect()
        })
        .collect();
    cube.uv_maps.insert("Texture".into(), uvs);
    for (toggle_kind, level) in [(Toggle::CatmullClark, 2), (Toggle::Subdivision, 3)] {
        let smooth = toggle(&cube, &[], toggle_kind).unwrap();
        let settings = SubdivisionSettings {
            catmull_clark_level: level,
            subdivision_level: level,
            ..Default::default()
        };
        let frozen = freeze(&smooth, &settings).unwrap();
        let map = &frozen.uv_maps["Texture"];
        assert_eq!(map.len(), frozen.faces.len());
        // Each frozen polygon is an axis-aligned square of side 1/(2L) or
        // 1/2^L in UV space, inside the unit square.
        let side = if toggle_kind == Toggle::Subdivision {
            1.0 / (2.0 * level as f64)
        } else {
            1.0 / 2f64.powi(level as i32)
        };
        for corners in map {
            let c: Vec<glam::DVec2> = corners.iter().map(|u| u.unwrap()).collect();
            let (lo, hi) = c.iter().fold(
                (glam::DVec2::splat(9.), glam::DVec2::splat(-9.)),
                |(a, b), p| (a.min(*p), b.max(*p)),
            );
            assert!(lo.min_element() >= -1e-12 && hi.max_element() <= 1. + 1e-12);
            assert!(
                ((hi - lo) - glam::DVec2::splat(side)).abs().max_element() < 1e-9,
                "{c:?}"
            );
        }
    }
}

#[test]
fn local_updates_match_a_full_recompute() {
    // Both kinds mixed in one region: subdivided as Catmull-Clark (SDV-14).
    local_updates_case(|i, j| match (j * 30 + i) % 7 {
        0 => PolygonRepresentation::Face,
        1 | 2 => PolygonRepresentation::Subdivs,
        _ => PolygonRepresentation::CatmullClark,
    });
    // Separate regions, a Subdivision one carrying a crease (EW-05).
    local_updates_case(|i, _| match i {
        0..12 => PolygonRepresentation::Subdivs,
        12 => PolygonRepresentation::Face,
        _ => PolygonRepresentation::CatmullClark,
    });
}

/// A 30 x 30 grid with an open border and a crease, kinds by (column, row).
fn local_updates_case(kind_at: impl Fn(u32, u32) -> PolygonRepresentation) {
    let n = 30u32;
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(
                i as f64,
                j as f64,
                ((i * 7 + j * 3) % 5) as f64 * 0.1,
            ));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    mesh.attributes.representations = (0..mesh.faces.len() as u32)
        .map(|f| kind_at(f % n, f / n))
        .collect();
    mesh.attributes.creases.push(MeshCrease {
        a: 40,
        b: 41,
        weight: 0.3,
    });
    for (cc, sd, rule) in [
        (1, 1, BoundaryRule::Crease),
        (2, 2, BoundaryRule::Smooth),
        (3, 3, BoundaryRule::CreaseEdges),
    ] {
        let settings = SubdivisionSettings {
            catmull_clark_level: cc,
            subdivision_level: sd,
            boundary_rule: rule,
            ..Default::default()
        };
        let (mut surface, sources) = display_surface(&mesh, &settings).unwrap();
        let blocks = source_blocks(&sources, mesh.faces.len());
        let mut moved = mesh.clone();
        // Points inside and on the open border.
        for v in [70u32, 71, 101, 400, 15] {
            moved.vertices[v as usize] += DVec3::new(0.13, -0.07, 0.4);
        }
        let dirty = update_surface(&mesh, &moved, &settings, &mut surface, &blocks).unwrap();
        assert!(!dirty.is_empty() && dirty.len() < mesh.faces.len() / 2);
        let (full, _) = display_surface(&moved, &settings).unwrap();
        assert_eq!(full.faces, surface.faces);
        for (a, b) in full.vertices.iter().zip(&surface.vertices) {
            assert!(
                (*a - *b).abs().max_element() < 1e-9,
                "level {cc}: {a} vs {b}"
            );
        }
        // Creases added, reweighted and removed update in place too (an
        // Edge Weight drag), matching a full recompute.
        let mut creased = moved.clone();
        creased.attributes.creases = vec![
            MeshCrease {
                a: 40,
                b: 41,
                weight: 0.8,
            },
            MeshCrease {
                a: 200,
                b: 201,
                weight: 1.0,
            },
        ];
        let dirty = update_surface(&moved, &creased, &settings, &mut surface, &blocks).unwrap();
        assert!(!dirty.is_empty() && dirty.len() < mesh.faces.len() / 2);
        let (full, _) = display_surface(&creased, &settings).unwrap();
        for (a, b) in full.vertices.iter().zip(&surface.vertices) {
            assert!(
                (*a - *b).abs().max_element() < 1e-9,
                "creased level {cc}: {a} vs {b}"
            );
        }
        // A UV-only edit (a UV drag) leaves the surface as it is: the views
        // read UVs from the cage, so nothing is evaluated again.
        let mut uv_moved = creased.clone();
        uv_moved.uv_maps.insert(
            "Texture".into(),
            uv_moved
                .faces
                .iter()
                .map(|f| vec![Some(glam::DVec2::splat(0.25)); f.vertices.len()])
                .collect(),
        );
        let before = surface.clone();
        let dirty = update_surface(&creased, &uv_moved, &settings, &mut surface, &blocks).unwrap();
        assert!(dirty.is_empty());
        assert_eq!(surface, before);
        let moved = creased;
        // A topology change can't be updated in place.
        let mut other = moved.clone();
        other.faces.pop();
        assert!(update_surface(&moved, &other, &settings, &mut surface, &blocks).is_none());
    }
}

#[test]
fn edge_weight_hand_off_targets_match() {
    // specifications/edge-weights-2026-11-05: `targets` only; the
    // `reference_only` Subdivision-kind results are not Operandi's rule.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specifications/edge-weights-2026-11-05/ORACLES-EDGE-WEIGHTS.json");
    let all: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let targets = all["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 14);
    let failures: Vec<String> = targets
        .iter()
        .filter_map(|o| check(o["id"].as_str().unwrap(), o).err())
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn edge_weight_v2_catmull_clark_oracles_match() {
    // Edge weights v2 (EW2-01/02): every Catmull-Clark case, including
    // weights beyond ±100 % and negatives. The Subdivision-kind cases are
    // not Operandi's rule yet (EW2-04), so they aren't checked.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specifications/edge-weights-2026-11-05/ORACLES-EDGE-WEIGHTS-v2.json");
    let all: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let cases: Vec<&Value> = all["oracles"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| {
            o["input"]["all_polygons_type"]
                .as_str()
                .unwrap()
                .starts_with("catmull-clark")
        })
        .collect();
    assert_eq!(cases.len(), 56);
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|o| check(o["id"].as_str().unwrap(), o).err())
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The cube of the oracles as one kind, with edge 5-7 weighted.
fn weighted_cube(kind: &str, weight: f64, level: u8) -> (Mesh, SubdivisionSettings) {
    let o = oracles();
    let mut input = o["SDV-O-SUBD-cube-L1"]["input"].clone();
    input["all_polygons_type"] = Value::from(kind);
    input["display_level"] = Value::from(level);
    if weight != 0.0 {
        input["edge_weights"] = serde_json::json!([{ "edge": [5, 7], "weight": weight }]);
    }
    cage(&input)
}

/// Distance from the surface to the weighted edge's midpoint (0.5, 0, 0.5).
fn gap_to_edge(surface: &Mesh) -> f64 {
    let mid = DVec3::new(0.5, 0.0, 0.5);
    surface
        .vertices
        .iter()
        .map(|p| p.distance(mid))
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn subdivision_kind_weights_sharpen_the_edge() {
    // EW-05 (Operandi choice): no Modo oracle, so check the shape of it.
    for level in 1..=3 {
        let (plain, settings) = weighted_cube("subdiv", 0.0, level);
        let base = subdivide(&plain, &settings).unwrap();
        let mut last = gap_to_edge(&base);
        for weight in [0.05, 0.1, 0.25, 0.5, 1.0] {
            let (cage, settings) = weighted_cube("subdiv", weight, level);
            let out = subdivide(&cage, &settings).unwrap();
            assert_eq!(out.vertices.len(), base.vertices.len());
            assert!(out.is_closed());
            let gap = gap_to_edge(&out);
            assert!(
                gap < last + 1e-12,
                "level {level} weight {weight}: {gap} vs {last}"
            );
            last = gap;
        }
        assert!(last < gap_to_edge(&base) * 0.5, "level {level}: {last}");
        // EW2-01: the effect stops at 100 %.
        let (cage, settings) = weighted_cube("subdiv", 5.0, level);
        let (full, _) = weighted_cube("subdiv", 1.0, level);
        assert_eq!(
            subdivide(&cage, &settings).unwrap().vertices,
            subdivide(&full, &settings).unwrap().vertices
        );
    }
}

#[test]
fn subdivision_kind_negative_weights_soften_the_edge() {
    // EW2-04 (Operandi choice, interim): negatives push the edge away from
    // the cage, more as the weight falls, and stop at -100 % (EW2-01).
    for level in 1..=3 {
        let (plain, settings) = weighted_cube("subdiv", 0.0, level);
        let base = subdivide(&plain, &settings).unwrap();
        let mut last = gap_to_edge(&base);
        for weight in [-0.1, -0.25, -0.5, -1.0] {
            let (cage, settings) = weighted_cube("subdiv", weight, level);
            let out = subdivide(&cage, &settings).unwrap();
            assert!(out.is_closed());
            let gap = gap_to_edge(&out);
            assert!(
                gap > last + 1e-6,
                "level {level} weight {weight}: {gap} vs {last}"
            );
            last = gap;
        }
        let (cage, settings) = weighted_cube("subdiv", -3.0, level);
        let (full, _) = weighted_cube("subdiv", -1.0, level);
        assert_eq!(
            subdivide(&cage, &settings).unwrap().vertices,
            subdivide(&full, &settings).unwrap().vertices
        );
    }
}

#[test]
fn catmull_clark_ignores_negative_weights() {
    // EW2-02: on Catmull-Clark every negative weight is the plain surface.
    for level in 1..=3 {
        let (plain, settings) = weighted_cube("catmull-clark", 0.0, level);
        let base = subdivide(&plain, &settings).unwrap();
        for weight in [-0.5, -1.0, -3.0] {
            let (cage, settings) = weighted_cube("catmull-clark", weight, level);
            assert_eq!(subdivide(&cage, &settings).unwrap().vertices, base.vertices);
        }
    }
}

#[test]
fn mixed_smooth_kinds_stay_connected_as_catmull_clark() {
    // SDV-14 (Operandi choice): a region mixing both kinds is all
    // Catmull-Clark, so no gap opens between them.
    let o = oracles();
    let (mixed, settings) = cage(&o["SDV-O-MIX-subd3-psub3-L1"]["input"]);
    let out = subdivide(&mixed, &settings).unwrap();
    assert!(out.is_closed());
    let mut all_cc = mixed.clone();
    all_cc.attributes.representations =
        vec![PolygonRepresentation::CatmullClark; mixed.faces.len()];
    assert_eq!(
        out.vertices,
        subdivide(&all_cc, &settings).unwrap().vertices
    );
}

#[test]
fn freeze_keeps_weights_only_on_face_polygon_edges() {
    // EW-10: one Face polygon and five smooth ones, weights on a Face edge
    // and a smooth-only edge.
    let o = oracles();
    let (mut mesh, settings) = cage(&o["SDV-O-MIX-psub5-face1-L1"]["input"]);
    let face = (0..mesh.faces.len())
        .find(|&f| kind(&mesh, f) == PolygonRepresentation::Face)
        .unwrap();
    let fv = mesh.faces[face].vertices.clone();
    let on_face = edge_key(fv[0], fv[1]);
    let smooth_only = mesh
        .edges()
        .into_iter()
        .find(|&(a, b)| !fv.contains(&a) && !fv.contains(&b))
        .unwrap();
    for (a, b) in [on_face, smooth_only] {
        mesh.attributes
            .creases
            .push(MeshCrease { a, b, weight: 0.4 });
    }
    mesh.attributes.creases.sort_by_key(|c| (c.a, c.b));
    let frozen = freeze(&mesh, &settings).unwrap();
    assert_eq!(frozen.attributes.creases.len(), 1);
    let c = &frozen.attributes.creases[0];
    assert_eq!(c.weight, 0.4);
    let (pa, pb) = (frozen.vertices[c.a as usize], frozen.vertices[c.b as usize]);
    let (qa, qb) = (
        mesh.vertices[on_face.0 as usize],
        mesh.vertices[on_face.1 as usize],
    );
    assert!((pa == qa && pb == qb) || (pa == qb && pb == qa));
}

/// A 10 x 10 open grid with every fourth quad split into two triangles
/// (valence-5 and valence-3 points), next to a closed 3 x 3 x 3 box
/// (valence-3 corners), so partial updates meet irregular points, triangles,
/// borders and creases.
fn irregular_cage(kind: impl Fn(usize) -> PolygonRepresentation) -> Mesh {
    let n = 10u32;
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(
                i as f64,
                j as f64,
                ((i * 5 + j * 3) % 7) as f64 * 0.13,
            ));
        }
    }
    let face = |vertices: Vec<u32>| Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    };
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            let (b, c, d) = (a + 1, a + n + 2, a + n + 1);
            if (i + j * n) % 4 == 1 {
                mesh.faces.push(face(vec![a, b, c]));
                mesh.faces.push(face(vec![a, c, d]));
            } else {
                mesh.faces.push(face(vec![a, b, c, d]));
            }
        }
    }
    let mut cube = Mesh::box_mesh(DVec3::new(20., 5., 0.), DVec3::splat(6.), 0).unwrap();
    for _ in 0..1 {
        cube = super::subdivide(
            &toggle(&cube, &[], Toggle::CatmullClark).unwrap(),
            &SubdivisionSettings {
                catmull_clark_level: 1,
                ..Default::default()
            },
        )
        .unwrap();
    }
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend(&cube.vertices);
    for f in &cube.faces {
        mesh.faces
            .push(face(f.vertices.iter().map(|&v| v + base).collect()));
    }
    mesh.attributes.representations = (0..mesh.faces.len()).map(kind).collect();
    mesh.attributes.creases = vec![
        MeshCrease {
            a: 12,
            b: 13,
            weight: 0.6,
        },
        MeshCrease {
            a: 13,
            b: 24,
            weight: -0.4,
        },
        MeshCrease {
            a: base,
            b: base + 1,
            weight: 1.0,
        },
    ];
    mesh
}

#[test]
fn partial_updates_match_full_rebuilds() {
    // Every polygon kind mix, level and boundary rule; the partial surface
    // must equal a full rebuild exactly, point for point.
    let kinds: [fn(usize) -> PolygonRepresentation; 3] = [
        |_| PolygonRepresentation::CatmullClark,
        |_| PolygonRepresentation::Subdivs,
        |f| match f % 5 {
            0 => PolygonRepresentation::Face,
            1 => PolygonRepresentation::Subdivs,
            _ => PolygonRepresentation::CatmullClark,
        },
    ];
    for kind in kinds {
        let mesh = irregular_cage(kind);
        for level in 1..=3 {
            for rule in [
                BoundaryRule::Crease,
                BoundaryRule::Smooth,
                BoundaryRule::CreaseEdges,
            ] {
                let settings = SubdivisionSettings {
                    catmull_clark_level: level,
                    subdivision_level: level,
                    boundary_rule: rule,
                    ..Default::default()
                };
                let (mut surface, sources) = display_surface(&mesh, &settings).unwrap();
                let blocks = source_blocks(&sources, mesh.faces.len());
                let mut old = mesh.clone();
                // Single points, a whole row, a crease end and the box.
                let moves: [&[u32]; 4] = [&[56], &[33, 34, 35, 36, 37, 38], &[13], &[125, 130]];
                for (k, points) in moves.iter().enumerate() {
                    let mut new = old.clone();
                    for &v in *points {
                        new.vertices[v as usize] += DVec3::new(0.21, -0.13 * k as f64, 0.37);
                    }
                    update_surface(&old, &new, &settings, &mut surface, &blocks)
                        .expect("a local move updates in place");
                    let (full, _) = display_surface(&new, &settings).unwrap();
                    assert_eq!(full.faces, surface.faces);
                    for (i, (a, b)) in full.vertices.iter().zip(&surface.vertices).enumerate() {
                        assert!(
                            a.to_array().map(f64::to_bits) == b.to_array().map(f64::to_bits),
                            "{:?} level {level} {rule:?} move {k}: point {i} {a} vs {b}",
                            kind(1)
                        );
                    }
                    old = new;
                }
            }
        }
    }
}
