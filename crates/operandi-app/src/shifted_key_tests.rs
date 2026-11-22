//! Shift+symbol shortcuts (bug, Jason 2026-11-21): with Shift held the
//! window reports the symbol typed, so Shift+] arrives as "}" and Close
//! Loop's factory "Shift+]" never fired.
//!
//! 1. Shift+"}" runs Close Loop from two adjacent polygons.
//! 2. Every shifted symbol matches its key's Shift binding; without Shift
//!    the symbol doesn't take the plain key's binding.
//! 3. The input map records Shift+] as "Shift+]", not "Shift+}".
use super::*;

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    ctx.options_mut(|o| o.max_passes = std::num::NonZeroUsize::MIN);
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440., 900.))),
            modifiers: Modifiers::SHIFT,
            events,
            ..Default::default()
        },
        |ctx| {
            a.begin_input(ctx);
            a.compose(ctx);
            a.keyboard(ctx);
        },
    );
}
fn key(key: Key, pressed: bool) -> Event {
    Event::Key {
        key,
        physical_key: Some(Key::CloseBracket),
        pressed,
        repeat: false,
        modifiers: Modifiers::SHIFT,
    }
}

#[test]
fn shift_close_bracket_runs_close_loop() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
            .unwrap()
            .into();
    a.editor.synchronize_identities().unwrap();
    a.run(Command::Mode(Mode::Polygon));
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, vec![]);
    let mesh = a.editor.scene.active().unwrap().mesh.clone();
    let by_normal = |n: DVec3| {
        (0..mesh.faces.len())
            .find(|&f| mesh.normal(f).dot(n) > 0.9)
            .unwrap() as u32
    };
    let (front, top) = (by_normal(DVec3::Z), by_normal(DVec3::Y));
    a.editor.scene.selection.faces = vec![front, top];
    frame(&mut a, &ctx, vec![key(Key::CloseCurlyBracket, true)]);
    frame(&mut a, &ctx, vec![key(Key::CloseCurlyBracket, false)]);
    assert_eq!(a.editor.scene.selection.faces.len(), 4, "{}", a.status);
    assert_eq!(&a.editor.scene.selection.faces[..2], &[front, top]);
}

#[test]
fn shifted_symbols_match_their_keys_shift_binding() {
    for (binding, typed) in [
        ("Shift+]", Key::CloseCurlyBracket),
        ("Shift+[", Key::OpenCurlyBracket),
        ("Shift+Slash", Key::Questionmark),
        ("Shift+;", Key::Colon),
        ("Shift+\\", Key::Pipe),
        ("Shift+=", Key::Plus),
        ("Shift+1", Key::Exclamationmark),
    ] {
        assert!(
            input::matches_chord(binding, typed, Modifiers::SHIFT),
            "{binding} should take {typed:?}"
        );
        // The plain key's binding never takes the shifted symbol.
        let plain = binding.trim_start_matches("Shift+");
        assert!(
            !input::matches_chord(plain, typed, Modifiers::SHIFT),
            "{plain}"
        );
    }
    // The key itself (another layout, or the physical key) still matches.
    assert!(input::matches_chord(
        "Shift+]",
        Key::CloseBracket,
        Modifiers::SHIFT
    ));
    assert!(!input::matches_chord(
        "Shift+]",
        Key::CloseBracket,
        Modifiers::NONE
    ));
}

#[test]
fn the_input_map_records_the_key_under_the_symbol() {
    let captured = input_map_editor::capture(&[key(Key::CloseCurlyBracket, true)], &[]);
    assert!(
        matches!(&captured, Some(input_map_editor::Captured::Chord(c)) if c == "Shift+]"),
        "{captured:?}"
    );
}
