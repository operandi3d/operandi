//! Polygon tag selection modes in the modes bar (colour-nav note 04): after
//! Vertices / Edges / Polygons, a Materials mode icon button and a list of
//! the tag type it selects by (Material, Part, Smoothing Group, Selection Set).
//! The mode itself lives in operandi-core (tag_selection.rs).
use super::*;
use crate::unavailable::Gate;
use operandi_core::PolygonTagKind;
use operandi_core::shader_tree::{NodeKind, TagMask, TagType};
use std::cell::RefCell;

/// The action name of the mode button and its key (4).
pub(super) const ACTION: &str = "Materials";

/// The modes bar icon for each tag type (batch 9 icons).
pub(super) fn tag_kind_icon(kind: PolygonTagKind) -> icons::Icon {
    match kind {
        PolygonTagKind::Material => icons::Icon::TagMaterial,
        PolygonTagKind::Part => icons::Icon::TagPart,
        PolygonTagKind::SmoothingGroup => icons::Icon::TagSmoothingGroup,
        PolygonTagKind::SelectionSet => icons::Icon::TagSelectionSet,
    }
}

/// A region outline: its edges, shared.
pub(super) type Outline = std::sync::Arc<std::collections::BTreeSet<(u32, u32)>>;

/// The outline of the selected regions, remembered while the item, its
/// counts, the history and the selection stay the same, so a drag on a big
/// mesh doesn't rebuild it every frame.
pub(super) fn outline(item: &operandi_core::Item, faces: &[u32], serial: u64) -> Outline {
    type Key = (u64, usize, usize, u64, Vec<u32>);
    thread_local! {
        static MEMO: RefCell<Option<(Key, Outline)>> =
            const { RefCell::new(None) };
    }
    let key = (
        item.id,
        item.mesh.faces.len(),
        item.mesh.vertices.len(),
        serial,
        faces.to_vec(),
    );
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if let Some((k, v)) = m.as_ref()
            && *k == key
        {
            return v.clone();
        }
        let edges: Outline = std::sync::Arc::new(
            operandi_core::tag_selection::outline_edges(&item.mesh, faces)
                .into_iter()
                .collect(),
        );
        *m = Some((key, edges.clone()));
        edges
    })
}

thread_local! {
    /// The regions the Shader Tree selection was last matched to.
    static SYNCED: RefCell<Option<Vec<operandi_core::TagRegion>>> = const { RefCell::new(None) };
}

impl Operandi {
    pub(super) fn tag_mode_active(&self) -> bool {
        self.editor.scene.tag_mode().is_some()
    }

    /// The mode button: enter the tag mode; again while on, back to Items.
    /// Alt+click converts the current selection to whole regions.
    pub(super) fn toggle_tag_mode(&mut self, convert: bool) {
        if self.tag_mode_active() && !convert {
            self.run(Command::Mode(Mode::Item));
        } else {
            self.run(Command::EnterTagMode { convert });
        }
    }

    /// The modes bar's Materials control: one square icon button showing
    /// the current tag type, and a narrow triangle button beside it that
    /// opens the tag type list (Jason, 2026-11-07, asked for the triangle
    /// beside the button). The icon button also carries the UIC-25 corner
    /// triangle and opens the same list on right-click or press-and-hold;
    /// a quick plain click runs the mode toggle.
    pub(super) fn tag_mode_controls(&mut self, ui: &mut Ui) {
        let active = self.tag_mode_active();
        let kind = self.editor.polygon_tag_kind;
        // Same height as the Vertices / Edges / Polygons buttons (22 px),
        // with no gap before the triangle so they read as one control.
        let popup_id = Id::new("tag-kind-popup");
        let (button, arrow) = ui
            .scope(|ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                let (rect, button) = ui.allocate_exact_size(vec2(22., 22.), Sense::click());
                // UIC-01 / UIC-23: lit in the accent while the mode is on.
                ui.painter()
                    .rect_filled(rect, 2., theme::chrome::button_fill(&button, active));
                icons::paint(
                    ui,
                    Rect::from_center_size(rect.center(), vec2(20., 20.)),
                    tag_kind_icon(kind),
                );
                // UIC-25: the pop-up cue in the lower-right corner.
                let c = rect.right_bottom() - vec2(2., 2.);
                ui.painter().add(Shape::convex_polygon(
                    vec![c, c - vec2(0., 4.), c - vec2(4., 0.)],
                    theme::text(),
                    Stroke::NONE,
                ));
                let label = ACTION.to_owned();
                button.widget_info(|| {
                    WidgetInfo::selected(WidgetType::SelectableLabel, true, active, &label)
                });
                let (rect, arrow) = ui.allocate_exact_size(vec2(11., 22.), Sense::click());
                ui.painter()
                    .rect_filled(rect, 2., theme::chrome::button_fill(&arrow, false));
                let c = rect.center();
                ui.painter().add(Shape::convex_polygon(
                    vec![c + vec2(-3., -1.5), c + vec2(3., -1.5), c + vec2(0., 2.5)],
                    theme::text(),
                    Stroke::NONE,
                ));
                arrow.widget_info(|| {
                    WidgetInfo::labeled(WidgetType::Button, true, "Polygon Tag Type")
                });
                (button, arrow)
            })
            .inner;
        // UIC-25: press and hold opens the list. The hold is the toolkit's
        // longest click (egui's default `max_click_duration`), so the
        // release that follows is never a click that would toggle the mode
        // or close the list again.
        const HOLD_SECS: f64 = 0.8;
        let pressed_for = ui.input(|i| i.pointer.press_start_time().map(|t| i.time - t));
        let holding = button.is_pointer_button_down_on() && button.contains_pointer();
        let held = holding && pressed_for.is_some_and(|t| t > HOLD_SECS);
        if holding && !held {
            let left = HOLD_SECS - pressed_for.unwrap_or(0.) + 0.01;
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(left.max(0.01)));
        }
        let open = if arrow.clicked() {
            Some(egui::SetOpenCommand::Toggle)
        } else if button.secondary_clicked() || held {
            Some(egui::SetOpenCommand::Bool(true))
        } else {
            None
        };
        if button.clicked() {
            let alt = ui.input(|i| i.modifiers.alt);
            self.toggle_tag_mode(alt);
        }
        let shortcut = self.shortcut(ACTION);
        // UIC-04: name, shortcut and usage.
        button.on_hover_text(format!(
            "{ACTION} mode: select by {}{shortcut}\nClick a polygon to select every polygon \
             with the same {} tag.\nAlt+click: convert the selection to whole regions. \
             Click again to go back to Items.\nRight-click or ▾: choose the tag type.",
            kind.label(),
            kind.label().to_lowercase(),
        ));
        let anchor = arrow.rect.union(arrow.rect.translate(vec2(-22., 0.)));
        let arrow = arrow.on_hover_text(format!("Polygon Tag Type: {}", kind.label()));
        let chosen = egui::Popup::menu(&arrow)
            .id(popup_id)
            .anchor(anchor)
            .open_memory(open)
            .show(|ui| {
                self.menu_owned_input = true;
                let chosen = self.tag_kind_list(ui);
                if chosen.is_some() {
                    ui.close();
                }
                chosen
            })
            .and_then(|r| r.inner);
        if let Some(kind) = chosen {
            self.run(Command::SetPolygonTagKind(kind));
        }
    }

    /// The four tag types, each with its icon; the current one is selected.
    /// Smoothing Group is listed but greyed out: there are no smoothing
    /// group tags yet. Used by the modes bar list and Select > Polygon Tag
    /// Type; returns the type picked this frame.
    pub(super) fn tag_kind_list(&self, ui: &mut Ui) -> Option<PolygonTagKind> {
        let current = self.editor.polygon_tag_kind;
        let mut chosen = None;
        for kind in PolygonTagKind::ALL {
            let button =
                icons::button(ui, tag_kind_icon(kind), kind.label()).selected(current == kind);
            if let Some(reason) = kind.unavailable_reason() {
                ui.gated(Some(reason), button);
            } else if ui.add(button).clicked() {
                chosen = Some(kind);
            }
        }
        chosen
    }

    /// Material type only: selecting a region also selects the matching
    /// material group in the Shader Tree. Runs once per change.
    pub(super) fn sync_tag_shader_selection(&mut self) {
        let regions = match &self.editor.scene.selection.tag {
            Some(t) if t.kind == PolygonTagKind::Material && !t.regions.is_empty() => {
                t.regions.clone()
            }
            _ => {
                SYNCED.with(|s| *s.borrow_mut() = None);
                return;
            }
        };
        if SYNCED.with(|s| s.borrow().as_ref() == Some(&regions)) {
            return;
        }
        let tree = &self.editor.scene.shader_tree;
        let ids: Vec<_> = regions
            .iter()
            .filter_map(|r| match r {
                Some(name) => tree
                    .nodes
                    .iter()
                    .find(|n| {
                        matches!(&n.kind, NodeKind::Group(g)
                            if g.tag_type == TagType::Material
                                && g.tag == TagMask::Name(name.clone()))
                    })
                    .map(|n| n.id),
                // Untagged polygons use the base material.
                None => tree.material_for_tag("Default"),
            })
            .collect();
        if !ids.is_empty() {
            self.shading.selected = ids;
        }
        SYNCED.with(|s| *s.borrow_mut() = Some(regions));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::Component;

    /// A box with polygons 0-2 "Glass" and a Material Group for "Glass".
    fn app() -> (Operandi, u64) {
        let mut app = Operandi::empty();
        app.run(Command::NewMesh);
        app.activate(Tool::Box);
        app.run(Command::Apply);
        app.run(Command::Drop);
        let item = app.editor.scene.active().unwrap().id;
        let glass = operandi_core::Material::default();
        app.editor
            .scene
            .shader_tree
            .ensure_material("Glass", &glass)
            .unwrap();
        let mesh = &mut app.editor.scene.active_mut().unwrap().mesh;
        for f in 0..3 {
            mesh.faces[f].material = "Glass".into();
        }
        (app, item)
    }

    #[test]
    fn the_button_and_key_enter_and_leave_the_mode() {
        let (mut app, item) = app();
        app.execute(ACTION);
        assert!(app.tag_mode_active());
        // Polygons doesn't light while the tag mode is on.
        use crate::button_variants::ButtonAction;
        assert!(!app.action_active(ButtonAction::Mode(Mode::Polygon)));
        app.run(Command::Select {
            item,
            hit: Component::Polygon(1),
            replace: true,
            remove: false,
        });
        let mut faces = app.editor.scene.selection.faces.clone();
        faces.sort_unstable();
        assert_eq!(faces, [0, 1, 2]);
        // Material regions select their Shader Tree group.
        app.sync_tag_shader_selection();
        let group = app.shading.selected.first().copied().unwrap();
        let node = app.editor.scene.shader_tree.get(group).unwrap();
        assert!(matches!(&node.kind, NodeKind::Group(g) if g.tag == TagMask::Name("Glass".into())));
        // Clicking it again goes back to Items.
        app.toggle_tag_mode(false);
        assert_eq!(app.editor.scene.selection.mode, Mode::Item);
        assert!(!app.tag_mode_active());
    }

    #[test]
    fn the_mode_key_is_four_and_free() {
        let four: Vec<_> = crate::input::ACTIONS
            .iter()
            .filter(|(_, key)| *key == "4")
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(four, [ACTION]);
    }

    #[test]
    fn the_icon_follows_the_tag_type() {
        let icons: Vec<_> = PolygonTagKind::ALL.map(tag_kind_icon).into();
        assert_eq!(
            icons,
            [
                icons::Icon::TagMaterial,
                icons::Icon::TagPart,
                icons::Icon::TagSmoothingGroup,
                icons::Icon::TagSelectionSet,
            ]
        );
    }

    /// The modes bar: the Materials icon button toggles the mode, and the
    /// triangle beside it (or a right-click on the button) opens the tag
    /// type list; picking from it sets the type.
    #[test]
    fn the_triangle_and_right_click_choose_the_tag_type() {
        use crate::tests::draw;
        use egui::accesskit::{Node, Role};
        let size = vec2(1440., 900.);
        let (mut app, _) = app();
        let c = Context::default();
        theme::install(&c);
        c.enable_accesskit();
        let frame = |app: &mut Operandi, events: Vec<Event>| -> Vec<Node> {
            draw(app, &c, size, events)
                .platform_output
                .accesskit_update
                .unwrap()
                .nodes
                .into_iter()
                .map(|(_, n)| n)
                .collect()
        };
        let find = |nodes: &[Node], role: Role, name: &str| {
            let n = nodes
                .iter()
                .find(|n| n.role() == role && n.label() == Some(name))
                .unwrap_or_else(|| panic!("no {name}"));
            let b = n.bounds().unwrap();
            pos2((b.x0 + b.x1) as f32 / 2., (b.y0 + b.y1) as f32 / 2.)
        };
        let click = |app: &mut Operandi, p: Pos2, secondary: bool| {
            frame(app, vec![Event::PointerMoved(p)]);
            let button = if secondary {
                PointerButton::Secondary
            } else {
                PointerButton::Primary
            };
            for pressed in [true, false] {
                frame(
                    app,
                    vec![Event::PointerButton {
                        pos: p,
                        button,
                        pressed,
                        modifiers: Modifiers::NONE,
                    }],
                );
            }
            frame(app, vec![])
        };
        let nodes = frame(&mut app, vec![]);
        let mode = find(&nodes, Role::Button, ACTION);
        let arrow = find(&nodes, Role::Button, "Polygon Tag Type");
        assert!(arrow.x > mode.x, "the triangle sits right of the button");

        // A plain click on the icon button enters the mode.
        click(&mut app, mode, false);
        assert!(app.tag_mode_active());

        // The triangle opens the list; picking Part sets the type.
        let nodes = click(&mut app, arrow, false);
        let part = find(&nodes, Role::Button, "Part");
        click(&mut app, part, false);
        assert_eq!(app.editor.polygon_tag_kind, PolygonTagKind::Part);
        assert!(app.tag_mode_active(), "choosing a type keeps the mode");

        // Right-click on the icon button opens the same list.
        let nodes = click(&mut app, mode, true);
        let material = find(&nodes, Role::Button, "Material");
        click(&mut app, material, false);
        assert_eq!(app.editor.polygon_tag_kind, PolygonTagKind::Material);
        assert!(app.tag_mode_active(), "right-click doesn't toggle the mode");
    }
}
