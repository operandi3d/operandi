//! Operandi's own MeshOp types (Mesh Stack v1, SCOPE Stage 0 first batch):
//! their values and how they run. Each is a [`TypedOp`] with a table entry
//! in [`builtin`]; the stack itself lives in `mesh_stack.rs`.
//!
//! The ops call the direct tools' kernels (`bevel_geometry_applied`,
//! `smooth_shift_geometry`, `copy_faces` ...), never their live sessions.
use crate::meshop_types::{Field, FieldKind, OpContext, OpDef, Typed, TypedOp};
use crate::{Mesh, Result};
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub(crate) fn finite(values: &[f64], what: &str) -> Result<()> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(format!("{what} must be a finite number"))
    }
}

/// Bevel-like results: `front` = the moved polygons, `sides` = the new
/// polygons around them (provisional, MOQ-22: new polygons past the
/// input's count that aren't fronts).
fn front_and_sides(ctx: &OpContext, mut mesh: Mesh, front: &[u32]) -> Mesh {
    let n = ctx.input.faces.len() as u32;
    let sides: Vec<u32> = (n..mesh.faces.len() as u32)
        .filter(|f| !front.contains(f))
        .collect();
    ctx.tag(&mut mesh, "front", front.iter().copied());
    ctx.tag(&mut mesh, "sides", sides);
    mesh
}
/// Clone ops' groups (PG table): `clone` = every copy, `clone1`.. = each
/// copy in order; the original is not a clone.
fn clones(ctx: &OpContext, mut mesh: Mesh, new_faces: usize, per_copy: usize) -> Mesh {
    let copies = new_faces.checked_div(per_copy).unwrap_or(0);
    let end = mesh.faces.len() as u32;
    // Copies land after the input's polygons; anything else (merged
    // away) gets no groups rather than wrong ones.
    let Some(start) = end.checked_sub(new_faces as u32) else {
        return mesh;
    };
    ctx.tag(&mut mesh, "clone", start..end);
    if copies > 0 && new_faces.is_multiple_of(copies) {
        let per = (new_faces / copies) as u32;
        for k in 0..copies as u32 {
            let a = start + k * per;
            ctx.tag(&mut mesh, &format!("clone{}", k + 1), a..a + per);
        }
    }
    mesh
}

// Op values use the reference tools' attribute names (registry thread,
// MESHOPS-NAMES 2026-11-11), so `item.channel <name> <value> item:<op>`
// ports. Distances in metres, angles in radians, percents as fractions
// (1.0 = 100%). Only the attributes Operandi builds are stored; an
// unknown name is refused.

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BevelOp {
    pub shift: f64,
    pub inset: f64,
    /// Group Polygons: bevel touching polygons as one region.
    pub group: bool,
}
impl Default for BevelOp {
    fn default() -> Self {
        Self {
            shift: 0.,
            inset: 0.,
            group: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SmoothShiftOp {
    pub shift: f64,
    /// 1.0 = 100%.
    pub scale: f64,
    /// Max. Smoothing Angle, radians.
    pub max_angle: f64,
    pub thicken: bool,
    pub sharp: bool,
    pub angle_weight: bool,
    pub both_sides: bool,
}
impl Default for SmoothShiftOp {
    fn default() -> Self {
        let p = crate::SmoothShiftParameters::default();
        Self {
            shift: p.offset,
            scale: p.scale,
            max_angle: p.angle.to_radians(),
            thicken: p.thicken,
            sharp: p.sharp,
            angle_weight: p.angle_weight,
            both_sides: p.both_sides,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MirrorOp {
    /// 0 X, 1 Y, 2 Z: the plane's normal.
    pub axis: u32,
    #[serde(rename = "cenX")]
    pub cen_x: f64,
    #[serde(rename = "cenY")]
    pub cen_y: f64,
    #[serde(rename = "cenZ")]
    pub cen_z: f64,
    /// Merge Vertices on the plane, within `dist`.
    pub merge: bool,
    pub dist: f64,
}
impl Default for MirrorOp {
    fn default() -> Self {
        let tool = crate::duplicate::Mirror::default();
        Self {
            axis: 0,
            cen_x: 0.,
            cen_y: 0.,
            cen_z: 0.,
            merge: tool.copies.merge,
            dist: tool.copies.distance,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArrayOp {
    /// Cells along X, Y, Z, the original included.
    #[serde(rename = "numX")]
    pub num_x: u32,
    #[serde(rename = "numY")]
    pub num_y: u32,
    #[serde(rename = "numZ")]
    pub num_z: u32,
    #[serde(rename = "offX")]
    pub off_x: f64,
    #[serde(rename = "offY")]
    pub off_y: f64,
    #[serde(rename = "offZ")]
    pub off_z: f64,
    /// Merge Vertices, within `dist` (Clone Effector channels).
    pub merge: bool,
    /// Missing from v1 files, which saved Array flat (`array.clone.item`).
    #[serde(default)]
    pub dist: f64,
}
impl Default for ArrayOp {
    fn default() -> Self {
        let tool = crate::duplicate::Array::default();
        Self {
            num_x: tool.count[0],
            num_y: tool.count[1],
            num_z: tool.count[2],
            off_x: tool.offset.x,
            off_y: tool.offset.y,
            off_z: tool.offset.z,
            merge: tool.copies.merge,
            dist: tool.copies.distance,
        }
    }
}

/// subdivide.tool attributes (registry rev 2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubdivideOp {
    /// 0 Faceted, 1 Smooth, 2 SDS Subdivide, 3 Catmull-Clark (dialog
    /// order; the reference default is 3, the rest unpublished, SUBT-Q01).
    #[serde(rename = "type")]
    pub method: u32,
    /// Max. Smoothing Angle, radians.
    pub angle: f64,
    #[serde(rename = "linearUV")]
    pub linear_uv: bool,
    /// 0 Smooth All, 1 Crease All, 2 Crease Edges.
    pub boundary: u32,
    #[serde(rename = "noEdgeWeight")]
    pub no_edge_weight: bool,
}
impl Default for SubdivideOp {
    fn default() -> Self {
        let o = crate::SubdivideOptions::default();
        Self {
            method: crate::SubdivideMethod::ALL
                .iter()
                .position(|m| *m == o.method)
                .unwrap_or(3) as u32,
            angle: o.max_angle_degrees.to_radians(),
            linear_uv: o.linear_uvs,
            boundary: 0,
            no_edge_weight: o.ignore_edge_weights,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriangulateOp {}

// ---- running ----

impl TypedOp for BevelOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let o = self;
        finite(&[o.shift, o.inset], "Shift and Inset")?;
        let whole = ctx.chosen()?.is_empty();
        let faces = ctx.chosen_or_all()?;
        let options = crate::BevelOptions::default();
        // #43: the Falloff below weighs the shift and inset, as the tool's.
        let weights = ctx.point_weights();
        let bevel = |group: bool| {
            crate::bevel_geometry_applied(
                ctx.input,
                &faces,
                o.shift,
                o.inset,
                group,
                &options,
                weights.as_deref(),
                None,
            )
        };
        let r = match bevel(o.group) {
            Ok(r) => r,
            // A whole closed mesh has no region edge to group by; bevel
            // each polygon instead (MOQ-11, provisional).
            Err(_) if whole && o.group => bevel(false)?,
            // GitHub #25/#29: a rule or random selection often picks
            // regions a grouped bevel can't take in one go (islands
            // touching at a corner, a whole closed part). Bevel each
            // region on its own instead of failing (MOQ-31, provisional).
            Err(e) if o.group => {
                match bevel_by_region(ctx.input, &faces, o, &options, weights.as_deref()) {
                    Some(r) => r,
                    None => return Err(e),
                }
            }
            Err(e) => return Err(e),
        };
        Ok(front_and_sides(ctx, r.mesh, &r.selected))
    }
}

/// Polygon Bevel with Group Polygons on, one edge-connected region at a
/// time (MOQ-31): each region is bevelled as a group, and a region a
/// grouped bevel refuses (a whole closed part, a pinched outline) is
/// bevelled polygon by polygon. Bevel keeps the input's polygon numbers
/// and adds the new polygons after them, so the regions still to go keep
/// their numbers; `None` when that doesn't hold or a region still fails.
fn bevel_by_region(
    input: &Mesh,
    faces: &[u32],
    o: &BevelOp,
    options: &crate::BevelOptions,
    weights: Option<&[f64]>,
) -> Option<crate::BevelResult> {
    let regions = edge_connected_regions(input, faces);
    let mut mesh = input.clone();
    let mut fronts = Vec::new();
    for region in regions {
        // Bevel keeps the input's points first, so the weights still line
        // up with them; points an earlier region added get full weight.
        let run = |m: &Mesh, group: bool| {
            let w = weights.map(|w| {
                let mut w = w.to_vec();
                w.resize(m.vertices.len(), 1.);
                w
            });
            crate::bevel_geometry_applied(
                m,
                &region,
                o.shift,
                o.inset,
                group,
                options,
                w.as_deref(),
                None,
            )
        };
        let r = run(&mesh, true).or_else(|_| run(&mesh, false)).ok()?;
        let mut kept = r.selected.clone();
        kept.sort_unstable();
        let mut want = region.clone();
        want.sort_unstable();
        if kept != want || r.mesh.faces.len() < mesh.faces.len() {
            return None;
        }
        fronts.extend(r.selected);
        mesh = r.mesh;
    }
    Some(crate::BevelResult {
        mesh,
        selected: fronts,
    })
}

/// `faces` split into groups joined by shared edges, each in `faces`'
/// order.
fn edge_connected_regions(mesh: &Mesh, faces: &[u32]) -> Vec<Vec<u32>> {
    use std::collections::HashMap;
    let mut by_edge: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (i, &f) in faces.iter().enumerate() {
        let v = &mesh.faces[f as usize].vertices;
        for c in 0..v.len() {
            let (a, b) = (v[c], v[(c + 1) % v.len()]);
            by_edge.entry((a.min(b), a.max(b))).or_default().push(i);
        }
    }
    // Union-find over positions in `faces`.
    let mut parent: Vec<usize> = (0..faces.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for users in by_edge.values() {
        for w in users.windows(2) {
            let (a, b) = (root(&mut parent, w[0]), root(&mut parent, w[1]));
            parent[a.max(b)] = a.min(b);
        }
    }
    let mut order: Vec<usize> = Vec::new();
    let mut groups: HashMap<usize, Vec<u32>> = HashMap::new();
    for (i, &f) in faces.iter().enumerate() {
        let r = root(&mut parent, i);
        if !groups.contains_key(&r) {
            order.push(r);
        }
        groups.entry(r).or_default().push(f);
    }
    order
        .into_iter()
        .filter_map(|r| groups.remove(&r))
        .collect()
}
impl TypedOp for SmoothShiftOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let o = self;
        finite(&[o.max_angle], "Max. Smoothing Angle")?;
        let p = crate::SmoothShiftParameters {
            offset: o.shift,
            scale: o.scale,
            angle: o.max_angle.to_degrees(),
            thicken: o.thicken,
            sharp: o.sharp,
            angle_weight: o.angle_weight,
            both_sides: o.both_sides,
        };
        // #43: the Falloff below weighs the shift, as the tool's.
        let weights = ctx.point_weights();
        let r = crate::smooth_shift_geometry(ctx.input, &ctx.chosen()?, p, weights.as_deref())?;
        Ok(front_and_sides(ctx, r.mesh, &r.selected))
    }
}
impl TypedOp for MirrorOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let o = self;
        let center = DVec3::new(o.cen_x, o.cen_y, o.cen_z);
        finite(&center.to_array(), "Center")?;
        finite(&[o.dist], "Distance")?;
        if o.axis > 2 {
            return Err("Mirror Axis must be 0 (X), 1 (Y) or 2 (Z)".into());
        }
        if o.dist < 0. {
            return Err("Mirror Distance can't be negative".into());
        }
        let axis = o.axis as usize;
        let (left, up) = crate::duplicate::Mirror::axis_frame(axis);
        let tool = crate::duplicate::Mirror {
            axis,
            center,
            left,
            up,
            ..Default::default()
        };
        let copies = crate::duplicate::Copies {
            merge: o.merge,
            distance: o.dist,
            ..tool.copies
        };
        let source = faces_usize(ctx.chosen_or_all()?);
        let r = crate::duplicate::copy_faces(
            ctx.input,
            &source,
            &[tool.transform()?],
            &copies,
            crate::duplicate::WeldAt::Mean,
            copies.distance,
        )?;
        Ok(clones(ctx, r.mesh, r.new_faces, source.len()))
    }
}
impl TypedOp for ArrayOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let o = self;
        let offset = DVec3::new(o.off_x, o.off_y, o.off_z);
        finite(&offset.to_array(), "Offset")?;
        let count = [o.num_x, o.num_y, o.num_z];
        if count.contains(&0) {
            return Err("Array needs at least 1 cell on every axis".into());
        }
        let source = faces_usize(ctx.chosen_or_all()?);
        let centre = faces_centre(ctx.input, &source);
        let mut tool = crate::duplicate::Array {
            count,
            offset,
            ..Default::default()
        };
        tool.copies.merge = o.merge;
        finite(&[o.dist], "Distance")?;
        if o.dist < 0. {
            return Err("Array Distance can't be negative".into());
        }
        tool.copies.distance = o.dist;
        let transforms: Vec<DMat4> = crate::duplicate::array_transforms(&tool, centre)?;
        let r = crate::duplicate::copy_faces(
            ctx.input,
            &source,
            &transforms,
            &tool.copies,
            crate::duplicate::WeldAt::Original,
            tool.copies.distance,
        )?;
        Ok(clones(ctx, r.mesh, r.new_faces, source.len()))
    }
}
impl TypedOp for SubdivideOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let o = self;
        finite(&[o.angle], "Max. Smoothing Angle")?;
        let method = *crate::SubdivideMethod::ALL
            .get(o.method as usize)
            .ok_or("Subdivide type must be 0 to 3")?;
        let boundary = *crate::polygon_subdivide::SubdivideBoundary::ALL
            .get(o.boundary as usize)
            .ok_or("Subdivide boundary must be 0 to 2")?;
        let options = crate::SubdivideOptions {
            method,
            max_angle_degrees: o.angle.to_degrees(),
            linear_uvs: o.linear_uv,
            boundary,
            ignore_edge_weights: o.no_edge_weight,
            ..Default::default()
        };
        // Fractal isn't an op value; a fixed seed keeps it repeatable
        // (SAFE-11).
        Ok(
            crate::polygon_subdivide::subdivide_polygons(ctx.input, &ctx.chosen()?, &options, 1)?
                .mesh,
        )
    }
}
impl TypedOp for TriangulateOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let input = ctx.input;
        let faces: Vec<u32> = ctx
            .chosen_or_all()?
            .into_iter()
            .filter(|&f| {
                !input.is_polyline(f as usize) && !input.attributes.degenerate_faces.contains(&f)
            })
            .collect();
        if faces.is_empty() {
            return Ok(input.clone());
        }
        Ok(match crate::triple_faces(input, &faces)? {
            Some(outcome) => outcome.mesh,
            None => input.clone(),
        })
    }
}

pub(crate) fn faces_usize(faces: Vec<u32>) -> Vec<usize> {
    faces.into_iter().map(|f| f as usize).collect()
}
pub(crate) fn faces_centre(mesh: &Mesh, faces: &[usize]) -> DVec3 {
    let mut lo = DVec3::splat(f64::INFINITY);
    let mut hi = DVec3::splat(f64::NEG_INFINITY);
    for &f in faces {
        for &v in &mesh.faces[f].vertices {
            let p = mesh.vertices[v as usize];
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    if lo.is_finite() && hi.is_finite() {
        (lo + hi) / 2.
    } else {
        DVec3::ZERO
    }
}

// ---- the table entries ----

pub(crate) fn xyz(prefix: &'static str, label: &'static str, kind: FieldKind) -> [Field; 3] {
    // Keys like `cenX`; labels like `Position X`.
    let f = |a: &str| Field {
        key: format!("{prefix}{a}").into(),
        label: format!("{label} {a}").into(),
        kind: kind.clone(),
    };
    [f("X"), f("Y"), f("Z")]
}
pub(crate) fn distance(key: &'static str, label: &'static str) -> Field {
    Field::new(key, label, FieldKind::Distance)
}
pub(crate) fn check(key: &'static str, label: &'static str) -> Field {
    Field::new(key, label, FieldKind::Check)
}
pub(crate) fn count(key: &'static str, label: &'static str, min: u32, max: u32) -> Field {
    Field::new(key, label, FieldKind::Count { min, max })
}
pub(crate) fn choice(key: &'static str, label: &'static str, names: &[&'static str]) -> Field {
    Field::new(
        key,
        label,
        FieldKind::Choice(names.iter().map(|n| (*n).into()).collect()),
    )
}
const FRONT_SIDES: &[(&str, &str)] = &[("front", "Front"), ("sides", "Sides")];
const CLONE_GROUPS: &[(&str, &str)] = &[
    ("clone", "Cloned Polygons"),
    ("clone1", "Clone 1"),
    ("clone2", "Clone 2"),
    ("clone3", "Clone 3"),
    ("clone4", "Clone 4"),
    ("clone5", "Clone 5"),
    ("clone6", "Clone 6"),
    ("clone7", "Clone 7"),
    ("clone8", "Clone 8"),
];

/// Operandi's own op types, in Add Operator order. Ids are the reference
/// tool ids (registry thread, MESHOPS-NAMES rev 3): what files store and
/// `meshop.create` takes.
pub(crate) fn builtin() -> Vec<Arc<dyn OpDef>> {
    let mut mirror = vec![choice("axis", "Axis", &["X", "Y", "Z"])];
    mirror.extend(xyz("cen", "Position", FieldKind::Distance));
    mirror.extend([
        check("merge", "Merge Vertices"),
        distance("dist", "Distance"),
    ]);
    let mut array = Vec::new();
    array.extend(xyz(
        "num",
        "Count",
        FieldKind::Count {
            min: 1,
            max: 100_000,
        },
    ));
    array.extend(xyz("off", "Offset", FieldKind::Distance));
    array.extend([
        check("merge", "Merge Vertices"),
        distance("dist", "Distance"),
    ]);
    let max_angle = |key| Field::new(key, "Max Smoothing Angle", FieldKind::Angle);
    // The primitives, each through its tool's one generator (primitive_ops.rs).
    let mut table: Vec<Arc<dyn OpDef>> = crate::primitive_ops::entries();
    table.extend::<Vec<Arc<dyn OpDef>>>(vec![
        Arc::new(
            Typed::<BevelOp>::new(
                "poly.bevel.item",
                "Polygon Bevel",
                "Polygon",
                true,
                vec![
                    distance("shift", "Shift"),
                    distance("inset", "Inset"),
                    check("group", "Group Polygons"),
                ],
            )
            .with_groups(FRONT_SIDES),
        ),
        Arc::new(
            Typed::<SmoothShiftOp>::new(
                "poly.smshift.item",
                "Smooth Shift",
                "Polygon",
                true,
                vec![
                    distance("shift", "Shift"),
                    Field::new("scale", "Scale", FieldKind::Percent),
                    max_angle("maxAngle"),
                    check("thicken", "Thicken"),
                    check("sharp", "Sharp"),
                    check("angleWeight", "Angle Weight"),
                    check("bothSides", "Both Sides"),
                ],
            )
            .with_groups(FRONT_SIDES),
        ),
        Arc::new(Typed::<SubdivideOp>::new(
            "subdivide.tool.item",
            "Subdivide",
            "Polygon",
            true,
            vec![
                choice(
                    "type",
                    "Method",
                    &["Faceted", "Smooth", "SDS Subdivide", "Catmull-Clark"],
                ),
                max_angle("angle"),
                check("linearUV", "Linear UVs"),
                choice(
                    "boundary",
                    "Boundary",
                    &["Smooth All", "Crease All", "Crease Edges"],
                ),
                check("noEdgeWeight", "Ignore Edge Weights"),
            ],
        )),
        Arc::new(Typed::<TriangulateOp>::new(
            "pmodel.triangulate.item",
            "Triangulate",
            "Polygon",
            true,
            Vec::new(),
        )),
        Arc::new(
            Typed::<MirrorOp>::new("mirror.clone.item", "Mirror", "Duplicate", true, mirror)
                .with_groups(CLONE_GROUPS),
        ),
        Arc::new(
            Typed::<ArrayOp>::new("array.clone.item", "Array", "Duplicate", true, array)
                .with_groups(CLONE_GROUPS),
        ),
    ]);
    table.extend(crate::meshop_text::entries());
    table.extend(crate::meshop_arc::entries());
    // Ops that read other items (Boolean, Merge Meshes).
    table.extend(crate::mesh_stack_items::entries());
    table.extend(crate::meshblend::op::entries());
    // Direct tools as ops, by group (mesh_stack_tools.rs).
    table.extend(crate::mesh_stack_tools::polygon_entries());
    table.extend(crate::meshop_thicken::entries());
    table.extend(crate::meshop_polygon_type::entries());
    table.extend(crate::meshop_part_tag::entries());
    // Their Selection input picks edges and points (#19).
    table.extend(crate::meshop_types::with_element(
        crate::mesh_stack_tools::edge_entries(),
        crate::meshop_types::SelElement::Edge,
    ));
    table.extend(crate::meshop_bridge::entries());
    table.extend(crate::meshop_edge_slide::entries());
    table.extend(crate::meshop_chamfer_edit::entries());
    table.extend(crate::meshop_types::with_element(
        crate::mesh_stack_tools::vertex_entries(),
        crate::meshop_types::SelElement::Point,
    ));
    table.extend(crate::mesh_stack_tools::duplicate_entries());
    table.extend(crate::meshop_radial_sweep::entries());
    table.extend(crate::mesh_stack_tools::edit_entries());
    table.extend(crate::meshop_loop_slice::entries());
    table.extend(crate::meshop_primitive_slice::entries());
    table.extend(crate::meshop_curve_slice::entries());
    table.extend(crate::meshop_pen_slice::entries());
    table.extend(crate::meshop_hard_edge::entries());
    table.extend(crate::mesh_stack_tools::deform_entries());
    table.extend(crate::mesh_stack_tools::uv_entries());
    table.extend(crate::meshop_create_uv_map::entries());
    table.extend(crate::meshop_uv_relax::entries());
    table.extend(crate::meshop_uv_unwrap_relax::entries());
    table.extend(crate::meshop_uv_flip::entries());
    table.extend(crate::meshop_uv_to_mesh::entries());
    table.extend(crate::meshop_set_edge_weight::entries());
    table.extend(crate::meshop_create_vertex_map::entries());
    table.extend(crate::meshop_set_weight::entries());
    table.extend(crate::meshop_flood_weight::entries());
    table.extend(crate::meshop_set_rgb::entries());
    table.extend(crate::meshop_set_seam::entries());
    table.extend(crate::meshop_set_vertex_normals::entries());
    table.extend(crate::meshop_geodesic_weight::entries());
    table.extend(crate::meshop_assign_selection_set::entries());
    table.extend(crate::meshop_tool_pipe::entries());
    // The Deform finish tools' ops (mesh_stack_deform.rs).
    table.extend(crate::mesh_stack_deform::entries());
    table.extend(crate::meshop_bend::entries());
    table.extend(crate::meshop_twist::entries());
    table.extend(crate::meshop_transform_effector::entries());
    table.extend(crate::meshop_rotate_effector::entries());
    table.extend(crate::meshop_magnet::entries());
    table.extend(crate::meshop_weight_influence::entries());
    table.extend(crate::meshop_push_influence::entries());
    table.extend(crate::meshop_morph_influence::entries());
    table.extend(crate::meshop_vortex::entries());
    table.extend(crate::meshop_uv_transform::entries());
    table.extend(crate::meshop_wrap::entries());
    table.extend(crate::meshop_lattice::entries());
    table.extend(crate::meshop_spline_deformer::entries());
    table.extend(crate::meshop_bezier_deformer::entries());
    table.extend(crate::meshop_curve_constraint::entries());
    table.extend(crate::curve_ops::entries());
    table.extend(crate::meshop_curve_fill::entries());
    table.extend(crate::meshop_curve_rebuild::entries());
    table.extend(crate::meshop_curve_rebuild_spans::entries());
    table.extend(crate::meshop_curve_sweep::entries());
    table.extend(crate::meshop_lace::entries());
    table.extend(crate::meshop_edges_to_curves::entries());
    table.extend(crate::meshop_curve_boolean::entries());
    table.extend(crate::meshop_curve_convert::entries());
    table.extend(crate::meshop_curve_winding::entries());
    table.extend(crate::generator_ops::entries());
    table.extend(crate::skin::entries());
    table
}
