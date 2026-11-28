//! Screen-space falloffs in the viewport (falloff spec 2026-11-15 batch 4):
//! drawing Screen (FO-43) and Lasso (FO-52), painting Airbrush (FO-44),
//! and the stamps their weights become (FO-Q08: fixed when drawn).
use super::*;
use glam::DVec2;
use operandi_core::falloff_screen::{self as fs, StampView};
use operandi_core::{AirbrushFalloff, ScreenFalloff};

/// Pixels of right-drag per pixel of Airbrush Size.
const BRUSH_SIZE_RATE: f64 = 1.;

/// What an Airbrush stroke does to the painted weight (MN-03.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AirbrushStroke {
    /// Plain drag: builds weight up.
    Add,
    /// Shift drag: takes weight away.
    Lower,
    /// Ctrl drag (Cmd on a Mac): puts painted weight back to none.
    Erase,
}
impl AirbrushStroke {
    /// The stroke a press with `modifiers` paints, if any.
    pub(crate) fn for_press(modifiers: Modifiers) -> Option<Self> {
        let cmd = modifiers.command;
        match (modifiers.shift, cmd, modifiers.alt) {
            (false, false, false) => Some(Self::Add),
            (true, false, false) => Some(Self::Lower),
            (false, true, false) => Some(Self::Erase),
            _ => None,
        }
    }
    /// The weight once a dab of `amount` lands on `weight`.
    pub(crate) fn apply(self, weight: f64, amount: f64) -> f64 {
        match self {
            Self::Add => (weight + amount).min(1.),
            Self::Lower => (weight - amount).max(0.),
            Self::Erase if amount > 0. => 0.,
            Self::Erase => weight,
        }
    }
}

impl Operandi {
    /// The meshes a stamp covers: the shown active meshes, or the primary.
    fn stamp_items(&self) -> Vec<u64> {
        let scene = &self.editor.scene;
        let mut ids = scene.shown_active_meshes();
        if ids.is_empty() {
            ids.extend(scene.active().map(|i| i.id));
        }
        ids
    }

    /// Work out `weights` for every stamped mesh in the current view.
    fn stamp_with(
        &self,
        r: Rect,
        weights: impl Fn(&operandi_core::Mesh, glam::DMat4, &StampView<'_>) -> Vec<f64>,
    ) -> Vec<(u64, Vec<f64>)> {
        let camera = self.camera.clone();
        let project = move |p: DVec3| {
            camera
                .project(p, r)
                .map(|(q, _)| DVec2::new((q.x - r.min.x) as f64, (q.y - r.min.y) as f64))
        };
        let (eye, _, _, forward) = self.camera.basis();
        let perspective = self.camera.view == crate::camera::View::Perspective;
        let facing = move |p: DVec3, n: DVec3| {
            if perspective {
                n.dot(eye - p) > 0.
            } else {
                n.dot(-forward) > 0.
            }
        };
        let view = StampView {
            project: &project,
            facing: &facing,
        };
        self.stamp_items()
            .into_iter()
            .filter_map(|id| {
                let item = self.editor.scene.item(id)?;
                let world = self.editor.scene.world_matrix(id).ok()?;
                Some((id, weights(&item.mesh, world, &view)))
            })
            .collect()
    }

    /// Replace layer `layer`'s stamps with `stamps`.
    fn set_stamps(&mut self, layer: usize, stamps: Vec<(u64, Vec<f64>)>) {
        let mut all: std::collections::BTreeMap<(usize, u64), Vec<f64>> = self
            .editor
            .falloff_stamps
            .iter()
            .filter(|((l, _), _)| *l != layer)
            .map(|(k, v)| (*k, v.as_ref().clone()))
            .collect();
        for (id, w) in stamps {
            all.insert((layer, id), w);
        }
        self.run(Command::FalloffStamps(all));
    }

    /// The stack's first layer of a screen-space kind, with its index.
    pub(crate) fn screen_layer(&self) -> Option<(usize, Falloff)> {
        let first = self.editor.spatial_falloff?;
        std::iter::once(first)
            .chain(self.editor.added_falloffs.iter().map(|a| a.0))
            .enumerate()
            .find(|(_, f)| f.stamped())
    }

    /// FO-43/52: work Screen and Lasso stamps out again after their options
    /// change (Lasso from the outline drawn last).
    pub(crate) fn restamp_screen_falloffs(&mut self) {
        let r = self.viewport_rect;
        let Some((layer, f)) = self.screen_layer() else {
            return;
        };
        match f {
            Falloff::Screen(s) => {
                let stamps = self.stamp_with(r, |m, w, v| fs::screen_weights(m, w, v, s));
                self.set_stamps(layer, stamps);
            }
            // FO-54: through the selected UV map, or laid over the view.
            Falloff::Image(f) => {
                let Some((_, image)) = self.falloff_image.clone() else {
                    return;
                };
                let stamps = if f.use_uv {
                    self.stamp_items()
                        .into_iter()
                        .filter_map(|id| {
                            let mesh = &self.editor.scene.item(id)?.mesh;
                            let map = operandi_core::uv_maps::current_map(mesh)?;
                            Some((
                                id,
                                operandi_core::falloff_pattern::image_uv_weights(
                                    mesh, &map, &image, f,
                                ),
                            ))
                        })
                        .collect()
                } else {
                    let size = DVec2::new(r.width() as f64, r.height() as f64);
                    self.stamp_with(r, |m, w, v| {
                        m.vertices
                            .iter()
                            .map(|&p| {
                                (v.project)(w.transform_point3(p)).map_or(0., |q| {
                                    let uv = operandi_core::falloff_pattern::image_screen_uv(
                                        &image, q, size,
                                    );
                                    f.weight_at(&image, uv)
                                })
                            })
                            .collect()
                    })
                };
                self.set_stamps(layer, stamps);
            }
            Falloff::Lasso(l) if self.lasso_outline.len() >= 3 => {
                let outline = self.lasso_outline.clone();
                let stamps = self.stamp_with(r, |m, w, v| {
                    fs::lasso_weights(m, w, v, &outline, l.soft_border)
                });
                self.set_stamps(layer, stamps);
            }
            _ => {}
        }
    }

    /// Whether a press should draw (Screen, Lasso: armed) or paint
    /// (Airbrush: always) the screen-space falloff.
    pub(crate) fn screen_falloff_press(&self) -> Option<Falloff> {
        let (_, f) = self.screen_layer()?;
        let tool_ok = self
            .editor
            .session
            .live
            .as_ref()
            .is_none_or(|l| l.tool.card().layers.falloff);
        match f {
            Falloff::Airbrush(_) if tool_ok => Some(f),
            Falloff::Screen(_) | Falloff::Lasso(_) if tool_ok && self.falloff_draw_armed => Some(f),
            _ => None,
        }
    }

    pub(crate) fn begin_screen_falloff(
        &mut self,
        f: Falloff,
        pos: Pos2,
        r: Rect,
        stroke: AirbrushStroke,
    ) -> input::GestureKind {
        let at = DVec2::new((pos.x - r.min.x) as f64, (pos.y - r.min.y) as f64);
        self.falloff_draw_armed = false;
        match f {
            Falloff::Airbrush(_) => {
                self.status = match stroke {
                    AirbrushStroke::Add => "Airbrush: drag over the mesh to build up the falloff",
                    AirbrushStroke::Lower => "Airbrush: drag to lower the painted falloff",
                    AirbrushStroke::Erase => "Airbrush: drag to erase the painted falloff",
                }
                .into();
                self.airbrush_dab(at, r, stroke);
                input::GestureKind::Airbrush(stroke)
            }
            _ => {
                self.lasso_outline.clear();
                self.status = "Drag to draw the falloff; release to accept".into();
                input::GestureKind::ScreenFalloffDraw {
                    start: at,
                    path: vec![at],
                }
            }
        }
    }

    /// FO-43/52 drag: Screen grows from its centre; Lasso follows the
    /// style's outline.
    pub(crate) fn screen_falloff_drag(
        &mut self,
        start: DVec2,
        path: &mut Vec<DVec2>,
        pos: Pos2,
        r: Rect,
    ) {
        let at = DVec2::new((pos.x - r.min.x) as f64, (pos.y - r.min.y) as f64);
        if path.last().is_none_or(|l| l.distance(at) >= 2.) {
            path.push(at);
        }
        let Some((layer, f)) = self.screen_layer() else {
            return;
        };
        match f {
            Falloff::Screen(s) => {
                let next = ScreenFalloff {
                    center: start,
                    size: start.distance(at),
                    ..s
                };
                self.replace_falloff_layer(layer, Falloff::Screen(next));
                self.restamp_screen_falloffs();
            }
            Falloff::Lasso(l) => {
                self.lasso_outline = fs::lasso_outline(l.style, start, at, path);
            }
            _ => {}
        }
    }

    /// Lasso is stamped once the drag ends.
    pub(crate) fn screen_falloff_release(&mut self) {
        if matches!(self.screen_layer(), Some((_, Falloff::Lasso(_)))) {
            self.restamp_screen_falloffs();
        }
        self.status = "Falloff drawn".into();
    }

    /// FO-44: one brush pass at `at` adds to the painted weights; the tool
    /// re-applies with them.
    pub(crate) fn airbrush_dab(&mut self, at: DVec2, r: Rect, stroke: AirbrushStroke) {
        let Some((layer, Falloff::Airbrush(a))) = self.screen_layer() else {
            return;
        };
        let a = AirbrushFalloff { center: at, ..a };
        let before = self.editor.falloff_stamps.clone();
        let stamps = self.stamp_with(r, |m, w, v| {
            let id_weights = vec![0.; m.vertices.len()];
            let mut weights = id_weights;
            fs::airbrush_dab(&mut weights, m, w, v, a, at);
            weights
        });
        let stamps = stamps
            .into_iter()
            .map(|(id, add)| {
                let mut w = before
                    .get(&(layer, id))
                    .map(|w| w.as_ref().clone())
                    .filter(|w| w.len() == add.len())
                    .unwrap_or_else(|| vec![0.; add.len()]);
                for (w, d) in w.iter_mut().zip(add) {
                    *w = stroke.apply(*w, d);
                }
                (id, w)
            })
            .collect();
        self.set_stamps(layer, stamps);
    }

    pub(crate) fn airbrush_drag(&mut self, pos: Pos2, r: Rect, stroke: AirbrushStroke) {
        let at = DVec2::new((pos.x - r.min.x) as f64, (pos.y - r.min.y) as f64);
        self.airbrush_dab(at, r, stroke);
    }

    /// MN-03.2: a transform tool's right-drag sets up the falloff in use,
    /// when the tool's hauls row leaves the right button free. `None`
    /// keeps today's right-drag (no falloff, or one with nothing to set
    /// this way).
    pub(crate) fn right_drag_falloff_setup(
        &mut self,
        pos: Pos2,
        r: Rect,
    ) -> Option<input::GestureKind> {
        let tool = self.editor.session.live.as_ref()?.tool;
        if !tool.transforms_items()
            || !tool.card().layers.falloff
            || operandi_core::haul::claims(tool, operandi_core::haul::Button::Right)
        {
            return None;
        }
        match self.editor.spatial_falloff? {
            Falloff::Linear(_) | Falloff::Radial(_) | Falloff::Cylinder(_) => {
                Some(self.begin_falloff_draw(pos, r))
            }
            Falloff::Screen(_) => {
                let at = DVec2::new((pos.x - r.min.x) as f64, (pos.y - r.min.y) as f64);
                self.status = "Drag to size the Screen falloff; release to accept".into();
                Some(input::GestureKind::ScreenFalloffDraw {
                    start: at,
                    path: vec![at],
                })
            }
            _ => None,
        }
    }

    /// MN-03.3: with the Screen falloff on, a non-transform edit tool that
    /// can start a fresh edit (Bevel, Twist Bevel) follows the transform
    /// rule: a plain drag starts fresh, Shift continues the edit. False
    /// when the rule doesn't apply (no Screen falloff, or another tool).
    pub(crate) fn screen_fresh_rule(&self) -> bool {
        self.editor.session.live.as_ref().is_some_and(|live| {
            let card = live.tool.card();
            card.continues && card.layers.falloff && !live.tool.transforms_items()
        }) && matches!(self.screen_layer(), Some((_, Falloff::Screen(_))))
    }

    /// Whether the open tool preset is Soft Drag (Move + Screen falloff).
    pub(crate) fn soft_drag_open(&self) -> bool {
        self.editor.session.preset.as_ref().is_some_and(|p| {
            p.row().falloff_kind() == Some(operandi_core::tool_preset::FalloffKind::Screen)
        })
    }

    /// Soft Drag: a press centres its circle at `pos` and stamps it there.
    pub(crate) fn soft_drag_press(&mut self, pos: Pos2, r: Rect) {
        if !self.soft_drag_open() {
            return;
        }
        let Some((layer, Falloff::Screen(s))) = self.screen_layer() else {
            return;
        };
        let center = DVec2::new((pos.x - r.min.x) as f64, (pos.y - r.min.y) as f64);
        self.replace_falloff_layer(layer, Falloff::Screen(ScreenFalloff { center, ..s }));
        self.restamp_screen_falloffs();
    }

    /// FO-44: right-drag resizes the brush; under Soft Drag it resizes the
    /// screen circle (DEFORM: "right-drag sets the circle size").
    pub(crate) fn begin_airbrush_size(&mut self) -> Option<input::GestureKind> {
        if self.soft_drag_open()
            && let Some((layer, Falloff::Screen(s))) = self.screen_layer()
        {
            self.status = format!(
                "Soft Drag circle: {:.0} px · drag right for more, left for less",
                s.size
            );
            return Some(input::GestureKind::AirbrushSize {
                layer,
                start: s.size,
            });
        }
        let Some((layer, Falloff::Airbrush(a))) = self.screen_layer() else {
            return None;
        };
        self.status = format!(
            "Airbrush Size: {:.0} px · drag right for more, left for less",
            a.size
        );
        Some(input::GestureKind::AirbrushSize {
            layer,
            start: a.size,
        })
    }

    pub(crate) fn airbrush_size_drag(&mut self, layer: usize, start: f64, from: Pos2, to: Pos2) {
        let size = (start + (to.x - from.x) as f64 * BRUSH_SIZE_RATE).max(1.);
        if let Some((at, Falloff::Screen(s))) = self.screen_layer()
            && at == layer
            && s.size != size
        {
            self.replace_falloff_layer(layer, Falloff::Screen(ScreenFalloff { size, ..s }));
            self.restamp_screen_falloffs();
            self.status =
                format!("Soft Drag circle: {size:.0} px · drag right for more, left for less");
            return;
        }
        if let Some((at, Falloff::Airbrush(a))) = self.screen_layer()
            && at == layer
            && a.size != size
        {
            self.replace_falloff_layer(layer, Falloff::Airbrush(AirbrushFalloff { size, ..a }));
            self.status =
                format!("Airbrush Size: {size:.0} px · drag right for more, left for less");
        }
    }

    /// Swap layer `layer` of the stack for `next`, keeping the rest.
    pub(crate) fn replace_falloff_layer(&mut self, layer: usize, next: Falloff) {
        if layer == 0 {
            self.run(Command::SpatialFalloff(Some(next)));
        } else {
            let mut added = self.editor.added_falloffs.clone();
            if let Some(slot) = added.get_mut(layer - 1) {
                slot.0 = next;
                self.run(Command::AddedFalloffs(added));
            }
        }
    }

    /// The Screen circle, the Lasso outline and the Airbrush brush.
    pub(crate) fn draw_screen_falloff(
        &self,
        painter: &egui::Painter,
        hover: Option<Pos2>,
        r: Rect,
    ) {
        let Some((_, f)) = self.screen_layer() else {
            return;
        };
        let to_screen = |p: DVec2| Pos2::new(r.min.x + p.x as f32, r.min.y + p.y as f32);
        let stroke = Stroke::new(1.5_f32, palette::selection());
        match f {
            Falloff::Screen(s) if s.size > 0. => {
                painter.circle_stroke(to_screen(s.center), s.size as f32, stroke);
            }
            Falloff::Lasso(_) if self.lasso_outline.len() >= 2 => {
                let mut pts: Vec<Pos2> = self.lasso_outline.iter().map(|&p| to_screen(p)).collect();
                pts.push(pts[0]);
                painter.add(egui::Shape::line(
                    pts,
                    Stroke::new(1_f32, palette::selection().gamma_multiply(0.7)),
                ));
            }
            Falloff::Airbrush(a) => {
                if let Some(h) = hover.filter(|h| r.contains(*h)) {
                    painter.circle_stroke(h, a.size as f32, stroke);
                }
            }
            _ => {}
        }
    }

    /// FO-54 Browse: choose the Image falloff's picture.
    pub(crate) fn choose_falloff_image(&mut self) {
        let folder = self
            .falloff_image
            .as_ref()
            .and_then(|(p, _)| std::path::Path::new(p).parent().map(|p| p.to_path_buf()))
            .unwrap_or_default();
        if !self.choose(
            crate::file_chooser::Choose::Image { folder },
            crate::file_chooser::Purpose::FalloffImage,
        ) {
            self.status = "No file panel is available to choose a picture".into();
        }
    }

    /// The chosen picture, read in grey, then stamped.
    pub(crate) fn falloff_image_chosen(&mut self, path: std::path::PathBuf) {
        match crate::image_decode::open(&path) {
            Ok(img) => {
                let grey = img.to_luma32f();
                let (width, height) = (grey.width() as usize, grey.height() as usize);
                self.falloff_image = Some((
                    path.to_string_lossy().into_owned(),
                    std::sync::Arc::new(operandi_core::GrayImage {
                        width,
                        height,
                        pixels: grey.into_raw(),
                    }),
                ));
                self.restamp_screen_falloffs();
                self.status = format!("Image falloff: {}", path.display());
            }
            Err(e) => self.status = format!("Couldn't read {}: {e}", path.display()),
        }
    }
}
