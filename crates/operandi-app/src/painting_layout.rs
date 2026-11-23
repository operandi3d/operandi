//! Painting stage 6 in the app (PNT-210..212): the Paint layout, the
//! Sculpt/Paint toolbox set and the brush preset strip. Painting builds
//! only.
//!
//! - The Paint layout (`assets/layouts-painting/paint.json`) sits at
//!   switcher position 90, unstarred. Its open command
//!   `operandi.paint.layout` turns on the Sculpt/Paint set for that layout
//!   and puts its 3D view in the Texture shading style.
//! - The set's tabs, in order: Sculpt Tools (the Sculpt tab), Paint Tools
//!   (the Paint tab), Vertex Map Tools and Utilities. Hair and Particle
//!   Tools are left out (out of scope).
//! - The brush preset strip runs along the bottom of the 3D view while the
//!   set shows: one thumbnail per saved brush preset; a left-click applies
//!   it (`operandi.paint.applyPreset`).
use super::*;
use crate::painting::word;
use crate::toolbox::{Category, ToolSet};
use operandi_core::command_registry::{self as table, Area, ArgType, Unit};
use operandi_core::command_runtime::{Call, CommandState, HostHandler};
use operandi_core::paint_tips::TipChoice;

/// The strip's height in points.
pub(crate) const STRIP_HEIGHT: f32 = 64.;
/// Sculpt commands Operandi doesn't have (PNT-212).
pub(crate) const NO_SCULPT_COMMAND: &str = "Not in Operandi's sculpting yet";

/// One thumbnail: the preset's name and its tip's look.
#[derive(Clone, Debug)]
pub(crate) struct StripItem {
    pub(crate) name: String,
    /// One of Operandi's starter brushes (PNT-245), not a saved preset.
    pub(crate) starter: bool,
    choice: TipChoice,
    shape: operandi_core::brush_engine::TipShape,
}

type State = fn(&Operandi, &Call<'_>) -> CommandState;

const NAME_ARGS: &[table::Arg] = &[table::opt("name", ArgType::Text, Unit::None, None)];

const fn cmd(name: &'static str, help: &'static str) -> table::Entry {
    table::script(name, Area::Tools, help).operandi()
}

pub(crate) fn commands() -> Vec<(table::Entry, HostHandler<Operandi>, State)> {
    vec![
        (
            cmd("operandi.paint.layout", "The Paint layout's open command (PNT-210): the Sculpt/Paint toolbox set on the Paint Tools tab, and the 3D view in Texture shading.").ui(),
            |a, _| a.open_paint_set(),
            |a, _| CommandState::lit(a.paint_set_tabs().is_some()),
        ),
        (
            cmd("operandi.paint.applyStarter", "Applies one of Operandi's starter brushes (PNT-245) to the current paint tool or sculpt brush: its tip, values and nozzle; the tool stays.")
                .args(NAME_ARGS)
                .example("operandi.paint.applyStarter name:Jitter"),
            |a, c| a.apply_starter(&word(c.rest, "name", 0).unwrap_or_default()),
            |_, _| CommandState::default(),
        ),
        (
            cmd("operandi.paint.unavailable", "A Sculpt/Paint Utilities button for a sculpt command Operandi doesn't have (PNT-212).").args(NAME_ARGS),
            |a, c| {
                a.status = format!(
                    "{}: {NO_SCULPT_COMMAND}",
                    word(c.rest, "name", 0).unwrap_or_default()
                )
            },
            |_, _| CommandState::unless(Some(NO_SCULPT_COMMAND)),
        ),
    ]
}

pub(crate) fn register(r: &mut crate::command_handlers::Commands) {
    for (entry, run, state) in commands() {
        let name = entry.name;
        r.add_host_command(entry, run)
            .unwrap_or_else(|e| panic!("{e}"));
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
}

impl Operandi {
    /// `operandi.paint.layout`: the set for the layout showing.
    fn open_paint_set(&mut self) {
        self.painting.set_layout = Some(self.layouts.current.clone());
        self.select_tool_set(ToolSet::Brush);
        // Ready to paint: Paintbrush when no paint tool is on.
        if self.painting.brush.tool.is_none() {
            self.activate_paint(operandi_core::paint_stroke::PaintTool::Paintbrush);
        }
        self.painting.brush.tab_shown = true;
        self.show_toolbox_tab(Category::Paint);
        // PNT-210: paint shows in the view.
        self.display
            .set_style(crate::view_display::Style::Texture, false);
        self.status = "Paint layout: Sculpt/Paint tools".into();
    }

    /// The Brush set's tabs (the old Sculpt/Paint set), regardless of
    /// which layout is showing.
    pub(crate) fn paint_set_tabs(&self) -> Option<Vec<Category>> {
        (self.layouts.tool_set == ToolSet::Brush).then(|| ToolSet::Brush.tabs())
    }

    /// The set's names for its tabs (PNT-211).
    pub(crate) fn paint_set_label(&self, c: Category) -> Option<&'static str> {
        self.paint_set_tabs()?;
        Some(match c {
            Category::Sculpt => "Sculpt Tools",
            Category::Paint => "Paint Tools",
            Category::VertexMaps => "Vertex Map Tools",
            Category::PaintUtilities => "Utilities",
            _ => return None,
        })
    }

    /// A starter brush onto the active tool (PNT-244, 245). A crumb brush's
    /// picture becomes a scene image the first time.
    pub(crate) fn apply_starter(&mut self, name: &str) {
        let Some(starter) = operandi_core::paint_starters::starter_brushes()
            .into_iter()
            .find(|s| s.name.eq_ignore_ascii_case(name.trim()))
        else {
            self.status = format!("No starter brush {name}");
            return;
        };
        let mut brush = starter.brush;
        if let Some(picture) = starter.picture {
            let existing = self
                .editor
                .scene
                .images
                .iter()
                .find(|i| {
                    i.name == starter.name && self.editor.scene.paint.buffers.contains_key(&i.id)
                })
                .map(|i| i.id);
            let id = match existing {
                Some(id) => id,
                None => {
                    let px = picture();
                    if !self.run_ok(Command::Paint(operandi_core::paint::PaintEdit::NewImage(
                        operandi_core::paint::NewImage {
                            name: Some(starter.name.into()),
                            resolution: px.width,
                            format: px.format,
                            float: true,
                            color: None,
                            ..Default::default()
                        },
                    ))) {
                        return;
                    }
                    let Some(id) = self.editor.paint.last_image else {
                        return;
                    };
                    self.editor
                        .scene
                        .paint
                        .buffers
                        .insert(id, std::sync::Arc::new(px));
                    id
                }
            };
            brush.image.image = Some(id);
        }
        self.apply_brush(brush);
        self.status = format!("Starter brush {}", starter.name);
    }

    /// The saved presets for the strip, read at most every two seconds.
    fn strip_items(&mut self) -> Vec<StripItem> {
        let fresh = self
            .painting
            .strip
            .as_ref()
            .is_some_and(|(at, _)| at.elapsed() < std::time::Duration::from_secs(2));
        if !fresh {
            let folder = crate::painting_brush::brush_folder(self);
            // The starter brushes first (PNT-245), then the saved ones.
            let starters = operandi_core::paint_starters::starter_brushes()
                .into_iter()
                .map(|s| StripItem {
                    name: s.name.into(),
                    starter: true,
                    choice: s.brush.choice,
                    shape: s.brush.tip().shape,
                });
            let saved = crate::painting_brush::preset_names(self)
                .into_iter()
                .filter_map(|name| {
                    let path = folder.join(format!(
                        "{name}.{}",
                        operandi_core::paint_tips::BRUSH_PRESET_EXTENSION
                    ));
                    let p = operandi_core::paint_tips::BrushPreset::load(&path).ok()?;
                    Some(StripItem {
                        name,
                        starter: false,
                        choice: p.brush.choice,
                        shape: p.brush.tip().shape,
                    })
                });
            let items = starters.chain(saved).collect();
            self.painting.strip = Some((std::time::Instant::now(), items));
        }
        self.painting
            .strip
            .as_ref()
            .map(|(_, i)| i.clone())
            .unwrap_or_default()
    }

    /// Where the strip goes: the bottom of the 3D view area.
    pub(crate) fn strip_rect(&self) -> Option<Rect> {
        // The Paint layout's viewport strip remains part of that workflow
        // (PNT-210). Choosing a toolbox set never changes viewport chrome.
        if self.painting.set_layout.as_deref() != Some(self.layouts.current.as_str()) {
            return None;
        }
        let v = self.viewport_rect;
        (v.height() > STRIP_HEIGHT * 2.)
            .then(|| Rect::from_min_max(pos2(v.left(), v.bottom() - STRIP_HEIGHT), v.max))
    }

    /// The brush preset strip (PNT-210), once a frame.
    pub(crate) fn preset_strip(&mut self, ctx: &Context) {
        let Some(rect) = self.strip_rect() else {
            return;
        };
        let items = self.strip_items();
        let mut run = None;
        egui::Area::new(egui::Id::new("paint-preset-strip"))
            .fixed_pos(rect.min)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let (frame, _) = ui.allocate_exact_size(rect.size(), egui::Sense::hover());
                ui.painter()
                    .rect_filled(frame, 0., egui::Color32::from_black_alpha(170));
                if items.is_empty() {
                    ui.painter().text(
                        frame.center(),
                        egui::Align2::CENTER_CENTER,
                        "Saved brushes appear here (Save Brush Preset on Paint Tools)",
                        FontId::proportional(12.),
                        egui::Color32::from_gray(200),
                    );
                    return;
                }
                let size = STRIP_HEIGHT - 8.;
                for (k, item) in items.iter().enumerate() {
                    let cell = Rect::from_min_size(
                        frame.min + vec2(4. + k as f32 * (size + 6.), 4.),
                        vec2(size, size),
                    );
                    if cell.right() > frame.right() {
                        break;
                    }
                    let line = if item.starter {
                        format!("operandi.paint.applyStarter name:\"{}\"", item.name)
                    } else {
                        format!("operandi.paint.applyPreset name:\"{}\"", item.name)
                    };
                    let r = ui
                        .interact(
                            cell,
                            egui::Id::new(("paint-strip", &item.name)),
                            egui::Sense::click(),
                        )
                        .on_hover_text(format!("{} · {line}", item.name));
                    draw_thumbnail(ui.painter(), cell, item, r.hovered());
                    if r.clicked() {
                        run = Some(line);
                    }
                }
            });
        if let Some(line) = run {
            self.execute(&line);
        }
    }
}

/// A preset's thumbnail: its tip's falloff as a soft disc, the tip kind's
/// initial and the name below.
fn draw_thumbnail(p: &Painter, cell: Rect, item: &StripItem, hovered: bool) {
    let bg = if hovered {
        egui::Color32::from_gray(70)
    } else {
        egui::Color32::from_gray(45)
    };
    p.rect_filled(cell, 4., bg);
    let centre = cell.center() - vec2(0., 7.);
    let radius = cell.width() * 0.3;
    // Rings from the outside in, each the tip's weight there.
    let steps = 12;
    for k in (0..steps).rev() {
        let t = (k as f64 + 0.5) / steps as f64;
        let w = item.shape.weight(t).clamp(0., 1.);
        let grey = (40. + 215. * w) as u8;
        p.circle_filled(
            centre,
            radius * (k as f32 + 1.) / steps as f32,
            egui::Color32::from_gray(grey),
        );
    }
    p.text(
        centre,
        egui::Align2::CENTER_CENTER,
        &item.choice.label()[..1],
        FontId::proportional(11.),
        egui::Color32::from_rgb(255, 212, 112),
    );
    let name: String = item.name.chars().take(9).collect();
    p.text(
        pos2(cell.center().x, cell.bottom() - 7.),
        egui::Align2::CENTER_CENTER,
        name,
        FontId::proportional(9.),
        egui::Color32::from_gray(220),
    );
}
