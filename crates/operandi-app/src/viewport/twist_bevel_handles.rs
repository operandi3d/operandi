//! Twist Bevel's handles (twist-bevel-2026-11-15 section 4), on the first
//! group's tube: the Shift arrow along the cap's tangent, the Inset square
//! out sideways in the cap's plane and the twist ring around the cap; the
//! bend arrow along the bend direction and the Bend Direction ring around
//! the base. The cap handles ride on the current shape. Each drag edits its
//! property for every group at once.
use super::*;
use glam::DMat4;
use operandi_core::{TubeFrame, TwistAxis, TwistBevelOptions, TwistBevelPipe};

// Handle ids live with the hover tips that name them (view/handle_ids.rs).
pub(crate) use crate::handle_ids::{TB_BEND, TB_BEND_DIRECTION, TB_INSET, TB_SHIFT, TB_TWIST};

/// On-screen sizes at Handle Scale 1.
const ARROW_PIXELS: f32 = 72.;
const INSET_PIXELS: f32 = 48.;
const TWIST_RING_PIXELS: f64 = 40.;
const BEND_ARROW_PIXELS: f32 = 56.;
const BEND_RING_PIXELS: f64 = 64.;
/// Points per ring.
const RING_SEGMENTS: usize = 48;

/// The first group's tube as the handles see it, in world space.
pub(crate) struct SpineView {
    pub frame: TubeFrame,
    pub world: DMat4,
    /// Item scale (world distance per local unit).
    pub scale: f64,
}
impl SpineView {
    fn point(&self, p: DVec3) -> DVec3 {
        self.world.transform_point3(p)
    }
    fn vector(&self, d: DVec3) -> DVec3 {
        self.world.transform_vector3(d).normalize_or_zero()
    }
    /// Base centre C.
    pub fn base(&self) -> DVec3 {
        self.point(self.frame.center)
    }
    /// Cap centre P(1).
    pub fn cap(&self) -> DVec3 {
        self.point(self.frame.spine(1.))
    }
    /// The cap's tangent T(1).
    pub fn tangent(&self) -> DVec3 {
        self.vector(self.frame.tangent(1.))
    }
    pub fn normal(&self) -> DVec3 {
        self.vector(self.frame.normal)
    }
    pub fn bend_dir(&self) -> DVec3 {
        self.vector(self.frame.bend_dir)
    }
    /// Sideways in the cap's plane: the bend direction carried to the cap.
    pub fn cap_side(&self) -> DVec3 {
        self.vector(self.frame.ring_turn(1.) * self.frame.bend_dir)
    }
    /// World direction D with P(1) − C = L·D for a spine of local length L
    /// (the bend fixed): Shift snapping solves for L along it.
    pub fn reach(&self) -> DVec3 {
        let f = &self.frame;
        let d = if f.bend.abs() < 1e-9 {
            f.normal
        } else {
            f.normal * (f.bend.sin() / f.bend) + f.bend_dir * ((1. - f.bend.cos()) / f.bend)
        };
        self.world.transform_vector3(d)
    }
}

impl Operandi {
    /// The first group's base frame (C, N, side axis, W) for Group on and
    /// off, once per base scene: only the properties change it during a
    /// drag.
    fn twist_bevel_base_frames(&self, base: &Scene) -> [Option<TubeFrame>; 2] {
        self.twist_bevel_frame_memo.get(base, || {
            let Some(item) = base.active() else {
                return [None, None];
            };
            let selected: Vec<u32> = if base.selection.faces.is_empty() {
                (0..item.mesh.faces.len() as u32).collect()
            } else {
                base.selection.faces.clone()
            };
            // The side axis is world X on a rotated item too, as in the
            // geometry (preview_twist_bevel).
            let to_local = base
                .world_matrix(item.id)
                .map(|m| m.inverse())
                .unwrap_or(DMat4::IDENTITY);
            let pipe = TwistBevelPipe {
                world_axes: glam::DMat3::from_cols(
                    to_local.transform_vector3(DVec3::X),
                    to_local.transform_vector3(DVec3::Y),
                    to_local.transform_vector3(DVec3::Z),
                ),
                ..Default::default()
            };
            [false, true].map(|group| {
                let o = TwistBevelOptions {
                    group_polygons: group,
                    ..Default::default()
                };
                operandi_core::twist_bevel_frames(&item.mesh, &selected, &o, &pipe)
                    .ok()
                    .and_then(|(frames, _)| frames.first().copied())
            })
        })
    }

    /// The live Twist Bevel's first tube with `params`' values (weight 1).
    pub(crate) fn twist_bevel_spine(&self, params: &Parameters) -> Option<SpineView> {
        let l = self
            .editor
            .session
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::TwistBevel)?;
        let item = l.base.active()?;
        let world = l.base.world_matrix(item.id).ok()?;
        let scale = operandi_core::scene::uniform_scale(world).max(1e-12);
        let o = params.twist_bevel.clamped();
        let base = self.twist_bevel_base_frames(&l.base)[usize::from(o.group_polygons)]?;
        let bend_dir = glam::DQuat::from_axis_angle(base.normal, o.bend_direction.to_radians())
            * base.bend_dir;
        let shift = o.shift / scale;
        let frame = TubeFrame {
            bend_dir,
            up: base.normal.cross(bend_dir).normalize_or_zero(),
            shift,
            inset: o.inset / scale,
            rotation: o.rotation.to_radians(),
            bend: if shift == 0. { 0. } else { o.bend.to_radians() },
            profile: o.profile,
            profile_bias: o.profile_bias,
            twist_axis: (o.rotation_axis == TwistAxis::ActionAxis).then(|| {
                // Falls back to world Y, as the geometry does.
                let to_local = world.inverse();
                l.action_frame
                    .map(|f| to_local.transform_vector3(self.editor.action_axis(f)))
                    .unwrap_or_else(|| to_local.transform_vector3(DVec3::Y))
                    .normalize_or(base.normal)
            }),
            ..base
        };
        Some(SpineView {
            frame,
            world,
            scale,
        })
    }

    /// World radius of a ring of `pixels` on screen at `center`.
    fn spine_ring_radius(&self, center: DVec3, pixels: f64, r: Rect) -> Option<f64> {
        let (_, depth) = self.camera.project(center, r)?;
        let pixels = pixels * self.handle_scale_at(center) as f64;
        Some(
            pixels
                * if self.camera.view == View::Perspective {
                    depth
                } else {
                    self.camera.distance
                }
                / (r.height() as f64 * 0.9),
        )
    }

    /// The screen polyline of the twist ring (around the cap, square to
    /// its tangent) or the Bend Direction ring (around the base, in its
    /// plane).
    pub(super) fn spine_ring(&self, id: usize, r: Rect) -> Vec<Pos2> {
        let Some(l) = &self.editor.session.live else {
            return Vec::new();
        };
        let Some(view) = self.twist_bevel_spine(&l.params) else {
            return Vec::new();
        };
        let (center, axis, pixels) = if id == TB_TWIST {
            (view.cap(), view.tangent(), TWIST_RING_PIXELS)
        } else {
            (view.base(), view.normal(), BEND_RING_PIXELS)
        };
        let Some(radius) = self.spine_ring_radius(center, pixels, r) else {
            return Vec::new();
        };
        let u = axis.any_orthonormal_vector();
        let v = axis.cross(u);
        (0..=RING_SEGMENTS)
            .filter_map(|i| {
                let a = i as f64 * std::f64::consts::TAU / RING_SEGMENTS as f64;
                self.camera
                    .project(center + (u * a.cos() + v * a.sin()) * radius, r)
                    .map(|p| p.0)
            })
            .collect()
    }

    /// The handles of the live Twist Bevel, as `handles()` entries.
    pub(super) fn twist_bevel_handles(
        &self,
        l: &Live,
        r: Rect,
    ) -> Vec<(usize, Pos2, Pos2, String)> {
        let Some(view) = self.twist_bevel_spine(&l.params) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let arrow = |from: DVec3, direction: DVec3, pixels: f32| -> Option<(Pos2, Pos2)> {
            let s = self.handle_scale_at(from);
            let (a, v) = self.handle_arrow(from, direction, pixels * s, r)?;
            Some((a, a + v))
        };
        let cap = view.cap();
        if let Some((a, b)) = arrow(cap, view.tangent(), ARROW_PIXELS) {
            out.push((TB_SHIFT, a, b, "Shift".into()));
        }
        if let Some((a, b)) = arrow(cap, view.cap_side(), INSET_PIXELS) {
            out.push((TB_INSET, a, b, "Inset".into()));
        }
        for (id, label) in [
            (TB_TWIST, "Rotation"),
            (TB_BEND_DIRECTION, "Bend Direction"),
        ] {
            let ring = self.spine_ring(id, r);
            if let Some(&last) = ring.last() {
                let center = if id == TB_TWIST { cap } else { view.base() };
                if let Some((c, _)) = self.camera.project(center, r) {
                    out.push((id, c, last, label.into()));
                }
            }
        }
        if let Some((a, b)) = arrow(view.base(), view.bend_dir(), BEND_ARROW_PIXELS) {
            out.push((TB_BEND, a, b, "Bend".into()));
        }
        out
    }

    /// The Twist Bevel handle under `p`: an arrow's tip or shaft, or a
    /// ring's line. `None` when two overlap at the press.
    pub(super) fn twist_bevel_hit(&self, p: Pos2, r: Rect) -> Option<usize> {
        let handles = self.handles(r);
        let tip = Vec2::splat(self.handle_preferences.point_hit_size);
        let mut hits: Vec<usize> = handles
            .iter()
            .filter(|h| matches!(h.0, TB_SHIFT | TB_INSET | TB_BEND))
            .filter(|(_, a, b, _)| {
                Rect::from_center_size(*b, tip).contains(p)
                    || input::segment_distance(p, *a, *b)
                        <= self.handle_preferences.line_hit_width / 2.
            })
            .map(|h| h.0)
            .collect();
        if hits.is_empty() {
            for id in [TB_TWIST, TB_BEND_DIRECTION] {
                if self
                    .spine_ring(id, r)
                    .windows(2)
                    .any(|s| input::segment_distance(p, s[0], s[1]) <= 5.)
                {
                    hits.push(id);
                }
            }
        }
        // The arrows start on the rings' centres: the tip wins.
        if hits.len() > 1
            && let Some(&near) = hits.iter().find(|&&id| {
                handles
                    .iter()
                    .any(|h| h.0 == id && Rect::from_center_size(h.2, tip).contains(p))
            })
        {
            return Some(near);
        }
        (hits.len() == 1).then(|| hits[0])
    }

    /// Draws one Twist Bevel handle.
    pub(super) fn draw_twist_bevel_handle(
        &self,
        painter: &egui::Painter,
        id: usize,
        (origin, end): (Pos2, Pos2),
        active: bool,
        fade: f32,
        r: Rect,
    ) {
        // Blue along the spine (Shift, twist), red sideways (Inset), green
        // for the bend: Bevel's Shift/Inset colours, the bend in the third.
        let colour = if active {
            palette::handle_active()
        } else {
            palette::handles()[match id {
                TB_INSET => 0,
                TB_BEND | TB_BEND_DIRECTION => 1,
                _ => 2,
            }]
        }
        .gamma_multiply(fade);
        let stroke = Stroke::new(self.handle_preferences.line_width, colour);
        match id {
            TB_TWIST | TB_BEND_DIRECTION => {
                let ring = self.spine_ring(id, r);
                painter.add(Shape::line(ring.clone(), stroke));
            }
            TB_INSET => {
                painter.line_segment([origin, end], stroke);
                painter.rect_filled(
                    Rect::from_center_size(
                        end,
                        Vec2::splat(
                            self.handle_preferences.large_point_size() * self.handle_zoom.get(),
                        ),
                    ),
                    0.,
                    colour,
                );
            }
            _ => {
                painter.line_segment([origin, end], stroke);
                handle_arrow_head(painter, origin, end, colour);
            }
        }
    }

    /// One sample of a Twist Bevel handle drag, from the values at the
    /// press (`base`).
    pub(super) fn update_twist_bevel_handle(
        &mut self,
        id: usize,
        base: Parameters,
        start: Pos2,
        p: Pos2,
        r: Rect,
    ) {
        let Some(view) = self.twist_bevel_spine(&base) else {
            return;
        };
        let camera = self.camera.clone();
        let (eye, right, up, _) = camera.basis();
        // World distance of the pointer's travel along `direction` on screen.
        let along = |direction: DVec3| {
            let screen = vec2(direction.dot(right) as f32, -direction.dot(up) as f32);
            let pixels = (p - start).dot(screen) / screen.length_sq().max(0.001);
            pixels as f64 * camera.distance / (r.height() as f64 * 0.9)
        };
        // The pointer's turn about `center` on screen, in degrees,
        // anticlockwise as seen, made right-handed about `axis`. Turns
        // past half a circle carry on from the previous sample.
        let turn = |center: DVec3, axis: DVec3, last: &mut Option<f64>| -> Option<f64> {
            let (c, _) = camera.project(center, r)?;
            let (a, b) = (start - c, p - c);
            if a.length() < 2. || b.length() < 2. {
                return Some(last.unwrap_or(0.));
            }
            // Screen y runs down: an anticlockwise turn lowers atan2.
            let raw = -((b.y.atan2(b.x) - a.y.atan2(a.x)) as f64).to_degrees();
            let previous = last.unwrap_or(0.);
            let unwrapped = raw + 360. * ((previous - raw) / 360.).round();
            *last = Some(unwrapped);
            let facing = axis.dot(eye - center) >= 0.;
            Some(if facing { unwrapped } else { -unwrapped })
        };
        if p.distance(start) < 0.5 {
            self.twist_bevel_turn = None;
        }
        let mut params = base;
        let o = &mut params.twist_bevel;
        let readout = match id {
            TB_SHIFT => {
                o.shift += along(view.tangent());
                // Spec section 5: only the Shift arrow snaps, landing the
                // cap centre P(1) on a target: Shift solves for it along
                // the spine's reach.
                let mut moved = view;
                moved.frame.shift = o.shift / moved.scale;
                let reach = moved.reach();
                if let Some(target) = self.snap_point(moved.cap(), None, r)
                    && reach.length_squared() > 1e-18
                {
                    let local = (target - moved.base()).dot(reach) / reach.length_squared();
                    params.twist_bevel.shift = local * moved.scale;
                }
                format!("Shift {:.4} m", params.twist_bevel.shift)
            }
            TB_INSET => {
                // As Bevel's Inset square: out grows the cap (less Inset).
                o.inset -= along(view.cap_side());
                format!("Inset {:.4} m", o.inset)
            }
            TB_BEND => {
                // Toward B curls more: the cap's sideways travel over the
                // tube's length, as an angle.
                let length = (o.shift.abs()).max(camera.distance * 0.05);
                o.bend += (along(view.bend_dir()) / length).to_degrees();
                format!("Bend {:.1}°", o.bend.clamp(-360., 360.))
            }
            TB_TWIST | TB_BEND_DIRECTION => {
                let (center, axis) = if id == TB_TWIST {
                    (view.cap(), view.tangent())
                } else {
                    (view.base(), view.normal())
                };
                let mut last = self.twist_bevel_turn;
                let Some(degrees) = turn(center, axis, &mut last) else {
                    return;
                };
                self.twist_bevel_turn = last;
                let o = &mut params.twist_bevel;
                if id == TB_TWIST {
                    o.rotation += degrees;
                    format!("Rotation {:.1}°", o.rotation)
                } else {
                    o.bend_direction += degrees;
                    format!("Bend Direction {:.1}°", o.bend_direction.rem_euclid(360.))
                }
            }
            _ => return,
        };
        params.twist_bevel = params.twist_bevel.clamped();
        self.publish_handle_params(Tool::TwistBevel, params);
        // The readout: the property and its value (spec section 4).
        if self
            .editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.preview_error.is_none())
        {
            self.status = format!("Twist Bevel · {readout}");
        }
    }
}
