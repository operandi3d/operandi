//! Original literal wire witnesses: expectations are declared before encode.
//! These are semantic JSON contracts, not whitespace/object-key byte goldens.
use operandi_core::{Mode, Scene, persistence};
use serde_json::{Value, json};

// Deliberately omit defaultable additions: zxy; item parent/source metadata;
// mesh attributes/uv_maps; face part/material_label; scene import_source.
// Versions 1..3 have no invented historical whitelist: this tests these exact
// omissions in the currently supported reader, not every historical document.
const LEGACY_SCENE: &str = r#"{
  "items":[{"id":4294967301,"name":"Wire witness","visible":true,
    "transform":{"translation":[1.0,2.0,3.0],"rotation":[0.0,0.0,0.0],"scale":[1.0,1.0,1.0]},
    "mesh":{"vertices":[[0.0,0.0,0.0],[1.0,0.0,0.0],[0.0,1.0,0.0]],
      "faces":[{"vertices":[0,1,2],"uv":null,"material":"Mat","smooth":false}]}}],
  "next_id":4294967302,
  "selection":{"item":4294967301,"mode":"Edge","vertices":[2,0],"edges":[[1,2],[0,1]],"faces":[0]},
  "workplane":{"fixed":true,"plane":2,"center":[4.0,5.0,6.0]}
}"#;

fn literal(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn document(version: u32) -> Value {
    json!({"format":"Operandi", "version":version, "scene":literal(LEGACY_SCENE)})
}

fn decode(document: &Value) -> Scene {
    // Only serializes hand-authored JSON values, never a current Scene DTO.
    persistence::decode(&serde_json::to_vec(document).unwrap()).unwrap()
}

fn attributes(document: &mut Value) -> &mut Value {
    &mut document["scene"]["items"][0]["mesh"]["attributes"]
}

fn canonical_expectation(input: &Value, version: u32) -> Value {
    // Explicit omissions/defaults oracle, unrelated to feature-floor selection.
    // Every caller supplies its expected version as an independent literal.
    let mut expected = input.clone();
    expected["version"] = json!(version);
    expected["scene"]["import_source"] = input["scene"]
        .get("import_source")
        .cloned()
        .unwrap_or(Value::Null);
    let item = &mut expected["scene"]["items"][0];
    item["parent"] = Value::Null;
    item["source_id"] = Value::Null;
    item["source_display_name"] = Value::Null;
    item["transform"]["zxy"] = json!(false);
    item["mesh"]["uv_maps"] = input["scene"]["items"][0]["mesh"]
        .get("uv_maps")
        .cloned()
        .unwrap_or(json!({}));
    for face in item["mesh"]["faces"].as_array_mut().unwrap() {
        face["part"] = Value::Null;
        face["material_label"] = Value::Null;
    }
    let defaults = literal(
        r#"{"representations":[],"creases":[],"uv_records":{},"corner_normals":{},"point_weights":{},"point_colors":{},"degenerate_faces":[]}"#,
    );
    let mut authored = item["mesh"].get("attributes").cloned().unwrap_or(json!({}));
    for (key, value) in defaults.as_object().unwrap() {
        authored
            .as_object_mut()
            .unwrap()
            .entry(key)
            .or_insert(value.clone());
    }
    item["mesh"]["attributes"] = authored;
    expected
}

fn check_writer(input: &Value, expected: &Value) {
    let scene = decode(input);
    let actual: Value = serde_json::from_slice(&persistence::encode(&scene).unwrap()).unwrap();
    assert_eq!(actual, *expected);
}

#[test]
fn reader_one_through_four_legacy_omissions_and_vector_enum_order() {
    for version in [1, 2, 3, 4] {
        let input = document(version);
        let expected = canonical_expectation(&input, 4);
        let scene = decode(&input);
        assert_eq!(scene.items[0].id, 4_294_967_301);
        assert_eq!(scene.selection.mode, Mode::Edge);
        assert_eq!(scene.selection.vertices, [2, 0]);
        assert_eq!(scene.selection.edges, [(1, 2), (0, 1)]);
        assert_eq!(scene.selection.faces, [0]);
        assert!(!scene.items[0].transform.zxy);
        assert!(scene.items[0].parent.is_none());
        assert!(scene.items[0].source_id.is_none());
        assert!(scene.items[0].source_display_name.is_none());
        assert!(scene.items[0].mesh.faces[0].part.is_none());
        assert!(scene.items[0].mesh.faces[0].material_label.is_none());
        assert!(scene.import_source.is_none());
        check_writer(&input, &expected);
    }
    for (spelling, mode) in [
        ("Vertex", Mode::Vertex),
        ("Edge", Mode::Edge),
        ("Polygon", Mode::Polygon),
        ("Item", Mode::Item),
    ] {
        let mut input = document(4);
        input["scene"]["selection"]["mode"] = json!(spelling);
        let expected = canonical_expectation(&input, 4);
        assert_eq!(decode(&input).selection.mode, mode);
        check_writer(&input, &expected);
    }
}

#[test]
fn independent_feature_witnesses_pin_writer_floors_and_reader_refusals() {
    // Each floor/fragment is declared here, never inferred by a codec-like helper.
    let cases = [
        (
            5,
            r#"{"relative_morphs":{"Delta":{"0":[0.0,0.0,0.0]}}}"#,
            "Relative morphs",
        ),
        (5, r#"{"smoothing_labels":["Group"]}"#, "Relative morphs"),
        (
            6,
            r#"{"point_uvs":{"Chart":{"0":[0.0,0.0]}}}"#,
            "Extrude point/corner",
        ),
        (
            6,
            r#"{"corner_weights":{"Weight":[[null,0.0,1.0]]}}"#,
            "Extrude point/corner",
        ),
        (
            6,
            r#"{"corner_colors":{"Color":[[null,[0.0,0.0,0.0,0.0],[1.0,0.5,0.25,1.0]]]}}"#,
            "Extrude point/corner",
        ),
        (
            7,
            r#"{"typed_maps":{"Empty typed":{"kind":"vector3","dimension":3,"zero_default":true,"points":{},"corners":[]}}}"#,
            "Typed maps",
        ),
        (7, r#"{"vertex_sets":{"Empty vertices":[]}}"#, "Typed maps"),
        (7, r#"{"polygon_sets":{"Empty polygons":[]}}"#, "Typed maps"),
        (
            8,
            r#"{"corner_normals":{"Zeta":[[null,[0.0,0.0,1.0],null]],"Alpha":[[[0.0,0.0,1.0],null,null]]},"normal_map_order":["Zeta","Alpha"]}"#,
            "Normal-map creation order",
        ),
    ];
    for (floor, fragment, diagnostic) in cases {
        let mut input = document(floor);
        *attributes(&mut input) = literal(fragment);
        if fragment.contains("point_uvs") {
            input["scene"]["items"][0]["mesh"]["uv_maps"] =
                literal(r#"{"Chart":[[[0.0,0.0],null,null]]}"#);
        }
        let expected = canonical_expectation(&input, floor);
        check_writer(&input, &expected);
        // All supported lower profiles must reject this literal feature input.
        for lower in 1..floor {
            let mut below_floor = input.clone();
            below_floor["version"] = json!(lower);
            let error =
                persistence::decode(&serde_json::to_vec(&below_floor).unwrap()).unwrap_err();
            assert!(
                error.contains(diagnostic),
                "floor {floor}, version {lower}: {error}"
            );
        }
    }
}

#[test]
fn exceptional_incidence_triples_are_version_six_wire_data() {
    let mut input = document(6);
    // Two original same-winding triangles: the three directed incidences are
    // authored literal data, not generated by an Extrude operation/serializer.
    input["scene"]["items"][0]["mesh"]["faces"] = literal(
        r#"[
      {"vertices":[0,1,2],"uv":null,"material":"","smooth":false},
      {"vertices":[0,1,2],"uv":null,"material":"","smooth":false}]"#,
    );
    *attributes(&mut input) = literal(
        r#"{"extrude_edge_uses":{
      "0:1":[[0,0,1],[1,0,1]],"0:2":[[0,2,0],[1,2,0]],"1:2":[[0,1,2],[1,1,2]]}}"#,
    );
    let expected = canonical_expectation(&input, 6);
    check_writer(&input, &expected);
    input["version"] = json!(5);
    let error = persistence::decode(&serde_json::to_vec(&input).unwrap()).unwrap_err();
    assert!(error.contains("requires native version 6"));
}

#[test]
fn empty_skipped_fields_do_not_raise_floor_but_named_empty_definitions_do() {
    let mut input = document(4);
    *attributes(&mut input) = literal(
        r#"{
      "typed_maps":{},"vertex_sets":{},"polygon_sets":{},"point_uvs":{},
      "corner_weights":{},"corner_colors":{},"extrude_edge_uses":{},
      "relative_morphs":{},"smoothing_labels":[],"normal_map_order":[]}"#,
    );
    // Independently expect all these empty top-level fields to be omitted.
    let expected = canonical_expectation(&document(4), 4);
    check_writer(&input, &expected);
    // Named empty typed/set definitions are covered separately in the floor7
    // cases above; they are retained and raise the floor even with no records.
}

#[test]
fn mixed_features_keep_highest_profile_and_sparse_assignment_identity() {
    let mut input = document(8);
    input["scene"]["import_source"] = literal(
        r#"{
      "path":"/original-wire-witness/source.lxo","producer":"original fixture",
      "device":4294967303,"inode":4294967304}"#,
    );
    input["scene"]["items"][0]["mesh"]["uv_maps"] =
        literal(r#"{"Unconventional chart":[[[0.0,0.0],null,[0.0,0.0]]]}"#);
    *attributes(&mut input) = literal(
        r#"{
      "relative_morphs":{"Delta":{"0":[0.0,0.0,0.0]}},
      "corner_weights":{"Weight":[[null,0.0,1.0]]},
      "typed_maps":{"Typed":{"kind":"weight1","dimension":1,"zero_default":false,"points":{"0":[0.0]},"corners":[[null,[0.0],null]]}},
      "vertex_sets":{"Ends":[0,2]},"polygon_sets":{"Face":[0]},
      "uv_records":{"Unconventional chart":[[701,null,702]]},
      "corner_normals":{"Second":[[null,[0.0,0.0,1.0],null]],"First":[[[0.0,0.0,1.0],null,null]]},
      "normal_map_order":["Second","First"]}"#,
    );
    let expected = canonical_expectation(&input, 8);
    check_writer(&input, &expected);
    // A nonempty typed definition wins over floors5/6 when floor8 is absent.
    attributes(&mut input)
        .as_object_mut()
        .unwrap()
        .remove("normal_map_order");
    let expected7 = canonical_expectation(&input, 7);
    check_writer(&input, &expected7);
    for key in ["typed_maps", "vertex_sets", "polygon_sets"] {
        attributes(&mut input).as_object_mut().unwrap().remove(key);
    }
    let expected6 = canonical_expectation(&input, 6);
    check_writer(&input, &expected6);
    attributes(&mut input)
        .as_object_mut()
        .unwrap()
        .remove("corner_weights");
    let expected5 = canonical_expectation(&input, 5);
    check_writer(&input, &expected5);
}

#[test]
fn literal_seven_eight_fixtures_remain_readable_without_changing_them() {
    // Reuse the existing admitted literals; their complete independent semantic
    // oracle/negative variants remain owned by i1_identity's existing gate.
    for (bytes, version, ids) in [
        (
            include_bytes!("fixtures/i1/cube-source.opscene").as_slice(),
            7,
            vec![1],
        ),
        (
            include_bytes!("fixtures/i1/pair-source.opscene").as_slice(),
            7,
            vec![1],
        ),
        (
            include_bytes!("fixtures/i1/p05-legacy8.opscene").as_slice(),
            8,
            vec![7, 9],
        ),
    ] {
        let wire: Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(wire["version"], version);
        let scene = persistence::decode(bytes).unwrap();
        assert_eq!(
            scene.items.iter().map(|item| item.id).collect::<Vec<_>>(),
            ids
        );
    }
}

fn absent_runtime_fields(value: &Value) {
    const RUNTIME: &[&str] = &[
        "identities",
        "scope",
        "epoch",
        "generation",
        "publication",
        "request",
        "request_id",
        "history",
        "undo",
        "redo",
        "live",
        "live_extrude",
        "allocation",
        "allocation_bank",
        "next_runtime_id",
        "memo",
        "self_checked",
        "partner_checked",
        "unconfirmed_text",
        "gpu_handles",
    ];
    // Inspect schema records only. Keys inside named-map/set dictionaries are
    // user-authored names, not fields, and may legitimately equal this list.
    let scene = &value["scene"];
    let mut records = vec![value, scene, &scene["selection"], &scene["workplane"]];
    if scene["import_source"].is_object() {
        records.push(&scene["import_source"]);
    }
    for item in scene["items"].as_array().unwrap() {
        let mesh = &item["mesh"];
        let attrs = &mesh["attributes"];
        records.extend([item, &item["transform"], mesh, attrs]);
        records.extend(mesh["faces"].as_array().unwrap());
        if let Some(maps) = attrs.get("typed_maps") {
            records.extend(maps.as_object().unwrap().values());
        }
    }
    for record in records {
        for key in record.as_object().unwrap().keys() {
            assert!(
                !RUNTIME.contains(&key.as_str()),
                "runtime schema field: {key}"
            );
        }
    }
}

#[test]
fn runtime_sidecars_are_absent_while_persistent_ids_remain() {
    // Existing Shared tests establish copy/encode transparency; use its literal
    // v8 graph here to pin named exclusions without conflating persisted IDs.
    let scene = persistence::decode(include_bytes!("fixtures/i1/p05-legacy8.opscene")).unwrap();
    let expected_item_id = json!(7);
    let expected_uv_ids = json!([[901, 902, 903, null], [904, null, null, 903]]);
    let wire: Value = serde_json::from_slice(&persistence::encode(&scene).unwrap()).unwrap();
    absent_runtime_fields(&wire);
    assert_eq!(wire["scene"]["items"][0]["id"], expected_item_id);
    assert_eq!(
        wire["scene"]["items"][0]["mesh"]["attributes"]["uv_records"]["Main"],
        expected_uv_ids
    );

    // Names matching runtime fields remain legal authored dictionary keys.
    let mut named = document(7);
    *attributes(&mut named) = literal(
        r#"{"typed_maps":{"history":{"kind":"vector3","dimension":3,"zero_default":false,"points":{},"corners":[]}},
          "vertex_sets":{"scope":[]},"polygon_sets":{"memo":[]},
          "corner_weights":{"live":[[null,0.0,1.0]]}}"#,
    );
    let expected = canonical_expectation(&named, 7);
    check_writer(&named, &expected);
    let scene = decode(&named);
    let wire: Value = serde_json::from_slice(&persistence::encode(&scene).unwrap()).unwrap();
    absent_runtime_fields(&wire);
}

#[test]
fn additional_literal_shape_and_integer_boundaries_refuse() {
    // Existing I1 literal variants already cover unknown fields, truncation,
    // unsupported versions, hierarchy/identity and invalid vertex references.
    // Add only complementary scalar/vector/range cases to this compact matrix.
    for (pointer, value) in [
        ("/scene/items/0/transform/translation", json!([1.0, 2.0])),
        ("/scene/items/0/mesh/vertices/0", json!(0)),
        ("/scene/selection/mode", json!("polygon")),
        ("/scene/workplane/plane", json!(256)),
        ("/scene/selection/vertices/0", json!(4294967296_u64)),
        ("/scene/items/0/id", json!(-1)),
    ] {
        let mut input = document(4);
        *input.pointer_mut(pointer).unwrap() = value;
        assert!(
            persistence::decode(&serde_json::to_vec(&input).unwrap()).is_err(),
            "{pointer}"
        );
    }
    let mut input = document(4);
    input["version"] = json!(4294967296_u64);
    assert!(persistence::decode(&serde_json::to_vec(&input).unwrap()).is_err());
}
