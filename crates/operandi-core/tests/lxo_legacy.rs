//! Format 4.0/4.1 (Modo 8-10) import: oracles LEG-O-01 to 04 from
//! specifications/lxo-legacy-import-2026-11-03, on its Operandi-made fixtures.
use glam::{DVec2, DVec3};
use operandi_core::{Scene, lxo};
use std::f64::consts::{FRAC_PI_3, FRAC_PI_4, FRAC_PI_6};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../specifications/lxo-legacy-import-2026-11-03/fixtures")
        .join(name)
}

fn open(name: &str) -> (Scene, lxo::Report) {
    let mut go = |_: &str| true;
    let mut control = lxo::Control::new(&mut go);
    let c = lxo::load(&fixture(name), &mut control)
        .and_then(|c| c.strict())
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    (c.scene, c.report)
}

fn near(a: DVec3, b: [f64; 3]) -> bool {
    (a - DVec3::from_array(b)).abs().max_element() < 1e-5
}

#[test]
fn leg_o_01_actions_win_over_item_values_and_maps_tags_survive() {
    let (scene, report) = open("LG-01-v40-appv9-tail70.lxo");
    assert!(report.producer.contains("4.0"), "{}", report.producer);
    let parent = scene
        .items
        .iter()
        .find(|i| i.name == "LegacyParent")
        .unwrap();
    // The ITEM holds (9,9,9); the setup action's (3,-2,5) wins.
    assert!(near(parent.transform.translation, [3., -2., 5.]));
    assert!(near(
        parent.transform.rotation,
        [FRAC_PI_6, FRAC_PI_4, FRAC_PI_3]
    ));
    assert!(near(parent.transform.scale, [1., 2., 0.5]));
    let world = [
        [3.0, -2.0, 5.0],
        [3.65974, -1.25, 4.952633],
        [2.788548, -0.383975, 6.530931],
        [2.128809, -1.133975, 6.578298],
        [4.319479, -0.5, 4.905266],
        [3.448288, 0.366025, 6.483564],
    ];
    for (p, w) in parent.mesh.vertices.iter().zip(world) {
        assert!(near(scene.world_point(parent.id, *p), w));
    }
    assert!(
        parent
            .mesh
            .faces
            .iter()
            .all(|f| f.material_label.as_deref() == Some("Steel"))
    );
    assert!(
        parent
            .mesh
            .faces
            .iter()
            .all(|f| f.part.as_deref() == Some("Body"))
    );
    let uv = &parent.mesh.uv_maps["Texture"];
    let want = [
        [[0.0, 0.0], [0.5, 0.0], [0.5, 1.0], [0.0, 1.0]],
        [[0.9, 0.1], [1.0, 0.0], [1.0, 1.0], [0.5, 1.0]],
    ];
    for (got, want) in uv.iter().zip(want) {
        for (g, w) in got.iter().zip(want) {
            assert!((g.unwrap() - DVec2::from_array(w)).abs().max_element() < 1e-5);
        }
    }
    // The child's edit position (1.5, 0.25, 0) overrides its setup (1, 0, 0),
    // and the file has no UV map on it (LEG-07).
    let child = scene
        .items
        .iter()
        .find(|i| i.name == "LegacyChild")
        .unwrap();
    assert_eq!(child.parent, Some(parent.id));
    assert!(near(child.transform.translation, [1.5, 0.25, 0.]));
    assert!(child.mesh.uv_maps.is_empty());
}

#[test]
fn leg_o_02_no_appv_and_an_1100_corner_polygon() {
    let (scene, _) = open("LG-02-v40-noappv-tail68-ngon1100.lxo");
    let mesh = &scene.items[0].mesh;
    assert_eq!((mesh.vertices.len(), mesh.faces.len()), (1100, 1));
    assert_eq!(mesh.faces[0].vertices, (0..1100).collect::<Vec<u32>>());
    assert!(near(mesh.vertices[1], [0.999984, 0.005712, 0.]));
}

#[test]
fn leg_o_03_and_04_other_layer_record_forms() {
    for (file, name) in [
        ("LG-03-v41-appv10-tail74.lxo", "Tail74"),
        ("LG-04-v40-tail120.lxo", "Tail120"),
    ] {
        let (scene, _) = open(file);
        let item = &scene.items[0];
        assert_eq!(item.name, name);
        let world = [[0.25, 0.5, 0.75], [2.25, 0.5, 0.75], [0.25, 3.5, 1.75]];
        for (p, w) in item.mesh.vertices.iter().zip(world) {
            assert!(near(scene.world_point(item.id, *p), w), "{file}");
        }
    }
}

/// Rewrite one fixture's bytes for refusal and SUBD cases.
fn patched(name: &str, edit: impl Fn(&mut Vec<u8>)) -> lxo::Result<lxo::Candidate> {
    let mut bytes = std::fs::read(fixture(name)).unwrap();
    edit(&mut bytes);
    let mut go = |_: &str| true;
    lxo::parse(&bytes, &mut lxo::Control::new(&mut go))
}

#[test]
fn leg_01_formats_2_and_3_read_like_format_4() {
    // VRSN major is the first payload word after the 20-byte FORM/VRSN header.
    let c = patched("LG-03-v41-appv10-tail74.lxo", |b| b[23] = 3).unwrap();
    assert!(
        c.report.producer.contains("older than 8 (LXO format 3.1)"),
        "{}",
        c.report.producer
    );
    assert_eq!(c.scene.items[0].name, "Tail74");
}

#[test]
fn leg_06_subd_polygons_import_as_their_cage() {
    let c = patched("LG-03-v41-appv10-tail74.lxo", |b| {
        let at = b.windows(4).position(|w| w == b"FACE").unwrap();
        b[at..at + 4].copy_from_slice(b"SUBD");
    })
    .and_then(|c| c.strict())
    .unwrap();
    let mesh = &c.scene.items[0].mesh;
    assert_eq!(mesh.faces.len(), 1);
    assert_eq!(
        mesh.attributes.representations,
        [operandi_core::PolygonRepresentation::Subdivs]
    );
    assert!(
        c.report.summary().contains("control cage"),
        "{}",
        c.report.summary()
    );
}

#[test]
fn leg_14_bad_geometry_refuses_the_mesh_not_the_file() {
    // Move the triangle's third point onto the line through the first two.
    let c = patched("LG-03-v41-appv10-tail74.lxo", |b| {
        let at = b.windows(4).position(|w| w == b"PNTS").unwrap() + 8 + 24;
        let point = [1.25f32, 0.5, -0.75];
        for (i, v) in point.iter().enumerate() {
            b[at + i * 4..at + i * 4 + 4].copy_from_slice(&v.to_be_bytes());
        }
    })
    .unwrap_err();
    // With no other mesh left, the refusal names the mesh and its counts
    // rather than failing on the file's structure.
    assert_eq!(c.category, "unsupported_modeling_group");
    assert!(
        c.detail
            .contains("Mesh 'Tail74' skipped: 1 zero-area polygons"),
        "{}",
        c.detail
    );
}

#[test]
fn nonplanar_polygons_import() {
    // Lift one corner of LegacyParent's first quad off its plane.
    let c = patched("LG-01-v40-appv9-tail70.lxo", |b| {
        let at = b.windows(4).position(|w| w == b"PNTS").unwrap() + 8 + 2 * 12 + 8;
        b[at..at + 4].copy_from_slice(&(-0.4f32).to_be_bytes());
    })
    .and_then(|c| c.strict())
    .unwrap();
    let parent = c
        .scene
        .items
        .iter()
        .find(|i| i.name == "LegacyParent")
        .unwrap();
    assert!((parent.mesh.vertices[2].z - 0.4).abs() < 1e-6);
    assert_eq!(parent.mesh.faces.len(), 2);
}

#[test]
fn self_crossing_polygons_are_kept_and_counted() {
    // Make LegacyParent's first quad a lopsided bow tie (its outline
    // crosses itself but still has area).
    let c = patched("LG-01-v40-appv9-tail70.lxo", |b| {
        let pnts = b.windows(4).position(|w| w == b"PNTS").unwrap() + 8;
        b[pnts + 24..pnts + 28].copy_from_slice(&0f32.to_be_bytes());
        b[pnts + 36..pnts + 40].copy_from_slice(&0.6f32.to_be_bytes());
    })
    .and_then(|c| c.strict())
    .unwrap();
    let parent = c
        .scene
        .items
        .iter()
        .find(|i| i.name == "LegacyParent")
        .unwrap();
    assert_eq!(parent.mesh.faces.len(), 2);
    assert!(parent.mesh.surface_triangles(0).is_err());
    let summary = c.report.summary();
    assert!(
        summary.contains("1 polygon(s) with a self-crossing"),
        "{summary}"
    );
}

/// Insert a top-level chunk right after the first `after` chunk.
fn insert_chunk(b: &mut Vec<u8>, after: &[u8; 4], id: &[u8; 4], payload: &[u8]) {
    let mut at = 12;
    loop {
        let len = u32::from_be_bytes(b[at + 4..at + 8].try_into().unwrap()) as usize;
        let end = at + 8 + len + len % 2;
        if &b[at..at + 4] == after {
            let mut chunk = id.to_vec();
            chunk.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            chunk.extend_from_slice(payload);
            if payload.len() % 2 == 1 {
                chunk.push(0);
            }
            b.splice(end..end, chunk);
            break;
        }
        at = end;
    }
    let form = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&form.to_be_bytes());
}

#[test]
fn unknown_maps_tags_and_chunks_are_left_out_not_the_mesh() {
    let c = patched("LG-01-v40-appv9-tail70.lxo", |b| {
        // An empty weight map, an unknown polygon tag type and an unknown
        // layer chunk on the first mesh, plus an unknown top-level chunk.
        let mut wght = b"WGHT".to_vec();
        wght.extend_from_slice(&1u16.to_be_bytes());
        wght.extend_from_slice(b"Falloff\0");
        insert_chunk(b, b"POLS", b"VMAP", &wght);
        insert_chunk(b, b"POLS", b"PTAG", b"SMGP\0\0\0\0");
        insert_chunk(b, b"POLS", b"XYZW", &[1, 2, 3]);
        insert_chunk(b, b"ENCO", b"QQQQ", &[0; 4]);
    })
    .and_then(|c| c.strict())
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c.scene.items.len(), 2);
    let summary = c.report.summary();
    for note in [
        "WGHT map not imported",
        "SMGP polygon tags not imported",
        "XYZW chunk",
        "QQQQ chunk",
    ] {
        assert!(summary.contains(note), "{note}: {summary}");
    }
}

/// Like OBJ/glTF import, the first visible mesh is the active item after an
/// LXO import, so it draws in the view's active display style.
#[test]
fn import_makes_first_visible_mesh_active() {
    for name in [
        "LG-01-v40-appv9-tail70.lxo",
        "LG-02-v40-noappv-tail68-ngon1100.lxo",
        "LG-03-v41-appv10-tail74.lxo",
        "LG-04-v40-tail120.lxo",
    ] {
        let (scene, _) = open(name);
        let expected = scene.items.iter().find(|i| i.visible).map(|i| i.id);
        assert!(expected.is_some(), "{name}: no visible mesh");
        assert_eq!(scene.selection.item, expected, "{name}");
        assert!(scene.active().is_some(), "{name}");
    }
}

#[test]
fn a_link_to_no_item_is_left_out_not_the_file() {
    // Point LegacyChild's parent LINK at 0xFFFFFFFF.
    let c = patched("LG-01-v40-appv9-tail70.lxo", |b| {
        let at = b
            .windows(11)
            .position(|w| &w[..4] == b"LINK" && &w[6..11] == b"paren")
            .unwrap();
        // LINK, u16 size, "parent\0" padded to 8, then the target.
        b[at + 14..at + 18].copy_from_slice(&u32::MAX.to_be_bytes());
    })
    .and_then(|c| c.strict())
    .unwrap();
    assert_eq!(
        c.report
            .omissions
            .get("links with no usable target, left out"),
        Some(&1)
    );
    assert!(c.scene.items.iter().all(|i| i.parent.is_none()));
}

/// Builders for synthetic records appended to an Operandi-made fixture.
fn s0(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
    out.push(0);
    if out.len() % 2 == 1 {
        out.push(0);
    }
}
fn sub(out: &mut Vec<u8>, id: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
}
fn append(b: &mut Vec<u8>, id: &[u8; 4], payload: &[u8]) {
    b.extend_from_slice(id);
    b.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    b.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        b.push(0);
    }
    let form = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&form.to_be_bytes());
}

#[test]
fn unknown_and_unreadable_records_are_skipped_with_notes() {
    let c = patched("LG-01-v40-appv9-tail70.lxo", |b| {
        // Sub-scene and reference records point at other files.
        for id in [b"IASS", b"SUBS", b"XREF"] {
            append(b, id, &[0, 0, 0, 1, 0, 0]);
        }
        // A non-modelling item with an unknown record, a channel vector of
        // a type Operandi can't read, and a string channel stored twice
        // with different values.
        let mut item = Vec::new();
        s0(&mut item, "videoStill");
        s0(&mut item, "Clip");
        item.extend_from_slice(&9999u32.to_be_bytes());
        sub(&mut item, b"ZZZZ", &[1, 2, 3, 4]);
        let mut chnv = Vec::new();
        s0(&mut chnv, "pos");
        chnv.extend_from_slice(&[0, 7, 0, 1]);
        s0(&mut chnv, "X");
        chnv.extend_from_slice(&[0; 4]);
        sub(&mut item, b"CHNV", &chnv);
        for value in ["a", "b"] {
            let mut chns = Vec::new();
            s0(&mut chns, "file");
            s0(&mut chns, value);
            sub(&mut item, b"CHNS", &chns);
        }
        append(b, b"ITEM", &item);
        // An action on that item with an unknown record, a repeated parent
        // and a value of an unreadable type.
        let mut actn = Vec::new();
        s0(&mut actn, "setup");
        s0(&mut actn, "setup");
        actn.extend_from_slice(&7777u32.to_be_bytes());
        for _ in 0..2 {
            sub(&mut actn, b"PRNT", &7777u32.to_be_bytes());
        }
        sub(&mut actn, b"ITEM", &9999u32.to_be_bytes());
        sub(&mut actn, b"CHAN", &[0, 0, 0, 7, 0, 0, 0, 0, 0, 0]);
        sub(&mut actn, b"ZZZZ", &[0, 0]);
        append(b, b"ACTN", &actn);
    })
    .and_then(|c| c.strict())
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c.scene.items.len(), 2);
    let o = &c.report.omissions;
    for key in [
        "IASS sub-scene/reference record(s) not imported (other files aren't opened)",
        "SUBS sub-scene/reference record(s) not imported (other files aren't opened)",
        "XREF sub-scene/reference record(s) not imported (other files aren't opened)",
        "ITEM.ZZZZ records not imported",
        "ACTN.ZZZZ records not imported",
        "unreadable channel vectors left out",
        "repeated action parents (first kept)",
        "unreadable action values on items not imported",
    ] {
        assert!(o.contains_key(key), "{key}: {o:?}");
    }
}

#[test]
fn an_unreadable_value_on_a_mesh_refuses_its_group_only() {
    let e = patched("LG-01-v40-appv9-tail70.lxo", |b| {
        // An action value of an unreadable type on the mesh itself.
        let mesh = b
            .windows(5)
            .position(|w| w == b"mesh\0")
            .map(|at| at + 6)
            .unwrap();
        let name_end = mesh + b[mesh..].iter().position(|&x| x == 0).unwrap() + 1;
        let id_at = name_end + name_end % 2;
        let id: [u8; 4] = b[id_at..id_at + 4].try_into().unwrap();
        let mut actn = Vec::new();
        s0(&mut actn, "setup");
        s0(&mut actn, "setup");
        actn.extend_from_slice(&7777u32.to_be_bytes());
        sub(&mut actn, b"ITEM", &id);
        sub(&mut actn, b"CHAN", &[0, 0, 0, 7, 0, 0, 0, 0, 0, 0]);
        append(b, b"ACTN", &actn);
    })
    .unwrap_err();
    // The only mesh is refused by its own reason, not the file's structure.
    assert_eq!(e.category, "unsupported_modeling_group", "{e}");
    assert!(e.detail.contains("unreadable action value"), "{e}");
}

#[test]
fn a_missing_or_other_encoding_record_reads_strings_as_utf8() {
    // Rename ENCO so the file has none, then give it encoding 0.
    let none = patched("LG-03-v41-appv10-tail74.lxo", |b| {
        let at = b.windows(4).position(|w| w == b"ENCO").unwrap();
        b[at..at + 4].copy_from_slice(b"ENCX");
    })
    .and_then(|c| c.strict())
    .unwrap();
    assert!(
        none.report
            .omissions
            .contains_key("no encoding record; read as UTF-8")
    );
    let other = patched("LG-03-v41-appv10-tail74.lxo", |b| {
        let at = b.windows(4).position(|w| w == b"ENCO").unwrap() + 8;
        b[at..at + 4].copy_from_slice(&0u32.to_be_bytes());
    })
    .and_then(|c| c.strict())
    .unwrap();
    assert!(
        other
            .report
            .omissions
            .contains_key("encoding 0 read as UTF-8")
    );
}
