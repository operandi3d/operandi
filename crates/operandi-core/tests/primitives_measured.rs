//! Primitive measured cases (specifications/primitives-2026-11-02). Cases
//! whose options Operandi has are compared by geometry (same positions and
//! polygon shapes); Operandi numbers points and polygons its own way, which
//! is logged as a deliberate difference. Options Operandi doesn't have yet
//! are listed in MISSING.
#![cfg_attr(not(operandi_specs), allow(dead_code, unused_imports))]
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;
fn vec3(v: &Value) -> DVec3 {
    let a: Vec<f64> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    DVec3::new(a[0], a[1], a[2])
}
fn shape(points: &[DVec3], polygon: &[u32]) -> Vec<[i64; 3]> {
    let q: Vec<[i64; 3]> = polygon
        .iter()
        .map(|&v| {
            points[v as usize]
                .to_array()
                .map(|x| (x * 1e4).round() as i64)
        })
        .collect();
    let start = (0..q.len()).min_by_key(|&i| q[i]).unwrap_or(0);
    (0..q.len()).map(|k| q[(start + k) % q.len()]).collect()
}
/// Options not built yet: rounded/bulged Cube, UV Gaps and Compensation on
/// the Cube, Quad Ball, compensated Ellipsoid UVs.
const MISSING: usize = 12;

#[test]
#[cfg(operandi_specs)]
fn primitive_measured_cases() {
    let doc: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("OPERANDI_SPECS_DIR"),
            "/specifications/primitives-2026-11-02/ORACLES.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut matched = Vec::new();
    let mut missing = 0;
    let mut failures = Vec::new();
    // Batch 3 cases (PRIM-O-PR3-*) are for options not built yet.
    for o in doc["oracles"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| !o["id"].as_str().unwrap().starts_with("PRIM-O-PR3"))
    {
        let s = &o["settings"];
        let f = |k: &str, d: f64| s[k].as_f64().unwrap_or(d);
        let id = o["id"].as_str().unwrap();
        let mesh: std::result::Result<Mesh, String> = match o["tool"].as_str().unwrap() {
            "Cube"
                if s.as_object()
                    .unwrap()
                    .keys()
                    .all(|k| k.starts_with("size") || k.starts_with("segments")) =>
            {
                let size = DVec3::new(f("sizeX", 1.), f("sizeY", 1.), f("sizeZ", 1.)).abs();
                primitive::segmented_cube(
                    DVec3::ZERO,
                    size,
                    [
                        f("segmentsX", 1.) as u32,
                        f("segmentsY", 1.) as u32,
                        f("segmentsZ", 1.) as u32,
                    ],
                )
            }
            "Sphere" if s["method"] == "tess" => tessellated_sphere(
                DVec3::ZERO,
                DVec3::splat(0.5),
                f("order", 0.) as u32,
                1,
                false,
            ),
            "Sphere" if s.get("method").is_none() => globe_geometry_with(
                DVec3::ZERO,
                RadialParameters {
                    radius: DVec3::splat(0.5),
                    sides: f("sides", 24.) as u32,
                    segments: f("segments", 12.) as u32,
                },
                f("axis", 1.) as usize,
                false,
            ),
            // PRIM-Y-02: Edge Weight doesn't apply to Face polygons; the
            // cylinder is made without it.
            "Cylinder" => {
                let cap = |k: &str| match s[k].as_str() {
                    Some("none") => CylinderCap::None,
                    Some("quads") => CylinderCap::Quads,
                    Some("radial") => CylinderCap::Radial,
                    _ => CylinderCap::Single,
                };
                cylinder_geometry_with(
                    DVec3::ZERO,
                    RadialParameters {
                        radius: DVec3::new(0.5, 0.75, 0.5),
                        sides: f("sides", 24.) as u32,
                        segments: f("segments", 1.) as u32,
                    },
                    [cap("capBottom"), cap("capTop")],
                    f("axis", 1.) as usize,
                    false,
                )
            }
            "Cone" => cone_geometry(&ConeParameters {
                sides: f("sides", 24.) as u32,
                size: DVec3::new(0.5, f("sizeY", 0.75), 0.5),
                bottom_cap: if s["capBottom"] == "quads" {
                    ConeCap::QuadGrid
                } else {
                    ConeCap::Single
                },
                make_uvs: false,
                ..Default::default()
            }),
            "Ellipsoid" if s.get("uvCompensation").is_none() => {
                ellipsoid_geometry(&EllipsoidParameters {
                    radius: DVec3::splat(0.5),
                    sides: 8,
                    segments: 4,
                    make_uvs: false,
                    ..Default::default()
                })
            }
            _ => {
                missing += 1;
                continue;
            }
        };
        let Ok(m) = mesh else {
            missing += 1;
            continue;
        };
        let x = &o["expected"];
        let pts: Vec<DVec3> = x["points"].as_array().unwrap().iter().map(vec3).collect();
        let polys: Vec<Vec<u32>> = x["polygons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                p.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect()
            })
            .collect();
        let mut a: Vec<_> = m
            .faces
            .iter()
            .map(|f| shape(&m.vertices, &f.vertices))
            .collect();
        let mut b: Vec<_> = polys.iter().map(|f| shape(&pts, f)).collect();
        a.sort();
        b.sort();
        if m.vertices.len() == pts.len() && a == b {
            matched.push(id);
        } else {
            failures.push(id);
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(
        (matched.len(), missing),
        (20, MISSING),
        "{matched:?} {missing}"
    );
}

/// PRIM-Y-01 through the editor: caps and axis change the live cylinder,
/// UVs cover every new polygon, Drop then Undo leaves the mesh empty, and the
/// next Cylinder starts with the same caps and axis until Reset.
#[test]
fn cylinder_caps_and_axis_through_the_editor() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Cylinder)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.radial = Some(RadialParameters {
        radius: DVec3::new(0.5, 0.75, 0.5),
        sides: 8,
        segments: 1,
    });
    p.cylinder_caps = [CylinderCap::Quads, CylinderCap::Radial];
    p.radial_axis = 0;
    e.dispatch(Command::Preview(p)).unwrap();
    let mesh = e.scene.active().unwrap().mesh.to_owned_value();
    // 16 rim + 8 inner + centre (Quads bottom) + centre (Radial top).
    assert_eq!((mesh.vertices.len(), mesh.faces.len()), (26, 8 + 12 + 8));
    let x: Vec<f64> = mesh.vertices.iter().map(|v| v.x).collect();
    assert!(x.iter().all(|x| (x.abs() - 0.5).abs() < 1e-9));
    let uvs = &mesh.uv_maps["Texture"];
    assert_eq!(uvs.len(), mesh.faces.len());
    assert!(
        uvs.iter()
            .flatten()
            .all(|uv| uv.is_some_and(|uv| uv.is_finite()))
    );
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.active().unwrap().mesh.vertices.is_empty());
    e.dispatch(Command::Activate(Tool::Cylinder)).unwrap();
    let p = &e.session.live.as_ref().unwrap().params;
    assert_eq!(p.cylinder_caps, [CylinderCap::Quads, CylinderCap::Radial]);
    assert_eq!(p.radial_axis, 0);
    e.dispatch(Command::ResetPrimitive).unwrap();
    let p = &e.session.live.as_ref().unwrap().params;
    assert_eq!(p.cylinder_caps, [CylinderCap::Single; 2]);
    assert_eq!(p.radial_axis, 1);
}

/// PRIM-S-02 through the editor: Tessellated Level 1 on Axis Z gives 42
/// points / 80 triangles with UVs, and the next Sphere starts the same way.
#[test]
fn tessellated_sphere_through_the_editor() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Sphere)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.radial = Some(RadialParameters {
        radius: DVec3::new(0.5, 0.5, 1.),
        sides: 8,
        segments: 4,
    });
    p.sphere_level = Some(1);
    p.radial_axis = 2;
    e.dispatch(Command::Preview(p)).unwrap();
    let mesh = e.scene.active().unwrap().mesh.to_owned_value();
    assert_eq!((mesh.vertices.len(), mesh.faces.len()), (42, 80));
    assert!(mesh.vertices.iter().any(|v| (v.z - 1.).abs() < 1e-9));
    let uvs = &mesh.uv_maps["Texture"];
    assert!(
        uvs.iter()
            .flatten()
            .all(|uv| uv.is_some_and(|uv| uv.is_finite()))
    );
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Activate(Tool::Sphere)).unwrap();
    let p = &e.session.live.as_ref().unwrap().params;
    assert_eq!((p.sphere_level, p.radial_axis), (Some(1), 2));
}
