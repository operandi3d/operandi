//! Shader tree remaining, stage 5 in Cycles: Rounded Edge, absorption and
//! subsurface anisotropy reach the compiled surface and render.
use super::*;
use operandi_core::mesh::Mesh;

fn glass_box() -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.items.last_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    if let Some(NodeKind::Material(m)) = scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.transparent_amount = 1.;
        m.refractive_index = 1.5;
        m.transparent_color = [0.2, 0.8, 1.];
        m.absorption_distance = 0.5;
        m.more.rounded_edge_width = 0.05;
        m.more.subsurface_phase = 0.9;
    }
    scene
}

#[test]
fn extras_compile_into_the_surface() {
    let meshes = scene_meshes(&glass_box()).unwrap();
    let sh = meshes[0].shader.as_ref().unwrap();
    assert!(sh.is_valid());
    assert_eq!(sh.rounded_edge, 0.05);
    assert!(sh.absorption.1 > 0.0);
    assert!(sh.inputs[Input::SubsurfaceAnisotropy as usize] >= 0);
}

#[cfg(feature = "cycles")]
#[test]
fn extras_render() {
    let mut settings = Settings::studio(
        16,
        16,
        View::perspective(
            DVec3::new(2.0, 1.5, 3.0),
            DVec3::new(0.83, 0.0, -0.55),
            DVec3::Y,
            DVec3::new(-0.5, -0.38, -0.78).normalize(),
            0.6,
        ),
    );
    settings.samples = 4;
    settings.gpu = false;
    settings.denoise = false;
    let image = render(&scene_meshes(&glass_box()).unwrap(), &settings).unwrap();
    assert!(image.rgba.iter().all(|v| v.is_finite()));
}

/// A slab `thickness` metres deep along Z (1 m × 1 m across), its
/// material fully see-through at index `ior` with Transparent Color
/// `tint` and Absorption Distance `distance`.
fn slab(thickness: f64, tint: [f64; 3], ior: f64, distance: f64) -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.items.last_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::ZERO, DVec3::new(1.0, 1.0, thickness), 1)
            .unwrap()
            .into();
    if let Some(NodeKind::Material(m)) = scene
        .shader_tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.transparent_amount = 1.;
        m.refractive_index = ior;
        m.transparent_color = tint;
        m.absorption_distance = distance;
    }
    scene
}

/// Absorption Distance above 0 moves the tint into the inside: the
/// surface passes light white and a volume absorbs, for index 1 too.
#[test]
fn absorption_tints_the_inside_not_the_surface() {
    for ior in [1.0, 1.5] {
        let meshes = scene_meshes(&slab(0.01, [0.5; 3], ior, 0.02)).unwrap();
        let sh = meshes[0].shader.as_ref().unwrap();
        let (color, density) = sh.absorption;
        // Cycles' Volume Absorption keeps exp(-density (1 - colour) x).
        let kept = (-density * (1.0 - color[0]) * 0.02).exp();
        assert!((kept - 0.5).abs() < 1e-4, "ior {ior}: kept {kept}");
        let t = sh.transparency_color;
        assert!(t >= 0 && sh.ops[t as usize].value[0] == 1.0, "ior {ior}");
    }
    // At 0 the surface tints and there is no inside. Render fix F4: each
    // of a solid's two crossings passes the square root, so the solid
    // passes the colour once.
    let meshes = scene_meshes(&slab(0.01, [0.5; 3], 1.0, 0.0)).unwrap();
    let sh = meshes[0].shader.as_ref().unwrap();
    assert_eq!(sh.absorption.1, 0.0);
    let t = sh.ops[sh.transparency_color as usize].value[0];
    assert!((t - 0.5f32.sqrt()).abs() < 1e-6, "{t}");
}

/// AD-01..03: the light a slab passes, seen straight through against a
/// white world.
#[cfg(feature = "cycles")]
#[test]
fn absorption_follows_beers_law() {
    let passed = |scene: Scene| {
        let mut view = View::perspective(
            DVec3::new(0.0, 0.0, 3.0),
            DVec3::X,
            DVec3::Y,
            -DVec3::Z,
            0.1,
        );
        view.orthographic = Some(0.2);
        let mut settings = Settings::studio(4, 4, view);
        settings.world = vec![EnvironmentLayer::flat([1.0; 3], 1.0)];
        settings.lights = Some(Vec::new());
        settings.sun_strength = 0.0;
        settings.samples = 4;
        settings.gpu = false;
        settings.denoise = false;
        let image = render(&scene_meshes(&scene).unwrap(), &settings).unwrap();
        let n = (image.rgba.len() / 4) as f32;
        image.rgba.chunks(4).map(|p| p[0]).sum::<f32>() / n
    };
    let close = |got: f32, want: f32| assert!((got - want).abs() < 0.02, "{got} vs {want}");
    // AD-01
    close(passed(slab(0.01, [0.5; 3], 1.0, 0.02)), 0.5f32.sqrt());
    close(passed(slab(0.04, [0.5; 3], 1.0, 0.02)), 0.25);
    // AD-02 (render fix F4): at 0 the tint doesn't depend on thickness,
    // and a closed slab (in and out) passes the colour once.
    close(passed(slab(0.01, [0.5; 3], 1.0, 0.0)), 0.5);
    close(passed(slab(0.04, [0.5; 3], 1.0, 0.0)), 0.5);
    // AD-03
    close(passed(slab(0.01, [1.0; 3], 1.0, 0.02)), 1.0);
    close(passed(slab(0.04, [1.0; 3], 1.0, 0.02)), 1.0);
}
