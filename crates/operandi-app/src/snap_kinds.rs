//! The twelve snap target types (snapping spec v2, snapping-2026-11-14,
//! SN2-22..37) and the small rules each one follows. Items marked
//! "Operandi choice" in the spec are built as written and may change when
//! its open questions (Q-01..13) are answered: each rule is one small
//! function here, so a change touches only that function. The snap
//! service (`snap_service.rs`) gathers and searches the targets; nothing
//! here knows about tools.
use crate::camera::Projector;
use eframe::egui::Pos2;
use glam::DVec3;

/// One target type, in the order Snapping Options lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Grid,
    Vertex,
    Edge,
    EdgeCenter,
    Polygon,
    PolygonCenter,
    Intersection,
    Box,
    Pivot,
    WorldAxis,
    StraightLine,
    RightAngle,
}

impl Kind {
    pub const ALL: [Kind; 12] = [
        Kind::Grid,
        Kind::Vertex,
        Kind::Edge,
        Kind::EdgeCenter,
        Kind::Polygon,
        Kind::PolygonCenter,
        Kind::Intersection,
        Kind::Box,
        Kind::Pivot,
        Kind::WorldAxis,
        Kind::StraightLine,
        Kind::RightAngle,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Kind::Grid => "Grid",
            Kind::Vertex => "Vertex",
            Kind::Edge => "Edge",
            Kind::EdgeCenter => "Edge Center",
            Kind::Polygon => "Polygon",
            Kind::PolygonCenter => "Polygon Center",
            Kind::Intersection => "Intersection",
            Kind::Box => "Box",
            Kind::Pivot => "Pivot",
            Kind::WorldAxis => "World Axis",
            Kind::StraightLine => "Straight Line",
            Kind::RightAngle => "Right Angle",
        }
    }
    /// The row's tooltip (UIC-04): what the target is.
    pub fn tip(self) -> &'static str {
        match self {
            Kind::Grid => "Snap to grid crossings",
            Kind::Vertex => "Snap to any vertex",
            Kind::Edge => "Snap to any point along an edge",
            Kind::EdgeCenter => "Snap to the middle of an edge",
            Kind::Polygon => {
                "Snap to any point on a polygon's surface, when nothing more specific is in range"
            }
            Kind::PolygonCenter => "Snap to the average of a polygon's vertices",
            Kind::Intersection => "Snap to where an edge passes through a polygon",
            Kind::Box => "Snap to the corners of each item's box and of one box around them all",
            Kind::Pivot => "Snap to an item's pivot",
            Kind::WorldAxis => "Pen: a new segment locks to the nearest world axis",
            Kind::StraightLine => "Pen: a new segment locks onto the line of the one before",
            Kind::RightAngle => "Pen: a new segment locks at a right angle to the one before",
        }
    }
    /// Whether points of this type are tested for being hidden behind a
    /// surface (SN2-17). Pivots and boxes aren't surface geometry, so they
    /// always count (Operandi choice).
    pub fn occluded(self) -> bool {
        matches!(
            self,
            Kind::Vertex | Kind::Edge | Kind::EdgeCenter | Kind::PolygonCenter | Kind::Intersection
        )
    }
    /// SN2-15: candidates within [`TIE_PX`] of each other go to the more
    /// specific type; lower wins.
    pub fn priority(self) -> u8 {
        match self {
            Kind::Vertex => 0,
            Kind::EdgeCenter => 1,
            Kind::PolygonCenter => 2,
            Kind::Pivot => 3,
            Kind::Box => 4,
            Kind::Intersection => 5,
            Kind::Edge => 6,
            Kind::Polygon => 7,
            Kind::Grid => 8,
            Kind::WorldAxis | Kind::StraightLine | Kind::RightAngle => 9,
        }
    }
}

/// SN2-15: how close two candidates must be, in screen points, to count
/// as a tie that the type order settles.
pub const TIE_PX: f32 = 2.;

/// Where the path being drawn or dragged has got to, for Straight Line
/// and Right Angle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Path {
    /// The point the new segment starts from: the shape's last point, or
    /// where a dragged handle started.
    pub from: Option<DVec3>,
    /// The point before `from`, when the shape has a segment already.
    pub before: Option<DVec3>,
    /// The work plane's normal (Right Angle turns in that plane).
    pub normal: DVec3,
}

/// SN2-32: the 8 corners of a box `lo`..`hi`.
pub fn box_corners(lo: DVec3, hi: DVec3) -> impl Iterator<Item = DVec3> {
    (0..8).map(move |n| {
        DVec3::new(
            if n & 1 == 0 { lo.x } else { hi.x },
            if n & 2 == 0 { lo.y } else { hi.y },
            if n & 4 == 0 { lo.z } else { hi.z },
        )
    })
}

/// SN2-34..36 (SNH-Q01 answered): the lock lines through `path.from`, as
/// unit directions in the work plane: the world axes (World Axis), the
/// previous segment's continuation (Straight Line, provisional SNH-Q03)
/// and its right angle in the work plane (Right Angle). An axis along the
/// plane's normal has no line in the plane and is left out.
pub fn lock_lines(
    world_axis: bool,
    straight_line: bool,
    right_angle: bool,
    path: &Path,
) -> Vec<DVec3> {
    let n = path.normal.normalize_or_zero();
    let flat = |v: DVec3| (v - n * v.dot(n)).normalize_or_zero();
    let mut lines = Vec::new();
    if world_axis {
        lines.extend([DVec3::X, DVec3::Y, DVec3::Z].map(flat));
    }
    if let (Some(from), Some(before)) = (path.from, path.before) {
        let previous = flat(from - before);
        if straight_line {
            lines.push(previous);
        }
        if right_angle {
            lines.push(n.cross(previous).normalize_or_zero());
        }
    }
    lines.retain(|d| *d != DVec3::ZERO);
    lines
}

/// SN2-34..36 (SNH-Q01 answered): a dragged path point locks onto the
/// nearest lock line through `path.from` when it is within `radius` screen
/// points of that line (Inner Range), whatever the segment's length; Snap
/// Angle plays no part. On a lock the point is projected straight onto the
/// line. `project` gives a world point's screen position. Unchanged when
/// no lock is ticked, there is no `from`, or no line is near.
pub fn lock_segment(
    world_axis: bool,
    straight_line: bool,
    right_angle: bool,
    path: &Path,
    point: DVec3,
    project: impl Fn(DVec3) -> Option<Pos2>,
    radius: f32,
) -> DVec3 {
    let Some(from) = path.from else {
        return point;
    };
    let Some(at) = project(point) else {
        return point;
    };
    let Some(origin) = project(from) else {
        return point;
    };
    let mut best: Option<(f32, DVec3)> = None;
    for dir in lock_lines(world_axis, straight_line, right_angle, path) {
        let Some(along) = project(from + dir) else {
            continue;
        };
        let screen = along - origin;
        if screen.length() < 1e-3 {
            // The line runs into the screen: no line to be near.
            continue;
        }
        let u = screen.normalized();
        let off = at - origin;
        let distance = (off.x * u.y - off.y * u.x).abs();
        if distance <= radius && best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, dir));
        }
    }
    match best {
        Some((_, dir)) => from + dir * (point - from).dot(dir),
        None => point,
    }
}

/// The nearest point of segment `a`..`b` to `at` on screen: (world point,
/// screen point, distance). The screen parameter is turned back into the
/// world one, so the point lies on the 3D edge under the pointer.
pub fn nearest_on_segment(
    projector: &Projector,
    a: DVec3,
    b: DVec3,
    at: Pos2,
) -> Option<(DVec3, Pos2, f32)> {
    let (qa, za) = projector.project(a)?;
    let (qb, zb) = projector.project(b)?;
    let delta = qb - qa;
    let t = ((at - qa).dot(delta) / delta.length_sq().max(1e-8)).clamp(0., 1.);
    let s = projector.world_parameter(t, za, zb);
    let world = a.lerp(b, s);
    let (screen, _) = projector.project(world)?;
    Some((world, screen, screen.distance(at)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_corners_are_the_8_corners() {
        let corners: Vec<DVec3> = box_corners(DVec3::ZERO, DVec3::new(2., 4., 6.)).collect();
        assert_eq!(corners.len(), 8);
        assert!(corners.contains(&DVec3::new(2., 4., 6.)));
        assert!(corners.contains(&DVec3::new(0., 4., 0.)));
    }

    /// Plan view of XZ: 100 screen points a metre, Z down the screen.
    fn top(p: DVec3) -> Option<Pos2> {
        Some(Pos2::new(p.x as f32 * 100., p.z as f32 * 100.))
    }

    #[test]
    fn a_point_within_inner_range_of_a_lock_line_lands_on_it() {
        let alone = Path {
            from: Some(DVec3::ZERO),
            before: None,
            normal: DVec3::Y,
        };
        // 20 px off world X, 3 m out: locks, projected straight on.
        let p = lock_segment(
            true,
            false,
            false,
            &alone,
            DVec3::new(3., 0., 0.2),
            top,
            24.,
        );
        assert_eq!(p, DVec3::new(3., 0., 0.));
        // 30 px off: free, whatever the angle.
        let free = DVec3::new(3., 0., 0.3);
        assert_eq!(
            lock_segment(true, false, false, &alone, free, top, 24.),
            free
        );
        // Close in, a large angle still locks (pixels, not degrees).
        let q = lock_segment(
            true,
            false,
            false,
            &alone,
            DVec3::new(0.1, 0., 0.1),
            top,
            24.,
        );
        assert!(q.z == 0. || q.x == 0., "{q}");
        // Nothing ticked: unchanged.
        assert_eq!(
            lock_segment(false, false, false, &alone, free, top, 24.),
            free
        );
    }

    #[test]
    fn straight_line_and_right_angle_lines_follow_the_previous_segment() {
        let path = Path {
            from: Some(DVec3::new(1., 0., 1.)),
            before: Some(DVec3::ZERO),
            normal: DVec3::Y,
        };
        let diagonal = DVec3::new(1., 0., 1.).normalize();
        // Straight on, 10 px off the continuation.
        let p = lock_segment(
            false,
            true,
            false,
            &path,
            DVec3::new(2.05, 0., 1.95),
            top,
            24.,
        );
        assert!(
            ((p - DVec3::new(1., 0., 1.)).normalize() - diagonal).length() < 1e-9,
            "{p}"
        );
        // At right angles.
        let q = lock_segment(
            false,
            false,
            true,
            &path,
            DVec3::new(1.55, 0., 0.5),
            top,
            24.,
        );
        assert!(
            (q - DVec3::new(1., 0., 1.)).dot(diagonal).abs() < 1e-9,
            "{q}"
        );
    }
}
