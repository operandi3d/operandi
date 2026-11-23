//! Mesh Stack that read other mesh items: Boolean (BOOL-32, BOP-xx) and
//! Merge Meshes (MM-xx), from specifications meshops-specs-2026-11-17.
//! Both link items through the op table's `Item` / `Items` fields, so the
//! stack evaluates those items first and hands over their results with
//! their placement (SAFE-07..09, meshop_types.rs).
//!
//! - **Boolean** (`boolean.meshop.item`): the direct Boolean's engine
//!   (`boolean::boolean_gathered`: pre-flight, Manifold with the boolmesh
//!   cross-check, polygons rebuilt from provenance) with the mesh below as
//!   the target and one linked Driver Surface (BOP-30). Use World
//!   Transform on places the driver where it appears relative to the
//!   target; off, as if the target sat at the origin (BOP-22). Union All
//!   is a fifth Operation (BOP-31).
//! - **Merge Meshes** (`pmodel.meshmerge`): adds each source's result to
//!   the mesh below, nothing welded (MM-07/08), placed by the source's
//!   world transform without undoing the target's, or by its own points
//!   with Source World Transform off (MM-05). Index picks sources by link
//!   order (MM-06); Merge Hierarchy adds their child meshes (MM-11); the
//!   polygon type and map switches drop what they turn off (MM-10).
//!
//! Operandi keeps the links in the op's values (`driver`, `sources`,
//! `hierarchy`); reference files store them as links and a tag (BOP-01,
//! MM-02/03), which the LXO reader and writer translate.
use crate::boolean::{BooleanOp, Gathered, Input, boolean_gathered};
use crate::meshop_types::{
    Field, FieldKind, OpContext, OpDef, Source, Typed, TypedOp, field_links,
};
use crate::{Mesh, PolygonRepresentation, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Saved type ids (BOP-01, MM-01).
pub const BOOLEAN_OP: &str = "boolean.meshop.item";
pub const MERGE_MESHES_OP: &str = "pmodel.meshmerge";

/// Boolean's Operation choice: BOOL-05's four and Union All (BOP-31).
/// The drills' Operation choice (DRILL-10..13).
pub const DRILL_MODES: [&str; 4] = ["Core", "Tunnel", "Stencil", "Slice"];
pub const BOOLEAN_MODES: [&str; 5] = ["Union", "Intersect", "Subtract", "Add", "Union All"];

/// Boolean's values (BOP-02, BOP-30).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanMeshOp {
    /// 0 Union, 1 Intersect, 2 Subtract, 3 Add, 4 Union All.
    pub mode: u32,
    /// The Driver Surface: a mesh item, or a group whose meshes drive
    /// (provisional, MOQ-16); 0 = none.
    pub driver: u64,
    /// Use World Transform (BOP-22).
    #[serde(rename = "useWorldXfrm")]
    pub use_world: bool,
}
impl Default for BooleanMeshOp {
    fn default() -> Self {
        Self {
            mode: 0,
            driver: 0,
            use_world: true,
        }
    }
}

impl TypedOp for BooleanMeshOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let (op, label) = match self.mode {
            0..=3 => {
                let op = BooleanOp::ALL[self.mode as usize];
                (op, op.label())
            }
            // Every driver already acts as one solid (BOOL-07), so Union
            // All is Union with all of them (BOP-20).
            4 => (BooleanOp::Union, BOOLEAN_MODES[4]),
            _ => return Err("Boolean Operation must be 0 (Union) to 4 (Union All)".into()),
        };
        let what = format!("Boolean {label}");
        if self.driver == 0 {
            return Err(format!("{what}: choose its Driver Surface"));
        }
        let Some(g) = gather(ctx, self.use_world, &what)? else {
            return Ok(ctx.input.clone());
        };
        Ok(boolean_gathered(&g, op, &what)?.0)
    }
}

/// The mesh below as the target and the linked drivers, all in this
/// item's space, for the Boolean and drill engines. `None` when the mesh
/// below is empty (nothing to cut). Faces from the mesh below are tagged
/// `geometry` and from drivers `driver` (PG), and the tags ride through
/// the engines' rebuild.
fn gather(ctx: &OpContext, use_world: bool, what: &str) -> Result<Option<Gathered>> {
    let sources = ctx.sources()?;
    if ctx.input.faces.is_empty() {
        return Ok(None);
    }
    let mut combined = ctx.input.clone();
    let target = Input {
        item: 0,
        name: "The mesh below".into(),
        faces: (0..combined.faces.len()).collect(),
        point_offset: 0,
    };
    let mut drivers = Vec::new();
    for s in sources {
        let mut mesh = if use_world {
            s.local_mesh()
        } else {
            // As if the target sat at the origin (BOP-22).
            s.placed_mesh(s.world)
        };
        if mesh.faces.is_empty() {
            continue;
        }
        // BOOL-08: a copy of the driver's geometry, without its selection
        // sets, comes along.
        let a = &mut mesh.attributes;
        a.vertex_sets.clear();
        a.polygon_sets.clear();
        a.edge_sets.clear();
        a.set_groups.clear();
        let point_offset = combined.vertices.len() as u32;
        let first = combined.faces.len();
        crate::append_mesh(&mut combined, mesh)?;
        drivers.push(Input {
            item: s.item,
            name: s.name.clone(),
            faces: (first..combined.faces.len()).collect(),
            point_offset,
        });
    }
    if drivers.is_empty() {
        return Err(format!("{what}: the driver has no polygons"));
    }
    let n = ctx.input.faces.len() as u32;
    let total = combined.faces.len() as u32;
    ctx.tag(&mut combined, "geometry", 0..n);
    ctx.tag(&mut combined, "driver", n..total);
    Ok(Some(Gathered {
        combined,
        target,
        drivers,
        // Everything is already in this item's space.
        to_world: glam::DMat4::IDENTITY,
    }))
}

/// Solid Drill and Axis Drill ops (BOP-01/02): the direct drills' engine
/// (`drill::drill_mesh`) with the mesh below as the target and one linked
/// driver (Driver Surface / Template Surface).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DrillMeshOp {
    /// 0 Core, 1 Tunnel, 2 Stencil, 3 Slice.
    pub mode: u32,
    /// Axis Drill only: 0 X, 1 Y, 2 Z.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<u32>,
    pub material: String,
    pub driver: u64,
    #[serde(rename = "useWorldXfrm")]
    pub use_world: bool,
    /// Axis Drill's Select Sliced Edges, Edge Selection Set and Front Face
    /// Only: kept, not built yet (MOQ-26).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select_edges: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_set_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub front_only: Option<bool>,
}
impl Default for DrillMeshOp {
    fn default() -> Self {
        Self {
            mode: 1,
            axis: None,
            material: "Default".into(),
            driver: 0,
            use_world: true,
            select_edges: None,
            selection_set_name: None,
            front_only: None,
        }
    }
}
impl DrillMeshOp {
    fn run_as(&self, ctx: &OpContext, kind: crate::DrillKind) -> Result<Mesh> {
        let op = *[
            crate::DrillOp::Core,
            crate::DrillOp::Tunnel,
            crate::DrillOp::Stencil,
            crate::DrillOp::Slice,
        ]
        .get(self.mode as usize)
        .ok_or("Operation must be 0 (Core) to 3 (Slice)")?;
        let axis = match self.axis.unwrap_or(2) {
            0 => crate::DrillAxis::X,
            1 => crate::DrillAxis::Y,
            2 => crate::DrillAxis::Z,
            _ => return Err("Axis must be 0 (X), 1 (Y) or 2 (Z)".into()),
        };
        let what = format!("{} {}", kind.label(), op.label());
        if self.driver == 0 {
            return Err(format!("{what}: choose its driver"));
        }
        let Some(g) = gather(ctx, self.use_world, &what)? else {
            return Ok(ctx.input.clone());
        };
        let o = crate::DrillOptions {
            op,
            axis,
            material: self.material.clone(),
            ..Default::default()
        };
        Ok(crate::drill::drill_mesh(&g, kind, &o, &what)?.0)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SolidDrillOp(pub DrillMeshOp);
impl TypedOp for SolidDrillOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        self.0.run_as(ctx, crate::DrillKind::Solid)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AxisDrillOp(pub DrillMeshOp);
impl Default for AxisDrillOp {
    fn default() -> Self {
        Self(DrillMeshOp {
            axis: Some(2),
            select_edges: Some(false),
            selection_set_name: Some("Sliced Edges".into()),
            front_only: Some(false),
            ..Default::default()
        })
    }
}
impl TypedOp for AxisDrillOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        self.0.run_as(ctx, crate::DrillKind::Axis)
    }
}

/// Merge Meshes' values (MM-04), with Operandi's `sources` (the links,
/// MM-02) and `hierarchy` (the Merge Hierarchy tag, MM-03).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MergeMeshesOp {
    /// The sources, in link order (Index counts in this order).
    pub sources: Vec<u64>,
    /// Merge Hierarchy: the sources' child meshes merge too.
    pub hierarchy: bool,
    /// Source World Transform.
    pub world: bool,
    /// Use World Transforms (MOP-MM-M03/M04): stored and saved, but the
    /// placement follows Source World Transform alone; off places the
    /// sources just as on does (MOQ-30, provisional). Files from before
    /// it read as on.
    #[serde(rename = "useWorldXfrm", default = "on")]
    pub use_world: bool,
    /// Which sources merge: empty or `...` = all; `1`, `1...`, `0,2`,
    /// `1-3` (0-based).
    pub index: String,
    pub copy_ptag: bool,
    pub copy_morph: bool,
    pub copy_normal: bool,
    #[serde(rename = "copyUV")]
    pub copy_uv: bool,
    pub copy_weight: bool,
    pub copy_color: bool,
    pub copy_selection_set: bool,
    pub copy_other_maps: bool,
    pub face: bool,
    pub subdiv: bool,
    pub psubdiv: bool,
    pub curve: bool,
    pub bezier: bool,
    #[serde(rename = "BSpline")]
    pub b_spline: bool,
    pub spatch: bool,
    pub text: bool,
    pub line: bool,
    pub curve_fill: bool,
    /// Internal; kept and saved as read.
    pub filter_ident: String,
}
impl Default for MergeMeshesOp {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            hierarchy: false,
            world: true,
            use_world: true,
            index: String::new(),
            copy_ptag: true,
            copy_morph: true,
            copy_normal: false,
            copy_uv: true,
            copy_weight: true,
            copy_color: true,
            copy_selection_set: true,
            copy_other_maps: true,
            face: true,
            subdiv: true,
            psubdiv: true,
            curve: true,
            bezier: true,
            b_spline: true,
            spatch: true,
            text: true,
            line: true,
            curve_fill: true,
            filter_ident: "none".into(),
        }
    }
}

fn on() -> bool {
    true
}

/// MM-06: which of `count` sources an Index text picks, in order. Bad
/// text is an error (the op fails and passes through).
pub fn index_picks(text: &str, count: usize) -> Result<Vec<usize>> {
    let mut picked = vec![false; count];
    let mut any = false;
    for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        any = true;
        let bad = || format!("Index “{part}” isn't a number, n..., a-b or ...");
        let (from, to) = if part == "..." {
            (0, usize::MAX)
        } else if let Some(n) = part.strip_suffix("...") {
            (n.trim().parse().map_err(|_| bad())?, usize::MAX)
        } else if let Some((a, b)) = part.split_once('-') {
            let a: usize = a.trim().parse().map_err(|_| bad())?;
            let b: usize = b.trim().parse().map_err(|_| bad())?;
            if b < a {
                return Err(bad());
            }
            (a, b)
        } else {
            let n: usize = part.parse().map_err(|_| bad())?;
            (n, n)
        };
        for p in picked.iter_mut().take(to.saturating_add(1)).skip(from) {
            *p = true;
        }
    }
    if !any {
        return Ok((0..count).collect());
    }
    Ok((0..count).filter(|&i| picked[i]).collect())
}

impl MergeMeshesOp {
    /// MM-10: whether a polygon of this kind is copied.
    fn keeps(&self, r: Option<&PolygonRepresentation>) -> bool {
        use crate::curves::CurveKind;
        match r {
            // A live patch is a polygon of its own kind; Merge Meshes has no
            // switch for it yet, so it follows Face (PC-7, provisional).
            None | Some(PolygonRepresentation::Face) | Some(PolygonRepresentation::Patch) => {
                self.face
            }
            Some(PolygonRepresentation::Subdivs) => self.subdiv,
            Some(PolygonRepresentation::CatmullClark) => self.psubdiv,
            Some(PolygonRepresentation::Polyline { curve, .. }) => match curve {
                CurveKind::Line => self.line,
                CurveKind::Curve { .. } => self.curve,
                CurveKind::BSpline => self.b_spline,
                CurveKind::Bezier => self.bezier,
            },
        }
    }
    /// One source's copy, filtered by the type and map switches.
    fn copy(&self, s: &Source) -> Result<Mesh> {
        let mut m = if self.world {
            // Placed by the source's world transform; the target's own
            // transform is not undone (MM-05).
            s.placed_mesh(s.world)
        } else {
            s.mesh.to_owned_value()
        };
        let dropped: Vec<u32> = (0..m.faces.len())
            .filter(|&f| !self.keeps(m.attributes.representations.get(f)))
            .map(|f| f as u32)
            .collect();
        if !dropped.is_empty() {
            m = crate::polygon_remove::delete_geometry(&m, &dropped, &[])?;
            // Points only the dropped polygons used go too.
            m = crate::mesh_cleanup::remove_unreferenced(&m, None)?.mesh;
        }
        if !self.copy_ptag {
            // Provisional (MM-Q1): part tags go, materials stay.
            for f in m.faces.iter_mut() {
                f.part = None;
            }
        }
        if !self.copy_uv {
            m.uv_maps = Default::default();
        }
        let a = &mut *m.attributes;
        if !self.copy_uv {
            a.point_uvs.clear();
            a.uv_records.clear();
            a.current_uv_map = None;
        }
        if !self.copy_morph {
            a.relative_morphs.clear();
        }
        if !self.copy_normal {
            a.corner_normals.clear();
            a.normal_map_order.clear();
        }
        if !self.copy_weight {
            a.corner_weights.clear();
            a.point_weights.clear();
        }
        if !self.copy_color {
            a.corner_colors.clear();
            a.point_colors.clear();
        }
        if !self.copy_selection_set {
            a.vertex_sets.clear();
            a.polygon_sets.clear();
            a.edge_sets.clear();
            a.set_groups.clear();
        }
        if !self.copy_other_maps {
            a.typed_maps.clear();
        }
        Ok(m)
    }
}

impl TypedOp for MergeMeshesOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        // Checked here too, so bad text fails the op (SAFE-10).
        index_picks(&self.index, self.sources.len())?;
        let mut out = ctx.input.clone();
        for linked in ctx.links {
            // A missing source contributes nothing; the op still runs
            // (MM-14, SAFE-08).
            if let Ok(s) = &linked.source {
                crate::append_mesh(&mut out, self.copy(s)?)?;
            }
        }
        Ok(out)
    }
}

/// The meshes `id` stands for: itself if it's a mesh, else its mesh
/// descendants (a group); with `children`, a mesh's descendants too. An id
/// that names nothing is kept, so it shows as a missing input (SAFE-08).
fn meshes_of(scene: &crate::Scene, id: u64, children: bool) -> Vec<u64> {
    let Some(item) = scene.item(id) else {
        return vec![id];
    };
    let mut out = Vec::new();
    if item.is_mesh() {
        out.push(id);
    }
    if !item.is_mesh() || children {
        let mut stack = vec![id];
        while let Some(p) = stack.pop() {
            for i in scene.items.iter().filter(|i| i.parent == Some(p)) {
                if i.id != id && !out.contains(&i.id) && !stack.contains(&i.id) {
                    if i.is_mesh() && i.kit.is_none() {
                        out.push(i.id);
                    }
                    stack.push(i.id);
                }
            }
        }
    }
    out
}

/// A table entry whose links expand: a group stands for its meshes, and
/// `expand` may pick and add more (Merge Meshes' Index and Hierarchy).
struct ExpandLinks<T> {
    inner: Typed<T>,
    expand: fn(&serde_json::Value, &crate::Scene, Vec<u64>) -> Vec<u64>,
    created: fn(&mut serde_json::Value, &[u64]),
}
impl<T: TypedOp> OpDef for ExpandLinks<T> {
    fn id(&self) -> &str {
        self.inner.id()
    }
    fn label(&self) -> &str {
        self.inner.label()
    }
    fn folder(&self) -> &str {
        self.inner.folder()
    }
    fn takes_selection(&self) -> bool {
        self.inner.takes_selection()
    }
    fn fields(&self) -> &[Field] {
        self.inner.fields()
    }
    fn defaults(&self) -> serde_json::Value {
        self.inner.defaults()
    }
    fn read(&self, version: u32, values: &serde_json::Value) -> Result<serde_json::Value> {
        self.inner.read(version, values)
    }
    fn links(&self, values: &serde_json::Value, scene: &crate::Scene) -> Vec<u64> {
        let ids = field_links(self.inner.fields(), values);
        let mut out = Vec::new();
        for id in (self.expand)(values, scene, ids) {
            if !out.contains(&id) {
                out.push(id);
            }
        }
        out
    }
    fn created(&self, values: &mut serde_json::Value, picked: &[u64]) {
        (self.created)(values, picked)
    }
    fn groups(&self) -> Vec<(String, String)> {
        self.inner.groups()
    }
    fn run(&self, ctx: &OpContext, values: &serde_json::Value) -> Result<Mesh> {
        self.inner.run(ctx, values)
    }
}

fn boolean_links(_: &serde_json::Value, scene: &crate::Scene, ids: Vec<u64>) -> Vec<u64> {
    ids.into_iter()
        .flat_map(|id| meshes_of(scene, id, false))
        .collect()
}
fn merge_links(values: &serde_json::Value, scene: &crate::Scene, ids: Vec<u64>) -> Vec<u64> {
    let text = values["index"].as_str().unwrap_or_default();
    let hierarchy = values["hierarchy"].as_bool().unwrap_or(false);
    let Ok(picks) = index_picks(text, ids.len()) else {
        // Bad text: link nothing; the op fails with the reason.
        return Vec::new();
    };
    picks
        .into_iter()
        .flat_map(|i| meshes_of(scene, ids[i], hierarchy))
        .collect()
}

pub(crate) fn entries() -> Vec<Arc<dyn OpDef>> {
    let check = |key, label| Field::new(key, label, FieldKind::Check);
    let merge_fields = vec![
        Field::new("sources", "Source", FieldKind::Items),
        check("hierarchy", "Merge Hierarchy"),
        check("world", "Source World Transform"),
        check("useWorldXfrm", "Use World Transforms"),
        Field::new("index", "Index", FieldKind::Text),
        check("copyPtag", "Polygon Tags"),
        check("copyMorph", "Morph Maps"),
        check("copyNormal", "Normal Maps"),
        check("copyUV", "UV Maps"),
        check("copyWeight", "Weight Maps"),
        check("copyColor", "Color Maps"),
        check("copySelectionSet", "Selection Set"),
        check("copyOtherMaps", "Other Maps"),
        check("face", "Faces"),
        check("subdiv", "Subdivs"),
        check("psubdiv", "Catmull-Clark"),
        check("curve", "Curve"),
        check("bezier", "Bezier"),
        check("BSpline", "B-Spline"),
        check("spatch", "Spline Patch"),
        check("text", "Text"),
        check("line", "Polyline"),
        check("curveFill", "Curve Fill"),
        Field::new("filterIdent", "Filter", FieldKind::Hidden),
    ];
    vec![
        Arc::new(ExpandLinks::<BooleanMeshOp> {
            inner: Typed::new(
                BOOLEAN_OP,
                "Boolean",
                "Edit",
                false,
                vec![
                    Field::new(
                        "mode",
                        "Operation",
                        FieldKind::Choice(BOOLEAN_MODES.iter().map(|&m| m.into()).collect()),
                    ),
                    Field::new("driver", "Driver Surface", FieldKind::Item),
                    check("useWorldXfrm", "Use World Transform"),
                ],
            )
            .with_groups(&[
                ("geometry", "Polygons from Geometry"),
                ("driver", "Polygons from Driver Surface"),
            ]),
            expand: boolean_links,
            // BOP-10: the other selected mesh drives (provisional stand-in
            // for the create dialog, MOQ-16).
            created: |values, picked| {
                if let Some(&driver) = picked.first() {
                    values["driver"] = driver.into();
                }
            },
        }),
        Arc::new(ExpandLinks::<SolidDrillOp> {
            inner: Typed::new(
                "solidDrill.meshop.item",
                "Solid Drill",
                "Edit",
                false,
                vec![
                    Field::new(
                        "mode",
                        "Operation",
                        FieldKind::Choice(DRILL_MODES.iter().map(|&m| m.into()).collect()),
                    ),
                    Field::new("material", "Material", FieldKind::Text),
                    Field::new("driver", "Driver Surface", FieldKind::Item),
                    check("useWorldXfrm", "Use World Transform"),
                ],
            )
            .with_groups(&[
                ("geometry", "Polygons from Geometry"),
                ("driver", "Polygons from Driver Surface"),
            ]),
            expand: boolean_links,
            created: |values, picked| {
                if let Some(&driver) = picked.first() {
                    values["driver"] = driver.into();
                }
            },
        }),
        Arc::new(ExpandLinks::<AxisDrillOp> {
            inner: Typed::new(
                "axisDrill.meshop.item",
                "Axis Drill",
                "Edit",
                false,
                vec![
                    Field::new(
                        "mode",
                        "Operation",
                        FieldKind::Choice(DRILL_MODES.iter().map(|&m| m.into()).collect()),
                    ),
                    Field::new(
                        "axis",
                        "Axis",
                        FieldKind::Choice(vec!["X".into(), "Y".into(), "Z".into()]),
                    ),
                    Field::new("material", "Material", FieldKind::Text),
                    Field::new("driver", "Template Surface", FieldKind::Item),
                    check("useWorldXfrm", "Use World Transform"),
                    Field::new("selectEdges", "selectEdges", FieldKind::Hidden),
                    Field::new("selectionSetName", "selectionSetName", FieldKind::Hidden),
                    Field::new("frontOnly", "frontOnly", FieldKind::Hidden),
                ],
            ),
            expand: boolean_links,
            created: |values, picked| {
                if let Some(&driver) = picked.first() {
                    values["driver"] = driver.into();
                }
            },
        }),
        Arc::new(ExpandLinks::<MergeMeshesOp> {
            inner: Typed::new(MERGE_MESHES_OP, "Merge Meshes", "Edit", false, merge_fields),
            expand: merge_links,
            // MM-20: it starts with no sources.
            created: |_, _| {},
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::index_picks;

    #[test]
    fn index_reads_every_form() {
        assert_eq!(index_picks("", 3).unwrap(), [0, 1, 2]);
        assert_eq!(index_picks("...", 3).unwrap(), [0, 1, 2]);
        assert_eq!(index_picks("1", 3).unwrap(), [1]);
        assert_eq!(index_picks("1...", 3).unwrap(), [1, 2]);
        assert_eq!(index_picks("0,2", 3).unwrap(), [0, 2]);
        assert_eq!(index_picks("1-3", 5).unwrap(), [1, 2, 3]);
        assert_eq!(index_picks("7", 3).unwrap(), Vec::<usize>::new());
        assert!(index_picks("x", 3).is_err());
        assert!(index_picks("3-1", 3).is_err());
    }
}
