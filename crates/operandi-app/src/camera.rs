use eframe::egui::{Pos2, Rect};
use glam::DVec3;
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    Perspective,
    Orthographic,
    Top,
    Bottom,
    Front,
    Back,
    Right,
    Left,
}
/// The viewport's default lens: the view's half height spans 1/1.8 at
/// unit depth.
pub const DEFAULT_LENS: f64 = 0.9;
/// The default distance of a new view from its target.
const DEFAULT_DISTANCE: f64 = 7.;
/// How close and far the eye may sit from the orbit centre when zooming
/// about it (everything but the wheel's zoom to surface).
pub const MIN_DISTANCE: f64 = 0.03;
pub const MAX_DISTANCE: f64 = 1e7;
/// Radians a two-axis orbit (and look around) turns per point of drag.
const ORBIT_SPEED: f64 = 0.008;
/// How far a two-axis orbit tilts up or down, just short of the poles.
pub const ORBIT_PITCH: f64 = 1.5;
/// The perspective near clip: fixed at `NEAR_CLIP` until a wheel zoom to
/// surface comes closer, then `NEAR_PER_DEPTH` of the surface's depth,
/// never below `NEAR_FLOOR` (ZP-03: the surface stays visible however
/// close the zoom gets).
pub const NEAR_CLIP: f64 = 0.01;
const NEAR_PER_DEPTH: f64 = 0.05;
const NEAR_FLOOR: f64 = 1e-6;
/// The closest a wheel zoom to surface brings the eye to the surface
/// (0.1 mm): well clear of the near-clip floor and of the GPU's single
/// precision, so the surface never flickers or vanishes.
pub const MIN_SURFACE_DEPTH: f64 = 1e-4;
/// Where a line running behind the near clip is cut, as a multiple of the
/// near distance: a hair in front, so the cut end is still drawn (gpu.rs
/// `edge_out_fading` uses the same value).
pub const NEAR_KEEP: f64 = 1.01;
/// The part of segment `a`–`b` at least `near * NEAR_KEEP` in front of
/// `eye` along `forward`: an end behind is moved along the segment to that
/// depth. `None` when both ends are behind.
pub fn clip_to_near(
    a: DVec3,
    b: DVec3,
    eye: DVec3,
    forward: DVec3,
    near: f64,
) -> Option<(DVec3, DVec3)> {
    let keep = near * NEAR_KEEP;
    let (za, zb) = ((a - eye).dot(forward), (b - eye).dot(forward));
    match (za >= keep, zb >= keep) {
        (true, true) => Some((a, b)),
        (false, false) => None,
        (false, true) => Some((b + (a - b) * ((zb - keep) / (zb - za)), b)),
        (true, false) => Some((a, a + (b - a) * ((za - keep) / (za - zb)))),
    }
}
/// Orthographic views draw what lies within `ORTHO_REACH` of the orbit
/// centre along the view, in front of it or behind (further when the view
/// distance is larger). The range is fixed to the orbit centre, not the
/// eye: zooming an orthographic view only scales it, so zooming in never
/// clips the mesh away (bug report #2: Front view lost the mesh after a
/// modest zoom until Frame All).
pub const ORTHO_REACH: f64 = 1000.;
/// `distance` zoomed by `factor` about the orbit centre, within
/// [`MIN_DISTANCE`, `MAX_DISTANCE`]. A view a wheel zoom to surface took
/// closer than `MIN_DISTANCE` doesn't jump back out to it.
pub fn zoomed_distance(distance: f64, factor: f64) -> f64 {
    (distance * factor).clamp(MIN_DISTANCE.min(distance), MAX_DISTANCE)
}

thread_local! {
    /// Flatness of Perspective as a lens multiple (1 at the default 60%).
    static LENS_SCALE: std::cell::Cell<f64> = const { std::cell::Cell::new(1.) };
}
/// The lens multiple for Flatness of Perspective `percent`: 60% is
/// Operandi's lens, each 40 points doubles or halves it (0% ≈ a 115°
/// view, 100% ≈ 31°). Provisional (PREF-Q8): the reference's curve isn't
/// known.
pub fn flatness_scale(percent: u8) -> f64 {
    2f64.powf((f64::from(percent.min(100)) - 60.) / 40.)
}
/// Set the flatness new views start with (this thread).
pub fn set_flatness(percent: u8) {
    LENS_SCALE.with(|c| c.set(flatness_scale(percent)));
}
thread_local! {
    /// Up Axis (Preferences > Distance Units): 0 X, 1 Y, 2 Z.
    static UP: std::cell::Cell<u8> = const { std::cell::Cell::new(1) };
}
/// Set which world axis is up (this thread).
pub fn set_up_axis(axis: u8) {
    UP.with(|c| c.set(axis.min(2)));
}
pub fn up_axis() -> u8 {
    UP.with(std::cell::Cell::get)
}
/// The turn from Operandi's Y-up view frame to the world with the chosen up
/// axis: Y-up's +Y goes to the up axis.
pub fn up_frame() -> glam::DMat3 {
    match up_axis() {
        // Y → Z, Z → -Y.
        2 => glam::DMat3::from_cols(DVec3::X, DVec3::Z, -DVec3::Y),
        // Y → X, X → -Y.
        0 => glam::DMat3::from_cols(-DVec3::Y, DVec3::X, DVec3::Z),
        _ => glam::DMat3::IDENTITY,
    }
}
/// The work plane index lying flat under the up axis (0 XZ, 1 YZ, 2 XY).
pub fn ground_plane() -> u8 {
    match up_axis() {
        0 => 1,
        2 => 2,
        _ => 0,
    }
}
/// The lens of a new view at the current Flatness of Perspective.
/// See `Camera::handle_zoom`: the default view's depth-per-lens over the
/// current one. Screen size is `height * lens / depth`, so this ratio is
/// the handle's on-screen size at a fixed world size relative to the
/// default view.
pub fn world_handle_zoom(depth: f64, lens: f64) -> f64 {
    (DEFAULT_DISTANCE / DEFAULT_LENS) * lens / depth.max(f64::MIN_POSITIVE)
}
pub fn default_lens() -> f64 {
    DEFAULT_LENS * LENS_SCALE.with(std::cell::Cell::get)
}
#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Camera {
    pub view: View,
    pub target: DVec3,
    pub distance: f64,
    pub yaw: f64,
    pub pitch: f64,
    /// Turn about the view direction (Perspective and Orthographic only),
    /// left by trackball orbit; 0 keeps world up at the top of the view.
    pub roll: f64,
    /// Points per world unit at unit depth, as a fraction of the viewport
    /// height (`DEFAULT_LENS` normally; a Camera view sets it from the
    /// camera item's lens and film).
    pub lens: f64,
    /// CL-1 Camera view: this view looks through the scene's render
    /// camera (`view` is then Perspective or Orthographic to match it).
    pub through: bool,
    /// Camera view: the placement last taken from or written to the camera
    /// item (target, distance, yaw, pitch), to notice navigation.
    pub synced: Option<[f64; 6]>,
    /// Camera and Light view: navigation not written to the item yet, as
    /// (placement, when it last changed). A wheel, trackpad or 3D mouse
    /// move holds no button, so the item is written once the view has held
    /// still for `SETTLE` (one undo step per gesture, not per frame); a
    /// pointer drag is written on release.
    pub settling: Option<([f64; 6], std::time::Instant)>,
    /// Camera view through this camera item rather than the active or
    /// newest one (MCP `view_set` / `viewport_capture` camera).
    pub through_item: Option<u64>,
    /// Light view: this view looks through a light (`view` is then
    /// Perspective), and navigating moves the light, as Camera view moves
    /// its camera. `None` in every other view, Camera view included.
    pub light: Option<LightView>,
    /// Depth of the surface the last wheel zoom to surface moved toward;
    /// sets the near clip (`near`). Infinite (the fixed 0.01 clip) until
    /// then and after Frame.
    pub near_depth: f64,
    /// The view distance when `near_depth` was taken. The near clip scales
    /// with the distance since then (`near_surface_depth`), so a view that
    /// zoomed in to a surface and then moved well away by other means
    /// (drag zoom, 3D mouse) doesn't keep a tiny near clip, which would
    /// leave the depth buffer no precision for distant meshes: the wire
    /// then sank into its own polygons. Infinite: no scaling.
    pub near_distance: f64,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            view: View::Perspective,
            target: DVec3::ZERO,
            // A longer lens sits further back, so the target keeps its size
            // on screen and only the perspective changes.
            distance: DEFAULT_DISTANCE * LENS_SCALE.with(std::cell::Cell::get),
            yaw: 0.65,
            pitch: 0.48,
            roll: 0.,
            lens: default_lens(),
            through: false,
            synced: None,
            settling: None,
            through_item: None,
            light: None,
            near_depth: f64::INFINITY,
            near_distance: f64::INFINITY,
        }
    }
}
/// Which light a Light view looks through.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightView {
    /// The view menu's "(Current Selection)": the view follows the selected
    /// light, keeping `item` while something else is selected.
    pub follow_selection: bool,
    /// The light shown now.
    pub item: u64,
}
impl Camera {
    /// Camera view or Light view: the view is an item's eye, its lens comes
    /// from the item and navigating moves the item.
    pub fn looks_through(&self) -> bool {
        self.through || self.light.is_some()
    }
    /// The perspective near clip (see `NEAR_CLIP`); also where `project`
    /// stops drawing.
    pub fn near(&self) -> f64 {
        let depth = self.near_surface_depth();
        if depth.is_finite() {
            (depth * NEAR_PER_DEPTH).clamp(NEAR_FLOOR, NEAR_CLIP)
        } else {
            NEAR_CLIP
        }
    }
    /// `near_depth` moved with the view distance since it was taken
    /// (`near_distance`); infinite with no zoom to surface since Frame.
    pub fn near_surface_depth(&self) -> f64 {
        let scale = if self.near_distance.is_finite() && self.near_distance > 0. {
            self.distance / self.near_distance
        } else {
            1.
        };
        self.near_depth * scale
    }
    /// Orthographic depth: (`offset`, `range`). A point's depth from the
    /// eye plus `offset` over `range` runs 0..1 across the drawn slab,
    /// which spans `ORTHO_REACH` (or the distance) either side of the orbit
    /// centre whatever the zoom. (0, 1) in perspective.
    pub fn ortho_depth(&self) -> (f64, f64) {
        if self.view == View::Perspective {
            return (0., 1.);
        }
        let reach = self.distance.max(ORTHO_REACH);
        (reach - self.distance, 2. * reach)
    }
    /// Flatness of Perspective changed by `ratio` (new lens / old): lens and
    /// distance scale together, so what sits at the target keeps its size
    /// in every view type and only the perspective's strength changes.
    /// Camera view keeps its render camera's lens.
    pub fn reflatten(&mut self, ratio: f64) {
        if self.looks_through() || !(ratio.is_finite() && ratio > 0.) {
            return;
        }
        self.lens *= ratio;
        self.distance *= ratio;
    }
    /// The placement fields Camera view watches.
    pub fn placement(&self) -> [f64; 6] {
        [
            self.target.x,
            self.target.y,
            self.target.z,
            self.distance,
            self.yaw,
            self.pitch,
        ]
    }
    /// The view-menu name of this view.
    pub fn view_label(&self) -> String {
        if self.through {
            "Camera".into()
        } else if self.light.is_some() {
            "Light".into()
        } else {
            format!("{:?}", self.view)
        }
    }
    pub fn basis(&self) -> (DVec3, DVec3, DVec3, DVec3) {
        let toward = match self.view {
            View::Perspective | View::Orthographic => DVec3::new(
                self.yaw.sin() * self.pitch.cos(),
                self.pitch.sin(),
                self.yaw.cos() * self.pitch.cos(),
            ),
            View::Top => DVec3::Y,
            View::Bottom => -DVec3::Y,
            View::Front => DVec3::Z,
            View::Back => -DVec3::Z,
            View::Right => DVec3::X,
            View::Left => -DVec3::X,
        };
        let forward = -toward;
        let up_seed = if self.view == View::Top {
            -DVec3::Z
        } else if self.view == View::Bottom {
            DVec3::Z
        } else {
            DVec3::Y
        };
        let right = forward.cross(up_seed).normalize();
        let up = right.cross(forward);
        let (right, up) =
            if self.roll != 0. && matches!(self.view, View::Perspective | View::Orthographic) {
                let (sin, cos) = self.roll.sin_cos();
                (right * cos + up * sin, up * cos - right * sin)
            } else {
                (right, up)
            };
        // Up Axis: the Y-up frame turned so its up is the chosen axis.
        let frame = up_frame();
        let (toward, right, up, forward) =
            (frame * toward, frame * right, frame * up, frame * forward);
        (self.target + toward * self.distance, right, up, forward)
    }
    /// Yaw and pitch for an eye looking along `toward` (world), in the Up
    /// Axis frame: what Camera view and the Preview take from a camera.
    pub fn yaw_pitch_of(toward: DVec3) -> (f64, f64) {
        let t = up_frame().transpose() * toward;
        (t.x.atan2(t.z), t.y.clamp(-1.0, 1.0).asin())
    }
    /// Trackball (free) orbit for a pointer move from `from` to `to` in `r`:
    /// the scene turns as if the pointer dragged a ball filling the view.
    /// Near the centre that tilts and turns the view about its own axes
    /// (with no stop at the poles); near the edge it rolls about the view
    /// direction. `speed` multiplies the angle (Trackball Speed Multiplier).
    pub fn trackball(&mut self, from: Pos2, to: Pos2, r: Rect, speed: f64) {
        use glam::{DMat3, DQuat};
        if from == to || !r.is_positive() {
            return;
        }
        if !matches!(self.view, View::Perspective | View::Orthographic) {
            let (_, _, up, forward) = self.basis();
            self.view = View::Orthographic;
            self.set_orientation(-forward, up);
        }
        let radius = 0.5 * r.width().min(r.height()) as f64;
        // Bell's trackball: a sphere in the middle, a hyperbolic sheet
        // outside it, so drags near the edge roll smoothly.
        let ball = |p: Pos2| {
            let x = (p.x - r.center().x) as f64 / radius;
            let y = -(p.y - r.center().y) as f64 / radius;
            let d2 = x * x + y * y;
            let z = if d2 <= 0.5 {
                (1. - d2).sqrt()
            } else {
                0.5 / d2.sqrt()
            };
            DVec3::new(x, y, z).normalize()
        };
        let (a, b) = (ball(from), ball(to));
        let axis = a.cross(b);
        if axis.length_squared() < 1e-18 {
            return;
        }
        let angle = a.dot(b).clamp(-1., 1.).acos() * speed;
        let (_, right, up, forward) = self.basis();
        let frame = DQuat::from_mat3(&DMat3::from_cols(right, up, -forward));
        // The scene turns by (axis, angle) in view space, so the camera turns
        // the other way about the target.
        let turned = frame * DQuat::from_axis_angle(axis.normalize(), -angle);
        let toward = (turned * DVec3::Z).normalize();
        let up = (turned * DVec3::Y).normalize();
        self.set_orientation(toward, up);
    }
    /// Make a fixed view (Top, Front, ...) a free Orthographic view looking
    /// the same way, so it can turn.
    pub fn free(&mut self) {
        if !matches!(self.view, View::Perspective | View::Orthographic) {
            let (_, _, up, forward) = self.basis();
            self.view = View::Orthographic;
            self.set_orientation(-forward, up);
        }
    }
    /// Before a two-axis mouse orbit or look around: Top, Front and Right
    /// become a free Orthographic view at the matching yaw and pitch (Top
    /// stops just short of the pole, where the orbit's pitch clamp sits).
    pub fn unfix_for_orbit(&mut self) {
        let (yaw, pitch) = match self.view {
            View::Top => (0., ORBIT_PITCH),
            View::Front => (0., 0.),
            View::Right => (std::f64::consts::FRAC_PI_2, 0.),
            _ => return,
        };
        self.yaw = yaw;
        self.pitch = pitch;
        self.roll = 0.;
        self.view = View::Orthographic;
    }
    /// The two-axis orbit for a drag of (`dx`, `dy`) points: the eye
    /// circles the target, tilting no further than just short of a pole.
    pub fn orbit_by(&mut self, dx: f64, dy: f64) {
        self.yaw -= dx * ORBIT_SPEED;
        self.pitch = (self.pitch + dy * ORBIT_SPEED).clamp(-ORBIT_PITCH, ORBIT_PITCH);
    }
    /// NAV-03 look around (Alt+right drag): the eye stays put and the view
    /// turns about it, by the same angles a drag of (`dx`, `dy`) orbits.
    /// Dragging right turns the view to look right, dragging up looks up;
    /// the target swings round in front of the eye at the same distance.
    pub fn look_around(&mut self, dx: f64, dy: f64) {
        let (eye, ..) = self.basis();
        self.unfix_for_orbit();
        self.orbit_by(dx, dy);
        let (moved, ..) = self.basis();
        self.target += eye - moved;
    }
    /// NAV-02: bank the view about its own viewing axis. A render camera
    /// view has no roll.
    pub fn roll_by(&mut self, angle: f64) {
        if self.looks_through() {
            return;
        }
        self.free();
        self.roll = (self.roll + angle).rem_euclid(std::f64::consts::TAU);
    }
    /// NAV-14: world up points up on screen; the view direction stays.
    pub fn upright(&mut self) {
        self.free();
        self.roll = 0.;
    }
    /// NAV-17: look along `-normal` (world) at the target, world up kept
    /// at the top of the view (another up when looking straight down it).
    pub fn face(&mut self, normal: DVec3) {
        let Some(toward) = normal.try_normalize() else {
            return;
        };
        self.free();
        let world_up = up_frame() * DVec3::Y;
        let up = if toward.dot(world_up).abs() > 0.999 {
            up_frame() * -DVec3::Z
        } else {
            world_up
        };
        let up = (up - toward * up.dot(toward)).normalize();
        self.set_orientation(toward, up);
    }
    /// Set yaw, pitch and roll so the eye sits along `toward` from the
    /// target with `up` at the top of the view.
    fn set_orientation(&mut self, toward: DVec3, up: DVec3) {
        // Work in the Y-up frame; `basis` turns it back.
        let back = up_frame().transpose();
        let (toward, up) = (back * toward, back * up);
        // Exactly over a pole the yaw is undefined; stay a hair off it.
        let limit = std::f64::consts::FRAC_PI_2 - 1e-6;
        self.pitch = toward.y.clamp(-1., 1.).asin().clamp(-limit, limit);
        self.yaw = toward.x.atan2(toward.z);
        self.roll = 0.;
        let (_, right0, up0, _) = self.basis();
        let (right0, up0) = (back * right0, back * up0);
        self.roll = (-up.dot(right0)).atan2(up.dot(up0));
    }
    /// `project` with the view basis worked out once, for projecting many
    /// points in one frame.
    pub fn projector(&self, r: Rect) -> Projector {
        let (eye, right, up, forward) = self.basis();
        Projector {
            eye,
            right,
            up,
            forward,
            perspective: self.view == View::Perspective,
            distance: self.distance,
            near: self.near(),
            ortho_offset: self.ortho_depth().0,
            lens: self.lens,
            r,
        }
    }
    pub fn project(&self, p: DVec3, r: Rect) -> Option<(Pos2, f64)> {
        let (eye, right, up, forward) = self.basis();
        let d = p - eye;
        let z = d.dot(forward);
        if !self.drawn_depth(z) {
            return None;
        }
        let scale = r.height() as f64 * self.lens
            / if self.view == View::Perspective {
                z
            } else {
                self.distance
            };
        Some((
            Pos2::new(
                r.center().x + (d.dot(right) * scale) as f32,
                r.center().y - (d.dot(up) * scale) as f32,
            ),
            z,
        ))
    }
    /// A segment on screen: `project` of both ends, with an end behind the
    /// near clip (perspective) first moved along the segment to just in
    /// front of it (`clip_to_near`), as the GPU wire does, so a long line
    /// running past the eye keeps its visible part. `None` when the whole
    /// segment is behind.
    pub fn project_segment(&self, a: DVec3, b: DVec3, r: Rect) -> Option<(Pos2, Pos2)> {
        let (a, b) = if self.view == View::Perspective {
            let (eye, _, _, forward) = self.basis();
            clip_to_near(a, b, eye, forward, self.near())?
        } else {
            (a, b)
        };
        Some((self.project(a, r)?.0, self.project(b, r)?.0))
    }
    /// Whether depth `z` from the eye is in front of the near clip:
    /// `near` in perspective, the front of the orbit-centre slab in
    /// orthographic (`ortho_depth`), so zoom never clips there.
    fn drawn_depth(&self, z: f64) -> bool {
        if self.view == View::Perspective {
            z >= self.near()
        } else {
            z + self.ortho_depth().0 >= 0.
        }
    }
    /// The world length one screen point covers at world point `p` (at its
    /// depth in perspective); `None` behind the camera.
    pub fn metres_per_pixel(&self, p: DVec3, r: Rect) -> Option<f64> {
        Some(self.screen_depth(p)? / (r.height() as f64 * self.lens))
    }
    /// The depth that sets on-screen size at `p`: its distance in front of
    /// the eye in perspective, the view distance otherwise. `None` behind
    /// the camera.
    fn screen_depth(&self, p: DVec3) -> Option<f64> {
        if self.view != View::Perspective {
            return Some(self.distance);
        }
        let (eye, _, _, forward) = self.basis();
        let z = (p - eye).dot(forward);
        (z >= self.near()).then_some(z)
    }
    /// Original-compatible A1: how many times bigger than its constant
    /// screen size a handle at `p` draws when handles have a world size.
    /// 1 at the default view (target at the default distance, default
    /// lens, any Flatness), so the handle looks as it does today there;
    /// halving the distance doubles it, doubling it halves it, without
    /// limit. Viewport height doesn't change it. 1 behind the camera.
    pub fn handle_zoom(&self, p: DVec3) -> f64 {
        self.screen_depth(p)
            .map_or(1., |depth| world_handle_zoom(depth, self.lens))
    }
    pub fn ray(&self, p: Pos2, r: Rect) -> (DVec3, DVec3) {
        let (eye, right, up, forward) = self.basis();
        let x = (p.x - r.center().x) as f64 / (r.height() as f64 * self.lens);
        let y = -(p.y - r.center().y) as f64 / (r.height() as f64 * self.lens);
        if self.view == View::Perspective {
            (eye, (forward + right * x + up * y).normalize())
        } else {
            // From the front of the drawn slab, so a ray hits whatever is
            // drawn, even in front of the zoomed-in eye.
            (
                eye + right * x * self.distance + up * y * self.distance
                    - forward * self.ortho_depth().0,
                forward,
            )
        }
    }
    /// Perspective wheel zoom toward the surface point `hit` by `factor`
    /// (< 1 is closer). ZP-01: the eye and the orbit centre both move toward
    /// the hit by the same fraction, orientation unchanged, so the hit
    /// stays under the same screen point. ZP-02: the orbit centre closes in
    /// on the surface step by step rather than jumping to it. ZP-03: the
    /// eye never reaches or passes the surface and no fixed 3 cm limit
    /// stops it; it halts `MIN_SURFACE_DEPTH` short, and the near clip
    /// follows the surface's depth so the surface stays drawn. Zooming out
    /// keeps `MAX_DISTANCE`. `false` (nothing changed) for other views, a
    /// hit behind the eye, or a bad factor.
    pub fn dolly_toward(&mut self, hit: DVec3, factor: f64) -> bool {
        if self.view != View::Perspective || !(factor.is_finite() && factor > 0.) {
            return false;
        }
        let (eye, _, _, forward) = self.basis();
        let depth = (hit - eye).dot(forward);
        if !(depth.is_finite() && depth > 0.) {
            return false;
        }
        // Both limits shrink the step, still about the hit.
        let factor = factor
            .max(MIN_SURFACE_DEPTH.min(depth) / depth)
            .min(MAX_DISTANCE.max(self.distance) / self.distance);
        self.target = hit + (self.target - hit) * factor;
        self.distance *= factor;
        self.near_depth = depth * factor;
        self.near_distance = self.distance;
        true
    }
    /// The point under `p` on the plane through the orbit centre facing
    /// the view. Always defined in orthographic views (the ray runs along
    /// the plane's normal); `None` in perspective only for a ray that
    /// misses it.
    pub fn view_plane_point(&self, p: Pos2, r: Rect) -> Option<DVec3> {
        let (o, d) = self.ray(p, r);
        let (_, _, _, forward) = self.basis();
        let denom = d.dot(forward);
        if denom.abs() < 1e-8 {
            return None;
        }
        Some(o + d * ((self.target - o).dot(forward) / denom))
    }
    /// Where zoom at a screen point anchors: in perspective the ground
    /// plane through the orbit centre (`plane_point` plane 0), in every
    /// other view the plane facing the view (`view_plane_point`), which
    /// the ground plane is edge-on to in Front, Back and side views.
    pub fn zoom_anchor(&self, p: Pos2, r: Rect) -> Option<DVec3> {
        if self.view == View::Perspective {
            self.plane_point(p, r, 0, self.target)
        } else {
            self.view_plane_point(p, r)
        }
    }
    pub fn plane_point(&self, p: Pos2, r: Rect, plane: u8, center: DVec3) -> Option<DVec3> {
        let (o, d) = self.ray(p, r);
        let n = match plane {
            1 => DVec3::X,
            2 => DVec3::Z,
            _ => DVec3::Y,
        };
        let denom = d.dot(n);
        if denom.abs() < 1e-8 {
            return None;
        }
        let t = (center - o).dot(n) / denom;
        if t < 0. { None } else { Some(o + d * t) }
    }
}
pub fn triangle_hit(o: DVec3, d: DVec3, a: DVec3, b: DVec3, c: DVec3) -> Option<f64> {
    let e = b - a;
    let f = c - a;
    let h = d.cross(f);
    let det = e.dot(h);
    if det <= 1e-10 {
        return None;
    }
    let inv = 1. / det;
    let s = o - a;
    let u = inv * s.dot(h);
    if !(0. ..=1.).contains(&u) {
        return None;
    }
    let q = s.cross(e);
    let v = inv * d.dot(q);
    if v < 0. || u + v > 1. {
        return None;
    }
    let t = inv * f.dot(q);
    (t > 0.).then_some(t)
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Original-compatible A1: a handle with a world size is today's size
    /// at the default view, and its screen size goes as 1 / distance.
    #[test]
    fn a1_world_size_handles_follow_the_zoom() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        let mut c = Camera::default();
        assert!(close(c.handle_zoom(c.target), 1.));
        c.distance = DEFAULT_DISTANCE / 2.;
        assert!(close(c.handle_zoom(c.target), 2.));
        c.distance = DEFAULT_DISTANCE * 4.;
        assert!(close(c.handle_zoom(c.target), 0.25));
        // A longer lens at the matching distance (Flatness) looks the same.
        assert!(close(world_handle_zoom(14., DEFAULT_LENS * 2.), 1.));
        // Orthographic views use the view distance wherever the handle is.
        c.view = View::Top;
        c.distance = DEFAULT_DISTANCE;
        assert!(close(c.handle_zoom(DVec3::new(3., -40., 2.)), 1.));
        // Behind the eye: no zoom (the handle isn't drawn there).
        c.view = View::Perspective;
        let (eye, _, _, forward) = c.basis();
        assert!(close(c.handle_zoom(eye - forward), 1.));
    }
    #[test]
    fn projection_ray_invariant_across_logical_sizes() {
        for (w, h) in [
            (1920., 1080.),
            (2560., 1440.),
            (3840., 2160.),
            (5120., 2880.),
        ] {
            for scale in [1., 2.] {
                let r = Rect::from_min_size(
                    Pos2::new(244., 52.),
                    eframe::egui::vec2(w / scale, h / scale),
                );
                let c = Camera::default();
                let p = DVec3::new(0.3, 0.5, 0.2);
                let (s, _) = c.project(p, r).unwrap();
                let (o, d) = c.ray(s, r);
                assert!((p - o).cross(d).length() < 1e-5);
            }
        }
    }

    fn view_rect() -> Rect {
        Rect::from_min_size(Pos2::new(0., 0.), eframe::egui::vec2(1200., 800.))
    }
    /// A camera whose orbit centre sits well past the surface point `hit`,
    /// which is off to the side of the view.
    fn off_centre() -> (Camera, DVec3) {
        let c = Camera {
            target: DVec3::new(0., 0., -20.),
            ..Camera::default()
        };
        let (eye, right, up, forward) = c.basis();
        (c, eye + forward * 3. + right * 0.4 - up * 0.25)
    }
    fn depth(c: &Camera, p: DVec3) -> f64 {
        let (eye, _, _, forward) = c.basis();
        (p - eye).dot(forward)
    }

    /// ZP-01: eye and orbit centre move by the same fraction; the hit keeps
    /// its screen position, in and out.
    #[test]
    fn dolly_keeps_the_hit_under_the_pointer() {
        let r = view_rect();
        let (mut c, hit) = off_centre();
        let (before, _) = c.project(hit, r).unwrap();
        for factor in [0.5, 0.8, 1.6, 0.1] {
            let (target, distance) = (c.target, c.distance);
            assert!(c.dolly_toward(hit, factor));
            assert!((c.target - (hit + (target - hit) * factor)).length() < 1e-9);
            assert!((c.distance - distance * factor).abs() < 1e-9);
            let (after, _) = c.project(hit, r).unwrap();
            assert!((after - before).length() < 1e-3, "{before:?} -> {after:?}");
        }
    }

    /// ZP-02: the orbit centre closes in on the surface step by step; it
    /// does not jump to the hit's depth.
    #[test]
    fn dolly_moves_the_orbit_centre_gradually() {
        let (mut c, hit) = off_centre();
        let mut gap = (c.target - hit).length();
        assert!(c.dolly_toward(hit, 0.8));
        let after = (c.target - hit).length();
        assert!((after - gap * 0.8).abs() < 1e-9, "{gap} -> {after}");
        for _ in 0..20 {
            c.dolly_toward(hit, 0.8);
            let next = (c.target - hit).length();
            assert!(next < gap);
            gap = next;
        }
    }

    /// ZP-03: no 3 cm floor; the eye gets to within 0.1 mm of the surface,
    /// never through it, and the near clip keeps the surface drawn.
    #[test]
    fn repeated_dolly_reaches_the_surface_and_never_passes_it() {
        let r = view_rect();
        let (mut c, hit) = off_centre();
        let (at, _) = c.project(hit, r).unwrap();
        for _ in 0..400 {
            c.dolly_toward(hit, 0.8);
            let d = depth(&c, hit);
            assert!(d >= MIN_SURFACE_DEPTH * (1. - 1e-9), "passed: {d}");
            assert!(c.near() < d, "clipped: near {} depth {d}", c.near());
            let (now, _) = c.project(hit, r).expect("hit drawn");
            assert!((now - at).length() < 1e-2);
        }
        assert!(c.distance < MIN_DISTANCE, "stalled at {}", c.distance);
        assert!((depth(&c, hit) - MIN_SURFACE_DEPTH).abs() < 1e-12);
        // Zooming back out works and the old clamp doesn't snap it to 3 cm.
        let d = c.distance;
        assert_eq!(zoomed_distance(d, 0.5), d);
        assert!((zoomed_distance(d, 2.) - 2. * d).abs() < 1e-15);
        assert!(c.dolly_toward(hit, 2.));
        assert!((depth(&c, hit) - 2. * MIN_SURFACE_DEPTH).abs() < 1e-12);
    }

    /// Until a wheel zoom to surface comes close, the near clip and the
    /// old 3 cm zoom limit are exactly as before.
    #[test]
    fn near_clip_is_fixed_until_a_close_zoom() {
        let c = Camera::default();
        assert_eq!(c.near(), NEAR_CLIP);
        let close = Camera {
            near_depth: 1e-3,
            ..Camera::default()
        };
        assert!((close.near() - 5e-5).abs() < 1e-12);
        assert_eq!(zoomed_distance(1., 0.01), MIN_DISTANCE);
        assert_eq!(zoomed_distance(1e7, 2.), MAX_DISTANCE);
    }

    /// Forum report (disappearing wireframe): after a wheel zoom close to
    /// a surface, moving far away by drag zoom kept the tiny near clip,
    /// which leaves a 32-bit depth buffer no precision for a wall tens of
    /// metres off (its wire sank into its polygons). The near clip now
    /// scales with the view distance since the zoom; wheel zooming itself
    /// is unchanged.
    #[test]
    fn near_clip_follows_the_view_distance_after_a_close_zoom() {
        let mut c = Camera::default();
        let (eye, _, _, forward) = c.basis();
        let hit = eye + forward * 0.01;
        assert!(c.dolly_toward(hit, 0.1));
        let close = c.near();
        assert!(close < NEAR_CLIP * 0.01, "near {close}");
        // Wheel zoom: the near clip still follows the surface's depth.
        let depth = (hit - c.basis().0).dot(forward);
        assert!((close - depth * NEAR_PER_DEPTH).abs() < 1e-12);
        // Drag zoom out 100 times: the near clip grows with it.
        c.distance *= 100.;
        assert!((c.near() - close * 100.).abs() < 1e-12);
        // Far enough, back to the fixed clip.
        c.distance *= 1e6;
        assert_eq!(c.near(), NEAR_CLIP);
    }

    /// Forum report (disappearing wireframe): a long edge running past
    /// the eye (one end behind it) was dropped whole while its polygon
    /// stayed drawn. It is cut at the near clip instead: the cut end lies
    /// on the edge, just in front of the clip, and the part in front is
    /// drawn; an edge wholly behind is still gone.
    #[test]
    fn a_long_edge_through_the_near_clip_keeps_its_visible_part() {
        let r = view_rect();
        let c = Camera::default();
        let (eye, right, _, forward) = c.basis();
        // 30 m long, from 2 m behind the eye to 28 m ahead, 1 m aside.
        let a = eye - forward * 2. + right;
        let b = eye + forward * 28. + right;
        assert!(c.project(a, r).is_none());
        let (ca, cb) = clip_to_near(a, b, eye, forward, c.near()).unwrap();
        assert_eq!(cb, b);
        let along = (ca - a).cross(b - a).length() / (b - a).length();
        assert!(along < 1e-9, "cut point off the edge by {along}");
        assert!(((ca - eye).dot(forward) - c.near() * NEAR_KEEP).abs() < 1e-12);
        let (pa, pb) = c.project_segment(a, b, r).expect("visible part drawn");
        assert_eq!(pb, c.project(b, r).unwrap().0);
        assert_eq!(pa, c.project(ca, r).unwrap().0);
        // The other way round, and wholly behind.
        assert!(c.project_segment(b, a, r).is_some());
        assert!(c.project_segment(a, eye - forward * 5., r).is_none());
        // Orthographic views clip nothing here.
        let top = Camera {
            view: View::Top,
            ..Camera::default()
        };
        assert_eq!(
            top.project_segment(a, b, r),
            Some((top.project(a, r).unwrap().0, top.project(b, r).unwrap().0))
        );
    }

    /// Bug report #2: in Front view a mesh in front of the orbit centre
    /// vanished after a modest zoom in (the eye passed it), until Frame
    /// All. Zooming an orthographic view now only scales it: the point
    /// stays projected, keeps a depth inside the drawn slab, and a ray
    /// through it still reaches it.
    #[test]
    fn ortho_zoom_never_clips_the_mesh() {
        let r = view_rect();
        for view in [View::Front, View::Top, View::Right, View::Orthographic] {
            let mut c = Camera {
                view,
                ..Camera::default()
            };
            let (_, _, _, forward) = c.basis();
            // 2 m nearer the viewer than the orbit centre.
            let p = c.target - forward * 2.;
            for _ in 0..200 {
                c.distance = zoomed_distance(c.distance, 0.8);
                let (at, z) = c
                    .project(p, r)
                    .unwrap_or_else(|| panic!("{view:?} clipped at {}", c.distance));
                assert!((at - r.center()).length() < 1e-3);
                let (offset, range) = c.ortho_depth();
                let depth = (z + offset) / range;
                assert!((0. ..=1.).contains(&depth), "{view:?} depth {depth}");
                let (o, d) = c.ray(at, r);
                assert!((p - o).dot(d) > 0., "{view:?} ray starts past the mesh");
                // The GPU depth matches.
                let u = crate::gpu_scene::camera_uniform(&c, r, false);
                let e = DVec3::new(u[0] as f64, u[1] as f64, u[2] as f64);
                let gpu = ((p - e).dot(forward) + u[3] as f64) / u[19] as f64;
                assert!((gpu - depth).abs() < 1e-4, "{view:?} gpu {gpu} cpu {depth}");
            }
            assert_eq!(c.distance, MIN_DISTANCE);
            // Something far behind the orbit centre is still drawn.
            assert!(c.project(c.target + forward * 900., r).is_some());
        }
    }

    #[test]
    fn dolly_leaves_other_views_alone() {
        let (mut c, hit) = off_centre();
        c.view = View::Orthographic;
        let before = c.placement();
        assert!(!c.dolly_toward(hit, 0.5));
        assert_eq!(c.placement(), before);
        // Perspective, but behind the eye.
        c.view = View::Perspective;
        let (eye, _, _, forward) = c.basis();
        assert!(!c.dolly_toward(eye - forward, 0.5));
        assert_eq!(c.placement(), before);
    }
}

/// A camera's projection for one viewport rectangle; same math as
/// `Camera::project`. Equal projectors put every point in the same place
/// (the snap service keys its screen grid on that).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projector {
    eye: DVec3,
    right: DVec3,
    up: DVec3,
    forward: DVec3,
    perspective: bool,
    distance: f64,
    near: f64,
    /// `Camera::ortho_depth`'s offset (0 in perspective).
    ortho_offset: f64,
    lens: f64,
    r: Rect,
}
impl Projector {
    /// The viewport rectangle this projects into.
    pub fn rect(&self) -> Rect {
        self.r
    }
    /// A screen-space fraction `t` along a segment whose ends lie at
    /// depths `za` and `zb`, as the fraction along the 3D segment (screen
    /// interpolation is perspective-divided).
    pub fn world_parameter(&self, t: f32, za: f64, zb: f64) -> f64 {
        let t = t as f64;
        if self.perspective {
            (t / zb) / ((1. - t) / za + t / zb)
        } else {
            t
        }
    }
    pub fn project(&self, p: DVec3) -> Option<(Pos2, f64)> {
        let d = p - self.eye;
        let z = d.dot(self.forward);
        let drawn = if self.perspective {
            z >= self.near
        } else {
            z + self.ortho_offset >= 0.
        };
        if !drawn {
            return None;
        }
        let scale =
            self.r.height() as f64 * self.lens / if self.perspective { z } else { self.distance };
        Some((
            Pos2::new(
                self.r.center().x + (d.dot(self.right) * scale) as f32,
                self.r.center().y - (d.dot(self.up) * scale) as f32,
            ),
            z,
        ))
    }
}
