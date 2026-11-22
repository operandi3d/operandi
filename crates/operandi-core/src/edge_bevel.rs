//! ME-T01 Edge Bevel and ME-T07 Edge Chamfer.
//!
//! Both widen selected edges into strips while the
//! polygons on either side stay in their planes and shrink:
//! * **Edge Bevel** (Inset mode): each end of the edge moves the Inset
//!   distance along the side edges of the neighbouring polygons. Round Level
//!   adds rows between the two new edges.
//! * **Edge Chamfer**: the new edges are offset in parallel to the source edge
//!   (each end moves along its side edge far enough that the new edge is
//!   Offset away), which keeps the strip even. Segments rows round it.
//!
//! Rows follow a circular arc whose corner is the source edge (Depth 100%);
//! Depth 0% keeps them flat on the chord. The construction itself is in
//! `edge_bevel_junctions` (specifications/edge-bevel-chamfer-2026-11-02);
//! this module holds the options and the entry points. UV maps follow the
//! shared provisional rule in `corner_uv`; meshes with other per-corner data
//! are refused for now.
use crate::topology::edge_key;
use crate::{Mesh, Result};
use std::collections::BTreeSet;

/// How the width is measured (EBC-D-01/02, EBC-P-04, EBC-B3-01).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EdgeBevelMode {
    /// Each side moves the width along its polygon.
    #[default]
    Inset,
    /// The width is read across the corner the edge makes; the distance each
    /// side moves depends on the angle between the two polygons.
    Width,
}

/// Profile shape (EBC-D-01: Round, Square, Sharp; Edge Chamfer has Round and
/// Square).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EdgeBevelShape {
    #[default]
    Round,
    /// EBC-P-02 / EBC-B3-08: the profile goes through the original edge, two
    /// flat segments whatever the level.
    Square,
    /// Measured the same as Square at Levels 1–3 (EBC-B3-08).
    Sharp,
}

/// Edge Chamfer's Edge End (EBC-D-02, EBC-B3-07).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EdgeEnd {
    #[default]
    Profile,
    TriangleFan,
}

/// The options of Edge Bevel and Edge Chamfer besides width, rows and Depth.
/// Defaults are the values after Reset (EBC-D-01/02).
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeBevelOptions {
    pub mode: EdgeBevelMode,
    pub shape: EdgeBevelShape,
    /// Edge Chamfer: each side moves far enough that the new edge is Offset
    /// away from the source edge (on), or the Offset along the side edge
    /// (off, like Edge Bevel's Inset). Edge Bevel ignores it.
    pub parallel: bool,
    /// A side point that would pass the far end of its edge stops there
    /// (on), or carries on along the same line (off). Provisional reading
    /// (EBC-Q10).
    pub stop_at_edge: bool,
    /// New points that land on another point merge with it (provisional,
    /// EBC-Q10).
    pub auto_weld: bool,
    pub edge_end: EdgeEnd,
    /// The new polygons get `material` (provisional, EBC-Q05).
    pub use_material: bool,
    pub material: String,
}
impl Default for EdgeBevelOptions {
    fn default() -> Self {
        Self {
            mode: EdgeBevelMode::Inset,
            shape: EdgeBevelShape::Round,
            parallel: true,
            stop_at_edge: true,
            auto_weld: false,
            edge_end: EdgeEnd::Profile,
            use_material: false,
            material: "Default".into(),
        }
    }
}

/// What Edge Bevel / Edge Chamfer keep between uses: everything but the
/// width, which starts at zero each time (provisional, EBC-Q10).
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeBevelPreset {
    pub segments: u32,
    pub depth: f64,
    pub options: EdgeBevelOptions,
}
impl Default for EdgeBevelPreset {
    fn default() -> Self {
        Self {
            segments: 1,
            depth: 100.,
            options: Default::default(),
        }
    }
}

/// One Edge Bevel / Edge Chamfer application.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeBevelSettings {
    /// Inset (Edge Bevel), Offset (Edge Chamfer) or Width, in mesh units.
    pub width: f64,
    /// Profile segments: Segments, or 2 × Round Level (EBC-B3-02), at least 1.
    pub rows: u32,
    /// Depth in percent (EBC-P-03).
    pub depth: f64,
    /// Edge Chamfer rather than Edge Bevel.
    pub chamfer: bool,
    pub options: EdgeBevelOptions,
}

/// The beveled mesh and what each tool selects afterwards.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeBevelResult {
    pub mesh: Mesh,
    /// Every edge of the new strips.
    pub strip_edges: Vec<(u32, u32)>,
    /// What the tool selects after it applies: the strips' edges for Edge
    /// Bevel (EBC-J-01), nothing for Edge Chamfer, except the edges through
    /// the original corner with the Square shape (EBC-P-02).
    pub selected_after: Vec<(u32, u32)>,
}

/// Bevel (`parallel = false`) or chamfer (`parallel = true`) the selected
/// edges by `width` with `rows` rows (1 = flat strip) and `depth` (0..100 %).
/// Returns the mesh and the strip's edges.
pub fn edge_bevel_geometry(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    width: f64,
    rows: u32,
    depth: f64,
    parallel: bool,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    edge_bevel_geometry_weighted(mesh, edges, width, rows, depth, parallel, None)
}

/// Edge Bevel with a falloff: the width at each end of an edge is scaled by
/// that end point's weight (FAL-T-02).
pub fn edge_bevel_geometry_weighted(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    width: f64,
    rows: u32,
    depth: f64,
    parallel: bool,
    weights: Option<&[f64]>,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    let settings = EdgeBevelSettings {
        width,
        rows,
        depth,
        chamfer: parallel,
        options: Default::default(),
    };
    edge_bevel_with(mesh, edges, &settings, weights).map(|r| (r.mesh, r.strip_edges))
}

/// Edge Bevel / Edge Chamfer with every option.
pub fn edge_bevel_with(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    settings: &EdgeBevelSettings,
    weights: Option<&[f64]>,
) -> Result<EdgeBevelResult> {
    let what = if settings.chamfer {
        "Edge Chamfer"
    } else {
        "Edge Bevel"
    };
    mesh.validate()?;
    let EdgeBevelSettings {
        width, rows, depth, ..
    } = *settings;
    if !width.is_finite()
        || width < 0.
        || !(1..=64).contains(&rows)
        || !(0. ..=100.).contains(&depth)
    {
        return Err(format!(
            "{what} needs a nonnegative width, 1–64 rows and Depth 0–100%"
        ));
    }
    if settings.options.use_material && settings.options.material.trim().is_empty() {
        return Err(format!(
            "{what}: enter a material name or untick Use Material"
        ));
    }
    let selected: BTreeSet<(u32, u32)> = edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    if selected.is_empty() {
        return Err(format!("Select edges for {what}"));
    }
    if width == 0. {
        let edges: Vec<_> = selected.into_iter().collect();
        return Ok(EdgeBevelResult {
            mesh: mesh.clone(),
            selected_after: if settings.chamfer {
                Vec::new()
            } else {
                edges.clone()
            },
            strip_edges: edges,
        });
    }
    // Vertex normal maps are carried (`edge_bevel_junctions`): a mesh with
    // hard edges always has one, and refusing it left Edge Bevel doing
    // nothing on such meshes (forum report, 2026-11-22).
    if crate::corner_uv::has_uncarried_corner_data_besides_normals(mesh)
        || !mesh.attributes.polygon_sets.is_empty()
        || !mesh.attributes.extrude_edge_uses.is_empty()
    {
        return Err(format!(
            "{what} cannot yet remap corner weights, colors or polygon sets; nothing changed"
        ));
    }
    if weights.is_some_and(|w| w.len() != mesh.vertices.len()) {
        return Err(format!("{what}: falloff weights are out of date"));
    }
    crate::edge_bevel_junctions::general(mesh, &selected, settings, weights, what)
}
