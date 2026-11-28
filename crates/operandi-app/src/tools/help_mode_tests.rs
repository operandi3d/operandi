//! Help mode (help_mode.rs, UIC-06 / UIC-301): F1, a click, the guide page
//! for what was clicked, and nothing else happens.
use super::help_mode::*;
use super::tests::{draw, key};
use super::*;
use crate::update_check::Channel;

const SIZE: Vec2 = vec2(1440., 1000.);

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    draw(a, ctx, SIZE, events);
}

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx
}

fn release(k: Key) -> Event {
    Event::Key {
        key: k,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn tap(a: &mut Operandi, ctx: &Context, k: Key) {
    frame(a, ctx, vec![key(k)]);
    frame(a, ctx, vec![release(k)]);
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2, button: PointerButton) {
    let event = |pressed| Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    };
    frame(a, ctx, vec![Event::PointerMoved(at)]);
    frame(a, ctx, vec![event(true)]);
    frame(a, ctx, vec![event(false)]);
}

fn tool_on(a: &Operandi) -> bool {
    a.editor.session.has_active_task() || a.utility.is_some() || a.draw.is_some()
}

/// The box scene on its first frames, with the Cube button drawn.
fn app() -> (Operandi, Context, Rect) {
    let ctx = context();
    let mut a = super::tests::box_app();
    frame(&mut a, &ctx, vec![]);
    frame(&mut a, &ctx, vec![]);
    let cube = a
        .form_rects
        .get("Cube")
        .or_else(|| a.toolbox_rects.get("Cube"))
        .copied()
        .expect("the Create tab's Cube button is drawn");
    (a, ctx, cube)
}

#[test]
fn help_urls_go_through_the_guides_go_redirects() {
    assert_eq!(
        help_url_for("0.2.0", Channel::Release, "poly.bevel"),
        "https://operandi3d.org/guide/0.2.0/go/poly.bevel/"
    );
    assert_eq!(
        help_url_for("0.2.0", Channel::Test, "Items List"),
        "https://operandi3d.org/guide/test/go/Items%20List/"
    );
    assert_eq!(
        help_url_for("0.2.0", Channel::Test, "What's This?"),
        "https://operandi3d.org/guide/test/go/What%27s%20This%3F/"
    );
    assert!(help_url("Cube").starts_with(&help_menu::guide_url()));
}

#[test]
fn controls_name_their_commands_registry_name() {
    assert_eq!(control_id("Cube").as_deref(), Some("Cube"));
    assert_eq!(control_id("Bevel").as_deref(), Some("Bevel"));
    // A flyout's button is its tool; an alias resolves to the entry.
    assert_eq!(control_id("flyout:Soft Drag").as_deref(), Some("Soft Drag"));
    assert_eq!(control_id("What's This?").as_deref(), Some(COMMAND));
    // A form line with arguments is its command.
    assert_eq!(
        control_id("operandi.help.reportBug screenshot:false").as_deref(),
        Some("operandi.help.reportBug")
    );
    // A tab isn't a command: the panel answers for it. Tool Properties'
    // header is its panel command.
    assert_eq!(control_id("Category Curve"), None);
    assert_eq!(
        control_id("Tool Properties").as_deref(),
        Some(TOOL_PROPERTIES)
    );
}

#[test]
fn panels_and_tools_have_the_guides_ids() {
    use panel_registry::PanelKind;
    assert_eq!(panel_help_id(PanelKind::Items), "Items List");
    assert_eq!(panel_help_id(PanelKind::ShaderTree), "Shader Tree");
    assert_eq!(panel_help_id(PanelKind::Properties), "Properties");
    assert_eq!(panel_help_id(PanelKind::Missing(0)), INTERFACE);
    assert_eq!(tool_help_id("Bevel"), "Bevel");
    assert_eq!(tool_help_id("Tool Properties"), TOOL_PROPERTIES);
}

#[test]
fn the_smallest_control_then_the_smallest_panel_then_interface() {
    let r = |x0, y0, x1, y1| Rect::from_min_max(pos2(x0, y0), pos2(x1, y1));
    let controls = vec![
        (r(0., 0., 100., 100.), "Big".to_owned()),
        (r(10., 10., 20., 20.), "Small".into()),
    ];
    let regions = vec![
        (r(0., 0., 500., 500.), "Toolbox".to_owned()),
        (r(200., 200., 300., 300.), "Tool Properties".into()),
    ];
    assert_eq!(pick(pos2(15., 15.), controls.clone(), &regions), "Small");
    assert_eq!(pick(pos2(50., 50.), controls.clone(), &regions), "Big");
    assert_eq!(
        pick(pos2(250., 250.), controls.clone(), &regions),
        "Tool Properties"
    );
    assert_eq!(
        pick(pos2(400., 400.), controls.clone(), &regions),
        "Toolbox"
    );
    assert_eq!(pick(pos2(900., 900.), controls, &regions), INTERFACE);
}

/// UIC-06: F1, then a click on a toolbox button, opens that tool's page
/// and doesn't start the tool; help mode then ends.
#[test]
fn f1_then_a_toolbox_click_opens_the_tools_page_without_running_it() {
    let (mut a, ctx, cube) = app();
    tap(&mut a, &ctx, Key::F1);
    assert!(a.help_mode.on, "F1 turns help mode on: {}", a.status);
    assert_eq!(a.status, HINT);
    click(&mut a, &ctx, cube.center(), PointerButton::Primary);
    assert_eq!(a.opened_url.take(), Some(help_url("Cube")));
    assert!(!tool_on(&a), "the click didn't start Cube");
    assert!(!a.help_mode.on);
    // The next click is an ordinary one again.
    click(&mut a, &ctx, cube.center(), PointerButton::Primary);
    assert!(tool_on(&a), "Cube starts on a click outside help mode");
    assert_eq!(a.opened_url, None);
}

#[test]
fn escape_or_a_right_click_leaves_without_opening_anything() {
    let (mut a, ctx, cube) = app();
    a.execute(COMMAND);
    assert!(a.help_mode.on);
    tap(&mut a, &ctx, Key::Escape);
    assert!(!a.help_mode.on);
    assert_eq!(a.opened_url, None);
    a.execute(COMMAND);
    click(&mut a, &ctx, cube.center(), PointerButton::Secondary);
    assert!(!a.help_mode.on);
    assert_eq!(a.opened_url, None);
    assert!(!tool_on(&a));
    // Running it again leaves too.
    a.execute(COMMAND);
    a.execute(COMMAND);
    assert!(!a.help_mode.on);
}

#[test]
fn a_click_in_the_view_or_a_panel_opens_its_page() {
    let (mut a, ctx, _) = app();
    a.execute(COMMAND);
    let view = a.viewport_rect.center();
    click(&mut a, &ctx, view, PointerButton::Primary);
    assert_eq!(a.opened_url.take(), Some(help_url(VIEW)));
    assert!(a.editor.scene.selection.faces.is_empty());
    a.execute(COMMAND);
    let header = a.toolbox_rects["Tool Properties"];
    click(&mut a, &ctx, header.center(), PointerButton::Primary);
    assert_eq!(a.opened_url.take(), Some(help_url(TOOL_PROPERTIES)));
}

#[test]
fn a_menu_entry_opens_its_commands_page_instead_of_running() {
    let (mut a, ctx, _) = app();
    a.execute(COMMAND);
    a.menu_action(
        &ctx,
        crate::app_menu::line_action("operandi.help.reportBug"),
    );
    assert_eq!(
        a.opened_url.take(),
        Some(help_url("operandi.help.reportBug"))
    );
    assert!(!a.report_bug.is_modal(), "Report Bug didn't run");
    assert!(!a.help_mode.on);
    // The Help menu has the entry, with F1 from the key map.
    let model = a.menu_model(&ctx);
    let help = model.last().unwrap();
    let entry = help
        .leaves()
        .into_iter()
        .find(|i| i.action == crate::app_menu::line_action(COMMAND))
        .expect("Help > What's This?");
    assert!(entry.title.starts_with("What's This?"), "{}", entry.title);
    assert!(entry.title.contains("F1"), "{}", entry.title);
    a.menu_action(&ctx, crate::app_menu::line_action(COMMAND));
    assert!(a.help_mode.on);
    a.menu_action(&ctx, crate::app_menu::line_action(COMMAND));
    assert!(!a.help_mode.on, "choosing it again leaves help mode");
}

/// Every help id the guide claims (docs/guide/src page headers, RULES.md
/// section 5).
fn guide_ids() -> std::collections::BTreeSet<String> {
    fn walk(dir: &std::path::Path, out: &mut std::collections::BTreeSet<String>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "md") {
                let text = std::fs::read_to_string(&path).unwrap();
                let first = text.lines().next().unwrap_or_default();
                if let Some(rest) = first.trim().strip_prefix("<!--")
                    && let Some(rest) = rest.trim().strip_prefix("help-ids:")
                {
                    let ids = rest.split([';', '>']).next().unwrap_or_default();
                    out.extend(
                        ids.split(',')
                            .map(|i| i.trim().trim_end_matches("--").trim().to_owned())
                            .filter(|i| !i.is_empty()),
                    );
                }
            }
        }
    }
    let mut ids = Default::default();
    walk(std::path::Path::new("../../docs/guide/src"), &mut ids);
    ids
}

/// Registry panels the guide has no page for yet: help mode still opens
/// their id, which lands on the guide's search.
const PANELS_WITHOUT_A_PAGE: &[&str] = &[
    "Command History",
    "Groups",
    "Mesh Stack",
    "Node View",
    "Preset Browser",
    "Statistics",
    "UV Maps",
    "Vertex Normal Tools",
];

#[test]
fn every_fixed_help_id_has_a_guide_page() {
    let ids = guide_ids();
    for id in [INTERFACE, VIEW, TOOLBOX, TOOL_PROPERTIES, COMMAND] {
        assert!(ids.contains(id), "no guide page claims help id {id:?}");
    }
    for panel in panel_registry::REGISTRY {
        let id = panel_help_id(panel.kind);
        assert_eq!(
            ids.contains(&id),
            !PANELS_WITHOUT_A_PAGE.contains(&id.as_str()),
            "panel {id:?}: a page claims it, or it is in PANELS_WITHOUT_A_PAGE (not both)"
        );
    }
}

/// The commands on the menus and toolbox forms that no guide page claims:
/// help mode opens the guide's search for them. A report, not a check:
///
///   cargo test --locked --workspace -- help_mode_tests::guide_gaps --ignored --nocapture
#[test]
#[ignore = "report of commands without a guide page"]
fn guide_gaps() {
    let ids = guide_ids();
    let mut missing = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir("assets/forms").unwrap().flatten() {
        let text = std::fs::read_to_string(entry.path()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let mut stack = vec![json];
        while let Some(v) = stack.pop() {
            match v {
                serde_json::Value::Object(map) => {
                    if let Some(serde_json::Value::String(line)) = map.get("command")
                        && let Some(id) = control_id(line)
                        && !ids.contains(&id)
                    {
                        missing.insert(id);
                    }
                    stack.extend(map.into_iter().map(|(_, v)| v));
                }
                serde_json::Value::Array(list) => stack.extend(list),
                _ => {}
            }
        }
    }
    println!(
        "{} menu/toolbox commands without a guide page:",
        missing.len()
    );
    for id in &missing {
        println!("  {id}");
    }
}
