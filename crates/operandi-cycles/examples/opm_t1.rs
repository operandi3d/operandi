//! OPM-T1 (OpenPBR spec): old scenes must render the same after the
//! OpenPBR channels arrive. This writes a set of test scenes with the
//! build it runs in, renders scenes to PNG with a fixed CPU setup, and
//! compares two folders of renders.
//!
//!     cargo run --release -p operandi-cycles --features cycles --example opm_t1 -- \
//!         write <scene-dir> [preset.oppreset...]
//!     ... --example opm_t1 -- render <scene-dir> <png-dir> [samples]
//!     ... --example opm_t1 -- compare <png-dir-a> <png-dir-b>
//!
//! Write the scenes with the older build, render them with both builds,
//! then compare: every picture should match to within rounding.
use glam::DVec3;
use operandi_core::shader_tree::{MaterialChannels, NewLayer, NodeKind, ShaderTreeEdit};
use operandi_core::{Command, Editor};
use operandi_cycles::{Settings, View, render, scene_meshes};
use std::path::{Path, PathBuf};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("write") if args.len() >= 2 => write(Path::new(&args[1]), &args[2..]),
        Some("render") if args.len() >= 3 => render_all(
            Path::new(&args[1]),
            Path::new(&args[2]),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(32),
        ),
        Some("compare") if args.len() == 3 => compare(Path::new(&args[1]), Path::new(&args[2])),
        _ => Err("usage: opm_t1 write|render|compare ...".into()),
    }
}

/// A sphere and a cube with UVs, both in the default material.
fn base_editor() -> Result<Editor, String> {
    let mut e = Editor::default();
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
    Ok(e)
}

fn with_material(f: impl Fn(&mut MaterialChannels)) -> Result<Editor, String> {
    let mut e = base_editor()?;
    let mut tree = e.scene.shader_tree.clone();
    for node in &mut tree.nodes {
        if let NodeKind::Material(m) = &mut node.kind {
            f(m);
        }
    }
    e.scene.shader_tree = tree;
    Ok(e)
}

fn write(dir: &Path, presets: &[String]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    type Edit = fn(&mut MaterialChannels);
    let cases: &[(&str, Edit)] = &[
        ("clay", |_| {}),
        ("coat", |m| {
            m.diffuse_color = [0.6, 0.1, 0.1];
            m.clearcoat_amount = 0.8;
            m.clearcoat_roughness = 0.1;
        }),
        ("subsurface", |m| {
            m.diffuse_color = [0.9, 0.6, 0.5];
            m.subsurface_amount = 0.7;
            m.subsurface_distance = 0.05;
        }),
        ("glass", |m| {
            m.transparent_amount = 0.9;
            m.transparent_color = [0.6, 0.9, 0.7];
            m.refractive_index = 1.5;
            m.absorption_distance = 0.3;
        }),
        ("emission", |m| {
            m.luminous_color = [1., 0.5, 0.2];
            m.luminous_intensity = 2.;
        }),
        ("metal", |m| {
            m.diffuse_color = [0.9, 0.7, 0.3];
            m.metallic = 1.;
            m.roughness = 0.25;
            m.anisotropy = 0.5;
        }),
        ("sheen", |m| {
            m.diffuse_color = [0.2, 0.2, 0.5];
            m.sheen = 1.;
        }),
        ("ior", |m| {
            m.more.use_refractive_index = true;
            m.refractive_index = 2.2;
            m.roughness = 0.15;
        }),
    ];
    for (name, edit) in cases {
        let e = with_material(edit)?;
        operandi_core::persistence::save(&dir.join(format!("{name}.opscene")), &e.scene)?;
    }
    for path in presets {
        let preset = operandi_core::persistence::load_preset(path.as_ref())?;
        let mut e = base_editor()?;
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
        let stem = Path::new(path).file_stem().unwrap().to_string_lossy();
        operandi_core::persistence::save(&dir.join(format!("preset_{stem}.opscene")), &e.scene)?;
    }
    println!("wrote scenes to {}", dir.display());
    Ok(())
}

fn scenes(dir: &Path, ext: &str) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == ext))
        .collect();
    out.sort();
    Ok(out)
}

fn render_all(dir: &Path, out: &Path, samples: u32) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    for path in scenes(dir, "opscene")? {
        let scene = operandi_core::persistence::load(&path)?;
        let meshes = scene_meshes(&scene)?;
        let (yaw, pitch): (f64, f64) = (0.35, 0.35);
        let toward = DVec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        );
        let forward = -toward;
        let right = forward.cross(DVec3::Y).normalize();
        let up = right.cross(forward);
        let view = View::perspective(toward * 3.2, right, up, forward, 1.0 / 2.6);
        let mut settings = Settings::studio(400, 210, view);
        settings.samples = samples;
        settings.gpu = false;
        settings.denoise = false;
        let image = render(&meshes, &settings)?;
        let png = out.join(format!(
            "{}.png",
            path.file_stem().unwrap().to_string_lossy()
        ));
        image::save_buffer(
            &png,
            &image.to_srgb8(),
            image.width,
            image.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
        println!("{}", png.display());
    }
    Ok(())
}

fn compare(a: &Path, b: &Path) -> Result<(), String> {
    let mut worst = 0u8;
    let mut failed = false;
    for pa in scenes(a, "png")? {
        let pb = b.join(pa.file_name().unwrap());
        let ia = image::open(&pa).map_err(|e| e.to_string())?.to_rgba8();
        let ib = image::open(&pb)
            .map_err(|e| format!("{}: {e}", pb.display()))?
            .to_rgba8();
        if ia.dimensions() != ib.dimensions() {
            return Err(format!("{}: sizes differ", pa.display()));
        }
        let mut max = 0u8;
        let mut sum = 0u64;
        for (x, y) in ia.as_raw().iter().zip(ib.as_raw()) {
            let d = x.abs_diff(*y);
            max = max.max(d);
            sum += u64::from(d);
        }
        let mean = sum as f64 / ia.as_raw().len() as f64;
        // Within rounding: no channel more than 2/255 off, and the mean
        // difference well under one step.
        let same = max <= 2 && mean < 0.05;
        failed |= !same;
        worst = worst.max(max);
        println!(
            "{} {:<28} max {max:3} mean {mean:.4}",
            if same { "same" } else { "DIFF" },
            pa.file_name().unwrap().to_string_lossy()
        );
    }
    if failed {
        Err(format!("renders differ (worst channel {worst}/255)"))
    } else {
        println!("all renders match");
        Ok(())
    }
}
