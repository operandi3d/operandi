//! Oracles from specifications/lxo-export-2026-11-03/ORACLES.json, checked by
//! reading the written bytes and by reopening them with Operandi's own LXO
//! importer (format/01-03). Modo readback itself can't run here.
use super::*;
use crate::{Face, Item, Transform};
use glam::DVec3;

fn face(vertices: &[u32]) -> Face {
    Face {
        vertices: vertices.to_vec(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn triangle() -> Mesh {
    Mesh {
        vertices: vec![
            DVec3::new(0.25, 0.5, 0.75),
            DVec3::new(2.25, 0.5, 0.75),
            DVec3::new(0.25, 3.5, 1.75),
        ],
        faces: vec![face(&[0, 1, 2])].into(),
        ..Default::default()
    }
}

fn scene_of(meshes: Vec<(&str, Mesh)>) -> Scene {
    let mut scene = Scene::default();
    for (name, mesh) in meshes {
        scene.new_mesh().unwrap();
        let item: &mut Item = scene.items.last_mut().unwrap();
        item.name = name.into();
        item.mesh = mesh.into();
    }
    scene
}

fn write_ok(scene: &Scene) -> Vec<u8> {
    write(scene, &mut Vec::new()).unwrap()
}

/// Reopen with Operandi's LXO importer, which admits this export's layout
/// as its own producer profile (BEHAVIOR-IMPORT-PROFILE.md). Refuses any
/// partial result.
fn reopen(bytes: &[u8]) -> Scene {
    try_reopen(bytes).unwrap_or_else(|e| panic!("{e}"))
}

fn try_reopen(bytes: &[u8]) -> std::result::Result<Scene, crate::lxo::Error> {
    let mut keep_going = |_: &str| true;
    let mut control = crate::lxo::Control::new(&mut keep_going);
    crate::lxo::parse(bytes, &mut control)
        .and_then(|c| c.strict())
        .map(|c| c.scene)
}

/// Rebuild a FORM from (id, payload) chunks, for tampering tests.
fn form(chunks: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut body = Bytes::default();
    for (id, payload) in chunks {
        body.chunk(id, &Bytes(payload.clone())).unwrap();
    }
    let mut out = b"FORM".to_vec();
    out.extend_from_slice(&(body.0.len() as u32 + 4).to_be_bytes());
    out.extend_from_slice(b"LXOB");
    out.extend_from_slice(&body.0);
    out
}

/// LXE-IMP-06: import gives back the same scene, and exporting that scene
/// again gives the same bytes.
fn assert_round_trip(scene: &Scene) {
    let bytes = write_ok(scene);
    let back = reopen(&bytes);
    assert_eq!(back.items.len(), scene.items.len());
    let close = |a: DVec3, b: DVec3| (a - b).abs().max_element() < 1e-6;
    for (a, b) in scene.items.iter().zip(&back.items) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.visible, b.visible);
        let parent = |s: &Scene, i: &Item| i.parent.map(|p| s.item_index(p).unwrap());
        assert_eq!(parent(scene, a), parent(&back, b));
        assert!(close(a.transform.translation, b.transform.translation));
        assert!(close(a.transform.rotation, b.transform.rotation));
        assert!(close(a.transform.scale, b.transform.scale));
        assert_eq!(a.mesh.vertices.len(), b.mesh.vertices.len());
        for (p, q) in a.mesh.vertices.iter().zip(&b.mesh.vertices) {
            assert!(close(*p, *q));
        }
        for (f, g) in a.mesh.faces.iter().zip(&b.mesh.faces) {
            assert_eq!(f.vertices, g.vertices);
            assert_eq!(f.material_label, g.material_label);
            assert_eq!(f.part, g.part);
        }
        assert_eq!(
            a.mesh.uv_maps.keys().collect::<Vec<_>>(),
            b.mesh.uv_maps.keys().collect::<Vec<_>>()
        );
        for (m, n) in a.mesh.uv_maps.values().zip(b.mesh.uv_maps.values()) {
            for (u, w) in m.iter().flatten().zip(n.iter().flatten()) {
                assert!((u.unwrap() - w.unwrap()).abs().max_element() < 1e-6);
            }
        }
    }
    assert_eq!(write_ok(&back), bytes, "second export differs");
}

/// Top-level chunk ids and payloads, in order.
fn chunks(bytes: &[u8]) -> Vec<([u8; 4], &[u8])> {
    assert_eq!(&bytes[..4], b"FORM");
    assert_eq!(
        u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize,
        bytes.len() - 8
    );
    assert_eq!(&bytes[8..12], b"LXOB");
    let mut out = Vec::new();
    let mut at = 12;
    while at < bytes.len() {
        let id: [u8; 4] = bytes[at..at + 4].try_into().unwrap();
        let len = u32::from_be_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        out.push((id, &bytes[at + 8..at + 8 + len]));
        at += 8 + len + len % 2;
    }
    assert_eq!(at, bytes.len());
    out
}

fn find<'a>(chunks: &[([u8; 4], &'a [u8])], id: &[u8; 4]) -> Vec<&'a [u8]> {
    chunks.iter().filter(|c| &c.0 == id).map(|c| c.1).collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn lxe_o_02_smallest_file_has_the_verified_header_and_geometry_bytes() {
    let bytes = write_ok(&scene_of(vec![("AsymTriangle", triangle())]));
    let c = chunks(&bytes);
    let ids: Vec<&[u8; 4]> = c.iter().map(|c| &c.0).collect();
    assert_eq!(ids[..3], [b"VRSN", b"APPV", b"ENCO"]);
    assert_eq!(
        hex(&find(&c, b"APPV")[0][..16]),
        "0000001100000fa10000000100000000"
    );
    assert_eq!(hex(find(&c, b"ENCO")[0]), "00000002");
    // Same bytes as the import spec's triangle fixture (format/01).
    assert_eq!(
        hex(find(&c, b"PNTS")[0]),
        "3e8000003f000000bf40000040100000\
         3f000000bf4000003e80000040600000bfe00000"
    );
    assert_eq!(
        hex(find(&c, b"POLS")[0]),
        "46414345000300000002 0001".replace(' ', "")
    );
    assert_round_trip(&scene_of(vec![("AsymTriangle", triangle())]));
    let scene = reopen(&bytes);
    assert_eq!(scene.items.len(), 1);
    assert_eq!(scene.items[0].name, "AsymTriangle");
    for (a, b) in triangle()
        .vertices
        .iter()
        .zip(&scene.items[0].mesh.vertices)
    {
        assert!((*a - *b).abs().max_element() < 1e-6);
    }
    assert_eq!(scene.items[0].mesh.faces[0].vertices, [0, 1, 2]);
}

#[test]
fn lxe_04_layr_payload_matches_the_verified_layout() {
    let bytes = write_ok(&scene_of(vec![("AB", triangle())]));
    let c = chunks(&bytes);
    let layr = find(&c, b"LAYR")[0];
    // ordinal 0, flags 5, zero pivot, "AB\0" + pad, -1, 2.0, 5 degrees.
    assert_eq!(
        hex(&layr[..30]),
        "00000005000000000000000000000000414200 00ffff400000003db2b8c2".replace(' ', "")
    );
    let k = 20;
    assert_eq!(hex(&layr[k + 46..k + 50]), "00000001");
    assert_eq!(&layr[k + 50..], &LAYR_TAIL);
    assert_eq!(layr.len(), k + 72);
}

#[test]
fn lxe_o_05_transforms_reopen_with_oracle_world_points() {
    let mut scene = scene_of(vec![("Xb", triangle())]);
    scene.items[0].transform = Transform {
        translation: DVec3::new(3., -2., 5.),
        rotation: DVec3::new(30f64, 45., 60.).map(f64::to_radians),
        scale: DVec3::new(1., 2., 0.5),
        zxy: true,
        order: None,
    };
    assert_round_trip(&scene);
    let back = reopen(&write_ok(&scene));
    let item = &back.items[0];
    assert!(item.transform.zxy);
    let expected = [
        [2.958979, -1.566987, 6.006947],
        [4.278458, -0.066987, 5.912213],
        [0.651591, 0.781089, 11.048028],
    ];
    for (p, e) in item.mesh.vertices.iter().zip(expected) {
        let w = back.world_point(item.id, *p);
        assert!((w - DVec3::from_array(e)).abs().max_element() < 1e-5, "{w}");
    }
    // Values live in the setup action, never an edit layer (LXE-O-05c).
    let bytes = write_ok(&scene);
    let c = chunks(&bytes);
    let actn = find(&c, b"ACTN");
    assert_eq!(actn.len(), 1);
    assert!(actn[0].starts_with(b"setup\0setup\0"));
}

#[test]
fn xyz_rotations_are_written_as_the_same_zxy_rotation() {
    let t = Transform {
        translation: DVec3::ZERO,
        rotation: DVec3::new(0.3, -1.1, 2.0),
        scale: DVec3::ONE,
        zxy: false,
        order: None,
    };
    let zxy = Transform {
        rotation: zxy_angles(&t),
        zxy: true,
        order: None,
        ..t.clone()
    };
    let (a, b) = (t.matrix(), zxy.matrix());
    assert!(a.abs_diff_eq(b, 1e-12), "{a} vs {b}");
}

#[test]
fn lxe_o_07_parenting_and_hidden_state_reopen() {
    let mut scene = scene_of(vec![
        ("Parent", triangle()),
        ("Child", triangle()),
        ("Hidden", triangle()),
    ]);
    let ids: Vec<u64> = scene.items.iter().map(|i| i.id).collect();
    scene.items[0].transform.translation = DVec3::new(3., -2., 5.);
    scene.items[0].transform.zxy = true;
    scene.items[1].parent = Some(ids[0]);
    scene.items[1].transform.translation = DVec3::X;
    scene.items[2].visible = false;
    assert_round_trip(&scene);
    let back = reopen(&write_ok(&scene));
    let names: Vec<_> = back.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["Parent", "Child", "Hidden"]);
    assert_eq!(back.items[1].parent, Some(back.items[0].id));
    assert!(back.items[0].visible && !back.items[2].visible);
    let w = back.world_point(back.items[1].id, DVec3::ZERO);
    assert!((w - DVec3::new(4., -2., 5.)).abs().max_element() < 1e-6);
}

#[test]
fn lxe_o_09_uv_maps_with_seams_and_labels_reopen_exactly() {
    let p = |x: f64, y: f64| DVec3::new(x, y, 0.);
    let mut mesh = Mesh {
        vertices: vec![
            p(0., 0.),
            p(1., 0.),
            p(1., 1.),
            p(0., 1.),
            p(2., 0.),
            p(2., 1.),
        ],
        faces: vec![face(&[0, 1, 2, 3]), face(&[1, 4, 5, 2])].into(),
        ..Default::default()
    };
    let uv = |f: &dyn Fn(DVec3) -> DVec2, mesh: &Mesh| -> Vec<Vec<Option<DVec2>>> {
        mesh.faces
            .iter()
            .map(|face| {
                face.vertices
                    .iter()
                    .map(|&v| Some(f(mesh.vertices[v as usize])))
                    .collect()
            })
            .collect()
    };
    let mut a = uv(&|q| DVec2::new(q.x / 2., q.y), &mesh);
    a[1][0] = Some(DVec2::new(0.9, 0.1));
    a[1][3] = Some(DVec2::new(0.9, 0.9));
    let b = uv(&|q| DVec2::new(3. * q.x - 1., 2. * q.y + 5.), &mesh);
    mesh.uv_maps.insert("UV_A".into(), a.clone());
    mesh.uv_maps.insert("UV_B".into(), b.clone());
    for (f, (m, part)) in [("Red", "Front"), ("Blue", "Back")].into_iter().enumerate() {
        mesh.faces[f].material = m.into();
        mesh.faces[f].material_label = Some(m.into());
        mesh.faces[f].part = Some(part.into());
    }
    assert_round_trip(&scene_of(vec![("Tagged", mesh.clone())]));
    let bytes = write_ok(&scene_of(vec![("Tagged", mesh.clone())]));
    // LXE-07: VMAD only after POLS.
    let c = chunks(&bytes);
    let at = |id: &[u8; 4]| c.iter().position(|c| &c.0 == id).unwrap();
    assert!(at(b"POLS") < at(b"VMAD"));
    let back = reopen(&bytes);
    let got = &back.items[0].mesh;
    assert_eq!(
        got.faces
            .iter()
            .map(|f| f.vertices.clone())
            .collect::<Vec<_>>(),
        [[0, 1, 2, 3], [1, 4, 5, 2]]
    );
    for (name, want) in [("UV_A", &a), ("UV_B", &b)] {
        for (gf, wf) in got.uv_maps[name].iter().zip(want) {
            for (g, w) in gf.iter().zip(wf) {
                assert!(
                    (g.unwrap() - w.unwrap()).abs().max_element() < 1e-6,
                    "{name}"
                );
            }
        }
    }
    assert_eq!(got.faces[0].material_label.as_deref(), Some("Red"));
    assert_eq!(got.faces[1].part.as_deref(), Some("Back"));
}

#[test]
fn lxe_o_12_names_survive_including_unicode_and_duplicates() {
    let names = ["", "Ünïcødé 名前", "Dup", "Dup"];
    let scene = scene_of(names.iter().map(|n| (*n, triangle())).collect());
    assert_round_trip(&scene);
    let back = reopen(&write_ok(&scene));
    let got: Vec<_> = back.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(got[1..], names[1..]);
}

#[test]
fn lxe_o_14_large_grid_uses_long_indices_and_reopens() {
    let n = 256u32;
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices
                .push(DVec3::new(i as f64 / n as f64, j as f64 / n as f64, 0.));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * 257 + i;
            mesh.faces.push(face(&[a, a + 1, a + 258, a + 257]));
        }
    }
    let scene = scene_of(vec![("Grid", mesh)]);
    assert_round_trip(&scene);
    let back = reopen(&write_ok(&scene));
    let m = &back.items[0].mesh;
    assert_eq!((m.vertices.len(), m.faces.len()), (66049, 65536));
    assert_eq!(m.faces[255].vertices, [255, 256, 513, 512]);
    assert_eq!(m.faces[65535].vertices, [65790, 65791, 66048, 66047]);
    assert!(
        (m.vertices[65280] - DVec3::new(0.007812, 0.992188, 0.))
            .abs()
            .max_element()
            < 1e-5
    );
}

#[test]
fn unverified_content_is_left_out_or_refused() {
    // An empty mesh with no children is skipped with a note.
    let mut scene = scene_of(vec![("Empty", Mesh::default()), ("Tri", triangle())]);
    let mut notes = Vec::new();
    let back = reopen(&write(&scene, &mut notes).unwrap());
    assert_eq!(back.items.len(), 1);
    assert!(notes.iter().any(|n| n.contains("Empty")));
    // An empty mesh that parents another item is refused.
    let parent = scene.items[0].id;
    scene.items[1].parent = Some(parent);
    assert!(write(&scene, &mut Vec::new()).is_err());
    // A scene with no polygons at all is refused.
    assert!(write(&scene_of(vec![("E", Mesh::default())]), &mut Vec::new()).is_err());
}

#[test]
fn lxe_imp_01_only_the_exact_profile_is_admitted() {
    let bytes = write_ok(&scene_of(vec![("T", triangle())]));
    let owned: Vec<([u8; 4], Vec<u8>)> = chunks(&bytes)
        .into_iter()
        .map(|(id, p)| (id, p.to_vec()))
        .collect();
    // A changed LAYR tail is not LXE; the Modo rules (lxo-import-phase1)
    // then read it as an ordinary Modo scene file.
    let mut tampered = owned.clone();
    let layr = tampered.iter_mut().find(|c| &c.0 == b"LAYR").unwrap();
    *layr.1.last_mut().unwrap() = 1;
    let read = try_reopen(&form(&tampered)).unwrap();
    assert_eq!(read.items.len(), 1);
    assert_eq!(read.items[0].mesh.faces.len(), 1);
    // An extra top-level chunk also leaves the profile; the Modo rules skip it.
    let mut extra = owned.clone();
    extra.push((*b"BBOX", vec![0; 24]));
    assert!(try_reopen(&form(&extra)).is_ok());
    // Inside the profile, a repeated setup channel refuses the file.
    let mut repeated = owned.clone();
    let actn = repeated.iter_mut().find(|c| &c.0 == b"ACTN").unwrap();
    // The last record is CHAN pos.Z: 6-byte header + VX, U16, VX, F32.
    let tail = actn.1[actn.1.len() - 16..].to_vec();
    actn.1.extend_from_slice(&tail);
    let err = try_reopen(&form(&repeated)).unwrap_err();
    assert_eq!(err.category, "duplicate_action_channel");
}

#[test]
fn lxe_imp_03_absent_transforms_are_identity() {
    let bytes = write_ok(&scene_of(vec![("T", triangle())]));
    // LXE-16: exactly one bare render item is written.
    let renders: Vec<_> = chunks(&bytes)
        .into_iter()
        .filter(|(id, p)| id == b"ITEM" && p.starts_with(b"polyRender\0"))
        .collect();
    assert_eq!(renders.len(), 1);
    assert_eq!(renders[0].1.len(), 12 + 2 + 4);
    // Drop the three transform ITEMs and the setup action (and CHNM with it).
    let kept: Vec<([u8; 4], Vec<u8>)> = chunks(&bytes)
        .into_iter()
        .filter(|(id, p)| {
            !matches!(id, b"ACTN" | b"CHNM")
                && !(id == b"ITEM" && p.starts_with(b"scale\0"))
                && !(id == b"ITEM" && p.starts_with(b"rotation\0"))
                && !(id == b"ITEM" && p.starts_with(b"translation\0"))
        })
        .map(|(id, p)| (id, p.to_vec()))
        .collect();
    let back = reopen(&form(&kept));
    assert_eq!(back.items[0].transform.translation, DVec3::ZERO);
    assert_eq!(back.items[0].transform.scale, DVec3::ONE);
    assert!(back.items[0].visible);
}

/// IC-Q1: Editor Color travels as the COLR item tag; names are read in any
/// case, and an unknown name (or "none") reads as none.
#[test]
fn editor_colors_round_trip_as_colr_item_tags() {
    let mut scene = scene_of(vec![("Red one", triangle()), ("Plain", triangle())]);
    scene.items[0].common.editor_color = Some(crate::EditorColor::Purple);
    let bytes = write_ok(&scene);
    let items = find(&chunks(&bytes), b"ITEM");
    let tagged = items
        .iter()
        .filter(|p| p.windows(4).any(|w| w == b"COLR"))
        .count();
    assert_eq!(tagged, 1, "only the coloured item carries the tag");
    let back = reopen(&bytes);
    assert_eq!(
        back.items[0].common.editor_color,
        Some(crate::EditorColor::Purple)
    );
    assert_eq!(back.items[1].common.editor_color, None);
    // Same-length edits of the stored name: case doesn't matter; an
    // unknown name is none.
    let swap = |from: &[u8], to: &[u8]| {
        let mut b = bytes.clone();
        let at = b.windows(from.len()).position(|w| w == from).unwrap();
        b[at..at + from.len()].copy_from_slice(to);
        reopen(&b).items[0].common.editor_color
    };
    assert_eq!(swap(b"purple", b"PURPLE"), Some(crate::EditorColor::Purple));
    assert_eq!(swap(b"purple", b"purplx"), None);
    assert_eq!(swap(b"purple\0\0", b"none\0\0\0\0"), None);
}

/// Everything the export writes, in one scene: a three-level hierarchy with
/// XYZ and ZXY rotations, a hidden branch, n-gons, two UV maps (one only
/// on some polygons), shared materials and parts across meshes, editor
/// colours and awkward names. Import gives the same world geometry, and
/// every later export is byte-identical to the first.
#[test]
fn combined_scene_round_trips_byte_identically() {
    let p = |x: f64, y: f64, z: f64| DVec3::new(x, y, z);
    let ngon = |n: u32, r: f64| Mesh {
        vertices: (0..n)
            .map(|i| {
                let a = i as f64 / n as f64 * std::f64::consts::TAU;
                p(r * a.cos(), r * a.sin(), 0.25 * i as f64)
            })
            .collect(),
        faces: vec![face(&(0..n).collect::<Vec<_>>())].into(),
        ..Default::default()
    };
    // A cube-ish strip of quads and a triangle, with labels and UVs.
    let mut strip = Mesh {
        vertices: vec![
            p(0., 0., 0.),
            p(1., 0., 0.),
            p(1., 1., 0.),
            p(0., 1., 0.),
            p(2., 0., 0.5),
            p(2., 1., 0.5),
            p(3., 0.5, 1.),
        ],
        faces: vec![face(&[0, 1, 2, 3]), face(&[1, 4, 5, 2]), face(&[4, 6, 5])].into(),
        ..Default::default()
    };
    let full: Vec<Vec<Option<DVec2>>> = strip
        .faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|&v| {
                    let q = strip.vertices[v as usize];
                    Some(DVec2::new(q.x / 3., q.y))
                })
                .collect()
        })
        .collect();
    let mut partial = full.clone();
    partial[2] = vec![None; 3];
    strip.uv_maps.insert("Texture".into(), full);
    strip.uv_maps.insert("Detail ✓".into(), partial);
    for (f, (m, part)) in [("Steel", "Body"), ("Steel", "Lid"), ("Glass", "Body")]
        .into_iter()
        .enumerate()
    {
        strip.faces[f].material = m.into();
        strip.faces[f].material_label = Some(m.into());
        strip.faces[f].part = Some(part.into());
    }
    let mut other = ngon(7, 1.5);
    other.faces[0].material = "Glass".into();
    other.faces[0].material_label = Some("Glass".into());
    let mut scene = scene_of(vec![
        ("Root", strip.clone()),
        ("Child ünïcode", ngon(12, 0.75)),
        ("Grandchild", triangle()),
        ("Root", other),
        ("Hidden", strip),
    ]);
    let ids: Vec<u64> = scene.items.iter().map(|i| i.id).collect();
    scene.items[1].parent = Some(ids[0]);
    scene.items[2].parent = Some(ids[1]);
    scene.items[4].parent = Some(ids[3]);
    scene.items[4].visible = false;
    let transforms = [
        (p(1., 2., 3.), p(0.1, 0.2, 0.3), p(1., 1., 1.), false),
        (p(-0.5, 0., 4.), p(-0.7, 0.4, 1.2), p(2., 0.5, 1.5), true),
        (p(0., 0., -1.), p(0.3, -1.1, 0.25), p(1., 3., 1.), false),
        (p(5., -2., 0.), p(0., 0., 0.), p(0.5, 0.5, 0.5), true),
        (p(0.25, 0.5, 0.75), p(1.0, 0.5, -0.25), p(1., 1., 2.), false),
    ];
    for (item, (t, r, s, zxy)) in scene.items.iter_mut().zip(transforms) {
        item.transform = Transform {
            translation: t,
            rotation: r,
            scale: s,
            zxy,
            order: None,
        };
    }
    scene.items[0].common.editor_color = Some(crate::EditorColor::Purple);
    scene.items[3].common.editor_color = Some(crate::EditorColor::Green);

    let mut notes = Vec::new();
    let first = write(&scene, &mut notes).unwrap();
    // LXO maps hold one UV per point: the UV-less corners on points that
    // another polygon maps take that point's UV, and the export says so.
    assert!(
        notes.iter().any(|n| n.contains("UV-less corner")),
        "{notes:?}"
    );
    let back = reopen(&first);
    assert_eq!(back.items.len(), scene.items.len());
    for (a, b) in scene.items.iter().zip(&back.items) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.common.editor_color, b.common.editor_color, "{}", a.name);
        assert_eq!(scene.shows(a), back.shows(b), "{}", a.name);
        let parent = |s: &Scene, i: &Item| i.parent.map(|p| s.item_index(p).unwrap());
        assert_eq!(parent(&scene, a), parent(&back, b), "{}", a.name);
        // XYZ rotations are written as ZXY, so compare world points.
        for (p, q) in a.mesh.vertices.iter().zip(&b.mesh.vertices) {
            let (wp, wq) = (scene.world_point(a.id, *p), back.world_point(b.id, *q));
            assert!((wp - wq).abs().max_element() < 1e-5, "{}", a.name);
        }
        for (f, g) in a.mesh.faces.iter().zip(&b.mesh.faces) {
            assert_eq!(f.vertices, g.vertices, "{}", a.name);
            assert_eq!(f.material_label, g.material_label, "{}", a.name);
            assert_eq!(f.part, g.part, "{}", a.name);
        }
        assert_eq!(
            a.mesh.uv_maps.keys().collect::<Vec<_>>(),
            b.mesh.uv_maps.keys().collect::<Vec<_>>(),
            "{}",
            a.name
        );
        for (m, n) in a.mesh.uv_maps.values().zip(b.mesh.uv_maps.values()) {
            for (u, w) in m.iter().flatten().zip(n.iter().flatten()) {
                match (u, w) {
                    (Some(u), Some(w)) => assert!((u - w).abs().max_element() < 1e-6),
                    (None, None) | (None, Some(_)) => {}
                    (Some(_), None) => panic!("{}: a UV was lost", a.name),
                }
            }
        }
    }
    // Point 6 is mapped by no polygon, so its corner stays UV-less.
    assert_eq!(back.items[0].mesh.uv_maps["Detail ✓"][2][1], None);
    let second = write_ok(&back);
    assert_eq!(second, first, "export of the reopened scene differs");
    assert_eq!(write_ok(&reopen(&second)), first, "third export differs");
}

/// Every rotation order exports as the same orientation (written as ZXY)
/// and reopens to the same world points.
#[test]
fn every_rotation_order_exports_as_the_same_orientation() {
    for order in crate::scene::RotationOrder::ALL {
        let mut scene = scene_of(vec![("Turned", triangle())]);
        scene.items[0].transform.rotation = DVec3::new(0.4, -0.9, 1.3);
        scene.items[0].transform.set_rotation_order(order);
        let bytes = write_ok(&scene);
        let back = reopen(&bytes);
        let (a, b) = (&scene.items[0], &back.items[0]);
        for (p, q) in a.mesh.vertices.iter().zip(&b.mesh.vertices) {
            let d = scene.world_point(a.id, *p) - back.world_point(b.id, *q);
            assert!(d.abs().max_element() < 1e-5, "{order:?}");
        }
        assert_eq!(write_ok(&back), bytes, "{order:?}");
    }
}

/// Hidden-limits A5: layer numbers are 16-bit, so 65,536 mesh layers fit
/// and one more is refused instead of wrapping around into a broken file.
#[test]
fn more_mesh_layers_than_the_format_holds_are_refused() {
    let mut scene = scene_of(vec![("Bolt", triangle())]);
    let base = scene.items[0].clone();
    let add = |scene: &mut Scene, count: usize| {
        for _ in 0..count {
            let mut item = base.clone();
            item.id = scene.next_id;
            scene.next_id += 1;
            scene.items.push(item);
        }
    };
    add(&mut scene, MAX_LAYERS - 1);
    assert!(write(&scene, &mut Vec::new()).is_ok());
    add(&mut scene, 1);
    let e = write(&scene, &mut Vec::new()).unwrap_err();
    assert!(e.contains("at most 65536 mesh layers"), "{e}");
}
