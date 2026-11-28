//! Keyboard shortcuts, text-field focus, modal ownership and tool switching.
use super::tests::{box_app, draw, key, pointer};
use super::*;

#[test]
fn tool_switch_routes_confirm_fields_and_commit_once() {
    for scale in [1., 2.] {
        let ctx = Context::default();
        theme::install(&ctx);
        ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), scale));
        let mut a = box_app();
        a.editor.scene.selection.faces = vec![5];
        a.activate(Tool::Bevel);
        // Start an interactive stage; armed numeric configuration alone is deferred (BV-R23).
        a.run(Command::Preview(
            a.editor.session.live.as_ref().unwrap().params.clone(),
        ));
        a.fields.insert("inset".into(), "0.1 m".into());
        let history = a.editor.history.undo.len();
        // Real toolbox click confirms pending text, drops Bevel and starts Move.
        for _ in 0..2 {
            let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let pos = a.toolbox_rects["Move"].center();
        let _ = draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![
                pointer(pos, true, Modifiers::NONE),
                pointer(pos, false, Modifiers::NONE),
            ],
        );
        assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Move);
        assert_eq!(a.editor.history.undo.len(), history + 1);
        assert_eq!(a.editor.history.undo.last().unwrap().label, "Bevel");
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 10);
        assert!(!a.fields.contains_key("inset"));
        a.fields.insert("delta0".into(), "0.25 m".into());
        a.menu_action(&ctx, app_menu::line_action(Tool::Rotate.label()));
        assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Rotate);
        assert_eq!(a.editor.history.undo.len(), history + 2);
        assert!(!a.fields.contains_key("delta0"));
        // Keyboard switching drops a neutral tool without another checkpoint.
        ctx.memory_mut(|m| m.surrender_focus(m.focused().unwrap_or(Id::NULL)));
        let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::R)]);
        assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Scale);
        assert_eq!(a.editor.history.undo.len(), history + 2);
        a.execute("extrude");
        assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Extrude);
        assert_eq!(a.editor.history.undo.len(), history + 2);
    }
}
#[test]
fn invalid_text_blocks_switch_until_corrected() {
    let ctx = Context::default();
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.activate(Tool::Bevel);
    // Start an interactive stage; armed numeric configuration alone is deferred (BV-R23).
    a.run(Command::Preview(
        a.editor.session.live.as_ref().unwrap().params.clone(),
    ));
    a.fields.insert("inset".into(), "abc".into());
    let before = a.editor.clone();
    a.menu_action(&ctx, app_menu::line_action(Tool::Move.label()));
    assert!(a.editor == before);
    assert_eq!(a.fields["inset"], "abc");
    assert!(a.field_errors.contains_key("inset"));
    a.fields.insert("inset".into(), "0.1 m".into());
    a.activate(Tool::Move);
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Move);
    assert_eq!(a.editor.history.undo.len(), before.history.undo.len() + 1);
    assert!(a.field_errors.is_empty());
}
#[test]
fn t11_002_save_failure_keeps_task_history_buffers_and_association() {
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.activate(Tool::Bevel);
    a.run(Command::Preview(Parameters {
        inset: 0.25,
        ..Default::default()
    }));
    let before = a.editor.clone();
    let path = a.path.clone();
    assert!(!a.save(std::env::temp_dir().join("operandi-no-such-directory-m11/scene.opscene")));
    assert_eq!(a.editor.scene, before.scene);
    assert_eq!(
        a.editor.session.live.as_ref().unwrap().params,
        before.session.live.as_ref().unwrap().params
    );
    assert_eq!(a.editor.history.undo.len(), before.history.undo.len());
    assert_eq!(a.path, path);
    a.fields.insert("inset".into(), "abc".into());
    assert!(!a.save(std::env::temp_dir().join("must-not-be-written.opscene")));
    assert_eq!(a.fields["inset"], "abc");
    assert_eq!(a.editor.scene, before.scene);
}
#[test]
fn t11_005_captured_navigation_never_becomes_edit_even_coalesced() {
    let ctx = Context::default();
    theme::install(&ctx);
    for tool in [
        None,
        Some(Tool::Box),
        Some(Tool::Move),
        Some(Tool::Rotate),
        Some(Tool::Scale),
        Some(Tool::Bevel),
        Some(Tool::Extrude),
    ] {
        for mods in [
            Modifiers::ALT,
            Modifiers {
                alt: true,
                shift: true,
                ..Modifiers::NONE
            },
            Modifiers {
                alt: true,
                ctrl: true,
                ..Modifiers::NONE
            },
        ] {
            let mut a = box_app();
            a.editor.scene.selection.faces = vec![5];
            if let Some(t) = tool {
                a.activate(t);
            }
            let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
            let before = a.editor.clone();
            let start = a
                .handles(a.viewport_rect)
                .first()
                .map(|h| h.2)
                .unwrap_or(a.viewport_rect.center());
            let end = start + vec2(60., 35.);
            let _ = draw(
                &mut a,
                &ctx,
                vec2(1440., 900.),
                vec![
                    pointer(start, true, mods),
                    Event::PointerMoved(end),
                    pointer(end, false, Modifiers::NONE),
                ],
            );
            assert_eq!(a.editor.scene, before.scene, "{tool:?}");
            assert_eq!(a.editor.history.undo.len(), before.history.undo.len());
            assert_eq!(
                a.editor.session.live.as_ref().map(|l| &l.params),
                before.session.live.as_ref().map(|l| &l.params)
            );
            assert_ne!(a.camera.view, View::Perspective); // orbit preserves orthographic projection
            assert!(a.navigation.is_none());
        }
    }
}
#[test]
fn t11_008_space_opens_mini_properties_alt_shift_space_cycles_and_text_priority() {
    // CV-03 / UIC-66: Space taps open and close Mini Properties and leave
    // the tool on; Alt+Shift+Space does Space's old mode cycle.
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.activate(Tool::Bevel);
    a.run(Command::Preview(Parameters {
        inset: 0.25,
        ..Default::default()
    }));
    let release = |key: Key, modifiers: Modifiers| Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers,
    };
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Space)]);
    assert!(
        a.editor.session.live.is_some(),
        "Space no longer drops the tool"
    );
    assert!(a.mini_properties.is_some());
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![release(Key::Space, Modifiers::NONE)],
    );
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Escape)]);
    assert!(a.mini_properties.is_some(), "Esc doesn't close it (MP-06)");
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Space)]);
    assert!(a.mini_properties.is_none(), "a second tap closes it");
    let alt_shift = Modifiers::ALT | Modifiers::SHIFT;
    a.run(Command::Drop);
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![release(Key::Space, Modifiers::NONE)],
    );
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![Event::Key {
            key: Key::Space,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: alt_shift,
        }],
    );
    assert_eq!(a.editor.scene.selection.mode, Mode::Vertex);
    assert!(a.mini_properties.is_none());
    a.editor_owned_input = true;
    a.execute("move"); // focused editor guard is at keyboard boundary
    let mode = a.editor.scene.selection.mode;
    ctx.memory_mut(|m| m.request_focus(Id::new("focused-editor")));
    ctx.input_mut(|i| i.events = vec![key(Key::Space)]);
    a.keyboard(&ctx);
    assert_eq!(mode, a.editor.scene.selection.mode);
    assert!(a.mini_properties.is_none());
}
#[test]
fn t11_006_013_editor_enter_tab_blur_escape_and_invalid_save() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.activate(Tool::Move);
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    let x = a.field_rects["delta0"].center();
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(x, true, Modifiers::NONE),
            pointer(x, false, Modifiers::NONE),
        ],
    );
    a.fields.insert("delta0".into(), "1 m/4".into());
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Tab)]);
    assert_eq!(a.editor.session.live.as_ref().unwrap().params.delta.x, 0.25);
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(x, true, Modifiers::NONE),
            pointer(x, false, Modifiers::NONE),
        ],
    );
    a.fields.insert("delta0".into(), "abc".into());
    let before = a.editor.scene.clone();
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Enter)]);
    assert_eq!(a.editor.scene, before);
    assert!(a.field_errors.contains_key("delta0"));
    a.save_current();
    assert!(a.file_dialog.is_none());
    assert!(a.editor.session.live.is_some());
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Escape)]);
    // A confirmed value shows in its unit (FORM-Q14).
    assert_eq!(a.fields["delta0"], "0.25 m");
    assert!(a.editor.session.live.is_some());
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(x, true, Modifiers::NONE),
            pointer(x, false, Modifiers::NONE),
        ],
    );
    a.fields.insert("delta0".into(), "1 m + 25 cm".into());
    let y = a.field_rects["delta1"].center();
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(y, true, Modifiers::NONE),
            pointer(y, false, Modifiers::NONE),
        ],
    );
    assert_eq!(a.editor.session.live.as_ref().unwrap().params.delta.x, 1.25);
}
#[test]
fn t11_002_valid_unconfirmed_save_is_transactional_and_path_cancel_is_neutral() {
    let mut a = box_app();
    a.activate(Tool::Move);
    a.fields.insert("delta0".into(), "0.25 m".into());
    a.confirmed_fields.insert("delta0".into(), "0 m".into());
    let before = a.editor.clone();
    a.request_file(FileAction::SaveAs);
    assert!(a.file_dialog.is_some());
    assert_eq!(a.editor.scene, before.scene);
    assert_eq!(
        a.editor.session.live.as_ref().unwrap().params,
        before.session.live.as_ref().unwrap().params
    );
    a.file_dialog = None;
    assert!(!a.save(std::env::temp_dir().join("operandi-no-such-directory-m11/scene.opscene")));
    assert_eq!(a.editor.scene, before.scene);
    assert_eq!(
        a.editor.session.live.as_ref().unwrap().params,
        before.session.live.as_ref().unwrap().params
    );
    assert_eq!(a.fields["delta0"], "0.25 m");
    assert_eq!(a.confirmed_fields["delta0"], "0 m");
}
#[test]
fn t11_006_native_file_key_modifiers_are_read_at_press_even_after_release() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    let events = vec![
        Event::Key {
            key: Key::S,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: super::tests::PLATFORM_CMD,
        },
        Event::Key {
            key: Key::S,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: super::tests::PLATFORM_CMD,
        },
    ];
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), events);
    assert!(a.file_dialog.is_some());
    assert_eq!(
        input::file_shortcut(&[Event::Key {
            key: Key::W,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: super::tests::PLATFORM_CMD
        }]),
        Some(input::FileShortcut::Close)
    );
}
#[test]
fn t11_006_025_dismissing_one_modal_never_replays_keys_into_another_owner() {
    let ctx = Context::default();
    let mut a = box_app();
    a.activate(Tool::Move);
    a.bindings.insert("Move".into(), "g".into());
    a.show_bindings = true;
    a.bindings_draft.insert("Move".into(), "x".into());
    a.file_dialog = Some(FileDialog {
        action: FileAction::SaveAs,
        path: "/tmp/unused.opscene".into(),
        overwrite: false,
    });
    let scene = a.editor.scene.clone();
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![key(Key::Escape), key(Key::Space), key(Key::G)],
    );
    assert!(a.file_dialog.is_none());
    assert!(
        a.show_bindings,
        "the resumed Input Map must not see the same Escape"
    );
    assert_eq!(a.bindings_draft["Move"], "x");
    assert_eq!(a.editor.scene, scene);
    assert!(a.editor.session.live.is_some());
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![key(Key::Escape), key(Key::Space)],
    );
    assert!(!a.show_bindings);
    assert!(
        a.editor.session.live.is_some(),
        "modal dismissal must not replay Space into Drop"
    );
    assert_eq!(a.editor.scene, scene);
}
#[test]
fn t11_002_021_unconfirmed_name_cannot_be_silently_discarded_by_save() {
    let mut a = box_app();
    let scene = a.editor.scene.clone();
    a.name = "Unconfirmed name".into();
    a.request_file(FileAction::SaveAs);
    assert!(a.file_dialog.is_none());
    assert!(a.status.contains("Confirm the mesh name"));
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.name, "Unconfirmed name");
    a.run(Command::Rename(a.name.clone()));
    a.request_file(FileAction::SaveAs);
    assert!(a.file_dialog.is_some());
    assert_eq!(a.editor.scene.active().unwrap().name, "Unconfirmed name");
}
#[test]
fn t11_006_menu_owns_local_numeric_escape_even_if_popup_closed_this_frame() {
    let ctx = Context::default();
    let mut a = box_app();
    a.activate(Tool::Move);
    let _ = draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    let x = a.field_rects["delta0"].center();
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(x, true, Modifiers::NONE),
            pointer(x, false, Modifiers::NONE),
        ],
    );
    a.fields.insert("delta0".into(), "unfinished".into());
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440., 900.))),
            events: vec![key(Key::Escape)],
            focused: true,
            ..Default::default()
        },
        |ctx| {
            a.begin_input(ctx);
            // Popup owned the input at frame start; it has since closed.
            a.menu_owned_input = true;
            a.compose(ctx);
            a.keyboard(ctx);
        },
    );
    assert_eq!(a.fields["delta0"], "unfinished");
    assert!(a.editor.session.live.is_some());
}
#[test]
fn m11_amendment_focused_disclosure_space_does_not_reach_global_drop() {
    let ctx = Context::default();
    let mut a = box_app();
    a.activate(Tool::Move);
    let mut id = None;
    let _ = ctx.run(RawInput::default(), |ctx| {
        CentralPanel::default().show(ctx, |ui| {
            let r = theme::chrome::disclosure(ui, "test-group", "Primitives", true);
            id = Some(r.id);
            r.request_focus();
        });
    });
    assert_eq!(ctx.memory(|m| m.focused()), id);
    let _ = ctx.run(
        RawInput {
            events: vec![key(Key::Space)],
            ..Default::default()
        },
        |ctx| {
            a.begin_input(ctx);
            CentralPanel::default().show(ctx, |ui| {
                let r = theme::chrome::disclosure(ui, "test-group", "Primitives", true);
                assert!(r.clicked());
            });
            a.keyboard(ctx);
        },
    );
    assert!(
        a.editor.session.live.is_some(),
        "focused disclosure owns Space; no global Drop"
    );
}
#[test]
fn m15_literal_chords_remap_and_component_context() {
    assert!(input::matches_chord("Ctrl+D", Key::D, Modifiers::CTRL));
    assert!(!input::matches_chord("Ctrl+D", Key::D, Modifiers::MAC_CMD));
    assert!(input::chord("Numpad1").is_some());
    assert!(!input::available("Connected", Mode::Item, false));
    let mut a = box_app();
    let ctx = Context::default();
    a.bindings.insert("Move".into(), "Shift+M".into());
    assert!(a.shortcut("Move").contains("Shift+M"));
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![Event::Key {
            key: Key::M,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::SHIFT,
        }],
    );
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Move);
    a.run(Command::Drop);
    a.bindings.insert("Move".into(), String::new());
    assert_eq!(a.shortcut("Move"), "");
    a.execute("Move");
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Move);
}
#[test]
fn m15_native_digit_provenance_prevents_keypad_mode_alias() {
    for keypad in [false, true] {
        let mut a = box_app();
        let ctx = Context::default();
        a.native_numbers = vec![native::NumberKey {
            digit: 1,
            keypad,
            pressed: true,
        }];
        draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Num1)]);
        assert_eq!(
            a.editor.scene.selection.mode,
            if keypad { Mode::Polygon } else { Mode::Vertex }
        );
    }
}
#[test]
fn m15_keypad_views_toggle_opposites_without_component_mode_changes() {
    let mut a = box_app();
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    let p = a.viewport_rect.center();
    for expected in [View::Bottom, View::Top] {
        a.native_numbers = vec![native::NumberKey {
            digit: 1,
            keypad: true,
            pressed: true,
        }];
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![
                Event::PointerMoved(p),
                key(Key::Num1),
                Event::Key {
                    key: Key::Num1,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
        assert_eq!(a.camera.view, expected);
        assert_eq!(a.editor.scene.selection.mode, Mode::Polygon);
    }
}
#[test]
fn mode_keys_drop_an_open_transform_tool_first() {
    // Jason, 2026-11-05: in Polygon mode, W then 2 lands in Edge mode with
    // Move dropped, in one keypress. Rotate (E) and Scale (R) the same.
    for (tool_key, tool) in [
        (Key::W, Tool::Move),
        (Key::E, Tool::Rotate),
        (Key::R, Tool::Scale),
    ] {
        let mut a = box_app();
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces = vec![0];
        let ctx = Context::default();
        draw(&mut a, &ctx, vec2(1440., 900.), vec![key(tool_key)]);
        assert_eq!(a.editor.session.live.as_ref().map(|l| l.tool), Some(tool));
        a.native_numbers = vec![native::NumberKey {
            digit: 2,
            keypad: false,
            pressed: true,
        }];
        draw(&mut a, &ctx, vec2(1440., 900.), vec![key(Key::Num2)]);
        assert!(a.editor.session.live.is_none(), "{tool:?}: {}", a.status);
        assert_eq!(a.editor.scene.selection.mode, Mode::Edge, "{tool:?}");
    }
}

/// While any text field has focus, app shortcuts are off and the keys go to
/// the field: M doesn't open Set Material and Tab doesn't toggle Sub-D, even
/// for a field that never put itself on the app's list of text editors
/// (Jason, 2026-11-10: typing "10m" set a material).
#[test]
fn a_focused_text_field_turns_every_shortcut_off() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    let size = vec2(1440., 900.);
    let _ = draw(&mut a, &ctx, size, vec![]);
    let p = a.viewport_rect.center();
    let _ = draw(&mut a, &ctx, size, vec![Event::PointerMoved(p)]);
    let field = Id::new("unlisted-text-field");
    assert!(!a.text_editors.contains(&field));
    egui::text_edit::TextEditState::default().store(&ctx, field);
    ctx.memory_mut(|m| m.request_focus(field));
    assert!(a.text_editor_focused(&ctx));
    ctx.input_mut(|i| i.events = vec![key(Key::M)]);
    a.keyboard(&ctx);
    assert!(a.set_material.is_none(), "M reached Set Material");
    // Tab is not held back as the Sub-D shortcut.
    let mut raw = egui::RawInput {
        events: vec![key(Key::Tab)],
        ..Default::default()
    };
    eframe::App::raw_input_hook(&mut a, &ctx, &mut raw);
    assert!(a.held_tab.is_empty(), "Tab became a shortcut");
    // With the field let go, the same keys are shortcuts again.
    ctx.memory_mut(|m| m.surrender_focus(field));
    assert!(!a.text_editor_focused(&ctx));
    eframe::App::raw_input_hook(&mut a, &ctx, &mut raw);
    assert!(!a.held_tab.is_empty(), "Tab is bound to Sub-D");
    ctx.input_mut(|i| i.events = vec![key(Key::M)]);
    a.keyboard(&ctx);
    assert!(a.set_material.is_some(), "{}", a.status);
}

#[test]
fn modified_file_key_runs_the_layout_binding_except_in_focused_text() {
    let ctx = Context::default();
    let mut a = box_app();
    a.start_layouts();
    let chord = input::display_chord("Cmd+Alt+S");
    let layout = a.layouts.current.clone();
    a.set_layout_key("Scale", Some(&chord), &layout).unwrap();
    assert!(a.set_layout_key("Scale", Some("Cmd+S"), &layout).is_err());
    a.set_layout_key(crate::image_viewer_ui::SAVE, Some("Cmd+S"), &layout)
        .unwrap();

    draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    let press = |a: &mut Operandi, modifiers| {
        ctx.input_mut(|i| {
            i.events = vec![Event::Key {
                key: Key::S,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }]
        });
        a.keyboard(&ctx);
    };
    let cmd = super::tests::PLATFORM_CMD;
    press(&mut a, cmd | Modifiers::ALT);
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Scale);
    assert!(a.file_dialog.is_none());
    a.run(Command::Cancel);

    let field = Id::new("file-shortcut-text-field");
    egui::text_edit::TextEditState::default().store(&ctx, field);
    ctx.memory_mut(|m| m.request_focus(field));
    assert!(a.text_editor_focused(&ctx));
    press(&mut a, cmd | Modifiers::ALT);
    assert!(a.editor.session.live.is_none());
    assert!(a.file_dialog.is_none());
    press(&mut a, cmd);
    assert!(a.file_dialog.is_some());
}
