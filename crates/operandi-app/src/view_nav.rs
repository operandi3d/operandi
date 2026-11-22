//! View navigation keys and the mouse gestures that go beyond a plain
//! orbit/pan/zoom (specifications/nav-lock-hide-2026-11-07, NAV-02..18).
//! Every one acts on the active view's camera, which follows the pointer
//! (UIC-70). Pure view changes are never undo steps.
use crate::Operandi;
use crate::camera::{View, zoomed_distance};
use crate::input::NavigationKind;
use eframe::egui::{Context, Key, Modifiers, Pos2, Rect, Vec2};
use glam::DVec3;
use operandi_core::Mode;
use std::time::Instant;

/// NAV-10/11: each step halves or doubles the distance [OP].
const ZOOM_STEP: f64 = 2.;
/// NAV-13: turntable speed, radians per second [OP].
const TURN_SPEED: f64 = 0.6;

/// A view that keeps moving on its own.
#[derive(Clone, Copy, Debug)]
pub enum Motion {
    /// NAV-13 turntable.
    Turn { last: Instant },
}

impl Operandi {
    pub(super) fn nav_rect(&self) -> Rect {
        let r = self.viewport_rect;
        if r.is_finite() && r.is_positive() {
            r
        } else {
            Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.))
        }
    }

    /// NAV-10/11: one step closer or further, about the view centre. In
    /// perspective with Zoom to Surface on, the step heads for the surface
    /// at the view centre as the wheel does (fix-stuck-zoom): the eye and
    /// the orbit centre both close in, so the keys never stall 3 cm from an
    /// orbit centre hidden inside or behind the model.
    pub(super) fn zoom_step(&mut self, zoom_in: bool) {
        if self.uv_view_zoom_step(zoom_in) {
            return;
        }
        let factor = if zoom_in { 1. / ZOOM_STEP } else { ZOOM_STEP };
        let r = self.nav_rect();
        if self.zoom_toward_surface(r.center(), r, factor) {
            return;
        }
        self.camera.distance = zoomed_distance(self.camera.distance, factor);
    }

    /// Zoom to Surface for one perspective zoom step of `factor` (< 1 is
    /// closer) aimed at `p`: the eye and the orbit centre move toward the
    /// surface under `p` by the same fraction, never reaching it
    /// (`Camera::dolly_toward`). `false`, changing nothing, over empty
    /// space, in other views or with the option off; the caller then
    /// zooms about the orbit centre as before (ZP-04). Shared by the wheel,
    /// drag zoom, the Zoom In/Out keys and the 3D mouse, so every zoom
    /// follows one rule. Symmetry plays no part: the pick sees the drawn
    /// surface only.
    pub(super) fn zoom_toward_surface(&mut self, p: Pos2, r: Rect, factor: f64) -> bool {
        self.zoom_surface_point(p, r)
            .is_some_and(|hit| self.camera.dolly_toward(hit, factor))
    }

    /// One wheel event over the view at `hover`. Zoom to Surface (ZP-01..04,
    /// perspective only): toward the surface under the pointer, or at the
    /// view centre with Wheel Zoom at Mouse Cursor off. Over empty space,
    /// in other views or with the option off (ZP-04): the distance to the
    /// orbit centre changes, Wheel Zoom at Mouse Cursor keeping the
    /// pointer's spot on the plane through the orbit centre in place.
    pub(super) fn wheel_zoom(&mut self, hover: Option<Pos2>, scroll: f32, r: Rect) {
        let options = self.input_preferences.options;
        let factor = options.wheel_zoom_factor(scroll);
        let aim = if options.wheel_zoom_at_cursor {
            hover
        } else {
            Some(r.center())
        };
        if aim.is_some_and(|p| self.zoom_toward_surface(p, r, factor)) {
            return;
        }
        let before = hover.and_then(|p| self.camera.zoom_anchor(p, r));
        self.camera.distance = zoomed_distance(self.camera.distance, factor);
        if options.wheel_zoom_at_cursor
            && let Some(p) = hover
            && let (Some(a), Some(b)) = (before, self.camera.zoom_anchor(p, r))
        {
            self.camera.target += a - b;
        }
    }

    /// Zoom to Surface: the surface point under `p` that a perspective
    /// zoom moves toward. `None` with the option off, in other views,
    /// or over empty space. One pick on the cached surface per zoom step.
    pub(super) fn zoom_surface_point(&self, p: Pos2, r: Rect) -> Option<DVec3> {
        if !self.input_preferences.options.zoom_to_surface
            || self.camera.view != View::Perspective
            || !r.contains(p)
        {
            return None;
        }
        let surface = self.pick_surface();
        let (_, _, t) = surface.pick(&self.camera, p, r)?;
        let (o, d) = self.camera.ray(p, r);
        Some(o + d.normalize() * t)
    }

    /// NAV-12: the surface point under the pointer becomes the orbit
    /// centre; the zoom stays (the eye moves with the target).
    pub(super) fn recentre(&mut self) {
        let r = self.nav_rect();
        let Some(p) = self.nav_pointer.filter(|p| r.contains(*p)) else {
            self.status = "Re-centre: point at the view".into();
            return;
        };
        let surface = self.pick_surface();
        let Some((_, _, t)) = surface.pick(&self.camera, p, r) else {
            self.status = "Re-centre: nothing under the pointer".into();
            return;
        };
        let (o, d) = self.camera.ray(p, r);
        self.camera.target = o + d.normalize() * t;
    }

    /// NAV-13: start or stop the turntable.
    pub(super) fn toggle_turn(&mut self) {
        self.view_motion = match self.view_motion {
            Some(Motion::Turn { .. }) => None,
            _ => Some(Motion::Turn {
                last: Instant::now(),
            }),
        };
    }

    /// NAV-14. Says what happened: a plain orbit never tilts the view, so
    /// most of the time it is already level and nothing visibly changes.
    pub(super) fn turn_upright(&mut self) {
        let tilted =
            matches!(self.camera.view, View::Perspective | View::Orthographic)
                && self.camera.roll.rem_euclid(std::f64::consts::TAU).min(
                    std::f64::consts::TAU - self.camera.roll.rem_euclid(std::f64::consts::TAU),
                ) > 1e-6;
        self.camera.upright();
        self.status = if tilted {
            "Turn Upright: view levelled".into()
        } else {
            "Turn Upright: the view is already level (Alt+middle drag tilts it)".into()
        };
    }

    /// NAV-17/18: face the selection along its average normal; `fit` also
    /// frames it (Fit and Align Selected).
    pub(super) fn align_selected(&mut self, fit: bool) {
        let Some(normal) = self.selection_normal() else {
            self.status = "Align Selected: select polygons, edges or points first".into();
            return;
        };
        self.camera.face(normal);
        if fit {
            self.frame(true);
        }
    }

    /// The selection's average normal in world space: polygon normals
    /// (area weighted), or the normals of the polygons around the selected
    /// points/edges; in Item mode the item's polygons.
    fn selection_normal(&self) -> Option<DVec3> {
        let scene = &self.editor.scene;
        let item = scene.active()?;
        let mesh = &item.mesh;
        let s = &scene.selection;
        let faces: Vec<usize> = match s.mode {
            Mode::Polygon => s.faces.iter().map(|&f| f as usize).collect(),
            Mode::Item => (0..mesh.faces.len()).collect(),
            Mode::Vertex | Mode::Edge => {
                let points: std::collections::BTreeSet<u32> = s
                    .vertices
                    .iter()
                    .copied()
                    .chain(s.edges.iter().flat_map(|&(a, b)| [a, b]))
                    .collect();
                (0..mesh.faces.len())
                    .filter(|&f| mesh.faces[f].vertices.iter().any(|v| points.contains(v)))
                    .collect()
            }
        };
        let matrix = scene.world_matrix(item.id).ok()?;
        let mut sum = DVec3::ZERO;
        for f in faces.into_iter().filter(|&f| f < mesh.faces.len()) {
            let p: Vec<DVec3> = mesh.faces[f]
                .vertices
                .iter()
                .map(|&v| matrix.transform_point3(mesh.vertices[v as usize]))
                .collect();
            // Newell's method: twice the area times the unit normal.
            for i in 0..p.len() {
                sum += p[i].cross(p[(i + 1) % p.len()]);
            }
        }
        sum.try_normalize()
    }

    /// A navigation drag step. NAV-06 zooms about where the drag started.
    pub(super) fn navigate_gesture(
        &mut self,
        kind: NavigationKind,
        start: Pos2,
        last: Pos2,
        pos: Pos2,
        r: Rect,
    ) {
        match kind {
            NavigationKind::Zoom => {
                // Zoom to Surface: toward the surface under the press, by
                // the drag's own fraction (fix-stuck-zoom).
                let factor = self.input_preferences.options.drag_zoom_factor(pos - last);
                if factor != 1. && self.zoom_toward_surface(start, r, factor) {
                    return;
                }
                let before = self.camera.zoom_anchor(start, r);
                self.navigate_from(kind, last, pos, r);
                if let (Some(a), Some(b)) = (before, self.camera.zoom_anchor(start, r)) {
                    self.camera.target += a - b;
                }
            }
            _ => self.navigate_from(kind, last, pos, r),
        }
    }

    /// Release of a navigation drag: NAV-07 box zoom acts now.
    pub(super) fn navigation_release(
        &mut self,
        kind: NavigationKind,
        start: Pos2,
        pos: Pos2,
        r: Rect,
    ) {
        if kind == NavigationKind::BoxZoom {
            self.box_zoom(Rect::from_two_pos(start, pos), r);
        }
    }

    /// NAV-07: the dragged box fills the view.
    pub(super) fn box_zoom(&mut self, rect: Rect, r: Rect) {
        if rect.width() < 4. || rect.height() < 4. {
            return;
        }
        let Some(centre) = self.camera.zoom_anchor(rect.center(), r).or_else(|| {
            let (o, d) = self.camera.ray(rect.center(), r);
            Some(o + d.normalize() * self.camera.distance)
        }) else {
            return;
        };
        let scale = (rect.width() / r.width()).max(rect.height() / r.height()) as f64;
        self.camera.target = centre;
        self.camera.distance = (self.camera.distance * scale).clamp(0.03, 1e7);
    }

    /// Advance a turning view; any click or Esc stops it [OP].
    pub(super) fn tick_view_motion(&mut self, ctx: &Context) {
        self.nav_pointer = ctx.pointer_hover_pos().or(self.nav_pointer);
        let Some(motion) = self.view_motion else {
            return;
        };
        let (clicked, escape) = ctx.input(|i| {
            (
                i.pointer.any_pressed(),
                i.key_pressed(Key::Escape) && i.modifiers == Modifiers::NONE,
            )
        });
        if clicked || escape {
            self.view_motion = None;
            return;
        }
        let now = Instant::now();
        let Motion::Turn { last } = motion;
        let dt = now.duration_since(last).as_secs_f64().min(0.1);
        self.camera.free();
        self.camera.yaw -= TURN_SPEED * dt;
        self.view_motion = Some(Motion::Turn { last: now });
        ctx.request_repaint();
    }
}
