//! Text primitive (Create panel spec v2, BEHAVIOR-TEXT.md): flat lettering
//! from a font, built as ordinary polygons.
//!
//! Provisional choices (Jason, 2026-11-05: "build a sensible version"):
//! - The text stays editable while the tool is on; Drop (or Apply) turns it
//!   into ordinary polygons. A dropped text can't be edited as text again
//!   (TXT-01/TXT-18 keep a special "text polygon"; not built yet).
//! - Letters with holes (A, B, O…) are split into a few simple polygons,
//!   because a polygon can't have a hole. Curves are flattened with a fixed
//!   number of steps (TXT-Q01).
//! - Size is the font's em size (TXT-12). Tracking is extra space after
//!   each letter, in metres (TXT-13). Location Bottom puts the baseline on
//!   the start point, Middle the middle of the capitals, Top their top.
//! - Fonts: two fonts that ship with the app (Ubuntu Light, the default,
//!   and Hack), then the fonts installed on the computer.
//! - Split, Output Type (Bezier) and Font Type aren't built.
use crate::{Face, Mesh, Result};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// Name of the default font (shipped with the app).
pub const DEFAULT_FONT: &str = "Ubuntu Light";
const HACK_FONT: &str = "Hack";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextLocation {
    Top,
    Middle,
    #[default]
    Bottom,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextJustification {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextParameters {
    pub text: String,
    pub font: String,
    /// Em size in metres.
    pub size: f64,
    /// Extra space after each letter, in metres.
    pub tracking: f64,
    /// The start point.
    pub position: DVec3,
    pub location: TextLocation,
    pub justification: TextJustification,
    /// Which way the text faces: 0 X, 1 Y, 2 Z (TXT-11 frames).
    pub axis: usize,
    pub flip: bool,
    pub make_uvs: bool,
}
impl Default for TextParameters {
    /// TXT-09 factory values (the font is ours).
    fn default() -> Self {
        Self {
            text: String::new(),
            font: DEFAULT_FONT.into(),
            size: 1.,
            tracking: 0.,
            position: DVec3::ZERO,
            location: TextLocation::Bottom,
            justification: TextJustification::Left,
            axis: 2,
            flip: false,
            make_uvs: false,
        }
    }
}
impl TextParameters {
    pub fn normalized(mut self) -> Self {
        self.axis = self.axis.min(2);
        self
    }
    /// The baseline and up directions of the text (TXT-11).
    pub fn frame(&self) -> (DVec3, DVec3) {
        match self.axis {
            0 => (DVec3::Z, DVec3::Y),
            1 => (DVec3::X, DVec3::NEG_Z),
            _ => (DVec3::X, DVec3::Y),
        }
    }
    pub fn validate(&self) -> Result<()> {
        if !self.size.is_finite() || self.size <= 1e-6 || self.size > 1e6 {
            return Err("Text Size must be between 1 µm and 1,000 km".into());
        }
        if !self.tracking.is_finite() || self.tracking.abs() > 1e6 {
            return Err("Text Tracking must be finite".into());
        }
        if !self.position.is_finite() || self.position.abs().max_element() > 1e9 {
            return Err("Text position must be finite and within the coordinate budget".into());
        }
        if self.text.chars().count() > 1000 {
            return Err("Text is limited to 1,000 characters".into());
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ fonts

#[derive(Clone)]
enum FontSource {
    Builtin(&'static [u8]),
    File(PathBuf, u32),
}

fn builtin_fonts() -> Vec<(String, FontSource)> {
    vec![
        (
            DEFAULT_FONT.into(),
            FontSource::Builtin(epaint_default_fonts::UBUNTU_LIGHT),
        ),
        (
            HACK_FONT.into(),
            FontSource::Builtin(epaint_default_fonts::HACK_REGULAR),
        ),
    ]
}

/// Folders searched for installed fonts (a few levels deep).
fn font_folders() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = [
        "/System/Library/Fonts",
        "/Library/Fonts",
        "/usr/share/fonts",
        "/usr/local/share/fonts",
        "C:\\Windows\\Fonts",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join("Library/Fonts"));
        dirs.push(home.join(".local/share/fonts"));
        dirs.push(home.join(".fonts"));
    }
    dirs
}

/// Fonts bigger than this aren't offered (colour emoji and the like).
const MAX_FONT_BYTES: u64 = 32 << 20;

fn scan_installed() -> Vec<(String, FontSource)> {
    let mut files = Vec::new();
    let mut stack: Vec<(PathBuf, u32)> = font_folders().into_iter().map(|d| (d, 0)).collect();
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                if depth < 4 {
                    stack.push((path, depth + 1));
                }
            } else if meta.len() <= MAX_FONT_BYTES
                && path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                    matches!(e.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc")
                })
            {
                files.push(path);
            }
        }
    }
    let mut found = BTreeMap::new();
    for path in files {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let count = ttf_parser::fonts_in_collection(&bytes).unwrap_or(1);
        for index in 0..count.min(64) {
            let Ok(face) = ttf_parser::Face::parse(&bytes, index) else {
                continue;
            };
            // Only fonts we can draw: they need outlines for Latin letters.
            if face.glyph_index('A').is_none() {
                continue;
            }
            let name = face_name(&face).unwrap_or_else(|| {
                path.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
            if name.is_empty() || name == DEFAULT_FONT || name == HACK_FONT {
                continue;
            }
            found
                .entry(name)
                .or_insert_with(|| FontSource::File(path.clone(), index));
        }
    }
    found.into_iter().collect()
}

fn face_name(face: &ttf_parser::Face) -> Option<String> {
    for id in [ttf_parser::name_id::FULL_NAME, ttf_parser::name_id::FAMILY] {
        if let Some(name) = face
            .names()
            .into_iter()
            .filter(|n| n.name_id == id && n.is_unicode())
            .find_map(|n| n.to_string())
        {
            return Some(name);
        }
    }
    None
}

fn registry() -> &'static Vec<(String, FontSource)> {
    static FONTS: OnceLock<Vec<(String, FontSource)>> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut all = builtin_fonts();
        all.extend(scan_installed());
        all
    })
}

/// Every font the Text tool offers, shipped fonts first.
pub fn font_names() -> Vec<String> {
    registry().iter().map(|(n, _)| n.clone()).collect()
}

/// Font file bytes and the face index inside the file.
type FontFile = (Arc<Vec<u8>>, u32);

fn font_data(name: &str) -> Result<FontFile> {
    static CACHE: OnceLock<Mutex<HashMap<String, FontFile>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(hit) = cache.lock().ok().and_then(|c| c.get(name).cloned()) {
        return Ok(hit);
    }
    // Shipped fonts are known without scanning the computer's fonts.
    let source = builtin_fonts()
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, s)| s)
        .or_else(|| {
            registry()
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, s)| s.clone())
        })
        .ok_or_else(|| format!("Font \"{name}\" isn't installed"))?;
    let loaded = match source {
        FontSource::Builtin(bytes) => (Arc::new(bytes.to_vec()), 0),
        FontSource::File(path, index) => (
            Arc::new(std::fs::read(&path).map_err(|e| format!("Can't read font \"{name}\": {e}"))?),
            index,
        ),
    };
    if let Ok(mut c) = cache.lock() {
        c.insert(name.into(), loaded.clone());
    }
    Ok(loaded)
}

// ---------------------------------------------------------------- outlines

/// Straight steps per quadratic / cubic curve piece.
const QUAD_STEPS: usize = 4;
const CUBIC_STEPS: usize = 6;

#[derive(Default)]
struct Outline {
    contours: Vec<Vec<DVec2>>,
    current: Vec<DVec2>,
    offset: DVec2,
    scale: f64,
}
impl Outline {
    fn at(&self, x: f32, y: f32) -> DVec2 {
        self.offset + DVec2::new(f64::from(x), f64::from(y)) * self.scale
    }
    fn last(&self) -> DVec2 {
        self.current.last().copied().unwrap_or(self.offset)
    }
}
impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        let p = self.at(x, y);
        self.current.push(p);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.at(x, y);
        self.current.push(p);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (p0, c, p1) = (self.last(), self.at(x1, y1), self.at(x, y));
        for i in 1..=QUAD_STEPS {
            let t = i as f64 / QUAD_STEPS as f64;
            let s = 1. - t;
            self.current.push(p0 * s * s + c * 2. * s * t + p1 * t * t);
        }
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (p0, c1, c2, p1) = (self.last(), self.at(x1, y1), self.at(x2, y2), self.at(x, y));
        for i in 1..=CUBIC_STEPS {
            let t = i as f64 / CUBIC_STEPS as f64;
            let s = 1. - t;
            self.current
                .push(p0 * s * s * s + c1 * 3. * s * s * t + c2 * 3. * s * t * t + p1 * t * t * t);
        }
    }
    fn close(&mut self) {
        let mut c = std::mem::take(&mut self.current);
        // Drop repeated points, including the closing copy of the first.
        c.dedup_by(|a, b| a.distance_squared(*b) < 1e-18);
        while c.len() > 1 && c[0].distance_squared(*c.last().unwrap()) < 1e-18 {
            c.pop();
        }
        if c.len() >= 3 && signed_area(&c).abs() > 1e-18 {
            self.contours.push(c);
        }
    }
}

fn signed_area(c: &[DVec2]) -> f64 {
    (0..c.len())
        .map(|i| c[i].perp_dot(c[(i + 1) % c.len()]))
        .sum::<f64>()
        / 2.
}
fn inside(p: DVec2, c: &[DVec2]) -> bool {
    let mut odd = false;
    let mut j = c.len() - 1;
    for i in 0..c.len() {
        let (a, b) = (c[i], c[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            odd = !odd;
        }
        j = i;
    }
    odd
}

/// Flat polygons (counter-clockwise, as 2D point lists) for one glyph's
/// contours: each filled region with its holes is triangulated, then the
/// triangles are joined back into as few simple polygons as possible.
fn fill(contours: Vec<Vec<DVec2>>) -> Vec<Vec<DVec2>> {
    // Nesting depth decides outside (even) and hole (odd), whatever the
    // font's own winding convention.
    let parent: Vec<Option<usize>> = (0..contours.len())
        .map(|i| {
            (0..contours.len())
                .filter(|&j| j != i && inside(contours[i][0], &contours[j]))
                .min_by(|&a, &b| {
                    signed_area(&contours[a])
                        .abs()
                        .total_cmp(&signed_area(&contours[b]).abs())
                })
        })
        .collect();
    let depth = |mut i: usize| {
        let mut d = 0;
        while let Some(p) = parent[i] {
            d += 1;
            i = p;
            if d > contours.len() {
                break;
            }
        }
        d
    };
    let mut out = Vec::new();
    for (i, outer) in contours.iter().enumerate() {
        if depth(i) % 2 == 1 {
            continue;
        }
        let mut points = oriented(outer.clone(), true);
        let mut holes: Vec<u32> = Vec::new();
        for (j, hole) in contours.iter().enumerate() {
            if parent[j] == Some(i) {
                holes.push(points.len() as u32);
                points.extend(oriented(hole.clone(), false));
            }
        }
        let mut triangles: Vec<u32> = Vec::new();
        earcut::Earcut::new().earcut(points.iter().map(|p| [p.x, p.y]), &holes, &mut triangles);
        let tris: Vec<Vec<u32>> = triangles
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| t.to_vec())
            .filter(|t| {
                let [a, b, c] = [0, 1, 2].map(|k| points[t[k] as usize]);
                (b - a).perp_dot(c - a) > 1e-14
            })
            .collect();
        let tris: Vec<Vec<u32>> = tris.into_iter().filter(|t| simple(t, &points)).collect();
        for poly in join(tris, &points) {
            out.push(poly.iter().map(|&k| points[k as usize]).collect());
        }
    }
    out
}

fn oriented(mut c: Vec<DVec2>, ccw: bool) -> Vec<DVec2> {
    if (signed_area(&c) > 0.) != ccw {
        c.reverse();
    }
    c
}

/// Greedily merge counter-clockwise polygons that share an edge while the
/// result stays a simple polygon (no point used twice).
fn join(mut polys: Vec<Vec<u32>>, points: &[DVec2]) -> Vec<Vec<u32>> {
    loop {
        let mut merged = false;
        'outer: for i in 0..polys.len() {
            for j in i + 1..polys.len() {
                if let Some(m) = merge(&polys[i], &polys[j]).filter(|m| simple(m, points)) {
                    polys[i] = m;
                    polys.swap_remove(j);
                    merged = true;
                    break 'outer;
                }
            }
        }
        if !merged {
            return polys;
        }
    }
}
fn merge(p: &[u32], q: &[u32]) -> Option<Vec<u32>> {
    let n = p.len();
    for i in 0..n {
        let (a, b) = (p[i], p[(i + 1) % n]);
        // q must run b -> a.
        let Some(k) = (0..q.len()).find(|&k| q[k] == b && q[(k + 1) % q.len()] == a) else {
            continue;
        };
        // p from b round to a, then q from a round to b, without the ends.
        let mut out: Vec<u32> = (0..n).map(|s| p[(i + 1 + s) % n]).collect();
        out.extend((1..q.len() - 1).map(|s| q[(k + 1 + s) % q.len()]));
        let mut seen = out.clone();
        seen.sort_unstable();
        seen.dedup();
        if seen.len() == out.len() && out.len() <= 4096 {
            return Some(out);
        }
    }
    None
}

/// The same boundary test the mesh uses before drawing a polygon: no
/// zero-length edge, and no two edges that touch or cross.
fn simple(poly: &[u32], points: &[DVec2]) -> bool {
    let q: Vec<DVec2> = poly.iter().map(|&k| points[k as usize]).collect();
    let n = q.len();
    let origin = q[0];
    let scale = q.iter().map(|p| (*p - origin).length()).fold(1., f64::max);
    let eps = scale * scale * 1e-12;
    let cross = |a: DVec2, b: DVec2, c: DVec2| (b - a).perp_dot(c - a);
    for i in 0..n {
        let (a, b) = (q[i], q[(i + 1) % n]);
        if (a - b).length_squared() <= eps {
            return false;
        }
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let (c, d) = (q[j], q[(j + 1) % n]);
            if cross(a, b, c) * cross(a, b, d) <= eps * eps
                && cross(c, d, a) * cross(c, d, b) <= eps * eps
                && a.min(b).cmple(c.max(d) + DVec2::splat(eps)).all()
                && c.min(d).cmple(a.max(b) + DVec2::splat(eps)).all()
            {
                return false;
            }
        }
    }
    signed_area(&q) > eps
}

/// Every polygon of the text as 2D points in metres on its own frame:
/// x along the baseline from the start point, y up.
fn layout(p: &TextParameters) -> Result<Vec<Vec<DVec2>>> {
    let (bytes, index) = font_data(&p.font)?;
    let face = ttf_parser::Face::parse(&bytes, index)
        .map_err(|e| format!("Font \"{}\" can't be read: {e}", p.font))?;
    let scale = p.size / f64::from(face.units_per_em());
    let line_height =
        f64::from(face.ascender() - face.descender() + face.line_gap()).max(1.) * scale;
    let cap = face
        .capital_height()
        .map(|h| f64::from(h) * scale)
        .unwrap_or(0.7 * p.size);
    let shift_y = match p.location {
        TextLocation::Bottom => 0.,
        TextLocation::Middle => -cap / 2.,
        TextLocation::Top => -cap,
    };
    let mut polys = Vec::new();
    for (row, line) in p.text.split('\n').enumerate() {
        // First pass: the line's width (letter advances plus tracking).
        let advance = |c: char| {
            face.glyph_index(c)
                .and_then(|g| face.glyph_hor_advance(g))
                .map_or(0.5 * p.size, |a| f64::from(a) * scale)
        };
        let chars: Vec<char> = line.chars().filter(|c| !c.is_control()).collect();
        let width = chars.iter().map(|&c| advance(c) + p.tracking).sum::<f64>()
            - if chars.is_empty() { 0. } else { p.tracking };
        let mut x = match p.justification {
            TextJustification::Left => 0.,
            TextJustification::Center => -width / 2.,
            TextJustification::Right => -width,
        };
        let y = shift_y - row as f64 * line_height;
        for c in chars {
            if let Some(g) = face.glyph_index(c) {
                let mut outline = Outline {
                    offset: DVec2::new(x, y),
                    scale,
                    ..Default::default()
                };
                if face.outline_glyph(g, &mut outline).is_some() {
                    outline_close(&mut outline);
                    polys.extend(fill(outline.contours));
                }
            }
            x += advance(c) + p.tracking;
        }
    }
    Ok(polys)
}
fn outline_close(o: &mut Outline) {
    ttf_parser::OutlineBuilder::close(o);
}

/// The text as a mesh, in the frame given by Axis, Position and Flip.
/// Empty text (or only spaces) gives an empty mesh.
pub fn text_geometry(parameters: &TextParameters) -> Result<Mesh> {
    let p = parameters.clone().normalized();
    p.validate()?;
    let polys = layout(&p)?;
    let (along, up) = p.frame();
    let mut mesh = Mesh::default();
    let (lo, hi) = polys.iter().flatten().fold(
        (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN)),
        |(lo, hi), q| (lo.min(*q), hi.max(*q)),
    );
    let span = (hi - lo).max_element().max(1e-9);
    let mut uvs = p.make_uvs.then(Vec::new);
    for poly in polys {
        let start = mesh.vertices.len() as u32;
        if start as usize + poly.len() > 1_000_000 {
            return Err("Text exceeds the polygon budget; use fewer letters".into());
        }
        mesh.vertices
            .extend(poly.iter().map(|q| p.position + along * q.x + up * q.y));
        let mut vertices: Vec<u32> = (start..start + poly.len() as u32).collect();
        let mut corner_uvs: Vec<Option<DVec2>> =
            poly.iter().map(|q| Some((*q - lo) / span)).collect();
        if p.flip {
            vertices.reverse();
            corner_uvs.reverse();
        }
        mesh.faces.push(Face {
            vertices,
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
        if let Some(uvs) = &mut uvs {
            uvs.push(corner_uvs);
        }
    }
    if let Some(uvs) = uvs {
        mesh.uv_maps.insert(crate::PRIMITIVE_UV_MAP.into(), uvs);
    }
    mesh.validate()?;
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> TextParameters {
        TextParameters {
            text: s.into(),
            ..Default::default()
        }
    }

    #[test]
    fn letters_become_flat_polygons_facing_the_axis() {
        let m = text_geometry(&text("AH")).unwrap();
        assert!(!m.faces.is_empty());
        for f in 0..m.faces.len() {
            assert!(m.normal(f).z > 0.99, "face {f} faces {}", m.normal(f));
            m.surface_triangles(f).unwrap();
        }
        assert!(m.vertices.iter().all(|v| v.z == 0.));
    }

    #[test]
    fn a_letter_with_a_hole_leaves_the_hole_open() {
        let m = text_geometry(&text("O")).unwrap();
        assert!(m.faces.len() >= 2, "an O needs at least two polygons");
        // The middle of the O is not covered by any polygon.
        let (lo, hi) = m.vertices.iter().fold(
            (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
            |(lo, hi), v| (lo.min(*v), hi.max(*v)),
        );
        let centre = (lo + hi) / 2.;
        let area_at = |p: DVec3| {
            (0..m.faces.len()).any(|f| {
                m.surface_triangles(f).unwrap().iter().any(|t| {
                    let [a, b, c] =
                        t.map(|k| m.vertices[m.faces[f].vertices[k] as usize].truncate());
                    let q = p.truncate();
                    let s = |u: DVec2, v: DVec2| (v - u).perp_dot(q - u);
                    s(a, b) >= 0. && s(b, c) >= 0. && s(c, a) >= 0.
                })
            })
        };
        assert!(!area_at(centre));
    }

    #[test]
    fn size_tracking_and_justification() {
        let one = text_geometry(&text("H")).unwrap();
        let height = |m: &Mesh| {
            m.vertices.iter().map(|v| v.y).fold(f64::MIN, f64::max)
                - m.vertices.iter().map(|v| v.y).fold(f64::MAX, f64::min)
        };
        // Size is the em size: a capital is a bit less than Size tall.
        assert!(height(&one) > 0.5 && height(&one) < 1., "{}", height(&one));
        let two = text_geometry(&TextParameters {
            size: 2.,
            ..text("H")
        })
        .unwrap();
        assert!((height(&two) - 2. * height(&one)).abs() < 1e-9);
        let max_x = |m: &Mesh| m.vertices.iter().map(|v| v.x).fold(f64::MIN, f64::max);
        let plain = text_geometry(&text("HH")).unwrap();
        let tracked = text_geometry(&TextParameters {
            tracking: 0.5,
            ..text("HH")
        })
        .unwrap();
        assert!((max_x(&tracked) - max_x(&plain) - 0.5).abs() < 1e-9);
        let right = text_geometry(&TextParameters {
            justification: TextJustification::Right,
            ..text("HH")
        })
        .unwrap();
        assert!(max_x(&right) <= 1e-9);
    }

    #[test]
    fn axis_y_reads_from_above_and_flip_turns_it_over() {
        let m = text_geometry(&TextParameters {
            axis: 1,
            ..text("L")
        })
        .unwrap();
        assert!(m.normal(0).y > 0.99);
        assert!(m.vertices.iter().all(|v| v.y == 0.));
        let f = text_geometry(&TextParameters {
            axis: 1,
            flip: true,
            ..text("L")
        })
        .unwrap();
        assert!(f.normal(0).y < -0.99);
    }

    #[test]
    fn empty_text_and_unknown_font() {
        assert!(text_geometry(&text("")).unwrap().faces.is_empty());
        assert!(text_geometry(&text("   ")).unwrap().faces.is_empty());
        assert!(
            text_geometry(&TextParameters {
                font: "No Such Font 123".into(),
                ..text("A")
            })
            .is_err()
        );
    }

    #[test]
    fn every_printable_ascii_letter_builds() {
        let all: String = (33u8..127).map(char::from).collect();
        let m = text_geometry(&text(&all)).unwrap();
        for f in 0..m.faces.len() {
            m.surface_triangles(f)
                .unwrap_or_else(|e| panic!("face {f}: {e}"));
        }
    }
}
