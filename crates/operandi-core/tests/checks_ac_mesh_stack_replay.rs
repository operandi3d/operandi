//! Replays the 2026-11-18 Action Center and Mesh Stack check cases against
//! their exact inputs (`specifications/checks-ac-meshops-2026-11-18`):
//! AC-rotate-Selection, AC-scale-Local, AC-rotate-Local,
//! MOP-MM-M03_useWorld0 and MOP-MM-M04_both0. The cases give behaviour in
//! words; the expected points here follow those words, computed apart from
//! the tools. Pivot cases are left out (Pivot is not built).
use glam::{DMat3, DMat4, DQuat, DVec3, EulerRot};
use operandi_core::mesh_stack::OpStatus;
use operandi_core::mesh_stack_editor::OpRef;
use operandi_core::mesh_stack_items::MERGE_MESHES_OP;
use operandi_core::*;
use serde_json::{Value, json};

fn inputs() -> Value {
    let path = concat!(
        env!("OPERANDI_SPECS_DIR"),
        "/specifications/checks-ac-meshops-2026-11-18/inputs.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn vec3(v: &Value) -> DVec3 {
    let a = v.as_array().unwrap();
    DVec3::new(
        a[0].as_f64().unwrap(),
        a[1].as_f64().unwrap(),
        a[2].as_f64().unwrap(),
    )
}

fn close(a: DVec3, b: DVec3, what: &str) {
    assert!((a - b).length() < 1e-9, "{what}: {a:?} != {b:?}");
}

/// The AC mesh in its moved, turned item, all three polygons selected.
fn ac_scene(center: CenterMode) -> Editor {
    let input = inputs();
    let m = &input["action_center_mesh"];
    let vertices = m["points"].as_array().unwrap().iter().map(vec3).collect();
    let faces = m["polygons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| Face {
            vertices: p
                .as_array()
                .unwrap()
                .iter()
                .map(|i| i.as_u64().unwrap() as u32)
                .collect(),
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        })
        .collect::<Vec<_>>();
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let item = e.scene.active_mut().unwrap();
    item.mesh = Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
    .into();
    item.transform.translation = vec3(&m["item_transform"]["pos"]);
    item.transform.rotation = vec3(&m["item_transform"]["rot"]).map(f64::to_radians);
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0, 1, 2];
    e.dispatch(Command::ActionContext(ActionContext {
        center,
        // Local by name is Local's centre with Local's axes.
        axis: if center == CenterMode::Local {
            AxisOrientation::Local
        } else {
            AxisOrientation::World
        },
    }))
    .unwrap();
    e
}

fn world(e: &Editor) -> DMat4 {
    e.scene.world_matrix(e.scene.active().unwrap().id).unwrap()
}

fn world_points(e: &Editor) -> Vec<DVec3> {
    let w = world(e);
    e.scene
        .active()
        .unwrap()
        .mesh
        .vertices
        .iter()
        .map(|&v| w.transform_point3(v))
        .collect()
}

fn run(e: &mut Editor, tool: Tool, delta: DVec3) -> Vec<DVec3> {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = delta;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    world_points(e)
}

/// The case's Rotate: 30 degrees about Z.
const ROTATE: DVec3 = DVec3::new(0., 0., 30.);
/// The case's Scale: factors 2, 1, 1.
const SCALE: DVec3 = DVec3::new(2., 1., 1.);

fn rotation() -> DQuat {
    let r = ROTATE.map(f64::to_radians);
    DQuat::from_euler(EulerRot::XYZ, r.x, r.y, r.z)
}

/// AC-rotate-Selection: about the middle of the selected points' bounding
/// box, world axes. The box is taken in the item's own space (AC-Q20,
/// provisional).
#[test]
fn ac_rotate_selection() {
    let mut e = ac_scene(CenterMode::Selection);
    let w = world(&e);
    let item = e.scene.active().unwrap();
    let (lo, hi) = item.mesh.vertices.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(l, h), &p| (l.min(p), h.max(p)),
    );
    let centre = w.transform_point3((lo + hi) / 2.);
    let before = world_points(&e);
    let after = run(&mut e, Tool::Rotate, ROTATE);
    for (i, (&p, &q)) in before.iter().zip(&after).enumerate() {
        close(q, centre + rotation() * (p - centre), &format!("point {i}"));
    }
}

/// Each island's frame (A-LOCAL): its item-space box middle; X the
/// direction of most spread of its points (for the strip also the one
/// nearest item X), or item X when the two in-plane spreads tie (the square
/// quad, ACL-Q4); Z the direction of least spread, pointed with its
/// polygons' normals; Y = Z × X. Spreads by power iteration on the
/// covariance.
fn island_frame(e: &Editor, points: &[usize], polygons: &[usize]) -> (DVec3, DMat3) {
    let w = world(e);
    let mesh = &e.scene.active().unwrap().mesh;
    let (lo, hi) = points.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(l, h), &i| (l.min(mesh.vertices[i]), h.max(mesh.vertices[i])),
    );
    let normal: DVec3 = polygons
        .iter()
        .map(|&f| {
            let v: Vec<DVec3> = mesh.faces[f]
                .vertices
                .iter()
                .map(|&i| mesh.vertices[i as usize])
                .collect();
            // Newell's normal of the polygon.
            let mut n = DVec3::ZERO;
            for k in 0..v.len() {
                let (a, b) = (v[k], v[(k + 1) % v.len()]);
                n += DVec3::new(
                    (a.y - b.y) * (a.z + b.z),
                    (a.z - b.z) * (a.x + b.x),
                    (a.x - b.x) * (a.y + b.y),
                );
            }
            n.normalize()
        })
        .sum();
    let cloud: Vec<DVec3> = points.iter().map(|&i| mesh.vertices[i]).collect();
    let mean = cloud.iter().copied().sum::<DVec3>() / cloud.len() as f64;
    let c = cloud.iter().fold(DMat3::ZERO, |c, &p| {
        let d = p - mean;
        c + DMat3::from_cols(d * d.x, d * d.y, d * d.z) / cloud.len() as f64
    });
    let power = |m: DMat3| {
        let mut v = DVec3::new(1., 0.3, 0.1).normalize();
        for _ in 0..2000 {
            v = (m * v).normalize();
        }
        (v, v.dot(m * v))
    };
    let (major, most) = power(c);
    let trace = c.x_axis.x + c.y_axis.y + c.z_axis.z;
    let (least, _) = power(DMat3::from_diagonal(DVec3::splat(trace)) - c);
    let least = if least.dot(normal) < 0. {
        -least
    } else {
        least
    };
    let middle_spread = {
        let m = least.cross(major);
        m.dot(c * m)
    };
    let x_item = if (most - middle_spread).abs() < 1e-6 {
        DVec3::X
    } else if major.x < 0. {
        -major
    } else {
        major
    };
    let z = (DMat3::from_mat4(w).inverse().transpose() * least).normalize();
    let y = z.cross(DMat3::from_mat4(w) * x_item).normalize();
    (
        w.transform_point3((lo + hi) / 2.),
        DMat3::from_cols(y.cross(z), y, z),
    )
}

/// AC-scale-Local and AC-rotate-Local: each connected island about its own
/// centre, along its own axes.
#[test]
fn ac_scale_and_rotate_local() {
    for tool in [Tool::Scale, Tool::Rotate] {
        let mut e = ac_scene(CenterMode::Local);
        let strip = island_frame(&e, &[0, 1, 2, 3, 4, 5], &[0, 1]);
        let flat = island_frame(&e, &[6, 7, 8, 9], &[2]);
        let before = world_points(&e);
        let delta = if tool == Tool::Scale { SCALE } else { ROTATE };
        let after = run(&mut e, tool, delta);
        for (i, (&p, &q)) in before.iter().zip(&after).enumerate() {
            let (c, b) = if i < 6 { strip } else { flat };
            let local = b.transpose() * (p - c);
            let moved = if tool == Tool::Scale {
                local * SCALE
            } else {
                rotation() * local
            };
            close(q, c + b * moved, &format!("{tool:?} point {i}"));
        }
    }
}

fn mesh_item(e: &mut Editor, m: Mesh) -> u64 {
    e.scene.new_mesh().unwrap();
    let id = e.scene.items.last().unwrap().id;
    e.scene.item_mut(id).unwrap().mesh = m.into();
    id
}

/// A case's items: S1 (a cube with its material, placed and turned) and T
/// (an empty mesh item, placed); a Merge Meshes op on T with `options`.
/// Returns the merged result on T.
fn merge_case(case: &str, options: Value) -> Mesh {
    let input = inputs();
    let items = input["meshops"][case]["items"].as_array().unwrap();
    let mut e = Editor::default();
    let mut ids = std::collections::BTreeMap::new();
    for it in items {
        let mesh = match it["shape"].as_str().unwrap() {
            "cube" => {
                let mut m = cube_geometry(
                    vec3(&it["centre"]),
                    DVec3::splat(it["full_size"].as_f64().unwrap()),
                )
                .unwrap();
                for f in m.faces.iter_mut() {
                    f.material = it["material"].as_str().unwrap().into();
                }
                m
            }
            _ => Mesh::default(),
        };
        let id = mesh_item(&mut e, mesh);
        let t = &mut e.scene.item_mut(id).unwrap().transform;
        t.translation = vec3(&it["position"]);
        t.rotation = vec3(&it["rotation_deg"]).map(f64::to_radians);
        ids.insert(it["name"].as_str().unwrap().to_string(), id);
    }
    let target = ids["T"];
    e.scene.select_item(Some(target));
    e.dispatch(Command::MeshOpCreate(MERGE_MESHES_OP.into()))
        .unwrap();
    let op = OpRef {
        item: target,
        op: e
            .scene
            .item(target)
            .unwrap()
            .mesh_stack
            .as_ref()
            .unwrap()
            .ops[0]
            .id,
    };
    let mut values = options.as_object().unwrap().clone();
    values.insert("sources".into(), json!([ids["S1"]]));
    e.dispatch(Command::MeshOpSet { op, values }).unwrap();
    let r = e
        .mesh_stack
        .current(&e.scene, e.scene.item(target).unwrap())
        .unwrap();
    assert_eq!(r.statuses, [OpStatus::Ok], "{case}");
    r.mesh.to_owned_value()
}

fn extent(m: &Mesh) -> (DVec3, DVec3) {
    m.vertices.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(l, h), &p| (l.min(p), h.max(p)),
    )
}

/// MOP-MM-M03_useWorld0: Use World Transforms off places the source just
/// as the default (M01) does, by its world transform; MOP-MM-M04_both0
/// places it as Source World Transform off alone (M02) does, by its own
/// points. Materials are kept either way.
#[test]
fn mop_mm_use_world_transforms_off() {
    let m01 = merge_case("MOP-MM-M01_default", json!({}));
    let m02 = merge_case("MOP-MM-M02_world0", json!({ "world": false }));
    let m03 = merge_case("MOP-MM-M03_useWorld0", json!({ "useWorldXfrm": false }));
    let m04 = merge_case(
        "MOP-MM-M04_both0",
        json!({ "world": false, "useWorldXfrm": false }),
    );
    let (lo, hi) = extent(&m01);
    close(lo, DVec3::new(2.5, -0.5, -0.5), "M01 low");
    close(hi, DVec3::new(3.5, 0.5, 0.5), "M01 high");
    let (lo, hi) = extent(&m02);
    close(lo, DVec3::splat(-0.5), "M02 low");
    close(hi, DVec3::splat(0.5), "M02 high");
    assert_eq!(m03.vertices, m01.vertices, "M03 places as M01");
    assert_eq!(m04.vertices, m02.vertices, "M04 places as M02");
    for (case, m) in [("M03", &m03), ("M04", &m04)] {
        assert_eq!(m.faces.len(), 6, "{case}");
        assert!(m.faces.iter().all(|f| f.material == "mat_S1"), "{case}");
    }
}
