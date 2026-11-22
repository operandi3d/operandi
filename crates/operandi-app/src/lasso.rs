//! Original screen-space enclosure rules approved provisionally for Operandi.
//! Facing eligibility is supplied separately; this module performs no depth test.
use eframe::egui::{Pos2, Rect, vec2};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    #[default]
    Freeform,
    Rectangle,
    Circle,
    Ellipse,
}
impl Style {
    pub const ALL: [Self; 4] = [Self::Freeform, Self::Rectangle, Self::Circle, Self::Ellipse];
    pub fn label(self) -> &'static str {
        match self {
            Self::Freeform => "Freeform",
            Self::Rectangle => "Rectangle",
            Self::Circle => "Circle",
            Self::Ellipse => "Ellipse",
        }
    }
}
pub fn outline(style: Style, path: &[Pos2]) -> Vec<Pos2> {
    let (Some(&start), Some(&end)) = (path.first(), path.last()) else {
        return vec![];
    };
    match style {
        Style::Freeform => path.to_vec(),
        Style::Rectangle => {
            let r = Rect::from_two_pos(start, end);
            vec![
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
            ]
        }
        Style::Circle | Style::Ellipse => {
            let r = Rect::from_two_pos(start, end);
            let (center, radius) = if style == Style::Circle {
                (start, vec2(start.distance(end), start.distance(end)))
            } else {
                (r.center(), r.size() * 0.5)
            };
            (0..128)
                .map(|i| {
                    let angle = i as f32 * std::f32::consts::TAU / 128.;
                    center + vec2(angle.cos() * radius.x, angle.sin() * radius.y)
                })
                .collect()
        }
    }
}
fn cross(a: eframe::egui::Vec2, b: eframe::egui::Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}
fn on_segment(p: Pos2, a: Pos2, b: Pos2) -> bool {
    super::input::segment_distance(p, a, b) <= 0.001
}
pub fn contains(region: &[Pos2], p: Pos2) -> bool {
    let mut inside = false;
    for (a, b) in segments(region) {
        if on_segment(p, a, b) {
            return true;
        }
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}
fn segments(points: &[Pos2]) -> impl Iterator<Item = (Pos2, Pos2)> + '_ {
    points
        .iter()
        .copied()
        .zip(points.iter().copied().cycle().skip(1))
        .take(points.len())
}
fn intersection(a: Pos2, b: Pos2, c: Pos2, d: Pos2) -> Option<f32> {
    let denominator = cross(b - a, d - c);
    if denominator.abs() < 1e-8 {
        return None;
    }
    let t = cross(c - a, d - c) / denominator;
    let u = cross(c - a, b - a) / denominator;
    ((0. ..=1.).contains(&t) && (0. ..=1.).contains(&u)).then_some(t)
}
fn full_edge(region: &[Pos2], a: Pos2, b: Pos2) -> bool {
    if !contains(region, a) || !contains(region, b) {
        return false;
    }
    let mut cuts = vec![0., 1.];
    cuts.extend(segments(region).filter_map(|(c, d)| intersection(a, b, c, d)));
    cuts.sort_by(f32::total_cmp);
    cuts.windows(2)
        .all(|t| contains(region, a + (b - a) * ((t[0] + t[1]) * 0.5)))
}
pub fn eligible(region: &[Pos2], element: &[Pos2], partial: bool) -> bool {
    if region.len() < 3 || element.is_empty() {
        return false;
    }
    if element.len() == 1 {
        return contains(region, element[0]);
    }
    let element_edges: Vec<_> = if element.len() == 2 {
        vec![(element[0], element[1])]
    } else {
        segments(element).collect()
    };
    if partial {
        element.iter().any(|&p| contains(region, p))
            || element_edges.iter().any(|&(a, b)| {
                segments(region)
                    .any(|(c, d)| intersection(a, b, c, d).is_some() || on_segment(c, a, b))
            })
            || (element.len() > 2 && region.iter().any(|&p| contains(element, p)))
    } else {
        element_edges.iter().all(|&(a,b)|full_edge(region,a,b))
        // A lasso boundary wholly inside a polygon encloses only part of it.
        && (element.len()==2 || !segments(region).any(|(a,b)| {
            let midpoint=a+(b-a)*0.5;
            contains(element,midpoint) && !segments(element).any(|(c,d)|on_segment(midpoint,c,d))
        }))
    }
}
/// The outline with points that move it by less than `tolerance` removed
/// (Ramer–Douglas–Peucker). A hand-drawn lasso records a point per pixel of
/// pointer travel; the hit test's cost grows with the point count, and
/// sub-pixel wiggles never change which components are enclosed.
pub fn simplify(points: &[Pos2], tolerance: f32) -> Vec<Pos2> {
    if points.len() <= 4 {
        return points.to_vec();
    }
    // Split the closed loop at the point farthest from the start so both
    // halves are open chains with distinct ends.
    let far = (1..points.len())
        .max_by(|&a, &b| {
            points[0]
                .distance_sq(points[a])
                .total_cmp(&points[0].distance_sq(points[b]))
        })
        .unwrap_or(0);
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[far] = true;
    let mut stack = vec![(0, far), (far, points.len())];
    while let Some((start, end)) = stack.pop() {
        let a = points[start];
        let b = points[end % points.len()];
        let mut best = (0., start);
        for (i, &p) in points.iter().enumerate().take(end).skip(start + 1) {
            let d = super::input::segment_distance(p, a, b);
            if d > best.0 {
                best = (d, i);
            }
        }
        if best.0 > tolerance {
            keep[best.1] = true;
            stack.push((start, best.1));
            stack.push((best.1, end));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(&p, k)| k.then_some(p))
        .collect()
}
pub fn valid(region: &[Pos2]) -> bool {
    region.len() >= 3
        && segments(region)
            .any(|(a, b)| segments(region).any(|(c, d)| cross(b - a, d - c).abs() > 0.001))
}
#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;
    #[test]
    fn boundary_full_partial_and_inside_polygon() {
        let r = outline(Style::Rectangle, &[pos2(0., 0.), pos2(10., 10.)]);
        assert!(eligible(&r, &[pos2(0., 0.), pos2(10., 0.)], false));
        assert!(!eligible(&r, &[pos2(-1., 5.), pos2(5., 5.)], false));
        assert!(eligible(&r, &[pos2(-1., 5.), pos2(5., 5.)], true));
        let enclosing = [
            pos2(-1., -1.),
            pos2(11., -1.),
            pos2(11., 11.),
            pos2(-1., 11.),
        ];
        assert!(!eligible(&r, &enclosing, false));
        assert!(eligible(&r, &enclosing, true));
        assert!(!valid(&[pos2(0., 0.), pos2(0., 0.), pos2(0., 0.)]));
    }
    #[test]
    fn simplify_keeps_shape_and_drops_pixel_steps() {
        // A 400-point square traced one point per unit, plus a notch.
        let mut path = vec![];
        for i in 0..100 {
            path.push(pos2(i as f32, 0.));
        }
        for i in 0..100 {
            path.push(pos2(100., i as f32));
        }
        for i in 0..100 {
            path.push(pos2(100. - i as f32, 100.));
        }
        for i in 0..100 {
            path.push(pos2(0., 100. - i as f32 + 0.2 * (i % 2) as f32));
        }
        let simple = simplify(&path, 0.5);
        assert!(simple.len() <= 8, "{} points left", simple.len());
        for p in [pos2(50., 50.), pos2(1., 1.), pos2(99., 99.)] {
            assert_eq!(contains(&simple, p), contains(&path, p));
        }
        assert!(!contains(&simple, pos2(101., 50.)));
    }
    #[test]
    fn concave_enclosure_and_all_styles() {
        let r = [
            pos2(0., 0.),
            pos2(10., 0.),
            pos2(10., 10.),
            pos2(6., 10.),
            pos2(6., 4.),
            pos2(4., 4.),
            pos2(4., 10.),
            pos2(0., 10.),
        ];
        assert!(!eligible(&r, &[pos2(2., 8.), pos2(8., 8.)], false));
        for style in Style::ALL {
            assert!(valid(&outline(
                style,
                &[pos2(0., 0.), pos2(10., 0.), pos2(10., 10.)]
            )));
        }
    }
}
