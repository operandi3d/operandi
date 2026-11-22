//! SVG drawing of the visible items seen from the front (looking down -Z,
//! X right, Y up), orthographic. Polygons are filled with a gray that follows
//! how directly they face the viewer and are drawn back to front, so the
//! drawing reads like a flat-shaded front view. Provisional: a fixed front
//! view, not the current viewport camera.
use super::{WorldMesh, num};
use glam::{DVec2, DVec3};
use std::fmt::Write;

const WIDTH: f64 = 1024.0;
const MARGIN: f64 = 16.0;

pub(super) fn write(meshes: &[WorldMesh], _notes: &mut Vec<String>) -> Vec<u8> {
    let points = meshes.iter().flat_map(|m| m.mesh.vertices.iter());
    let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
    for p in points {
        lo = lo.min(p.truncate());
        hi = hi.max(p.truncate());
    }
    let size = (hi - lo).max(DVec2::splat(1e-9));
    let scale = (WIDTH - 2.0 * MARGIN) / size.x.max(size.y);
    let height = size.y * scale + 2.0 * MARGIN;
    let width = size.x * scale + 2.0 * MARGIN;
    // Flip Y: SVG's y grows downward.
    let to_svg =
        |p: DVec3| DVec2::new((p.x - lo.x) * scale + MARGIN, (hi.y - p.y) * scale + MARGIN);
    // (depth, element) so farther shapes draw first.
    let mut shapes: Vec<(f64, String)> = Vec::new();
    for m in meshes {
        for (f, face) in m.mesh.faces.iter().enumerate() {
            let corners: Vec<DVec3> = face
                .vertices
                .iter()
                .map(|&v| m.mesh.vertices[v as usize])
                .collect();
            let depth = corners.iter().map(|p| p.z).sum::<f64>() / corners.len() as f64;
            let list = corners
                .iter()
                .map(|&p| {
                    let s = to_svg(p);
                    format!("{},{}", num(s.x), num(s.y))
                })
                .collect::<Vec<_>>()
                .join(" ");
            let element = if m.polyline[f] {
                format!("<polyline points=\"{list}\" fill=\"none\" stroke=\"#202020\"/>")
            } else {
                let facing = m.normal(f).z.abs();
                let gray = (90.0 + 150.0 * facing).round() as u8;
                format!("<polygon points=\"{list}\" fill=\"#{gray:02x}{gray:02x}{gray:02x}\"/>")
            };
            shapes.push((depth, element));
        }
    }
    shapes.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = String::new();
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = num(width.ceil()),
        h = num(height.ceil())
    );
    out.push_str("<title>Operandi front view</title>\n");
    out.push_str("<g stroke=\"#303030\" stroke-width=\"0.75\" stroke-linejoin=\"round\">\n");
    for (_, element) in shapes {
        out.push_str(&element);
        out.push('\n');
    }
    out.push_str("</g>\n</svg>\n");
    out.into_bytes()
}
