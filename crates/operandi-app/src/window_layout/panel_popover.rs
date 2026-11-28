//! Panel popovers (Jason, 2026-11-28; the Panels pie): one panel, on its
//! own, as a popover near the pointer, as O opens Viewport Properties
//! (UIC-181). It is an extra copy: the docked panel stays where it is and
//! its tab group is untouched (UIC-184: the same panel, the same
//! settings). Unpinned, it closes when the pointer rolls off it (UIC-62,
//! the Popover Dismissal preference as Viewport Properties has it), on
//! Esc, or when it is opened again; the circle in its title strip pins it
//! where it is, and a pinned one drags by the strip (UIC-185).
//!
//! `operandi.panel.popover panel:<id>` opens it, so any pie slot, button
//! or key can (the Form Editor's Panel popover slots write this line).
//! Provisional: each panel's size, and that a pinned popover doesn't come
//! back after a relaunch.
use super::*;
use crate::command_handlers::Commands;
use crate::panel_registry::{self, PanelKind};
use operandi_core::command_registry::{self as table, Area};

/// The command that opens a panel popover.
pub(crate) const COMMAND: &str = "operandi.panel.popover";
/// An unpinned popover closes once the pointer is this far outside it.
const ROLL_OFF: f32 = 8.;
/// The popover's top-left sits this far up and left of the pointer, so the
/// pointer starts inside it.
const NEAR_POINTER: Vec2 = vec2(14., 12.);

/// What a panel popover shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PopPanel {
    /// A registered panel kind (UIC-157) on its own, without its group.
    Kind(PanelKind),
    Toolbox,
    Timeline,
    GraphEditor,
}

impl PopPanel {
    /// Every panel a popover can show, in the Form Editor's order.
    pub(crate) fn all() -> Vec<PopPanel> {
        let mut out = vec![PopPanel::Toolbox];
        out.extend(
            panel_registry::REGISTRY
                .iter()
                // It has its own window (preset_browser.rs).
                .filter(|t| t.kind != PanelKind::PresetBrowser)
                .map(|t| PopPanel::Kind(t.kind)),
        );
        out.extend([PopPanel::Timeline, PopPanel::GraphEditor]);
        out
    }

    /// The panel's name, on its title strip and in the Form Editor.
    pub(crate) fn name(self) -> &'static str {
        match self {
            PopPanel::Kind(kind) => panel_registry::panel_type(kind).name,
            PopPanel::Toolbox => "Toolbox",
            PopPanel::Timeline => "Timeline",
            PopPanel::GraphEditor => "Graph Editor",
        }
    }

    /// The id its command line names: the name in camel case
    /// ("Shader Tree" is shaderTree).
    pub(crate) fn id(self) -> String {
        let mut out = String::new();
        for (i, word) in self.name().split_whitespace().enumerate() {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                if i == 0 {
                    out.extend(first.to_lowercase());
                } else {
                    out.extend(first.to_uppercase());
                }
                out.push_str(chars.as_str());
            }
        }
        out
    }

    /// The panel named `id` (any case).
    pub(crate) fn from_id(id: &str) -> Option<PopPanel> {
        let id = id.trim().trim_matches(|c| matches!(c, '"' | '{' | '}'));
        Self::all()
            .into_iter()
            .find(|p| p.id().eq_ignore_ascii_case(id))
    }

    /// The line that opens it.
    pub(crate) fn line(self) -> String {
        format!("{COMMAND} panel:{}", self.id())
    }

    /// Its size (provisional): narrow lists, wide strips.
    fn size(self) -> Vec2 {
        match self {
            PopPanel::Toolbox => vec2(260., 600.),
            PopPanel::Timeline => vec2(900., 200.),
            PopPanel::GraphEditor => vec2(900., 340.),
            PopPanel::Kind(PanelKind::Properties) => vec2(340., 560.),
            PopPanel::Kind(_) => vec2(320., 460.),
        }
    }
}

/// The panel a line opens, when it is `operandi.panel.popover panel:<id>`
/// (or the id bare).
pub(crate) fn line_panel(line: &str) -> Option<PopPanel> {
    let line = line.trim();
    let (verb, rest) = line.split_once(char::is_whitespace)?;
    if !verb.eq_ignore_ascii_case(COMMAND) {
        return None;
    }
    let rest = rest.trim();
    let id = rest
        .get(..6)
        .filter(|h| h.eq_ignore_ascii_case("panel:"))
        .map_or(rest, |_| &rest[6..]);
    PopPanel::from_id(id)
}

/// The open popover.
#[derive(Clone, Debug)]
pub(crate) struct PanelPopover {
    pub panel: PopPanel,
    /// Its top-left; `None` until it first draws, near the pointer then.
    pub at: Option<Pos2>,
    pub pinned: bool,
    /// The pointer has been over it since it opened: rolling off closes it
    /// only after that.
    entered: bool,
    /// Where it drew last frame.
    pub rect: Option<Rect>,
}

const PANEL_ARGS: &[table::Arg] = &[table::arg("panel", table::ArgType::Text, table::Unit::None)];

/// `operandi.panel.popover panel:<id>`.
pub(crate) fn register(r: &mut Commands) {
    r.add_host_command(
        table::script(
            COMMAND,
            Area::View,
            "Opens panel: (items, properties, toolbox, timeline, graphEditor, shaderTree, groups, meshStack, …) on its own as a popover near the pointer, beside the docked one; again closes it (the Panels pie).",
        )
        .args(PANEL_ARGS)
        .example("operandi.panel.popover panel:items")
        .operandi()
        .ui(),
        |a, c| {
            let rest = c.rest.trim();
            let id = rest
                .get(..6)
                .filter(|h| h.eq_ignore_ascii_case("panel:"))
                .map_or(rest, |_| &rest[6..]);
            match PopPanel::from_id(id) {
                Some(panel) => a.toggle_panel_popover(panel),
                None => {
                    a.status = format!(
                        "{COMMAND}: no panel {id}; Operandi has {}",
                        PopPanel::all()
                            .iter()
                            .map(|p| p.id())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
}

impl Operandi {
    /// Open `panel` as a popover near the pointer, or close it when it is
    /// the one open. UIC-181: one unpinned popover at a time.
    pub(crate) fn toggle_panel_popover(&mut self, panel: PopPanel) {
        if self
            .panel_popover
            .as_ref()
            .is_some_and(|p| p.panel == panel)
        {
            self.panel_popover = None;
            return;
        }
        if !self.corner.properties.pinned {
            self.close_view_properties();
        }
        self.panel_popover = Some(PanelPopover {
            panel,
            at: None,
            pinned: false,
            entered: false,
            rect: None,
        });
    }

    /// Where a docked copy of `panel` would draw while its popover is
    /// open: a line saying so instead (two copies in one window would
    /// share their widgets' ids). True when it drew the line.
    pub(crate) fn docked_while_popped(&self, ui: &mut Ui, rect: Rect, panel: PopPanel) -> bool {
        if !self.panel_popover_open(panel) {
            return false;
        }
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            format!("{} is open in a popover", panel.name()),
            crate::typography::standard_font(),
            theme::disabled_text(),
        );
        true
    }

    /// Whether `panel`'s popover is open (a pie slot's tick).
    pub(crate) fn panel_popover_open(&self, panel: PopPanel) -> bool {
        self.panel_popover
            .as_ref()
            .is_some_and(|p| p.panel == panel)
    }

    /// Each frame: draw the open popover and close it when the pointer
    /// rolls off. Returns where it is, so the views leave its clicks alone.
    pub(crate) fn panel_popover_frame(&mut self, ctx: &Context) -> Option<Rect> {
        let open = self.panel_popover.clone()?;
        let screen = ctx.content_rect();
        let size = open.panel.size().min(screen.size() - vec2(16., 16.));
        let at = open.at.unwrap_or_else(|| {
            ctx.input(|i| i.pointer.latest_pos())
                .unwrap_or(screen.center())
                - NEAR_POINTER
        });
        let mut title = crate::popup_title::TitleAction::None;
        let shown = egui::Area::new(Id::new("panel-popover"))
            .order(egui::Order::Foreground)
            .constrain_to(screen)
            .fixed_pos(at)
            .show(ctx, |ui| {
                let frame = egui::Frame::popup(ui.style()).fill(theme::popup_fill());
                let inner = size - frame.total_margin().sum();
                frame.show(ui, |ui| {
                    ui.set_min_size(inner);
                    ui.set_max_size(inner);
                    ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                    title = crate::popup_title::title_bar(ui, open.panel.name(), open.pinned);
                    let body = ui.available_rect_before_wrap();
                    ui.push_id(("panel-popover", open.panel.id()), |ui| match open.panel {
                        // Its own tab ids, apart from the docked panel's.
                        PopPanel::Kind(PanelKind::Properties) => {
                            self.properties_body(ui, body, "popover-properties-category")
                        }
                        PopPanel::Kind(kind) => {
                            (panel_registry::panel_type(kind).draw)(self, ui, body)
                        }
                        PopPanel::Toolbox => self.toolbox(ui),
                        PopPanel::Timeline => self.timeline_ui(ui),
                        PopPanel::GraphEditor => self.graph_editor_ui(ui),
                    });
                });
            });
        let rect = shown.response.rect;
        let Some(p) = self.panel_popover.as_mut() else {
            // Its contents closed it (a command run from inside).
            return Some(rect);
        };
        p.rect = Some(rect);
        p.at = Some(rect.min);
        match title {
            crate::popup_title::TitleAction::Circle if p.pinned => {
                self.panel_popover = None;
                return Some(rect);
            }
            crate::popup_title::TitleAction::Circle => p.pinned = true,
            // UIC-185: a drag pins it, so it stays where it is dropped.
            crate::popup_title::TitleAction::Drag(delta) => {
                p.pinned = true;
                p.at = Some(rect.min + delta);
            }
            crate::popup_title::TitleAction::None => {}
        }
        let hover = ctx.input(|i| i.pointer.hover_pos());
        let near = |q: Pos2| rect.expand(ROLL_OFF).contains(q);
        if hover.is_some_and(near) {
            p.entered = true;
        }
        if p.pinned {
            return Some(rect);
        }
        // A pop-up opened from one of its fields (a dropdown, the colour
        // picker, a context menu) hangs outside it: that isn't leaving.
        if egui::Popup::is_any_open(ctx) || crate::colour_picker::is_open() {
            return Some(rect);
        }
        let entered = p.entered;
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.panel_popover = None;
            return Some(rect);
        }
        let clicked_outside = ctx
            .input(|i| i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|q| !near(q)));
        // As Viewport Properties: Force Click-Off keeps it until a click
        // outside; otherwise rolling off closes it. A held button (a drag
        // out of it) never rolls it off.
        let rolled_off = !self.ui_preferences.click_off()
            && entered
            && !ctx.input(|i| i.pointer.any_down())
            && hover.is_some_and(|q| !near(q));
        if rolled_off || ((self.ui_preferences.click_off() || !entered) && clicked_outside) {
            self.panel_popover = None;
        }
        Some(rect)
    }
}
