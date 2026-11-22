//! Front, Light Probe and Implicit UV texture locators in Cycles
//! (shader-tree-complete stage 1; TL-Q30..Q32).
#![cfg(feature = "cycles")]
use super::*;
use operandi_core::mesh::Mesh;
use operandi_core::shader_tree::{NewLayer, Projection, ShaderTreeEdit, TextureEffect};
use operandi_core::{Command, Editor};

/// A 16 × 1 picture whose red runs 0 → 1 left to right (linear data).
fn ramp_png() -> String {
    let dir = std::env::temp_dir().join("operandi-locator-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ramp.png");
    let px: Vec<u8> = (0..16u32)
        .flat_map(|x| [(x * 17) as u8, 0, 0, 255])
        .collect();
    image::save_buffer(&path, &px, 16, 1, image::ColorType::Rgba8).unwrap();
    path.to_string_lossy().into_owned()
}

/// A square at z = 0.5 facing +Z that glows with an Image Map (the ramp)
/// placed by a locator of `projection`, adjusted by `f`.
fn glowing(projection: Projection, f: impl FnOnce(&mut Editor, u64)) -> Scene {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let item = e.scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    let vertices = item.mesh.vertices.clone();
    item.mesh
        .faces
        .retain(|f| f.vertices.iter().all(|&v| vertices[v as usize].z > 0.4));
    if let Some(NodeKind::Material(m)) = e
        .scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.diffuse_color = [0.; 3];
        m.specular_amount = 0.;
        m.luminous_intensity = 1.;
    }
    let node = e.scene.shader_tree.next_id;
    let run = |e: &mut Editor, edit| e.dispatch(Command::ShaderTree(edit)).unwrap();
    run(
        &mut e,
        ShaderTreeEdit::Add {
            layer: NewLayer::ImageMap,
            selected: None,
        },
    );
    run(
        &mut e,
        ShaderTreeEdit::SetImage {
            node,
            image: Some((ramp_png(), 16, 1)),
        },
    );
    for n in &mut e.scene.shader_tree.nodes {
        if let NodeKind::Texture(t) = &mut n.kind {
            t.effect = TextureEffect::LuminousColor;
        }
    }
    let locator = match &e.scene.shader_tree.get(node).unwrap().kind {
        NodeKind::Texture(t) => t.locator.unwrap(),
        _ => unreachable!(),
    };
    for i in &mut e.scene.items {
        if i.id == locator
            && let Some(operandi_core::texture_locator::TextureItem::Locator(l)) = &mut i.texture
        {
            l.projection = projection;
        }
    }
    f(&mut e, locator);
    e.scene
}

/// Mean red of each of the 8 pixel columns across the middle rows; the
/// camera 5 m in front of the square, as in gradient_tests.
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

/// Each column at least as red as the one before, by `span` overall.
fn rises(c: &[f32], span: f32) -> bool {
    c.windows(2).all(|w| w[1] >= w[0] - 0.02) && c[7] - c[0] > span
}

/// The CPU colour (the same expression the viewport compiles) at world
/// point `p` on the square.
fn cpu_red(scene: &Scene, p: [f64; 3]) -> f64 {
    use operandi_core::shader_eval::{Ch, Context, ShadingPoint};
    let item = scene.items.iter().find(|i| i.is_mesh()).unwrap().id;
    let frames = scene.locator_frames(item);
    let ctx = Context {
        item,
        ancestors: &[],
        material: operandi_core::material::DEFAULT_MATERIAL,
        part: None,
        sets: &[],
        locators: &frames,
    };
    let at = ShadingPoint {
        position: p,
        uv: [0.; 2],
        normal: [0., 0., 1.],
    };
    // The ramp: red = u (nearest of 16 texels is close enough here).
    let sample = |_: &operandi_core::shader_eval::ImageLookup, uv: [f64; 2]| {
        let u = uv[0] - uv[0].floor();
        [u, 0., 0., 1.]
    };
    scene
        .shader_tree
        .evaluate(&ctx)
        .get(Ch::LuminousColor)
        .eval_at(&at, &sample)[0]
}

#[test]
fn front_projects_from_the_camera() {
    // A camera where the render's eye is, but looking wider: the square
    // (1 m) fills the middle of its picture.
    let scene = glowing(Projection::Front, |e, _| {
        e.scene
            .add_camera(
                operandi_core::camera_light::CameraSettings::default(),
                operandi_core::scene::Transform {
                    translation: DVec3::new(0., 0., 5.5),
                    ..Default::default()
                },
            )
            .unwrap();
    });
    let got = columns(&scene);
    // The square fills only the middle of the camera's wider picture.
    assert!(rises(&got, 0.05), "{got:?}");
    // Centre of the square = centre of the camera picture: u 0.5.
    let mid = (got[3] + got[4]) / 2.;
    let srgb_half = 0.5f32.powf(2.2);
    assert!((mid - srgb_half).abs() < 0.08, "{got:?}");
    assert!((cpu_red(&scene, [0., 0., 0.5]) - 0.5).abs() < 1e-6);
}

#[test]
fn light_probe_reads_the_mirror_ball() {
    // Locator 2 m behind the square's centre, axis Y: rays toward +Z hit
    // the ball near its centre, u rising with x.
    let scene = glowing(Projection::LightProbe, |e, locator| {
        let item = e.scene.item_mut(locator).unwrap();
        item.transform.translation = DVec3::new(0., 0., -1.5);
        if let Some(operandi_core::texture_locator::TextureItem::Locator(l)) = &mut item.texture {
            l.axis = 1;
        }
    });
    let got = columns(&scene);
    assert!(rises(&got, 0.05), "{got:?}");
    let cpu_mid = cpu_red(&scene, [0., 0., 0.5]);
    assert!((cpu_mid - 0.5).abs() < 1e-6, "{cpu_mid}");
}

#[test]
fn implicit_uv_spans_each_polygon() {
    let scene = glowing(Projection::ImplicitUv, |_, _| {});
    let got = columns(&scene);
    // One quad: u runs 0 → 1 across it one way or the other.
    let reversed: Vec<f32> = got.iter().rev().copied().collect();
    assert!(rises(&got, 0.3) || rises(&reversed, 0.3), "{got:?}");
}
