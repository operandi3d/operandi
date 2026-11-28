//! The command line: tool IDs, attributes, help topics and drafts.
use super::tests::{box_app, draw, key, pen_command};
use super::*;

#[test]
fn command_catalog_activation_query_and_targeted_off() {
    for (expected, (canonical, alias)) in [
        ("prim.cube", "cube"),
        ("prim.sphere", "sphere"),
        ("prim.cylinder", "cylinder"),
        ("prim.ellipsoid", "ellipsoid"),
        ("prim.cone", "cone"),
        ("poly.bevel", "bevel"),
        ("poly.extrude", "extrude"),
        ("poly.smshift", "smooth shift"),
        ("Thicken", "thicken"),
        ("poly.spikey", "spike"),
        ("Transform", "transform"),
        ("TransformMove", "move"),
        ("TransformRotate", "rotate"),
        ("TransformScale", "scale"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut a = box_app();
        let mut b = box_app();
        a.editor.scene.selection.faces = vec![5];
        b.editor.scene.selection.faces = vec![5];
        a.execute(&format!("tool.set {canonical} on"));
        b.execute(alias);
        assert_eq!(a.editor.scene, b.editor.scene, "{canonical}");
        assert_eq!(
            a.command_active(),
            Some(expected),
            "{canonical}: {}",
            a.status
        );
        assert_eq!(a.command_active(), b.command_active(), "{canonical}");
        assert_eq!(a.editor.history.undo.len(), b.editor.history.undo.len());
        assert_eq!(a.status, b.status);
        a.execute(&format!("tool.set {canonical} on"));
        b.execute(alias);
        assert_eq!(a.command_active(), b.command_active(), "repeat {canonical}");
        assert_eq!(a.editor.scene, b.editor.scene);
        a.execute(&format!("tool.set preset:{canonical} mode:?"));
        assert_eq!(
            a.status,
            if a.command_active().is_some() {
                "1"
            } else {
                "0"
            }
        );
    }
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.execute("cube");
    let scene = a.editor.scene.clone();
    a.execute("tool.set prim.sphere off");
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.command_active(), Some(0));
    a.execute("tool.set prim.cube off");
    assert_eq!(a.command_active(), None);
}
#[test]
fn command_surface_units_and_readonly_errors() {
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.execute("thicken");
    assert_eq!(a.command_active(), Some(8));
    a.execute("tool.attr poly.smshift scale 0.5");
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .smooth_shift
            .scale,
        0.5
    );
    a.execute("tool.attr poly.smshift scale 50%");
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .smooth_shift
            .scale,
        0.5
    );
    a.execute("tool.attr poly.smshift shift {0.1m + 100mm}");
    assert!(
        (a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .smooth_shift
            .offset
            - 0.2)
            .abs()
            < 1e-9
    );
    let scene = a.editor.scene.clone();
    let active = a.command_active();
    for text in [
        "help",
        "tool.set Thicken ?",
        "tool.attr poly.smshift scale ?",
        "tool.set Thicken mode:on extra",
        "tool.reset prim.cube",
    ] {
        a.execute(text);
        assert_eq!(a.editor.scene, scene);
        assert_eq!(a.command_active(), active);
    }
    // tool.doApply is Modo's Apply Tool: the same as tool.apply (UIC-165).
    let undo = a.editor.history.undo.len();
    a.execute("tool.doApply");
    assert!(a.editor.history.undo.len() > undo, "{}", a.status);
    assert_eq!(a.command_active(), active, "the tool stays active");
}
#[test]
fn review_regressions_before_fixes() {
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.execute("spike");
    assert!(a.editor.session.spike.is_some());
    a.fields.insert("spi_strength".into(), "0".into());
    a.confirmed_fields.insert("spi_strength".into(), "0".into());
    a.execute("tool.attr poly.spikey factor 0.25");
    assert_eq!(a.fields.get("spi_strength").unwrap(), "0.25");
    let mut p = Operandi::empty();
    p.run(Command::NewMesh);
    p.execute(&pen_command("vertices", false).replacen("pen ", "OPERANDI.PEN.CREATE\t", 1));
    assert_eq!(p.editor.scene.active().unwrap().mesh.vertices.len(), 4);
    a.execute("help tool.attr");
    assert!(a.status.starts_with("Help: tool.attr"));
}
#[test]
fn review_surface_fields_all_attributes_and_drafts() {
    for tool in ["smooth shift", "thicken"] {
        let mut a = box_app();
        a.editor.scene.selection.faces = vec![5];
        a.execute(tool);
        assert!(
            a.editor
                .session
                .live
                .as_ref()
                .unwrap()
                .tool
                .is_smooth_shift()
        );
        for (key, text) in [
            ("sst_offset", "0"),
            ("sst_scale", "100"),
            ("sst_angle", "0"),
        ] {
            a.fields.insert(key.into(), text.into());
            a.confirmed_fields.insert(key.into(), text.into());
        }
        for (attr, value, key) in [
            ("shift", "0.2m", "sst_offset"),
            ("scale", "0.5", "sst_scale"),
            ("maxAngle", "30deg", "sst_angle"),
        ] {
            a.execute(&format!("tool.attr poly.smshift {attr} {value}"));
            assert_eq!(a.status, "Ready");
            assert_eq!(a.fields.get(key), a.confirmed_fields.get(key));
            let field = a.fields[key].clone();
            a.execute(&format!("tool.attr poly.smshift {attr} ?"));
            assert!(!a.status.is_empty());
            assert_eq!(a.fields[key], field);
        }
        for attr in ["thicken", "sharp", "angleWeight", "bothSides"] {
            a.execute(&format!("tool.attr poly.smshift {attr} on"));
            a.execute(&format!("tool.attr poly.smshift {attr} ?"));
            assert_eq!(a.status, "1");
        }
        let before = a.editor.scene.clone();
        let old = a.editor.session.live.as_ref().unwrap().params.smooth_shift;
        a.fields.insert("sst_offset".into(), "bad draft".into());
        for cmd in [
            "help tool.attr",
            "tool.set Thicken ?",
            "tool.attr poly.smshift scale ?",
            "tool.attr poly.smshift scale nonsense",
            "tool.set prim.cube off",
            "tool.set {broken",
        ] {
            a.execute(cmd);
            assert_eq!(a.fields["sst_offset"], "bad draft");
            assert_eq!(a.editor.scene, before);
        }
        a.execute("tool.attr poly.smshift shift 0.4m");
        assert_eq!(
            a.editor.session.live.as_ref().unwrap().params.smooth_shift,
            old
        );
    }
}
#[test]
fn review_pen_aliases_and_help_topics() {
    for (kind, flag) in [
        ("polygon", false),
        ("vertices", false),
        ("polyline", false),
        ("polyline", true),
    ] {
        let mut a = Operandi::empty();
        let mut b = Operandi::empty();
        a.run(Command::NewMesh);
        b.run(Command::NewMesh);
        let legacy = pen_command(kind, flag);
        a.execute(&legacy);
        b.execute(&legacy.replacen("pen ", "OPERANDI.PEN.CREATE\t", 1));
        assert_eq!(a.status, "Ready");
        assert_eq!(a.editor.scene, b.editor.scene);
        assert_eq!(a.editor.history.undo.len(), b.editor.history.undo.len());
    }
    let mut a = Operandi::empty();
    for (topic, expected) in [
        ("tool.set", "box"),
        ("tool.attr", "maxAngle"),
        ("tool.reset", "resets"),
        ("tool.apply", "tool.apply"),
        ("tool.drop", "tool.drop"),
        ("operandi.pen.create", "JSON"),
    ] {
        a.execute(&format!("help {topic}"));
        assert!(a.status.contains(expected), "{}", a.status);
        if topic == "tool.set" {
            assert!(a.status.contains("smooth shift, poly.smshift"));
        }
    }
}
#[test]
fn review_help_layout_and_pen_correction() {
    for width in [640., 1100.] {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        theme::install(&ctx);
        a.execute("help tool.set");
        let out = draw(&mut a, &ctx, vec2(width, 720.), vec![]);
        fn has_help(shape: &egui::Shape) -> bool {
            match shape {
                egui::Shape::Text(t) => {
                    t.galley.job.text.contains("prim.cube")
                        && t.galley.job.text.contains("poly.smshift")
                }
                egui::Shape::Vec(v) => v.iter().any(has_help),
                _ => false,
            }
        }
        assert!(
            out.shapes.iter().any(|s| has_help(&s.shape)),
            "help text missing at {width}"
        );
        assert!(a.viewport_rect.height() < 650.);
    }
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let ctx = Context::default();
    theme::install(&ctx);
    draw(&mut a, &ctx, vec2(1100., 720.), vec![]);
    a.command = "OPERANDI.PEN.CREATE\t{invalid}".into();
    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    draw(&mut a, &ctx, vec2(1100., 720.), vec![key(Key::Enter)]);
    assert_eq!(a.command, "OPERANDI.PEN.CREATE\t{invalid}");
    a.command = pen_command("vertices", false).replacen("pen ", "OPERANDI.PEN.CREATE\t", 1);
    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    draw(&mut a, &ctx, vec2(1100., 720.), vec![key(Key::Enter)]);
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 4);
}
#[test]
fn review_parser_drafts_and_lifecycle() {
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.execute("smooth shift");
    a.fields.insert("sst_offset".into(), "invalid".into());
    a.confirmed_fields.insert("sst_offset".into(), "0".into());
    a.name = "Pending Name".into();
    let before = a.editor.scene.clone();
    let undo = a.editor.history.undo.len();
    for cmd in [
        "tool.set mode:? preset:poly.smshift",
        "tool.set {poly.smshift} ?",
        "help tool.drop",
        "tool.set poly.smshift on extra",
        "tool.set preset:poly.smshift preset:Thicken mode:on",
        "tool.set foo:on",
        "tool.set {open",
        "tool.attr poly.smshift unknown ?",
        "tool.reset prim.cube",
    ] {
        a.execute(cmd);
        assert_eq!(a.editor.scene, before);
        assert_eq!(a.editor.history.undo.len(), undo);
        assert_eq!(a.fields["sst_offset"], "invalid");
        assert_eq!(a.name, "Pending Name");
    }
    let mut a = box_app();
    let mut b = box_app();
    a.editor.scene.selection.faces = vec![5];
    b.editor.scene.selection.faces = vec![5];
    a.execute("smooth shift");
    b.execute("smooth shift");
    a.execute("tool.attr poly.smshift shift 0.2m");
    b.execute("smooth shift offset 0.2m");
    a.execute("tool.apply");
    b.execute("apply");
    assert_eq!(a.editor.scene, b.editor.scene);
    assert_eq!(a.editor.history.undo.len(), b.editor.history.undo.len());
    a.execute("tool.drop");
    b.execute("drop");
    assert_eq!(a.editor.scene, b.editor.scene);
    assert_eq!(a.editor.history.undo.len(), b.editor.history.undo.len());
    a.execute("undo");
    b.execute("undo");
    assert_eq!(a.editor.scene, b.editor.scene);
}
#[test]
fn acceptance_identity_queries_off_and_pending_fields() {
    let ids = [
        "prim.cube",
        "prim.sphere",
        "prim.cylinder",
        "prim.ellipsoid",
        "prim.cone",
        "poly.bevel",
        "poly.extrude",
        "poly.smshift",
        "Thicken",
        "poly.spikey",
        "Transform",
        "TransformMove",
        "TransformRotate",
        "TransformScale",
    ];
    for (idx, id) in ids.iter().enumerate() {
        let mut idle = box_app();
        idle.fields.insert("sst_offset".into(), "bad".into());
        idle.confirmed_fields
            .insert("sst_offset".into(), "0".into());
        idle.field_errors
            .insert("sst_offset".into(), "error".into());
        idle.name = "Name Draft".into();
        let before = idle.editor.clone();
        let fields = idle.fields.clone();
        let confirmed = idle.confirmed_fields.clone();
        let errors = idle.field_errors.clone();
        idle.execute(&format!("tool.set {id} ?"));
        assert_eq!(idle.status, "0");
        idle.execute(&format!("tool.set {id} off"));
        assert!(idle.editor == before);
        assert_eq!(idle.fields, fields);
        assert_eq!(idle.confirmed_fields, confirmed);
        assert_eq!(idle.field_errors, errors);
        assert_eq!(idle.name, "Name Draft");
        let mut a = box_app();
        a.editor.scene.selection.faces = vec![5];
        a.execute(&format!("tool.set {id} on"));
        assert_eq!(a.command_active(), Some(idx), "{id}: {}", a.status);
        a.fields.insert("draft".into(), "bad".into());
        a.confirmed_fields.insert("draft".into(), "1".into());
        a.field_errors.insert("draft".into(), "error".into());
        a.name = "Name Draft".into();
        let before = a.editor.clone();
        let fields = a.fields.clone();
        let confirmed = a.confirmed_fields.clone();
        let errors = a.field_errors.clone();
        a.execute(&format!("tool.set {id} ?"));
        assert_eq!(a.status, "1");
        let other = ids[(idx + 1) % ids.len()];
        a.execute(&format!("tool.set {other} ?"));
        assert_eq!(a.status, "0");
        a.execute(&format!("tool.set {other} off"));
        assert!(a.editor == before);
        assert_eq!(a.fields, fields);
        assert_eq!(a.confirmed_fields, confirmed);
        assert_eq!(a.field_errors, errors);
        assert_eq!(a.name, "Name Draft");
        a.fields.clear();
        a.confirmed_fields.clear();
        a.field_errors.clear();
        let mut b = box_app();
        b.editor.scene.selection.faces = vec![5];
        b.execute(&format!("tool.set {id} on"));
        b.editor = a.editor.clone();
        a.execute(&format!("tool.set {id} off"));
        b.run(Command::Drop);
        assert!(a.editor == b.editor, "off {id}");
    }
}
#[test]
fn acceptance_properties_expected_values_and_pending_drafts() {
    for tool in ["smooth shift", "thicken"] {
        let mut a = box_app();
        a.editor.scene.selection.faces = vec![5];
        a.execute(tool);
        for (k, v) in [
            ("sst_offset", "0 m"),
            ("sst_scale", "100 %"),
            ("sst_angle", "0 °"),
        ] {
            a.fields.insert(k.into(), v.into());
            a.confirmed_fields.insert(k.into(), v.into());
        }
        for (attr, value, key, expect, query) in [
            ("shift", "-0.2m", "sst_offset", "-0.2 m", "-0.2 m"),
            ("scale", "0.5", "sst_scale", "50 %", "0.5"),
            ("maxAngle", "30deg", "sst_angle", "30 °", "30 deg"),
        ] {
            a.execute(&format!("tool.attr poly.smshift {attr} {value}"));
            assert_eq!(a.status, "Ready");
            assert_eq!(a.fields[key], expect);
            assert_eq!(a.confirmed_fields[key], expect);
            a.execute(&format!("tool.attr poly.smshift {attr} ?"));
            assert_eq!(a.status, query);
        }
        a.fields.insert("sst_offset".into(), "-0.3 m".into());
        assert!(a.confirm_fields());
        assert_eq!(
            a.editor
                .session
                .live
                .as_ref()
                .unwrap()
                .params
                .smooth_shift
                .offset,
            -0.3
        );
        a.fields.insert("sst_offset".into(), "-0.4 m".into());
        a.execute("tool.attr poly.smshift scale 50%");
        assert_eq!(
            a.editor
                .session
                .live
                .as_ref()
                .unwrap()
                .params
                .smooth_shift
                .offset,
            -0.4
        );
        assert_eq!(a.fields["sst_scale"], "50 %");
        for attr in ["thicken", "sharp", "angleWeight", "bothSides"] {
            a.execute(&format!("tool.attr poly.smshift {attr} on"));
            a.execute(&format!("tool.attr poly.smshift {attr} ?"));
            assert_eq!(a.status, "1");
            a.execute(&format!("tool.attr poly.smshift {attr} off"));
            a.execute(&format!("tool.attr poly.smshift {attr} ?"));
            assert_eq!(a.status, "0");
        }
        a.fields.insert("sst_offset".into(), "bad".into());
        let before = a.editor.clone();
        a.execute("tool.attr poly.smshift scale 0.8");
        assert!(a.editor == before);
        assert_eq!(a.fields["sst_offset"], "bad");
    }
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.execute("spike");
    assert!(a.editor.session.spike.is_some());
    a.fields.insert("spi_strength".into(), "0".into());
    a.confirmed_fields.insert("spi_strength".into(), "0".into());
    a.fields.insert("spi_strength".into(), "0.1".into());
    a.execute("tool.attr poly.spikey factor 0.25");
    assert_eq!(a.editor.session.spike.as_ref().unwrap().requested, 0.25);
    assert_eq!(a.fields["spi_strength"], "0.25");
    assert_eq!(a.confirmed_fields["spi_strength"], "0.25");
    a.execute("tool.attr poly.spikey factor ?");
    assert_eq!(a.status, "0.25");
    a.fields.insert("spi_strength".into(), "0.4".into());
    assert!(a.confirm_fields());
    assert_eq!(a.editor.session.spike.as_ref().unwrap().requested, 0.4);
    a.fields.insert("spi_strength".into(), "bad".into());
    let before = a.editor.clone();
    a.execute("tool.attr poly.spikey factor 0.8");
    assert!(a.editor == before);
    assert_eq!(a.fields["spi_strength"], "bad");
    let mut p = Operandi::empty();
    p.run(Command::NewMesh);
    let before = p.editor.clone();
    p.execute("OPERANDI.PEN.CREATE\t{\"POSITIONS_M\":[],\"type\":\"VERTICES\",\"flip\":false,\"close\":false,\"select_new\":true}");
    assert!(p.editor == before);
    assert!(p.status.contains("invalid"));
}
#[test]
fn acceptance_reset_and_lifecycle_shared_routes() {
    for (id, alias, reset) in [
        ("prim.ellipsoid", "ellipsoid", Command::ResetEllipsoid),
        ("poly.bevel", "bevel", Command::ResetBevel),
        ("poly.extrude", "extrude", Command::ResetExtrude),
        ("poly.smshift", "smooth shift", Command::ResetSmoothShift),
        ("Thicken", "thicken", Command::ResetSmoothShift),
        ("poly.spikey", "spike", Command::ResetSpike),
    ] {
        let mut a = box_app();
        a.editor.scene.selection.faces = vec![5];
        a.execute(alias);
        assert!(a.command_active().is_some(), "{id}");
        let mut b = box_app();
        b.editor = a.editor.clone();
        a.execute(&format!("tool.reset {id}"));
        b.run(reset);
        // MN-05.4: tool.reset is its own named step, so compare what it
        // did rather than the tool's undo history.
        assert!(a.editor.scene == b.editor.scene, "reset {id}");
        assert_eq!(
            a.editor.session.active_task(),
            b.editor.session.active_task(),
            "{id}"
        );
        assert_eq!(
            a.editor.session.live.as_ref().map(|l| &l.params),
            b.editor.session.live.as_ref().map(|l| &l.params),
            "{id}"
        );
        let before = a.editor.clone();
        a.execute("tool.reset prim.cube");
        assert!(a.editor == before, "wrong reset {id}");
    }
    for phase in [0, 1, 2, 3] {
        let mut a = Operandi::empty();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/i2/expectations/graphs/G0.json");
        a.editor.scene = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        a.saved = a.editor.scene.clone();
        a.run(Command::Activate(Tool::Extrude));
        a.run(Command::ConfigureExtrude(Parameters {
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            ..Default::default()
        }));
        assert_eq!(a.status, "Ready", "arm/configure phase {phase}");
        assert_eq!(a.command_active(), Some(6));
        assert!(a.editor.i2.selected.is_none());
        if phase > 0 {
            let (_, ticket) = i2_route::capture(&mut a.editor, 2, true).unwrap();
            assert!(ticket.is_some(), "begin must acquire a stage ticket");
            assert_eq!(a.editor.i2.selected.as_ref().unwrap().cursor, 1);
            assert_eq!(
                a.editor.history_eligibility(HistoryAction::Undo).label,
                Some("Begin Extrude")
            );
        }
        if phase > 1 {
            let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
            p.extent = DVec3::Z * 0.5;
            a.run(Command::ConfigureExtrude(p));
            assert_eq!(a.status, "Ready", "movement must succeed");
            let stage = a.editor.i2.selected.as_ref().unwrap();
            assert_eq!(stage.cursor, 2);
            assert!(stage.movement.is_some());
            assert_eq!(
                a.editor.session.live.as_ref().unwrap().params.extent,
                DVec3::Z * 0.5
            );
            assert_eq!(
                a.editor.history_eligibility(HistoryAction::Undo).label,
                Some("Move Extrude")
            );
        }
        if phase > 2 {
            a.run(Command::Undo);
            assert_eq!(a.status, "Ready");
            assert_eq!(a.editor.i2.selected.as_ref().unwrap().cursor, 1);
            assert_eq!(
                a.editor.history_eligibility(HistoryAction::Redo).label,
                Some("Move Extrude")
            );
            a.run(Command::Undo);
            assert_eq!(a.status, "Ready");
            let stage = a.editor.i2.selected.as_ref().unwrap();
            assert_eq!(stage.cursor, 0);
            assert!(stage.ever_parked && stage.movement.is_some());
            assert_eq!(
                a.editor.history_eligibility(HistoryAction::Redo).label,
                Some("Begin Extrude")
            );
            assert_eq!(a.editor.scene, stage.base.scene);
        }
        let expected_labels = match phase {
            // The Extrude settings change is a tweak of the armed tool.
            0 => (Some("Extrude"), None),
            1 => (Some("Begin Extrude"), None),
            2 => (Some("Move Extrude"), None),
            _ => (None, Some("Begin Extrude")),
        };
        assert_eq!(
            a.editor.history_eligibility(HistoryAction::Undo).label,
            expected_labels.0,
            "undo phase {phase}"
        );
        assert_eq!(
            a.editor.history_eligibility(HistoryAction::Redo).label,
            expected_labels.1,
            "redo phase {phase}"
        );
        a.editor.scene.validate().unwrap();
        a.editor
            .identities
            .current
            .validate(&a.editor.scene)
            .unwrap();
        for (text, typed) in [("tool.apply", Command::Apply), ("tool.drop", Command::Drop)] {
            let mut b = Operandi::empty();
            b.editor = a.editor.clone();
            let mut c = Operandi::empty();
            c.editor = a.editor.clone();
            b.execute(text);
            c.run(typed);
            assert_eq!(b.editor.scene, c.editor.scene, "phase {phase} {text}");
            assert_eq!(b.editor.history.undo.len(), c.editor.history.undo.len());
            assert_eq!(b.editor.history.redo.len(), c.editor.history.redo.len());
            assert_eq!(b.command_active(), c.command_active());
            assert_eq!(b.status, c.status);
            for action in [HistoryAction::Undo, HistoryAction::Redo] {
                assert_eq!(
                    b.editor.history_eligibility(action),
                    c.editor.history_eligibility(action)
                );
            }
            let stage_state = |e: &Editor| {
                e.i2.selected.as_ref().map(|s| {
                    (
                        s.cursor,
                        s.ever_parked,
                        s.movement.is_some(),
                        s.anchor,
                        s.begun.scene.clone(),
                        s.movement.as_ref().map(|m| m.scene.clone()),
                    )
                })
            };
            assert_eq!(
                stage_state(&b.editor),
                stage_state(&c.editor),
                "I2 state {phase} {text}"
            );
            for result in [&b.editor, &c.editor] {
                result.scene.validate().unwrap();
                result.identities.current.validate(&result.scene).unwrap();
            }
            if b.status != "Ready" {
                assert!(
                    b.editor == a.editor && c.editor == a.editor,
                    "refusal must preserve the shared base"
                );
            }
        }
    }
}
#[test]
fn acceptance_surface_typed_equivalence_and_switch_confirmation() {
    for tool in ["smooth shift", "thicken"] {
        for (attr, value) in [
            ("shift", "-0.2m"),
            ("scale", "0.5"),
            ("maxAngle", "30deg"),
            ("thicken", "on"),
            ("sharp", "on"),
            ("angleWeight", "on"),
            ("bothSides", "on"),
        ] {
            let mut a = box_app();
            a.editor.scene.selection.faces = vec![5];
            a.execute(tool);
            let mut b = Operandi::empty();
            b.editor = a.editor.clone();
            a.execute(&format!("tool.attr poly.smshift {attr} {value}"));
            let mut p = b.editor.session.live.as_ref().unwrap().params.smooth_shift;
            match attr {
                "shift" => p.offset = -0.2,
                "scale" => p.scale = 0.5,
                "maxAngle" => p.angle = 30.,
                "thicken" => p.thicken = true,
                "sharp" => p.sharp = true,
                "angleWeight" => p.angle_weight = true,
                _ => p.both_sides = true,
            }
            b.run(Command::ConfigureSmoothShift(p));
            assert!(a.editor == b.editor, "{tool} {attr}");
        }
    }
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.execute("smooth shift");
    let mut b = Operandi::empty();
    b.editor = a.editor.clone();
    for x in [&mut a, &mut b] {
        x.fields.insert("sst_offset".into(), "0.2m".into());
        x.confirmed_fields.insert("sst_offset".into(), "0 m".into());
    }
    a.execute("tool.set TransformMove on");
    b.execute("move");
    assert!(a.editor == b.editor);
    assert_eq!(a.fields, b.fields);
    assert_eq!(a.status, b.status);
    let before = a.editor.clone();
    a.fields.insert("delta0".into(), "invalid".into());
    a.confirmed_fields.insert("delta0".into(), "0".into());
    a.execute("tool.set prim.cube on");
    assert!(a.editor == before);
    assert_eq!(a.fields["delta0"], "invalid");
}
#[test]
fn acceptance_help_scroll_reaches_end() {
    for width in [640., 1100.] {
        for topic in [
            "tool.set",
            "tool.attr",
            "tool.reset",
            "tool.apply",
            "tool.drop",
            "operandi.pen.create",
        ] {
            let mut a = Operandi::empty();
            let ctx = Context::default();
            theme::install(&ctx);
            a.execute(&format!("help {topic}"));
            let size = vec2(width, 720.);
            draw(&mut a, &ctx, size, vec![]);
            let pos = pos2(a.viewport_rect.center().x, a.viewport_rect.bottom() + 25.);
            let mut out = draw(
                &mut a,
                &ctx,
                size,
                vec![
                    Event::PointerMoved(pos),
                    Event::MouseWheel {
                        unit: MouseWheelUnit::Point,
                        delta: vec2(0., -2000.),
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
            // egui scrolls smoothly over a few frames; a narrow view wraps
            // the help into more lines.
            for _ in 0..30 {
                out = draw(&mut a, &ctx, size, vec![]);
            }
            let mut found = false;
            for s in out.shapes {
                if let egui::Shape::Text(t) = s.shape
                    && t.galley.job.text == a.status
                {
                    found = true;
                    let bottom = t.pos.y + t.galley.size().y;
                    assert!(
                        bottom <= s.clip_rect.bottom() + 2.,
                        "{width} {topic}: {bottom} vs {}",
                        s.clip_rect.bottom()
                    );
                }
            }
            assert!(found, "missing {width} {topic}");
        }
    }
}
#[test]
fn acceptance_spike_legacy_scale_and_off_drafts() {
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.execute("spike");
    let mut b = Operandi::empty();
    b.editor = a.editor.clone();
    a.execute("tool.attr poly.spikey factor 0.25");
    b.run(Command::ConfigureSpike(0.25));
    assert!(a.editor == b.editor);
    a.fields.insert("spi_strength".into(), "bad".into());
    a.confirmed_fields
        .insert("spi_strength".into(), "0.25".into());
    let before = a.editor.clone();
    a.execute("tool.set poly.spikey off");
    assert!(a.editor == before);
    assert_eq!(a.fields["spi_strength"], "bad");
    a.fields.insert("spi_strength".into(), "0.3".into());
    b.editor = a.editor.clone();
    b.fields = a.fields.clone();
    b.confirmed_fields = a.confirmed_fields.clone();
    b.field_errors = a.field_errors.clone();
    a.execute("tool.set poly.spikey off");
    b.run(Command::Drop);
    assert!(a.editor == b.editor);
    for tool in ["smooth shift", "thicken"] {
        let mut x = box_app();
        x.editor.scene.selection.faces = vec![5];
        x.execute(tool);
        let mut y = Operandi::empty();
        y.editor = x.editor.clone();
        x.execute("tool.attr poly.smshift scale 0.5");
        y.execute(&format!("{tool} scale 50"));
        assert!(x.editor == y.editor);
    }
}
