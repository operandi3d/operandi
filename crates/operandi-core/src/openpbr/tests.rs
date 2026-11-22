use super::*;
use crate::shader_eval::{Context, Principled};
use crate::shader_tree::{NodeKind, ShaderTree};

fn ctx() -> Context<'static> {
    Context {
        item: 1,
        ancestors: &[],
        material: "Default",
        part: None,
        sets: &[],
        locators: &[],
    }
}

/// The default tree with its Base Material replaced.
fn principled(m: &MaterialChannels) -> Principled {
    let mut tree = ShaderTree::default();
    for node in &mut tree.nodes {
        if let NodeKind::Material(base) = &mut node.kind {
            **base = m.clone();
        }
    }
    tree.evaluate(&ctx()).principled()
}

fn inputs(p: &Principled) -> Vec<(&'static str, [f64; 3])> {
    p.inputs
        .iter()
        .map(|(i, e)| (i.socket(), e.constant().expect("constant input")))
        .collect()
}

fn near(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
}

#[test]
fn every_panel_channel_has_an_identity() {
    let mut m = MaterialChannels::default();
    let panel: Vec<&str> = m.channels().into_iter().map(|(n, _)| n).collect();
    let table: Vec<&str> = CHANNELS.iter().map(|(n, _)| *n).collect();
    assert_eq!(panel, table);
    // Each OpenPBR input is at most one channel's identity.
    let mut seen = std::collections::BTreeSet::new();
    for (name, id) in CHANNELS {
        if let Identity::OpenPbr(p, _) = id {
            assert!(seen.insert(*p), "{name}: {p:?} twice");
        }
    }
}

#[test]
fn param_names_round_trip_and_defaults_match_the_types() {
    assert_eq!(Param::ALL.len(), 41);
    for p in Param::ALL {
        assert_eq!(Param::from_name(p.name()), Some(*p));
        let ok = matches!(
            (p.kind(), p.default()),
            (Kind::Float, Value::Float(_))
                | (Kind::Color3, Value::Color(_))
                | (Kind::Boolean, Value::Bool(_))
                | (Kind::Vector3, Value::Geometry)
        );
        assert!(ok, "{p:?}");
    }
}

#[test]
fn effects_know_their_openpbr_name() {
    use crate::shader_tree::TextureEffect as E;
    assert_eq!(E::DiffuseColor.openpbr_name(), Some("base_color"));
    assert_eq!(E::Roughness.openpbr_name(), Some("specular_roughness"));
    assert_eq!(E::Normal.openpbr_name(), Some("geometry_normal"));
    assert_eq!(E::Dissolve.openpbr_name(), Some("geometry_opacity"));
    assert_eq!(E::DriverA.openpbr_name(), None);
    // Every named effect drives a channel whose identity is that input.
    for e in E::ALL {
        if let Some(name) = e.openpbr_name() {
            assert!(Param::from_name(name).is_some(), "{e:?}");
        }
    }
}

#[test]
fn older_materials_read_the_legacy_slots_and_translate_as_before() {
    let mut m = MaterialChannels {
        openpbr: crate::shader_tree::OpenPbrChannels::legacy(),
        ..Default::default()
    };
    let json = serde_json::to_value(&m).unwrap();
    assert!(
        json.get("openpbr").is_none(),
        "legacy values aren't written"
    );
    let back: MaterialChannels = serde_json::from_value(json).unwrap();
    assert_eq!(back.openpbr, crate::shader_tree::OpenPbrChannels::legacy());
    // New materials start at OpenPBR's defaults and write them (OPM-Q11).
    let fresh = MaterialChannels::default();
    assert_eq!(fresh.openpbr.coat_ior, 1.6);
    assert!(!fresh.openpbr.subsurface_uses_diffuse);
    assert!(
        serde_json::to_value(&fresh)
            .unwrap()
            .get("openpbr")
            .is_some()
    );

    // With coat, subsurface, sheen and glass on, an older material gives
    // none of the new inputs and its Base Color is the Diffuse Color.
    m.clearcoat_amount = 0.5;
    m.subsurface_amount = 0.5;
    m.sheen = 0.5;
    m.transparent_amount = 0.5;
    m.refractive_index = 1.4;
    m.absorption_distance = 0.2;
    let p = principled(&m);
    let new = [
        Input::CoatTint,
        Input::CoatIor,
        Input::SheenTint,
        Input::SheenRoughness,
        Input::ThinFilmThickness,
        Input::ThinFilmIor,
        Input::DispersionScale,
        Input::DispersionAbbe,
    ];
    assert!(
        p.inputs.iter().all(|(i, _)| !new.contains(i)),
        "{:?}",
        p.inputs
    );
    let base = inputs(&p)
        .into_iter()
        .find(|(s, _)| *s == "Base Color")
        .unwrap()
        .1;
    assert!(near(base, m.diffuse_color));
    assert!(p.scatter.is_none() && !p.thin_walled && !p.glass_principled);
}

#[test]
fn new_defaults_leave_the_default_surface_unchanged() {
    // Coat IOR 1.6 and the albedo only show with coat and subsurface on.
    let old = MaterialChannels {
        openpbr: crate::shader_tree::OpenPbrChannels::legacy(),
        ..Default::default()
    };
    assert_eq!(
        principled(&MaterialChannels::default()).normalized(),
        principled(&old).normalized()
    );
}

/// Varied OpenPBR inputs that Operandi's channels hold exactly.
fn sample() -> Values {
    use Param::*;
    let mut v = Values::default();
    for (p, val) in [
        (BaseColor, C([0.5, 0.3, 0.2])),
        (BaseWeight, F(0.9)),
        (BaseMetalness, F(0.2)),
        (BaseDiffuseRoughness, F(0.3)),
        (SpecularWeight, F(0.8)),
        (SpecularColor, C([0.9, 0.8, 0.7])),
        (SpecularRoughness, F(0.4)),
        (SpecularRoughnessAnisotropy, F(0.25)),
        (SubsurfaceWeight, F(0.4)),
        (SubsurfaceColor, C([0.9, 0.5, 0.4])),
        (SubsurfaceRadius, F(0.02)),
        (SubsurfaceRadiusScale, C([1., 0.4, 0.2])),
        (SubsurfaceScatterAnisotropy, F(0.3)),
        (FuzzWeight, F(0.6)),
        (FuzzColor, C([0.7, 0.6, 1.])),
        (FuzzRoughness, F(0.3)),
        (CoatWeight, F(0.7)),
        (CoatColor, C([0.9, 0.95, 1.])),
        (CoatRoughness, F(0.15)),
        (CoatIor, F(1.7)),
        (ThinFilmWeight, F(1.)),
        (ThinFilmThickness, F(0.35)),
        (ThinFilmIor, F(1.5)),
        (EmissionLuminance, F(2.5)),
        (EmissionColor, C([1., 0.5, 0.25])),
        (GeometryOpacity, F(0.75)),
    ] {
        v.set(p, val);
    }
    v
}

#[test]
fn both_routes_to_cycles_agree() {
    // OpenPBR -> Material -> the render path's principled inputs, and
    // OpenPBR -> Cycles straight from the table: the same sockets.
    for transmission in [0., 0.6] {
        let mut v = sample();
        v.set(Param::TransmissionWeight, F(transmission));
        v.set(
            Param::SpecularIor,
            F(if transmission > 0. { 1.45 } else { 1.5 }),
        );
        v.set(Param::TransmissionDispersionScale, F(0.4));
        v.set(Param::TransmissionDispersionAbbeNumber, F(35.));
        let (m, notes) = to_material(&v);
        assert!(notes.is_empty(), "{notes:?}");
        let render = inputs(&principled(&m));
        for (socket, want) in principled_values(&v) {
            let got = render
                .iter()
                .find(|(s, _)| *s == socket)
                .unwrap_or_else(|| panic!("{socket} missing ({transmission})"))
                .1;
            assert!(near(got, want), "{socket}: {got:?} vs {want:?}");
        }
    }
}

#[test]
fn opaque_index_of_refraction_uses_the_materials_own_index() {
    let mut v = Values::default();
    v.set(Param::SpecularIor, F(2.4));
    let (m, notes) = to_material(&v);
    assert!(m.more.use_refractive_index && m.refractive_index == 2.4);
    assert!(notes.is_empty());
    let render = inputs(&principled(&m));
    for (socket, want) in principled_values(&v) {
        if socket == "IOR" || socket == "Specular IOR Level" {
            let got = render.iter().find(|(s, _)| *s == socket).unwrap().1;
            assert!(near(got, want), "{socket}");
        }
    }
    v.set(Param::SpecularWeight, F(0.5));
    assert_eq!(to_material(&v).1.len(), 1, "the weight is noted");
}

#[test]
fn constants_round_trip_through_openpbr() {
    let (m, _) = to_material(&sample());
    let (v, losses) = from_material(&m);
    assert!(losses.is_empty(), "{losses:?}");
    let (back, _) = to_material(&v);
    assert_eq!(back, m);
    // A see-through material keeps its index and Specular Amount.
    let mut glass = MaterialChannels::default();
    glass.transparent_amount = 1.;
    glass.refractive_index = 1.33;
    glass.specular_amount = 0.6;
    glass.refraction_roughness = glass.roughness;
    let (v, _) = from_material(&glass);
    let (back, _) = to_material(&v);
    assert!((back.specular_amount - 0.6).abs() < 1e-9);
    assert_eq!(back.refractive_index, 1.33);
}

#[test]
fn exports_list_what_openpbr_cannot_hold() {
    let mut m = MaterialChannels {
        match_specular: false,
        reflection_amount: 0.5,
        transparent_amount: 0.5,
        refraction_roughness: 0.9,
        ..Default::default()
    };
    m.more.rounded_edge_width = 0.01;
    let (_, losses) = from_material(&m);
    assert_eq!(losses.len(), 3, "{losses:?}");
}

#[test]
fn interior_volume_without_scatter_is_the_older_absorption() {
    let t = [0.6, 0.9, 0.7];
    let d = 0.3;
    let (abs, sc) = interior_volume(t, [0.; 3], 0., d);
    assert!(sc.is_none());
    let sigma = t.map(|v: f64| -v.ln() / d);
    let density = sigma.iter().copied().fold(0., f64::max);
    let (colour, got) = abs.unwrap();
    assert!((got - density).abs() < 1e-12);
    assert!(near(colour, sigma.map(|s| 1. - s / density)));
    // Scatter takes its share of the extinction; a negative absorption
    // is lifted by grey.
    let (abs, sc) = interior_volume([0.5; 3], [0.2, 0.9, 0.2], 0.4, 1.);
    let (sc_colour, sc_density, aniso) = sc.unwrap();
    assert!((sc_density - 0.9).abs() < 1e-12 && aniso == 0.4);
    assert!(near(sc_colour, [0.2 / 0.9, 1., 0.2 / 0.9]));
    let (a_colour, a_density) = abs.unwrap();
    let mu_t = -(0.5f64).ln();
    let mu_a = [mu_t - 0.2, mu_t - 0.9, mu_t - 0.2].map(|v| v - (mu_t - 0.9));
    assert!((a_density - mu_a[0]).abs() < 1e-12);
    assert!(near(a_colour, mu_a.map(|v| 1. - v / a_density)));
}

#[test]
fn thin_film_and_dispersion_reach_cycles_only_when_on() {
    let mut m = MaterialChannels::default();
    let has = |p: &Principled, i: Input| p.inputs.iter().any(|(x, _)| *x == i);
    let p = principled(&m);
    assert!(!has(&p, Input::ThinFilmThickness) && !has(&p, Input::DispersionScale));
    m.openpbr.thin_film_weight = 0.5;
    m.openpbr.thin_film_thickness = 0.4;
    let p = principled(&m);
    let got = inputs(&p);
    let film = got
        .iter()
        .find(|(s, _)| *s == "Thin Film Thickness")
        .unwrap()
        .1;
    assert!(near(film, [400.; 3]));
    assert!(has(&p, Input::ThinFilmIor));
    // Dispersion on a see-through glass builds the principled glass.
    m.transparent_amount = 1.;
    m.refractive_index = 1.5;
    m.more.dispersion = 0.5;
    let p = principled(&m);
    assert!(has(&p, Input::DispersionScale) && p.glass_principled);
    m.more.dispersion = 0.;
    m.openpbr.thin_walled = true;
    m.absorption_distance = 0.1;
    let p = principled(&m);
    assert!(p.thin_walled && p.glass_principled && p.absorption.is_none());
}

#[test]
fn subsurface_albedo_leans_the_base_colour() {
    let mut m = MaterialChannels {
        diffuse_color: [0.2, 0.2, 0.2],
        subsurface_amount: 0.5,
        ..Default::default()
    };
    m.openpbr.subsurface_albedo = [1., 0.6, 0.4];
    m.openpbr.subsurface_uses_diffuse = false;
    let base = inputs(&principled(&m))
        .into_iter()
        .find(|(s, _)| *s == "Base Color")
        .unwrap()
        .1;
    assert!(near(base, [0.6, 0.4, 0.3]));
    m.openpbr.subsurface_uses_diffuse = true;
    let base = inputs(&principled(&m))
        .into_iter()
        .find(|(s, _)| *s == "Base Color")
        .unwrap()
        .1;
    assert!(near(base, [0.2; 3]));
}

#[test]
fn every_param_has_a_cycles_target() {
    for p in Param::ALL {
        let _ = cycles_target(*p);
    }
    assert_eq!(
        cycles_target(Param::CoatColor),
        Cycles::Socket("Coat Tint"),
        "OPM-Q03: coat_color is Cycles' Coat Tint"
    );
}

#[test]
fn material_rows_grey_while_their_controlling_row_turns_them_off() {
    let mut m = MaterialChannels {
        clearcoat_amount: 0.,
        match_specular: true,
        sheen: 0.,
        ..Default::default()
    };
    m.openpbr.thin_film_weight = 0.;
    m.more.dispersion = 0.;
    m.openpbr.transmission_scatter = [0.; 3];
    m.transparent_amount = 0.;
    m.subsurface_amount = 0.;
    m.smoothing = 0.;
    m.more.rounded_edge_width = 0.;
    m.more.clip_enable = false;
    m.more.matte = false;
    m.more.gl_color_override = false;
    // Every gated row greys (OpenPBR rows and the older ones alike).
    for g in GATES {
        for name in g.rows {
            assert!(inert_reason(&m, name).is_some(), "{name}");
        }
    }
    for (name, want) in [
        ("Clearcoat Roughness", "Clearcoat Amount"),
        ("Clearcoat Bump", "Clearcoat Amount"),
        ("Sheen Tint", "Sheen"),
        ("Reflection Fresnel", "Match Specular"),
        ("Refraction Roughness", "Transparent Amount"),
        ("Subsurface Distance", "Subsurface Amount"),
        ("Smoothing Angle", "Smoothing"),
        ("Clip Value", "Clip Enable"),
    ] {
        assert!(inert_reason(&m, name).unwrap().contains(want), "{name}");
    }
    // Controlling rows themselves stay live.
    for name in [
        "Clearcoat Amount",
        "Sheen",
        "Transparent Amount",
        "Match Specular",
        "Thin Film",
    ] {
        assert!(inert_reason(&m, name).is_none(), "{name}");
    }
    m.openpbr.subsurface_uses_diffuse = true;
    m.subsurface_amount = 0.5;
    assert!(
        inert_reason(&m, "Subsurface Albedo")
            .unwrap()
            .contains("Albedo from Diffuse")
    );
    // Everything on: nothing greys.
    m.clearcoat_amount = 0.5;
    m.match_specular = false;
    m.reflection_amount = 0.5;
    m.reflection_fresnel = 0.5;
    m.sheen = 0.5;
    m.openpbr.thin_film_weight = 1.;
    m.more.dispersion = 0.2;
    m.openpbr.transmission_scatter = [0.1; 3];
    m.openpbr.subsurface_uses_diffuse = false;
    m.transparent_amount = 0.5;
    m.smoothing = 1.;
    m.more.rounded_edge_width = 0.01;
    m.more.clip_enable = true;
    m.more.matte = true;
    m.more.gl_color_override = true;
    for (name, _) in CHANNELS {
        assert!(inert_reason(&m, name).is_none(), "{name}");
    }
    // Reflection feeds the coat while Match Specular is off, so the coat
    // rows stay live at Clearcoat Amount 0 then.
    m.clearcoat_amount = 0.;
    assert!(inert_reason(&m, "Clearcoat Roughness").is_none());
    m.reflection_amount = 0.;
    assert!(inert_reason(&m, "Clearcoat Roughness").is_some());
    assert!(
        inert_reason(&m, "Reflection Fresnel")
            .unwrap()
            .contains("Reflection Amount")
    );
    // UIC-93: each controlling row sits above the rows it greys, and every
    // gate names real rows.
    let order: Vec<&str> = CHANNELS.iter().map(|(n, _)| *n).collect();
    let at = |n: &str| {
        order
            .iter()
            .position(|x| *x == n)
            .unwrap_or_else(|| panic!("{n} is a Material row"))
    };
    for g in GATES {
        for c in g.controls {
            at(c);
        }
        for r in g.rows {
            assert!(at(g.controls[0]) < at(r), "{} above {r}", g.controls[0]);
        }
    }
}

#[test]
fn texture_driven_rows_stay_live() {
    let m = MaterialChannels {
        clearcoat_amount: 0.,
        match_specular: true,
        transparent_amount: 0.,
        ..Default::default()
    };
    assert!(inert_reason(&m, "Clearcoat Roughness").is_some());
    // The row's own channel is driven by a texture: live.
    let own = |r: &str| r == "Clearcoat Roughness";
    assert!(inert_reason_driven(&m, "Clearcoat Roughness", &own).is_none());
    assert!(inert_reason_driven(&m, "Clearcoat Bump", &own).is_some());
    // The controlling row is driven: every row it controls is live.
    let control = |r: &str| r == "Clearcoat Amount";
    for row in [
        "Clearcoat Roughness",
        "Clearcoat Bump",
        "Coat Color",
        "Coat IOR",
    ] {
        assert!(inert_reason_driven(&m, row, &control).is_none(), "{row}");
    }
    // OpenPBR rows too: Transparent Amount driven keeps Dispersion live.
    let t = |r: &str| r == "Transparent Amount";
    assert!(inert_reason(&m, "Dispersion").is_some());
    assert!(inert_reason_driven(&m, "Dispersion", &t).is_none());
}

#[test]
fn driven_rows_come_from_enabled_texture_layers_around_the_material() {
    use crate::shader_tree::NewLayer;
    let mut tree = ShaderTree::default();
    let group = tree.add(NewLayer::Group, None).unwrap();
    let material = tree.add(NewLayer::Material, Some(group)).unwrap();
    let texture = tree.add(NewLayer::Constant, Some(group)).unwrap();
    let set = |tree: &mut ShaderTree, effect: TextureEffect, enable: bool| {
        let n = tree.nodes.iter_mut().find(|n| n.id == texture).unwrap();
        n.layer.enable = enable;
        if let NodeKind::Texture(t) = &mut n.kind {
            t.effect = effect;
        }
    };
    set(&mut tree, TextureEffect::ClearcoatAmount, true);
    assert_eq!(driven_rows(&tree, material), ["Clearcoat Amount"]);
    set(&mut tree, TextureEffect::LuminousAmount, true);
    assert_eq!(driven_rows(&tree, material), ["Luminous Intensity"]);
    set(&mut tree, TextureEffect::ClearcoatAmount, false);
    assert!(
        driven_rows(&tree, material).is_empty(),
        "a hidden layer drives nothing"
    );
}
