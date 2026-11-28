//! Headless checks of topology tool properties and action state.
use super::*;
use crate::create_utility::{Utility, UtilityDrag, UtilityFields};
use crate::tool_form::{Bind, ToolFields};
use crate::topology_tools_ui::retopo_sample_is_spaced;
use operandi_core::{CurveKind, PolygonRepresentation, TackOptions};

#[test]
fn tack_checkboxes_bind_to_their_own_options() {
    let app = Operandi::empty();
    let defaults = TackOptions::default();
    let form = UtilityFields::form(&app, &Utility::Tack(defaults));
    for (key, initial, expected) in [
        (
            "tack_align",
            true,
            TackOptions {
                align: false,
                ..defaults
            },
        ),
        (
            "tack_connected",
            true,
            TackOptions {
                connected: false,
                ..defaults
            },
        ),
        (
            "tack_copy",
            false,
            TackOptions {
                copy: true,
                ..defaults
            },
        ),
        (
            "tack_bridge",
            false,
            TackOptions {
                bridge: true,
                ..defaults
            },
        ),
    ] {
        let field = form
            .fields()
            .into_iter()
            .find(|f| f.row.key == key)
            .unwrap();
        let Bind::Check { get, set } = &field.bind else {
            panic!("{key} should be a checkbox");
        };
        let mut utility = Utility::Tack(defaults);
        assert_eq!(get(&utility), initial, "{key}");
        set(&mut utility, !initial);
        assert_eq!(utility, Utility::Tack(expected), "{key}");
        assert_eq!(get(&utility), !initial, "{key}");
    }
}

#[test]
fn tack_numeric_fields_map_distance_percent_and_angle_to_options() {
    let mut app = Operandi::empty();
    app.run(Command::Mode(Mode::Polygon));
    app.execute("tack tool");
    app.fields.insert("tack_offset".into(), "-25 cm".into());
    app.fields.insert("tack_size".into(), "150 %".into());
    app.fields.insert("tack_twist".into(), "-90°".into());

    assert!(app.confirm_form::<UtilityFields>(), "{}", app.status);
    assert_eq!(
        app.utility,
        Some(Utility::Tack(TackOptions {
            offset: -0.25,
            size: 1.5,
            twist: -90.,
            ..TackOptions::default()
        }))
    );
    let utility = app.utility.unwrap();
    let form = UtilityFields::form(&app, &utility);
    let size = form
        .fields()
        .into_iter()
        .find(|f| f.row.key == "tack_size")
        .unwrap();
    let Bind::Number { get, .. } = &size.bind else {
        panic!("Size should be a number field");
    };
    assert_eq!(get(&utility), 150.);
}

#[test]
fn tack_rejects_nonpositive_size_without_publishing_pending_edits() {
    let mut app = Operandi::empty();
    app.run(Command::Mode(Mode::Polygon));
    app.execute("tack tool");
    let before = app.utility;
    assert!(matches!(before, Some(Utility::Tack(_))));
    app.fields.insert("tack_offset".into(), "2 m".into());
    for invalid in ["0 %", "-25 %"] {
        app.fields.insert("tack_size".into(), invalid.into());
        assert!(!app.confirm_form::<UtilityFields>());
        assert!(app.field_errors["tack_size"].contains("more than 0"));
        assert_eq!(app.utility, before);
    }
    app.fields.insert("tack_size".into(), "125 %".into());
    assert!(app.confirm_form::<UtilityFields>(), "{}", app.status);
    assert!(!app.field_errors.contains_key("tack_size"));
    assert_eq!(
        app.utility,
        Some(Utility::Tack(TackOptions {
            offset: 2.,
            size: 1.25,
            ..TackOptions::default()
        }))
    );
}

#[test]
fn only_the_topology_pen_preset_opens_topology_pen_gestures() {
    let mut app = tests::box_app();
    app.run(Command::Mode(Mode::Polygon));
    assert!(app.editor.session.preset.is_none());
    assert!(!app.topology_pen_open());
    app.execute("soft move");
    assert!(app.editor.session.preset.is_some(), "{}", app.status);
    assert!(!app.topology_pen_open());
    app.execute("topology pen");
    assert!(app.topology_pen_open(), "{}", app.status);
    app.run(Command::Drop);
    assert!(!app.topology_pen_open());
}

#[test]
fn retopo_sketch_accepts_first_sample_and_samples_at_least_three_cm_apart() {
    let last = DVec3::ZERO;
    assert!(retopo_sample_is_spaced(None, last));
    assert!(!retopo_sample_is_spaced(Some(&last), last));
    assert!(!retopo_sample_is_spaced(Some(&last), DVec3::Z * 0.029));
    assert!(retopo_sample_is_spaced(Some(&last), DVec3::Z * 0.03));
    assert!(retopo_sample_is_spaced(Some(&last), DVec3::Z * 0.031));
}

#[test]
fn retopo_sketch_clears_the_previous_stroke_and_discards_short_strokes() {
    let mut app = Operandi::empty();
    let before = app.editor.scene.clone();
    let undo = app.editor.history.undo.len();
    app.retopo_stroke = vec![DVec3::X, DVec3::Y];
    let rect = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
    assert!(matches!(
        app.begin_retopo_sketch(rect.center(), rect),
        input::GestureKind::Utility(UtilityDrag::RetopoSketch)
    ));
    assert!(app.retopo_stroke.is_empty(), "there is no background hit");
    app.retopo_sketch_release();
    assert!(app.status.contains("drag across a background mesh"));
    app.retopo_stroke.push(DVec3::X);
    app.retopo_sketch_release();
    assert!(app.status.contains("drag further"));
    assert!(app.retopo_stroke.is_empty());
    assert_eq!(app.editor.scene, before);
    assert_eq!(app.editor.history.undo.len(), undo);
}

#[test]
fn retopo_sketch_passes_ordered_points_and_open_curve_options_to_the_edit() {
    let mut app = tests::box_app();
    let points = [DVec3::new(2., 1., 0.), DVec3::new(1., 2., 0.)];
    let before = app.editor.scene.clone();
    let undo = app.editor.history.undo.len();
    let face = before.active().unwrap().mesh.faces.len();
    app.retopo_stroke = points.to_vec();
    app.retopo_sketch_release();

    let mesh = &app.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.faces.len(), face + 1, "{}", app.status);
    assert_eq!(
        mesh.attributes.representations[face],
        PolygonRepresentation::Polyline {
            closed: false,
            curve: CurveKind::Curve {
                start_control: false,
                end_control: false,
            },
        }
    );
    let actual: Vec<_> = mesh.faces[face]
        .vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .collect();
    assert_eq!(actual, points);
    assert!(app.retopo_stroke.is_empty());
    assert_eq!(app.editor.history.undo.len(), undo + 1);
    assert_eq!(
        app.editor.history.undo.last().unwrap().label,
        "Retopo Sketch"
    );
    let after = app.editor.scene.clone();
    app.run(Command::Undo);
    assert_eq!(app.editor.scene, before);
    app.run(Command::Redo);
    assert_eq!(app.editor.scene, after);
}

#[test]
fn retopo_guide_commands_require_a_polygon_selection() {
    let mut app = tests::box_app();
    for (command, label) in [
        ("retopo edge lock", "Retopo Edge Lock"),
        ("retopo edge flow", "Retopo Edge Flow"),
    ] {
        for (mode, faces) in [(Mode::Polygon, vec![]), (Mode::Edge, vec![0])] {
            app.editor.scene.selection.mode = mode;
            app.editor.scene.selection.faces = faces;
            let before = app.editor.scene.clone();
            let undo = app.editor.history.undo.len();
            app.execute(command);
            assert!(app.status.starts_with(label), "{}", app.status);
            assert!(app.status.contains("select the guide curves first"));
            assert_eq!(app.editor.scene, before);
            assert_eq!(app.editor.history.undo.len(), undo);
        }
    }
}
