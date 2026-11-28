//! Select tab of the toolbox: rows, a working command and placeholders.
//! The tab is a form (`toolbox.select.json`); tests find its buttons by
//! their command lines.
use super::tests::{box_app, draw, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;
use crate::form::command_form::{self as cf, FormRef};

/// Click the centre of the Select tab's button running `line`.
fn click(a: &mut Operandi, ctx: &Context, size: Vec2, line: &str) {
    let point = button_by_line(a, line).center();
    click_at(a, ctx, size, point);
}

/// Press and release at `point`.
fn click_at(a: &mut Operandi, ctx: &Context, size: Vec2, point: Pos2) {
    draw(
        a,
        ctx,
        size,
        vec![
            pointer(point, true, Modifiers::NONE),
            pointer(point, false, Modifiers::NONE),
        ],
    );
    draw(a, ctx, size, vec![]);
}

/// A box app showing the Select tab, drawn tall enough that every row is on
/// screen.
fn select_tab_app(ctx: &Context, size: Vec2) -> Operandi {
    show_select_tab(box_app(), ctx, size)
}

fn show_select_tab(mut a: Operandi, ctx: &Context, size: Vec2) -> Operandi {
    theme::install(ctx);
    for _ in 0..3 {
        draw(&mut a, ctx, size, vec![]);
    }
    let tab = a.toolbox_rects["Category Select"].center();
    click_at(&mut a, ctx, size, tab);
    assert_eq!(a.category, toolbox::Category::Select);
    for _ in 0..2 {
        draw(&mut a, ctx, size, vec![]);
    }
    a
}

#[test]
fn select_tab_lists_selection_rows() {
    let ctx = Context::default();
    let size = vec2(1440., 1600.);
    let a = select_tab_app(&ctx, size);
    for line in [
        "Select All",
        "Loop",
        "Select Between",
        "Shortest Path",
        "select.editSet",
    ] {
        button_by_line(&a, line);
    }
    assert!(a.toolbox_rects.contains_key("Selection Sets"));
    // Rows run top to bottom in the tab's order.
    assert!(button_by_line(&a, "Select All").top() < button_by_line(&a, "Loop").top());
    assert!(button_by_line(&a, "Loop").top() < button_by_line(&a, "Select Between").top());
}

#[test]
fn select_all_row_selects_every_polygon() {
    let ctx = Context::default();
    let size = vec2(1440., 1600.);
    let mut a = select_tab_app(&ctx, size);
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces.clear();
    click(&mut a, &ctx, size, "Select All");
    let mut faces = a.editor.scene.selection.faces.clone();
    faces.sort_unstable();
    assert_eq!(faces, (0..6).collect::<Vec<u32>>());
}

/// Modelling parity batch 4 (SL-xx): the new rows run their commands,
/// "..." rows open their dialogs, and Lazy Selection toggles.
#[test]
fn selection_tool_rows_run_their_commands() {
    let ctx = Context::default();
    let size = vec2(1440., 1600.);
    let mut a = select_tab_app(&ctx, size);
    // Shortest Path between two corners of the box's top face.
    a.editor.scene.selection.mode = Mode::Vertex;
    let top: Vec<u32> = (0..a.editor.scene.active().unwrap().mesh.vertices.len() as u32)
        .filter(|&v| a.editor.scene.active().unwrap().mesh.vertices[v as usize].y > 0.)
        .collect();
    a.editor.scene.selection.vertices = vec![top[0], *top.last().unwrap()];
    let undo = a.editor.history.undo.len();
    click(&mut a, &ctx, size, "Shortest Path");
    assert!(a.editor.scene.selection.vertices.len() >= 3, "{}", a.status);
    assert_eq!(a.editor.history.undo.len(), undo + 1);
    // Sharp... opens its dialog; OK selects the box's twelve edges.
    a.editor.scene.selection.mode = Mode::Edge;
    click(&mut a, &ctx, size, "Sharp...");
    assert!(a.select_dialog.is_some(), "{}", a.status);
    a.select_dialog = None;
    draw(&mut a, &ctx, size, vec![]);
    a.execute("select.edgeSharp min:80 max:180");
    assert_eq!(a.editor.scene.selection.edges.len(), 12, "{}", a.status);
    // Coplanar Polygons Tool: live from the start polygon; Cancel undoes.
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    let before = a.editor.scene.selection.clone();
    click(&mut a, &ctx, size, "Coplanar Polygons Tool");
    assert!(a.select_dialog.is_some(), "{}", a.status);
    a.select_dialog = None;
    draw(&mut a, &ctx, size, vec![]);
    let _ = before;
    // Lazy Selection widens the pick reach (a fresh tab: the dialogs above
    // leave the panel scrolled).
    let mut a = select_tab_app(&ctx, size);
    let reach = a.pick_radius();
    assert!(a.form_rects.contains_key("Lazy Selection"));
    a.execute("Lazy Selection");
    assert!(a.selection_preferences.lazy_selection, "{}", a.status);
    assert!(a.line_state("Lazy Selection").unwrap().on);
    assert_eq!(a.pick_radius(), reach * 3.);
    a.execute("select.lazyState");
    assert!(!a.selection_preferences.lazy_selection);
    // Conditional Loop needs an edge.
    a.editor.scene.selection.mode = Mode::Edge;
    a.editor.scene.selection.edges = vec![a.editor.scene.active().unwrap().mesh.edges()[0]];
    click(&mut a, &ctx, size, "Conditional Loop");
    assert!(!a.editor.scene.selection.edges.is_empty(), "{}", a.status);
    assert_eq!(a.category, toolbox::Category::Select);
}

#[test]
fn five_tabs_fit_at_the_smallest_window() {
    let ctx = Context::default();
    theme::install(&ctx);
    for scale in [1., 2.] {
        ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), scale));
        let mut a = box_app();
        // The app's minimum window size.
        let size = vec2(640., 480.);
        for _ in 0..3 {
            draw(&mut a, &ctx, size, vec![]);
        }
        let last = a.toolbox_rects["Category Polygon"];
        assert!(last.bottom() <= size.y, "{last:?} at {scale}");
        let tab = a.toolbox_rects["Category Select"].center();
        click_at(&mut a, &ctx, size, tab);
        assert_eq!(a.category, toolbox::Category::Select);
    }
}

/// Click a viewport point: hover, press and release in separate frames,
/// as a real mouse does.
fn click_viewport(a: &mut Operandi, ctx: &Context, size: Vec2, point: Pos2) {
    for event in [
        Event::PointerMoved(point),
        pointer(point, true, Modifiers::NONE),
        pointer(point, false, Modifiers::NONE),
    ] {
        draw(a, ctx, size, vec![event]);
    }
    draw(a, ctx, size, vec![]);
}

/// Jason's QA case: pick one polygon in the viewport, then use the Select
/// tab's Grow, Shrink and Loop buttons.
#[test]
fn grow_shrink_and_loop_buttons_act_on_a_viewport_pick() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    for (label, expected) in [
        ("Grow", vec![0, 1, 2, 3, 5]),
        ("Shrink", vec![]),
        ("Loop", vec![2, 3, 4, 5]),
    ] {
        let mut a = select_tab_app(&ctx, size);
        a.editor.scene.selection.mode = Mode::Polygon;
        let centre = a.viewport_rect.center();
        click_viewport(&mut a, &ctx, size, centre);
        assert_eq!(a.editor.scene.selection.faces, vec![5], "pick for {label}");
        click(&mut a, &ctx, size, label);
        let mut faces = a.editor.scene.selection.faces.clone();
        faces.sort_unstable();
        assert_eq!(faces, expected, "{label}");
    }
}

#[test]
fn ring_and_close_loop_buttons_act_on_their_selection() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    // Ring: one edge of the box gives the four parallel edges around it.
    let mut a = select_tab_app(&ctx, size);
    let edge = a.editor.scene.active().unwrap().mesh.edges()[0];
    a.editor.scene.selection.mode = Mode::Edge;
    a.editor.scene.selection.edges = vec![edge];
    click(&mut a, &ctx, size, "Ring");
    assert_eq!(a.editor.scene.selection.edges.len(), 4);
    // Close Loop needs a loop that splits the mesh unevenly: on a box with
    // three rows of side polygons, two neighbours in the top row close their
    // band (4) plus the smaller side, the top cap (1).
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Box);
    a.run(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        segments: [1, 3, 1],
        ..Default::default()
    }));
    a.run(Command::Apply);
    a.run(Command::Drop);
    let mut a = show_select_tab(a, &ctx, size);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.faces.len(), 14);
    let centre = |f: &operandi_core::Face| {
        f.vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .sum::<DVec3>()
            / f.vertices.len() as f64
    };
    // Side polygons of the top row: centre between the top cap and the middle row.
    let top_row: Vec<u32> = (0..mesh.faces.len() as u32)
        .filter(|&f| {
            let c = centre(&mesh.faces[f as usize]);
            c.y > 0.5 && c.y < 0.9
        })
        .collect();
    assert_eq!(top_row.len(), 4);
    let first = &mesh.faces[top_row[0] as usize].vertices;
    let neighbour = top_row[1..]
        .iter()
        .copied()
        .find(|&f| {
            let other = &mesh.faces[f as usize].vertices;
            first.iter().filter(|v| other.contains(v)).count() == 2
        })
        .unwrap();
    // Edge Loop on the same box: a horizontal edge between two side rows
    // loops around the box (4 edges). A plain box has no such edge loop.
    let ring_edge = mesh
        .edges()
        .iter()
        .copied()
        .find(|&(p, q)| {
            let (p, q) = (mesh.vertices[p as usize], mesh.vertices[q as usize]);
            (p.y - q.y).abs() < 1e-9 && p.y.abs() < 0.9
        })
        .unwrap();
    let faces = vec![top_row[0], neighbour];
    a.editor.scene.selection.mode = Mode::Edge;
    a.editor.scene.selection.edges = vec![ring_edge];
    click(&mut a, &ctx, size, "Loop");
    assert_eq!(a.editor.scene.selection.edges.len(), 4, "{}", a.status);
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = faces;
    draw(&mut a, &ctx, size, vec![]);
    click(&mut a, &ctx, size, "Close Loop");
    assert_eq!(a.editor.scene.selection.faces.len(), 5, "{}", a.status);
}

/// With nothing selected these rows refuse. The refusal must reach the
/// status bar and the tooltip must say what to select, or the button looks
/// dead (Jason's QA, 2026-11-05).
#[test]
fn rows_that_need_a_selection_say_so() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    for label in ["Grow", "Shrink", "Close Loop", "Loop", "Ring"] {
        let tip = form_tooltip(label);
        assert!(tip.contains("needs a selection"), "{tip}");
        let mut a = select_tab_app(&ctx, size);
        a.editor.scene.selection.mode = Mode::Polygon;
        a.editor.scene.selection.faces.clear();
        a.status = "Ready".into();
        click(&mut a, &ctx, size, label);
        assert!(a.editor.scene.selection.faces.is_empty());
        assert_ne!(a.status, "Ready", "{label} refused silently");
    }
    let tip = form_tooltip("Select All");
    assert!(!tip.contains("needs"), "{tip}");
    assert!(tip.contains("Shortcut: {shortcut}"), "{tip}");
}

/// The Select form's tooltip for `line`.
fn form_tooltip(line: &str) -> String {
    let forms = toolbox_form::shipped_store(None);
    let form = forms.get(toolbox_form::SELECT_FORM).unwrap();
    fn find(controls: &[cf::Control], line: &str) -> Option<String> {
        controls.iter().find_map(|c| match &c.form {
            Some(FormRef::Inline(inner)) => find(&inner.controls, line),
            _ => (c.command.as_deref() == Some(line)).then(|| c.tooltip.clone()),
        })
    }
    find(&form.controls, line).unwrap_or_else(|| panic!("no {line} in the Select form"))
}

/// Jason's QA, 2026-11-05: on a sphere (triangle poles) Grow refused with
/// "only works on all-quad meshes". One quad now grows to its neighbours.
#[test]
fn grow_button_works_on_a_sphere() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Sphere);
    let params = a.editor.session.live.as_ref().unwrap().params.clone();
    a.run(Command::Preview(params));
    a.run(Command::Apply);
    a.run(Command::Drop);
    let mut a = show_select_tab(a, &ctx, size);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    let quad = (0..mesh.faces.len())
        .find(|&f| mesh.faces[f].vertices.len() == 4)
        .unwrap() as u32;
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![quad];
    // PF-01: every polygon sharing a point with it.
    let sharing = {
        let mesh = &a.editor.scene.active().unwrap().mesh;
        let own = &mesh.faces[quad as usize].vertices;
        mesh.faces
            .iter()
            .filter(|f| f.vertices.iter().any(|v| own.contains(v)))
            .count()
    };
    click(&mut a, &ctx, size, "Grow");
    assert_eq!(
        a.editor.scene.selection.faces.len(),
        sharing,
        "{}",
        a.status
    );
    assert!(sharing > 5);
}

/// Next Loop / Previous Loop rows (SEL-LP-02): a sphere band steps to the
/// band beside it and back; the Right/Left arrow defaults do the same.
#[test]
fn next_and_previous_loop_rows_step_a_band() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Sphere);
    let params = a.editor.session.live.as_ref().unwrap().params.clone();
    a.run(Command::Preview(params));
    a.run(Command::Apply);
    a.run(Command::Drop);
    let mut a = show_select_tab(a, &ctx, size);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    let quad = (0..mesh.faces.len())
        .find(|&f| {
            let c = mesh.center(f);
            mesh.faces[f].vertices.len() == 4 && c.y > 0. && c.y < 0.1
        })
        .unwrap() as u32;
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![quad];
    click(&mut a, &ctx, size, "Loop");
    let band = a.editor.scene.selection.faces.clone();
    assert!(band.len() >= 12, "{band:?}");
    click(&mut a, &ctx, size, "Next Loop");
    let next = a.editor.scene.selection.faces.clone();
    assert_eq!(next.len(), band.len(), "{}", a.status);
    assert!(next.iter().all(|f| !band.contains(f)));
    click(&mut a, &ctx, size, "Previous Loop");
    let mut back = a.editor.scene.selection.faces.clone();
    back.sort_unstable();
    let mut sorted = band.clone();
    sorted.sort_unstable();
    assert_eq!(back, sorted);
    // Keyboard defaults: Right = Next, Left = Previous.
    draw(&mut a, &ctx, size, vec![super::tests::key(Key::ArrowRight)]);
    draw(&mut a, &ctx, size, vec![]);
    let mut keyed = a.editor.scene.selection.faces.clone();
    keyed.sort_unstable();
    let mut sorted_next = next.clone();
    sorted_next.sort_unstable();
    assert_eq!(keyed, sorted_next, "{}", a.status);
    draw(&mut a, &ctx, size, vec![super::tests::key(Key::ArrowLeft)]);
    draw(&mut a, &ctx, size, vec![]);
    let mut keyed = a.editor.scene.selection.faces.clone();
    keyed.sort_unstable();
    assert_eq!(keyed, sorted);
    // Nothing selected: refused with a message, not silently.
    a.editor.scene.selection.faces.clear();
    a.status = "Ready".into();
    click(&mut a, &ctx, size, "Next Loop");
    assert!(a.status.contains("Select a loop first"), "{}", a.status);
}

/// UIC-223: the rows grey from their commands' answers, not UI logic: a
/// live tool greys every selection row; Symmetrical also needs Symmetry.
#[test]
fn rows_grey_from_their_commands() {
    let mut a = box_app();
    for line in ["Select All", "Grow", "Convert to Edges", "Select Between"] {
        assert_eq!(a.line_state(line).unwrap().unavailable, None, "{line}");
    }
    let off = a.line_state("Select Symmetrical").unwrap().unavailable;
    assert_eq!(off.as_deref(), Some("Turn Symmetry on (X, Y or Z) first"));
    a.editor.symmetry.axis = Some(0);
    assert_eq!(
        a.line_state("Select Symmetrical").unwrap().unavailable,
        None
    );
    a.activate(Tool::Move);
    for line in [
        "Select All",
        "Ring",
        "Convert to Polygons",
        "Select Symmetrical",
    ] {
        assert_eq!(
            a.line_state(line).unwrap().unavailable.as_deref(),
            Some(crate::unavailable::TOOL_ACTIVE),
            "{line}"
        );
    }
}

/// Placeholders are `operandi.notBuilt` lines: always greyed with what
/// isn't built; typed, they only say so.
#[test]
fn not_built_lines_say_what_is_missing() {
    let mut a = box_app();
    let state = a.line_state("operandi.notBuilt \"Shortest Path\"").unwrap();
    assert_eq!(
        state.unavailable.as_deref(),
        Some("Shortest Path · not in Operandi yet")
    );
    let scene = a.editor.scene.clone();
    a.execute("operandi.notBuilt name:\"Lazy Selection\"");
    assert_eq!(a.status, "Lazy Selection · not in Operandi yet");
    assert_eq!(a.editor.scene, scene);
}

/// The Convert flyout (a `menu` sub-form): its button opens the three
/// targets; picking one converts the selection.
#[test]
fn convert_flyout_runs_its_line() {
    let ctx = Context::default();
    let size = vec2(1440., 1600.);
    let mut a = select_tab_app(&ctx, size);
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    click(&mut a, &ctx, size, "menu:Convert");
    draw(&mut a, &ctx, size, vec![]);
    click(&mut a, &ctx, size, "Convert to Edges");
    assert_eq!(a.editor.scene.selection.mode, Mode::Edge, "{}", a.status);
    assert_eq!(a.editor.scene.selection.edges.len(), 4);
    // With a tool live the flyout is greyed with the commands' reason.
    a.activate(Tool::Move);
    draw(&mut a, &ctx, size, vec![]);
    let at = button_by_line(&a, "menu:Convert").center();
    click_at(&mut a, &ctx, size, at);
    assert!(!a.form_rects.contains_key("Convert to Vertices"));
}

/// A tooltip's `{shortcut}` follows the editable input map.
#[test]
fn tooltips_show_the_current_binding() {
    let mut a = box_app();
    a.bindings.insert("Grow".into(), "Shift+Up".into());
    assert_eq!(
        a.form_tooltip_for_test("Grow · Shortcut: {shortcut}", "Grow"),
        "Grow · Shortcut: Shift+Up"
    );
    a.bindings.remove("Grow");
    assert_eq!(
        a.form_tooltip_for_test("Grow · Shortcut: {shortcut}", "Grow"),
        "Grow · Shortcut: unassigned"
    );
}

/// Selection and Selection Sets fold, and Selection hides the Sets with
/// it (the Sets are a section inside Selection).
#[test]
fn sections_fold_and_nest() {
    let ctx = Context::default();
    let size = vec2(1440., 1600.);
    let mut a = select_tab_app(&ctx, size);
    let sets = a.toolbox_rects["Selection Sets"].center();
    click_at(&mut a, &ctx, size, sets);
    assert!(a.create_folded.contains("selection-sets"));
    assert!(!a.form_rects.contains_key("select.editSet"));
    assert!(a.form_rects.contains_key("Lazy Selection"));
    let top = a.toolbox_rects["Selection"].center();
    click_at(&mut a, &ctx, size, top);
    assert!(a.create_folded.contains("select-tools"));
    assert!(a.form_rects.is_empty(), "{:?}", a.form_rects.keys());
    assert!(!a.toolbox_rects.contains_key("Selection Sets"));
}

/// The four Selection Sets rows, by line.
const SET_LINES: [&str; 4] = [
    "select.editSet",
    "select.convertGroup",
    "select.useSet",
    "select.deleteSet",
];

/// The Selection Sets rows grey for the reasons their dialogs refuse
/// (UIC-223): Item mode, no selection for Assign, no sets for the rest,
/// a live tool; with a selection and a set every row is live.
#[test]
fn selection_set_rows_grey_with_the_commands_reasons() {
    let mut a = box_app();
    let reason = |a: &Operandi, line: &str| a.line_state(line).unwrap().unavailable;
    a.editor.scene.selection.mode = Mode::Item;
    for line in SET_LINES {
        let why = reason(&a, line).unwrap_or_default();
        assert!(why.contains("Vertex, Edge or Polygon"), "{line}: {why}");
    }
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces.clear();
    assert_eq!(
        reason(&a, "select.editSet").as_deref(),
        Some("Select polygons before Assign Selection Set")
    );
    for line in &SET_LINES[1..] {
        let why = reason(&a, line).unwrap_or_default();
        assert!(why.starts_with("This mesh has no"), "{line}: {why}");
    }
    a.editor.scene.selection.faces = vec![2];
    assert_eq!(reason(&a, "select.editSet"), None);
    a.execute("select.editSet Front");
    for line in SET_LINES {
        assert_eq!(reason(&a, line), None, "{line}");
    }
    a.activate(Tool::Move);
    for line in SET_LINES {
        assert!(reason(&a, line).is_some(), "{line} with a tool live");
    }
}

/// Each row runs its bare line, which opens that command's dialog
/// (CMD-12); a greyed row opens nothing.
#[test]
fn selection_set_rows_open_their_dialogs() {
    let ctx = Context::default();
    // Tall enough that the Sets rows clear the tool drawer.
    let size = vec2(1440., 2200.);
    let mut a = select_tab_app(&ctx, size);
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces.clear();
    draw(&mut a, &ctx, size, vec![]);
    click(&mut a, &ctx, size, "select.useSet");
    assert!(a.selection_set.is_none(), "a greyed row opened a dialog");
    for (line, title) in [
        ("select.editSet", "Assign Selection Set"),
        ("select.convertGroup", "Create Selection Set Group"),
        ("select.useSet", "Use Selection Set"),
        ("select.deleteSet", "Delete Selection Set"),
    ] {
        // A fresh app per row: quick clicks in one context would count
        // as a double or triple click.
        let ctx = Context::default();
        let mut a = select_tab_app(&ctx, size);
        a.editor.scene.selection.mode = Mode::Polygon;
        a.editor.scene.selection.faces = vec![2];
        a.execute("select.editSet Front");
        draw(&mut a, &ctx, size, vec![]);
        click(&mut a, &ctx, size, line);
        let open = a.selection_set.as_ref().map(|d| d.title());
        assert_eq!(open, Some(title), "{line}: {}", a.status);
    }
}
