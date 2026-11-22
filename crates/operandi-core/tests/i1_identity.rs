use glam::DVec3;
use operandi_core::*;
use serde_json::{Value, json};
fn file(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/i1")
            .join(name),
    )
    .unwrap()
}
fn source(name: &str) -> Scene {
    persistence::decode(&file(&format!("{name}-source.opscene"))).unwrap()
}
fn expected(name: &str) -> Scene {
    serde_json::from_slice(&file(&format!("{name}.expected.json"))).unwrap()
}
fn ids_json(ids: &MeshIdentity, next: u64) -> Value {
    json!({"vertices":ids.vertices,"faces":ids.faces,"corners":ids.corners,"edges":ids.edges.iter().map(|((a,b),id)|json!({"vertices":[a,b],"id":id})).collect::<Vec<_>>(),"logical_faces":ids.logical_faces,"next_id":next})
}
fn parameters(z: f64) -> Parameters {
    Parameters {
        extent: DVec3::new(0., 0., z),
        extrude_mode: ExtrudeMode::Shift,
        extrude_sides: 1,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }
}
fn armed(name: &str) -> Editor {
    let mut e = Editor {
        scene: source(name),
        ..Default::default()
    };
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e
}
#[test]
fn literal_decoded_sources_and_full_zero_moved_graphs() {
    for name in ["cube", "pair"] {
        let s = source(name);
        assert_eq!(s, expected(&format!("{name}-source")));
        let mut next = 1;
        let ids = MeshIdentity::bootstrap(
            &s.items[0].mesh,
            IdentityScope {
                process: std::process::id(),
                epoch: 1,
                item: 1,
                generation: 1,
            },
            &mut next,
        )
        .unwrap();
        let oracle: Value = serde_json::from_slice(&file(&format!("{name}-ids.json"))).unwrap();
        assert_eq!(ids_json(&ids, next), oracle);
        for (suffix, z) in [("zero", 0.), ("moved", 0.5)] {
            let out = extrude_geometry(
                &s.items[0].mesh,
                &s.selection.faces,
                DVec3::new(0., 0., z),
                1,
                ExtrudeMode::Shift,
                ExtrudeUvs::None,
                None,
            )
            .unwrap();
            out.correspondence
                .validate(&s.items[0].mesh, &out.mesh)
                .unwrap();
            let mut got = s.clone();
            got.items[0].mesh = out.mesh.clone().into();
            got.selection.faces = out.caps.clone();
            assert_eq!(
                got,
                expected(&format!("{name}-{suffix}")),
                "{name}-{suffix} full authored scene"
            );
            let mut n = next;
            let mapped = ids
                .transported(&s.items[0].mesh, &out.mesh, &out.correspondence, &mut n)
                .unwrap();
            let oracle: Value =
                serde_json::from_slice(&file(&format!("{name}-{suffix}-ids.json"))).unwrap();
            assert_eq!(ids_json(&mapped, n), oracle);
        }
    }
}
#[test]
fn sparse_corner_holes_and_empty_definitions_are_not_assigned_zero() {
    let s = source("pair-sparse");
    let out = extrude_geometry(
        &s.items[0].mesh,
        &[0],
        DVec3::Z * 0.5,
        1,
        ExtrudeMode::Shift,
        ExtrudeUvs::None,
        None,
    )
    .unwrap();
    let mut got = s;
    got.items[0].mesh = out.mesh.into();
    assert_eq!(got, expected("pair-sparse-moved"));
    assert_eq!(got.items[0].mesh.uv_maps["Texture"][0][3], None);
    assert_eq!(
        got.items[0].mesh.uv_maps["Texture"][0][0],
        Some(glam::DVec2::ZERO)
    );
}
#[test]
fn editor_numeric_apply_drop_undo_redo_restores_ids_and_authored_data() {
    for name in ["cube", "pair"] {
        let mut e = armed(name);
        let initial = e.identities.current.clone();
        let next = e.identities.next_id().unwrap();
        e.dispatch(Command::ConfigureExtrude(parameters(0.5)))
            .unwrap();
        assert_eq!(e.scene, source(name));
        assert!(e.undo.is_empty());
        e.dispatch(Command::Apply).unwrap();
        assert_eq!(e.scene, expected(&format!("{name}-moved")));
        let moved = e.identities.current.clone();
        let high = e.identities.next_id().unwrap();
        assert!(high > next);
        e.dispatch(Command::Drop).unwrap();
        let token = e.identities.publication;
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, source(name));
        assert_eq!(e.identities.current, initial);
        assert!(e.identities.publication > token);
        assert_eq!(e.identities.next_id().unwrap(), high);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, expected(&format!("{name}-moved")));
        assert_eq!(e.identities.current, moved);
        assert_eq!(e.identities.next_id().unwrap(), high);
    }
}
#[test]
fn known_cap_uv_ids_survive_and_owner_membership_is_separate() {
    let mut e = armed("cube");
    e.dispatch(Command::ConfigureExtrude(parameters(0.5)))
        .unwrap();
    e.dispatch(Command::Apply).unwrap();
    let m = &e.scene.items[0].mesh;
    assert_eq!(
        m.attributes.uv_records["Texture"][1],
        vec![Some(100), Some(101), Some(102), Some(103)]
    );
    assert_eq!(
        m.attributes.polygon_sets["top"],
        std::collections::BTreeSet::from([1])
    );
    assert_eq!(
        m.attributes.polygon_sets["front"],
        std::collections::BTreeSet::from([2, 6])
    );
}
#[test]
fn known_selected_owner_assignment_split_is_deferred_without_stripping() {
    let s = persistence::decode(&file("pair-known-split-deferred.opscene")).unwrap();
    let before = s.clone();
    let err = extrude_geometry(
        &s.items[0].mesh,
        &[0],
        DVec3::Z * 0.5,
        1,
        ExtrudeMode::Shift,
        ExtrudeUvs::None,
        None,
    )
    .unwrap_err();
    assert!(err.contains("I1-D01"));
    assert_eq!(s, before);
}
#[test]
fn explicit_face_permutation_deletion_recall_and_retirement() {
    let s = source("cube");
    let target = expected("cube-delete-bottom");
    let mut n = 1;
    let ids = MeshIdentity::bootstrap(
        &s.items[0].mesh,
        IdentityScope {
            process: std::process::id(),
            epoch: 8,
            item: 1,
            generation: 1,
        },
        &mut n,
    )
    .unwrap();
    let top = FaceHandle {
        scope: ids.scope,
        face: ids.faces[1],
    };
    let bottom = FaceHandle {
        scope: ids.scope,
        face: ids.faces[0],
    };
    let remap = ids
        .remap_faces(&s.items[0].mesh, &target.items[0].mesh, &[5, 4, 3, 2, 1])
        .unwrap();
    assert_eq!(remap.face_index(top).unwrap(), 4);
    assert!(remap.face_index(bottom).is_err());
    assert_eq!(remap.logical_faces, ids.faces[1..]);
    assert_eq!(
        target.items[0].mesh.attributes.polygon_sets["top"],
        std::collections::BTreeSet::from([4])
    );
    assert!(target.items[0].mesh.attributes.polygon_sets["bottom"].is_empty());
    let mut wrong = top;
    wrong.scope.epoch += 1;
    assert!(remap.face_index(wrong).is_err());
    let mut bad = target.items[0].mesh.clone();
    bad.vertices[0].x += 1.;
    assert!(
        ids.remap_faces(&s.items[0].mesh, &bad, &[5, 4, 3, 2, 1])
            .is_err()
    );
}
#[test]
fn named_set_recall_is_ordered_and_selection_history_uses_existing_policy() {
    let mut e = armed("pair");
    e.dispatch(Command::ConfigureExtrude(parameters(0.5)))
        .unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let geometry = e.scene.items[0].mesh.clone();
    let ids = e.identities.current.clone();
    e.dispatch(Command::SelectPolygonSet("r".into())).unwrap();
    assert_eq!(e.scene.selection.faces, vec![1, 3]);
    assert_eq!(e.scene.items[0].mesh, geometry);
    assert_eq!(e.identities.current, ids);
    let records = e.undo.len();
    e.dispatch(Command::SelectPolygonSet("r".into())).unwrap();
    assert_eq!(e.undo.len(), records);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection.faces, vec![0]);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.selection.faces, vec![1, 3]);
}
#[test]
fn branch_reservations_stale_requests_and_cross_document_handles() {
    let mut e = armed("cube");
    let old = e.identity_ticket().unwrap();
    e.dispatch(Command::ConfigureExtrude(parameters(0.5)))
        .unwrap();
    e.dispatch(Command::Apply).unwrap();
    let first = e.identities.current.clone();
    let high = e.identities.next_id().unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Undo).unwrap();
    let before = e.scene.clone();
    assert!(e.dispatch_with_ticket(old, Command::Redo).is_err());
    assert_eq!(e.scene, before);
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(parameters(0.25)))
        .unwrap();
    e.dispatch(Command::Apply).unwrap();
    assert!(e.redo.is_empty());
    assert!(e.identities.current.meshes[&1].vertices[8].0 >= high);
    assert_ne!(
        e.identities.current.meshes[&1].vertices[8],
        first.meshes[&1].vertices[8]
    );
    let new_ticket = e.identity_ticket().unwrap();
    let newer = e.identity_ticket().unwrap();
    assert!(e.dispatch_with_ticket(new_ticket, Command::Drop).is_err());
    e.dispatch_with_ticket(newer, Command::Drop).unwrap();
    let old_ids = e.identities.current.meshes[&1].clone();
    let bytes = persistence::encode(&e.scene).unwrap();
    let mut opened = Editor {
        scene: persistence::decode(&bytes).unwrap(),
        ..Default::default()
    };
    opened.synchronize_identities().unwrap();
    let face = FaceHandle {
        scope: old_ids.scope,
        face: old_ids.faces[1],
    };
    assert!(
        opened.identities.current.meshes[&1]
            .face_index(face)
            .is_err()
    );
    assert_eq!(opened.scene, e.scene);
}
#[test]
fn unsupported_new_set_profiles_and_bad_numbers_refuse_atomically() {
    for p in [
        Parameters {
            extrude_mode: ExtrudeMode::Thick,
            ..parameters(0.5)
        },
        Parameters {
            extrude_mode: ExtrudeMode::Auto,
            ..parameters(0.5)
        },
        Parameters {
            extrude_sides: 2,
            ..parameters(0.5)
        },
        Parameters {
            extrude_uvs: ExtrudeUvs::U,
            ..parameters(0.5)
        },
        parameters(f64::NAN),
        parameters(f64::INFINITY),
    ] {
        let mut e = armed("cube");
        e.dispatch(Command::ConfigureExtrude(p)).ok();
        let before = e.scene.clone();
        let ids = e.identities.current.clone();
        let history = e.undo.clone();
        let token = e.identities.publication;
        assert!(e.dispatch(Command::Apply).is_err());
        assert_eq!(e.scene, before);
        assert_eq!(e.identities.current, ids);
        assert!(e.undo == history);
        assert_eq!(e.identities.publication, token);
    }
    let mut e = armed("cube");
    e.dispatch(Command::ConfigureExtrude(parameters(0.5)))
        .unwrap();
    e.identities.reserve_through(u64::MAX).unwrap();
    let before = e.scene.clone();
    assert!(e.dispatch(Command::Apply).is_err());
    assert_eq!(e.scene, before);
    assert_eq!(e.identities.next_id().unwrap(), u64::MAX);
}
#[test]
fn full_native_roundtrip_preserves_authored_data_without_serializing_sidecars() {
    let mut e = armed("cube");
    e.dispatch(Command::ConfigureExtrude(parameters(0.5)))
        .unwrap();
    e.dispatch(Command::Apply).unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    let d: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(d["version"], 7);
    assert!(d["scene"].get("identities").is_none());
    assert_eq!(persistence::decode(&bytes).unwrap(), expected("cube-moved"));
    for text in [r#"{"format":"Operandi","version":9,"scene":{}}"#, r#"{}"#] {
        assert!(persistence::decode(text.as_bytes()).is_err());
    }
}
#[test]
fn malformed_correspondence_and_identity_coverage_refuse() {
    let s = source("cube");
    let out = extrude_geometry(
        &s.items[0].mesh,
        &[1],
        DVec3::Z * 0.5,
        1,
        ExtrudeMode::Shift,
        ExtrudeUvs::None,
        None,
    )
    .unwrap();
    let mut bad = out.correspondence.clone();
    bad.vertex_sources.pop();
    assert!(bad.validate(&s.items[0].mesh, &out.mesh).is_err());
    let mut bad = out.correspondence.clone();
    bad.walls[0].owner = 0;
    assert!(bad.validate(&s.items[0].mesh, &out.mesh).is_err());
    let mut n = 1;
    let mut ids = MeshIdentity::bootstrap(
        &s.items[0].mesh,
        IdentityScope {
            process: std::process::id(),
            epoch: 1,
            item: 1,
            generation: 1,
        },
        &mut n,
    )
    .unwrap();
    ids.faces[1] = ids.faces[0];
    assert!(ids.validate(&s.items[0].mesh).is_err());
}

#[test]
fn admitted_unchanged_production_decoder_gate_and_invalid_variants() {
    let bytes = file("p05-legacy8.opscene");
    let decoded = persistence::decode(&bytes).unwrap();
    let fixed: Scene = serde_json::from_slice(&file("p05-legacy-decoded.json")).unwrap();
    assert_eq!(decoded, fixed);
    assert!(persistence::decode(&file("p05-legacy-valid-outside-profile.opscene")).is_ok());
    for name in [
        "field",
        "hierarchy",
        "identity",
        "index",
        "truncated",
        "version",
    ] {
        assert!(
            persistence::decode(&file(&format!("p05-legacy-invalid-{name}.opscene"))).is_err(),
            "{name}"
        );
    }
    assert_eq!(
        persistence::decode(&persistence::encode(&decoded).unwrap()).unwrap(),
        fixed
    );
}
#[test]
fn scoped_target_list_refuses_without_retargeting_or_partial_publication() {
    let mut e = armed("cube");
    e.dispatch(Command::ConfigureExtrude(parameters(0.5)))
        .unwrap();
    let ids = &e.identities.current.meshes[&1];
    let top = FaceHandle {
        scope: ids.scope,
        face: ids.faces[1],
    };
    for faces in [
        vec![],
        vec![top, top],
        vec![FaceHandle {
            scope: IdentityScope {
                epoch: top.scope.epoch + 1,
                ..top.scope
            },
            ..top
        }],
        vec![FaceHandle {
            face: FaceId(u64::MAX),
            ..top
        }],
    ] {
        let ticket = e.identity_ticket().unwrap();
        let scene = e.scene.clone();
        let ids = e.identities.current.clone();
        let pubid = e.identities.publication;
        assert!(e.dispatch_faces(ticket, &faces, Command::Apply).is_err());
        assert_eq!(e.scene, scene);
        assert_eq!(e.identities.current, ids);
        assert_eq!(e.identities.publication, pubid);
        assert!(e.undo.is_empty());
    }
    let ticket = e.identity_ticket().unwrap();
    e.dispatch_faces(ticket, &[top], Command::Apply).unwrap();
    assert_eq!(e.scene, expected("cube-moved"));
}
#[test]
fn unrelated_item_deletion_and_undo_restore_captured_sidecars() {
    let mut e = Editor {
        scene: source("cube"),
        ..Default::default()
    };
    e.synchronize_identities().unwrap();
    let initial = e.identities.current.clone();
    let high = e.identities.next_id().unwrap();
    e.dispatch(Command::DeleteItem).unwrap();
    assert!(e.identities.current.meshes.is_empty());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.identities.current, initial);
    assert_eq!(e.identities.next_id().unwrap(), high);
    assert_eq!(e.scene, source("cube"));
    e.dispatch(Command::Redo).unwrap();
    assert!(e.scene.items.is_empty());
}
#[test]
fn immutable_read_view_keeps_distinct_coincident_and_unused_points() {
    let face = |vertices: Vec<u32>| operandi_core::mesh::Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    };
    let m = Mesh {
        vertices: vec![
            DVec3::ZERO,
            DVec3::X,
            DVec3::new(1., 1., 0.),
            DVec3::Y,
            DVec3::ZERO,
            DVec3::X,
            DVec3::new(1., 1., 0.),
            DVec3::Y,
            DVec3::ZERO,
        ],
        faces: vec![face(vec![0, 1, 2, 3]), face(vec![4, 5, 6, 7])].into(),
        ..Default::default()
    };
    let mut n = 1;
    let ids = MeshIdentity::bootstrap(
        &m,
        IdentityScope {
            process: std::process::id(),
            epoch: 1,
            item: 1,
            generation: 1,
        },
        &mut n,
    )
    .unwrap();
    assert_eq!(ids.vertices, (1..=9).map(VertexId).collect::<Vec<_>>());
    assert_ne!(ids.vertices[0], ids.vertices[4]);
    assert_ne!(ids.vertices[0], ids.vertices[8]);
    assert_eq!(ids.edges.len(), 8);
    assert_eq!(n, 28);
    ids.validate(&m).unwrap();
    let mut e = Editor {
        scene: source("pair-sparse"),
        ..Default::default()
    };
    let view = e.read_mesh(1).unwrap();
    let h = FaceHandle {
        scope: view.identities.scope,
        face: view.identities.faces[0],
    };
    assert_eq!(view.authored_uv("Texture", h, 3).unwrap(), None);
    assert_eq!(
        view.authored_uv("Texture", h, 0).unwrap(),
        Some(glam::DVec2::ZERO)
    );
}
#[test]
fn failed_save_preserves_live_stage_history_and_current_identities() {
    let mut e = armed("cube");
    e.dispatch(Command::ConfigureExtrude(parameters(0.5)))
        .unwrap();
    e.dispatch(Command::Apply).unwrap();
    let before = e.clone();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/i1/local/nonexistent-parent/save.opscene");
    assert!(e.save_committed(&path).is_err());
    assert!(e == before);
    e.dispatch(Command::Drop).unwrap();
    let token = e.identity_ticket().unwrap();
    e.scene.selection.faces = vec![0];
    assert!(
        e.dispatch_with_ticket(token, Command::SelectPolygonSet("top".into()))
            .is_err()
    );
}

#[test]
fn allocation_owner_is_shared_by_staging_clones_but_not_other_documents() {
    let mut e = armed("cube");
    let held = e.clone();
    let old_ids = e.identities.current.clone();
    assert_eq!(e.identities.next_id().unwrap(), 51);
    e.identities.reserve_through(100).unwrap();
    assert_eq!(held.identities.next_id().unwrap(), 100);
    assert!(held.identities.reserve_through(99).is_err());
    assert_eq!(e.identities.current, old_ids);
    let mut other = Editor {
        scene: source("cube"),
        ..Default::default()
    };
    other.synchronize_identities().unwrap();
    assert_eq!(other.identities.next_id().unwrap(), 51);
    assert_ne!(other.identities.epoch, e.identities.epoch);
    e.dispatch(Command::ConfigureExtrude(parameters(0.5)))
        .unwrap();
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(e.identities.current.meshes[&1].vertices[8], VertexId(100));
    assert_eq!(held.scene, source("cube"));
    assert_eq!(held.identities.current, old_ids);
}
#[test]
fn failed_private_save_burns_ids_without_mutating_frozen_authored_snapshots() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::ArmEllipsoid).unwrap();
    e.dispatch(Command::ConfigureEllipsoid(EllipsoidParameters::default()))
        .unwrap();
    let held = e.clone();
    let before = e.identities.next_id().unwrap();
    let ids = e.identities.current.clone();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/i1/local/nonexistent-parent/save-bank.opscene");
    assert!(e.save_committed(&path).is_err());
    let burned = e.identities.next_id().unwrap();
    assert!(burned > before);
    assert_eq!(held.identities.next_id().unwrap(), burned);
    assert_eq!(e.scene, held.scene);
    assert_eq!(e.identities.current, ids);
    assert_eq!(e.identities.publication, held.identities.publication);
    assert!(e.undo == held.undo);
    e.dispatch(Command::Apply).unwrap();
    let item = e.scene.selection.item.unwrap();
    assert!(e.identities.current.meshes[&item].vertices[0].0 >= burned);
    assert!(held.scene.items[0].mesh.vertices.is_empty());
}
