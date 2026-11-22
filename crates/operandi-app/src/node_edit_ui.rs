//! Editing a Material Group's node view (node editing NE-01..NE-17,
//! docs/NODE_EDITING_PLAN.md). The window draws `ShaderTree::board`, made
//! from the layers and the group's free nodes each frame, so the Shader
//! Tree, undo and redo show here at once (NE-17, UIC-244). Every gesture
//! ends as a named command (UIC-160): `operandi.nodes.*` for the node
//! view's own changes, and the Properties edit for a layer's values (NE-01).
use super::*;
use crate::unavailable::Gate;
use operandi_core::node_view::{self, NodeEdit, Target};
use operandi_core::node_view_board::{Board, NodeBox, Role, Value};
use operandi_core::node_view_commands::target_text;
use operandi_core::shader_tree::{BlendMode, NodeId, NodeKind, ShaderTreeEdit};

/// The node view's own state (not part of the document).
#[derive(Default)]
pub(crate) struct BoardState {
    /// The selected box, by view name.
    pub selected: Option<String>,
    /// "As rendered": the read-only expanded form instead (slice 1).
    pub as_rendered: bool,
    drag: Option<Drag>,
    /// What the open right-click menu is for, and where it opened.
    menu: Option<(Option<String>, Pos2)>,
}

enum Drag {
    Pan,
    /// Boxes being moved: the name, the grab offset, whether a step was
    /// already made (later frames fold into it, NE-16).
    Box {
        name: String,
        grab: Vec2,
        started: bool,
    },
    /// A wire from a free node's output, and the input it was picked up
    /// from (dropped elsewhere it disconnects).
    Wire {
        from: String,
        ty: String,
        lifted: Option<Target>,
    },
}

/// What a frame's gestures ask for; run after the window is drawn.
pub(crate) enum BoardAction {
    Edit(NodeEdit),
    /// A Properties edit of a layer (NE-01, NE-05).
    Layer(Box<operandi_core::shader_tree::Node>, bool),
    Refused(String),
}

const NODE_W: f32 = 190.;
const ROW: f32 = 18.;
const HEAD: f32 = 26.;

fn box_height(b: &NodeBox) -> f32 {
    HEAD + ROW * b.inputs.len().max(b.outputs.len()).max(1) as f32 + 6.
}

/// Places for boxes the user hasn't placed: columns by distance from the
/// Material Output, rows in list order.
fn places(board: &Board) -> Vec<Pos2> {
    let n = board.boxes.len();
    let index = |name: &str| board.boxes.iter().position(|b| b.name == name);
    let mut depth = vec![0usize; n];
    for _ in 0..n {
        let mut moved = false;
        for l in &board.links {
            if let (Some(a), Some(b)) = (index(&l.from), index(&l.to))
                && depth[a] < depth[b] + 1
            {
                depth[a] = depth[b] + 1;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    let deepest = depth.iter().copied().max().unwrap_or(0);
    let mut rows = vec![0f32; deepest + 1];
    board
        .boxes
        .iter()
        .zip(&depth)
        .map(|(b, &d)| {
            if let Some([x, y]) = b.at {
                return pos2(x as f32, y as f32);
            }
            let y = rows[d];
            rows[d] += box_height(b) + 30.;
            pos2((deepest - d) as f32 * (NODE_W + 70.), y)
        })
        .collect()
}

fn value_text(v: &Value) -> String {
    match v {
        Value::None => String::new(),
        Value::Numbers(n) => n
            .iter()
            .map(|x| format!("{x:.2}"))
            .collect::<Vec<_>>()
            .join(", "),
        Value::Choice(options, i) => options.get(*i).copied().unwrap_or_default().to_string(),
    }
}

/// The type a box's output carries (`color3`, `float`), for NE-08.
fn output_type(
    tree: &operandi_core::shader_tree::ShaderTree,
    group: NodeId,
    name: &str,
) -> Option<String> {
    tree.group_nodes(group)?.node(name).map(|n| n.ty.clone())
}

/// The canvas: boxes, rows, wires and stubs; drag to move, wire, pan.
pub(crate) fn board_canvas(
    ui: &mut Ui,
    board: &Board,
    tree: &operandi_core::shader_tree::ShaderTree,
    group: NodeId,
    w: &mut super::node_material_ui::GraphWindow,
    out: &mut Vec<BoardAction>,
) {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0., theme::workspace::list());
    let places = places(board);
    if w.zoom <= 0. {
        let (lo, hi) = places.iter().zip(&board.boxes).fold(
            (pos2(f32::MAX, f32::MAX), pos2(f32::MIN, f32::MIN)),
            |(lo, hi), (p, b)| (lo.min(*p), hi.max(*p + vec2(NODE_W, box_height(b)))),
        );
        let span = (hi - lo).max(vec2(1., 1.));
        w.zoom = (rect.width() / (span.x + 80.))
            .min(rect.height() / (span.y + 80.))
            .clamp(0.2, 1.3);
        w.pan = rect.center().to_vec2() - ((lo.to_vec2() + hi.to_vec2()) * 0.5) * w.zoom;
    }
    if response.hovered() {
        let (scroll, pinch, at) = ui.input(|i| {
            (
                i.smooth_scroll_delta.y,
                i.zoom_delta(),
                i.pointer.hover_pos(),
            )
        });
        let factor = pinch * (scroll * 0.002).exp();
        if factor != 1.
            && let Some(at) = at
        {
            let z = (w.zoom * factor).clamp(0.1, 3.);
            let k = z / w.zoom;
            w.pan = at.to_vec2() + (w.pan - at.to_vec2()) * k;
            w.zoom = z;
        }
    }
    let (zoom, pan) = (w.zoom, w.pan);
    let to_screen = |p: Pos2| pos2(p.x * zoom + pan.x, p.y * zoom + pan.y);
    let to_canvas = |p: Pos2| pos2((p.x - pan.x) / zoom, (p.y - pan.y) / zoom);
    let rects: Vec<Rect> = places
        .iter()
        .zip(&board.boxes)
        .map(|(p, b)| Rect::from_min_size(to_screen(*p), vec2(NODE_W, box_height(b)) * zoom))
        .collect();
    let row_y = |r: Rect, k: usize| r.top() + (HEAD + ROW * k as f32 + ROW * 0.5) * zoom;
    let input_at = |i: usize, name: &str| -> Option<Pos2> {
        let k = board.boxes[i].inputs.iter().position(|s| s.name == name)?;
        Some(pos2(rects[i].left(), row_y(rects[i], k)))
    };
    let output_at = |i: usize, name: &str| -> Option<Pos2> {
        let k = board.boxes[i].outputs.iter().position(|s| s == name)?;
        Some(pos2(rects[i].right(), row_y(rects[i], k)))
    };
    let index = |name: &str| board.boxes.iter().position(|b| b.name == name);
    let pointer = ui.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()));
    let near = |p: Pos2, q: Option<Pos2>| q.is_some_and(|q| p.distance(q) < 8. * zoom.max(0.6));
    // What's under a point: an input or output row, or a box.
    let hit_input = |p: Pos2| -> Option<(usize, usize)> {
        board.boxes.iter().enumerate().find_map(|(i, b)| {
            if b.hidden {
                return None;
            }
            (0..b.inputs.len())
                .find(|&k| {
                    p.distance(pos2(rects[i].left(), row_y(rects[i], k))) < 9. * zoom.max(0.6)
                })
                .map(|k| (i, k))
        })
    };
    let hit_output = |p: Pos2| -> Option<usize> {
        board.boxes.iter().enumerate().find_map(|(i, b)| {
            (!b.hidden && !b.outputs.is_empty() && near(p, output_at(i, "out"))).then_some(i)
        })
    };
    let hit_box = |p: Pos2| -> Option<usize> {
        (0..board.boxes.len())
            .rev()
            .find(|&i| !board.boxes[i].hidden && rects[i].contains(p))
    };

    // -- Gestures ---------------------------------------------------
    if response.drag_started()
        && let Some(p) = response.interact_pointer_pos()
    {
        w.board.drag = Some(if let Some(i) = hit_output(p) {
            let b = &board.boxes[i];
            if b.role == Role::Free {
                Drag::Wire {
                    from: b.name.clone(),
                    ty: output_type(tree, group, &b.name).unwrap_or_default(),
                    lifted: None,
                }
            } else {
                out.push(BoardAction::Refused(
                    "Only free nodes' outputs take new wires; the layers stack in the Shader Tree"
                        .into(),
                ));
                Drag::Pan
            }
        } else if let Some((i, k)) = hit_input(p) {
            let b = &board.boxes[i];
            let link = board
                .links
                .iter()
                .find(|l| l.to == b.name && l.input == b.inputs[k].name);
            match (link, &b.inputs[k].target) {
                (Some(l), Some(t)) if !l.locked => Drag::Wire {
                    from: l.from.clone(),
                    ty: output_type(tree, group, &l.from).unwrap_or_default(),
                    lifted: Some(t.clone()),
                },
                (Some(_), _) | (None, None) => {
                    // NE-05: the stack's wires don't move.
                    out.push(BoardAction::Refused(
                        "Mixing nodes are locked: reorder layers in the Shader Tree".into(),
                    ));
                    Drag::Pan
                }
                (None, Some(_)) => Drag::Pan,
            }
        } else if let Some(i) = hit_box(p) {
            w.board.selected = Some(board.boxes[i].name.clone());
            Drag::Box {
                name: board.boxes[i].name.clone(),
                grab: to_canvas(p) - places[i],
                started: false,
            }
        } else {
            Drag::Pan
        });
    }
    if response.dragged() {
        match &mut w.board.drag {
            Some(Drag::Pan) | None => w.pan += response.drag_delta(),
            Some(Drag::Box {
                name,
                grab,
                started,
            }) => {
                if let Some(p) = pointer {
                    let at = to_canvas(p) - *grab;
                    out.push(BoardAction::Edit(NodeEdit::Move {
                        places: vec![(name.clone(), [f64::from(at.x), f64::from(at.y)])],
                        continuing: *started,
                    }));
                    *started = true;
                }
            }
            Some(Drag::Wire { .. }) => {}
        }
    }
    // The wire being dragged: red over an input it can't drive (NE-08).
    let mut wire_drop: Option<(Target, bool)> = None;
    if let Some(Drag::Wire { from, ty, .. }) = &w.board.drag
        && let (Some(p), Some(i)) = (pointer, index(from))
        && let Some(start) = output_at(i, "out")
    {
        let target = hit_input(p).and_then(|(bi, k)| board.boxes[bi].inputs[k].target.clone());
        let ok = target.as_ref().is_some_and(|t| {
            let want = match t {
                Target::Free { node, input } => tree
                    .group_nodes(group)
                    .and_then(|g| g.node(node))
                    .and_then(|n| n.input_type(input)),
                Target::Layer { channel, .. } => {
                    node_view::channel(channel).map(|c| node_view::channel_type(c).to_string())
                }
            };
            want.is_some_and(|want| node_view::compatible(ty, &want))
                && !matches!(t, Target::Free { node, .. } if node == from)
        });
        let colour = if target.is_some() && !ok {
            theme::error()
        } else {
            theme::orange()
        };
        let dx = ((p.x - start.x).abs() * 0.5).max(30. * zoom);
        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
            [start, start + vec2(dx, 0.), p - vec2(dx, 0.), p],
            false,
            Color32::TRANSPARENT,
            Stroke::new(2_f32, colour),
        ));
        if let Some(t) = target {
            wire_drop = Some((t, ok));
        }
    }
    if response.drag_stopped() {
        if let Some(Drag::Wire { from, lifted, .. }) = w.board.drag.take() {
            match (wire_drop, lifted) {
                (Some((to, true)), lifted) if lifted.as_ref() != Some(&to) => {
                    out.push(BoardAction::Edit(NodeEdit::Connect {
                        from,
                        output: "out".into(),
                        to,
                    }))
                }
                (Some((_, false)), _) => out.push(BoardAction::Refused(
                    "Those types don't fit: nothing was wired".into(),
                )),
                (None, Some(lifted)) => out.push(BoardAction::Edit(NodeEdit::Disconnect(lifted))),
                _ => {}
            }
        }
        w.board.drag = None;
    }
    if response.clicked()
        && let Some(p) = response.interact_pointer_pos()
    {
        w.board.selected = hit_box(p).map(|i| board.boxes[i].name.clone());
    }

    // -- Wires and stubs --------------------------------------------
    let mut stubs: Vec<(Pos2, String)> = Vec::new();
    for l in &board.links {
        let (Some(a), Some(b)) = (index(&l.from), index(&l.to)) else {
            continue;
        };
        let (Some(p), Some(q)) = (output_at(a, &l.output), input_at(b, &l.input)) else {
            continue;
        };
        let (ha, hb) = (board.boxes[a].hidden, board.boxes[b].hidden);
        if ha && hb {
            continue;
        }
        let colour = if l.locked {
            theme::text().gamma_multiply(0.35)
        } else {
            theme::orange().gamma_multiply(0.85)
        };
        // NE-07: a wire to a hidden box ends in a marked stub at the
        // visible end; double-click it to show the box.
        if ha || hb {
            let (at, dir, hidden) = if ha {
                (q, -1., l.from.clone())
            } else {
                (p, 1., l.to.clone())
            };
            let end = at + vec2(dir * 24. * zoom, 0.);
            painter.line_segment([at, end], Stroke::new(1.5_f32, colour));
            painter.circle_stroke(end, 4. * zoom, Stroke::new(1.5_f32, colour));
            painter.circle_filled(end, 1.5 * zoom, colour);
            stubs.push((end, hidden));
            continue;
        }
        let dx = ((q.x - p.x).abs() * 0.5).max(30. * zoom);
        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
            [p, p + vec2(dx, 0.), q - vec2(dx, 0.), q],
            false,
            Color32::TRANSPARENT,
            Stroke::new(1.5_f32, colour),
        ));
    }
    if response.double_clicked()
        && let Some(p) = response.interact_pointer_pos()
        && let Some((_, name)) = stubs
            .iter()
            .find(|(at, _)| at.distance(p) < 8. * zoom.max(0.6))
    {
        out.push(BoardAction::Edit(NodeEdit::Show(vec![name.clone()])));
    }

    // -- Boxes --------------------------------------------------------
    let font = FontId::proportional((11. * zoom).clamp(6., 16.));
    let small = FontId::proportional((10. * zoom).clamp(5., 14.));
    for (b, r) in board.boxes.iter().zip(&rects) {
        if b.hidden {
            continue;
        }
        let fade: f32 = if b.dimmed { 0.45 } else { 1. };
        let selected = w.board.selected.as_deref() == Some(b.name.as_str());
        painter.rect_filled(*r, 4. * zoom, theme::panel().gamma_multiply(fade.max(0.8)));
        let header = Rect::from_min_size(r.min, vec2(r.width(), 22. * zoom));
        let head_colour = match b.role {
            Role::Free => theme::orange().gamma_multiply(0.45 * fade),
            Role::Mix(_) => theme::text().gamma_multiply(0.18),
            _ => theme::text().gamma_multiply(0.28 * fade),
        };
        painter.rect_filled(header, 4. * zoom, head_colour);
        painter.rect_stroke(
            *r,
            4. * zoom,
            Stroke::new(
                if selected { 2_f32 } else { 1_f32 },
                if selected {
                    theme::orange()
                } else {
                    theme::text().gamma_multiply(0.3)
                },
            ),
            egui::StrokeKind::Inside,
        );
        painter.text(
            header.left_center() + vec2(6. * zoom, 0.),
            Align2::LEFT_CENTER,
            &b.title,
            font.clone(),
            theme::text().gamma_multiply(fade),
        );
        if b.locked {
            // NE-05: a padlock on the header's right.
            let c = header.right_center() - vec2(10. * zoom, -zoom);
            let body = Rect::from_center_size(c + vec2(0., 2. * zoom), vec2(8., 6.) * zoom);
            painter.circle_stroke(
                c - vec2(0., 2. * zoom),
                2.5 * zoom,
                Stroke::new(1.2_f32, theme::text().gamma_multiply(0.7)),
            );
            painter.rect_filled(body, 1., theme::text().gamma_multiply(0.7));
        }
        for (k, s) in b.inputs.iter().enumerate() {
            let y = row_y(*r, k);
            let colour = if s.driven {
                theme::orange()
            } else if s.target.is_some() {
                theme::text()
            } else {
                theme::text().gamma_multiply(0.4)
            };
            painter.circle_filled(pos2(r.left(), y), 3. * zoom, colour);
            painter.text(
                pos2(r.left() + 7. * zoom, y),
                Align2::LEFT_CENTER,
                &s.label,
                small.clone(),
                theme::text().gamma_multiply(0.85 * fade),
            );
            // NE-01: values edit on the box itself (a drag folds into one
            // undo step on release, UIC-224/226). Mixing nodes edit only
            // their opacity and blend (NE-05). Zoomed far out, the values
            // only show.
            let cell = Rect::from_min_max(
                pos2(r.left() + r.width() * 0.48, y - ROW * zoom * 0.45),
                pos2(r.right() - 6. * zoom, y + ROW * zoom * 0.45),
            );
            let editable = !s.driven
                && (s.target.is_some() || matches!(b.role, Role::Mix(_)))
                && !matches!(s.value, Value::None);
            if editable && zoom >= 0.6 && rect.contains_rect(cell) {
                if let Some(a) = box_value(ui, cell, b, s, tree) {
                    out.push(a);
                }
                continue;
            }
            // UIC-92: a driven value stays in place, greyed.
            let shown = if s.driven {
                format!("{} (driven)", value_text(&s.value))
            } else {
                value_text(&s.value)
            };
            if !shown.is_empty() {
                painter.text(
                    pos2(r.right() - 7. * zoom, y),
                    Align2::RIGHT_CENTER,
                    shown,
                    small.clone(),
                    theme::text().gamma_multiply(if s.driven { 0.4 } else { 0.6 } * fade),
                );
            }
        }
        for (k, s) in b.outputs.iter().enumerate() {
            let y = row_y(*r, k);
            painter.circle_filled(pos2(r.right(), y), 3.5 * zoom, theme::text());
            if b.inputs.is_empty() {
                painter.text(
                    pos2(r.right() - 7. * zoom, y),
                    Align2::RIGHT_CENTER,
                    s,
                    small.clone(),
                    theme::text().gamma_multiply(0.6 * fade),
                );
            }
        }
    }

    // -- Keys and menus -----------------------------------------------
    // The pointer picks who gets Delete / Backspace (MN-08.2): with the
    // pointer over a 3D view they delete its selection, even while the
    // node view keeps keyboard focus (bug 2026-11-21).
    if response.hovered() || (response.has_focus() && !ui.ctx().is_pointer_over_area()) {
        let (delete, hide) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::Delete),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace),
            )
        });
        if let Some(name) = w.board.selected.clone() {
            if delete {
                out.push(BoardAction::Edit(NodeEdit::Delete(vec![name])));
            } else if hide {
                out.push(BoardAction::Edit(NodeEdit::Hide(vec![name])));
            }
        }
    }
    if response.secondary_clicked()
        && let Some(p) = response.interact_pointer_pos()
    {
        let on = hit_box(p).map(|i| board.boxes[i].name.clone());
        if let Some(n) = &on {
            w.board.selected = Some(n.clone());
        }
        w.board.menu = Some((on, to_canvas(p)));
    }
    let menu = w.board.menu.clone();
    response.context_menu(|ui| {
        let Some((on, at)) = &menu else {
            return;
        };
        match on {
            Some(name) => {
                let locked = board.get(name).is_some_and(|b| b.locked);
                let output = name == node_view::OUTPUT;
                let why = if locked {
                    Some("Mixing nodes are locked: delete the layer in the Shader Tree")
                } else if output {
                    Some("The Material Output can't be deleted")
                } else {
                    None
                };
                if ui.gated(why, egui::Button::new("Delete Node")).clicked() {
                    out.push(BoardAction::Edit(NodeEdit::Delete(vec![name.clone()])));
                    ui.close();
                }
                if ui.button("Hide Node").clicked() {
                    out.push(BoardAction::Edit(NodeEdit::Hide(vec![name.clone()])));
                    ui.close();
                }
            }
            None => {
                ui.menu_button("Add", |ui| {
                    let mut groups: Vec<&str> = Vec::new();
                    for k in node_view::CATALOG {
                        if !groups.contains(&k.menu) {
                            groups.push(k.menu);
                        }
                    }
                    for g in groups {
                        ui.menu_button(g, |ui| {
                            for k in node_view::CATALOG.iter().filter(|k| k.menu == g) {
                                if ui.button(k.label).clicked() {
                                    out.push(BoardAction::Edit(NodeEdit::Add {
                                        category: k.category.into(),
                                        ty: k.ty.into(),
                                        at: [f64::from(at.x), f64::from(at.y)],
                                    }));
                                    ui.close();
                                }
                            }
                        });
                    }
                });
                let any_hidden = board.boxes.iter().any(|b| b.hidden);
                if ui
                    .gated(
                        (!any_hidden).then_some("No hidden nodes"),
                        egui::Button::new("Show All"),
                    )
                    .clicked()
                {
                    out.push(BoardAction::Edit(NodeEdit::ShowAll));
                    ui.close();
                }
            }
        }
    });
}

/// One editable value on a box (NE-01, NE-05): numbers as drag fields,
/// a colour as its swatch, a blend mode as a menu. The edit it makes.
fn box_value(
    ui: &mut Ui,
    cell: Rect,
    b: &NodeBox,
    s: &operandi_core::node_view_board::Socket,
    tree: &operandi_core::shader_tree::ShaderTree,
) -> Option<BoardAction> {
    let id = ui.id().with(("node-value", &b.name, &s.name));
    ui.scope_builder(egui::UiBuilder::new().max_rect(cell).id_salt(id), |ui| {
        ui.spacing_mut().interact_size.y = cell.height();
        ui.spacing_mut().item_spacing.x = 2.;
        let layer = match b.role {
            Role::Layer(l) | Role::Mix(l) => Some(l),
            _ => None,
        };
        match &s.value {
            Value::Choice(options, current) => {
                let mut pick = *current;
                egui::ComboBox::from_id_salt(id)
                    .width(cell.width())
                    .selected_text(options.get(pick).copied().unwrap_or_default())
                    .show_ui(ui, |ui| {
                        for (i, o) in options.iter().enumerate() {
                            ui.selectable_value(&mut pick, i, *o);
                        }
                    });
                let mut node = tree.get(layer?)?.clone();
                let blend = *BlendMode::ALL.get(pick)?;
                (pick != *current).then(|| {
                    node.layer.blend = blend;
                    BoardAction::Layer(Box::new(node), false)
                })
            }
            Value::Numbers(v) => {
                let mut vals = v.clone();
                let (changed, continuing) = if s.ty == "color3" && vals.len() == 3 {
                    let mut c = [vals[0] as f32, vals[1] as f32, vals[2] as f32];
                    let r = ui.color_edit_button_rgb(&mut c);
                    vals = c.iter().map(|x| f64::from(*x)).collect();
                    (r.changed(), r.dragged())
                } else {
                    let w = (cell.width() / vals.len().max(1) as f32 - 2.).max(16.);
                    let mut changed = false;
                    let mut continuing = false;
                    for x in &mut vals {
                        let r = ui.add_sized(
                            [w, cell.height()],
                            crate::numeric_tab::DragValue::new(x).speed(0.005),
                        );
                        changed |= r.changed();
                        continuing |= r.dragged() && !r.drag_started();
                    }
                    (changed, continuing)
                };
                if !changed {
                    return None;
                }
                match b.role {
                    Role::Free => Some(BoardAction::Edit(NodeEdit::SetValue {
                        node: b.name.clone(),
                        input: s.name.clone(),
                        value: vals
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(","),
                        continuing,
                    })),
                    // The Properties edit of the layer (NE-01), opacity for
                    // a mixing node (NE-05).
                    _ => tree
                        .edited_layer(layer?, &s.name, &vals)
                        .map(|n| BoardAction::Layer(Box::new(n), continuing)),
                }
            }
            Value::None => None,
        }
    })
    .inner
}

/// The selected box's values (NE-01): a layer's are its Properties
/// values, a mixing node's its layer's opacity and blend (NE-05), a free
/// node's its inputs. Driven values are greyed in place (UIC-92/117).
pub(crate) fn board_values(
    ui: &mut Ui,
    board: &Board,
    tree: &operandi_core::shader_tree::ShaderTree,
    w: &mut super::node_material_ui::GraphWindow,
    out: &mut Vec<BoardAction>,
) {
    let Some(b) = w.board.selected.as_deref().and_then(|n| board.get(n)) else {
        ui.weak("Click a node to see its values. Right-click the canvas to add one.");
        return;
    };
    ui.strong(&b.title);
    if b.dimmed && b.role == Role::Free {
        ui.weak("Feeds nothing yet: drag from its output to an input.");
    }
    match b.role {
        Role::Layer(id) | Role::Mix(id) => {
            let Some(node) = tree.get(id) else { return };
            if let Role::Mix(_) = b.role {
                let mut edited = node.clone();
                ui.horizontal(|ui| {
                    ui.label("Opacity");
                    let r = ui.add(
                        crate::numeric_tab::DragValue::new(&mut edited.layer.opacity)
                            .range(0.0..=1.0)
                            .speed(0.005),
                    );
                    if r.changed() {
                        out.push(BoardAction::Layer(
                            Box::new(edited.clone()),
                            r.dragged() && !r.drag_started(),
                        ));
                    }
                });
                let mut blend = edited.layer.blend;
                egui::ComboBox::from_label("Blend")
                    .selected_text(blend.label())
                    .show_ui(ui, |ui| {
                        for m in BlendMode::ALL {
                            ui.selectable_value(&mut blend, *m, m.label());
                        }
                    });
                if blend != edited.layer.blend {
                    edited.layer.blend = blend;
                    out.push(BoardAction::Layer(Box::new(edited), false));
                }
                ui.weak("Locked: reorder in the Shader Tree.");
                return;
            }
            let NodeKind::Material(m) = &node.kind else {
                ui.weak("Edit this layer in the Shader Tree's Properties.");
                return;
            };
            for s in &b.inputs {
                let Some(ch) = node_view::channel(&s.name) else {
                    continue;
                };
                let Value::Numbers(v) = &s.value else {
                    continue;
                };
                ui.horizontal(|ui| {
                    ui.label(&s.label);
                    ui.gated_ui(
                        s.driven
                            .then_some("Driven by nodes: remove the wire to edit it"),
                        |ui| {
                            let mut edited = node.clone();
                            let NodeKind::Material(em) = &mut edited.kind else {
                                return;
                            };
                            let (changed, continuing) = if v.len() >= 3 {
                                let mut c = [v[0] as f32, v[1] as f32, v[2] as f32];
                                let r = ui.color_edit_button_rgb(&mut c);
                                let rgb = c.map(f64::from);
                                node_view_set(em, ch, rgb);
                                (r.changed(), r.dragged())
                            } else {
                                let mut x = v[0];
                                let r =
                                    ui.add(crate::numeric_tab::DragValue::new(&mut x).speed(0.005));
                                node_view_set(em, ch, [x; 3]);
                                (r.changed(), r.dragged() && !r.drag_started())
                            };
                            if changed && **em != **m {
                                out.push(BoardAction::Layer(Box::new(edited.clone()), continuing));
                            }
                        },
                    );
                    if s.driven {
                        // NE-04 in the node view itself.
                        ui.weak("driven by nodes");
                    }
                });
            }
        }
        Role::Free => {
            for s in &b.inputs {
                let Value::Numbers(v) = &s.value else {
                    ui.horizontal(|ui| {
                        ui.label(&s.label);
                        ui.weak(if s.driven { "wired" } else { "default" });
                    });
                    continue;
                };
                ui.horizontal(|ui| {
                    ui.label(&s.label);
                    ui.gated_ui(
                        s.driven
                            .then_some("Driven by nodes: remove the wire to edit it"),
                        |ui| {
                            let mut vals = v.clone();
                            let mut changed = false;
                            let mut continuing = false;
                            for x in &mut vals {
                                let r = ui.add(crate::numeric_tab::DragValue::new(x).speed(0.01));
                                changed |= r.changed();
                                continuing |= r.dragged() && !r.drag_started();
                            }
                            if changed {
                                out.push(BoardAction::Edit(NodeEdit::SetValue {
                                    node: b.name.clone(),
                                    input: s.name.clone(),
                                    value: vals
                                        .iter()
                                        .map(|x| x.to_string())
                                        .collect::<Vec<_>>()
                                        .join(","),
                                    continuing,
                                }));
                            }
                        },
                    );
                    if s.driven {
                        ui.weak("wired");
                    }
                });
            }
        }
        Role::Output => {
            ui.weak("Takes the stack and the Shader; nothing to set here.");
        }
    }
}

fn node_view_set(
    m: &mut operandi_core::shader_tree::MaterialChannels,
    ch: operandi_core::shader_eval::Ch,
    v: [f64; 3],
) {
    operandi_core::node_view_board::set_channel(m, ch, v);
}

/// A node-view change as its command line (UIC-160).
pub(crate) fn edit_line(group: NodeId, edit: &NodeEdit) -> Option<String> {
    let g = format!("group:{group}");
    let list = |v: &[String]| v.join(",");
    Some(match edit {
        NodeEdit::Add { category, ty, at } => format!(
            "operandi.nodes.add {g} node:{category} type:{ty} x:{} y:{}",
            at[0].round(),
            at[1].round()
        ),
        NodeEdit::Delete(n) => format!("operandi.nodes.delete {g} node:{}", list(n)),
        NodeEdit::Connect { from, to, .. } => {
            format!(
                "operandi.nodes.connect {g} from:{from} to:{}",
                target_text(to)
            )
        }
        NodeEdit::Disconnect(to) => format!("operandi.nodes.disconnect {g} to:{}", target_text(to)),
        NodeEdit::Hide(n) => format!("operandi.nodes.hide {g} node:{}", list(n)),
        NodeEdit::Show(n) => format!("operandi.nodes.show {g} node:{}", list(n)),
        NodeEdit::ShowAll => format!("operandi.nodes.showAll {g}"),
        NodeEdit::Move { places, .. } => {
            let (name, at) = places.first()?;
            format!(
                "operandi.nodes.move {g} node:{name} x:{} y:{}",
                at[0].round(),
                at[1].round()
            )
        }
        NodeEdit::SetValue {
            node, input, value, ..
        } => format!("operandi.nodes.setValue {g} node:{node} input:{input} value:{value}"),
    })
}

impl Operandi {
    /// Runs the node view's gestures (after the window is drawn).
    pub(crate) fn apply_board_actions(&mut self, group: NodeId, actions: Vec<BoardAction>) {
        for a in actions {
            match a {
                BoardAction::Refused(why) => self.status = why,
                BoardAction::Layer(node, continuing) => self.replace_layer(*node, continuing),
                // A drag's frames fold into one step (NE-16, UIC-226).
                // A drag's frames fold into one step (NE-16, UIC-226):
                // the same command as `edit_line` writes, with its fold flag.
                BoardAction::Edit(edit @ (NodeEdit::Move { .. } | NodeEdit::SetValue { .. })) => {
                    self.run(Command::ShaderTree(ShaderTreeEdit::Nodes { group, edit }));
                }
                BoardAction::Edit(edit) => {
                    if let NodeEdit::Delete(names) = &edit
                        && names.iter().any(|n| node_view::layer_of(n).is_some())
                    {
                        self.graph_window.board.selected = None;
                    }
                    if let Some(line) = edit_line(group, &edit) {
                        self.execute(&line);
                    }
                }
            }
        }
    }
}
