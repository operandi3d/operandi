//! Items list header (spec item-list-header v1, IL-00..IL-08): the Add
//! Item split button and its menu, the Select ("Item Patterns") and Filter
//! ("Item List Filters") popovers, and the three small toggles. The scene
//! work is `Command::AddItem` / `Command::ItemPattern` and the view is
//! `item_list::list_rows`, all in operandi-core.
use crate::unavailable::Gate;
use eframe::egui::{
    self, Button, PopupCloseBehavior, TextEdit, Ui,
    containers::menu::{MenuButton, MenuConfig},
};
use operandi_core::camera_light::LightKind;
use operandi_core::editor::Command;
use operandi_core::item_list::{
    AddItemKind, ItemType, ListFilter, PatternAction, PatternQuery, TypeTicks, push_recent,
};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

pub(crate) const ADD_ITEM: &str = "add-item";

/// Add Item's contents (AB §6, AB-50): the items Operandi can make, the
/// everyday ones in Items and the rest in their category folders. Entries
/// it can't make yet aren't listed and empty folders don't show.
pub(crate) fn item_catalog() -> crate::add_browser::Catalog {
    fn walk(out: &mut Vec<crate::add_browser::Entry>, folder: &[String], entries: &[Entry]) {
        for e in entries {
            match e {
                Item(label, Some(kind)) => out.push(crate::add_browser::Entry {
                    id: kind.key().into(),
                    label: (*label).into(),
                    folder: folder.to_vec(),
                    icon: crate::icons::Icon::from_slug(match kind {
                        AddItemKind::Light(_) => "tree-light",
                        AddItemKind::GroupLocator => "tree-group",
                        AddItemKind::Environment => "tree-environment",
                        AddItemKind::Backdrop => "tree-texture",
                        AddItemKind::Camera => "render",
                        AddItemKind::Mesh => "tree-mesh",
                    }),
                }),
                Item(_, None) => {}
                Sub(name, inner) => {
                    let mut f = folder.to_vec();
                    f.push((*name).into());
                    walk(out, &f, inner);
                }
            }
        }
    }
    let items = vec!["Items".to_string()];
    let mut entries = Vec::new();
    walk(&mut entries, &items, &TOP);
    for (name, inner) in CATEGORIES {
        let mut f = items.clone();
        f.push(name.into());
        walk(&mut entries, &f, inner);
    }
    crate::add_browser::Catalog {
        roots: vec!["Items".into(), "Assemblies".into()],
        entries,
        plain: false,
    }
}

/// Tooltip on every Add Item entry Operandi can't make yet (spec
/// "Recommendation for the builder").
pub(crate) const NOT_YET: &str = "Not yet in Operandi";

/// One Add Item menu entry: a type (made by `kind`, or disabled when
/// `None`) or a submenu.
pub(crate) enum Entry {
    Item(&'static str, Option<AddItemKind>),
    Sub(&'static str, &'static [Entry]),
}
use Entry::{Item, Sub};

const fn light(kind: LightKind) -> Option<AddItemKind> {
    Some(AddItemKind::Light(kind))
}

/// IL-02, top level above the separator.
pub(crate) const TOP: [Entry; 3] = [
    Item("Backdrop Item", Some(AddItemKind::Backdrop)),
    Item("Camera", Some(AddItemKind::Camera)),
    Item("Mesh", Some(AddItemKind::Mesh)),
];

/// IL-02 categories, in the spec's order (alphabetical, Other last).
pub(crate) const CATEGORIES: [(&str, &[Entry]); 10] = [
    (
        "Deformers",
        &[
            Item("Bend Effector", None),
            Item("Curve Constraint Effector", None),
            Item("Lag Effector", None),
            Item("Lattice Effector", None),
            Item("Magnet Effector", None),
            Item("Mesh Wrap Effector", None),
            Item("Morph Container", None),
            Item("Morph Influence", None),
            Item("Soft Lag", None),
            Item("Twist Effector", None),
            Item("Weight Container", None),
        ],
    ),
    (
        "Dynamics",
        &[
            Item("Collision Emitter", None),
            Item("Dynamic Replicator", None),
            Item("Procedural Shatter", None),
            Sub(
                "Constraints",
                &[
                    Item("Constraint Modifier", None),
                    Item("Hinge", None),
                    Item("Pin", None),
                    Item("Point", None),
                    Item("Slide Hinge", None),
                    Item("Spring", None),
                ],
            ),
        ],
    ),
    (
        "Falloffs",
        &[
            Item("Area Falloff", None),
            Item("Bezier Falloff", None),
            Item("Blend Falloff", None),
            Item("Capsule Falloff", None),
            Item("Curve Falloff", None),
            Item("Falloff Operator", None),
            Item("Incidence Falloff", None),
            Item("Linear Falloff", None),
            Item("Radial Falloff", None),
            Item("Spline Falloff", None),
            Item("Texture Falloff", None),
            Item("Weight Map Falloff", None),
        ],
    ),
    (
        "Forces",
        &[
            Item("Curve Force", None),
            Item("Drag Force", None),
            Item("Linear Force", None),
            Item("Newton Force", None),
            Item("Radial Force", None),
            Item("Turbulence Force", None),
            Item("Vortex Force", None),
            Item("Wind Force", None),
        ],
    ),
    (
        "Lights",
        &[
            Item("Area Light", light(LightKind::Area)),
            Item("Cylinder Light", light(LightKind::Cylinder)),
            Item("Directional Light", light(LightKind::Directional)),
            Item("Dome Light", light(LightKind::Dome)),
            Item("Mesh Light", light(LightKind::Mesh)),
            Item("Photometric Light", light(LightKind::Photometric)),
            Item("Point Light", light(LightKind::Point)),
            Item("Portal", light(LightKind::Portal)),
            Item("Spot Light", light(LightKind::Spot)),
        ],
    ),
    (
        "Locators",
        &[
            Item("Group Locator", Some(AddItemKind::GroupLocator)),
            Item("Locator", None),
            Item("Texture Locator", None),
        ],
    ),
    (
        "Particles",
        &[
            Item("Replicator", None),
            Sub(
                "Modifiers",
                &[
                    Item("Particle Audio", None),
                    Item("Particle Expression", None),
                    Item("Particle Look At", None),
                    Item("Particle Modifier", None),
                    Item("Particle Random", None),
                    Item("Particle Sieve", None),
                    Item("Particle Snap", None),
                    Item("Particle Step", None),
                    Item("Particle UVT Modifier", None),
                ],
            ),
            Sub(
                "Point Clouds",
                &[
                    Item("CSV Point Cache", None),
                    Item("Curve Particle Generator", None),
                    Item("Particle Cloud", None),
                    Item("Particle Generator", None),
                    Item("Realflow Particles", None),
                    Item("Stitch Particle Generator", None),
                    Item("Surface Particle Generator", None),
                ],
            ),
            Sub(
                "Simulation",
                &[
                    Item("Collector / Emitter", None),
                    Item("Curve Emitter", None),
                    Item("Dynamic Collider", None),
                    Item("Dynamic Fluid", None),
                    Item("Particle Flocking", None),
                    Item("Particle Operator", None),
                    Item("Particle Simulation", None),
                    Item("Radial Emitter", None),
                    Item("Source Emitter", None),
                    Item("Surface Emitter", None),
                    Item("Terminator", None),
                ],
            ),
        ],
    ),
    (
        "Procedural",
        &[
            Item("Deferred Mesh", None),
            Item("Fractal Gasket", None),
            Item("Lace Item", None),
            Item("Poisson Particles", None),
            Item("Proxy", None),
            Item("Sphere", None),
            Item("SubDFusion", None),
        ],
    ),
    (
        "Volumes",
        &[
            Item("Blob", None),
            Item("Ground Plane", None),
            Item("Render Boolean", None),
            Item("Sprite", None),
            Item("VDBVoxel", None),
            Item("Volume", None),
        ],
    ),
    (
        "Other",
        &[
            Item("Bezier Node", None),
            Item("Canvas", None),
            Item("Channel Handle", None),
        ],
    ),
];

/// True when some entry under `entries` can be made.
pub(crate) fn any_enabled(entries: &[Entry]) -> bool {
    entries.iter().any(|e| match e {
        Item(_, kind) => kind.is_some(),
        Sub(_, inner) => any_enabled(inner),
    })
}

/// Every type entry whose label contains `find` (case-insensitive), in
/// menu order, with whether its category is on show.
pub(crate) fn find_entries(
    find: &str,
    show_unsupported: bool,
) -> Vec<(&'static str, Option<AddItemKind>)> {
    fn walk(
        entries: &'static [Entry],
        find: &str,
        shown: bool,
        out: &mut Vec<(&'static str, Option<AddItemKind>)>,
    ) {
        for e in entries {
            match e {
                Item(label, kind) => {
                    if shown && label.to_lowercase().contains(find) {
                        out.push((label, *kind));
                    }
                }
                Sub(_, inner) => walk(inner, find, shown, out),
            }
        }
    }
    let find = find.trim().to_lowercase();
    let mut out = Vec::new();
    walk(&TOP, &find, true, &mut out);
    for (_, entries) in CATEGORIES {
        walk(
            entries,
            &find,
            show_unsupported || any_enabled(entries),
            &mut out,
        );
    }
    out
}

/// What the header remembers across restarts, in `item-list.json`:
/// Recent (IL-04) and the Show unsupported choice.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ItemListPreferences {
    /// Add Item keys (`AddItemKind::key`), newest first.
    pub recent: Vec<String>,
    pub show_unsupported: bool,
}

impl ItemListPreferences {
    pub fn load(path: &Path) -> Self {
        fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= 16 * 1024)
            .and_then(|_| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec(self).map_err(io::Error::other)?;
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }
    /// Recent as kinds, skipping keys this build doesn't know.
    pub fn recent_kinds(&self) -> Vec<AddItemKind> {
        self.recent
            .iter()
            .filter_map(|k| AddItemKind::from_key(k))
            .collect()
    }
    pub fn push_recent(&mut self, kind: AddItemKind) {
        let mut kinds = self.recent_kinds();
        push_recent(&mut kinds, kind);
        self.recent = kinds.iter().map(|k| k.key().to_owned()).collect();
    }
}

/// The header's state. The filter is saved with the layout (IL-06), the
/// Select settings last the session (IL-05).
#[derive(Default)]
pub struct ItemListUi {
    pub prefs: ItemListPreferences,
    pub filter: ListFilter,
    pub pattern: PatternQuery,
    /// The Add Item menu's find field (IL-02, IL-Q1 provisional).
    pub find: String,
}

fn popover() -> MenuConfig {
    MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside)
}

/// All / None / Invert for a set of type ticks.
fn tick_buttons(ui: &mut Ui, ticks: &mut TypeTicks) {
    ui.horizontal(|ui| {
        if ui.button("All").clicked() {
            *ticks = TypeTicks::all();
        }
        if ui.button("None").clicked() {
            *ticks = TypeTicks::none();
        }
        if ui.button("Invert").clicked() {
            *ticks = ticks.invert();
        }
    });
}
fn type_checkboxes(ui: &mut Ui, ticks: &mut TypeTicks) {
    for t in ItemType::ALL {
        let mut on = ticks.get(t);
        if ui.checkbox(&mut on, t.label()).changed() {
            ticks.set(t, on);
        }
    }
}

/// Command-line commands, `<domain>.<verb>` like backdrop.add
/// (provisional names, IL-CMD).
pub(crate) const COMMANDS: [(&str, &str); 3] = [
    (
        "item.create",
        "Help: item.create <type> (old spelling item.add) adds one item after the selected item, alone selected (Add Item); an environment goes last, so it is the newest. Types: mesh, camera, backdrop, environment, light.area, light.cylinder, light.directional, light.dome, light.mesh, light.photometric, light.point, light.portal, light.spot.",
    ),
    (
        "item.pattern",
        "Help: item.pattern <select|unselect|hide> [pattern] acts on every item whose name matches (* any run, ? one character, any case; default *), as in the Items list Select popover.",
    ),
    (
        // Matched lower-cased; the name is item.editorColor (note 10).
        "item.editorcolor",
        "Help: item.editorColor [color:<name>] sets the selected items' Editor Color (none, red, magenta, pink, brown, orange, yellow, green, lightgreen, blue, lightblue, cyan, ultramarine, purple, lightpurple, white, grey, darkgrey); with no color it reports it.",
    ),
];

impl super::Operandi {
    /// Run an item.create / item.pattern line or its help. False when `text`
    /// isn't one.
    pub(super) fn item_list_canonical(&mut self, text: &str) -> bool {
        let Ok(args) = crate::command::tokens(text) else {
            return false;
        };
        let Some(first) = args.first().map(|a| a.to_ascii_lowercase()) else {
            return false;
        };
        let help = |topic: &str| COMMANDS.iter().find(|(c, _)| *c == topic).map(|(_, h)| *h);
        if first == "help" && args.len() == 2 {
            return match help(&args[1].to_ascii_lowercase()) {
                Some(h) => {
                    self.status = h.into();
                    true
                }
                None => false,
            };
        }
        let Some(usage) = help(&first) else {
            return false;
        };
        let usage = usage.trim_start_matches("Help: ").to_owned();
        match (first.as_str(), args.len()) {
            ("item.create", 2) => match AddItemKind::from_key(&args[1].to_ascii_lowercase()) {
                Some(kind) => self.add_item(kind),
                None => self.status = usage,
            },
            ("item.pattern", 2 | 3) => {
                match PatternAction::from_key(&args[1].to_ascii_lowercase()) {
                    Some(action) => {
                        let query = PatternQuery {
                            pattern: args.get(2).cloned().unwrap_or_else(|| "*".into()),
                            ..Default::default()
                        };
                        self.run(Command::ItemPattern { query, action });
                    }
                    None => self.status = usage,
                }
            }
            ("item.editorcolor", _) => self.editor_color_command(&args[1..]),
            _ => self.status = usage,
        }
        true
    }

    /// IL-00 tool row (UIC-132, UIC-150): Add Item ▾ on the left; Select,
    /// Filter and the toggles on the right, then the funnel, which opens
    /// the list's search row (IL-07, UIC-40).
    pub(super) fn item_list_header(&mut self, ui: &mut Ui) {
        let mut state = std::mem::take(&mut self.items_tree);
        let mut rects: Vec<(&'static str, egui::Rect)> = Vec::new();
        crate::tree_list::tool_row(ui, &mut state, true, |ui, side| {
            if side == crate::tree_list::Side::Left {
                // IL-01, AB-01: the label opens the Add Item browser, the
                // arrow the Add Item menu.
                let label = ui
                    .button("Add Item")
                    .on_hover_text("Add new items to the scene.");
                if label.clicked() {
                    let entry = self
                        .add_browsers
                        .entry(ADD_ITEM)
                        .or_insert_with(|| (Default::default(), egui::Rect::NOTHING));
                    entry.0.open = !entry.0.open;
                    entry.1 = label.rect;
                }
                let (arrow, _) = MenuButton::new("⏷")
                    .config(popover())
                    .ui(ui, |ui| self.add_item_menu(ui));
                let arrow = arrow.on_hover_text("Add new items to the scene.");
                rects.push(("Scenes Add Item", label.rect));
                rects.push(("Item list Add Item arrow", arrow.rect));
                return;
            }
            {
                // Right to left: the funnel is already at the far right.
                let notes = ui.gated(
                    Some("Only search the notes for an item. Not built yet in the Items list"),
                    Button::selectable(false, "Notes"),
                );
                // Text toggles until the icon generator has recipes for
                // them (provisional).
                let only =
                    crate::tree_list::toggle_button(ui, "Sel", self.item_list.filter.only_selected)
                        .on_hover_text("Show only selected items.");
                if only.clicked() {
                    self.item_list.filter.only_selected ^= true;
                }
                let (filter, _) = MenuButton::new("Filter")
                    .config(popover())
                    .ui(ui, |ui| self.item_filters_popover(ui));
                let filter = filter.on_hover_text("Sort and filter items by item type.");
                let (select, _) = MenuButton::new("Select")
                    .config(popover())
                    .ui(ui, |ui| self.item_patterns_popover(ui));
                let select = select.on_hover_text("Select items in the tree by a name pattern.");
                rects.push(("Item list Select", select.rect));
                rects.push(("Item list Filter", filter.rect));
                rects.push(("Item list Only Selected", only.rect));
                rects.push(("Item list Notes", notes.rect));
            }
        });
        if let Some(&funnel) = state.tool_rects.get("funnel") {
            rects.push(("Item list Text", funnel));
        }
        self.items_tree = state;
        if let Some(key) = self.show_add_browser(ui.ctx(), ADD_ITEM, "Add Item", item_catalog())
            && let Some(kind) = AddItemKind::from_key(&key)
        {
            self.add_item(kind);
        }
        #[cfg(test)]
        for (name, rect) in rects {
            self.toolbox_rects.insert(name.into(), rect);
        }
        #[cfg(not(test))]
        let _ = rects;
    }

    /// IL-02 Add Item menu.
    fn add_item_menu(&mut self, ui: &mut Ui) {
        self.menu_owned_input = true;
        ui.set_min_width(180.);
        let find = ui.add(
            TextEdit::singleline(&mut self.item_list.find)
                .hint_text("Find")
                .id_salt("add-item-find"),
        );
        self.text_editors.insert(find.id);
        crate::numeric_tab::register(ui, &find, Some(&self.item_list.find));
        let show_unsupported = self.item_list.prefs.show_unsupported;
        if !self.item_list.find.trim().is_empty() {
            let found = find_entries(&self.item_list.find, show_unsupported);
            if found.is_empty() {
                ui.gated(
                    Some("No item type matches the search"),
                    Button::new("(no match)"),
                );
            }
            // Enter makes the first entry that can be made.
            if find.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && let Some(kind) = found.iter().find_map(|(_, k)| *k)
            {
                self.add_item(kind);
                ui.close();
                return;
            }
            for (label, kind) in found {
                self.add_item_entry(ui, label, kind);
            }
            return;
        }
        let _recent = ui.menu_button("Recent", |ui| {
            let recent = self.item_list.prefs.recent_kinds();
            if recent.is_empty() {
                let _none = ui.gated(Some("Nothing added yet"), Button::new("(none)"));
                #[cfg(test)]
                self.toolbox_rects
                    .insert("Add Item: (none)".into(), _none.rect);
            }
            for kind in recent {
                self.add_item_entry(ui, kind.label(), Some(kind));
            }
        });
        #[cfg(test)]
        self.toolbox_rects
            .insert("Add Item menu: Recent".into(), _recent.response.rect);
        for entry in &TOP {
            self.add_item_entries(ui, entry);
        }
        ui.separator();
        for (name, entries) in CATEGORIES {
            // IL-Q3 (provisional): a category with nothing Operandi can make
            // is hidden unless Show unsupported is on.
            if show_unsupported || any_enabled(entries) {
                let _menu = ui.menu_button(name, |ui| {
                    for entry in entries {
                        self.add_item_entries(ui, entry);
                    }
                });
                #[cfg(test)]
                self.toolbox_rects
                    .insert(format!("Add Item menu: {name}"), _menu.response.rect);
            }
        }
        ui.separator();
        let mut show = show_unsupported;
        if ui
            .checkbox(&mut show, "Show unsupported")
            .on_hover_text("Also list item types Operandi can't make yet")
            .changed()
        {
            self.item_list.prefs.show_unsupported = show;
            self.save_item_list_prefs();
        }
    }
    fn add_item_entries(&mut self, ui: &mut Ui, entry: &'static Entry) {
        match entry {
            Item(label, kind) => self.add_item_entry(ui, label, *kind),
            Sub(name, entries) => {
                ui.menu_button(*name, |ui| {
                    for entry in *entries {
                        self.add_item_entries(ui, entry);
                    }
                });
            }
        }
    }
    fn add_item_entry(&mut self, ui: &mut Ui, label: &str, kind: Option<AddItemKind>) {
        // Mesh stays available with a tool live, as New Mesh was (the
        // editor resolves a Smooth Shift stage first); the others wait.
        let why = match kind {
            None => Some(NOT_YET),
            Some(k) => crate::unavailable::unless(
                k == AddItemKind::Mesh || self.editor.live.is_none(),
                crate::unavailable::TOOL_ACTIVE,
            ),
        };
        let response = ui.gated(why, Button::new(label));
        #[cfg(test)]
        {
            self.toolbox_rects
                .insert(format!("Add Item: {label}"), response.rect);
            if kind == Some(AddItemKind::Mesh) {
                self.toolbox_rects
                    .insert("Scenes menu New Mesh".into(), response.rect);
            }
        }
        if response.clicked()
            && let Some(kind) = kind
        {
            self.add_item(kind);
            ui.close();
        }
    }
    /// IL-03: make one item of `kind`; on success it goes to the top of
    /// Recent (IL-04).
    pub(super) fn add_item(&mut self, kind: AddItemKind) {
        if self.run_ok(Command::AddItem(kind)) {
            if kind == AddItemKind::Environment {
                self.reveal_new_environment();
            }
            self.status = match kind {
                AddItemKind::Backdrop => "New Backdrop: choose its Image in Properties".into(),
                _ => format!("New {}", kind.label()),
            };
            self.item_list.find.clear();
            self.item_list.prefs.push_recent(kind);
            self.save_item_list_prefs();
        }
    }
    fn save_item_list_prefs(&mut self) {
        if let Err(error) = self
            .item_list
            .prefs
            .save(&self.environment.paths().item_list())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }

    /// IL-05 "Item Patterns".
    fn item_patterns_popover(&mut self, ui: &mut Ui) {
        self.menu_owned_input = true;
        ui.set_min_width(200.);
        ui.strong("Item Patterns");
        let query = &mut self.item_list.pattern;
        let pattern = ui
            .horizontal(|ui| {
                ui.label("Pattern");
                ui.add(
                    TextEdit::singleline(&mut query.pattern)
                        .id_salt("item-pattern")
                        .desired_width(120.),
                )
            })
            .inner
            .on_hover_text("* matches any run of characters, ? one character; case doesn't matter");
        self.text_editors.insert(pattern.id);
        crate::numeric_tab::register(ui, &pattern, Some(&query.pattern));
        tick_buttons(ui, &mut query.types);
        ui.checkbox(
            &mut query.limit_to_children,
            "Limit to current selection child items",
        );
        type_checkboxes(ui, &mut query.types);
        crate::editor_color_ui::by_color_rows(
            ui,
            "pattern-color",
            &mut query.by_color,
            &mut query.color,
        );
        egui::CollapsingHeader::new("Action")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for action in [PatternAction::Select, PatternAction::Unselect] {
                        self.pattern_action_button(ui, action);
                    }
                    ui.gated(
                        Some("Not built yet: use the Groups tab's New Group after Select"),
                        Button::new("Group"),
                    );
                    self.pattern_action_button(ui, PatternAction::Hide);
                });
            });
    }
    fn pattern_action_button(&mut self, ui: &mut Ui, action: PatternAction) {
        let response = ui.gated(
            crate::unavailable::unless(self.editor.live.is_none(), crate::unavailable::TOOL_ACTIVE),
            Button::new(action.label()),
        );
        #[cfg(test)]
        self.toolbox_rects
            .insert(format!("Pattern {}", action.label()), response.rect);
        if response.clicked() {
            let query = self.item_list.pattern.clone();
            self.run(Command::ItemPattern { query, action });
        }
    }

    /// IL-06 "Item List Filters": view only.
    fn item_filters_popover(&mut self, ui: &mut Ui) {
        self.menu_owned_input = true;
        ui.set_min_width(180.);
        ui.strong("Item List Filters");
        let filter = &mut self.item_list.filter;
        ui.horizontal(|ui| {
            for (on, label) in [
                (&mut filter.flatten, "Flatten"),
                (&mut filter.sort_by_name, "Sort by Name"),
                (&mut filter.sort_by_type, "Sort by Type"),
            ] {
                if ui.selectable_label(*on, label).clicked() {
                    *on ^= true;
                }
            }
        });
        tick_buttons(ui, &mut filter.types);
        type_checkboxes(ui, &mut filter.types);
        ui.gated(
            Some("Not built yet in the Items list"),
            egui::Checkbox::new(&mut false, "Has Notes"),
        );
        crate::editor_color_ui::by_color_rows(
            ui,
            "filter-color",
            &mut filter.by_color,
            &mut filter.color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_lists_the_spec_order_and_only_supported_entries_are_enabled() {
        let tops: Vec<_> = TOP
            .iter()
            .map(|e| match e {
                Item(l, _) => *l,
                Sub(l, _) => *l,
            })
            .collect();
        assert_eq!(tops, ["Backdrop Item", "Camera", "Mesh"]);
        let names: Vec<_> = CATEGORIES.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            [
                "Deformers",
                "Dynamics",
                "Falloffs",
                "Forces",
                "Lights",
                "Locators",
                "Particles",
                "Procedural",
                "Volumes",
                "Other"
            ]
        );
        // Entries in each submenu are alphabetical (IL-02).
        // (Submenus follow a category's own entries.)
        fn labels(entries: &[Entry]) -> Vec<String> {
            entries
                .iter()
                .filter_map(|e| match e {
                    Item(l, _) => Some(l.to_lowercase()),
                    Sub(..) => None,
                })
                .collect()
        }
        for (name, entries) in CATEGORIES {
            let got = labels(entries);
            let mut sorted = got.clone();
            sorted.sort();
            assert_eq!(got, sorted, "{name}");
        }
        // Today only Lights and Locators (Group Locator, GL-02) have
        // entries Operandi can make.
        let shown: Vec<_> = CATEGORIES
            .iter()
            .filter(|(_, e)| any_enabled(e))
            .map(|(n, _)| *n)
            .collect();
        assert_eq!(shown, ["Lights", "Locators"]);
        let made: Vec<_> = find_entries("", true)
            .into_iter()
            .filter_map(|(_, k)| k)
            .collect();
        // An Environment is made from Item > Create Environment and the
        // Shader Tree (ENV §6), not from the Add Item list (IL-02 has none).
        let listed: Vec<_> = AddItemKind::ALL
            .into_iter()
            .filter(|&k| k != AddItemKind::Environment)
            .collect();
        assert_eq!(made.len(), listed.len());
        for kind in listed {
            assert!(made.contains(&kind), "{kind:?} is reachable");
        }
    }

    #[test]
    fn find_filters_entries_and_hides_unsupported_categories() {
        let spot: Vec<_> = find_entries("SPOT", false);
        assert_eq!(spot, [("Spot Light", light(LightKind::Spot))]);
        // Unsupported entries in a shown category stay (disabled).
        assert_eq!(
            find_entries("dome", false),
            [("Dome Light", light(LightKind::Dome))]
        );
        // A hidden category's entries need Show unsupported.
        assert!(find_entries("vortex", false).is_empty());
        assert_eq!(find_entries("vortex", true), [("Vortex Force", None)]);
    }

    #[test]
    fn preferences_keep_recent_across_restarts() {
        let dir = std::env::temp_dir().join(format!("operandi-item-list-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("item-list.json");
        assert_eq!(ItemListPreferences::load(&path), Default::default());
        let mut prefs = ItemListPreferences::default();
        prefs.push_recent(AddItemKind::Camera);
        prefs.push_recent(AddItemKind::Light(LightKind::Spot));
        prefs.push_recent(AddItemKind::Camera);
        prefs.show_unsupported = true;
        prefs.save(&path).unwrap();
        let back = ItemListPreferences::load(&path);
        assert_eq!(back, prefs);
        assert_eq!(
            back.recent_kinds(),
            [AddItemKind::Camera, AddItemKind::Light(LightKind::Spot)]
        );
        // Unknown keys (a type from a newer build) are skipped.
        let odd = ItemListPreferences {
            recent: vec!["locator".into(), "mesh".into()],
            show_unsupported: false,
        };
        assert_eq!(odd.recent_kinds(), [AddItemKind::Mesh]);
        fs::remove_dir_all(&dir).ok();
    }
}
