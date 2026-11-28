//! Thin native-handle adapter. The domain command owns topology/history; this
//! only establishes a fresh, revision-checked capture after a successful Begin.
use operandi_core::*;
pub(super) fn capture(
    editor: &mut Editor,
    axis: usize,
    unmodified: bool,
) -> Result<(Parameters, Option<ExtrudeStageTicket>)> {
    let live = editor.session.live.as_ref().ok_or("No live handle owner")?;
    if live.tool == Tool::Extrude
        && axis == 2
        && unmodified
        && !live.applied
        && editor.i2_begin_source_supported()
    {
        editor.dispatch(Command::BeginExtrude)?;
    }
    let params = editor
        .session
        .live
        .as_ref()
        .ok_or("No live handle owner")?
        .params
        .clone();
    let ticket = if editor.i2_active() {
        Some(editor.extrude_stage_ticket()?)
    } else {
        None
    };
    Ok((params, ticket))
}
/// The production publication boundary for captured I2 movement. A UI
/// precheck alone cannot authorize publication after history/capture changes.
pub(super) fn move_captured(
    editor: &mut Editor,
    ticket: ExtrudeStageTicket,
    params: Parameters,
) -> Result<()> {
    editor.dispatch_extrude_stage(ticket, Command::Preview(params))
}
#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    fn editor() -> Editor {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../operandi-core/tests/fixtures/i1/cube-source.expected.json");
        let mut e = Editor {
            scene: serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap(),
            ..Default::default()
        };
        e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
        e.dispatch(Command::ConfigureExtrude(Parameters {
            extent: DVec3::Z * 0.5,
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            ..Default::default()
        }))
        .unwrap();
        e
    }
    #[test]
    fn i2_route_reset_before_capture_and_recapture_current_displacement() {
        let mut e = editor();
        let (p, t) = capture(&mut e, 2, true).unwrap();
        assert_eq!(p.extent, DVec3::ZERO);
        assert_eq!(e.scene.items[0].mesh.vertices.len(), 12);
        e.dispatch_extrude_stage(
            t.unwrap(),
            Command::Preview(Parameters {
                extent: DVec3::Z * 0.5,
                ..p
            }),
        )
        .unwrap();
        let (p, t) = capture(&mut e, 2, true).unwrap();
        assert_eq!(p.extent, DVec3::Z * 0.5);
        let old = t.unwrap();
        e.dispatch(Command::Undo).unwrap();
        assert!(e.dispatch_extrude_stage(old, Command::Preview(p)).is_err());
        // Every axis handle (and a free drag) may move an active stage.
        assert!(capture(&mut e, 0, true).is_ok());
    }
    #[test]
    fn i2_route_legacy_axis_does_not_begin() {
        let mut e = editor();
        let (p, t) = capture(&mut e, 0, true).unwrap();
        assert_eq!(p.extent, DVec3::Z * 0.5);
        assert!(t.is_none());
        assert!(e.i2.selected.is_none());
    }
}

#[cfg(test)]
mod app_boundary_tests {
    use super::*;
    fn app() -> crate::Operandi {
        let mut a = crate::Operandi::empty();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../operandi-core/tests/fixtures/i1/cube-source.expected.json");
        a.editor.scene = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        a.saved = a.editor.scene.clone();
        a.name = a.editor.scene.items[0].name.clone();
        a.run(Command::Activate(Tool::Extrude));
        a.run(Command::ConfigureExtrude(Parameters {
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            ..Default::default()
        }));
        a
    }
    #[test]
    fn i2_route_numeric_field_error_history_buffers_and_failed_save() {
        let mut a = app();
        capture(&mut a.editor, 2, true).unwrap();
        a.fields.insert("extent2".into(), "250 mm".into());
        assert!(a.confirm_fields());
        assert_eq!(
            a.editor.session.live.as_ref().unwrap().params.extent.z,
            0.25
        );
        assert_eq!(
            a.editor
                .presets
                .get::<operandi_core::ExtrudePreset>()
                .extent
                .z,
            0.25
        );
        a.fields.insert("extent2".into(), "abc".into());
        let before = a.editor.clone();
        a.run(Command::Undo);
        assert!(a.editor == before);
        assert_eq!(a.fields["extent2"], "abc");
        assert!(a.field_errors.contains_key("extent2"));
        a.fields.insert("extent2".into(), "250 mm".into());
        assert!(a.confirm_fields());
        a.run(Command::Undo);
        assert_eq!(a.editor.session.live.as_ref().unwrap().params.extent.z, 0.);
        assert!(a.fields.is_empty());
        a.run(Command::Redo);
        assert_eq!(
            a.editor.session.live.as_ref().unwrap().params.extent.z,
            0.25
        );
        a.fields.insert("extent2".into(), "0.5 m".into());
        let before = a.editor.clone();
        let fields = a.fields.clone();
        let confirmed = a.confirmed_fields.clone();
        let path = a.path.clone();
        let saved = a.saved.clone();
        // Exclusive task-owned directory plus an ordinary-file parent makes
        // failure deterministic even when earlier ignored QA output exists.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../development/i2-review-fixes/test-io");
        std::fs::create_dir_all(&root).unwrap();
        let directory = root.join(format!(
            "save-refusal-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let blocker = directory.join("not-a-directory");
        std::fs::write(&blocker, b"owned save obstruction").unwrap();
        assert!(!a.save(blocker.join("test.opscene")));
        assert_eq!(std::fs::read(&blocker).unwrap(), b"owned save obstruction");
        assert!(!blocker.join("test.opscene").exists());
        assert!(a.editor == before);
        assert_eq!(a.fields, fields);
        assert_eq!(a.confirmed_fields, confirmed);
        assert_eq!(a.path, path);
        assert_eq!(a.saved, saved);
        std::fs::remove_file(blocker).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn i2_route_unsupported_prebegin_profile_keeps_existing_handle_lifecycle() {
        let mut a = app();
        let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
        p.extrude_sides = 3;
        a.run(Command::ConfigureExtrude(p));
        let (_, ticket) = capture(&mut a.editor, 2, true).unwrap();
        assert!(ticket.is_none());
        assert!(a.editor.i2.selected.is_none());
    }
}

#[cfg(test)]
mod event_tests {
    use super::*;
    use crate::tests::{draw, pointer};
    use crate::*;
    #[test]
    fn i2_route_simulated_native_press_gain_focus_loss_history_and_escape() {
        let r = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/i2/expectations/graphs");
        let fixed = |name: &str| -> Scene {
            serde_json::from_slice(&std::fs::read(r.join(format!("{name}.json"))).unwrap()).unwrap()
        };
        let mut a = Operandi::empty();
        a.editor.scene = fixed("G0");
        a.saved = a.editor.scene.clone();
        a.name = a.editor.scene.items[0].name.clone();
        a.camera.view = View::Right;
        a.frame(false);
        a.run(Command::Activate(Tool::Extrude));
        a.run(Command::ConfigureExtrude(Parameters {
            extent: DVec3::Z * 0.5,
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            ..Default::default()
        }));
        let ctx = Context::default();
        theme::install(&ctx);
        let size = vec2(1440., 900.);
        draw(&mut a, &ctx, size, vec![]);
        // No gizmo until the first viewport click begins the extrusion.
        assert!(a.handles(a.viewport_rect).is_empty());
        let click = a.viewport_rect.left_top() + vec2(30., 60.);
        draw(
            &mut a,
            &ctx,
            size,
            vec![
                pointer(click, true, Modifiers::NONE),
                pointer(click, false, Modifiers::NONE),
            ],
        );
        assert_eq!(a.editor.scene, fixed("G1"), "{}", a.status);
        let start = a
            .handles(a.viewport_rect)
            .into_iter()
            .find(|h| h.0 == 2)
            .unwrap()
            .2;
        draw(
            &mut a,
            &ctx,
            size,
            vec![pointer(start, true, Modifiers::NONE)],
        );
        assert_eq!(a.editor.scene, fixed("G1"));
        assert_eq!(
            a.editor.session.live.as_ref().unwrap().params.extent,
            DVec3::ZERO
        );
        assert!(a.i2_gesture_ticket.is_some());
        let ticket = a.i2_gesture_ticket.unwrap();
        let request = a.editor.identities.request;
        draw(
            &mut a,
            &ctx,
            size,
            vec![Event::WindowFocused(false), Event::PointerGone],
        );
        assert!(a.navigation.is_none() && a.i2_gesture_ticket.is_none());
        assert_eq!(a.editor.identities.request, request + 1);
        assert!(a.editor.check_extrude_stage_ticket(ticket).is_err());
        assert_eq!(a.editor.scene, fixed("G1"));
        draw(&mut a, &ctx, size, vec![Event::WindowFocused(true)]);
        let start = a
            .handles(a.viewport_rect)
            .into_iter()
            .find(|h| h.0 == 2)
            .unwrap()
            .2;
        let end = start + vec2(-24., 0.);
        // Frozen before event delivery: Right projects world+Z toward screen-X.
        // This independent signed gain predicts every cap coordinate.
        let dz = 24.0 * a.camera.distance / (f64::from(a.viewport_rect.height()) * 0.9);
        let mut expected = fixed("G2");
        for v in &mut expected.items[0].mesh.vertices[8..] {
            v.z = 1.0 + dz;
        }
        draw(
            &mut a,
            &ctx,
            size,
            vec![
                pointer(start, true, Modifiers::NONE),
                Event::PointerMoved(end),
                pointer(end, false, Modifiers::NONE),
            ],
        );
        assert_eq!(a.editor.scene, expected, "{}", a.status);
        assert!(a.navigation.is_none() && a.i2_gesture_ticket.is_none());
        assert_eq!(a.editor.history.undo.len(), 0);
        let menus = a.menu_model(&ctx);
        let undo = menus
            .iter()
            .flat_map(|m| m.leaves())
            .find(|i| i.action == app_menu::line_action("app.undo"))
            .unwrap();
        assert!(undo.enabled && undo.title.starts_with("Undo Move Extrude"));
        a.run(Command::Undo);
        assert_eq!(a.editor.scene, fixed("G1"));
        a.run(Command::Undo);
        assert_eq!(a.editor.scene, fixed("G0"));
        a.run(Command::Redo);
        assert_eq!(a.editor.scene, fixed("G1"));
        a.run(Command::Redo);
        assert_eq!(a.editor.scene, expected);
        assert!(a.i2_gesture_ticket.is_none());
        let empty = a.viewport_rect.left_top() + vec2(20., 20.);
        draw(
            &mut a,
            &ctx,
            size,
            vec![
                Event::PointerMoved(empty),
                Event::Key {
                    key: Key::Escape,
                    physical_key: Some(Key::Escape),
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
        assert_eq!(a.editor.scene, expected);
        assert!(a.editor.session.live.is_none());
        assert_eq!(a.editor.history.undo.len(), 1);
    }
}

#[cfg(test)]
mod review_tests {
    use super::*;
    use crate::tests::draw;
    use crate::*;
    fn app() -> Operandi {
        let mut a = Operandi::empty();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/i2/expectations/graphs/G0.json");
        a.editor.scene = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        a.saved = a.editor.scene.clone();
        a.name = a.editor.scene.items[0].name.clone();
        a.run(Command::Activate(Tool::Extrude));
        a.run(Command::ConfigureExtrude(Parameters {
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            ..Default::default()
        }));
        a
    }
    fn park(a: &mut Operandi) {
        capture(&mut a.editor, 2, true).unwrap();
        let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
        p.extent = DVec3::Z * 0.5;
        a.run(Command::ConfigureExtrude(p));
        a.run(Command::Undo);
        a.run(Command::Undo);
    }
    #[test]
    fn rf02_partial_profiles_fall_back_without_begin_or_allocation() {
        for kind in [
            "extent_x",
            "extent_y",
            "vertex_mode",
            "nonplanar",
            "concave",
            "falloff",
            "missing_map",
        ] {
            let mut a = app();
            match kind {
                "extent_x" => a.editor.session.live.as_mut().unwrap().params.extent.x = 0.25,
                "extent_y" => a.editor.session.live.as_mut().unwrap().params.extent.y = -0.25,
                "vertex_mode" => {
                    a.editor.scene.selection.mode = Mode::Vertex;
                    a.editor.session.live.as_mut().unwrap().base.selection.mode = Mode::Vertex;
                }
                "nonplanar" | "concave" => {
                    let f = a.editor.scene.items[0].mesh.faces
                        [a.editor.scene.selection.faces[0] as usize]
                        .vertices
                        .clone();
                    let mesh = &mut a.editor.scene.items[0].mesh;
                    if kind == "nonplanar" {
                        mesh.vertices[f[3] as usize].z += 0.25;
                    } else {
                        mesh.vertices[f[2] as usize] = (mesh.vertices[f[0] as usize]
                            + mesh.vertices[f[1] as usize]
                            + mesh.vertices[f[3] as usize])
                            / 3.;
                    }
                    a.editor.session.live.as_mut().unwrap().base = a.editor.scene.clone();
                }
                "falloff" => {
                    a.editor.spatial_falloff = Some(Falloff::Linear(LinearFalloff {
                        start: DVec3::ZERO,
                        end: DVec3::X,
                        ..Default::default()
                    }))
                }
                _ => {
                    a.editor.scene.items[0].mesh.uv_maps.remove("Texture");
                    a.editor.session.live.as_mut().unwrap().base = a.editor.scene.clone();
                }
            }
            // Match the independently written legacy path, including a kernel
            // refusal. Fallback itself must be wholly observational.
            let before = a.editor.clone();
            let marks = (
                a.editor.identities.next_id().unwrap(),
                a.editor.i2.serial.next().unwrap(),
            );
            let (p, t) = capture(&mut a.editor, 2, true).unwrap_or_else(|e| panic!("{kind}: {e}"));
            assert!(t.is_none(), "{kind}");
            assert!(a.editor == before, "{kind}");
            assert_eq!(
                (
                    a.editor.identities.next_id().unwrap(),
                    a.editor.i2.serial.next().unwrap()
                ),
                marks,
                "{kind}"
            );
            let mut legacy = before;
            let mut movement = p;
            movement.extent.z += 0.25;
            let expected = legacy.dispatch(Command::Preview(movement.clone()));
            let actual = a.editor.dispatch(Command::Preview(movement));
            assert_eq!(actual, expected, "legacy outcome {kind}");
            assert_eq!(a.editor.scene, legacy.scene, "legacy Scene {kind}");
            assert_eq!(
                a.editor
                    .session
                    .live
                    .as_ref()
                    .map(|l| (&l.base, &l.params, &l.preview_error)),
                legacy
                    .session
                    .live
                    .as_ref()
                    .map(|l| (&l.base, &l.params, &l.preview_error)),
                "legacy owner {kind}"
            );
            assert!(a.editor.i2.selected.is_none());
            assert_eq!(a.editor.history.undo.len(), legacy.history.undo.len());
            assert_eq!(a.editor.history.redo.len(), legacy.history.redo.len());
            // Clones intentionally share the nonrewindable allocation bank.
            // Consecutive legacy previews may allocate different runtime IDs;
            // exact capture nonmutation was asserted before either preview.
            if actual.is_ok() {
                a.editor
                    .identities
                    .current
                    .validate(&a.editor.scene)
                    .unwrap();
            }
        }
        let mut a = app();
        let (_, t) = capture(&mut a.editor, 2, true).unwrap();
        assert!(t.is_some());
        assert_eq!(a.editor.scene.items[0].mesh.vertices.len(), 12);
        assert!(a.editor.i2_active());
    }
    #[test]
    fn rf02_begin_budget_failure_propagates_without_legacy_fallback() {
        let mut a = app();
        a.editor.identities.request = u64::MAX;
        let before = a.editor.clone();
        assert!(capture(&mut a.editor, 2, true).is_err());
        assert!(a.editor == before);
        assert!(a.editor.i2.selected.is_none());
    }
    #[test]
    fn rf04_parked_stage_does_not_claim_armed_move_escape() {
        for parked in [false, true] {
            let mut a = app();
            if parked {
                park(&mut a);
            } else {
                a.run(Command::Drop);
            }
            a.run(Command::Activate(Tool::Move));
            let before = a.editor.clone();
            let ctx = Context::default();
            theme::install(&ctx);
            let size = vec2(1440., 900.);
            draw(&mut a, &ctx, size, vec![]);
            let point = a.viewport_rect.left_top() + vec2(20., 20.);
            draw(
                &mut a,
                &ctx,
                size,
                vec![
                    Event::PointerMoved(point),
                    Event::Key {
                        key: Key::Escape,
                        physical_key: Some(Key::Escape),
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
            // The parked stage doesn't take the Esc: it reaches the
            // general rule, which drops Move (drop_keys.rs, UIC-24) and
            // leaves the scene alone.
            assert!(a.editor.scene == before.scene, "parked={parked}");
            assert!(a.editor.session.live.is_none(), "parked={parked}");
        }
    }
    #[test]
    fn rf05_menu_agrees_with_parked_guards_and_keeps_document_redo() {
        let mut a = app();
        a.run(Command::Drop);
        a.run(Command::SelectPolygonSet("bottom".into()));
        a.run(Command::SelectPolygonSet("top".into()));
        a.run(Command::Activate(Tool::Extrude));
        capture(&mut a.editor, 2, true).unwrap();
        a.run(Command::Undo);
        a.run(Command::Undo);
        a.run(Command::ActionContext(ActionContext {
            center: CenterMode::Origin,
            ..Default::default()
        }));
        let ctx = Context::default();
        let redo = |a: &Operandi| {
            a.menu_model(&ctx)
                .into_iter()
                .flat_map(|m| m.leaves().into_iter().cloned().collect::<Vec<_>>())
                .find(|i| i.action == app_menu::line_action("app.redo"))
                .unwrap()
        };
        assert!(redo(&a).enabled);
        a.run(Command::Redo);
        assert_eq!(a.editor.i2.selected.as_ref().unwrap().cursor, 0);
        assert!(!redo(&a).enabled);
    }
    #[test]
    fn rf06_idle_other_tool_history_preserves_existing_field_buffers() {
        let mut a = app();
        a.run(Command::Drop);
        a.run(Command::SelectPolygonSet("bottom".into()));
        a.fields.insert("delta0".into(), "0.125 m".into());
        a.confirmed_fields = a.fields.clone();
        let fields = a.fields.clone();
        a.run(Command::Undo);
        assert_eq!(a.fields, fields);
        a.run(Command::Redo);
        assert_eq!(a.fields, fields);
    }
    #[test]
    fn rf07_production_adapter_rejects_old_foreign_undo_and_resume_tickets() {
        for kind in [
            "old",
            "foreign_process",
            "wrong_stage",
            "undo",
            "resume",
            "other_owner",
        ] {
            let mut a = app();
            let (p, t) = capture(&mut a.editor, 2, true).unwrap();
            let mut t = t.unwrap();
            let mut movement = p;
            movement.extent = DVec3::Z * 0.5;
            match kind {
                "old" => {
                    a.editor.extrude_stage_ticket().unwrap();
                }
                "foreign_process" => t.identity.process = t.identity.process.wrapping_add(1),
                "wrong_stage" => t.stage = t.stage.wrapping_add(1),
                "undo" => a.run(Command::Undo),
                "resume" => {
                    a.run(Command::Undo);
                    a.run(Command::Redo);
                }
                "other_owner" => {
                    a.run(Command::Drop);
                    a.run(Command::Activate(Tool::Move));
                }
                _ => unreachable!(),
            }
            let before = a.editor.clone();
            let marks = format!("{:?}", a.editor.identities);
            let serial = a.editor.i2.serial.next().unwrap();
            assert!(move_captured(&mut a.editor, t, movement).is_err(), "{kind}");
            assert!(a.editor == before, "{kind}");
            assert_eq!(format!("{:?}", a.editor.identities), marks, "{kind}");
            assert_eq!(a.editor.i2.serial.next().unwrap(), serial, "{kind}");
        }
        let mut a = app();
        let (mut p, t) = capture(&mut a.editor, 2, true).unwrap();
        p.extent = DVec3::Z * 0.25;
        move_captured(&mut a.editor, t.unwrap(), p).unwrap();
        assert_eq!(a.editor.i2.selected.as_ref().unwrap().cursor, 2);
        assert_eq!(a.editor.history.undo.len(), 0);
        // Ordinary numeric confirmation still uses its existing semantic path.
        a.fields.insert("extent2".into(), "0.5 m".into());
        assert!(a.confirm_fields());
        assert_eq!(a.editor.session.live.as_ref().unwrap().params.extent.z, 0.5);
    }

    #[test]
    fn rf07_actual_viewport_boundary_refuses_stale_capture_without_clearing_draft() {
        for phase in ["old", "foreign", "undo", "resume", "other_owner"] {
            let mut a = app();
            let (base, t) = capture(&mut a.editor, 2, true).unwrap();
            let mut t = t.unwrap();
            match phase {
                "old" => {
                    a.editor.extrude_stage_ticket().unwrap();
                }
                "foreign" => t.identity.process = t.identity.process.wrapping_add(1),
                "undo" => a.run(Command::Undo),
                "resume" => {
                    a.run(Command::Undo);
                    a.run(Command::Redo);
                }
                _ => {
                    a.run(Command::Drop);
                    a.run(Command::Activate(Tool::Move));
                }
            }
            a.i2_gesture_ticket = Some(t);
            a.fields.insert("extent2".into(), "unconfirmed text".into());
            let before = a.editor.clone();
            let bank = format!("{:?}", a.editor.identities);
            let fields = a.fields.clone();
            a.update_handle(
                2,
                base,
                pos2(100., 100.),
                pos2(124., 100.),
                Tool::Extrude,
                Rect::from_min_size(Pos2::ZERO, vec2(1440., 900.)),
            );
            assert!(a.editor == before, "{phase}");
            assert_eq!(format!("{:?}", a.editor.identities), bank, "{phase}");
            assert_eq!(a.fields, fields, "{phase}");
        }
    }
    #[test]
    fn rf02_begin_preview_error_is_not_reclassified_as_legacy_fallback() {
        let mut a = app();
        a.editor.session.live.as_mut().unwrap().preview_error =
            Some("existing preview error".into());
        let before = a.editor.clone();
        let bank = format!("{:?}", a.editor.identities);
        assert!(capture(&mut a.editor, 2, true).is_err());
        assert!(a.editor == before);
        assert_eq!(format!("{:?}", a.editor.identities), bank);
    }
    #[test]
    fn rf06_other_tool_cancel_invalidates_capture_and_retains_cancel_semantics() {
        let mut a = app();
        a.run(Command::Drop);
        a.run(Command::Activate(Tool::Move));
        let base = a.editor.scene.clone();
        let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
        p.delta.x = 0.25;
        a.run(Command::Preview(p.clone()));
        a.navigation = Some(input::Navigation {
            kind: input::GestureKind::Handle(0, p, Tool::Move),
            start: Pos2::ZERO,
            last: pos2(24., 0.),
            modifiers: Modifiers::NONE,
            button: PointerButton::Primary,
        });
        a.fields.insert("delta0".into(), "0.25 m".into());
        a.run(Command::Cancel);
        assert!(a.navigation.is_none());
        assert!(a.editor.session.live.is_none());
        assert_eq!(a.editor.scene, base);
        assert!(a.fields.is_empty());
        assert!(a.editor.history.undo.is_empty());
    }

    fn outcome_test_navigation() -> input::Navigation {
        input::Navigation {
            kind: input::GestureKind::Handle(0, Parameters::default(), Tool::Move),
            start: Pos2::ZERO,
            last: pos2(24., 0.),
            modifiers: Modifiers::NONE,
            button: PointerButton::Primary,
        }
    }
    #[test]
    fn stab03a_empty_history_success_releases_capture_without_clearing_idle_buffers() {
        for command in [Command::Undo, Command::Redo, Command::Cancel] {
            let mut a = app();
            let (_, ticket) = capture(&mut a.editor, 2, true).unwrap();
            a.run(Command::Cancel);
            a.run(Command::Drop);
            assert!(a.editor.history.undo.is_empty() && a.editor.history.redo.is_empty());
            a.navigation = Some(outcome_test_navigation());
            a.i2_gesture_ticket = ticket;
            a.fields.insert("delta0".into(), "0.125 m".into());
            a.confirmed_fields = a.fields.clone();
            let fields = a.fields.clone();
            a.run(command);
            assert!(a.navigation.is_none() && a.i2_gesture_ticket.is_none());
            assert_eq!(a.fields, fields);
            assert_eq!(a.status, "Ready");
        }
    }
    #[test]
    fn stab03a_parked_only_cancel_keeps_unrelated_buffers() {
        let mut a = app();
        park(&mut a);
        a.fields
            .insert("delta0".into(), "unrelated retained draft".into());
        let fields = a.fields.clone();
        a.navigation = Some(outcome_test_navigation());
        a.run(Command::Cancel);
        assert!(a.editor.i2.selected.is_none());
        assert!(a.navigation.is_none());
        assert_eq!(a.fields, fields);
        assert_eq!(a.status, "Ready");
    }
    #[test]
    fn stab03a_confirmation_and_requested_refusal_remain_separate() {
        let mut a = app();
        let (_, ticket) = capture(&mut a.editor, 2, true).unwrap();
        a.run(Command::Drop);
        a.run(Command::Activate(Tool::Move));
        a.navigation = Some(outcome_test_navigation());
        a.i2_gesture_ticket = ticket;
        a.fields.insert("delta0".into(), "abc".into());
        let before = a.editor.clone();
        a.run(Command::Undo);
        assert!(a.editor == before);
        assert!(a.field_errors.contains_key("delta0"));
        assert!(a.navigation.is_some() && a.i2_gesture_ticket == ticket);
        // A valid field is confirmed first; that confirmed offset is a tweak
        // of Move, and Undo takes it back with the tool still live (Jason,
        // 2026-11-04).
        a.fields.insert("delta0".into(), "0.125 m".into());
        a.run(Command::Undo);
        assert_eq!(a.editor.scene, before.scene, "{}", a.status);
        assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Move);
        assert!(!a.editor.session.live.as_ref().unwrap().applied);
        assert_eq!(a.status, "Undo · Move is still active");
        assert!(a.navigation.is_none() && a.i2_gesture_ticket.is_none());
        // Cancel bypasses invalid text and retains its established closure.
        a.fields.insert("delta0".into(), "abc".into());
        a.run(Command::Cancel);
        assert!(a.fields.is_empty());
        assert!(a.editor.session.live.is_none());
        assert_eq!(a.editor.scene, before.scene);
    }
}
