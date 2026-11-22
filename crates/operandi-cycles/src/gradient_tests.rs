//! Shader tree remaining, stage 2 in Cycles: Gradient inputs and ramps.
#![cfg(feature = "cycles")]
use super::*;
use operandi_core::gradient::{Curve, Gradient, GradientInput, Key, KeyShape};
use operandi_core::mesh::Mesh;
use operandi_core::shader_tree::{Layer, Node, TextureEffect, TextureLayer, TextureSource};

/// A single square at z = 0.5 facing the camera that glows with its
/// Luminous Color (intensity 1) and reflects nothing; a Gradient drives
/// the Luminous Color.
fn glowing(input: GradientInput, from: [f64; 3], to: [f64; 3], span: (f64, f64)) -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    let vertices = item.mesh.vertices.clone();
    item.mesh
        .faces
        .retain(|f| f.vertices.iter().all(|&v| vertices[v as usize].z > 0.4));
    let tree = &mut scene.shader_tree;
    if let Some(NodeKind::Material(m)) = tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.diffuse_color = [0.; 3];
        m.specular_amount = 0.;
        m.luminous_intensity = 1.;
    }
    let key = |at: f64, c: [f64; 3]| Key {
        at,
        out: [c[0], c[1], c[2], 1.],
        shape: KeyShape::Linear,
    };
    let id = tree.next_id;
    tree.next_id += 1;
    tree.nodes.insert(
        3,
        Node {
            extras: Default::default(),
            id,
            parent: None,
            name: "Gradient".into(),
            layer: Layer::default(),
            kind: NodeKind::Texture(TextureLayer {
                source: TextureSource::Gradient(Box::new(Gradient {
                    input,
                    color: Curve {
                        keys: vec![key(span.0, from), key(span.1, to)],
                    },
                    ..Default::default()
                })),
                effect: TextureEffect::LuminousColor,
                locator: None,
                legacy_projection: None,
                legacy_axis: None,
                ignore_scale_group: false,
                node_input: None,
            }),
        },
    );
    tree.validate().unwrap();
    scene
}

/// The picture's centre, the camera 5 m in front of the square.
fn centre(scene: &Scene) -> [f32; 3] {
    let eye = DVec3::new(0.0, 0.0, 5.5);
    let mut settings = Settings::studio(
        8,
        8,
        View::perspective(eye, DVec3::X, DVec3::Y, -DVec3::Z, 0.05),
    );
    settings.sun_strength = 0.0;
    settings.world[0].intensity = 0.0;
    settings.samples = 16;
    settings.gpu = false;
    settings.denoise = false;
    let img = render(&scene_meshes(scene).unwrap(), &settings).unwrap();
    let mut sum = [0.0f32; 3];
    for y in 3..5 {
        for x in 3..5 {
            for (k, s) in sum.iter_mut().enumerate() {
                *s += img.rgba[(y * 8 + x) * 4 + k] / 4.0;
            }
        }
    }
    sum
}

fn near(got: [f32; 3], want: [f32; 3]) -> bool {
    (0..3).all(|k| (got[k] - want[k]).abs() < 0.04)
}

#[test]
fn gradient_inputs_render_through_the_ramp() {
    // Distance to Camera: 5 m on a 0..10 m ramp from black to white.
    let got = centre(&glowing(
        GradientInput::DistanceToCamera,
        [0.; 3],
        [1.; 3],
        (0., 10.),
    ));
    assert!(near(got, [0.5; 3]), "distance {got:?}");
    // Incidence: the square faces the camera (0°): the first key's red.
    let got = centre(&glowing(
        GradientInput::Incidence,
        [1., 0., 0.],
        [0., 0., 1.],
        (0., 90.),
    ));
    assert!(near(got, [1., 0., 0.]), "incidence {got:?}");
    // Slope: a wall facing +Z is 90° from up: halfway on 0..180°.
    let got = centre(&glowing(GradientInput::Slope, [0.; 3], [1.; 3], (0., 180.)));
    assert!(near(got, [0.5; 3]), "slope {got:?}");
    // Back Facing: the front side reads 0.
    let got = centre(&glowing(
        GradientInput::BackFacing,
        [0., 1., 0.],
        [1., 0., 0.],
        (0., 1.),
    ));
    assert!(near(got, [0., 1., 0.]), "back facing {got:?}");
}

/// Mean red of each of the 8 pixel columns across the picture's middle
/// rows, the camera 5 m in front of the square.
fn columns(scene: &Scene) -> Vec<f32> {
    let eye = DVec3::new(0.0, 0.0, 5.5);
    let mut settings = Settings::studio(
        8,
        8,
        View::perspective(eye, DVec3::X, DVec3::Y, -DVec3::Z, 0.05),
    );
    settings.sun_strength = 0.0;
    settings.world[0].intensity = 0.0;
    settings.samples = 16;
    settings.gpu = false;
    settings.denoise = false;
    let img = render(&scene_meshes(scene).unwrap(), &settings).unwrap();
    (0..8)
        .map(|x| (3..5).map(|y| img.rgba[(y * 8 + x) * 4] / 2.0).sum())
        .collect()
}

/// Gives the glowing square a "Texture" UV map: u = x + 0.5, v = y + 0.5.
fn with_texture_uvs(scene: &mut Scene) {
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
}

#[test]
fn texture_u_reads_a_primitive_uv_map() {
    // Primitives keep their UVs in the "Texture" map, not the legacy
    // per-polygon UVs; Texture U must read 0 → 1 left to right.
    let mut scene = glowing(GradientInput::TextureU, [0.; 3], [1.; 3], (0., 1.));
    with_texture_uvs(&mut scene);
    let got = columns(&scene);
    // The picture spans u 0.25..0.75.
    assert!((got[0] - 0.28).abs() < 0.06, "left {got:?}");
    assert!((got[7] - 0.72).abs() < 0.06, "right {got:?}");
}

#[test]
fn texture_u_follows_the_layers_uv_map_projection() {
    use operandi_core::shader_tree::Projection;
    use operandi_core::texture_locator::TextureLocator;
    let mut scene = glowing(GradientInput::TextureU, [0.; 3], [1.; 3], (0., 1.));
    with_texture_uvs(&mut scene);
    // "Zed" (after "Texture" by name) runs U right to left.
    let mesh = &mut *scene.items.last_mut().unwrap().mesh;
    let mut second = mesh.uv_maps["Texture"].clone();
    for uv in second.iter_mut().flatten().flatten() {
        uv.x = 1. - uv.x;
    }
    mesh.uv_maps.insert("Zed".into(), second);
    let mut next = scene.next_id;
    let mut settings = TextureLocator::new(Projection::UvMap);
    settings.uv_map = Some("Zed".into());
    let locator = scene
        .add_texture_locator(&mut next, "Gradient", settings)
        .unwrap();
    scene.next_id = next;
    for n in &mut scene.shader_tree.nodes {
        if let NodeKind::Texture(t) = &mut n.kind {
            t.locator = Some(locator);
        }
    }
    let got = columns(&scene);
    assert!(got[0] > 0.6 && got[7] < 0.4, "named map {got:?}");
    // Any other projection reads the first map by name, "Texture".
    for item in &mut scene.items {
        if let Some(operandi_core::texture_locator::TextureItem::Locator(l)) = &mut item.texture {
            l.projection = Projection::Solid;
        }
    }
    let got = columns(&scene);
    assert!(got[0] < 0.4 && got[7] > 0.6, "first map {got:?}");
}

#[test]
fn texture_u_without_uvs_reads_zero() {
    let mut scene = glowing(GradientInput::TextureU, [0.2; 3], [1.; 3], (0., 1.));
    let got = columns(&scene);
    assert!(got.iter().all(|v| (v - 0.2).abs() < 0.04), "{got:?}");
    // The render notes say why.
    assert_eq!(shading_notes(&scene).len(), 1);
    with_texture_uvs(&mut scene);
    assert!(shading_notes(&scene).is_empty());
}

/// The red channel of a `w` × `h` picture (top row first), the camera 5 m
/// in front of the square, which fills the frame.
fn picture(scene: &Scene, w: usize, h: usize) -> Vec<f32> {
    let eye = DVec3::new(0.0, 0.0, 5.5);
    let mut settings = Settings::studio(
        w as u32,
        h as u32,
        View::perspective(eye, DVec3::X, DVec3::Y, -DVec3::Z, 0.04),
    );
    settings.sun_strength = 0.0;
    settings.world[0].intensity = 0.0;
    settings.samples = 16;
    settings.gpu = false;
    settings.denoise = false;
    let img = render(&scene_meshes(scene).unwrap(), &settings).unwrap();
    (0..w * h).map(|i| img.rgba[i * 4]).collect()
}

#[test]
fn screen_inputs_run_across_the_frame() {
    let (w, h) = (16, 8);
    // Screen X: 0 at the left edge, 1 at the right, at pixel centres.
    let got = picture(
        &glowing(GradientInput::ScreenX, [0.; 3], [1.; 3], (0., 1.)),
        w,
        h,
    );
    for x in [0, 5, 15] {
        let want = (x as f32 + 0.5) / w as f32;
        assert!(
            (got[3 * w + x] - want).abs() < 0.04,
            "screen x {x}: {got:?}"
        );
    }
    // Screen Y: 0 at the bottom edge, 1 at the top.
    let got = picture(
        &glowing(GradientInput::ScreenY, [0.; 3], [1.; 3], (0., 1.)),
        w,
        h,
    );
    for y in [0, 3, 7] {
        let want = 1. - (y as f32 + 0.5) / h as f32;
        assert!(
            (got[y * w + 4] - want).abs() < 0.04,
            "screen y {y}: {got:?}"
        );
    }
    // Screen R: pixels from the centre over half the diagonal.
    let got = picture(
        &glowing(GradientInput::ScreenR, [0.; 3], [1.; 3], (0., 1.)),
        w,
        h,
    );
    let half_diagonal = ((w * w + h * h) as f32).sqrt() / 2.;
    for (x, y) in [(0, 0), (8, 4), (12, 1), (15, 7)] {
        let dx = x as f32 + 0.5 - w as f32 / 2.;
        let dy = y as f32 + 0.5 - h as f32 / 2.;
        let want = (dx * dx + dy * dy).sqrt() / half_diagonal;
        assert!(
            (got[y * w + x] - want).abs() < 0.04,
            "screen r ({x}, {y}) want {want}: {got:?}"
        );
    }
}

/// GR-Q10: Locator Incidence is the angle between the normal and the
/// direction to the locator (the world origin without one): the square
/// faces +Z with the origin behind it, so 180°.
#[test]
fn locator_incidence_renders_the_angle_to_the_locator() {
    let got = centre(&glowing(
        GradientInput::LocatorIncidence,
        [0.; 3],
        [1.; 3],
        (0., 180.),
    ));
    assert!(near(got, [1.; 3]), "{got:?}");
}
