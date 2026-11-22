//! Angle snapping, shared by every tool that turns a direction onto a
//! nicer one: the Ruler's 15° steps, Polygon Slice's Angle Snap and Pen
//! Slice's self-snaps. Point snapping (onto mesh points and the grid)
//! lives in the app's snap service, because it works on screen distances.
use glam::DVec3;

/// `d` turned about `n` to the nearest multiple of `step` (radians),
/// measured in the plane of the orthonormal axes `u` and `v` from `u`
/// toward `v`. The length in the plane and the part along `n` are kept.
/// A `d` with no length in the plane comes back unchanged.
pub fn round_in_plane(d: DVec3, u: DVec3, v: DVec3, n: DVec3, step: f64) -> DVec3 {
    let off = d.dot(n);
    let flat = d - n * off;
    let length = flat.length();
    if length < 1e-12 || step.is_nan() || step <= 0. {
        return d;
    }
    let angle = (flat.dot(v).atan2(flat.dot(u)) / step).round() * step;
    (u * angle.cos() + v * angle.sin()) * length + n * off
}

/// The candidate direction (either way along it) nearest the unit
/// direction `dir`, when it is within `max_angle` (radians). Candidates
/// shorter than 0.5 are ignored; the result is unit length when the
/// candidates are.
pub fn nearest_direction(
    dir: DVec3,
    candidates: impl IntoIterator<Item = DVec3>,
    max_angle: f64,
) -> Option<DVec3> {
    let limit = max_angle.cos();
    candidates
        .into_iter()
        .filter(|a| a.length() > 0.5)
        .flat_map(|a| [a, -a])
        .map(|a| (a.dot(dir), a))
        .filter(|(c, _)| *c >= limit)
        .max_by(|x, y| x.0.total_cmp(&y.0))
        .map(|(_, a)| a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_to_the_nearest_step_and_keeps_length_and_height() {
        let d = DVec3::new(1., 0.4, 0.3);
        let out = round_in_plane(d, DVec3::X, DVec3::Z, DVec3::Y, 15f64.to_radians());
        assert!((out.z.atan2(out.x).to_degrees() - 15.).abs() < 1e-9);
        assert!((out.y - 0.4).abs() < 1e-12);
        let flat = |p: DVec3| DVec3::new(p.x, 0., p.z).length();
        assert!((flat(out) - flat(d)).abs() < 1e-12);
    }

    #[test]
    fn straight_along_the_normal_or_no_step_is_unchanged() {
        let up = DVec3::new(0., 2., 0.);
        assert_eq!(round_in_plane(up, DVec3::X, DVec3::Z, DVec3::Y, 0.2), up);
        let d = DVec3::new(1., 0., 0.3);
        assert_eq!(round_in_plane(d, DVec3::X, DVec3::Z, DVec3::Y, 0.), d);
    }

    #[test]
    fn nearest_direction_takes_either_way_within_the_limit() {
        let dir = DVec3::new(-1., 0.05, 0.).normalize();
        let got = nearest_direction(dir, [DVec3::X, DVec3::Y], 5f64.to_radians());
        assert_eq!(got, Some(-DVec3::X));
        assert_eq!(
            nearest_direction(
                DVec3::new(1., 1., 0.).normalize(),
                [DVec3::X],
                5f64.to_radians()
            ),
            None
        );
        assert_eq!(nearest_direction(dir, [DVec3::ZERO], 1.), None);
    }
}
