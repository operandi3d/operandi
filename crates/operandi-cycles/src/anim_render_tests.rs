//! TK-53 / TL-70: Cycles renders a frame's pose. The bouncing ball (the
//! BEHAVIOR-TIMELINE section 10 keys) rendered at frames 0..24; heavy, so
//! opt-in:
//! `OPERANDI_BALL_CHECK=<folder> cargo test --locked --workspace --features
//! operandi-app/cycles -- --ignored ball_frames_render`.
use crate::{Settings, View, render, scene_meshes};
use glam::DVec3;
use operandi_core::*;

fn ok(e: &mut Editor, text: &str) {
    if let Some(Err(err)) = command_line::run_line(e, text) {
        panic!("{text}: {err}");
    }
}

#[test]
#[ignore = "renders 25 frames with Cycles; run with --ignored"]
fn ball_frames_render() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Sphere)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        size: DVec3::splat(1.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.scene.selection.mode = Mode::Item;
    let id = e.scene.active().unwrap().id;
    let set = |e: &mut Editor, ch: &str, v: f64| {
        ok(
            e,
            &format!("transform.channel name:{ch} value:{v} item:{id}"),
        );
    };
    set(&mut e, "pos.Y", 2.);
    ok(
        &mut e,
        &format!("operandi.key.set channels:{id}:pos.Y,{id}:scl.X,{id}:scl.Y,{id}:scl.Z"),
    );
    ok(&mut e, "operandi.time.set frame:12");
    set(&mut e, "pos.Y", 0.5);
    set(&mut e, "scl.Y", 0.6);
    set(&mut e, "scl.X", 1.2);
    set(&mut e, "scl.Z", 1.2);
    ok(&mut e, "operandi.time.set frame:24");
    set(&mut e, "pos.Y", 2.);
    set(&mut e, "scl.Y", 1.);
    set(&mut e, "scl.X", 1.);
    set(&mut e, "scl.Z", 1.);
    let keys = |f: u32| {
        ["pos.Y", "scl.X", "scl.Y", "scl.Z"]
            .map(|c| format!("{id}:{c}@{f}"))
            .join(",")
    };
    ok(&mut e, &format!("operandi.key.select keys:{}", keys(12)));
    ok(&mut e, "operandi.key.slope type:linear");
    let dir = std::env::var_os("OPERANDI_BALL_CHECK")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("operandi-ball-frames"))
        .join("frames");
    std::fs::create_dir_all(&dir).unwrap();
    let view = View::perspective(
        DVec3::new(0., 1.4, 7.),
        DVec3::X,
        DVec3::Y,
        DVec3::new(0., -0.05, -1.).normalize(),
        (25f64.to_radians()).tan(),
    );
    let mut heights = Vec::new();
    for f in 0..=24 {
        // The ball's pose at frame f, as the render reads it (TK-53).
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
        let scene = e.evaluated(evaluate::Purpose::Final);
        heights.push(scene.items[0].transform.translation.y);
        let meshes = scene_meshes(&scene).unwrap();
        let mut settings = Settings::studio(320, 240, view);
        settings.samples = 16;
        let image = render(&meshes, &settings).unwrap();
        let png = image::RgbaImage::from_raw(image.width, image.height, image.to_srgb8()).unwrap();
        png.save(dir.join(format!("Ball_{f:04}.png"))).unwrap();
    }
    assert!((heights[0] - 2.).abs() < 1e-9 && (heights[12] - 0.5).abs() < 1e-9);
    assert!(heights[6] < heights[0] && heights[6] > heights[12]);
}
