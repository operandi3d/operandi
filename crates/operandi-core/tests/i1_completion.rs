use glam::{DVec2, DVec3, DVec4};
use operandi_core::*;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}
fn fixed(name: &str) -> Scene {
    serde_json::from_slice(&bytes(&format!("i1_completion/{name}.json"))).unwrap()
}
fn old(name: &str) -> Scene {
    serde_json::from_slice(&bytes(&format!("i1/{name}.expected.json"))).unwrap()
}
fn params(z: f64) -> Parameters {
    Parameters {
        extent: DVec3::Z * z,
        extrude_mode: ExtrudeMode::Shift,
        extrude_sides: 1,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }
}
fn editor(scene: Scene) -> Editor {
    let mut e = Editor {
        scene,
        ..Default::default()
    };
    e.synchronize_identities().unwrap();
    e
}
fn stage(e: &mut Editor, z: f64) {
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(params(z))).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
}
fn roles(ids: &MeshIdentity, next: u64) -> Value {
    json!({"vertices":ids.vertices,"faces":ids.faces,"corners":ids.corners,"edges":ids.edges.iter().map(|((a,b),id)|json!({"vertices":[a,b],"id":id})).collect::<Vec<_>>(),"logical_faces":ids.logical_faces,"next_id":next})
}
fn check_ids(e: &Editor, folder: &str, name: &str) {
    let oracle: Value =
        serde_json::from_slice(&bytes(&format!("{folder}/{name}-ids.json"))).unwrap();
    assert_eq!(
        roles(
            &e.identities.current.meshes[&1],
            e.identities.next_id().unwrap()
        ),
        oracle
    );
}

#[test]
fn c01_two_stages_zero_or_nonzero_full_graphs_history_and_branch() {
    for (name, first_z, first_expected) in [("moved", 0.5, "cube-moved"), ("zero", 0., "cube-zero")]
    {
        let mut e = editor(old("cube-source"));
        let original = e.identities.current.clone();
        stage(&mut e, first_z);
        assert_eq!(e.scene, old(first_expected));
        check_ids(&e, "i1", first_expected);
        let first_ids = e.identities.current.clone();
        stage(&mut e, 0.5);
        assert_eq!(e.scene, fixed(&format!("{name}-second")));
        check_ids(&e, "i1_completion", &format!("{name}-second"));
        assert_eq!(e.undo.len(), 2);
        let second_ids = e.identities.current.clone();
        let token = e.identities.publication;
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, old(first_expected));
        assert_eq!(e.identities.current, first_ids);
        assert_eq!(e.identities.next_id().unwrap(), 115);
        assert!(e.identities.publication > token);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, old("cube-source"));
        assert_eq!(e.identities.current, original);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, old(first_expected));
        assert_eq!(e.identities.current, first_ids);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, fixed(&format!("{name}-second")));
        assert_eq!(e.identities.current, second_ids);
        assert_eq!(e.identities.next_id().unwrap(), 115);
        e.dispatch(Command::Undo).unwrap();
        stage(&mut e, 0.25);
        assert_eq!(e.scene, fixed(&format!("{name}-branch")));
        check_ids(&e, "i1_completion", &format!("{name}-branch"));
        assert!(e.redo.is_empty());
        assert_eq!(e.undo.len(), 2);
        assert_eq!(e.identities.current.meshes[&1].faces[1], FaceId(10));
        assert_eq!(
            e.identities.current.meshes[&1].corners[1],
            first_ids.meshes[&1].corners[1]
        );
        if name == "zero" {
            assert_eq!(
                e.scene.items[0].mesh.attributes.degenerate_faces,
                BTreeSet::from([6, 7, 8, 9])
            );
            assert_ne!(
                e.identities.current.meshes[&1].vertices[4],
                e.identities.current.meshes[&1].vertices[8]
            );
        }
    }
}

#[test]
fn c02_equal_coincident_assignment_seam_absence_and_assigned_zero() {
    let mut e = editor(fixed("equal-seam-source"));
    stage(&mut e, 0.);
    assert_eq!(e.scene, fixed("equal-seam-zero"));
    check_ids(&e, "i1", "cube-zero");
    let m = &e.scene.items[0].mesh;
    assert_eq!(m.vertices[4], m.vertices[8]);
    assert_ne!(
        e.identities.current.meshes[&1].vertices[4],
        e.identities.current.meshes[&1].vertices[8]
    );
    assert_eq!(m.uv_maps["Texture"][1][0], Some(DVec2::ZERO));
    assert_eq!(m.uv_maps["Texture"][6][1], Some(DVec2::ZERO));
    assert_eq!(m.attributes.uv_records["Texture"][1][0], Some(100));
    assert_eq!(m.attributes.uv_records["Texture"][6][1], Some(211));
    assert_eq!(m.uv_maps["Texture"][1][2], None);
    assert_eq!(m.attributes.uv_records["Texture"][1][2], None);
    assert_eq!(m.uv_maps["Texture"][1][0].unwrap().x.to_bits(), 0);
    let ids = e.identities.current.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, fixed("equal-seam-source"));
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, fixed("equal-seam-zero"));
    assert_eq!(e.identities.current, ids);
}

#[test]
fn c03_explicit_caller_cap_deletion_and_retired_handle() {
    let mut e = editor(old("cube-source"));
    stage(&mut e, 0.5);
    let source = e.scene.clone();
    let source_ids = e.identities.current.meshes[&1].clone();
    let cap = FaceHandle {
        scope: source_ids.scope,
        face: FaceId(10),
    };
    let permutation = [9, 8, 7, 6, 5, 4, 3, 2, 0];
    // Explicit caller remapping for this adapter control, not a product delete tool.
    let mut prepared = source.clone();
    let before = &source.items[0].mesh;
    let m = &mut prepared.items[0].mesh;
    m.faces = permutation
        .iter()
        .map(|&f| before.faces[f].clone())
        .collect();
    for (name, table) in &mut m.uv_maps {
        *table = permutation
            .iter()
            .map(|&f| before.uv_maps[name][f].clone())
            .collect();
    }
    for (name, table) in &mut m.attributes.uv_records {
        *table = permutation
            .iter()
            .map(|&f| before.attributes.uv_records[name][f].clone())
            .collect();
    }
    for (name, members) in &mut m.attributes.polygon_sets {
        *members = permutation
            .iter()
            .enumerate()
            .filter(|(_, f)| before.attributes.polygon_sets[name].contains(&(**f as u32)))
            .map(|(i, _)| i as u32)
            .collect();
    }
    prepared.selection.faces.clear();
    assert_eq!(prepared, fixed("delete-cap"));
    prepared.validate().unwrap();
    let mapped = source_ids
        .remap_faces(before, &prepared.items[0].mesh, &permutation)
        .unwrap();
    let oracle: Value =
        serde_json::from_slice(&bytes("i1_completion/delete-cap-ids.json")).unwrap();
    assert_eq!(roles(&mapped, 83), oracle);
    assert!(mapped.face_index(cap).unwrap_err().contains("Retired"));
    assert!(prepared.items[0].mesh.attributes.polygon_sets["top"].is_empty());
    assert!(prepared.selection.faces.is_empty());
    assert_eq!(source_ids.face_index(cap).unwrap(), 1);
    assert_eq!(source, old("cube-moved"));
}

#[test]
fn c04_logical_order_reversal_is_independent_of_physical_layout() {
    let mut e = editor(fixed("logical-source"));
    let mesh = e.scene.items[0].mesh.clone();
    let original = e.identities.current.meshes[&1].clone();
    e.identities
        .current
        .meshes
        .get_mut(&1)
        .unwrap()
        .logical_faces
        .reverse();
    e.identities.current.validate(&e.scene).unwrap();
    check_ids(&e, "i1_completion", "logical");
    e.dispatch(Command::SelectPolygonSet("all".into())).unwrap();
    assert_eq!(e.scene, fixed("logical-recalled"));
    check_ids(&e, "i1_completion", "logical");
    assert_eq!(e.scene.items[0].mesh, mesh);
    let ids = &e.identities.current.meshes[&1];
    assert_eq!(ids.faces, original.faces);
    assert_eq!(ids.corners, original.corners);
    assert_eq!(ids.layout, original.layout);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, fixed("logical-source"));
    check_ids(&e, "i1_completion", "logical");
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, fixed("logical-recalled"));
}

fn refusal(name: &str, scene: Scene, context: ActionContext) {
    scene
        .validate()
        .unwrap_or_else(|err| panic!("{name} must be a valid source: {err}"));
    let mut e = editor(scene);
    e.dispatch(Command::ActionContext(context)).unwrap();
    let mut parameters = params(0.5);
    if !e.scene.items[0].mesh.uv_maps.contains_key("Texture") {
        parameters.extrude_uv_map = None;
        parameters.extrude_uv_selected = false;
        // No-map fixture starts with the already chosen None preset. Changing
        // Make UVs from the legacy U default requires a selected map by design.
        e.extrude_preset = parameters.clone();
    }
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(parameters)).unwrap();
    let before = e.clone();
    let mark = e.identities.next_id().unwrap();
    assert!(
        e.dispatch(Command::Apply).is_err(),
        "unsupported {name} must refuse"
    );
    assert!(
        e == before,
        "{name} changed accepted state/session/history/token"
    );
    assert_eq!(
        e.identities.next_id().unwrap(),
        mark,
        "{name} reserved IDs before eligibility"
    );
}

#[test]
fn c05_valid_unsupported_authored_payload_matrix_is_atomic() {
    let mut cases = Vec::new();
    for (kind, dimension) in [
        ("uv2", 2),
        ("weight1", 1),
        ("rgba4", 4),
        ("normal3", 3),
        ("vector3", 3),
        ("tangent_basis6", 6),
    ] {
        let mut s = old("cube-source");
        s.items[0].mesh.attributes.typed_maps.insert(
            "Typed".into(),
            TypedMap {
                kind: kind.into(),
                dimension,
                zero_default: true,
                points: BTreeMap::from([(0, vec![0.25; dimension as usize])]),
                corners: Vec::new(),
            },
        );
        cases.push((kind, s));
    }
    for name in [
        "vertex-set",
        "point-uv",
        "point-weight",
        "point-color",
        "corner-weight",
        "corner-color",
        "normal",
        "morph",
        "crease",
        "smoothing",
        "representation",
        "face-uv",
    ] {
        let mut s = old("cube-source");
        let m = &mut s.items[0].mesh;
        match name {
            "vertex-set" => {
                m.attributes
                    .vertex_sets
                    .insert("V".into(), BTreeSet::from([0, 4]));
            }
            "point-uv" => {
                m.attributes.point_uvs.insert(
                    "Texture".into(),
                    BTreeMap::from([(0, DVec2::new(0.25, 0.5))]),
                );
            }
            "point-weight" => {
                m.attributes
                    .point_weights
                    .insert("W".into(), BTreeMap::from([(0, 0.75)]));
            }
            "point-color" => {
                m.attributes.point_colors.insert(
                    "C".into(),
                    BTreeMap::from([(0, DVec4::new(0.25, 0.5, 0.75, 1.))]),
                );
            }
            "corner-weight" => {
                let mut a = vec![vec![None; 4]; 6];
                a[0][0] = Some(0.75);
                m.attributes.corner_weights.insert("W".into(), a);
            }
            "corner-color" => {
                let mut a = vec![vec![None; 4]; 6];
                a[0][0] = Some(DVec4::ONE);
                m.attributes.corner_colors.insert("C".into(), a);
            }
            "normal" => {
                let mut a = vec![vec![None; 4]; 6];
                a[0][0] = Some(DVec3::Z);
                m.attributes.corner_normals.insert("N".into(), a);
                m.attributes.normal_map_order = vec!["N".into()];
            }
            "morph" => {
                m.attributes
                    .relative_morphs
                    .insert("D".into(), BTreeMap::from([(0, DVec3::X * 0.25)]));
            }
            "crease" => {
                m.attributes.creases.push(MeshCrease {
                    a: 0,
                    b: 1,
                    weight: 0.75,
                });
            }
            "smoothing" => {
                m.attributes.smoothing_labels = vec![Some("7".into()); 6];
            }
            "representation" => {
                m.attributes.representations = vec![PolygonRepresentation::CatmullClark; 6];
            }
            "face-uv" => {
                m.faces[1].uv = Some(vec![DVec2::ZERO; 4]);
            }
            _ => unreachable!(),
        }
        cases.push((name, s));
    }
    assert_eq!(cases.len(), 18);
    for (name, s) in cases {
        refusal(name, s, Default::default());
    }
}

fn quad_scene(points: Vec<DVec3>) -> Scene {
    let mut s = Scene::default();
    s.new_mesh().unwrap();
    s.items[0].mesh = Mesh {
        vertices: points,
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    }
    .into();
    s.items[0]
        .mesh
        .attributes
        .polygon_sets
        .insert("cap".into(), BTreeSet::from([0]));
    s.selection.faces = vec![0];
    s
}

#[test]
fn c06_valid_unsupported_context_topology_and_selection_are_atomic() {
    for (name, t) in [
        (
            "translated",
            Transform {
                translation: DVec3::new(3., 0., 0.),
                ..Default::default()
            },
        ),
        (
            "rotated",
            Transform {
                rotation: DVec3::new(0., 0.5, 0.),
                ..Default::default()
            },
        ),
        (
            "scaled",
            Transform {
                scale: DVec3::new(2., 1., 1.),
                ..Default::default()
            },
        ),
    ] {
        let mut s = old("cube-source");
        s.items[0].transform = t;
        refusal(name, s, Default::default());
    }
    for (name, c) in [
        (
            "origin",
            ActionContext {
                center: CenterMode::Origin,
                ..Default::default()
            },
        ),
        (
            "selection-axis",
            ActionContext {
                axis: AxisOrientation::Selection,
                ..Default::default()
            },
        ),
        (
            "workplane-axis",
            ActionContext {
                axis: AxisOrientation::Workplane(0),
                ..Default::default()
            },
        ),
    ] {
        refusal(name, old("cube-source"), c);
    }
    let mut s = old("cube-source");
    s.selection.faces = vec![0, 1];
    refusal("multiple selected", s, Default::default());
    refusal(
        "nonplanar",
        quad_scene(vec![
            DVec3::ZERO,
            DVec3::X,
            DVec3::new(1., 1., 0.25),
            DVec3::Y,
        ]),
        Default::default(),
    );
    refusal(
        "concave",
        quad_scene(vec![
            DVec3::ZERO,
            DVec3::new(2., 0., 0.),
            DVec3::new(0.5, 0.5, 0.),
            DVec3::new(0., 2., 0.),
        ]),
        Default::default(),
    );
}

#[test]
fn c07_malformed_sources_are_separate_validation_refusals() {
    for name in [
        "bad-set",
        "orphan-record",
        "conflicting-record",
        "repeated-index",
    ] {
        let mut s = old("cube-source");
        let m = &mut s.items[0].mesh;
        match name {
            "bad-set" => {
                m.attributes.polygon_sets.get_mut("top").unwrap().insert(99);
            }
            "orphan-record" => m.uv_maps.get_mut("Texture").unwrap()[1][0] = None,
            "conflicting-record" => {
                m.attributes.uv_records.get_mut("Texture").unwrap()[1][1] = Some(100)
            }
            "repeated-index" => m.faces[1].vertices[1] = m.faces[1].vertices[0],
            _ => unreachable!(),
        }
        assert!(
            s.validate().is_err(),
            "{name} is malformed,not valid unsupported"
        );
        let mut e = Editor {
            scene: s,
            ..Default::default()
        };
        let before = e.clone();
        let mark = e.identities.next_id().unwrap();
        assert!(e.dispatch(Command::Activate(Tool::Extrude)).is_err());
        assert!(e == before);
        assert_eq!(e.identities.next_id().unwrap(), mark);
    }
}

#[test]
fn c08_byte_identical_unported_topology_replacement_retires_handles() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    let p = Parameters {
        size: DVec3::splat(2.),
        ..Default::default()
    };
    e.dispatch(Command::Preview(p.clone())).unwrap();
    let scene = e.scene.clone();
    let encoded = persistence::encode(&scene).unwrap();
    let original = e.identities.current.clone();
    let ticket = e.identity_ticket().unwrap();
    let before_mark = e.identities.next_id().unwrap();
    let history = e.undo.clone();
    let h = FaceHandle {
        scope: original.meshes[&1].scope,
        face: original.meshes[&1].faces[0],
    };
    e.dispatch(Command::Preview(p)).unwrap();
    assert_eq!(e.scene, scene);
    assert_eq!(persistence::encode(&e.scene).unwrap(), encoded);
    assert!(e.undo == history);
    assert!(e.identities.publication > ticket.publication);
    assert_ne!(e.identities.current.meshes[&1].scope, h.scope);
    assert!(e.identities.next_id().unwrap() > before_mark);
    assert!(e.identities.current.meshes[&1].face_index(h).is_err());
    let before = e.clone();
    assert!(e.dispatch_with_ticket(ticket, Command::Drop).is_err());
    assert!(e == before);
    assert_eq!(original.meshes[&1].faces[0], h.face);
}

#[test]
fn c09_counter_exhaustion_is_atomic_with_declared_private_reservations() {
    for publication_limit in [true, false] {
        for allocate in [false, true] {
            let mut e = editor(old("cube-source"));
            if allocate {
                e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
                e.dispatch(Command::ConfigureExtrude(params(0.5))).unwrap();
            }
            if publication_limit {
                e.identities.publication = u64::MAX;
            } else {
                e.identities.request = u64::MAX;
            }
            let before = e.clone();
            let start = e.identities.next_id().unwrap();
            assert_eq!(start, 51);
            let command = if allocate {
                Command::Apply
            } else {
                Command::SelectPolygonSet("bottom".into())
            };
            let error = e.dispatch(command).unwrap_err();
            assert!(
                error.contains(if publication_limit {
                    "Publication budget exhausted"
                } else {
                    "Request budget exhausted"
                }),
                "{error}"
            );
            assert!(e == before);
            assert_eq!(
                e.identities.next_id().unwrap(),
                if allocate { 83 } else { 51 }
            );
            if !publication_limit {
                let before = e.clone();
                assert!(
                    e.identity_ticket()
                        .unwrap_err()
                        .contains("Request budget exhausted")
                );
                assert!(e == before);
            }
        }
    }
}

#[test]
fn c10_delayed_tickets_after_app_equivalent_new_and_open_refuse() {
    let mut origin = editor(old("cube-source"));
    let old_ticket = origin.identity_ticket().unwrap();
    let ids = &origin.identities.current.meshes[&1];
    let old_handle = FaceHandle {
        scope: ids.scope,
        face: ids.faces[1],
    };
    // Same portable controller construction used by the app's New/Open routes.
    let opened = persistence::decode(&bytes("i1/cube-source.opscene")).unwrap();
    for (name, mut current) in [
        ("New", Editor::default()),
        (
            "Open",
            Editor {
                scene: opened,
                ..Default::default()
            },
        ),
    ] {
        let fresh = current.identity_ticket().unwrap();
        assert_eq!(fresh.process, old_ticket.process);
        assert_ne!(fresh.epoch, old_ticket.epoch);
        let before = current.clone();
        let mark = current.identities.next_id().unwrap();
        assert!(
            current
                .dispatch_faces(old_ticket, &[old_handle], Command::Preview(params(0.5)))
                .unwrap_err()
                .contains("Stale")
        );
        assert!(current == before, "{name}:late preview published");
        assert_eq!(current.identities.next_id().unwrap(), mark);
        assert!(
            current
                .dispatch_with_ticket(old_ticket, Command::Apply)
                .is_err()
        );
        assert!(current == before);
        if name == "Open" {
            let new = &current.identities.current.meshes[&1];
            assert_eq!(new.faces[1], old_handle.face);
            assert!(new.face_index(old_handle).is_err());
            assert_eq!(current.scene, old("cube-source"));
        }
    }
}
