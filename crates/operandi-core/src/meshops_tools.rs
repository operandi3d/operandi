//! Direct tools as MeshOps (MO-40/41; types, channels and defaults from
//! specifications meshops-specs-2026-11-17 TYPE-IDS.md). Tool families:
//! every op here runs the same engine as its direct tool, either its pure
//! kernel or, for tools that only exist as editor commands, the command
//! itself on a throwaway editor holding the op's input ([`via_editor`]).
//!
//! Polygon group: Polygon Extrude, Polygon Merge, Polygon Flip, Symmetrize,
//! Detriangulate, UnSubdivide, Polygon Unify, Material Tag, Spike, Delete.
//!
//! Provisional (MOQ-25..): a choice channel the reference stores as text is
//! stored by position; Delete works on the selected polygons (Sel POLY,
//! not VRTX, until selection ops pick points, MOQ-21).
use crate::meshop_types::{Field, FieldKind, OpContext, OpDef, Typed, TypedOp};
use crate::meshops_builtin::finite;
use crate::{Command, Editor, Mesh, Mode, Result};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Runs `command` on a throwaway editor whose one mesh is the op's input,
/// with the op's polygons selected in Polygon mode (none = the tool's own
/// "nothing selected" rule, usually every polygon), and returns the mesh.
pub(crate) fn via_editor(ctx: &OpContext, command: Command) -> Result<Mesh> {
    let mut e = Editor::default();
    e.scene.new_mesh()?;
    let id = e.scene.items.last().map(|i| i.id).ok_or("No mesh")?;
    e.scene.item_mut(id).ok_or("No mesh")?.mesh = ctx.input.clone().into();
    e.scene.select_item(Some(id));
    e.dispatch(Command::Mode(Mode::Polygon))?;
    if let Some(faces) = ctx.selection {
        e.scene.selection.faces = faces.to_vec();
    }
    e.dispatch(command)?;
    Ok((*e.scene.item(id).ok_or("No mesh")?.mesh).clone())
}

fn choice(key: &'static str, label: &'static str, names: &[&'static str]) -> Field {
    Field::new(
        key,
        label,
        FieldKind::Choice(names.iter().map(|n| (*n).into()).collect()),
    )
}
fn check(key: &'static str, label: &'static str) -> Field {
    Field::new(key, label, FieldKind::Check)
}
fn distance(key: &'static str, label: &'static str) -> Field {
    Field::new(key, label, FieldKind::Distance)
}

// ---- Polygon Extrude ----

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExtrudeOp {
    pub shift_x: f64,
    pub shift_y: f64,
    pub shift_z: f64,
    /// Sides (segments along the extrusion).
    pub segs: u32,
    /// Make UVs: 0 None, 1 U, 2 V.
    pub uvs: u32,
    /// 0 Auto, 1 Thick, 2 Shift.
    pub mode: u32,
}
impl Default for ExtrudeOp {
    fn default() -> Self {
        Self {
            shift_x: 0.,
            shift_y: 0.,
            shift_z: 0.,
            segs: 1,
            uvs: 1,
            mode: 0,
        }
    }
}
impl TypedOp for ExtrudeOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let extent = DVec3::new(self.shift_x, self.shift_y, self.shift_z);
        finite(&extent.to_array(), "Extent")?;
        if extent == DVec3::ZERO {
            return Ok(ctx.input.clone());
        }
        let uvs = match self.uvs {
            0 => crate::ExtrudeUvs::None,
            1 => crate::ExtrudeUvs::U,
            2 => crate::ExtrudeUvs::V,
            _ => return Err("Make UVs must be 0 (None), 1 (U) or 2 (V)".into()),
        };
        let mode = match self.mode {
            0 => crate::ExtrudeMode::Auto,
            1 => crate::ExtrudeMode::Thick,
            2 => crate::ExtrudeMode::Shift,
            _ => return Err("Mode must be 0 (Auto), 1 (Thick) or 2 (Shift)".into()),
        };
        let r = crate::extrude_geometry(
            ctx.input,
            &ctx.chosen_or_all()?,
            extent,
            self.segs.max(1),
            mode,
            uvs,
            None,
        )?;
        let mut mesh = r.mesh;
        let n = ctx.input.faces.len() as u32;
        let front = r.moved_caps.clone();
        let sides: Vec<u32> = (n..mesh.faces.len() as u32)
            .filter(|f| !front.contains(f))
            .collect();
        ctx.tag(&mut mesh, "front", front);
        ctx.tag(&mut mesh, "sides", sides);
        Ok(mesh)
    }
}

// ---- Polygon Merge, Flip, Unify, Material Tag (editor commands) ----

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MergePolygonsOp {}
impl TypedOp for MergePolygonsOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let r = crate::polygon_merge::merge_faces(ctx.input, &ctx.chosen_or_all()?)?;
        Ok(r.mesh)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlipOp {}
impl TypedOp for FlipOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        via_editor(ctx, Command::FlipPolygons)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnifyOp {
    pub force: bool,
}
impl TypedOp for UnifyOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let mut mesh = via_editor(
            ctx,
            Command::UnifyPolygons(crate::UnifyRequest {
                force: self.force,
                expected: None,
            }),
        )?;
        let all = 0..mesh.faces.len() as u32;
        ctx.tag(&mut mesh, "unified", all);
        Ok(mesh)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MaterialTagOp {
    pub material_name: String,
}
impl Default for MaterialTagOp {
    fn default() -> Self {
        Self {
            material_name: "Default".into(),
        }
    }
}
impl TypedOp for MaterialTagOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        if self.material_name.trim().is_empty() {
            return Err("Material Tag needs a Material Name".into());
        }
        // The tag only, on the chosen polygons (or every polygon).
        let mut mesh = ctx.input.clone();
        for f in ctx.chosen_or_all()? {
            mesh.faces[f as usize].material = self.material_name.clone();
        }
        Ok(mesh)
    }
}

// ---- Symmetrize ----

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SymmetrizeOp {
    /// 0 X, 1 Y, 2 Z.
    pub axis: u32,
    /// 0 Positive to Negative, 1 Negative to Positive.
    pub direction: u32,
    pub merge: bool,
    pub offset: f64,
    pub topology: bool,
}
impl Default for SymmetrizeOp {
    fn default() -> Self {
        Self {
            axis: 0,
            direction: 0,
            merge: true,
            offset: 0.,
            topology: false,
        }
    }
}
impl TypedOp for SymmetrizeOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        if self.axis > 2 {
            return Err("Symmetrize Axis must be 0 (X), 1 (Y) or 2 (Z)".into());
        }
        finite(&[self.offset], "Offset")?;
        let side = match self.direction {
            0 => crate::SymmetrizeSide::Positive,
            1 => crate::SymmetrizeSide::Negative,
            _ => return Err("Symmetrize Direction must be 0 or 1".into()),
        };
        let mut e = Editor::default();
        e.scene.new_mesh()?;
        let id = e.scene.items.last().map(|i| i.id).ok_or("No mesh")?;
        e.scene.item_mut(id).ok_or("No mesh")?.mesh = ctx.input.clone().into();
        e.scene.select_item(Some(id));
        e.dispatch(Command::Symmetry(crate::Symmetry {
            axis: Some(self.axis as usize),
            offset: self.offset,
            topology: self.topology,
            ..Default::default()
        }))?;
        e.dispatch(Command::SymmetrizePolygons {
            side,
            merge: self.merge,
        })?;
        Ok((*e.scene.item(id).ok_or("No mesh")?.mesh).clone())
    }
}

// ---- Detriangulate, UnSubdivide ----

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DetriangulateOp {
    /// Flatness, fraction (0.2 = 20%).
    pub flatness: f64,
    /// Radians.
    pub max_angle: f64,
    pub material: bool,
    pub part: bool,
    pub smoothing_group: bool,
    pub selection_set: bool,
    pub texture: bool,
}
impl Default for DetriangulateOp {
    fn default() -> Self {
        let o = crate::DetriangulateOptions::default();
        Self {
            flatness: o.flatness / 100.,
            max_angle: o.max_angle.to_radians(),
            material: o.keep_material,
            part: o.keep_part,
            smoothing_group: o.keep_smoothing,
            selection_set: o.keep_sets,
            texture: o.keep_uv_seams,
        }
    }
}
impl TypedOp for DetriangulateOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.flatness, self.max_angle], "Flatness and Max Angle")?;
        let o = crate::DetriangulateOptions {
            flatness: self.flatness * 100.,
            max_angle: self.max_angle.to_degrees(),
            keep_material: self.material,
            keep_part: self.part,
            keep_smoothing: self.smoothing_group,
            keep_sets: self.selection_set,
            keep_uv_seams: self.texture,
        };
        o.validate()?;
        Ok(crate::detriangulate::detriangulate_geometry(ctx.input, &ctx.chosen()?, &o)?.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnsubdivideOp {
    pub iteration: u32,
    pub flat: bool,
    pub uv: bool,
    pub material: bool,
    pub part: bool,
    pub smoothing_group: bool,
    pub selection_set: bool,
    pub selection_set_name: String,
}
impl Default for UnsubdivideOp {
    fn default() -> Self {
        // TYPE-IDS: Iteration 0 (the direct tool's provisional default is 1).
        let o = crate::UnsubdivideOptions::default();
        Self {
            iteration: 0,
            flat: o.flat,
            uv: o.uv_hint,
            material: o.material_hint,
            part: o.part_hint,
            smoothing_group: o.smoothing_group_hint,
            selection_set: o.selection_set_hint,
            selection_set_name: o.selection_set_name,
        }
    }
}
impl TypedOp for UnsubdivideOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        if self.iteration == 0 {
            return Ok(ctx.input.clone());
        }
        let o = crate::UnsubdivideOptions {
            iterations: self.iteration,
            flat: self.flat,
            uv_hint: self.uv,
            material_hint: self.material,
            part_hint: self.part,
            smoothing_group_hint: self.smoothing_group,
            selection_set_hint: self.selection_set,
            selection_set_name: self.selection_set_name.clone(),
        };
        Ok(crate::polygon_subdivide::unsubdivide_polygons(ctx.input, &ctx.chosen()?, &o)?.0)
    }
}

// ---- Spike, Delete ----

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpikeOp {
    /// Spike Strength.
    pub factor: f64,
}
impl TypedOp for SpikeOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.factor], "Spike Strength")?;
        // PG's Center Point is a point group; it waits for point
        // selection ops (MOQ-21).
        Ok(crate::spike_geometry(ctx.input, &ctx.chosen_or_all()?, self.factor)?.mesh)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteOp {
    /// Keep Vertices: leave the points of deleted polygons.
    pub keep: bool,
}
impl TypedOp for DeleteOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        // PG-05: with no Selection input every polygon goes.
        let faces = ctx.chosen_or_all()?;
        if self.keep {
            Ok(crate::polygon_remove::remove_faces(ctx.input, &faces)?.mesh)
        } else {
            crate::polygon_remove::delete_geometry(ctx.input, &faces, &[])
        }
    }
}

pub(crate) fn polygon_entries() -> Vec<Arc<dyn OpDef>> {
    const FRONT_SIDES: &[(&str, &str)] = &[("front", "Front"), ("sides", "Sides")];
    vec![
        Arc::new(
            Typed::<ExtrudeOp>::new(
                "poly.extrude.item",
                "Polygon Extrude",
                "Polygon",
                true,
                vec![
                    distance("shiftX", "Extent X"),
                    distance("shiftY", "Extent Y"),
                    distance("shiftZ", "Extent Z"),
                    Field::new("segs", "Sides", FieldKind::Count { min: 1, max: 1000 }),
                    choice("uvs", "Make UVs", &["None", "U", "V"]),
                    choice("mode", "Mode", &["Auto", "Thick", "Shift"]),
                ],
            )
            .with_groups(FRONT_SIDES),
        ),
        Arc::new(Typed::<MergePolygonsOp>::new(
            "pmodel.mergePolygons.item",
            "Polygon Merge",
            "Polygon",
            true,
            Vec::new(),
        )),
        Arc::new(Typed::<FlipOp>::new(
            "pmodel.invertPolys.item",
            "Flip Polygons",
            "Polygon",
            true,
            Vec::new(),
        )),
        Arc::new(Typed::<SymmetrizeOp>::new(
            "symmetrize.meshop.item",
            "Symmetrize",
            "Polygon",
            false,
            vec![
                choice("axis", "Axis", &["X", "Y", "Z"]),
                choice(
                    "direction",
                    "Direction",
                    &["Positive to Negative", "Negative to Positive"],
                ),
                check("merge", "Merge"),
                distance("offset", "Offset"),
                check("topology", "Topology"),
            ],
        )),
        Arc::new(Typed::<DetriangulateOp>::new(
            "detriangulate.meshop.item",
            "Detriangulate",
            "Polygon",
            true,
            vec![
                Field::new("flatness", "Flatness", FieldKind::Percent),
                Field::new("maxAngle", "Max Angle", FieldKind::Angle),
                check("material", "Keep Material Boundary"),
                check("part", "Keep Part Boundary"),
                check("smoothingGroup", "Keep Smoothing Group Boundary"),
                check("selectionSet", "Keep Selection Set Boundary"),
                check("texture", "Keep UV Texture Boundary"),
            ],
        )),
        Arc::new(Typed::<UnsubdivideOp>::new(
            "poly.unsubdiv.item",
            "UnSubdivide",
            "Polygon",
            true,
            vec![
                Field::new(
                    "iteration",
                    "Iteration",
                    FieldKind::Count { min: 0, max: 16 },
                ),
                check("flat", "Flat"),
                check("uv", "UV Corner Hint"),
                check("material", "Material Corner Hint"),
                check("part", "Part Corner Hint"),
                check("smoothingGroup", "Smoothing Group Corner Hint"),
                check("selectionSet", "Selection Set Corner Hint"),
                Field::new("selectionSetName", "Selection Set Name", FieldKind::Text),
            ],
        )),
        Arc::new(
            Typed::<UnifyOp>::new(
                "pmodel.unifyPolygons.item",
                "Polygon Unify",
                "Deform",
                true,
                vec![check("force", "Force")],
            )
            .with_groups(&[("unified", "Unified Polygons")]),
        ),
        Arc::new(Typed::<MaterialTagOp>::new(
            "pmodel.materialTag.item",
            "Material Tag",
            "Polygon",
            true,
            vec![Field::new("materialName", "Material Name", FieldKind::Text)],
        )),
        Arc::new(Typed::<SpikeOp>::new(
            "poly.spikey.item",
            "Spike",
            "Polygon",
            true,
            vec![Field::new("factor", "Spike Strength", FieldKind::Number)],
        )),
        Arc::new(Typed::<DeleteOp>::new(
            "delete.meshop.item",
            "Delete",
            "Edit",
            true,
            vec![check("keep", "Keep Vertices")],
        )),
    ]
}

// ======== Edge group ========
//
// Edge ops' Selection input works on edges (Sel EDGE). Until selection ops
// pick edges (MOQ-21), an op works on every edge, or on the edges of the
// polygons its Selection input picked (MOQ-25, provisional).

/// The edges an edge op works on.
fn chosen_edges(ctx: &OpContext) -> Vec<(u32, u32)> {
    let Some(faces) = ctx.selection else {
        return ctx.input.edges();
    };
    let mut out = std::collections::BTreeSet::new();
    for &f in faces {
        let v = &ctx.input.faces[f as usize].vertices;
        for i in 0..v.len() {
            let (a, b) = (v[i], v[(i + 1) % v.len()]);
            out.insert((a.min(b), a.max(b)));
        }
    }
    out.into_iter().collect()
}
/// Edge ops' New Polygon group (PG table): the polygons past the input's.
fn new_polygons(ctx: &OpContext, mut mesh: Mesh) -> Mesh {
    let n = ctx.input.faces.len() as u32;
    let end = mesh.faces.len() as u32;
    ctx.tag(&mut mesh, "new", n..end);
    mesh
}
const NEW_POLYGON: &[(&str, &str)] = &[("new", "New Polygon")];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EdgeBevelOp {
    /// 0 Inset, 1 Width.
    pub mode: u32,
    pub inset: f64,
    pub use_mat: bool,
    pub mat_name: String,
    /// Round Level: the profile has 2 × Level segments.
    pub level: u32,
    pub sharp: bool,
    /// 0 Round, 1 Square, 2 Sharp.
    pub shape: u32,
    pub mitering: f64,
    pub coplanar: bool,
    /// Fraction (1 = 100%).
    pub depth: f64,
    pub bound_extend: bool,
}
impl Default for EdgeBevelOp {
    fn default() -> Self {
        Self {
            mode: 0,
            inset: 0.,
            use_mat: false,
            mat_name: String::new(),
            level: 0,
            sharp: false,
            shape: 0,
            mitering: 0.,
            coplanar: false,
            depth: 1.,
            bound_extend: true,
        }
    }
}
fn bevel_mode(mode: u32) -> Result<crate::EdgeBevelMode> {
    match mode {
        0 => Ok(crate::EdgeBevelMode::Inset),
        1 => Ok(crate::EdgeBevelMode::Width),
        _ => Err("Mode must be 0 (Inset) or 1 (Width)".into()),
    }
}
fn bevel_shape(shape: u32) -> Result<crate::EdgeBevelShape> {
    match shape {
        0 => Ok(crate::EdgeBevelShape::Round),
        1 => Ok(crate::EdgeBevelShape::Square),
        2 => Ok(crate::EdgeBevelShape::Sharp),
        _ => Err("Edge Shape must be 0 (Round), 1 (Square) or 2 (Sharp)".into()),
    }
}
impl TypedOp for EdgeBevelOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.inset, self.depth], "Value and Depth")?;
        if self.inset == 0. {
            return Ok(ctx.input.clone());
        }
        let settings = crate::EdgeBevelSettings {
            width: self.inset,
            rows: (2 * self.level).max(1),
            depth: self.depth * 100.,
            chamfer: false,
            options: crate::EdgeBevelOptions {
                mode: bevel_mode(self.mode)?,
                shape: bevel_shape(self.shape)?,
                use_material: self.use_mat,
                material: self.mat_name.clone(),
                ..Default::default()
            },
        };
        let r = crate::edge_bevel::edge_bevel_with(ctx.input, &chosen_edges(ctx), &settings, None)?;
        Ok(new_polygons(ctx, r.mesh))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EdgeChamferOp {
    pub offset: f64,
    /// 0 Inset, 1 Width.
    pub mode: u32,
    pub segments: u32,
    /// 0 Round, 1 Square.
    pub shape: u32,
    pub stop_at_edge: bool,
    pub auto_weld: bool,
    pub use_mat: bool,
    pub mat_name: String,
    pub depth: f64,
    pub parallel: bool,
    pub coplanar: bool,
    /// 0 Profile, 1 Triangle Fan.
    pub edge_end: u32,
    pub flatness: f64,
    pub reversex: bool,
    pub reversey: bool,
    pub bound_extend: bool,
    pub mitering: f64,
    pub sharp_corner: bool,
    #[serde(rename = "bCoplanar")]
    pub b_coplanar: bool,
}
impl Default for EdgeChamferOp {
    fn default() -> Self {
        Self {
            offset: 0.,
            mode: 0,
            segments: 1,
            shape: 0,
            stop_at_edge: true,
            auto_weld: false,
            use_mat: false,
            mat_name: "Default".into(),
            depth: 1.,
            parallel: true,
            coplanar: false,
            edge_end: 0,
            flatness: 1e-5,
            reversex: false,
            reversey: false,
            bound_extend: true,
            mitering: 0.,
            sharp_corner: false,
            b_coplanar: true,
        }
    }
}
impl TypedOp for EdgeChamferOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.offset, self.depth], "Offset and Depth")?;
        if self.offset == 0. {
            return Ok(ctx.input.clone());
        }
        let settings = crate::EdgeBevelSettings {
            width: self.offset,
            rows: self.segments.max(1),
            depth: self.depth * 100.,
            chamfer: true,
            options: crate::EdgeBevelOptions {
                mode: bevel_mode(self.mode)?,
                shape: bevel_shape(self.shape.min(1))?,
                parallel: self.parallel,
                stop_at_edge: self.stop_at_edge,
                auto_weld: self.auto_weld,
                edge_end: if self.edge_end == 1 {
                    crate::edge_bevel::EdgeEnd::TriangleFan
                } else {
                    crate::edge_bevel::EdgeEnd::Profile
                },
                use_material: self.use_mat,
                material: self.mat_name.clone(),
            },
        };
        let r = crate::edge_bevel::edge_bevel_with(ctx.input, &chosen_edges(ctx), &settings, None)?;
        Ok(new_polygons(ctx, r.mesh))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EdgeExtrudeOp {
    /// Width.
    pub inset: f64,
    /// Extrude.
    pub shift: f64,
    pub use_mat: bool,
    pub mat_name: String,
}
impl Default for EdgeExtrudeOp {
    fn default() -> Self {
        Self {
            inset: 0.,
            shift: 0.,
            use_mat: false,
            mat_name: String::new(),
        }
    }
}
impl TypedOp for EdgeExtrudeOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.inset, self.shift], "Width and Extrude")?;
        if self.inset == 0. && self.shift == 0. {
            return Ok(ctx.input.clone());
        }
        let material = self.use_mat.then_some(self.mat_name.as_str());
        let (mesh, _) = crate::edge_extrude::edge_extrude_with(
            ctx.input,
            &chosen_edges(ctx),
            self.shift,
            self.inset,
            material,
        )?;
        Ok(new_polygons(ctx, mesh))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EdgeRelaxOp {
    pub offset: f64,
    pub tension: f64,
    pub propagate: u32,
    pub convergence: bool,
    pub propscale: f64,
    pub propside: u32,
    pub shape: u32,
    pub p0: f64,
    pub p1: f64,
    pub depth: f64,
    pub add_point: u32,
    pub add_mode: u32,
    pub arc_approx: bool,
}
impl Default for EdgeRelaxOp {
    fn default() -> Self {
        Self {
            offset: 0.,
            tension: 1.,
            propagate: 0,
            convergence: false,
            propscale: 1.,
            propside: 0,
            shape: 0,
            p0: 0.,
            p1: 0.,
            depth: 1.,
            add_point: 0,
            add_mode: 0,
            arc_approx: false,
        }
    }
}
impl TypedOp for EdgeRelaxOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.offset], "Offset")?;
        if self.offset == 0. {
            return Ok(ctx.input.clone());
        }
        crate::edge_relax::edge_relax_geometry(ctx.input, &chosen_edges(ctx), self.offset)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeSplitOp {
    pub caps: bool,
    pub gap: f64,
}
impl TypedOp for EdgeSplitOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        Ok(crate::edge_commands::edge_command_geometry(
            ctx.input,
            &chosen_edges(ctx),
            crate::EdgeCommand::Split,
        )?
        .0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeSubdivideOp {
    /// With Segments 1 or 2 places the one new point this far along.
    pub percent: f64,
    pub count: u32,
}
impl Default for EdgeSubdivideOp {
    fn default() -> Self {
        Self {
            percent: 0.5,
            count: 2,
        }
    }
}
impl TypedOp for EdgeSubdivideOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.percent], "Percent")?;
        let percent = (self.count <= 1).then_some(self.percent * 100.);
        Ok(crate::edge_commands::edge_subdivide_geometry(
            ctx.input,
            &chosen_edges(ctx),
            self.count.max(1),
            percent,
        )?
        .0)
    }
}

fn hidden(key: &'static str) -> Field {
    Field::new(key, key, FieldKind::Hidden)
}

pub(crate) fn edge_entries() -> Vec<Arc<dyn OpDef>> {
    let material = || {
        [
            check("useMat", "Use Material"),
            Field::new("matName", "Material Name", FieldKind::Text),
        ]
    };
    let mut bevel = vec![
        choice("mode", "Mode", &["Inset", "Width"]),
        distance("inset", "Value"),
        Field::new("level", "Round Level", FieldKind::Count { min: 0, max: 64 }),
        choice("shape", "Edge Shape", &["Round", "Square", "Sharp"]),
        Field::new("depth", "Depth", FieldKind::Percent),
    ];
    bevel.extend(material());
    // Saved and kept, not built yet (MOQ-26).
    bevel.extend(["sharp", "mitering", "coplanar", "boundExtend"].map(hidden));
    let mut chamfer = vec![
        distance("offset", "Offset"),
        choice("mode", "Mode", &["Inset", "Width"]),
        Field::new(
            "segments",
            "Segments",
            FieldKind::Count { min: 1, max: 128 },
        ),
        choice("shape", "Shape", &["Round", "Square"]),
        check("stopAtEdge", "Stop at Edge"),
        check("autoWeld", "Auto Weld"),
        Field::new("depth", "Depth", FieldKind::Percent),
        check("parallel", "Offset in Parallel"),
        choice("edgeEnd", "Edge End", &["Profile", "Triangle Fan"]),
    ];
    chamfer.extend(material());
    chamfer.extend(
        [
            "coplanar",
            "flatness",
            "reversex",
            "reversey",
            "boundExtend",
            "mitering",
            "sharpCorner",
            "bCoplanar",
        ]
        .map(hidden),
    );
    let mut extrude = vec![distance("inset", "Width"), distance("shift", "Extrude")];
    extrude.extend(material());
    let mut relax = vec![distance("offset", "Offset")];
    relax.extend(
        [
            "tension",
            "propagate",
            "convergence",
            "propscale",
            "propside",
            "shape",
            "p0",
            "p1",
            "depth",
            "addPoint",
            "addMode",
            "arcApprox",
        ]
        .map(hidden),
    );
    vec![
        Arc::new(
            Typed::<EdgeBevelOp>::new("edge.bevel.item", "Edge Bevel", "Edge", true, bevel)
                .with_groups(NEW_POLYGON),
        ),
        Arc::new(
            Typed::<EdgeChamferOp>::new("edge.chamfer.item", "Edge Chamfer", "Edge", true, chamfer)
                .with_groups(NEW_POLYGON),
        ),
        Arc::new(Typed::<EdgeSplitOp>::new(
            "pmodel.splitEdges.item",
            "Edge Split",
            "Edge",
            true,
            vec![hidden("caps"), hidden("gap")],
        )),
        Arc::new(
            Typed::<EdgeExtrudeOp>::new("edge.extrude.item", "Edge Extrude", "Edge", true, extrude)
                .with_groups(NEW_POLYGON),
        ),
        Arc::new(Typed::<EdgeRelaxOp>::new(
            "edge.relax.item",
            "Edge Relax",
            "Edge",
            true,
            relax,
        )),
        Arc::new(Typed::<EdgeSubdivideOp>::new(
            "pmodel.edge.subdivide",
            "Edge Subdivide",
            "Edge",
            true,
            vec![
                Field::new("percent", "Percent", FieldKind::Percent),
                Field::new("count", "Segments", FieldKind::Count { min: 1, max: 256 }),
            ],
        )),
    ]
}

// ======== Vertex group ========
//
// Vertex ops work on points (Sel VRTX): every used point, or the points of
// the polygons the Selection input picked (MOQ-25, provisional).

/// The points a vertex op works on.
fn chosen_points(ctx: &OpContext) -> Vec<u32> {
    let mut out = std::collections::BTreeSet::new();
    match ctx.selection {
        Some(faces) => {
            for &f in faces {
                out.extend(ctx.input.faces[f as usize].vertices.iter().copied());
            }
        }
        None => {
            for f in ctx.input.faces.iter() {
                out.extend(f.vertices.iter().copied());
            }
        }
    }
    out.into_iter().collect()
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VertexBevelOp {
    pub inset: f64,
    pub use_mat: bool,
    pub mat_name: String,
    pub level: u32,
}
impl TypedOp for VertexBevelOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.inset], "Value")?;
        if self.inset == 0. {
            return Ok(ctx.input.clone());
        }
        let (mesh, _) = crate::vertex_commands::vertex_bevel_round(
            ctx.input,
            &chosen_points(ctx),
            self.inset,
            self.level,
            None,
        )?;
        Ok(mesh)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VertexExtrudeOp {
    /// Width.
    pub inset: f64,
    /// Extrude.
    pub shift: f64,
    pub use_mat: bool,
    pub mat_name: String,
}
impl TypedOp for VertexExtrudeOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.inset, self.shift], "Width and Extrude")?;
        if self.inset == 0. && self.shift == 0. {
            return Ok(ctx.input.clone());
        }
        let material = self.use_mat.then_some(self.mat_name.as_str());
        let (mesh, _) = crate::vertex_commands::vertex_extrude_with(
            ctx.input,
            &chosen_points(ctx),
            self.shift,
            self.inset,
            material,
        )?;
        Ok(new_polygons(ctx, mesh))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateVertexOp {
    #[serde(rename = "position.X")]
    pub x: f64,
    #[serde(rename = "position.Y")]
    pub y: f64,
    #[serde(rename = "position.Z")]
    pub z: f64,
}
impl TypedOp for CreateVertexOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.x, self.y, self.z], "Position")?;
        let mut mesh = ctx.input.clone();
        mesh.vertices.push(DVec3::new(self.x, self.y, self.z));
        Ok(mesh)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VertexMergeOp {
    pub dist: f64,
    pub keep: bool,
    pub morph: bool,
    pub allow_self_merge: bool,
    pub merge_to_closest: bool,
}
impl Default for VertexMergeOp {
    fn default() -> Self {
        Self {
            dist: 0.001,
            keep: false,
            morph: false,
            allow_self_merge: true,
            merge_to_closest: false,
        }
    }
}
impl TypedOp for VertexMergeOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.dist], "Distance")?;
        if self.dist < 0. {
            return Err("Distance can't be negative".into());
        }
        let command = crate::VertexCommand::Merge {
            distance: self.dist,
            options: crate::vertex_commands::MergeOptions {
                keep_small: self.keep,
                test_morphs: self.morph,
                ..Default::default()
            },
        };
        let points = match ctx.selection {
            Some(_) => chosen_points(ctx),
            // Nothing selected: Merge works on every point (CMD-V-01).
            None => Vec::new(),
        };
        Ok(crate::vertex_commands::vertex_command_geometry(ctx.input, &points, command)?.0)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VertexSplitOp {}
impl TypedOp for VertexSplitOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        Ok(crate::vertex_commands::vertex_command_geometry(
            ctx.input,
            &chosen_points(ctx),
            crate::VertexCommand::Split,
        )?
        .0)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SetPositionOp {
    pub enable_x: bool,
    pub enable_y: bool,
    pub enable_z: bool,
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
}
impl TypedOp for SetPositionOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let p = [self.position_x, self.position_y, self.position_z];
        finite(&p, "Position")?;
        let on = [self.enable_x, self.enable_y, self.enable_z];
        let mut mesh = ctx.input.clone();
        for v in chosen_points(ctx) {
            let q = &mut mesh.vertices[v as usize];
            for axis in 0..3 {
                if on[axis] {
                    q[axis] = p[axis];
                }
            }
        }
        Ok(mesh)
    }
}

pub(crate) fn vertex_entries() -> Vec<Arc<dyn OpDef>> {
    let material = || {
        [
            check("useMat", "Use Material"),
            Field::new("matName", "Material Name", FieldKind::Text),
        ]
    };
    let mut extrude = vec![distance("inset", "Width"), distance("shift", "Extrude")];
    extrude.extend(material());
    let set = vec![
        check("enableX", "Enable X"),
        distance("positionX", "Position X"),
        check("enableY", "Enable Y"),
        distance("positionY", "Position Y"),
        check("enableZ", "Enable Z"),
        distance("positionZ", "Position Z"),
    ];
    vec![
        Arc::new(Typed::<VertexBevelOp>::new(
            "vert.bevel.item",
            "Vertex Bevel",
            "Vertex",
            true,
            vec![
                distance("inset", "Value"),
                Field::new("level", "Round Level", FieldKind::Count { min: 0, max: 64 }),
                // Vertex Bevel's kernel has no Use Material yet (MOQ-26).
                hidden("useMat"),
                hidden("matName"),
            ],
        )),
        Arc::new(
            Typed::<VertexExtrudeOp>::new(
                "vert.extrude.item",
                "Vertex Extrude",
                "Vertex",
                true,
                extrude,
            )
            .with_groups(NEW_POLYGON),
        ),
        Arc::new(Typed::<CreateVertexOp>::new(
            "pmodel.createVertex",
            "Create Vertex",
            "Create",
            false,
            vec![
                distance("position.X", "Position X"),
                distance("position.Y", "Position Y"),
                distance("position.Z", "Position Z"),
            ],
        )),
        Arc::new(Typed::<VertexMergeOp>::new(
            "vert.merge.item",
            "Vertex Merge",
            "Vertex",
            true,
            vec![
                distance("dist", "Distance"),
                check("keep", "Keep 1-Vertex Polygons"),
                check("morph", "Test Morph Map Positions"),
                hidden("allowSelfMerge"),
                hidden("mergeToClosest"),
            ],
        )),
        Arc::new(Typed::<VertexSplitOp>::new(
            "vert.split.meshop.item",
            "Vertex Split",
            "Vertex",
            true,
            Vec::new(),
        )),
        Arc::new(Typed::<SetPositionOp>::new(
            "vert.setpos.meshop.item",
            "Vertex Set Position",
            "Vertex",
            true,
            set,
        )),
    ]
}

// ======== Duplicate group ========
//
// Radial Array and Clone, saved like Mirror and Array as a Clone Effector
// (`effector.clone.item`: replace, flip, merge, dist) with a generator in
// its Tool Pipe (`pmodel.helix`, `pmodel.linear`); meshops.rs turns one
// form into the other. Their copies are `clone`, `clone1`.. (PG table).

/// The Clone Effector's copy options.
fn copies(replace: bool, flip: bool, merge: bool, dist: f64) -> Result<crate::duplicate::Copies> {
    finite(&[dist], "Distance")?;
    if dist < 0. {
        return Err("Distance can't be negative".into());
    }
    Ok(crate::duplicate::Copies {
        replace_source: replace,
        flip,
        merge,
        distance: dist,
        item: Default::default(),
    })
}
/// The copies of the chosen polygons by `transforms`, with clone groups.
fn copied(
    ctx: &OpContext,
    transforms: &[glam::DMat4],
    c: &crate::duplicate::Copies,
) -> Result<Mesh> {
    let source: Vec<usize> = ctx
        .chosen_or_all()?
        .into_iter()
        .map(|f| f as usize)
        .collect();
    let r = crate::duplicate::copy_faces(
        ctx.input,
        &source,
        transforms,
        c,
        crate::duplicate::WeldAt::Original,
        c.distance,
    )?;
    let mut mesh = r.mesh;
    let end = mesh.faces.len() as u32;
    if let Some(start) = end.checked_sub(r.new_faces as u32) {
        ctx.tag(&mut mesh, "clone", start..end);
        let per = source.len() as u32;
        if per > 0 && (r.new_faces as u32).is_multiple_of(per) {
            for k in 0..r.new_faces as u32 / per {
                let a = start + k * per;
                ctx.tag(&mut mesh, &format!("clone{}", k + 1), a..a + per);
            }
        }
    }
    Ok(mesh)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RadialArrayOp {
    /// 0 X, 1 Y, 2 Z.
    pub axis: u32,
    /// Count: pieces, the original included.
    pub sides: u32,
    pub vec_x: f64,
    pub vec_y: f64,
    pub vec_z: f64,
    /// Radians.
    pub start: f64,
    pub end: f64,
    pub offset: f64,
    pub cen_x: f64,
    pub cen_y: f64,
    pub cen_z: f64,
    pub square: bool,
    pub replace: bool,
    pub flip: bool,
    pub merge: bool,
    pub dist: f64,
}
impl Default for RadialArrayOp {
    fn default() -> Self {
        Self {
            axis: 1,
            sides: 8,
            vec_x: 0.,
            vec_y: 1.,
            vec_z: 0.,
            start: 0.,
            end: std::f64::consts::TAU,
            offset: 0.,
            cen_x: 0.,
            cen_y: 0.,
            cen_z: 0.,
            square: false,
            replace: false,
            flip: false,
            merge: false,
            dist: 0.,
        }
    }
}
impl TypedOp for RadialArrayOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        if self.axis > 2 {
            return Err("Axis must be 0 (X), 1 (Y) or 2 (Z)".into());
        }
        let r = crate::duplicate::RadialArray {
            circle: crate::duplicate::Circle {
                count: self.sides,
                axis: self.axis as usize,
                center: DVec3::new(self.cen_x, self.cen_y, self.cen_z),
                direction: DVec3::new(self.vec_x, self.vec_y, self.vec_z),
                start: self.start.to_degrees(),
                end: self.end.to_degrees(),
                offset: self.offset,
            },
            copies: copies(self.replace, self.flip, self.merge, self.dist)?,
        };
        let transforms = crate::duplicate::radial_transforms(&r)?;
        copied(ctx, &transforms, &r.copies)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LinearCloneOp {
    /// Copies, not counting the original.
    pub num: u32,
    pub off_x: f64,
    pub off_y: f64,
    pub off_z: f64,
    /// Radians per step.
    pub ang_p: f64,
    pub ang_h: f64,
    pub ang_b: f64,
    /// Fraction per step (1 = 100%).
    pub scl_x: f64,
    pub scl_y: f64,
    pub scl_z: f64,
    pub between: bool,
    pub independent: bool,
    pub replace: bool,
    pub flip: bool,
    pub merge: bool,
    pub dist: f64,
}
impl Default for LinearCloneOp {
    fn default() -> Self {
        Self {
            num: 1,
            off_x: 0.,
            off_y: 1.,
            off_z: 0.,
            ang_p: 0.,
            ang_h: 0.,
            ang_b: 0.,
            scl_x: 1.,
            scl_y: 1.,
            scl_z: 1.,
            between: false,
            independent: false,
            replace: false,
            flip: false,
            merge: false,
            dist: 0.,
        }
    }
}
impl TypedOp for LinearCloneOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let c = crate::duplicate::CloneLine {
            count: self.num,
            offset: DVec3::new(self.off_x, self.off_y, self.off_z),
            scale: DVec3::new(self.scl_x, self.scl_y, self.scl_z) * 100.,
            rotate: DVec3::new(self.ang_p, self.ang_h, self.ang_b) * (180. / std::f64::consts::PI),
            copies: copies(self.replace, self.flip, self.merge, self.dist)?,
        };
        let faces: Vec<usize> = ctx
            .chosen_or_all()?
            .into_iter()
            .map(|f| f as usize)
            .collect();
        let centre = crate::meshops_builtin::faces_centre(ctx.input, &faces);
        let transforms = crate::duplicate::clone_transforms(&c, centre)?;
        copied(ctx, &transforms, &c.copies)
    }
}

const CLONE_GROUPS: &[(&str, &str)] = &[
    ("clone", "Cloned Polygons"),
    ("clone1", "Clone 1"),
    ("clone2", "Clone 2"),
    ("clone3", "Clone 3"),
    ("clone4", "Clone 4"),
];

pub(crate) fn duplicate_entries() -> Vec<Arc<dyn OpDef>> {
    let effector = || {
        [
            check("replace", "Replace Source"),
            check("flip", "Invert Polygons"),
            check("merge", "Merge Vertices"),
            distance("dist", "Distance"),
        ]
    };
    let mut radial = vec![
        choice("axis", "Axis", &["X", "Y", "Z"]),
        Field::new(
            "sides",
            "Count",
            FieldKind::Count {
                min: 1,
                max: 10_000,
            },
        ),
        distance("vecX", "Axis X"),
        distance("vecY", "Axis Y"),
        distance("vecZ", "Axis Z"),
        Field::new("start", "Start Angle", FieldKind::Angle),
        Field::new("end", "End Angle", FieldKind::Angle),
        distance("offset", "Offset"),
        distance("cenX", "Center X"),
        distance("cenY", "Center Y"),
        distance("cenZ", "Center Z"),
        hidden("square"),
    ];
    radial.extend(effector());
    let mut clone = vec![
        Field::new(
            "num",
            "Count",
            FieldKind::Count {
                min: 0,
                max: 10_000,
            },
        ),
        distance("offX", "Offset X"),
        distance("offY", "Offset Y"),
        distance("offZ", "Offset Z"),
        Field::new("angP", "Rotate X", FieldKind::Angle),
        Field::new("angH", "Rotate Y", FieldKind::Angle),
        Field::new("angB", "Rotate Z", FieldKind::Angle),
        Field::new("sclX", "Scale X", FieldKind::Percent),
        Field::new("sclY", "Scale Y", FieldKind::Percent),
        Field::new("sclZ", "Scale Z", FieldKind::Percent),
        hidden("between"),
        hidden("independent"),
    ];
    clone.extend(effector());
    vec![
        Arc::new(
            Typed::<RadialArrayOp>::new(
                "radial.array.item",
                "Radial Array",
                "Duplicate",
                true,
                radial,
            )
            .with_groups(CLONE_GROUPS),
        ),
        Arc::new(
            Typed::<LinearCloneOp>::new("linear.clone.item", "Clone", "Duplicate", true, clone)
                .with_groups(CLONE_GROUPS),
        ),
    ]
}

// ======== Edit group ========
//
// Slice, Axis Slice, Polygon Reduce and Mesh Cleanup. (Boolean, Solid Drill,
// Axis Drill and Merge Meshes read other items; see meshops_items.rs.)

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SliceOp {
    pub start_x: f64,
    pub start_y: f64,
    pub start_z: f64,
    pub end_x: f64,
    pub end_y: f64,
    pub end_z: f64,
    pub snap: bool,
    pub snap_angle: f64,
    pub fast: bool,
    pub split: bool,
    pub caps: bool,
    pub infinite: bool,
    pub gap: f64,
    /// 0 Center, 1 Above, 2 Below.
    pub gap_side: u32,
    /// The plane holds the line and this direction (0 X, 1 Y, 2 Z), or
    /// the Vector.
    pub axis: u32,
    pub vector_x: f64,
    pub vector_y: f64,
    pub vector_z: f64,
}
impl Default for SliceOp {
    fn default() -> Self {
        Self {
            start_x: -1.,
            start_y: 0.,
            start_z: 0.,
            end_x: 1.,
            end_y: 0.,
            end_z: 0.,
            snap: true,
            snap_angle: std::f64::consts::FRAC_PI_4,
            fast: false,
            split: false,
            caps: false,
            infinite: false,
            gap: 0.,
            gap_side: 0,
            axis: 1,
            vector_x: 0.,
            vector_y: 1.,
            vector_z: 0.,
        }
    }
}
impl TypedOp for SliceOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let start = DVec3::new(self.start_x, self.start_y, self.start_z);
        let end = DVec3::new(self.end_x, self.end_y, self.end_z);
        let up = DVec3::new(self.vector_x, self.vector_y, self.vector_z);
        finite(
            &[start.to_array(), end.to_array(), up.to_array()].concat(),
            "Slice values",
        )?;
        let cut = crate::slice::PlaneCut { start, end, up };
        if cut.normal().is_none() {
            return Err("Slice: Start and End are too close, or the Vector runs along them".into());
        }
        let o = crate::slice::PolygonSlice {
            cuts: vec![cut],
            start,
            end,
            split: self.split,
            cap_sections: self.caps,
            gap: self.gap,
            offset_side: match self.gap_side {
                1 => crate::slice::OffsetSide::Above,
                2 => crate::slice::OffsetSide::Below,
                _ => crate::slice::OffsetSide::Center,
            },
            infinite: self.infinite,
            ..Default::default()
        };
        let (mesh, _, _) = crate::slice::polygon_slice_geometry(
            ctx.input,
            glam::DMat4::IDENTITY,
            &o,
            &ctx.chosen()?,
        )?;
        Ok(mesh)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AxisSliceOp {
    /// 0 Number, 1 Size.
    pub mode: u32,
    pub num_x: u32,
    pub num_y: u32,
    pub num_z: u32,
    pub siz_x: f64,
    pub siz_y: f64,
    pub siz_z: f64,
    pub off_x: f64,
    pub off_y: f64,
    pub off_z: f64,
    pub hold: bool,
}
impl Default for AxisSliceOp {
    fn default() -> Self {
        Self {
            mode: 0,
            num_x: 0,
            num_y: 0,
            num_z: 0,
            siz_x: 1.,
            siz_y: 1.,
            siz_z: 1.,
            off_x: 0.,
            off_y: 0.,
            off_z: 0.,
            hold: false,
        }
    }
}
impl TypedOp for AxisSliceOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        if self.hold {
            return Ok(ctx.input.clone());
        }
        let o = crate::axis_slice::AxisSlice {
            mode: if self.mode == 1 {
                crate::axis_slice::AxisSliceMode::Size
            } else {
                crate::axis_slice::AxisSliceMode::Number
            },
            number: [self.num_x, self.num_y, self.num_z],
            size: DVec3::new(self.siz_x, self.siz_y, self.siz_z),
            offset: DVec3::new(self.off_x, self.off_y, self.off_z),
            hold: false,
        };
        let (mesh, _, _) = crate::axis_slice::axis_slice_geometry(
            ctx.input,
            glam::DMat4::IDENTITY,
            &o,
            &ctx.chosen()?,
        )?;
        Ok(mesh)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReduceOp {
    /// The result's polygon count (PRD-10); 0 = unchanged (PRD-11).
    pub number: u32,
    pub boundary_weight: f64,
    pub material_weight: f64,
    /// Legacy Mode: kept, not built (PRD-13, MOQ-26).
    pub method: bool,
}
impl Default for ReduceOp {
    fn default() -> Self {
        Self {
            number: 0,
            boundary_weight: 1.,
            material_weight: 1.,
            method: false,
        }
    }
}
impl TypedOp for ReduceOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.boundary_weight, self.material_weight], "Weights")?;
        let total = ctx.input.faces.len() as u32;
        // PRD-11: 0, or a Number at or above the current count, changes
        // nothing.
        if self.number == 0 || self.number >= total {
            return Ok(ctx.input.clone());
        }
        let region = ctx.chosen_or_all()?;
        let kept = total - region.len() as u32;
        let Some(target) = self.number.checked_sub(kept).filter(|&t| t > 0) else {
            return Err(format!(
                "Number {} is below the {kept} polygons outside the selection",
                self.number
            ));
        };
        let source = crate::reduction::reduction_source_count(ctx.input, &region)?;
        let p = crate::ReductionParameters {
            number: target.min(source),
            boundary_weight: self.boundary_weight * 100.,
            material_border_weight: self.material_weight * 100.,
            legacy: false,
        };
        Ok(crate::reduction::reduce_polygons(ctx.input, &region, p.number, &p)?.mesh)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MeshCleanupOp {
    pub floating_vertex: bool,
    pub one_point_polygon: bool,
    pub two_point_polygon: bool,
    pub dup_point_polygon: bool,
    pub colinear: bool,
    pub face_normal: bool,
    pub merge_vertex: bool,
    pub merge_disco: bool,
    pub unify_polygon: bool,
    pub force_unify: bool,
    pub remove_disco_weight: bool,
    pub fix_gaps: bool,
}
impl Default for MeshCleanupOp {
    fn default() -> Self {
        Self {
            floating_vertex: true,
            one_point_polygon: true,
            two_point_polygon: true,
            dup_point_polygon: true,
            colinear: true,
            face_normal: true,
            merge_vertex: true,
            merge_disco: true,
            unify_polygon: true,
            force_unify: true,
            remove_disco_weight: true,
            fix_gaps: true,
        }
    }
}
impl TypedOp for MeshCleanupOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        // In CleanupOption order.
        let flags = [
            self.floating_vertex,
            self.one_point_polygon,
            self.two_point_polygon,
            self.dup_point_polygon,
            self.colinear,
            self.face_normal,
            self.merge_vertex,
            self.merge_disco,
            self.unify_polygon,
            self.force_unify,
            self.remove_disco_weight,
            self.fix_gaps,
        ];
        let scope = ctx.selection.map(|f| {
            f.iter()
                .map(|&f| f as usize)
                .collect::<std::collections::BTreeSet<_>>()
        });
        Ok(crate::mesh_cleanup_ops::mesh_cleanup_geometry(ctx.input, scope.as_ref(), flags)?.mesh)
    }
}

pub(crate) fn edit_entries() -> Vec<Arc<dyn OpDef>> {
    let slice = vec![
        distance("startX", "Start X"),
        distance("startY", "Start Y"),
        distance("startZ", "Start Z"),
        distance("endX", "End X"),
        distance("endY", "End Y"),
        distance("endZ", "End Z"),
        check("split", "Split"),
        check("caps", "Cap Sections"),
        check("infinite", "Infinite"),
        distance("gap", "Gap"),
        choice("gapSide", "Offset Side", &["Center", "Above", "Below"]),
        // Axis is kept, not built: the Vector sets the plane (MOQ-26).
        hidden("axis"),
        Field::new("vectorX", "Vector X", FieldKind::Number),
        Field::new("vectorY", "Vector Y", FieldKind::Number),
        Field::new("vectorZ", "Vector Z", FieldKind::Number),
        hidden("snap"),
        hidden("snapAngle"),
        hidden("fast"),
    ];
    let count = |key, label| {
        Field::new(
            key,
            label,
            FieldKind::Count {
                min: 0,
                max: 10_000,
            },
        )
    };
    let axis = vec![
        choice("mode", "Mode", &["Number", "Size"]),
        count("numX", "Number X"),
        count("numY", "Number Y"),
        count("numZ", "Number Z"),
        distance("sizX", "Size X"),
        distance("sizY", "Size Y"),
        distance("sizZ", "Size Z"),
        distance("offX", "Offset X"),
        distance("offY", "Offset Y"),
        distance("offZ", "Offset Z"),
        check("hold", "Hold Slice"),
    ];
    let cleanup = vec![
        check("floatingVertex", "Remove Floating Vertices"),
        check("onePointPolygon", "Remove One Point Polygons"),
        check("twoPointPolygon", "Remove Two Points Polygons"),
        check("dupPointPolygon", "Fix Duplicate Points in Polygon"),
        check("colinear", "Remove Colinear Vertices"),
        check("faceNormal", "Fix Face Normal Vectors"),
        check("mergeVertex", "Merge Vertices"),
        check("mergeDisco", "Merge Disco Values"),
        check("unifyPolygon", "Unify Polygons"),
        check("forceUnify", "Force Unify"),
        check("removeDiscoWeight", "Remove Disco Weight Values"),
        check("fixGaps", "Fix Gaps"),
    ];
    vec![
        Arc::new(Typed::<SliceOp>::new(
            "poly.knife.item",
            "Slice",
            "Edit",
            true,
            slice,
        )),
        Arc::new(Typed::<AxisSliceOp>::new(
            "poly.julienne.item",
            "Axis Slice",
            "Edit",
            true,
            axis,
        )),
        Arc::new(Typed::<ReduceOp>::new(
            "poly.reduct.item",
            "Polygon Reduce",
            "Polygon",
            true,
            vec![
                Field::new(
                    "number",
                    "Number",
                    FieldKind::Count {
                        min: 0,
                        max: 100_000_000,
                    },
                ),
                Field::new("boundaryWeight", "Boundary Weight", FieldKind::Percent),
                Field::new(
                    "materialWeight",
                    "Material Border Weight",
                    FieldKind::Percent,
                ),
                hidden("method"),
            ],
        )),
        Arc::new(Typed::<MeshCleanupOp>::new(
            "meshcleanup.tool.item",
            "Mesh Cleanup",
            "Edit",
            true,
            cleanup,
        )),
    ]
}

// ======== Deform group ========
//
// Jitter, Push and Smooth move points only (MO-70). They work on every
// point or the points of the chosen polygons (MOQ-25).

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct JitterOp {
    pub range_x: f64,
    pub range_y: f64,
    pub range_z: f64,
    pub seed: u32,
    pub enable_x: bool,
    pub enable_y: bool,
    pub enable_z: bool,
    #[serde(rename = "lockUV")]
    pub lock_uv: bool,
    pub morph: u32,
    pub rigid_trans: bool,
    pub rigid_rotate: bool,
    pub rigid_scale: bool,
    pub stacked: bool,
}
impl Default for JitterOp {
    fn default() -> Self {
        Self {
            range_x: 0.,
            range_y: 0.,
            range_z: 0.,
            seed: 137,
            enable_x: true,
            enable_y: true,
            enable_z: true,
            lock_uv: false,
            morph: 0,
            rigid_trans: false,
            rigid_rotate: false,
            rigid_scale: false,
            stacked: false,
        }
    }
}
impl TypedOp for JitterOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let range = [self.range_x, self.range_y, self.range_z];
        finite(&range, "Range")?;
        // The direct Jitter tool's function (deform_finish.rs), so the op
        // and the tool give the same points for the same values (DP-02).
        let o = crate::deform_finish::JitterOptions {
            range: glam::DVec3::from_array(range).abs(),
            seed: self.seed,
            enable: [self.enable_x, self.enable_y, self.enable_z],
            rigid_translate: self.rigid_trans,
            rigid_rotate: self.rigid_rotate,
            rigid_scale: self.rigid_scale,
            planar: false,
        };
        let points = chosen_points(ctx).into_iter().collect();
        crate::deform_finish::jitter_geometry(ctx.input, &points, &o, None)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PushOp {
    pub dist: f64,
    /// Vertex Normals: 0 Averaged, 1 Selected.
    pub select: u32,
}
impl TypedOp for PushOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let points: std::collections::BTreeSet<u32> = chosen_points(ctx).into_iter().collect();
        let normals = if self.select == 1 {
            crate::deform::PushNormals::Selected
        } else {
            crate::deform::PushNormals::Averaged
        };
        crate::deform::push_geometry(ctx.input, &points, self.dist, normals, ctx.selection, None)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SmoothOp {
    /// Strength, fraction (1 = 100%).
    pub strn: f64,
    pub iter: u32,
    pub lock_bound: bool,
    pub lock_corner: bool,
    pub lock_sharp: bool,
    /// Radians.
    pub sharp_threshold: f64,
    pub preserve: bool,
}
impl Default for SmoothOp {
    fn default() -> Self {
        let o = crate::deform::SmoothOptions::default();
        Self {
            strn: o.strength,
            iter: o.iterations,
            lock_bound: o.lock_boundary,
            lock_corner: o.lock_corner,
            lock_sharp: o.lock_sharp,
            sharp_threshold: o.sharp_angle.to_radians(),
            preserve: o.preserve_volume,
        }
    }
}
impl TypedOp for SmoothOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(
            &[self.strn, self.sharp_threshold],
            "Strength and Sharp Angle",
        )?;
        let o = crate::deform::SmoothOptions {
            strength: self.strn,
            iterations: self.iter,
            lock_boundary: self.lock_bound,
            lock_corner: self.lock_corner,
            preserve_volume: self.preserve,
            lock_sharp: self.lock_sharp,
            sharp_angle: self.sharp_threshold.to_degrees(),
        };
        let points: std::collections::BTreeSet<u32> = chosen_points(ctx).into_iter().collect();
        crate::deform::smooth_geometry(ctx.input, &points, &o, None)
    }
}

pub(crate) fn deform_entries() -> Vec<Arc<dyn OpDef>> {
    let jitter = vec![
        distance("rangeX", "Range X"),
        distance("rangeY", "Range Y"),
        distance("rangeZ", "Range Z"),
        Field::new(
            "seed",
            "Seed",
            FieldKind::Count {
                min: 0,
                max: u32::MAX,
            },
        ),
        check("enableX", "Enable X"),
        check("enableY", "Enable Y"),
        check("enableZ", "Enable Z"),
        hidden("lockUV"),
        hidden("morph"),
        hidden("rigidTrans"),
        hidden("rigidRotate"),
        hidden("rigidScale"),
        hidden("stacked"),
    ];
    vec![
        Arc::new(Typed::<JitterOp>::new(
            "xfrm.jitter.item",
            "Jitter",
            "Deform",
            true,
            jitter,
        )),
        Arc::new(Typed::<PushOp>::new(
            "xfrm.push.item",
            "Push",
            "Deform",
            true,
            vec![
                distance("dist", "Distance"),
                choice("select", "Vertex Normals", &["Averaged", "Selected"]),
            ],
        )),
        Arc::new(Typed::<SmoothOp>::new(
            "smooth.meshop.item",
            "Smooth",
            "Deform",
            true,
            vec![
                Field::new("strn", "Strength", FieldKind::Percent),
                Field::new("iter", "Iterations", FieldKind::Count { min: 0, max: 1000 }),
                check("lockBound", "Lock Boundary"),
                check("lockCorner", "Lock Corner"),
                check("lockSharp", "Lock Sharp Edges"),
                Field::new("sharpThreshold", "Sharp Angle", FieldKind::Angle),
                check("preserve", "Preserve Volume"),
            ],
        )),
    ]
}

// ======== UV group ========
//
// UV Unwrap, Pack UVs and Transform UV Map on a UV map of the mesh below
// (`name` / `uvName`; empty = the mesh's selected map, as the direct
// tools pick it). Polygons outside the Selection input are left alone.

/// Per polygon: true when the op leaves it alone.
fn hidden_faces(ctx: &OpContext) -> Vec<bool> {
    match ctx.selection {
        None => vec![false; ctx.input.faces.len()],
        Some(faces) => {
            let mut h = vec![true; ctx.input.faces.len()];
            for &f in faces {
                h[f as usize] = false;
            }
            h
        }
    }
}
fn map_name(mesh: &Mesh, name: &str) -> String {
    crate::uv_tools::target_uv_map(mesh, (!name.is_empty()).then_some(name))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UvUnwrapOp {
    pub iter: u32,
    pub gaps: f64,
    pub matr: bool,
    pub part: bool,
    pub project: u32,
    pub pinning: u32,
    pub axis: u32,
    pub angle: f64,
    pub seal: bool,
    pub layout: bool,
    pub mode: u32,
    pub symmetrize: bool,
    pub symm_axis: u32,
    pub symm_comp: u32,
    pub name: String,
    pub seam: u32,
    pub seam_name: String,
    pub pick_name: String,
}
impl Default for UvUnwrapOp {
    fn default() -> Self {
        Self {
            iter: 100,
            gaps: 0.2,
            matr: false,
            part: false,
            project: 0,
            pinning: 0,
            axis: 1,
            angle: 0.,
            seal: true,
            layout: true,
            mode: 0,
            symmetrize: false,
            symm_axis: 0,
            symm_comp: 0,
            name: "Texture".into(),
            seam: 0,
            seam_name: String::new(),
            pick_name: String::new(),
        }
    }
}
impl TypedOp for UvUnwrapOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.gaps, self.angle], "Gaps and Offset Angle")?;
        let o = crate::uv_tools::UnwrapOptions {
            iterations: self.iter,
            relative_gap: self.gaps,
            segment_by_material: self.matr,
            segment_by_part: self.part,
            offset_angle: self.angle.to_degrees(),
            seal_holes: self.seal,
            update_layout: self.layout,
            ..Default::default()
        };
        let mut input = ctx.input.clone();
        let map = map_name(&input, &self.name);
        if !input.uv_maps.contains_key(&map) {
            let rows = crate::uv_maps::empty_rows(&input);
            input.uv_maps.insert(map.clone(), rows);
        }
        crate::uv_tools::unwrap_mesh(&input, &map, &hidden_faces(ctx), &[], &o)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PackUvsOp {
    pub name: String,
    pub write_new: bool,
    pub pack: bool,
    pub stretch: bool,
    pub orient: bool,
    pub keep_size: bool,
    pub direction: u32,
    pub gaps: f64,
    pub bbox: bool,
    pub stack: bool,
    pub region: u32,
    pub udim: u32,
    pub region_x: f64,
    pub region_y: f64,
    pub region_w: f64,
    pub region_h: f64,
    pub tile_u: u32,
    pub tile_v: u32,
    pub polygon_tag: u32,
    pub new_name: String,
    pub by_pixel: bool,
    pub gaps_by_pixel: f64,
    pub image_size: u32,
    pub bestrot: bool,
}
impl Default for PackUvsOp {
    fn default() -> Self {
        Self {
            name: String::new(),
            write_new: false,
            pack: true,
            stretch: true,
            orient: false,
            keep_size: false,
            direction: 0,
            gaps: 0.2,
            bbox: false,
            stack: false,
            region: 0,
            udim: 1001,
            region_x: 0.,
            region_y: 0.,
            region_w: 1.,
            region_h: 1.,
            tile_u: 1,
            tile_v: 1,
            polygon_tag: 0,
            new_name: "New Texture".into(),
            by_pixel: false,
            gaps_by_pixel: 10.,
            image_size: 1024,
            bestrot: true,
        }
    }
}
impl TypedOp for PackUvsOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(&[self.gaps], "Gaps")?;
        let o = crate::uv_tools::PackOptions {
            pack: self.pack,
            stretch: self.stretch,
            orient: self.orient,
            keep_size: self.keep_size,
            gaps: self.gaps,
            bounding_box: self.bbox,
            lock_stacked: self.stack,
            udim: self.udim,
            find_best_box_rotation: self.bestrot,
            ..Default::default()
        };
        let map = map_name(ctx.input, &self.name);
        if !ctx.input.uv_maps.contains_key(&map) {
            // No UVs to pack: nothing to do.
            return Ok(ctx.input.clone());
        }
        crate::uv_tools::pack_mesh(ctx.input, &map, &hidden_faces(ctx), &ctx.chosen()?, &o)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformUvOp {
    #[serde(rename = "uvName")]
    pub uv_name: String,
    #[serde(rename = "tile.U")]
    pub tile_u: f64,
    #[serde(rename = "tile.V")]
    pub tile_v: f64,
    #[serde(rename = "tile.W")]
    pub tile_w: f64,
    #[serde(rename = "offset.U")]
    pub offset_u: f64,
    #[serde(rename = "offset.V")]
    pub offset_v: f64,
    #[serde(rename = "offset.W")]
    pub offset_w: f64,
    /// Radians.
    pub angle: f64,
    #[serde(rename = "rotCentre")]
    pub rot_centre: bool,
}
impl TypedOp for TransformUvOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        finite(
            &[self.offset_u, self.offset_v, self.angle],
            "Offset and Rotation",
        )?;
        let mut mesh = ctx.input.clone();
        let map = map_name(&mesh, &self.uv_name);
        let Some(rows) = mesh.uv_maps.get(&map).cloned() else {
            return Ok(mesh);
        };
        let keep = hidden_faces(ctx);
        // Rotation about the map's origin, or the chosen UVs' centre.
        let mut centre = glam::DVec2::ZERO;
        if self.rot_centre {
            let (mut lo, mut hi) = (glam::DVec2::splat(f64::MAX), glam::DVec2::splat(f64::MIN));
            for (f, row) in rows.iter().enumerate() {
                if keep.get(f).copied().unwrap_or(true) {
                    continue;
                }
                for uv in row.iter().flatten() {
                    lo = lo.min(*uv);
                    hi = hi.max(*uv);
                }
            }
            if lo.x <= hi.x {
                centre = (lo + hi) / 2.;
            }
        }
        let (s, c) = self.angle.sin_cos();
        let offset = glam::DVec2::new(self.offset_u, self.offset_v);
        let mut out = rows;
        for (f, row) in out.iter_mut().enumerate() {
            if keep.get(f).copied().unwrap_or(true) {
                continue;
            }
            for uv in row.iter_mut().flatten() {
                let d = *uv - centre;
                *uv = centre + glam::DVec2::new(d.x * c - d.y * s, d.x * s + d.y * c) + offset;
            }
        }
        mesh.uv_maps.insert(map, out);
        Ok(mesh)
    }
}

pub(crate) fn uv_entries() -> Vec<Arc<dyn OpDef>> {
    let unwrap = vec![
        Field::new(
            "iter",
            "Iterations",
            FieldKind::Count {
                min: 0,
                max: 100_000,
            },
        ),
        Field::new("gaps", "Relative Gap Size", FieldKind::Percent),
        check("matr", "Segment by Material"),
        check("part", "Segment by Part"),
        Field::new("angle", "Offset Angle", FieldKind::Angle),
        check("seal", "Seal Holes"),
        check("layout", "Update Layout"),
        Field::new("name", "UV Map", FieldKind::Text),
        // Kept, not built in the op yet (MOQ-26).
        hidden("project"),
        hidden("pinning"),
        hidden("axis"),
        hidden("mode"),
        hidden("symmetrize"),
        hidden("symmAxis"),
        hidden("symmComp"),
        hidden("seam"),
        hidden("seamName"),
        hidden("pickName"),
    ];
    let pack = vec![
        Field::new("name", "UV Map", FieldKind::Text),
        check("pack", "Pack"),
        check("stretch", "Stretch"),
        check("orient", "Orient"),
        check("keepSize", "Keep Size"),
        Field::new("gaps", "Gaps", FieldKind::Percent),
        check("bbox", "Based on Bounding Box"),
        check("stack", "Lock Stacked"),
        Field::new(
            "udim",
            "UDIM",
            FieldKind::Count {
                min: 1001,
                max: 9999,
            },
        ),
        check("bestrot", "Find Best Box Rotation"),
        hidden("writeNew"),
        hidden("direction"),
        hidden("region"),
        hidden("regionX"),
        hidden("regionY"),
        hidden("regionW"),
        hidden("regionH"),
        hidden("tileU"),
        hidden("tileV"),
        hidden("polygonTag"),
        hidden("newName"),
        hidden("byPixel"),
        hidden("gapsByPixel"),
        hidden("imageSize"),
    ];
    let transform = vec![
        Field::new("uvName", "UV Map", FieldKind::Text),
        Field::new("offset.U", "Offset U", FieldKind::Number),
        Field::new("offset.V", "Offset V", FieldKind::Number),
        Field::new("angle", "Rotation", FieldKind::Angle),
        check("rotCentre", "Rotate at Centre"),
        hidden("tile.U"),
        hidden("tile.V"),
        hidden("tile.W"),
        hidden("offset.W"),
    ];
    vec![
        Arc::new(Typed::<UvUnwrapOp>::new(
            "uv.unwrap.item",
            "UV Unwrap",
            "UV",
            true,
            unwrap,
        )),
        Arc::new(Typed::<PackUvsOp>::new(
            "uvpack.meshop.item",
            "Pack UVs",
            "UV",
            true,
            pack,
        )),
        Arc::new(Typed::<TransformUvOp>::new(
            "pmodel.transformMap",
            "Transform UV Map",
            "UV",
            true,
            transform,
        )),
    ]
}
