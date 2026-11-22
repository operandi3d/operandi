//! LXO formats 2.x and 3.x (Modo older than 8) and Modo 8–10 mesh presets:
//! synthetic files written here from the container layout (no Modo-saved
//! file). Mesh content imports; records Operandi can't use are skipped with
//! a named note; real corruption still refuses.
use operandi_core::lxo;

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
fn f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_be_bytes());
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

/// An ITEM: kind, name, id, then its sub-records.
fn item(body: &mut Vec<u8>, kind: &str, name: &str, id: u32, subs: &[([u8; 4], Vec<u8>)]) {
    let mut d = Vec::new();
    text(&mut d, kind);
    text(&mut d, name);
    u32(&mut d, id);
    for (sid, data) in subs {
        sub(&mut d, sid, data);
    }
    chunk(body, b"ITEM", &d);
}
fn link(graph: &str, target: u32, ordinal: u32) -> ([u8; 4], Vec<u8>) {
    let mut d = Vec::new();
    text(&mut d, graph);
    u32(&mut d, target);
    u32(&mut d, ordinal);
    (*b"LINK", d)
}
/// ITEM.LAYR: layer index, flags, colour (12 bytes).
fn item_layer() -> ([u8; 4], Vec<u8>) {
    (*b"LAYR", vec![0; 12])
}

/// LAYR. For formats 2.x/3.x, settings after the item id that the format
/// 4.x profile doesn't allow (subdivision level, curve angle, tail length);
/// for 4.x, the observed profile.
fn layer(body: &mut Vec<u8>, name: &str, owner: u32, major: u32) {
    let v4 = major == 4;
    let mut d = Vec::new();
    u16(&mut d, 0);
    u16(&mut d, 0);
    for _ in 0..3 {
        f32(&mut d, 0.);
    }
    text(&mut d, name);
    u16(&mut d, 0xffff);
    if v4 {
        d.extend_from_slice(&[0x40, 0, 0, 0, 0x3d, 0xb2, 0xb8, 0xc2]);
    } else {
        f32(&mut d, 3.);
        f32(&mut d, 0.1);
    }
    for v in [1., 0., 0., 0., 1., 0., 0., 0., 1.] {
        f32(&mut d, v);
    }
    u32(&mut d, owner);
    if v4 {
        d.extend_from_slice(&[0, 0x10, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 0, 2, 0, 2, 0, 1]);
    } else {
        d.extend_from_slice(&[0, 7, 0, 0, 0, 1]);
    }
    chunk(body, b"LAYR", &d);
}
fn points(body: &mut Vec<u8>, p: &[[f32; 3]]) {
    let mut d = Vec::new();
    for [x, y, z] in p {
        f32(&mut d, *x);
        f32(&mut d, *y);
        f32(&mut d, *z);
    }
    chunk(body, b"PNTS", &d);
}
fn polygons(body: &mut Vec<u8>, polys: &[&[u16]]) {
    let mut d = b"FACE".to_vec();
    for p in polys {
        u16(&mut d, p.len() as u16);
        for v in *p {
            u16(&mut d, *v);
        }
    }
    chunk(body, b"POLS", &d);
}

struct Options {
    major: u32,
    minor: u32,
    form: &'static [u8; 4],
    /// The repeated "Crate" ITEM's kind (a different kind is corruption).
    repeat_kind: &'static str,
    /// The quad's last corner (out of range is corruption).
    last_corner: u16,
}
const PRESET_21: Options = Options {
    major: 2,
    minor: 1,
    form: b"LXPM",
    repeat_kind: "mesh",
    last_corner: 3,
};

fn file(o: &Options) -> Vec<u8> {
    let mut body = Vec::new();
    let mut d = Vec::new();
    u32(&mut d, o.major);
    u32(&mut d, o.minor);
    text(&mut d, "Synthetic");
    chunk(&mut body, b"VRSN", &d);
    let mut d = Vec::new();
    text(&mut d, "Wood");
    text(&mut d, "Body");
    chunk(&mut body, b"TAGS", &d);
    // The mesh, written twice as presets do; the second copy adds a
    // visible name. Its parent and the render item's shadeLoc point at
    // items this file didn't save.
    let mut visible = Vec::new();
    text(&mut visible, "visible");
    u16(&mut visible, 3);
    text(&mut visible, "default");
    let mut prst = b"PRST".to_vec();
    text(&mut prst, "1");
    item(
        &mut body,
        "mesh",
        "Crate",
        5,
        &[
            item_layer(),
            (*b"CHNL", visible),
            (*b"ITAG", prst),
            link("parent", 99, 0),
        ],
    );
    let mut vnam = Vec::new();
    text(&mut vnam, "Crate box");
    item(&mut body, o.repeat_kind, "Crate", 5, &[(*b"VNAM", vnam)]);
    // A translation in the mesh's core stack, as a CHNV vector.
    let mut pos = Vec::new();
    text(&mut pos, "pos");
    u16(&mut pos, 2);
    u16(&mut pos, 3);
    for (axis, v) in [("X", 1.), ("Y", 2.), ("Z", 3.)] {
        text(&mut pos, axis);
        f32(&mut pos, v);
    }
    item(
        &mut body,
        "translation",
        "",
        9,
        &[(*b"CHNV", pos), link("xfrmCore", 5, 2)],
    );
    item(
        &mut body,
        "polyRender",
        "Render",
        7,
        &[link("shadeLoc", 98, 0)],
    );
    item(
        &mut body,
        "mask",
        "Crate mask",
        8,
        &[link("shadeLoc", 5, 0)],
    );
    // A mesh item with no layer.
    item(&mut body, "mesh", "Empty", 6, &[]);
    // The scene's setup action, for an item that wasn't saved.
    let mut d = Vec::new();
    text(&mut d, "setup");
    text(&mut d, "setup");
    u32(&mut d, 1);
    let mut owner = Vec::new();
    u32(&mut owner, 50);
    sub(&mut d, b"ITEM", &owner);
    chunk(&mut body, b"ACTN", &d);
    // The mesh's layer: a two-point line first, then the quad, so the
    // quad's tags and corner UV count it as stored polygon 1.
    layer(&mut body, "CrateLayer", 5, o.major);
    points(
        &mut body,
        &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
    );
    polygons(&mut body, &[&[0, 1], &[0, 1, 2, o.last_corner]]);
    for (kind, poly, tag) in [(b"MATR", 1u16, 0u16), (b"PART", 1, 1), (b"MATR", 0, 1)] {
        let mut d = kind.to_vec();
        u16(&mut d, poly);
        u16(&mut d, tag);
        chunk(&mut body, b"PTAG", &d);
    }
    let mut d = b"TXUV".to_vec();
    u16(&mut d, 2);
    text(&mut d, "Texture");
    for (v, (u, w)) in [(0., 0.), (1., 0.), (1., 1.), (0., 1.)]
        .into_iter()
        .enumerate()
    {
        u16(&mut d, v as u16);
        f32(&mut d, u);
        f32(&mut d, w);
    }
    chunk(&mut body, b"VMAP", &d);
    let mut d = b"TXUV".to_vec();
    u16(&mut d, 2);
    text(&mut d, "Texture");
    u16(&mut d, 2);
    u16(&mut d, 1);
    f32(&mut d, 0.25);
    f32(&mut d, 0.75);
    chunk(&mut body, b"VMAD", &d);
    // A layer with no mesh item: a mesh of its own in a preset.
    layer(&mut body, "Loose", 0xffff_ffff, o.major);
    points(&mut body, &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.]]);
    polygons(&mut body, &[&[0, 1, 2]]);
    let mut out = b"FORM".to_vec();
    u32(&mut out, body.len() as u32 + 4);
    out.extend_from_slice(o.form);
    out.extend(body);
    out
}

/// A scene (`LXOB`) through `parse`; a preset (`LXPM`) through
/// `load_mesh_preset`, the only way presets are read.
fn parse(bytes: &[u8]) -> lxo::Result<lxo::Candidate> {
    let mut go = |_: &str| true;
    if &bytes[8..12] != b"LXPM" {
        return lxo::parse(bytes, &mut lxo::Control::new(&mut go));
    }
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "lxl-old-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("preset.lxl");
    std::fs::write(&path, bytes).unwrap();
    let c = lxo::load_mesh_preset(&path, &mut lxo::Control::new(&mut go));
    std::fs::remove_dir_all(&dir).ok();
    c
}

fn note(report: &lxo::Report, text: &str) -> bool {
    report.omissions.keys().any(|k| k.contains(text))
}

#[test]
fn an_older_format_preset_imports_its_meshes() {
    for (major, minor) in [(2, 1), (3, 0), (3, 1)] {
        let c = parse(&file(&Options {
            major,
            minor,
            ..PRESET_21
        }))
        .unwrap_or_else(|e| panic!("{major}.{minor}: {e}"));
        assert!(
            c.report
                .producer
                .contains(&format!("older than 8 (LXO format {major}.{minor})")),
            "{}",
            c.report.producer
        );
        let names: Vec<_> = c.scene.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["Crate", "Loose"]);
        let crate_ = &c.scene.items[0];
        // The repeat's visible name is merged in; the missing parent is
        // dropped, so the mesh sits at the root, placed by its translation.
        assert_eq!(crate_.source_display_name.as_deref(), Some("Crate box"));
        assert_eq!(crate_.parent, None);
        assert!((crate_.transform.translation - glam::DVec3::new(1., 2., 3.)).length() < 1e-6);
        assert_eq!(crate_.mesh.vertices.len(), 4);
        // The line is left out; the quad keeps its own tags and corner UV.
        assert_eq!(crate_.mesh.faces.len(), 1);
        let face = &crate_.mesh.faces[0];
        assert_eq!(face.material_label.as_deref(), Some("Wood"));
        assert_eq!(face.part.as_deref(), Some("Body"));
        let uv = &crate_.mesh.uv_maps["Texture"][0];
        let corner = face.vertices.iter().position(|&v| v == 2).unwrap();
        assert!((uv[corner].unwrap() - glam::DVec2::new(0.25, 0.75)).length() < 1e-6);
        assert_eq!(c.scene.items[1].mesh.faces.len(), 1);
        for text in [
            "one- or two-point polygon",
            "repeated item records merged",
            "links to items not saved",
            "action values for items not saved",
            "preset item tags",
            "layer subdivision/display settings not read",
            "shader items limited to a mesh",
            "mesh items with no layer",
        ] {
            assert!(note(&c.report, text), "{major}.{minor}: no note '{text}'");
        }
    }
}

#[test]
fn format_4_presets_get_the_same_preset_rules() {
    // A Modo 8–10 preset whose layers have no mesh item, or that lists
    // items it didn't save, now imports.
    let mut bytes = file(&Options {
        major: 4,
        minor: 0,
        ..PRESET_21
    });
    let c = parse(&bytes).unwrap();
    assert!(c.scene.items.iter().any(|i| i.name == "Loose"));
    assert!(note(&c.report, "links to items not saved"));
    // As a scene (LXOB), a layer without its mesh item still refuses.
    bytes[8..12].copy_from_slice(b"LXOB");
    let e = parse(&bytes).unwrap_err();
    assert_eq!(e.category, "invalid_layer_item_reference");
}

#[test]
fn corruption_still_refuses() {
    // The same id on two different kinds of item.
    let e = parse(&file(&Options {
        repeat_kind: "locator",
        ..PRESET_21
    }))
    .unwrap_err();
    assert_eq!(e.category, "duplicate_item_id");
    assert!(e.detail.contains("[LXO format 2.1]"), "{}", e.detail);
    // A polygon corner past the last point.
    let e = parse(&file(&Options {
        last_corner: 40,
        ..PRESET_21
    }))
    .unwrap_err();
    assert_eq!(e.category, "invalid_point_reference");
}
