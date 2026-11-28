//! EA-012 numeric subset in the admitted row order, using shared field ownership.
use super::*;

use crate::form::Unit;
use crate::primitive_panel::{plain, plain_vector, polygon_type, uv_map, whole};
use crate::tool_form::{self as tf, Button, ToolFields, ToolForm};

/// Ellipsoid's Tool Properties (EA-012 numeric subset, admitted row order).
pub(super) struct EllipsoidFields;

impl ToolFields for EllipsoidFields {
    type P = EllipsoidParameters;
    fn params(app: &Operandi) -> Option<EllipsoidParameters> {
        app.editor
            .session
            .ellipsoid
            .as_ref()
            .map(|s| s.requested.clone())
    }
    fn form(app: &Operandi, _: &EllipsoidParameters) -> ToolForm<EllipsoidParameters> {
        type P = EllipsoidParameters;
        let mut f = ToolForm::new();
        f.title("Ellipsoid · numeric construction")
            .fields_from(plain_vector(
                "ell_position",
                "Position",
                |p: &P| p.position,
                |p, a, v| p.position[a] = v,
            ))
            .fields_from(plain_vector(
                "ell_radius",
                "Radius",
                |p: &P| p.radius,
                |p, a, v| p.radius[a] = v,
            ))
            .separator()
            .field(whole(
                "ell_sides",
                "Sides",
                |p: &P| p.sides,
                |p, n| p.sides = n,
            ))
            .field(whole(
                "ell_segments",
                "Segments",
                |p: &P| p.segments,
                |p, n| p.segments = n,
            ))
            .field(polygon_type(
                "ell-type",
                "Catmull-Clark",
                |p: &P| p.polygon,
                |p, v| p.polygon = v,
            ))
            .separator()
            .field(plain(
                "ell_top",
                "Bulge Top",
                Unit::Scalar,
                |p: &P| p.bulge_top,
                |p, v| p.bulge_top = v,
            ))
            .field(plain(
                "ell_side",
                "Side",
                Unit::Scalar,
                |p: &P| p.bulge_side,
                |p, v| p.bulge_side = v,
            ))
            .separator()
            .field(tf::axis(|p: &P| p.axis, |p, a| p.axis = a))
            .field(tf::check(
                "ell-uvs",
                "Make UVs",
                |p: &P| p.make_uvs,
                |p, b| p.make_uvs = b,
            ))
            .field(tf::check(
                "ell-compensation",
                "Distortion Compensation",
                |p: &P| p.compensation,
                |p, b| p.compensation = b,
            ))
            .field(plain(
                "ell_gaps",
                "Gaps %",
                Unit::Scalar,
                |p: &P| p.gaps,
                |p, v| p.gaps = v,
            ))
            .separator()
            // Handle Mode and Show Corner (EA-Q01) are left out until built
            // (UIC-117).
            .separator()
            .fields_from(plain_vector(
                "ell_min",
                "Min",
                |p: &P| p.min(),
                |p, a, v| {
                    let mut b = p.min();
                    b[a] = v;
                    p.set_min(b);
                },
            ))
            .fields_from(plain_vector(
                "ell_max",
                "Max",
                |p: &P| p.max(),
                |p, a, v| {
                    let mut b = p.max();
                    b[a] = v;
                    p.set_max(b);
                },
            ));
        // #35: the falloff shapes the Ellipsoid as it's made.
        crate::primitive_panel::shape_falloff_section(
            &mut f,
            app,
            |p: &P| p.falloff,
            |p| Some(&mut p.falloff),
            Some("Moving an ellipsoid's rings would change its shape"),
            Vec::new(),
            None,
        );
        let session = app.editor.session.ellipsoid.as_ref();
        let ready = app.field_errors.is_empty() && session.is_none_or(|s| s.error.is_none());
        f.wide(Button::command("Apply", Command::Apply).enabled_if(
            ready,
            if app.field_errors.is_empty() {
                session
                    .and_then(|s| s.error.clone())
                    .unwrap_or_else(|| "Nothing to apply yet".into())
            } else {
                crate::unavailable::FIELD_ERRORS.into()
            },
        ))
        .buttons(vec![
            Button::command("Drop", Command::Drop),
            Button::command("Cancel", Command::Cancel),
            Button::command("Reset", Command::ResetEllipsoid),
        ]);
        if let Some(e) = session.and_then(|s| s.error.as_ref()) {
            f.error(e);
        }
        if let Some(m) = session.and_then(|s| s.preview.as_ref()) {
            f.note(format!(
                "Preview cage: {} vertices / {} polygons",
                m.vertices.len(),
                m.faces.len()
            ));
        }
        // Ownership selector follows the specified panel, with the same
        // explicit single-map convention as the numeric construction service.
        f.field(uv_map(
            app,
            "ell-uv-map",
            |p: &P| p.selected_uv_map.clone(),
            |p, m| p.selected_uv_map = m,
        ))
        // Spec: EA-011 bounded profile
        .field(
            tf::choice(
                "ell-symmetry",
                "Numeric Symmetry",
                &[
                    (None, "Off"),
                    (Some(0), "X"),
                    (Some(1), "Y"),
                    (Some(2), "Z"),
                ],
                |p: &P| p.symmetry_axis,
                |p, s| p.symmetry_axis = s,
            )
            .tip(
                "Supported for the tested symmetric setup only; other symmetry setups may not work",
            ),
        )
        .note("World metres · signed semiaxes / named bounds · cage display only. Viewport drawing, handles, calibrated links/presets and evaluated subdivision remain incomplete.")
        .note("Unit commands: Ellipsoid Unit / Ellipsoid New Unit (Drop first). Modifier-button routes remain unverified.");
        f
    }
    fn publish(app: &mut Operandi, p: EllipsoidParameters) -> Result<()> {
        app.editor.dispatch(Command::ConfigureEllipsoid(p))
    }
    fn published(app: &mut Operandi) {
        app.status =
            "Ellipsoid numeric preview · Apply appends; Drop commits the draft; Cancel discards it"
                .into();
    }
    fn settled(app: &mut Operandi) -> bool {
        app.editor
            .session
            .ellipsoid
            .as_ref()
            .is_none_or(|s| s.error.is_none())
    }
    fn option_error_key(_: &Operandi, _: &str) -> Option<String> {
        Some("ell_gaps".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> Operandi {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.execute("Ellipsoid");
        a
    }
    #[test]
    fn ea_t06_t09_t11_fields_bounds_replacement_error_recovery_and_history() {
        let mut a = app();
        let base = a.editor.scene.clone();
        for (k, v) in [
            ("ell_position0", "1 m"),
            ("ell_position1", "-2 m"),
            ("ell_position2", "3 m"),
            ("ell_radius0", "2 m"),
            ("ell_radius1", "3 m"),
            ("ell_radius2", "4 m"),
            ("ell_sides", "8"),
            ("ell_segments", "4"),
            ("ell_top", "0.37"),
            ("ell_side", "0.81"),
        ] {
            a.fields.insert(k.into(), v.into());
        }
        assert!(a.confirm_fields());
        let p = a
            .editor
            .session
            .ellipsoid
            .as_ref()
            .unwrap()
            .requested
            .clone();
        assert_eq!(p.position, DVec3::new(1., -2., 3.));
        assert_eq!(p.radius, DVec3::new(2., 3., 4.));
        assert_eq!(p.bulge_top, 0.37);
        assert_eq!(p.bulge_side, 0.81);
        let mesh = a.editor.session.ellipsoid.as_ref().unwrap().preview.clone();
        a.fields.insert("ell_segments".into(), "4.5".into());
        a.execute("Apply");
        assert_eq!(a.editor.scene, base);
        assert!(a.field_errors.contains_key("ell_segments"));
        assert_eq!(a.editor.session.ellipsoid.as_ref().unwrap().preview, mesh);
        a.fields.insert("ell_segments".into(), "4".into());
        assert!(a.confirm_fields());
        assert!(a.field_errors.is_empty());
        for (key, value) in [
            ("ell_min0", "0 m"),
            ("ell_max1", "2 m"),
            ("ell_min2", "7 m"),
            ("ell_min2", "9 m"),
        ] {
            a.fields.insert(key.into(), value.into());
            assert!(a.confirm_fields());
        }
        let p = &a.editor.session.ellipsoid.as_ref().unwrap().requested;
        assert_eq!(p.position, DVec3::new(1.5, -1.5, 8.));
        assert_eq!(p.radius, DVec3::new(1.5, 3.5, -1.));
        assert_eq!(p.max().z, 7.);
        let expected = a.editor.ellipsoid_preview_scene().unwrap().unwrap();
        a.execute("Ellipsoid Apply");
        assert_eq!(a.editor.scene, expected);
        let checkpoints = a.editor.history.undo.len();
        a.execute("Drop");
        assert_eq!(a.editor.history.undo.len(), checkpoints);
        a.execute("Undo");
        assert_eq!(a.editor.scene, base);
        a.execute("Redo");
        assert_eq!(a.editor.scene, expected);
        assert!(a.editor.session.ellipsoid.is_none());
    }
    #[test]
    fn ea_t09_t11_signed_compensation_choice_correction_pending_text_and_cancel() {
        let mut a = app();
        a.fields.insert("ell_gaps".into(), "7".into());
        assert!(a.confirm_fields());
        let mut p = a
            .editor
            .session
            .ellipsoid
            .as_ref()
            .unwrap()
            .requested
            .clone();
        p.compensation = true;
        p.make_uvs = false;
        assert!(a.publish_form::<EllipsoidFields>(p, vec![]));
        a.fields.insert("ell_radius0".into(), "0 m".into());
        assert!(a.confirm_fields());
        let (mut p, keys) = a.form_candidate::<EllipsoidFields>().unwrap();
        p.make_uvs = true;
        assert!(!a.publish_form::<EllipsoidFields>(p, keys));
        let valid = a.editor.session.ellipsoid.as_ref().unwrap().preview.clone();
        let (mut p, keys) = a.form_candidate::<EllipsoidFields>().unwrap();
        p.compensation = false;
        assert!(a.publish_form::<EllipsoidFields>(p, keys));
        assert!(a.field_errors.is_empty());
        assert_ne!(a.editor.session.ellipsoid.as_ref().unwrap().preview, valid);
        a.fields
            .insert("ell_radius0".into(), "not a quantity".into());
        a.execute("Drop");
        assert!(a.editor.session.ellipsoid.is_some());
        a.execute("Cancel");
        assert!(a.editor.session.ellipsoid.is_none());
        assert!(a.field_errors.is_empty());
        assert!(a.editor.scene.active().unwrap().mesh.faces.is_empty());
    }
    #[test]
    fn ea_t01_t11_reset_minimum_clamps_units_and_failed_save_are_atomic() {
        let mut a = app();
        a.fields.insert("ell_sides".into(), "1".into());
        a.fields.insert("ell_segments".into(), "0".into());
        a.fields.insert("ell_top".into(), "-100".into());
        a.fields.insert("ell_side".into(), "-2".into());
        a.fields.insert("ell_gaps".into(), "60".into());
        assert!(a.confirm_fields());
        let p = &a.editor.session.ellipsoid.as_ref().unwrap().requested;
        assert_eq!(
            (p.sides, p.segments, p.bulge_top, p.bulge_side, p.gaps),
            (3, 2, -1.5, -1.5, 50.)
        );
        a.fields.insert("ell_top".into(), "bad".into());
        a.execute("Ellipsoid Reset");
        assert!(a.field_errors.is_empty());
        assert_eq!(
            a.editor.session.ellipsoid.as_ref().unwrap().requested,
            EllipsoidParameters::default()
        );
        // Unit insertion drops the open Ellipsoid first, then inserts.
        a.execute("Ellipsoid New Unit");
        assert!(a.editor.session.ellipsoid.is_none());
        assert_eq!(a.editor.scene.active().unwrap().name, "Ellipsoid");
        assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 266);
    }
    #[test]
    fn ea_t13_real_panel_option_routes_layout_and_focus_at_simulated_scales() {
        for scale in [1., 1.5, 2.] {
            let mut a = app();
            let ctx = Context::default();
            theme::install(&ctx);
            ctx.set_pixels_per_point(scale);
            let _ = ctx.run(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1100., 1000.))),
                    ..Default::default()
                },
                |ctx| {
                    CentralPanel::default().show(ctx, |ui| {
                        ui.set_max_width(230.);
                        a.parameters(ui);
                    });
                },
            );
            for key in [
                "ell_position0",
                "ell_radius2",
                "ell_sides",
                "ell_segments",
                "ell_top",
                "ell_side",
                "ell_gaps",
                "ell_min0",
                "ell_max2",
            ] {
                assert!(a.field_rects.contains_key(key), "{key}");
            }
            let menus = a.menu_model(&ctx);
            assert!(
                menus
                    .iter()
                    .flat_map(|m| m.leaves())
                    .any(|i| i.action == app_menu::line_action("Ellipsoid"))
            );
            assert!(
                menus
                    .iter()
                    .flat_map(|m| m.leaves())
                    .filter_map(|i| match i.action {
                        a if a == app_menu::line_action("app.undo") => {
                            Some((i.enabled, HistoryAction::Undo))
                        }
                        a if a == app_menu::line_action("app.redo") => {
                            Some((i.enabled, HistoryAction::Redo))
                        }
                        _ => None,
                    })
                    .all(|(enabled, action)| enabled
                        == a.editor.history_eligibility(action).is_available())
            );
            for polygon in [
                PolygonRepresentation::Face,
                PolygonRepresentation::Subdivs,
                PolygonRepresentation::CatmullClark,
            ] {
                for axis in 0..3 {
                    let mut p = a
                        .editor
                        .session
                        .ellipsoid
                        .as_ref()
                        .unwrap()
                        .requested
                        .clone();
                    p.polygon = polygon;
                    p.axis = axis;
                    assert!(a.publish_form::<EllipsoidFields>(p, vec![]));
                    assert!(
                        a.editor
                            .session
                            .ellipsoid
                            .as_ref()
                            .unwrap()
                            .preview
                            .as_ref()
                            .unwrap()
                            .attributes
                            .representations
                            .iter()
                            .all(|&t| t == polygon)
                    );
                }
            }
        }
    }
}
