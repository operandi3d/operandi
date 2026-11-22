use super::*;

pub(crate) const NOISE_RAMP: &str = include_str!("../../../fixtures/node-graphs/noise_ramp.json");

fn graph() -> StoredGraph {
    import(NOISE_RAMP, None).unwrap().remove(0)
}

fn node<'a>(f: &'a FlatGraph, cycles: &str) -> (usize, &'a FlatNode) {
    f.nodes
        .iter()
        .enumerate()
        .find(|(_, n)| n.cycles == cycles)
        .unwrap_or_else(|| panic!("no {cycles} in {f:#?}"))
}

#[test]
fn imports_the_shared_format_and_flattens_to_cycles_nodes() {
    let g = graph();
    assert_eq!(g.material.name, "Noise Ramp");
    let f = g.flatten(&Default::default()).unwrap();
    let kinds: Vec<_> = f.nodes.iter().map(|n| n.cycles).collect();
    assert_eq!(
        kinds,
        [
            "noise_texture",
            "rgb_ramp",
            "mix_color",
            "principled_bsdf",
            "output"
        ]
    );
    let (noise, n) = node(&f, "noise_texture");
    assert!(
        n.values
            .contains(&("Type".into(), FlatValue::Text("fBM".into())))
    );
    assert!(n.values.contains(&("Dimensions".into(), FlatValue::Int(3))));
    assert!(n.values.contains(&("Scale".into(), FlatValue::Float(4.0))));
    let (ramp, r) = node(&f, "rgb_ramp");
    let FlatValue::Ramp(table) = &r.values.iter().find(|(s, _)| s == "Ramp").unwrap().1 else {
        panic!()
    };
    assert_eq!(table.len(), RAMP_TABLE_SIZE);
    // Held at the ends, half way between at 0.5.
    assert_eq!(table[0], [0.05, 0.1, 0.6, 1.0]);
    assert_eq!(table[255], [0.9, 0.6, 0.1, 1.0]);
    let mid = ramp_eval(&ramp_stops(&g.material.nodes[1]).0, "LINEAR", 0.5);
    assert!((mid[0] - 0.475).abs() < 1e-9);
    let (mix, m) = node(&f, "mix_color");
    assert!(
        m.values
            .contains(&("Type".into(), FlatValue::Text("multiply".into())))
    );
    let (bsdf, _) = node(&f, "principled_bsdf");
    let link = |from: usize, fs: &str, to: usize, ts: &str| FlatLink {
        from,
        from_socket: fs.into(),
        to,
        to_socket: ts.into(),
    };
    assert!(f.links.contains(&link(noise, "Fac", ramp, "Fac")));
    assert!(f.links.contains(&link(ramp, "Color", mix, "A")));
    assert!(f.links.contains(&link(mix, "Result", bsdf, "Base Color")));
    assert_eq!(f.feeding("Surface"), Some((bsdf, "BSDF")));
}

#[test]
fn exposed_inputs_take_defaults_values_and_drivers() {
    let g = graph();
    let f = g.flatten(&Default::default()).unwrap();
    let (mix, _) = node(&f, "mix_color");
    assert_eq!(f.value(mix, "B"), Some(&FlatValue::Vec3([1., 1., 1.])));
    let mut ev = ExposedValues::default();
    ev.values.insert("Tint".into(), vec![1., 0., 0.]);
    ev.drivers.insert("Roughness".into(), 7);
    let f = g.flatten(&ev).unwrap();
    assert_eq!(f.value(mix, "B"), Some(&FlatValue::Vec3([1., 0., 0.])));
    let (bsdf, _) = node(&f, "principled_bsdf");
    assert_eq!(f.value(bsdf, "Roughness"), Some(&FlatValue::Driven(7)));
}

#[test]
fn unsupported_nodes_refuse_the_whole_file_naming_the_node() {
    let text = NOISE_RAMP.replace("ShaderNodeTexNoise", "ShaderNodeTexMagic");
    let err = import(&text, None).unwrap_err();
    assert!(err.contains("Noise Texture"), "{err}");
    assert!(err.contains("ShaderNodeTexMagic"), "{err}");
    assert!(err.contains("Nothing was imported"), "{err}");
}

#[test]
fn files_of_another_format_or_newer_version_are_refused() {
    assert!(import("{}", None).is_err());
    let newer = NOISE_RAMP.replace("\"format_version\": 1", "\"format_version\": 2");
    assert!(import(&newer, None).unwrap_err().contains("newer"));
    let other = NOISE_RAMP.replace("operandi-node-graph", "something-else");
    assert!(import(&other, None).is_err());
}

#[test]
fn unknown_fields_and_props_survive_a_round_trip() {
    let text = NOISE_RAMP.replace(
        "\"location\": [-600, 0]",
        "\"location\": [-600, 0], \"colour_tag\": \"red\"",
    );
    let g = import(&text, None).unwrap().remove(0);
    let back = serde_json::to_string(&g).unwrap();
    assert!(back.contains("colour_tag"));
    let again: StoredGraph = serde_json::from_str(&back).unwrap();
    assert_eq!(again, g);
    assert_eq!(g.material.nodes[0].location(), Some([-600., 0.]));
}

#[test]
fn relative_image_paths_become_absolute_on_import() {
    let text = r#"{"format":"operandi-node-graph","format_version":1,"materials":[{
      "name":"Img","nodes":[
        {"name":"Image Texture","type":"ShaderNodeTexImage",
         "props":{"interpolation":"Closest","extension":"EXTEND"},
         "image":{"path":"tex/a.png","colorspace":"Non-Color","packed":false},"inputs":{}},
        {"name":"Material Output","type":"ShaderNodeOutputMaterial","inputs":{}}],
      "links":[{"from":["Image Texture","Color"],"to":["Material Output","Surface"]}]}]}"#;
    let g = import(text, Some(std::path::Path::new("/data/pack")))
        .unwrap()
        .remove(0);
    let img = g.material.nodes[0].image.as_ref().unwrap();
    assert_eq!(
        std::path::Path::new(&img.path),
        std::path::Path::new("/data/pack/tex/a.png")
    );
    let f = g.flatten(&Default::default()).unwrap();
    let (_, n) = node(&f, "image_texture");
    assert!(
        n.values
            .contains(&("Colorspace".into(), FlatValue::Text("data".into())))
    );
    assert!(
        n.values
            .contains(&("Interpolation".into(), FlatValue::Text("closest".into())))
    );
    assert!(
        n.values
            .contains(&("Extension".into(), FlatValue::Text("clamp".into())))
    );
}

#[test]
fn groups_reroutes_and_frames_resolve_away() {
    let text = r#"{"format":"operandi-node-graph","format_version":1,
    "materials":[{"name":"G","nodes":[
        {"name":"Value","type":"ShaderNodeValue","outputs":{"Value":{"type":"VALUE","value":0.25}},"inputs":{}},
        {"name":"Reroute","type":"NodeReroute","inputs":{}},
        {"name":"Frame","type":"NodeFrame","inputs":{}},
        {"name":"Grp","type":"ShaderNodeGroup","props":{"group":"Half"},
         "inputs":{"Amount":{"type":"VALUE","value":0.9}}},
        {"name":"Emission","type":"ShaderNodeEmission","inputs":{"Strength":{"type":"VALUE","value":1.0}}},
        {"name":"Material Output","type":"ShaderNodeOutputMaterial","inputs":{}}],
      "links":[
        {"from":["Value","Value"],"to":["Reroute","Input"]},
        {"from":["Reroute","Output"],"to":["Grp","In"]},
        {"from":["Grp","Out"],"to":["Emission","Color"]},
        {"from":["Emission","Emission"],"to":["Material Output","Surface"]}],
      "groups_used":["Half"]}],
    "groups":[{"name":"Half","nodes":[
        {"name":"Group Input","type":"NodeGroupInput","inputs":{}},
        {"name":"Math","type":"ShaderNodeMath","props":{"operation":"MULTIPLY"},"inputs":{}},
        {"name":"Group Output","type":"NodeGroupOutput","inputs":{}}],
      "links":[
        {"from":["Group Input","In"],"to":["Math","Value"]},
        {"from":["Group Input","Amount"],"to":["Math","Value","Value_001"]},
        {"from":["Math","Value"],"to":["Group Output","Out"]}],
      "inputs":[{"name":"In","type":"VALUE","default":0},{"name":"Amount","type":"VALUE","default":0.5}],
      "outputs":[{"name":"Out","type":"VALUE"}]}]}"#;
    let g = import(text, None).unwrap().remove(0);
    let f = g.flatten(&Default::default()).unwrap();
    let kinds: Vec<_> = f.nodes.iter().map(|n| n.cycles).collect();
    assert_eq!(kinds, ["value", "math", "emission", "output"]);
    let (value, v) = node(&f, "value");
    assert_eq!(
        v.values,
        vec![("Value".to_string(), FlatValue::Float(0.25))]
    );
    let (math, m) = node(&f, "math");
    assert_eq!(m.name, "Grp/Math");
    assert!(
        m.values
            .contains(&("Type".into(), FlatValue::Text("multiply".into())))
    );
    // The group node's own value reaches the second input.
    assert_eq!(f.value(math, "Value2"), Some(&FlatValue::Float(0.9)));
    let (emission, _) = node(&f, "emission");
    assert!(f.links.contains(&FlatLink {
        from: value,
        from_socket: "Value".into(),
        to: math,
        to_socket: "Value1".into()
    }));
    assert!(f.links.contains(&FlatLink {
        from: math,
        from_socket: "Value".into(),
        to: emission,
        to_socket: "Color".into()
    }));
}

#[test]
fn a_missing_group_or_output_or_a_loop_is_refused() {
    let no_output = NOISE_RAMP.replace("ShaderNodeOutputMaterial", "ShaderNodeEmission");
    assert!(import(&no_output, None).is_err());
    let looped = NOISE_RAMP.replace(
        r#"{ "from": ["Noise Texture", "Fac"], "to": ["Color Ramp", "Fac"] },"#,
        r#"{ "from": ["Noise Texture", "Fac"], "to": ["Color Ramp", "Fac"] },
           { "from": ["Mix", "Result", "Result_Color"], "to": ["Noise Texture", "Scale"] },"#,
    );
    assert!(import(&looped, None).unwrap_err().contains("loop"));
    let group = NOISE_RAMP.replace(
        "\"type\": \"ShaderNodeValToRGB\",",
        "\"type\": \"ShaderNodeGroup\",",
    );
    assert!(import(&group, None).is_err());
}

#[test]
fn math_and_mix_shader_sockets_use_cycles_names() {
    let text = r#"{"format":"operandi-node-graph","format_version":1,"materials":[{
      "name":"M","nodes":[
        {"name":"E","type":"ShaderNodeEmission","inputs":{}},
        {"name":"T","type":"ShaderNodeBsdfTransparent","inputs":{}},
        {"name":"Mix Shader","type":"ShaderNodeMixShader","inputs":{"Fac":{"type":"VALUE","value":0.3}}},
        {"name":"Material Output","type":"ShaderNodeOutputMaterial","inputs":{}}],
      "links":[
        {"from":["E","Emission"],"to":["Mix Shader","Shader"]},
        {"from":["T","BSDF"],"to":["Mix Shader","Shader","Shader_001"]},
        {"from":["Mix Shader","Shader"],"to":["Material Output","Surface"]}]}]}"#;
    let f = import(text, None)
        .unwrap()
        .remove(0)
        .flatten(&Default::default())
        .unwrap();
    let (mix, _) = node(&f, "mix_closure");
    let mut sockets: Vec<_> = f
        .links
        .iter()
        .filter(|l| l.to == mix)
        .map(|l| l.to_socket.as_str())
        .collect();
    sockets.sort();
    assert_eq!(sockets, ["Closure1", "Closure2"]);
    assert_eq!(f.feeding("Surface"), Some((mix, "Closure")));
    assert_eq!(f.value(mix, "Fac"), Some(&FlatValue::Float(0.3)));
}

#[test]
fn a_file_imports_its_renderable_materials_and_names_the_refused_ones() {
    let mut file: serde_json::Value = serde_json::from_str(NOISE_RAMP).unwrap();
    let mut other = file["materials"][0].clone();
    other["name"] = "Stroke".into();
    other["nodes"][0]["type"] = "ShaderNodeBsdfDiffuse".into();
    file["materials"].as_array_mut().unwrap().push(other);
    let (graphs, refused) = import_report(&file.to_string(), None).unwrap();
    assert_eq!(graphs.len(), 1);
    assert_eq!(refused.len(), 1);
    assert!(refused[0].contains("Stroke") && refused[0].contains("ShaderNodeBsdfDiffuse"));
}

#[test]
fn group_sockets_read_blender_socket_idnames() {
    let s: GroupSocket = serde_json::from_str(
        r#"{"name":"Amount","identifier":"Socket_0","type":"NodeSocketFloat","default":0.0}"#,
    )
    .unwrap();
    assert_eq!(s.kind, Some(SocketType::Value));
    let s: GroupSocket =
        serde_json::from_str(r#"{"name":"Result","type":"NodeSocketColor"}"#).unwrap();
    assert_eq!(s.kind, Some(SocketType::Rgba));
}
