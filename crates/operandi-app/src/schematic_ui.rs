//! The Schematic window (RG-70..72, F12): the rig as nodes and wires,
//! drawn from `operandi_core::rig_schematic::build` each frame. Wiring,
//! unwiring, adding modifiers and deleting pieces each run one
//! `operandi.rig.*` line (UIC-160), so undo, scripts and the bridge agree.
//! A floating window (UIC-183), hidden with the animation switch.
use super::*;
use crate::command_handlers::Commands;
use operandi_core::command_registry::{self as table, Area};
use operandi_core::rig_schematic::{self, Node, Schematic, WORLD};
use std::collections::BTreeMap;

const NODE_W: f32 = 170.;
const ROW_H: f32 = 18.;
const HEAD_H: f32 = 22.;
/// The "add channel" port on items (RG-70).
const ADD: &str = "+";

#[derive(Default)]
pub(crate) struct SchematicView {
    pub(crate) open: bool,
    /// Node positions the user dragged to, by key.
    positions: BTreeMap<String, Pos2>,
    /// Items added to the view by hand (RG-70), shown even unwired.
    extra_items: Vec<u64>,
    pan: Vec2,
    /// A wire being drawn: from node key and output port, and where.
    drawing: Option<(String, String)>,
    moving: Option<(String, Vec2)>,
    selected_wire: Option<(u64, String)>,
    /// A drop on an "add channel" port waits for a channel pick: the
    /// line to finish, with `{}` for the channel.
    pick: Option<(u64, String, Pos2)>,
    /// What the right-click menu is for.
    menu: Option<(String, Pos2)>,
}

pub(crate) fn register(r: &mut Commands) {
    r.add_host_command(
        table::script(
            "operandi.rig.schematicView",
            Area::Items,
            "Shows or hides the Schematic (RG-70): rig pieces and the items they use as nodes and wires.",
        )
        .operandi()
        .ui(),
        |a, _| {
            if !crate::anim_switch::enabled() {
                a.status = "Animation is off (Preferences > Experimental > Animation)".into();
                return;
            }
            a.schematic.open = !a.schematic.open;
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
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
            on_top: false,
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

    /// The model, with the hand-added items as nodes too.
    fn schematic_model(&self) -> Schematic {
        let mut s = rig_schematic::build(&self.editor.scene);
        for &id in &self.schematic.extra_items {
            if s.nodes.iter().any(|n| n.key == format!("item:{id}")) {
                continue;
            }
            if let Some(item) = self.editor.scene.item(id) {
                s.nodes.push(Node {
                    key: format!("item:{id}"),
                    label: item.name.clone(),
                    id,
                    piece: false,
                    kind: None,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    broken: false,
                    column: 0,
                });
            }
        }
        // Every item node offers "add channel" both ways (RG-70).
        for n in s.nodes.iter_mut().filter(|n| !n.piece) {
            n.inputs.push(ADD.into());
            n.outputs.push(ADD.into());
        }
        s
    }

    fn schematic_ui(&mut self, ui: &mut Ui) {
        let mut lines: Vec<String> = Vec::new();
        let model = self.schematic_model();
        ui.horizontal(|ui| {
            let r = ui.button("Add ▾").on_hover_text("Add a modifier (RG-71: wire it by hand)");
            egui::Popup::menu(&r).show(|ui| {
                for kind in operandi_core::rig::MODIFIERS {
                    if ui.button(operandi_core::rig::label(kind)).clicked() {
                        let item = self.editor.scene.selection.item.or_else(|| {
                            model.nodes.iter().find(|n| !n.piece).map(|n| n.id)
                        });
                        match item {
                            Some(i) => lines.push(format!(
                                "operandi.rig.modifierAdd kind:{} item:{i}",
                                &kind[4..]
                            )),
                            None => self.status = "Select an item for the modifier to belong to".into(),
                        }
                        ui.close();
                    }
                }
            });
            if ui
                .button("Add Selected Item")
                .on_hover_text("Show the selected items as nodes (their linked channels and an add-channel port)")
                .clicked()
            {
                for id in self.editor.scene.selected_items() {
                    if !self.schematic.extra_items.contains(&id) {
                        self.schematic.extra_items.push(id);
                    }
                }
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
        // Layout: columns, then the user's moves.
        let mut column_rows = [0usize; 3];
        let mut at: BTreeMap<String, Rect> = BTreeMap::new();
        for n in &model.nodes {
            let rows = n.inputs.len().max(n.outputs.len()).max(1);
            let h = HEAD_H + rows as f32 * ROW_H + 6.;
            let col = n.column.min(2) as usize;
            let default = rect.min
                + vec2(
                    20. + col as f32 * (NODE_W + 90.),
                    20. + column_rows[col] as f32 * 12.,
                )
                + self.schematic.pan;
            column_rows[col] += (h / 12.).ceil() as usize + 2;
            let pos = self
                .schematic
                .positions
                .get(&n.key)
                .map_or(default, |p| *p + self.schematic.pan);
            at.insert(n.key.clone(), Rect::from_min_size(pos, vec2(NODE_W, h)));
        }
        let port_pos = |key: &str, port: &str, output: bool| -> Option<Pos2> {
            let n = model.nodes.iter().find(|n| n.key == key)?;
            let r = at.get(key)?;
            let list = if output { &n.outputs } else { &n.inputs };
            let i = list.iter().position(|p| p == port)?;
            Some(pos2(
                if output { r.right() } else { r.left() },
                r.top() + HEAD_H + (i as f32 + 0.5) * ROW_H,
            ))
        };
        let pointer = response.hover_pos();
        // Wires (UIC: bezier from output to input).
        let mut nearest: Option<(f32, (u64, String))> = None;
        for w in &model.wires {
            let (Some(a), Some(b)) = (
                port_pos(&w.from, &w.from_port, true),
                port_pos(&w.to, &w.to_port, false),
            ) else {
                continue;
            };
            let dx = ((b.x - a.x).abs() * 0.5).max(40.);
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
                Stroke::new(if selected { 2.5f32 } else { 1.5f32 }, colour),
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
        let mut hovered_port: Option<(String, String, bool)> = None;
        let mut hovered_node: Option<String> = None;
        for n in &model.nodes {
            let r = at[&n.key];
            let fill = if n.broken {
                ui.visuals().error_fg_color.gamma_multiply(0.35)
            } else if n.piece {
                ui.visuals().widgets.inactive.bg_fill
            } else {
                ui.visuals().widgets.noninteractive.bg_fill
            };
            painter.rect_filled(r, 4., fill);
            painter.rect_stroke(
                r,
                4.,
                Stroke::new(1f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
                egui::StrokeKind::Inside,
            );
            painter.text(
                r.min + vec2(6., 4.),
                egui::Align2::LEFT_TOP,
                &n.label,
                egui::FontId::proportional(13.),
                text,
            );
            if pointer.is_some_and(|p| r.contains(p)) {
                hovered_node = Some(n.key.clone());
            }
            for (output, list) in [(false, &n.inputs), (true, &n.outputs)] {
                for port in list {
                    let Some(p) = port_pos(&n.key, port, output) else {
                        continue;
                    };
                    painter.circle_filled(p, 4., crate::theme::orange());
                    let label_at = if output {
                        p - vec2(8., 0.)
                    } else {
                        p + vec2(8., 0.)
                    };
                    let align = if output {
                        egui::Align2::RIGHT_CENTER
                    } else {
                        egui::Align2::LEFT_CENTER
                    };
                    painter.text(label_at, align, port, egui::FontId::proportional(11.), text);
                    if pointer.is_some_and(|q| (q - p).length() < 7.) {
                        hovered_port = Some((n.key.clone(), port.clone(), output));
                    }
                }
            }
        }
        // Gestures: drag from an output port draws a wire; drag a node
        // moves it; drag the background pans; click a wire selects it.
        if response.drag_started() {
            if let Some((key, port, true)) = hovered_port.clone() {
                self.schematic.drawing = Some((key, port));
            } else if let Some(key) = hovered_node.clone()
                && let Some(p) = pointer
            {
                self.schematic.moving = Some((key.clone(), p - at[&key].min));
            }
        }
        if response.dragged() {
            if let (Some((key, grab)), Some(p)) = (self.schematic.moving.clone(), pointer) {
                self.schematic
                    .positions
                    .insert(key, p - grab - self.schematic.pan);
            } else if self.schematic.drawing.is_none() {
                self.schematic.pan += response.drag_delta();
            }
        }
        if let (Some((key, port)), Some(p)) = (&self.schematic.drawing, pointer)
            && let Some(a) = port_pos(key, port, true)
        {
            painter.line_segment([a, p], Stroke::new(1.5f32, crate::theme::orange()));
        }
        if response.drag_stopped() {
            if let Some((from, from_port)) = self.schematic.drawing.take()
                && let Some((to, to_port, false)) = hovered_port.clone()
            {
                self.schematic_drop(
                    &model, &from, &from_port, &to, &to_port, pointer, &mut lines,
                );
            }
            self.schematic.moving = None;
        }
        if response.clicked() {
            self.schematic.selected_wire = nearest.map(|n| n.1);
        }
        if response.secondary_clicked()
            && let (Some(key), Some(p)) = (hovered_node.clone(), pointer)
        {
            self.schematic.menu = Some((key, p));
        }
        if self.schematic.selected_wire.is_some()
            && ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
            && let Some((piece, port)) = self.schematic.selected_wire.take()
        {
            lines.push(format!("operandi.rig.unwire piece:{piece} port:{{{port}}}"));
        }
        // The right-click menu (never empty, UIC-60).
        if let Some((key, at_pos)) = self.schematic.menu.clone() {
            let mut close = false;
            egui::Area::new(egui::Id::new("schematic-menu"))
                .fixed_pos(at_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::menu(ui.style()).show(ui, |ui| {
                        if let Some(id) = key.strip_prefix("piece:") {
                            if ui
                                .button("Delete")
                                .on_hover_text(
                                    "Delete the piece and its wires\noperandi.rig.remove",
                                )
                                .clicked()
                            {
                                lines.push(format!("operandi.rig.remove piece:{id}"));
                                close = true;
                            }
                        } else if let Some(id) = key.strip_prefix("item:") {
                            if ui.button("Select").clicked() {
                                lines.push(format!("select.item {id}"));
                                close = true;
                            }
                            if ui.button("Hide from Schematic").clicked() {
                                if let Ok(id) = id.parse::<u64>() {
                                    self.schematic.extra_items.retain(|i| *i != id);
                                }
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
        for l in lines {
            self.execute(&l);
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
