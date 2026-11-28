//! The Schematic window (RG-70..72, BEHAVIOR-SCHEMATIC, F12): one
//! workspace page at a time (SC-01, SC-75), its nodes at their saved
//! places (SC-91), wires between them and marked stubs for wires leaving
//! the page (SC-14). Pages, places, wiring, unwiring, adding modifiers
//! and deleting pieces each run one `operandi.rig.*` line (UIC-160), so
//! undo, scripts and the bridge agree. Pan and zoom are the view's own,
//! per page, for the session (SC-68). A floating window (UIC-183),
//! hidden with the animation switch.
use super::*;
use crate::command_handlers::Commands;
use operandi_core::command_registry::{self as table, Area};
use operandi_core::rig_schematic::{self, Schematic, WORLD};
use operandi_core::schematic_layout::{ADD, Metrics};
use operandi_core::schematic_pages::{self, PageId, PageView};
use std::collections::BTreeMap;

/// Zoom limits (SC-65).
const ZOOM: (f32, f32) = (0.1, 3.);

#[derive(Default)]
pub(crate) struct SchematicView {
    pub(crate) open: bool,
    /// The page shown; `None` until one is picked (then the first, SC-75).
    pub(crate) page: Option<PageId>,
    /// Pan (points) and zoom per page, for the session (SC-68).
    views: BTreeMap<PageId, (Vec2, f32)>,
    /// The shown page and the scene state it was worked out for (a hash).
    cache: Option<(u64, PageId, PageView)>,
    /// A wire being drawn: from node key and output port.
    drawing: Option<(String, String)>,
    /// A node being dragged: its key, the grab offset and where it is now
    /// (page units); the place is saved when the drag ends.
    moving: Option<(String, Vec2, Pos2)>,
    /// The node last clicked (Delete takes it off the page, SC-41).
    selected_node: Option<String>,
    selected_wire: Option<(u64, String)>,
    /// A drop on an "add channel" port waits for a channel pick: the
    /// line to finish, with `{}` for the channel.
    pick: Option<(u64, String, Pos2)>,
    /// What the right-click menu is for (a node key, or "" for the page).
    menu: Option<(String, Pos2)>,
    /// A workspace name being typed: (renaming which, the text).
    naming: Option<(Option<u64>, String)>,
    /// Lines that need the workspace a page save just made: `{ws}` in
    /// them is replaced by its id.
    after_save: Vec<String>,
    /// Fit on the next frame: all nodes, or only the selected (SC-66).
    pub(crate) fit: Option<bool>,
    /// View navigation in progress: what the press started (from the
    /// user's navigation preset, as in the 3D views) and where it began.
    nav: Option<(crate::input::NavigationKind, Pos2)>,
    /// The middle of the view last frame, in page units: where added
    /// nodes land (SC-13).
    centre: [f32; 2],
    /// What the last Add did, shown on the page (Jason, 2026-11-25: the
    /// Add buttons gave no sign they had worked).
    pub(crate) note: Option<String>,
    /// A modifier just added from Add ▾: it is put on the page in the
    /// middle of the view once it exists.
    place_new_piece: bool,
}

/// `operandi.rig.schematicFit`'s argument.
const FIT_ARGS: &[table::Arg] = &[table::opt(
    "selected",
    table::ArgType::Bool,
    table::Unit::None,
    Some("false"),
)];

pub(crate) fn register(r: &mut Commands) {
    r.add_host_command(
        table::script(
            "operandi.rig.schematicView",
            Area::Items,
            "Shows or hides the Schematic (RG-70): workspace pages of rig pieces and items as nodes and wires.",
        )
        .operandi()
        .ui(),
        |a, _| {
            if !crate::anim_switch::enabled() {
                a.status = "Animation is off (OPERANDI_ANIMATION=0 in the environment)".into();
                return;
            }
            a.schematic.open = !a.schematic.open;
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    r.add_host_command(
        table::script(
            "operandi.rig.schematicFit",
            Area::Items,
            "Frames the Schematic page's nodes (SC-66): all of them, or with selected:true only the selected node.",
        )
        .args(FIT_ARGS)
        .operandi()
        .ui(),
        |a, args| {
            let rest = args.rest.to_lowercase();
            let selected = rest.contains("selected:true") || rest.contains("selected:1");
            a.schematic.fit = Some(selected);
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
}

impl SchematicView {
    fn view(&self, page: PageId) -> (Vec2, f32) {
        self.views.get(&page).copied().unwrap_or((Vec2::ZERO, 1.))
    }
}

/// The line that joins an output port to an input port, or why not.
pub(crate) fn connect_line(
    s: &Schematic,
    from: &str,
    from_port: &str,
    to: &str,
    to_port: &str,
) -> std::result::Result<String, String> {
    let node = |k: &str| s.nodes.iter().find(|n| n.key == k);
    let (Some(a), Some(b)) = (node(from), node(to)) else {
        return Err("That node is gone".into());
    };
    match (a.piece, b.piece) {
        // A channel into a modifier input (or a link's driver).
        (false, true) => {
            if from_port == WORLD || from_port == ADD {
                return Err("Pick one of the item's channels to wire".into());
            }
            if b.kind.as_deref().is_some_and(|k| k.starts_with("mod.")) {
                Ok(format!(
                    "operandi.rig.modifierSet piece:{} inputs:{to_port}={}:{from_port}",
                    b.id, a.id
                ))
            } else {
                Err("Wire constraints from the Rig menu (Animate > Constrain)".into())
            }
        }
        // A modifier's output into an item channel.
        (true, false) => {
            if !a.kind.as_deref().is_some_and(|k| k.starts_with("mod.")) {
                return Err("That piece's output is its item's world".into());
            }
            if to_port == WORLD {
                return Err("Pick a channel to drive".into());
            }
            Ok(format!(
                "operandi.rig.modifierSet piece:{} output:{}:{to_port}",
                a.id, b.id
            ))
        }
        // Channel to channel: a direct link (RG-50).
        (false, false) => {
            if [from_port, to_port].contains(&WORLD) {
                return Err("Link channels, not worlds".into());
            }
            Ok(format!(
                "operandi.rig.linkAdd driven:{}:{to_port} driver:{}:{from_port}",
                b.id, a.id
            ))
        }
        (true, true) => Err("Wire pieces through an item's channels".into()),
    }
}

/// `key@x/y` pairs for `schematicPlace` / `workspaceAdd`.
fn place_arg<'a>(places: impl IntoIterator<Item = (&'a String, &'a [f32; 2])>) -> String {
    let list: Vec<String> = places
        .into_iter()
        .map(|(k, p)| format!("{k}@{}/{}", p[0].round(), p[1].round()))
        .collect();
    format!("{{{}}}", list.join(","))
}

/// The name an unsaved page is saved under.
fn page_name(page: PageId) -> &'static str {
    match page {
        PageId::Unplaced => "Unplaced",
        _ => "Workspace",
    }
}

impl Operandi {
    pub(super) fn schematic_window(&mut self, ctx: &Context) {
        if !self.schematic.open || !crate::anim_switch::enabled() {
            return;
        }
        const PLACE: &str = "schematic";
        let place = self.palettes.places.get(PLACE).copied();
        let size = place.map_or([900., 520.], |p| p.size);
        let spec = crate::floating::OsWindow {
            id: egui::ViewportId::from_hash_of("operandi-schematic"),
            title: "Schematic".into(),
            size,
            position: place.and_then(|p| p.position),
            min_size: [420., 260.],
        };
        let shown = self.os_window(ctx, spec, |app, ctx| {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::side_top_panel(&ctx.style())
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(4.),
                )
                .show(ctx, |ui| app.schematic_ui(ui));
        });
        let mut place = place.unwrap_or(crate::palettes::Place {
            size,
            position: None,
        });
        if let Some(s) = shown.size
            && s.iter().all(|v| *v >= 100.)
        {
            place.size = s;
        }
        if let Some(at) = shown.position {
            place.position = Some(at);
        }
        self.palettes.places.insert(PLACE.into(), place);
        if !shown.open {
            self.schematic.open = false;
        }
    }

    /// What the shown page holds, worked out again only when the rig or
    /// the item names change.
    pub(crate) fn schematic_page(&mut self, page: PageId) -> PageView {
        use std::hash::{Hash, Hasher};
        let scene = &self.editor.scene;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(&scene.animation.rig)
            .unwrap_or_default()
            .hash(&mut h);
        for i in &scene.items {
            (i.id, &i.name).hash(&mut h);
        }
        let hash = h.finish();
        if let Some((k, p, view)) = &self.schematic.cache
            && *k == hash
            && *p == page
        {
            return view.clone();
        }
        let view = schematic_pages::page(scene, page, &Metrics::default());
        self.schematic.cache = Some((hash, page, view.clone()));
        view
    }

    /// The line that saves `places` on the page: a move on a saved page,
    /// or, on a page not saved yet, the whole page as a new workspace
    /// (the moved places included), so it is never re-arranged after.
    fn schematic_save_line(
        &mut self,
        page: PageId,
        view: &PageView,
        places: &[(String, [f32; 2])],
    ) -> String {
        match page {
            PageId::Saved(ws) => format!(
                "operandi.rig.schematicPlace workspace:{ws} place:{}",
                place_arg(places.iter().map(|(k, p)| (k, p)))
            ),
            _ => {
                let mut all = view.positions.clone();
                for (k, p) in places {
                    all.insert(k.clone(), *p);
                }
                format!(
                    "operandi.rig.workspaceAdd name:{{{}}} place:{}",
                    page_name(page),
                    place_arg(all.iter())
                )
            }
        }
    }

    /// Runs `line` on the page's workspace (`{ws}`), saving an unsaved
    /// page first.
    fn schematic_on_page(
        &mut self,
        page: PageId,
        view: &PageView,
        line: String,
        lines: &mut Vec<String>,
    ) {
        match page {
            PageId::Saved(ws) => lines.push(line.replace("{ws}", &ws.to_string())),
            _ => {
                lines.push(self.schematic_save_line(page, view, &[]));
                self.schematic.after_save.push(line);
            }
        }
    }

    fn schematic_ui(&mut self, ui: &mut Ui) {
        let mut lines: Vec<String> = Vec::new();
        let pages = schematic_pages::pages(&self.editor.scene);
        let page = self
            .schematic
            .page
            .filter(|p| pages.iter().any(|(q, _)| q == p))
            .or(pages.first().map(|p| p.0));
        self.schematic.page = page;
        let metrics = Metrics::default();
        let view = page.map(|p| self.schematic_page(p)).unwrap_or_default();
        let model = &view.schematic;

        // The top row: page selector, New Workspace…, Add, Add Selected,
        // Add Rig, Arrange, Fit (SC-02, SC-10, SC-12, SC-17, SC-60, SC-66).
        ui.horizontal(|ui| {
            let name = page
                .and_then(|p| pages.iter().find(|q| q.0 == p))
                .map_or("No workspace", |p| p.1.as_str());
            egui::ComboBox::from_id_salt("schematic-page")
                .selected_text(name)
                .show_ui(ui, |ui| {
                    for (id, label) in &pages {
                        if ui.selectable_label(page == Some(*id), label).clicked() {
                            self.schematic.page = Some(*id);
                        }
                    }
                })
                .response
                .on_hover_text("The workspace shown (SC-02): each page shows only the nodes put on it");
            if ui
                .button("New Workspace…")
                .on_hover_text("Add an empty workspace page, asking for its name\noperandi.rig.workspaceAdd")
                .clicked()
            {
                self.schematic.naming = Some((None, "Workspace".into()));
            }
            use crate::unavailable::{Gate, unless};
            let item = self.editor.scene.selection.item;
            let r = ui
                .gated(
                    unless(item.is_some(), "Select the item the modifier will belong to first"),
                    egui::Button::new("Add ▾"),
                )
                .on_hover_text("Add a modifier to the selected item and put it in the middle of this page; then wire it by dragging from port to port (RG-71)\noperandi.rig.modifierAdd");
            egui::Popup::menu(&r).show(|ui| {
                for kind in operandi_core::rig::MODIFIERS {
                    if ui.button(operandi_core::rig::label(kind)).clicked() {
                        if let Some(i) = item {
                            lines.push(format!(
                                "operandi.rig.modifierAdd kind:{} item:{i}",
                                &kind[4..]
                            ));
                            self.schematic.place_new_piece = true;
                        }
                        ui.close();
                    }
                }
            });
            let selected: Vec<String> = self
                .editor
                .scene
                .selected_items()
                .into_iter()
                .map(|id| format!("item:{id}"))
                .collect();
            let none_selected =
                unless(!selected.is_empty(), "Select items in a 3D view or the Items list first");
            if ui
                .gated(none_selected, egui::Button::new("Add Selected"))
                .on_hover_text("Put the selected items on this page, in the middle of the view (SC-10)\noperandi.rig.schematicPlace")
                .clicked()
            {
                self.schematic_add(page, &view, &selected, &mut lines);
            }
            if ui
                .gated(none_selected, egui::Button::new("Add Rig"))
                .on_hover_text("Put the selected items' rig pieces on this page with the items they use, arranged (SC-17)\noperandi.rig.schematicPlace")
                .clicked()
            {
                let full = rig_schematic::build(&self.editor.scene);
                let mut keys = selected.clone();
                for w in &full.wires {
                    let piece = if selected.contains(&w.from) {
                        &w.to
                    } else if selected.contains(&w.to) {
                        &w.from
                    } else {
                        continue;
                    };
                    for x in full.wires.iter().filter(|x| &x.from == piece || &x.to == piece) {
                        for k in [&x.from, &x.to] {
                            if !keys.contains(k) {
                                keys.push(k.clone());
                            }
                        }
                    }
                }
                self.schematic_add(page, &view, &keys, &mut lines);
            }
            if let Some(p) = page
                && ui
                    .button("Arrange")
                    .on_hover_text("Lay the page out by data flow; nodes you moved by hand stay put (SC-60)\noperandi.rig.schematicArrange")
                    .clicked()
            {
                self.schematic_on_page(p, &view, "operandi.rig.schematicArrange workspace:{ws}".into(), &mut lines);
            }
            if ui
                .button("Fit")
                .on_hover_text("Frame every node on the page; Shift: only the selected node (SC-66)\noperandi.rig.schematicFit")
                .clicked()
            {
                let only = ui.input(|i| i.modifiers.shift);
                lines.push(format!("operandi.rig.schematicFit selected:{only}"));
            }
            if let Some((piece, port)) = self.schematic.selected_wire.clone()
                && ui
                    .button("Delete Wire")
                    .on_hover_text("Delete the selected wire (Delete)\noperandi.rig.unwire")
                    .clicked()
            {
                lines.push(format!("operandi.rig.unwire piece:{piece} port:{{{port}}}"));
                self.schematic.selected_wire = None;
            }
        });

        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0., ui.visuals().extreme_bg_color);
        let Some(page) = page else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No rig yet. Select items and use Add Selected, or New Workspace…",
                egui::FontId::proportional(13.),
                ui.visuals().weak_text_color(),
            );
            self.schematic_popups(ui, &view, None, &mut lines);
            self.schematic_run(lines);
            return;
        };
        let (mut pan, mut zoom) = self.schematic.view(page);
        let origin = rect.min + vec2(20., 20.);

        // Fit (SC-66): frame all nodes, or the selected one.
        if let Some(only) = self.schematic.fit.take() {
            let keys: Vec<&String> = view
                .positions
                .keys()
                .filter(|k| !only || self.schematic.selected_node.as_ref() == Some(*k))
                .collect();
            let mut bounds = Rect::NOTHING;
            for k in keys {
                let p = view.positions[k];
                let h = model
                    .nodes
                    .iter()
                    .find(|n| &n.key == k)
                    .map_or(40., |n| metrics.height(n));
                bounds = bounds.union(Rect::from_min_size(
                    pos2(p[0], p[1]),
                    vec2(metrics.node_w, h),
                ));
            }
            if bounds.is_positive() {
                let room = rect.shrink(30.).size();
                zoom = (room.x / bounds.width())
                    .min(room.y / bounds.height())
                    .clamp(ZOOM.0, 1.5);
                pan = rect.center() - origin - bounds.center().to_vec2() * zoom;
            }
        }

        // Navigation as in the orthographic 3D views (SC-65; Jason,
        // 2026-11-25): the wheel and pinch zoom about the pointer with the
        // view's wheel settings; drags follow the navigation preset below.
        let pointer = response.hover_pos();
        if response.hovered() {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
            let options = &self.input_preferences.options;
            let factor = pinch / options.wheel_zoom_factor(scroll.y) as f32;
            if (factor - 1.).abs() > 1e-4
                && let Some(p) = pointer
            {
                let new = (zoom * factor).clamp(ZOOM.0, ZOOM.1);
                let at = (p - origin - pan) / zoom;
                pan = p - origin - at * new;
                zoom = new;
            }
        }
        let to_screen = move |p: [f32; 2]| origin + pan + vec2(p[0], p[1]) * zoom;
        let to_page = move |p: Pos2| ((p - origin - pan) / zoom).to_pos2();

        // Node rectangles: saved or shown places, then a drag in progress.
        let mut at: BTreeMap<String, Rect> = BTreeMap::new();
        for n in &model.nodes {
            let mut p = view.positions.get(&n.key).copied().unwrap_or([0., 0.]);
            if let Some((k, _, now)) = &self.schematic.moving
                && k == &n.key
            {
                p = [now.x, now.y];
            }
            at.insert(
                n.key.clone(),
                Rect::from_min_size(to_screen(p), vec2(metrics.node_w, metrics.height(n)) * zoom),
            );
        }
        let drawn_zoom = zoom;
        let port_pos = |key: &str, port: &str, output: bool| -> Option<Pos2> {
            let n = model.nodes.iter().find(|n| n.key == key)?;
            let r = at.get(key)?;
            let y = metrics.port_y(n, port, output)?;
            Some(pos2(
                if output { r.right() } else { r.left() },
                r.top() + y * drawn_zoom,
            ))
        };

        // Wires (bezier from output to input, SC-33).
        let mut nearest: Option<(f32, (u64, String))> = None;
        let wire_w = (1.5 * zoom).clamp(0.6, 2.5);
        for w in &model.wires {
            let (Some(a), Some(b)) = (
                port_pos(&w.from, &w.from_port, true),
                port_pos(&w.to, &w.to_port, false),
            ) else {
                continue;
            };
            let dx = ((b.x - a.x).abs() * 0.5).max(40. * zoom);
            let pts = [a, a + vec2(dx, 0.), b - vec2(dx, 0.), b];
            let port = if w.from.starts_with("piece:") {
                "out".to_owned()
            } else {
                w.to_port.clone()
            };
            let selected = self.schematic.selected_wire.as_ref() == Some(&(w.piece, port.clone()));
            let colour = if selected {
                crate::theme::orange()
            } else {
                ui.visuals().text_color()
            };
            painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                pts,
                false,
                Color32::TRANSPARENT,
                Stroke::new(if selected { wire_w + 1. } else { wire_w }, colour),
            ));
            if let Some(p) = pointer {
                let d = (0..=16)
                    .map(|i| {
                        let t = i as f32 / 16.;
                        let u = 1. - t;
                        let q = pts[0].to_vec2() * u * u * u
                            + pts[1].to_vec2() * 3. * u * u * t
                            + pts[2].to_vec2() * 3. * u * t * t
                            + pts[3].to_vec2() * t * t * t;
                        (q - p.to_vec2()).length()
                    })
                    .fold(f32::MAX, f32::min);
                if d < 6. && nearest.as_ref().is_none_or(|n| d < n.0) {
                    nearest = Some((d, (w.piece, port)));
                }
            }
        }

        // Nodes.
        let text = ui.visuals().text_color();
        let accent = crate::theme::orange();
        let selected_item = self.editor.scene.selection.item;
        let mut hovered_port: Option<(String, String, bool)> = None;
        let mut hovered_node: Option<String> = None;
        let mut hovered_stub: Option<&schematic_pages::Stub> = None;
        let labels = zoom >= 0.45;
        for n in &model.nodes {
            let r = at[&n.key];
            let fill = if n.broken {
                ui.visuals().error_fg_color.gamma_multiply(0.35)
            } else if n.piece {
                ui.visuals().widgets.inactive.bg_fill
            } else {
                ui.visuals().widgets.noninteractive.bg_fill
            };
            painter.rect_filled(r, 4. * zoom, fill);
            // Its item selected (SC-70) or the node picked: accent outline (UIC-01).
            let lit = (!n.piece && selected_item == Some(n.id))
                || self.schematic.selected_node.as_ref() == Some(&n.key);
            painter.rect_stroke(
                r,
                4. * zoom,
                if lit {
                    Stroke::new(2f32, accent)
                } else {
                    Stroke::new(1f32, ui.visuals().widgets.noninteractive.bg_stroke.color)
                },
                egui::StrokeKind::Inside,
            );
            painter.text(
                r.min + vec2(6., 4.) * zoom,
                egui::Align2::LEFT_TOP,
                &n.label,
                egui::FontId::proportional(13. * zoom),
                text,
            );
            if labels && let Some(detail) = &n.detail {
                // The formula, cut to the node's width (the tooltip has it all).
                let fits = ((metrics.node_w - 12.) / 6.2).floor().max(4.) as usize;
                let shown: String = if detail.chars().count() > fits {
                    detail.chars().take(fits - 1).chain(['…']).collect()
                } else {
                    detail.clone()
                };
                painter.text(
                    r.min + vec2(6., metrics.head_h - 2.) * zoom,
                    egui::Align2::LEFT_TOP,
                    shown,
                    egui::FontId::monospace(10. * zoom),
                    ui.visuals().weak_text_color(),
                );
            }
            if pointer.is_some_and(|p| r.contains(p)) {
                hovered_node = Some(n.key.clone());
            }
            let add_port = [ADD.to_owned()];
            let add: &[String] = if n.piece { &[] } else { &add_port };
            for (output, list) in [(false, &n.inputs), (true, &n.outputs)] {
                for port in list.iter().chain(add) {
                    let Some(p) = port_pos(&n.key, port, output) else {
                        continue;
                    };
                    let stub = view
                        .stubs
                        .iter()
                        .find(|s| s.node == n.key && &s.port == port && s.output == output);
                    let radius = (4. * zoom).max(2.);
                    if stub.is_some() {
                        // SC-14: a wire to a node not on this page.
                        painter.circle_stroke(p, radius + 2., Stroke::new(1.5f32, accent));
                    }
                    painter.circle_filled(p, radius, accent);
                    if labels {
                        let (label_at, align) = if output {
                            (p - vec2(8. * zoom, 0.), egui::Align2::RIGHT_CENTER)
                        } else {
                            (p + vec2(8. * zoom, 0.), egui::Align2::LEFT_CENTER)
                        };
                        painter.text(
                            label_at,
                            align,
                            port,
                            egui::FontId::proportional(11. * zoom),
                            text,
                        );
                    }
                    if pointer.is_some_and(|q| (q - p).length() < 7.) {
                        hovered_port = Some((n.key.clone(), port.clone(), output));
                        hovered_stub = stub;
                    }
                }
            }
        }

        // Gestures: a press the navigation preset claims navigates (as in
        // an orthographic 3D view: orbit pans, since a page can't turn);
        // otherwise drag from an output port draws a wire, drag a node moves
        // it, and drag empty space or middle-drag pans; click selects.
        let mods = ui.input(|i| i.modifiers);
        for button in [
            egui::PointerButton::Primary,
            egui::PointerButton::Middle,
            egui::PointerButton::Secondary,
        ] {
            if response.drag_started_by(button)
                && let Some(p) = pointer
                && let Some(kind) = self.input_preferences.navigation.navigation(button, mods)
            {
                self.schematic.nav = Some((kind, p));
            }
        }
        let panning = self.schematic.nav.is_some()
            || response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary);
        if let Some((kind, anchor)) = self.schematic.nav
            && response.dragged()
        {
            use crate::input::NavigationKind as K;
            let delta = response.drag_delta();
            match kind {
                K::Pan | K::Orbit | K::Look => pan += delta,
                K::PanVertical => pan.y += delta.y,
                K::Zoom => {
                    let factor = self.input_preferences.options.drag_zoom_factor(delta) as f32;
                    let new = (zoom / factor).clamp(ZOOM.0, ZOOM.1);
                    let at = (anchor - origin - pan) / zoom;
                    pan = anchor - origin - at * new;
                    zoom = new;
                }
                K::BoxZoom => {
                    if let Some(p) = pointer {
                        painter.rect_stroke(
                            Rect::from_two_pos(anchor, p),
                            0.,
                            Stroke::new(1f32, accent),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
                K::Roll => {}
            }
        }
        if response.drag_stopped()
            && let Some((kind, anchor)) = self.schematic.nav.take()
            && kind == crate::input::NavigationKind::BoxZoom
            && let Some(p) = pointer
        {
            // NAV-07: the dragged box fills the view.
            let b = Rect::from_two_pos(anchor, p);
            if b.width() > 4. && b.height() > 4. {
                let new = (zoom * (rect.width() / b.width()).min(rect.height() / b.height()))
                    .clamp(ZOOM.0, ZOOM.1);
                let centre = (b.center() - origin - pan) / zoom;
                pan = rect.center() - origin - centre * new;
                zoom = new;
            }
        }
        if response.drag_started_by(egui::PointerButton::Primary) && !panning {
            if let Some((key, port, true)) = hovered_port.clone() {
                self.schematic.drawing = Some((key, port));
            } else if let Some(key) = hovered_node.clone()
                && let Some(p) = pointer
            {
                let r = at[&key];
                let grab = (p - r.min) / zoom;
                self.schematic.moving = Some((key, grab, to_page(r.min)));
            }
        }
        if response.dragged() {
            if let (Some((key, grab, _)), Some(p), false) =
                (self.schematic.moving.clone(), pointer, panning)
            {
                self.schematic.moving = Some((key, grab, to_page(p) - grab));
            } else if self.schematic.drawing.is_none() && self.schematic.nav.is_none() {
                pan += response.drag_delta();
            }
        }
        if let (Some((key, port)), Some(p)) = (&self.schematic.drawing, pointer)
            && let Some(a) = port_pos(key, port, true)
        {
            painter.line_segment([a, p], Stroke::new(1.5f32, accent));
        }
        if response.drag_stopped() {
            self.schematic.nav = None;
            if let Some((from, from_port)) = self.schematic.drawing.take()
                && let Some((to, to_port, false)) = hovered_port.clone()
            {
                self.schematic_drop(model, &from, &from_port, &to, &to_port, pointer, &mut lines);
            }
            if let Some((key, _, now)) = self.schematic.moving.take() {
                let line = self.schematic_save_line(page, &view, &[(key, [now.x, now.y])]);
                lines.push(line);
            }
        }
        if response.double_clicked()
            && let Some(stub) = hovered_stub
            && let Some(p) = view.positions.get(&stub.node)
        {
            // SC-14: bring the other end onto the page beside this node.
            let side = if stub.output { 1. } else { -1. };
            let x = p[0] + side * (metrics.node_w + metrics.gap_x);
            let line = self.schematic_save_line(page, &view, &[(stub.other.clone(), [x, p[1]])]);
            lines.push(line);
        } else if response.clicked() {
            self.schematic.selected_wire = nearest.map(|n| n.1);
            self.schematic.selected_node = hovered_node.clone();
            // SC-70: clicking an item's node selects the item.
            if let Some(key) = &hovered_node
                && let Some(id) = key.strip_prefix("item:")
            {
                lines.push(format!("select.item {id}"));
            }
        }
        if response.secondary_clicked()
            && let Some(p) = pointer
        {
            self.schematic.menu = Some((hovered_node.clone().unwrap_or_default(), p));
        }
        let delete =
            ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));
        if delete && response.hovered() {
            if let Some((piece, port)) = self.schematic.selected_wire.take() {
                lines.push(format!("operandi.rig.unwire piece:{piece} port:{{{port}}}"));
                self.status = "Deleted the wire".into();
            } else if let Some(key) = self.schematic.selected_node.take() {
                // SC-41: keys take a node off the page, never out of the scene.
                self.schematic_on_page(
                    page,
                    &view,
                    format!("operandi.rig.schematicRemove workspace:{{ws}} nodes:{{{key}}}"),
                    &mut lines,
                );
                self.status = "Took the node off this page (the scene keeps it)".into();
            }
        }
        // Hovering a piece says what it is and what it drives.
        if self.schematic.moving.is_none()
            && self.schematic.drawing.is_none()
            && let Some(key) = &hovered_node
            && let Some(n) = model.nodes.iter().find(|n| &n.key == key && n.piece)
        {
            let label = |k: &str| {
                model
                    .nodes
                    .iter()
                    .find(|n| n.key == k)
                    .map_or_else(|| k.to_owned(), |n| n.label.clone())
            };
            let mut tip = n.label.clone();
            if let Some(d) = &n.detail {
                tip.push_str(&format!("\n{d}"));
            }
            for w in model.wires.iter().filter(|w| w.from == n.key) {
                tip.push_str(&format!("\nDrives {} {}", label(&w.to), w.to_port));
            }
            response.clone().on_hover_text_at_pointer(tip);
        } else if let Some(stub) = hovered_stub {
            response.clone().on_hover_text_at_pointer(format!(
                "Wired to {}, not on this page. Double-click to bring it here.",
                stub.other
            ));
        }
        self.schematic.views.insert(page, (pan, zoom));
        let mid = (rect.center() - origin - pan) / zoom;
        self.schematic.centre = [mid.x, mid.y];
        if let Some(note) = &self.schematic.note {
            painter.text(
                rect.left_bottom() + vec2(8., -8.),
                egui::Align2::LEFT_BOTTOM,
                note,
                egui::FontId::proportional(12.),
                ui.visuals().weak_text_color(),
            );
        }
        self.schematic_popups(ui, &view, Some(page), &mut lines);
        self.schematic_run(lines);
    }

    /// Puts `keys` on the page, arranged (SC-10, SC-17); with no page yet,
    /// a new workspace holding them.
    pub(crate) fn schematic_add(
        &mut self,
        page: Option<PageId>,
        view: &PageView,
        keys: &[String],
        lines: &mut Vec<String>,
    ) {
        // Only what isn't on the page yet; it lands in the middle of the
        // view (SC-13), laid out together (SC-60).
        let new: Vec<String> = keys
            .iter()
            .filter(|k| !view.positions.contains_key(*k))
            .cloned()
            .collect();
        if new.is_empty() {
            self.schematic.note = Some(if keys.is_empty() {
                "Nothing to add: select items first".into()
            } else {
                "Already on this page".into()
            });
            return;
        }
        let c = self.schematic.centre;
        let places: Vec<(String, [f32; 2])> =
            schematic_pages::arranged(&self.editor.scene, &new, &Metrics::default())
                .into_iter()
                .map(|(k, p)| (k, [p[0] + c[0] - 100., p[1] + c[1] - 60.]))
                .collect();
        let place = place_arg(places.iter().map(|(k, p)| (k, p)));
        self.schematic.note = Some(format!(
            "Added {} node{} to this page",
            new.len(),
            if new.len() == 1 { "" } else { "s" }
        ));
        match page {
            Some(p) => self.schematic_on_page(
                p,
                view,
                format!("operandi.rig.schematicPlace workspace:{{ws}} place:{place} pinned:false"),
                lines,
            ),
            None => lines.push(format!(
                "operandi.rig.workspaceAdd name:Workspace place:{place}"
            )),
        }
    }

    /// Runs the frame's lines; a line that made a workspace shows it and
    /// runs what was waiting for it.
    pub(crate) fn schematic_run(&mut self, lines: Vec<String>) {
        for l in lines {
            self.schematic_run_one(&l);
        }
        self.schematic.after_save.clear();
    }

    fn schematic_run_one(&mut self, l: &str) {
        let before = self.editor.scene.animation.rig.workspaces.len();
        let pieces = self.editor.scene.animation.rig.pieces.len();
        self.execute(l);
        // A modifier from Add ▾: onto this page, mid-view.
        if l.starts_with("operandi.rig.modifierAdd")
            && std::mem::take(&mut self.schematic.place_new_piece)
            && self.editor.scene.animation.rig.pieces.len() > pieces
            && let Some(p) = self.editor.scene.animation.rig.pieces.last()
        {
            let key = format!("piece:{}", p.id);
            let page = self.schematic.page;
            let view = page.map(|p| self.schematic_page(p)).unwrap_or_default();
            let mut more = Vec::new();
            self.schematic_add(page, &view, &[key], &mut more);
            self.schematic.note = Some("Added the modifier; drag from a port to wire it".into());
            for m in more {
                self.schematic_run_one(&m);
            }
            return;
        }
        let rig = &self.editor.scene.animation.rig;
        if l.starts_with("operandi.rig.workspaceAdd") && rig.workspaces.len() > before {
            let ws = rig.workspaces.last().expect("added").id;
            self.schematic.page = Some(PageId::Saved(ws));
            for waiting in std::mem::take(&mut self.schematic.after_save) {
                self.execute(&waiting.replace("{ws}", &ws.to_string()));
            }
        }
    }

    /// The right-click menu, the channel pick and the name prompt.
    fn schematic_popups(
        &mut self,
        ui: &mut Ui,
        view: &PageView,
        page: Option<PageId>,
        lines: &mut Vec<String>,
    ) {
        // The right-click menu (never empty, UIC-07).
        if let Some((key, at_pos)) = self.schematic.menu.clone() {
            let mut close = false;
            let saved = match page {
                Some(PageId::Saved(ws)) => Some(ws),
                _ => None,
            };
            egui::Area::new(egui::Id::new("schematic-menu"))
                .fixed_pos(at_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::menu(ui.style()).show(ui, |ui| {
                        if let Some(id) = key.strip_prefix("piece:") {
                            if ui
                                .button("Delete Piece")
                                .on_hover_text("Delete the rig piece and its wires from the scene\noperandi.rig.remove")
                                .clicked()
                            {
                                lines.push(format!("operandi.rig.remove piece:{id}"));
                                close = true;
                            }
                        } else if let Some(id) = key.strip_prefix("item:")
                            && ui.button("Select").clicked()
                        {
                            lines.push(format!("select.item {id}"));
                            close = true;
                        }
                        if !key.is_empty()
                            && let Some(p) = page
                            && ui
                                .button("Remove from Page")
                                .on_hover_text("Take the node off this page; the scene keeps it (Delete, SC-41)\noperandi.rig.schematicRemove")
                                .clicked()
                        {
                            self.schematic_on_page(
                                p,
                                view,
                                format!("operandi.rig.schematicRemove workspace:{{ws}} nodes:{{{key}}}"),
                                lines,
                            );
                            close = true;
                        }
                        if key.is_empty()
                            && let Some(ws) = saved
                        {
                            if ui
                                .button("Rename Workspace…")
                                .on_hover_text("operandi.rig.workspaceRename")
                                .clicked()
                            {
                                let name = self
                                    .editor
                                    .scene
                                    .animation
                                    .rig
                                    .workspaces
                                    .iter()
                                    .find(|w| w.id == ws)
                                    .map(|w| w.name.clone())
                                    .unwrap_or_default();
                                self.schematic.naming = Some((Some(ws), name));
                                close = true;
                            }
                            if ui
                                .button("Clear Workspace")
                                .on_hover_text("Take every node off this page; the scene keeps them (SC-06)\noperandi.rig.workspaceClear")
                                .clicked()
                            {
                                lines.push(format!("operandi.rig.workspaceClear workspace:{ws}"));
                                close = true;
                            }
                            if ui
                                .button("Delete Workspace")
                                .on_hover_text("Delete this page; the scene keeps everything on it\noperandi.rig.workspaceRemove")
                                .clicked()
                            {
                                lines.push(format!("operandi.rig.workspaceRemove workspace:{ws}"));
                                close = true;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });
            if close {
                self.schematic.menu = None;
            }
        }
        // A drop on "+" waits for the channel.
        if let Some((item, line, p)) = self.schematic.pick.clone() {
            let mut done = false;
            egui::Area::new(egui::Id::new("schematic-pick"))
                .fixed_pos(p)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::menu(ui.style()).show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(260.)
                            .show(ui, |ui| {
                                let mut names = operandi_core::animation::keyable_channels(
                                    &self.editor.scene,
                                    item,
                                );
                                if line.contains("driver:{}") || line.contains("={}") {
                                    names.extend(operandi_core::rig::matrix_channel_names());
                                }
                                for n in names {
                                    if ui.button(&n).clicked() {
                                        lines.push(line.replace("{}", &n));
                                        done = true;
                                    }
                                }
                            });
                        if ui.button("Cancel").clicked() {
                            done = true;
                        }
                    });
                });
            if done {
                self.schematic.pick = None;
            }
        }
        // New Workspace… / Rename Workspace…: the name (UIC-03).
        if let Some((renaming, mut name)) = self.schematic.naming.clone() {
            let mut close = false;
            egui::Area::new(egui::Id::new("schematic-name"))
                .anchor(egui::Align2::CENTER_TOP, vec2(0., 40.))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::menu(ui.style()).show(ui, |ui| {
                        ui.label(if renaming.is_some() {
                            "Rename Workspace"
                        } else {
                            "New Workspace"
                        });
                        let edit = ui.text_edit_singleline(&mut name);
                        edit.request_focus();
                        let enter =
                            edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        ui.horizontal(|ui| {
                            if (ui.button("OK").clicked() || enter) && !name.trim().is_empty() {
                                lines.push(match renaming {
                                    Some(ws) => format!(
                                        "operandi.rig.workspaceRename workspace:{ws} name:{{{}}}",
                                        name.trim()
                                    ),
                                    None => format!(
                                        "operandi.rig.workspaceAdd name:{{{}}}",
                                        name.trim()
                                    ),
                                });
                                close = true;
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                        });
                    });
                });
            self.schematic.naming = (!close).then_some((renaming, name));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn schematic_drop(
        &mut self,
        model: &Schematic,
        from: &str,
        from_port: &str,
        to: &str,
        to_port: &str,
        pointer: Option<Pos2>,
        lines: &mut Vec<String>,
    ) {
        let id_of = |k: &str| k.split_once(':').and_then(|(_, i)| i.parse::<u64>().ok());
        // "+" on either end: pick the channel, then wire.
        if from_port == ADD || to_port == ADD {
            let placeholder = "{}";
            let (fp, tp, item) = if from_port == ADD {
                (placeholder, to_port, id_of(from))
            } else {
                (from_port, placeholder, id_of(to))
            };
            if let Some(item) = item {
                match connect_line(model, from, fp, to, tp) {
                    Ok(line) => {
                        self.schematic.pick = Some((item, line, pointer.unwrap_or_default()));
                    }
                    Err(e) => self.status = e,
                }
            }
            return;
        }
        match connect_line(model, from, from_port, to, to_port) {
            Ok(line) => lines.push(line),
            Err(e) => self.status = e,
        }
    }
}
