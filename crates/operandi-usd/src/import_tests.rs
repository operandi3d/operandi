//! The scene builder on stages described by hand, without OpenUSD (the
//! bridge's own tests in tests.rs read real files with `--features usd`).
use crate::import::{Assets, NORMAL_MAP, build};
use crate::stage::{Instancer, MaterialData, MeshData, Prim, Primvar, StageData, Subset, Trs};
use glam::{DMat4, DQuat, DVec2, DVec3, DVec4};
use operandi_core::interchange::{Imported, UsdImportOptions, UsdMaterials};
use operandi_core::subdivision::BoundaryRule;
use operandi_core::{Item, MeshSlot, PolygonRepresentation, Result, Scene};
use std::path::Path;

/// No pictures on disk.
struct NoAssets;
impl Assets for NoAssets {
    fn image_size(&self, _: &Path) -> Option<(u32, u32)> {
        None
    }
    fn copy_packaged(&self, _: &str, _: &Path) -> Result<()> {
        Err("no packages here".into())
    }
}

fn stage(up: &str) -> StageData {
    StageData {
        up_axis: up.into(),
        meters_per_unit: 1.0,
        ..Default::default()
    }
}

/// USD's row-major row-vector array for a glam matrix.
fn usd(m: DMat4) -> [f64; 16] {
    m.to_cols_array()
}

fn prim(s: &mut StageData, name: &str, ty: &str, parent: i64, local: DMat4) -> usize {
    s.prims.push(Prim {
        path: format!("/{name}"),
        name: name.into(),
        ty: ty.into(),
        parent,
        visible: true,
        local: Some(usd(local)),
        material: Some(-1),
        ..Default::default()
    });
    s.prims.len() - 1
}

/// A Mesh prim under `parent` with polygons `faces` over `points`.
fn mesh_prim(
    s: &mut StageData,
    name: &str,
    parent: i64,
    points: &[[f32; 3]],
    faces: &[&[i32]],
) -> usize {
    let data = MeshData {
        points: s.push_points(points),
        counts: s.push_i32(&faces.iter().map(|f| f.len() as i32).collect::<Vec<_>>()),
        indices: s.push_i32(&faces.concat()),
        orientation: "rightHanded".into(),
        scheme: "none".into(),
        boundary: "edgeAndCorner".into(),
        fvli: "cornersPlus1".into(),
        ..Default::default()
    };
    s.meshes.push(data);
    let p = prim(s, name, "Mesh", parent, DMat4::IDENTITY);
    s.prims[p].mesh = Some(s.meshes.len() - 1);
    p
}

const QUAD: [[f32; 3]; 4] = [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]];

fn run(s: &StageData, options: &UsdImportOptions) -> Imported {
    build(s, Path::new("/nowhere/scene.usdc"), options, &NoAssets).unwrap()
}

fn item<'a>(scene: &'a Scene, name: &str) -> &'a Item {
    scene.items.iter().find(|i| i.name == name).unwrap()
}

/// A Z-up file: the one /root Xform drops out, an Xform holding a Mesh is
/// one mesh item, a turn about the file's up is a turn about the scene's
/// Y, and points land where the file puts them (Y-up).
#[test]
fn a_z_up_hierarchy_keeps_local_transforms_turned_to_y_up() {
    let mut s = stage("Z");
    let root = prim(&mut s, "root", "Xform", -1, DMat4::IDENTITY);
    let local = DMat4::from_translation(DVec3::new(1., 2., 3.))
        * DMat4::from_rotation_z(90f64.to_radians())
        * DMat4::from_scale(DVec3::new(1., 2., 3.));
    let parent = prim(&mut s, "Box", "Xform", root as i64, local);
    s.prims[parent].trs = Some(Trs {
        t: [1., 2., 3.],
        r: [0., 0., 90.],
        s: [1., 2., 3.],
        order: "XYZ".into(),
    });
    mesh_prim(&mut s, "BoxShape", parent as i64, &QUAD, &[&[0, 1, 2, 3]]);
    let child = prim(
        &mut s,
        "Child",
        "Xform",
        parent as i64,
        DMat4::from_translation(DVec3::X),
    );
    mesh_prim(
        &mut s,
        "ChildShape",
        child as i64,
        &QUAD,
        &[&[0, 1, 2], &[0, 2, 3]],
    );
    let imported = run(&s, &UsdImportOptions::default());
    let scene = &imported.scene;
    assert!(scene.items.iter().all(|i| i.name != "root"));
    let b = item(scene, "Box");
    assert!(b.parent.is_none() && b.is_mesh());
    assert_eq!(b.mesh.faces.len(), 1);
    assert!(
        b.transform
            .translation
            .abs_diff_eq(DVec3::new(1., 3., -2.), 1e-9)
    );
    assert!(b.transform.scale.abs_diff_eq(DVec3::new(1., 3., 2.), 1e-9));
    assert!(
        b.transform
            .rotation_quat()
            .abs_diff_eq(DQuat::from_rotation_y(90f64.to_radians()), 1e-9)
    );
    // A point of the child's mesh: where the file puts it, turned.
    let c = item(scene, "Child");
    assert_eq!(c.parent, Some(b.id));
    let file = local * DMat4::from_translation(DVec3::X);
    let want = file.transform_point3(DVec3::new(1., 1., 0.));
    let got = scene
        .world_matrix(c.id)
        .unwrap()
        .transform_point3(c.mesh.vertices[2]);
    assert!(
        got.abs_diff_eq(DVec3::new(want.x, want.z, -want.y), 1e-9),
        "{got}"
    );
    assert!(
        imported.notes[0]
            .starts_with("2 item(s), 3 polygon(s): 1 quad(s), 0 n-gon(s), 2 triangle(s)"),
        "{:?}",
        imported.notes
    );
}

/// The options override the file's units and up axis.
#[test]
fn units_and_up_axis_can_be_overridden() {
    let mut s = stage("Y");
    s.meters_per_unit = 0.01;
    mesh_prim(&mut s, "Quad", -1, &QUAD, &[&[0, 1, 2, 3]]);
    let cm = run(&s, &UsdImportOptions::default());
    assert!(
        item(&cm.scene, "Quad").mesh.vertices[2].abs_diff_eq(DVec3::new(0.01, 0.01, 0.), 1e-12)
    );
    let z = run(
        &s,
        &UsdImportOptions {
            up_axis: Some('Z'),
            meters_per_unit: Some(1.0),
            ..Default::default()
        },
    );
    assert!(item(&z.scene, "Quad").mesh.vertices[2].abs_diff_eq(DVec3::new(1., 0., -1.), 1e-12));
}

/// n-gons, left-handed winding, UV primvars (st as the primary map, one
/// indexed), colours, normals and smoothing.
#[test]
fn polygons_and_their_maps_come_across() {
    let mut s = stage("Y");
    let pent: [[f32; 3]; 5] = [
        [0., 0., 0.],
        [2., 0., 0.],
        [3., 1., 0.],
        [1., 2., 0.],
        [-1., 1., 0.],
    ];
    let p = mesh_prim(&mut s, "Pent", -1, &pent, &[&[0, 1, 2, 3, 4]]);
    let m = s.prims[p].mesh.unwrap();
    s.meshes[m].orientation = "leftHanded".into();
    let st = s.push_f32(&[0., 0., 1., 0., 1., 1., 0., 1., 0.5, 0.5]);
    let second = s.push_f32(&[0.25, 0.75, 0.5, 0.5]);
    let second_idx = s.push_i32(&[0, 1, 0, 1, 0]);
    let col = s.push_f32(&[1., 0., 0., 0., 1., 0., 0., 0., 1., 1., 1., 0., 0., 1., 1.]);
    let flat = s.push_f32(&[0., 0., 1.]);
    let tint = s.push_f32(&[0.5, 0.5, 0.5]);
    let mesh = &mut s.meshes[m];
    mesh.primvars = vec![
        Primvar {
            name: "st".into(),
            kind: "uv".into(),
            dims: 2,
            interpolation: "faceVarying".into(),
            values: st,
            indices: None,
            ..Default::default()
        },
        Primvar {
            name: "second".into(),
            kind: "uv".into(),
            dims: 2,
            interpolation: "faceVarying".into(),
            values: second,
            indices: Some(second_idx),
            ..Default::default()
        },
        Primvar {
            name: "Col".into(),
            kind: "color".into(),
            dims: 3,
            interpolation: "vertex".into(),
            values: col,
            indices: None,
            ..Default::default()
        },
        Primvar {
            name: "displayColor".into(),
            kind: "color".into(),
            dims: 3,
            interpolation: "constant".into(),
            values: tint,
            indices: None,
            ..Default::default()
        },
    ];
    mesh.normals = Some(Primvar {
        interpolation: "constant".into(),
        dims: 3,
        values: flat,
        ..Default::default()
    });
    let imported = run(&s, &UsdImportOptions::default());
    let pentagon = item(&imported.scene, "Pent");
    let face = &pentagon.mesh.faces[0];
    assert_eq!(
        face.vertices,
        [0, 4, 3, 2, 1],
        "left-handed reverses, first corner first"
    );
    assert!(!face.smooth, "one normal for every corner: flat");
    let uv = &pentagon.mesh.uv_maps[operandi_core::PRIMITIVE_UV_MAP];
    // Corner 1 is file corner 4.
    assert_eq!(uv[0][1], Some(DVec2::new(0.5, 0.5)));
    // Indexed: file corner 4 reads element 0.
    assert_eq!(
        pentagon.mesh.uv_maps["second"][0][1],
        Some(DVec2::new(0.25, 0.75))
    );
    let col = &pentagon.mesh.attributes.corner_colors["Col"];
    assert_eq!(
        col[0][1],
        Some(DVec4::new(0., 1., 1., 1.)),
        "point 4's colour"
    );
    assert!(
        !pentagon
            .mesh
            .attributes
            .corner_colors
            .contains_key("displayColor")
    );
    assert_eq!(pentagon.mesh.attributes.normal_map_order, [NORMAL_MAP]);
    let notes = imported.notes.join("\n");
    assert!(notes.contains("1 n-gon(s)"), "{notes}");
    assert!(
        notes.contains("Constant colour primvar(s) displayColor left out"),
        "{notes}"
    );
}

/// catmullClark: Catmull-Clark polygons, the boundary rules, creases
/// (sharpness ÷ 10 per edge of a chain), corners reported.
#[test]
fn subdivision_and_creases_come_across() {
    let mut s = stage("Y");
    let p = mesh_prim(&mut s, "Cage", -1, &QUAD, &[&[0, 1, 2, 3]]);
    let ci = s.push_i32(&[0, 1, 2, 0]);
    let cl = s.push_i32(&[4]);
    let cs = s.push_f32(&[5.]);
    let corners = s.push_i32(&[3]);
    let m = s.prims[p].mesh.unwrap();
    let mesh = &mut s.meshes[m];
    mesh.scheme = "catmullClark".into();
    mesh.boundary = "edgeOnly".into();
    mesh.fvli = "all".into();
    mesh.crease_indices = Some(ci);
    mesh.crease_lengths = Some(cl);
    mesh.crease_sharpnesses = Some(cs);
    mesh.corner_indices = Some(corners);
    let imported = run(&s, &UsdImportOptions::default());
    let cage = item(&imported.scene, "Cage");
    assert_eq!(
        cage.mesh.attributes.representations,
        [PolygonRepresentation::CatmullClark]
    );
    let creases: Vec<(u32, u32, f64)> = cage
        .mesh
        .attributes
        .creases
        .iter()
        .map(|c| (c.a, c.b, c.weight))
        .collect();
    // 0-1 and 1-2 are edges; 2-0 is a diagonal, not an edge.
    assert_eq!(creases, [(0, 1, 0.5), (1, 2, 0.5)]);
    let settings = cage.subdivision.as_ref().unwrap();
    assert_eq!(settings.boundary_rule, BoundaryRule::Smooth);
    assert!(settings.linear_uvs);
    assert!(
        imported
            .notes
            .iter()
            .any(|n| n.contains("1 corner sharpness(es) left out"))
    );
}

/// A PointInstancer's copies share their prototype's mesh; bound
/// materials and subsets become tags; hidden prims come in hidden.
#[test]
fn instances_tags_and_visibility() {
    let mut s = stage("Y");
    s.materials = vec![
        MaterialData {
            name: "Red".into(),
            ..Default::default()
        },
        MaterialData {
            name: "Default".into(),
            ..Default::default()
        },
    ];
    let quad = mesh_prim(&mut s, "Tagged", -1, &QUAD, &[&[0, 1, 2], &[0, 2, 3]]);
    let faces = s.push_i32(&[1]);
    s.prims[quad].material = Some(1);
    s.prims[quad].subsets = vec![Subset {
        name: "redPart".into(),
        faces,
        material: 0,
    }];
    let hidden = prim(&mut s, "Hidden", "Xform", -1, DMat4::IDENTITY);
    s.prims[hidden].visible = false;
    mesh_prim(
        &mut s,
        "HiddenShape",
        hidden as i64,
        &QUAD,
        &[&[0, 1, 2, 3]],
    );
    let scatter = prim(&mut s, "Scatter", "PointInstancer", -1, DMat4::IDENTITY);
    let proto = mesh_prim(&mut s, "Proto", -1, &QUAD, &[&[0, 1, 2, 3]]);
    s.prims[proto].proto_of = Some(scatter);
    let indices = s.push_i32(&[0, 0]);
    let mut matrices = usd(DMat4::IDENTITY).to_vec();
    matrices.extend(usd(DMat4::from_translation(DVec3::new(5., 0., 0.))));
    let matrices = s.push_f64(&matrices);
    s.prims[scatter].instancer = Some(Instancer {
        prototypes: vec![proto],
        proto_indices: indices,
        matrices,
        hidden: None,
        inactive: None,
    });
    let imported = run(
        &s,
        &UsdImportOptions {
            materials: UsdMaterials::None,
            ..Default::default()
        },
    );
    let scene = &imported.scene;
    let tagged = item(scene, "Tagged");
    let tags: Vec<&str> = tagged
        .mesh
        .faces
        .iter()
        .map(|f| f.material.as_str())
        .collect();
    assert_eq!(tags, ["", "Red"], "\"Default\" is the default material");
    let hidden = item(scene, "Hidden");
    assert!(!hidden.visible && hidden.hides_children);
    let scatter = item(scene, "Scatter");
    let copies: Vec<&Item> = scene
        .items
        .iter()
        .filter(|i| i.parent == Some(scatter.id))
        .collect();
    assert_eq!(copies.len(), 2);
    assert!(MeshSlot::same(&copies[0].mesh, &copies[1].mesh));
    assert!(
        copies[1]
            .transform
            .translation
            .abs_diff_eq(DVec3::new(5., 0., 0.), 1e-12)
    );
    assert_eq!(scene.items.iter().filter(|i| i.name == "Proto").count(), 2);
    assert!(imported.notes.iter().any(|n| {
        n.starts_with("Instances: 2 instance(s) in the file; 1 item(s) came in as Mesh Instances")
    }));
}

#[test]
fn a_stage_without_polygons_is_refused_and_bad_meshes_are_reported() {
    let mut s = stage("Y");
    prim(&mut s, "Empty", "Xform", -1, DMat4::IDENTITY);
    let e = build(
        &s,
        Path::new("/x.usda"),
        &UsdImportOptions::default(),
        &NoAssets,
    )
    .unwrap_err();
    assert!(e.contains("no polygons"), "{e}");
    mesh_prim(&mut s, "Bad", -1, &QUAD, &[&[0, 1, 9]]);
    mesh_prim(&mut s, "Good", -1, &QUAD, &[&[0, 1, 2, 3]]);
    let imported = run(&s, &UsdImportOptions::default());
    assert!(
        imported
            .notes
            .iter()
            .any(|n| n == "Bad: a polygon refers to a missing point"),
        "{:?}",
        imported.notes
    );
    assert_eq!(item(&imported.scene, "Good").mesh.faces.len(), 1);
}

/// Blender closes a crease ring from its last point back to its first
/// (Jason's chess pieces: `60, 0`, `95, 65`), and a file may list an edge
/// both ways. Operandi keeps each crease lower point first, once; those
/// used to fail the mesh rules and empty the whole mesh.
#[test]
fn creases_running_backwards_or_twice_keep_the_mesh() {
    let mut s = stage("Z");
    let parent = prim(
        &mut s,
        "Piece",
        "Xform",
        -1,
        DMat4::from_scale(DVec3::new(1., 1., 0.005)),
    );
    let p = mesh_prim(&mut s, "Shape", parent as i64, &QUAD, &[&[0, 1, 2, 3]]);
    let ci = s.push_i32(&[3, 0, 1, 0, 0, 1, 2, 1]);
    let cl = s.push_i32(&[2, 2, 2, 2]);
    let cs = s.push_f32(&[5., 2., 8., 1.]);
    let m = s.prims[p].mesh.unwrap();
    let mesh = &mut s.meshes[m];
    mesh.scheme = "catmullClark".into();
    mesh.crease_indices = Some(ci);
    mesh.crease_lengths = Some(cl);
    mesh.crease_sharpnesses = Some(cs);
    let imported = run(&s, &UsdImportOptions::default());
    let piece = item(&imported.scene, "Piece");
    assert_eq!(piece.mesh.faces.len(), 1, "{:?}", imported.notes);
    let creases: Vec<(u32, u32, f64)> = piece
        .mesh
        .attributes
        .creases
        .iter()
        .map(|c| (c.a, c.b, c.weight))
        .collect();
    // 0-1 listed twice keeps its sharper weight.
    assert_eq!(creases, [(0, 1, 0.8), (0, 3, 0.5), (1, 2, 0.1)]);
    // The file's unapplied Z scale is the scene's Y scale.
    assert!(
        piece
            .transform
            .scale
            .abs_diff_eq(DVec3::new(1., 0.005, 1.), 1e-12)
    );
}

/// U5: with Instances on, the second copy of a PointInstancer prototype is
/// the first's Mesh Instance; Copies leaves them separate.
#[test]
fn instances_or_copies() {
    let mut s = stage("Y");
    let scatter = prim(&mut s, "Scatter", "PointInstancer", -1, DMat4::IDENTITY);
    let proto = mesh_prim(&mut s, "Proto", -1, &QUAD, &[&[0, 1, 2, 3]]);
    s.prims[proto].proto_of = Some(scatter);
    let indices = s.push_i32(&[0, 0, 0]);
    let mut m = Vec::new();
    for x in 0..3 {
        m.extend(usd(DMat4::from_translation(DVec3::new(x as f64, 0., 0.))));
    }
    let matrices = s.push_f64(&m);
    s.prims[scatter].instancer = Some(Instancer {
        prototypes: vec![proto],
        proto_indices: indices,
        matrices,
        hidden: None,
        inactive: None,
    });
    let inst = run(&s, &UsdImportOptions::default());
    let copies: Vec<&Item> = inst
        .scene
        .items
        .iter()
        .filter(|i| i.name == "Proto")
        .collect();
    assert_eq!(copies[0].instance_of, None);
    assert!(
        copies[1..]
            .iter()
            .all(|c| c.instance_of == Some(copies[0].id))
    );
    let separate = run(
        &s,
        &UsdImportOptions {
            instances: false,
            ..Default::default()
        },
    );
    assert!(separate.scene.items.iter().all(|i| i.instance_of.is_none()));
}

/// USD-IQ21: the camera prim the stage's RenderSettings name (here inside
/// a Blender-style Xform) becomes the render camera; none named, none
/// chosen.
#[test]
fn the_named_render_camera_is_chosen() {
    let mut s = stage("Y");
    mesh_prim(&mut s, "Floor", -1, &QUAD, &[&[0, 1, 2, 3]]);
    for name in ["Front", "Side"] {
        let x = prim(&mut s, name, "Xform", -1, DMat4::IDENTITY);
        let c = prim(&mut s, name, "Camera", x as i64, DMat4::IDENTITY);
        s.prims[c].path = format!("/{name}/{name}");
        s.prims[c].camera = Some(Default::default());
    }
    let none = run(&s, &UsdImportOptions::default());
    assert_eq!(none.scene.render_camera, None);
    s.render_camera = "/Side/Side".into();
    let named = run(&s, &UsdImportOptions::default());
    assert_eq!(
        named.scene.render_camera,
        Some(item(&named.scene, "Side").id)
    );
}

/// A scene that fills up brings in what fits, reports the rest as left
/// out and finishes instead of refusing the file (scene-budget).
#[test]
fn a_full_scene_takes_what_fits_and_reports_the_rest() {
    let mut s = stage("Y");
    for k in 0..12 {
        let group = prim(
            &mut s,
            &format!("Group{k}"),
            "Xform",
            -1,
            DMat4::from_translation(DVec3::X * k as f64),
        );
        // Two meshes with transforms of their own: two items below it.
        for side in ["A", "B"] {
            let m = mesh_prim(
                &mut s,
                &format!("Shape{k}{side}"),
                group as i64,
                &QUAD,
                &[&[0, 1, 2, 3]],
            );
            s.prims[m].path = format!("/Group{k}/Shape{k}{side}");
            s.prims[m].local = Some(usd(DMat4::from_translation(DVec3::Y)));
        }
    }
    let options = UsdImportOptions::default();
    let all = run(&s, &options);
    assert_eq!(all.scene.items.iter().filter(|i| i.is_mesh()).count(), 24);
    let full = crate::import::build_within(
        &s,
        Path::new("/nowhere/scene.usdc"),
        &options,
        &NoAssets,
        10,
    )
    .unwrap();
    let meshes = full.scene.items.iter().filter(|i| i.is_mesh()).count();
    let prims = full
        .scene
        .items
        .iter()
        .filter(|i| i.name.starts_with("Group") || i.name.starts_with("Shape"))
        .count();
    assert_eq!(prims, 10, "{:?}", full.notes);
    assert!(meshes > 0);
    // Groups 0-2 came in whole and group 3's locator (10 items); its two
    // shapes and groups 4-11 (8 × 3 = 24) are left out.
    let line = full
        .notes
        .iter()
        .find(|n| n.contains("left out: the scene was full"))
        .expect("a report line");
    assert!(
        line.starts_with("26 item(s) left out") && line.contains("/Group3/Shape3A"),
        "{line}"
    );
    full.scene.validate().unwrap();
}
