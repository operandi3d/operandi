//! Name-map renames (command_names.rs): every old tool id, command verb and
//! argument name still works through `execute` and does exactly what the new
//! name does.
use super::tests::box_app;
use super::*;
use operandi_core::command_names::{COMMAND_RENAMES, TOOL_RENAMES};

/// Runs `old` on one copy of `setup` and `new` on another; both must end in
/// the same scene, active tool, status and history.
/// Boxed: the app is large and debug builds give each call its own stack
/// slot, which overflowed the test thread.
fn same(setup: impl Fn() -> Operandi, old: &str, new: &str) -> Box<Operandi> {
    let (mut a, mut b) = (Box::new(setup()), Box::new(setup()));
    a.execute(old);
    b.execute(new);
    assert_eq!(a.editor.scene, b.editor.scene, "{old} vs {new}");
    assert_eq!(a.command_active(), b.command_active(), "{old} vs {new}");
    assert_eq!(a.vertex_weld, b.vertex_weld, "{old} vs {new}");
    assert_eq!(
        a.editor.history.undo.len(),
        b.editor.history.undo.len(),
        "{old} vs {new}"
    );
    assert_eq!(a.status, b.status, "{old} vs {new}");
    b
}

fn edge_app() -> Operandi {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(0, 1), (2, 3)];
    a
}

fn vertex_app() -> Operandi {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![0, 1];
    a
}

#[test]
fn old_tool_ids_start_the_same_tool() {
    for (old, new) in TOOL_RENAMES {
        for setup in [box_app, edge_app, vertex_app] {
            same(
                setup,
                &format!("tool.set {old} on"),
                &format!("tool.set {new} on"),
            );
            // The query answers under either name.
            let (mut a, mut b) = (Box::new(setup()), Box::new(setup()));
            a.execute(&format!("tool.set {new} on"));
            b.execute(&format!("tool.set {new} on"));
            a.execute(&format!("tool.set {old} ?"));
            b.execute(&format!("tool.set {new} ?"));
            assert_eq!(a.status, b.status, "{old} ?");
        }
    }
    // The table itself only holds new ids.
    for (old, _) in TOOL_RENAMES {
        if old != "vert.weld" {
            assert_eq!(command::id(old), None, "{old} is still a table row");
        }
    }
    let mut a = Box::new(vertex_app());
    a.execute("tool.set vert.weld on");
    assert!(a.vertex_weld, "{}", a.status);
}

#[test]
fn old_command_verbs_and_help_topics_still_work() {
    same(edge_app, "edge.spin", "edge.spinQuads");
    same(edge_app, "edge.toCurves", "edges.toCurve");
    same(edge_app, "edge.joinAverage", "edge.join average:true");
    same(edge_app, "edge.joinAverage", "edge.join 1");
    let joined = same(edge_app, "edge.join", "edge.join average:false");
    assert_eq!(
        joined.editor.history.undo.len(),
        edge_app().editor.history.undo.len() + 1
    );
}

#[test]
fn old_view_and_selection_set_verbs_still_work() {
    same(
        vertex_app,
        "view.showVertices on",
        "view3d.showVertices state:on",
    );
    same(
        vertex_app,
        "view.showVertices off",
        "view3d.showVertices off",
    );
    same(box_app, "selset.assign Rim", "select.editSet Rim");
    let sets = || {
        let mut a = box_app();
        a.editor.scene.selection.faces = vec![0, 1];
        a.execute("select.editSet Rim");
        a.execute("select.editSet {Top Cap} add");
        a.editor.scene.selection.faces.clear();
        a
    };
    same(
        sets,
        "selset.use Rim add",
        "select.useSet name:Rim mode:add",
    );
    same(sets, "selset.delete Rim", "select.deleteSet Rim");
    let gone = same(sets, "selset.deleteAll", "select.deleteSet all:true");
    assert!(
        gone.status.contains("Delet")
            || gone.editor.history.undo.len() > sets().editor.history.undo.len()
    );
    same(sets, "selset.group G Rim", "select.convertGroup G Rim");
    same(box_app, "backdrop.create", "backdrop.add");
}

#[test]
fn old_help_topics_show_the_new_topic() {
    for (old, new) in COMMAND_RENAMES {
        if old == "poly.flattenNgons" {
            continue; // an MCP-only spelling
        }
        same(box_app, &format!("help {old}"), &format!("help {new}"));
    }
}

#[test]
fn old_argument_names_still_bind() {
    // The old key arrives renamed and binds (parsed after the rewrite).
    let old = operandi_core::command_names::rewrite(
        "chamfer.edit offset:0.1 segments:2 sourceSegments:3 depth:0.5",
        Mode::Polygon,
    )
    .unwrap()
    .text;
    let args = command::tokens(&old).unwrap();
    let p = command::chamfer_edit_arguments(&args[1..]).unwrap();
    assert_eq!((p.source_segments, p.segments), (3, 2));
    let bevel = || {
        let mut a = edge_app();
        a.execute("tool.set edge.chamfer on");
        a
    };
    let b = same(
        bevel,
        "tool.attr edge.chamfer useMaterial on",
        "tool.attr edge.chamfer useMat on",
    );
    assert!(
        b.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .edge_bevel
            .use_material
    );
    same(
        bevel,
        "tool.attr edge.chamfer material Trim",
        "tool.attr edge.chamfer matName Trim",
    );
    same(box_app, "vert.new 1 2 3", "vert.new x:1 y:2 z:3");
    same(box_app, "poly.unify true", "poly.unify force:true");
}

#[test]
fn star_slide_picks_the_variant_by_mode() {
    same(
        vertex_app,
        "tool.set \"*.Slide\" on",
        "tool.set vert.slide on",
    );
    same(
        edge_app,
        "tool.set \"*.Slide\" on",
        "tool.set edge.slide on",
    );
    let mut a = box_app();
    a.execute("tool.set *.Slide on");
    assert_eq!(a.command_active(), None);
    assert!(a.status.contains("Vertex or Edge"), "{}", a.status);
}

#[test]
fn map_only_commands_reach_existing_behaviour() {
    same(vertex_app, "@ppi_vert_tear.py", "tool.set vert.tear on");
    same(
        box_app,
        "@ppi_PolyScale_Offset.py detach",
        "@ppi_PolyScale_Offset_Detach.py",
    );
}

/// tool.reset matched table rows by index, and rows added later shifted
/// them (Merge reset Bevel's row; Capsule, Arc and Tube weren't reset).
#[test]
fn tool_reset_follows_names_not_rows() {
    let mut a = vertex_app();
    a.execute("tool.set vert.merge on");
    a.execute("tool.reset vert.merge");
    assert_ne!(a.status, "Reset is unsupported for this tool");
    let mut a = vertex_app();
    a.execute("tool.set vert.bevel on");
    a.execute("tool.reset");
    // MN-05: every tool resets now.
    assert_ne!(a.status, "Reset is unsupported for this tool");
    assert!(a.editor.session.live.is_some(), "{}", a.status);
}

/// Scripts give percents as 0–1 and angles in radians, as in the name map
/// (Jason, 2026-11-07); the tools and panels keep % and degrees.
#[test]
fn script_percent_and_angle_arguments_take_the_maps_units() {
    let chamfer = |line: &str| {
        let args = command::tokens(line).unwrap();
        command::chamfer_edit_arguments(&args[1..]).map(|p| p.depth)
    };
    let base = "chamfer.edit offset:0.1 segments:2 sourceSegs:1";
    assert_eq!(chamfer(&format!("{base} depth:0.5")), Ok(50.));
    assert_eq!(chamfer(&format!("{base} depth:1")), Ok(100.));
    assert_eq!(chamfer(&format!("{base} depth:50%")), Ok(50.));

    // tool.attr edge.chamfer|edge.bevel depth.
    for tool in ["edge.chamfer", "edge.bevel"] {
        let mut a = Box::new(edge_app());
        a.execute(&format!("tool.set {tool} on"));
        a.execute(&format!("tool.attr {tool} depth 0.25"));
        let depth = |a: &Operandi| a.editor.session.live.as_ref().unwrap().params.chamfer.depth;
        assert_eq!(depth(&a), 25., "{tool}: {}", a.status);
        a.execute(&format!("tool.attr {tool} depth 40%"));
        assert_eq!(depth(&a), 40., "{tool}");
    }

    // tool.attr poly.smshift maxAngle: radians, or with a unit.
    let mut a = Box::new(box_app());
    a.editor.scene.selection.faces = vec![0];
    a.execute("tool.set poly.smshift on");
    let angle = |a: &Operandi| {
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .smooth_shift
            .angle
    };
    a.execute(&format!(
        "tool.attr poly.smshift maxAngle {}",
        std::f64::consts::FRAC_PI_4
    ));
    assert!((angle(&a) - 45.).abs() < 1e-9, "{}", a.status);
    a.execute("tool.attr poly.smshift maxAngle 30deg");
    assert!((angle(&a) - 30.).abs() < 1e-9, "{}", a.status);

    // tool.attr poly.reduct weights.
    let mut a = Box::new(box_app());
    a.execute("tool.set poly.reduct on");
    a.execute("tool.attr poly.reduct boundaryWeight 0.5");
    a.execute("tool.attr poly.reduct boundaryWeight ?");
    assert_eq!(a.status, "50 %");
    a.execute("tool.attr poly.reduct materialWeight 0.75");
    a.execute("tool.attr poly.reduct materialWeight ?");
    assert_eq!(a.status, "75 %");

    // edge.addLoop / edge.addPoint position.
    for verb in ["edge.addLoop", "edge.addPoint"] {
        same(
            edge_app,
            &format!("{verb} position:0.25"),
            &format!("{verb} position:25%"),
        );
        let mut a = Box::new(edge_app());
        a.execute(&format!("{verb} position:25"));
        assert!(a.status.contains("between 0 and 1"), "{verb}: {}", a.status);
        let mut b = Box::new(edge_app());
        b.execute(&format!("{verb} position:0.25"));
        let mut c = Box::new(edge_app());
        c.execute(&format!("{verb} position:0.5"));
        assert_ne!(
            b.editor.scene, c.editor.scene,
            "{verb} position moves the cut"
        );
        assert_ne!(b.editor.scene, edge_app().editor.scene, "{verb} edits");
    }
}
