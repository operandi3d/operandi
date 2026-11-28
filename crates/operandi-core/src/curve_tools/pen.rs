//! PEN-N01..05: original Operandi atomic numeric route, not a mouse Pen session.
//! Geometry and input correspondence are portable; Editor owns publication/history.
use crate::*;
use glam::DVec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PenType {
    Polygon,
    Vertices,
    /// One Subdivs polygon (pen-2026-11-05 rule 4).
    Subdivs,
    Polyline,
    /// One Catmull-Clark polygon.
    CatmullClark,
    /// One open B-Spline curve through the points.
    BSpline,
}
impl PenType {
    /// The types that make one polygon (and walls).
    pub fn surface(self) -> bool {
        matches!(self, Self::Polygon | Self::Subdivs | Self::CatmullClark)
    }
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PenInput {
    #[serde(deserialize_with = "positions")]
    pub positions_m: Vec<[f64; 3]>,
    #[serde(rename = "type")]
    pub output: PenType,
    pub flip: bool,
    pub close: bool,
    pub select_new: bool,
    #[serde(default)]
    pub merge: bool,
    #[serde(default)]
    pub make_quads: bool,
    #[serde(default = "wall_off")]
    pub wall: String,
    #[serde(default)]
    pub make_uvs: bool,
}
fn positions<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<[f64; 3]>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Coordinate {
        Number(f64),
        Expression(String),
    }
    let raw = Vec::<[Coordinate; 3]>::deserialize(d)?;
    raw.into_iter()
        .map(|point| {
            let mut out = [0.; 3];
            for (i, value) in point.into_iter().enumerate() {
                out[i] = match value {
                    Coordinate::Number(v) => v,
                    Coordinate::Expression(s) => {
                        numeric::parse_quantity(&s, numeric::Quantity::Distance)
                            .map_err(serde::de::Error::custom)?
                    }
                };
            }
            Ok(out)
        })
        .collect()
}
fn wall_off() -> String {
    "off".into()
}
impl PenInput {
    /// No implicit fixture or retained point slots. Unknown fields/types refuse,
    /// including interactive Current Point, walls, projections and Show requests.
    pub fn parse(payload: &str) -> Result<Self> {
        if payload.len() > 16_384 {
            return Err("Pen input is larger than 16 KiB".into());
        }
        serde_json::from_str(payload).map_err(|e| format!("Pen input is invalid: {e}. Use polygon, vertices or polyline with four XYZ positions"))
    }
    /// The numeric route builds what the same clicks would (pen-2026-11-05):
    /// any number of points, in entry order, with the panel's options.
    /// Make Quads with four points is the same single quad (PN06); more points
    /// are refused until the strip rule is specified (Q-PEN-04).
    pub fn geometry(&self) -> Result<PenResult> {
        let points: Vec<_> = self
            .positions_m
            .iter()
            .copied()
            .map(DVec3::from_array)
            .collect();
        if points
            .iter()
            .any(|p| !p.is_finite() || p.abs().max_element() > 1e9)
        {
            return Err("Pen positions must be finite and within the mesh size limit".into());
        }
        if self.make_quads && points.len() != 4 {
            return Err("Pen Make Quads is specified for four points only".into());
        }
        let wall = match self.wall.as_str() {
            "off" => WallMode::Off,
            "inner" => WallMode::Inner,
            "outer" => WallMode::Outer,
            "both" => WallMode::BothSides,
            other => {
                return Err(format!(
                    "Pen wall must be off, inner, outer or both (not {other:?})"
                ));
            }
        };
        let options = PenOptions {
            output: self.output,
            flip: self.flip,
            close: self.close,
            select_new: self.select_new,
            wall,
            make_uvs: self.make_uvs,
            plane_normal: DVec3::Z,
            ..Default::default()
        };
        options.validate()?;
        if points.len() < options.minimum_points() {
            return Err("Pen needs more positions for this type".into());
        }
        let mesh = pen_geometry(&points, &options)?;
        mesh.validate()?;
        let n = points.len().min(4);
        let mut input_vertices = [0u32; 4];
        for (i, v) in input_vertices.iter_mut().enumerate().take(n) {
            *v = i as u32;
        }
        Ok(PenResult {
            mesh,
            input_vertices,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PenResult {
    pub mesh: Mesh,
    /// Input index -> new mesh point, never inferred from rotated face cycles.
    pub input_vertices: [u32; 4],
}
impl Editor {
    pub(crate) fn create_pen(&mut self, input: &PenInput) -> Result<()> {
        if self.session.has_active_task() || self.selection_start.is_some() {
            return Err("Drop or Cancel the active tool or selection before using Pen".into());
        }
        self.scene.validate()?;
        if self.scene.workplane.fixed
            || self.action_context != ActionContext::default()
            || self.spatial_falloff.is_some()
        {
            return Err("Pen needs the work plane off, the default action center, and no falloff, constraints or snapping".into());
        }
        if self.scene.selection.mode == Mode::Item {
            return Err("Pen works in Vertex, Edge or Polygon mode".into());
        }
        let target = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select one visible mesh for Pen")?;
        let mut result = input.geometry()?;
        // Pen points are world positions; a moved mesh takes them in its
        // local space, as the interactive Pen does.
        result.mesh = self.scene.mesh_world_to_local(target.id, result.mesh)?;
        // Symmetry's creation path (SYM3-20..25), as the interactive Pen.
        // Input points stay first, so `input_vertices` still holds.
        let (mesh, creation) = self.active_symmetry().welded(result.mesh)?;
        result.mesh = mesh;
        self.symmetry_note = creation.message().map(String::from);
        let vertex_start = target.mesh.vertices.len() as u32;
        let element_start = target.mesh.faces.len() as u32;
        let new_edges = result.mesh.edges();
        let elements = result.mesh.faces.len();
        let mut candidate = self.scene.clone();
        crate::append_geometry(
            &mut candidate.active_mut().ok_or("Pen target missing")?.mesh,
            result.mesh,
        )?;
        if input.select_new {
            candidate
                .selection
                .vertices
                .extend(result.input_vertices.map(|v| v + vertex_start));
            candidate.selection.edges.extend(
                new_edges
                    .into_iter()
                    .map(|(a, b)| (a + vertex_start, b + vertex_start)),
            );
            if candidate.selection.mode == Mode::Polygon {
                candidate
                    .selection
                    .faces
                    .extend(element_start..element_start + elements as u32);
            }
        }
        candidate.validate()?;
        let before = self.scene.clone();
        self.scene = candidate;
        self.record("Pen".into(), before);
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
}
