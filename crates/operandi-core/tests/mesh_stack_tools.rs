//! Direct tools as Mesh Stack (MO-40/41, TYPE-IDS.md), group by group. Each
//! op runs its direct tool's engine, so each test checks the op's result
//! on a known mesh, and where the direct command exists, that it matches.
use glam::DVec3;
use operandi_core::mesh_stack::OpStatus;
use operandi_core::mesh_stack_editor::{OpRef, SelOpEdit};
use operandi_core::meshop_selection::{SelKind, SelOp};
use operandi_core::{Command, Editor, Mesh, Mode};
use serde_json::{Value, json};

pub(crate) fn cube() -> Mesh {
    operandi_core::cube_geometry(DVec3::ZERO, DVec3::ONE).unwrap()
}
/// An editor whose one mesh has `base` as its Base Mesh.
pub(crate) fn with_base(base: Mesh) -> (Editor, u64) {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let id = e.scene.items[0].id;
    e.scene.item_mut(id).unwrap().mesh = base.into();
    e.scene.select_item(Some(id));
    (e, id)
}
pub(crate) fn add(e: &mut Editor, item: u64, type_id: &str, values: Value) -> OpRef {
    e.scene.select_item(Some(item));
    e.dispatch(Command::MeshOpCreate(type_id.into())).unwrap();
    let op = OpRef {
        item,
        op: e
            .scene
            .item(item)
            .unwrap()
            .mesh_stack
            .as_ref()
            .unwrap()
            .ops
            .last()
            .unwrap()
            .id,
    };
    let values = values.as_object().unwrap().clone();
    if !values.is_empty() {
        e.dispatch(Command::MeshOpSet { op, values }).unwrap();
    }
    op
}
pub(crate) fn only(e: &mut Editor, op: OpRef, index: &str) {
    e.dispatch(Command::MeshOpSelOp {
        op,
        edit: SelOpEdit::Add(SelOp::new(SelKind::Index {
            index: index.into(),
        })),
    })
    .unwrap();
}
/// The stack's result; every op must have run cleanly.
pub(crate) fn ok(e: &Editor, item: u64) -> Mesh {
    let r = e
        .mesh_stack
        .current(&e.scene, e.scene.item(item).unwrap())
        .unwrap();
    assert!(
        r.statuses.iter().all(|s| *s == OpStatus::Ok),
        "{:?}",
        r.statuses
    );
    (*r.mesh).clone()
}
/// The same direct command on the same mesh, polygons `faces` selected.
pub(crate) fn direct(base: Mesh, faces: &[u32], command: Command) -> Mesh {
    let (mut e, id) = with_base(base);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = faces.to_vec();
    e.dispatch(command).unwrap();
    (*e.scene.item(id).unwrap().mesh).clone()
}

// ---- Polygon ----

#[test]
fn polygon_extrude_pushes_the_chosen_polygon_out_with_sides() {
    let (mut e, id) = with_base(cube());
    let op = add(&mut e, id, "poly.extrude.item", json!({ "shiftY": 1.0 }));
    only(&mut e, op, "0");
    let m = ok(&e, id);
    assert_eq!(m.faces.len(), 6 + 4);
    let top = m.vertices.iter().map(|p| p.y).fold(f64::MIN, f64::max);
    assert!(top > 0.5, "{top}");
    // Zero extent: nothing to do.
    let (mut e, id) = with_base(cube());
    add(&mut e, id, "poly.extrude.item", json!({}));
    assert_eq!(ok(&e, id), cube());
}

#[test]
fn polygon_flip_matches_the_direct_command() {
    let (mut e, id) = with_base(cube());
    let op = add(&mut e, id, "pmodel.invertPolys.item", json!({}));
    only(&mut e, op, "0,2");
    assert_eq!(ok(&e, id), direct(cube(), &[0, 2], Command::FlipPolygons));
}

#[test]
fn polygon_merge_joins_the_chosen_polygons() {
    // A 2x1 grid: two quads in a plane merge into one.
    let base = operandi_core::segmented_cube(DVec3::ZERO, DVec3::ONE, [2, 1, 1]).unwrap();
    let n = base.faces.len();
    let (mut e, id) = with_base(base.clone());
    let op = add(&mut e, id, "pmodel.mergePolygons.item", json!({}));
    // The two polygons facing +Y.
    let up: Vec<u32> = (0..n as u32)
        .filter(|&f| {
            base.faces[f as usize]
                .vertices
                .iter()
                .all(|&v| (base.vertices[v as usize].y - 0.5).abs() < 1e-9)
        })
        .collect();
    assert_eq!(up.len(), 2);
    only(&mut e, op, &format!("{},{}", up[0], up[1]));
    assert_eq!(ok(&e, id).faces.len(), n - 1);
}

#[test]
fn material_tag_tags_only_the_chosen_polygons() {
    let (mut e, id) = with_base(cube());
    let op = add(
        &mut e,
        id,
        "pmodel.materialTag.item",
        json!({ "materialName": "Red" }),
    );
    only(&mut e, op, "1");
    let m = ok(&e, id);
    assert_eq!(m.faces[1].material, "Red");
    assert_ne!(m.faces[0].material, "Red");
}

#[test]
fn symmetrize_copies_one_side_onto_the_other() {
    // A cube shifted to +X: Positive to Negative mirrors it to -X.
    let mut base = cube();
    for p in &mut base.vertices {
        p.x += 1.0;
    }
    let (mut e, id) = with_base(base);
    add(&mut e, id, "symmetrize.meshop.item", json!({}));
    let m = ok(&e, id);
    let lo = m.vertices.iter().map(|p| p.x).fold(f64::MAX, f64::min);
    assert!((lo + 1.5).abs() < 1e-9, "{lo}");
}

#[test]
fn detriangulate_and_triangulate_round_trip() {
    let (mut e, id) = with_base(cube());
    add(&mut e, id, "pmodel.triangulate.item", json!({}));
    assert_eq!(ok(&e, id).faces.len(), 12);
    add(&mut e, id, "detriangulate.meshop.item", json!({}));
    assert_eq!(ok(&e, id).faces.len(), 6);
}

#[test]
fn unsubdivide_undoes_a_subdivide() {
    let (mut e, id) = with_base(cube());
    add(&mut e, id, "subdivide.tool.item", json!({ "type": 0 }));
    assert_eq!(ok(&e, id).faces.len(), 24);
    let op = add(&mut e, id, "poly.unsubdiv.item", json!({}));
    assert_eq!(ok(&e, id).faces.len(), 24, "Iteration 0 does nothing");
    let mut v = serde_json::Map::new();
    v.insert("iteration".into(), 1.into());
    e.dispatch(Command::MeshOpSet { op, values: v }).unwrap();
    assert_eq!(ok(&e, id).faces.len(), 6);
}

#[test]
fn polygon_unify_runs() {
    let (mut e, id) = with_base(cube());
    add(&mut e, id, "pmodel.unifyPolygons.item", json!({}));
    assert_eq!(ok(&e, id).faces.len(), 6);
}

#[test]
fn spike_raises_a_point_in_each_chosen_polygon() {
    let (mut e, id) = with_base(cube());
    let op = add(&mut e, id, "poly.spikey.item", json!({ "factor": 1.0 }));
    only(&mut e, op, "0");
    let m = ok(&e, id);
    assert_eq!((m.faces.len(), m.vertices.len()), (5 + 4, 9));
}

#[test]
fn delete_removes_the_chosen_polygons_and_keep_vertices_keeps_points() {
    let (mut e, id) = with_base(cube());
    let op = add(&mut e, id, "delete.meshop.item", json!({}));
    only(&mut e, op, "0");
    let m = ok(&e, id);
    assert_eq!((m.faces.len(), m.vertices.len()), (5, 8));
    // Every polygon (no selection) without Keep Vertices: nothing left.
    let (mut e, id) = with_base(cube());
    add(&mut e, id, "delete.meshop.item", json!({}));
    let m = ok(&e, id);
    assert_eq!((m.faces.len(), m.vertices.len()), (0, 0));
    let (mut e, id) = with_base(cube());
    add(&mut e, id, "delete.meshop.item", json!({ "keep": true }));
    let m = ok(&e, id);
    assert_eq!((m.faces.len(), m.vertices.len()), (0, 8));
}

// ---- Edge ----

fn edges_of(m: &Mesh) -> usize {
    m.edges().len()
}

#[test]
fn edge_bevel_on_every_edge_and_on_the_edges_of_chosen_polygons() {
    let (mut e, id) = with_base(cube());
    add(&mut e, id, "edge.bevel.item", json!({ "inset": 0.1 }));
    // Every edge bevelled flat: 6 faces + 12 edge strips + 8 corners.
    assert_eq!(ok(&e, id).faces.len(), 26);
    // Value 0: nothing to do.
    let (mut e, id) = with_base(cube());
    add(&mut e, id, "edge.bevel.item", json!({}));
    assert_eq!(ok(&e, id), cube());
    // Only the top polygon's four edges.
    let (mut e, id) = with_base(cube());
    let op = add(&mut e, id, "edge.bevel.item", json!({ "inset": 0.1 }));
    only(&mut e, op, "0");
    let m = ok(&e, id);
    assert!(m.faces.len() > 6 && m.faces.len() < 26, "{}", m.faces.len());
}

#[test]
fn edge_chamfer_segments_add_rows() {
    let (mut e, id) = with_base(cube());
    let op = add(&mut e, id, "edge.chamfer.item", json!({ "offset": 0.1 }));
    only(&mut e, op, "0");
    let one = ok(&e, id).faces.len();
    let mut v = serde_json::Map::new();
    v.insert("segments".into(), 3.into());
    e.dispatch(Command::MeshOpSet { op, values: v }).unwrap();
    assert!(ok(&e, id).faces.len() > one);
}

#[test]
fn edge_extrude_and_relax_and_subdivide_and_split_run() {
    // Extrude the open edges of a single quad.
    let base =
        operandi_core::polygon_remove::delete_geometry(&cube(), &[1, 2, 3, 4, 5], &[]).unwrap();
    let (mut e, id) = with_base(base.clone());
    add(
        &mut e,
        id,
        "edge.extrude.item",
        json!({ "shift": 0.5, "inset": 0.1 }),
    );
    let m = ok(&e, id);
    assert!(m.faces.len() > base.faces.len());

    let (mut e, id) = with_base(cube());
    add(&mut e, id, "pmodel.edge.subdivide", json!({}));
    assert_eq!(ok(&e, id).vertices.len(), 8 + 12);

    let (mut e, id) = with_base(cube());
    let op = add(&mut e, id, "pmodel.splitEdges.item", json!({}));
    only(&mut e, op, "0");
    let m = ok(&e, id);
    assert!(edges_of(&m) >= 12, "{}", edges_of(&m));

    let (mut e, id) = with_base(cube());
    add(&mut e, id, "edge.relax.item", json!({}));
    assert_eq!(ok(&e, id), cube(), "Offset 0 does nothing");
}

// ---- Vertex ----

#[test]
fn vertex_bevel_cuts_the_chosen_corners() {
    let (mut e, id) = with_base(cube());
    add(&mut e, id, "vert.bevel.item", json!({ "inset": 0.1 }));
    // Every corner cut: 6 faces + 8 corner triangles.
    assert_eq!(ok(&e, id).faces.len(), 14);
}

#[test]
fn vertex_extrude_merge_split_create_and_set_position() {
    let (mut e, id) = with_base(cube());
    let op = add(
        &mut e,
        id,
        "vert.extrude.item",
        json!({ "inset": 0.1, "shift": 0.2 }),
    );
    only(&mut e, op, "0");
    assert!(ok(&e, id).faces.len() > 6);

    // Two loose quads of the cube that share an edge's place: its two
    // point pairs merge.
    let quad = |keep: u32| {
        let gone: Vec<u32> = (0..6).filter(|&f| f != keep).collect();
        operandi_core::polygon_remove::delete_geometry(&cube(), &gone, &[]).unwrap()
    };
    let (a, b) = (quad(0), quad(2));
    let mut base = a.clone();
    operandi_core::append_mesh(&mut base, b.clone()).unwrap();
    let shared = a.vertices.iter().filter(|p| b.vertices.contains(p)).count();
    let (mut e, id) = with_base(base);
    add(&mut e, id, "vert.merge.item", json!({}));
    assert_eq!(ok(&e, id).vertices.len(), 8 - shared);

    let (mut e, id) = with_base(cube());
    let op = add(&mut e, id, "vert.split.meshop.item", json!({}));
    only(&mut e, op, "0");
    assert!(ok(&e, id).vertices.len() > 8);

    let (mut e, id) = with_base(cube());
    add(
        &mut e,
        id,
        "pmodel.createVertex",
        json!({ "position.Y": 3.0 }),
    );
    let m = ok(&e, id);
    assert_eq!(m.vertices.len(), 9);
    assert_eq!(m.vertices[8], DVec3::new(0., 3., 0.));

    let (mut e, id) = with_base(cube());
    let op = add(
        &mut e,
        id,
        "vert.setpos.meshop.item",
        json!({ "enableY": true, "positionY": 0.0 }),
    );
    // The top polygon (all its points at y 0.5) drops to y 0.
    let c = cube();
    let top = (0..6)
        .find(|&f| {
            c.faces[f]
                .vertices
                .iter()
                .all(|&v| c.vertices[v as usize].y == 0.5)
        })
        .unwrap();
    only(&mut e, op, &top.to_string());
    let m = ok(&e, id);
    let flat = m.vertices.iter().filter(|p| p.y == 0.).count();
    assert_eq!(flat, 4, "the chosen polygon's four points");
}

// ---- Duplicate ----

#[test]
fn radial_array_and_clone_copy_and_save_as_clone_effectors() {
    let mut base = cube();
    for p in &mut base.vertices {
        p.x += 3.0;
    }
    let (mut e, id) = with_base(base);
    add(&mut e, id, "radial.array.item", json!({}));
    assert_eq!(ok(&e, id).faces.len(), 6 * 8, "Count 8 pieces round Y");
    let (mut e2, id2) = with_base(cube());
    add(&mut e2, id2, "linear.clone.item", json!({ "num": 2 }));
    let m = ok(&e2, id2);
    assert_eq!(m.faces.len(), 18);
    let top = m.vertices.iter().map(|p| p.y).fold(f64::MIN, f64::max);
    assert!((top - 2.5).abs() < 1e-9, "{top}");
    for (e, id, generator) in [(&e, id, "pmodel.helix"), (&e2, id2, "pmodel.linear")] {
        let bytes = operandi_core::persistence::encode(&e.scene).unwrap();
        let doc: Value = serde_json::from_slice(&bytes).unwrap();
        let text = doc.to_string();
        assert!(
            text.contains("effector.clone.item") && text.contains(generator),
            "{generator}"
        );
        let back = operandi_core::persistence::decode(&bytes).unwrap();
        assert_eq!(back, e.scene);
        let theirs = operandi_core::mesh_stack::Runtime::default()
            .current(&back, back.item(id).unwrap())
            .unwrap();
        assert_eq!(*theirs.mesh, ok(e, id));
    }
}

// ---- Edit ----

#[test]
fn slice_cuts_the_cube_through_the_middle() {
    let (mut e, id) = with_base(cube());
    // A line along X at y 0 with the plane holding Z: cuts the 4 sides.
    add(
        &mut e,
        id,
        "poly.knife.item",
        json!({ "vectorY": 0.0, "vectorZ": 1.0 }),
    );
    assert_eq!(ok(&e, id).faces.len(), 10);
}

#[test]
fn axis_slice_cuts_by_number() {
    let (mut e, id) = with_base(cube());
    add(&mut e, id, "poly.julienne.item", json!({ "numX": 1 }));
    assert_eq!(ok(&e, id).faces.len(), 10);
}

#[test]
fn saved_poly_julienne_item_loads_as_axis_slice() {
    let (e, id) = with_base(cube());
    let mut doc: Value =
        serde_json::from_slice(&operandi_core::persistence::encode(&e.scene).unwrap()).unwrap();
    doc["version"] = json!(39);
    doc["scene"]["items"][0]["mesh_ops"] = json!({
        "ops": [{
            "type": "poly.julienne.item",
            "version": 1,
            "id": 0,
            "enabled": true,
            "values": {
                "mode": 0, "numX": 1, "numY": 0, "numZ": 0,
                "sizX": 1.0, "sizY": 1.0, "sizZ": 1.0,
                "offX": 0.0, "offY": 0.0, "offZ": 0.0, "hold": false
            }
        }],
        "next_id": 1
    });

    let scene = operandi_core::persistence::decode(&serde_json::to_vec(&doc).unwrap()).unwrap();
    let item = scene.item(id).unwrap();
    let op = &item.mesh_stack.as_ref().unwrap().ops[0];
    assert_eq!(op.kind.label(), "Axis Slice");
    let result = e.mesh_stack.current(&scene, item).unwrap();
    assert_eq!(result.statuses, [OpStatus::Ok]);
    assert_eq!(result.mesh.faces.len(), 10);
}

#[test]
fn polygon_reduce_hits_the_number_and_zero_is_a_pass_through() {
    let sphere = operandi_core::globe_geometry(
        DVec3::ZERO,
        operandi_core::RadialParameters {
            radius: DVec3::splat(1.),
            sides: 24,
            segments: 12,
        },
    )
    .unwrap();
    let (mut e, id) = with_base(sphere.clone());
    let op = add(&mut e, id, "poly.reduct.item", json!({}));
    assert_eq!(ok(&e, id), sphere, "PRD-11: Number 0");
    for (n, expect) in [(100, 100), (50, 50), (100_000, sphere.faces.len())] {
        let mut v = serde_json::Map::new();
        v.insert("number".into(), n.into());
        e.dispatch(Command::MeshOpSet { op, values: v }).unwrap();
        assert_eq!(ok(&e, id).faces.len(), expect, "Number {n}");
    }
}

#[test]
fn mesh_cleanup_merges_coincident_points() {
    let quad = |keep: u32| {
        let gone: Vec<u32> = (0..6).filter(|&f| f != keep).collect();
        operandi_core::polygon_remove::delete_geometry(&cube(), &gone, &[]).unwrap()
    };
    let mut base = quad(0);
    operandi_core::append_mesh(&mut base, quad(2)).unwrap();
    let before = base.vertices.len();
    let (mut e, id) = with_base(base);
    add(&mut e, id, "meshcleanup.tool.item", json!({}));
    assert!(ok(&e, id).vertices.len() <= before);
}

// ---- Deform ----

#[test]
fn jitter_is_seeded_and_push_and_smooth_move_points() {
    let (mut e, id) = with_base(cube());
    let op = add(
        &mut e,
        id,
        "xfrm.jitter.item",
        json!({ "rangeX": 0.1, "rangeY": 0.1 }),
    );
    let a = ok(&e, id);
    assert_eq!(a.faces.len(), 6);
    assert_ne!(a.vertices, cube().vertices);
    assert!(
        a.vertices
            .iter()
            .zip(&cube().vertices)
            .all(|(p, q)| (p.z - q.z).abs() < 1e-12 && (p.x - q.x).abs() <= 0.1)
    );
    let mut v = serde_json::Map::new();
    v.insert("seed".into(), 5.into());
    e.dispatch(Command::MeshOpSet { op, values: v }).unwrap();
    assert_ne!(
        ok(&e, id).vertices,
        a.vertices,
        "another seed, another shake"
    );

    let (mut e, id) = with_base(cube());
    add(&mut e, id, "xfrm.push.item", json!({ "dist": 0.5 }));
    let top = ok(&e, id)
        .vertices
        .iter()
        .map(|p| p.y)
        .fold(f64::MIN, f64::max);
    assert!(top > 0.5);

    let (mut e, id) = with_base(cube());
    add(
        &mut e,
        id,
        "smooth.meshop.item",
        json!({ "lockCorner": false }),
    );
    let m = ok(&e, id);
    let r = m
        .vertices
        .iter()
        .map(|p| p.length())
        .fold(f64::MIN, f64::max);
    assert!(r < cube().vertices[0].length(), "corners pulled in");
}

// ---- UV ----

#[test]
fn uv_unwrap_writes_a_map_pack_and_transform_move_it() {
    let (mut e, id) = with_base(cube());
    add(&mut e, id, "uv.unwrap.item", json!({}));
    let m = ok(&e, id);
    let rows = m.uv_maps.get("Texture").expect("Unwrap writes Texture");
    assert!(
        rows.iter().flatten().all(|uv| uv.is_some()),
        "every corner has a UV"
    );
    add(&mut e, id, "uvpack.meshop.item", json!({}));
    let packed = ok(&e, id);
    let inside = packed.uv_maps["Texture"]
        .iter()
        .flatten()
        .flatten()
        .all(|uv| (-1e-9..=1. + 1e-9).contains(&uv.x) && (-1e-9..=1. + 1e-9).contains(&uv.y));
    assert!(inside, "packed into 0..1");
    add(
        &mut e,
        id,
        "pmodel.transformMap",
        json!({ "offset.U": 2.0 }),
    );
    let moved = ok(&e, id);
    let a = packed.uv_maps["Texture"][0][0].unwrap();
    let b = moved.uv_maps["Texture"][0][0].unwrap();
    assert!((b.x - a.x - 2.).abs() < 1e-9 && (b.y - a.y).abs() < 1e-9);
}
