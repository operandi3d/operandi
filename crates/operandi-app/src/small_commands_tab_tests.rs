//! Missing tools group 1 (specifications/missing-tools-g1): the Select
//! tab's Polygon Tag Type drop-down (MT1) and the Edit tab's Commands
//! buttons Fix Black Dots, Edge Zip and Diagonal Re-Direct (MT2-MT4),
//! driven through the drawn toolbox.
use super::tests::draw;
use super::tests::pointer;
use super::toolbox_form_tests::button_by_line;
use super::*;
use operandi_core::PolygonTagKind;

// Tall enough that the Edit tab's Commands rows are on screen.
const SIZE: Vec2 = vec2(1440., 2200.);

/// An n×n quad grid in the XY plane: point (i, j) is (n+1)j + i.
fn grid(n: u32) -> operandi_core::Mesh {
    let mut m = operandi_core::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            m.vertices.push(DVec3::new(f64::from(i), f64::from(j), 0.));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = (n + 1) * j + i;
            m.faces.push(operandi_core::Face {
                vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    m
}

/// Clicks the form button that runs `line`.
fn click_line(a: &mut Operandi, ctx: &Context, line: &str, _: Modifiers) {
    let at = button_by_line(a, line).center();
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    let mods = Modifiers::NONE;
    draw(
        a,
        ctx,
        SIZE,
        vec![pointer(at, true, mods), pointer(at, false, mods)],
    );
}

fn app_with(mesh: operandi_core::Mesh, category: toolbox::Category) -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.editor.synchronize_identities().unwrap();
    a.camera.view = View::Front;
    a.category = category;
    a
}

#[test]
fn edit_tab_commands_run_fix_black_dots_edge_zip_and_diagonal_redirect() {
    let ctx = Context::default();
    theme::install(&ctx);

    // Fix Black Dots: G4 with quad 5 split through its middle point.
    let mut m = grid(4);
    m.vertices.push(DVec3::new(1.5, 1.5, 0.));
    m.faces[5].vertices = vec![6, 7, 12, 25];
    let half = operandi_core::Face {
        vertices: vec![6, 25, 12, 11],
        ..m.faces[5].clone()
    };
    m.faces.push(half);
    let mut a = app_with(m, toolbox::Category::Edit);
    let steps = a.editor.undo.len();
    draw(&mut a, &ctx, SIZE, vec![]);
    click_line(&mut a, &ctx, "operandi.mesh.fixBlackDots", Modifiers::NONE);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.faces.len(), 16, "{}", a.status);
    assert_eq!(a.editor.scene.selection.mode, Mode::Vertex);
    assert_eq!(a.editor.undo.len(), steps + 1);
    // Again: nothing left, a message and no step.
    click_line(&mut a, &ctx, "operandi.mesh.fixBlackDots", Modifiers::NONE);
    assert_eq!(a.editor.undo.len(), steps + 1);
    assert!(a.status.contains("No black dots found."), "{}", a.status);

    // Edge Zip: the full column x = 2.
    let mut a = app_with(grid(4), toolbox::Category::Edit);
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(2, 7), (7, 12), (12, 17), (17, 22)];
    draw(&mut a, &ctx, SIZE, vec![]);
    click_line(&mut a, &ctx, "operandi.edge.zip", Modifiers::NONE);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.faces.len(), 8, "{}", a.status);
    assert_eq!(mesh.vertices.len(), 15);

    // Diagonal Re-Direct: quads 5 and 10 (DR-A).
    let mut a = app_with(grid(4), toolbox::Category::Edit);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![5, 10];
    draw(&mut a, &ctx, SIZE, vec![]);
    click_line(
        &mut a,
        &ctx,
        "operandi.poly.diagonalRedirect",
        Modifiers::NONE,
    );
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.faces.len(), 18, "{}", a.status);
    assert_eq!(mesh.vertices.len(), 27);
    // Refused without a diagonal pair: the message, no change.
    let mut a = app_with(grid(4), toolbox::Category::Edit);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![5, 6];
    let steps = a.editor.undo.len();
    draw(&mut a, &ctx, SIZE, vec![]);
    click_line(
        &mut a,
        &ctx,
        "operandi.poly.diagonalRedirect",
        Modifiers::NONE,
    );
    assert_eq!(a.editor.undo.len(), steps);
    assert!(
        a.status.contains("Select two diagonal polygons."),
        "{}",
        a.status
    );
}

#[test]
fn the_select_tab_polygon_tag_type_drop_down_is_live() {
    use egui::accesskit::{Node, Role};
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.enable_accesskit();
    let mut a = app_with(grid(2), toolbox::Category::Select);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![1];
    let frame = |a: &mut Operandi, events: Vec<Event>| -> Vec<Node> {
        draw(a, &ctx, SIZE, events)
            .platform_output
            .accesskit_update
            .unwrap()
            .nodes
            .into_iter()
            .map(|(_, n)| n)
            .collect()
    };
    let click = |a: &mut Operandi, p: Pos2| {
        frame(a, vec![Event::PointerMoved(p)]);
        for pressed in [true, false] {
            frame(
                a,
                vec![Event::PointerButton {
                    pos: p,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                }],
            );
        }
        frame(a, vec![])
    };
    frame(&mut a, vec![]);
    let field = crate::ui_inventory::take_rects()
        .get("Polygon Tag Type")
        .copied()
        .expect("the drop-down is drawn");
    // MT1-01: four choices, Material first; Smoothing Group is greyed.
    let nodes = click(&mut a, field.center());
    // The popup's rows top to bottom (accesskit nodes come unordered).
    let mut rows: Vec<(f64, &str)> = nodes
        .iter()
        .filter(|n| n.role() == Role::Button)
        .filter_map(|n| Some((n.bounds()?.y0, n.label()?)))
        .filter(|(_, l)| PolygonTagKind::ALL.iter().any(|k| k.label() == *l))
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let labels: Vec<&str> = rows.iter().map(|r| r.1).collect();
    assert_eq!(
        labels,
        ["Material", "Part", "Smoothing Group", "Selection Set"]
    );
    let part = nodes
        .iter()
        .find(|n| n.role() == Role::Button && n.label() == Some("Part"))
        .unwrap()
        .bounds()
        .unwrap();
    click(
        &mut a,
        pos2(
            (part.x0 + part.x1) as f32 / 2.,
            (part.y0 + part.y1) as f32 / 2.,
        ),
    );
    // MT1-02/03: the setting changes; the selection doesn't.
    assert_eq!(
        a.editor.polygon_tag_kind,
        PolygonTagKind::Part,
        "{}",
        a.status
    );
    assert_eq!(a.editor.scene.selection.faces, vec![1]);
    // Changed elsewhere, the drop-down shows it.
    a.run_command_line("operandi.select.polygonTagType type:selectionSet");
    let nodes = frame(&mut a, vec![]);
    assert!(
        nodes
            .iter()
            .any(|n| n.label() == Some("Selection Set") || n.value() == Some("Selection Set")),
        "the drop-down shows the current type"
    );
}
