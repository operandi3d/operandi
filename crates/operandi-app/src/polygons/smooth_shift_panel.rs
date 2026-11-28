//! SST-1 shared panel and text transport. Geometry/lifecycle remain in core.
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, MenuEntry, ToolFields, ToolForm};

/// Smooth Shift / Thicken's Tool Properties (SST-1).
pub(super) struct SurfaceFields;

fn surface(app: &Operandi) -> Option<&Live> {
    app.editor
        .session
        .live
        .as_ref()
        .filter(|l| l.tool.is_smooth_shift())
}

impl ToolFields for SurfaceFields {
    type P = SmoothShiftParameters;
    fn params(app: &Operandi) -> Option<SmoothShiftParameters> {
        surface(app).map(|l| l.params.smooth_shift)
    }
    fn form(app: &Operandi, _: &SmoothShiftParameters) -> ToolForm<SmoothShiftParameters> {
        type P = SmoothShiftParameters;
        let mut f = ToolForm::new();
        let Some(l) = surface(app) else {
            return f;
        };
        f.title("Smooth Shift")
            .field(tf::number(
                "sst_offset",
                "Offset",
                Unit::Distance,
                |p: &P| p.offset,
                |p, v| {
                    p.offset = v;
                    Ok(())
                },
            ))
            .field(tf::percent(
                "sst_scale",
                "Scale",
                " %",
                |p: &P| p.scale,
                |p, v| {
                    p.scale = v / 100.;
                    Ok(())
                },
            ))
            .field(
                tf::number(
                    "sst_angle",
                    "Max. Smoothing Angle",
                    Unit::Angle,
                    |p: &P| p.angle,
                    |p, v| {
                        p.angle = v;
                        Ok(())
                    },
                )
                .suffix(" °"),
            )
            .field(tf::check(
                "sst_thicken",
                "Thicken",
                |p: &P| p.thicken,
                |p, on| p.thicken = on,
            ))
            .field(tf::check(
                "sst_sharp",
                "Sharp",
                |p: &P| p.sharp,
                |p, on| p.sharp = on,
            ))
            .field(tf::check(
                "sst_angle_weight",
                "Polygon Angle Weighting",
                |p: &P| p.angle_weight,
                |p, on| p.angle_weight = on,
            ))
            .field(tf::check(
                "sst_both_sides",
                "Both Sides",
                |p: &P| p.both_sides,
                |p, on| p.both_sides = on,
            ))
            .wide(
                tf::Button::command("Apply", Command::Apply)
                    .tip("Apply one independent stage while armed; unavailable while interactive")
                    .enabled_if(
                        !l.applied && app.field_errors.is_empty(),
                        if l.applied {
                            "Applied; change a value to apply again"
                        } else {
                            crate::unavailable::FIELD_ERRORS
                        },
                    ),
            );
        let key = l.tool.label();
        let current = app.editor.surface_state.get(key).cloned();
        let mut states = vec![MenuEntry::new("(none)", current.is_none(), |app| {
            app.run(Command::SurfaceStateRecall(None))
        })];
        for name in app
            .editor
            .surface_states
            .get(key)
            .into_iter()
            .flat_map(|m| m.keys())
        {
            let picked = name.clone();
            states.push(MenuEntry::new(
                name,
                current.as_ref() == Some(name),
                move |app| {
                    app.run(Command::SurfaceStateRecall(Some(picked.clone())));
                    app.clear_fields();
                },
            ));
        }
        let some = current.is_some();
        let name = |app: &Operandi| app.draft_text(STATE_NAME, "State 1");
        f.section(
            "Tool States",
            true,
            vec![
                tf::Item::Menu {
                    key: "surface_states".into(),
                    label: "Tool State".into(),
                    shown: current.unwrap_or_else(|| "(none)".into()),
                    entries: states,
                    disabled: None,
                },
                tf::Item::Field(Box::new(tf::draft(STATE_NAME, "Name", "State 1"))),
                tf::Item::Buttons(vec![
                    tf::Button::run("Save", move |app| {
                        let n = name(app);
                        app.run(Command::SurfaceStateSave(n))
                    }),
                    tf::Button::command("Delete", Command::SurfaceStateDelete)
                        .enabled_if(some, "Choose a saved state first"),
                    tf::Button::run("Rename", move |app| {
                        let n = name(app);
                        app.run(Command::SurfaceStateRename(n))
                    })
                    .enabled_if(some, "Choose a saved state first"),
                    tf::Button::run("Reset", |app| {
                        app.run(Command::ResetSmoothShift);
                        app.clear_fields();
                    }),
                ]),
            ],
        )
        .buttons(vec![
            tf::Button::command("Drop", Command::Drop),
            tf::Button::command("Cancel", Command::Cancel),
            tf::Button::command("Continue", Command::Continue),
            tf::Button::run("Clone", |app| {
                app.run(Command::CloneSmoothShift);
                app.clear_fields();
            }),
        ]);
        if let Some(e) = &l.preview_error {
            f.error(e.clone());
        }
        f
    }
    fn publish(app: &mut Operandi, p: SmoothShiftParameters) -> Result<()> {
        app.editor.dispatch(Command::ConfigureSmoothShift(p))
    }
}

/// The Tool State name field's key.
const STATE_NAME: &str = "surface_state_name";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{draw, pointer};
    fn app(tool: Tool) -> Operandi {
        let mut a = Operandi::empty();
        a.editor.dispatch(Command::NewMesh).unwrap();
        a.editor.scene.active_mut().unwrap().mesh = operandi_core::Mesh {
            vertices: vec![
                DVec3::new(-1., -1., 0.),
                DVec3::new(1., -1., 0.),
                DVec3::new(1., 1., 0.),
                DVec3::new(-1., 1., 0.),
            ],
            faces: vec![operandi_core::Face {
                vertices: vec![0, 1, 2, 3],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            }]
            .into(),
            ..Default::default()
        }
        .into();
        a.category = toolbox::Category::Polygon;
        a.camera.view = View::Perspective;
        a.frame(false);
        a.activate(tool);
        a
    }
    #[test]
    fn sst_fields_commands_all_controls_and_panel_at_scales() {
        for tool in [Tool::SmoothShift, Tool::Thicken] {
            let mut a = app(tool);
            let ctx = Context::default();
            theme::install(&ctx);
            let source = a.editor.scene.clone();
            for (name, value) in [
                ("offset", "0.1m + 100mm"),
                ("scale", "50%"),
                ("angle", "90.1 deg"),
                ("thicken", "off"),
                ("sharp", "on"),
                ("angle_weight", "on"),
                ("both_sides", "on"),
            ] {
                a.execute(&format!("{} {name} {value}", tool.label()));
                assert_eq!(a.status, "Ready");
            }
            assert_eq!(a.editor.scene, source);
            let p = a.editor.session.live.as_ref().unwrap().params.smooth_shift;
            assert_eq!(p.offset, 0.2);
            assert_eq!(p.scale, 0.5);
            assert!(p.sharp && p.angle_weight && p.both_sides);
            assert!(!p.thicken);
            for size in [vec2(1100., 720.), vec2(1920., 1080.), vec2(2560., 1440.)] {
                draw(&mut a, &ctx, size, vec![]);
                for key in ["sst_offset", "sst_scale", "sst_angle"] {
                    assert!(a.field_rects.contains_key(key));
                }
                assert!(a.toolbox_rects.contains_key(tool.label()));
            }
            a.fields.insert("sst_offset".into(), "1/0".into());
            a.run(Command::Apply);
            assert_eq!(a.editor.scene, source);
            a.fields.insert("sst_offset".into(), "0.2m".into());
            a.run(Command::Apply);
            assert_ne!(a.editor.scene, source);
            assert!(!a.editor.session.live.as_ref().unwrap().applied);
            assert_eq!(a.editor.history.undo.last().unwrap().label, tool.label());
            a.execute("tool properties");
            assert!(a.surface_mini_properties);
            a.execute("Drop");
            let committed = a.editor.scene.clone();
            a.execute("Undo");
            assert_eq!(a.editor.scene, source);
            a.execute("Redo");
            assert_eq!(a.editor.scene, committed);
        }
    }
    #[test]
    fn sst_stationary_click_replacement_handles_continue_clone_cancel() {
        for tool in [Tool::SmoothShift, Tool::Thicken] {
            let mut a = app(tool);
            let ctx = Context::default();
            theme::install(&ctx);
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
            let source = a.editor.scene.clone();
            let empty = a.viewport_rect.left_top() + vec2(20., 60.);
            draw(
                &mut a,
                &ctx,
                vec2(1440., 900.),
                vec![
                    pointer(empty, true, Modifiers::NONE),
                    pointer(empty, false, Modifiers::NONE),
                ],
            );
            assert!(a.editor.session.live.as_ref().unwrap().applied);
            assert_ne!(a.editor.scene, source);
            a.execute("smooth shift offset 0.2m");
            let first = a.editor.scene.clone();
            a.execute("smooth shift offset 0.1m");
            a.execute("smooth shift offset 0.2m");
            assert_eq!(a.editor.scene, first);
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
            let h = a
                .handles(a.viewport_rect)
                .into_iter()
                .find(|h| h.0 == 0)
                .unwrap();
            let end = h.2 + vec2(20., -15.);
            draw(
                &mut a,
                &ctx,
                vec2(1440., 900.),
                vec![
                    pointer(h.2, true, Modifiers::NONE),
                    Event::PointerMoved(end),
                    pointer(end, false, Modifiers::NONE),
                ],
            );
            assert_ne!(a.editor.scene, first);
            a.run(Command::Cancel);
            assert_eq!(a.editor.scene, source);
            // Reset continuation and retained clone are separate operations.
            a.activate(tool);
            a.execute("smooth shift offset 0.1m");
            let p = a.editor.session.live.as_ref().unwrap().params.clone();
            a.run(Command::Preview(p));
            let stage = a.editor.scene.clone();
            a.run(Command::CloneSmoothShift);
            assert!(a.editor.session.live.as_ref().unwrap().applied);
            assert_eq!(
                a.editor
                    .session
                    .live
                    .as_ref()
                    .unwrap()
                    .params
                    .smooth_shift
                    .offset,
                0.1
            );
            a.run(Command::Cancel);
            assert_eq!(a.editor.scene, stage);
            a.run(Command::Undo);
            assert_eq!(a.editor.scene, source);
        }
    }
}
