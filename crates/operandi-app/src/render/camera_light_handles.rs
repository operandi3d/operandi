//! Viewport handles for the selected camera or light while Move, Rotate or
//! Transform is active (CL-14, spec v3/v4). Positions are observed; what
//! each drag does is a guess (logged in docs/SPECIFICATION_QUESTIONS.md):
//!
//! - Area: square grabs at the middle of the +X side and the +Y (top) side;
//!   a drag resizes Width or Height about the centre (both sides move).
//! - Spot: a solid circle at the cone's edge and a dotted circle where the
//!   Soft Edge starts, a fixed screen distance down the −Z axis, each with
//!   a grab cross on a radius line (Cone Angle on +X, Soft Edge on +Y).
//! - Camera: a grab cross on the axis at Focus Distance (drag along the
//!   axis edits it). The cross seen on the camera body is the transform
//!   tool's own centre handle, not a camera channel (spec v4.1).
//! - Point: Radius as three great circles with grab crosses at +X and +Y;
//!   at Radius 0 a single cross at the centre that grows it.
//!
//! Each drag is one tweak of the active tool (one undo step inside it).
use super::*;
use crate::camera_light_ui::{Mark, ring};
use operandi_core::camera_light::{
    CameraEdit, Frame, ItemChannelEdit, LightEdit, LightKind, item_frame, tool_shows_item_channels,
};
use operandi_core::camera_light_defaults as defaults;

/// A grab handle of a camera or light.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Grab {
    AreaWidth,
    AreaHeight,
    SpotCone,
    SpotEdge,
    CameraFocus,
    PointRadius,
}

impl Grab {
    /// Area handles are squares; the rest are crosses.
    fn square(self) -> bool {
        matches!(self, Grab::AreaWidth | Grab::AreaHeight)
    }
}

/// The selected item's handles in world space.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Handles {
    pub item: u64,
    pub grabs: Vec<(Grab, DVec3)>,
    /// Circles and radius lines, in the handle-circle colour.
    pub circles: Vec<Mark>,
}

/// An orthonormal frame matrix centred at `at` (no scale).
fn frame_at(f: &Frame, at: DVec3) -> glam::DMat4 {
    glam::DMat4::from_cols(
        f.right.extend(0.0),
        f.up.extend(0.0),
        (-f.forward).extend(0.0),
        at.extend(1.0),
    )
}

/// Where item `id`'s handles are. `spot_distance` is how far down its axis
/// a spot's circles sit (metres; the caller keeps it a fixed screen
/// distance). `None` for a directional light or a non-light item.
pub(super) fn handles_for(scene: &Scene, id: u64, spot_distance: f64) -> Option<Handles> {
    let item = scene.item(id)?;
    let f = item_frame(scene, id)?;
    let mut grabs = Vec::new();
    let mut circles = Vec::new();
    if let Some(c) = &item.camera {
        grabs.push((Grab::CameraFocus, f.position + f.forward * c.focus_distance));
    } else {
        let light = item.light.as_ref()?;
        match light.kind {
            LightKind::Directional
            | LightKind::Cylinder
            | LightKind::Mesh
            | LightKind::Photometric
            | LightKind::Dome => return None,
            LightKind::Area | LightKind::Portal => {
                let m = scene.world_matrix(id).ok()?;
                grabs.push((
                    Grab::AreaWidth,
                    m.transform_point3(DVec3::new(light.width / 2.0, 0.0, 0.0)),
                ));
                grabs.push((
                    Grab::AreaHeight,
                    m.transform_point3(DVec3::new(0.0, light.height / 2.0, 0.0)),
                ));
            }
            LightKind::Spot => {
                let d = spot_distance;
                let centre = f.position + f.forward * d;
                let m = frame_at(&f, centre);
                let outer = d * (light.cone_angle / 2.0).min(1.55).tan();
                let inner = d * light.spot_full_angle().min(1.55).tan();
                circles.extend(ring(m, outer, outer, 0.0, 48, false));
                if inner > 1e-9 {
                    circles.extend(ring(m, inner, inner, 0.0, 48, true));
                }
                let cone = centre + f.right * outer;
                let edge = centre + f.up * inner;
                circles.push(Mark::Line([centre, cone]));
                circles.push(Mark::Line([centre, edge]));
                grabs.push((Grab::SpotCone, cone));
                grabs.push((Grab::SpotEdge, edge));
            }
            LightKind::Point => {
                let r = light.radius;
                if r > 0.0 {
                    let m = frame_at(&f, f.position);
                    circles.extend(ring(m, r, r, 0.0, 48, false));
                    let yz = m * glam::DMat4::from_rotation_y(std::f64::consts::FRAC_PI_2);
                    circles.extend(ring(yz, r, r, 0.0, 48, false));
                    let xz = m * glam::DMat4::from_rotation_x(std::f64::consts::FRAC_PI_2);
                    circles.extend(ring(xz, r, r, 0.0, 48, false));
                    grabs.push((Grab::PointRadius, f.position + f.right * r));
                    grabs.push((Grab::PointRadius, f.position + f.up * r));
                } else {
                    grabs.push((Grab::PointRadius, f.position));
                }
            }
        }
    }
    Some(Handles {
        item: id,
        grabs,
        circles,
    })
}

/// Where ray `o + t·d` meets the plane through `p` with normal `n`.
fn plane_hit(o: DVec3, d: DVec3, p: DVec3, n: DVec3) -> Option<DVec3> {
    let denom = d.dot(n);
    if denom.abs() < 1e-9 {
        return None;
    }
    let t = (p - o).dot(n) / denom;
    (t >= 0.0).then(|| o + d * t)
}

/// The channel edit for dragging `grab` of item `id` to where the pointer
/// ray (`o`, `d`) points. `view_forward` is the view's direction (the point
/// light's radius is measured in the plane facing the view).
pub(super) fn drag_edit(
    scene: &Scene,
    id: u64,
    grab: Grab,
    (o, d): (DVec3, DVec3),
    view_forward: DVec3,
    spot_distance: f64,
) -> Option<ItemChannelEdit> {
    let item = scene.item(id)?;
    let f = item_frame(scene, id)?;
    match grab {
        Grab::AreaWidth | Grab::AreaHeight => {
            let m = scene.world_matrix(id).ok()?;
            let normal = m.transform_vector3(DVec3::Z).try_normalize()?;
            let centre = m.transform_point3(DVec3::ZERO);
            let hit = plane_hit(o, d, centre, normal)?;
            let axis = m.transform_vector3(if grab == Grab::AreaWidth {
                DVec3::X
            } else {
                DVec3::Y
            });
            let scale = axis.length();
            // About the centre: the size is twice the distance out.
            let size = (2.0 * (hit - centre).dot(axis / scale).abs() / scale).max(1e-3);
            Some(ItemChannelEdit::Light(if grab == Grab::AreaWidth {
                LightEdit::Width(size)
            } else {
                LightEdit::Height(size)
            }))
        }
        Grab::SpotCone | Grab::SpotEdge => {
            let light = item.light.as_ref()?;
            let centre = f.position + f.forward * spot_distance;
            let hit = plane_hit(o, d, centre, f.forward)?;
            let half = ((hit - centre).length() / spot_distance).atan();
            Some(ItemChannelEdit::Light(if grab == Grab::SpotCone {
                LightEdit::ConeAngle((2.0 * half).clamp(1e-3, std::f64::consts::PI - 1e-3))
            } else {
                let h = light.cone_angle / 2.0;
                LightEdit::SoftEdge((h - half).clamp(0.0, h))
            }))
        }
        Grab::CameraFocus => {
            // The point on the camera's axis closest to the pointer ray.
            let (p, u) = (f.position, f.forward);
            let w0 = p - o;
            let (a, b, c) = (u.dot(u), u.dot(d), d.dot(d));
            let denom = a * c - b * b;
            if denom.abs() < 1e-12 {
                return None;
            }
            let s = (b * d.dot(w0) - c * u.dot(w0)) / denom;
            Some(ItemChannelEdit::Camera(CameraEdit::FocusDistance(
                s.max(0.01),
            )))
        }
        Grab::PointRadius => {
            let hit = plane_hit(o, d, f.position, view_forward)?;
            Some(ItemChannelEdit::Light(LightEdit::Radius(
                (hit - f.position).length(),
            )))
        }
    }
}

impl Operandi {
    /// The selected camera or light's handles, when Move, Rotate or
    /// Transform is active on it in Item mode.
    pub(super) fn item_handles(&self, r: Rect) -> Option<Handles> {
        let live = self.editor.session.live.as_ref()?;
        let scene = &self.editor.scene;
        if !tool_shows_item_channels(live.tool) || scene.selection.mode != Mode::Item {
            return None;
        }
        // CP-A02: a locked item shows no handles.
        let item = scene
            .active()
            .filter(|i| scene.shows(i) && !i.is_locked())?;
        if item.camera.is_none() && item.light.is_none() {
            return None;
        }
        handles_for(scene, item.id, self.spot_handle_distance(item.id, r)?)
    }

    /// Metres down a spot's axis that keep its handle circles a fixed
    /// distance on screen (spec v4 leaves it to the view; guess).
    fn spot_handle_distance(&self, id: u64, r: Rect) -> Option<f64> {
        let at = item_frame(&self.editor.scene, id)?.position;
        let mpp = self
            .camera
            .metres_per_pixel(at, r)
            .unwrap_or(self.camera.distance / f64::from(r.height().max(1.)));
        Some(mpp * defaults::SPOT_HANDLE_POINTS)
    }

    /// Draw the selected camera or light's handles over the view.
    pub(super) fn draw_item_handles(&self, painter: &egui::Painter, r: Rect) {
        let Some(h) = self.item_handles(r) else {
            return;
        };
        let projector = self.camera.projector(r);
        let circle = Stroke::new(1.2f32, theme::viewport::light_handle_circle());
        for mark in &h.circles {
            let (Mark::Line([a, b]) | Mark::Dotted([a, b])) = mark else {
                continue;
            };
            let (Some((a, _)), Some((b, _))) = (projector.project(*a), projector.project(*b))
            else {
                continue;
            };
            if matches!(mark, Mark::Dotted(_)) {
                painter.extend(egui::Shape::dashed_line(&[a, b], circle, 3.0, 3.0));
            } else {
                painter.line_segment([a, b], circle);
            }
        }
        let grab = Stroke::new(1.6f32, theme::viewport::light_handle_grab());
        let s = defaults::GRAB_HALF_POINTS;
        for (kind, at) in &h.grabs {
            let Some((p, _)) = projector.project(*at) else {
                continue;
            };
            if kind.square() {
                painter.rect_stroke(
                    Rect::from_center_size(p, vec2(2. * s, 2. * s)),
                    0.,
                    grab,
                    egui::StrokeKind::Middle,
                );
            } else {
                painter.line_segment([p - vec2(s, 0.), p + vec2(s, 0.)], grab);
                painter.line_segment([p - vec2(0., s), p + vec2(0., s)], grab);
            }
        }
    }

    /// The grab under `pos`, if any.
    pub(super) fn item_handle_hit(&self, pos: Pos2, r: Rect) -> Option<Grab> {
        let h = self.item_handles(r)?;
        let projector = self.camera.projector(r);
        h.grabs
            .iter()
            .filter_map(|(g, at)| Some((*g, projector.project(*at)?.0.distance(pos))))
            .filter(|(_, d)| *d <= defaults::GRAB_HIT_POINTS)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(g, _)| g)
    }

    /// One frame of a handle drag: edit the channel the grab drives.
    pub(super) fn drag_item_handle(&mut self, grab: Grab, pos: Pos2, r: Rect) {
        let Some(id) = self.editor.scene.selection.item else {
            return;
        };
        let Some(distance) = self.spot_handle_distance(id, r) else {
            return;
        };
        let (_, _, _, forward) = self.camera.basis();
        let ray = self.camera.ray(pos, r);
        if let Some(edit) = drag_edit(&self.editor.scene, id, grab, ray, forward, distance) {
            self.run(Command::ToolItemChannel { item: id, edit });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::camera_light::LightSettings;
    use operandi_core::scene::Transform;

    fn scene_with(kind: LightKind) -> (Scene, u64) {
        let mut scene = Scene::default();
        let id = scene
            .add_light(LightSettings::new(kind), Transform::default())
            .unwrap();
        (scene, id)
    }
    fn grab(h: &Handles, g: Grab) -> DVec3 {
        h.grabs.iter().find(|(k, _)| *k == g).unwrap().1
    }

    #[test]
    fn spot_circles_follow_cone_and_soft_edge() {
        let (scene, id) = scene_with(LightKind::Spot);
        // 45° cone, 5° edge, circles 2 m down −Z.
        let h = handles_for(&scene, id, 2.0).unwrap();
        let cone = grab(&h, Grab::SpotCone);
        let edge = grab(&h, Grab::SpotEdge);
        let centre = DVec3::new(0.0, 0.0, -2.0);
        assert!((cone - (centre + DVec3::X * 2.0 * 22.5f64.to_radians().tan())).length() < 1e-12);
        assert!((edge - (centre + DVec3::Y * 2.0 * 17.5f64.to_radians().tan())).length() < 1e-12);
        // Solid outer circle and dotted inner one.
        assert!(h.circles.iter().any(|m| matches!(m, Mark::Dotted(_))));
        // Dragging them back to where they are gives the same channels.
        // Rays straight down −Z through a point.
        let ray = |p: DVec3| (p + DVec3::Z, -DVec3::Z);
        let Some(ItemChannelEdit::Light(LightEdit::ConeAngle(a))) =
            drag_edit(&scene, id, Grab::SpotCone, ray(cone), -DVec3::Z, 2.0)
        else {
            panic!()
        };
        assert!((a.to_degrees() - 45.0).abs() < 1e-9);
        let Some(ItemChannelEdit::Light(LightEdit::SoftEdge(e))) =
            drag_edit(&scene, id, Grab::SpotEdge, ray(edge), -DVec3::Z, 2.0)
        else {
            panic!()
        };
        assert!((e.to_degrees() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn spot_handles_turn_with_the_item() {
        let (mut scene, id) = scene_with(LightKind::Spot);
        scene.items[0].transform.rotation = DVec3::new(0.0, 0.0, std::f64::consts::FRAC_PI_2);
        let h = handles_for(&scene, id, 1.0).unwrap();
        // Local +X now points along world +Y.
        assert!(grab(&h, Grab::SpotCone).y > 0.1);
        assert!(grab(&h, Grab::SpotCone).x.abs() < 1e-9);
    }

    #[test]
    fn area_handles_resize_about_the_centre() {
        let (mut scene, id) = scene_with(LightKind::Area);
        scene.items[0].transform.scale = DVec3::new(2.0, 1.0, 1.0);
        let h = handles_for(&scene, id, 1.0).unwrap();
        // Width 1 × scale 2: the side handle is 1 m out; the top 0.5 m up.
        assert!((grab(&h, Grab::AreaWidth) - DVec3::new(1.0, 0.0, 0.0)).length() < 1e-12);
        assert!((grab(&h, Grab::AreaHeight) - DVec3::new(0.0, 0.5, 0.0)).length() < 1e-12);
        // Dragging the side handle to x = 1.5 (from in front, looking −Z):
        // width 2 × 1.5 / 2 (scale) = 1.5, either side of the centre.
        let ray = (DVec3::new(1.5, 0.0, 3.0), -DVec3::Z);
        let Some(ItemChannelEdit::Light(LightEdit::Width(w))) =
            drag_edit(&scene, id, Grab::AreaWidth, ray, -DVec3::Z, 1.0)
        else {
            panic!()
        };
        assert!((w - 1.5).abs() < 1e-12);
        let ray = (DVec3::new(-1.5, 0.0, 3.0), -DVec3::Z);
        let Some(ItemChannelEdit::Light(LightEdit::Width(w))) =
            drag_edit(&scene, id, Grab::AreaWidth, ray, -DVec3::Z, 1.0)
        else {
            panic!()
        };
        assert!(
            (w - 1.5).abs() < 1e-12,
            "the other side gives the same size"
        );
    }

    /// A handle drag through the app: the press finds the grab, the drag
    /// resizes, and the whole drag is one tweak of the Move tool.
    #[test]
    fn area_handle_drag_is_one_tweak() {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        a.run(Command::NewLight(LightKind::Area));
        let id = a.editor.scene.selection.item.unwrap();
        a.run(Command::Activate(operandi_core::Tool::Move));
        let r = Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.));
        let h = a.item_handles(r).expect("handles with Move active");
        let at = a.camera.project(grab(&h, Grab::AreaWidth), r).unwrap().0;
        assert_eq!(a.item_handle_hit(at, r), Some(Grab::AreaWidth));
        let before = a.editor.session.tweaks.undo_steps();
        a.pointer_held = true;
        for dx in [10., 25., 40.] {
            a.drag_item_handle(Grab::AreaWidth, at + vec2(dx, 0.), r);
        }
        a.pointer_held = false;
        a.editor.end_tweak();
        let width = a.editor.scene.item(id).unwrap();
        assert!(width.light.as_ref().unwrap().width > 1.0);
        assert_eq!(a.editor.session.tweaks.undo_steps(), before + 1);
        a.run(Command::Undo);
        let item = a.editor.scene.item(id).unwrap();
        assert_eq!(item.light.as_ref().unwrap().width, 1.0);
        assert!(a.editor.session.live.is_some(), "the tool stays");
        // No handles without a channel tool.
        a.run(Command::Cancel);
        assert!(a.item_handles(r).is_none());
    }

    #[test]
    fn camera_focus_and_point_radius_handles() {
        let mut scene = Scene::default();
        let cam = scene
            .add_camera(Default::default(), Transform::default())
            .unwrap();
        let h = handles_for(&scene, cam, 1.0).unwrap();
        // Focus Distance 1 m down −Z; nothing on the body (that cross is
        // the transform tool's centre handle).
        assert!((grab(&h, Grab::CameraFocus) - DVec3::new(0.0, 0.0, -1.0)).length() < 1e-12);
        assert_eq!(h.grabs.len(), 1);
        // A ray crossing the axis at z = −3 sets Focus Distance 3.
        let ray = (DVec3::new(2.0, 0.0, -3.0), -DVec3::X);
        let Some(ItemChannelEdit::Camera(CameraEdit::FocusDistance(f))) =
            drag_edit(&scene, cam, Grab::CameraFocus, ray, -DVec3::Z, 1.0)
        else {
            panic!()
        };
        assert!((f - 3.0).abs() < 1e-9);
        // Point: Radius 0 is one cross at the centre; a drag grows it.
        let (mut scene, id) = scene_with(LightKind::Point);
        let h = handles_for(&scene, id, 1.0).unwrap();
        assert_eq!(h.grabs, vec![(Grab::PointRadius, DVec3::ZERO)]);
        assert!(h.circles.is_empty());
        let ray = (DVec3::new(0.4, 0.3, 5.0), -DVec3::Z);
        let Some(ItemChannelEdit::Light(LightEdit::Radius(r))) =
            drag_edit(&scene, id, Grab::PointRadius, ray, -DVec3::Z, 1.0)
        else {
            panic!()
        };
        assert!((r - 0.5).abs() < 1e-12);
        // Radius 0.5: three great circles, crosses at +X and +Y.
        scene.items[0].light.as_mut().unwrap().radius = 0.5;
        let h = handles_for(&scene, id, 1.0).unwrap();
        assert_eq!(h.circles.len(), 3 * 48);
        let at: Vec<DVec3> = h.grabs.iter().map(|g| g.1).collect();
        assert_eq!(at, vec![DVec3::X * 0.5, DVec3::Y * 0.5]);
        // Directional lights have none.
        let (scene, id) = scene_with(LightKind::Directional);
        assert!(handles_for(&scene, id, 1.0).is_none());
    }
}
