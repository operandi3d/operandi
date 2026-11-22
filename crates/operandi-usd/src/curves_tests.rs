//! USD-IQ24: polylines and curves as BasisCurves / NurbsCurves, both
//! ways, through OpenUSD.
use glam::DVec3;
use operandi_core::curves::{CurveKind, sample};
use operandi_core::interchange::{ExportFormat, Imported};
use operandi_core::{Face, Mesh, PolygonRepresentation, Scene, TypedMap};
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("operandi-usd-curves")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn import(path: &Path) -> Imported {
    crate::import(path, &Default::default()).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: true,
        part: None,
        material_label: None,
    }
}

/// Every kind of line element, open and closed, beside one quad.
fn every_kind() -> Vec<(bool, CurveKind, usize)> {
    let curve = |s, e| CurveKind::Curve {
        start_control: s,
        end_control: e,
    };
    vec![
        (false, CurveKind::Line, 3),
        (true, CurveKind::Line, 4),
        (false, curve(false, false), 4),
        (false, curve(true, false), 5),
        (false, curve(false, true), 5),
        (false, curve(true, true), 6),
        (true, curve(false, false), 5),
        (false, CurveKind::BSpline, 6),
        (true, CurveKind::BSpline, 5),
        (false, CurveKind::Bezier, 7),
        (true, CurveKind::Bezier, 6),
    ]
}

fn curves_scene() -> Scene {
    let mut scene = Scene::default();
    scene.items.clear();
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.name = "Shapes".into();
    let mut mesh = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 0., 1.),
            DVec3::new(0., 0., 1.),
        ],
        ..Default::default()
    };
    let mut faces = vec![face(vec![0, 1, 2, 3])];
    let mut reps = vec![PolygonRepresentation::Face];
    for (k, (closed, curve, n)) in every_kind().into_iter().enumerate() {
        let base = mesh.vertices.len() as u32;
        for i in 0..n {
            let a = i as f64 / n as f64 * std::f64::consts::TAU;
            mesh.vertices.push(DVec3::new(
                a.cos() * (1.0 + 0.1 * i as f64),
                k as f64 * 0.5 + 0.2 * (i % 3) as f64,
                a.sin(),
            ));
        }
        faces.push(face((base..base + n as u32).collect()));
        reps.push(PolygonRepresentation::Polyline { closed, curve });
    }
    let widths = TypedMap {
        kind: "weight1".into(),
        dimension: 1,
        zero_default: false,
        points: (4..mesh.vertices.len() as u32)
            .map(|v| (v, vec![0.01 * v as f64]))
            .collect(),
        corners: Vec::new(),
    };
    mesh.faces = faces.into();
    let a = &mut *mesh.attributes;
    a.representations = reps;
    a.typed_maps.insert("widths".into(), widths);
    item.mesh = mesh.into();
    scene
}

/// Each line element's kind, closure and drawn curve (36 steps a span),
/// sorted so element order doesn't matter.
fn drawn(mesh: &Mesh) -> Vec<(String, Vec<DVec3>)> {
    let mut out: Vec<(String, Vec<DVec3>)> = (0..mesh.faces.len())
        .filter_map(|f| match mesh.attributes.representations.get(f) {
            Some(PolygonRepresentation::Polyline { closed, curve }) => {
                let pts: Vec<DVec3> = mesh.faces[f]
                    .vertices
                    .iter()
                    .map(|&v| mesh.vertices[v as usize])
                    .collect();
                Some((
                    format!("{curve:?} {closed}"),
                    sample(&pts, *curve, *closed, 36),
                ))
            }
            _ => None,
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1[0].x.total_cmp(&b.1[0].x)));
    out
}

fn same(a: &[DVec3], b: &[DVec3]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.distance(*y) < 1e-5)
}

/// Every kind comes back the same kind, drawing the same curve, with its
/// widths, and other programs see BasisCurves and NurbsCurves.
#[test]
fn every_curve_kind_round_trips() {
    let scene = curves_scene();
    let dir = scratch("round-trip");
    let path = dir.join("curves.usda");
    let report = crate::export(&path, &scene, ExportFormat::Usda).unwrap();
    assert!(
        !report.notes.iter().any(|n| n.contains("not exported")),
        "{:?}",
        report.notes
    );
    let text = std::fs::read_to_string(&path).unwrap();
    for want in [
        "def BasisCurves \"Shapes_lines\"",
        "def BasisCurves \"Shapes_catmullRom\"",
        "def BasisCurves \"Shapes_bezier\"",
        "def BasisCurves \"Shapes_bspline_closed\"",
        "def BasisCurves \"Shapes_bezier_closed\"",
        "def NurbsCurves \"Shapes_nurbs\"",
        "uniform token type = \"cubic\"",
        "uniform token wrap = \"periodic\"",
        "float[] widths = ",
        "def Mesh \"Shapes\"",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
    let back = import(&path);
    let item = back.scene.items.iter().find(|i| i.is_mesh()).unwrap();
    assert_eq!(item.name, "Shapes");
    let (a, b) = (drawn(&scene.items[0].mesh), drawn(&item.mesh));
    assert_eq!(a.len(), every_kind().len());
    for ((ka, pa), (kb, pb)) in a.iter().zip(&b) {
        assert_eq!(ka, kb);
        assert!(same(pa, pb), "{ka}");
    }
    // The quad is still there, and widths came back per point.
    assert_eq!(
        item.mesh
            .faces
            .iter()
            .enumerate()
            .filter(|(f, _)| !item.mesh.is_polyline(*f))
            .count(),
        1
    );
    let w = &item.mesh.attributes.typed_maps["widths"];
    assert_eq!(w.kind, "weight1");
    assert!(
        w.points.values().any(|v| (v[0] - 0.04).abs() < 1e-6),
        "{w:?}"
    );
    // The report counts curves, not polygons.
    assert!(back.notes[0].contains("1 polygon(s)"), "{:?}", back.notes);
    assert!(
        back.notes
            .iter()
            .any(|n| n.contains("11 polyline(s) and curve(s)")),
        "{:?}",
        back.notes
    );
}

/// A file of only curves exports and imports.
#[test]
fn a_scene_of_only_curves_exports() {
    let mut scene = curves_scene();
    let item = &mut scene.items[0];
    let mesh: &mut Mesh = item.mesh.shared_mut();
    let mut faces: Vec<Face> = mesh.faces.iter().cloned().collect();
    faces.remove(0);
    mesh.faces = faces.into();
    mesh.attributes.representations.remove(0);
    let path = scratch("only").join("only.usdc");
    crate::export(&path, &scene, ExportFormat::Usdc).unwrap();
    let back = import(&path);
    assert_eq!(drawn(&back.scene.items[0].mesh).len(), every_kind().len());
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/usd/curves")
        .join(name)
}

/// Hand-written curves of every USD type, basis and wrap
/// (fixtures/usd/curves/every_basis.usda) come in as the curve each draws.
#[test]
fn other_programs_curves_come_in_as_the_curves_they_draw() {
    let imported = import(&fixture("every_basis.usda"));
    let by_name = |n: &str| {
        let i = imported
            .scene
            .items
            .iter()
            .find(|i| i.name == n)
            .unwrap_or_else(|| panic!("no {n}"));
        drawn(&i.mesh)
    };
    let kinds = |n: &str| by_name(n).into_iter().map(|(k, _)| k).collect::<Vec<_>>();
    assert_eq!(kinds("lines"), ["Line false", "Line true"]);
    assert_eq!(kinds("bezier"), ["Bezier false", "Bezier true"]);
    assert_eq!(kinds("bspline_periodic"), ["BSpline true"]);
    // An open uniform B-Spline is the same curve as a Bezier: it starts
    // at (P0 + 4 P1 + P2) / 6.
    let open = by_name("bspline_open");
    assert_eq!(open[0].0, "Bezier false");
    assert!(
        open[0].1[0].distance(DVec3::new(1.0, 0.0, 0.0)) < 1e-9,
        "{:?}",
        open[0].1[0]
    );
    // Pinned: through the end points.
    let pinned = by_name("bspline_pinned");
    assert!(pinned[0].1[0].distance(DVec3::ZERO) < 1e-9);
    assert!(
        pinned[0]
            .1
            .last()
            .unwrap()
            .distance(DVec3::new(3.0, 0.0, 0.0))
            < 1e-9
    );
    let control = "Curve { start_control: true, end_control: true } false";
    assert_eq!(kinds("catmull"), [control]);
    // Pinned Catmull-Rom passes through its first and last points.
    let pc = by_name("catmull_pinned");
    assert_eq!(pc[0].0, control);
    assert!(pc[0].1[0].distance(DVec3::ZERO) < 1e-9);
    assert_eq!(
        kinds("catmull_periodic"),
        ["Curve { start_control: false, end_control: false } true"]
    );
    assert_eq!(kinds("nurbs_clamped"), ["BSpline false"]);
    assert_eq!(kinds("nurbs_weighted"), ["Line false"]);
    assert!(
        imported
            .notes
            .iter()
            .any(|n| n.contains("sampled into polylines")),
        "{:?}",
        imported.notes
    );
    // Constant widths: on every point.
    let item = imported
        .scene
        .items
        .iter()
        .find(|i| i.name == "lines")
        .unwrap();
    let w = &item.mesh.attributes.typed_maps["widths"];
    assert_eq!(w.points.len(), item.mesh.vertices.len());
    assert!(w.points.values().all(|v| (v[0] - 0.05).abs() < 1e-6));
}

/// Blender 5.2.2's Bezier circle, NURBS path and poly line
/// (fixtures/usd/curves/make_blender_curves.py) come in, and Operandi's
/// export of them reads back the same.
#[test]
fn blender_curves_round_trip() {
    let imported = import(&fixture("blender_curves.usda"));
    let mut all: Vec<(String, Vec<DVec3>)> = imported
        .scene
        .items
        .iter()
        .flat_map(|i| drawn(&i.mesh))
        .collect();
    all.sort_by(|a, b| a.0.cmp(&b.0));
    let kinds: Vec<&str> = all.iter().map(|(k, _)| k.as_str()).collect();
    assert!(kinds.contains(&"Bezier true"), "{kinds:?}");
    assert!(kinds.iter().any(|k| k.starts_with("Line")), "{kinds:?}");
    // The Bezier circle (radius 1, Z up): every drawn point near radius 1.
    let circle = &all.iter().find(|(k, _)| k == "Bezier true").unwrap().1;
    let item = imported
        .scene
        .items
        .iter()
        .find(|i| drawn(&i.mesh).iter().any(|(k, _)| k == "Bezier true"))
        .unwrap();
    let w = imported.scene.world_matrix(item.id).unwrap();
    for p in circle {
        let r = (w.transform_point3(*p) - w.transform_point3(DVec3::ZERO)).length();
        assert!((r - 1.0).abs() < 0.01, "radius {r}");
    }
    let path = scratch("blender").join("again.usda");
    crate::export(&path, &imported.scene, ExportFormat::Usda).unwrap();
    let again = import(&path);
    let mut back: Vec<(String, Vec<DVec3>)> = again
        .scene
        .items
        .iter()
        .flat_map(|i| drawn(&i.mesh))
        .collect();
    back.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(all.len(), back.len());
    for ((ka, pa), (kb, pb)) in all.iter().zip(&back) {
        assert_eq!(ka, kb);
        assert!(same(pa, pb), "{ka}");
    }
}
