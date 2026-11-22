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
    for (action, _, _) in crate::anim_ui::ACTIONS {
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
    let steps = a.editor.undo.len();
    key(&mut a, &ctx, Key::W, Modifiers::SHIFT);
    assert!(
        a.editor
            .scene
            .animation
            .is_animated(&ChannelKey::new(id, "pos.X")),
        "{}",
        a.status
    );
    assert_eq!(a.editor.undo.len(), steps + 1, "one undo step");
    key(&mut a, &ctx, Key::ArrowRight, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.time.current_frame(), 1);
    assert_eq!(a.editor.undo.len(), steps + 1, "time is no undo step");
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
