//! Mesh presets (`.lxl`, FORM `LXPM`): a synthetic preset written here from
//! the container layout (no Modo-saved file) loads as meshes with their
//! polygon tags and UVs, and its PRVW gives the thumbnail.
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

const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

/// A quad layer with a material and part tag and a UV map, no mesh ITEM.
fn preset() -> Vec<u8> {
    preset_with_pivot([0.0; 3])
}

/// `preset()` with the LAYR pivot set.
fn preset_with_pivot(pivot: [f32; 3]) -> Vec<u8> {
    let mut body = Vec::new();
    let mut d = Vec::new();
    u32(&mut d, 16);
    u32(&mut d, 0);
    text(&mut d, "Synthetic");
    chunk(&mut body, b"VRSN", &d);
    let mut d = Vec::new();
    u32(&mut d, 2);
    chunk(&mut body, b"ENCO", &d);
    let mut d = Vec::new();
    text(&mut d, "mesh");
    text(&mut d, "A test tile");
    chunk(&mut body, b"DESC", &d);
    let mut d = vec![0, 16, 0, 16, 0, 0, 0, 0, 0, 0, 0, 1];
    d.extend_from_slice(&PNG);
    d.extend_from_slice(b"not really a png");
    chunk(&mut body, b"PRVW", &d);
    let mut d = Vec::new();
    text(&mut d, "Tile");
    text(&mut d, "Top");
    chunk(&mut body, b"TAGS", &d);
    // LAYR: index, flags, pivot, name, parent, subdivision, curve angle,
    // scale pivot + six values, item id (none here: 0xFFFFFFFF).
    let mut d = Vec::new();
    u16(&mut d, 0);
    u16(&mut d, 0);
    for v in pivot {
        f32(&mut d, v);
    }
    text(&mut d, "Tile Mesh");
    u16(&mut d, 0xffff);
    f32(&mut d, 2.);
    f32(&mut d, 0.5);
    for _ in 0..9 {
        f32(&mut d, 0.);
    }
    u32(&mut d, 0xffff_ffff);
    chunk(&mut body, b"LAYR", &d);
    let mut d = Vec::new();
    for (x, z) in [(0., 0.), (1., 0.), (1., 1.), (0., 1.)] {
        f32(&mut d, x);
        f32(&mut d, 0.);
        f32(&mut d, -z);
    }
    chunk(&mut body, b"PNTS", &d);
    let mut d = b"FACE".to_vec();
    u16(&mut d, 4);
    for v in [0, 1, 2, 3] {
        u16(&mut d, v);
    }
    chunk(&mut body, b"POLS", &d);
    for (kind, tag) in [(b"MATR", 0u16), (b"PART", 1)] {
        let mut d = kind.to_vec();
        u16(&mut d, 0);
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
    let mut out = b"FORM".to_vec();
    u32(&mut out, body.len() as u32 + 4);
    out.extend_from_slice(b"LXPM");
    out.extend(body);
    out
}

#[test]
fn a_mesh_preset_without_items_loads_its_layers_as_meshes() {
    let dir = std::env::temp_dir().join(format!("lxl-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Tile.lxl");
    std::fs::write(&path, preset()).unwrap();
    let mut go = |_: &str| true;
    let c = lxo::load_mesh_preset(&path, &mut lxo::Control::new(&mut go))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c.scene.items.len(), 1);
    let item = &c.scene.items[0];
    assert_eq!(item.name, "Tile Mesh");
    assert_eq!(item.mesh.vertices.len(), 4);
    assert_eq!(item.mesh.faces.len(), 1);
    let face = &item.mesh.faces[0];
    assert_eq!(face.material_label.as_deref(), Some("Tile"));
    assert_eq!(face.part.as_deref(), Some("Top"));
    let uv = &item.mesh.uv_maps["Texture"][0];
    assert!(uv.iter().all(Option::is_some));
    // The browser's thumbnail is the PNG after PRVW's 12-byte header.
    let png = operandi_core::preset::modo::preview_png(&path).unwrap();
    assert!(png.starts_with(&PNG));
    assert!(png.ends_with(b"not really a png"));
    // The same layout as a scene (LXOB) still leaves item-less layers out.
    let mut scene = preset();
    scene[8..12].copy_from_slice(b"LXOB");
    let mut go = |_: &str| true;
    let e = lxo::parse(&scene, &mut lxo::Control::new(&mut go));
    assert!(e.map(|c| c.scene.items.is_empty()).unwrap_or(true));
    std::fs::remove_dir_all(&dir).ok();
}

/// Q-LX-04 (answered 2026-11-15): stored points are absolute; a nonzero
/// layer pivot doesn't move them and is noted.
#[test]
fn a_layer_pivot_does_not_move_the_points() {
    let dir = std::env::temp_dir().join(format!("lxl-pivot-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let load = |name: &str, bytes: Vec<u8>| {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        let mut go = |_: &str| true;
        lxo::load_mesh_preset(&path, &mut lxo::Control::new(&mut go)).unwrap()
    };
    let base = load("Base.lxl", preset());
    let c = load("Pivot.lxl", preset_with_pivot([0.5, -2.0, 1.25]));
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(
        c.scene.items[0].mesh.vertices,
        base.scene.items[0].mesh.vertices
    );
    assert!(
        c.report.notes.contains(
            &"Mesh 'Tile Mesh': layer pivot (0.5, -2, 1.25) kept as information only, not applied"
                .to_string()
        ),
        "{:?}",
        c.report.notes
    );
    assert!(base.report.notes.is_empty());
}
