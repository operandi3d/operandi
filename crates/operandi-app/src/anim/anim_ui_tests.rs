//! Animation in the app (BEHAVIOR-TIME-KEYS, BEHAVIOR-TIMELINE): the
//! Items-mode keys, the status light, playback, the timeline strip, the
//! Animate layout, and per-document keys and undo.
use super::*;
use crate::tests::{box_app, draw, draw_held};
use operandi_core::animation::{self, ChannelKey};

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

fn frames(a: &mut Operandi, ctx: &Context, n: usize) {
    for _ in 0..n {
        let _ = draw(a, ctx, SIZE, vec![]);
    }
}

fn key(a: &mut Operandi, ctx: &Context, key: Key, modifiers: Modifiers) {
    let at = a.viewport_rect.center();
    let events = vec![
        Event::PointerMoved(at),
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        },
    ];
    let _ = draw_held(a, ctx, SIZE, events, modifiers);
    frames(a, ctx, 1);
}

/// The box item in Items mode, the box selected.
fn items_app() -> (Operandi, u64) {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Item));
    let id = a.editor.scene.active().unwrap().id;
    (a, id)
}

#[test]
fn animation_keys_are_items_mode_only_and_the_map_is_valid() {
    assert!(input::bindings_valid(&input::default_bindings()));
    for (action, _, _) in crate::anim_keys::ACTIONS {
        assert!(input::available(action, Mode::Item, false), "{action}");
        assert!(!input::available(action, Mode::Edge, false), "{action}");
    }
    // S keeps Slide in Edge mode (TK-65).
    assert!(input::available("Slide", Mode::Edge, false));
}

#[test]
fn shift_w_keys_position_and_shift_right_steps_a_frame() {
    let ctx = Context::default();
    let (mut a, id) = items_app();
    frames(&mut a, &ctx, 2);
    let steps = a.editor.history.undo.len();
    key(&mut a, &ctx, Key::W, Modifiers::SHIFT);
    assert!(
        a.editor
            .scene
            .animation
            .is_animated(&ChannelKey::new(id, "pos.X")),
        "{}",
        a.status
    );
    assert_eq!(a.editor.history.undo.len(), steps + 1, "one undo step");
    key(&mut a, &ctx, Key::ArrowRight, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.time.current_frame(), 1);
    assert_eq!(
        a.editor.history.undo.len(),
        steps + 1,
        "time is no undo step"
    );
    // In Edge mode the same key doesn't step time.
    a.run(Command::Mode(Mode::Edge));
    key(&mut a, &ctx, Key::ArrowRight, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.time.current_frame(), 1);
}

#[test]
fn status_light_click_keys_and_the_row_shows_key_then_animated() {
    let ctx = Context::default();
    let (mut a, id) = items_app();
    frames(&mut a, &ctx, 2);
    // What a light click asks for, run as the app runs it.
    a.execute(&format!("operandi.key.toggle channels:{id}:pos.Y"));
    let k = ChannelKey::new(id, "pos.Y");
    assert!(a.editor.scene.animation.is_animated(&k));
    a.anim_row("pos.Y", false);
    let row = crate::form::rows::take_anim_requests();
    assert!(row.is_empty());
    a.execute("operandi.time.set frame:10");
    assert!(!animation::is_pending(&a.editor.scene, &k));
    frames(&mut a, &ctx, 2);
}

#[test]
fn playback_advances_and_stops_with_one_history_line() {
    let ctx = Context::default();
    let (mut a, _) = items_app();
    a.execute("operandi.time.settings realTime:false");
    a.execute("operandi.time.play");
    frames(&mut a, &ctx, 5);
    let f = a.editor.scene.time.current_frame();
    assert!(f >= 3, "played to {f}");
    a.execute("operandi.time.play");
    frames(&mut a, &ctx, 2);
    let stopped = a.editor.scene.time.current_frame();
    frames(&mut a, &ctx, 3);
    assert_eq!(a.editor.scene.time.current_frame(), stopped);
    assert!(
        a.editor
            .history
            .undo
            .iter()
            .all(|c| c.label == "Time Settings" || !c.label.contains("Time"))
    );
}

#[test]
fn the_animate_layout_opens_the_tall_timeline_and_its_keys() {
    let ctx = Context::default();
    let (mut a, id) = items_app();
    frames(&mut a, &ctx, 2);
    a.execute("layout.restore Animate");
    frames(&mut a, &ctx, 3);
    assert_eq!(a.layouts.current, "Animate", "{}", a.status);
    assert!(a.timeline.open);
    assert!(a.timeline.rect.height() > 100., "{:?}", a.timeline.rect);
    assert_eq!(a.bindings["Play/Stop"], "Slash");
    // Draw the strip with keys, markers and a selected key.
    a.execute("operandi.key.transforms type:all");
    a.execute("operandi.time.set frame:12");
    a.execute("operandi.key.transforms type:position");
    a.execute("operandi.marker.add frame:6");
    a.execute(&format!("operandi.key.select keys:{id}:pos.X@12"));
    frames(&mut a, &ctx, 3);
    // `/` plays in the Animate layout.
    key(&mut a, &ctx, Key::Slash, Modifiers::NONE);
    assert!(a.editor.anim.playing.is_some(), "{}", a.status);
    a.execute("operandi.time.play stop:true");
    // The clock button's command hides the strip.
    a.execute("operandi.anim.timeline");
    frames(&mut a, &ctx, 1);
    assert!(!a.timeline.open);
}

#[test]
fn every_animate_menu_entry_runs_a_known_command() {
    let ctx = Context::default();
    let (mut a, _) = items_app();
    frames(&mut a, &ctx, 1);
    for kind in ["ranges", "settings", "render", "playblast"] {
        a.execute(&format!("operandi.anim.dialog kind:{kind}"));
        assert!(a.anim_ui.dialog.is_some(), "{kind}: {}", a.status);
        frames(&mut a, &ctx, 1);
        a.anim_ui.dialog = None;
    }
}

/// #50: Time Ranges, Render Frame Range and Playblast use the shared
/// command dialog (UIC-81..85: button column on the right, OK and Cancel
/// with their keys, no close box), and OK still runs one command line
/// built from the fields.
#[test]
fn animate_dialogs_use_the_shared_command_dialog() {
    let ctx = Context::default();
    let (mut a, _) = items_app();
    frames(&mut a, &ctx, 1);
    for (kind, title, line) in [
        ("ranges", "Time Ranges", "operandi.time.settings"),
        ("render", "Render Frame Range", "operandi.render.frameRange"),
        ("playblast", "Playblast", "operandi.view.playblast"),
    ] {
        a.execute(&format!("operandi.anim.dialog kind:{kind}"));
        let d = a.anim_ui.dialog.as_ref().expect(kind);
        let form = d
            .form
            .as_ref()
            .unwrap_or_else(|| panic!("{kind}: a shared dialog"));
        assert_eq!(form.title, title);
        let built = d.command_line();
        assert!(built.starts_with(line), "{kind}: {built}");
        assert!(
            built.contains(" start:"),
            "{kind}: the fields go into the line: {built}"
        );
        a.anim_ui.dialog = None;
    }
    // Time Settings edits live (its rows apply at once), so it keeps its own panel.
    a.execute("operandi.anim.dialog kind:settings");
    assert!(a.anim_ui.dialog.as_ref().unwrap().form.is_none());
}

/// PLAN "per-document" rule: key in scene A, switch to B, undo in B, A's
/// keys untouched; each tab keeps its own time (audit risk P2-001 is about
/// undo following the wrong document; this checks the animation side).
#[test]
fn keys_and_undo_stay_with_their_own_scene_tab() {
    let ctx = Context::default();
    let (mut a, id) = items_app();
    frames(&mut a, &ctx, 1);
    a.execute("operandi.key.transforms type:position");
    a.execute("operandi.time.set frame:30");
    let keys_a = a.editor.scene.animation.clone();
    a.new_scene_tab();
    frames(&mut a, &ctx, 1);
    assert_eq!(a.scenes.active, 1);
    assert!(a.editor.scene.animation.channels.is_empty());
    assert_eq!(a.editor.scene.time.current_frame(), 0);
    a.run(Command::Undo);
    a.run(Command::Undo);
    assert!(a.switch_scene(0));
    frames(&mut a, &ctx, 1);
    assert_eq!(a.editor.scene.animation, keys_a);
    assert!(
        a.editor
            .scene
            .animation
            .is_animated(&ChannelKey::new(id, "pos.X"))
    );
    assert_eq!(a.editor.scene.time.current_frame(), 30);
}

#[test]
fn render_frame_range_and_playblast_queue_their_jobs() {
    let ctx = Context::default();
    let (mut a, _) = items_app();
    let dir = std::env::temp_dir().join(format!("operandi-playblast-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    a.execute(&format!(
        "operandi.view.playblast start:0 end:2 scale:0.5 open:false folder:{{{}}}",
        dir.display()
    ));
    frames(&mut a, &ctx, 1);
    assert!(a.anim_ui.job.is_some(), "{}", a.status);
    assert_eq!(a.editor.anim.progress.as_ref().map(|p| p.1), Some(3));
    a.execute("operandi.render.cancelRange");
    frames(&mut a, &ctx, 1);
    assert!(a.anim_ui.job.is_none());
    assert!(a.editor.anim.progress.is_none());
}

/// TL-71: a playblast's frames become an .mp4 through ffmpeg (skipped
/// when ffmpeg isn't installed), and the frames folder goes.
#[test]
fn playblast_frames_encode_to_a_movie() {
    if crate::anim_ui::ffmpeg().is_none() {
        eprintln!("no ffmpeg here; movie encode skipped");
        return;
    }
    let dir = std::env::temp_dir().join(format!("operandi-movie-{}", std::process::id()));
    let frames = dir.join(".Ball-frames");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&frames).unwrap();
    for f in 0..4u8 {
        let img = image::RgbaImage::from_pixel(33, 21, image::Rgba([f * 60, 100, 200, 255]));
        img.save(crate::anim_ui::frame_path(&frames, "Ball", f as i64))
            .unwrap();
    }
    let movie = dir.join("Ball.mp4");
    let out = crate::anim_ui::encode_movie(&frames, "Ball", 0, 24., &movie).unwrap();
    assert!(std::fs::metadata(&out).unwrap().len() > 0);
    assert!(!frames.exists(), "the frames folder goes");
    let _ = std::fs::remove_dir_all(&dir);
}

/// GE-01..93 in the app: the graph editor opens from its command, draws
/// the ball's curve, its keys select the same keys as the track row, a
/// drag is one undo step, and its view lines work.
#[test]
fn graph_editor_opens_draws_and_shares_the_selection() {
    let ctx = Context::default();
    let (mut a, id) = items_app();
    frames(&mut a, &ctx, 1);
    let y = format!("{id}:pos.Y");
    for (f, v) in [(0, 2.), (12, 0.), (24, 2.)] {
        a.execute(&format!("operandi.key.add channel:{y} frame:{f} value:{v}"));
    }
    a.execute("operandi.graph.open");
    a.execute(&format!("operandi.channel.select channel:{y}"));
    frames(&mut a, &ctx, 3);
    assert!(a.graph.open);
    let rect = a.graph.graph_rect.expect("the graph drew");
    assert!(rect.width() > 100.);
    // GE-91: the selection is the track row's.
    a.execute(&format!("operandi.key.select keys:{y}@12"));
    a.execute("operandi.anim.timeline open:tall");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.editor.live_selected_keys().len(), 1);
    // A drag's result is one command, one undo step (GE-45).
    let steps = a.editor.history.undo.len();
    a.execute("operandi.key.offset frames:1 value:0.5");
    assert_eq!(a.editor.history.undo.len(), steps + 1);
    for line in [
        "operandi.graph.view fit:all",
        "operandi.graph.view normalise:toggle stacked:toggle isolate:toggle",
        "operandi.graph.view velocity:true shadow:true",
        "operandi.curve.shadow action:copy",
        "operandi.curve.behavior post:repeat",
    ] {
        a.execute(line);
        frames(&mut a, &ctx, 1);
    }
    assert!(a.graph.normalised && a.graph.stacked && a.graph.isolate);
    a.execute("operandi.graph.open open:false");
    frames(&mut a, &ctx, 1);
    assert!(a.graph.graph_rect.is_none());
}

#[test]
fn graph_keys_go_through_the_routing_seam() {
    let lines: Vec<&str> = crate::anim_keys::GRAPH_KEYS
        .iter()
        .map(|(_, _, l)| *l)
        .collect();
    assert!(lines.contains(&"operandi.graph.view fit:all"));
    assert!(lines.contains(&"operandi.graph.view normalise:toggle"));
}

/// The graph editor window keeps its size frame after frame (it used to
/// grow taller every frame), and opens at the size it was left at.
#[test]
fn graph_editor_window_keeps_its_size() {
    let ctx = Context::default();
    let (mut a, id) = items_app();
    let y = format!("{id}:pos.Y");
    for (f, v) in [(0, 2.), (12, 0.), (24, 2.)] {
        a.execute(&format!("operandi.key.add channel:{y} frame:{f} value:{v}"));
    }
    a.execute("operandi.graph.open");
    frames(&mut a, &ctx, 3);
    let first = a.graph.graph_rect.expect("the graph drew");
    frames(&mut a, &ctx, 20);
    let later = a.graph.graph_rect.unwrap();
    assert!(
        (later.height() - first.height()).abs() < 0.5,
        "grew from {first:?} to {later:?}"
    );
    // (Tests draw OS windows inside the main one, so the window's own
    // shape, a wide short strip, is a GUI-checklist item.)
    // Reopened at a remembered (resized) place, it keeps that size.
    a.execute("operandi.graph.open open:false");
    frames(&mut a, &ctx, 1);
    a.palettes.places.insert(
        "graphEditor".into(),
        crate::palettes::Place {
            size: [700., 300.],
            position: Some([10., 10.]),
        },
    );
    a.execute("operandi.graph.open open:true");
    frames(&mut a, &ctx, 3);
    let a1 = a.graph.graph_rect.unwrap();
    frames(&mut a, &ctx, 10);
    let a2 = a.graph.graph_rect.unwrap();
    assert!((a1.height() - a2.height()).abs() < 0.5, "{a1:?} {a2:?}");
    // Its place is remembered as the window reports it.
    assert!(a.palettes.places.contains_key("graphEditor"));
}

/// The animation switch (anim_switch.rs) off: no Animate layout, menu,
/// timeline, graph editor or animation keys; keys in a scene still play
/// and save, and the commands still run.
#[test]
fn the_animation_switch_off_hides_the_ui_but_keeps_the_data() {
    crate::anim_switch::set_for_test(false);
    let ctx = Context::default();
    let (mut a, id) = items_app();
    frames(&mut a, &ctx, 2);
    assert!(!a.layouts.names().iter().any(|n| n == "Animate"));
    assert!(!a.menu_model(&ctx).iter().any(|m| m.title == "Animate"));
    // Shift+W keeps no animation meaning.
    key(&mut a, &ctx, Key::W, Modifiers::SHIFT);
    assert!(a.editor.scene.animation.channels.is_empty());
    // The strip and the graph editor don't draw, even asked for.
    a.execute("operandi.anim.timeline open:tall");
    a.execute("operandi.graph.open");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.timeline.rect, Rect::NOTHING);
    assert!(a.graph.graph_rect.is_none());
    // Commands (scripts, the MCP bridge) still animate, and the pose plays.
    let y = format!("{id}:pos.Y");
    a.execute(&format!("operandi.key.add channel:{y} frame:0 value:0"));
    a.execute(&format!("operandi.key.add channel:{y} frame:10 value:5"));
    a.execute("operandi.time.set frame:10");
    assert!((a.editor.scene.items[0].transform.translation.y - 5.).abs() < 1e-9);
    // Saved and reopened with the switch off, the keys are all there.
    let bytes = operandi_core::persistence::encode(&a.editor.scene).unwrap();
    let back = operandi_core::persistence::decode(&bytes).unwrap();
    assert_eq!(back.animation, a.editor.scene.animation);
    crate::anim_switch::set_for_test(true);
}

#[test]
fn the_animation_switch_on_shows_the_ui() {
    crate::anim_switch::set_for_test(true);
    let ctx = Context::default();
    let (mut a, _) = items_app();
    frames(&mut a, &ctx, 2);
    assert!(a.layouts.names().iter().any(|n| n == "Animate"));
    assert!(a.menu_model(&ctx).iter().any(|m| m.title == "Animate"));
    key(&mut a, &ctx, Key::W, Modifiers::SHIFT);
    assert!(!a.editor.scene.animation.channels.is_empty());
}

/// GE-01/44/81/82/X4/X5/83: the graph editor's tools draw and their
/// lines run.
#[test]
fn graph_editor_gap_tools_draw_and_run() {
    let ctx = Context::default();
    let (mut a, id) = items_app();
    let y = format!("{id}:pos.Y");
    for (f, v) in [(0, 2.), (12, 0.), (24, 2.)] {
        a.execute(&format!("operandi.key.add channel:{y} frame:{f} value:{v}"));
    }
    a.execute("operandi.anim.editors");
    frames(&mut a, &ctx, 2);
    assert!(
        a.graph.open && a.timeline.open,
        "Animation Editors opens both"
    );
    a.execute(&format!("operandi.key.select keys:{y}@0,{y}@12"));
    a.execute("operandi.key.slope type:auto");
    for line in [
        "operandi.graph.view boxScale:true speed:true",
        "operandi.graph.view stacked:true rows:2 rowOffset:0",
        "operandi.graph.view retime:true",
    ] {
        a.execute(line);
        frames(&mut a, &ctx, 2);
    }
    assert!(a.graph.box_scale && a.graph.speed && a.graph.retime);
    assert_eq!(a.graph.rows, 2);
    // The box drag's result: one transform line, one undo step.
    let steps = a.editor.history.undo.len();
    a.execute("operandi.key.transform timeFactor:2 timePivot:0");
    assert_eq!(a.editor.history.undo.len(), steps + 1);
    // Envelope presets land in Presets/Envelopes.
    let folder = a.envelope_folder();
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("bounce.openvelope");
    a.execute(&format!(
        "operandi.curve.saveEnvelope channels:{y} path:{{{}}}",
        path.display()
    ));
    assert!(path.is_file(), "{}", a.status);
    a.graph.retime_marks = vec![6., 12.];
    frames(&mut a, &ctx, 2);
}

/// GE-X7 probe: frame time with 100 curves of 1,000 keys shown, panning
/// every frame. Opt-in (timing):
/// `cargo test --profile qa -p operandi-app graph_editor_speed_probe -- --ignored --nocapture`.
#[test]
#[ignore = "timing probe; run with --ignored --nocapture"]
fn graph_editor_speed_probe() {
    use operandi_core::anim_curve::{Curve, Key};
    use operandi_core::animation::AnimChannel;
    let ctx = Context::default();
    let (mut a, _) = items_app();
    for _ in 0..33 {
        a.run(Command::NewMesh);
    }
    let rate = a.editor.scene.time.rate;
    let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    let mut n = 0;
    'fill: for id in &ids {
        for ch in ["pos.X", "pos.Y", "pos.Z"] {
            let keys = (0..1000)
                .map(|f| Key::new(rate.tick_of(f), ((f as f64) * 0.1 + n as f64).sin()))
                .collect();
            let k = ChannelKey::new(*id, ch);
            a.editor.scene.animation.channels.insert(
                k.clone(),
                AnimChannel {
                    curve: Curve {
                        keys,
                        ..Default::default()
                    },
                    rest: 0.,
                },
            );
            a.editor.anim.selected_channels.insert(k);
            n += 1;
            if n == 100 {
                break 'fill;
            }
        }
    }
    a.editor.scene.selection.also = ids.clone();
    a.graph.open = true;
    frames(&mut a, &ctx, 3);
    let shown = a.graph.graph_rect.is_some();
    let started = std::time::Instant::now();
    let count = 60;
    for i in 0..count {
        // Pan a little every frame.
        a.execute("operandi.graph.view fit:all");
        let _ = i;
        frames(&mut a, &ctx, 1);
    }
    let per = started.elapsed().as_secs_f64() * 1000. / count as f64;
    println!("GE-X7: {n} curves x 1000 keys, graph shown {shown}: {per:.2} ms per frame");
}
