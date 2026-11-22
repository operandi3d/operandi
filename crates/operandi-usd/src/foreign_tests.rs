//! USD-IQ23: other programs' maps, sets and blend shapes on import, and
//! their round trip through Operandi's own export. Blender-made and
//! Houdini-style fixtures in fixtures/usd/foreign.
use operandi_core::interchange::{ExportFormat, Imported};
use operandi_core::{Item, MeshAttributes, Scene};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/usd/foreign")
        .join(name)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("operandi-usd-foreign")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn import(path: &Path) -> Imported {
    crate::import(path, &Default::default()).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn mesh_item<'a>(scene: &'a Scene, name: &str) -> &'a Item {
    scene
        .items
        .iter()
        .find(|i| i.is_mesh() && i.name == name)
        .unwrap_or_else(|| panic!("no mesh item {name}"))
}

/// Exports `scene` as .usda and reads it back.
fn again(scene: &Scene, name: &str) -> Imported {
    let path = scratch(name).join("again.usda");
    crate::export(&path, scene, ExportFormat::Usda).unwrap();
    import(&path)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-5
}

/// The parts of the attributes these tests compare, with numbers rounded
/// so float32 storage doesn't matter.
fn extras(a: &MeshAttributes) -> String {
    let r = |x: f64| format!("{:.5}", x + 0.0);
    let mut out = Vec::new();
    for (n, m) in &a.point_weights {
        let v: Vec<String> = m.iter().map(|(p, w)| format!("{p}:{}", r(*w))).collect();
        out.push(format!("weight {n} {v:?}"));
    }
    for (n, m) in &a.corner_weights {
        let v: Vec<Vec<String>> = m
            .iter()
            .map(|f| f.iter().map(|c| c.map(r).unwrap_or_default()).collect())
            .collect();
        out.push(format!("corner weight {n} {v:?}"));
    }
    for (n, m) in &a.typed_maps {
        let p: Vec<String> = m
            .points
            .iter()
            .map(|(p, v)| format!("{p}:{:?}", v.iter().map(|x| r(*x)).collect::<Vec<_>>()))
            .collect();
        let c: Vec<Vec<String>> = m
            .corners
            .iter()
            .map(|f| {
                f.iter()
                    .map(|c| {
                        c.as_ref()
                            .map(|v| format!("{:?}", v.iter().map(|x| r(*x)).collect::<Vec<_>>()))
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();
        out.push(format!("typed {n} {} {} {p:?} {c:?}", m.kind, m.dimension));
    }
    for (n, m) in &a.relative_morphs {
        let v: Vec<String> = m
            .iter()
            .map(|(p, d)| format!("{p}:{} {} {}", r(d.x), r(d.y), r(d.z)))
            .collect();
        out.push(format!("morph {n} {v:?}"));
    }
    for (n, s) in &a.polygon_sets {
        out.push(format!("polygon set {n} {s:?}"));
    }
    for (n, s) in &a.vertex_sets {
        out.push(format!("point set {n} {s:?}"));
    }
    for (n, s) in &a.edge_sets {
        out.push(format!("edge set {n} {s:?}"));
    }
    out.join("\n")
}

/// Blender 5.2.2's cube (fixtures/usd/make_foreign_extras.py): two vertex
/// groups, two shape keys, point and corner colours and three attributes
/// come in with Blender's values, and survive Operandi's own round trip.
#[test]
fn blender_vertex_groups_shape_keys_and_attributes_come_across() {
    let imported = import(&fixture("blender_extras.usda"));
    let cube = mesh_item(&imported.scene, "Cube");
    let a = &cube.mesh.attributes;
    // Vertex groups: float primvars per point.
    let top = &a.point_weights["top"];
    let side = &a.point_weights["side"];
    assert_eq!(top.len(), 8);
    for p in 0..8u32 {
        // Blender's cube: odd points are on top (z > 0), 4..8 at x > 0.
        assert!(
            near(top[&p], if p % 2 == 1 { 0.75 } else { 0.0 }),
            "top {p}"
        );
        let want = if p >= 4 { 0.25 + 0.1 * p as f64 } else { 0.0 };
        assert!(near(side[&p], want), "side {p}: {}", side[&p]);
    }
    // A float attribute is a weight map too; ints and vectors typed maps.
    assert!(near(a.point_weights["heat"][&3], 1.5));
    let tag = &a.typed_maps["tag"];
    assert_eq!((tag.kind.as_str(), tag.dimension), ("weight1", 1));
    assert_eq!(tag.points[&5], vec![15.0]);
    let dir = &a.typed_maps["dir"];
    assert_eq!((dir.kind.as_str(), dir.dimension), ("vector3", 3));
    // Blender's Z-up (i, 0, 1) is Operandi's Y-up (i, 1, 0).
    let first = dir.corners[0][0].clone().unwrap();
    assert!(near(first[1], 1.0) && near(first[2], 0.0), "{first:?}");
    // Skinning weights aren't maps.
    assert!(!a.point_weights.keys().any(|k| k.starts_with("skel:")));
    // Shape keys: Tall raises the top by 1 (Y up), Wide pushes +X by 0.5.
    let tall = &a.relative_morphs["Tall"];
    let wide = &a.relative_morphs["Wide"];
    for p in 0..8u32 {
        let t = tall.get(&p).copied().unwrap_or_default();
        let w = wide.get(&p).copied().unwrap_or_default();
        let up = if p % 2 == 1 { 1.0 } else { 0.0 };
        let out = if p >= 4 { 0.5 } else { 0.0 };
        assert!(
            near(t.y, up) && near(t.x, 0.0) && near(t.z, 0.0),
            "Tall {p}: {t}"
        );
        assert!(
            near(w.x, out) && near(w.y, 0.0) && near(w.z, 0.0),
            "Wide {p}: {w}"
        );
    }
    // Colours: the point one per corner, the corner one as written.
    let col = &a.corner_colors["Col"];
    let corner = &a.corner_colors["CornerCol"];
    let p = cube.mesh.faces[0].vertices[0];
    assert!(near(col[0][0].unwrap().x, p as f64 / 8.0));
    assert!(near(corner[0][1].unwrap().x, 1.0 / 24.0));
    assert!(
        !imported
            .notes
            .iter()
            .any(|n| n.contains("top") || n.contains("Tall")),
        "{:?}",
        imported.notes
    );

    // Operandi's export writes them back as its own maps.
    let back = again(&imported.scene, "blender-rt");
    let b = &mesh_item(&back.scene, "Cube").mesh.attributes;
    assert_eq!(extras(a), extras(b));
    assert_eq!(
        a.corner_colors.keys().collect::<Vec<_>>(),
        b.corner_colors.keys().collect::<Vec<_>>()
    );
}

/// Houdini-style attributes and groups (fixtures/usd/foreign/houdini_style.usda,
/// hand-written): every interpolation and type, primitive groups with and
/// without a family, a point group, and a material subset that stays one.
#[test]
fn houdini_style_attributes_and_groups_come_across() {
    let imported = import(&fixture("houdini_style.usda"));
    let grid = mesh_item(&imported.scene, "grid");
    let a = &grid.mesh.attributes;
    assert!(near(a.point_weights["Alpha"][&5], 0.5));
    assert!(near(a.point_weights["pscale"][&1], 0.2));
    // Per primitive (uniform): on each corner of its polygon.
    assert_eq!(a.corner_weights["area"][1], vec![Some(2.0); 4]);
    assert!(near(a.corner_weights["vtx_mask"][1][3].unwrap(), 0.875));
    let kind = |n: &str| (a.typed_maps[n].kind.clone(), a.typed_maps[n].dimension);
    assert_eq!(kind("rest"), ("absolute_position3".into(), 3));
    assert_eq!(kind("v"), ("vector3".into(), 3));
    assert_eq!(kind("id"), ("weight1".into(), 1));
    assert_eq!(kind("prim_id"), ("weight1".into(), 1));
    assert_eq!(kind("offset2"), ("uv2".into(), 2));
    assert_eq!(kind("rgba"), ("rgba4".into(), 4));
    assert_eq!(a.typed_maps["id"].points[&2], vec![12.0]);
    assert_eq!(a.typed_maps["prim_id"].corners[1], vec![Some(vec![8.0]); 4]);
    assert_eq!(a.typed_maps["offset2"].corners[0][2], Some(vec![0.5, 1.5]));
    assert_eq!(a.typed_maps["rgba"].points[&1], vec![1.0, 0.75, 0.5, 0.0]);
    assert!(a.corner_colors.contains_key("Cd"));
    assert!(!a.point_weights.contains_key("detail_scale"));
    assert!(
        imported.notes.iter().any(|n| n.contains("detail_scale")),
        "{:?}",
        imported.notes
    );
    // Groups.
    assert_eq!(a.polygon_sets["left"], [0].into());
    assert_eq!(a.polygon_sets["right"], [1].into());
    assert_eq!(a.vertex_sets["edge_pts"], [0, 3].into());
    assert!(!a.polygon_sets.contains_key("plastic_faces"));

    let back = again(&imported.scene, "houdini-rt");
    let b = &mesh_item(&back.scene, "grid").mesh.attributes;
    assert_eq!(extras(a), extras(b));
}
