//! §3–4 rules from hand-made values (BUILD-BRIEF "Tests").
use super::*;
use crate::lxo::Value;
use crate::lxo::preset_items::{Envelope, GradientChannel};

fn item(kind: &str, channels: &[(&str, Value)]) -> PresetItem {
    PresetItem {
        id: 1,
        kind: kind.into(),
        name: "M".into(),
        channels: channels
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        ..Default::default()
    }
}
fn f(v: f64) -> Value {
    Value::Float(v)
}
fn rgb(key: &str, c: [f64; 3]) -> Vec<(String, Value)> {
    ["R", "G", "B"]
        .iter()
        .zip(c)
        .map(|(k, v)| (format!("{key}.{k}"), Value::Float(v)))
        .collect()
}
fn with(mut m: PresetItem, extra: Vec<(String, Value)>) -> PresetItem {
    m.channels.extend(extra);
    m
}
fn values(m: &PresetItem) -> (Values, Vec<(bool, String)>) {
    let mut notes = Vec::new();
    let (v, _) = advanced_values(m, &mut notes);
    (v, notes)
}

#[test]
fn spec_amount_is_f0() {
    assert!((ior_from_f0(0.04) - 1.5).abs() < 1e-9);
    let m = item(
        "advancedMaterial",
        &[("specAmt", f(0.04)), ("diffAmt", f(0.8))],
    );
    let (v, _) = values(&m);
    assert!((v.get(Param::SpecularIor).float() - 1.5).abs() < 1e-9);
}

#[test]
fn use_refractive_index_ignores_spec_amount() {
    let m = item(
        "advancedMaterial",
        &[
            ("useRefIdx", Value::Int(1)),
            ("refIndex", f(1.52)),
            ("specAmt", f(0.02)),
            ("tranAmt", f(0.98)),
            ("diffAmt", f(0.8)),
        ],
    );
    let (v, _) = values(&m);
    assert!((v.get(Param::SpecularIor).float() - 1.52).abs() < 1e-9);
    assert_eq!(v.get(Param::SpecularWeight).float(), 1.);
    let mc = material_channels(&m, &mut Vec::new());
    assert!(mc.more.use_refractive_index);
    assert!((mc.refractive_index - 1.52).abs() < 1e-9);
    assert!((mc.transparent_amount - 0.98).abs() < 1e-9);
}

#[test]
fn metal_made_by_hand() {
    let m = with(
        item(
            "advancedMaterial",
            &[("diffAmt", f(0.)), ("specAmt", f(1.)), ("rough", f(0.1))],
        ),
        rgb("specCol", [1., 0.77, 0.34]),
    );
    let (v, notes) = values(&m);
    assert_eq!(v.get(Param::BaseMetalness).float(), 1.);
    assert_eq!(v.get(Param::BaseColor).color(), [1., 0.77, 0.34]);
    assert_eq!(v.get(Param::SpecularColor).color(), [1.; 3]);
    assert!(notes.iter().any(|(_, n)| n == "converted to metal"));
    let mc = material_channels(&m, &mut Vec::new());
    assert_eq!(mc.metallic, 1.);
    assert_eq!(mc.diffuse_color, [1., 0.77, 0.34]);
    assert_eq!(mc.diffuse_amount, 1.);
    assert!((mc.roughness - 0.1).abs() < 1e-9);
}

#[test]
fn diffuse_amount_over_one_is_clamped_and_noted() {
    let m = item("advancedMaterial", &[("diffAmt", f(1.4))]);
    let (v, notes) = values(&m);
    assert_eq!(v.get(Param::BaseWeight).float(), 1.);
    assert!(notes.iter().any(|(look, n)| *look && n.contains("clamped")));
}

#[test]
fn legacy_reflection_counts_toward_specular() {
    let m = item(
        "advancedMaterial",
        &[
            ("brdfType", Value::Text("ashikhmin".into())),
            ("specAmt", f(0.01)),
            ("reflAmt", f(0.04)),
            ("reflSpec", Value::Int(0)),
            ("diffAmt", f(0.6)),
        ],
    );
    let (v, notes) = values(&m);
    assert!((v.get(Param::SpecularIor).float() - 1.5).abs() < 1e-9);
    assert!(notes.iter().any(|(_, n)| n.contains("legacy")));
}

#[test]
fn emission_spec_fresnel_phase_and_dissolve() {
    let m = with(
        item(
            "advancedMaterial",
            &[
                ("radiance", f(1.7)),
                ("specFres", f(0.02)),
                ("subsPhase", f(0.75)),
                ("dissAmt", f(0.25)),
                ("smAngle", f(std::f64::consts::FRAC_PI_4)),
            ],
        ),
        rgb("lumiCol", [0.98; 3]),
    );
    let (v, notes) = values(&m);
    assert!((v.get(Param::EmissionLuminance).float() - 1.7).abs() < 1e-9);
    assert_eq!(v.get(Param::EmissionColor).color(), [0.98; 3]);
    assert!((v.get(Param::SubsurfaceScatterAnisotropy).float() - 0.5).abs() < 1e-9);
    assert!((v.get(Param::GeometryOpacity).float() - 0.75).abs() < 1e-9);
    assert!(notes.iter().any(|(look, n)| !look && n.contains("Fresnel")));
    let mc = material_channels(&m, &mut Vec::new());
    assert!((mc.smoothing_angle - 45.).abs() < 1e-9);
    // LXP-B02b: radiance 1:1 (OPM-Q09), so White LED's Luminous
    // Intensity is 1.7.
    assert!((mc.luminous_intensity - 1.7).abs() < 1e-9);
    assert_eq!(mc.luminous_color, [0.98; 3]);
}

#[test]
fn unreal_and_unity() {
    let ue = with(
        item(
            "unrealShader",
            &[
                ("metallic", f(0.04)),
                ("specular", f(0.5)),
                ("roughness", f(0.5)),
            ],
        ),
        rgb("base", [0.; 3]),
    );
    let (v, _) = engine_values(&ue, &mut Vec::new());
    assert!((v.get(Param::SpecularIor).float() - 1.5).abs() < 1e-9);
    assert_eq!(v.get(Param::SpecularRoughness).float(), 0.5);
    assert!((v.get(Param::BaseMetalness).float() - 0.04).abs() < 1e-12);
    let ut = item(
        "unityShader",
        &[("smoothness", f(0.7)), ("fresrel0", f(0.04))],
    );
    let (v, _) = engine_values(&ut, &mut Vec::new());
    assert!((v.get(Param::SpecularRoughness).float() - 0.3).abs() < 1e-9);
    assert!((v.get(Param::SpecularIor).float() - 1.5).abs() < 1e-9);
    assert_eq!(
        material_channels(&ut, &mut Vec::new()).shading_model,
        ShadingModel::Principled
    );
}

#[test]
fn effect_table_round_trips() {
    for (id, effect, invert) in EFFECTS {
        assert_eq!(
            target(id),
            Target::To {
                effect: *effect,
                invert: *invert
            },
            "{id}"
        );
    }
    for (id, _) in DROPPED_EFFECTS {
        assert!(matches!(target(id), Target::Drop(_)), "{id}");
    }
    assert!(matches!(target("noSuchEffect"), Target::Drop(_)));
    // No id is listed twice.
    let mut ids: Vec<&str> = EFFECTS
        .iter()
        .map(|e| e.0)
        .chain(DROPPED_EFFECTS.iter().map(|e| e.0))
        .collect();
    ids.sort();
    let n = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), n);
}

#[test]
fn default_colour_space_on_a_roughness_map_is_linear() {
    assert!(reads_linear(Some("(default)"), TextureEffect::Roughness));
    assert!(reads_linear(Some("(default)"), TextureEffect::Metallic));
    assert!(!reads_linear(
        Some("(default)"),
        TextureEffect::DiffuseColor
    ));
    assert!(reads_linear(Some("(none)"), TextureEffect::Normal));
    assert!(reads_linear(Some("(none)"), TextureEffect::DiffuseColor));
}

#[test]
fn gradient_inputs_by_name() {
    assert_eq!(
        gradient_input("incidenceAngle"),
        Some(GradientInput::Incidence)
    );
    assert_eq!(gradient_input("driverB"), Some(GradientInput::DriverB));
    assert_eq!(gradient_input("slope"), Some(GradientInput::Slope));
    assert_eq!(gradient_input("hairLength"), None);
}

#[test]
fn image_paths_resolve_relative_alias_and_moved() {
    let search = ImageSearch {
        folder: "/presets/Metal".into(),
        roots: vec!["/content".into()],
    };
    let have = |want: &'static str| move |p: &Path| p == Path::new(want);
    assert_eq!(
        resolve_image("a.png", &search, &have("/presets/Metal/a.png")),
        Some("/presets/Metal/a.png".into())
    );
    assert_eq!(
        resolve_image(
            "kit_Modo_Content:Assets/Images/b.png",
            &search,
            &have("/content/Assets/Images/b.png")
        ),
        Some("/content/Assets/Images/b.png".into())
    );
    // An absolute path from another machine, found under the root.
    assert_eq!(
        resolve_image(
            "/Library/Application Support/Vendor/Content/Assets/Images/c.png",
            &search,
            &have("/content/Assets/Images/c.png")
        ),
        Some("/content/Assets/Images/c.png".into())
    );
    assert_eq!(resolve_image("x.png", &search, &|_| false), None);
}

/// A preset: mask, material, a Modo noise on bump, a plug-in noise on
/// diffuse colour and an Incidence gradient on roughness.
fn sample() -> PresetItems {
    use crate::lxo::preset_items::Envelope;
    let link = |t: u32, o: u32| ("parent".to_string(), t, o);
    let mut items = BTreeMap::new();
    let mut mask = item("mask", &[]);
    mask.id = 1;
    mask.name = "Leather".into();
    items.insert(1, mask);
    let mut m = item(
        "advancedMaterial",
        &[("diffAmt", f(0.8)), ("specAmt", f(0.04))],
    );
    m.id = 2;
    m.links.push(link(1, 0));
    items.insert(2, m);
    let mut noise = item(
        "noise",
        &[("effect", Value::Text("bump".into())), ("freqs", f(3.))],
    );
    noise.id = 3;
    noise.links.push(link(1, 1));
    items.insert(3, noise);
    let mut rjj = item(
        "val.Noise_Scruffed.RJJ",
        &[("effect", Value::Text("diffColor".into()))],
    );
    rjj.id = 4;
    rjj.links.push(link(1, 2));
    items.insert(4, rjj);
    let mut g = item(
        "gradient",
        &[
            ("effect", Value::Text("rough".into())),
            ("param", Value::Text("incidenceAngle".into())),
            ("blend", Value::Text("multiply".into())),
        ],
    );
    g.id = 5;
    g.links.push(link(1, 3));
    g.gradients.push(GradientChannel {
        channel: "".into(),
        envelope: 9,
        interpolation: Some(0),
        // Percent-typed values read as fractions (LXP-B07c).
        types: Some(("percent".into(), "percent".into())),
    });
    items.insert(5, g);
    let mut envelopes = BTreeMap::new();
    envelopes.insert(
        9,
        Envelope {
            keys: vec![(0., 1.), (std::f64::consts::FRAC_PI_2, 0.5)],
        },
    );
    PresetItems {
        form: *b"LXPR",
        items,
        envelopes,
        skipped: BTreeMap::new(),
    }
}

#[test]
fn converts_layers_drops_plugins_and_badges_partial() {
    let c = convert_with(
        &sample(),
        "Leather",
        &ImageSearch::default(),
        &|_| None,
        &|_| false,
    )
    .unwrap();
    let tree = &c.preset.scene.shader_tree;
    // Group, gradient, noise, material: the plug-in layer is gone (LXP-Q7).
    let names: Vec<&str> = tree.nodes.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(tree.nodes[0].name, "Leather");
    assert_eq!(tree.nodes.len(), 4, "{names:?}");
    let NodeKind::Texture(t) = &tree.nodes[1].kind else {
        panic!("{names:?}")
    };
    let TextureSource::Gradient(g) = &t.source else {
        panic!("{names:?}")
    };
    assert_eq!(g.input, GradientInput::Incidence);
    // Radians onto degrees.
    assert!((g.value.keys[1].at - 90.).abs() < 1e-9);
    assert_eq!(tree.nodes[1].layer.blend, BlendMode::Multiply);
    assert!(matches!(&tree.nodes[2].kind, NodeKind::Texture(t) if t.effect == TextureEffect::Bump));
    assert!(matches!(tree.nodes[3].kind, NodeKind::Material(_)));
    assert_eq!((c.layers, c.dropped), (3, 1));
    // The only diffuse colour layer was dropped: partial (LXP-05).
    assert!(c.partial);
    assert!(c.changed_look);
    assert!(
        c.notes
            .iter()
            .any(|n| n.contains("plug-in layer val.Noise_Scruffed.RJJ")),
        "{:?}",
        c.notes
    );
    // Every layer has its own locator.
    for n in &tree.nodes[1..3] {
        assert!(matches!(&n.kind, NodeKind::Texture(t) if t.locator.is_some()));
    }
}

/// LXP-B07c (answered): percent keys are 0–1 fractions, and over 100 %
/// is legitimate, so 1.5 stays 1.5.
#[test]
fn percent_gradient_keys_are_fractions_and_not_clamped() {
    let mut p = sample();
    p.envelopes.get_mut(&9).unwrap().keys[1].1 = 1.5;
    let c = convert_with(&p, "x", &ImageSearch::default(), &|_| None, &|_| false).unwrap();
    let NodeKind::Texture(t) = &c.preset.scene.shader_tree.nodes[1].kind else {
        panic!()
    };
    let TextureSource::Gradient(g) = &t.source else {
        panic!()
    };
    assert_eq!(g.value.keys[0].out, 1.);
    assert_eq!(g.value.keys[1].out, 1.5);
}

#[test]
fn a_preset_without_material_is_refused() {
    let mut p = sample();
    p.items.remove(&2);
    let e = convert_with(&p, "x", &ImageSearch::default(), &|_| None, &|_| false).unwrap_err();
    assert_eq!(e, NO_MATERIAL);
}

#[test]
fn a_dropped_driver_takes_its_gradient() {
    let mut p = sample();
    let mut d = item("val.Camo.RJJ", &[("effect", Value::Text("driverA".into()))]);
    d.id = 6;
    p.items.insert(6, d);
    if let Some(g) = p.items.get_mut(&5) {
        g.channels
            .insert("param".into(), Value::Text("driverA".into()));
    }
    let c = convert_with(&p, "x", &ImageSearch::default(), &|_| None, &|_| false).unwrap();
    assert_eq!(c.dropped, 3);
    assert!(c.notes.iter().any(|n| n.contains("dropped driver")));
}

// ------------------------------------------------- LXP-B07 gradients

fn rec(channel: &str, envelope: u32, flag: u32) -> GradientChannel {
    GradientChannel {
        channel: channel.into(),
        envelope,
        interpolation: Some(flag),
        types: None,
    }
}
fn grad_items(
    effect: &str,
    records: Vec<GradientChannel>,
    envs: &[(u32, Vec<(f64, f64)>)],
) -> (PresetItem, PresetItems) {
    let mut g = item(
        "gradient",
        &[
            ("effect", Value::Text(effect.into())),
            ("param", Value::Text("texValue".into())),
        ],
    );
    g.gradients = records;
    let items = PresetItems {
        envelopes: envs
            .iter()
            .map(|(i, k)| (*i, Envelope { keys: k.clone() }))
            .collect(),
        ..Default::default()
    };
    (g, items)
}
fn run(
    g: &PresetItem,
    items: &PresetItems,
    effect: TextureEffect,
) -> (Result<Gradient, String>, Vec<String>) {
    let mut notes = Notes {
        lines: vec![],
        look: false,
    };
    let r = gradient(g, items, effect, &mut notes, "G");
    (r, notes.lines)
}

#[test]
fn b07_interpolation_flags_map_to_key_shapes() {
    assert_eq!(key_shape(Some(0)), Some(KeyShape::Smooth));
    assert_eq!(key_shape(Some(1)), Some(KeyShape::Linear));
    assert_eq!(key_shape(Some(2)), Some(KeyShape::Stepped));
    assert_eq!(key_shape(None), Some(KeyShape::Smooth));
    assert_eq!(key_shape(Some(7)), None);
}

#[test]
fn b07_scalar_effect_reads_the_value_record() {
    let (g, items) = grad_items(
        "rough",
        vec![rec("color.R", 1, 0), rec("value", 2, 1)],
        &[(1, vec![(0., 0.9)]), (2, vec![(0., 0.2), (1., 0.8)])],
    );
    let (r, notes) = run(&g, &items, TextureEffect::Roughness);
    let g = r.unwrap();
    assert_eq!(g.value.keys.len(), 2);
    assert_eq!(g.value.keys[0].out, 0.2);
    assert_eq!(g.value.keys[1].shape, KeyShape::Linear);
    assert!((g.value.sample(0.5) - 0.5).abs() < 1e-9);
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn b07_colour_from_independent_component_curves() {
    // R keys at 0 and 1, G at 0.5 only, B linear 0 → 1; A missing with a
    // static alpha of 0.5 on the item.
    let (mut g, items) = grad_items(
        "diffColor",
        vec![
            rec("color.R", 1, 1),
            rec("color.G", 2, 1),
            rec("color.B", 3, 1),
        ],
        &[
            (1, vec![(0., 1.), (1., 0.)]),
            (2, vec![(0.5, 0.25)]),
            (3, vec![(0., 0.), (1., 1.)]),
        ],
    );
    g.channels.insert("color.A".into(), f(0.5));
    let (r, notes) = run(&g, &items, TextureEffect::DiffuseColor);
    let c = r.unwrap().color;
    let at: Vec<f64> = c.keys.iter().map(|k| k.at).collect();
    assert_eq!(at, vec![0., 0.5, 1.]);
    assert!(c.keys.iter().all(|k| k.shape == KeyShape::Linear));
    let close = |a: [f64; 4], b: [f64; 4]| (0..4).all(|i| (a[i] - b[i]).abs() < 1e-9);
    assert!(
        close(c.keys[0].out, [1., 0.25, 0., 0.5]),
        "{:?}",
        c.keys[0].out
    );
    assert!(
        close(c.keys[1].out, [0.5, 0.25, 0.5, 0.5]),
        "{:?}",
        c.keys[1].out
    );
    assert!(
        close(c.keys[2].out, [0., 0.25, 1., 0.5]),
        "{:?}",
        c.keys[2].out
    );
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn b07_missing_component_holds_static_value_or_zero() {
    let (mut g, items) = grad_items(
        "specColor",
        vec![rec("color.R", 1, 0)],
        &[(1, vec![(0., 0.3), (1., 0.6)])],
    );
    g.channels.insert("color.G".into(), f(0.7));
    let (r, _) = run(&g, &items, TextureEffect::SpecularColor);
    let c = r.unwrap().color;
    assert_eq!(c.keys.len(), 2);
    assert_eq!(c.keys[1].out, [0.6, 0.7, 0., 1.]);
    assert_eq!(c.keys[0].shape, KeyShape::Smooth);
}

#[test]
fn b07_one_stepped_component_steps_the_whole_colour() {
    let (g, items) = grad_items(
        "diffColor",
        vec![
            rec("color.R", 1, 1),
            rec("color.G", 2, 2),
            rec("color.B", 3, 0),
        ],
        &[
            (1, vec![(0., 0.), (1., 1.)]),
            (2, vec![(0., 0.), (1., 1.)]),
            (3, vec![(0., 0.), (1., 1.)]),
        ],
    );
    let (r, _) = run(&g, &items, TextureEffect::DiffuseColor);
    let c = r.unwrap().color;
    assert!(c.keys.iter().all(|k| k.shape == KeyShape::Stepped));
    assert_eq!(c.sample(0.5), [0., 0., 0., 1.]);
}

#[test]
fn b07_colour_effect_with_only_a_value_curve_is_grey() {
    for name in ["value", ""] {
        let (g, items) = grad_items(
            "diffColor",
            vec![rec(name, 1, 1)],
            &[(1, vec![(0., 0.2), (1., 0.8)])],
        );
        let (r, notes) = run(&g, &items, TextureEffect::DiffuseColor);
        let c = r.unwrap().color;
        assert_eq!(c.keys[0].out, [0.2, 0.2, 0.2, 1.]);
        assert_eq!(c.keys[1].out, [0.8, 0.8, 0.8, 1.]);
        assert_eq!(c.keys[1].shape, KeyShape::Linear);
        assert_eq!(notes, vec!["G: colour gradient had only a value curve"]);
    }
}

#[test]
fn b07_unknown_flag_is_approximated_with_a_note() {
    let (g, items) = grad_items("rough", vec![rec("value", 1, 9)], &[(1, vec![(0., 0.5)])]);
    let (r, notes) = run(&g, &items, TextureEffect::Roughness);
    assert_eq!(r.unwrap().value.keys[0].shape, KeyShape::Smooth);
    assert_eq!(notes, vec!["G: gradient curve approximated"]);
}

#[test]
fn b07_scalar_effect_without_a_value_curve_refuses() {
    let (g, items) = grad_items(
        "rough",
        vec![rec("color.R", 1, 0), rec("color.G", 2, 0)],
        &[(1, vec![(0., 0.5)]), (2, vec![(0., 0.5)])],
    );
    let (r, _) = run(&g, &items, TextureEffect::Roughness);
    assert_eq!(r.unwrap_err(), "gradient with no value curve");
}
