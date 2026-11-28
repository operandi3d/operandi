//! Platform menu descriptions and routing. Native callbacks carry intent only;
//! editor state, confirmation, history and file protection stay in Operandi.
use super::*;
use crate::unavailable::Gate;
/// What a menu entry does. Every menu is a form (menus as forms), so an
/// entry runs a command line; About and Quit are the macOS app menu's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Quit (the app menu's Quit). Only the macOS native menu sends it.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Close,
    /// About Operandi (about.rs): the macOS app menu's entry, or Help >
    /// About Operandi in the in-window menus. Not in `menu_model`, since
    /// the macOS app menu is AppKit's own.
    About,
    /// An entry of a menu drawn from a form (UIC-10 / UIC-210): the command
    /// line it runs, by its place in [`line_action`]'s table.
    Line(u32),
}

/// Command lines menu entries run, each kept once so an [`Action::Line`]
/// stays the same line for as long as the app runs (the native menu
/// holds actions between frames). The map finds a line's place quickly:
/// the bar asks for every entry's action every frame.
static LINES: std::sync::Mutex<Lines> = std::sync::Mutex::new(Lines {
    list: Vec::new(),
    index: None,
});

struct Lines {
    list: Vec<String>,
    index: Option<std::collections::HashMap<String, u32>>,
}

/// The action running `line` from a menu.
pub(crate) fn line_action(line: &str) -> Action {
    let mut lines = LINES.lock().unwrap_or_else(|e| e.into_inner());
    let Lines { list, index } = &mut *lines;
    let index = index.get_or_insert_with(Default::default);
    if let Some(at) = index.get(line) {
        return Action::Line(*at);
    }
    let at = list.len() as u32;
    list.push(line.to_owned());
    index.insert(line.to_owned(), at);
    Action::Line(at)
}

/// The line an [`Action::Line`] runs.
pub(crate) fn action_line(index: u32) -> Option<String> {
    let lines = LINES.lock().unwrap_or_else(|e| e.into_inner());
    lines.list.get(index as usize).cloned()
}
/// The line the macOS app menu's Preferences… entry runs (#67): the same
/// as Edit > Preferences… and the input map's Preferences action.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) const PREFERENCES_LINE: &str = "Preferences";
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub action: Action,
    pub title: String,
    pub enabled: bool,
    /// Why it is greyed (its tooltip); `None` when enabled.
    pub why: Option<String>,
    pub checked: bool,
    /// Only the existing fixed File shortcuts become native key equivalents
    /// (a form control's `keyEquivalent`). Remappable modeling chords stay
    /// in egui's focus-aware input map.
    pub command_key: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    Item(Item),
    Submenu(Menu),
    Separator,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    /// The form it is drawn from (`menu.file`); empty for a menu that
    /// isn't a form.
    pub id: String,
    /// Its title now (a form's label, maybe filled from a command's
    /// answer).
    pub title: String,
    pub entries: Vec<Entry>,
}
/// Two menu bars have the same shape: the same menus and submenus (by
/// title), separators and entries (by action), so the native menu can
/// update its items in place instead of rebuilding (native_menu.rs).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn same_shape(a: &[Menu], b: &[Menu]) -> bool {
    fn entries(a: &[Entry], b: &[Entry]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| match (x, y) {
                (Entry::Item(x), Entry::Item(y)) => x.action == y.action,
                (Entry::Separator, Entry::Separator) => true,
                (Entry::Submenu(x), Entry::Submenu(y)) => {
                    x.title == y.title && entries(&x.entries, &y.entries)
                }
                _ => false,
            })
    }
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.title == y.title && entries(&x.entries, &y.entries))
}
impl Menu {
    pub fn leaves(&self) -> Vec<&Item> {
        self.entries
            .iter()
            .flat_map(|entry| match entry {
                Entry::Item(item) => vec![item],
                Entry::Submenu(menu) => menu.leaves(),
                Entry::Separator => vec![],
            })
            .collect()
    }
}
impl Operandi {
    /// The menu bar: the form `menu.bar` and its menus (menu_forms.rs).
    pub(super) fn menu_model(&self, ctx: &Context) -> Vec<Menu> {
        // The in-window bar's own popup is open while it draws and runs its
        // entries; any other popup greys the menus.
        let ready = self.navigation.is_none()
            && !self.has_modal()
            && (self.in_window_bar || !egui::Popup::is_any_open(ctx));
        // Greyed entries say why in their tooltip (unavailable.rs).
        let item = |action: Action,
                    title: String,
                    why: Option<String>,
                    checked: bool,
                    command_key: String| {
            let why = if ready {
                why
            } else {
                Some("Finish the open dialog, menu or drag first".to_owned())
            };
            Entry::Item(Item {
                action,
                title,
                enabled: why.is_none(),
                why,
                checked,
                command_key,
            })
        };
        #[allow(unused_mut)]
        let mut menus = self.menus_from_forms(&item);
        // The animation switch (anim_switch.rs): no Animate menu while off.
        if !crate::anim_switch::enabled() {
            menus.retain(|m| m.id != "menu.animate");
        }
        // The scripting switch (python_switch.rs): no script entries while off.
        if !crate::python_switch::enabled() {
            for m in &mut menus {
                m.entries.retain(|e| {
                    !matches!(e, Entry::Item(Item { action: Action::Line(n), .. })
                        if action_line(*n).is_some_and(|l| l.starts_with("operandi.python.")))
                });
            }
        }
        #[cfg(feature = "painting")]
        self.painting_menus(&mut menus, &item);
        menus
    }
    /// The in-window menu bar (Windows, Linux): every menu of
    /// `menu_model`, so it shows what the macOS menu bar shows (menus as
    /// forms). About Operandi heads Help, as macOS has it in the app menu.
    /// Whether File…Help (and Save/Undo/Redo beside them) live in the system
    /// menu bar, so the window's top row leaves them out: the installed Mac
    /// menu, or the offscreen QA harness standing in for it.
    pub(super) fn menus_in_system_bar(&self) -> bool {
        self.menus_in_system_bar || native_menu::installed()
    }
    pub(super) fn in_window_menu_bar(&mut self, ui: &mut Ui, ctx: &Context) {
        self.in_window_bar = true;
        let model = self.menu_model(ctx);
        for menu in model {
            ui.menu_button(menu.title.as_str(), |ui| {
                self.menu_owned_input = true;
                if menu.id == "menu.help" {
                    if ui.button("About Operandi").clicked() {
                        self.menu_action(ctx, Action::About);
                        ui.close();
                    }
                    ui.separator();
                }
                self.in_window_entries(ui, ctx, &menu.entries);
            });
        }
        self.in_window_bar = false;
    }
    /// Draws a menu's entries in the in-window bar; a click runs the entry
    /// through `menu_action`, as the native menu does.
    fn in_window_entries(&mut self, ui: &mut Ui, ctx: &Context, entries: &[Entry]) {
        for entry in entries {
            match entry {
                Entry::Item(item) => {
                    let label = if item.checked {
                        format!("✔ {}", item.title)
                    } else {
                        item.title.clone()
                    };
                    if ui.gated(item.why.as_deref(), Button::new(label)).clicked() {
                        self.menu_action(ctx, item.action);
                        ui.close();
                    }
                }
                Entry::Separator => {
                    ui.separator();
                }
                Entry::Submenu(menu) => {
                    ui.menu_button(menu.title.as_str(), |ui| {
                        self.in_window_entries(ui, ctx, &menu.entries)
                    });
                }
            }
        }
    }
    /// Runs a menu entry; the lines it runs are recorded as from a menu
    /// (UIC-224).
    pub(super) fn menu_action(&mut self, ctx: &Context, action: Action) {
        // Commands that need the window read it here (menus as forms).
        self.frame_ctx = Some(ctx.clone());
        let source = std::mem::replace(&mut self.history.ui_source, operandi_core::Source::Menu);
        self.menu_action_inner(ctx, action);
        self.history.ui_source = source;
    }
    fn menu_action_inner(&mut self, ctx: &Context, action: Action) {
        // What's This? on: the entry's page instead (help_mode.rs).
        if self.help_mode_takes_menu(&action) {
            return;
        }
        // About only shows information, so it is always allowed.
        if action == Action::About {
            self.show_about = true;
            ctx.request_repaint();
            return;
        }
        // Revalidate at dispatch; AppKit's displayed state may be one frame old.
        if !self
            .menu_model(ctx)
            .iter()
            .flat_map(Menu::leaves)
            .any(|item| item.action == action && item.enabled)
        {
            return;
        }
        match action {
            Action::Close => self.request_file(FileAction::Close),
            Action::About => self.show_about = true,
            // UIC-220: every menu entry runs its registry command.
            Action::Line(index) => {
                if let Some(line) = action_line(index) {
                    self.execute(&line);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Select's mode entry's line.
    fn mode_line(mode: Mode) -> Action {
        line_action(match mode {
            Mode::Vertex => "Vertices",
            Mode::Edge => "Edges",
            Mode::Polygon => "Polygons",
            Mode::Item => "Items",
        })
    }
    fn find(a: &Operandi, ctx: &Context, action: Action) -> Item {
        a.menu_model(ctx)
            .iter()
            .flat_map(Menu::leaves)
            .find(|i| i.action == action)
            .unwrap()
            .clone()
    }
    /// #67: the app menu's Preferences… runs the Edit menu's line, which
    /// opens Preferences, and takes ⌘, natively only while the input map
    /// binds Preferences to Cmd+, (so a key press never runs it twice or
    /// shadows a remapped chord).
    #[cfg(target_os = "macos")]
    #[test]
    fn app_menu_preferences_opens_preferences_and_follows_its_binding() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        let action = line_action(PREFERENCES_LINE);
        assert!(find(&a, &ctx, action).enabled);
        a.menu_action(&ctx, action);
        assert!(a.show_preferences);
        let default = crate::input::default_bindings();
        assert_eq!(
            crate::input::preferences_key_equivalent(
                default.get(PREFERENCES_LINE).map(String::as_str)
            ),
            ","
        );
        assert_eq!(crate::input::preferences_key_equivalent(Some("Cmd+P")), "");
        assert_eq!(
            crate::input::preferences_key_equivalent(Some("Shift+Cmd+,")),
            ""
        );
        assert_eq!(crate::input::preferences_key_equivalent(Some("")), "");
        assert_eq!(crate::input::preferences_key_equivalent(None), "");
    }
    #[test]
    fn native_model_keeps_full_menu_inventory_and_focus_aware_shortcuts() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        assert_eq!(
            a.menu_model(&ctx)
                .iter()
                .map(|m| m.title.as_str())
                .collect::<Vec<_>>(),
            [
                "File", "Edit", "Select", "Item", "Geometry", "Texture", "Tools", "View", "Layout",
                "Animate", "Render", "Window", "System", "Help"
            ]
        );
        assert_eq!(find(&a, &ctx, line_action("scene.save")).command_key, "s");
        assert_eq!(find(&a, &ctx, line_action("scene.close")).command_key, "w");
        assert_eq!(
            find(&a, &ctx, line_action("scene.closeAll")).command_key,
            "W"
        );
        a.bindings.insert("Move".into(), "Ctrl+M".into());
        let item = find(&a, &ctx, line_action(Tool::Move.label()));
        assert!(item.title.contains("Ctrl+M"));
        assert!(item.command_key.is_empty());
        for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon, Mode::Item] {
            assert_eq!(
                find(&a, &ctx, mode_line(mode)).checked,
                a.editor.scene.selection.mode == mode
            );
        }
        for action in [
            line_action("scene.new"),
            line_action("scene.open"),
            line_action("scene.saveAs"),
            line_action("app.undo"),
            line_action("app.redo"),
            line_action("operandi.inputMap"),
            line_action("operandi.window.diagnostics"),
            line_action("operandi.window.size size:compact"),
            line_action("operandi.window.size size:standard"),
            line_action("operandi.lasso.partial"),
        ] {
            let _ = find(&a, &ctx, action);
        }
    }
    #[test]
    fn native_routing_uses_live_history_and_staged_file_guards() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        a.run(Command::NewMesh);
        let base = a.editor.scene.clone();
        a.menu_action(&ctx, line_action(Tool::Box.label()));
        a.run(Command::Preview(Parameters {
            size: DVec3::splat(2.),
            ..Default::default()
        }));
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 6);
        let result = a.editor.scene.clone();
        a.fields.insert("size0".into(), "bad".into());
        a.menu_action(&ctx, line_action("scene.saveAs"));
        assert!(a.file_dialog.is_none());
        assert_eq!(a.editor.scene, result);
        a.fields.insert("size0".into(), "2 m".into());
        assert!(a.confirm_fields());
        // Mid-tool undo (Jason, 2026-11-04): Undo takes back the cube's
        // last tweak and Cube stays active; Redo puts it back.
        assert!(find(&a, &ctx, line_action("app.undo")).enabled);
        assert!(!find(&a, &ctx, line_action("app.redo")).enabled);
        // Changing mode drops the open tool first, so it stays offered.
        assert!(find(&a, &ctx, mode_line(Mode::Edge)).enabled);
        let shown = a.editor.scene.clone();
        a.menu_action(&ctx, line_action("app.undo"));
        assert_eq!(a.editor.scene, base);
        assert_eq!(
            a.editor.session.live.as_ref().map(|l| l.tool),
            Some(Tool::Box)
        );
        assert_eq!(a.status, "Undo · Cube is still active");
        assert!(find(&a, &ctx, line_action("app.redo")).enabled);
        a.menu_action(&ctx, line_action("app.redo"));
        assert_eq!(
            a.editor.session.live.as_ref().map(|l| l.tool),
            Some(Tool::Box)
        );
        assert_eq!(a.editor.scene, shown);
        a.run(Command::Preview(Parameters {
            size: DVec3::splat(2.),
            ..Default::default()
        }));
        assert_eq!(a.editor.scene, result);
        a.run(Command::Drop);
        assert!(
            find(&a, &ctx, line_action("app.undo"))
                .title
                .contains("Cube"),
            "{}; live={:?}; undo={:?}",
            a.status,
            a.editor.session.live.as_ref().map(|l| l.tool),
            a.editor
                .history
                .undo
                .iter()
                .map(|c| &c.label)
                .collect::<Vec<_>>()
        );
        a.menu_action(&ctx, line_action("app.undo"));
        assert_eq!(a.editor.scene, base);
        a.menu_action(&ctx, line_action("app.redo"));
        assert_eq!(a.editor.scene, result);
        // New opens a scene beside this one (scenes.rs); a modal blocks
        // the menu's tools.
        a.menu_action(&ctx, line_action("scene.new"));
        assert!(a.pending.is_none());
        assert_eq!(a.scenes.tabs.len(), 2);
        a.menu_action(&ctx, line_action("operandi.scene.current index:0"));
        assert_eq!(a.editor.scene, result);
        a.pending = Some(FileAction::CloseScene);
        let before = a.editor.clone();
        a.menu_action(&ctx, line_action(Tool::Sphere.label()));
        assert!(a.editor == before);
        a.pending = None;
        a.menu_action(&ctx, line_action("scene.saveAs"));
        assert!(a.file_dialog.is_some());
        assert_eq!(a.editor.scene, result);
    }
    #[test]
    fn native_selection_lasso_and_modal_revalidation() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        a.run(Command::NewMesh);
        a.menu_action(&ctx, mode_line(Mode::Edge));
        assert_eq!(a.editor.scene.selection.mode, Mode::Edge);
        a.menu_action(&ctx, line_action("operandi.lasso.style style:rectangle"));
        assert!(
            find(
                &a,
                &ctx,
                line_action("operandi.lasso.style style:rectangle")
            )
            .checked
        );
        a.menu_action(&ctx, line_action("operandi.lasso.partial"));
        assert!(a.lasso_partial);
        let mode = a.editor.scene.selection.mode;
        a.menu_action(&ctx, line_action("operandi.inputMap"));
        assert!(a.show_bindings);
        assert!(
            a.menu_model(&ctx)
                .iter()
                .flat_map(Menu::leaves)
                .all(|i| !i.enabled)
        );
        a.menu_action(&ctx, mode_line(Mode::Polygon));
        assert_eq!(a.editor.scene.selection.mode, mode);
    }
    fn history_app() -> Operandi {
        let mut app = Operandi::empty();
        app.editor.scene = serde_json::from_str(include_str!(
            "../../../../docs/i2/expectations/graphs/G0.json"
        ))
        .unwrap();
        app.editor.synchronize_identities().unwrap();
        app
    }
    fn agrees(app: &Operandi, ctx: &Context) {
        let before = app.editor.clone();
        let marks = (
            app.editor.identities.next_id().unwrap(),
            app.editor.i2.serial.next().unwrap(),
        );
        for (action, direction, prefix) in [
            (line_action("app.undo"), HistoryAction::Undo, "Undo"),
            (line_action("app.redo"), HistoryAction::Redo, "Redo"),
        ] {
            let q = app.editor.history_eligibility(direction);
            let item = find(app, ctx, action);
            assert_eq!(item.enabled, q.is_available());
            let title = q
                .label
                .map(|label| format!("{prefix} {label}"))
                .unwrap_or_else(|| prefix.into());
            assert_eq!(item.title, format!("{title}{}", app.shortcut(prefix)));
        }
        assert!(app.editor == before);
        assert_eq!(
            marks,
            (
                app.editor.identities.next_id().unwrap(),
                app.editor.i2.serial.next().unwrap()
            )
        );
    }
    #[test]
    fn stab01_menu_follows_core_for_configured_categories_and_ordinary_routes() {
        let ctx = Context::default();
        let mut app = history_app();
        agrees(&app, &ctx);
        for tool in [Tool::Bevel, Tool::Extrude, Tool::SmoothShift, Tool::Thicken] {
            let mut app = history_app();
            app.editor
                .dispatch(Command::Rename("renamed".into()))
                .unwrap();
            app.editor.dispatch(Command::Activate(tool)).unwrap();
            agrees(&app, &ctx);
            assert!(find(&app, &ctx, line_action("app.undo")).enabled);
            assert!(!find(&app, &ctx, line_action("app.redo")).enabled);
            // Undo closes the armed tool; the next one undoes the rename.
            app.menu_action(&ctx, line_action("app.undo"));
            assert!(app.editor.session.live.is_none());
            assert_eq!(app.editor.scene.active().unwrap().name, "renamed");
            app.menu_action(&ctx, line_action("app.undo"));
            assert_eq!(app.editor.scene, history_app().editor.scene);
            agrees(&app, &ctx);
            app.execute("redo");
            assert_eq!(app.editor.scene.active().unwrap().name, "renamed");
        }
        app.editor.dispatch(Command::ArmEllipsoid).unwrap();
        agrees(&app, &ctx);
        // Undo would close Ellipsoid (MU-MID-02).
        assert!(find(&app, &ctx, line_action("app.undo")).enabled);
        app.editor.dispatch(Command::Cancel).unwrap();
        app.editor.dispatch(Command::ArmCone).unwrap();
        agrees(&app, &ctx);
        app.editor
            .dispatch(Command::ConfigureCone(ConeParameters {
                sides: 8,
                segments: 3,
                ..Default::default()
            }))
            .unwrap();
        app.editor.dispatch(Command::Apply).unwrap();
        agrees(&app, &ctx);
        assert!(find(&app, &ctx, line_action("app.undo")).enabled);
        let result = app.editor.scene.clone();
        app.menu_action(&ctx, line_action("app.undo"));
        agrees(&app, &ctx);
        assert!(find(&app, &ctx, line_action("app.redo")).enabled);
        app.menu_action(&ctx, line_action("app.redo"));
        assert_eq!(app.editor.scene, result);
    }
    #[test]
    fn stab01_local_labels_shortcuts_and_stale_menu_revalidation() {
        let ctx = Context::default();
        let mut app = history_app();
        app.bindings.insert("Undo".into(), "Ctrl+U".into());
        app.bindings.insert("Redo".into(), "Ctrl+R".into());
        app.editor
            .dispatch(Command::Activate(Tool::Extrude))
            .unwrap();
        app.editor
            .dispatch(Command::ConfigureExtrude(Parameters {
                extrude_mode: ExtrudeMode::Shift,
                extrude_uvs: ExtrudeUvs::None,
                ..Default::default()
            }))
            .unwrap();
        app.editor.dispatch(Command::BeginExtrude).unwrap();
        agrees(&app, &ctx);
        assert!(
            find(&app, &ctx, line_action("app.undo"))
                .title
                .contains("Undo Begin Extrude")
        );
        let mut p = app.editor.session.live.as_ref().unwrap().params.clone();
        p.extent = DVec3::Z * 0.5;
        app.editor.dispatch(Command::Preview(p)).unwrap();
        agrees(&app, &ctx);
        assert!(
            find(&app, &ctx, line_action("app.undo"))
                .title
                .contains("Undo Move Extrude")
        );
        app.menu_action(&ctx, line_action("app.undo"));
        app.menu_action(&ctx, line_action("app.undo"));
        agrees(&app, &ctx);
        let displayed = find(&app, &ctx, line_action("app.redo"));
        assert!(displayed.enabled && displayed.title.contains("Redo Begin Extrude"));
        app.editor
            .dispatch(Command::ActionContext(ActionContext {
                center: CenterMode::Origin,
                ..Default::default()
            }))
            .unwrap();
        agrees(&app, &ctx);
        assert!(!find(&app, &ctx, line_action("app.redo")).enabled);
        let before = app.editor.clone();
        app.menu_action(&ctx, line_action("app.redo"));
        assert!(app.editor == before);
        app.editor
            .dispatch(Command::ActionContext(Default::default()))
            .unwrap();
        app.editor.dispatch(Command::Activate(Tool::Move)).unwrap();
        agrees(&app, &ctx);
        app.menu_action(&ctx, line_action("app.redo"));
        assert_eq!(
            app.editor.session.live.as_ref().unwrap().tool,
            Tool::Extrude
        );
        assert_eq!(app.editor.i2.selected.as_ref().unwrap().cursor, 1);
    }
    #[test]
    fn mode_availability_native_model_characterization() {
        for tool in ["cone", "ellipsoid", "spike", "smooth shift", "thicken"] {
            let mut a = Operandi::empty();
            a.editor.scene = serde_json::from_str(include_str!(
                "../../../../docs/i2/expectations/graphs/G0.json"
            ))
            .unwrap();
            a.execute(tool);
            let ctx = Context::default();
            let entry = find(&a, &ctx, mode_line(Mode::Edge));
            println!("Mode native model {tool}: enabled={}", entry.enabled);
            // Every open tool is dropped first, so mode is always offered.
            assert!(entry.enabled);
        }
    }
    #[test]
    fn mode_availability_run_confirmation_still_precedes_refused_mode() {
        let mut a = Operandi::empty();
        a.editor.scene = serde_json::from_str(include_str!(
            "../../../../docs/i2/expectations/graphs/G0.json"
        ))
        .unwrap();
        a.activate(Tool::Move);
        a.fields.insert("delta0".into(), "abc".into());
        let before = a.editor.clone();
        a.run(Command::Mode(Mode::Edge));
        assert!(a.editor == before);
        assert!(a.field_errors.contains_key("delta0"));
        a.fields.insert("delta0".into(), "0.125 m".into());
        a.run(Command::Mode(Mode::Edge));
        assert_ne!(a.editor.scene, before.scene);
        // The confirmed Move is dropped as its own step, then Edge mode.
        assert!(a.editor.session.live.is_none());
        assert_eq!(a.editor.scene.selection.mode, Mode::Edge);
        assert_eq!(a.editor.history.undo.len(), before.history.undo.len() + 1);
    }
    #[test]
    fn mode_availability_native_readiness_checks_and_remapping() {
        let mut a = Operandi::empty();
        a.editor.scene = serde_json::from_str(include_str!(
            "../../../../docs/i2/expectations/graphs/G0.json"
        ))
        .unwrap();
        a.bindings.insert("Edges".into(), "Ctrl+2".into());
        let ctx = Context::default();
        for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon, Mode::Item] {
            let item = find(&a, &ctx, mode_line(mode));
            assert!(item.enabled);
            assert_eq!(item.checked, mode == Mode::Polygon);
        }
        assert_eq!(
            find(&a, &ctx, mode_line(Mode::Edge)).title,
            "Edges  [Ctrl+2] "
        );
        a.show_bindings = true;
        assert!(!find(&a, &ctx, mode_line(Mode::Edge)).enabled);
        a.show_bindings = false;
        a.editor.dispatch(Command::ArmCone).unwrap();
        assert!(find(&a, &ctx, mode_line(Mode::Edge)).enabled);
        // Selection-operation eligibility is explicitly outside this slice.
        assert!(find(&a, &ctx, line_action("Grow")).enabled);
    }

    /// Edit ▸ Center Selected (center-selected-2026-11-11 §3, §7): seven
    /// items, All first; each runs `vert.center` on its axes only, one undo
    /// step; greyed in Item mode, where Center refuses (§5).
    #[test]
    fn center_selected_menu_runs_vert_center_on_its_axes() {
        let ctx = Context::default();
        let entries = [
            ("All", "vert.center all", [true, true, true]),
            ("X", "vert.center x", [true, false, false]),
            ("Y", "vert.center y", [false, true, false]),
            ("Z", "vert.center z", [false, false, true]),
            ("XY", "vert.center xy", [true, true, false]),
            ("YZ", "vert.center yz", [false, true, true]),
            ("ZX", "vert.center zx", [true, false, true]),
        ];
        let app = || {
            let mut a = Operandi::empty();
            a.run(Command::NewMesh);
            a.editor.scene.active_mut().unwrap().mesh =
                operandi_core::Mesh::box_mesh(DVec3::new(2., 3., 4.), DVec3::ONE, 0)
                    .unwrap()
                    .into();
            a.editor.synchronize_identities().unwrap();
            a.run(Command::Mode(Mode::Polygon));
            a.editor.history.undo.clear();
            a
        };
        let a = app();
        let edit = a
            .menu_model(&ctx)
            .into_iter()
            .find(|m| m.title == "Edit")
            .unwrap();
        let center = edit
            .entries
            .iter()
            .find_map(|e| match e {
                Entry::Submenu(m) if m.title == "Center Selected" => Some(m.clone()),
                _ => None,
            })
            .expect("Edit ▸ Center Selected");
        let labels: Vec<&str> = center.leaves().iter().map(|i| i.title.as_str()).collect();
        assert_eq!(labels, ["All", "X", "Y", "Z", "XY", "YZ", "ZX"]);
        for (label, line, axes) in entries {
            let mut a = app();
            let item = find(&a, &ctx, line_action(line));
            assert!(item.enabled, "{label}");
            a.menu_action(&ctx, line_action(line));
            let mesh = &a.editor.scene.active().unwrap().mesh;
            let (lo, hi) = mesh.vertices.iter().fold(
                (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
                |(lo, hi), p| (lo.min(*p), hi.max(*p)),
            );
            let mid = (lo + hi) / 2.;
            for (i, before) in [2., 3., 4.].into_iter().enumerate() {
                let want = if axes[i] { 0. } else { before };
                assert!(
                    (mid[i] - want).abs() < 1e-9,
                    "{label}: axis {i} at {}",
                    mid[i]
                );
            }
            assert_eq!(a.editor.history.undo.len(), 1, "{label}: one undo step");
        }
        // Item mode: greyed with Center's reason, and running it changes nothing.
        let mut a = app();
        a.run(Command::Mode(Mode::Item));
        let item = find(&a, &ctx, line_action("vert.center all"));
        assert!(!item.enabled);
        assert!(item.why.as_deref().unwrap_or("").contains("component mode"));
        let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
        a.menu_action(&ctx, line_action("vert.center all"));
        assert_eq!(a.editor.scene.active().unwrap().mesh.vertices, before);
    }

    /// Spec freeze-transforms Q10: Item > Transform lists Reset and Freeze
    /// (Position, Rotation, Scale, All each, no Shear) and runs them.
    #[test]
    fn item_transform_menu_resets_and_freezes() {
        let ctx = Context::default();
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
                .unwrap()
                .into();
        a.editor.scene.active_mut().unwrap().transform.scale = DVec3::new(2., 1., 1.);
        a.editor.synchronize_identities().unwrap();
        a.run(Command::Mode(Mode::Item));
        a.editor.history.undo.clear();
        let menu = a
            .menu_model(&ctx)
            .into_iter()
            .find(|m| m.title == "Item")
            .unwrap();
        let transform = menu
            .entries
            .iter()
            .find_map(|e| match e {
                Entry::Submenu(m) if m.title == "Transform" => Some(m.clone()),
                _ => None,
            })
            .expect("Item > Transform");
        let labels: Vec<&str> = transform
            .leaves()
            .iter()
            .map(|i| i.title.as_str())
            .collect();
        assert_eq!(
            labels,
            [
                "Reset Position",
                "Reset Rotation",
                "Reset Scale",
                "Reset All",
                "Freeze Position",
                "Freeze Rotation",
                "Freeze Scale",
                "Freeze All",
            ]
        );
        assert!(transform.leaves().iter().all(|i| i.enabled));
        let x = |a: &Operandi| {
            let item = a.editor.scene.active().unwrap();
            let w = a.editor.scene.world_matrix(item.id).unwrap();
            item.mesh
                .vertices
                .iter()
                .map(|p| w.transform_point3(*p).x)
                .fold(f64::MIN, f64::max)
        };
        let before = x(&a);
        a.menu_action(&ctx, line_action("transform.freeze scale"));
        assert_eq!(a.editor.history.undo.last().unwrap().label, "Freeze Scale");
        assert_eq!(a.editor.scene.active().unwrap().transform.scale, DVec3::ONE);
        assert!((x(&a) - before).abs() < 1e-12, "nothing moves");
        a.menu_action(&ctx, line_action("transform.reset all"));
        // Already identity after the freeze: no step.
        assert_eq!(a.editor.history.undo.len(), 1);
    }
}
