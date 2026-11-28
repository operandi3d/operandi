//! USD animation export (TL-80..84): the bouncing ball written with time
//! samples, its stage timing, and (TL-84) read back with OpenUSD's own
//! Python from Blender's bundled interpreter when one is installed.
use operandi_core::interchange::ExportFormat;
use operandi_core::*;
use std::path::PathBuf;

fn ok(e: &mut Editor, text: &str) {
    if let Some(Err(err)) = command_line::run_line(e, text) {
        panic!("{text}: {err}");
    }
}

/// The bouncing-ball run's keys (BEHAVIOR-TIMELINE section 10, steps 1-4).
pub(crate) fn ball() -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Sphere)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        size: glam::DVec3::splat(1.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.scene.selection.mode = Mode::Item;
    let id = e.scene.active().unwrap().id;
    e.scene.items[0].name = "Ball".into();
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
    set(&mut e, "pos.Y", 0.);
    set(&mut e, "scl.Y", 0.6);
    set(&mut e, "scl.X", 1.2);
    set(&mut e, "scl.Z", 1.2);
    ok(&mut e, "operandi.time.set frame:24");
    set(&mut e, "pos.Y", 2.);
    set(&mut e, "scl.Y", 1.);
    set(&mut e, "scl.X", 1.);
    set(&mut e, "scl.Z", 1.);
    let at = |f: u32| {
        ["pos.Y", "scl.X", "scl.Y", "scl.Z"]
            .map(|c| format!("{id}:{c}@{f}"))
            .join(",")
    };
    ok(&mut e, &format!("operandi.key.select keys:{}", at(12)));
    ok(&mut e, "operandi.key.slope type:linear");
    ok(&mut e, &format!("operandi.key.select keys:{}", at(0)));
    ok(&mut e, "operandi.key.copy");
    ok(&mut e, "operandi.time.keyTime frame:24");
    ok(&mut e, "operandi.key.paste mode:replace");
    ok(&mut e, "operandi.time.settings end:24");
    ok(&mut e, "operandi.time.set frame:0");
    (e, id)
}

/// Where the check writes: `OPERANDI_BALL_CHECK` (a folder), else a temp.
fn out_dir() -> PathBuf {
    let dir = std::env::var_os("OPERANDI_BALL_CHECK")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("operandi-ball-usd-{}", std::process::id()))
        });
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn tl80_82_the_ball_exports_time_samples_and_stage_timing() {
    let (e, _) = ball();
    let path = out_dir().join("ball.usda");
    let scene = e.evaluated(operandi_core::evaluate::Purpose::Final);
    crate::export(&path, &scene, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    for needle in [
        "framesPerSecond = 24",
        "timeCodesPerSecond = 24",
        "startTimeCode = 0",
        "endTimeCode = 24",
        "xformOp:translate.timeSamples",
        "xformOp:scale.timeSamples",
    ] {
        assert!(text.contains(needle), "{needle} missing:\n{text}");
    }
    // Rotation isn't animated: a default, no samples (TL-81).
    assert!(!text.contains("xformOp:rotateXYZ.timeSamples"));
    // One sample per frame, 0..24.
    let samples = text
        .split("xformOp:translate.timeSamples")
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap()
        .matches(':')
        .count();
    assert_eq!(samples, 25);
    // Animation off: no samples (TL-83).
    let off = out_dir().join("ball-static.usda");
    operandi_core::interchange::set_io_options(operandi_core::interchange::IoOptions {
        usd_animation: operandi_core::interchange::UsdAnimation::Off,
        ..Default::default()
    });
    let r = crate::export(&off, &scene, ExportFormat::Usda);
    operandi_core::interchange::set_io_options(Default::default());
    r.unwrap();
    assert!(
        !std::fs::read_to_string(&off)
            .unwrap()
            .contains("timeSamples")
    );
}

/// TL-84 with OpenUSD itself: Blender's Python opens the file and reads the
/// ball's height at frames 0, 6, 12 and 24. Skipped (passes) without
/// Blender.
#[test]
fn tl84_openusd_reads_the_same_heights() {
    let python =
        PathBuf::from("/Applications/Blender.app/Contents/Resources/5.2/python/bin/python3.13");
    let (e, id) = ball();
    let dir = out_dir();
    let path = dir.join("ball.usd");
    let scene = e.evaluated(operandi_core::evaluate::Purpose::Final);
    crate::export(&path, &scene, ExportFormat::Usd).unwrap();
    if !python.is_file() {
        eprintln!("TL-84: no Blender Python here; read-back skipped");
        return;
    }
    let script = dir.join("read_ball.py");
    std::fs::write(
        &script,
        r#"import sys
from pxr import Usd, UsdGeom
stage = Usd.Stage.Open(sys.argv[1])
ball = next(p for p in stage.Traverse() if p.GetName() == "Ball")
x = UsdGeom.Xformable(ball)
for f in (0, 6, 12, 24):
    m = x.ComputeLocalToWorldTransform(Usd.TimeCode(f))
    s = x.GetLocalTransformation(Usd.TimeCode(f))
    print(f, m.ExtractTranslation()[1], s.GetRow3(1).GetLength())
"#,
    )
    .unwrap();
    let out = std::process::Command::new(&python)
        .arg("-I")
        .arg(&script)
        .arg(&path)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::write(dir.join("ball-readback.txt"), text.as_bytes()).unwrap();
    let rate = e.scene.time.rate;
    let curve = |ch: &str, f: i64| {
        e.scene.animation.channels[&animation::ChannelKey::new(id, ch)]
            .curve
            .value_at(rate.tick_of(f))
    };
    let mut seen = 0;
    for line in text.lines() {
        let v: Vec<f64> = line
            .split_whitespace()
            .map(|w| w.parse().unwrap())
            .collect();
        let f = v[0] as i64;
        assert!((v[1] - curve("pos.Y", f)).abs() < 1e-4, "frame {f}: {line}");
        assert!((v[2] - curve("scl.Y", f)).abs() < 1e-4, "frame {f}: {line}");
        seen += 1;
    }
    assert_eq!(seen, 4, "{text}");
}

/// Export then import: the ball comes back moving the same way, at the
/// same rate and range, with keys only on what moved.
#[test]
fn the_ball_round_trips_with_its_motion() {
    let (e, id) = ball();
    let path = out_dir().join("ball-round-trip.usdc");
    let scene = e.evaluated(operandi_core::evaluate::Purpose::Final);
    crate::export(&path, &scene, ExportFormat::Usd).unwrap();
    let back = crate::import(&path, &Default::default()).unwrap();
    let b = &back.scene;
    assert_eq!(b.time.rate, e.scene.time.rate);
    assert_eq!(
        (b.time.start, b.time.end),
        (e.scene.time.start, e.scene.time.end)
    );
    let ball = b.items.iter().find(|i| i.name == "Ball").unwrap().id;
    let keyed: std::collections::BTreeSet<String> = b
        .animation
        .item_channels(ball)
        .map(|(k, _)| k.channel.clone())
        .collect();
    let want: std::collections::BTreeSet<String> = ["pos.Y", "scl.X", "scl.Y", "scl.Z"]
        .map(String::from)
        .into();
    assert_eq!(keyed, want);
    let rate = e.scene.time.rate;
    for ch in ["pos.Y", "scl.Y", "scl.X"] {
        let src = &e.scene.animation.channels[&animation::ChannelKey::new(id, ch)].curve;
        let got = &b.animation.channels[&animation::ChannelKey::new(ball, ch)].curve;
        for f in 0..=24 {
            let (a, v) = (src.value_at(rate.tick_of(f)), got.value_at(rate.tick_of(f)));
            assert!((a - v).abs() < 1e-5, "{ch} at {f}: {a} vs {v}");
        }
    }
    assert!(
        back.notes.iter().any(|n| n.starts_with("Animation:")),
        "{:?}",
        back.notes
    );
}

/// A camera's focal length and a light's intensity, keyed, come back
/// keyed (TL-81 the other way).
#[test]
fn camera_and_light_keys_round_trip() {
    use operandi_core::anim_curve::Curve;
    use operandi_core::animation::{AnimChannel, ChannelKey};
    use operandi_core::camera_light::{CameraSettings, LightSettings};
    let mut scene = crate::plan_tests::box_scene("Box");
    let cam = scene
        .add_camera(CameraSettings::default(), Default::default())
        .unwrap();
    let lamp = scene
        .add_light(LightSettings::default(), Default::default())
        .unwrap();
    let rate = scene.time.rate;
    let curve = |a: f64, b: f64| Curve {
        keys: vec![
            animation::new_key(rate.tick_of(0), a),
            animation::new_key(rate.tick_of(10), b),
        ],
        ..Curve::default()
    };
    let f0 = scene
        .item(cam)
        .unwrap()
        .camera
        .as_ref()
        .unwrap()
        .focal_length;
    for (key, c) in [
        (ChannelKey::new(cam, "focalLength"), curve(f0, f0 * 2.0)),
        (ChannelKey::new(lamp, "intensity"), curve(1.0, 3.0)),
    ] {
        let rest = c.keys[0].value;
        scene
            .animation
            .channels
            .insert(key, AnimChannel { curve: c, rest });
    }
    scene.time.end = rate.tick_of(10);
    // A document is always posed at its current time (export reads it as
    // it is there).
    let now = scene.time.current;
    animation::pose(&mut scene, now);
    let path = out_dir().join("lens-and-lamp.usda");
    crate::export(&path, &scene, ExportFormat::Usda).unwrap();
    let back = crate::import(&path, &Default::default()).unwrap().scene;
    let item = |pred: &dyn Fn(&Item) -> bool| back.items.iter().find(|i| pred(i)).unwrap().id;
    let c = item(&|i| i.camera.is_some());
    let l = item(&|i| i.light.is_some());
    let value = |id: u64, ch: &str, f: i64| {
        back.animation.channels[&ChannelKey::new(id, ch)]
            .curve
            .value_at(rate.tick_of(f))
    };
    for f in [0, 5, 10] {
        let want = scene.animation.channels[&ChannelKey::new(cam, "focalLength")]
            .curve
            .value_at(rate.tick_of(f));
        assert!(
            (value(c, "focalLength", f) - want).abs() < 1e-5 * want,
            "frame {f}"
        );
    }
    let (a, b) = (value(l, "intensity", 0), value(l, "intensity", 10));
    assert!((b / a - 3.0).abs() < 1e-4, "{a} {b}");
}
