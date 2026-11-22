//! Environments in the Shader Tree and Properties (environments-2026-11-09,
//! "ENV" §6-§8): rows, selection, Properties tabs, create / duplicate /
//! delete as one undo step each, and every Properties row drawn for every
//! Environment Type.
use super::tests::{box_app, draw};
use super::*;
use crate::environment_ui::{ENVIRONMENT_MATERIAL_TAB, ENVIRONMENT_TAB, EnvironmentRow};
use operandi_core::environment::{EnvironmentEdit, EnvironmentType};

const SIZE: Vec2 = vec2(1440., 900.);

fn app() -> (Operandi, Context) {
    let mut a = box_app();
    let ctx = Context::default();
    theme::install(&ctx);
    a.upper_frame.show_page(1);
    a.shading.open_sections.insert("Environments");
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx)
}

fn env_ids(a: &Operandi) -> Vec<u64> {
    a.editor.scene.environments().map(|i| i.id).collect()
}

#[test]
fn create_environment_from_the_menu_and_add_layer() {
    let (mut a, ctx) = app();
    let steps = a.editor.undo.len();
    a.menu_action(&ctx, app_menu::line_action("item.create environment"));
    assert_eq!(a.editor.undo.len(), steps + 1);
    let first = env_ids(&a);
    assert_eq!(first.len(), 1);
    assert_eq!(a.shading.environment, Some(EnvironmentRow::Item(first[0])));
    // The typed command does the same.
    a.execute("item.create environment");
    let both = env_ids(&a);
    assert_eq!(both.len(), 2);
    assert_ne!(both[0], first[0], "the new one is the newest (top-most)");
    assert_eq!(a.editor.scene.selection.item, Some(both[0]));
    a.run(Command::Undo);
    assert_eq!(env_ids(&a), first);
}

#[test]
fn rows_select_and_show_their_properties() {
    let (mut a, ctx) = app();
    a.create_environment();
    let id = env_ids(&a)[0];
    a.shading.environment = None;
    a.select_environment_row(EnvironmentRow::Material(id));
    assert_eq!(a.property_tabs(), vec![ENVIRONMENT_MATERIAL_TAB]);
    a.select_environment_row(EnvironmentRow::Item(id));
    assert_eq!(a.property_tabs(), vec![ENVIRONMENT_TAB]);
    assert_eq!(
        a.editor.scene.selection.item,
        Some(id),
        "selects the item too"
    );
    // A layer click takes the selection back.
    a.shading.selected = vec![4];
    a.shading.environment = None;
    assert_ne!(a.property_tabs(), vec![ENVIRONMENT_TAB]);
    // ENVIRONMENT-PLACEMENT §2: from the Items side too it is the
    // Environment tab alone (no Transform, no shared item tabs).
    a.upper_frame.show_page(0);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.property_tabs(), vec![ENVIRONMENT_TAB]);
}

#[test]
fn every_type_draws_its_rows() {
    let (mut a, ctx) = app();
    a.create_environment();
    let id = env_ids(&a)[0];
    for &kind in EnvironmentType::ALL {
        a.run(Command::EditEnvironment {
            item: id,
            edit: EnvironmentEdit::Kind(kind),
        });
        for row in [EnvironmentRow::Item(id), EnvironmentRow::Material(id)] {
            a.select_environment_row(row);
            for _ in 0..2 {
                draw(&mut a, &ctx, SIZE, vec![]);
            }
        }
    }
    assert!(
        a.status.is_empty() || !a.status.contains("rror"),
        "{}",
        a.status
    );
}

#[test]
fn duplicate_and_delete_are_one_step() {
    let (mut a, _ctx) = app();
    a.create_environment();
    let id = env_ids(&a)[0];
    let steps = a.editor.undo.len();
    a.run(Command::SelectItem(Some(id)));
    a.run(Command::DuplicateItem);
    assert_eq!(a.editor.undo.len(), steps + 1, "Duplicate is one step");
    let ids = env_ids(&a);
    assert_eq!(ids.len(), 2);
    let copy = ids[0];
    a.shading.environment = Some(EnvironmentRow::Item(copy));
    let steps = a.editor.undo.len();
    assert!(a.shading_delete_selected());
    assert_eq!(a.editor.undo.len(), steps + 1, "Delete is one step");
    assert_eq!(env_ids(&a), vec![id]);
    assert_eq!(a.shading.environment, None);
    a.run(Command::Undo);
    assert_eq!(env_ids(&a).len(), 2);
}

/// ENV §8: Add Layer with an environment row selected puts the texture
/// inside it (above the selected layer; above the material = bottom), its
/// Properties draw, and Delete removes it. Each is one undo step.
#[test]
fn layers_under_an_environment() {
    use operandi_core::shader_tree::NewLayer;
    let (mut a, ctx) = app();
    a.create_environment();
    let id = env_ids(&a)[0];
    let steps = a.editor.undo.len();
    a.add_environment_layer(NewLayer::Constant);
    assert_eq!(a.editor.undo.len(), steps + 1);
    let first = a.environment(id).unwrap().layers[0].id;
    assert_eq!(
        a.shading.environment,
        Some(EnvironmentRow::Layer(id, first))
    );
    a.add_environment_layer(NewLayer::ImageMap);
    let names = |a: &Operandi| -> Vec<String> {
        a.environment(id)
            .unwrap()
            .layers
            .iter()
            .map(|n| n.name.clone())
            .collect()
    };
    assert_eq!(
        names(&a),
        ["Image Map", "Constant"],
        "above the selected layer"
    );
    a.select_environment_row(EnvironmentRow::Material(id));
    a.add_environment_layer(NewLayer::Procedural(
        operandi_core::procedural::ProceduralKind::Noise,
    ));
    assert_eq!(
        names(&a),
        ["Image Map", "Constant", "Noise"],
        "above the material"
    );
    a.add_environment_layer(NewLayer::Material);
    assert_eq!(
        names(&a).len(),
        3,
        "a Material can't go under an environment"
    );
    for layer in a.environment(id).unwrap().layers.clone() {
        a.select_environment_row(EnvironmentRow::Layer(id, layer.id));
        assert_eq!(
            a.property_tabs(),
            vec![crate::environment_ui::ENVIRONMENT_LAYER_TAB]
        );
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
    }
    let steps = a.editor.undo.len();
    a.toggle_environment_layer(id, first);
    assert!(
        !a.environment(id)
            .unwrap()
            .layers
            .iter()
            .find(|n| n.id == first)
            .unwrap()
            .layer
            .enable
    );
    a.shading.environment = Some(EnvironmentRow::Layer(id, first));
    assert!(a.shading_delete_selected());
    assert_eq!(names(&a), ["Image Map", "Noise"]);
    assert_eq!(a.editor.undo.len(), steps + 2);
}

/// UIC-160/163: the typed commands behind the rows: item.channel on the
/// item and on its material's layer id, environment.light, item.name.
#[test]
fn typed_commands_set_and_read_environments() {
    let (mut a, _ctx) = app();
    a.create_environment();
    let id = env_ids(&a)[0];
    let material = a.environment(id).unwrap().material.id;
    let steps = a.editor.undo.len();
    a.execute(&format!("item.channel intensity 2 item:{id}"));
    assert_eq!(a.environment(id).unwrap().intensity, 2.0, "{}", a.status);
    a.execute(&format!("item.channel zenColor 1,0,0 layer:{material}"));
    assert_eq!(
        a.environment(id).unwrap().material.settings.zenith,
        [1.0, 0.0, 0.0],
        "{}",
        a.status
    );
    a.execute(&format!("item.channel type Overcast layer:{material}"));
    assert_eq!(
        a.environment(id).unwrap().material.settings.kind,
        EnvironmentType::Overcast
    );
    assert_eq!(a.editor.undo.len(), steps + 3, "one step each");
    a.execute(&format!("item.channel type ? layer:{material}"));
    assert!(a.status.contains("Overcast"), "{}", a.status);
    a.execute("item.create light.directional");
    let sun = a.editor.scene.selection.item.unwrap();
    a.run(Command::SelectItem(Some(id)));
    a.execute(&format!("environment.light sunLight:{sun}"));
    assert_eq!(
        a.environment(id)
            .unwrap()
            .material
            .settings
            .daylight
            .sun_light,
        Some(sun),
        "{}",
        a.status
    );
    a.execute("item.name Sky");
    assert_eq!(a.editor.scene.item(id).unwrap().name, "Sky");
    a.run(Command::Undo);
    a.run(Command::Undo);
    assert_eq!(
        a.environment(id)
            .unwrap()
            .material
            .settings
            .daylight
            .sun_light,
        None
    );
}

/// ENV §9: Background = Environment (view3d.bgEnvironment) draws the
/// camera's environment: zenith colour at the top of a view looking up,
/// sky just above and ground just below the horizon with a hard edge.
#[test]
fn viewport_background_shows_the_environment() {
    let (mut a, ctx) = app();
    a.create_environment();
    let id = env_ids(&a)[0];
    let material = a.environment(id).unwrap().material.id;
    a.execute(&format!("item.channel skyColor 1,0,0 layer:{material}"));
    a.execute(&format!("item.channel gndColor 0,0,1 layer:{material}"));
    a.execute("view3d.bgEnvironment Environment");
    assert_eq!(
        a.display.settings.drawing.background,
        view_display::Background::Environment,
        "{}",
        a.status
    );
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    // A level perspective view: the horizon runs across the middle.
    a.camera.view = View::Perspective;
    a.camera.yaw = 0.0;
    a.camera.pitch = 0.0;
    let r = Rect::from_min_size(Pos2::ZERO, vec2(400., 300.));
    let mesh = a.environment_background_mesh(r).unwrap();
    let mid = r.center().y;
    let near = |above: bool| {
        mesh.vertices
            .iter()
            .filter(|v| (v.pos.y - mid).abs() < 0.5 && (v.pos.x - r.center().x).abs() < 40.)
            .map(|v| v.color)
            .collect::<Vec<_>>()
            .into_iter()
            .find(|c| if above { c.r() > c.b() } else { c.b() > c.r() })
    };
    assert!(near(true).is_some(), "red sky just above the horizon");
    assert!(near(false).is_some(), "blue ground just below");
    // No environment for the camera: none.
    a.execute(&format!("item.channel visCamera false item:{id}"));
    assert!(a.environment_background_mesh(r).is_none());
}

/// ENVIRONMENT-PLACEMENT §1: the Items list of a new scene has no
/// environment rows, and Create Environment adds none; the Shader Tree
/// lists them.
#[test]
fn item_list_has_no_environment_rows() {
    use crate::items_tree::ItemKey;
    use crate::shading_panel::ShaderKey;
    let mut a = Operandi::empty();
    a.initialize_new_document();
    let item_ids = |a: &Operandi| -> Vec<u64> {
        a.item_rows()
            .iter()
            .filter_map(|r| match r.key {
                ItemKey::Item(_, id) => Some(id),
                _ => None,
            })
            .collect()
    };
    let envs = env_ids(&a);
    assert_eq!(envs.len(), 1, "a new scene has its Environment");
    let rows = item_ids(&a);
    assert_eq!(rows.len(), a.editor.scene.items.len() - 1);
    assert!(rows.iter().all(|id| !envs.contains(id)));
    let ctx = Context::default();
    a.menu_action(&ctx, app_menu::line_action("item.create environment"));
    let envs = env_ids(&a);
    assert_eq!(envs.len(), 2);
    assert_eq!(item_ids(&a), rows, "Create Environment adds no Items row");
    // The Shader Tree lists both, newest first.
    let tree: Vec<u64> = a
        .shading_tree_rows()
        .iter()
        .filter_map(|r| match r.key {
            ShaderKey::Environment(id) => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(tree, envs);
}

/// ENVIRONMENT-PLACEMENT §3.1/§3.2/§3.6: Environments is the 6th of 7
/// sections, a plain header that starts collapsed; open, it lists each
/// environment (Environment icon, an eye) above its Environment Material
/// (the material letter).
#[test]
fn environments_section_rows() {
    use crate::shading_panel::ShaderKey;
    use tree_list::{Cell, RowIcon};
    let mut a = Operandi::empty();
    a.initialize_new_document();
    let rows = a.shading_tree_rows();
    let sections: Vec<&str> = rows
        .iter()
        .filter_map(|r| match r.key {
            ShaderKey::Section(name) => Some(name),
            _ => None,
        })
        .collect();
    assert_eq!(
        sections,
        [
            "Render",
            "Library",
            "Nodes",
            "Lights",
            "Environments",
            "Bake Items",
            "FX"
        ]
    );
    let header = rows
        .iter()
        .find(|r| r.key == ShaderKey::Section("Environments"))
        .unwrap();
    assert_eq!(header.disclosure, Some(false), "collapsed, with an arrow");
    assert!(matches!(header.icon, RowIcon::None));
    assert!(header.fixed && !header.renamable && !header.draggable);
    assert!(
        header.cells.iter().all(|c| matches!(c, Cell::Empty)),
        "no eye"
    );
    assert!(
        !rows
            .iter()
            .any(|r| matches!(r.key, ShaderKey::Environment(_))),
        "collapsed"
    );
    // Clicking the header changes nothing but the arrow.
    let selection = a.editor.scene.selection.clone();
    a.shading.open_sections.insert("Environments");
    assert_eq!(a.editor.scene.selection, selection);
    let id = env_ids(&a)[0];
    let rows = a.shading_tree_rows();
    let at = rows
        .iter()
        .position(|r| r.key == ShaderKey::Environment(id))
        .unwrap();
    let env_row = &rows[at];
    assert!(matches!(
        env_row.icon,
        RowIcon::Icon(crate::icons::Icon::Environment)
    ));
    assert!(matches!(env_row.cells[0], Cell::Toggle { on: true, .. }));
    assert_eq!(env_row.disclosure, Some(true));
    let material = &rows[at + 1];
    assert_eq!(material.key, ShaderKey::EnvironmentMaterial(id));
    assert!(matches!(
        material.icon,
        RowIcon::Icon(crate::icons::Icon::TreeMaterial)
    ));
    // The eye hides it: no ray sees it, one undo step.
    let steps = a.editor.undo.len();
    a.run(Command::Visibility(id));
    assert_eq!(a.editor.undo.len(), steps + 1);
    assert!(
        a.editor
            .scene
            .environment_for(operandi_core::environment::RayKind::Camera)
            .is_none()
    );
    // Deleting the last one is allowed: the header loses its arrow.
    a.delete_environment(id);
    assert!(env_ids(&a).is_empty());
    let header = a
        .shading_tree_rows()
        .into_iter()
        .find(|r| r.key == ShaderKey::Section("Environments"))
        .unwrap();
    assert_eq!(header.disclosure, None);
}

/// ENVIRONMENT-PLACEMENT §3.3: click, Shift range and Cmd/Ctrl toggle over
/// Environment rows; Properties shows the primary's item settings (no
/// Transform, none of the shared item tabs).
#[test]
fn environment_rows_select_like_the_shader_tree() {
    use tree_list::ClickHow;
    let (mut a, _ctx) = app();
    for _ in 0..3 {
        a.create_environment();
    }
    let ids = env_ids(&a);
    assert_eq!(ids.len(), 3);
    assert_eq!(
        a.shading.environment,
        Some(EnvironmentRow::Item(ids[0])),
        "newest selected"
    );
    a.click_environment_row(ids[0], ClickHow::Only);
    a.click_environment_row(ids[2], ClickHow::Range);
    let mut selected = a.editor.scene.selected_items();
    selected.sort();
    let mut all = ids.clone();
    all.sort();
    assert_eq!(selected, all, "Shift: the range");
    a.click_environment_row(ids[1], ClickHow::Toggle);
    assert!(
        !a.editor.scene.selected_items().contains(&ids[1]),
        "Cmd: off"
    );
    assert_eq!(a.property_tabs(), vec![ENVIRONMENT_TAB]);
    a.click_environment_row(ids[1], ClickHow::Only);
    assert_eq!(a.editor.scene.selected_items(), vec![ids[1]]);
    // The same tab alone from the Items side too.
    a.shading.environment = None;
    a.upper_frame.show_page(0);
    assert_eq!(a.property_tabs(), vec![ENVIRONMENT_TAB]);
}
