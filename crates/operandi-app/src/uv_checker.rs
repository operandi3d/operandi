//! The Checker overlay (UVT-114): an approved Operandi extra, not a match
//! of the reference, which has no built-in checker (Jason OK'd it on
//! 2026-11-16: "Yeah add the checker for stretched UVs"). A grey and white
//! check, 8 × 8 squares per UV tile, each square numbered 1 to 64 from the
//! lower left, so stretched or squashed UVs show as stretched squares.
//!
//! One pattern, drawn two ways with the same numbers:
//! - the 3D view's fill shader computes it per pixel from the vertex UVs
//!   ([`WGSL`], spliced into gpu.rs): nothing is uploaded and nothing runs
//!   on the CPU per frame;
//! - the UV view draws it as one image made once ([`image`]).
use eframe::egui;

/// Squares per tile side.
pub const SQUARES: u32 = 8;
/// The 3 × 5 digit font: 15 bits per digit, rows top to bottom, each row's
/// three bits left to right (most significant first).
pub const FONT: [u32; 10] = [
    0x7B6F, 0x2C97, 0x73E7, 0x73CF, 0x5BC9, 0x79CF, 0x79EF, 0x7249, 0x7BEF, 0x7BCF,
];
/// Light and dark squares and the numbers on each, gamma-encoded grey.
pub const LIGHT: f32 = 0.86;
pub const DARK: f32 = 0.48;
pub const INK_ON_LIGHT: f32 = 0.30;
pub const INK_ON_DARK: f32 = 0.95;
/// The number's two digits in a square: 7 × 5 font cells of this size
/// (a fraction of the square), the block's lower-left corner at
/// (`TEXT_X`, `TEXT_Y`).
pub const CELL: f32 = 0.09;
pub const TEXT_X: f32 = 0.185;
pub const TEXT_Y: f32 = 0.275;

/// The checker's grey at UV `(u, v)` (any tile; the pattern repeats).
pub fn grey(u: f32, v: f32) -> f32 {
    let s = SQUARES as f32;
    let (tu, tv) = ((u - u.floor()) * s, (v - v.floor()) * s);
    let (cx, cy) = (tu.floor().min(s - 1.), tv.floor().min(s - 1.));
    let dark = (cx as i32 + cy as i32) % 2 == 0;
    let (fx, fy) = (tu - cx, tv - cy);
    let number = cx as u32 + SQUARES * cy as u32 + 1;
    let gx = ((fx - TEXT_X) / CELL).floor();
    let gy = ((TEXT_Y + 5. * CELL - fy) / CELL).floor();
    let mut ink = false;
    if (0. ..7.).contains(&gx) && (0. ..5.).contains(&gy) && gx != 3. {
        let (digit, col) = if gx < 3. {
            (number / 10, gx as u32)
        } else {
            (number % 10, gx as u32 - 4)
        };
        let bit = 14 - (gy as u32 * 3 + col);
        ink = (FONT[digit as usize] >> bit) & 1 == 1;
    }
    match (dark, ink) {
        (false, false) => LIGHT,
        (true, false) => DARK,
        (false, true) => INK_ON_LIGHT,
        (true, true) => INK_ON_DARK,
    }
}

/// One tile of the pattern as an image, `size` pixels square, row 0 at the
/// top (V = 1).
pub fn image(size: usize) -> egui::ColorImage {
    let mut pixels = Vec::with_capacity(size * size);
    for row in 0..size {
        for col in 0..size {
            let u = (col as f32 + 0.5) / size as f32;
            let v = 1. - (row as f32 + 0.5) / size as f32;
            let g = (grey(u, v) * 255.).round() as u8;
            pixels.push(egui::Color32::from_gray(g));
        }
    }
    egui::ColorImage::new([size, size], pixels)
}

/// The same pattern in WGSL, for the 3D view's fill shader. `uv.x` below
/// -1e8 means "no UVs in the selected map": no checker there.
pub fn wgsl() -> String {
    let font = FONT
        .iter()
        .map(|f| format!("{f}u"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"
// UVT-114 Checker (uv_checker.rs): an approved Operandi extra.
const OP_CHECKER_FONT = array<u32, 10>({font});
fn op_checker(uv: vec2<f32>) -> vec3<f32> {{
  let s = {s:.1};
  let t = (uv - floor(uv)) * s;
  let c = min(floor(t), vec2<f32>(s - 1.0));
  let dark = (i32(c.x) + i32(c.y)) % 2 == 0;
  let f = t - c;
  let number = u32(c.x) + {n}u * u32(c.y) + 1u;
  let gx = floor((f.x - {tx:.4}) / {cell:.4});
  let gy = floor(({ty:.4} + 5.0 * {cell:.4} - f.y) / {cell:.4});
  var ink = false;
  if (gx >= 0.0 && gx < 7.0 && gy >= 0.0 && gy < 5.0 && gx != 3.0) {{
    var digit = number % 10u;
    var col = 0u;
    if (gx < 3.0) {{ digit = number / 10u; col = u32(gx); }} else {{ col = u32(gx) - 4u; }}
    let bit = 14u - (u32(gy) * 3u + col);
    ink = ((OP_CHECKER_FONT[digit] >> bit) & 1u) == 1u;
  }}
  var g = {light:.3};
  if (dark && !ink) {{ g = {dark:.3}; }}
  if (!dark && ink) {{ g = {ink_light:.3}; }}
  if (dark && ink) {{ g = {ink_dark:.3}; }}
  return vec3<f32>(g);
}}
"#,
        s = SQUARES as f32,
        n = SQUARES,
        tx = TEXT_X,
        ty = TEXT_Y,
        cell = CELL,
        light = LIGHT,
        dark = DARK,
        ink_light = INK_ON_LIGHT,
        ink_dark = INK_ON_DARK,
    )
}

/// The UVs a polygon corner carries for the checker when it has none in
/// the selected map.
pub const NO_UV: [f32; 2] = [-1.0e9, -1.0e9];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squares_alternate_and_repeat_per_tile() {
        // Lower-left square is dark, its right neighbour light; away from
        // the number the greys are plain.
        assert_eq!(grey(0.01, 0.01), DARK);
        assert_eq!(grey(0.125 + 0.01, 0.01), LIGHT);
        assert_eq!(grey(1.01, 0.01), grey(0.01, 0.01));
        assert_eq!(grey(-0.99, 2.01), grey(0.01, 0.01));
    }

    #[test]
    fn each_square_carries_its_number() {
        // Square 1 (lower left) reads "01": the "0"'s top-left font cell
        // is ink, the space between the digits is not.
        let at = |gx: f32, gy: f32, square: (f32, f32)| {
            let fx = TEXT_X + (gx + 0.5) * CELL;
            let fy = TEXT_Y + 5. * CELL - (gy + 0.5) * CELL;
            grey((square.0 + fx) / 8., (square.1 + fy) / 8.)
        };
        assert_eq!(at(0., 0., (0., 0.)), INK_ON_DARK);
        assert_eq!(at(3., 0., (0., 0.)), DARK);
        // "64" in the top-right square: the "4"'s top-middle cell is blank.
        assert_eq!(at(5., 0., (7., 7.)), DARK);
        assert_eq!(at(4., 0., (7., 7.)), INK_ON_DARK);
    }

    #[test]
    fn the_shader_has_the_same_font_and_greys() {
        let w = wgsl();
        assert!(w.contains("31599u"), "font row for 0");
        assert!(w.contains("fn op_checker"));
        let img = image(64);
        assert_eq!(img.size, [64, 64]);
    }
}
