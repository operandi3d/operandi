//! LXO import against `specifications/lxo-import-phase1-2026-11-05`.
//!
//! No Modo-saved files are used (Q-LX-12): each case's file is written here
//! from the spec's own description of the format (chunk order, LAYR fields,
//! mirrored Z, reversed corners, PTAG/VMAP/VMAD/VMED, ITEM channels and
//! links), filled with the case's expected meshes, plus the non-modelling
//! items, envelopes and actions a Modo file carries. The import must give
//! back the expected meshes.
use glam::DVec3;
use operandi_core::lxo;
use serde_json::Value;

const SPEC: &str = "lxo-import-phase1-2026-11-05";

fn cases() -> Vec<Value> {
    let path = format!(
        "{}/../../specifications/{SPEC}/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let root: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    root["cases"].as_array().unwrap().clone()
}

// --- writing --------------------------------------------------------------

/// A string inside a payload: pad relative to the payload start.
fn text(out: &mut Vec<u8>, s: &str) {
    let start = out.len();
    out.extend_from_slice(s.as_bytes());
    out.push(0);
    if (out.len() - start) % 2 == 1 {
        out.push(0);
    }
}
fn u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_be_bytes());
}
fn u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}
fn f32(out: &mut Vec<u8>, v: f64) {
    out.extend_from_slice(&(v as f32).to_be_bytes());
}
fn vx(out: &mut Vec<u8>, v: u32) {
    if v < 0xff00 {
        u16(out, v as u16);
    } else {
        u32(out, v | 0xff00_0000);
    }
}
fn chunk(out: &mut Vec<u8>, id: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(id);
    u32(out, data.len() as u32);
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
}
fn sub(out: &mut Vec<u8>, id: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(id);
    u16(out, data.len() as u16);
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
}

fn point(v: &Value) -> DVec3 {
    DVec3::new(
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    )
}
fn list(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect()
}

const CHANNELS: [&str; 6] = ["visible", "pos", "rot", "scl", "order", "fov"];
fn channel(name: &str) -> u32 {
    CHANNELS.iter().position(|c| *c == name).unwrap() as u32
}

/// A Modo-like file holding `meshes` (the case's expected meshes).
fn write(meshes: &[Value], others: &[Value]) -> Vec<u8> {
    let mut body = Vec::new();
    let mut d = Vec::new();
    u32(&mut d, 16);
    u32(&mut d, 0);
    text(&mut d, "Modo by The Foundry");
    chunk(&mut body, b"VRSN", &d);
    let mut d = Vec::new();
    for v in [17, 4001, 1, 123456] {
        u32(&mut d, v);
    }
    text(&mut d, "Modo 17.1v1");
    chunk(&mut body, b"APPV", &d);
    let mut d = Vec::new();
    u32(&mut d, 2);
    chunk(&mut body, b"ENCO", &d);
    chunk(&mut body, b"IASS", &[]);
    // TAGS: "Default" twice (PART points at one, MATR at the other), then
    // the other material names, resolved by index.
    let mut tags = vec!["Default".to_string(), "Default".to_string()];
    for m in meshes {
        for name in m["polygon_materials"].as_array().unwrap() {
            let name = name.as_str().unwrap().to_string();
            if name != "Default" && !tags.contains(&name) {
                tags.push(name);
            }
        }
    }
    let any_polygons = meshes
        .iter()
        .any(|m| !m["polygons"].as_array().unwrap().is_empty());
    if any_polygons {
        let mut d = Vec::new();
        for t in &tags {
            text(&mut d, t);
        }
        chunk(&mut body, b"TAGS", &d);
    }
    let mut d = Vec::new();
    u32(&mut d, CHANNELS.len() as u32);
    for c in CHANNELS {
        text(&mut d, c);
    }
    chunk(&mut body, b"CHNM", &d);
    let mesh_id = |i: usize| 100 + i as u32 * 10;
    for (i, m) in meshes.iter().enumerate() {
        let flags = m["layer_visibility_flags"].as_u64().unwrap() as u16;
        let mut d = Vec::new();
        u16(&mut d, i as u16);
        u16(&mut d, flags);
        for _ in 0..3 {
            f32(&mut d, 0.);
        }
        text(&mut d, m["name"].as_str().unwrap());
        u16(&mut d, 0xffff);
        f32(&mut d, 2.);
        f32(&mut d, 0.087266);
        for v in [1., 0., 0., 0., 0., 0., 0., 0., 0.] {
            f32(&mut d, v);
        }
        u32(&mut d, mesh_id(i));
        for _ in 0..4 {
            u16(&mut d, 0);
        }
        chunk(&mut body, b"LAYR", &d);
        let points: Vec<DVec3> = m["points"].as_array().unwrap().iter().map(point).collect();
        if points.is_empty() {
            continue;
        }
        let mut d = Vec::new();
        for p in &points {
            // File coordinates: Z mirrored.
            f32(&mut d, p.x);
            f32(&mut d, p.y);
            f32(&mut d, -p.z);
        }
        chunk(&mut body, b"PNTS", &d);
        chunk(&mut body, b"BBOX", &[0; 24]);
        let polygons: Vec<Vec<u32>> = m["polygons"].as_array().unwrap().iter().map(list).collect();
        // UVs: a point value from each point's first corner (VMAP), and a
        // VMAD entry for every corner that differs.
        let uvs = m["uv_per_corner"].as_array();
        let mut point_uv = std::collections::BTreeMap::new();
        if let Some(rows) = uvs {
            for (f, row) in rows.iter().enumerate() {
                for (k, uv) in row.as_array().unwrap().iter().enumerate() {
                    point_uv.entry(polygons[f][k]).or_insert(uv.clone());
                }
            }
        }
        let mut vmpa = Vec::new();
        u32(&mut vmpa, 1);
        u32(&mut vmpa, 6);
        chunk(&mut body, b"VMPA", &vmpa);
        let mut d = Vec::new();
        d.extend_from_slice(b"TXUV");
        u16(&mut d, 2);
        text(&mut d, "Texture");
        for (v, uv) in &point_uv {
            vx(&mut d, *v);
            f32(&mut d, uv[0].as_f64().unwrap());
            f32(&mut d, uv[1].as_f64().unwrap());
        }
        chunk(&mut body, b"VMAP", &d);
        chunk(&mut body, b"VMPA", &vmpa);
        let mut d = Vec::new();
        d.extend_from_slice(b"BSPL");
        u16(&mut d, 1);
        text(&mut d, "B-Spline");
        chunk(&mut body, b"VMAP", &d);
        let kind = match m["polygon_type"].as_str() {
            Some("subdiv") => b"SUBD",
            _ => b"FACE",
        };
        let mut d = Vec::new();
        d.extend_from_slice(kind);
        for f in &polygons {
            u16(&mut d, f.len() as u16);
            // File order: the first corner, then the rest reversed.
            let mut file = f.clone();
            file[1..].reverse();
            for v in file {
                vx(&mut d, v);
            }
        }
        chunk(&mut body, b"POLS", &d);
        if let Some(rows) = uvs {
            let mut d = Vec::new();
            d.extend_from_slice(b"TXUV");
            u16(&mut d, 2);
            text(&mut d, "Texture");
            for (f, row) in rows.iter().enumerate() {
                for (k, uv) in row.as_array().unwrap().iter().enumerate() {
                    let v = polygons[f][k];
                    if point_uv[&v] != *uv {
                        vx(&mut d, v);
                        vx(&mut d, f as u32);
                        f32(&mut d, uv[0].as_f64().unwrap());
                        f32(&mut d, uv[1].as_f64().unwrap());
                    }
                }
            }
            chunk(&mut body, b"VMAD", &d);
        }
        let materials = m["polygon_materials"].as_array().unwrap();
        for (kind, pick) in [(b"MATR", true), (b"PART", false)] {
            let mut d = Vec::new();
            d.extend_from_slice(kind);
            for (f, name) in materials.iter().enumerate() {
                vx(&mut d, f as u32);
                let name = name.as_str().unwrap();
                let tag = if name == "Default" {
                    if pick { 1 } else { 0 }
                } else {
                    tags.iter().position(|t| t == name).unwrap()
                };
                u16(&mut d, if pick { tag as u16 } else { 0 });
            }
            chunk(&mut body, b"PTAG", &d);
        }
        if let Some(weights) = m["edge_weights"].as_object() {
            let mut d = Vec::new();
            d.extend_from_slice(b"SUBV");
            u16(&mut d, 1);
            text(&mut d, "Subdivision");
            for (edge, w) in weights {
                let ends: Vec<u32> = edge.split('-').map(|x| x.parse().unwrap()).collect();
                vx(&mut d, ends[0]);
                vx(&mut d, ends[1]);
                f32(&mut d, w.as_f64().unwrap());
            }
            chunk(&mut body, b"VMED", &d);
        }
    }
    let name_id = |name: &str| {
        meshes
            .iter()
            .position(|m| m["name"] == name)
            .map(mesh_id)
            .unwrap()
    };
    for (i, m) in meshes.iter().enumerate() {
        let id = mesh_id(i);
        let mut d = Vec::new();
        text(&mut d, "mesh");
        // LF01's default mesh has an empty item name and VNAM "Mesh".
        let name = m["name"].as_str().unwrap();
        text(&mut d, if name == "Mesh" { "" } else { name });
        u32(&mut d, id);
        let mut s = Vec::new();
        text(&mut s, name);
        sub(&mut d, b"VNAM", &s);
        let mut s = Vec::new();
        u32(&mut s, 7); // not the layer index
        u32(&mut s, m["layer_visibility_flags"].as_u64().unwrap() as u32);
        s.extend_from_slice(&[200, 200, 200, 255]);
        sub(&mut d, b"LAYR", &s);
        let mut s = Vec::new();
        vx(&mut s, channel("visible"));
        u16(&mut s, 0x23);
        text(&mut s, m["visible_channel"].as_str().unwrap());
        sub(&mut d, b"CHAN", &s);
        if let Some(parent) = m["parent"].as_str() {
            let mut s = Vec::new();
            text(&mut s, "parent");
            u32(&mut s, name_id(parent));
            u32(&mut s, 0);
            sub(&mut d, b"LINK", &s);
        }
        sub(&mut d, b"XXXX", &[1, 2, 3]);
        chunk(&mut body, b"ITEM", &d);
        for (k, (kind, base, values, ordinal)) in [
            ("scale", "scl", [1., 1., 1.], 0),
            ("rotation", "rot", [0., 0., 0.], 1),
            ("translation", "pos", [0., 0., 0.], 2),
        ]
        .into_iter()
        .enumerate()
        {
            let values = if kind == "translation" {
                let p = point(&m["position"]);
                [p.x, p.y, p.z]
            } else {
                values
            };
            let mut d = Vec::new();
            text(&mut d, kind);
            text(&mut d, "");
            u32(&mut d, id + 1 + k as u32);
            let mut s = Vec::new();
            text(&mut s, base);
            u16(&mut s, 0x22);
            u16(&mut s, 3);
            for (c, v) in ["X", "Y", "Z"].iter().zip(values) {
                text(&mut s, c);
                f32(&mut s, v);
            }
            sub(&mut d, b"CHNV", &s);
            if kind == "rotation" {
                let mut s = Vec::new();
                vx(&mut s, channel("order"));
                u16(&mut s, 3);
                text(&mut s, "zxy");
                sub(&mut d, b"CHAN", &s);
            }
            let mut s = Vec::new();
            text(&mut s, "xfrmCore");
            u32(&mut s, id);
            u32(&mut s, ordinal);
            sub(&mut d, b"LINK", &s);
            chunk(&mut body, b"ITEM", &d);
        }
    }
    for (k, kind) in others.iter().enumerate() {
        let mut d = Vec::new();
        text(&mut d, kind.as_str().unwrap());
        text(&mut d, "");
        u32(&mut d, 9000 + k as u32);
        let mut s = Vec::new();
        vx(&mut s, channel("fov"));
        u16(&mut s, 0x22);
        f32(&mut s, 0.5);
        sub(&mut d, b"CHAN", &s);
        sub(&mut d, b"UNIQ", &{
            let mut s = Vec::new();
            text(&mut s, "unique");
            s
        });
        chunk(&mut body, b"ITEM", &d);
    }
    chunk(&mut body, b"ENVL", &[0, 1, 2, 3, 4, 5]);
    for action in ["edit", "scene", "setup"] {
        let mut d = Vec::new();
        text(&mut d, action);
        text(&mut d, action);
        u32(&mut d, 0);
        chunk(&mut body, b"ACTN", &d);
    }
    chunk(&mut body, b"AANI", &[0x41, 0x41, 0x53, 0x45, 0, 10, 0, 0]);
    let mut out = Vec::new();
    out.extend_from_slice(b"FORM");
    u32(&mut out, body.len() as u32 + 4);
    out.extend_from_slice(b"LXOB");
    out.extend_from_slice(&body);
    out
}

fn import(bytes: &[u8]) -> lxo::Result<lxo::Candidate> {
    let mut progress = |_: &str| true;
    let mut control = lxo::Control::new(&mut progress);
    lxo::parse(bytes, &mut control)
}

fn check(c: &Value) -> Result<(), String> {
    let meshes = c["expected_meshes"].as_array().unwrap();
    let bytes = write(meshes, c["other_items_present"].as_array().unwrap());
    check_bytes(c, &bytes)
}

fn check_bytes(c: &Value, bytes: &[u8]) -> Result<(), String> {
    let meshes = c["expected_meshes"].as_array().unwrap();
    let got = import(bytes).map_err(|e| format!("refused: {e}"))?.scene;
    if got.items.len() != meshes.len() {
        return Err(format!(
            "{} meshes, expected {}",
            got.items.len(),
            meshes.len()
        ));
    }
    for m in meshes {
        let name = m["name"].as_str().unwrap();
        let item = got
            .items
            .iter()
            .find(|i| i.name == name)
            .ok_or_else(|| format!("no mesh {name}"))?;
        let parent = item
            .parent
            .and_then(|p| got.items.iter().find(|i| i.id == p))
            .map(|i| i.name.as_str());
        if parent != m["parent"].as_str() {
            return Err(format!("{name}: parent {parent:?}"));
        }
        if (item.transform.translation - point(&m["position"])).length() > 1e-6 {
            return Err(format!("{name}: position {}", item.transform.translation));
        }
        if item.visible != (m["visible_channel"] != "off") {
            return Err(format!("{name}: visible {}", item.visible));
        }
        let mesh = &item.mesh;
        let points: Vec<DVec3> = m["points"].as_array().unwrap().iter().map(point).collect();
        if mesh.vertices.len() != points.len()
            || mesh
                .vertices
                .iter()
                .zip(&points)
                .any(|(a, b)| (*a - *b).length() > 1e-6)
        {
            return Err(format!("{name}: points"));
        }
        let polygons: Vec<Vec<u32>> = m["polygons"].as_array().unwrap().iter().map(list).collect();
        let got_polygons: Vec<Vec<u32>> = mesh.faces.iter().map(|f| f.vertices.clone()).collect();
        if got_polygons != polygons {
            return Err(format!("{name}: polygons {got_polygons:?}"));
        }
        let subd = m["polygon_type"] == "subdiv";
        let is_subd = |f: usize| {
            mesh.attributes.representations.get(f)
                == Some(&operandi_core::PolygonRepresentation::Subdivs)
        };
        if (0..mesh.faces.len()).any(|f| is_subd(f) != subd) {
            return Err(format!("{name}: polygon types"));
        }
        for (f, want) in m["polygon_materials"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            if mesh.faces[f].material != want.as_str().unwrap() {
                return Err(format!(
                    "{name}: polygon {f} material {}",
                    mesh.faces[f].material
                ));
            }
        }
        if !mesh.uv_maps.contains_key("Texture") {
            return Err(format!("{name}: no Texture map"));
        }
        if let Some(rows) = m["uv_per_corner"].as_array() {
            for (f, row) in rows.iter().enumerate() {
                for (k, uv) in row.as_array().unwrap().iter().enumerate() {
                    let g = mesh.uv_maps["Texture"][f][k].ok_or("missing UV")?;
                    let w = glam::DVec2::new(uv[0].as_f64().unwrap(), uv[1].as_f64().unwrap());
                    if (g - w).length() > 1e-6 {
                        return Err(format!("{name}: polygon {f} corner {k} UV {g}"));
                    }
                }
            }
        }
        let weights = m["edge_weights"].as_object().cloned().unwrap_or_default();
        if mesh.attributes.creases.len() != weights.len() {
            return Err(format!(
                "{name}: {} edge weights",
                mesh.attributes.creases.len()
            ));
        }
        for (edge, w) in &weights {
            let ends: Vec<u32> = edge.split('-').map(|x| x.parse().unwrap()).collect();
            let (a, b) = (ends[0].min(ends[1]), ends[0].max(ends[1]));
            let got = mesh
                .attributes
                .creases
                .iter()
                .find(|k| (k.a, k.b) == (a, b))
                .map(|k| k.weight);
            if got.is_none_or(|g| (g - w.as_f64().unwrap()).abs() > 1e-6) {
                return Err(format!("{name}: edge {edge} weight {got:?}"));
            }
        }
    }
    Ok(())
}

#[test]
fn every_case_imports_as_expected() {
    let mut failures = Vec::new();
    for c in cases() {
        if let Err(e) = check(&c) {
            failures.push(format!("{}: {e}", c["id"].as_str().unwrap()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn not_an_lxo_and_truncated_files_are_refused() {
    let all = cases();
    let good = write(
        all[1]["expected_meshes"].as_array().unwrap(),
        all[1]["other_items_present"].as_array().unwrap(),
    );
    assert!(import(&good).is_ok());
    let mut lwo = good.clone();
    lwo[8..12].copy_from_slice(b"LWO2");
    assert!(import(&lwo).is_err());
    assert!(import(&good[..good.len() - 10]).is_err());
    assert!(import(b"not a file at all").is_err());
}

/// The phase-1 reader keeps two import fixes from other branches: the
/// first visible mesh is the active item (e691870), and the app's
/// `ensure_camera_and_light` gives the scene a camera and a light.
#[test]
fn phase1_import_selects_the_first_visible_mesh_and_takes_camera_and_light() {
    for c in cases() {
        let id = c["id"].as_str().unwrap();
        let bytes = write(
            c["expected_meshes"].as_array().unwrap(),
            c["other_items_present"].as_array().unwrap(),
        );
        let mut scene = import(&bytes).unwrap().scene;
        let first = scene.items.iter().find(|i| i.visible).map(|i| i.id);
        assert_eq!(scene.selection.item, first, "{id}");
        scene.ensure_camera_and_light().unwrap();
        assert!(scene.items.iter().any(|i| i.camera.is_some()), "{id}");
        assert!(scene.items.iter().any(|i| i.light.is_some()), "{id}");
        assert_eq!(scene.selection.item, first, "{id}: selection kept");
    }
}

/// Records the reader meets but doesn't use are left out with a named
/// note, never a refusal: a font polygon tag, a non-UV map and an unknown
/// item record, plus the B-Spline map every written file carries.
#[test]
fn unused_records_are_left_out_with_named_notes() {
    let c = cases()
        .into_iter()
        .find(|c| {
            c["expected_meshes"].as_array().is_some_and(|m| {
                m.iter()
                    .any(|m| !m["polygons"].as_array().unwrap().is_empty())
            })
        })
        .unwrap();
    let mut bytes = write(
        c["expected_meshes"].as_array().unwrap(),
        c["other_items_present"].as_array().unwrap(),
    );
    let mut add = |id: &[u8; 4], data: &[u8]| {
        chunk(&mut bytes, id, data);
        let form = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&form.to_be_bytes());
    };
    // Geometry records attach to the last layer.
    let mut ptag = b"FONT".to_vec();
    vx(&mut ptag, 0);
    u16(&mut ptag, 0);
    add(b"PTAG", &ptag);
    let mut wght = b"WGHT".to_vec();
    u16(&mut wght, 1);
    text(&mut wght, "Weight");
    add(b"VMAP", &wght);
    let mut item = Vec::new();
    text(&mut item, "locator");
    text(&mut item, "Extra");
    u32(&mut item, 9999);
    sub(&mut item, b"ZZZZ", &[1, 2]);
    add(b"ITEM", &item);
    let report = import(&bytes).unwrap_or_else(|e| panic!("{e}")).report;
    for key in [
        "PTAG.FONT records not imported",
        "VMAP.WGHT records not imported",
        "VMAP.BSPL records not imported",
        "ITEM.ZZZZ records not imported",
    ] {
        assert!(
            report.omissions.contains_key(key),
            "{key}: {:?}",
            report.omissions
        );
    }
}
