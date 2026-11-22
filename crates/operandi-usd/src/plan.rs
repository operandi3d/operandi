//! What a USD export writes, worked out in plain Rust before anything
//! touches OpenUSD, so it can be tested without the bridge.
//!
//! The layout follows Blender's exporter: one `/root` Xform holds the
//! scene; each item is an Xform with its own translate / rotate / scale
//! and its children below it; an item's polygons are a Mesh prim inside
//! its Xform. A mesh that mixes plain polygons with Sub-D or Catmull-Clark
//! polygons becomes two Mesh prims, because USD's subdivision scheme is
//! per mesh, not per polygon. Its line elements (polylines and curves)
//! are BasisCurves and NurbsCurves prims beside the meshes (curves.rs).
use crate::lights::{CameraPrim, LightPrim};
use crate::materials::MaterialPrim;
use operandi_core::interchange::IoOptions;
use operandi_core::material::DEFAULT_MATERIAL;
use operandi_core::subdivision::{self, BoundaryRule};
use operandi_core::{Item, Mesh, PolygonRepresentation, Result, Scene};
use std::collections::{BTreeMap, BTreeSet};

/// Rotation orders, named like USD's rotate ops (`ZYX` turns about Z
/// first, then Y, then X). Values match `OU_ROTATE_*` in operandi_usd.h.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum RotateOrder {
    Xyz = 0,
    Xzy = 1,
    Yxz = 2,
    Yzx = 3,
    Zxy = 4,
    Zyx = 5,
}

/// USD's interpolateBoundary (`OU_BOUNDARY_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Boundary {
    EdgeAndCorner = 0,
    EdgeOnly = 1,
}

/// USD's faceVaryingLinearInterpolation (`OU_FACE_VARYING_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum UvInterpolation {
    CornersPlus1 = 0,
    None = 1,
    Boundaries = 2,
    All = 3,
}

/// The whole export.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// 'Y' or 'Z': the stage's upAxis.
    pub up_axis: char,
    /// A matrix op on `/root` (row-major, USD's row-vector convention),
    /// for an X-up scene, which USD can't name: it turns the scene Y-up.
    pub root_matrix: Option<[f64; 16]>,
    /// The scene's top-level items, under `/root`.
    pub prims: Vec<XformPrim>,
    /// Mesh items with polygons or curves written.
    pub items: usize,
    pub polygons: usize,
    /// Polylines and curves written.
    pub curves: usize,
    /// UsdPreviewSurface materials under `/root/_materials` (stage 2).
    pub materials: Vec<MaterialPrim>,
    /// The environment as a DomeLight under `/root/Environment`, with the
    /// Xform rotation (degrees, XYZ) that turns its +Y pole to the scene's
    /// up (stage 3).
    pub dome: Option<(LightPrim, [f64; 3])>,
    /// Mesh Instance prototypes under `/root/_prototypes` (a class, so not
    /// drawn): each source item's meshes, which it and its instances
    /// reference ([`XformPrim::instance`]).
    pub prototypes: Vec<XformPrim>,
    /// Scatters of top-level Mesh Instances (point_instancer.rs).
    pub instancers: Vec<crate::point_instancer::InstancerPrim>,
    /// The Render item as UsdRender settings (render_usd.rs).
    pub render: crate::render_usd::RenderPlan,
    /// The stage's frame rate and time range when animation is written
    /// (anim_usd.rs, TL-80).
    pub time: Option<crate::anim_usd::StageTime>,
    pub notes: Vec<String>,
}

/// An item showing a prototype's meshes instead of its own: a child Xform
/// (no transform) that references the prototype and is instanceable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instance {
    /// Index into [`Plan::prototypes`].
    pub prototype: usize,
    /// False when the item's own eye is off but its children stay shown.
    pub visible: bool,
}

/// One item.
#[derive(Clone, Debug, PartialEq)]
pub struct XformPrim {
    pub name: String,
    pub translate: [f64; 3],
    /// Degrees.
    pub rotate: [f64; 3],
    pub order: RotateOrder,
    pub scale: [f64; 3],
    /// False hides the item and everything below it (its eye is off with
    /// Auto Hide Child Items).
    pub visible: bool,
    pub meshes: Vec<MeshPrim>,
    /// The item's polylines and curves (curves.rs).
    pub curves: Vec<crate::curves::CurvesPrim>,
    /// Set for a Mesh Instance and its source: the meshes come from a
    /// prototype (and `meshes` is empty).
    pub instance: Option<Instance>,
    /// A camera item's camera (stage 3).
    pub camera: Option<CameraPrim>,
    /// A light item's light (stage 3).
    pub light: Option<LightPrim>,
    pub children: Vec<XformPrim>,
    /// Scatters of child Mesh Instances (point_instancer.rs).
    pub instancers: Vec<crate::point_instancer::InstancerPrim>,
    /// Time samples of its animated transform, camera or light values
    /// (anim_usd.rs, TL-81).
    pub anim: Option<Box<crate::anim_usd::Samples>>,
}

/// One USD mesh: an item's plain polygons, or its subdivision polygons.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshPrim {
    pub name: String,
    pub points: Vec<[f32; 3]>,
    pub face_counts: Vec<i32>,
    pub face_indices: Vec<i32>,
    /// One per face corner, in the item's own space; plain polygons only.
    pub normals: Option<Vec<[f32; 3]>>,
    pub subdivision: bool,
    pub boundary: Option<Boundary>,
    pub uv_interpolation: Option<UvInterpolation>,
    /// Creased edges (point pairs) and their USD sharpness.
    pub creases: Vec<[i32; 2]>,
    pub crease_sharpness: Vec<f32>,
    /// Name and one UV per face corner.
    pub uv_sets: Vec<(String, Vec<[f32; 2]>)>,
    /// The material bound to the whole mesh (index into Plan::materials).
    pub material: Option<usize>,
    /// Material-tag subsets.
    pub subsets: Vec<Subset>,
    /// False when the item's own eye is off but its children stay shown.
    pub visible: bool,
    /// Maps, morphs and sets (plan_extras.rs).
    pub extras: crate::plan_extras::MeshExtras,
}

/// A material tag's polygons in one mesh prim.
#[derive(Clone, Debug, PartialEq)]
pub struct Subset {
    pub name: String,
    /// Face indices in this prim.
    pub faces: Vec<i32>,
    /// The material bound to them (index into Plan::materials).
    pub material: Option<usize>,
}

/// The UV map other programs look for first: USD's convention, which
/// Blender follows too, is "st".
pub const PRIMARY_UV_NAME: &str = "st";

/// Works out what `scene` exports as.
pub fn plan(scene: &Scene, options: &IoOptions) -> Result<Plan> {
    let mut plan = Plan {
        up_axis: if options.up_axis == 2 { 'Z' } else { 'Y' },
        root_matrix: (options.up_axis == 0).then(x_up_root_matrix),
        prims: Vec::new(),
        items: 0,
        polygons: 0,
        curves: 0,
        materials: Vec::new(),
        dome: None,
        prototypes: Vec::new(),
        instancers: Vec::new(),
        render: crate::render_usd::plan(scene),
        time: crate::anim_usd::stage_time(scene, options.usd_animation),
        notes: Vec::new(),
    };
    let ids: BTreeSet<u64> = scene.items.iter().map(|i| i.id).collect();
    let mut children: BTreeMap<u64, Vec<&Item>> = BTreeMap::new();
    let mut roots = Vec::new();
    for item in &scene.items {
        match item.parent.filter(|p| ids.contains(p)) {
            Some(parent) => children.entry(parent).or_default().push(item),
            None => roots.push(item),
        }
    }
    let mut skipped = Skipped::default();
    let mut notes = Notes::new(scene);
    notes.frames = crate::anim_usd::frames(scene, options.usd_animation);
    notes.instances = instance_groups(scene);
    notes.mesh_lights = crate::mesh_lights::carried(scene, &mut notes.lights);
    let (scattered, instancers) =
        crate::point_instancer::scatters(scene, &roots, &children, &mut plan, &mut notes);
    plan.instancers = instancers;
    for root in roots.into_iter().filter(|r| !scattered.contains(&r.id)) {
        if let Some(prim) = item_prim(
            scene,
            root,
            &children,
            &mut plan,
            &mut skipped,
            &mut notes,
            0,
        )? {
            plan.prims.push(prim);
        }
    }
    plan.materials = std::mem::take(&mut notes.materials.prims);
    let surfaces = std::mem::take(&mut notes.materials.surfaces);
    // U6: each Material Group's MaterialX network: its graph layer's
    // graph (MaterialX as it is, a Blender graph as the MaterialX nodes it
    // maps to), or its Material and Image Map layers (MX-05) when that
    // leaves nothing out (it reads back as the same layers), else the tree
    // surface's standard MaterialX form with the group's free nodes
    // (stage 3, node editing), else MX-05 anyway.
    let mut no_mtlx = Vec::new();
    let mut partial = Vec::new();
    let mut simplified = Vec::new();
    for (i, m) in plan.materials.iter_mut().enumerate() {
        let tag = if m.tag.is_empty() {
            DEFAULT_MATERIAL
        } else {
            m.tag.as_str()
        };
        match crate::materialx_out::graph_network(scene, tag) {
            Ok(Some((net, left_out))) => {
                m.mtlx = Some(net);
                if !left_out.is_empty() {
                    partial.push(format!("{} ({})", m.name, left_out.join(", ")));
                }
                continue;
            }
            Ok(None) => {}
            Err(e) => {
                no_mtlx.push(format!("{} ({e})", m.name));
                continue;
            }
        }
        let free = crate::materialx_out::group_for(scene, tag)
            .and_then(|g| scene.shader_tree.group_nodes(g));
        let (simple, losses) = match crate::materialx_out::network(scene, tag) {
            Ok(Some(n)) => n,
            Ok(None) => continue,
            Err(e) => {
                no_mtlx.push(format!("{} ({e})", m.name));
                continue;
            }
        };
        if losses.is_empty() {
            m.mtlx = Some(simple);
            continue;
        }
        let tree = surfaces
            .get(i)
            .ok_or_else(|| "no surface".to_string())
            .and_then(|s| crate::materialx_out::tree_network(scene, s, &m.name, free));
        match tree {
            Ok((n, left_out)) => {
                m.mtlx = Some(n);
                if !left_out.is_empty() {
                    partial.push(format!("{} ({})", m.name, left_out.join(", ")));
                }
            }
            Err(why) => {
                m.mtlx = Some(simple);
                simplified.push(format!("{} ({why})", m.name));
            }
        }
    }
    if !no_mtlx.is_empty() {
        plan.notes.push(format!(
            "MaterialX networks left out (Preview Surface only): {}",
            no_mtlx.join(", ")
        ));
    }
    if !simplified.is_empty() {
        plan.notes.push(format!(
            "MaterialX networks with the materials' constants and images only: {}",
            simplified.join(", ")
        ));
    }
    if !partial.is_empty() {
        plan.notes.push(format!(
            "MaterialX networks without some parts: {}",
            partial.join("; ")
        ));
    }
    // The dome's +Y pole turned to the scene's up axis (its frame under
    // /root, which already turns an X-up scene).
    let (turn, rotate) = match options.up_axis {
        2 => (
            glam::DMat3::from_rotation_x(90f64.to_radians()),
            [90.0, 0.0, 0.0],
        ),
        0 => (
            glam::DMat3::from_rotation_z(-90f64.to_radians()),
            [0.0, 0.0, -90.0],
        ),
        _ => (glam::DMat3::IDENTITY, [0.0; 3]),
    };
    plan.dome = crate::lights::environment_dome(scene, turn).map(|d| (d, rotate));
    if plan.dome.is_some()
        && scene
            .environment_for(operandi_core::environment::RayKind::Indirect)
            .and_then(|i| i.environment.as_ref())
            .is_some_and(|e| !e.layers.is_empty())
    {
        plan.notes
            .push("The environment's image layers are baked into a 256 × 128 dome picture".into());
    }
    plan.notes.extend(notes.into_lines());
    plan.notes.extend(skipped.into_lines());
    Ok(plan)
}

/// Turns an X-up scene into USD's Y-up (the inverse of interchange's
/// import turn), as a row-vector matrix.
fn x_up_root_matrix() -> [f64; 16] {
    // interchange::IoOptions::file_to_scene for X up is the matrix with
    // columns (-Y, X, Z); the scene-to-file turn is its transpose. In USD's
    // row-vector convention a column-vector matrix A is written as A^T,
    // so the row-major entries are A's column-major ones.
    let file_to_scene = glam::DMat3::from_cols(-glam::DVec3::Y, glam::DVec3::X, glam::DVec3::Z);
    let scene_to_file = glam::DMat4::from_mat3(file_to_scene.transpose());
    scene_to_file.to_cols_array()
}

const MAX_DEPTH: usize = 10_000;

fn item_prim(
    scene: &Scene,
    item: &Item,
    children: &BTreeMap<u64, Vec<&Item>>,
    plan: &mut Plan,
    skipped: &mut Skipped,
    notes: &mut Notes<'_>,
    depth: usize,
) -> Result<Option<XformPrim>> {
    if depth > MAX_DEPTH {
        return Err("The item hierarchy is too deep to export".into());
    }
    let siblings: &[&Item] = children.get(&item.id).map_or(&[], Vec::as_slice);
    let (scattered, instancers) =
        crate::point_instancer::scatters(scene, siblings, children, plan, notes);
    let mut kids = Vec::new();
    for child in siblings.iter().filter(|c| !scattered.contains(&c.id)) {
        if let Some(prim) = item_prim(scene, child, children, plan, skipped, notes, depth + 1)? {
            kids.push(prim);
        }
    }
    let is_mesh = item.is_mesh();
    // Cameras and lights become USD cameras and lights (stage 3).
    let camera = item.camera.as_ref().map(|c| crate::lights::CameraPrim {
        render: scene.render_camera() == Some(item.id),
        ..crate::lights::camera_prim(scene, item.id, c, &mut notes.lights)
    });
    let light = item
        .light
        .as_ref()
        .and_then(|l| crate::lights::light_prim(l, &mut notes.lights));
    // A mesh light its mesh carries needs nothing of its own.
    let carried = item
        .light
        .as_ref()
        .and_then(|l| l.prototype)
        .and_then(|p| notes.mesh_lights.get(&p))
        .is_some_and(|c| c.name == item.name);
    if carried && kids.is_empty() && instancers.is_empty() {
        return Ok(None);
    }
    // Items USD can't show stay only as a plain transform, and only when
    // they hold exported items, so the hierarchy survives.
    if !is_mesh
        && !item.is_group_locator()
        && camera.is_none()
        && light.is_none()
        && kids.is_empty()
        && instancers.is_empty()
    {
        skipped.count(item);
        return Ok(None);
    }
    let mut instance = None;
    let mut curves = Vec::new();
    let meshes = if is_mesh {
        // A Mesh Instance (or its source) shows its prototype's meshes;
        // the counts are per item shown either way.
        let meshes = match instance_prototype(scene, item, plan, notes) {
            Some(k) => {
                instance = Some(Instance {
                    prototype: k,
                    visible: true,
                });
                Vec::new()
            }
            None => {
                let (mut m, c) = mesh_prims(scene, item, notes);
                curves = c;
                if let Some(c) = notes.mesh_lights.get(&item.id) {
                    for mesh in &mut m {
                        mesh.extras.light = Some(c.light.clone());
                        mesh.extras
                            .custom
                            .insert("meshLight".into(), c.name.clone().into());
                    }
                }
                m
            }
        };
        let (shown, shown_curves) = match instance {
            Some(i) => (
                &plan.prototypes[i.prototype].meshes,
                &plan.prototypes[i.prototype].curves,
            ),
            None => (&meshes, &curves),
        };
        let lines: usize = shown_curves.iter().map(|c| c.counts.len()).sum();
        if shown.iter().any(|m| !m.face_counts.is_empty()) || lines > 0 {
            plan.polygons += shown.iter().map(|m| m.face_counts.len()).sum::<usize>();
            plan.curves += lines;
            plan.items += 1;
        }
        meshes
    } else {
        Vec::new()
    };
    let t = &item.transform;
    // Operandi's rotation orders (scene.rs) are named the way USD names
    // them, the axis that turns first first (XYZ turns X first since
    // native v44, ETL-Q01).
    use operandi_core::scene::RotationOrder as R;
    let order = match t.rotation_order() {
        R::Xyz => RotateOrder::Xyz,
        R::Zyx => RotateOrder::Zyx,
        R::Zxy => RotateOrder::Zxy,
        R::Xzy => RotateOrder::Xzy,
        R::Yxz => RotateOrder::Yxz,
        R::Yzx => RotateOrder::Yzx,
    };
    // An item whose eye is off hides itself; with Auto Hide Child Items it
    // hides its children too (GV-01), which is what USD's inherited
    // visibility does on the Xform.
    let hides_all = !item.visible && item.hides_children;
    let mut meshes = meshes;
    if !item.visible && !item.hides_children {
        for m in &mut meshes {
            m.visible = false;
        }
        if let Some(i) = &mut instance {
            i.visible = false;
        }
    }
    Ok(Some(XformPrim {
        name: item.name.clone(),
        translate: t.translation.to_array(),
        rotate: t.rotation.to_array().map(f64::to_degrees),
        order,
        scale: t.scale.to_array(),
        visible: !hides_all,
        meshes,
        curves,
        instance,
        camera,
        light,
        children: kids,
        instancers,
        anim: notes
            .frames
            .as_ref()
            .and_then(|f| crate::anim_usd::item_samples(f, scene, item.id)),
    }))
}

/// Items left out, by kind, for the export report.
#[derive(Default)]
struct Skipped {
    backdrops: usize,
    locators: usize,
    kits: BTreeMap<String, usize>,
}
impl Skipped {
    fn count(&mut self, item: &Item) {
        if item.backdrop.is_some() {
            self.backdrops += 1;
        } else if let Some(kit) = &item.kit {
            *self.kits.entry(kit.kind.clone()).or_default() += 1;
        } else if item.locator.is_some() {
            self.locators += 1;
        }
        // Environments become the dome light; Texture Locators live in
        // the materials.
    }
    fn into_lines(self) -> Vec<String> {
        let mut out = Vec::new();
        if self.backdrops > 0 {
            out.push(format!("{} Backdrop(s) not exported", self.backdrops));
        }
        if self.locators > 0 {
            out.push(format!(
                "{} Locator(s) with nothing exported below them not exported",
                self.locators
            ));
        }
        for (kind, n) in self.kits {
            out.push(format!("{n} {kind} kit item(s) not exported"));
        }
        out
    }
}

/// Mesh Instances and their sources, by item id: the source each one shows
/// (a source maps to itself).
type InstanceGroups = BTreeMap<u64, u64>;

fn instance_groups(scene: &Scene) -> InstanceGroups {
    let mut out = BTreeMap::new();
    for item in scene.items.iter().filter(|i| i.instance_of.is_some()) {
        let Some(source) = scene.instance_source(item.id) else {
            continue;
        };
        if item.is_mesh() && scene.item(source).is_some_and(Item::is_mesh) {
            out.insert(item.id, source);
            out.insert(source, source);
        }
    }
    out
}

/// The prototype `item` shows, made the first time its group is met:
/// `None` for an item that isn't in an instance group, or an instance
/// that can't share its source's prototype (other materials or
/// subdivision settings), which is written as a copy.
pub(crate) fn instance_prototype(
    scene: &Scene,
    item: &Item,
    plan: &mut Plan,
    notes: &mut Notes<'_>,
) -> Option<usize> {
    let &source_id = notes.instances.get(&item.id)?;
    let source = scene.item(source_id)?;
    if source_id != item.id
        && (item.subdivision != source.subdivision
            || notes.materials.per_face(scene, item) != notes.materials.per_face(scene, source))
    {
        notes.instance_copies += 1;
        return None;
    }
    if let Some(&k) = notes.prototype_of.get(&source_id) {
        return Some(k);
    }
    let (meshes, curves) = mesh_prims(scene, source, notes);
    plan.prototypes.push(XformPrim {
        name: source.name.clone(),
        translate: [0.0; 3],
        rotate: [0.0; 3],
        order: RotateOrder::Xyz,
        scale: [1.0; 3],
        visible: true,
        meshes,
        curves,
        instance: None,
        camera: None,
        light: None,
        children: Vec::new(),
        instancers: Vec::new(),
        anim: None,
    });
    let k = plan.prototypes.len() - 1;
    notes.prototype_of.insert(source_id, k);
    Some(k)
}

/// Things worth one line each in the report, however often they happen,
/// and the materials gathered on the way.
pub(crate) struct Notes<'s> {
    pub(crate) materials: crate::materials::Collector<'s>,
    lights: Vec<String>,
    polylines: usize,
    /// Mesh Instances written as points of a PointInstancer.
    pub(crate) scattered: usize,
    sub_d: bool,
    negative_weights: bool,
    missing_uvs: BTreeSet<String>,
    legacy_uvs_dropped: bool,
    merged_tags: bool,
    pub(crate) instances: InstanceGroups,
    /// Each instance group's prototype, by source item id.
    prototype_of: BTreeMap<u64, usize>,
    instance_copies: usize,
    /// Meshes with data USD gets nothing of (one line each).
    set_groups: usize,
    smoothing_labels: usize,
    sketches: usize,
    extra_normal_maps: usize,
    /// Mesh lights their prototype's mesh carries (mesh_lights.rs).
    mesh_lights: BTreeMap<u64, crate::mesh_lights::Carried>,
    /// The scene posed at each animation sample time (anim_usd.rs).
    frames: Option<crate::anim_usd::Frames>,
}
impl<'s> Notes<'s> {
    fn new(scene: &'s Scene) -> Self {
        Self {
            materials: crate::materials::Collector::new(scene),
            mesh_lights: BTreeMap::new(),
            frames: None,
            lights: Vec::new(),
            polylines: 0,
            scattered: 0,
            sub_d: false,
            negative_weights: false,
            missing_uvs: BTreeSet::new(),
            legacy_uvs_dropped: false,
            merged_tags: false,
            instances: BTreeMap::new(),
            prototype_of: BTreeMap::new(),
            instance_copies: 0,
            set_groups: 0,
            smoothing_labels: 0,
            sketches: 0,
            extra_normal_maps: 0,
        }
    }
    fn into_lines(self) -> Vec<String> {
        let mut out = self.materials.notes.into_lines();
        let mut lights = self.lights;
        lights.sort();
        lights.dedup();
        out.extend(lights);
        if self.polylines > 0 {
            out.push(format!(
                "{} polyline(s) and curve(s) with too few points not exported",
                self.polylines
            ));
        }
        if self.scattered > 0 {
            out.push(format!(
                "{} Mesh Instance(s) written as PointInstancer points",
                self.scattered
            ));
        }
        if self.sub_d {
            out.push("Sub-D polygons are written as Catmull-Clark surfaces".into());
        }
        if self.negative_weights {
            out.push("Negative edge weights have no USD equivalent and were left out".into());
        }
        if !self.missing_uvs.is_empty() {
            let names: Vec<&str> = self.missing_uvs.iter().map(String::as_str).collect();
            out.push(format!(
                "Some polygons have no UVs in {}; they get 0, 0",
                names.join(", ")
            ));
        }
        if self.merged_tags {
            out.push(
                "Parts or selection sets that shade one material tag differently share its first material"
                    .into(),
            );
        }
        if self.legacy_uvs_dropped {
            out.push("Imported per-polygon UVs not exported beside the UV maps".into());
        }
        if self.instance_copies > 0 {
            out.push(format!(
                "{} Mesh Instance(s) shading or subdividing differently from their source written as copies",
                self.instance_copies
            ));
        }
        if self.set_groups > 0 {
            out.push(format!(
                "Selection set groups not exported ({} mesh(es)); their sets are",
                self.set_groups
            ));
        }
        if self.smoothing_labels > 0 {
            out.push(format!(
                "Smoothing group names not exported ({} mesh(es)); the normals they make are",
                self.smoothing_labels
            ));
        }
        if self.sketches > 0 {
            out.push(format!(
                "Solid Sketch nodes not exported ({} mesh(es)); the polygons are",
                self.sketches
            ));
        }
        if self.extra_normal_maps > 0 {
            out.push(format!(
                "Only the shading normals were exported, not every normal map ({} mesh(es))",
                self.extra_normal_maps
            ));
        }
        out
    }
}

/// The Mesh prims of one mesh item, and its curves prims.
fn mesh_prims(
    scene: &Scene,
    item: &Item,
    notes: &mut Notes,
) -> (Vec<MeshPrim>, Vec<crate::curves::CurvesPrim>) {
    let mesh: &Mesh = &item.mesh;
    let a = &mesh.attributes;
    notes.set_groups += usize::from(!a.set_groups.is_empty());
    notes.smoothing_labels += usize::from(a.smoothing_labels.iter().any(Option::is_some));
    notes.sketches += usize::from(a.solid_sketch.is_some());
    notes.extra_normal_maps += usize::from(a.corner_normals.len() > 1);
    let mut flat = Vec::new();
    let mut smooth = Vec::new();
    for f in 0..mesh.faces.len() {
        if mesh.is_polyline(f) {
            continue;
        } else if subdivision::is_smooth_polygon(mesh, f) {
            if matches!(
                mesh.attributes.representations.get(f),
                Some(PolygonRepresentation::Subdivs)
            ) {
                notes.sub_d = true;
            }
            smooth.push(f);
        } else {
            flat.push(f);
        }
    }
    let uv_names = uv_map_names(mesh);
    let face_materials = notes.materials.per_face(scene, item);
    let curves = if mesh.has_polylines() {
        crate::curves::curves_prims(
            &item.name,
            mesh,
            !flat.is_empty() || !smooth.is_empty(),
            &face_materials,
            &mut notes.polylines,
        )
    } else {
        Vec::new()
    };
    let whole = flat.is_empty() || smooth.is_empty();
    let mut out = Vec::new();
    if !flat.is_empty() {
        let normals = (!flat.is_empty())
            .then(|| operandi_core::shading::mesh_corner_normals(mesh, &scene.smoothing));
        let mut prim = mesh_prim(
            item.name.clone(),
            mesh,
            &flat,
            whole,
            &uv_names,
            &face_materials,
            notes,
        );
        // Creases on plain polygons change nothing here or in USD, but the
        // file keeps them (a Blender mesh exported with its subdivision
        // applied still has its creases; U6 round trip).
        add_creases(&mut prim, mesh, &flat, whole, notes);
        if let Some(normals) = normals {
            prim.normals = Some(
                flat.iter()
                    .flat_map(|&f| normals[f].iter().map(|n| n.as_vec3().to_array()))
                    .collect(),
            );
        }
        out.push(prim);
    }
    if !smooth.is_empty() {
        let name = if whole {
            item.name.clone()
        } else {
            format!("{}_subd", item.name)
        };
        let mut prim = mesh_prim(
            name,
            mesh,
            &smooth,
            whole,
            &uv_names,
            &face_materials,
            notes,
        );
        let settings = item.subdivision.clone().unwrap_or_default();
        prim.subdivision = true;
        // SDV-06: Crease keeps boundary corners sharp; Smooth and Crease
        // Edges smooth them (provisional mapping, USD-Q01).
        prim.boundary = Some(match settings.boundary_rule {
            BoundaryRule::Crease => Boundary::EdgeAndCorner,
            BoundaryRule::Smooth | BoundaryRule::CreaseEdges => Boundary::EdgeOnly,
        });
        prim.uv_interpolation = Some(if settings.linear_uvs {
            UvInterpolation::All
        } else {
            match settings.uv_boundary_rule {
                BoundaryRule::Crease => UvInterpolation::CornersPlus1,
                BoundaryRule::Smooth => UvInterpolation::None,
                BoundaryRule::CreaseEdges => UvInterpolation::Boundaries,
            }
        });
        add_creases(&mut prim, mesh, &smooth, whole, notes);
        out.push(prim);
    }
    (out, curves)
}

/// UV map names in export order: the primary map first (written as "st"),
/// then the rest by name.
fn uv_map_names(mesh: &Mesh) -> Vec<String> {
    let mut names: BTreeSet<&String> = mesh.uv_maps.keys().collect();
    names.extend(mesh.attributes.point_uvs.keys());
    let mut out: Vec<String> = Vec::new();
    if names.contains(&operandi_core::PRIMITIVE_UV_MAP.to_string()) {
        out.push(operandi_core::PRIMITIVE_UV_MAP.to_string());
    }
    out.extend(
        names
            .into_iter()
            .filter(|n| n.as_str() != operandi_core::PRIMITIVE_UV_MAP)
            .cloned(),
    );
    out
}

/// The point remapping of `faces`: all points in order when `whole`, else
/// only the points those faces use.
fn point_map(mesh: &Mesh, faces: &[usize], whole: bool) -> (Vec<u32>, Vec<Option<u32>>) {
    if whole {
        let all: Vec<u32> = (0..mesh.vertices.len() as u32).collect();
        let map = all.iter().map(|&p| Some(p)).collect();
        return (all, map);
    }
    let mut map = vec![None; mesh.vertices.len()];
    let mut kept = Vec::new();
    for &f in faces {
        for &v in &mesh.faces[f].vertices {
            let slot = &mut map[v as usize];
            if slot.is_none() {
                *slot = Some(kept.len() as u32);
                kept.push(v);
            }
        }
    }
    (kept, map)
}

fn mesh_prim(
    name: String,
    mesh: &Mesh,
    faces: &[usize],
    whole: bool,
    uv_names: &[String],
    face_materials: &[usize],
    notes: &mut Notes<'_>,
) -> MeshPrim {
    let (kept, map) = point_map(mesh, faces, whole);
    let mut prim = MeshPrim {
        name,
        points: kept
            .iter()
            .map(|&p| mesh.vertices[p as usize].as_vec3().to_array())
            .collect(),
        visible: true,
        ..Default::default()
    };
    for &f in faces {
        let face = &mesh.faces[f];
        prim.face_counts.push(face.vertices.len() as i32);
        prim.face_indices.extend(
            face.vertices
                .iter()
                .map(|&v| map[v as usize].expect("face point is mapped") as i32),
        );
    }
    // UV maps; the primary one is "st".
    for (i, map_name) in uv_names.iter().enumerate() {
        let mut values = Vec::with_capacity(prim.face_indices.len());
        let mut any = false;
        let mut missing = false;
        for &f in faces {
            for c in 0..mesh.faces[f].vertices.len() {
                match operandi_core::extrude::effective_uv(mesh, map_name, f, c) {
                    Some(uv) => {
                        any = true;
                        values.push(uv.as_vec2().to_array());
                    }
                    None => {
                        missing = true;
                        values.push([0.0, 0.0]);
                    }
                }
            }
        }
        if !any {
            continue;
        }
        if missing {
            notes.missing_uvs.insert(map_name.clone());
        }
        let out_name = if i == 0 {
            PRIMARY_UV_NAME.to_string()
        } else {
            map_name.clone()
        };
        prim.uv_sets.push((out_name, values));
    }
    // Imported polygons can carry their own UVs (Face::uv) instead of a map.
    let legacy = faces.iter().any(|&f| mesh.faces[f].uv.is_some());
    if legacy && prim.uv_sets.is_empty() {
        let mut values = Vec::with_capacity(prim.face_indices.len());
        for &f in faces {
            let face = &mesh.faces[f];
            for c in 0..face.vertices.len() {
                let uv = face.uv.as_ref().and_then(|uv| uv.get(c)).copied();
                values.push(uv.unwrap_or_default().as_vec2().to_array());
            }
        }
        prim.uv_sets.push((PRIMARY_UV_NAME.to_string(), values));
    } else if legacy {
        notes.legacy_uvs_dropped = true;
    }
    // Weight, typed and colour maps, morphs, parts and sets.
    let taken: Vec<String> = prim.uv_sets.iter().map(|(n, _)| n.clone()).collect();
    prim.extras = crate::plan_extras::extras(
        &crate::plan_extras::PrimSlice {
            mesh,
            faces,
            points: &kept,
            point_map: &map,
        },
        &taken,
    );
    // The material bound to the whole prim: its first polygon's (USD's
    // viewers bind per mesh first; Blender does the same).
    prim.material = faces.first().and_then(|&f| face_materials.get(f)).copied();
    // Material tags as geometry subsets, when any polygon has one or the
    // prim's polygons don't all share one material. An empty tag is the
    // Default material. A tag's subset takes its first polygon's material
    // (parts or selection sets shading one tag differently merge there).
    let mixed = faces
        .iter()
        .any(|&f| face_materials.get(f).copied() != prim.material);
    if mixed || faces.iter().any(|&f| !mesh.faces[f].material.is_empty()) {
        let mut tags: BTreeMap<&str, (Vec<i32>, Option<usize>)> = BTreeMap::new();
        for (i, &f) in faces.iter().enumerate() {
            let tag = mesh.faces[f].material.as_str();
            let tag = if tag.is_empty() {
                DEFAULT_MATERIAL
            } else {
                tag
            };
            let entry = tags
                .entry(tag)
                .or_insert_with(|| (Vec::new(), face_materials.get(f).copied()));
            if entry.1 != face_materials.get(f).copied() {
                notes.merged_tags = true;
            }
            entry.0.push(i as i32);
        }
        prim.subsets = tags
            .into_iter()
            .map(|(tag, (faces, material))| Subset {
                name: tag.to_string(),
                faces,
                material,
            })
            .collect();
    }
    prim
}

fn add_creases(prim: &mut MeshPrim, mesh: &Mesh, faces: &[usize], whole: bool, notes: &mut Notes) {
    if mesh.attributes.creases.is_empty() {
        return;
    }
    let (_, map) = point_map(mesh, faces, whole);
    for crease in &mesh.attributes.creases {
        if crease.weight < 0.0 {
            notes.negative_weights = true;
            continue;
        }
        if crease.weight == 0.0 || !crease.weight.is_finite() {
            continue;
        }
        let (Some(Some(a)), Some(Some(b))) = (
            map.get(crease.a as usize).copied(),
            map.get(crease.b as usize).copied(),
        ) else {
            continue;
        };
        prim.creases.push([a as i32, b as i32]);
        // Operandi's semi-sharp creases (SDV-07): sharpness = 10 x weight.
        prim.crease_sharpness.push((crease.weight * 10.0) as f32);
    }
}
