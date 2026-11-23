//! Texture Locators (docs/specs/BEHAVIOR-TEXTURE-LOCATOR.md, stage 1).
//!
//! Every texture layer places its texture through a Texture Locator item: a
//! normal scene item (Position, Rotation, Scale = Size) carrying the
//! projection settings. Locators live under one "Texture Group" item.
//!
//! Locator space: `L = inverse(locator world matrix) · P`, where P is the
//! shading point in the surface item's own space (World Coordinates off) or
//! in world space (on). A 1 m locator at the origin gives L = P in metres.
//!
//! Every projection type renders (Front, Light Probe and Implicit UV since
//! shader-tree-complete, provisional TL-Q30..Q32). Random Offsets, Real
//! World Size, Tangent and Particle Source are stored only.
use crate::Result;
use crate::scene::{Item, Scene, Transform};
use crate::shader_tree::{NodeId, NodeKind, Projection, TextureSource};
use glam::{DMat3, DMat4, DVec3};
use serde::{Deserialize, Serialize};

macro_rules! labelled_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub const ALL: &[$name] = &[$($name::$variant),+];
            pub fn label(self) -> &'static str {
                match self { $($name::$variant => $label),+ }
            }
        }
    };
}

labelled_enum! {
    /// What happens outside the 0..1 tile, per axis (Horizontal / Vertical
    /// Repeat), applied after the repeat counts.
    Tiling {
        Reset => "Reset",
        Repeat => "Repeat",
        Mirror => "Mirror",
        Edge => "Edge",
    }
}

/// An enum with display names; `alias` keeps the names stage 1 saved
/// (provisional placeholders, native version 19) readable.
macro_rules! named_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[serde(alias = $alias:literal)])? $variant:ident => $label:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name { $($(#[serde(alias = $alias)])? $variant),+ }
        impl $name {
            pub const ALL: &[$name] = &[$($name::$variant),+];
            pub fn label(self) -> &'static str {
                match self { $($name::$variant => $label),+ }
            }
        }
    };
}

named_enum! {
    /// Front Culling (spec §6). "Toward" = the normal's dot with the
    /// projection axis direction > 0. Renders for Planar (TL-Q12).
    FrontCulling {
        ProjectThrough => "Project Through",
        #[serde(alias = "FrontOnly")]
        CullBackfacing => "Cull Backfacing",
        FlipBackfacing => "Flip Backfacing",
        FlipUBackfacing => "Flip U Backfacing",
        FlipVBackfacing => "Flip V Backfacing",
    }
}

named_enum! {
    /// Random Offsets (spec §6; stored only).
    RandomOffsets {
        None => "(none)",
        #[serde(alias = "Item")]
        ParticlesAndReplicas => "Particles and Replicas",
        #[serde(alias = "Part")]
        MeshParts => "Mesh Parts",
        Surfaces => "Surfaces",
    }
}

named_enum! {
    /// Falloff shape (spec §6).
    FalloffType {
        Cubic => "Cubic",
        Spherical => "Spherical",
        #[serde(alias = "Linear")]
        LinearX => "Linear X",
        LinearY => "Linear Y",
        LinearZ => "Linear Z",
    }
}

named_enum! {
    /// Tangent Vector Type (spec §6; stored only, belongs with normal maps).
    TangentType {
        DpduDpdv => "dPdu dPdv",
        DpduCross => "dPdu Cross Product",
    }
}

/// A Texture Locator's settings besides its transform (the item's
/// Position, Rotation and Scale; Size X/Y/Z is the scale).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextureLocator {
    pub projection: Projection,
    /// 0, 1, 2 = X, Y, Z.
    pub axis: u8,
    /// `None`: the mesh's first UV map.
    pub uv_map: Option<String>,
    /// Horizontal / Vertical repeat counts (U, V).
    pub repeat: [f64; 2],
    /// UV Rotation, degrees, about (0, 0).
    pub uv_rotation: f64,
    /// Horizontal / Vertical tiling.
    pub tiling: [Tiling; 2],
    pub world_coordinates: bool,
    /// Stored only.
    pub world_transforms: bool,
    pub front_culling: FrontCulling,
    /// 0..1 (shown as percent). Stored only.
    pub triplanar_hardness: f64,
    pub random_offsets: RandomOffsets,
    /// Texture Offset Strength, 0..10 (stored only).
    pub random_strength: f64,
    pub falloff_type: FalloffType,
    /// Falloff X/Y/Z: the layer is gone at Size / f from the centre;
    /// 0 is off.
    pub falloff: [f64; 3],
    pub tangent: TangentType,
    pub real_world_size: bool,
    /// Particle Source item; stored only.
    pub particle_source: Option<u64>,
    /// Use Real World Size's Size X/Y (stored only; native version 23).
    #[serde(default = "unit_pair")]
    pub real_world_size_xy: [f64; 2],
    /// Texture Replicator item (stored only).
    #[serde(default)]
    pub replicator: Option<u64>,
    /// Front projection's Projection Camera (`None`: the render camera).
    #[serde(default)]
    pub projection_camera: Option<u64>,
    /// U / V Tile Offset, in tiles (stored only; native version 43).
    #[serde(default, skip_serializing_if = "is_zero_pair")]
    pub tile_offset: [f64; 2],
}

fn is_zero_pair(v: &[f64; 2]) -> bool {
    *v == [0.0; 2]
}

fn unit_pair() -> [f64; 2] {
    [1., 1.]
}

impl Default for TextureLocator {
    fn default() -> Self {
        Self::new(Projection::Solid)
    }
}

impl TextureLocator {
    /// A new locator's starting values (spec §2).
    pub fn new(projection: Projection) -> Self {
        Self {
            projection,
            axis: 0,
            uv_map: None,
            repeat: [1., 1.],
            uv_rotation: 0.,
            tiling: [Tiling::Repeat, Tiling::Repeat],
            world_coordinates: false,
            world_transforms: true,
            front_culling: FrontCulling::ProjectThrough,
            triplanar_hardness: 0.,
            random_offsets: RandomOffsets::None,
            random_strength: 0.,
            falloff_type: FalloffType::Cubic,
            falloff: [0.; 3],
            tangent: TangentType::DpduDpdv,
            real_world_size: false,
            particle_source: None,
            real_world_size_xy: [1., 1.],
            replicator: None,
            projection_camera: None,
            tile_offset: [0., 0.],
        }
    }
    pub fn validate(&self) -> Result<()> {
        let finite = |v: f64| v.is_finite();
        if self.axis > 2
            || !self.repeat.iter().all(|&r| finite(r) && r.abs() <= 1e6)
            || !self
                .tile_offset
                .iter()
                .all(|&o| finite(o) && o.abs() <= 1e6)
            || !finite(self.uv_rotation)
            || self.uv_rotation.abs() > 1e6
            || !self
                .real_world_size_xy
                .iter()
                .all(|&v| finite(v) && v > 0. && v <= 1e6)
            || !finite(self.triplanar_hardness)
            || !(0.0..=1.0).contains(&self.triplanar_hardness)
            || !finite(self.random_strength)
            || !(0.0..=10.0).contains(&self.random_strength)
            || !self
                .falloff
                .iter()
                .all(|&f| finite(f) && (0.0..=1e9).contains(&f))
            || self
                .uv_map
                .as_ref()
                .is_some_and(|m| m.trim().is_empty() || m.len() > 1024)
        {
            return Err("Invalid Texture Locator settings".into());
        }
        Ok(())
    }
    /// What this locator renders as. Every type renders now; Front without
    /// a usable camera falls back to UV Map where it is placed
    /// (`shader_eval::Placement::of`).
    pub fn rendered_projection(&self) -> Projection {
        self.projection
    }
    /// Whether the projection reads mesh UVs, so Group Scale divides its
    /// repeat counts rather than growing its Size (UV Map, Implicit UV).
    pub fn reads_uvs(&self) -> bool {
        matches!(self.projection, Projection::UvMap | Projection::ImplicitUv)
    }
    /// Triplanar's weight exponent: k = 1 + 31 h² (spec §4).
    pub fn triplanar_exponent(&self) -> f64 {
        1. + 31. * self.triplanar_hardness * self.triplanar_hardness
    }
}

/// The smallest Size on any axis (Planar spec 2026-11-09: Size clamps at
/// 0.001 m), whichever way it is set: tools, typed channels, the panel.
pub const MIN_SIZE: f64 = 1e-3;

/// `scale` with every axis at least `MIN_SIZE` from zero (its sign kept;
/// zero becomes +MIN_SIZE).
pub fn clamp_size(scale: DVec3) -> DVec3 {
    scale.map(|s| {
        if s.abs() >= MIN_SIZE {
            s
        } else if s < 0. {
            -MIN_SIZE
        } else {
            MIN_SIZE
        }
    })
}

/// Whether Projection Axis applies to a projection type (it is greyed
/// otherwise).
pub fn uses_axis(p: Projection) -> bool {
    matches!(
        p,
        Projection::Planar
            | Projection::Cylindrical
            | Projection::Spherical
            | Projection::Cubic
            | Projection::Box
            | Projection::LightProbe
    )
}

/// The item-side part of a texture locator item.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TextureItem {
    /// A Texture Locator.
    Locator(Box<TextureLocator>),
    /// The "Texture Group" item new locators are put in.
    Group,
}

/// The reserved UV map name Implicit UV reads (TL-Q32, provisional):
/// renderers fill it from `implicit_uv`, never from a stored map.
pub const IMPLICIT_UV_MAP: &str = "operandi:implicit-uv";

/// Implicit UV (TL-Q32, provisional): Operandi has no fur fibres, so each
/// polygon is its own 0..1 patch. A triangle's corners are (0,0) (1,0)
/// (0,1); a quad's the unit square's, counter-clockwise from (0,0); a
/// larger polygon's sit evenly on the circle inside the unit square,
/// starting at the bottom left.
pub fn implicit_uv(corners: usize, corner: usize) -> glam::DVec2 {
    use glam::DVec2;
    match corners {
        3 => [DVec2::ZERO, DVec2::X, DVec2::Y][corner.min(2)],
        4 => [DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y][corner.min(3)],
        n if n > 4 => {
            let a = std::f64::consts::TAU * corner as f64 / n as f64 - 0.75 * std::f64::consts::PI;
            DVec2::splat(0.5) + 0.5 * DVec2::new(a.cos(), a.sin())
        }
        _ => DVec2::ZERO,
    }
}

/// Name of the group item new locators go into.
pub const TEXTURE_GROUP_NAME: &str = "Texture Group";

// ---------------------------------------------------------------------------
// Maths

/// A point in locator space from a point in the space `to_locator` maps.
pub fn locator_point(to_locator: &DMat4, p: DVec3) -> DVec3 {
    to_locator.transform_point3(p)
}

/// The planar image coordinates of a locator-space point (spec §4): axis Z
/// `(Lx, Ly)`, X `(-Lz, Ly)`, Y `(Lx, -Lz)`, plus `centre` (0.5 for images,
/// so the image is centred on the locator; 0 for procedurals).
pub fn planar(axis: u8, l: DVec3, centre: f64) -> [f64; 2] {
    match axis {
        0 => [-l.z + centre, l.y + centre],
        1 => [l.x + centre, -l.z + centre],
        _ => [l.x + centre, l.y + centre],
    }
}

/// Mesh UVs times the repeat counts, then rotated `degrees` about (0, 0)
/// (counter-clockwise for positive angles).
pub fn uv_transform(uv: [f64; 2], repeat: [f64; 2], degrees: f64) -> [f64; 2] {
    let (u, v) = (uv[0] * repeat[0], uv[1] * repeat[1]);
    if degrees == 0. {
        return [u, v];
    }
    let (s, c) = degrees.to_radians().sin_cos();
    [u * c - v * s, u * s + v * c]
}

/// One axis's tiling (spec §5): `None` where Reset leaves the layer
/// transparent.
pub fn tile(t: f64, mode: Tiling) -> Option<f64> {
    match mode {
        Tiling::Reset => (0.0..=1.0).contains(&t).then_some(t),
        Tiling::Repeat => Some(t - t.floor()),
        // A triangle wave: 0..1 on even tiles, 1..0 on odd ones.
        Tiling::Mirror => {
            let f = (t * 0.5 - (t * 0.5).floor()) * 2.;
            Some(if f > 1. { 2. - f } else { f })
        }
        Tiling::Edge => Some(t.clamp(0., 1.)),
    }
}

/// Both axes tiled, `None` where the layer is transparent.
pub fn tile_uv(uv: [f64; 2], tiling: [Tiling; 2]) -> Option<[f64; 2]> {
    Some([tile(uv[0], tiling[0])?, tile(uv[1], tiling[1])?])
}

/// A locator as seen from one surface item: its settings and the matrix
/// taking that item's shading points (world positions) into locator space.
#[derive(Clone, Debug, PartialEq)]
pub struct LocatorFrame {
    pub id: u64,
    pub settings: TextureLocator,
    /// World position -> locator space for this item (World Coordinates
    /// off folds the item's own inverse transform in).
    pub world_to_locator: DMat4,
    /// Box: world position -> the locator's axis-aligned space (its
    /// position and Size, not its rotation).
    pub world_to_box: DMat4,
    /// World normal -> locator space (Cubic, Triplanar, culling).
    pub normal_to_locator: DMat3,
    /// World normal -> the point space (item or world; Box).
    pub normal_to_space: DMat3,
    /// Front: world position -> (u·w, v·w, w), where (u, v) runs 0..1
    /// across the Projection Camera's picture (`front_matrix`). `None`
    /// for other types or without a usable camera.
    pub front: Option<DMat4>,
}
impl LocatorFrame {
    /// A frame with every matrix the identity (the viewport's averages,
    /// which don't place anything).
    pub fn unplaced(id: u64, settings: TextureLocator) -> Self {
        Self {
            id,
            settings,
            world_to_locator: DMat4::IDENTITY,
            world_to_box: DMat4::IDENTITY,
            normal_to_locator: DMat3::IDENTITY,
            normal_to_space: DMat3::IDENTITY,
            front: None,
        }
    }
}

impl Item {
    pub fn texture_locator(&self) -> Option<&TextureLocator> {
        match &self.texture {
            Some(TextureItem::Locator(l)) => Some(l),
            _ => None,
        }
    }
    pub fn is_texture_item(&self) -> bool {
        self.texture.is_some()
    }
}

impl Scene {
    /// Clamp every Texture Locator's Size (`clamp_size`).
    pub fn clamp_locator_sizes(&mut self) {
        for item in self
            .items
            .iter_mut()
            .filter(|i| i.texture_locator().is_some())
        {
            item.transform.scale = clamp_size(item.transform.scale);
        }
    }
    pub fn texture_locator(&self, id: u64) -> Option<&TextureLocator> {
        self.item(id).and_then(Item::texture_locator)
    }
    /// The matrix taking world positions on `item` into the space of
    /// locator `locator`.
    pub fn world_to_locator(&self, locator: u64, item: u64) -> Option<DMat4> {
        let settings = self.texture_locator(locator)?;
        let to_locator = self.world_matrix(locator).ok()?.inverse();
        if settings.world_coordinates {
            return Some(to_locator);
        }
        let item_inverse = self.world_matrix(item).ok()?.inverse();
        Some(to_locator * item_inverse)
    }
    /// Every locator the shader tree's texture layers use, as seen from
    /// surface item `item`.
    pub fn locator_frames(&self, item: u64) -> Vec<LocatorFrame> {
        let mut out: Vec<LocatorFrame> = Vec::new();
        for node in &self.shader_tree.nodes {
            let NodeKind::Texture(t) = &node.kind else {
                continue;
            };
            let Some(id) = t.locator else { continue };
            if out.iter().any(|f| f.id == id) {
                continue;
            }
            let (Some(settings), Some(m)) =
                (self.texture_locator(id), self.world_to_locator(id, item))
            else {
                continue;
            };
            let Ok(locator_world) = self.world_matrix(id) else {
                continue;
            };
            let space_from_world = if settings.world_coordinates {
                DMat4::IDENTITY
            } else {
                match self.world_matrix(item) {
                    Ok(w) => w.inverse(),
                    Err(_) => continue,
                }
            };
            // Box keeps the locator's position and Size, not its rotation.
            let (scale, _, translation) = locator_world.to_scale_rotation_translation();
            let world_to_box =
                DMat4::from_scale_rotation_translation(scale, glam::DQuat::IDENTITY, translation)
                    .inverse()
                    * space_from_world;
            let normal_of = |m: &DMat4| DMat3::from_mat4(*m).inverse().transpose();
            let frame = LocatorFrame {
                id,
                settings: settings.clone(),
                world_to_locator: m,
                world_to_box,
                normal_to_locator: normal_of(&m),
                normal_to_space: normal_of(&space_from_world),
                front: (settings.projection == Projection::Front)
                    .then(|| self.front_matrix(settings.projection_camera))
                    .flatten(),
            };
            if !(m.is_finite()
                && world_to_box.is_finite()
                && frame.normal_to_locator.is_finite()
                && frame.normal_to_space.is_finite())
            {
                continue;
            }
            out.push(frame);
        }
        out
    }
    /// Front projection (TL-Q30, provisional): the matrix taking a world
    /// position to (u·w, v·w, w), where u, v run 0 → 1 from the bottom
    /// left to the top right of camera `camera`'s render picture (the
    /// render camera when `None`), Film Fit and Film Offset as the render
    /// frames it. Perspective w is the depth in front of the camera;
    /// orthographic w is 1. `None` without a usable camera.
    pub fn front_matrix(&self, camera: Option<u64>) -> Option<DMat4> {
        let id = camera
            .filter(|id| self.item(*id).is_some_and(|i| i.camera.is_some()))
            .or_else(|| self.render_camera())?;
        let settings = self.item(id)?.camera.as_ref()?;
        let frame = crate::camera_light::item_frame(self, id)?;
        let [w, h] = settings.render_size(self.render.frame);
        let aspect = f64::from(w.max(1)) / f64::from(h.max(1));
        let row = |v: DVec3, k: f64| v.extend(-v.dot(frame.position) + k);
        let rows = if settings.projection == crate::camera_light::Projection::Orthographic {
            let (hw, hh) = settings.orthographic_half_size(aspect);
            // u = 0.5 + x / (2 hw), w = 1.
            [
                row(frame.right / (2. * hw), 0.5),
                row(frame.up / (2. * hh), 0.5),
                DVec3::ZERO.extend(1.),
            ]
        } else {
            let (tw, th) = settings.half_tangents(aspect);
            let [sx, sy] = settings.film_shift(aspect);
            // u = 0.5 + 0.5 (x / (z tw) − sx), so u·z is linear.
            [
                row(
                    frame.right / (2. * tw) + frame.forward * (0.5 - 0.5 * sx),
                    0.,
                ),
                row(frame.up / (2. * th) + frame.forward * (0.5 - 0.5 * sy), 0.),
                row(frame.forward, 0.),
            ]
        };
        let m = DMat4::from_cols(rows[0], rows[1], rows[2], glam::DVec4::W).transpose();
        m.is_finite().then_some(m)
    }
    /// The Texture Group item, if there is one.
    pub fn texture_group(&self) -> Option<u64> {
        self.items
            .iter()
            .find(|i| matches!(i.texture, Some(TextureItem::Group)))
            .map(|i| i.id)
    }
    fn push_texture_item(
        &mut self,
        next: &mut u64,
        name: String,
        texture: TextureItem,
    ) -> Result<u64> {
        if *next >= u64::MAX - 1 || self.items.len() >= crate::MAX_SCENE_ITEMS {
            return Err("Scene item/identity budget exhausted".into());
        }
        let id = (*next).max(self.next_id);
        self.items.push(Item {
            id,
            name,
            visible: true,
            transform: Transform::default(),
            mesh: Default::default(),
            parent: None,
            source_id: None,
            source_display_name: None,
            subdivision: None,
            backdrop: None,
            camera: None,
            light: None,
            texture: Some(texture),
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
        *next = id + 1;
        self.next_id = self.next_id.max(*next);
        Ok(id)
    }
    /// The Texture Group, made on first need (spec §1).
    pub fn ensure_texture_group(&mut self, next: &mut u64) -> Result<u64> {
        if let Some(id) = self.texture_group() {
            return Ok(id);
        }
        self.push_texture_item(next, TEXTURE_GROUP_NAME.into(), TextureItem::Group)
    }
    /// A new locator for layer `layer_name` in the Texture Group, named
    /// "<layer name> (Texture)".
    pub fn add_texture_locator(
        &mut self,
        next: &mut u64,
        layer_name: &str,
        settings: TextureLocator,
    ) -> Result<u64> {
        let group = self.ensure_texture_group(next)?;
        let base = format!("{layer_name} (Texture)");
        let name = self.free_name(&base);
        let id = self.push_texture_item(next, name, TextureItem::Locator(Box::new(settings)))?;
        if let Some(item) = self.item_mut(id) {
            item.parent = Some(group);
        }
        Ok(id)
    }
    /// Starting locator settings for a texture source (spec §2).
    pub fn starting_locator(source: &TextureSource) -> TextureLocator {
        // The spec starts an Image Map on UV Map "when the mesh has UVs";
        // without UVs it starts on UV Map too (provisional, TL-Q01), so it
        // renders once UVs exist.
        let projection = match source {
            TextureSource::ImageMap { .. } => Projection::UvMap,
            TextureSource::Constant { .. }
            | TextureSource::Procedural(_)
            | TextureSource::Gradient(_)
            | TextureSource::VertexMap { .. }
            | TextureSource::Variation(_)
            | TextureSource::Process(_)
            | TextureSource::Occlusion(_) => Projection::Solid,
        };
        TextureLocator::new(projection)
    }
    /// Give every texture layer without a locator one (older files and
    /// layers added outside the Shading panel): UV Map, repeat 1, so they
    /// look as before (spec §6). Returns whether anything changed.
    pub fn give_layers_locators(&mut self, next: &mut u64) -> Result<bool> {
        let missing: Vec<(NodeId, String, u8)> = self
            .shader_tree
            .nodes
            .iter()
            .filter_map(|n| match &n.kind {
                NodeKind::Texture(t) if t.locator.is_none() => {
                    Some((n.id, n.name.clone(), t.legacy_axis.unwrap_or(0).min(2)))
                }
                _ => None,
            })
            .collect();
        for (id, name, axis) in &missing {
            let locator = self.add_texture_locator(
                next,
                name,
                TextureLocator {
                    axis: *axis,
                    ..TextureLocator::new(Projection::UvMap)
                },
            )?;
            if let Some(node) = self.shader_tree.nodes.iter_mut().find(|n| n.id == *id)
                && let NodeKind::Texture(t) = &mut node.kind
            {
                t.locator = Some(locator);
                t.legacy_projection = None;
                t.legacy_axis = None;
            }
        }
        Ok(!missing.is_empty())
    }
    /// Remove locator items no texture layer uses any more, of `candidates`
    /// (spec §1: deleting a layer deletes its unshared locator; an empty
    /// Texture Group stays).
    pub fn drop_unused_locators(&mut self, candidates: &[u64]) {
        let used = |scene: &Scene, id: u64| {
            let uses = |n: &crate::shader_tree::Node| matches!(&n.kind, NodeKind::Texture(t) if t.locator == Some(id));
            scene.shader_tree.nodes.iter().any(uses)
                || scene
                    .items
                    .iter()
                    .filter_map(|i| i.environment.as_ref())
                    .any(|e| e.layers.iter().any(uses))
        };
        let gone: Vec<u64> = candidates
            .iter()
            .copied()
            .filter(|&id| self.texture_locator(id).is_some() && !used(self, id))
            .collect();
        if gone.is_empty() {
            return;
        }
        self.items.retain(|i| !gone.contains(&i.id));
        for g in &mut self.groups {
            g.members.retain(|m| !gone.contains(m));
        }
        if self.selection.item.is_some_and(|i| gone.contains(&i)) {
            self.select_item(None);
        }
        self.selection.also.retain(|i| !gone.contains(i));
    }
    /// Auto Size: the position (bounding-box centre) and size of the
    /// surfaces texture layer `layer` applies to, in the locator's point
    /// space (world, or each item's own space for World Coordinates off).
    /// `None` when it applies to no polygon.
    pub fn auto_size_box(&self, layer: NodeId, world: bool) -> Option<(DVec3, DVec3)> {
        let tree = self.shading_tree();
        tree.get(layer)?;
        let mut low = DVec3::splat(f64::INFINITY);
        let mut high = DVec3::splat(f64::NEG_INFINITY);
        for item in self.items.iter().filter(|i| i.is_mesh() && self.shows(i)) {
            let ancestors = self.item_ancestors(item.id);
            let matrix = if world {
                self.world_matrix(item.id).ok()?
            } else {
                DMat4::IDENTITY
            };
            let mut sets: Vec<Vec<&str>> = vec![Vec::new(); item.mesh.faces.len()];
            for (name, faces) in &item.mesh.attributes.polygon_sets {
                for &f in faces {
                    if let Some(list) = sets.get_mut(f as usize) {
                        list.push(name.as_str());
                    }
                }
            }
            let mut checked: std::collections::HashMap<(String, Option<String>, Vec<&str>), bool> =
                Default::default();
            for (f, face) in item.mesh.faces.iter().enumerate() {
                let key = (face.material.clone(), face.part.clone(), sets[f].clone());
                let applies = *checked.entry(key).or_insert_with(|| {
                    tree.layer_applies(
                        layer,
                        &crate::shader_eval::Context {
                            item: item.id,
                            ancestors: &ancestors,
                            material: &face.material,
                            part: face.part.as_deref(),
                            sets: &sets[f],
                            locators: &[],
                        },
                    )
                });
                if !applies {
                    continue;
                }
                for &v in &face.vertices {
                    if let Some(p) = item.mesh.vertices.get(v as usize) {
                        let p = matrix.transform_point3(*p);
                        low = low.min(p);
                        high = high.max(p);
                    }
                }
            }
        }
        low.is_finite().then(|| ((low + high) * 0.5, high - low))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_tree::NewLayer;

    fn near(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
    }

    #[test]
    fn locator_space_subtracts_rotates_and_divides() {
        let t = Transform {
            translation: DVec3::new(1., 2., 3.),
            rotation: DVec3::new(0., 0., std::f64::consts::FRAC_PI_2),
            scale: DVec3::new(2., 4., 8.),
            zxy: false,
            order: None,
        };
        let to = t.matrix().inverse();
        // A point one Size along the locator's own X (world +Y after the
        // 90° turn about Z) is L = (1, 0, 0).
        let p = DVec3::new(1., 2., 3.) + DVec3::new(0., 2., 0.);
        assert!((locator_point(&to, p) - DVec3::X).length() < 1e-9);
        // 1 m locator at the origin: L = P.
        let unit = Transform::default().matrix().inverse();
        let p = DVec3::new(0.3, -2., 5.);
        assert!((locator_point(&unit, p) - p).length() < 1e-12);
    }

    #[test]
    fn planar_axes_follow_the_spec() {
        let l = DVec3::new(0.1, 0.2, 0.3);
        assert!(near(planar(2, l, 0.5), [0.6, 0.7]));
        assert!(near(planar(0, l, 0.5), [0.2, 0.7]));
        assert!(near(planar(1, l, 0.5), [0.6, 0.2]));
        // Procedurals: in-plane coordinates, no centring.
        assert!(near(planar(2, l, 0.), [0.1, 0.2]));
    }

    #[test]
    fn uv_repeat_then_rotation() {
        assert!(near(uv_transform([0.25, 0.5], [2., 3.], 0.), [0.5, 1.5]));
        assert!(near(uv_transform([1., 0.], [1., 1.], 90.), [0., 1.]));
        assert!(near(uv_transform([0.5, 0.], [2., 1.], 180.), [-1., 0.]));
    }

    #[test]
    fn each_tiling_mode() {
        use Tiling::*;
        assert_eq!(tile(1.25, Reset), None);
        assert_eq!(tile(-0.01, Reset), None);
        assert_eq!(tile(0.5, Reset), Some(0.5));
        assert!((tile(1.25, Repeat).unwrap() - 0.25).abs() < 1e-12);
        assert!((tile(-0.25, Repeat).unwrap() - 0.75).abs() < 1e-12);
        // Mirror flips every other tile.
        assert!((tile(0.25, Mirror).unwrap() - 0.25).abs() < 1e-12);
        assert!((tile(1.25, Mirror).unwrap() - 0.75).abs() < 1e-12);
        assert!((tile(2.25, Mirror).unwrap() - 0.25).abs() < 1e-12);
        assert!((tile(-0.25, Mirror).unwrap() - 0.25).abs() < 1e-12);
        assert_eq!(tile(3., Edge), Some(1.));
        assert_eq!(tile(-3., Edge), Some(0.));
        assert_eq!(tile_uv([0.5, 2.], [Repeat, Reset]), None);
    }

    #[test]
    fn starting_values() {
        let l = TextureLocator::new(Projection::Solid);
        assert_eq!(l.axis, 0);
        assert_eq!(l.repeat, [1., 1.]);
        assert_eq!(l.tiling, [Tiling::Repeat; 2]);
        assert!(!l.world_coordinates && l.world_transforms);
        assert_eq!(l.front_culling, FrontCulling::ProjectThrough);
        assert_eq!(l.falloff, [0.; 3]);
        assert_eq!(l.falloff_type, FalloffType::Cubic);
        assert_eq!(l.random_offsets, RandomOffsets::None);
        assert!(l.uv_map.is_none() && l.particle_source.is_none() && !l.real_world_size);
        assert!(uses_axis(Projection::Planar) && !uses_axis(Projection::UvMap));
        assert!(!uses_axis(Projection::Solid) && !uses_axis(Projection::Triplanar));
    }

    #[test]
    fn group_is_made_once_and_reused() {
        let mut scene = Scene::default();
        let mut next = scene.next_id;
        let a = scene
            .add_texture_locator(&mut next, "Image Map", TextureLocator::default())
            .unwrap();
        let b = scene
            .add_texture_locator(&mut next, "Image Map", TextureLocator::default())
            .unwrap();
        let groups: Vec<_> = scene
            .items
            .iter()
            .filter(|i| matches!(i.texture, Some(TextureItem::Group)))
            .collect();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, TEXTURE_GROUP_NAME);
        let group = groups[0].id;
        let item = |id| scene.item(id).unwrap();
        assert_eq!(item(a).name, "Image Map (Texture)");
        assert_eq!(item(b).name, "Image Map (Texture) (2)");
        assert_eq!(item(a).parent, Some(group));
        assert!(!item(a).is_mesh());
        scene.validate().unwrap();
    }

    #[test]
    fn old_layers_get_uv_locators_and_unused_ones_go() {
        let mut scene = Scene::default();
        let id = scene.shader_tree.add(NewLayer::ImageMap, None).unwrap();
        let mut next = scene.next_id;
        assert!(scene.give_layers_locators(&mut next).unwrap());
        let NodeKind::Texture(t) = &scene.shader_tree.get(id).unwrap().kind else {
            panic!()
        };
        let locator = t.locator.unwrap();
        let l = scene.texture_locator(locator).unwrap();
        assert_eq!(l.projection, Projection::UvMap);
        assert_eq!(l.repeat, [1., 1.]);
        assert!(!scene.give_layers_locators(&mut next).unwrap());
        scene.shader_tree.remove(&[id]).unwrap();
        scene.drop_unused_locators(&[locator]);
        assert!(scene.texture_locator(locator).is_none());
        // The empty Texture Group stays.
        assert!(scene.texture_group().is_some());
    }
}

#[cfg(test)]
mod behaviour_tests {
    use super::*;
    use crate::shader_eval::{Ch, Context, ImageLookup, Placement, ShadingPoint, extended};
    use crate::shader_tree::{NewLayer, ShaderTreeEdit, TextureLayer};
    use crate::{Command, Editor};

    fn edit(e: &mut Editor, edit: ShaderTreeEdit) {
        e.dispatch(Command::ShaderTree(edit)).unwrap();
    }
    fn add(e: &mut Editor, layer: NewLayer) -> NodeId {
        let id = e.scene.shader_tree.next_id;
        edit(
            e,
            ShaderTreeEdit::Add {
                layer,
                selected: None,
            },
        );
        id
    }
    fn layer(e: &Editor, id: NodeId) -> TextureLayer {
        match &e.scene.shader_tree.get(id).unwrap().kind {
            NodeKind::Texture(t) => t.clone(),
            _ => panic!("not a texture"),
        }
    }
    fn set_locator(
        e: &mut Editor,
        locator: u64,
        f: impl FnOnce(&mut TextureLocator, &mut Transform),
    ) {
        let item = e.scene.item(locator).unwrap();
        let mut settings = item.texture_locator().unwrap().clone();
        let mut transform = item.transform.clone();
        f(&mut settings, &mut transform);
        edit(
            e,
            ShaderTreeEdit::SetLocator {
                locator,
                settings: Box::new(settings),
                transform,
                continuing: false,
            },
        );
    }
    /// A 2 m square plane in XY at z = 0 with UVs 0..1, Base Material blue.
    fn plane_editor() -> (Editor, u64) {
        let mut e = Editor::default();
        e.scene.new_mesh().unwrap();
        let item = e.scene.selection.item.unwrap();
        let mesh = crate::Mesh {
            vertices: vec![
                DVec3::new(-1., -1., 0.),
                DVec3::new(1., -1., 0.),
                DVec3::new(1., 1., 0.),
                DVec3::new(-1., 1., 0.),
            ],
            faces: vec![crate::mesh::Face {
                vertices: vec![0, 1, 2, 3],
                uv: Some(vec![
                    glam::DVec2::new(0., 0.),
                    glam::DVec2::new(1., 0.),
                    glam::DVec2::new(1., 1.),
                    glam::DVec2::new(0., 1.),
                ]),
                material: crate::material::DEFAULT_MATERIAL.into(),
                smooth: true,
                part: None,
                material_label: None,
            }]
            .into(),
            ..Default::default()
        };
        e.scene.item_mut(item).unwrap().mesh = mesh.into();
        let mut base = e.scene.shader_tree.get(4).unwrap().clone();
        if let NodeKind::Material(m) = &mut base.kind {
            m.diffuse_color = [0., 0., 1.];
        }
        e.scene.shader_tree.replace(base).unwrap();
        (e, item)
    }
    /// An Image Map layer reading image 1 (a uv-coded test picture).
    fn image_layer(e: &mut Editor) -> (NodeId, u64) {
        let id = add(e, NewLayer::ImageMap);
        edit(
            e,
            ShaderTreeEdit::SetImage {
                node: id,
                image: Some(("/pics/grid.png".into(), 4, 4)),
            },
        );
        (id, layer(e, id).locator.unwrap())
    }
    /// The picture: colour (u, v, 0.5) inside 0..1 after the extension,
    /// transparent black where a clip leaves nothing.
    fn picture(l: &ImageLookup, uv: [f64; 2]) -> [f64; 4] {
        match extended(uv, l.extension) {
            Some([u, v]) => [u, v, 0.5, 1.],
            None => [0.; 4],
        }
    }
    /// The diffuse colour at world point `p` on `item` with mesh UV `uv`.
    fn colour_at(e: &Editor, item: u64, p: [f64; 3], uv: [f64; 2]) -> [f64; 3] {
        let frames = e.scene.locator_frames(item);
        let ctx = Context {
            item,
            ancestors: &[],
            material: crate::material::DEFAULT_MATERIAL,
            part: None,
            sets: &[],
            locators: &frames,
        };
        let s = e.scene.shader_tree.evaluate(&ctx);
        s.get(Ch::DiffuseColor).eval_at(
            &ShadingPoint {
                position: p,
                uv,
                ..Default::default()
            },
            &picture,
        )
    }
    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() < 1e-9)
    }

    #[test]
    fn add_makes_one_locator_per_layer_in_one_undo_step() {
        let mut e = Editor::default();
        let before = e.scene.clone();
        let image = add(&mut e, NewLayer::ImageMap);
        let constant = add(&mut e, NewLayer::Constant);
        let (a, b) = (
            layer(&e, image).locator.unwrap(),
            layer(&e, constant).locator.unwrap(),
        );
        assert_ne!(a, b);
        assert_eq!(
            e.scene.texture_locator(a).unwrap().projection,
            Projection::UvMap
        );
        assert_eq!(
            e.scene.texture_locator(b).unwrap().projection,
            Projection::Solid
        );
        let item = |id| e.scene.item(id).unwrap();
        assert_eq!(item(a).name, "Image Map (Texture)");
        assert_eq!(item(b).name, "Constant (Texture)");
        assert_eq!(item(a).transform, Transform::default());
        let group = e.scene.texture_group().unwrap();
        assert_eq!((item(a).parent, item(b).parent), (Some(group), Some(group)));
        assert_eq!(
            e.scene
                .items
                .iter()
                .filter(|i| matches!(i.texture, Some(TextureItem::Group)))
                .count(),
            1
        );
        e.dispatch(Command::Undo).unwrap();
        assert!(e.scene.texture_locator(b).is_none());
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before);
        e.dispatch(Command::Redo).unwrap();
        assert!(e.scene.texture_locator(a).is_some());
    }

    #[test]
    fn deleting_a_layer_deletes_only_an_unshared_locator() {
        let mut e = Editor::default();
        let first = add(&mut e, NewLayer::ImageMap);
        let second = add(&mut e, NewLayer::Constant);
        let shared = layer(&e, first).locator.unwrap();
        let own = layer(&e, second).locator.unwrap();
        // Point the second layer at the first one's locator: its own,
        // now unused, goes.
        let mut node = e.scene.shader_tree.get(second).unwrap().clone();
        if let NodeKind::Texture(t) = &mut node.kind {
            t.locator = Some(shared);
        }
        edit(
            &mut e,
            ShaderTreeEdit::Replace {
                node,
                continuing: false,
            },
        );
        assert!(e.scene.texture_locator(own).is_none());
        // Still used by the second layer: deleting the first keeps it.
        edit(&mut e, ShaderTreeEdit::Remove(vec![first]));
        assert!(e.scene.texture_locator(shared).is_some());
        edit(&mut e, ShaderTreeEdit::Remove(vec![second]));
        assert!(e.scene.texture_locator(shared).is_none());
        // The empty Texture Group stays.
        assert!(e.scene.texture_group().is_some());
        // Only Texture Locators may be a layer's Locator.
        let third = add(&mut e, NewLayer::Constant);
        let mut node = e.scene.shader_tree.get(third).unwrap().clone();
        if let NodeKind::Texture(t) = &mut node.kind {
            t.locator = e.scene.texture_group();
        }
        assert!(
            e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
                node,
                continuing: false
            }))
            .is_err()
        );
    }

    #[test]
    fn every_locator_edit_is_one_undo_step() {
        let mut e = Editor::default();
        let id = add(&mut e, NewLayer::ImageMap);
        let locator = layer(&e, id).locator.unwrap();
        let undo = e.undo.len();
        set_locator(&mut e, locator, |l, t| {
            l.projection = Projection::Planar;
            t.scale = DVec3::splat(2.);
        });
        assert_eq!(e.undo.len(), undo + 1);
        // A drag folds into one step.
        let item = e.scene.item(locator).unwrap().clone();
        for (k, x) in [0.1, 0.2, 0.3].into_iter().enumerate() {
            let mut transform = item.transform.clone();
            transform.translation.x = x;
            edit(
                &mut e,
                ShaderTreeEdit::SetLocator {
                    locator,
                    settings: Box::new(item.texture_locator().unwrap().clone()),
                    transform,
                    continuing: k > 0,
                },
            );
        }
        assert_eq!(e.undo.len(), undo + 2);
        e.dispatch(Command::Undo).unwrap();
        let item = e.scene.item(locator).unwrap();
        assert_eq!(item.transform.translation.x, 0.);
        assert_eq!(item.transform.scale, DVec3::splat(2.));
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(
            e.scene.texture_locator(locator).unwrap().projection,
            Projection::UvMap
        );
        // Invalid settings are refused.
        let mut bad = TextureLocator::new(Projection::Planar);
        bad.axis = 3;
        assert!(
            e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetLocator {
                locator,
                settings: Box::new(bad),
                transform: Transform::default(),
                continuing: false,
            }))
            .is_err()
        );
    }

    #[test]
    fn planar_image_on_a_plane_follows_the_formula() {
        let (mut e, item) = plane_editor();
        let (_, locator) = image_layer(&mut e);
        set_locator(&mut e, locator, |l, _| {
            l.projection = Projection::Planar;
            l.axis = 2;
        });
        // 1 m locator at the origin, axis Z: u = x + 0.5, v = y + 0.5.
        for (x, y) in [(0., 0.), (0.25, -0.4), (-0.5, 0.45)] {
            let c = colour_at(&e, item, [x, y, 0.], [0.9, 0.9]);
            assert!(close(c, [x + 0.5, y + 0.5, 0.5]), "{x},{y}: {c:?}");
        }
        // Repeat: one image per Size, so x = 0.75 reads u = 0.25.
        let c = colour_at(&e, item, [0.75, 0., 0.], [0., 0.]);
        assert!(close(c, [0.25, 0.5, 0.5]), "{c:?}");
        // Axis X: u = -z + 0.5, v = y + 0.5; axis Y: u = x + 0.5, v = -z + 0.5.
        set_locator(&mut e, locator, |l, _| l.axis = 0);
        let c = colour_at(&e, item, [0.3, 0.1, -0.2], [0., 0.]);
        assert!(close(c, [0.7, 0.6, 0.5]), "{c:?}");
        set_locator(&mut e, locator, |l, _| l.axis = 1);
        let c = colour_at(&e, item, [0.3, 0.1, -0.2], [0., 0.]);
        assert!(close(c, [0.8, 0.7, 0.5]), "{c:?}");
        // Position, rotation and Size move, turn and stretch the image.
        set_locator(&mut e, locator, |l, t| {
            l.axis = 2;
            t.translation = DVec3::new(0.2, 0., 0.);
            t.scale = DVec3::new(2., 2., 1.);
        });
        let c = colour_at(&e, item, [0.6, 0.4, 0.], [0., 0.]);
        assert!(close(c, [0.7, 0.7, 0.5]), "{c:?}");
        set_locator(&mut e, locator, |_, t| {
            *t = Transform {
                rotation: DVec3::new(0., 0., std::f64::consts::FRAC_PI_2),
                ..Transform::default()
            };
        });
        // Turned 90° about Z: world +Y is the locator's +X.
        let c = colour_at(&e, item, [0., 0.25, 0.], [0., 0.]);
        assert!(close(c, [0.75, 0.5, 0.5]), "{c:?}");
    }

    #[test]
    fn uv_map_projection_repeats_then_rotates() {
        let (mut e, item) = plane_editor();
        let (_, locator) = image_layer(&mut e);
        // Starting UV Map, repeat 1: the mesh UVs as they are.
        let c = colour_at(&e, item, [5., 5., 0.], [0.3, 0.6]);
        assert!(close(c, [0.3, 0.6, 0.5]), "{c:?}");
        set_locator(&mut e, locator, |l, t| {
            l.repeat = [2., 3.];
            // Position/rotation/Size don't affect UV Map.
            t.translation = DVec3::new(9., 9., 9.);
            t.scale = DVec3::splat(5.);
        });
        let c = colour_at(&e, item, [0., 0., 0.], [0.3, 0.6]);
        assert!(close(c, [0.6, 0.8, 0.5]), "{c:?}");
        set_locator(&mut e, locator, |l, _| {
            l.repeat = [1., 1.];
            l.uv_rotation = 90.;
        });
        // (0.25, 0.5) turned 90° about (0, 0) is (-0.5, 0.25): -0.5 repeats to 0.5.
        let c = colour_at(&e, item, [0., 0., 0.], [0.25, 0.5]);
        assert!(close(c, [0.5, 0.25, 0.5]), "{c:?}");
    }

    #[test]
    fn solid_image_is_transparent_and_solid_hands_procedurals_l() {
        let (mut e, item) = plane_editor();
        let (id, locator) = image_layer(&mut e);
        set_locator(&mut e, locator, |l, t| {
            l.projection = Projection::Solid;
            t.scale = DVec3::splat(2.);
        });
        let c = colour_at(&e, item, [0.1, 0.1, 0.], [0.5, 0.5]);
        assert!(close(c, [0., 0., 1.]), "{c:?}");
        // Stage 2's procedurals read L: a 2 m locator halves the point.
        let frames = e.scene.locator_frames(item);
        let ctx = Context {
            item,
            ancestors: &[],
            material: "Default",
            part: None,
            sets: &[],
            locators: &frames,
        };
        let t = layer(&e, id);
        let v = Placement::of(&t, &ctx).procedural_vector();
        let l = v.eval_at(
            &ShadingPoint {
                position: [1., -2., 4.],
                uv: [0.; 2],
                ..Default::default()
            },
            &picture,
        );
        assert!(close(l, [0.5, -1., 2.]), "{l:?}");
        // Planar hands the in-plane coordinates, depth 0.
        set_locator(&mut e, locator, |l, _| {
            l.projection = Projection::Planar;
            l.axis = 2;
        });
        let frames = e.scene.locator_frames(item);
        let ctx = Context {
            locators: &frames,
            ..ctx
        };
        let v = Placement::of(&t, &ctx).procedural_vector();
        let l = v.eval_at(
            &ShadingPoint {
                position: [1., -2., 4.],
                uv: [0.; 2],
                ..Default::default()
            },
            &picture,
        );
        assert!(close(l, [0.5, -1., 0.]), "{l:?}");
    }

    #[test]
    fn each_tiling_mode_on_a_planar_image() {
        let (mut e, item) = plane_editor();
        let (_, locator) = image_layer(&mut e);
        let at = |e: &Editor, x: f64| colour_at(e, item, [x, 0., 0.], [0., 0.]);
        let mode = |e: &mut Editor, h: Tiling, v: Tiling| {
            set_locator(e, locator, |l, _| {
                l.projection = Projection::Planar;
                l.axis = 2;
                l.tiling = [h, v];
            })
        };
        use Tiling::*;
        // x = 0.75 is u = 1.25 (one tile to the right).
        mode(&mut e, Repeat, Repeat);
        assert!(close(at(&e, 0.75), [0.25, 0.5, 0.5]));
        mode(&mut e, Mirror, Mirror);
        assert!(close(at(&e, 0.75), [0.75, 0.5, 0.5]));
        mode(&mut e, Edge, Edge);
        assert!(close(at(&e, 0.75), [1., 0.5, 0.5]));
        // Reset: outside the tile the lower layer (the blue Base Material)
        // shows; inside, the image.
        mode(&mut e, Reset, Reset);
        assert!(close(at(&e, 0.75), [0., 0., 1.]));
        assert!(close(at(&e, 0.25), [0.75, 0.5, 0.5]));
        // Mixed modes, per axis.
        mode(&mut e, Mirror, Reset);
        assert!(close(at(&e, 0.75), [0.75, 0.5, 0.5]));
        assert!(close(
            colour_at(&e, item, [0., 0.75, 0.], [0., 0.]),
            [0., 0., 1.]
        ));
        mode(&mut e, Reset, Repeat);
        assert!(close(at(&e, 0.75), [0., 0., 1.]));
        let c = colour_at(&e, item, [0., 0.75, 0.], [0., 0.]);
        assert!(close(c, [0.5, 0.25, 0.5]), "{c:?}");
        // Reset hides a Constant layer outside the tile too.
        let constant = add(&mut e, NewLayer::Constant);
        let mut node = e.scene.shader_tree.get(constant).unwrap().clone();
        if let NodeKind::Texture(t) = &mut node.kind {
            t.source = crate::shader_tree::TextureSource::Constant {
                color: [1., 0., 0.],
                value: 1.,
            };
            t.locator = Some(locator);
        }
        edit(
            &mut e,
            ShaderTreeEdit::Replace {
                node,
                continuing: false,
            },
        );
        mode(&mut e, Reset, Reset);
        assert!(close(at(&e, 0.25), [1., 0., 0.]));
        assert!(close(at(&e, 0.75), [0., 0., 1.]));
    }

    #[test]
    fn world_coordinates_decide_whether_the_texture_follows_the_item() {
        let (mut e, item) = plane_editor();
        let (_, locator) = image_layer(&mut e);
        set_locator(&mut e, locator, |l, _| {
            l.projection = Projection::Planar;
            l.axis = 2;
            l.tiling = [Tiling::Reset; 2];
        });
        // Move the mesh item 10 m along X. Its local point (0.25, 0) is now
        // at world (10.25, 0).
        e.scene.item_mut(item).unwrap().transform.translation.x = 10.;
        // Off: the texture moves with the item.
        let c = colour_at(&e, item, [10.25, 0., 0.], [0., 0.]);
        assert!(close(c, [0.75, 0.5, 0.5]), "{c:?}");
        // On: it stays in the world, so the moved item is outside the tile.
        set_locator(&mut e, locator, |l, _| l.world_coordinates = true);
        let c = colour_at(&e, item, [10.25, 0., 0.], [0., 0.]);
        assert!(close(c, [0., 0., 1.]), "{c:?}");
    }

    #[test]
    fn auto_size_fits_the_surfaces_the_layer_applies_to() {
        let (mut e, _) = plane_editor();
        let (id, locator) = image_layer(&mut e);
        let item_id = e.scene.selection.item.unwrap();
        e.scene.item_mut(item_id).unwrap().transform.translation = DVec3::new(3., 0., 0.);
        let undo = e.undo.len();
        edit(&mut e, ShaderTreeEdit::AutoSize { layer: id });
        assert_eq!(e.undo.len(), undo + 1);
        let t = &e.scene.item(locator).unwrap().transform;
        // World Coordinates off: the item's own space.
        assert!((t.translation - DVec3::ZERO).length() < 1e-9);
        // A uniform Size stays uniform at the largest side.
        assert!((t.scale - DVec3::splat(2.)).length() < 1e-9);
        // A non-uniform Size fits each axis; the flat Z side keeps its size.
        set_locator(&mut e, locator, |_, t| t.scale = DVec3::new(1., 1., 0.5));
        edit(&mut e, ShaderTreeEdit::AutoSize { layer: id });
        let t = &e.scene.item(locator).unwrap().transform;
        assert!((t.scale - DVec3::new(2., 2., 0.5)).length() < 1e-9);
        set_locator(&mut e, locator, |l, _| l.world_coordinates = true);
        edit(&mut e, ShaderTreeEdit::AutoSize { layer: id });
        let t = &e.scene.item(locator).unwrap().transform;
        assert!((t.translation - DVec3::new(3., 0., 0.)).length() < 1e-9);
    }

    #[test]
    fn old_scenes_get_uv_locators_and_look_the_same() {
        // A version-18 file whose image layer stored a Planar projection
        // on the layer (never rendered: everything rendered as UV).
        let (mut e, item) = plane_editor();
        let (id, _) = image_layer(&mut e);
        let mut scene = e.scene.clone();
        scene.items.retain(|i| i.texture.is_none());
        let i = scene.shader_tree.index(id).unwrap();
        if let NodeKind::Texture(t) = &mut scene.shader_tree.nodes[i].kind {
            t.locator = None;
        }
        let json = String::from_utf8(crate::persistence::encode(&scene).unwrap()).unwrap();
        let json = json.replace(
            "\"effect\": \"DiffuseColor\"",
            "\"effect\": \"DiffuseColor\",\n\"projection\": \"Planar\",\n\"axis\": 2",
        );
        assert!(json.contains("\"projection\": \"Planar\""));
        let loaded = crate::persistence::decode(json.as_bytes()).unwrap();
        let NodeKind::Texture(t) = &loaded.shader_tree.get(id).unwrap().kind else {
            panic!()
        };
        let l = loaded.texture_locator(t.locator.unwrap()).unwrap();
        assert_eq!(
            (l.projection, l.axis, l.repeat),
            (Projection::UvMap, 2, [1., 1.])
        );
        assert_eq!(t.legacy_projection, None);
        // Saved again it is version 19 and reopens the same.
        let mut loaded = loaded;
        // A pre-OpenPBR material, so the older version is what is tested.
        loaded.shader_tree.legacy_openpbr();
        let bytes = crate::persistence::encode(&loaded).unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 19"));
        assert_eq!(crate::persistence::decode(&bytes).unwrap(), loaded);
        // It renders from the mesh UVs, as before.
        let after = Editor {
            scene: loaded,
            ..Default::default()
        };
        let c = colour_at(&after, item, [7., 7., 0.], [0.3, 0.6]);
        assert!(close(c, [0.3, 0.6, 0.5]), "{c:?}");
        // A file claiming version 18 with locator items is refused.
        let text = String::from_utf8(bytes).unwrap();
        let old = text.replace("\"version\": 19", "\"version\": 18");
        assert!(crate::persistence::decode(old.as_bytes()).is_err());
    }

    /// Stage 3: Cylindrical, Spherical, Cubic, Box, Triplanar, Front
    /// Culling and Falloff.
    mod projections {
        use super::*;

        fn colour_at_n(e: &Editor, item: u64, p: [f64; 3], n: [f64; 3]) -> [f64; 3] {
            let frames = e.scene.locator_frames(item);
            let ctx = Context {
                item,
                ancestors: &[],
                material: crate::material::DEFAULT_MATERIAL,
                part: None,
                sets: &[],
                locators: &frames,
            };
            e.scene
                .shader_tree
                .evaluate(&ctx)
                .get(Ch::DiffuseColor)
                .eval_at(
                    &ShadingPoint {
                        position: p,
                        uv: [0.; 2],
                        normal: n,
                    },
                    &picture,
                )
        }
        fn uv(c: [f64; 3]) -> [f64; 2] {
            [c[0], c[1]]
        }
        fn near2(a: [f64; 2], b: [f64; 2]) -> bool {
            (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
        }
        fn setup(f: impl FnOnce(&mut TextureLocator, &mut Transform)) -> (Editor, u64) {
            let (mut e, item) = plane_editor();
            let (_, locator) = image_layer(&mut e);
            set_locator(&mut e, locator, f);
            (e, item)
        }

        #[test]
        fn cylindrical_and_spherical_wrap_around_the_axis() {
            let (e, item) = setup(|l, _| {
                l.projection = Projection::Cylindrical;
                l.axis = 1;
            });
            let up = [0., 0., 1.];
            // Axis Y: the angle runs from +Z (u 0.5) toward +X (u 0.75).
            assert!(near2(
                uv(colour_at_n(&e, item, [0., 0.2, 1.], up)),
                [0.5, 0.7]
            ));
            assert!(near2(
                uv(colour_at_n(&e, item, [1., -0.3, 0.], up)),
                [0.75, 0.2]
            ));
            assert!(near2(
                uv(colour_at_n(&e, item, [-1., 0., 0.], up)),
                [0.25, 0.5]
            ));
            let (e, item) = setup(|l, _| {
                l.projection = Projection::Spherical;
                l.axis = 1;
            });
            // Latitude from the plane normal to the axis, +Y up: v 1 at the pole.
            assert!(near2(
                uv(colour_at_n(&e, item, [0., 0., 1.], up)),
                [0.5, 0.5]
            ));
            let c = uv(colour_at_n(&e, item, [0., 1., 1.], up));
            assert!(near2(c, [0.5, 0.75]), "{c:?}");
            // Axis Z: from +X toward +Y.
            let (e, item) = setup(|l, _| {
                l.projection = Projection::Cylindrical;
                l.axis = 2;
            });
            assert!(near2(
                uv(colour_at_n(&e, item, [0., 1., 0.1], up)),
                [0.75, 0.6]
            ));
        }

        #[test]
        fn cubic_follows_rotation_without_mirroring_box_ignores_rotation() {
            let (e, item) = setup(|l, _| l.projection = Projection::Cubic);
            let p = [0.1, 0.2, 0.3];
            // +Z and -Z faces read the same (no mirroring).
            assert!(near2(
                uv(colour_at_n(&e, item, p, [0., 0., 1.])),
                [0.6, 0.7]
            ));
            assert!(near2(
                uv(colour_at_n(&e, item, p, [0., 0., -1.])),
                [0.6, 0.7]
            ));
            // +X face: u = -z + 0.5, v = y + 0.5.
            assert!(near2(
                uv(colour_at_n(&e, item, p, [1., 0.2, 0.])),
                [0.2, 0.7]
            ));
            // Turned 90° about Z: world +Y is the locator's +X.
            let (e, item) = setup(|l, t| {
                l.projection = Projection::Cubic;
                t.rotation = DVec3::new(0., 0., std::f64::consts::FRAC_PI_2);
            });
            assert!(near2(
                uv(colour_at_n(&e, item, [0., 0.25, 0.], [0., 0., 1.])),
                [0.75, 0.5]
            ));
            // Box: the rotation is ignored; negative faces mirror u.
            let (e, item) = setup(|l, t| {
                l.projection = Projection::Box;
                t.rotation = DVec3::new(0., 0., std::f64::consts::FRAC_PI_2);
            });
            assert!(near2(
                uv(colour_at_n(&e, item, p, [0., 0., 1.])),
                [0.6, 0.7]
            ));
            assert!(near2(
                uv(colour_at_n(&e, item, p, [0., 0., -1.])),
                [0.4, 0.7]
            ));
            assert!(near2(
                uv(colour_at_n(&e, item, p, [-1., 0., 0.])),
                [0.8, 0.7]
            ));
            // Box keeps the locator's position and Size.
            let (e, item) = setup(|l, t| {
                l.projection = Projection::Box;
                t.translation = DVec3::new(0.2, 0., 0.);
                t.scale = DVec3::splat(2.);
            });
            assert!(near2(
                uv(colour_at_n(&e, item, [0.6, 0.4, 0.], [0., 0., 1.])),
                [0.7, 0.7]
            ));
        }

        #[test]
        fn triplanar_blends_by_normal_weights() {
            let (e, item) = setup(|l, _| l.projection = Projection::Triplanar);
            let p = [0.1, 0.2, 0.3];
            let n = [1., 1., 0.];
            // Hardness 0 (k = 1): X and Y halves: X face (0.2, 0.7), Y face
            // (0.6, 0.2).
            assert!(near2(uv(colour_at_n(&e, item, p, n)), [0.4, 0.45]));
            // A pure +Z normal is the Z face.
            assert!(near2(
                uv(colour_at_n(&e, item, p, [0., 0., 1.])),
                [0.6, 0.7]
            ));
            // Hardness 100 % (k = 32): nearly a hard switch.
            let (e, item) = setup(|l, _| {
                l.projection = Projection::Triplanar;
                l.triplanar_hardness = 1.;
            });
            let c = uv(colour_at_n(&e, item, p, [1., 0.8, 0.]));
            assert!(
                (c[0] - 0.2).abs() < 0.01 && (c[1] - 0.7).abs() < 0.01,
                "{c:?}"
            );
            assert_eq!(
                TextureLocator {
                    triplanar_hardness: 0.5,
                    ..TextureLocator::default()
                }
                .triplanar_exponent(),
                1. + 31. * 0.25
            );
        }

        #[test]
        fn front_culling_modes_on_planar() {
            let mode = |c: FrontCulling| {
                setup(move |l, _| {
                    l.projection = Projection::Planar;
                    l.axis = 2;
                    l.front_culling = c;
                })
            };
            let p = [0.1, 0.2, 0.];
            let (front, back) = ([0., 0., 1.], [0., 0., -1.]);
            let (e, item) = mode(FrontCulling::ProjectThrough);
            assert!(near2(uv(colour_at_n(&e, item, p, back)), [0.6, 0.7]));
            let (e, item) = mode(FrontCulling::CullBackfacing);
            assert!(near2(uv(colour_at_n(&e, item, p, front)), [0.6, 0.7]));
            assert!(close(colour_at_n(&e, item, p, back), [0., 0., 1.]));
            let (e, item) = mode(FrontCulling::FlipBackfacing);
            assert!(near2(uv(colour_at_n(&e, item, p, front)), [0.6, 0.7]));
            assert!(near2(uv(colour_at_n(&e, item, p, back)), [0.4, 0.3]));
            let (e, item) = mode(FrontCulling::FlipUBackfacing);
            assert!(near2(uv(colour_at_n(&e, item, p, back)), [0.4, 0.7]));
            let (e, item) = mode(FrontCulling::FlipVBackfacing);
            assert!(near2(uv(colour_at_n(&e, item, p, back)), [0.6, 0.3]));
        }

        #[test]
        fn falloff_fades_to_nothing_at_size_over_f() {
            use crate::shader_eval::falloff;
            use FalloffType::*;
            // f 0.5 on a 1 m locator: gone at 2 m, half at 1 m.
            assert_eq!(falloff(Cubic, [0.5; 3], [1., 0., 0.]), 0.5);
            assert_eq!(falloff(Cubic, [0.5; 3], [2., 0., 0.]), 0.);
            assert_eq!(falloff(Cubic, [0.5; 3], [1., 1.5, 0.]), 0.25);
            assert!((falloff(Spherical, [0.5; 3], [0.6, 0.8, 0.]) - 0.5).abs() < 1e-12);
            assert_eq!(falloff(LinearX, [0.5; 3], [1., 9., 9.]), 0.5);
            assert_eq!(falloff(LinearY, [0.5, 0., 0.], [1., 9., 9.]), 1.);
            // In a layer: half the image over the blue base at 1 m.
            let (e, item) = setup(|l, _| {
                l.projection = Projection::Planar;
                l.axis = 2;
                l.falloff = [0.5, 0.5, 0.];
            });
            let c = colour_at_n(&e, item, [1., 0., 0.], [0., 0., 1.]);
            assert!(close(c, [0.25, 0.25, 0.75]), "{c:?}");
            let c = colour_at_n(&e, item, [0., 0., 0.], [0., 0., 1.]);
            assert!(close(c, [0.5, 0.5, 0.5]), "{c:?}");
        }

        /// A camera 5 m up +Z looking down -Z at the plane.
        fn with_camera(e: &mut Editor) -> u64 {
            let t = Transform {
                translation: DVec3::new(0., 0., 5.),
                ..Default::default()
            };
            e.scene
                .add_camera(crate::camera_light::CameraSettings::default(), t)
                .unwrap()
        }

        /// TL-Q30: Front maps the camera's picture 0..1 onto the surface:
        /// the centre of the view reads (0.5, 0.5), its right edge u = 1,
        /// its top v = 1, whatever the locator's own placement.
        #[test]
        fn front_projects_the_camera_picture() {
            let (mut e, item) = setup(|l, t| {
                l.projection = Projection::Front;
                t.translation = DVec3::new(3., -2., 1.);
            });
            // No camera: the UV Map fallback (the plane's UVs). The
            // picture's edges (u = 1) wrap under the default Repeat, so
            // the checks stay inside the frame.
            let c = colour_at(&e, item, [0., 0., 0.], [0.25, 0.75]);
            assert!(close(c, [0.25, 0.75, 0.5]), "{c:?}");
            let cam = with_camera(&mut e);
            let settings = e.scene.item(cam).unwrap().camera.clone().unwrap();
            let [w, h] = settings.render_size(e.scene.render.frame);
            let (tw, th) = settings.half_tangents(f64::from(w) / f64::from(h));
            let c = colour_at_n(&e, item, [0., 0., 0.], [0., 0., 1.]);
            assert!(near2(uv(c), [0.5, 0.5]), "{c:?}");
            let c = colour_at_n(&e, item, [5. * tw * 0.8, 0., 0.], [0., 0., 1.]);
            assert!(near2(uv(c), [0.9, 0.5]), "{c:?}");
            let c = colour_at_n(&e, item, [0., 5. * th * 0.5, 0.], [0., 0., 1.]);
            assert!(near2(uv(c), [0.5, 0.75]), "{c:?}");
            // Nearer the camera, the same picture spot is closer in.
            let c = colour_at_n(&e, item, [2.5 * tw * 0.8, 0., 2.5], [0., 0., 1.]);
            assert!(near2(uv(c), [0.9, 0.5]), "{c:?}");
        }

        /// A chosen Projection Camera wins over the render camera; a
        /// missing one falls back to it.
        #[test]
        fn front_uses_the_chosen_camera() {
            let (mut e, item) = plane_editor();
            let (_, locator) = image_layer(&mut e);
            let render = with_camera(&mut e);
            let side = e
                .scene
                .add_camera(
                    crate::camera_light::CameraSettings::default(),
                    Transform {
                        translation: DVec3::new(1., 0., 5.),
                        ..Default::default()
                    },
                )
                .unwrap();
            e.scene.set_render_camera(render).unwrap();
            set_locator(&mut e, locator, |l, _| {
                l.projection = Projection::Front;
                l.projection_camera = Some(side);
            });
            // The side camera looks at x = 1: there is its centre.
            let c = colour_at_n(&e, item, [1., 0., 0.], [0., 0., 1.]);
            assert!(near2(uv(c), [0.5, 0.5]), "{c:?}");
            set_locator(&mut e, locator, |l, _| l.projection_camera = Some(9999));
            let c = colour_at_n(&e, item, [0., 0., 0.], [0., 0., 1.]);
            assert!(near2(uv(c), [0.5, 0.5]), "{c:?}");
        }

        /// TL-Q31: Light Probe reads the direction from the locator's
        /// centre off a mirror ball, as an environment Light Probe does:
        /// axis Y, the ball's centre is +Z, its top +Y, its right +X.
        #[test]
        fn light_probe_reads_a_mirror_ball() {
            use crate::shader_eval::probe_coords;
            let near = |a: [f64; 2], b: [f64; 2]| near2(a, b);
            assert!(near(probe_coords(1, [0., 0., 3.]), [0.5, 0.5]));
            let s = 0.5f64.sqrt();
            // 90° off the viewer: halfway to the rim.
            assert!(near(probe_coords(1, [0., 1., 0.]), [0.5, 0.5 + 0.5 * s]));
            assert!(near(probe_coords(1, [1., 0., 0.]), [0.5 + 0.5 * s, 0.5]));
            // Agrees with the environment's mirror ball (its v runs down).
            for d in [[0.3, 0.4, 0.5], [-0.7, 0.1, -0.2], [0.2, -0.9, 0.1]] {
                for axis in 0..3u8 {
                    let frame = crate::environment::pole_frame(axis);
                    let env = crate::environment::environment_probe_uv(frame, d.into());
                    let ours = probe_coords(axis, d);
                    assert!(near(ours, [env[0], 1. - env[1]]), "{axis} {d:?}");
                }
            }
            // In a layer, × repeat.
            let (e, item) = setup(|l, t| {
                l.projection = Projection::LightProbe;
                l.axis = 1;
                l.repeat = [2., 1.];
                t.translation = DVec3::new(0., 0., -1.);
            });
            let c = colour_at_n(&e, item, [0., 0., 0.], [0., 0., 1.]);
            // Direction +Z from the locator: the ball's centre, u 0.5 × 2.
            assert!(
                near2(uv(c), [0.0, 0.5]) || near2(uv(c), [1.0, 0.5]),
                "{c:?}"
            );
        }

        /// TL-Q32: Implicit UV gives each polygon its own 0..1 patch.
        #[test]
        fn implicit_uv_is_a_patch_per_polygon() {
            use glam::DVec2;
            assert_eq!(implicit_uv(4, 2), DVec2::ONE);
            assert_eq!(implicit_uv(3, 2), DVec2::Y);
            for k in 0..6 {
                let p = implicit_uv(6, k);
                assert!((p - DVec2::splat(0.5)).length() - 0.5 < 1e-12);
            }
            // The layer reads the reserved map, so renderers fill it.
            let (e, item) = setup(|l, _| {
                l.projection = Projection::ImplicitUv;
                l.repeat = [2., 2.];
            });
            let frames = e.scene.locator_frames(item);
            let ctx = Context {
                item,
                ancestors: &[],
                material: crate::material::DEFAULT_MATERIAL,
                part: None,
                sets: &[],
                locators: &frames,
            };
            let expr = e
                .scene
                .shader_tree
                .evaluate(&ctx)
                .get(Ch::DiffuseColor)
                .clone();
            assert!(format!("{expr:?}").contains(IMPLICIT_UV_MAP));
            let c = colour_at(&e, item, [0., 0., 0.], [0.2, 0.3]);
            assert!(close(c, [0.4, 0.6, 0.5]), "{c:?}");
        }

        #[test]
        fn old_placeholder_names_still_read() {
            let l: TextureLocator = serde_json::from_value(serde_json::json!({
                "projection": "Planar", "axis": 2, "uv_map": null, "repeat": [1.0, 1.0],
                "uv_rotation": 0.0, "tiling": ["Repeat", "Repeat"],
                "world_coordinates": false, "world_transforms": true,
                "front_culling": "FrontOnly", "triplanar_hardness": 0.0,
                "random_offsets": "Part", "random_strength": 0.0,
                "falloff_type": "Linear", "falloff": [0.0, 0.0, 0.0],
                "tangent": "DpduDpdv", "real_world_size": false, "particle_source": null
            }))
            .unwrap();
            assert_eq!(l.front_culling, FrontCulling::CullBackfacing);
            assert_eq!(l.random_offsets, RandomOffsets::MeshParts);
            assert_eq!(l.falloff_type, FalloffType::LinearX);
        }
    }

    /// Stage 6 settings save and reopen unchanged; Front, Light Probe and
    /// Implicit UV render as themselves (shader-tree-complete).
    #[test]
    fn stage6_settings_are_stored_and_saved() {
        let (mut e, _) = plane_editor();
        let (_, locator) = image_layer(&mut e);
        for projection in [
            Projection::Front,
            Projection::LightProbe,
            Projection::ImplicitUv,
        ] {
            set_locator(&mut e, locator, |l, _| {
                l.projection = projection;
                l.random_offsets = RandomOffsets::MeshParts;
                l.random_strength = 7.5;
                l.real_world_size = true;
                l.real_world_size_xy = [0.3, 2.];
                l.tangent = TangentType::DpduCross;
                l.particle_source = Some(77);
                l.replicator = Some(78);
                l.projection_camera = Some(79);
                l.world_transforms = false;
            });
            // A pre-OpenPBR material, so the older version is what is tested.
            e.scene.shader_tree.legacy_openpbr();
            let bytes = crate::persistence::encode(&e.scene).unwrap();
            assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 25"));
            let back = crate::persistence::decode(&bytes).unwrap();
            assert_eq!(
                back.texture_locator(locator),
                e.scene.texture_locator(locator)
            );
            assert_eq!(
                e.scene
                    .texture_locator(locator)
                    .unwrap()
                    .rendered_projection(),
                projection
            );
        }
    }
}
