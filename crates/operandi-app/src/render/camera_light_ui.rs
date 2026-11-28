//! Camera and light items in the app (CL-1, spec v0, provisional): their
//! viewport shapes, the Camera view, Properties controls and Items-list
//! glyphs. Settings, defaults and undo live in the core
//! (operandi_core::camera_light, camera_light_defaults).
//!
//! Provisional choices, logged in docs/SPECIFICATION_QUESTIONS.md:
//! - CL-Q09: the shapes are simple white wireframes (camera: a box body and
//!   a frustum sized by its film and lens; directional light: a disc and
//!   arrows along its light direction). Clicks in a viewport don't pick
//!   them yet; select them in the Items list.
//! - CL-Q11: Camera view looks through the render camera. Navigating moves
//!   the camera item; each press-to-release (or wheel step) is one undo
//!   step, written when the button is released. Roll and focal-length
//!   drags are not handled; the view is always upright.
//! - CL-Q16 (Jason, 2026-11-05): modeling views don't draw cameras or
//!   lights, selected or not, until the view's Display menu turns Show
//!   Cameras / Show Lights on.
use super::*;
use crate::camera::View;
use crate::unavailable::Gate;
use operandi_core::camera_light::{
    AreaShape, Frame, LightEdit, LightFalloff, LightKind, LightRender, LightSettings, Projection,
    ShadowType, item_frame, local_transform_for,
};
use operandi_core::camera_light_defaults as defaults;

/// How long a Camera or Light view must hold still after a wheel,
/// trackpad or 3D mouse move before the camera or light is moved to match
/// (provisional: long enough to join one zoom gesture's wheel notches).
pub(super) const SETTLE: std::time::Duration = std::time::Duration::from_millis(300);

/// Which placeholder glyph an Items row shows (until generated icons).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LocatorGlyph {
    Camera,
    Light(LightKind),
    /// A Texture Locator: a checkered square.
    TextureLocator,
    /// The Texture Group: a folder.
    TextureGroup,
    /// A Group Locator (GL-12): a little blue folder (Jason, 2026-11-08;
    /// placeholder until a generated icon). The Texture Group's folder is
    /// drawn in the row's text colour, this one always in blue.
    GroupLocator,
}

impl LocatorGlyph {
    /// The glyph for a camera, light or texture locator item.
    pub(super) fn of(item: &operandi_core::Item) -> Option<Self> {
        use operandi_core::texture_locator::TextureItem;
        match &item.texture {
            Some(TextureItem::Locator(_)) => return Some(Self::TextureLocator),
            Some(TextureItem::Group) => return Some(Self::TextureGroup),
            None => {}
        }
        if item.is_group_locator() {
            Some(Self::GroupLocator)
        } else if item.camera.is_some() {
            Some(Self::Camera)
        } else {
            item.light.as_ref().map(|l| Self::Light(l.kind))
        }
    }
}

/// A small line drawing for an Items row, one per kind (CL-21): a movie
/// camera; a sun (directional); a star (point); a lamp housing (spot); a
/// panel with its arrow (area). Drawn here until the icon generator has
/// recipes for them.
pub(super) fn paint_glyph(ui: &Ui, rect: Rect, glyph: LocatorGlyph, color: Color32) {
    let painter = ui.painter();
    let stroke = Stroke::new(1.2f32, color);
    let c = rect.center();
    match glyph {
        LocatorGlyph::TextureLocator => {
            let square = Rect::from_center_size(c, vec2(11., 11.));
            painter.rect_stroke(square, 0., stroke, egui::StrokeKind::Inside);
            let fill = color.gamma_multiply(0.7);
            let half = Rect::from_min_size(square.min, square.size() * 0.5);
            painter.rect_filled(half, 0., fill);
            painter.rect_filled(half.translate(square.size() * 0.5), 0., fill);
        }
        LocatorGlyph::TextureGroup => {
            let body = Rect::from_center_size(c + vec2(0., 1.), vec2(13., 9.));
            painter.rect_stroke(body, 1., stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    body.left_top() + vec2(1., 0.),
                    body.left_top() + vec2(5., -3.),
                ],
                stroke,
            );
        }
        LocatorGlyph::GroupLocator => {
            let blue = theme::workspace::group_locator_icon();
            let (tab, body) = folder_shape(c);
            painter.rect_filled(tab, 1., blue);
            painter.rect_filled(body, 1., blue.gamma_multiply(0.85));
            painter.rect_stroke(
                body,
                1.,
                Stroke::new(1.2f32, blue),
                egui::StrokeKind::Inside,
            );
        }
        LocatorGlyph::Light(LightKind::Point) => {
            for k in 0..4 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let d = vec2(a.cos(), a.sin()) * 6.5;
                painter.line_segment([c - d, c + d], stroke);
            }
        }
        LocatorGlyph::Light(LightKind::Spot) => {
            // A lamp housing opening downward, with its light below.
            painter.add(egui::Shape::closed_line(
                vec![
                    pos2(c.x - 2.5, c.y - 6.),
                    pos2(c.x + 2.5, c.y - 6.),
                    pos2(c.x + 5.5, c.y + 1.),
                    pos2(c.x - 5.5, c.y + 1.),
                ],
                stroke,
            ));
            for dx in [-4., 0., 4.] {
                painter.line_segment(
                    [pos2(c.x + dx * 0.6, c.y + 3.), pos2(c.x + dx, c.y + 6.5)],
                    stroke,
                );
            }
        }
        LocatorGlyph::Light(LightKind::Area) => {
            let panel = Rect::from_center_size(c - vec2(0., 2.5), vec2(12., 6.));
            painter.rect_stroke(panel, 0., stroke, egui::StrokeKind::Inside);
            let tip = pos2(c.x, c.y + 7.);
            painter.line_segment([pos2(c.x, panel.bottom()), tip], stroke);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    tip,
                    pos2(c.x - 2.5, tip.y - 3.),
                    pos2(c.x + 2.5, tip.y - 3.),
                ],
                color,
                Stroke::NONE,
            ));
        }
        LocatorGlyph::Camera => {
            let body = Rect::from_center_size(c - vec2(2., 0.), vec2(9., 7.));
            painter.rect_stroke(body, 1., stroke, egui::StrokeKind::Inside);
            let x = body.right();
            painter.add(egui::Shape::closed_line(
                vec![
                    pos2(x, c.y - 1.5),
                    pos2(x + 4., c.y - 3.5),
                    pos2(x + 4., c.y + 3.5),
                    pos2(x, c.y + 1.5),
                ],
                stroke,
            ));
        }
        LocatorGlyph::Light(LightKind::Portal) => {
            // A window: a frame split into four panes.
            let frame = Rect::from_center_size(c, vec2(12., 12.));
            painter.rect_stroke(frame, 0., stroke, egui::StrokeKind::Inside);
            painter.line_segment([pos2(c.x, frame.top()), pos2(c.x, frame.bottom())], stroke);
            painter.line_segment([pos2(frame.left(), c.y), pos2(frame.right(), c.y)], stroke);
        }
        LocatorGlyph::Light(LightKind::Cylinder) => {
            // A tube lying sideways.
            let tube = Rect::from_center_size(c, vec2(14., 6.));
            painter.rect_stroke(tube, 3., stroke, egui::StrokeKind::Inside);
        }
        LocatorGlyph::Light(LightKind::Mesh) => {
            // A cube with a glow line above it.
            let cube = Rect::from_center_size(c + vec2(0., 2.), vec2(9., 9.));
            painter.rect_stroke(cube, 0., stroke, egui::StrokeKind::Inside);
            for dx in [-3.5, 0., 3.5] {
                painter.line_segment(
                    [pos2(c.x + dx, c.y - 4.), pos2(c.x + dx * 1.3, c.y - 7.)],
                    stroke,
                );
            }
        }
        LocatorGlyph::Light(LightKind::Photometric) => {
            // A bulb on a base.
            painter.circle_stroke(c - vec2(0., 2.), 4.5, stroke);
            painter.line_segment(
                [pos2(c.x - 2.5, c.y + 4.), pos2(c.x + 2.5, c.y + 4.)],
                stroke,
            );
            painter.line_segment(
                [pos2(c.x - 2., c.y + 6.5), pos2(c.x + 2., c.y + 6.5)],
                stroke,
            );
        }
        LocatorGlyph::Light(LightKind::Dome) => {
            // A dome over a ground line.
            painter.add(egui::Shape::line(
                (0..=12)
                    .map(|k| {
                        let a = std::f32::consts::PI * k as f32 / 12.;
                        pos2(c.x + 6.5 * a.cos(), c.y + 3. - 6.5 * a.sin())
                    })
                    .collect(),
                stroke,
            ));
            painter.line_segment(
                [pos2(c.x - 7.5, c.y + 3.), pos2(c.x + 7.5, c.y + 3.)],
                stroke,
            );
        }
        LocatorGlyph::Light(LightKind::Directional) => {
            painter.circle_stroke(c, 3., stroke);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let d = vec2(a.cos(), a.sin());
                painter.line_segment([c + d * 4.8, c + d * 7.], stroke);
            }
        }
    }
}

/// The Group Locator row glyph around `c`: the folder's tab and body.
pub(super) fn folder_shape(c: Pos2) -> (Rect, Rect) {
    let body = Rect::from_center_size(c + vec2(0., 1.), vec2(13., 9.));
    let tab = Rect::from_min_size(body.left_top() + vec2(0., -2.5), vec2(6., 3.));
    (tab, body)
}

/// A Group Locator's viewport shape (GL-10, provisional): a cube `size`
/// across, centred on the item, drawn only as its corner brackets (at each
/// of the 8 corners a tick a quarter of the edge long along each of its 3
/// edges), plus a small cross at the centre. `world` is the item's world
/// matrix, so rotation and scale show.
pub(crate) fn group_locator_marks(world: glam::DMat4, size: f64) -> Vec<Mark> {
    let h = size * 0.5;
    let tick = size * 0.25;
    let cross = size * 0.1;
    let p = |v: DVec3| world.transform_point3(v);
    let mut marks = Vec::with_capacity(27);
    for corner in 0..8 {
        let s = DVec3::new(
            if corner & 1 == 0 { -1. } else { 1. },
            if corner & 2 == 0 { -1. } else { 1. },
            if corner & 4 == 0 { -1. } else { 1. },
        );
        let at = s * h;
        for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
            // Inwards along this edge.
            let inward = -axis * s.dot(axis);
            marks.push(Mark::Line([p(at), p(at + inward * tick)]));
        }
    }
    for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
        marks.push(Mark::Line([p(-axis * cross), p(axis * cross)]));
    }
    marks
}

/// IK-43: for each limited axis, a thin fan from Min to Max about that
/// axis of the joint's parent space, through the joint.
pub(crate) fn limit_fan_marks(
    parent: glam::DMat4,
    at: DVec3,
    ik: &operandi_core::ik_joint::JointIk,
    size: f64,
) -> Vec<Mark> {
    let mut marks = Vec::new();
    let axes = [
        parent.x_axis.truncate().normalize_or(DVec3::X),
        parent.y_axis.truncate().normalize_or(DVec3::Y),
        parent.z_axis.truncate().normalize_or(DVec3::Z),
    ];
    for (k, a) in ik.axes.iter().enumerate().filter(|(_, a)| a.use_limits) {
        let axis = axes[k];
        let start = axes[(k + 1) % 3] * size;
        let at_angle = |t: f64| at + glam::DQuat::from_axis_angle(axis, t) * start;
        const STEPS: usize = 16;
        let (lo, hi) = (a.min.min(a.max), a.min.max(a.max));
        marks.push(Mark::Line([at, at_angle(lo)]));
        marks.push(Mark::Line([at, at_angle(hi)]));
        for i in 0..STEPS {
            let t0 = lo + (hi - lo) * i as f64 / STEPS as f64;
            let t1 = lo + (hi - lo) * (i + 1) as f64 / STEPS as f64;
            marks.push(Mark::Dotted([at_angle(t0), at_angle(t1)]));
        }
    }
    marks
}

/// IK-02: an IK goal's box is 10% of its chain's first bone.
fn ik_goal_size(scene: &operandi_core::Scene, goal: u64) -> f64 {
    let pos = |id: u64| scene.world_matrix(id).map(|m| m.w_axis.truncate()).ok();
    scene
        .animation
        .rig
        .pieces
        .iter()
        .filter(|p| operandi_core::ik::is_ik(&p.kind) && p.targets.contains(&goal))
        .find_map(|p| {
            let first = operandi_core::ik::named(p, "mid")
                .or_else(|| operandi_core::ik::named(p, "end"))?;
            Some(pos(p.item)?.distance(pos(first)?) * 0.1)
        })
        .filter(|s| *s > 1e-9)
        .unwrap_or(0.1)
}

/// IK-02 colours: +X green, −X red, on the centre line blue (the axis
/// colours).
fn ik_goal_color(scene: &operandi_core::Scene, goal: u64) -> Color32 {
    let x = scene.world_matrix(goal).map_or(0., |m| m.w_axis.x);
    let [red, green, blue] = theme::viewport::grid_axes();
    if x > 1e-4 {
        green
    } else if x < -1e-4 {
        red
    } else {
        blue
    }
}

/// A solid box `size` across in the goal's own axes.
pub(crate) fn ik_goal_marks(world: glam::DMat4, size: f64) -> Vec<Mark> {
    let h = size * 0.5;
    let p = |x: f64, y: f64, z: f64| world.transform_point3(DVec3::new(x * h, y * h, z * h));
    let faces: [[(f64, f64, f64); 4]; 6] = [
        [
            (-1., -1., -1.),
            (1., -1., -1.),
            (1., 1., -1.),
            (-1., 1., -1.),
        ],
        [(-1., -1., 1.), (1., -1., 1.), (1., 1., 1.), (-1., 1., 1.)],
        [
            (-1., -1., -1.),
            (1., -1., -1.),
            (1., -1., 1.),
            (-1., -1., 1.),
        ],
        [(-1., 1., -1.), (1., 1., -1.), (1., 1., 1.), (-1., 1., 1.)],
        [
            (-1., -1., -1.),
            (-1., 1., -1.),
            (-1., 1., 1.),
            (-1., -1., 1.),
        ],
        [(1., -1., -1.), (1., 1., -1.), (1., 1., 1.), (1., -1., 1.)],
    ];
    faces
        .iter()
        .map(|f| Mark::Fill(f.iter().map(|&(x, y, z)| p(x, y, z)).collect()))
        .collect()
}

/// A joint's viewport shape (SK-17/18): a wire sphere (three rings in
/// its own axes) and, to its first child, a solid bone (four edges out to
/// a square a sixth of the way along, then in to the child); other
/// children get thin dotted lines. The sphere scales with the bone
/// (a tenth of its length); a leaf uses `leaf` (a constant screen size).
pub(crate) fn joint_marks(world: glam::DMat4, children: &[DVec3], leaf: f64) -> Vec<Mark> {
    let at = world.w_axis.truncate();
    let bone = children
        .first()
        .map(|c| c.distance(at))
        .filter(|l| *l > 1e-9);
    let radius = bone.map_or(leaf, |l| l * 0.1);
    let axes = [
        world.x_axis.truncate().normalize_or(DVec3::X),
        world.y_axis.truncate().normalize_or(DVec3::Y),
        world.z_axis.truncate().normalize_or(DVec3::Z),
    ];
    let mut marks = Vec::new();
    const SEGMENTS: usize = 16;
    for (u, v) in [(0, 1), (1, 2), (2, 0)] {
        let ring = |k: usize| {
            let t = k as f64 / SEGMENTS as f64 * std::f64::consts::TAU;
            at + (axes[u] * t.cos() + axes[v] * t.sin()) * radius
        };
        for k in 0..SEGMENTS {
            marks.push(Mark::Line([ring(k), ring(k + 1)]));
        }
    }
    if let (Some(&tip), Some(len)) = (children.first(), bone) {
        let d = (tip - at) / len;
        // Two directions across the bone.
        let side = if d.cross(axes[0]).length_squared() > 1e-6 {
            axes[0]
        } else {
            axes[1]
        };
        let a = d.cross(side).normalize();
        let b = d.cross(a);
        let base = at + d * len / 6.;
        let w = len * 0.08;
        let square = [base + a * w, base + b * w, base - a * w, base - b * w];
        for k in 0..4 {
            marks.push(Mark::Line([at + d * radius, square[k]]));
            marks.push(Mark::Line([square[k], tip]));
            marks.push(Mark::Line([square[k], square[(k + 1) % 4]]));
        }
    }
    for &c in children.iter().skip(1) {
        marks.push(Mark::Dotted([at, c]));
    }
    marks
}

/// One mark of a camera or light glyph, in world space.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Mark {
    Line([DVec3; 2]),
    /// Selected-only guides (frustum, spot cone) and the area light's
    /// centre lines.
    Dotted([DVec3; 2]),
    /// A filled arrowhead.
    Fill(Vec<DVec3>),
}

/// `n` lines round an ellipse of half-sizes `a` (local X) and `b` (Y) at
/// depth `z`, in the frame `m`.
pub(crate) fn ring(m: glam::DMat4, a: f64, b: f64, z: f64, n: usize, dotted: bool) -> Vec<Mark> {
    let p = |t: f64| m.transform_point3(DVec3::new(a * t.cos(), b * t.sin(), z));
    let t = |k: usize| k as f64 / n as f64 * std::f64::consts::TAU;
    (0..n)
        .map(|k| {
            let seg = [p(t(k)), p(t(k + 1))];
            if dotted {
                Mark::Dotted(seg)
            } else {
                Mark::Line(seg)
            }
        })
        .collect()
}

/// A Texture Locator's viewport gizmo (locator spec §1): its axes, plus
/// the projection's shape at Size: Planar its plane (a unit square across
/// the axis), Cylindrical a cylinder, Spherical three rings, Cubic / Box /
/// Triplanar / Solid a unit cube; UV Map only the axes. `world` is the
/// locator's world matrix (Size included).
pub(crate) fn texture_locator_marks(
    world: glam::DMat4,
    settings: &operandi_core::texture_locator::TextureLocator,
) -> Vec<Mark> {
    use operandi_core::shader_tree::Projection as P;
    let p = |v: DVec3| world.transform_point3(v);
    let mut out = vec![
        Mark::Line([p(DVec3::ZERO), p(DVec3::X * 0.25)]),
        Mark::Line([p(DVec3::ZERO), p(DVec3::Y * 0.25)]),
        Mark::Line([p(DVec3::ZERO), p(DVec3::Z * 0.25)]),
    ];
    // A frame whose local Z is the Projection Axis.
    let axis = world
        * match settings.axis.min(2) {
            0 => glam::DMat4::from_cols(
                DVec3::Y.extend(0.),
                DVec3::Z.extend(0.),
                DVec3::X.extend(0.),
                glam::DVec4::W,
            ),
            1 => glam::DMat4::from_cols(
                DVec3::Z.extend(0.),
                DVec3::X.extend(0.),
                DVec3::Y.extend(0.),
                glam::DVec4::W,
            ),
            _ => glam::DMat4::IDENTITY,
        };
    let q = |v: DVec3| axis.transform_point3(v);
    match settings.projection {
        P::Planar => {
            let c = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
            for k in 0..4 {
                let (a, b) = (c[k], c[(k + 1) % 4]);
                out.push(Mark::Line([
                    q(DVec3::new(a.0, a.1, 0.)),
                    q(DVec3::new(b.0, b.1, 0.)),
                ]));
            }
            // Planar spec (2026-11-09): a short line from the centre along
            // the projection axis, 25 % of the smaller in-plane side, and
            // an "up" tick on the +V edge showing which way the image
            // faces (tick length 10 % of the smaller side, PD-Q13).
            let [(cu, su), (cv, sv)] = operandi_core::planar_texture::plane_axes(settings.axis);
            let unit = |c: usize, s: f64| {
                let mut v = DVec3::ZERO;
                v[c] = s;
                v
            };
            let (u, v) = (unit(cu, su), unit(cv, sv));
            let side_u = world.transform_vector3(u).length();
            let side_v = world.transform_vector3(v).length();
            let short = side_u.min(side_v);
            let normal = world
                .transform_vector3(unit(usize::from(settings.axis.min(2)), 1.))
                .normalize_or_zero();
            let centre = p(DVec3::ZERO);
            out.push(Mark::Line([centre, centre + normal * 0.25 * short]));
            let edge = p(v * 0.5);
            let up = world.transform_vector3(v).normalize_or_zero();
            out.push(Mark::Line([edge, edge + up * 0.1 * short]));
        }
        P::Cylindrical => {
            out.extend(ring(axis, 0.5, 0.5, -0.5, 24, false));
            out.extend(ring(axis, 0.5, 0.5, 0.5, 24, false));
            for k in 0..4 {
                let t = k as f64 * std::f64::consts::FRAC_PI_2;
                let (x, y) = (0.5 * t.cos(), 0.5 * t.sin());
                out.push(Mark::Line([
                    q(DVec3::new(x, y, -0.5)),
                    q(DVec3::new(x, y, 0.5)),
                ]));
            }
        }
        P::Spherical => {
            out.extend(ring(axis, 0.5, 0.5, 0., 24, false));
            let side = axis
                * glam::DMat4::from_cols(
                    DVec3::X.extend(0.),
                    DVec3::Z.extend(0.),
                    DVec3::Y.extend(0.),
                    glam::DVec4::W,
                );
            out.extend(ring(side, 0.5, 0.5, 0., 24, false));
            let front = axis
                * glam::DMat4::from_cols(
                    DVec3::Z.extend(0.),
                    DVec3::Y.extend(0.),
                    DVec3::X.extend(0.),
                    glam::DVec4::W,
                );
            out.extend(ring(front, 0.5, 0.5, 0., 24, false));
        }
        P::UvMap | P::ImplicitUv => {}
        _ => {
            // A unit cube.
            let corner = |i: u32| {
                DVec3::new(
                    if i & 1 == 0 { -0.5 } else { 0.5 },
                    if i & 2 == 0 { -0.5 } else { 0.5 },
                    if i & 4 == 0 { -0.5 } else { 0.5 },
                )
            };
            for i in 0..8u32 {
                for bit in [1, 2, 4] {
                    if i & bit == 0 {
                        out.push(Mark::Line([p(corner(i)), p(corner(i | bit))]));
                    }
                }
            }
        }
    }
    out
}

/// A solid line from `from` to `to` (local points in `m`) ending in a filled
/// arrowhead.
fn arrow(m: glam::DMat4, from: DVec3, to: DVec3) -> Vec<Mark> {
    let p = |v: DVec3| m.transform_point3(v);
    let d = to - from;
    let head = d * defaults::ARROWHEAD_SHARE;
    let base = to - head;
    let side = d.any_orthonormal_vector() * head.length() * 0.45;
    let other = d.normalize().cross(side);
    vec![
        Mark::Line([p(from), p(base)]),
        Mark::Fill(vec![p(to), p(base + side), p(base - side)]),
        Mark::Fill(vec![p(to), p(base + other), p(base - other)]),
    ]
}

/// True when a projected arrowhead fill is (nearly) flat on screen: almost
/// no area, a needle-sharp corner, or a non-finite point. egui feathers a
/// filled polygon by pushing each corner out along its miter, `0.5 /
/// sin(angle / 2)` pixels, so an arrowhead seen (nearly) edge-on paints a
/// spike hundreds of pixels long: the grey slab across the view.
fn fill_is_flat(points: &[Pos2]) -> bool {
    /// The sharpest corner kept filled, as sin(angle / 2): about 11.5°,
    /// which keeps egui's feather within 5 px of the corner.
    const MIN_HALF_ANGLE_SIN: f32 = 0.1;
    let n = points.len();
    if n < 3 || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return true;
    }
    // Shoelace formula: twice the signed area.
    let twice_area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    if twice_area.abs() * 0.5 < 0.5 {
        return true;
    }
    (0..n).any(|i| {
        let (prev, here, next) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
        let (e0, e1) = (here - prev, next - here);
        if e0.length() < 1e-3 || e1.length() < 1e-3 {
            return true;
        }
        // The same averaged edge normal egui mitres with.
        let normal = (e0.normalized().rot90() + e1.normalized().rot90()) / 2.0;
        normal.length() < MIN_HALF_ANGLE_SIN
    })
}

/// What a projected arrowhead fill paints: the filled polygon, or, when it
/// is flat on screen, its outline as line segments in the glyph's stroke
/// (the edge-on arrowhead still reads as a line).
fn fill_shapes(points: Vec<Pos2>, color: Color32, stroke: Stroke) -> Vec<egui::Shape> {
    if !fill_is_flat(&points) {
        return vec![egui::Shape::convex_polygon(points, color, Stroke::NONE)];
    }
    let finite = |p: &Pos2| p.x.is_finite() && p.y.is_finite();
    (0..points.len())
        .map(|i| [points[i], points[(i + 1) % points.len()]])
        .filter(|[a, b]| finite(a) && finite(b))
        .map(|seg| egui::Shape::line_segment(seg, stroke))
        .collect()
}

/// The camera glyph (CL-13): a movie-camera body with two reels and a lens
/// on the −Z side, `size` metres long (the caller keeps it a constant size
/// on screen). Selected: dotted frustum lines from the lens out to the film
/// frame at the Target Distance.
pub(super) fn camera_marks(
    world: glam::DMat4,
    camera: &operandi_core::camera_light::CameraSettings,
    size: f64,
    selected: bool,
) -> Vec<Mark> {
    // The glyph ignores the item's scale; the frame is the item's.
    let frame = Frame::from_matrix(world).map_or(world, |f| {
        glam::DMat4::from_cols(
            f.right.extend(0.0),
            f.up.extend(0.0),
            (-f.forward).extend(0.0),
            f.position.extend(1.0),
        )
    });
    let m = frame * glam::DMat4::from_scale(DVec3::splat(size));
    let p = |x: f64, y: f64, z: f64| m.transform_point3(DVec3::new(x, y, z));
    let mut out = Vec::new();
    let mut boxed = |x: f64, y0: f64, y1: f64, z0: f64, z1: f64| {
        let c = |z: f64| [p(-x, y0, z), p(x, y0, z), p(x, y1, z), p(-x, y1, z)];
        let (a, b) = (c(z0), c(z1));
        for k in 0..4 {
            out.push(Mark::Line([a[k], a[(k + 1) % 4]]));
            out.push(Mark::Line([b[k], b[(k + 1) % 4]]));
            out.push(Mark::Line([a[k], b[k]]));
        }
    };
    // Body behind the lens (the camera looks down −Z).
    boxed(0.18, -0.2, 0.15, 0.0, 0.6);
    // Lens hood widening toward −Z.
    let (n0, n1) = (0.09, 0.15);
    let near = [
        p(-n0, -n0, 0.0),
        p(n0, -n0, 0.0),
        p(n0, n0, 0.0),
        p(-n0, n0, 0.0),
    ];
    let far = [
        p(-n1, -n1, -0.25),
        p(n1, -n1, -0.25),
        p(n1, n1, -0.25),
        p(-n1, n1, -0.25),
    ];
    for k in 0..4 {
        out.push(Mark::Line([near[k], near[(k + 1) % 4]]));
        out.push(Mark::Line([far[k], far[(k + 1) % 4]]));
        out.push(Mark::Line([near[k], far[k]]));
    }
    // Two film reels on top, in the camera's YZ plane.
    for z in [0.15, 0.45] {
        let reel = m
            * glam::DMat4::from_translation(DVec3::new(0.0, 0.3, z))
            * glam::DMat4::from_rotation_y(std::f64::consts::FRAC_PI_2);
        out.extend(ring(reel, 0.14, 0.14, 0.0, 12, false));
    }
    if selected {
        let l = camera.focus_distance;
        let (hw, hh) = (
            l * camera.film_width / (2.0 * camera.focal_length),
            l * camera.film_height / (2.0 * camera.focal_length),
        );
        let q = |x: f64, y: f64, z: f64| frame.transform_point3(DVec3::new(x, y, z));
        let apex = q(0.0, 0.0, -0.25 * size);
        let edge = [
            q(-hw, -hh, -l),
            q(hw, -hh, -l),
            q(hw, hh, -l),
            q(-hw, hh, -l),
        ];
        for k in 0..4 {
            out.push(Mark::Dotted([apex, edge[k]]));
            out.push(Mark::Dotted([edge[k], edge[(k + 1) % 4]]));
        }
    }
    out
}

/// A light's glyph (CL-13), by kind. Directional, point and spot scale with
/// Display Size; the area light with its Width/Height and the item scale.
pub(crate) fn light_marks(world: glam::DMat4, light: &LightSettings, selected: bool) -> Vec<Mark> {
    let ds = light.stored.display_size.max(0.0);
    let m = world * glam::DMat4::from_scale(DVec3::splat(ds.max(1e-6)));
    match light.kind {
        LightKind::Directional => directional_marks(m),
        LightKind::Point => point_marks(m),
        LightKind::Spot => spot_marks(world, m, light, selected),
        // A portal is drawn like an area light: its outline, centre lines
        // and the arrow pointing into the room (LP-PO).
        LightKind::Area | LightKind::Portal => area_marks(world, light),
        LightKind::Cylinder => crate::light_tier2_ui::cylinder_marks(world, light),
        LightKind::Mesh => crate::light_tier2_ui::mesh_light_marks(world, light),
        LightKind::Photometric => crate::light_tier2_ui::photometric_marks(world, light),
        LightKind::Dome => crate::light_tier3_ui::dome_marks(world, light),
    }
}

/// Directional: a cylinder along local Z (two end circles and four lines)
/// and a solid line with a filled arrowhead the way the light travels (−Z).
fn directional_marks(m: glam::DMat4) -> Vec<Mark> {
    let r = defaults::DIRECTIONAL_GLYPH_DIAMETER / 2.0;
    let h = defaults::DIRECTIONAL_GLYPH_LENGTH / 2.0;
    let p = |x: f64, y: f64, z: f64| m.transform_point3(DVec3::new(x, y, z));
    let mut out = ring(m, r, r, h, 24, false);
    out.extend(ring(m, r, r, -h, 24, false));
    for (x, y) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        out.push(Mark::Line([p(x * r, y * r, h), p(x * r, y * r, -h)]));
    }
    out.extend(arrow(
        m,
        DVec3::new(0.0, 0.0, h),
        DVec3::new(0.0, 0.0, -h - defaults::DIRECTIONAL_ARROW_LENGTH),
    ));
    out
}

/// Point: a star of straight lines through the centre (three axes and the
/// four cube diagonals).
fn point_marks(m: glam::DMat4) -> Vec<Mark> {
    let h = defaults::POINT_GLYPH_SIZE / 2.0;
    let d = h / 3f64.sqrt();
    let p = |v: DVec3| m.transform_point3(v);
    let mut dirs = vec![DVec3::X * h, DVec3::Y * h, DVec3::Z * h];
    for (x, y) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        dirs.push(DVec3::new(x * d, y * d, d));
    }
    dirs.into_iter()
        .map(|v| Mark::Line([p(-v), p(v)]))
        .collect()
}

/// Spot: a lamp housing opening toward −Z (it doesn't follow the cone).
/// Selected: dotted cone lines whose spread is the Cone Angle, with the
/// Soft Edge's inner ring dotted too.
fn spot_marks(
    world: glam::DMat4,
    m: glam::DMat4,
    light: &LightSettings,
    selected: bool,
) -> Vec<Mark> {
    let l = defaults::SPOT_HOUSING_LENGTH;
    let p = |x: f64, y: f64, z: f64| m.transform_point3(DVec3::new(x, y, z));
    let (back, mouth) = (0.1, 0.25);
    let mut out = ring(m, back, back, l * 0.5, 16, false);
    out.extend(ring(m, mouth, mouth, -l * 0.5, 24, false));
    for (x, y) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        out.push(Mark::Line([
            p(x * back, y * back, l * 0.5),
            p(x * mouth, y * mouth, -l * 0.5),
        ]));
    }
    if selected {
        let len = defaults::SPOT_CONE_LENGTH;
        let q = |x: f64, y: f64, z: f64| world.transform_point3(DVec3::new(x, y, z));
        let outer = len * (light.cone_angle / 2.0).min(1.5).tan();
        let inner = len * light.spot_full_angle().min(1.5).tan();
        out.extend(ring(world, outer, outer, -len, 32, true));
        if inner > 1e-6 && inner < outer - 1e-6 {
            out.extend(ring(world, inner, inner, -len, 32, true));
        }
        for (x, y) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            out.push(Mark::Dotted([
                q(0.0, 0.0, 0.0),
                q(x * outer, y * outer, -len),
            ]));
        }
    }
    out
}

/// Area: its rectangle or ellipse (Width × Height in local XY, stretched by
/// the item's scale), dotted centre lines, and a solid line with a filled
/// arrowhead along −Z.
fn area_marks(world: glam::DMat4, light: &LightSettings) -> Vec<Mark> {
    let (w, h) = (light.width / 2.0, light.height / 2.0);
    let p = |x: f64, y: f64, z: f64| world.transform_point3(DVec3::new(x, y, z));
    let mut out = match light.shape {
        AreaShape::Rectangular => {
            let c = [p(-w, -h, 0.0), p(w, -h, 0.0), p(w, h, 0.0), p(-w, h, 0.0)];
            (0..4).map(|k| Mark::Line([c[k], c[(k + 1) % 4]])).collect()
        }
        AreaShape::Elliptical => ring(world, w, h, 0.0, 32, false),
    };
    out.push(Mark::Dotted([p(-w, 0.0, 0.0), p(w, 0.0, 0.0)]));
    out.push(Mark::Dotted([p(0.0, -h, 0.0), p(0.0, h, 0.0)]));
    let n = (w.min(h) * 2.0 * defaults::AREA_ARROW_SHARE).max(0.05);
    out.extend(arrow(world, DVec3::ZERO, DVec3::new(0.0, 0.0, -n)));
    out
}

/// Where the render picture sits in a Camera view of `r`: the render
/// frame's shape, as large as fits, centred.
#[cfg(test)]
pub(crate) fn render_frame_rect(r: Rect) -> Rect {
    let [fw, fh] = defaults::RENDER_FRAME;
    crate::camera_props_ui::frame_rect_for(r, fw as f32 / fh.max(1) as f32)
}

/// The picture size renders through camera `id` use (its Resolution
/// Override, else the render frame), CP-V17.
pub(crate) fn camera_render_size(scene: &operandi_core::Scene, id: u64) -> [u32; 2] {
    scene
        .item(id)
        .and_then(|i| i.camera.as_ref())
        .map_or(scene.render.frame, |c| c.render_size(scene.render.frame))
}

/// CL-17: the camera a Camera view looks through: the selected camera, or
/// else the newest one.
pub(super) fn view_camera(scene: &operandi_core::Scene) -> Option<u64> {
    if let Some(item) = scene.active().filter(|i| i.camera.is_some()) {
        return Some(item.id);
    }
    scene
        .items
        .iter()
        .filter(|i| i.camera.is_some())
        .map(|i| i.id)
        .max()
}

/// The selected light, if the primary selected item is one.
pub(super) fn selected_light(scene: &operandi_core::Scene) -> Option<u64> {
    scene
        .selection
        .item
        .filter(|&id| scene.item(id).is_some_and(|i| i.light.is_some()))
}

/// Light view's lens. A spot light shows its whole cone across the view's
/// shorter side, so the view shows what it lights; every other light (and
/// a spot too wide to look through) uses a new view's lens (provisional,
/// LV-Q2).
pub(super) fn light_view_lens(light: &operandi_core::camera_light::LightSettings, r: Rect) -> f64 {
    let half = light.cone_angle / 2.;
    if light.kind == operandi_core::camera_light::LightKind::Spot
        && half.is_finite()
        && (0.5f64.to_radians()..=80f64.to_radians()).contains(&half)
        && r.is_positive()
    {
        // `lens` is the view's height at unit depth: half the height spans
        // 0.5 / lens. Fit the cone's half angle to the shorter half side.
        let aspect = f64::from(r.width() / r.height());
        return 0.5 * aspect.min(1.) / half.tan();
    }
    crate::camera::default_lens()
}

/// A small render marker (a filled dot in a ring) for the render camera's
/// Items row (CL-21).
pub(super) fn paint_render_marker(ui: &Ui, centre: Pos2, color: Color32) {
    ui.painter()
        .circle_stroke(centre, 4., Stroke::new(1.2f32, color));
    ui.painter().circle_filled(centre, 2., color);
}

impl Operandi {
    /// The camera this view looks through in Camera view: the one chosen
    /// for it (Camera Item, or a script's `view_set`), else the selected or
    /// newest camera.
    pub(super) fn through_camera(&self) -> Option<u64> {
        let scene = &self.editor.scene;
        self.camera
            .through_item
            .filter(|id| {
                scene
                    .items
                    .iter()
                    .any(|i| i.id == *id && i.camera.is_some())
            })
            .or_else(|| view_camera(scene))
    }

    /// What the view header and view menu call this view: the camera's
    /// name in Camera view, with " · Render Camera" when it is the render
    /// camera (CL-18).
    pub(super) fn view_title(&self) -> String {
        if let Some(id) = self.through_light() {
            // Light view: the light's name.
            return self
                .editor
                .scene
                .item(id)
                .map_or_else(|| self.camera.view_label(), |i| i.name.clone());
        }
        if !self.camera.through {
            return self.camera.view_label();
        }
        let scene = &self.editor.scene;
        let Some(id) = self.through_camera() else {
            return self.camera.view_label();
        };
        let name = scene.item(id).map_or("Camera", |i| i.name.as_str());
        if scene.render_camera() == Some(id) {
            format!("{name} · Render Camera")
        } else {
            name.to_owned()
        }
    }

    /// Camera view: dim what lies outside the render picture and outline
    /// it, so what's inside the frame is what Render Viewport renders.
    pub(super) fn draw_render_frame(&self, painter: &egui::Painter, r: Rect) {
        if !self.camera.through {
            return;
        }
        let Some(id) = self.through_camera() else {
            return;
        };
        let [fw, fh] = camera_render_size(&self.editor.scene, id);
        let f = crate::camera_props_ui::frame_rect_for(r, fw as f32 / fh.max(1) as f32);
        let shade = theme::viewport::camera_frame_shade();
        for band in [
            Rect::from_min_max(r.min, pos2(r.max.x, f.min.y)),
            Rect::from_min_max(pos2(r.min.x, f.max.y), r.max),
            Rect::from_min_max(pos2(r.min.x, f.min.y), pos2(f.min.x, f.max.y)),
            Rect::from_min_max(pos2(f.max.x, f.min.y), pos2(r.max.x, f.max.y)),
        ] {
            if band.width() > 0.5 && band.height() > 0.5 {
                painter.rect_filled(band, 0., shade);
            }
        }
        painter.rect_stroke(
            f,
            0.,
            Stroke::new(1.0f32, theme::viewport::locator()),
            egui::StrokeKind::Inside,
        );
    }

    /// Draw every visible camera and light this view shows (CL-Q16). The
    /// selected one uses the selection colour; the camera this view looks
    /// through is not drawn.
    pub(super) fn draw_locators(&self, painter: &egui::Painter, r: Rect) {
        // IK-50: the Pose tool's circles.
        self.draw_pose_handles(painter, r);
        let scene = &self.editor.scene;
        let drawing = &self.display.settings.drawing;
        let through = self.camera.through.then(|| self.through_camera()).flatten();
        // Light view: the light the view looks through isn't drawn either.
        let through_light = self.through_light();
        let projector = self.camera.projector(r);
        // Item Drawing size (Preferences > Display): camera and light
        // glyphs draw this much larger.
        let grow = scene.item_draw_size;
        // Texture Locators draw when selected, or when a selected Shading
        // layer uses them (provisional, TL-Q20: all of them would clutter),
        // or all of them with Show Texture Locators on.
        let shown_locators: Vec<u64> = self
            .shading
            .selected
            .iter()
            .filter_map(|&id| match &scene.shader_tree.get(id)?.kind {
                operandi_core::shader_tree::NodeKind::Texture(t) => t.locator,
                _ => None,
            })
            .collect();
        for item in scene.items.iter().filter(|i| scene.shows(i)) {
            if let Some(settings) = item.texture_locator() {
                if self.brush_hides_locators() {
                    continue;
                }
                let selected = scene.selection.item == Some(item.id);
                // Show Texture Locators (Viewport Properties > Visibility)
                // draws every one.
                if !selected && !shown_locators.contains(&item.id) && !drawing.texture_locators {
                    continue;
                }
                // In the space the render places it (World Coordinates off:
                // through its surface item, PD-Q07).
                let Some(world) = self.locator_render_matrix(item.id) else {
                    continue;
                };
                // Planar spec: the selected-item colour when the locator or
                // its layer is selected; Show Texture Locators alone keeps
                // the locator colour.
                let color = if selected || shown_locators.contains(&item.id) {
                    theme::viewport::selection()
                } else {
                    theme::viewport::locator()
                };
                let stroke = Stroke::new(1.0f32, color);
                for mark in texture_locator_marks(world, settings) {
                    let (seg, dotted) = match mark {
                        Mark::Line(seg) => (seg, false),
                        Mark::Dotted(seg) => (seg, true),
                        Mark::Fill(_) => continue,
                    };
                    if let (Some((a, _)), Some((b, _))) =
                        (projector.project(seg[0]), projector.project(seg[1]))
                    {
                        if dotted {
                            painter.extend(egui::Shape::dashed_line(&[a, b], stroke, 3.0, 3.0));
                        } else {
                            painter.line_segment([a, b], stroke);
                        }
                    }
                }
                continue;
            }
            // KIT-1: an item whose kit is missing draws as its bounding
            // box, dashed, under Show Locators (provisional).
            if let Some(kit) = &item.kit {
                if let Some([lo, hi]) = kit.bounds
                    && scene.drawn_in_viewport(item.id, drawing.locators)
                    && let Ok(world) = scene.world_matrix(item.id)
                {
                    let selected = scene.selection.has_item(item.id);
                    let color = if selected {
                        theme::viewport::selection()
                    } else {
                        theme::viewport::locator()
                    };
                    let corner = |i: usize| {
                        world.transform_point3(DVec3::new(
                            if i & 1 == 0 { lo.x } else { hi.x },
                            if i & 2 == 0 { lo.y } else { hi.y },
                            if i & 4 == 0 { lo.z } else { hi.z },
                        ))
                    };
                    for a in 0..8usize {
                        for bit in [1, 2, 4] {
                            if a & bit != 0 {
                                continue;
                            }
                            if let (Some((p, _)), Some((q, _))) = (
                                projector.project(corner(a)),
                                projector.project(corner(a | bit)),
                            ) {
                                painter.extend(egui::Shape::dashed_line(
                                    &[p, q],
                                    Stroke::new(1.0f32, color),
                                    4.0,
                                    3.0,
                                ));
                            }
                        }
                    }
                }
                continue;
            }
            if item.camera.is_none() && item.light.is_none() && item.locator.is_none() {
                continue;
            }
            let selected = scene.selection.item == Some(item.id);
            // CP-D01: Visible Default follows Show Cameras / Show Lights /
            // Show Locators (GL-11), Yes always draws, No never.
            let kind_shown = if item.camera.is_some() {
                drawing.cameras
            } else if item.light.is_some() {
                drawing.lights
            } else {
                drawing.locators
            };
            if !scene.drawn_in_viewport(item.id, kind_shown) {
                continue;
            }
            let marks = match (&item.camera, &item.light) {
                (Some(camera), _) if through != Some(item.id) => {
                    let Ok(world) = scene.world_matrix(item.id) else {
                        continue;
                    };
                    // Display Size 0: a constant size on screen.
                    let at = world.transform_point3(DVec3::ZERO);
                    let size = if camera.stored.display_size > 0.0 {
                        camera.stored.display_size
                    } else {
                        let Some(mpp) = self.camera.metres_per_pixel(at, r) else {
                            continue;
                        };
                        mpp * defaults::CAMERA_GLYPH_POINTS
                    };
                    // CP-V10 Set Focus Distance: the frustum ends where
                    // renders focus.
                    let focus = scene.camera_focus_distance(item.id);
                    if focus.is_some_and(|d| d != camera.focus_distance) {
                        let mut shown = camera.clone();
                        shown.focus_distance = focus.unwrap_or(camera.focus_distance);
                        camera_marks(world, &shown, size * grow, selected)
                    } else {
                        camera_marks(world, camera, size * grow, selected)
                    }
                }
                (_, Some(light)) if through_light != Some(item.id) => {
                    let Ok(world) = scene.world_matrix(item.id) else {
                        continue;
                    };
                    // A dot in the light's own colour at its centre.
                    if let Some((c, _)) = projector.project(world.transform_point3(DVec3::ZERO)) {
                        let [r, g, b] = light.color.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8);
                        painter.circle_filled(c, 2.5, Color32::from_rgb(r, g, b));
                    }
                    if grow == 1. {
                        light_marks(world, light, selected)
                    } else {
                        let mut grown = light.clone();
                        grown.stored.display_size *= grow;
                        light_marks(world, &grown, selected)
                    }
                }
                _ if item.locator == Some(operandi_core::locator::LocatorKind::IkGoal) => {
                    let Ok(world) = scene.world_matrix(item.id) else {
                        continue;
                    };
                    ik_goal_marks(world, ik_goal_size(scene, item.id))
                }
                // ACS-30: a character rig's guide.
                _ if item.locator == Some(operandi_core::locator::LocatorKind::Guide) => {
                    let at = operandi_core::character_rig::world_pos(scene, item.id);
                    let Some(mpp) = self.camera.metres_per_pixel(at, r) else {
                        continue;
                    };
                    crate::character_rig_ui::guide_item_marks(
                        scene,
                        item.id,
                        mpp * defaults::GROUP_LOCATOR_GLYPH_POINTS * 0.6 * grow,
                    )
                }
                _ if item.is_joint() => {
                    let Ok(world) = scene.world_matrix(item.id) else {
                        continue;
                    };
                    let at = world.w_axis.truncate();
                    let Some(mpp) = self.camera.metres_per_pixel(at, r) else {
                        continue;
                    };
                    let kids: Vec<DVec3> = operandi_core::skeleton::children(scene, item.id)
                        .into_iter()
                        .filter_map(|c| scene.world_matrix(c).ok())
                        .map(|m| m.w_axis.truncate())
                        .collect();
                    let mut marks = joint_marks(
                        world,
                        &kids,
                        mpp * defaults::GROUP_LOCATOR_GLYPH_POINTS * 0.5 * grow,
                    );
                    // IK-43: a selected limited joint shows its allowed arcs.
                    if selected
                        && let Some(ik) = operandi_core::ik_joint::settings(scene, item.id)
                            .filter(|j| j.limited())
                    {
                        let size = kids.first().map_or(mpp * 30., |c| c.distance(at) * 0.4);
                        let parent = item
                            .parent
                            .and_then(|p| scene.world_matrix(p).ok())
                            .unwrap_or(glam::DMat4::IDENTITY);
                        marks.extend(limit_fan_marks(parent, at, ik, size));
                    }
                    marks
                }
                _ if item.is_group_locator() => {
                    let Ok(world) = scene.world_matrix(item.id) else {
                        continue;
                    };
                    // A constant size on screen, like a camera at Display
                    // Size 0 (GL-10).
                    let at = world.transform_point3(DVec3::ZERO);
                    let Some(mpp) = self.camera.metres_per_pixel(at, r) else {
                        continue;
                    };
                    group_locator_marks(world, mpp * defaults::GROUP_LOCATOR_GLYPH_POINTS * grow)
                }
                _ => continue,
            };
            let color = if selected {
                theme::viewport::selection()
            } else {
                // CP-D07: Draw Options' wire colour.
                crate::item_extras_ui::draw_options_color(item)
                    .unwrap_or_else(theme::viewport::locator)
            };
            // IK-02: a goal is coloured by its side of X.
            let color = if !selected
                && item.locator == Some(operandi_core::locator::LocatorKind::IkGoal)
            {
                ik_goal_color(scene, item.id)
            } else if !selected && item.locator == Some(operandi_core::locator::LocatorKind::Guide)
            {
                crate::character_rig_ui::guide_color(scene, item.id)
            } else {
                color
            };
            let stroke = Stroke::new(if selected { 1.5f32 } else { 1.0 }, color);
            let project = |a: DVec3, b: DVec3| match (projector.project(a), projector.project(b)) {
                (Some((a, _)), Some((b, _))) => Some([a, b]),
                _ => None,
            };
            for mark in marks {
                match mark {
                    Mark::Line([a, b]) => {
                        if let Some(seg) = project(a, b) {
                            painter.line_segment(seg, stroke);
                        }
                    }
                    Mark::Dotted([a, b]) => {
                        if let Some(seg) = project(a, b) {
                            painter.extend(egui::Shape::dashed_line(&seg, stroke, 3.0, 3.0));
                        }
                    }
                    Mark::Fill(points) => {
                        let points: Option<Vec<Pos2>> = points
                            .into_iter()
                            .map(|p| projector.project(p).map(|p| p.0))
                            .collect();
                        if let Some(points) = points {
                            painter.extend(fill_shapes(points, color, stroke));
                        }
                    }
                }
            }
        }
    }

    /// Camera view (CL-Q11), once per frame before the view draws: take the
    /// render camera's placement and lens into this view's camera, or, when
    /// the view was navigated since, move the camera item to match (written
    /// on release as one undo step).
    pub(super) fn sync_camera_view(&mut self, r: Rect) {
        if self.camera.light.is_some() {
            self.sync_light_view(r);
            return;
        }
        if !self.camera.through {
            return;
        }
        let Some(id) = self.through_camera() else {
            self.leave_camera_view();
            self.status = "Camera view needs a camera in the scene".into();
            return;
        };
        let scene = &self.editor.scene;
        let (Some(item), Some(frame)) = (scene.item(id), item_frame(scene, id)) else {
            self.leave_camera_view();
            return;
        };
        let Some(settings) = item.camera.clone() else {
            return;
        };
        let navigated = self
            .camera
            .synced
            .is_some_and(|s| s != self.camera.placement());
        if navigated {
            // Wait for the gesture to end; the view itself shows the move.
            if !self.navigation_settled() {
                return;
            }
            self.write_camera_view(id);
            // `frame` and `settings` were read before the write moved the
            // camera; re-read them so the view doesn't flash back to the old
            // placement. If the write failed, the item is unchanged and the
            // re-read gives the same placement as before.
            let scene = &self.editor.scene;
            let moved = item_frame(scene, id).zip(scene.item(id).and_then(|i| i.camera.clone()));
            if let Some((frame, settings)) = moved {
                self.camera_from_item(&frame, &settings, r);
                return;
            }
        }
        self.camera_from_item(&frame, &settings, r);
    }

    /// Put this view's camera where the camera item is, with its lens.
    fn camera_from_item(
        &mut self,
        frame: &Frame,
        settings: &operandi_core::camera_light::CameraSettings,
        r: Rect,
    ) {
        let toward = -frame.forward;
        // The view shows the render picture (the frame's shape) as large
        // as fits, with the lens worked out for that picture; the rest of
        // the view is extra seen around it, so the framing matches the
        // render.
        let [fw, fh] = settings.render_size(self.editor.scene.render.frame);
        let aspect = f64::from(fw) / f64::from(fh.max(1));
        let picture = crate::camera_props_ui::frame_rect_for(r, aspect as f32);
        let grow = f64::from(r.height() / picture.height().max(1.));
        let c = &mut self.camera;
        c.distance = settings.target_distance;
        c.target = frame.position + frame.forward * settings.target_distance;
        (c.yaw, c.pitch) = crate::camera::Camera::yaw_pitch_of(toward);
        c.roll = 0.;
        match settings.projection {
            // Panoramas render only; the view shows them as Perspective.
            Projection::Perspective
            | Projection::Cylindrical
            | Projection::CylindricalVr
            | Projection::Spherical
            | Projection::SphericalVr => {
                c.view = View::Perspective;
                c.lens = 0.5 / (settings.half_tangents(aspect).1 * grow);
            }
            Projection::Orthographic => {
                c.view = View::Orthographic;
                c.lens = 0.5 * c.distance / (settings.orthographic_half_size(aspect).1 * grow);
            }
        }
        c.synced = Some(c.placement());
        c.settling = None;
    }

    /// Whether navigation in Camera or Light view has ended, so the item
    /// can be moved to match (one undo step per gesture). A pointer drag
    /// ends on release. A wheel, trackpad or 3D mouse move holds nothing,
    /// so it ends once the view has held still for `SETTLE`; writing every
    /// frame made each frame an undo step and rebuilt scene caches
    /// (camera view navigation was very slow on heavy scenes).
    fn navigation_settled(&mut self) -> bool {
        let now = std::time::Instant::now();
        let placement = self.camera.placement();
        if self.pointer_held || self.navigation.is_some() {
            // Already settled: the release writes at once.
            let past = now.checked_sub(SETTLE).unwrap_or(now);
            self.camera.settling = Some((placement, past));
            return false;
        }
        match self.camera.settling {
            Some((seen, since)) if seen == placement => now.duration_since(since) >= SETTLE,
            _ => {
                self.camera.settling = Some((placement, now));
                false
            }
        }
    }

    /// How long until a waiting Camera or Light view move is written, for
    /// a repaint then (the view may be idle by that time). `None` when
    /// nothing waits, or a press is still held.
    pub(super) fn camera_view_settle_in(&self) -> Option<std::time::Duration> {
        let (_, since) = self.camera.settling?;
        (!self.pointer_held && self.navigation.is_none())
            .then(|| SETTLE.saturating_sub(since.elapsed()))
    }

    /// Write a Camera or Light view move that is still waiting to settle,
    /// now: before Undo or Redo (which would otherwise act on the step
    /// before it) and on leaving the view.
    pub(super) fn flush_camera_view(&mut self) {
        if self.camera.settling.is_none() || self.pointer_held || self.navigation.is_some() {
            return;
        }
        let id = if self.camera.light.is_some() {
            self.through_light()
        } else if self.camera.through {
            self.through_camera()
        } else {
            None
        };
        let navigated = self
            .camera
            .synced
            .is_some_and(|s| s != self.camera.placement());
        if let Some(id) = id.filter(|_| navigated) {
            self.write_camera_view(id);
            self.camera.synced = Some(self.camera.placement());
        }
        self.camera.settling = None;
    }

    /// Light view, once per frame before the view draws: the light Camera
    /// view's way. Take the light's placement into this view, or, when the
    /// view was navigated since, move the light to match (written on
    /// release as one undo step, the same `CameraView` command).
    fn sync_light_view(&mut self, r: Rect) {
        let Some(id) = self.through_light() else {
            self.leave_camera_view();
            self.status = "Light view needs a light in the scene".into();
            return;
        };
        if let Some(view) = self.camera.light.as_mut()
            && view.item != id
        {
            // (Current Selection) moved to another light: start afresh.
            view.item = id;
            self.camera.synced = None;
        }
        let navigated = self
            .camera
            .synced
            .is_some_and(|s| s != self.camera.placement());
        if navigated {
            if !self.navigation_settled() {
                return;
            }
            self.write_camera_view(id);
        }
        let scene = &self.editor.scene;
        let (Some(frame), Some(light)) = (
            item_frame(scene, id),
            scene.item(id).and_then(|i| i.light.clone()),
        ) else {
            self.leave_camera_view();
            return;
        };
        let c = &mut self.camera;
        // A light keeps no Target Distance. Orbit and dolly turn about a
        // point straight ahead of it: on entering, where its aim passes the
        // previous view's centre; after that, as far ahead as navigation
        // left it (provisional, LV-Q1).
        if c.synced.is_none() {
            let ahead = (c.target - frame.position).dot(frame.forward);
            c.distance = if ahead > crate::camera::MIN_DISTANCE {
                ahead
            } else {
                c.distance.max(1.)
            };
        }
        c.target = frame.position + frame.forward * c.distance;
        (c.yaw, c.pitch) = crate::camera::Camera::yaw_pitch_of(-frame.forward);
        c.roll = 0.;
        c.view = View::Perspective;
        c.lens = light_view_lens(&light, r);
        c.synced = Some(c.placement());
        c.settling = None;
    }

    /// Move camera or light `id` to this view's placement: one undo step.
    fn write_camera_view(&mut self, id: u64) {
        let (eye, right, up, forward) = self.camera.basis();
        let rotation = glam::DQuat::from_mat3(&glam::DMat3::from_cols(right, up, -forward));
        let result = local_transform_for(&self.editor.scene, id, eye, rotation).and_then(|t| {
            self.editor.dispatch(Command::CameraView {
                item: id,
                transform: t,
                target_distance: self.camera.distance,
            })
        });
        let what = if self.camera.light.is_some() {
            "Light"
        } else {
            "Camera"
        };
        match result {
            Ok(()) => self.status = format!("{what} moved"),
            Err(e) => self.status = format!("{what} view: {e}"),
        }
    }

    /// Back to an ordinary Perspective view from Camera or Light view.
    pub(super) fn leave_camera_view(&mut self) {
        self.flush_camera_view();
        // A view's own lens carries its world scale (Maximise keeps it);
        // only Camera and Light view's lens came from the item.
        if self.camera.looks_through() {
            self.camera.lens = crate::camera::default_lens();
        }
        self.camera.through = false;
        self.camera.synced = None;
        self.camera.through_item = None;
        self.camera.light = None;
        if self.camera.view == View::Orthographic {
            self.camera.view = View::Perspective;
        }
    }

    /// Perspective View (numpad ".", SC-054 "Perspective (press again:
    /// camera)"): any other view goes to Perspective; Perspective goes to
    /// Camera view through the scene's camera (CL-17); Camera or Light
    /// view goes back to Perspective. With no camera it stays Perspective
    /// and the status line says why.
    pub(crate) fn perspective_or_camera_view(&mut self) {
        if self.camera.looks_through() {
            self.leave_camera_view();
            self.camera.view = View::Perspective;
        } else if self.camera.view != View::Perspective {
            self.camera.view = View::Perspective;
        } else if view_camera(&self.editor.scene).is_some() {
            self.enter_camera_view();
        } else {
            self.status = "Perspective view: no camera in the scene to look through".into();
        }
    }

    /// The view menu entry for Camera view.
    pub(super) fn enter_camera_view(&mut self) {
        if view_camera(&self.editor.scene).is_none() {
            self.status = "Camera view needs a camera in the scene".into();
            return;
        }
        self.camera.light = None;
        self.camera.through = true;
        self.camera.synced = None;
        self.camera.through_item = None;
    }

    /// The light this view looks through in Light view: the one picked
    /// from the menu, or with (Current Selection) the selected light (the
    /// last one shown while something else is selected). `None` outside
    /// Light view, or when that light is gone.
    pub(super) fn through_light(&self) -> Option<u64> {
        let view = self.camera.light?;
        let scene = &self.editor.scene;
        let is_light = |id: u64| scene.item(id).is_some_and(|i| i.light.is_some());
        view.follow_selection
            .then(|| selected_light(scene))
            .flatten()
            .or_else(|| is_light(view.item).then_some(view.item))
    }

    /// The view menu's Light entries: look through light `item`, or with
    /// `None` (Current Selection) the selected light.
    pub(super) fn enter_light_view(&mut self, item: Option<u64>) {
        let scene = &self.editor.scene;
        let id = match item {
            Some(id) if scene.item(id).is_some_and(|i| i.light.is_some()) => id,
            Some(id) => {
                self.status = format!("Item {id} is not a light");
                return;
            }
            None => match selected_light(scene) {
                Some(id) => id,
                None => {
                    self.status = "Select a light to look through".into();
                    return;
                }
            },
        };
        if self.camera.through {
            self.leave_camera_view();
        }
        self.camera.light = Some(crate::camera::LightView {
            follow_selection: item.is_none(),
            item: id,
        });
        self.camera.synced = None;
    }

    /// Camera view through camera `id`, or Light view through light `id`
    /// (MCP `view_set`).
    pub(super) fn look_through(&mut self, id: u64) -> std::result::Result<(), String> {
        if self
            .editor
            .scene
            .item(id)
            .is_some_and(|i| i.light.is_some())
        {
            self.enter_light_view(Some(id));
            return Ok(());
        }
        if !self
            .editor
            .scene
            .items
            .iter()
            .any(|i| i.id == id && i.camera.is_some())
        {
            return Err(format!("Item {id} is not a camera or light"));
        }
        self.camera.light = None;
        self.camera.through = true;
        self.camera.synced = None;
        self.camera.through_item = Some(id);
        Ok(())
    }

    /// A number control shown in `scale` units (1000 for mm, 100 for %,
    /// degrees for angles via `to_shown`). One undo step when the drag or
    /// typing ends.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn locator_number(
        &mut self,
        ui: &mut Ui,
        item: u64,
        key: &'static str,
        label: &str,
        value: f64,
        range: std::ops::RangeInclusive<f64>,
        suffix: &str,
        scale: f64,
        make: impl Fn(f64) -> Command,
    ) {
        let drafting = matches!(self.locator_draft, Some((i, k, _)) if i == item && k == key);
        let mut shown = match self.locator_draft {
            Some((i, k, v)) if i == item && k == key => v,
            _ => value * scale,
        };
        self.probe_light_row(&make(value));
        if drafting {
            crate::form::rows::mixed(false);
        }
        let response = ui
            .horizontal(|ui| {
                crate::option_row::label(ui, label);
                crate::option_row::fill(ui, |ui| {
                    ui.add(
                        crate::numeric_tab::DragValue::new(&mut shown)
                            .range(range)
                            .speed(0.1)
                            .max_decimals(3)
                            .suffix(suffix),
                    )
                })
            })
            .inner;
        if response.changed() {
            self.locator_draft = Some((item, key, shown));
        }
        let finished = response.drag_stopped()
            || response.lost_focus()
            || (response.changed() && !response.dragged() && !response.has_focus());
        if finished
            && let Some((i, k, v)) = self.locator_draft
            && i == item
            && k == key
        {
            self.locator_draft = None;
            if (v / scale - value).abs() > 1e-12 {
                self.run_light(make(v / scale));
            }
        }
    }

    /// The light's Properties tab, rows in the LP-23 order: Render,
    /// Dissolve, intensity (labelled per kind, LP-12), Light Color, Falloff
    /// Type, Shadow Type; a divider; the size rows; Visible to Camera /
    /// Reflection / Refraction; spot cone rows after another divider. A
    /// portal shows only Render, Shape, Width, Height and Environment Only
    /// (LP-PO). The light material section follows (LM-10..16). Every
    /// change is one undo step named after its row and applies to every
    /// selected light (LP-51).
    pub(super) fn light_properties(&mut self, ui: &mut Ui, item: &operandi_core::Item) {
        let Some(l) = item.light.clone() else {
            return;
        };
        let id = item.id;
        let idle = self.editor.session.live.is_none();
        // LP-04: Targeting rows above the light's own section.
        if l.kind.has_targeting() {
            crate::form::rows::section(ui, "Targeting", false, |ui| {
                ui.gated_ui(
                    crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 2.;
                        self.targeting_rows(ui, item);
                    },
                );
            });
        }
        crate::form::rows::section(ui, l.kind.tab_name(), false, |ui| {
            ui.gated_ui(
                crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.;
                    self.light_rows(ui, id, &l);
                },
            );
        });
        // LP-SUN: a directional light's Physical Sun section (collapsed,
        // LP-02).
        if l.kind == LightKind::Directional {
            crate::form::rows::section(ui, "Physical Sun", true, |ui| {
                ui.gated_ui(
                    crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 2.;
                        self.physical_sun_rows(ui, id, &l);
                        self.sun_extra_rows(ui, id, &l);
                    },
                );
            });
        }
        // LP-30..32: Light Linking (collapsed, LP-02), every kind.
        crate::form::rows::section(ui, "Light Linking", true, |ui| {
            ui.gated_ui(
                crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.;
                    self.light_linking_rows(ui, id, &l);
                },
            );
        });
        // LP-40: Volumetrics (collapsed, LP-02).
        if l.kind.has_volumetrics() {
            crate::form::rows::section(ui, "Volumetrics", true, |ui| {
                ui.gated_ui(
                    crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 2.;
                        self.volumetric_rows(ui, id, &l);
                    },
                );
            });
        }
        if l.kind.emits() {
            crate::form::rows::section(ui, "Light Material", false, |ui| {
                ui.gated_ui(
                    crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 2.;
                        self.light_material_extra_rows(ui, id, &l);
                        self.light_material_rows(ui, id, &l);
                    },
                );
            });
        }
        // Channels stored with the light but not used yet (samples,
        // importance, shadow colour…) are left out until built (UIC-117).
    }

    /// A dropdown row editing one light setting.
    /// Before a light row: its status circle and "(mixed)" (UIC-110,
    /// UIC-118), worked out by trying `c` (the row's edit at the shown
    /// value) on a new light of the same kind and on the other selected
    /// lights. An edit that would change another light means it differs.
    /// A Light Properties row's edit: through its command (item.channel,
    /// item.setType, light.temperature or light.mesh; UIC-164/165) when it
    /// has one, else straight to the editor.
    pub(super) fn run_light(&mut self, c: Command) {
        if let Command::EditLight { item, edit } = &c
            && let Some(line) = operandi_core::channels::light_line(*item, edit)
        {
            self.run_command_line(&line);
            return;
        }
        self.run(c);
    }

    pub(super) fn probe_light_row(&self, c: &Command) {
        let Command::EditLight { item, edit } = c else {
            return;
        };
        let lights: Vec<(u64, LightSettings)> = self
            .common_targets()
            .into_iter()
            .filter_map(|id| {
                self.editor
                    .scene
                    .item(id)
                    .and_then(|i| i.light.clone().map(|l| (id, l)))
            })
            .collect();
        let Some((_, primary)) = lights.iter().find(|(id, _)| id == item) else {
            return;
        };
        let changes = |l: &LightSettings| {
            let mut x = l.clone();
            x.apply(edit.clone()).is_ok() && x != *l
        };
        let mixed = lights.iter().any(|(id, l)| id != item && changes(l));
        let fresh = LightSettings::new(primary.kind);
        crate::form::rows::status(changes(&fresh));
        // UIC-112: a keyable light channel gets its key light.
        self.anim_row_for(c, changes(&fresh));
        crate::form::rows::mixed(mixed);
    }

    fn light_choice<T: Copy + PartialEq>(
        &mut self,
        ui: &mut Ui,
        id: u64,
        label: &str,
        value: T,
        options: &[(T, &str)],
        make: impl Fn(T) -> LightEdit,
    ) -> egui::Response {
        let mut v = value;
        self.probe_light_row(&Command::EditLight {
            item: id,
            edit: make(value),
        });
        let r =
            crate::option_row::choice(ui, &format!("light-{label}-{id}"), label, &mut v, options);
        if r.inner {
            self.run_light(Command::EditLight {
                item: id,
                edit: make(v),
            });
        }
        r.response
    }

    fn light_rows(&mut self, ui: &mut Ui, id: u64, l: &LightSettings) {
        let edit = |edit: LightEdit| Command::EditLight { item: id, edit };
        let deg = 180. / std::f64::consts::PI;
        // Switches the light's type, keeping what still applies (MCP-Q02).
        let kinds = LightKind::ALL.map(|k| (k, k.label()));
        self.light_choice(ui, id, "Type", l.kind, &kinds, LightEdit::Kind)
            .on_hover_text("Keeps colour, intensity, shadow and settings both types share");
        let render = LightRender::ALL.map(|v| (v, v.label()));
        self.light_choice(ui, id, "Render", l.render, &render, LightEdit::Render)
            .on_hover_text(
                "Default: renders while the Items-list eye is on; Yes: always; No: never",
            );
        if l.kind.emits() {
            self.locator_number(
                ui,
                id,
                "dissolve",
                "Dissolve",
                l.dissolve,
                0.0..=100.0,
                " %",
                100.,
                |v| edit(LightEdit::Dissolve(v)),
            );
            self.locator_number(
                ui,
                id,
                "intensity",
                l.kind.intensity_label(),
                l.intensity,
                0.0..=1e8,
                l.kind.intensity_unit(),
                1.,
                |v| edit(LightEdit::Intensity(v)),
            );
            self.light_color_row(ui, id, l, "Light Color");
            if !matches!(l.kind, LightKind::Directional | LightKind::Dome) {
                let falloff = LightFalloff::ALL.map(|v| (v, v.label()));
                self.light_choice(
                    ui,
                    id,
                    "Falloff Type",
                    l.falloff,
                    &falloff,
                    LightEdit::Falloff,
                )
                .on_hover_text("Cycles renders every falloff as inverse square (provisional)");
            }
            let shadow = ShadowType::ALL.map(|v| (v, v.label()));
            self.light_choice(ui, id, "Shadow Type", l.shadow, &shadow, LightEdit::Shadow);
        }
        ui.separator();
        match l.kind {
            LightKind::Directional => self.locator_number(
                ui,
                id,
                "spread",
                "Spread Angle",
                l.spread_angle,
                0.0..=180.0,
                "°",
                deg,
                |v| edit(LightEdit::SpreadAngle(v)),
            ),
            LightKind::Point | LightKind::Spot => self.locator_number(
                ui,
                id,
                "radius",
                "Radius",
                l.radius,
                0.0..=1e6,
                " m",
                1.,
                |v| edit(LightEdit::Radius(v)),
            ),
            LightKind::Cylinder => {
                self.locator_number(
                    ui,
                    id,
                    "radius",
                    "Radius",
                    l.radius,
                    0.0..=1e6,
                    " m",
                    1.,
                    |v| edit(LightEdit::Radius(v)),
                );
                self.locator_number(
                    ui,
                    id,
                    "length",
                    "Length",
                    l.length,
                    0.0001..=1e6,
                    " m",
                    1.,
                    |v| edit(LightEdit::Length(v)),
                );
            }
            LightKind::Mesh => self.mesh_light_rows(ui, id, l),
            // LP-DM-01: nothing below the divider but Samples/Importance.
            LightKind::Dome => {}
            LightKind::Photometric => {
                self.locator_number(
                    ui,
                    id,
                    "width",
                    "Width",
                    l.width,
                    0.0..=1e6,
                    " m",
                    1.,
                    |v| edit(LightEdit::Width(v)),
                );
                self.locator_number(
                    ui,
                    id,
                    "height",
                    "Height",
                    l.height,
                    0.0..=1e6,
                    " m",
                    1.,
                    |v| edit(LightEdit::Height(v)),
                );
            }
            LightKind::Area | LightKind::Portal => {
                let shape = AreaShape::ALL.map(|v| (v, v.label()));
                self.light_choice(ui, id, "Shape", l.shape, &shape, LightEdit::Shape);
                self.locator_number(
                    ui,
                    id,
                    "width",
                    "Width",
                    l.width,
                    0.0001..=1e6,
                    " m",
                    1.,
                    |v| edit(LightEdit::Width(v)),
                );
                self.locator_number(
                    ui,
                    id,
                    "height",
                    "Height",
                    l.height,
                    0.0001..=1e6,
                    " m",
                    1.,
                    |v| edit(LightEdit::Height(v)),
                );
            }
        }
        if l.kind == LightKind::Portal {
            let mut on = l.environment_only;
            self.probe_light_row(&edit(LightEdit::EnvironmentOnly(on)));
            if crate::option_row::check(ui, "Environment Only", &mut on)
                .on_hover_text(
                    "Gather only environment light through the portal. Cycles portals \
                     always do, so this renders the same on or off",
                )
                .changed()
            {
                self.run_light(edit(LightEdit::EnvironmentOnly(on)));
            }
            return;
        }
        if !l.kind.has_visible_rows() {
            return;
        }
        // LP-23: Importance before the Visible rows.
        self.importance_and_lpe_rows(ui, id, l);
        for (label, value, make) in [
            (
                "Visible to Camera",
                l.visible_to_camera,
                LightEdit::VisibleToCamera as fn(bool) -> LightEdit,
            ),
            (
                "Visible to Reflection Rays",
                l.stored.visible_to_reflection,
                LightEdit::VisibleToReflection,
            ),
            (
                "Visible to Refraction Rays",
                l.stored.visible_to_refraction,
                LightEdit::VisibleToRefraction,
            ),
        ] {
            let mut on = value;
            self.probe_light_row(&edit(make(value)));
            let r = crate::option_row::check(ui, label, &mut on);
            let r = if label == "Visible to Camera" {
                r
            } else {
                r.on_hover_text(
                    "Cycles can't tell reflections from refractions: either one lets \
                     bounced rays see the light's shape (provisional)",
                )
            };
            if r.changed() {
                self.run_light(edit(make(on)));
            }
        }
        if crate::light_tier2_ui::has_cone(l.kind) {
            ui.separator();
            self.locator_number(
                ui,
                id,
                "cone",
                "Cone Angle",
                l.cone_angle,
                0.01..=360.0,
                "°",
                deg,
                |v| edit(LightEdit::ConeAngle(v)),
            );
            self.locator_number(
                ui,
                id,
                "edge",
                "Soft Edge",
                l.soft_edge,
                0.0..=180.0,
                "°",
                deg,
                |v| edit(LightEdit::SoftEdge(v)),
            );
        }
        if l.kind == LightKind::Photometric {
            ui.separator();
            self.photometry_rows(ui, id, l);
        }
    }

    /// The colours of the other selected lights an edit of `primary`
    /// carries to (LP-51), before the edit.
    fn other_light_colours(&self, primary: u64) -> Vec<(u64, [f64; 3])> {
        let s = &self.editor.scene.selection;
        if s.item != Some(primary) {
            return Vec::new();
        }
        self.editor
            .scene
            .items
            .iter()
            .filter(|i| i.id != primary && s.also.contains(&i.id))
            .filter_map(|i| i.light.as_ref().map(|l| (i.id, l.color)))
            .collect()
    }

    /// A colour row for the light's colour (LP-13 on the light tab and
    /// LM-10 on the material: the same value).
    fn light_color_row(&mut self, ui: &mut Ui, id: u64, l: &LightSettings, label: &str) {
        ui.horizontal(|ui| {
            crate::option_row::label(ui, label);
            crate::option_row::fill(ui, |ui| {
                let mut rgb = l.color;
                let field =
                    crate::colour_field::show(ui, &mut rgb, crate::colour_field::Options::ui());
                if field.changed() {
                    let others = self.other_light_colours(id);
                    self.run_light(Command::EditLight {
                        item: id,
                        edit: LightEdit::Color(rgb),
                    });
                    // A5: a modifier drag moves each other selected light's
                    // own colour by the same amount (the command above set
                    // them all to this one's).
                    if let Some(gang) = field.gang {
                        for (other, colour) in others {
                            self.run_light(Command::EditLight {
                                item: other,
                                edit: LightEdit::Color(gang.apply(colour)),
                            });
                        }
                    }
                }
            })
        });
    }

    /// Light material rows: Color (= Light Color), Affect Diffuse and
    /// Affect Specular as on/off switches (LPQ-07: stored 0 or 100 %; a
    /// loaded value in between counts as on and is flagged), and Set
    /// Temperature (LM-16).
    fn light_material_rows(&mut self, ui: &mut Ui, id: u64, l: &LightSettings) {
        self.light_color_row(ui, id, l, "Color");
        let partial = |v: f64| v > 0.0 && (v - 1.0).abs() > 1e-9;
        for (label, value, make) in [
            (
                "Affect Diffuse",
                l.affect_diffuse,
                LightEdit::AffectDiffuse as fn(f64) -> LightEdit,
            ),
            (
                "Affect Specular",
                l.affect_specular,
                LightEdit::AffectSpecular,
            ),
        ] {
            let mut on = value > 0.0;
            self.probe_light_row(&Command::EditLight {
                item: id,
                edit: make(value),
            });
            let mut r = crate::option_row::check(ui, label, &mut on);
            if partial(value) {
                r = r.on_hover_text(format!(
                    "Loaded as {} %; Cycles renders it as on (100 %)",
                    value * 100.
                ));
            }
            if r.changed() {
                self.run_light(Command::EditLight {
                    item: id,
                    edit: make(if on { 1.0 } else { 0.0 }),
                });
            }
            if partial(value) {
                ui.weak(format!("{label}: {} % renders as on", value * 100.));
            }
        }
        // LM-16: a button opening a number field; Apply sets Color once.
        let key = egui::Id::new(("light-kelvin", id));
        ui.horizontal(|ui| {
            crate::option_row::label(ui, "");
            ui.menu_button("Set Temperature (Kelvins)...", |ui| {
                let mut k: f64 = ui.data_mut(|d| *d.get_temp_mut_or(key, 6500.0));
                ui.horizontal(|ui| {
                    ui.label("Temperature");
                    ui.add(
                        crate::numeric_tab::DragValue::new(&mut k)
                            .range(operandi_core::camera_light::KELVIN_RANGE)
                            .speed(10.0)
                            .suffix(" K"),
                    );
                });
                ui.data_mut(|d| d.insert_temp(key, k));
                ui.weak("1500 K candle · 3200 K tungsten · 5500 K noon sun · 9000 K sky");
                if ui.button("Apply").clicked() {
                    self.run_light(Command::EditLight {
                        item: id,
                        edit: LightEdit::Temperature(k),
                    });
                    ui.close();
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> Operandi {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        a
    }

    fn camera_item(a: &Operandi) -> operandi_core::Item {
        let id = a.editor.scene.render_camera().unwrap();
        a.editor.scene.item(id).unwrap().clone()
    }

    #[test]
    fn camera_view_looks_through_the_render_camera() {
        let mut a = app();
        let r = Rect::from_min_size(Pos2::ZERO, vec2(900., 600.));
        a.enter_camera_view();
        a.sync_camera_view(r);
        assert!(a.camera.through);
        assert_eq!(a.view_title(), "Camera · Render Camera");
        // Orbits the point 4 m (Target Distance) in front of the camera.
        let f = item_frame(&a.editor.scene, camera_item(&a).id).unwrap();
        assert!((a.camera.target - (f.position + f.forward * 4.0)).length() < 1e-9);
        assert!((a.camera.distance - 4.0).abs() < 1e-12);
        let (eye, right, _, forward) = a.camera.basis();
        let item = camera_item(&a);
        let frame = item_frame(&a.editor.scene, item.id).unwrap();
        assert!((eye - frame.position).length() < 1e-9);
        // Horizontal Film Fit: the film's edge at the target is the view's
        // right edge (half width 0.36 per metre for 36 mm / 50 mm).
        let edge = eye + forward * 4.0 + right * 4.0 * 0.36;
        let (p, _) = a.camera.project(edge, r).unwrap();
        assert!((p.x - r.right()).abs() < 1e-3, "{p:?}");
        // Leaving restores the ordinary lens.
        a.leave_camera_view();
        assert_eq!(a.camera.lens, crate::camera::DEFAULT_LENS);
        assert_eq!(a.camera.view, View::Perspective);
    }

    /// Camera view frames what the render renders: the render picture's
    /// edges (from the renderer's own camera) land on the frame drawn in
    /// the view, whatever the view's shape.
    #[test]
    fn camera_view_frames_the_render_picture() {
        let [fw, fh] = defaults::RENDER_FRAME;
        for size in [vec2(900., 900.), vec2(2000., 600.), vec2(1600., 900.)] {
            let mut a = app();
            let r = Rect::from_min_size(pos2(30., 40.), size);
            a.enter_camera_view();
            a.sync_camera_view(r);
            let view =
                operandi_cycles::scene_camera_view(&a.editor.scene, f64::from(fw) / f64::from(fh))
                    .unwrap();
            let frame = render_frame_rect(r);
            let at = view.position + view.forward * 4.0;
            let top = at + view.up * 4.0 * view.half_height_tan;
            let (p, _) = a.camera.project(top, r).unwrap();
            assert!(
                (p.y - frame.top()).abs() < 0.5,
                "{size:?}: {p:?} vs {frame:?}"
            );
            assert!((p.x - frame.center().x).abs() < 0.5, "{size:?}: {p:?}");
            let aspect = f64::from(fw) / f64::from(fh);
            let right = at + view.right * 4.0 * view.half_height_tan * aspect;
            let (p, _) = a.camera.project(right, r).unwrap();
            assert!(
                (p.x - frame.right()).abs() < 0.5,
                "{size:?}: {p:?} vs {frame:?}"
            );
            assert!((frame.width() / frame.height() - aspect as f32).abs() < 1e-3);
            assert!(frame.width() <= size.x + 1e-3 && frame.height() <= size.y + 1e-3);
        }
    }

    /// Ground truth (mirrored renders, 2026-11-05): the default camera at
    /// (0, 0.75, 4) looks down −Z, so a point at world +X shows on the
    /// right of Camera view, and the info line names the Camera view.
    #[test]
    fn camera_view_shows_plus_x_on_the_right() {
        let mut a = app();
        let r = Rect::from_min_size(Pos2::ZERO, vec2(1600., 900.));
        a.enter_camera_view();
        a.sync_camera_view(r);
        let (p, _) = a.camera.project(DVec3::new(1.0, 0.75, 0.0), r).unwrap();
        assert!(p.x > r.center().x + 100., "{p:?}");
        // The camera tilts down 5°, so a point at its own height is above
        // the centre.
        assert!(p.y < r.center().y, "{p:?}");
        assert_eq!(a.view_title(), "Camera · Render Camera");
    }

    /// CL-19: the bands outside the render frame use the theme colour,
    /// black at about 43 % by default.
    #[test]
    fn camera_frame_shade_is_a_theme_colour() {
        assert_eq!(
            theme::viewport::camera_frame_shade(),
            Color32::from_black_alpha(110)
        );
    }

    /// Camera Item (Viewport Properties > Drawing and Control): a camera
    /// chosen for the view wins over the selected or newest one, in the
    /// header and the render frame; leaving Camera view forgets it.
    #[test]
    fn a_chosen_camera_item_is_the_one_the_view_looks_through() {
        let mut a = app();
        let first = camera_item(&a).id;
        let r = Rect::from_min_size(Pos2::ZERO, vec2(1600., 900.));
        a.run(Command::NewCamera);
        let second = a.editor.scene.selection.item.unwrap();
        a.look_through(first).unwrap();
        a.sync_camera_view(r);
        assert_eq!(
            a.through_camera(),
            Some(first),
            "chosen, not the selected one"
        );
        assert_eq!(a.view_title(), "Camera · Render Camera");
        // (Current Selection) again: the selected camera.
        a.camera.through_item = None;
        assert_eq!(a.through_camera(), Some(second));
        a.look_through(first).unwrap();
        a.leave_camera_view();
        assert_eq!(a.camera.through_item, None);
    }

    /// CL-17/18: Camera view follows the selected camera, else the newest;
    /// the header names it and says "Render Camera" only for that one.
    #[test]
    fn camera_view_follows_the_selected_or_newest_camera() {
        let mut a = app();
        let first = camera_item(&a).id;
        let r = Rect::from_min_size(Pos2::ZERO, vec2(1600., 900.));
        a.enter_camera_view();
        a.sync_camera_view(r);
        assert_eq!(view_camera(&a.editor.scene), Some(first));
        assert_eq!(a.view_title(), "Camera · Render Camera");
        // A new camera is selected: the view follows it.
        a.run(Command::NewCamera);
        let second = a.editor.scene.selection.item.unwrap();
        assert_eq!(view_camera(&a.editor.scene), Some(second));
        assert_eq!(a.view_title(), "Camera (2)");
        // Nothing selected: the newest camera.
        a.run(Command::SelectItem(None));
        assert_eq!(view_camera(&a.editor.scene), Some(second));
        // Selecting the first camera again.
        a.run(Command::SelectItem(Some(first)));
        assert_eq!(view_camera(&a.editor.scene), Some(first));
        // Setting the render camera moves the label.
        a.run(Command::SetRenderCamera(second));
        assert_eq!(a.view_title(), "Camera");
        a.run(Command::SelectItem(Some(second)));
        assert_eq!(a.view_title(), "Camera (2) · Render Camera");
        a.sync_camera_view(r);
        let f = item_frame(&a.editor.scene, second).unwrap();
        assert!((a.camera.target - (f.position + f.forward * 4.0)).length() < 1e-9);
    }

    #[test]
    fn camera_view_navigation_moves_the_camera_once_per_press() {
        let mut a = app();
        let r = Rect::from_min_size(Pos2::ZERO, vec2(900., 600.));
        a.enter_camera_view();
        a.sync_camera_view(r);
        let before = camera_item(&a);
        let undo = a.editor.history.undo.len();
        // A drag in progress: the view moves, the item waits.
        a.pointer_held = true;
        for _ in 0..3 {
            a.camera.yaw += 0.1;
            a.camera.distance *= 0.9;
            a.sync_camera_view(r);
        }
        assert_eq!(camera_item(&a), before);
        let (eye, _, _, forward) = a.camera.basis();
        // Release: one undo step moves the camera item to the view.
        a.pointer_held = false;
        a.sync_camera_view(r);
        // The view stays where the user left it (no flash back to the old
        // camera placement).
        let (eye2, _, _, forward2) = a.camera.basis();
        assert!((eye2 - eye).length() < 1e-6);
        assert!((forward2 - forward).length() < 1e-6);
        assert_eq!(a.editor.history.undo.len(), undo + 1);
        let after = camera_item(&a);
        let frame = item_frame(&a.editor.scene, after.id).unwrap();
        assert!((frame.position - eye).length() < 1e-9);
        assert!((frame.forward - forward).length() < 1e-9);
        assert!((after.camera.unwrap().target_distance - 4.0 * 0.9f64.powi(3)).abs() < 1e-9);
        // Undo puts the camera back, and the view follows it.
        a.run(Command::Undo);
        assert_eq!(camera_item(&a), before);
        a.sync_camera_view(r);
        assert!((a.camera.distance - 4.0).abs() < 1e-12);
        let f = item_frame(&a.editor.scene, before.id).unwrap();
        assert!((a.camera.target - (f.position + f.forward * 4.0)).length() < 1e-9);
    }

    #[test]
    fn texture_locator_gizmo_shows_the_projection_at_size() {
        use operandi_core::shader_tree::Projection;
        use operandi_core::texture_locator::TextureLocator;
        let world = glam::DMat4::from_scale(DVec3::splat(2.));
        let lines = |projection, axis| {
            let l = TextureLocator {
                axis,
                ..TextureLocator::new(projection)
            };
            texture_locator_marks(world, &l)
        };
        // Planar, axis Z: the three axes, a 2 m square at depth 0, the
        // projection-axis line (25 % of the smaller side) and the up tick
        // on the +V edge (Planar spec).
        let planar = lines(Projection::Planar, 2);
        assert_eq!(planar.len(), 3 + 4 + 2);
        let Mark::Line(axis_line) = planar[7] else {
            panic!()
        };
        assert_eq!(axis_line, [DVec3::ZERO, DVec3::new(0., 0., 0.5)]);
        let Mark::Line(tick) = planar[8] else {
            panic!()
        };
        assert_eq!(tick, [DVec3::new(0., 1., 0.), DVec3::new(0., 1.2, 0.)]);
        // Axis X: the image's V is +Y, so the tick still points up.
        let Mark::Line(tick) = lines(Projection::Planar, 0)[8] else {
            panic!()
        };
        assert_eq!(tick, [DVec3::new(0., 1., 0.), DVec3::new(0., 1.2, 0.)]);
        let square: Vec<[DVec3; 2]> = planar
            .iter()
            .filter_map(|m| match m {
                Mark::Line(s) if s[0].z == 0. && s[1].z == 0. && s[0].x.abs() == 1. => Some(*s),
                _ => None,
            })
            .collect();
        assert!(!square.is_empty());
        assert!(
            square
                .iter()
                .flatten()
                .all(|p| p.x.abs() == 1. && p.y.abs() == 1.)
        );
        // Axis X: the square lies in the Y-Z plane.
        let side = lines(Projection::Planar, 0);
        assert!(
            side[3..7]
                .iter()
                .all(|m| matches!(m, Mark::Line(s) if s[0].x == 0. && s[1].x == 0.))
        );
        // A unit cube for Box, axes only for UV Map.
        assert_eq!(lines(Projection::Box, 0).len(), 3 + 12);
        assert_eq!(lines(Projection::UvMap, 0).len(), 3);
        assert_eq!(lines(Projection::Cylindrical, 1).len(), 3 + 48 + 4);
        assert_eq!(lines(Projection::Spherical, 1).len(), 3 + 72);
    }

    #[test]
    fn flat_arrowhead_fills_are_detected() {
        let tri = [pos2(0., 0.), pos2(10., 0.), pos2(0., 10.)];
        assert!(!fill_is_flat(&tri));
        assert!(fill_is_flat(&[pos2(0., 0.), pos2(5., 5.), pos2(10., 10.)]));
        assert!(fill_is_flat(&[
            pos2(0., 0.),
            pos2(f32::NAN, 1.),
            pos2(0., 10.)
        ]));
        assert!(fill_is_flat(&[
            pos2(0., 0.),
            pos2(f32::INFINITY, 1.),
            pos2(0., 10.)
        ]));
        assert!(fill_is_flat(&tri[..2]));
        // Big enough area, but a needle tip (~0.6°): egui would spike it.
        assert!(fill_is_flat(&[
            pos2(0., 0.),
            pos2(200., -1.),
            pos2(200., 1.)
        ]));
        // An ordinary arrowhead tip (~48°) stays filled.
        assert!(!fill_is_flat(&[pos2(0., 0.), pos2(9., -4.), pos2(9., 4.)]));
    }

    /// A directional light seen side-on, or nearly, from an ortho Top
    /// view: one arrowhead triangle is a line or a sliver. Whatever reaches
    /// the painter must tessellate to something near the glyph, not a slab
    /// (egui's feathering spikes a sliver out by thousands of pixels).
    #[test]
    fn side_on_directional_light_paints_no_slab() {
        let l = LightSettings::new(LightKind::Directional);
        // Ortho Top: screen x = world X, screen y = world Z, 100 px a metre.
        let top = |v: DVec3| pos2(v.x as f32 * 100. + 500., v.z as f32 * 100. + 350.);
        let stroke = Stroke::new(1.0_f32, Color32::WHITE);
        let mut saw_flat = false;
        for angle in [0.0, 1e-5, 1e-4, 1e-3, 2e-3, 1e-2, 5e-2] {
            for world in [
                glam::DMat4::from_rotation_x(angle),
                glam::DMat4::from_rotation_y(angle),
                // About the arrow's own axis: the sliver case.
                glam::DMat4::from_rotation_z(angle),
                glam::DMat4::from_rotation_z(std::f64::consts::FRAC_PI_2 + angle),
            ] {
                for mark in light_marks(world, &l, false) {
                    let Mark::Fill(points) = mark else { continue };
                    let points: Vec<Pos2> = points.into_iter().map(top).collect();
                    saw_flat |= fill_is_flat(&points);
                    let glyph = Rect::from_points(&points).expand(4.0);
                    let mut tessellator =
                        egui::epaint::Tessellator::new(1.0, Default::default(), [1, 1], vec![]);
                    let mut mesh = egui::epaint::Mesh::default();
                    for shape in fill_shapes(points, Color32::WHITE, stroke) {
                        tessellator.tessellate_shape(shape, &mut mesh);
                    }
                    for v in &mesh.vertices {
                        assert!(
                            glyph.contains(v.pos),
                            "{angle}: {:?} outside {glyph:?}",
                            v.pos
                        );
                    }
                }
            }
        }
        assert!(saw_flat, "no side-on arrowhead was exercised");
    }

    #[test]
    fn glyphs_per_kind_and_selected_guides() {
        let a = app();
        let item = camera_item(&a);
        let world = a.editor.scene.world_matrix(item.id).unwrap();
        let camera = item.camera.as_ref().unwrap();
        let dotted = |m: &[Mark]| m.iter().filter(|m| matches!(m, Mark::Dotted(_))).count();
        let fills = |m: &[Mark]| m.iter().filter(|m| matches!(m, Mark::Fill(_))).count();
        // Camera: frustum only when selected.
        assert_eq!(dotted(&camera_marks(world, camera, 1.0, false)), 0);
        assert_eq!(dotted(&camera_marks(world, camera, 1.0, true)), 8);
        // Its body keeps a constant screen size: `size` sets it, item scale
        // doesn't.
        let small = camera_marks(world, camera, 0.1, false);
        let big = camera_marks(world, camera, 1.0, false);
        let span = |m: &[Mark]| match &m[0] {
            Mark::Line([a, b]) => a.distance(*b),
            _ => 0.0,
        };
        assert!((span(&big) / span(&small) - 10.0).abs() < 1e-9);
        let id = glam::DMat4::IDENTITY;
        for kind in LightKind::ALL {
            let l = LightSettings::new(kind);
            let marks = light_marks(id, &l, false);
            assert!(marks.len() >= 4, "{kind:?}");
            // Arrowheads on directional and area only.
            let arrows = matches!(
                kind,
                LightKind::Directional | LightKind::Area | LightKind::Portal
            );
            assert_eq!(fills(&marks) > 0, arrows, "{kind:?}");
            // Spot cone lines only when selected.
            let sel = light_marks(id, &l, true);
            assert_eq!(
                dotted(&sel) > dotted(&marks),
                kind == LightKind::Spot,
                "{kind:?}"
            );
        }
        // Display Size scales the directional glyph.
        let mut l = LightSettings::new(LightKind::Directional);
        let one = light_marks(id, &l, false);
        l.stored.display_size = 2.0;
        let two = light_marks(id, &l, false);
        assert!((span(&two) / span(&one) - 2.0).abs() < 1e-9);
        // Hidden by default (Jason, 2026-11-05).
        assert!(!a.display.settings.drawing.cameras && !a.display.settings.drawing.lights);
    }

    /// IK-43: a fan per limited axis, its edges at Min and Max.
    #[test]
    fn limit_fan_spans_min_to_max() {
        let mut ik = operandi_core::ik_joint::JointIk::new(1);
        ik.enable_limits = true;
        ik.axes[2].use_limits = true;
        ik.axes[2].min = 0.;
        ik.axes[2].max = std::f64::consts::FRAC_PI_2;
        let marks = limit_fan_marks(glam::DMat4::IDENTITY, DVec3::ZERO, &ik, 1.);
        assert_eq!(marks.len(), 2 + 16);
        let Mark::Line([_, hi]) = &marks[1] else {
            panic!()
        };
        assert!(hi.distance(DVec3::Y) < 1e-9, "Z from X round to Y: {hi}");
    }

    /// SK-17/18: a joint draws a sphere sized by its bone, a solid bone
    /// to its first child and dotted lines to the others.
    #[test]
    fn joint_marks_follow_the_first_child() {
        let kids = [DVec3::new(0., 2., 0.), DVec3::new(1., 0., 0.)];
        let marks = joint_marks(glam::DMat4::IDENTITY, &kids, 0.05);
        let lines = marks.iter().filter(|m| matches!(m, Mark::Line(_))).count();
        assert_eq!(lines, 3 * 16 + 12);
        assert_eq!(
            marks
                .iter()
                .filter(|m| matches!(m, Mark::Dotted(_)))
                .count(),
            1
        );
        // The sphere's radius is a tenth of the bone.
        let Mark::Line([p, _]) = &marks[0] else {
            panic!()
        };
        assert!((p.length() - 0.2).abs() < 1e-9);
        // A leaf: rings only, at the given size.
        let leaf = joint_marks(glam::DMat4::IDENTITY, &[], 0.05);
        assert_eq!(leaf.len(), 48);
    }

    /// GL-10..12: a Group Locator's row glyph, viewport marks and Show
    /// Locators.
    #[test]
    fn group_locator_glyph_and_marks() {
        let mut a = app();
        a.run(Command::AddItem(
            operandi_core::item_list::AddItemKind::GroupLocator,
        ));
        let id = a.editor.scene.selection.item.unwrap();
        let item = a.editor.scene.item(id).unwrap();
        assert_eq!(LocatorGlyph::of(item), Some(LocatorGlyph::GroupLocator));
        // A folder: the tab sits on the body's top left.
        let (tab, body) = folder_shape(pos2(10., 10.));
        assert!(tab.bottom() >= body.top() && tab.left() == body.left());
        assert!(tab.width() < body.width());
        assert_ne!(
            theme::workspace::group_locator_icon(),
            theme::workspace::field_text()
        );
        // 8 corners x 3 ticks, then the 3-line centre cross.
        let marks = group_locator_marks(glam::DMat4::IDENTITY, 2.0);
        assert_eq!(marks.len(), 27);
        let lines: Vec<[DVec3; 2]> = marks
            .iter()
            .map(|m| match m {
                Mark::Line(s) => *s,
                _ => panic!("only lines"),
            })
            .collect();
        for seg in &lines[..24] {
            // From a corner (all |coords| = 1) a quarter edge (0.5) inwards.
            assert!(seg[0].abs().abs_diff_eq(DVec3::ONE, 1e-12));
            assert!((seg[0].distance(seg[1]) - 0.5).abs() < 1e-12);
            assert!(seg[1].length() < seg[0].length());
        }
        // The item's world matrix moves, turns and scales it.
        let world = glam::DMat4::from_scale_rotation_translation(
            DVec3::splat(3.),
            glam::DQuat::from_rotation_z(0.5),
            DVec3::new(5., 0., 0.),
        );
        let moved = group_locator_marks(world, 2.0);
        match (&moved[24], &lines[24]) {
            (Mark::Line(m), l) => {
                assert!(m[0].abs_diff_eq(world.transform_point3(l[0]), 1e-9));
            }
            _ => unreachable!(),
        }
        // Show Locators is on by default; old settings without it too.
        assert!(a.display.settings.drawing.locators);
        let mut saved = serde_json::to_value(crate::view_display::Drawing::default()).unwrap();
        saved.as_object_mut().unwrap().remove("locators");
        saved["lights"] = serde_json::Value::Bool(true);
        let old: crate::view_display::Drawing = serde_json::from_value(saved).unwrap();
        assert!(old.locators && old.lights);
    }
}
