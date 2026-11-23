use crate::{Mesh, Result};
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub translation: DVec3,
    pub rotation: DVec3,
    pub scale: DVec3,
    #[serde(default)]
    pub zxy: bool,
    /// One of the four rotation orders `zxy` can't name (XZY, YXZ, YZX,
    /// ZYX); `None` for XYZ and ZXY, which `zxy` holds. Read and set it
    /// through [`Transform::rotation_order`] and
    /// [`Transform::set_rotation_order`]. Native version 43.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<RotationOrder>,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: DVec3::ZERO,
            rotation: DVec3::ZERO,
            scale: DVec3::ONE,
            zxy: false,
            order: None,
        }
    }
}

/// The order an item's three rotation angles apply in. The letters are the
/// axes in the order they turn the item ("ZXY" turns about Z, then X, then
/// Y; "XYZ" about X, then Y, then Z), as the measured ZXY and XYZ matrices
/// show (IT-PHY-01, IT-O-H3) and as LXO-LEG-Q01 reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RotationOrder {
    Xyz,
    Xzy,
    Yxz,
    Yzx,
    Zxy,
    Zyx,
}
impl RotationOrder {
    /// Every order, in the menu order. XYZ and ZXY come first, so a
    /// `rotOrder` channel index from before the other four keeps its meaning.
    pub const ALL: [RotationOrder; 6] = [
        RotationOrder::Xyz,
        RotationOrder::Zxy,
        RotationOrder::Xzy,
        RotationOrder::Yxz,
        RotationOrder::Yzx,
        RotationOrder::Zyx,
    ];
    pub fn label(self) -> &'static str {
        match self {
            RotationOrder::Xyz => "XYZ",
            RotationOrder::Xzy => "XZY",
            RotationOrder::Yxz => "YXZ",
            RotationOrder::Yzx => "YZX",
            RotationOrder::Zxy => "ZXY",
            RotationOrder::Zyx => "ZYX",
        }
    }
    /// The axes (0 = X) in the order Operandi turns them. Before native
    /// version 44 XYZ was composed Rx·Ry·Rz on column vectors (Z turned
    /// first, as ZYX does now); it turns X first since ETL-Q01, and older
    /// files are moved over on reading (`Scene::xyz_items_to_x_first`).
    pub fn axes(self) -> [usize; 3] {
        match self {
            RotationOrder::Xyz => [0, 1, 2],
            RotationOrder::Xzy => [0, 2, 1],
            RotationOrder::Yxz => [1, 0, 2],
            RotationOrder::Yzx => [1, 2, 0],
            RotationOrder::Zxy => [2, 0, 1],
            RotationOrder::Zyx => [2, 1, 0],
        }
    }
    /// The rotation `angles` (radians, per axis) make in this order.
    pub fn quat(self, angles: DVec3) -> glam::DQuat {
        self.axes().iter().fold(glam::DQuat::IDENTITY, |q, &axis| {
            let r = match axis {
                0 => glam::DQuat::from_rotation_x(angles.x),
                1 => glam::DQuat::from_rotation_y(angles.y),
                _ => glam::DQuat::from_rotation_z(angles.z),
            };
            r * q
        })
    }
    /// The angles (radians) giving rotation `q` in this order.
    pub fn angles(self, q: glam::DQuat) -> DVec3 {
        use glam::EulerRot as E;
        // q = R(last)·R(middle)·R(first), which glam's intrinsic orders
        // name last axis first.
        let [first, middle, last] = self.axes();
        let rot = match [last, middle, first] {
            [0, 1, 2] => E::XYZ,
            [0, 2, 1] => E::XZY,
            [1, 0, 2] => E::YXZ,
            [1, 2, 0] => E::YZX,
            [2, 0, 1] => E::ZXY,
            _ => E::ZYX,
        };
        let (a, b, c) = q.to_euler(rot);
        let mut out = DVec3::ZERO;
        out[last] = a;
        out[middle] = b;
        out[first] = c;
        out
    }
}

impl Transform {
    /// The order the rotation angles apply in.
    pub fn rotation_order(&self) -> RotationOrder {
        match self.order {
            Some(order) => order,
            None if self.zxy => RotationOrder::Zxy,
            None => RotationOrder::Xyz,
        }
    }
    /// Set the rotation order, keeping the angles (IT-PHY-01). XYZ and ZXY
    /// are stored in `zxy` as before, so files that only use them don't
    /// change.
    pub fn set_rotation_order(&mut self, order: RotationOrder) {
        self.zxy = order == RotationOrder::Zxy;
        self.order = (!matches!(order, RotationOrder::Xyz | RotationOrder::Zxy)).then_some(order);
    }
    /// The angles (radians) giving rotation `q` in this transform's order.
    pub fn angles_for(&self, q: glam::DQuat) -> DVec3 {
        self.rotation_order().angles(q)
    }
    pub fn matrix(&self) -> DMat4 {
        DMat4::from_scale_rotation_translation(self.scale, self.rotation_quat(), self.translation)
    }
    /// The rotation the angles make in this transform's order.
    pub fn rotation_quat(&self) -> glam::DQuat {
        self.rotation_order().quat(self.rotation)
    }
    pub fn point(&self, p: DVec3) -> DVec3 {
        self.matrix().transform_point3(p)
    }
    /// True for an XYZ transform that the pre-v44 Z-first composition turns
    /// differently from today's X-first one: XYZ with two or more non-zero
    /// angles. With one or none the two agree.
    pub fn xyz_meaning_changed(&self) -> bool {
        self.rotation_order() == RotationOrder::Xyz
            && self
                .rotation
                .to_array()
                .iter()
                .filter(|a| **a != 0.0)
                .count()
                >= 2
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: u64,
    pub name: String,
    /// The Items-list eye. Off with `hides_children` set is "all off": the
    /// item and its whole subtree are hidden (GV-01), while the children
    /// keep their own eyes. Use `Scene::shows` for "is it shown".
    pub visible: bool,
    /// With `visible` off, this item's eye also hides its descendants
    /// (Auto Hide Child Items, GV-01..03). Always false while visible.
    /// Native version 35.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hides_children: bool,
    pub transform: Transform,
    /// Packed or open (`mesh_slot.rs`). Reads and writes through it work
    /// like a `Mesh`.
    pub mesh: crate::MeshSlot,
    #[serde(default)]
    pub parent: Option<u64>,
    #[serde(default)]
    pub source_id: Option<u32>,
    #[serde(default)]
    pub source_display_name: Option<String>,
    /// SDV-03 subdivision levels and rules; `None` is a new mesh's defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdivision: Option<crate::subdivision::SubdivisionSettings>,
    /// Set for a Backdrop item (backdrop.rs); its mesh stays empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<crate::backdrop::Backdrop>,
    /// Set for a camera item (camera_light.rs); its mesh stays empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<crate::camera_light::CameraSettings>,
    /// Set for a light item (camera_light.rs); its mesh stays empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light: Option<crate::camera_light::LightSettings>,
    /// Set for a Texture Locator or the Texture Group item
    /// (texture_locator.rs); its mesh stays empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture: Option<crate::texture_locator::TextureItem>,
    /// Set for a locator item such as a Group Locator (locator.rs); its
    /// mesh stays empty. Native version 30.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<crate::locator::LocatorKind>,
    /// Set for an Environment item (environment.rs); its mesh stays
    /// empty. Native version 33.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Box<crate::environment::Environment>>,
    /// Display and Assembly tab settings shared by every item kind
    /// (item_common.rs).
    #[serde(
        default,
        skip_serializing_if = "crate::item_common::ItemCommon::is_default"
    )]
    pub common: crate::item_common::ItemCommon,
    /// Locked (saved) and hidden (not saved) vertices, edges and polygons
    /// (element_state.rs). Native version 28.
    #[serde(
        default,
        skip_serializing_if = "crate::element_state::ElementState::nothing_saved"
    )]
    pub elements: crate::element_state::ElementState,
    /// Mesh Stack (mesh_stack.rs): the procedural stack on top of `mesh`, which
    /// is then the Base Mesh (MO-01). Native version 37.
    /// Saved as `mesh_ops`, its name before 2026-11-23, so files open in
    /// older and newer builds alike.
    #[serde(rename = "mesh_ops", default, skip_serializing_if = "Option::is_none")]
    pub mesh_stack: Option<crate::mesh_stack::MeshStack>,
    /// Set for an item of a kind a kit adds (kit_item.rs, KIT-1): it loads
    /// and saves back unchanged even when the kit is missing; its mesh
    /// stays empty. Native version 49.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kit: Option<Box<crate::kit_item::KitItem>>,
    /// A Mesh Instance (mesh_instance.rs): the mesh item whose mesh this
    /// one shows and shares. Native version 51.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_of: Option<u64>,
    /// Set for a Replicator item (replicator.rs, G8 RP-1): copies of a
    /// prototype on the points of a point cloud; its mesh stays empty.
    /// Native version 57.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replicator: Option<Box<crate::replicator::Replicator>>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Vertex,
    Edge,
    #[default]
    Polygon,
    Item,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub item: Option<u64>,
    pub mode: Mode,
    pub vertices: Vec<u32>,
    pub edges: Vec<(u32, u32)>,
    pub faces: Vec<u32>,
    /// More selected items after `item`, in the order they were added
    /// (Ctrl/Shift-click in the Scenes list). Merge uses them; `item` stays
    /// the one every other tool works on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also: Vec<u64>,
    /// A polygon tag selection mode (Material, Part, Selection Set) is on:
    /// `mode` is Polygon and clicks select whole tagged regions
    /// (tag_selection.rs). Not saved: a reopened scene is in Polygon mode
    /// with the same polygons selected (provisional).
    #[serde(skip)]
    pub tag: Option<crate::TagSelection>,
    /// The other active meshes' components (multi_active.rs): every
    /// selected mesh after `item` keeps its own vertices, edges and
    /// polygons here. `item`'s stay in the fields above. Native version 29.
    #[serde(default, skip_serializing_if = "crate::multi_active::no_others")]
    pub others: crate::multi_active::OtherComponents,
}
/// How many world units one unit of an item's own space is, for turning
/// world distances into the item's units. Uniform scale is assumed (the X
/// axis's length); never below 1e-12, so it is safe to divide by.
pub fn uniform_scale(world: DMat4) -> f64 {
    world.transform_vector3(DVec3::X).length().max(1e-12)
}
impl Selection {
    /// The selected points in the current mode: the points themselves, the
    /// ends of the edges or the corners of the polygons (none in Item mode).
    /// Point indices are not checked against `mesh`.
    pub fn points(&self, mesh: &Mesh) -> BTreeSet<u32> {
        match self.mode {
            crate::Mode::Vertex => self.vertices.iter().copied().collect(),
            crate::Mode::Edge => self.edges.iter().flat_map(|&(a, b)| [a, b]).collect(),
            crate::Mode::Polygon => self
                .faces
                .iter()
                .filter_map(|&f| mesh.faces.get(f as usize))
                .flat_map(|f| f.vertices.iter().copied())
                .collect(),
            crate::Mode::Item => BTreeSet::new(),
        }
    }
    /// Whether item `id` is selected (the primary item or one of `also`).
    pub fn has_item(&self, id: u64) -> bool {
        self.item == Some(id) || self.also.contains(&id)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workplane {
    pub fixed: bool,
    pub plane: u8,
    pub center: DVec3,
}
impl Default for Workplane {
    fn default() -> Self {
        Self {
            fixed: false,
            plane: 0,
            center: DVec3::ZERO,
        }
    }
}
/// The most items a scene holds. Every way of adding items checks it, and
/// so does loading a scene, so a damaged or hostile file can't ask for
/// unbounded memory. Ids are `u64`, undo keeps whole-scene copies that
/// share meshes, saves are a list, and the Item List draws only the rows
/// on screen, so nothing else needs a smaller number: it was 10,000 (4,096
/// in some tools) only to be safe, and 1,000,000 items with empty meshes
/// take about 1 GB (scene-budget, 2026-11-20).
pub const MAX_SCENE_ITEMS: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub items: Vec<Item>,
    pub next_id: u64,
    pub selection: Selection,
    pub workplane: Workplane,
    #[serde(default)]
    pub import_source: Option<ImportSource>,
    /// Still images the scene's Backdrops refer to (BD-03).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<crate::backdrop::SceneImage>,
    /// Paintable images' pixels (paint.rs); saved as image files of their
    /// own, never in the scene file.
    #[cfg(feature = "painting")]
    #[serde(skip)]
    pub paint: crate::paint::PaintImages,
    /// The default material's smoothing (SS-01); it shades every mesh.
    #[serde(default, skip_serializing_if = "is_default_smoothing")]
    pub smoothing: crate::shading::SmoothingSettings,
    /// Final-render settings (render_settings.rs); native version 27.
    #[serde(
        default,
        skip_serializing_if = "crate::render_settings::RenderSettings::is_default"
    )]
    pub render: crate::render_settings::RenderSettings,
    /// Material settings by polygon tag name (material.rs). `Default` is
    /// the material of untagged polygons.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub materials: std::collections::BTreeMap<String, crate::Material>,
    /// The camera chosen to render through; `Scene::render_camera` falls
    /// back to the first camera when this is unset or stale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_camera: Option<u64>,
    /// Surfacing, shading and render-output layers (shader_tree.rs). Saved
    /// only when it differs from the default new-scene tree.
    #[serde(
        default,
        skip_serializing_if = "crate::shader_tree::ShaderTree::is_default"
    )]
    pub shader_tree: crate::shader_tree::ShaderTree,
    /// Named item groups (item_groups.rs).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<crate::item_groups::ItemGroup>,
    /// Item Drawing size (Preferences > Display > OpenGL): how large the
    /// views draw cameras and lights, as a multiple of their usual size.
    /// New scenes take the preference's value; 1 is not saved.
    #[serde(default = "unit_draw_size", skip_serializing_if = "is_unit_draw_size")]
    pub item_draw_size: f64,
    /// Frame rate, time range and current time (scene_time.rs, AR-02).
    /// Saved only when changed from the defaults; native version 49.
    #[serde(
        default,
        skip_serializing_if = "crate::scene_time::SceneTime::is_default"
    )]
    pub time: crate::scene_time::SceneTime,
    /// Key curves, time settings and markers (animation.rs, TK/TL specs).
    /// Saved only when there is any; native version 60.
    #[serde(
        default,
        skip_serializing_if = "crate::animation::Animation::is_default"
    )]
    pub animation: crate::animation::Animation,
    /// G9 GD-3: the scene's guides (guides.rs). Saved only when changed;
    /// native version 59.
    #[serde(default, skip_serializing_if = "crate::guides::Guides::is_default")]
    pub guides: crate::guides::Guides,
    /// The mesh that stays active while no mesh is selected (VP-04,
    /// last_mesh.rs). Not saved: a scene just opened has none.
    #[serde(skip)]
    pub last_mesh: crate::last_mesh::LastMesh,
    /// Where each item is in `items`, remembered (`item_index`). Not saved.
    #[serde(skip)]
    pub index: crate::scene_index::ItemIndex,
}
fn unit_draw_size() -> f64 {
    1.
}
fn is_unit_draw_size(size: &f64) -> bool {
    *size == 1.
}
/// The range Item Drawing size may take.
pub const ITEM_DRAW_SIZE_RANGE: std::ops::RangeInclusive<f64> = 0.01..=100.;
fn is_default_smoothing(s: &crate::shading::SmoothingSettings) -> bool {
    *s == crate::shading::SmoothingSettings::default()
}
impl Default for Scene {
    fn default() -> Self {
        Self {
            items: vec![],
            next_id: 1,
            selection: Selection::default(),
            workplane: Workplane::default(),
            import_source: None,
            images: Vec::new(),
            #[cfg(feature = "painting")]
            paint: Default::default(),
            smoothing: Default::default(),
            render: Default::default(),
            materials: Default::default(),
            render_camera: None,
            shader_tree: Default::default(),
            groups: Vec::new(),
            item_draw_size: 1.,
            time: Default::default(),
            animation: Default::default(),
            guides: Default::default(),
            last_mesh: Default::default(),
            index: Default::default(),
        }
    }
}
impl Scene {
    /// Version 44 (ETL-Q01): XYZ items whose angles the old Z-first XYZ
    /// turns differently from today's X-first one move to ZYX, which turns
    /// exactly as the old XYZ did, so they don't move.
    pub(crate) fn xyz_items_to_x_first(&mut self) {
        for item in &mut self.items {
            if item.transform.xyz_meaning_changed() {
                item.transform.set_rotation_order(RotationOrder::Zyx);
            }
        }
    }
    pub fn active(&self) -> Option<&Item> {
        self.items
            .iter()
            .find(|i| Some(i.id) == self.selection.item)
    }
    pub fn active_mut(&mut self) -> Option<&mut Item> {
        let id = self.selection.item;
        self.items.iter_mut().find(|i| Some(i.id) == id)
    }
    pub fn new_mesh(&mut self) -> Result<()> {
        // "Mesh", then "Mesh (2)", "Mesh (3)" … (spec new-mesh NM06).
        self.new_mesh_named(None)
    }
    /// `new_mesh`, named `name` as it is instead of the next free "Mesh"
    /// name. Importers that name every item use this: finding a free name
    /// reads the whole item list, so a hundred thousand of them would cost
    /// billions of comparisons.
    pub fn new_mesh_named(&mut self, name: Option<String>) -> Result<()> {
        if self.next_id >= u64::MAX - 1 || self.items.len() >= crate::MAX_SCENE_ITEMS {
            return Err("Scene item/identity budget exhausted".into());
        }
        let id = self.next_id;
        self.next_id += 1;
        let name = name.unwrap_or_else(|| self.free_name("Mesh"));
        // Preferences > Defaults > Mesh Items.
        let d = crate::user_defaults::current();
        let mut mesh = Mesh::default();
        if d.auto_uv_map {
            mesh.uv_maps.insert(d.texture_map.clone(), Vec::new());
        }
        let factory = crate::subdivision::SubdivisionSettings::default();
        let subdivision = (d.subdivision_level != factory.subdivision_level
            || d.catmull_clark_level != factory.catmull_clark_level)
            .then_some(crate::subdivision::SubdivisionSettings {
                subdivision_level: d.subdivision_level,
                catmull_clark_level: d.catmull_clark_level,
                ..factory
            });
        if d.auto_item_masks {
            self.shader_tree.add_item_mask(id, &name)?;
        }
        self.items.push(Item {
            id,
            name,
            visible: true,
            transform: Transform::default(),
            mesh: mesh.into(),
            parent: None,
            source_id: None,
            source_display_name: None,
            subdivision,
            backdrop: None,
            camera: None,
            light: None,
            texture: None,
            locator: None,
            environment: None,
            common: Default::default(),
            elements: Default::default(),
            hides_children: false,
            mesh_stack: None,
            kit: None,
            instance_of: None,
            replicator: None,
        });
        self.selection = Selection {
            item: Some(id),
            mode: self.selection.mode,
            ..Default::default()
        };
        Ok(())
    }
    pub fn select_item(&mut self, id: Option<u64>) {
        if id != self.selection.item || !self.selection.also.is_empty() {
            // VP-04: the mesh being let go stays active while nothing is.
            self.note_last_mesh();
            self.selection = Selection {
                item: id,
                mode: self.selection.mode,
                // A tag mode stays on; its regions belonged to the old item.
                tag: self.selection.tag.as_ref().map(|t| crate::TagSelection {
                    kind: t.kind,
                    regions: Vec::new(),
                }),
                ..Default::default()
            };
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.guides.validate()?;
        self.animation.validate()?;
        self.validate_with_range_marker(&mut false)
    }
    /// Every selected item, first-selected first, skipping stale ids.
    pub fn selected_items(&self) -> Vec<u64> {
        let mut out: Vec<u64> = self.selection.item.into_iter().collect();
        // A set, not a search of `out`: thousands of items get selected.
        let mut seen: crate::fxhash::FxHashSet<u64> = out.iter().copied().collect();
        for &id in &self.selection.also {
            if self.item(id).is_some() && seen.insert(id) {
                out.push(id);
            }
        }
        out
    }
    /// Add `id` to the item selection, or take it out if it's in already
    /// (Ctrl/Shift-click). With nothing selected it becomes the item.
    pub fn toggle_item(&mut self, id: u64) {
        if self.selection.item == Some(id) {
            let mut rest = std::mem::take(&mut self.selection.also);
            let mut others = std::mem::take(&mut self.selection.others);
            let next = if rest.is_empty() {
                None
            } else {
                Some(rest.remove(0))
            };
            self.select_item(next);
            self.selection.also = rest;
            // The next item becomes primary with the components it had.
            if let Some(next) = next {
                let c = others.remove(&next).unwrap_or_default();
                self.selection.vertices = c.vertices;
                self.selection.edges = c.edges;
                self.selection.faces = c.faces;
            }
            self.selection.others = others;
        } else if let Some(at) = self.selection.also.iter().position(|&a| a == id) {
            self.selection.also.remove(at);
            if !self.selection.also.contains(&id) {
                self.selection.others.remove(&id);
            }
        } else if self.selection.item.is_none() {
            self.select_item(Some(id));
        } else {
            self.selection.also.push(id);
        }
    }
    pub(crate) fn validate_for_import(&self) -> std::result::Result<(), SceneValidationError> {
        let mut finite_transform_range = false;
        self.validate_with_range_marker(&mut finite_transform_range)
            .map_err(|detail| {
                if finite_transform_range {
                    SceneValidationError::FiniteTransformRange(detail)
                } else {
                    SceneValidationError::Other(detail)
                }
            })
    }
    fn validate_with_range_marker(&self, finite_transform_range: &mut bool) -> Result<()> {
        if self.items.len() > crate::MAX_SCENE_ITEMS || self.next_id == u64::MAX {
            return Err("Scene item limit".into());
        }
        self.smoothing.validate()?;
        self.time.validate()?;
        if !ITEM_DRAW_SIZE_RANGE.contains(&self.item_draw_size) {
            return Err("Invalid item drawing size".into());
        }
        crate::material::validate_materials(&self.materials)?;
        self.shader_tree.validate()?;
        let mut ids = BTreeSet::new();
        for i in &self.items {
            if i.id == 0 || i.id >= self.next_id || !ids.insert(i.id) || i.name.len() > 1_048_576 {
                return Err("Invalid item identity or name".into());
            }
            if let Some(stack) = &i.mesh_stack {
                stack.validate()?;
            }
            if let Some(rep) = &i.replicator {
                rep.validate()?;
                if !i.mesh.vertices.is_empty() || i.mesh_stack.is_some() {
                    return Err("A Replicator holds no mesh or Mesh Stack".into());
                }
            }
            if let Some(kit) = &i.kit {
                kit.validate()?;
                if !i.mesh.vertices.is_empty() || i.mesh_stack.is_some() {
                    return Err("A kit item holds no mesh or Mesh Stack".into());
                }
            }
            if !i.transform.translation.is_finite()
                || !i.transform.rotation.is_finite()
                || !i.transform.scale.is_finite()
                || i.transform.scale.abs().min_element() <= 1e-8
                || i.transform.scale.abs().max_element() > 1e8
                || i.transform.translation.abs().max_element() > 1e9
                || i.transform.rotation.abs().max_element() > 1e9
            {
                *finite_transform_range = i.transform.translation.is_finite()
                    && i.transform.rotation.is_finite()
                    && i.transform.scale.is_finite();
                return Err("Invalid item transform".into());
            }
            i.mesh.validate()?;
            let world_matrix = self.world_matrix(i.id)?;
            // The farthest point from the origin bounds every world point
            // (remembered per mesh allocation), so a drag frame skips placing
            // each point unless the bound comes near the budget.
            let reach = i.mesh.shared().reach();
            let within = (0..3).all(|row| {
                let linear: f64 = (0..3).map(|c| world_matrix.col(c)[row].abs()).sum();
                let far = linear * reach + world_matrix.col(3)[row].abs();
                far.is_finite() && far <= 1e9 * (1. - 1e-9)
            });
            if !within
                && i.mesh.vertices.iter().any(|&p| {
                    let world = world_matrix.transform_point3(p);
                    !world.is_finite() || world.abs().max_element() > 1e9
                })
            {
                return Err(
                    "Item transform exceeds the finite world-coordinate budget (1,000,000 km)"
                        .into(),
                );
            }
        }
        for i in &self.items {
            self.world_matrix(i.id)?;
        }
        self.validate_backdrops()?;
        self.validate_cameras_lights()?;
        self.validate_environments()?;
        self.validate_groups()?;
        for i in &self.items {
            i.common.validate()?;
        }
        if self.workplane.plane > 2 || !self.workplane.center.is_finite() {
            return Err("Invalid workplane".into());
        }
        if let Some(id) = self.selection.item {
            let item = self.item(id).ok_or("Invalid selected item")?;
            crate::multi_active::check_components(
                &item.mesh,
                &self.selection.vertices,
                &self.selection.edges,
                &self.selection.faces,
            )?;
        } else if !self.selection.vertices.is_empty()
            || !self.selection.edges.is_empty()
            || !self.selection.faces.is_empty()
        {
            return Err("Components without an item".into());
        }
        self.validate_other_components()?;
        Ok(())
    }
}

pub(crate) enum SceneValidationError {
    FiniteTransformRange(String),
    Other(String),
}

/// A hit is geometric evidence supplied by the viewport; selection semantics
/// and validation remain portable domain operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Component {
    Vertex(u32),
    Edge(u32, u32),
    Polygon(u32),
    Item,
}
impl Scene {
    pub fn select_component(
        &mut self,
        item: u64,
        component: Component,
        replace: bool,
        remove: bool,
    ) -> Result<()> {
        let target = self
            .items
            .iter()
            .find(|i| i.id == item && self.shows(i))
            .ok_or("Select a visible mesh")?;
        match component {
            Component::Vertex(v) if v as usize >= target.mesh.vertices.len() => {
                return Err("Invalid vertex hit".into());
            }
            Component::Edge(a, b)
                if target
                    .mesh
                    .edges()
                    .binary_search(&(a.min(b), a.max(b)))
                    .is_err() =>
            {
                return Err("Invalid edge hit".into());
            }
            Component::Polygon(f) if f as usize >= target.mesh.faces.len() => {
                return Err("Invalid polygon hit".into());
            }
            _ => {}
        }
        // Tag modes pick whole regions (click and lasso alike).
        if let Component::Polygon(f) = component
            && self.tag_mode().is_some()
        {
            return self.select_tag_region(item, f, replace, remove);
        }
        // Every selected mesh is active (multi_active.rs): a component
        // pick on any of them goes to that mesh's selection and leaves the
        // item selection as it is.
        if !matches!(component, Component::Item) && self.active_meshes().contains(&item) {
            self.select_active_component(item, component, replace, remove);
            return Ok(());
        }
        // Item clicks (Jason, 2026-11-08): Shift adds the item to the item
        // selection, so several meshes are active at once; Ctrl (`remove`)
        // toggles it in or out; a plain click selects only it.
        if matches!(component, Component::Item) && !replace {
            if remove || !self.selected_items().contains(&item) {
                self.toggle_item(item);
            }
            return Ok(());
        }
        if remove {
            if self.selection.item != Some(item) {
                return Ok(());
            }
            if matches!(component, Component::Item) {
                self.select_item(None);
                return Ok(());
            }
        }
        self.select_item(Some(item));
        fn edit<T: PartialEq>(list: &mut Vec<T>, value: T, replace: bool, remove: bool) {
            if replace {
                list.clear();
            }
            if remove {
                list.retain(|v| v != &value);
            } else if !list.contains(&value) {
                list.push(value);
            }
        }
        match component {
            Component::Vertex(v) => edit(&mut self.selection.vertices, v, replace, remove),
            Component::Edge(a, b) => edit(
                &mut self.selection.edges,
                (a.min(b), a.max(b)),
                replace,
                remove,
            ),
            Component::Polygon(f) => edit(&mut self.selection.faces, f, replace, remove),
            Component::Item => {}
        }
        Ok(())
    }
    /// Clear the current mode's components on every active mesh (the
    /// item selection in Item mode).
    pub fn clear_components(&mut self) {
        match self.selection.mode {
            mode @ (Mode::Vertex | Mode::Edge) => self.clear_active_components(mode),
            Mode::Polygon => {
                self.clear_active_components(Mode::Polygon);
                if let Some(tag) = self.selection.tag.as_mut() {
                    tag.regions.clear();
                }
            }
            Mode::Item => self.select_item(None),
        }
    }
}

/// Read-only provenance, persisted through native save and recovery. Never a save path.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportSource {
    pub path: String,
    pub producer: String,
    pub device: Option<u64>,
    pub inode: Option<u64>,
}
impl Scene {
    pub fn world_matrix(&self, id: u64) -> Result<DMat4> {
        self.world_matrix_aimed(id, 0)
    }
    /// The matrix that takes world-space points into item `id`'s local
    /// space. Primitives are built in world space (where the user drew them)
    /// and mapped through this before they join a moved, rotated, scaled or
    /// parented mesh, so they land where drawn and the item keeps its
    /// transform. A transform that flattens an axis (zero scale) has no
    /// inverse and is refused.
    pub fn world_to_local(&self, id: u64) -> Result<DMat4> {
        let world = self.world_matrix(id)?;
        if world == DMat4::IDENTITY {
            return Ok(world);
        }
        let inverse = world.inverse();
        if world.determinant() == 0. || !world.determinant().is_finite() || !inverse.is_finite() {
            return Err(
                "This mesh's transform squashes it flat (a zero scale), so new geometry can't be placed in it; fix its Scale or use a new mesh".into(),
            );
        }
        Ok(inverse)
    }
    /// `mesh` (built in world space) mapped into item `id`'s local space;
    /// see `world_to_local`. Unchanged for an untransformed item.
    pub fn mesh_world_to_local(&self, id: u64, mut mesh: crate::Mesh) -> Result<crate::Mesh> {
        let local = self.world_to_local(id)?;
        if local != DMat4::IDENTITY {
            for v in &mut mesh.vertices {
                *v = local.transform_point3(*v);
            }
        }
        Ok(mesh)
    }
    /// `world_matrix`, `aim_depth` levels deep into targets being looked
    /// up (item_target.rs bounds it so targets aiming at each other end).
    /// Where item `id` is in `items` (remembered, so a view asking for
    /// every item's is not a search per item).
    pub fn item_index(&self, id: u64) -> Option<usize> {
        self.index.find(&self.items, id)
    }
    /// Item `id`, found through `item_index`.
    pub fn item(&self, id: u64) -> Option<&Item> {
        self.item_index(id).map(|i| &self.items[i])
    }
    /// Item `id` to change, found through `item_index`.
    pub fn item_mut(&mut self, id: u64) -> Option<&mut Item> {
        self.item_index(id).map(|i| &mut self.items[i])
    }
    /// Whether `id` names something with channels: an item or a rig piece
    /// (rig.rs).
    pub fn owns_channels(&self, id: u64) -> bool {
        self.item(id).is_some() || self.animation.rig.piece(id).is_some()
    }
    /// An item's local matrix, rotating and scaling about its pivot
    /// (RG-14, rig.rs).
    pub fn local_matrix(&self, item: &Item) -> DMat4 {
        let m = item.transform.matrix();
        match self.animation.rig.pivots.get(&item.id) {
            Some(p) => p.apply(m),
            None => m,
        }
    }
    pub(crate) fn world_matrix_aimed(&self, id: u64, aim_depth: u32) -> Result<DMat4> {
        let mut chain: Vec<&Item> = Vec::new();
        let mut next = Some(id);
        while let Some(id) = next {
            if chain.len() >= 256 || chain.iter().any(|seen| seen.id == id) {
                return Err("Hierarchy cycle or depth limit".into());
            }
            let item = self.item(id).ok_or("Dangling parent reference")?;
            chain.push(item);
            next = item.parent;
        }
        let mut matrix = DMat4::IDENTITY;
        for item in chain.iter().rev() {
            matrix *= self.local_matrix(item);
            // A target or a Physical Sun turns the item in world space
            // (LP-04, LP-SUN-01); its children follow.
            if let Some(rotation) = self.driven_rotation(item, matrix, aim_depth) {
                let position = matrix.w_axis.truncate();
                matrix = DMat4::from_scale_rotation_translation(
                    item.transform.scale,
                    rotation,
                    position,
                );
            }
        }
        if !matrix.is_finite() {
            return Err("Nonfinite world transform".into());
        }
        Ok(matrix)
    }
    /// SEL-IT-01..03. Refuses a missing item, itself or a descendant as
    /// parent. Keeping the position decomposes the new local matrix into
    /// translation, rotation (in the item's rotation order) and scale; shear
    /// from a non-uniformly scaled parent can't be kept and is refused.
    pub fn set_parent(
        &mut self,
        item: u64,
        parent: Option<u64>,
        keep_position: bool,
    ) -> Result<()> {
        if !self.item(item).is_some() {
            return Err("Select an item to parent".into());
        }
        if let Some(p) = parent {
            if p == item {
                return Err("An item can't be its own parent".into());
            }
            let mut next = Some(p);
            while let Some(id) = next {
                if id == item {
                    return Err("An item can't be parented to its own child".into());
                }
                next = self
                    .item(id)
                    .ok_or("Choose an existing parent item")?
                    .parent;
            }
        }
        let world = self.world_matrix(item)?;
        let index = self.item_index(item).unwrap_or(0);
        let previous = self.items[index].clone();
        self.items[index].parent = parent;
        if keep_position {
            let parent_world = match parent {
                Some(p) => self.world_matrix(p)?,
                None => DMat4::IDENTITY,
            };
            if parent_world.determinant().abs() < 1e-12 {
                self.items[index] = previous;
                return Err("The parent's transform can't be inverted".into());
            }
            let local = parent_world.inverse() * world;
            let (scale, rotation, translation) = local.to_scale_rotation_translation();
            let transform = Transform {
                translation,
                rotation: previous.transform.angles_for(rotation),
                scale,
                ..previous.transform.clone()
            };
            if !transform.matrix().abs_diff_eq(
                local,
                1e-9 * local
                    .abs()
                    .to_cols_array()
                    .iter()
                    .fold(1., |a: f64, &b| a.max(b)),
            ) {
                self.items[index] = previous;
                return Err("Keeping the position here would need shear; parent without keeping the position".into());
            }
            self.items[index].transform = transform;
        }
        Ok(())
    }
    /// Every item id in tree order: roots in list order, each followed by
    /// its children depth-first. Sibling order is the order of `items`.
    pub fn tree_order(&self) -> Vec<u64> {
        let exists = |id: u64| self.item(id).is_some();
        let mut out = Vec::with_capacity(self.items.len());
        let mut stack: Vec<u64> = self
            .items
            .iter()
            .rev()
            .filter(|i| i.parent.is_none_or(|p| !exists(p)))
            .map(|i| i.id)
            .collect();
        while let Some(id) = stack.pop() {
            if out.contains(&id) || out.len() > self.items.len() {
                break;
            }
            out.push(id);
            stack.extend(
                self.items
                    .iter()
                    .rev()
                    .filter(|c| c.parent == Some(id))
                    .map(|c| c.id),
            );
        }
        out
    }
    /// UIC-143 tree drops: make `items` children of `parent` (None: top
    /// level), placed just before the sibling `before` (None: after the
    /// last child). Sibling order is the order of `self.items`, which the
    /// native file stores as is.
    ///
    /// The moved items keep their relative order as listed (tree order).
    /// An item whose ancestor is also moved goes along inside it rather
    /// than being pulled out. Reparented items keep their world position,
    /// like a drag-parent. Dropping into an item's own branch is refused.
    /// Returns whether any parent changed (else it was a reorder). On an
    /// error nothing changes.
    pub fn move_items(
        &mut self,
        items: &[u64],
        parent: Option<u64>,
        before: Option<u64>,
    ) -> Result<bool> {
        if items.is_empty() {
            return Err("Select items to move".into());
        }
        fn parent_of(s: &Scene, id: u64) -> Option<u64> {
            s.item(id).and_then(|i| i.parent)
        }
        for &id in items {
            if !self.item(id).is_some() {
                return Err("Missing item".into());
            }
        }
        // Is `id` one of the moved items or inside one (`skip_self`: only
        // its ancestors count)?
        let inside_moved = |s: &Self, id: u64, skip_self: bool| {
            let mut next = if skip_self {
                parent_of(s, id)
            } else {
                Some(id)
            };
            let mut steps = 0;
            while let Some(at) = next {
                if items.contains(&at) {
                    return true;
                }
                steps += 1;
                if steps > s.items.len() {
                    break;
                }
                next = parent_of(s, at);
            }
            false
        };
        if let Some(p) = parent {
            if !self.item(p).is_some() {
                return Err("Choose an existing parent item".into());
            }
            if inside_moved(self, p, false) {
                return Err("An item can't be moved into its own branch".into());
            }
        }
        // The moved set in tree order; an item inside another moved item
        // stays inside it.
        let moved: Vec<u64> = self
            .tree_order()
            .into_iter()
            .filter(|&id| items.contains(&id) && !inside_moved(self, id, true))
            .collect();
        // The sibling to go in front of; when that is itself moved, the
        // next unmoved sibling after it.
        let anchor = match before {
            None => None,
            Some(b) => {
                let start = self.item_index(b).ok_or("Missing item")?;
                if self.items[start].parent != parent {
                    return Err("The drop position is not under that parent".into());
                }
                self.items[start..]
                    .iter()
                    .find(|i| i.parent == parent && !moved.contains(&i.id))
                    .map(|i| i.id)
            }
        };
        let saved = self.items.clone();
        let mut reparented = false;
        for &id in &moved {
            if parent_of(self, id) != parent {
                // LCK-05: a locked item keeps its links.
                if let Err(e) = self
                    .refuse_locked(&[id])
                    .and_then(|()| self.set_parent(id, parent, true))
                {
                    self.items = saved;
                    return Err(e);
                }
                reparented = true;
            }
        }
        let mut taken: Vec<Item> = Vec::with_capacity(moved.len());
        for &id in &moved {
            if let Some(index) = self.item_index(id) {
                taken.push(self.items.remove(index));
            }
        }
        let at = anchor
            .and_then(|a| self.item_index(a))
            .unwrap_or(self.items.len());
        self.items.splice(at..at, taken);
        Ok(reparented)
    }
    /// An item and all its descendants, root first, ready to go into
    /// another scene: the root is unparented with its world transform
    /// (its local one when that would need shear).
    pub fn branch(&self, root: u64) -> Result<Vec<Item>> {
        if !self.item(root).is_some() {
            return Err("Missing item".into());
        }
        let mut baked = self.clone();
        if baked.set_parent(root, None, true).is_err() {
            baked.set_parent(root, None, false)?;
        }
        let mut out: Vec<Item> = Vec::new();
        let mut queue = vec![root];
        while let Some(id) = queue.pop() {
            if out.iter().any(|i| i.id == id) || out.len() > 4096 {
                return Err("Hierarchy cycle or depth limit".into());
            }
            let item = baked.item(id).ok_or("Dangling parent reference")?;
            out.push(item.clone());
            queue.extend(
                baked
                    .items
                    .iter()
                    .rev()
                    .filter(|i| i.parent == Some(id))
                    .map(|i| i.id),
            );
        }
        Ok(out)
    }
    pub fn world_point(&self, id: u64, point: DVec3) -> DVec3 {
        self.world_matrix(id)
            .map(|m| m.transform_point3(point))
            .unwrap_or(DVec3::NAN)
    }
}

#[cfg(test)]
mod range_classification_tests {
    use super::*;
    #[test]
    fn public_error_and_precedence_are_preserved() {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.items[0].transform.translation.x = 1e10;
        assert_eq!(scene.validate().unwrap_err(), "Invalid item transform");
        assert!(matches!(
            scene.validate_for_import(),
            Err(SceneValidationError::FiniteTransformRange(_))
        ));
        scene.items[0].id = 0;
        assert_eq!(
            scene.validate().unwrap_err(),
            "Invalid item identity or name"
        );
        assert!(matches!(
            scene.validate_for_import(),
            Err(SceneValidationError::Other(_))
        ));
        scene.items[0].id = 1;
        scene.items[0].transform.translation.x = f64::NAN;
        assert_eq!(scene.validate().unwrap_err(), "Invalid item transform");
        assert!(matches!(
            scene.validate_for_import(),
            Err(SceneValidationError::Other(_))
        ));
        scene.items[0].transform.translation.x = 0.0;
        scene.workplane.plane = 3;
        assert_eq!(scene.validate().unwrap_err(), "Invalid workplane");
        assert!(matches!(
            scene.validate_for_import(),
            Err(SceneValidationError::Other(_))
        ));
    }
}
