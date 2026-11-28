//! The Edge tab: tool rows, handles, shortcuts and commands.
use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;

#[test]
fn edge_tab_add_loop_click_and_commands() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let cube = || {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
                .unwrap()
                .into();
        a.camera.view = View::Front;
        a.category = toolbox::Category::Edge;
        a
    };
    // Panel row arms click mode; a click on a visible edge adds a loop.
    let mut a = cube();
    draw(&mut a, &ctx, size, vec![]);
    let row = button_by_line(&a, "Add Loop").center();
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(row)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(row, true, Modifiers::NONE)],
    );
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(row, false, Modifiers::NONE)],
    );
    assert_eq!(
        a.edge_click,
        Some(edge_panel::EdgeClick::AddLoop),
        "{}",
        a.status
    );
    let r = a.viewport_rect;
    // Front view: the front face's top edge runs left-right; click its middle.
    let item = a.editor.scene.active().unwrap();
    let (top_a, top_b) = item
        .mesh
        .edges()
        .into_iter()
        .find(|&(x, y)| {
            let (p, q) = (
                item.mesh.vertices[x as usize],
                item.mesh.vertices[y as usize],
            );
            p.y > 0.5 && q.y > 0.5 && p.z > 0.5 && q.z > 0.5
        })
        .unwrap();
    let mid = item.mesh.vertices[top_a as usize].lerp(item.mesh.vertices[top_b as usize], 0.3);
    let (click, _) = a.camera.project(mid, r).unwrap();
    assert!(a.pick_edge_parameter(click, r).is_some());
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(click)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(click, true, Modifiers::NONE)],
    );
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(click, false, Modifiers::NONE)],
    );
    let m = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(m.faces.len(), 10, "{}", a.status);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Add Loop");
    // Drop (Q) disarms the click tool; the next click selects instead.
    let faces = a.editor.scene.active().unwrap().mesh.faces.len();
    draw(&mut a, &ctx, size, vec![key(Key::Q)]);
    assert_eq!(a.edge_click, None, "{}", a.status);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(click, true, Modifiers::NONE)],
    );
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(click, false, Modifiers::NONE)],
    );
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), faces);
    // Cancel and activating another tool disarm it too.
    a.toggle_edge_click(edge_panel::EdgeClick::AddLoop);
    a.run(Command::Cancel);
    assert_eq!(a.edge_click, None);
    a.toggle_edge_click(edge_panel::EdgeClick::AddLoop);
    a.run(Command::Activate(Tool::Move));
    assert_eq!(a.edge_click, None);
    assert!(a.editor.session.live.is_some());
    // Command routes.
    let mut b = cube();
    b.execute("tool.set edge.addPoint on");
    assert_eq!(b.edge_click, Some(edge_panel::EdgeClick::AddPoint));
    b.execute("tool.set edge.addPoint ?");
    assert_eq!(b.status, "1");
    b.execute("tool.set edge.addPoint off");
    assert_eq!(b.edge_click, None);
    b.run(Command::Mode(Mode::Edge));
    b.editor.scene.selection.edges = vec![(0, 1)];
    b.execute("edge.addPoint position:0.25"); // map percent: 0–1
    assert_eq!(
        b.editor.scene.active().unwrap().mesh.vertices.len(),
        9,
        "{}",
        b.status
    );
    let before = b.editor.scene.clone();
    b.execute("edge.addLoop position:150");
    assert_eq!(b.editor.scene, before);
    b.execute("help edge.addLoop");
    assert!(b.status.contains("position"));
}
#[test]
fn edge_extend_handle_drag_and_edge_extrude_fields() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    // One quad facing the front view; extend its top edge upward.
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = operandi_core::Mesh {
        vertices: vec![
            DVec3::new(-1., -1., 0.),
            DVec3::new(1., -1., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(-1., 1., 0.),
        ],
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: String::new(),
            material_label: None,
            smooth: false,
            part: None,
        }]
        .into(),
        ..Default::default()
    }
    .into();
    a.camera.view = View::Front;
    a.category = toolbox::Category::Edge;
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(2, 3)];
    draw(&mut a, &ctx, size, vec![]);
    let row = button_by_line(&a, "Extend").center();
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(row)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(row, true, Modifiers::NONE)],
    );
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(row, false, Modifiers::NONE)],
    );
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::EdgeExtend),
        "{}",
        a.status
    );
    draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    let (_, _, y_end, _) = a
        .handles(r)
        .into_iter()
        .find(|h| h.0 == 1)
        .expect("Y handle");
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(y_end)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(y_end, true, Modifiers::NONE)],
    );
    let up = y_end + vec2(0., -40.);
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(up)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(up, false, Modifiers::NONE)],
    );
    assert!(
        a.editor.session.live.as_ref().unwrap().params.delta.y > 0.,
        "{}",
        a.status
    );
    a.run(Command::Drop);
    let m = &a.editor.scene.active().unwrap().mesh;
    m.validate().unwrap();
    assert_eq!(m.faces.len(), 2);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Extend");
    assert_eq!(
        a.editor.scene.selection.edges.len(),
        1,
        "new outer edge selected"
    );

    // Edge Extrude on a cube edge through the numeric fields.
    let mut b = Operandi::empty();
    b.run(Command::NewMesh);
    b.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
            .unwrap()
            .into();
    b.run(Command::Mode(Mode::Edge));
    b.editor.scene.selection.edges = vec![(0, 1)];
    b.execute("tool.set edge.extrude on");
    assert_eq!(
        b.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::EdgeExtrude),
        "{}",
        b.status
    );
    b.fields.insert("shift".into(), "0.3 m".into());
    b.fields.insert("inset".into(), "0.2 m".into());
    assert!(b.confirm_fields(), "{}", b.status);
    b.run(Command::Drop);
    let m = &b.editor.scene.active().unwrap().mesh;
    m.validate().unwrap();
    assert_eq!(m.faces.len(), 10, "{}", b.status);
    assert_eq!(b.editor.history.undo.last().unwrap().label, "Edge Extrude");
    b.run(Command::Undo);
    assert_eq!(b.editor.scene.active().unwrap().mesh.faces.len(), 6);
}
#[test]
fn shift_x_is_edge_extrude_in_edge_mode_and_extrude_elsewhere() {
    // Shift+X is Polygon Extrude, and Edge Extrude in Edge context.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let shift_x = Event::Key {
        key: Key::X,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::SHIFT,
    };
    let cube = |mode: Mode| {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
                .unwrap()
                .into();
        a.run(Command::Mode(mode));
        a.editor.scene.selection.edges = vec![(0, 1)];
        a.editor.scene.selection.faces = vec![1];
        a
    };
    let mut edge = cube(Mode::Edge);
    draw(&mut edge, &ctx, size, vec![]);
    draw(&mut edge, &ctx, size, vec![shift_x.clone()]);
    assert_eq!(
        edge.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::EdgeExtrude),
        "{}",
        edge.status
    );
    // A fresh context: the Edge Extrude panel above left a field focused.
    let ctx = Context::default();
    theme::install(&ctx);
    let mut poly = cube(Mode::Polygon);
    draw(&mut poly, &ctx, size, vec![]);
    draw(&mut poly, &ctx, size, vec![shift_x]);
    assert_eq!(
        poly.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Extrude),
        "{}",
        poly.status
    );
    // The default map is valid; a real overlap is still refused.
    let mut map = input::default_bindings();
    assert!(input::bindings_valid(&map));
    map.insert("Bevel".into(), "Shift+X".into());
    assert!(!input::bindings_valid(&map));
}
#[test]
fn slide_shortcut_and_handle_drag() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
            .unwrap()
            .into();
    // The rail of edge 0–1 runs along Z (the old full-length probe merged
    // points and fell back to X), so look from the top to see it.
    a.camera.view = View::Top;
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(0, 1)];
    draw(&mut a, &ctx, size, vec![]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![Event::Key {
            key: Key::S,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::EdgeSlide),
        "{}",
        a.status
    );
    draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    let (_, start, end, label) = a.handles(r).into_iter().next().expect("Slide handle");
    assert_eq!(label, "Slide");
    let along = (end - start).normalized() * 30.;
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(end)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(end, true, Modifiers::NONE)],
    );
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(end + along)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(end + along, false, Modifiers::NONE)],
    );
    let shift = a.editor.session.live.as_ref().unwrap().params.shift;
    assert!(shift > 0. && shift <= 1., "{shift} {}", a.status);
    a.run(Command::Drop);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Slide");
}
#[test]
fn edge_weight_tool_starts_at_current_weight_and_records_once() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
            .unwrap()
            .into();
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(0, 1)];
    a.execute("tool.set edge.crease on");
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::EdgeCrease),
        "{}",
        a.status
    );
    // EW-07: the panel shows Weight in percent.
    a.fields.insert("edge_weight".into(), "60%".into());
    assert!(a.confirm_fields(), "{}", a.status);
    a.run(Command::Drop);
    let creases = &a.editor.scene.active().unwrap().mesh.attributes.creases;
    assert_eq!(creases.len(), 1);
    assert!((creases[0].weight - 0.6).abs() < 1e-12);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Edge Weight");
    // Re-activating starts at the stored weight, so no accidental reset.
    a.execute("tool.set edge.crease on");
    assert!((a.editor.session.live.as_ref().unwrap().params.shift - 0.6).abs() < 1e-12);
    a.run(Command::Cancel);
    assert_eq!(
        a.editor
            .scene
            .active()
            .unwrap()
            .mesh
            .attributes
            .creases
            .len(),
        1
    );
}
#[test]
fn edge_command_rows_shortcuts_and_verbs_share_one_operation() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 1800.);
    let cube = || {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
                .unwrap()
                .into();
        a.camera.view = View::Front;
        a.category = toolbox::Category::Edge;
        a.run(Command::Mode(Mode::Edge));
        let e = a.editor.scene.active().unwrap().mesh.edges()[0];
        a.editor.scene.selection.edges = vec![e];
        a.editor.history.undo.clear();
        a
    };
    // Panel row.
    let mut a = cube();
    draw(&mut a, &ctx, size, vec![]);
    for name in [
        "Grow Quads",
        "Fill Quads",
        "Set Position",
        "Edge Split",
        "Split Normals",
        "Edge Spin",
        "Join",
        "Join Averaged",
        "Edge Collapse",
        "Edge Remove",
        "Edges To Curves",
        "Unbevel",
        "Edge Subdivide",
    ] {
        button_by_line(&a, name);
    }
    let row = button_by_line(&a, "Edge Subdivide").center();
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(row)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![
            pointer(row, true, Modifiers::NONE),
            pointer(row, false, Modifiers::NONE),
        ],
    );
    let by_row = a.editor.scene.active().unwrap().mesh.clone();
    assert_eq!(by_row.vertices.len(), 9, "{}", a.status);
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Edge Subdivide"
    );
    // Shortcut D (Edge mode) and the command verb give the same result.
    let mut b = cube();
    draw(&mut b, &ctx, size, vec![]);
    let viewport = b.viewport_rect.center();
    draw(
        &mut b,
        &ctx,
        size,
        vec![Event::PointerMoved(viewport), key(Key::D)],
    );
    assert_eq!(
        b.editor.scene.active().unwrap().mesh,
        by_row,
        "{}",
        b.status
    );
    let mut c = cube();
    c.execute("edge.subdivide");
    assert_eq!(
        c.editor.scene.active().unwrap().mesh,
        by_row,
        "{}",
        c.status
    );
    c.execute("edge.subdivide extra");
    assert!(c.status.contains("takes no arguments"), "{}", c.status);
    c.execute("help edge.subdivide");
    assert!(c.status.contains("shortcut D"), "{}", c.status);
    // Shift+Backspace is Edge Remove in Edge mode and Remove in Polygon mode.
    assert!(!input::contexts_overlap("Edge Remove", "Remove"));
    assert!(!input::contexts_overlap(
        "Edge Collapse",
        "Polygon Collapse"
    ));
    assert!(input::bindings_valid(&input::default_bindings()));
}
#[test]
fn edge_bevel_chamfer_relax_rows_shortcut_handle_and_bridge() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 1800.);
    let cube = || {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
                .unwrap()
                .into();
        a.camera.view = View::Perspective;
        a.category = toolbox::Category::Edge;
        a.run(Command::Mode(Mode::Edge));
        let m = &a.editor.scene.active().unwrap().mesh;
        let e = m
            .edges()
            .into_iter()
            .find(|&(x, y)| {
                let (p, q) = (m.vertices[x as usize], m.vertices[y as usize]);
                p.y > 0.4 && q.y > 0.4 && p.z > 0.4 && q.z > 0.4
            })
            .unwrap();
        a.editor.scene.selection.edges = vec![e];
        a.editor.history.undo.clear();
        a
    };
    // KEY-02: B starts Edge Bevel in Edge mode.
    let mut a = cube();
    draw(&mut a, &ctx, size, vec![]);
    for row in ["Edge Bevel", "Edge Chamfer", "Edge Relax", "Bridge"] {
        button_by_line(&a, row);
    }
    let viewport = a.viewport_rect.center();
    draw(
        &mut a,
        &ctx,
        size,
        vec![Event::PointerMoved(viewport), key(Key::B)],
    );
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::EdgeBevel),
        "{}",
        a.status
    );
    a.run(Command::Cancel);
    // Edge Bevel from its command; dragging its handle insets.
    a.execute("tool.set edge.bevel on");
    let r = a.viewport_rect;
    let handle = a.handles(r).into_iter().next().expect("inset handle");
    let params = a.editor.session.live.as_ref().unwrap().params.clone();
    a.update_handle(
        handle.0,
        params,
        handle.2,
        handle.2 + (handle.2 - handle.1).normalized() * 20.,
        Tool::EdgeBevel,
        r,
    );
    assert!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .chamfer
            .offset
            > 0.,
        "{}",
        a.status
    );
    a.run(Command::Drop);
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.faces.len(),
        7,
        "{}",
        a.status
    );
    // Chamfer and Relax activate from their command routes.
    let mut b = cube();
    b.execute("tool.set edge.chamfer on");
    assert_eq!(
        b.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::EdgeChamfer)
    );
    b.fields.insert("chamfer_offset".into(), "0.1 m".into());
    b.fields.insert("chamfer_segments".into(), "3".into());
    assert!(b.confirm_fields(), "{}", b.status);
    b.run(Command::Drop);
    assert_eq!(
        b.editor.scene.active().unwrap().mesh.faces.len(),
        9,
        "{}",
        b.status
    );
    // Bridge in Edge mode: two facing border edges of two quads.
    let mut c = Operandi::empty();
    c.run(Command::NewMesh);
    c.editor.scene.active_mut().unwrap().mesh = operandi_core::Mesh {
        vertices: [
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [2., 0., 0.],
            [3., 0., 0.],
            [3., 1., 0.],
            [2., 1., 0.],
        ]
        .iter()
        .map(|&p| DVec3::from_array(p))
        .collect(),
        faces: [vec![0, 1, 2, 3], vec![4, 5, 6, 7]]
            .into_iter()
            .map(|vertices| Face {
                vertices,
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    }
    .into();
    c.run(Command::Mode(Mode::Edge));
    c.editor.scene.selection.edges = vec![(1, 2), (4, 7)];
    c.execute("tool.set edge.bridge on");
    c.run(Command::Apply);
    assert_eq!(
        c.editor.scene.active().unwrap().mesh.faces.len(),
        3,
        "{}",
        c.status
    );
    assert_eq!(c.editor.history.undo.last().unwrap().label, "Bridge Apply");
}
#[test]
fn every_m2_edge_row_is_callable_from_the_command_line() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    for id in [
        "edge.bevel",
        "edge.extrude",
        "edge.extend",
        "edge.bridge",
        "edge.slide",
        "edge.relax",
        "edge.chamfer",
        "edge.addLoop",
        "edge.addPoint",
        "edge.crease",
    ] {
        a.execute(&format!("tool.set {id} ?"));
        assert_eq!(a.status, "0", "{id}: {}", a.status);
    }
    for command in EdgeCommand::ALL {
        // Join Averaged is `edge.join average:true`; its topic is edge.join.
        let verb = command.verb().split_whitespace().next().unwrap();
        a.execute(&format!("help {verb}"));
        assert!(
            a.status.starts_with("Help: "),
            "{}: {}",
            command.verb(),
            a.status
        );
    }
    a.execute("help vert.set");
    assert!(a.status.starts_with("Help: "), "{}", a.status);
}

/// Edge Bevel / Edge Chamfer options through `tool.attr` and the panel's
/// fields share one preview; options come back on the next use.
#[test]
fn edge_bevel_chamfer_options_by_command_and_fields() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
            .unwrap()
            .into();
    a.run(Command::Mode(Mode::Edge));
    let m = &a.editor.scene.active().unwrap().mesh;
    let e = m
        .edges()
        .into_iter()
        .find(|&(x, y)| {
            let (p, q) = (m.vertices[x as usize], m.vertices[y as usize]);
            p.y > 0.4 && q.y > 0.4 && p.z > 0.4 && q.z > 0.4
        })
        .unwrap();
    a.editor.scene.selection.edges = vec![e];
    a.execute("tool.attr edge.chamfer shape square");
    assert_eq!(a.status, "Activate Edge Chamfer first");
    a.execute("tool.set edge.chamfer on");
    // The panel draws with every option.
    let ctx = Context::default();
    theme::install(&ctx);
    draw(&mut a, &ctx, vec2(1440., 1800.), vec![]);
    assert!(a.field_rects.contains_key("chamfer_segments"));
    for command in [
        "tool.attr edge.chamfer offset 0.1",
        "tool.attr edge.chamfer segments 3",
        "tool.attr edge.chamfer shape square",
        "tool.attr edge.chamfer useMaterial on",
        "tool.attr edge.chamfer material Trim",
    ] {
        a.execute(command);
    }
    a.execute("tool.attr edge.chamfer shape ?");
    assert_eq!(a.status, "square");
    a.execute("tool.attr edge.chamfer shape bevelled");
    assert_eq!(a.status, "Shape: round, square or sharp");
    // The field shows what the command set.
    assert_eq!(a.fields["chamfer_segments"], "3");
    a.run(Command::Drop);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(
        (mesh.vertices.len(), mesh.faces.len()),
        (12, 10),
        "{}",
        a.status
    );
    assert_eq!(
        mesh.faces.iter().filter(|f| f.material == "Trim").count(),
        4
    );
    assert_eq!(a.editor.scene.selection.edges.len(), 1);
    // Edge Bevel in Width mode, Round Level by command.
    a.run(Command::Undo);
    a.execute("tool.set edge.bevel on");
    a.execute("tool.attr edge.bevel mode width");
    a.execute("tool.attr edge.bevel width 0.1");
    a.execute("tool.attr edge.bevel level 1");
    a.run(Command::Drop);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(
        (mesh.vertices.len(), mesh.faces.len()),
        (12, 8),
        "{}",
        a.status
    );
    assert!(
        mesh.vertices
            .iter()
            .any(|p| (p.y - (0.5 - 0.141421)).abs() < 1e-5 && (p.z - 0.5).abs() < 1e-9)
    );
    // Chamfer's options come back; its width starts at zero.
    a.run(Command::Undo);
    a.execute("tool.set edge.chamfer on");
    let p = &a.editor.session.live.as_ref().unwrap().params;
    assert_eq!(p.edge_bevel.shape, operandi_core::EdgeBevelShape::Square);
    assert_eq!((p.chamfer.segments, p.chamfer.offset), (3, 0.));
}
#[test]
fn edge_extrude_handle_drag_from_fresh_tool_extrudes() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
            .unwrap()
            .into();
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(0, 1)];
    draw(&mut a, &ctx, size, vec![]);
    a.execute("tool.set edge.extrude on");
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::EdgeExtrude),
        "{}",
        a.status
    );
    draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    // A fresh tool already has a Width, so the Extrude arrow alone extrudes
    // (Width 0 refused every Extrude; Jason, 2026-11-04).
    assert!(a.editor.session.live.as_ref().unwrap().params.inset > 0.);
    let (_, start, end, label) = a.handles(r).into_iter().next().expect("Extrude handle");
    assert_eq!(label, "Extrude");
    let along = (end - start).normalized() * 40.;
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(end)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(end, true, Modifiers::NONE)],
    );
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(end + along)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(end + along, false, Modifiers::NONE)],
    );
    assert!(
        a.editor.session.live.as_ref().unwrap().params.shift > 0.,
        "{}",
        a.status
    );
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.faces.len(),
        10,
        "{}",
        a.status
    );
    // One press-to-release is one undo step.
    a.run(Command::Drop);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Edge Extrude");
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 6);
}
#[test]
fn vertex_extrude_starts_with_a_width() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
            .unwrap()
            .into();
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![0];
    a.execute("tool.set vert.extrude on");
    let l = a.editor.session.live.as_ref().expect("Vertex Extrude live");
    assert_eq!(l.tool, Tool::VertexExtrude, "{}", a.status);
    assert!((l.params.inset - 0.2).abs() < 1e-12, "{}", l.params.inset);
    // Extrude alone now works without touching Width.
    a.fields.insert("shift".into(), "0.3 m".into());
    assert!(a.confirm_fields(), "{}", a.status);
    a.run(Command::Drop);
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Vertex Extrude"
    );
}

#[test]
fn edge_weight_accepts_any_percent() {
    // EW2-01: any value is kept (the surface clamps the effect at ±100 %).
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
            .unwrap()
            .into();
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(0, 1)];
    a.execute("tool.set edge.crease on");
    a.fields.insert("edge_weight".into(), "-50%".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert!((a.editor.session.live.as_ref().unwrap().params.shift + 0.5).abs() < 1e-12);
    a.fields.insert("edge_weight".into(), "250%".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert_eq!(a.editor.session.live.as_ref().unwrap().params.shift, 2.5);
    a.fields.insert("edge_weight".into(), "-40%".into());
    assert!(a.confirm_fields(), "{}", a.status);
    a.run(Command::Drop);
    let creases = &a.editor.scene.active().unwrap().mesh.attributes.creases;
    assert!((creases[0].weight + 0.4).abs() < 1e-12);
}
#[test]
/// EBC-Q12 v1: Square and Sharp run through the original edge at every
/// Level, Level 0 included, and ignore Level and Depth (EBS-03); Depth
/// blends Round's middle points from flat to the arc and does nothing at
/// Level 0 (EBS-01/02). Rows that don't apply grey in place (UIC-92).
/// Jason found Edge Shape and Depth doing nothing at the default Level 0.
fn edge_bevel_shape_and_depth_follow_ebc_q12() {
    use crate::live_fields::LiveFields;
    use crate::tool_form::ToolFields;
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
            .unwrap()
            .into();
    a.run(Command::Mode(Mode::Edge));
    let m = &a.editor.scene.active().unwrap().mesh;
    let e = m
        .edges()
        .into_iter()
        .find(|&(x, y)| {
            let (p, q) = (m.vertices[x as usize], m.vertices[y as usize]);
            p.y > 0.4 && q.y > 0.4 && p.z > 0.4 && q.z > 0.4
        })
        .unwrap();
    a.editor.scene.selection.edges = vec![e];
    for tool in ["edge.bevel", "edge.chamfer"] {
        a.execute(&format!("tool.set {tool} on"));
        let bevel = tool == "edge.bevel";
        // Both show Segments, 1 flat (2026-11-26).
        let (rows_label, rows) = ("Segments", "segments");
        a.execute(&format!("tool.attr {tool} offset 0.1"));
        // Which of Segments / Edge Shape / Depth are greyed.
        let greyed = |a: &Operandi| {
            let p = LiveFields::params(a).unwrap();
            let form = LiveFields::form(a, &p);
            [rows_label, "Edge Shape", "Depth (%)"].map(|label| {
                form.fields()
                    .iter()
                    .find(|f| f.row.label == label)
                    .unwrap_or_else(|| panic!("{tool}: no {label}"))
                    .disabled_tip
                    .is_some()
            })
        };
        let counts = |a: &Operandi| {
            let m = &a.editor.scene.active().unwrap().mesh;
            (m.vertices.len(), m.faces.len())
        };
        // The profile's middle point at the +X end (its Y; Y = Z there).
        let middle = |a: &Operandi| {
            let m = &a.editor.scene.active().unwrap().mesh;
            m.vertices
                .iter()
                .find(|p| p.x > 0.4 && (p.y - p.z).abs() < 1e-9 && p.y > 0.4)
                .map(|p| p.y)
        };
        // Round, one segment: a flat strip; Depth greyed and inert.
        assert_eq!(greyed(&a), [false, false, true], "{tool}");
        assert_eq!(counts(&a), (10, 7), "{tool}");
        a.execute(&format!("tool.attr {tool} depth 0.5"));
        assert_eq!((counts(&a), middle(&a)), ((10, 7), None), "{tool}");
        a.execute(&format!("tool.attr {tool} depth 1"));
        // Square and Sharp at one segment keep the original corner; Level
        // and Depth grey out and change nothing.
        for shape in ["square", "sharp"] {
            a.execute(&format!("tool.attr {tool} shape {shape}"));
            assert_eq!(a.status, "Ready", "{tool} {shape}");
            assert_eq!(greyed(&a), [true, false, true], "{tool} {shape}");
            let caps = if bevel { (12, 8) } else { (12, 10) };
            assert_eq!(counts(&a), caps, "{tool} {shape}");
            assert_eq!(middle(&a), Some(0.5), "{tool} {shape}");
            for change in ["depth 0.5", &format!("{rows} 2")] {
                a.execute(&format!("tool.attr {tool} {change}"));
                assert_eq!((counts(&a), middle(&a)), (caps, Some(0.5)), "{change}");
            }
            a.execute(&format!("tool.attr {tool} depth 1"));
            a.execute(&format!("tool.attr {tool} {rows} 1"));
        }
        // EBS-04 end caps: Edge Bevel one 6-sided end face, Edge Chamfer a
        // triangle and a 5-sided face.
        let sides = |a: &Operandi| {
            let m = &a.editor.scene.active().unwrap().mesh;
            let mut s: Vec<usize> = m
                .faces
                .iter()
                .filter(|f| f.vertices.iter().all(|&v| m.vertices[v as usize].x > 0.4))
                .map(|f| f.vertices.len())
                .collect();
            s.sort_unstable();
            s
        };
        assert_eq!(
            sides(&a),
            if bevel { vec![6] } else { vec![3, 5] },
            "{tool}"
        );
        // Round with a middle point: on the arc at 100%, halfway at 50%.
        a.execute(&format!("tool.attr {tool} shape round"));
        a.execute(&format!("tool.attr {tool} {rows} 2"));
        assert_eq!(greyed(&a), [false, false, false], "{tool}");
        let arc = 0.4 + 0.1 * 0.5f64.sqrt();
        assert!((middle(&a).unwrap() - arc).abs() < 1e-6, "{tool}");
        a.execute(&format!("tool.attr {tool} depth 0.5"));
        let half = middle(&a).unwrap();
        assert!((half - (0.45 + arc) / 2.).abs() < 1e-6, "{tool}: {half}");
        a.run(Command::Cancel);
        a.run(Command::Drop);
    }
}
