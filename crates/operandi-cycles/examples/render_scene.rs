//! Renders an .opscene file with Cycles from Operandi's default viewpoint.
//!
//!     cargo run --release -p operandi-cycles --features cycles \
//!         --example render_scene -- scene.opscene out.png [samples] [--cpu] [--no-denoise]
use glam::DVec3;
use operandi_cycles::{Settings, View, render, scene_meshes};
use std::time::Instant;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(input), Some(output)) = (args.get(1), args.get(2)) else {
        return Err("usage: render_scene <scene.opscene> <out.png> [samples]".into());
    };
    let samples = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(16);
    let cpu = args.iter().any(|a| a == "--cpu");
    let no_denoise = args.iter().any(|a| a == "--no-denoise");
    let scene = operandi_core::persistence::load(input.as_ref())?;
    let meshes = scene_meshes(&scene)?;

    // Operandi's default perspective view (camera.rs), aimed at the scene's
    // centre and backed off to fit it.
    let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for p in meshes.iter().flat_map(|m| m.world_positions()) {
        let p = DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64);
        lo = lo.min(p);
        hi = hi.max(p);
    }
    let target = if lo.x <= hi.x {
        (lo + hi) * 0.5
    } else {
        DVec3::ZERO
    };
    let radius = if lo.x <= hi.x {
        (hi - lo).length() * 0.5
    } else {
        1.0
    };
    let (yaw, pitch): (f64, f64) = (0.65, 0.48);
    let toward = DVec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    );
    let forward = -toward;
    let right = forward.cross(DVec3::Y).normalize();
    let up = right.cross(forward);
    let half_height_tan: f64 = 1.0 / 1.8;
    let distance = radius / half_height_tan.atan().sin() * 1.1;
    let view = View::perspective(
        target + toward * distance,
        right,
        up,
        forward,
        half_height_tan,
    );
    let mut settings = Settings::studio(960, 540, view);
    settings.samples = samples;
    settings.gpu = !cpu;
    settings.denoise = !no_denoise;

    let triangles: usize = meshes.iter().map(|m| m.triangles.len()).sum();
    println!(
        "Cycles {} · {} meshes, {} triangles, {} samples",
        operandi_cycles::cycles_version().unwrap_or_default(),
        meshes.len(),
        triangles,
        samples
    );
    let start = Instant::now();
    let image = render(&meshes, &settings)?;
    println!(
        "Rendered in {:.2}s on {}{}",
        start.elapsed().as_secs_f64(),
        image.device,
        if image.denoised { ", denoised" } else { "" }
    );
    image::save_buffer(
        output,
        &image.to_srgb8(),
        image.width,
        image.height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| e.to_string())?;
    println!("Saved {output}");
    Ok(())
}
