//! Render Outputs through real Cycles passes.
#![cfg(feature = "cycles")]
use super::*;
use operandi_core::mesh::Mesh;
use operandi_core::render_outputs::{OutputSettings, RenderOutput};
use operandi_core::shader_tree::OutputEffect;

fn output(effect: OutputEffect) -> RenderOutput {
    RenderOutput {
        id: 1,
        name: effect.layer_name().into(),
        effect,
        settings: OutputSettings::default(),
        in_group: false,
    }
}

#[test]
fn passes_reach_the_outputs() {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.items.last_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    // Not clay, so the tree's surface (Diffuse Color 0.6) renders.
    if let Some(NodeKind::Material(m)) = scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.diffuse_color = [0.6, 0.3, 0.1];
    }
    let outputs = vec![
        output(OutputEffect::Depth),
        output(OutputEffect::DiffuseColor),
        output(OutputEffect::ShadingIncidence),
        output(OutputEffect::ShadingNormal),
    ];
    let mut settings = Settings::studio(
        8,
        8,
        View::perspective(
            DVec3::new(0.0, 0.0, 5.5),
            DVec3::X,
            DVec3::Y,
            -DVec3::Z,
            0.05,
        ),
    );
    settings.samples = 4;
    settings.gpu = false;
    settings.denoise = false;
    settings.passes = output_passes(&outputs);
    let image = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
    let pics = output_pictures(&outputs, &image);
    let centre = |k: usize| {
        let p = (4 * 8 + 4) * 4;
        [pics[k].1[p], pics[k].1[p + 1], pics[k].1[p + 2]]
    };
    let depth = centre(0)[0];
    assert!((depth - 5.0).abs() < 0.1, "depth {depth}");
    let colour = centre(1);
    assert!(
        (colour[0] - 0.6).abs() < 0.05 && (colour[2] - 0.1).abs() < 0.05,
        "{colour:?}"
    );
    assert!(centre(2)[0] < 0.1, "facing the camera: {:?}", centre(2));
    assert!(centre(3)[2] > 0.9, "normal +Z: {:?}", centre(3));
}

/// RO-Q03 v2: a Diffuse Color output inside a Material Group masked to
/// tag "Left" is filled only by the left box; the same output outside any
/// group sees both boxes.
#[test]
fn outputs_in_a_material_group_fill_only_its_surfaces() {
    use operandi_core::shader_tree::{NewLayer, TagMask};
    let mut scene = Scene::default();
    for (x, tag) in [(-0.6, "Left"), (0.6, "Right")] {
        scene.new_mesh().unwrap();
        let mut mesh = Mesh::box_mesh(DVec3::new(x, 0., 0.), DVec3::splat(1.), 1).unwrap();
        for f in &mut mesh.faces {
            f.material = tag.into();
        }
        scene.items.last_mut().unwrap().mesh = mesh.into();
    }
    if let Some(NodeKind::Material(m)) = scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.diffuse_color = [0.6, 0.6, 0.6];
    }
    let tree = &mut scene.shader_tree;
    let group = tree.add(NewLayer::Group, None).unwrap();
    if let Some(NodeKind::Group(g)) = tree
        .nodes
        .iter_mut()
        .find(|n| n.id == group)
        .map(|n| &mut n.kind)
    {
        g.tag = TagMask::Name("Left".into());
    }
    let inside = tree
        .add(
            NewLayer::RenderOutput(OutputEffect::DiffuseColor),
            Some(group),
        )
        .unwrap();
    if tree.get(inside).unwrap().parent != Some(group) {
        tree.move_to(
            &[inside],
            operandi_core::shader_tree::Drop {
                parent: Some(group),
                index: 0,
            },
        )
        .unwrap();
    }
    tree.add(NewLayer::RenderOutput(OutputEffect::DiffuseColor), None)
        .unwrap();
    let outputs: Vec<RenderOutput> = tree
        .render_outputs()
        .into_iter()
        .filter(|o| o.effect == OutputEffect::DiffuseColor)
        .collect();
    assert_eq!(outputs.iter().filter(|o| o.in_group).count(), 1);
    let mut settings = Settings::studio(
        16,
        8,
        View::perspective(DVec3::new(0., 0., 6.), DVec3::X, DVec3::Y, -DVec3::Z, 0.12),
    );
    settings.samples = 4;
    settings.gpu = false;
    settings.denoise = false;
    settings.passes = output_passes(&outputs);
    let image = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
    let pics = output_pictures(&outputs, &image);
    let at = |k: usize, x: usize| pics[k].1[(4 * 16 + x) * 4];
    let (grouped, all) = if outputs[0].in_group { (0, 1) } else { (1, 0) };
    // Left box at x ≈ 4, right box at x ≈ 11.
    assert!(at(all, 4) > 0.3 && at(all, 11) > 0.3, "ungrouped sees both");
    assert!(at(grouped, 4) > 0.3, "the group's box: {}", at(grouped, 4));
    assert!(at(grouped, 11) < 0.02, "the other box: {}", at(grouped, 11));
}

/// Material-value outputs: `effects` added to the scene's tree (the
/// surfaces read the wanted values from there), rendered 8×8 at the box.
fn material_value_pictures(
    scene: &mut Scene,
    effects: &[OutputEffect],
) -> Vec<(OutputEffect, [f32; 3])> {
    for &e in effects {
        scene
            .shader_tree
            .add(operandi_core::shader_tree::NewLayer::RenderOutput(e), None)
            .unwrap();
    }
    let outputs: Vec<RenderOutput> = scene
        .shading_tree()
        .render_outputs()
        .into_iter()
        .filter(|o| effects.contains(&o.effect))
        .collect();
    assert_eq!(outputs.len(), effects.len());
    let mut settings = Settings::studio(
        8,
        8,
        View::perspective(
            DVec3::new(0.0, 0.0, 5.5),
            DVec3::X,
            DVec3::Y,
            -DVec3::Z,
            0.05,
        ),
    );
    settings.samples = 4;
    settings.gpu = false;
    settings.denoise = false;
    settings.passes = output_passes(&outputs);
    settings.values = output_values(&outputs);
    let image = render(&scene_meshes(scene).unwrap(), &settings).unwrap();
    let pics = output_pictures(&outputs, &image);
    let p = (4 * 8 + 4) * 4;
    outputs
        .iter()
        .zip(pics)
        .map(|(o, (_, rgba))| (o.effect, [rgba[p], rgba[p + 1], rgba[p + 2]]))
        .collect()
}

fn boxed_scene() -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.items.last_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    scene
}

fn near3(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < tol)
}

#[test]
fn material_values_reach_their_outputs() {
    let mut scene = boxed_scene();
    if let Some(NodeKind::Material(m)) = scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.diffuse_color = [0.6, 0.3, 0.1];
        m.diffuse_amount = 0.37;
        m.roughness = 0.42;
        m.transparent_color = [0.2, 0.5, 0.8];
        m.transparent_amount = 0.;
        m.subsurface_amount = 0.;
    }
    let got = material_value_pictures(
        &mut scene,
        &[
            OutputEffect::DiffuseAmount,
            OutputEffect::Roughness,
            OutputEffect::TransparentColor,
            OutputEffect::DiffuseCoefficient,
        ],
    );
    let of = |e: OutputEffect| got.iter().find(|(f, _)| *f == e).unwrap().1;
    let amount = of(OutputEffect::DiffuseAmount);
    assert!(near3(amount, [0.37; 3], 0.02), "diffuse amount {amount:?}");
    let rough = of(OutputEffect::Roughness);
    assert!(near3(rough, [0.42; 3], 0.02), "roughness {rough:?}");
    let tc = of(OutputEffect::TransparentColor);
    assert!(
        near3(tc, [0.2, 0.5, 0.8], 0.02),
        "transparent colour {tc:?}"
    );
    let dc = of(OutputEffect::DiffuseCoefficient);
    assert!(
        near3(dc, [0.6 * 0.37, 0.3 * 0.37, 0.1 * 0.37], 0.02),
        "diffuse coefficient {dc:?}"
    );
}

/// MV-Q07: an untouched scene would render as the studio clay, whose
/// shader writes no values; with the output it writes the default
/// material's Diffuse Amount instead of 0.
#[test]
fn a_default_scene_writes_the_default_material_value() {
    let mut scene = boxed_scene();
    let expected = operandi_core::shader_tree::ShaderTree::default()
        .nodes
        .iter()
        .find_map(|n| match &n.kind {
            NodeKind::Material(m) => Some(m.diffuse_amount as f32),
            _ => None,
        })
        .unwrap();
    assert!(expected > 0.05);
    let got = material_value_pictures(&mut scene, &[OutputEffect::DiffuseAmount]);
    assert!(
        near3(got[0].1, [expected; 3], 0.02),
        "{:?} vs {expected}",
        got[0].1
    );
}

/// MV-Q06: dPdu is a world-space unit direction along U, perpendicular
/// to the normal; on the front face u grows with x.
#[test]
fn dpdu_is_a_unit_tangent_along_u() {
    let mut scene = boxed_scene();
    let mesh = &mut *scene.items.last_mut().unwrap().mesh;
    let uvs = mesh
        .faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|&v| {
                    let p = mesh.vertices[v as usize];
                    Some(glam::DVec2::new(p.x + 0.5, p.y + 0.5))
                })
                .collect()
        })
        .collect();
    mesh.uv_maps.insert("Texture".into(), uvs);
    let got = material_value_pictures(&mut scene, &[OutputEffect::Dpdu, OutputEffect::Dpdv]);
    let of = |e: OutputEffect| got.iter().find(|(f, _)| *f == e).unwrap().1;
    let (u, v) = (of(OutputEffect::Dpdu), of(OutputEffect::Dpdv));
    let len = |a: [f32; 3]| (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    assert!((len(u) - 1.0).abs() < 0.05, "dPdu {u:?}");
    assert!(u[2].abs() < 0.05, "dPdu ⟂ normal +Z: {u:?}");
    assert!(u[0] > 0.9, "u grows with x: {u:?}");
    assert!((len(v) - 1.0).abs() < 0.05, "dPdv {v:?}");
    assert!(v[2].abs() < 0.05 && v[1] > 0.9, "dPdv = N × T = +Y: {v:?}");
}

/// Cycles built-in outputs: two boxes ("Left" with UVs, "Right" in another
/// material) give light without colour, their UVs, two segment colours and
/// two Cryptomatte ids whose manifest names are the item names.
#[test]
fn illumination_uv_segment_and_cryptomatte_render() {
    use operandi_core::render_outputs::{
        CryptomatteType, hash_to_float, murmur3_32, segment_color,
    };
    let mut scene = Scene::default();
    let mut ids = Vec::new();
    for (x, name, tag) in [(-0.6, "Left", "Default"), (0.6, "Right", "Metal")] {
        scene.new_mesh().unwrap();
        let mut mesh = Mesh::box_mesh(DVec3::new(x, 0., 0.), DVec3::splat(1.), 1).unwrap();
        for f in &mut mesh.faces {
            f.material = tag.into();
        }
        if name == "Left" {
            let uv: Vec<Vec<Option<glam::DVec2>>> = mesh
                .faces
                .iter()
                .map(|f| vec![Some(glam::DVec2::new(0.25, 0.75)); f.vertices.len()])
                .collect();
            mesh.uv_maps.insert("Texture".into(), uv);
        }
        let item = scene.items.last_mut().unwrap();
        item.name = name.into();
        item.mesh = mesh.into();
        ids.push(item.id);
    }
    let crypto = |kind: CryptomatteType| {
        let mut o = output(OutputEffect::Cryptomatte);
        o.settings.cryptomatte_type = kind;
        o
    };
    let outputs = vec![
        output(OutputEffect::IlluminationTotal),
        output(OutputEffect::UvCoordinates),
        output(OutputEffect::SegmentId),
        crypto(CryptomatteType::Object),
        crypto(CryptomatteType::Material),
        output(OutputEffect::DiffuseColor),
    ];
    let mut settings = Settings::studio(
        16,
        8,
        View::perspective(DVec3::new(0., 0., 6.), DVec3::X, DVec3::Y, -DVec3::Z, 0.12),
    );
    settings.samples = 4;
    settings.gpu = false;
    settings.denoise = false;
    settings.passes = output_passes(&outputs);
    // A UV Coordinates output in the tree asks every mesh for its UVs.
    scene
        .shader_tree
        .add(
            operandi_core::shader_tree::NewLayer::RenderOutput(OutputEffect::UvCoordinates),
            None,
        )
        .unwrap();
    let meshes = scene_meshes(&scene).unwrap();
    assert!(meshes.iter().all(|m| m.uvs.len() == m.triangles.len() * 3));
    assert_eq!(meshes[0].name, "Left");
    assert_eq!(meshes[1].material_name, "Metal");
    let image = render(&meshes, &settings).unwrap();
    let pics = output_pictures(&outputs, &image);
    let at = |k: usize, x: usize| {
        let p = (4 * 16 + x) * 4;
        [pics[k].1[p], pics[k].1[p + 1], pics[k].1[p + 2]]
    };
    // Left box at x ≈ 4, right box at x ≈ 11.
    let light = at(0, 4);
    let albedo = at(5, 4);
    assert!(light[0] > 0.05, "light arrives: {light:?}");
    assert!(
        (light[0] - light[2]).abs() < 0.2 * light[0].max(1e-3),
        "white light, no surface colour: {light:?} (albedo {albedo:?})"
    );
    let uv = at(1, 4);
    assert!(
        (uv[0] - 0.25).abs() < 0.02 && (uv[1] - 0.75).abs() < 0.02 && uv[2].abs() < 0.02,
        "{uv:?}"
    );
    let (sl, sr) = (at(2, 4), at(2, 11));
    let want = segment_color(ids[0], "Default");
    assert!(
        sl.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-3),
        "{sl:?} vs {want:?}"
    );
    assert!(sl != sr, "two segments");
    // Cryptomatte: the top-ranked id at each box is its item's name hash.
    let top = |pass: Pass, x: usize| {
        let v = &image.passes.iter().find(|(p, _)| *p == pass).unwrap().1;
        let p = (4 * 16 + x) * 4;
        (v[p], v[p + 1])
    };
    let (left, cover) = top(Pass::CryptoObject0, 4);
    assert_eq!(left, hash_to_float(murmur3_32(b"Left")));
    assert!(cover > 0.9, "coverage {cover}");
    assert_eq!(
        top(Pass::CryptoObject0, 11).0,
        hash_to_float(murmur3_32(b"Right"))
    );
    assert_eq!(
        top(Pass::CryptoMaterial0, 11).0,
        hash_to_float(murmur3_32(b"Metal"))
    );
    let layers = cryptomatte_layers(&outputs, &image);
    assert_eq!(layers.len(), 2);
    let objects: serde_json::Value = serde_json::from_str(&layers[0].manifest).unwrap();
    assert_eq!(
        objects["Left"],
        format!("{:08x}", murmur3_32(b"Left")),
        "{}",
        layers[0].manifest
    );
    assert!(objects.get("Right").is_some());
    let materials: serde_json::Value = serde_json::from_str(&layers[1].manifest).unwrap();
    assert!(materials.get("Metal").is_some(), "{}", layers[1].manifest);
    // The preview shows each box in its id's colour.
    assert_eq!(
        at(3, 4),
        operandi_core::render_outputs::id_color(left),
        "preview"
    );
}
