//! The Statistics panel (statistics_ui.rs): drawn on the shared tree list,
//! counted off the UI thread, + / − clicked in the real list, every Sel
//! cell following the selection, undo and redo.
use super::tests::box_app;
use super::*;
use crate::panel_registry::{Category, PanelKind, REGISTRY};
use crate::statistics_ui::{Entry, statistics_spec};
use eframe::egui::PointerButton;
use operandi_core::statistics::{Kind, Row};

const SIZE: Vec2 = vec2(420., 900.);

/// Draws only the panel, as its palette window would, then waits for
/// the count it started.
fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)),
        events,
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();
            a.statistics_panel(ui, rect);
        });
    });
}

fn settled(a: &mut Operandi, ctx: &Context) {
    frame(a, ctx, vec![]);
    a.stats_settle(ctx);
    frame(a, ctx, vec![]);
}

fn tally(a: &Operandi, row: Row) -> (u64, u64) {
    let t = a
        .statistics
        .report
        .as_ref()
        .and_then(|r| r.get(&row))
        .unwrap_or_default();
    (t.count, t.selected)
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    let button = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    frame(a, ctx, vec![Event::PointerMoved(at)]);
    frame(a, ctx, vec![button(true), button(false)]);
}

fn app() -> (Operandi, Context) {
    let mut a = box_app();
    a.editor.scene.selection.mode = Mode::Polygon;
    let ctx = Context::default();
    theme::install(&ctx);
    settled(&mut a, &ctx);
    (a, ctx)
}

#[test]
fn registered_under_info_with_the_tree_list_shape() {
    let t = REGISTRY
        .iter()
        .find(|t| t.kind == PanelKind::Statistics)
        .unwrap();
    assert_eq!(t.name, "Statistics");
    assert_eq!(t.category, Category::Info);
    statistics_spec().check_shape().unwrap();
    assert!(
        crate::palettes::SHIPPED
            .iter()
            .any(|(name, text)| name.contains("statistics") && text.contains("\"Statistics\"")),
        "Layout > Windows/Palettes opens it as a floating window"
    );
}

#[test]
fn counts_the_box_and_follows_every_edit() {
    let (mut a, ctx) = app();
    assert_eq!(tally(&a, Row::All(Kind::Polygon)), (6, 0));
    assert_eq!(tally(&a, Row::PolygonVertices(4)).0, 6);
    assert_eq!(tally(&a, Row::All(Kind::Vertex)).0, 8);
    a.execute("select.all");
    settled(&mut a, &ctx);
    // Every Sel cell, not only one row's.
    assert_eq!(tally(&a, Row::All(Kind::Polygon)), (6, 6));
    assert_eq!(tally(&a, Row::PolygonVertices(4)), (6, 6));
    a.run(Command::Undo);
    settled(&mut a, &ctx);
    assert_eq!(tally(&a, Row::All(Kind::Polygon)).1, 0, "undo");
    a.run(Command::Redo);
    settled(&mut a, &ctx);
    assert_eq!(tally(&a, Row::All(Kind::Polygon)).1, 6, "redo");
    // Nothing changed: no new count.
    let runs = a.statistics.runs;
    settled(&mut a, &ctx);
    assert_eq!(a.statistics.runs, runs);
}

#[test]
fn plus_in_the_list_selects_and_switches_mode() {
    let (mut a, ctx) = app();
    a.editor.scene.selection.mode = Mode::Item;
    settled(&mut a, &ctx);
    // "4 (quads)" is open by default; click its + cell.
    let key = Row::PolygonVertices(4).key();
    let at = a.statistics.tree.cells[&(key, 0)].center();
    let undo = a.editor.undo.len();
    click(&mut a, &ctx, at);
    assert_eq!(a.editor.scene.selection.mode, Mode::Polygon, "{}", a.status);
    assert_eq!(a.editor.scene.selection.faces.len(), 6);
    assert_eq!(a.editor.undo.len(), undo + 1);
    settled(&mut a, &ctx);
    assert_eq!(tally(&a, Row::PolygonVertices(4)), (6, 6));
    // −.
    let at = a.statistics.tree.cells[&(key, 1)].center();
    click(&mut a, &ctx, at);
    assert!(a.editor.scene.selection.faces.is_empty());
    // The click went through the command line, so History has it.
    assert!(
        a.history
            .log
            .records
            .iter()
            .any(|r| r.line.contains("operandi.statistics.select")),
    );
}

#[test]
fn open_groups_count_in_full_and_are_remembered() {
    let (mut a, ctx) = app();
    // Vertices is closed: only its top row is counted.
    assert!(
        a.statistics
            .report
            .as_ref()
            .unwrap()
            .get(&Row::VertexEdges(3))
            .is_none()
    );
    let key = Row::All(Kind::Vertex).key();
    a.stats_event(tree_list::TreeEvent::Disclosure {
        key,
        subtree: false,
    });
    assert!(a.statistics.saved.open.contains("Vertices"));
    settled(&mut a, &ctx);
    assert_eq!(tally(&a, Row::VertexEdges(3)).0, 8);
    // Saved for the next session.
    let path = a.environment.paths().statistics_panel();
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains("Vertices"));
    assert!(
        a.statistics
            .entries
            .values()
            .any(|e| *e == Entry::Heading("vertices.edges"))
    );
}

#[test]
fn renaming_in_place_renames_the_part() {
    let (mut a, ctx) = app();
    a.execute("select.all");
    a.execute("poly.setPart Lid");
    a.statistics.saved.open.insert("polygons.parts".into());
    settled(&mut a, &ctx);
    let key = Row::Part("Lid".into()).key();
    a.stats_event(tree_list::TreeEvent::Rename {
        key,
        name: "Top".into(),
    });
    let part = a.editor.scene.active().unwrap().mesh.faces[0].part.clone();
    assert_eq!(part.as_deref(), Some("Top"), "{}", a.status);
    a.run(Command::Undo);
    let part = a.editor.scene.active().unwrap().mesh.faces[0].part.clone();
    assert_eq!(part.as_deref(), Some("Lid"));
}
