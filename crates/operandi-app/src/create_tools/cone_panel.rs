//! Bounded numeric Cone controls. No unverified draw/handle mapping is assigned.
use super::*;
use crate::form::Unit;
use crate::primitive_panel::{plain, plain_vector, polygon_type, uv_map, whole};
use crate::tool_form::{self as tf, Button, ToolFields, ToolForm};
use operandi_core::{ConeCap, ConeParameters, PolygonRepresentation};

/// Cone's Tool Properties (CA-D01).
pub(super) struct ConeFields;

impl ToolFields for ConeFields {
    type P = ConeParameters;
    fn params(app: &Operandi) -> Option<ConeParameters> {
        app.editor
            .session
            .cone
            .as_ref()
            .map(|s| s.requested.clone())
    }
    fn form(app: &Operandi, p: &ConeParameters) -> ToolForm<ConeParameters> {
        type P = ConeParameters;
        let label = |c: ConeCap| match c {
            ConeCap::None => "None",
            ConeCap::Single => "Single Polygon",
            ConeCap::QuadGrid => "Quad Grid",
            ConeCap::Radial => "Radial",
        };
        let caps = [
            ConeCap::None,
            ConeCap::Single,
            ConeCap::QuadGrid,
            ConeCap::Radial,
        ]
        .map(|c| (c, label(c)));
        let mut f = ToolForm::new();
        f.title("Cone · numeric construction")
            .fields_from(plain_vector(
                "cone_position",
                "Position",
                |p: &P| p.position,
                |p, a, v| p.position[a] = v,
            ))
            .fields_from(plain_vector(
                "cone_size",
                "Size",
                |p: &P| p.size,
                |p, a, v| p.size[a] = v,
            ))
            .field(whole(
                "cone_sides",
                "Sides",
                |p: &P| p.sides,
                |p, n| p.sides = n,
            ))
            .field(whole(
                "cone_segments",
                "Segments",
                |p: &P| p.segments,
                |p, n| p.segments = n,
            ))
            .field(polygon_type(
                "cone-type",
                "Catmull Clark",
                |p: &P| p.polygon,
                |p, v| p.polygon = v,
            ))
            .field(
                tf::choice(
                    "cone-top-cap",
                    "Top Cap",
                    &caps,
                    |p: &P| p.top_cap,
                    |p, c| p.top_cap = c,
                )
                // GitHub #18: the apex has no opening to fill, so say what
                // this choice changes there (cone-2026-11-05 "Top Cap").
                .tip(
                    "How the apex is built: None and Single Polygon make a fan of \
                     triangles; Quad Grid merges them in pairs into quads",
                )
                .greyed_option(3, "Not measured yet (cone spec Q-CN-03)"),
            )
            .field(
                tf::choice(
                    "cone-cap",
                    "Bottom Cap",
                    &caps,
                    |p: &P| p.bottom_cap,
                    |p, c| p.bottom_cap = c,
                )
                .tip("How the base opening is filled"),
            )
            .field(
                plain(
                    "cone_weight",
                    "Edge Weight %",
                    Unit::Scalar,
                    |p: &P| p.edge_weight,
                    |p, v| p.edge_weight = v,
                )
                .enabled_if(
                    p.polygon != PolygonRepresentation::Face,
                    "Face polygons have no edge weight; choose another Polygon type",
                ),
            )
            .field(tf::axis(|p: &P| p.axis, |p, a| p.axis = a))
            .separator()
            .heading("UVs")
            .field(tf::check(
                "cone-uvs",
                "Make UVs",
                |p: &P| p.make_uvs,
                |p, b| p.make_uvs = b,
            ))
            .field(tf::check(
                "cone-compensation",
                "Distortion Compensation",
                |p: &P| p.compensation,
                |p, b| p.compensation = b,
            ))
            .field(plain(
                "cone_gaps",
                "Gaps %",
                Unit::Scalar,
                |p: &P| p.gaps,
                |p, v| p.gaps = v,
            ))
            // Selected map ownership is explicit numeric-session state; no
            // scene-wide map inventory is confused with assignments in the
            // destination mesh.
            .field(uv_map(
                app,
                "cone-uv-map",
                |p: &P| p.selected_uv_map.clone(),
                |p, m| p.selected_uv_map = m,
            ))
            .separator()
            .heading("Bounds")
            .fields_from(plain_vector(
                "cone_min",
                "Min",
                |p: &P| p.min(),
                |p, a, v| {
                    let mut b = p.min();
                    b[a] = v;
                    p.set_min(b);
                },
            ))
            .fields_from(plain_vector(
                "cone_max",
                "Max",
                |p: &P| p.max(),
                |p, a, v| {
                    let mut b = p.max();
                    b[a] = v;
                    p.set_max(b);
                },
            ));
        // #35: the falloff shapes the Cone as it's made, as with the Cube.
        crate::primitive_panel::shape_falloff_section(
            &mut f,
            app,
            |p: &P| p.falloff,
            |p| Some(&mut p.falloff),
            None,
            Vec::new(),
            None,
        );
        let session = app.editor.session.cone.as_ref();
        if let Some(error) = session.and_then(|s| s.error.as_ref()) {
            f.error(error);
        }
        if let Some(mesh) = session.and_then(|s| s.preview.as_ref()) {
            f.note(format!(
                "Preview cage: {} vertices / {} polygons",
                mesh.vertices.len(),
                mesh.faces.len()
            ));
        }
        let ready = session.is_none_or(|s| s.error.is_none()) && app.field_errors.is_empty();
        f.buttons(vec![
            Button::command("Apply", Command::Apply).enabled_if(ready, if app.field_errors.is_empty() { session.and_then(|s| s.error.clone()).unwrap_or_else(|| "Nothing to apply yet".into()) } else { crate::unavailable::FIELD_ERRORS.into() }),
            Button::command("Drop", Command::Drop),
            Button::command("Cancel", Command::Cancel),
        ])
        // Spec: CA-D01; odd grids and gestures pending
        .note("Numeric settings only. Quad Grid supports even Sides (3 and 4 fall back). Odd grids and viewport dragging aren't available yet. No Bulge.");
        f
    }
    fn publish(app: &mut Operandi, p: ConeParameters) -> Result<()> {
        app.editor.dispatch(Command::ConfigureCone(p))
    }
    fn published(app: &mut Operandi) {
        app.status = "Cone numeric preview ready · Apply creates one result".into();
    }
    fn settled(app: &mut Operandi) -> bool {
        app.editor
            .session
            .cone
            .as_ref()
            .is_none_or(|s| s.error.is_none())
    }
    fn option_error_key(_: &Operandi, error: &str) -> Option<String> {
        Some(if error.contains("Quad Grid") {
            "cone_sides".into()
        } else {
            "cone_gaps".into()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_cone_commands_fields_invalid_uv_recovery_and_history() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.execute("Cone");
        assert!(a.editor.session.cone.is_some());
        assert!(a.editor.scene.active().unwrap().mesh.vertices.is_empty());
        for (key, value) in [
            ("cone_position0", "1 m"),
            ("cone_position1", "2 m"),
            ("cone_position2", "3 m"),
            ("cone_size2", "1 m"),
            ("cone_sides", "8"),
            ("cone_segments", "3"),
        ] {
            a.fields.insert(key.into(), value.into());
        }
        assert!(a.confirm_fields());
        let preview = a.editor.session.cone.as_ref().unwrap().preview.clone();
        a.fields.insert("cone_gaps".into(), "50".into());
        a.execute("Apply");
        assert!(a.editor.scene.active().unwrap().mesh.vertices.is_empty());
        assert_eq!(a.fields["cone_gaps"], "50");
        assert!(a.field_errors.contains_key("cone_gaps"));
        assert_eq!(a.editor.session.cone.as_ref().unwrap().preview, preview);
        a.execute("cone apply");
        assert!(a.editor.scene.active().unwrap().mesh.vertices.is_empty());
        a.fields.insert("cone_gaps".into(), "7".into());
        a.execute("Apply");
        assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 25);
        a.fields.insert("cone_min0".into(), "-2 m".into());
        assert!(a.confirm_fields());
        let p = &a.editor.session.cone.as_ref().unwrap().requested;
        assert_eq!(p.max().x, 1.5);
        assert_eq!(p.min().x, -2.);
        // The edited Min X shows a preview, so Drop keeps it as a Cone.
        a.execute("Drop");
        assert!(a.editor.session.cone.is_none());
        let dropped = a.editor.scene.clone();
        assert!(dropped.active().unwrap().mesh.vertices.len() > 25);
        a.execute("Undo");
        assert!(a.editor.session.cone.is_none());
        assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 25);
        a.execute("Undo");
        assert!(a.editor.scene.active().unwrap().mesh.vertices.is_empty());
        a.execute("Redo");
        assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 25);
        a.execute("Drop");
        a.execute("Redo");
        assert_eq!(a.editor.scene, dropped);
    }
    #[test]
    fn numeric_cone_invalid_text_keeps_ownership_and_cancel_discards_only_draft() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.execute("Cone");
        a.fields.insert("cone_sides".into(), "8.5".into());
        a.execute("Apply");
        assert!(a.field_errors.contains_key("cone_sides"));
        assert!(a.editor.scene.active().unwrap().mesh.vertices.is_empty());
        a.execute("Cancel");
        assert!(a.editor.session.cone.is_none());
        assert!(a.field_errors.is_empty());
        assert!(a.editor.scene.active().unwrap().mesh.vertices.is_empty());
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn correcting_uv_to_previously_confirmed_value_clears_error() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.execute("Cone");
        a.fields.insert("cone_gaps".into(), "0".into());
        assert!(a.confirm_fields());
        a.fields.insert("cone_gaps".into(), "50".into());
        assert!(!a.confirm_fields());
        a.fields.insert("cone_gaps".into(), "0".into());
        assert!(a.confirm_fields());
        assert!(a.field_errors.is_empty());
        assert!(a.editor.session.cone.as_ref().unwrap().error.is_none());
    }
}

#[cfg(test)]
mod quad_grid_tests {
    use super::*;
    #[test]
    fn quad_grid_field_error_recovery_and_shared_apply_route() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.execute("Cone");
        a.editor
            .dispatch(Command::ConfigureCone(ConeParameters {
                sides: 8,
                segments: 3,
                bottom_cap: ConeCap::QuadGrid,
                ..Default::default()
            }))
            .unwrap();
        let preview = a.editor.session.cone.as_ref().unwrap().preview.clone();
        // Odd sides are allowed since PRIM-K-02; a fraction is still refused.
        a.fields.insert("cone_sides".into(), "8.5".into());
        a.execute("Apply");
        assert!(a.field_errors.contains_key("cone_sides"));
        assert_eq!(a.editor.session.cone.as_ref().unwrap().preview, preview);
        assert!(a.editor.scene.active().unwrap().mesh.vertices.is_empty());
        a.fields.insert("cone_sides".into(), "8".into());
        a.execute("Cone Apply");
        let mesh = &a.editor.scene.active().unwrap().mesh;
        assert_eq!(
            (mesh.vertices.len(), mesh.edges().len(), mesh.faces.len()),
            (34, 68, 36)
        );
        let ids: std::collections::BTreeSet<_> = mesh.attributes.uv_records["Texture"]
            .iter()
            .flatten()
            .flatten()
            .collect();
        assert_eq!(ids.len(), 73);
        // Drop after Apply records nothing; Undo takes back the Apply.
        a.execute("Drop");
        assert!(a.editor.session.cone.is_none());
        a.execute("Undo");
        assert!(a.editor.scene.active().unwrap().mesh.vertices.is_empty());
    }
}
