//! Viewport transparency by eye (VT-01, VT-02): a see-through cube in
//! front of a red one, drawn on a headless GPU and written as PNGs.
//!
//! `OPERANDI_VT_DIR=<folder> cargo test --release -p operandi-app
//! viewport_transparency_capture -- --ignored --nocapture`
use super::*;

/// Two cubes: a red one behind (at `behind`), and at the origin one whose
/// material is Diffuse Amount 0, Transparent Amount `amount` and
/// Transparent Color `tint`.
fn glass_scene(a: &mut Operandi, tint: [f64; 3], amount: f64, behind: DVec3) {
    use operandi_core::shader_tree::NodeKind;
    let cube = |a: &mut Operandi, at: DVec3, tag: &str| {
        a.editor.scene.new_mesh().unwrap();
        let mut mesh = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        for f in &mut mesh.faces {
            f.material = tag.into();
        }
        let item = a.editor.scene.items.last_mut().unwrap();
        item.mesh = mesh.into();
        item.transform.translation = at;
    };
    cube(a, behind, "Red");
    cube(a, DVec3::ZERO, "Glass");
    let tree = &mut a.editor.scene.shader_tree;
    let red = operandi_core::Material {
        color: [1., 0., 0.],
        diffuse: 0.8,
        ..Default::default()
    };
    for (tag, material) in [("Red", red), ("Glass", Default::default())] {
        tree.ensure_material(tag, &material).unwrap();
        let id = tree.material_for_tag(tag).unwrap();
        let mut node = tree.get(id).unwrap().clone();
        if let NodeKind::Material(m) = &mut node.kind {
            m.copy_from(&material);
            if tag == "Glass" {
                m.diffuse_amount = 0.;
                m.transparent_amount = amount;
                m.transparent_color = tint;
            }
        }
        tree.replace(node).unwrap();
    }
}

#[test]
#[ignore = "opt-in pictures by eye (OPERANDI_VT_DIR)"]
fn viewport_transparency_capture() {
    let Some(dir) = std::env::var_os("OPERANDI_VT_DIR").map(PathBuf::from) else {
        println!("OPERANDI_VT_DIR not set");
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), 2f32));
    // (name, tint, Transparent Amount, Show Material Transparency). The
    // half-clear and opaque ones compare shadows (VT-Q07).
    for (name, tint, amount, on) in [
        ("VT01", [1., 1., 1.], 0.98, true),
        ("VT02", [1., 0.55, 0.1], 0.98, true),
        ("VT01-off", [1., 1., 1.], 0.98, false),
        ("half", [1., 1., 1.], 0.5, true),
        ("opaque", [1., 1., 1.], 0., true),
    ] {
        let state = super::performance_tests::headless_gpu().expect("graphics device");
        let mut a = Operandi::empty();
        a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
        a.viewport_preferences.material_transparency = on;
        // Transparency shows in the Advanced style only.
        a.display.settings.active.style = crate::view_display::Style::Advanced;
        a.display.settings.inactive.style = crate::view_display::Style::Advanced;
        a.camera.view = crate::camera::View::Perspective;
        a.camera.yaw += 0.35;
        a.camera.pitch -= 0.25;
        // Behind the glass as the camera looks, half a cube to the right,
        // so part of the red cube shows through and part beside it.
        let (yaw, pitch) = (a.camera.yaw, a.camera.pitch);
        let forward = -DVec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        );
        let right = forward.cross(DVec3::Y).normalize();
        glass_scene(&mut a, tint, amount, forward * 1.8 + right * 0.5);
        a.run(Command::Mode(Mode::Item));
        a.editor.scene.select_item(None);
        a.frame(false);
        for _ in 0..3 {
            super::tests::draw(&mut a, &ctx, vec2(1200., 800.), vec![]);
        }
        let (_, [w, h], rgba) = a.viewport_gpu.as_ref().unwrap().read_views().remove(0);
        // Over the viewport's grey, as the window shows it.
        let back = [72u8, 72, 72];
        let shown: Vec<u8> = rgba
            .chunks(4)
            .flat_map(|p| {
                let alpha = f32::from(p[3]) / 255.;
                let mix = |i: usize| {
                    (f32::from(p[i]) + f32::from(back[i]) * (1. - alpha)).min(255.) as u8
                };
                [mix(0), mix(1), mix(2), 255]
            })
            .collect();
        let file = dir.join(format!("viewport-transparency-{name}.png"));
        image::RgbaImage::from_raw(w, h, shown)
            .unwrap()
            .save(&file)
            .unwrap();
        println!("VT {}", file.display());
    }
}

/// How much shadow falls on a red wall from a cube in front of it, as the
/// summed darkening of the wall's red below its lit level. Front view,
/// Advanced, no wire; the view light comes from above the line of sight, so
/// the cube's shadow lands on the wall below the cube, in plain view.
/// `None` without a graphics device (GitHub's Linux runner has none).
fn wall_shadow(amount: f64, transparency_on: bool) -> Option<f64> {
    use crate::view_display::{Overlay, Style};
    use operandi_core::shader_tree::NodeKind;
    let state = super::performance_tests::headless_gpu()?;
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.viewport_preferences.material_transparency = transparency_on;
    for group in [
        &mut a.display.settings.active,
        &mut a.display.settings.inactive,
    ] {
        group.style = Style::Advanced;
        group.overlay = Overlay::None;
    }
    let add = |a: &mut Operandi, at: DVec3, size: DVec3, tag: &str| {
        a.editor.scene.new_mesh().unwrap();
        let mut mesh = operandi_core::Mesh::box_mesh(at, size, 1).unwrap();
        for f in &mut mesh.faces {
            f.material = tag.into();
        }
        a.editor.scene.items.last_mut().unwrap().mesh = mesh.into();
    };
    add(&mut a, DVec3::ZERO, DVec3::new(4., 4., 0.02), "Red");
    add(&mut a, DVec3::new(0., 0.6, 1.), DVec3::splat(0.6), "Glass");
    let tree = &mut a.editor.scene.shader_tree;
    for tag in ["Red", "Glass"] {
        tree.ensure_material(tag, &Default::default()).unwrap();
        let id = tree.material_for_tag(tag).unwrap();
        let mut node = tree.get(id).unwrap().clone();
        if let NodeKind::Material(m) = &mut node.kind {
            if tag == "Red" {
                m.diffuse_color = [1., 0., 0.];
                m.diffuse_amount = 0.8;
            } else {
                // Opaque, it draws light grey: never mistaken for wall.
                m.diffuse_color = [1., 1., 1.];
                m.diffuse_amount = 0.8;
                m.transparent_amount = amount;
            }
        }
        tree.replace(node).unwrap();
    }
    a.run(Command::Mode(Mode::Item));
    a.editor.scene.select_item(None);
    a.camera.view = crate::camera::View::Front;
    a.frame(false);
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..3 {
        super::tests::draw(&mut a, &ctx, vec2(800., 800.), vec![]);
    }
    let (_, _, rgba) = a.viewport_gpu.as_ref().unwrap().read_views().remove(0);
    // Wall pixels: red with little green (the grey cube and the glass over
    // grey don't count).
    let reds: Vec<f64> = rgba
        .chunks(4)
        .filter(|p| p[3] > 0 && f64::from(p[1]) < 0.35 * f64::from(p[0]))
        .map(|p| f64::from(p[0]))
        .collect();
    assert!(reds.len() > 1000, "no wall drawn");
    let mut sorted = reds.clone();
    sorted.sort_by(f64::total_cmp);
    let lit = sorted[sorted.len() * 9 / 10];
    Some(reds.iter().map(|r| (lit - r).max(0.)).sum())
}

/// VT-Q07 (Jason, 2026-11-14): with Show Material Transparency on, a
/// see-through material casts a shadow lighter by how much shows through;
/// with it off, shadows stay solid. Skips without a graphics device.
#[test]
fn see_through_materials_cast_lighter_shadows() {
    let Some(opaque) = wall_shadow(0., true) else {
        println!("no graphics device; skipped");
        return;
    };
    assert!(opaque > 1000., "the opaque cube casts no shadow ({opaque})");
    let shadow = |amount, on| wall_shadow(amount, on).expect("graphics device") / opaque;
    let half = shadow(0.5, true);
    let clear = shadow(0.98, true);
    let off = shadow(0.98, false);
    println!("shadow vs opaque: half {half:.2}, clear {clear:.2}, setting off {off:.2}");
    assert!((0.3..0.7).contains(&half), "half clear: {half}");
    // (Clear glass also dims the wall seen through it a little, which
    // counts here too.)
    assert!(clear < 0.25, "clear glass: {clear}");
    assert!((0.85..1.15).contains(&off), "setting off: {off}");
}

/// The Front view picture of a glass block, turned 35° about Y, in front
/// of a wall that is red left of x = 0 and green right of it, the glass at
/// Refractive Index `ior`. `None` without a graphics device.
fn refracted_wall(ior: f64) -> Option<Vec<u8>> {
    use crate::view_display::{Overlay, Style};
    use operandi_core::shader_tree::NodeKind;
    let state = super::performance_tests::headless_gpu()?;
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.viewport_preferences.material_transparency = true;
    for group in [
        &mut a.display.settings.active,
        &mut a.display.settings.inactive,
    ] {
        group.style = Style::Advanced;
        group.overlay = Overlay::None;
    }
    let add = |a: &mut Operandi, at: DVec3, size: DVec3, tag: &str, turn: f64| {
        a.editor.scene.new_mesh().unwrap();
        let mut mesh = operandi_core::Mesh::box_mesh(DVec3::ZERO, size, 1).unwrap();
        for f in &mut mesh.faces {
            f.material = tag.into();
        }
        let item = a.editor.scene.items.last_mut().unwrap();
        item.mesh = mesh.into();
        item.transform.translation = at;
        item.transform.rotation.y = turn.to_radians();
    };
    add(
        &mut a,
        DVec3::new(-1., 0., 0.),
        DVec3::new(2., 4., 0.02),
        "Red",
        0.,
    );
    add(
        &mut a,
        DVec3::new(1., 0., 0.),
        DVec3::new(2., 4., 0.02),
        "Green",
        0.,
    );
    add(
        &mut a,
        DVec3::new(0., 0., 1.),
        DVec3::splat(1.),
        "Glass",
        35.,
    );
    let tree = &mut a.editor.scene.shader_tree;
    for tag in ["Red", "Green", "Glass"] {
        tree.ensure_material(tag, &Default::default()).unwrap();
        let id = tree.material_for_tag(tag).unwrap();
        let mut node = tree.get(id).unwrap().clone();
        if let NodeKind::Material(m) = &mut node.kind {
            match tag {
                "Red" => m.diffuse_color = [1., 0., 0.],
                "Green" => m.diffuse_color = [0., 1., 0.],
                _ => {
                    m.diffuse_amount = 0.;
                    m.transparent_amount = 1.;
                    m.more.use_refractive_index = true;
                    m.refractive_index = ior;
                }
            }
        }
        tree.replace(node).unwrap();
    }
    a.run(Command::Mode(Mode::Item));
    a.editor.scene.select_item(None);
    a.camera.view = crate::camera::View::Front;
    a.frame(false);
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..3 {
        super::tests::draw(&mut a, &ctx, vec2(600., 600.), vec![]);
    }
    let (_, _, rgba) = a.viewport_gpu.as_ref().unwrap().read_views().remove(0);
    Some(rgba)
}

/// VM-Q09: glass refracts in the Advanced viewport. At Refractive Index 1
/// the wall shows straight through; at 1.6 the red / green boundary seen
/// through the turned block moves, so many pixels change colour. Skips
/// without a graphics device.
#[test]
fn glass_refracts_what_is_behind() {
    let Some(straight) = refracted_wall(1.0) else {
        println!("no graphics device; skipped");
        return;
    };
    let bent = refracted_wall(1.6).expect("graphics device");
    let swapped = straight
        .chunks(4)
        .zip(bent.chunks(4))
        .filter(|(a, b)| {
            let red = |p: &[u8]| i32::from(p[0]) > i32::from(p[1]) + 40;
            let green = |p: &[u8]| i32::from(p[1]) > i32::from(p[0]) + 40;
            (red(a) && green(b)) || (green(a) && red(b))
        })
        .count();
    assert!(
        swapped > 500,
        "only {swapped} pixels changed between red and green"
    );
}

/// How many pixels a smooth cube covers in the Front view, Advanced, with
/// a Displacement layer of `value` on its material (Displacement Distance
/// 0.3 m). `None` without a graphics device.
fn displaced_cube_pixels(value: f64) -> Option<usize> {
    use crate::view_display::{Overlay, Style};
    use operandi_core::shader_tree::{
        Layer, Node, NodeKind, TextureEffect, TextureLayer, TextureSource,
    };
    let state = super::performance_tests::headless_gpu()?;
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    for group in [
        &mut a.display.settings.active,
        &mut a.display.settings.inactive,
    ] {
        group.style = Style::Advanced;
        group.overlay = Overlay::None;
    }
    a.editor.scene.new_mesh().unwrap();
    let mesh = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    let item = a.editor.scene.items.last_mut().unwrap();
    item.mesh = mesh.into();
    // Turned, so each face moving out along its normal shows in the
    // silhouette (seen square on, the front face would only come closer).
    item.transform.rotation = DVec3::new(0.6, 0.7, 0.);
    let tree = &mut a.editor.scene.shader_tree;
    if let Some(NodeKind::Material(m)) = tree
        .nodes
        .iter_mut()
        .find(|n| n.id == 4)
        .map(|n| &mut n.kind)
    {
        m.displacement_distance = 0.3;
    }
    let id = tree.next_id;
    tree.next_id += 1;
    tree.nodes.insert(
        3,
        Node {
            extras: Default::default(),
            id,
            parent: None,
            name: "Displacement".into(),
            layer: Layer::default(),
            kind: NodeKind::Texture(TextureLayer {
                source: TextureSource::Constant {
                    color: [1.; 3],
                    value,
                },
                effect: TextureEffect::Displacement,
                locator: None,
                legacy_projection: None,
                legacy_axis: None,
                ignore_scale_group: false,
                node_input: None,
            }),
        },
    );
    tree.validate().unwrap();
    a.run(Command::Mode(Mode::Item));
    a.editor.scene.select_item(None);
    a.camera.view = crate::camera::View::Front;
    a.frame(false);
    // Keep the same framing for every value: frame the undisplaced cube.
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..3 {
        super::tests::draw(&mut a, &ctx, vec2(600., 600.), vec![]);
    }
    let (_, _, rgba) = a.viewport_gpu.as_ref().unwrap().read_views().remove(0);
    Some(rgba.chunks(4).filter(|p| p[3] > 200).count())
}

/// VM-Q11: displacement moves the surface in the Advanced viewport. The
/// box's corners carry each face's own normal, so pushed out 0.3 m its
/// faces part (gaps between them): what it covers changes a lot.
#[test]
fn displacement_moves_the_surface_in_advanced() {
    let Some(flat) = displaced_cube_pixels(0.) else {
        println!("no graphics device; skipped");
        return;
    };
    let pushed = displaced_cube_pixels(1.).expect("graphics device");
    assert!(
        (pushed as f64 - flat as f64).abs() > flat as f64 * 0.1,
        "displaced {pushed} vs flat {flat}"
    );
}

/// The empty perspective view's pixels with Draw Faded Grid on or off.
fn grid_pixels(fade: bool) -> Option<Vec<u8>> {
    let state = super::performance_tests::headless_gpu()?;
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.display.settings.drawing.faded_grid = fade;
    a.camera.view = crate::camera::View::Perspective;
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..3 {
        super::tests::draw(&mut a, &ctx, vec2(800., 600.), vec![]);
    }
    Some(a.viewport_gpu.as_ref().unwrap().read_views().remove(0).2)
}

/// #58: Draw Faded Grid makes a visible difference in the default
/// perspective view (the fade began six focus distances away, past where
/// the grid is seen, so the switch did nothing). Skips without a
/// graphics device.
#[test]
fn faded_grid_visibly_fades_the_default_view() {
    let (Some(faded), Some(plain)) = (grid_pixels(true), grid_pixels(false)) else {
        return;
    };
    let changed = faded
        .chunks(4)
        .zip(plain.chunks(4))
        .filter(|(p, q)| p.iter().zip(q.iter()).any(|(a, b)| a.abs_diff(*b) > 2))
        .count();
    let total = faded.len() / 4;
    assert!(
        changed * 50 > total,
        "only {changed} of {total} pixels differ"
    );
}
