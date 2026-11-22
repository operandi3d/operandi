//! Renders a material preset on a UV sphere and a cube with Cycles, for
//! checking converted materials (starter pack acceptance).
//!
//!     cargo run --release -p operandi-cycles --features cycles \
//!         --example render_preset -- material.oppreset out.png [samples]
//!
//! A `.mtlx` file renders its MaterialX material (OPM-T2).
use glam::DVec3;
use operandi_core::shader_tree::{NewLayer, NodeKind, ShaderTreeEdit};
use operandi_core::{Command, Editor};
use operandi_cycles::{Settings, View, render, scene_meshes};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(input), Some(output)) = (args.get(1), args.get(2)) else {
        return Err("usage: render_preset <material.oppreset> <out.png> [samples]".into());
    };
    let samples = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(64);
    // A MaterialX file loads as the Material preset its import gives.
    let path = std::path::Path::new(input);
    let preset = if path.extension().is_some_and(|e| e == "mtlx") {
        let (preset, notes) = operandi_core::materialx::material_preset(path, None, &|p| {
            image::image_dimensions(p).ok()
        })?;
        for n in notes {
            println!("note: {n}");
        }
        preset
    } else {
        operandi_core::persistence::load_preset(path)?
    };

    let mut e = Editor::default();
    // A UV sphere and a UV cube side by side, both in the default material.
    let sphere = operandi_core::ellipsoid::ellipsoid_geometry(
        &operandi_core::ellipsoid::EllipsoidParameters {
            position: DVec3::new(-0.6, 0., 0.),
            radius: DVec3::splat(0.5),
            sides: 48,
            segments: 24,
            make_uvs: true,
            ..Default::default()
        },
    )?;
    e.scene.new_mesh()?;
    e.scene.active_mut().ok_or("no mesh")?.mesh = sphere.into();
    let mut cube =
        operandi_core::mesh::Mesh::box_mesh(DVec3::new(0.6, 0., 0.), DVec3::splat(0.8), 1)?;
    operandi_core::add_cube_uvs(&mut cube, DVec3::new(0.6, 0., 0.), DVec3::splat(0.8))?;
    e.scene.new_mesh()?;
    e.scene.active_mut().ok_or("no mesh")?.mesh = cube.into();

    // A material group to load the preset into (PB-Q3: one is needed).
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Group,
        selected: None,
    }))?;
    let group = e
        .scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Group(_)))
        .map(|n| n.id);
    e.dispatch(Command::LoadPreset {
        preset: Box::new(preset),
        group,
    })?;
    let meshes = scene_meshes(&e.scene)?;

    let target = DVec3::ZERO;
    let (yaw, pitch): (f64, f64) = (0.35, 0.35);
    let toward = DVec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    );
    let forward = -toward;
    let right = forward.cross(DVec3::Y).normalize();
    let up = right.cross(forward);
    let view = View::perspective(target + toward * 3.2, right, up, forward, 1.0 / 2.6);
    let mut settings = Settings::studio(800, 420, view);
    settings.samples = samples;
    let image = render(&meshes, &settings)?;
    let rgba = image.to_srgb8();
    image::save_buffer(
        output,
        &rgba,
        image.width,
        image.height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| e.to_string())?;
    println!("wrote {output}");
    Ok(())
}
