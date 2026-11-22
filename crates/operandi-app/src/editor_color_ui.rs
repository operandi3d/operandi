//! Item editor colours in the app (note 10): the Editor Color submenu the
//! list row menus share, the By Color filter/select rows and the
//! `item.editorColor` command. The data lives in operandi-core
//! (editor_color.rs); the RGB comes from the theme (`theme::editor_color`).
use super::*;
use operandi_core::EditorColor;
use operandi_core::item_common::ItemCommonEdit;

/// The menu order: (none), then the 17 colours.
pub(super) fn choices() -> impl Iterator<Item = Option<EditorColor>> {
    std::iter::once(None).chain(EditorColor::ALL.into_iter().map(Some))
}

/// A small swatch and the colour's name.
fn swatch_label(ui: &mut Ui, color: Option<EditorColor>, checked: bool) -> Response {
    let text = format!(
        "{}{}",
        if checked { "✓ " } else { "   " },
        EditorColor::label_of(color)
    );
    let response = ui.add(Button::new(text).min_size(vec2(130., 0.)));
    if let Some(c) = color {
        let r = response.rect;
        let dot = Rect::from_center_size(pos2(r.right() - 10., r.center().y), vec2(10., 10.));
        ui.painter().rect_filled(dot, 2., theme::editor_color(c));
    }
    response
}

/// The Editor Color submenu: `Some(choice)` when one was picked.
pub(super) fn submenu(ui: &mut Ui, current: Option<EditorColor>) -> Option<Option<EditorColor>> {
    let mut picked = None;
    ui.menu_button("Editor Color", |ui| {
        for c in choices() {
            if swatch_label(ui, c, c == current).clicked() {
                picked = Some(c);
                ui.close();
            }
        }
    });
    picked
}

/// By Color and the colour dropdown (Filter and Select popovers).
pub(super) fn by_color_rows(ui: &mut Ui, id: &str, on: &mut bool, color: &mut Option<EditorColor>) {
    ui.horizontal(|ui| {
        ui.checkbox(on, "By Color");
        egui::ComboBox::from_id_salt(id)
            .selected_text(EditorColor::label_of(*color))
            .show_ui(ui, |ui| {
                for c in choices() {
                    if swatch_label(ui, c, c == *color).clicked() {
                        *color = c;
                    }
                }
            });
    });
}

impl Operandi {
    /// Set the Editor Color of `items`: one undo step (IC-B3).
    pub(super) fn set_item_editor_color(&mut self, items: Vec<u64>, color: Option<EditorColor>) {
        self.run(Command::EditItemCommon {
            items,
            edit: ItemCommonEdit::EditorColor(color),
        });
    }

    /// The row menu's targets: every selected item when the clicked one is
    /// selected, else just the clicked one.
    pub(super) fn menu_targets(&self, clicked: u64) -> Vec<u64> {
        let items = self.editor.scene.selected_items();
        if items.contains(&clicked) {
            items
        } else {
            vec![clicked]
        }
    }

    /// `item.editorColor [color]`: set the selected items' colour by name
    /// ("none" clears), or with no argument report it.
    pub(super) fn editor_color_command(&mut self, args: &[String]) {
        let items = self.editor.scene.selected_items();
        if items.is_empty() {
            self.status = "item.editorColor: select an item".into();
            return;
        }
        let bound = match operandi_core::command_args::bind("item.editorColor", args, &["color"]) {
            Ok(b) => b,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        match bound[0] {
            None => {
                let colors: std::collections::BTreeSet<_> = self
                    .editor
                    .scene
                    .items
                    .iter()
                    .filter(|i| items.contains(&i.id))
                    .map(|i| i.common.editor_color)
                    .collect();
                self.status = match colors.into_iter().collect::<Vec<_>>().as_slice() {
                    [one] => format!("Editor Color: {}", EditorColor::key_of(*one)),
                    _ => "Editor Color: mixed".into(),
                };
            }
            Some(name) => {
                let color = EditorColor::from_name(name);
                if color.is_none() && !name.trim().eq_ignore_ascii_case("none") {
                    let names: Vec<&str> = std::iter::once("none")
                        .chain(EditorColor::ALL.iter().map(|c| c.key()))
                        .collect();
                    self.status = format!(
                        "item.editorColor: unknown color {name}; colors are {}",
                        names.join(", ")
                    );
                    return;
                }
                self.set_item_editor_color(items, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> (Operandi, Vec<u64>) {
        let mut app = Operandi::empty();
        for _ in 0..3 {
            app.run(Command::NewMesh);
        }
        let ids: Vec<u64> = app.editor.scene.items.iter().map(|i| i.id).collect();
        (app, ids)
    }

    fn color_of(app: &Operandi, id: u64) -> Option<EditorColor> {
        let item = app.editor.scene.item(id).unwrap();
        item.common.editor_color
    }

    #[test]
    fn the_command_sets_and_reports_by_name() {
        let (mut app, ids) = app();
        app.run(Command::SelectItem(Some(ids[0])));
        app.execute("item.editorColor color:orange");
        assert_eq!(color_of(&app, ids[0]), Some(EditorColor::Orange));
        app.execute("item.editorColor");
        assert_eq!(app.status, "Editor Color: orange");
        app.execute("item.editorColor \"Light Blue\"");
        assert_eq!(color_of(&app, ids[0]), Some(EditorColor::LightBlue));
        app.execute("item.editorColor none");
        assert_eq!(color_of(&app, ids[0]), None);
        app.execute("item.editorColor chartreuse");
        assert!(app.status.contains("unknown color"), "{}", app.status);
    }

    #[test]
    fn menu_targets_follow_the_selection() {
        let (mut app, ids) = app();
        app.run(Command::SelectItem(Some(ids[0])));
        app.run(Command::ToggleItemSelection(ids[1]));
        let mut both = app.menu_targets(ids[1]);
        both.sort_unstable();
        assert_eq!(both, vec![ids[0], ids[1]]);
        assert_eq!(app.menu_targets(ids[2]), vec![ids[2]]);
        let steps = app.editor.undo.len();
        app.set_item_editor_color(both, Some(EditorColor::Orange));
        assert_eq!(app.editor.undo.len(), steps + 1);
        app.run(Command::Undo);
        assert_eq!(color_of(&app, ids[0]), None);
        assert_eq!(color_of(&app, ids[1]), None);
    }

    #[test]
    fn every_editor_color_has_a_theme_entry() {
        for c in EditorColor::ALL {
            assert_ne!(theme::editor_color(c), Color32::TRANSPARENT);
        }
    }
}
