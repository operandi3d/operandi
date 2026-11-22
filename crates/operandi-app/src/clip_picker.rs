//! The Clips chooser: a reusable image field and the floating chooser it
//! opens (spec backdrop-clips BC-01, BC-04, BC-05 first cut). The Backdrop
//! Image control uses it; the shader tree's Image Map field can later.
//!
//! - BC-01: the field is a small thumbnail, the current clip's name
//!   ("(none)" when empty) and a drop-down arrow ([`field`]).
//! - BC-05: clicking it opens a floating chooser ([`Chooser::show`]): a top
//!   bar with Choose "<current>" (keep it and close) and an "Add Clip"
//!   drop-down whose one entry, "(load image)…", opens the file panel, adds
//!   the picture to the scene's clips and assigns it. Below, "(none)" and
//!   the scene's clips as thumbnails with names; one click assigns and
//!   closes. A fresh scene shows only "(none)".
//! - BC-04: Esc closes an open Add Clip menu first, then the chooser, with
//!   no change.
//!
//! The clips are the scene's image entries (`clip.addStill`, `image.load`
//! and the shader tree share them). Picking a listed clip reads nothing
//! again and makes no second entry. Callers turn a pick into one undo step
//! with their existing commands. Left out for now (BC-05): other folders,
//! other Add Clip entries, back/forward/up, filter, thumbnail size and
//! view buttons.
use eframe::egui::{
    self, Align2, Color32, CursorIcon, Id, Key, Modifiers, Order, Pos2, Rect, Response, Sense,
    Stroke, StrokeKind, TextStyle, TextWrapMode, TextureId, Ui, Vec2, WidgetInfo, WidgetText,
    WidgetType, pos2, vec2,
};
use operandi_core::Scene;

/// One clip the chooser lists.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Clip {
    pub id: u64,
    pub name: String,
    pub path: String,
}

/// A clip's thumbnail: its texture and pixel size (for the aspect).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Thumb {
    pub texture: TextureId,
    pub size: Vec2,
}

/// What the user picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipPick {
    /// "(none)": the field shows no clip.
    Clear,
    /// A clip the scene already has.
    Existing(u64),
    /// Add Clip > "(load image)…": choose a picture file, add it as a clip.
    Add,
}

pub(crate) const NONE: &str = "(none)";
pub(crate) const ADD_CLIP: &str = "Add Clip";
pub(crate) const LOAD_IMAGE: &str = "(load image)…";
pub(crate) const TITLE: &str = "Clips";

/// The scene's clips, in load order.
pub(crate) fn clips(scene: &Scene) -> Vec<Clip> {
    scene
        .images
        .iter()
        .map(|i| Clip {
            id: i.id,
            name: i.name.clone(),
            path: i.path.clone(),
        })
        .collect()
}

/// The field's text: the clip's name, "(none)", or "(missing)" for an id
/// the scene no longer has.
pub(crate) fn shown(current: Option<u64>, clips: &[Clip]) -> String {
    match current {
        None => NONE.to_owned(),
        Some(id) => clips
            .iter()
            .find(|c| c.id == id)
            .map_or("(missing)".to_owned(), |c| c.name.clone()),
    }
}

/// A pick that changes something: re-picking the current clip, or "(none)"
/// when the field is already empty, is no edit (and no undo step).
pub(crate) fn change(current: Option<u64>, pick: ClipPick) -> Option<ClipPick> {
    match pick {
        ClipPick::Clear if current.is_none() => None,
        ClipPick::Existing(id) if current == Some(id) => None,
        pick => Some(pick),
    }
}

/// BC-01: the image field — thumbnail, name, drop-down arrow. Click it to
/// open the chooser.
pub(crate) fn field(ui: &mut Ui, thumb: Option<Thumb>, name: &str) -> Response {
    let height = ui.spacing().interact_size.y.max(22.);
    let width = ui.available_width().max(60.);
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, ui.is_enabled(), name));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let visuals = *ui.style().interact(&response);
    let painter = ui.painter();
    painter.rect(
        rect,
        visuals.corner_radius,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        StrokeKind::Inside,
    );
    let pad = 3.;
    let square = Rect::from_min_size(rect.min + vec2(pad, pad), Vec2::splat(height - 2. * pad));
    paint_thumb(ui, square, thumb);
    let arrow = Rect::from_min_max(pos2(rect.right() - height, rect.top()), rect.max);
    paint_arrow(ui, arrow.center(), visuals.fg_stroke.color);
    let left = square.right() + 6.;
    let text_width = (arrow.left() - left).max(0.);
    let galley = WidgetText::from(name).into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        text_width,
        TextStyle::Button,
    );
    let at = pos2(left, rect.center().y - galley.size().y / 2.);
    ui.painter().galley(at, galley, visuals.text_color());
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// A thumbnail fitted inside `square` (keeping its shape), or an empty box.
fn paint_thumb(ui: &Ui, square: Rect, thumb: Option<Thumb>) {
    let painter = ui.painter();
    let line = ui.visuals().widgets.noninteractive.bg_stroke;
    painter.rect_filled(square, 2., ui.visuals().extreme_bg_color);
    if let Some(thumb) = thumb
        && thumb.size.x > 0.
        && thumb.size.y > 0.
    {
        let scale = (square.width() / thumb.size.x).min(square.height() / thumb.size.y);
        let fit = Rect::from_center_size(square.center(), thumb.size * scale);
        painter.image(
            thumb.texture,
            fit,
            Rect::from_min_max(Pos2::ZERO, pos2(1., 1.)),
            Color32::WHITE,
        );
    }
    painter.rect_stroke(square, 2., line, StrokeKind::Inside);
}

/// A small downward triangle centred on `at`.
fn paint_arrow(ui: &Ui, at: Pos2, color: Color32) {
    let s = 4.;
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            pos2(at.x - s, at.y - s / 2.),
            pos2(at.x + s, at.y - s / 2.),
            pos2(at.x, at.y + s / 2.),
        ],
        color,
        Stroke::NONE,
    ));
}

/// The floating chooser's state: open or not, and its Add Clip menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Chooser {
    open: bool,
    add_menu: bool,
}

/// Tile size in the chooser (thumbnail square; the name goes below).
const TILE: f32 = 72.;

impl Chooser {
    pub(crate) fn open(&mut self) {
        *self = Chooser {
            open: true,
            add_menu: false,
        };
    }

    pub(crate) fn close(&mut self) {
        *self = Chooser::default();
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    #[cfg(test)]
    pub(crate) fn add_menu_open(&self) -> bool {
        self.open && self.add_menu
    }

    /// BC-04: Esc closes the Add Clip menu first, then the chooser.
    pub(crate) fn escape(&mut self) {
        if self.add_menu {
            self.add_menu = false;
        } else {
            self.close();
        }
    }

    /// Draw the chooser (when open) for a field showing `current`. `tiles`
    /// are the scene's clips with their thumbnails. Returns a pick that
    /// changes the field (see [`change`]); Choose "<current>", Esc and the
    /// window's close button return nothing.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        id: Id,
        current: Option<u64>,
        tiles: &[(Clip, Option<Thumb>)],
    ) -> Option<ClipPick> {
        if !self.open {
            return None;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.escape();
            if !self.open {
                return None;
            }
        }
        let clips: Vec<Clip> = tiles.iter().map(|(c, _)| c.clone()).collect();
        let name = shown(current, &clips);
        let mut picked = None;
        let mut keep = false;
        let mut window_open = true;
        let mut add_button = None;
        egui::Window::new(TITLE)
            .id(id)
            .open(&mut window_open)
            .collapsible(false)
            .resizable(true)
            .default_width(4.5 * (TILE + 16.))
            .default_height(260.)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(format!("Choose \"{name}\""))
                        .on_hover_text("Keep the current image and close")
                        .clicked()
                    {
                        keep = true;
                    }
                    let add = drop_button(ui, ADD_CLIP, self.add_menu);
                    if add.clicked() {
                        self.add_menu = !self.add_menu;
                    }
                    add_button = Some(add.rect);
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if tile(ui, None, NONE, current.is_none(), None).clicked() {
                                picked = Some(ClipPick::Clear);
                            }
                            for (clip, thumb) in tiles {
                                if tile(
                                    ui,
                                    *thumb,
                                    &clip.name,
                                    current == Some(clip.id),
                                    Some(&clip.path),
                                )
                                .clicked()
                                {
                                    picked = Some(ClipPick::Existing(clip.id));
                                }
                            }
                        });
                    });
            });
        if self.add_menu
            && let Some(button) = add_button
        {
            let area = egui::Area::new(id.with("add-clip-menu"))
                .order(Order::Foreground)
                .fixed_pos(button.left_bottom() + vec2(0., 2.))
                .show(ctx, |ui| {
                    egui::Frame::menu(ui.style()).show(ui, |ui| {
                        ui.set_min_width(button.width().max(120.));
                        ui.selectable_label(false, LOAD_IMAGE)
                            .on_hover_text("Choose a picture file and add it as a clip")
                            .clicked()
                    })
                });
            if area.inner.inner {
                picked = Some(ClipPick::Add);
            } else if ctx.input(|i| i.pointer.any_click())
                && ctx
                    .input(|i| i.pointer.interact_pos())
                    .is_some_and(|p| !area.response.rect.contains(p) && !button.contains(p))
            {
                // A click off the menu closes it (the chooser stays).
                self.add_menu = false;
            }
        }
        if keep || !window_open {
            self.close();
            return None;
        }
        if picked.is_some() {
            // One click assigns and closes (BC-05).
            self.close();
        }
        picked.and_then(|p| change(current, p))
    }
}

/// A button with a drop-down arrow, lit while its menu is open.
fn drop_button(ui: &mut Ui, text: &str, open: bool) -> Response {
    let response = ui.add(egui::Button::new(format!("{text}     ")).selected(open));
    let color = ui.style().interact(&response).fg_stroke.color;
    paint_arrow(
        ui,
        pos2(response.rect.right() - 11., response.rect.center().y),
        color,
    );
    response
}

/// One chooser tile: a thumbnail (or empty box) with the name below.
fn tile(
    ui: &mut Ui,
    thumb: Option<Thumb>,
    name: &str,
    selected: bool,
    hover: Option<&str>,
) -> Response {
    let label_height = ui.text_style_height(&TextStyle::Small) + 4.;
    let size = vec2(TILE + 8., TILE + 8. + label_height);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, name));
    if ui.is_rect_visible(rect) {
        let visuals = ui.visuals();
        if selected {
            ui.painter()
                .rect_filled(rect, 3., visuals.selection.bg_fill.gamma_multiply(0.6));
        } else if response.hovered() {
            ui.painter()
                .rect_filled(rect, 3., visuals.widgets.hovered.weak_bg_fill);
        }
        let square = Rect::from_min_size(rect.min + vec2(4., 4.), Vec2::splat(TILE));
        paint_thumb(ui, square, thumb);
        let galley = WidgetText::from(name).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            rect.width() - 4.,
            TextStyle::Small,
        );
        let at = pos2(rect.center().x, square.bottom() + 2.);
        let place = Align2::CENTER_TOP.anchor_size(at, galley.size());
        ui.painter()
            .galley(place.min, galley, ui.visuals().text_color());
    }
    match hover {
        Some(path) => response.on_hover_text(path),
        None => response,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Context, Event, RawInput};

    fn scene_with(paths: &[&str]) -> Scene {
        let mut scene = Scene::default();
        for path in paths {
            scene.image_for_path(path, 4, 2).unwrap();
        }
        scene
    }

    #[test]
    fn lists_every_clip_in_load_order_and_a_fresh_scene_has_none() {
        let scene = scene_with(&["/p/front.png", "/p/side.jpg"]);
        let list = clips(&scene);
        assert_eq!(
            list.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            ["front", "side"]
        );
        assert_eq!(list[1].path, "/p/side.jpg");
        assert!(clips(&Scene::default()).is_empty());
    }

    #[test]
    fn bc_01_field_text_names_the_clip_or_none_or_missing() {
        let scene = scene_with(&["/p/front.png"]);
        let list = clips(&scene);
        assert_eq!(shown(None, &list), "(none)");
        assert_eq!(shown(Some(list[0].id), &list), "front");
        assert_eq!(shown(Some(9999), &list), "(missing)");
    }

    #[test]
    fn repicking_the_current_value_is_no_change() {
        assert_eq!(change(Some(3), ClipPick::Existing(3)), None);
        assert_eq!(change(None, ClipPick::Clear), None);
        assert_eq!(
            change(Some(3), ClipPick::Existing(4)),
            Some(ClipPick::Existing(4))
        );
        assert_eq!(change(Some(3), ClipPick::Clear), Some(ClipPick::Clear));
        // (load image)… always opens the file panel, even over a set clip.
        assert_eq!(change(Some(3), ClipPick::Add), Some(ClipPick::Add));
        assert_eq!(change(None, ClipPick::Add), Some(ClipPick::Add));
    }

    /// UIC-03: the entry that opens a file panel ends in "…" (Jason,
    /// 2026-11-10); the window title has none (UIC-81).
    #[test]
    fn uic_03_load_image_entry_ends_in_an_ellipsis() {
        assert_eq!(LOAD_IMAGE, "(load image)\u{2026}");
        assert!(!TITLE.contains('\u{2026}'));
        assert!(
            !ADD_CLIP.contains('\u{2026}'),
            "Add Clip opens a menu, not a panel"
        );
    }

    #[test]
    fn bc_04_escape_closes_the_add_menu_first_then_the_chooser() {
        let mut c = Chooser::default();
        c.open();
        c.add_menu = true;
        c.escape();
        assert!(c.is_open() && !c.add_menu_open());
        c.escape();
        assert!(!c.is_open());
        // Reopening starts with the menu shut.
        c.add_menu = true;
        c.open();
        assert!(c.is_open() && !c.add_menu_open());
    }

    fn frame(ctx: &Context, c: &mut Chooser, events: Vec<Event>) -> Option<ClipPick> {
        let scene = scene_with(&["/p/front.png"]);
        let tiles: Vec<(Clip, Option<Thumb>)> =
            clips(&scene).into_iter().map(|c| (c, None)).collect();
        let current = Some(tiles[0].0.id);
        let mut picked = None;
        let _ = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))),
                events,
                ..Default::default()
            },
            |ctx| picked = c.show(ctx, Id::new("test-chooser"), current, &tiles),
        );
        picked
    }

    fn escape() -> Event {
        Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn bc_04_escape_in_the_drawn_chooser_changes_nothing() {
        let ctx = Context::default();
        let mut c = Chooser::default();
        c.open();
        assert_eq!(frame(&ctx, &mut c, vec![]), None);
        assert!(c.is_open());
        c.add_menu = true;
        assert_eq!(frame(&ctx, &mut c, vec![]), None);
        assert!(c.add_menu_open(), "the menu stays open by itself");
        assert_eq!(frame(&ctx, &mut c, vec![escape()]), None);
        assert!(c.is_open() && !c.add_menu_open(), "menu closed first");
        assert_eq!(frame(&ctx, &mut c, vec![escape()]), None);
        assert!(!c.is_open(), "then the chooser");
        // A closed chooser draws nothing and leaves Esc for others.
        assert_eq!(frame(&ctx, &mut c, vec![escape()]), None);
    }
}
