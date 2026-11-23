//! Timings for the evaluation engine (EVAL-GRAPH-PROPOSAL): what a frame
//! step, a key edit and the per-frame evaluated scene cost on a scene of
//! many animated items. Opt-in:
//! `cargo test --release -p operandi-core --test all eval_probe:: -- --ignored --nocapture`
use operandi_core::*;
use std::time::Instant;

const ITEMS: usize = 2000;

/// `ITEMS` mesh items, each with pos.X/Y/Z and rot.Y keyed at 0, 12, 24.
fn animated() -> Editor {
    let mut e = Editor::default();
    let mut ids = Vec::new();
    for _ in 0..ITEMS {
        e.scene.new_mesh().unwrap();
        ids.push(e.scene.items.last().unwrap().id);
    }
    for (n, id) in ids.iter().enumerate() {
        for ch in ["pos.X", "pos.Y", "pos.Z", "rot.Y"] {
            let key = animation::ChannelKey {
                item: *id,
                channel: ch.into(),
            };
            let mut channel = animation::AnimChannel {
                curve: Default::default(),
                rest: 0.,
            };
            for (f, v) in [(0, 0.), (12, n as f64), (24, 0.)] {
                channel
                    .curve
                    .set(animation::at_frames(e.scene.time.rate, f as f64), v);
            }
            e.scene.animation.channels.insert(key, channel);
        }
    }
    e
}

fn ms(f: impl FnOnce()) -> f64 {
    let t = Instant::now();
    f();
    t.elapsed().as_secs_f64() * 1e3
}

#[test]
#[ignore = "timing probe; see the file header"]
fn frame_step_key_edit_and_display() {
    let mut e = animated();
    let channels = e.scene.animation.channels.len();
    // Frame steps (playback): 48 frames.
    let step = ms(|| {
        for f in 0..48 {
            let t = animation::at_frames(e.scene.time.rate, f as f64);
            e.set_current_time(t, true).unwrap();
        }
    }) / 48.;
    // A key edit on one channel (what the graph editor's drag sends).
    let id = e.scene.items[0].id;
    let edit = ms(|| {
        for v in 0..20 {
            command_line::run_line(
                &mut e,
                &format!("operandi.key.add channel:{id}:pos.Y frame:12 value:{v}"),
            )
            .unwrap()
            .unwrap();
        }
    }) / 20.;
    // The viewport's evaluated scene, once a frame.
    let display = ms(|| {
        for _ in 0..60 {
            std::hint::black_box(e.evaluated_display());
        }
    }) / 60.;
    println!(
        "EVAL: {ITEMS} items, {channels} channels: frame step {step:.3} ms, key edit {edit:.3} ms, evaluated display {display:.4} ms"
    );
}
