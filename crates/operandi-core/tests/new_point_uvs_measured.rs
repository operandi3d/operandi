//! UVs on new points, measured (specifications/new-point-uvs-2026-11-02).
//! Every output polygon is compared as its corners' positions and UVs, in
//! any polygon order (Operandi numbers new points its own way).
use glam::{DVec2, DVec3};
use operandi_core::*;
use serde_json::Value;
use std::collections::BTreeMap;

fn oracles() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specifications/new-point-uvs-2026-11-02/ORACLES.json"
    ))
    .unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    doc["oracles"].as_array().unwrap().clone()
}
fn num(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}
fn vec3(v: &Value) -> DVec3 {
    let a = num(v);
    DVec3::new(a[0], a[1], a[2])
}
fn list(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect()
}
type Corner = ([i64; 3], [i64; 2]);
fn round3(p: DVec3) -> [i64; 3] {
    p.to_array().map(|x| (x * 1e4).round() as i64)
}
fn round2(p: DVec2) -> [i64; 2] {
    p.to_array().map(|x| (x * 1e4).round() as i64)
}
/// A polygon's corners rotated to start at the smallest position.
fn canonical(corners: Vec<Corner>) -> Vec<Corner> {
    let start = (0..corners.len()).min_by_key(|&i| corners[i]).unwrap_or(0);
    (0..corners.len())
        .map(|k| corners[(start + k) % corners.len()])
        .collect()
}

/// The fixture as a mesh: every corner's UV, and with `points` also each
/// point's continuous UV (the fixture's per-point values, which the seam
/// polygons override).
fn input_mesh(o: &Value, points: bool) -> Mesh {
    let input = &o["input"];
    let faces: Vec<Vec<u32>> = input["polygons"]
        .as_array()
        .unwrap()
        .iter()
        .map(list)
        .collect();
    let mut point_uvs = BTreeMap::new();
    for (k, v) in input["uv_continuous_per_point"].as_object().unwrap() {
        let a = num(v);
        point_uvs.insert(k.parse::<u32>().unwrap(), DVec2::new(a[0], a[1]));
    }
    let overrides = input["uv_per_polygon_corner"].as_object();
    let corners: Vec<Vec<Option<DVec2>>> = faces
        .iter()
        .enumerate()
        .map(|(f, vs)| {
            vs.iter()
                .map(|v| {
                    overrides
                        .and_then(|m| m.get(&format!("{f},{v}")))
                        .map(|x| {
                            let a = num(x);
                            DVec2::new(a[0], a[1])
                        })
                        .or(Some(point_uvs[v]))
                })
                .collect()
        })
        .collect();
    let mut mesh = Mesh {
        vertices: input["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(vec3)
            .collect(),
        faces: faces
            .into_iter()
            .map(|vertices| Face {
                vertices,
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    };
    mesh.uv_maps.insert("Texture".into(), corners);
    if points {
        mesh.attributes
            .point_uvs
            .insert("Texture".into(), point_uvs);
    }
    mesh
}

fn run(o: &Value, points: bool) -> std::result::Result<Mesh, String> {
    let mesh = input_mesh(o, points);
    let s = &o["settings"];
    let sel = &o["selection"];
    let edges: Vec<(u32, u32)> = sel["edge"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|e| {
                    let e = list(e);
                    (e[0].min(e[1]), e[0].max(e[1]))
                })
                .collect()
        })
        .unwrap_or_default();
    let get = |k: &str| s[k].as_f64().unwrap_or(0.);
    Ok(match s["tool"].as_str().unwrap() {
        "Edge Bevel" => edge_bevel_geometry(&mesh, &edges, get("inset"), 1, 100., false)?.0,
        "Edge Chamfer" => edge_bevel_geometry(&mesh, &edges, get("offset"), 1, 100., true)?.0,
        "Edge Extrude" => edge_extrude_geometry(&mesh, &edges, get("shift"), get("inset"))?.0,
        "Vertex Bevel" => vertex_bevel_geometry(&mesh, &list(&sel["vert"]), get("inset"))?.0,
        "Vertex Extrude" => {
            vertex_extrude_geometry(&mesh, &list(&sel["vert"]), get("shift"), get("inset"))?.0
        }
        "Grow Quads" => edge_command_geometry(&mesh, &edges, EdgeCommand::GrowQuads)?.0,
        "Fill Quads" => edge_command_geometry(&mesh, &edges, EdgeCommand::FillQuads)?.0,
        "Bridge" => edge_bridge_geometry(&mesh, &edges, get("segments") as u32, 0)?.0,
        "Chamfer Edit" => chamfer_geometry(
            &mesh,
            &list(&sel["poly"]),
            ChamferParameters {
                offset: get("offset"),
                segments: get("segments") as u32,
                source_segments: 1,
                depth: get("depth"),
                ..Default::default()
            },
        )?,
        other => return Err(format!("unknown tool {other}")),
    })
}

fn got_corners(mesh: &Mesh) -> Vec<Vec<Corner>> {
    let mut out: Vec<_> = (0..mesh.faces.len())
        .map(|f| {
            canonical(
                mesh.faces[f]
                    .vertices
                    .iter()
                    .enumerate()
                    .map(|(c, &v)| {
                        let uv = extrude::effective_uv(mesh, "Texture", f, c)
                            .unwrap_or(DVec2::splat(f64::NAN));
                        (round3(mesh.vertices[v as usize]), round2(uv))
                    })
                    .collect(),
            )
        })
        .collect();
    out.sort();
    out
}
fn want_corners(o: &Value) -> Vec<Vec<Corner>> {
    let x = &o["expected"];
    let points: Vec<DVec3> = x["points"].as_array().unwrap().iter().map(vec3).collect();
    let uvs = x["uv_per_polygon_corner"].as_array().unwrap();
    let mut out: Vec<_> = x["polygons"]
        .as_array()
        .unwrap()
        .iter()
        .zip(uvs)
        .map(|(p, uv)| {
            canonical(
                list(p)
                    .iter()
                    .zip(uv.as_array().unwrap())
                    .map(|(&v, t)| {
                        let t = num(t);
                        (round3(points[v as usize]), round2(DVec2::new(t[0], t[1])))
                    })
                    .collect(),
            )
        })
        .collect();
    out.sort();
    out
}

/// Cases whose UVs match the reference corner for corner. The others are
/// logged in SPECIFICATION_QUESTIONS.md (New-point UVs): Edge Chamfer ends
/// (the reference adds zero-area triangles), Fill Quads at a seam (see below),
/// Bridge Segments and flat Chamfer Edit build different geometry.
const MATCHING: &[&str] = &[
    "UVN-O-bridge-cont",
    "UVN-O-bridge-seam",
    "UVN-O-chamfer-edit-cont",
    "UVN-O-chamfer-edit-seam",
    "UVN-O-edge-bevel-cont",
    "UVN-O-edge-bevel-seam",
    "UVN-O-edge-extrude-cont",
    "UVN-O-edge-extrude-seam",
    "UVN-O-fill-quads-cont",
    "UVN-O-grow-quads-cont",
    "UVN-O-grow-quads-seam",
    "UVN-O-vertex-bevel-cont",
    "UVN-O-vertex-bevel-seam",
    "UVN-O-vertex-extrude-cont",
    "UVN-O-vertex-extrude-seam",
];
/// Matching cases that need the points' continuous UVs, which a mesh with
/// only corner UVs doesn't keep. None now: Grow Quads' border corners
/// (UVN-T-06) recover them across the seam's constant shift, since at
/// point 15 no polygon has that value.
const NEEDS_POINT_UVS: &[&str] = &[];

fn status(o: &Value, points: bool) -> String {
    match run(o, points) {
        Ok(mesh) => {
            let (got, want) = (got_corners(&mesh), want_corners(o));
            if got == want {
                "match".to_string()
            } else if got.len() != want.len() {
                format!("{} polygons vs {}", got.len(), want.len())
            } else {
                let geo = |c: &Vec<Vec<Corner>>| {
                    let mut g: Vec<Vec<[i64; 3]>> =
                        c.iter().map(|p| p.iter().map(|x| x.0).collect()).collect();
                    g.sort();
                    g
                };
                if geo(&got) == geo(&want) {
                    "uvs differ"
                } else {
                    "geometry differs"
                }
                .into()
            }
        }
        Err(e) => format!("error: {e}"),
    }
}

/// Every case, as the fixture gives it (point UVs plus seam overrides) and
/// as corner UVs only (how Operandi's own and imported meshes hold UVs).
#[test]
fn new_point_uv_measured_cases() {
    let mut report = Vec::new();
    for o in oracles() {
        let id = o["id"].as_str().unwrap();
        for points in [true, false] {
            // Batch 2's cases aren't replayed here; Chamfer Edit refuses
            // meshes with point UVs (it can't remap them yet).
            let chamfer_edit = o["settings"]["tool"] == "Chamfer Edit";
            if (id.starts_with("UVN-O-UV2") && !points) || (chamfer_edit && points) {
                continue;
            }
            let status = status(&o, points);
            let expected = MATCHING.contains(&id) && (points || !NEEDS_POINT_UVS.contains(&id));
            if (status == "match") != expected {
                report.push(format!("{id} (point UVs {points}): {status}"));
            }
        }
    }
    assert!(report.is_empty(), "{}", report.join("\n"));
}

/// UVN-O-fill-quads-seam (FQ-Q01): the three new polygons match the
/// reference corner for corner, with point UVs or corner UVs only. The one
/// difference left is held back: the reference also moves the untouched
/// polygon [2, 3, 8, 7]'s corner at point 8 across the seam (u 1 to 2).
#[test]
fn fill_quads_seam_new_polygons_match() {
    let o = oracles()
        .into_iter()
        .find(|o| o["id"] == "UVN-O-fill-quads-seam")
        .unwrap();
    for points in [true, false] {
        let got = got_corners(&run(&o, points).unwrap());
        let want = want_corners(&o);
        let missing: Vec<_> = want.iter().filter(|w| !got.contains(w)).collect();
        assert_eq!(missing.len(), 1, "point UVs {points}: {missing:?}");
        let corner_8 = missing[0]
            .iter()
            .find(|c| c.0 == [30000, 10000, 0])
            .unwrap();
        assert_eq!(corner_8.1, [20000, 3333], "point UVs {points}");
        assert!(
            missing[0].iter().all(|c| c.0[1] <= 10000),
            "only the polygon below the hole differs"
        );
    }
}
