//! UV stage 5 and 6 (BEHAVIOR-UV-TOOLS §2.4 Atlas types, §5.1 Peeler,
//! §12 Transfer, §15 Export): automatic charts, the quad-strip grid, UV
//! layout drawings, Convert UVs to Mesh and Transfer UVs.
use glam::{DVec2, DVec3};
use operandi_core::uv_export::{LayoutFormat, uv_layout, uvs_to_mesh};
use operandi_core::uv_project::ProjectionKind;
use operandi_core::*;

fn face(v: &[u32]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn editor_with(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

/// A UV sphere: 16 sides, 8 bands, triangle fans at the poles.
fn sphere() -> Mesh {
    let (sides, bands) = (16u32, 8u32);
    let mut m = Mesh::default();
    m.vertices.push(DVec3::new(0., -1., 0.));
    for b in 1..bands {
        let t = std::f64::consts::PI * b as f64 / bands as f64 - std::f64::consts::FRAC_PI_2;
        for s in 0..sides {
            let a = std::f64::consts::TAU * s as f64 / sides as f64;
            m.vertices
                .push(DVec3::new(t.cos() * a.cos(), t.sin(), t.cos() * a.sin()));
        }
    }
    m.vertices.push(DVec3::new(0., 1., 0.));
    let top = m.vertices.len() as u32 - 1;
    let ring = |b: u32, s: u32| 1 + (b - 1) * sides + s % sides;
    for s in 0..sides {
        m.faces.push(face(&[0, ring(1, s + 1), ring(1, s)]));
    }
    for b in 1..bands - 1 {
        for s in 0..sides {
            m.faces.push(face(&[
                ring(b, s),
                ring(b, s + 1),
                ring(b + 1, s + 1),
                ring(b + 1, s),
            ]));
        }
    }
    for s in 0..sides {
        m.faces
            .push(face(&[ring(bands - 1, s), ring(bands - 1, s + 1), top]));
    }
    m
}

fn run(e: &mut Editor, tool: Tool, edit: impl FnOnce(&mut Parameters)) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    edit(&mut p);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
}

fn in_tile(m: &Mesh) -> bool {
    m.uv_maps["Texture"]
        .iter()
        .flatten()
        .flatten()
        .all(|p| p.x >= -1e-6 && p.y >= -1e-6 && p.x <= 1. + 1e-6 && p.y <= 1. + 1e-6)
}

fn islands(m: &Mesh) -> usize {
    let faces: Vec<Vec<(u32, DVec2)>> = m
        .faces
        .iter()
        .zip(m.uv_maps["Texture"].iter())
        .map(|(f, r)| {
            f.vertices
                .iter()
                .zip(r)
                .map(|(&v, p)| (v, p.unwrap()))
                .collect()
        })
        .collect();
    operandi_uv::pack::uv_islands(&faces).1
}

#[test]
fn uvt16_17_atlas_makes_many_charts_and_atlas_2_fewer_all_in_the_tile() {
    let mut a = editor_with(sphere(), Mode::Polygon);
    run(&mut a, Tool::UvProject, |p| {
        p.uv_project.kind = ProjectionKind::Atlas
    });
    let mut b = editor_with(sphere(), Mode::Polygon);
    run(&mut b, Tool::UvProject, |p| {
        p.uv_project.kind = ProjectionKind::Atlas2;
        p.uv_project.atlas.segments = 4;
    });
    assert!(in_tile(mesh(&a)) && in_tile(mesh(&b)));
    let (many, few) = (islands(mesh(&a)), islands(mesh(&b)));
    assert!(many > few, "Atlas {many} charts, Atlas 2 {few}");
    assert!(few >= 2, "{few}");
    // Charts don't fold: every UV polygon is wound the same way.
    for (m, name) in [(mesh(&a), "Atlas"), (mesh(&b), "Atlas 2")] {
        let flipped = m.uv_maps["Texture"]
            .iter()
            .filter(|r| {
                let q: Vec<DVec2> = r.iter().flatten().copied().collect();
                (0..q.len())
                    .map(|i| q[i].perp_dot(q[(i + 1) % q.len()]))
                    .sum::<f64>()
                    < 0.
            })
            .count();
        assert_eq!(flipped, 0, "{name}");
    }
}

#[test]
fn atlas_segment_by_material_keeps_materials_apart() {
    let mut m = primitive::segmented_cube(DVec3::ZERO, DVec3::splat(2.), [2; 3]).unwrap();
    m.uv_maps.clear();
    m.faces[0].material = "Red".into();
    let mut e = editor_with(m, Mode::Polygon);
    run(&mut e, Tool::UvProject, |p| {
        p.uv_project.kind = ProjectionKind::Atlas2;
        p.uv_project.atlas.segment_by_material = true;
    });
    let m = mesh(&e);
    // Polygon 0's UV positions are shared with no other-material polygon.
    let rows = &m.uv_maps["Texture"];
    let key = |v: u32, p: DVec2| (v, (p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64);
    let red: std::collections::BTreeSet<_> = m.faces[0]
        .vertices
        .iter()
        .zip(&rows[0])
        .map(|(&v, p)| key(v, p.unwrap()))
        .collect();
    for (f, face) in m.faces.iter().enumerate().skip(1) {
        for (&v, p) in face.vertices.iter().zip(&rows[f]) {
            assert!(
                !red.contains(&key(v, p.unwrap())),
                "polygon {f} joins the red one: {:?} / {:?}",
                rows[0],
                rows[f]
            );
        }
    }
}

/// A strip of `n` quads round a tube (closed ring) of `rings` rows.
fn tube(n: u32, rings: u32) -> Mesh {
    let mut m = Mesh::default();
    for r in 0..=rings {
        for i in 0..n {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            m.vertices
                .push(DVec3::new(a.cos(), r as f64 * 0.5, a.sin()));
        }
    }
    for r in 0..rings {
        for i in 0..n {
            let j = (i + 1) % n;
            m.faces.push(face(&[
                r * n + i,
                r * n + j,
                (r + 1) * n + j,
                (r + 1) * n + i,
            ]));
        }
    }
    m
}

#[test]
fn uvt30_peeler_lays_a_cut_tube_out_as_an_even_grid() {
    let mut e = editor_with(tube(8, 2), Mode::Edge);
    // Cut along one vertical line: edges 0-8 and 8-16.
    e.scene.selection.edges = vec![(0, 8), (8, 16)];
    let points = mesh(&e).vertices.clone();
    run(&mut e, Tool::UvPeeler, |p| p.uv_peel.uniformity = 1.);
    let m = mesh(&e);
    assert_eq!(m.vertices, points);
    let rows = &m.uv_maps["Texture"];
    // Every quad is a 1/8 x 1/2 rectangle of the 0-1 box.
    for (f, r) in rows.iter().enumerate() {
        let q: Vec<DVec2> = r.iter().flatten().copied().collect();
        let lo = q.iter().fold(DVec2::MAX, |a, &b| a.min(b));
        let hi = q.iter().fold(DVec2::MIN, |a, &b| a.max(b));
        let size = hi - lo;
        assert!(
            (size.x - 0.125).abs() < 1e-9 && (size.y - 0.5).abs() < 1e-9,
            "polygon {f}: {size}"
        );
    }
    assert!(in_tile(m));
    assert_eq!(islands(m), 1, "one strip");
}

#[test]
fn uvt120_layout_drawings_and_convert_uvs_to_mesh() {
    let mut m = tube(4, 1);
    let rows = m
        .faces
        .iter()
        .enumerate()
        .map(|(f, _)| {
            let u0 = f as f64 / 4.;
            vec![
                Some(DVec2::new(u0, 0.)),
                Some(DVec2::new(u0 + 0.25, 0.)),
                Some(DVec2::new(u0 + 0.25, 1.)),
                Some(DVec2::new(u0, 1.)),
            ]
        })
        .collect();
    m.uv_maps.insert("Texture".into(), rows);
    let svg = uv_layout(&m, "Texture", LayoutFormat::Svg, 100.).unwrap();
    // 4 quads in a row: 4 tops, 4 bottoms, 5 verticals.
    assert_eq!(svg.matches("<line").count(), 13);
    assert!(
        svg.contains(r#"y1="100.000""#),
        "V is up: v 0 is the bottom"
    );
    let eps = uv_layout(&m, "Texture", LayoutFormat::Eps, 100.).unwrap();
    assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0") && eps.matches("lineto").count() == 13);
    let flat = uvs_to_mesh(&m, "Texture").unwrap();
    assert_eq!(flat.faces.len(), 4);
    // The tube's seam (u 0 and u 1 at one 3D point) makes 10 points, not 8.
    assert_eq!(flat.vertices.len(), 10);
    assert!(flat.vertices.iter().all(|p| p.z == 0.));
    let mut e = editor_with(m, Mode::Polygon);
    e.dispatch(Command::UvToMesh).unwrap();
    assert_eq!(e.scene.items.len(), 2);
    assert!(e.scene.active().unwrap().name.ends_with(" UVs"));
}

#[test]
fn uvt92_transfer_copies_uvs_to_a_mesh_of_the_same_topology() {
    let mut src = tube(4, 1);
    src.uv_maps.insert(
        "Texture".into(),
        (0..4)
            .map(|f| vec![Some(DVec2::splat(f as f64 * 0.1)); 4])
            .collect(),
    );
    let mut e = editor_with(tube(4, 1), Mode::Polygon);
    let target = e.scene.active().unwrap().id;
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = src.clone().into();
    let source = e.scene.active().unwrap().id;
    // The source primary, the target also active.
    e.scene.select_item(Some(target));
    e.dispatch(Command::ToggleItemSelection(source)).unwrap();
    e.dispatch(Command::UvTransfer).unwrap();
    assert_eq!(
        e.scene.item(target).unwrap().mesh.uv_maps["Texture"],
        src.uv_maps["Texture"]
    );
}

#[test]
fn uvt43_box_transform_moves_scales_and_turns_the_selected_uvs() {
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 0., 1.),
            DVec3::new(0., 0., 1.),
        ],
        faces: vec![face(&[0, 1, 2, 3])].into(),
        ..Default::default()
    };
    m.uv_maps.insert(
        "Texture".into(),
        vec![vec![
            Some(DVec2::new(0., 0.)),
            Some(DVec2::new(0.5, 0.)),
            Some(DVec2::new(0.5, 0.5)),
            Some(DVec2::new(0., 0.5)),
        ]],
    );
    let mut e = editor_with(m, Mode::Polygon);
    e.dispatch(Command::Activate(Tool::UvBoxTransform)).unwrap();
    e.dispatch(Command::UvToolSpace(Some(None))).unwrap();
    assert_eq!(e.uv_box_base(), Some((DVec2::ZERO, DVec2::splat(0.5))));
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.uv_box.offset = DVec2::new(0.1, 0.);
    p.uv_box.scale = DVec2::new(2., 1.);
    p.uv_box.anchor = Some(DVec2::ZERO);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let r = &mesh(&e).uv_maps["Texture"][0];
    assert!(
        (r[1].unwrap() - DVec2::new(1.1, 0.)).length() < 1e-9,
        "{r:?}"
    );
    assert!((r[0].unwrap() - DVec2::new(0.1, 0.)).length() < 1e-9);
}
