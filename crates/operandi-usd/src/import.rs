//! USD import, stages U1 and U2 of docs/BEHAVIOR-USD-IMPORT.md: a stage
//! described by the bridge ([`StageData`]) becomes a new scene.
//!
//! - Each Xform keeps its local transform and becomes an item under its
//!   parent; an Xform holding Mesh prims with no transform of their own
//!   (the way Blender and Operandi write an object) is one mesh item.
//!   Other transforms become Group Locators. Prims holding nothing
//!   importable are left out, and so is a top `/root`-style Xform with no
//!   transform of its own.
//! - The stage's upAxis and metersPerUnit (or the options' overrides) are
//!   converted: points and translations are scaled to metres, and points,
//!   normals and every transform are turned to the scene's up axis.
//! - Meshes keep their polygons (quads and n-gons), all UV primvars, colour
//!   primvars (as colour maps), normals (as a normal map, plus each
//!   polygon's smoothing), creases and the subdivision scheme.
//! - Native instances and PointInstancer instances arrive as copies that
//!   share their mesh until one is edited (Operandi has no instance items).
//! - Bound materials (GeomSubsets too) become polygon tags named after the
//!   material, and Material Groups for them (U2, `materials.rs`'s import
//!   half in `import_materials.rs`).
use crate::stage::{MeshData, Prim, Primvar, StageData};
use glam::{DMat3, DMat4, DVec2, DVec3, DVec4};
use operandi_core::interchange::{self, Imported, UsdImportOptions, UsdMaterials};
use operandi_core::scene::{RotationOrder, Transform};
use operandi_core::subdivision::{BoundaryRule, SubdivisionSettings};
use operandi_core::{
    Face, Mesh, MeshCrease, MeshSlot, PolygonRepresentation, Result, Scene, Shared,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

/// What an import needs beyond the stage's data: picture files.
pub trait Assets {
    /// A picture's pixel size, `None` when it can't be read.
    fn image_size(&self, path: &Path) -> Option<(u32, u32)>;
    /// Writes a packaged asset (OpenUSD's ".usdz[inner/name]" path) to
    /// `destination`.
    fn copy_packaged(&self, resolved: &str, destination: &Path) -> Result<()>;
}

/// The name a mesh's normals arrive under.
pub const NORMAL_MAP: &str = "normals";

/// Items the prims may make: the scene's budget less room for what comes
/// after them (environment, texture locators of image layers).
const ITEM_LIMIT: usize = operandi_core::MAX_SCENE_ITEMS - 10_000;

/// Builds the scene `stage` describes. `file` is the USD file (for
/// picture folders and the report).
pub fn build(
    stage: &StageData,
    file: &Path,
    options: &UsdImportOptions,
    assets: &dyn Assets,
) -> Result<Imported> {
    build_within(stage, file, options, assets, ITEM_LIMIT)
}

/// `build`, making at most `item_limit` items from the prims.
pub(crate) fn build_within(
    stage: &StageData,
    file: &Path,
    options: &UsdImportOptions,
    assets: &dyn Assets,
    item_limit: usize,
) -> Result<Imported> {
    let mut b = Builder::new(stage, options)?;
    b.item_limit = item_limit;
    b.add_all()?;
    b.set_render_camera()?;
    let mut render_notes = Vec::new();
    if let Some(r) = &stage.render_settings {
        crate::render_usd::apply(&mut b.scene, r, &mut render_notes);
    }
    if !b
        .scene
        .items
        .iter()
        .any(|i| i.is_mesh() && !i.mesh.faces.is_empty())
    {
        return Err("The file has no polygons to import; current document preserved".into());
    }
    let mut pictures = crate::import_materials::Pictures::new(file, options, assets);
    let mut materials = if options.materials == UsdMaterials::None {
        crate::import_materials::Summary::default()
    } else {
        crate::import_materials::add(
            &mut b.scene,
            stage,
            &b.tags,
            &b.used,
            &mut pictures,
            options,
        )?
    };
    b.add_dome(&mut pictures)?;
    materials.notes.extend(pictures.notes());
    // U4: Blender's own node graphs, from the sidecar, win over the
    // flattened materials.
    if let Some(sidecar) = crate::import_graphs::sidecar(file, options) {
        let tags: BTreeSet<String> = b
            .used
            .iter()
            .filter_map(|&m| b.tags.get(m).cloned())
            .collect();
        materials
            .notes
            .extend(crate::import_graphs::add(&mut b.scene, &sidecar, &tags)?);
    }
    let first = b
        .scene
        .items
        .iter()
        .find(|i| i.parent.is_none() && !i.is_texture_item())
        .map(|i| i.id);
    b.scene.select_item(first);
    b.scene.validate()?;
    let mut notes = b.report(&materials);
    notes.extend(materials.notes);
    notes.extend(render_notes);
    notes.extend(crate::import_variants::notes(stage));
    Ok(Imported {
        scene: b.scene,
        notes,
    })
}

/// Polygon counts for the report.
#[derive(Default)]
struct Counts {
    items: usize,
    triangles: usize,
    quads: usize,
    ngons: usize,
    native_instances: usize,
    /// Items made Mesh Instances of an earlier item.
    instances_made: usize,
    instancer_copies: usize,
    cameras: usize,
    lights: usize,
    domes: usize,
    cameras_in: usize,
    /// Line elements read from curves prims.
    curves: usize,
    lights_in: usize,
    domes_in: usize,
}

/// Things left out or approximated, one report line each.
#[derive(Default)]
struct Dropped {
    small_faces: usize,
    holes: usize,
    corners: usize,
    shear: usize,
    resets: usize,
    guides: Vec<String>,
    constant_colours: BTreeSet<String>,
    /// Other programs' primvars and subsets left out (import_extras.rs).
    foreign: BTreeSet<String>,
    schemes: BTreeSet<String>,
    no_boundary: bool,
    other: BTreeMap<String, usize>,
    broken: Vec<String>,
    normal_maps: usize,
    /// Curves approximated or left out (curves.rs).
    curves: BTreeSet<String>,
    lights: BTreeMap<String, &'static str>,
    domes: Vec<String>,
    /// Prims left out because the scene was full, and the first one's path.
    over_budget: usize,
    first_over_budget: String,
}

/// Mesh data, whole binding and subset bindings of each Mesh prim.
type MeshKey = Vec<(usize, i64, Vec<i64>)>;
type BuiltMesh = (Shared<Mesh>, Option<SubdivisionSettings>);

struct Builder<'a> {
    stage: &'a StageData,
    /// Metres per file unit.
    scale: f64,
    /// File axes to scene axes (a turn, no scale).
    turn: DMat4,
    children: Vec<Vec<usize>>,
    importable: Vec<bool>,
    scene: Scene,
    /// Material index → polygon tag.
    tags: Vec<String>,
    /// Materials some imported polygon uses.
    used: BTreeSet<usize>,
    /// Built meshes by the Mesh prims (and bindings) they came from, so
    /// instances share one.
    meshes: HashMap<MeshKey, BuiltMesh>,
    counts: Counts,
    dropped: Dropped,
    /// Import Cameras / Import Lights / Instances as instances.
    instances: bool,
    /// The first item built from each mesh (instances' sources).
    owners: HashMap<MeshKey, u64>,
    cameras: bool,
    lights: bool,
    /// DomeLight prims, for the environment.
    domes: Vec<usize>,
    /// Camera items by the prim paths they came from (the Camera prim and
    /// the Xform holding it), for the stage's render camera.
    camera_paths: HashMap<String, u64>,
    /// Items the import makes at most; the rest is left out and reported.
    item_limit: usize,
}

/// A PointInstancer Operandi wrote for a scatter, with no transform of
/// its own (USD-IQ25).
fn is_scatter(prim: &Prim) -> bool {
    identity(prim)
        && !prim.resets_xform
        && prim
            .custom
            .as_ref()
            .and_then(|c| c.get("scatter"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
}

fn is_light(ty: &str) -> bool {
    ty.ends_with("Light") || ty == "LightFilter"
}

fn identity(prim: &Prim) -> bool {
    prim.local
        .is_none_or(|m| matrix(&m).abs_diff_eq(DMat4::IDENTITY, 1e-9))
}

/// USD's row-major row-vector matrix as glam's column-vector one (the same
/// numbers read column-major).
fn matrix(m: &[f64; 16]) -> DMat4 {
    DMat4::from_cols_array(m)
}

impl<'a> Builder<'a> {
    fn new(stage: &'a StageData, options: &UsdImportOptions) -> Result<Self> {
        let scale = options
            .meters_per_unit
            .unwrap_or(stage.meters_per_unit)
            .abs();
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let up = options
            .up_axis
            .unwrap_or_else(|| stage.up_axis.chars().next().unwrap_or('Y'));
        // A Z-up file's +Z becomes +Y and its +Y becomes -Z (a quarter
        // turn about X), then the scene's own up axis preference (File
        // I/O) turns it as every importer does.
        let to_y = if up.eq_ignore_ascii_case(&'Z') {
            DMat3::from_cols(DVec3::X, -DVec3::Z, DVec3::Y)
        } else {
            DMat3::IDENTITY
        };
        let turn = DMat4::from_mat3(interchange::io_options().file_to_scene() * to_y);
        let n = stage.prims.len();
        let mut children = vec![Vec::new(); n];
        for (i, p) in stage.prims.iter().enumerate() {
            if p.parent >= 0 {
                let parent = p.parent as usize;
                if parent >= i {
                    return Err("The USD description lists a child before its parent".into());
                }
                children[parent].push(i);
            }
        }
        let mut b = Self {
            stage,
            scale,
            turn,
            children,
            importable: vec![false; n],
            scene: Scene::default(),
            tags: Vec::new(),
            used: BTreeSet::new(),
            meshes: HashMap::new(),
            counts: Counts::default(),
            dropped: Dropped::default(),
            camera_paths: HashMap::new(),
            instances: options.instances,
            owners: HashMap::new(),
            cameras: options.cameras,
            lights: options.lights,
            domes: Vec::new(),
            item_limit: ITEM_LIMIT,
        };
        b.tags = material_tags(stage);
        b.mark_importable();
        Ok(b)
    }

    /// Which prims hold something that comes in, and counts what doesn't.
    fn mark_importable(&mut self) {
        let prims = &self.stage.prims;
        for i in (0..prims.len()).rev() {
            let p = &prims[i];
            let guide = matches!(p.purpose.as_str(), "guide" | "proxy");
            let own = if p.mesh.is_some() {
                if guide {
                    self.dropped
                        .guides
                        .push(format!("{} ({})", p.name, p.purpose));
                }
                !guide
            } else if let Some(inst) = &p.instancer {
                inst.prototypes
                    .iter()
                    .any(|&q| self.importable.get(q) == Some(&true))
            } else if p.camera.is_some() {
                self.cameras
            } else if let Some(light) = &p.light {
                match crate::import_lights::light_settings(&p.ty, light, 1.0) {
                    crate::import_lights::Light::Item(_) => self.lights,
                    crate::import_lights::Light::Dome => {
                        if self.lights && p.proto_of.is_none() {
                            self.domes.push(i);
                        }
                        false
                    }
                    crate::import_lights::Light::Dropped(why) => {
                        if self.lights {
                            self.dropped.lights.insert(p.ty.clone(), why);
                        }
                        false
                    }
                }
            } else {
                false
            };
            let below = self.children[i].iter().any(|&c| self.importable[c]);
            self.importable[i] = own || (below && !guide);
            if p.proto_of.is_some() && p.parent < 0 {
                continue;
            }
            // What isn't geometry or a transform is counted, once.
            if p.ty == "Camera" {
                self.counts.cameras += 1;
            } else if p.ty == "DomeLight" || p.ty == "DomeLight_1" {
                self.counts.domes += 1;
            } else if is_light(&p.ty) {
                self.counts.lights += 1;
            } else if !self.importable[i]
                && p.mesh.is_none()
                && !matches!(p.ty.as_str(), "Xform" | "Scope" | "" | "PointInstancer")
            {
                *self.dropped.other.entry(p.ty.clone()).or_default() += 1;
            }
        }
        self.domes.reverse();
        for s in &self.stage.skipped {
            if s.ty != "Material" && s.ty != "Shader" && s.ty != "NodeGraph" {
                *self.dropped.other.entry(s.ty.clone()).or_default() += 1;
            }
        }
    }

    fn add_all(&mut self) -> Result<()> {
        let prims = &self.stage.prims;
        let mut roots: Vec<usize> = (0..prims.len())
            .filter(|&i| prims[i].parent < 0 && prims[i].proto_of.is_none() && self.importable[i])
            .collect();
        // Blender and Operandi wrap the scene in one "/root" Xform: its
        // children come in at the top instead.
        if let [only] = roots[..] {
            let p = &prims[only];
            if p.mesh.is_none() && p.instancer.is_none() && identity(p) && p.visible {
                roots = self.children[only]
                    .iter()
                    .copied()
                    .filter(|&c| self.importable[c])
                    .collect();
            }
        }
        for r in roots {
            self.add(r, None, None, false)?;
        }
        Ok(())
    }

    /// USD-IQ21 (provisional): the camera the stage's RenderSettings
    /// names becomes the render camera; otherwise the scene keeps none
    /// chosen (its first camera renders).
    fn set_render_camera(&mut self) -> Result<()> {
        if let Some(&id) = self.camera_paths.get(&self.stage.render_camera) {
            self.scene.set_render_camera(id)?;
        }
        Ok(())
    }

    /// The Mesh children an Xform item takes in as its own polygons: no
    /// transform of their own and nothing importable below them.
    fn absorbed(&self, p: usize) -> Vec<usize> {
        let prims = &self.stage.prims;
        if prims[p].mesh.is_some() || prims[p].instancer.is_some() {
            return Vec::new();
        }
        let own: Vec<usize> = self.children[p]
            .iter()
            .copied()
            .filter(|&c| self.absorbable_mesh(c))
            .collect();
        if !own.is_empty() {
            return own;
        }
        // A Mesh Instance as Operandi writes it: the item's one instanceable
        // child Xform, with no transform, referencing the prototype's
        // meshes (USD-IQ22).
        match self.instance_holder(p) {
            Some(h) => self.children[h]
                .iter()
                .copied()
                .filter(|&g| self.importable[g])
                .collect(),
            None => Vec::new(),
        }
    }

    /// A Mesh child an Xform item takes in: no transform of its own and
    /// nothing importable below it.
    fn absorbable_mesh(&self, c: usize) -> bool {
        let child = &self.stage.prims[c];
        self.importable[c]
            && child.mesh.is_some()
            && identity(child)
            && !child.resets_xform
            && self.children[c].iter().all(|&g| !self.importable[g])
    }

    /// The instance holder below `p` that `absorbed` takes the meshes of:
    /// `p`'s only instanceable child Operandi marked as one, an Xform with no transform of its own
    /// holding nothing but meshes `p` can take in, when `p` has no meshes
    /// of its own.
    fn instance_holder(&self, p: usize) -> Option<usize> {
        let prims = &self.stage.prims;
        if prims[p].mesh.is_some() || prims[p].instancer.is_some() {
            return None;
        }
        if self.children[p].iter().any(|&c| self.absorbable_mesh(c)) {
            return None;
        }
        let holders: Vec<usize> = self.children[p]
            .iter()
            .copied()
            .filter(|&c| {
                let h = &prims[c];
                let inside: Vec<usize> = self.children[c]
                    .iter()
                    .copied()
                    .filter(|&g| self.importable[g])
                    .collect();
                let marked = h
                    .custom
                    .as_ref()
                    .and_then(|c| c.get("instanceHolder"))
                    .and_then(serde_json::Value::as_bool)
                    == Some(true);
                self.importable[c]
                    && marked
                    && h.instance
                    && h.ty == "Xform"
                    && h.mesh.is_none()
                    && h.instancer.is_none()
                    && identity(h)
                    && !h.resets_xform
                    && !inside.is_empty()
                    && inside.iter().all(|&g| self.absorbable_mesh(g))
            })
            .collect();
        match holders[..] {
            [h] => Some(h),
            _ => None,
        }
    }

    /// The one camera or light an Xform holds with no transform of its own
    /// and nothing below it, when that's all it holds.
    fn absorbed_lens(&self, p: usize) -> Option<usize> {
        let prims = &self.stage.prims;
        if prims[p].instancer.is_some() {
            return None;
        }
        let kids: Vec<usize> = self.children[p]
            .iter()
            .copied()
            .filter(|&c| self.importable[c])
            .collect();
        let [c] = kids[..] else {
            return None;
        };
        let child = &prims[c];
        ((child.camera.is_some() || child.light.is_some())
            && identity(child)
            && !child.resets_xform
            && self.children[c].iter().all(|&g| !self.importable[g]))
        .then_some(c)
    }

    /// The first DomeLight's picture becomes the environment image, turned
    /// as the dome is (its local +Y is the picture's pole, as OpenUSD's
    /// lat-long convention has it, which is also Operandi's).
    fn add_dome(&mut self, pictures: &mut crate::import_materials::Pictures) -> Result<()> {
        let domes = std::mem::take(&mut self.domes);
        for (k, &d) in domes.iter().enumerate() {
            let prim = &self.stage.prims[d];
            if k > 0 {
                self.dropped
                    .domes
                    .push(format!("{} (only the first dome is used)", prim.name));
                continue;
            }
            let light = prim.light.clone().unwrap_or_default();
            let Some(asset) = light.texture.as_ref() else {
                self.dropped
                    .domes
                    .push(format!("{} (no picture)", prim.name));
                continue;
            };
            if light
                .texture_format
                .as_deref()
                .is_some_and(|f| !matches!(f, "automatic" | "latlong"))
            {
                self.dropped.domes.push(format!(
                    "{} ({} pictures aren't read; read as lat-long)",
                    prim.name,
                    light.texture_format.as_deref().unwrap_or_default()
                ));
            }
            let path = pictures.file(
                asset.value.as_deref().unwrap_or_default(),
                asset.resolved.as_deref().unwrap_or_default(),
                asset.anchored.as_deref().unwrap_or_default(),
            );
            let (w, h) = pictures.size(Path::new(&path)).unwrap_or((1, 1));
            let (env, layer) = self.scene.put_environment_image(&path, w, h, None)?;
            // The dome's world turn in the file, then into the scene.
            let mut world = DMat4::IDENTITY;
            let mut at = Some(d);
            while let Some(i) = at {
                let p = &self.stage.prims[i];
                world = p.local.as_ref().map_or(DMat4::IDENTITY, matrix) * world;
                at = usize::try_from(p.parent).ok();
            }
            let (_, rotation, _) = (self.turn * world).to_scale_rotation_translation();
            let locator = self
                .scene
                .environment_image_layer(env)
                .filter(|n| n.id == layer)
                .and_then(|n| match &n.kind {
                    operandi_core::shader_tree::NodeKind::Texture(t) => t.locator,
                    _ => None,
                });
            if let Some(item) =
                locator.and_then(|id| self.scene.items.iter_mut().find(|i| i.id == id))
            {
                let order = item.transform.rotation_order();
                item.transform.rotation = order.angles(rotation);
            }
            let (colour, gain) = (
                light.color.unwrap_or([1.0; 3]),
                light.intensity.unwrap_or(1.0) * 2f64.powf(light.exposure.unwrap_or(0.0)),
            );
            if let Some(e) = self
                .scene
                .items
                .iter_mut()
                .find(|i| i.id == env)
                .and_then(|i| i.environment.as_mut())
            {
                e.intensity = gain;
            }
            if colour != [1.0; 3] {
                self.dropped
                    .domes
                    .push(format!("{} (its colour tint was left out)", prim.name));
            }
            self.counts.domes_in += 1;
        }
        Ok(())
    }

    /// Adds prim `p` as an item under `parent`; `pre` places a
    /// PointInstancer copy.
    fn add(
        &mut self,
        p: usize,
        parent: Option<u64>,
        pre: Option<DMat4>,
        parent_is_lens: bool,
    ) -> Result<()> {
        let prim = &self.stage.prims[p];
        // A full scene takes nothing more: what's left is counted for the
        // report and the import finishes with what fits (scene-budget).
        if self.scene.items.len() >= self.item_limit {
            self.dropped.over_budget += self.items_below(p);
            if self.dropped.first_over_budget.is_empty() {
                self.dropped.first_over_budget = prim.path.clone();
            }
            return Ok(());
        }
        if prim.instancer.is_some() && is_scatter(prim) {
            return self.add_scatter(p, parent, parent_is_lens);
        }
        if prim.instance {
            self.counts.native_instances += 1;
        }
        if prim.resets_xform {
            self.dropped.resets += 1;
        }
        let absorbed = self.absorbed(p);
        let own_meshes: Vec<usize> = if prim.mesh.is_some() {
            vec![p]
        } else {
            absorbed.clone()
        };
        // A camera or light: the prim itself, or the one an Xform holds
        // with no transform of its own (Blender's object and its data).
        // A mesh with MeshLightAPI is a mesh (its light comes below).
        let lens = if prim.camera.is_some() || (prim.light.is_some() && prim.mesh.is_none()) {
            Some(p)
        } else if own_meshes.is_empty() {
            self.absorbed_lens(p)
        } else {
            None
        };
        let id = if let Some(l) = lens {
            let source = &self.stage.prims[l];
            if let Some(c) = &source.camera {
                self.counts.cameras_in += 1;
                let settings = crate::import_lights::camera_settings(c, self.scale);
                let id = self.scene.add_camera(settings, Transform::default())?;
                self.camera_paths.insert(source.path.clone(), id);
                self.camera_paths.insert(prim.path.clone(), id);
                id
            } else {
                let light = source.light.as_ref().expect("a camera or a light");
                let crate::import_lights::Light::Item(settings) =
                    crate::import_lights::light_settings(&source.ty, light, self.scale)
                else {
                    unreachable!("only light items are importable");
                };
                self.counts.lights_in += 1;
                self.scene.add_light(*settings, Transform::default())?
            }
        } else if own_meshes.is_empty() {
            self.scene
                .add_group_locator_named(Some(prim.name.clone()).filter(|n| !n.is_empty()))?
        } else {
            self.scene
                .new_mesh_named(Some(prim.name.clone()).filter(|n| !n.is_empty()))?;
            self.scene.items.last().expect("new_mesh added an item").id
        };
        let transform = self.transform(prim, pre, lens.is_some(), parent_is_lens);
        let (mesh, subdivision) = if own_meshes.is_empty() {
            (None, None)
        } else {
            match self.mesh_for(&own_meshes) {
                Ok((m, s)) => (Some(m), s),
                Err(e) => {
                    self.dropped.broken.push(format!("{}: {e}", prim.name));
                    (Some(Shared::new(Mesh::default())), None)
                }
            }
        };
        let instance_source = if own_meshes.is_empty() || !self.instances {
            None
        } else {
            let key = self.mesh_key(&own_meshes);
            match self.owners.get(&key) {
                Some(&first) => {
                    self.counts.instances_made += 1;
                    Some(first)
                }
                None => {
                    self.owners.insert(key, id);
                    None
                }
            }
        };
        // A mesh is hidden by its own eye, or by its instance holder's.
        let holder = self.instance_holder(p).filter(|_| !absorbed.is_empty());
        let all_hidden = !own_meshes.is_empty()
            && prim.mesh.is_none()
            && absorbed.iter().all(|&m| {
                !self.stage.prims[m].visible || holder.is_some_and(|h| !self.stage.prims[h].visible)
            });
        {
            // The item just added is the last one (every way above appends);
            // searching the list for it made big imports quadratic.
            let item = self
                .scene
                .items
                .last_mut()
                .filter(|i| i.id == id)
                .expect("item just added");
            if !prim.name.is_empty() && item.name != prim.name {
                item.name = prim.name.clone();
            }
            item.parent = parent;
            item.transform = transform;
            if let Some(mesh) = mesh {
                item.mesh = MeshSlot::new(mesh);
            }
            // U5: an item showing a mesh an earlier item already shows is
            // that item's Mesh Instance (Instances option on).
            if let Some(source) = instance_source {
                item.instance_of = Some(source);
            }
            if subdivision.is_some() {
                item.subdivision = subdivision;
            }
            // USD's invisibility hides everything below the prim too.
            if !prim.visible {
                item.visible = false;
                item.hides_children = true;
            } else if all_hidden {
                item.visible = false;
            }
        }
        self.counts.items += 1;
        // Mesh lights (MeshLightAPI) on the meshes this item took in.
        if self.lights {
            for &m in &own_meshes {
                let source = &self.stage.prims[m];
                if let (Some(light), Some(_)) = (&source.light, source.mesh) {
                    crate::mesh_lights::add(
                        &mut self.scene,
                        id,
                        parent,
                        &prim.name,
                        light,
                        source.custom.as_ref(),
                    )?;
                    self.counts.lights_in += 1;
                }
            }
        }
        if let Some(inst) = &prim.instancer {
            let proto_indices = self.stage.i32s(&inst.proto_indices)?;
            let matrices = self.stage.f64s(&inst.matrices)?;
            let (hidden, inactive) = self.hidden(inst)?;
            for (i, &k) in proto_indices.iter().enumerate() {
                if inactive.contains(&i) {
                    continue;
                }
                let (Some(&proto), Some(m)) = (
                    inst.prototypes.get(k.max(0) as usize).filter(|_| k >= 0),
                    matrices.get(i * 16..i * 16 + 16),
                ) else {
                    continue;
                };
                if !self.importable[proto] {
                    continue;
                }
                let m: [f64; 16] = m.try_into().expect("sixteen numbers");
                self.counts.instancer_copies += 1;
                let first = self.scene.items.len();
                self.add(
                    proto,
                    Some(id),
                    Some(self.scaled(matrix(&m))),
                    lens.is_some(),
                )?;
                if hidden.contains(&i)
                    && let Some(item) = self.scene.items.get_mut(first)
                {
                    item.visible = false;
                }
            }
            return Ok(());
        }
        let kids: Vec<usize> = self.children[p]
            .iter()
            .copied()
            .filter(|c| {
                self.importable[*c]
                    && !absorbed.contains(c)
                    && lens != Some(*c)
                    && holder != Some(*c)
            })
            .collect();
        for c in kids {
            self.add(c, Some(id), None, lens.is_some())?;
        }
        Ok(())
    }

    /// How many items prim `p` and the importable prims below it would
    /// have made (instancer copies not counted).
    fn items_below(&self, p: usize) -> usize {
        let mut n = 0;
        let mut todo = vec![p];
        while let Some(q) = todo.pop() {
            n += 1;
            todo.extend(self.children[q].iter().filter(|&&c| self.importable[c]));
        }
        n
    }

    /// The instances `inst`'s invisibleIds hide and its inactiveIds
    /// remove, by index.
    fn hidden(&self, inst: &crate::stage::Instancer) -> Result<(BTreeSet<usize>, BTreeSet<usize>)> {
        let set = |b: &Option<crate::stage::BufRef>| -> Result<BTreeSet<usize>> {
            Ok(match b {
                Some(h) => self
                    .stage
                    .i32s(h)?
                    .into_iter()
                    .filter_map(|i| usize::try_from(i).ok())
                    .collect(),
                None => BTreeSet::new(),
            })
        };
        Ok((set(&inst.hidden)?, set(&inst.inactive)?))
    }

    /// A PointInstancer Operandi wrote for a scatter of Mesh Instances
    /// (point_instancer.rs, USD-IQ25): its points come back as the items
    /// they were, under the instancer's parent, with their names and eyes.
    fn add_scatter(&mut self, p: usize, parent: Option<u64>, parent_is_lens: bool) -> Result<()> {
        let prim = &self.stage.prims[p];
        let inst = prim.instancer.as_ref().expect("an instancer");
        let names: Vec<String> = prim
            .custom
            .as_ref()
            .and_then(|c| c.get("names"))
            .and_then(serde_json::Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|n| n.as_str().unwrap_or_default().to_string())
                    .collect()
            })
            .unwrap_or_default();
        let proto_indices = self.stage.i32s(&inst.proto_indices)?;
        let matrices = self.stage.f64s(&inst.matrices)?;
        let (hidden, inactive) = self.hidden(inst)?;
        for (i, &k) in proto_indices.iter().enumerate() {
            if inactive.contains(&i) {
                continue;
            }
            let (Some(&proto), Some(m)) = (
                inst.prototypes.get(k.max(0) as usize).filter(|_| k >= 0),
                matrices.get(i * 16..i * 16 + 16),
            ) else {
                continue;
            };
            if !self.importable[proto] {
                continue;
            }
            let m: [f64; 16] = m.try_into().expect("sixteen numbers");
            self.counts.instancer_copies += 1;
            let first = self.scene.items.len();
            self.add(proto, parent, Some(self.scaled(matrix(&m))), parent_is_lens)?;
            if let Some(item) = self.scene.items.get_mut(first) {
                if let Some(name) = names.get(i).filter(|n| !n.is_empty()) {
                    item.name = name.clone();
                }
                if hidden.contains(&i) {
                    item.visible = false;
                }
            }
        }
        Ok(())
    }

    /// `m` with its translation in metres.
    fn scaled(&self, mut m: DMat4) -> DMat4 {
        m.w_axis.x *= self.scale;
        m.w_axis.y *= self.scale;
        m.w_axis.z *= self.scale;
        m
    }

    /// An item's local transform. Every transform is seen through the
    /// file-to-scene turn (`turn · local · turn⁻¹`) and the points turn
    /// with it, so a Z-up file's items arrive with the rotations a Y-up
    /// file would give them: an upright chair has no rotation, and a turn
    /// about the file's up axis is a turn about the scene's.
    ///
    /// A camera or light looks down its local −Z in both the file and the
    /// scene, so its item isn't seen through the turn on its own side: its
    /// transform is `turn · local` (it then looks where the file's does),
    /// and its children's are `local · turn⁻¹`.
    fn transform(
        &mut self,
        prim: &Prim,
        pre: Option<DMat4>,
        lens: bool,
        parent_is_lens: bool,
    ) -> Transform {
        let file_order = prim
            .trs
            .as_ref()
            .and_then(|t| rotation_order(&t.order))
            .unwrap_or(RotationOrder::Xyz);
        let order = self.scene_order(file_order);
        let local = self.scaled(prim.local.as_ref().map_or(DMat4::IDENTITY, matrix));
        // Plain translate / rotate / scale comes in exactly as written,
        // when it really makes the local matrix (OpenUSD's common-API read
        // gives zero angles for double-precision rotate ops).
        if let Some(trs) = prim
            .trs
            .as_ref()
            .filter(|_| pre.is_none() && self.turn == DMat4::IDENTITY && !lens && !parent_is_lens)
        {
            let mut t = Transform::default();
            t.set_rotation_order(order);
            t.translation = DVec3::from_array(trs.t) * self.scale;
            t.rotation = DVec3::from_array(trs.r.map(f64::to_radians));
            t.scale = DVec3::from_array(trs.s);
            let size = local
                .to_cols_array()
                .iter()
                .fold(1.0f64, |a, v| a.max(v.abs()));
            if t.matrix().abs_diff_eq(local, 1e-5 * size) {
                return t;
            }
        }
        let left = if parent_is_lens {
            DMat4::IDENTITY
        } else {
            self.turn
        };
        let right = if lens {
            DMat4::IDENTITY
        } else {
            self.turn.inverse()
        };
        let m = left * pre.unwrap_or(DMat4::IDENTITY) * local * right;
        let (scale, rotation, translation) = m.to_scale_rotation_translation();
        let rebuilt = DMat4::from_scale_rotation_translation(scale, rotation, translation);
        let size = m.to_cols_array().iter().fold(1.0f64, |a, v| a.max(v.abs()));
        if !rebuilt.abs_diff_eq(m, 1e-6 * size) {
            self.dropped.shear += 1;
        }
        let mut t = Transform::default();
        t.set_rotation_order(order);
        t.translation = translation;
        t.rotation = order.angles(rotation);
        t.scale = scale;
        t
    }

    /// The rotation order that names the same axes after the turn (a
    /// Z-up file's XYZ is the scene's XZY), so the angles stay the
    /// file's.
    fn scene_order(&self, order: RotationOrder) -> RotationOrder {
        let turn = glam::DMat3::from_mat4(self.turn);
        let target = |axis: usize| {
            let column = turn.col(axis).abs();
            (0..3).find(|&r| column[r] > 0.5).unwrap_or(axis)
        };
        let axes = order.axes().map(target);
        RotationOrder::ALL
            .into_iter()
            .find(|o| o.axes() == axes)
            .unwrap_or(order)
    }

    /// One item's mesh from Mesh prims `prims` (shared with every other
    /// item made from the same prims), and its subdivision settings.
    fn mesh_for(&mut self, prims: &[usize]) -> Result<BuiltMesh> {
        let key = self.mesh_key(prims);
        if let Some(found) = self.meshes.get(&key).cloned() {
            self.count_polygons(&found.0);
            return Ok(found);
        }
        self.build_mesh(prims, key)
    }

    /// What makes two items' meshes the same: the mesh data, whole binding
    /// and subset bindings of each Mesh prim.
    fn mesh_key(&self, prims: &[usize]) -> MeshKey {
        prims
            .iter()
            .map(|&p| {
                let prim = &self.stage.prims[p];
                (
                    prim.mesh.unwrap_or(usize::MAX),
                    prim.material.unwrap_or(-1),
                    prim.subsets.iter().map(|s| s.material).collect(),
                )
            })
            .collect()
    }

    fn build_mesh(&mut self, prims: &[usize], key: MeshKey) -> Result<BuiltMesh> {
        let mut out = Accumulator::default();
        let mut settings: Option<SubdivisionSettings> = None;
        for &p in prims {
            let prim = &self.stage.prims[p];
            let Some(data) = prim.mesh.and_then(|m| self.stage.meshes.get(m)) else {
                continue;
            };
            let s = self.read_mesh(prim, data, &mut out)?;
            settings = settings.or(s);
        }
        let name = self.stage.prims[prims[0]].name.clone();
        let mut mesh = out.finish();
        let mut notes = Vec::new();
        if let Err(e) = interchange::finish_mesh(&mut mesh, &name, &mut notes) {
            // Creases the mesh rules refuse shouldn't cost the polygons:
            // try again without them.
            if mesh.attributes.creases.is_empty() {
                return Err(e);
            }
            mesh.attributes.creases.clear();
            notes.clear();
            interchange::finish_mesh(&mut mesh, &name, &mut notes)?;
            notes.push(format!("{name}: creases left out ({e})"));
        }
        self.dropped.broken.extend(notes);
        let shared = Shared::new(mesh);
        self.count_polygons(&shared);
        self.meshes.insert(key, (shared.clone(), settings.clone()));
        Ok((shared, settings))
    }

    fn count_polygons(&mut self, mesh: &Mesh) {
        for (i, f) in mesh.faces.iter().enumerate() {
            if mesh.is_polyline(i) {
                continue;
            }
            match f.vertices.len() {
                3 => self.counts.triangles += 1,
                4 => self.counts.quads += 1,
                _ => self.counts.ngons += 1,
            }
        }
    }

    /// Appends one BasisCurves or NurbsCurves prim to `out` as line
    /// elements (curves.rs).
    fn read_curves(
        &mut self,
        prim: &Prim,
        data: &MeshData,
        c: &crate::stage::CurvesData,
        out: &mut Accumulator,
    ) -> Result<()> {
        let s = self.stage;
        let turn = glam::DMat3::from_mat4(self.turn);
        let points: Vec<DVec3> = s
            .f32s(&data.points)?
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| turn * DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64) * self.scale)
            .collect();
        if points.iter().any(|p| !p.is_finite()) {
            return Err("non-finite point coordinates".into());
        }
        let counts = s.i32s(&c.vertex_counts)?;
        if counts.iter().any(|&n| n < 0)
            || counts.iter().map(|&n| n as usize).sum::<usize>() > points.len()
        {
            return Err("its curve counts don't match its points".into());
        }
        let order = match &c.order {
            Some(b) => s.i32s(b)?,
            None => Vec::new(),
        };
        let knots = match &c.knots {
            Some(b) => s.f64s(b)?,
            None => Vec::new(),
        };
        let weights = match &c.point_weights {
            Some(b) => s.f64s(b)?,
            None => Vec::new(),
        };
        let kinds = c
            .custom
            .as_ref()
            .and_then(|v| v.get("kinds"))
            .and_then(serde_json::Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|k| k.as_str().unwrap_or_default().to_string())
                    .collect()
            });
        let mut notes = Vec::new();
        let elements = crate::curves::line_elements(
            &crate::curves::CurvesIn {
                schema: &c.schema,
                ty: &c.ty,
                basis: &c.basis,
                wrap: &c.wrap,
                counts: &counts,
                points: &points,
                order: &order,
                knots: &knots,
                weights: &weights,
                kinds,
            },
            &mut notes,
        );
        self.dropped.curves.extend(notes);
        // Widths, in metres, per file point.
        let widths = match &c.widths {
            Some(b) => s.f32s(b)?,
            None => Vec::new(),
        };
        let width_of = |e: &crate::curves::LineElement, k: usize| -> Option<f64> {
            let w = match c.widths_interpolation.as_str() {
                "constant" => widths.first(),
                "uniform" => widths.get(e.index),
                _ if widths.len() == points.len() => widths.get(e.from[k]?),
                _ => None,
            };
            w.map(|&w| w as f64 * self.scale).filter(|w| w.is_finite())
        };
        if !widths.is_empty()
            && !matches!(c.widths_interpolation.as_str(), "constant" | "uniform")
            && widths.len() != points.len()
        {
            self.dropped.curves.insert(format!(
                "{}: widths left out (not one per point)",
                prim.name
            ));
        }
        let tag = usize::try_from(prim.material.unwrap_or(-1))
            .ok()
            .inspect(|&m| {
                self.used.insert(m);
            })
            .and_then(|m| self.tags.get(m))
            .filter(|t| !operandi_core::material::is_default_name(t))
            .cloned()
            .unwrap_or_default();
        let mut width_values = std::collections::BTreeMap::new();
        for e in &elements {
            let base = out.points.len() as u32;
            out.points.extend(&e.points);
            for k in 0..e.points.len() {
                if let Some(w) = width_of(e, k) {
                    width_values.insert(base + k as u32, vec![w]);
                }
            }
            out.faces.push(Face {
                vertices: (base..base + e.points.len() as u32).collect(),
                uv: None,
                material: tag.clone(),
                smooth: true,
                part: None,
                material_label: None,
            });
            out.representations.push(PolygonRepresentation::Polyline {
                closed: e.closed,
                curve: e.curve,
            });
            self.counts.curves += 1;
        }
        if !width_values.is_empty() {
            out.extras
                .add_point_typed(crate::curves::WIDTHS_MAP, "weight1", width_values);
        }
        Ok(())
    }

    /// Appends one Mesh prim to `out`; returns its subdivision settings
    /// when it is a subdivision surface.
    fn read_mesh(
        &mut self,
        prim: &Prim,
        data: &MeshData,
        out: &mut Accumulator,
    ) -> Result<Option<SubdivisionSettings>> {
        if let Some(curves) = &data.curves {
            self.read_curves(prim, data, curves, out)?;
            return Ok(None);
        }
        let s = self.stage;
        let raw = s.f32s(&data.points)?;
        let turn = glam::DMat3::from_mat4(self.turn);
        let points: Vec<DVec3> = raw
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| turn * DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64) * self.scale)
            .collect();
        if points.iter().any(|p| !p.is_finite()) {
            return Err("non-finite point coordinates".into());
        }
        let counts = s.i32s(&data.counts)?;
        let indices = s.i32s(&data.indices)?;
        if counts.iter().any(|&c| c < 0)
            || counts.iter().map(|&c| c as usize).sum::<usize>() != indices.len()
        {
            return Err("its polygon counts don't match its corners".into());
        }
        if indices.iter().any(|&v| v < 0 || v as usize >= points.len()) {
            return Err("a polygon refers to a missing point".into());
        }
        let holes: BTreeSet<i32> = match &data.holes {
            Some(h) => s.i32s(h)?.into_iter().collect(),
            None => BTreeSet::new(),
        };
        let left = data.orientation == "leftHanded";
        let subdivided = matches!(data.scheme.as_str(), "catmullClark" | "loop" | "");
        if !matches!(data.scheme.as_str(), "catmullClark" | "none" | "") {
            self.dropped.schemes.insert(data.scheme.clone());
        }
        // Material tag per polygon: the subsets' bindings over the mesh's.
        let whole = prim.material.unwrap_or(-1);
        let mut face_material = vec![whole; counts.len()];
        for sub in &prim.subsets {
            for f in s.i32s(&sub.faces)? {
                if let Some(slot) = face_material.get_mut(f.max(0) as usize).filter(|_| f >= 0) {
                    *slot = sub.material;
                }
            }
        }
        let base = out.points.len() as u32;
        out.points.extend(&points);
        // Polygons, with the file corner each kept corner came from.
        let mut faces: Vec<(usize, Vec<usize>)> = Vec::new();
        let mut start = 0usize;
        for (f, &n) in counts.iter().enumerate() {
            let n = n as usize;
            let corners: Vec<usize> = if left {
                // Reversed winding keeps the first corner first.
                (0..n).map(|k| start + (n - k) % n).collect()
            } else {
                (start..start + n).collect()
            };
            start += n;
            if holes.contains(&(f as i32)) {
                self.dropped.holes += 1;
                continue;
            }
            // Consecutive repeats collapse (first to last too).
            let mut kept: Vec<usize> = Vec::with_capacity(n);
            for c in corners {
                if kept.last().is_none_or(|&k| indices[k] != indices[c]) {
                    kept.push(c);
                }
            }
            while kept.len() > 1 && indices[kept[0]] == indices[*kept.last().unwrap()] {
                kept.pop();
            }
            let distinct: BTreeSet<i32> = kept.iter().map(|&c| indices[c]).collect();
            if distinct.len() < 3 {
                self.dropped.small_faces += 1;
                continue;
            }
            faces.push((f, kept));
        }
        // Normals: per corner, flat polygons marked flat. A subdivision
        // surface ignores authored normals (USD's rule).
        let normals = match (&data.normals, subdivided) {
            (Some(n), false) => Some(corner_values(s, n, 3, &counts, &indices)?),
            _ => None,
        };
        let face_start = out.faces.len();
        for (f, kept) in &faces {
            let smooth = match &normals {
                Some(values) => !flat(kept.iter().map(|&c| values.get(c).cloned().flatten())),
                None => true,
            };
            let material = face_material[*f];
            // A material named "Default" is the default material (Operandi
            // writes untagged polygons that way); its group still comes in.
            let tag = usize::try_from(material)
                .ok()
                .and_then(|m| self.tags.get(m))
                .filter(|t| !operandi_core::material::is_default_name(t))
                .cloned()
                .unwrap_or_default();
            if let Ok(m) = usize::try_from(material) {
                self.used.insert(m);
            }
            out.faces.push(Face {
                vertices: kept.iter().map(|&c| base + indices[c] as u32).collect(),
                uv: None,
                material: tag,
                smooth,
                part: None,
                material_label: None,
            });
            out.representations.push(if subdivided {
                PolygonRepresentation::CatmullClark
            } else {
                PolygonRepresentation::Face
            });
        }
        let corner_map = |values: &[Option<Vec<f64>>]| -> Vec<Vec<Option<Vec<f64>>>> {
            faces
                .iter()
                .map(|(_, kept)| {
                    kept.iter()
                        .map(|&c| values.get(c).cloned().flatten())
                        .collect()
                })
                .collect()
        };
        if let Some(values) = &normals {
            self.dropped.normal_maps += 1;
            let map = corner_map(values);
            out.add_map(NORMAL_MAP, face_start, map, |v| {
                MapValue::Normal(turn * DVec3::new(v[0], v[1], v[2]))
            });
        }
        let mut primary_named = false;
        for pv in &data.primvars {
            match pv.kind.as_str() {
                "uv" => {
                    let values = corner_values(s, pv, 2, &counts, &indices)?;
                    // USD's "st" is Operandi's primary map.
                    let name = if pv.name == "st" && !primary_named {
                        primary_named = true;
                        operandi_core::PRIMITIVE_UV_MAP.to_string()
                    } else {
                        pv.name.clone()
                    };
                    let map = corner_map(&values);
                    out.add_map(&name, face_start, map, |v| {
                        MapValue::Uv(DVec2::new(v[0], v[1]))
                    });
                }
                "color" => {
                    if pv.interpolation == "constant" {
                        self.dropped.constant_colours.insert(pv.name.clone());
                        continue;
                    }
                    let values = corner_values(s, pv, pv.dims, &counts, &indices)?;
                    let map = corner_map(&values);
                    out.add_map(&pv.name, face_start, map, |v| {
                        MapValue::Colour(DVec4::new(
                            v[0],
                            v[1],
                            v[2],
                            v.get(3).copied().unwrap_or(1.0),
                        ))
                    });
                }
                _ => {}
            }
        }
        // Maps, morphs, parts and sets Operandi wrote (USD-IQ22).
        crate::import_extras::read(
            &crate::import_extras::Context {
                stage: s,
                data,
                base,
                point_count: points.len(),
                faces: &faces,
                face_start,
                turn,
                scale: self.scale,
            },
            &mut out.extras,
            &mut self.dropped.foreign,
        )?;
        // Creases: chains of points, a sharpness per chain or per edge.
        if let (Some(ci), Some(cl), Some(cs)) = (
            &data.crease_indices,
            &data.crease_lengths,
            &data.crease_sharpnesses,
        ) {
            let (ci, cl, cs) = (s.i32s(ci)?, s.i32s(cl)?, s.f32s(cs)?);
            let edges: BTreeSet<(u32, u32)> = faces
                .iter()
                .flat_map(|(_, kept)| {
                    (0..kept.len()).map(move |k| (kept[k], kept[(k + 1) % kept.len()]))
                })
                .map(|(a, b)| {
                    let (a, b) = (indices[a] as u32, indices[b] as u32);
                    (a.min(b), a.max(b))
                })
                .collect();
            let per_edge = cs.len() == cl.iter().map(|&l| (l.max(1) - 1) as usize).sum::<usize>();
            let mut at = 0usize;
            let mut edge = 0usize;
            for (chain, &len) in cl.iter().enumerate() {
                let len = len.max(0) as usize;
                for k in 0..len.saturating_sub(1) {
                    let (Some(&a), Some(&b)) = (ci.get(at + k), ci.get(at + k + 1)) else {
                        break;
                    };
                    let sharp = if per_edge {
                        cs.get(edge)
                    } else {
                        cs.get(chain)
                    };
                    edge += 1;
                    let (a, b) = (a.max(0) as u32, b.max(0) as u32);
                    if !edges.contains(&(a.min(b), a.max(b))) {
                        continue;
                    }
                    let weight = (sharp.copied().unwrap_or(0.0) as f64 / 10.0).clamp(0.0, 1.0);
                    if weight > 0.0 {
                        // Operandi keeps each crease lower point first,
                        // once (a file may run a chain either way, or
                        // list an edge twice).
                        out.creases.push(MeshCrease {
                            a: base + a.min(b),
                            b: base + a.max(b),
                            weight,
                        });
                    }
                }
                at += len;
            }
        }
        if let Some(c) = &data.corner_indices {
            self.dropped.corners += s.i32s(c)?.len();
        }
        if !subdivided {
            return Ok(None);
        }
        let boundary_rule = match data.boundary.as_str() {
            "edgeAndCorner" => BoundaryRule::Crease,
            "none" => {
                self.dropped.no_boundary = true;
                BoundaryRule::Smooth
            }
            _ => BoundaryRule::Smooth,
        };
        let (uv_boundary_rule, linear_uvs) = match data.fvli.as_str() {
            "all" => (BoundaryRule::Crease, true),
            "none" => (BoundaryRule::Smooth, false),
            "boundaries" => (BoundaryRule::CreaseEdges, false),
            _ => (BoundaryRule::Crease, false),
        };
        Ok(Some(SubdivisionSettings {
            boundary_rule,
            uv_boundary_rule,
            linear_uvs,
            ..SubdivisionSettings::default()
        }))
    }

    fn report(&self, materials: &crate::import_materials::Summary) -> Vec<String> {
        let c = &self.counts;
        let d = &self.dropped;
        let polygons = c.triangles + c.quads + c.ngons;
        let mut out = vec![format!(
            "{} item(s), {polygons} polygon(s): {} quad(s), {} n-gon(s), {} triangle(s)",
            c.items, c.quads, c.ngons, c.triangles
        )];
        if d.over_budget > 0 {
            out.push(format!(
                "{} item(s) left out: the scene was full at {} items (first left out: {})",
                d.over_budget, self.item_limit, d.first_over_budget
            ));
        }
        if c.curves > 0 {
            out.push(format!("{} polyline(s) and curve(s)", c.curves));
        }
        out.extend(d.curves.iter().cloned());
        if materials.total() > 0 || !self.used.is_empty() {
            out.push(format!(
                "Materials: {} MaterialX, {} Preview Surface, {} as plain polygon tags",
                materials.materialx, materials.preview, materials.plain
            ));
        }
        if c.native_instances + c.instancer_copies > 0 {
            out.push(if c.instances_made > 0 {
                format!(
                    "Instances: {} instance(s) in the file; {} item(s) came in as Mesh Instances of the first one showing their mesh",
                    c.native_instances + c.instancer_copies,
                    c.instances_made
                )
            } else {
                format!(
                    "Instances: {} came in as copies sharing their mesh until edited",
                    c.native_instances + c.instancer_copies
                )
            });
        }
        if c.cameras + c.lights + c.domes > 0 {
            out.push(format!(
                "Cameras: {} of {} imported; lights: {} of {}; dome lights: {} of {} (as the environment)",
                c.cameras_in, c.cameras, c.lights_in, c.lights, c.domes_in, c.domes
            ));
        }
        for (ty, why) in &d.lights {
            out.push(format!("{ty} left out: {why}"));
        }
        for dome in &d.domes {
            out.push(format!("Dome light {dome}"));
        }
        if d.small_faces > 0 {
            out.push(format!(
                "{} point/line element(s) with fewer than 3 corners left out",
                d.small_faces
            ));
        }
        if d.holes > 0 {
            out.push(format!("{} hole polygon(s) left out", d.holes));
        }
        if d.corners > 0 {
            out.push(format!(
                "{} corner sharpness(es) left out: Operandi has edge creases only",
                d.corners
            ));
        }
        if d.no_boundary {
            out.push("interpolateBoundary \"none\" read as smooth boundaries".into());
        }
        for scheme in &d.schemes {
            out.push(format!(
                "Subdivision scheme \"{scheme}\" read as {}",
                if scheme == "bilinear" {
                    "plain polygons"
                } else {
                    "Catmull-Clark"
                }
            ));
        }
        if d.normal_maps > 0 {
            out.push(format!(
                "Normals kept as \"{NORMAL_MAP}\" maps; the viewport shades from each polygon's smoothing"
            ));
        }
        if !d.constant_colours.is_empty() {
            let names: Vec<&str> = d.constant_colours.iter().map(String::as_str).collect();
            out.push(format!(
                "Constant colour primvar(s) {} left out (one colour for the whole mesh)",
                names.join(", ")
            ));
        }
        out.extend(d.foreign.iter().cloned());
        if d.shear > 0 {
            out.push(format!(
                "{} transform(s) had shear, which was left out",
                d.shear
            ));
        }
        if d.resets > 0 {
            out.push(format!(
                "{} prim(s) reset the transform stack; read as ordinary local transforms",
                d.resets
            ));
        }
        if !d.guides.is_empty() {
            out.push(format!(
                "Guide and proxy meshes left out: {}",
                d.guides.join(", ")
            ));
        }
        for (ty, n) in &d.other {
            out.push(format!("{n} {ty} prim(s) left out"));
        }
        out.extend(d.broken.iter().cloned());
        out
    }
}

fn rotation_order(name: &str) -> Option<RotationOrder> {
    Some(match name {
        "XYZ" => RotationOrder::Xyz,
        "XZY" => RotationOrder::Xzy,
        "YXZ" => RotationOrder::Yxz,
        "YZX" => RotationOrder::Yzx,
        "ZXY" => RotationOrder::Zxy,
        "ZYX" => RotationOrder::Zyx,
        _ => return None,
    })
}

/// A tag per material: its prim name, numbered when two share one.
fn material_tags(stage: &StageData) -> Vec<String> {
    let mut taken = BTreeSet::new();
    stage
        .materials
        .iter()
        .map(|m| {
            let base = if m.name.is_empty() {
                "Material".to_string()
            } else {
                m.name.clone()
            };
            let mut name = base.clone();
            let mut n = 2;
            while !taken.insert(name.clone()) {
                name = format!("{base} ({n})");
                n += 1;
            }
            name
        })
        .collect()
}

/// A primvar's value at every file corner (`None` where its indices or
/// values run out), `dims` numbers each.
fn corner_values(
    s: &StageData,
    pv: &Primvar,
    dims: usize,
    counts: &[i32],
    indices: &[i32],
) -> Result<Vec<Option<Vec<f64>>>> {
    let values = s.f32s(&pv.values)?;
    let lookup = match &pv.indices {
        Some(i) => Some(s.i32s(i)?),
        None => None,
    };
    let width = pv.dims.max(1);
    let element = |k: usize| -> Option<Vec<f64>> {
        let k = match &lookup {
            Some(l) => usize::try_from(*l.get(k)?).ok()?,
            None => k,
        };
        let v = values.get(k * width..k * width + width)?;
        let mut out: Vec<f64> = v.iter().map(|&x| x as f64).collect();
        out.resize(dims.max(1), 0.0);
        out.iter().all(|x| x.is_finite()).then_some(out)
    };
    let mut out = Vec::with_capacity(indices.len());
    let mut corner = 0usize;
    for (f, &n) in counts.iter().enumerate() {
        for _ in 0..n {
            let k = match pv.interpolation.as_str() {
                "constant" => 0,
                "uniform" => f,
                "vertex" | "varying" => indices[corner] as usize,
                _ => corner,
            };
            out.push(element(k));
            corner += 1;
        }
    }
    Ok(out)
}

/// True when a polygon's corner normals are all the same (a flat-shaded
/// polygon).
fn flat(normals: impl Iterator<Item = Option<Vec<f64>>>) -> bool {
    let mut first: Option<DVec3> = None;
    for n in normals {
        let Some(n) = n else { return false };
        let n = DVec3::new(n[0], n[1], n[2]).normalize_or_zero();
        match first {
            None => first = Some(n),
            Some(f) if f.dot(n) < 1.0 - 1e-6 => return false,
            _ => {}
        }
    }
    true
}

/// A per-corner map value.
enum MapValue {
    Uv(DVec2),
    Colour(DVec4),
    Normal(DVec3),
}

/// The polygons of every Mesh prim one item takes in.
#[derive(Default)]
struct Accumulator {
    points: Vec<DVec3>,
    faces: Vec<Face>,
    representations: Vec<PolygonRepresentation>,
    creases: Vec<MeshCrease>,
    uvs: BTreeMap<String, Vec<Vec<Option<DVec2>>>>,
    colours: BTreeMap<String, Vec<Vec<Option<DVec4>>>>,
    normals: BTreeMap<String, Vec<Vec<Option<DVec3>>>>,
    extras: crate::import_extras::Extras,
}

impl Accumulator {
    /// Adds map `name`'s values for the polygons from `face_start` on
    /// (already in `faces`); earlier polygons without the map get none.
    fn add_map(
        &mut self,
        name: &str,
        face_start: usize,
        map: Vec<Vec<Option<Vec<f64>>>>,
        value: impl Fn(&[f64]) -> MapValue,
    ) {
        let lists: Vec<Vec<Option<MapValue>>> = map
            .into_iter()
            .map(|corners| corners.into_iter().map(|c| c.map(|v| value(&v))).collect())
            .collect();
        let sizes: Vec<usize> = self.faces.iter().map(|f| f.vertices.len()).collect();
        macro_rules! put {
            ($store:expr, $variant:ident) => {{
                let entry = $store.entry(name.to_string()).or_default();
                while entry.len() < face_start {
                    entry.push(vec![None; sizes[entry.len()]]);
                }
                for corners in lists {
                    entry.push(
                        corners
                            .into_iter()
                            .map(|v| match v {
                                Some(MapValue::$variant(x)) => Some(x),
                                _ => None,
                            })
                            .collect(),
                    );
                }
            }};
        }
        match lists.iter().flatten().flatten().next() {
            Some(MapValue::Uv(_)) => put!(self.uvs, Uv),
            Some(MapValue::Colour(_)) => put!(self.colours, Colour),
            Some(MapValue::Normal(_)) => put!(self.normals, Normal),
            None => {}
        }
    }

    fn finish(mut self) -> Mesh {
        self.extras.apply_parts(&mut self.faces);
        let extras = std::mem::take(&mut self.extras);
        let n = self.faces.len();
        let sizes: Vec<usize> = self.faces.iter().map(|f| f.vertices.len()).collect();
        fn complete<T: Clone>(map: &mut Vec<Vec<Option<T>>>, sizes: &[usize]) {
            while map.len() < sizes.len() {
                map.push(vec![None; sizes[map.len()]]);
            }
        }
        let mut mesh = Mesh {
            vertices: self.points,
            faces: self.faces.into(),
            ..Default::default()
        };
        let mut uvs = self.uvs;
        for map in uvs.values_mut() {
            complete(map, &sizes);
        }
        if !uvs.is_empty() {
            *mesh.uv_maps = uvs;
        }
        let attributes = &mut *mesh.attributes;
        attributes.representations = if self
            .representations
            .iter()
            .all(|r| *r == PolygonRepresentation::Face)
        {
            Vec::new()
        } else {
            self.representations
        };
        debug_assert!(
            attributes.representations.is_empty() || attributes.representations.len() == n
        );
        // One crease per edge: the sharpest a file gave it.
        let mut creases = self.creases;
        creases.sort_by(|x, y| {
            (x.a, x.b)
                .cmp(&(y.a, y.b))
                .then(y.weight.total_cmp(&x.weight))
        });
        creases.dedup_by_key(|c| (c.a, c.b));
        attributes.creases = creases;
        let mut colours = self.colours;
        for map in colours.values_mut() {
            complete(map, &sizes);
        }
        attributes.corner_colors = colours;
        let mut normals = self.normals;
        for map in normals.values_mut() {
            complete(map, &sizes);
        }
        attributes.normal_map_order = normals.keys().cloned().collect();
        attributes.corner_normals = normals;
        extras.finish(&mut mesh);
        mesh
    }
}
