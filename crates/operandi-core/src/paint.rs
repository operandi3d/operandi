//! 3D painting, stage 1 in the scene (PNT-01..39): paintable images, the
//! texture layers made for them, the paint target and the image commands.
//! Only built with the `painting` feature.
//!
//! - Pixels live in [`Scene::paint`] as reference-counted buffers keyed by
//!   the scene image's id, so an undo step (two scene snapshots) costs a
//!   reference count, not a copy. They are never written to the scene
//!   file; images are saved as image files of their own (PNT-30).
//! - Images are made outside the undo history (PNT-11): undoing Add Color
//!   Texture takes the layer away but the image stays in the Images list.
//!   Undo and redo carry images the step didn't know into the restored
//!   scene ([`carry_images`]). Edits of an image's pixels (Invert, Clear,
//!   Set Resolution, Expand UV Seams) are ordinary undo steps.
//! - An image is changed ("*" in the Images list, PNT-30) when its buffer
//!   isn't the one last loaded or saved ([`PaintSaved`]); undoing back to
//!   the saved pixels makes it unchanged again.
use crate::Result;
use crate::shader_tree::{
    Drop, NewLayer, NodeId, NodeKind, TextureEffect, TextureLayer, TextureSource,
};
use crate::{Editor, Scene};
use operandi_paint::psd::{Pick, PsdDoc};
use operandi_paint::{Depth, Format, Layout, PaintImage};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

pub use operandi_paint as pixels;

/// The pixel buffers of the scene's paintable images, by scene image id.
#[derive(Clone, Debug, Default)]
pub struct PaintImages {
    /// Every painted image's picture; a Photoshop image's Composite.
    pub buffers: BTreeMap<u64, Arc<PaintImage>>,
    /// Photoshop images' whole documents (PNT-300), read once per file.
    pub psd: BTreeMap<u64, Arc<PsdDoc>>,
    /// The sculpt mask of each mesh (PNT-260): one value per point, 0
    /// free to 1 held. Undone with the scene, never saved.
    pub masks: crate::paint_sculpt::Masks,
}

/// Two stores are equal when they hold the very same buffers: comparing
/// scenes (every undo step does) must not compare megabytes of pixels.
impl PartialEq for PaintImages {
    fn eq(&self, other: &Self) -> bool {
        same_buffers(&self.buffers, &other.buffers)
            && same_buffers(&self.psd, &other.psd)
            && same_buffers(&self.masks, &other.masks)
    }
}

/// The same keys holding the very same buffers.
fn same_buffers<T>(a: &BTreeMap<u64, Arc<T>>, b: &BTreeMap<u64, Arc<T>>) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|((ka, x), (kb, y))| ka == kb && Arc::ptr_eq(x, y))
}

/// Each image's buffer as last loaded from or saved to its file. Not part
/// of the scene: saving isn't undone.
#[derive(Clone, Debug, Default)]
pub struct PaintSaved {
    pub buffers: BTreeMap<u64, Arc<PaintImage>>,
    pub psd: BTreeMap<u64, Arc<PsdDoc>>,
}

impl PartialEq for PaintSaved {
    fn eq(&self, other: &Self) -> bool {
        same_buffers(&self.buffers, &other.buffers) && same_buffers(&self.psd, &other.psd)
    }
}

/// Painting state on the editor that isn't scene data.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintSession {
    pub saved: PaintSaved,
    /// The image the last New Image or Add … Texture made.
    pub last_image: Option<u64>,
    /// The layer the last Add … Texture or Flatten Layers made: the new
    /// paint target (PNT-10).
    pub last_layer: Option<NodeId>,
}

/// New Image's panel (PNT-01..07).
#[derive(Clone, Debug, PartialEq)]
pub struct NewImage {
    /// `None`: "Image", made unique.
    pub name: Option<String>,
    /// Square side, 64..8192 (PNT-02).
    pub resolution: u32,
    pub format: Format,
    /// Floating Point (PNT-04).
    pub float: bool,
    /// Set Color on: every pixel this colour with full alpha; off: colour
    /// 0, alpha 0 (PNT-06).
    pub color: Option<[f32; 3]>,
    /// Add Texture's new Image Map layer has Antialiasing on (Preferences >
    /// Images & Painting > Enable Image Antialiasing by Default, PNT-214).
    pub antialiasing: bool,
}

impl Default for NewImage {
    fn default() -> Self {
        NewImage {
            name: None,
            resolution: operandi_paint::DEFAULT_SIZE,
            format: Format::Rgba,
            float: false,
            color: None,
            antialiasing: true,
        }
    }
}

impl NewImage {
    pub fn layout(&self) -> Layout {
        Layout {
            width: self.resolution,
            height: self.resolution,
            format: self.format,
            depth: if self.float { Depth::F32 } else { Depth::U8 },
        }
    }
}

/// Add Color / Mask / Displacement Texture (PNT-08..12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureKind {
    Color,
    Mask,
    Displacement,
}

impl TextureKind {
    pub const ALL: [TextureKind; 3] = [
        TextureKind::Color,
        TextureKind::Mask,
        TextureKind::Displacement,
    ];
    pub fn effect(self) -> TextureEffect {
        match self {
            TextureKind::Color => TextureEffect::DiffuseColor,
            TextureKind::Mask => TextureEffect::LayerMask,
            TextureKind::Displacement => TextureEffect::VectorDisplacement,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            TextureKind::Color => "Add Color Texture",
            TextureKind::Mask => "Add Mask Texture",
            TextureKind::Displacement => "Add Displacement Texture",
        }
    }
    /// The command word (`kind:color`).
    pub fn word(self) -> &'static str {
        match self {
            TextureKind::Color => "color",
            TextureKind::Mask => "mask",
            TextureKind::Displacement => "displacement",
        }
    }
    pub fn named(word: &str) -> Option<TextureKind> {
        TextureKind::ALL
            .into_iter()
            .find(|k| k.word().eq_ignore_ascii_case(word.trim()))
    }
}

/// The image commands (PNT-20..29) and texture creation. Each changes one
/// image or one place in the shader tree.
#[derive(Clone, Debug, PartialEq)]
pub enum PaintEdit {
    /// New Image: not an undo step (PNT-11).
    NewImage(NewImage),
    /// Add … Texture: New Image, then an Image Map layer for it. One undo
    /// step for the layer; the image stays when it is undone.
    AddTexture {
        kind: TextureKind,
        image: NewImage,
        /// The Shader Tree's selected layer: Color and Displacement go at
        /// the top of its Material Group, Mask directly above it.
        selected: Option<NodeId>,
        /// The mesh whose first UV map the layer uses (else the active one).
        item: Option<u64>,
    },
    /// `via`: the Image Map layer the image is edited through; for a
    /// Photoshop image its PSD Layer is the layer changed (PNT-304).
    Invert {
        image: u64,
        via: Option<NodeId>,
    },
    Clear {
        image: u64,
        via: Option<NodeId>,
    },
    SetResolution {
        image: u64,
        width: u32,
        height: u32,
    },
    /// Grows colour out of the UV islands of mesh `item` by `border`
    /// pixels (Bake UV Border Size, PNT-26).
    ExpandSeams {
        image: u64,
        via: Option<NodeId>,
        item: Option<u64>,
        uv_map: Option<String>,
        border: u32,
    },
    /// Composites Image Map layers into one new image layer (PNT-27).
    Flatten {
        layers: Vec<NodeId>,
    },
    /// A finished brush stroke (PNT-50): the image's (or, for a Photoshop
    /// image, the layer `via` shows) pixels after the stroke. One undo
    /// step labelled `label` ("Airbrush Stroke").
    Stroke {
        image: u64,
        via: Option<NodeId>,
        pixels: Arc<PaintImage>,
        label: String,
    },
    /// A sculpt mask stroke or command (PNT-260): mesh `item`'s mask
    /// becomes `values` (one per point; `None` clears it). One undo step.
    Mask {
        item: u64,
        values: Option<Arc<Vec<f32>>>,
        label: String,
    },
    /// "Paint into which layer?" > New Layer (PNT-304): adds "Operandi
    /// Paint N" to the Photoshop image Image Map layer `via` shows, and
    /// points the map at it. One undo step.
    NewPsdLayer {
        via: NodeId,
    },
}

/// Refusals the panel and the paint tools show (PNT-13..18).
pub const NO_TEXTURE: &str = "No texture selected. Add a new texture?";
pub const NOTHING_TO_PAINT: &str =
    "Nothing to paint on: the selected texture must be visible and carry the paintbrush.";
pub const EMPTY_IMAGE: &str = "This image is empty (0 × 0).";
pub const NO_UV: &str = "Can't find a texture with a valid UV map.";
pub const NO_IMAGE_SELECTED: &str = "Select an image first";
/// A Photoshop image showing its Composite: painting asks which layer
/// (PNT-304).
pub const PSD_WHICH_LAYER: &str = "Paint into which layer?";
pub const FLATTEN_REFUSAL: &str =
    "Flatten Layers needs two or more Image Map layers with the same effect and image size";

/// Why the selected layer can't be painted, or `None` when it can
/// (PNT-13..17). `layer` is the Shader Tree's selected layer.
pub fn paint_target_problem(scene: &Scene, layer: Option<NodeId>) -> Option<&'static str> {
    paint_target_problem_on(scene, layer, scene.selection.item)
}

/// [`paint_target_problem`] for painting mesh item `item` (the mesh under
/// the pointer, else the active one).
pub fn paint_target_problem_on(
    scene: &Scene,
    layer: Option<NodeId>,
    item: Option<u64>,
) -> Option<&'static str> {
    let Some(node) = layer.and_then(|id| scene.shader_tree.get(id)) else {
        return Some(NO_TEXTURE);
    };
    let NodeKind::Texture(TextureLayer {
        source: TextureSource::ImageMap { image, .. },
        locator,
        ..
    }) = &node.kind
    else {
        return Some(NO_TEXTURE);
    };
    let Some(image) = image.and_then(|id| scene.images.iter().find(|i| i.id == id)) else {
        return Some(NO_TEXTURE);
    };
    // Hidden itself or under a hidden group.
    let mut at = Some(node);
    while let Some(n) = at {
        if !n.layer.enable {
            return Some(NOTHING_TO_PAINT);
        }
        at = n.parent.and_then(|p| scene.shader_tree.get(p));
    }
    let (w, h) = scene
        .paint
        .buffers
        .get(&image.id)
        .map_or((image.width, image.height), |b| (b.width, b.height));
    if w == 0 || h == 0 {
        return Some(EMPTY_IMAGE);
    }
    let uv_ok = locator
        .and_then(|l| scene.texture_locator(l))
        .is_none_or(|l| matches!(l.projection, crate::shader_tree::Projection::UvMap));
    let mesh_has_uvs = item
        .and_then(|id| scene.item(id))
        .is_some_and(|i| i.is_mesh() && !i.mesh.uv_maps.is_empty());
    if !uv_ok || !mesh_has_uvs {
        return Some(NO_UV);
    }
    None
}

/// The image an Image Map layer shows, if it has one.
pub fn layer_image(scene: &Scene, layer: NodeId) -> Option<u64> {
    match &scene.shader_tree.get(layer)?.kind {
        NodeKind::Texture(TextureLayer {
            source: TextureSource::ImageMap { image, .. },
            ..
        }) => *image,
        _ => None,
    }
}

/// The PSD Layer Image Map layer `layer` shows; `None` is the Composite.
pub fn psd_layer_of(scene: &Scene, layer: NodeId) -> Option<String> {
    match &scene.shader_tree.get(layer)?.kind {
        NodeKind::Texture(TextureLayer {
            source: TextureSource::ImageMap { settings, .. },
            ..
        }) => settings.psd_layer.clone(),
        _ => None,
    }
}

/// Points Image Map layer `layer` at PSD layer `name` (`None`: Composite).
pub fn set_psd_layer(scene: &mut Scene, layer: NodeId, name: Option<String>) -> Result<()> {
    let mut node = scene
        .shader_tree
        .get(layer)
        .ok_or("Missing shader tree layer")?
        .clone();
    match &mut node.kind {
        NodeKind::Texture(TextureLayer {
            source: TextureSource::ImageMap { settings, .. },
            ..
        }) => settings.psd_layer = name,
        _ => return Err("Only an Image Map layer has a PSD Layer".into()),
    }
    scene.shader_tree.replace(node)
}

/// Undo and redo: painting's images in `from` that `to` doesn't know come along, with
/// their pixels (images aren't undone, PNT-11).
pub fn carry_images(from: &Scene, to: &mut Scene) {
    for image in &from.images {
        // Only painting's images (those with pixels here); images other
        // commands add (Backdrop, Add Clip) undo with them.
        if from.paint.buffers.contains_key(&image.id) && to.images.iter().all(|i| i.id != image.id)
        {
            to.images.push(image.clone());
            if let Some(b) = from.paint.buffers.get(&image.id) {
                to.paint.buffers.insert(image.id, b.clone());
            }
            if let Some(d) = from.paint.psd.get(&image.id) {
                to.paint.psd.insert(image.id, d.clone());
            }
        }
    }
}

impl Scene {
    /// A free image name: `base`, else "base (2)", "base (3)"… (BD-03).
    fn free_image_name(&self, base: &str) -> String {
        let taken = |n: &str| self.images.iter().any(|i| i.name == n);
        if !taken(base) {
            return base.to_owned();
        }
        (2..)
            .map(|n| format!("{base} ({n})"))
            .find(|n| !taken(n))
            .expect("some number is free")
    }

    /// Adds a new, unsaved image (no file yet) and returns its id.
    fn add_paint_image(&mut self, settings: &NewImage) -> Result<u64> {
        operandi_paint::check_side(settings.resolution)?;
        let pixels = PaintImage::filled(settings.layout(), settings.color)?;
        let id = self.images.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        let name = self.free_image_name(
            settings
                .name
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .unwrap_or("Image"),
        );
        self.images.push(crate::backdrop::SceneImage {
            id,
            name,
            path: String::new(),
            width: pixels.width,
            height: pixels.height,
        });
        self.paint.buffers.insert(id, Arc::new(pixels));
        Ok(id)
    }

    /// The Material Group a layer is in (itself when it is one).
    fn group_of(&self, layer: Option<NodeId>) -> Option<NodeId> {
        let mut at = layer.and_then(|id| self.shader_tree.get(id));
        // At most one step per node, so a broken parent chain can't loop.
        let mut steps = self.shader_tree.nodes.len();
        while let Some(n) = at {
            if steps == 0 {
                return None;
            }
            steps -= 1;
            if matches!(n.kind, NodeKind::Group(_)) {
                return Some(n.id);
            }
            at = n.parent.and_then(|p| self.shader_tree.get(p));
        }
        None
    }
}

impl Editor {
    /// Whether image `id` has changes not in its file (PNT-30): a buffer
    /// that isn't the one last loaded or saved. Images never loaded for
    /// painting are unchanged.
    pub fn image_changed(&self, id: u64) -> bool {
        match self.scene.paint.buffers.get(&id) {
            None => false,
            Some(b) => self
                .paint
                .saved
                .buffers
                .get(&id)
                .is_none_or(|s| !Arc::ptr_eq(s, b)),
        }
    }

    /// The scene's changed images, in Images list order.
    pub fn changed_images(&self) -> Vec<u64> {
        self.scene
            .images
            .iter()
            .map(|i| i.id)
            .filter(|&id| self.image_changed(id))
            .collect()
    }

    /// Image `id`'s pixels, loaded from its file the first time.
    pub fn image_pixels(&mut self, id: u64) -> Result<Arc<PaintImage>> {
        if let Some(b) = self.scene.paint.buffers.get(&id) {
            return Ok(b.clone());
        }
        let image = self
            .scene
            .images
            .iter()
            .find(|i| i.id == id)
            .ok_or("No such image")?;
        if image.path.is_empty() {
            return Err(format!("{} has no pixels or file", image.name));
        }
        let path = Path::new(&image.path);
        if operandi_paint::FileType::of(path).is_some_and(operandi_paint::FileType::is_psd) {
            let mut doc = PsdDoc::read(path)?;
            let (composite, _) = doc.picture(Pick::Composite)?;
            let composite = Arc::new(composite);
            let doc = Arc::new(doc);
            self.scene.paint.psd.insert(id, doc.clone());
            self.paint.saved.psd.insert(id, doc);
            self.scene.paint.buffers.insert(id, composite.clone());
            self.paint.saved.buffers.insert(id, composite.clone());
            return Ok(composite);
        }
        let loaded = Arc::new(operandi_paint::io::load(path)?);
        self.scene.paint.buffers.insert(id, loaded.clone());
        self.paint.saved.buffers.insert(id, loaded.clone());
        Ok(loaded)
    }

    /// Image `id`'s Photoshop document, when it is one (loaded once).
    pub fn psd_doc(&mut self, id: u64) -> Result<Option<Arc<PsdDoc>>> {
        self.image_pixels(id)?;
        Ok(self.scene.paint.psd.get(&id).cloned())
    }

    /// The PSD Layer picker's rows for image `id` (PNT-302); none for a
    /// flat image.
    pub fn psd_rows(&mut self, id: u64) -> Result<Vec<operandi_paint::psd::Row>> {
        Ok(self.psd_doc(id)?.map(|d| d.rows()).unwrap_or_default())
    }

    /// The picture Image Map layer `layer` shows: its PSD Layer for a
    /// Photoshop image, else the image itself.
    pub fn layer_picture(&mut self, layer: NodeId) -> Result<Arc<PaintImage>> {
        let image = layer_image(&self.scene, layer).ok_or(NO_TEXTURE)?;
        let pixels = self.image_pixels(image)?;
        let Some(doc) = self.scene.paint.psd.get(&image) else {
            return Ok(pixels);
        };
        match psd_layer_of(&self.scene, layer) {
            None => Ok(pixels),
            Some(name) => {
                let index = doc
                    .layer_named(&name)
                    .ok_or_else(|| format!("The Photoshop file has no layer {name}"))?;
                let (px, _) = (**doc).clone().picture(Pick::Layer(index))?;
                Ok(Arc::new(px))
            }
        }
    }

    /// Saving image `id` to `path` (Save Image / Save Image As, PNT-31):
    /// writes the file, points the image at it and marks it unchanged. Not
    /// an undo step.
    pub fn save_image(&mut self, id: u64, path: &Path) -> Result<()> {
        let pixels = self.image_pixels(id)?;
        let to_psd = operandi_paint::FileType::of(path);
        if let Some(t) = to_psd.filter(|t| t.is_psd()) {
            // The whole document at its own depth and format (PNT-306),
            // or a new one-layer document (PNT-308).
            let mut doc = match self.scene.paint.psd.get(&id) {
                Some(d) => (**d).clone(),
                None => PsdDoc::new_from(&pixels, t == operandi_paint::FileType::Psb)?,
            };
            doc.write(path)?;
            let doc = Arc::new(doc);
            self.scene.paint.psd.insert(id, doc.clone());
            self.paint.saved.psd.insert(id, doc);
        } else {
            if self.scene.paint.psd.contains_key(&id) {
                return Err("A Photoshop image saves as .psd or .psb, keeping its layers".into());
            }
            operandi_paint::io::save(&pixels, path)?;
        }
        let text = path.display().to_string();
        // Saving names the image's file in the scene and in every undo
        // snapshot, so undo doesn't point it back at nothing.
        let rename = |scene: &mut Scene| {
            if let Some(i) = scene.images.iter_mut().find(|i| i.id == id) {
                i.path = text.clone();
            }
        };
        rename(&mut self.scene);
        for c in self.undo.iter_mut().chain(self.redo.iter_mut()) {
            let c = std::sync::Arc::make_mut(c);
            rename(&mut c.before);
            rename(&mut c.after);
        }
        self.paint.saved.buffers.insert(id, pixels);
        Ok(())
    }

    /// Forgets the pixels of image `id` and reads its file again (an
    /// outside change with no unsaved paint, PNT-307). Not an undo step.
    pub fn reload_image(&mut self, id: u64) -> Result<()> {
        self.scene.paint.buffers.remove(&id);
        self.paint.saved.buffers.remove(&id);
        self.scene.paint.psd.remove(&id);
        self.paint.saved.psd.remove(&id);
        self.image_pixels(id).map(|_| ())
    }

    pub(crate) fn edit_paint(&mut self, edit: PaintEdit) -> Result<()> {
        self.require_idle()?;
        if let PaintEdit::NewImage(settings) = &edit {
            let id = self.scene.add_paint_image(settings)?;
            self.paint.last_image = Some(id);
            return Ok(());
        }
        let mut candidate = self.scene.clone();
        candidate.shader_tree = self.scene.render_shader_tree().into_owned();
        let label: String = match edit {
            PaintEdit::NewImage(_) => unreachable!("handled above"),
            PaintEdit::AddTexture {
                kind,
                image,
                selected,
                item,
            } => return self.add_texture(kind, image, selected, item),
            PaintEdit::Invert { image, via } => {
                self.edit_pixels(&mut candidate, image, via, |px| {
                    px.invert();
                    Ok(true)
                })?;
                "Invert Image".into()
            }
            PaintEdit::Clear { image, via } => {
                self.edit_pixels(&mut candidate, image, via, |px| {
                    px.clear();
                    Ok(true)
                })?;
                "Clear Image".into()
            }
            PaintEdit::Stroke {
                image,
                via,
                pixels,
                label,
            } => {
                if !self.scene.paint.psd.contains_key(&image) {
                    // The stroke's own buffer: no copy (PNT-50).
                    self.image_pixels(image)?;
                    candidate.paint.buffers.insert(image, pixels);
                } else {
                    self.edit_pixels(&mut candidate, image, via, |px| {
                        if px.width != pixels.width || px.height != pixels.height {
                            return Err("The image changed size during the stroke".into());
                        }
                        *px = (*pixels).clone();
                        Ok(true)
                    })?;
                }
                label
            }
            PaintEdit::Mask {
                item,
                values,
                label,
            } => {
                let n = candidate
                    .item(item)
                    .filter(|i| i.is_mesh())
                    .ok_or("The mask needs a mesh")?
                    .mesh
                    .vertices
                    .len();
                match values {
                    Some(v) if v.iter().any(|&x| x > 0.) => {
                        if v.len() != n {
                            return Err("The mask doesn't match the mesh's points".into());
                        }
                        candidate.paint.masks.insert(item, v);
                    }
                    _ => {
                        candidate.paint.masks.remove(&item);
                    }
                }
                label
            }
            PaintEdit::NewPsdLayer { via } => {
                let image = layer_image(&self.scene, via).ok_or(NO_TEXTURE)?;
                let doc = self
                    .psd_doc(image)?
                    .ok_or("That image isn't a Photoshop file")?;
                let mut doc = (*doc).clone();
                let index = doc.add_paint_layer()?;
                let name = doc.file.layers()[index].name();
                candidate.paint.psd.insert(image, Arc::new(doc));
                set_psd_layer(&mut candidate, via, Some(name))?;
                "New Layer".into()
            }
            PaintEdit::SetResolution {
                image,
                width,
                height,
            } => {
                if self.psd_doc(image)?.is_some() {
                    // A canvas size change edits the document's structure
                    // (PNT-309).
                    return Err("Set Resolution doesn't change Photoshop files".into());
                }
                let px = self.image_pixels(image)?.resized(width, height)?;
                if let Some(i) = candidate.images.iter_mut().find(|i| i.id == image) {
                    i.width = width;
                    i.height = height;
                }
                candidate.paint.buffers.insert(image, Arc::new(px));
                "Set Resolution".into()
            }
            PaintEdit::ExpandSeams {
                image,
                via,
                item,
                uv_map,
                border,
            } => {
                let item = item
                    .or(self.scene.selection.item)
                    .and_then(|id| self.scene.item(id))
                    .filter(|i| i.is_mesh())
                    .ok_or("Select the mesh whose UVs the image uses")?;
                if item.mesh.uv_maps.is_empty() {
                    return Err(NO_UV.into());
                }
                let mesh = crate::bake::BakeMesh::new(
                    &item.mesh,
                    &self.scene.smoothing,
                    uv_map.as_deref(),
                )?;
                let changed = self.edit_pixels(&mut candidate, image, via, |px| {
                    let covered: Vec<bool> = crate::bake::rasterise(&mesh, px.width, px.height)
                        .iter()
                        .map(Option::is_some)
                        .collect();
                    if !covered.iter().any(|c| *c) {
                        return Err(
                            "No polygon's UVs fall inside the 0-1 square; nothing to expand".into(),
                        );
                    }
                    let map = crate::bake::padding::DilationMap::new(
                        &covered,
                        px.width,
                        px.height,
                        Some(border),
                    );
                    let mut copied = 0;
                    for (to, &from) in map.src.iter().enumerate() {
                        if from != crate::bake::padding::NONE && from as usize != to {
                            px.copy_pixel(from as usize, to);
                            copied += 1;
                        }
                    }
                    Ok(copied > 0)
                })?;
                if !changed {
                    return Ok(());
                }
                "Expand UV Seams".into()
            }
            PaintEdit::Flatten { layers } => return self.flatten_layers(&layers),
        };
        self.commit_paint(candidate, label)
    }

    /// Runs `f` on image `image`'s pixels into `candidate`: the picture
    /// itself, or for a Photoshop image the layer `via` shows (its
    /// Composite asks which layer, PNT-304), with the Composite rebuilt.
    /// `f` answers whether it changed anything.
    fn edit_pixels(
        &mut self,
        candidate: &mut Scene,
        image: u64,
        via: Option<NodeId>,
        f: impl FnOnce(&mut PaintImage) -> Result<bool>,
    ) -> Result<bool> {
        let pixels = self.image_pixels(image)?;
        let Some(doc) = self.scene.paint.psd.get(&image).cloned() else {
            let mut px = (*pixels).clone();
            if f(&mut px)? {
                candidate.paint.buffers.insert(image, Arc::new(px));
                return Ok(true);
            }
            return Ok(false);
        };
        let name = via
            .and_then(|l| psd_layer_of(&self.scene, l))
            .ok_or(PSD_WHICH_LAYER)?;
        let mut doc = (*doc).clone();
        let index = doc
            .layer_named(&name)
            .ok_or_else(|| format!("The Photoshop file has no pixel layer {name}"))?;
        let (mut px, _) = doc.picture(Pick::Layer(index))?;
        if !f(&mut px)? {
            return Ok(false);
        }
        doc.set_layer(index, &px)?;
        let (composite, _) = doc.picture(Pick::Composite)?;
        candidate.paint.buffers.insert(image, Arc::new(composite));
        candidate.paint.psd.insert(image, Arc::new(doc));
        Ok(true)
    }

    fn commit_paint(&mut self, candidate: Scene, label: String) -> Result<()> {
        if candidate == self.scene {
            return Ok(());
        }
        self.next_item_id = self.next_item_id.max(candidate.next_id);
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record(label, before);
        Ok(())
    }

    /// PNT-08..12.
    fn add_texture(
        &mut self,
        kind: TextureKind,
        mut settings: NewImage,
        selected: Option<NodeId>,
        item: Option<u64>,
    ) -> Result<()> {
        // A layer id this scene doesn't have (a closed scene's selection)
        // is no selection: the new layer would take that id and become its
        // own parent.
        let selected = selected.filter(|&id| self.scene.shader_tree.get(id).is_some());
        if kind == TextureKind::Displacement {
            // Vector displacement is float, saved as OpenEXR (PNT-12).
            settings.float = true;
        }
        if kind == TextureKind::Mask
            && selected
                .and_then(|id| self.scene.shader_tree.get(id))
                .is_none()
        {
            return Err("Select the layer to mask first".into());
        }
        operandi_paint::check_side(settings.resolution)?;
        let antialiasing = settings.antialiasing;
        // The image is made outside the undo step (PNT-11).
        let image = self.scene.add_paint_image(&settings)?;
        self.paint.last_image = Some(image);
        let mut candidate = self.scene.clone();
        candidate.shader_tree = self.scene.render_shader_tree().into_owned();
        let id = candidate.shader_tree.add(NewLayer::ImageMap, selected)?;
        let tree = &candidate.shader_tree;
        // Where it goes: Color and Displacement at the top of the selected
        // Material Group (where Add Layer puts it when there is none),
        // Mask directly above the selected layer. Indices count the
        // children as they are, the new layer included (`move_to`).
        let drop = match kind {
            TextureKind::Mask => {
                let sel = tree.get(selected.expect("checked")).expect("checked");
                let parent = sel.parent;
                let index = tree
                    .children(parent)
                    .position(|n| n.id == sel.id)
                    .unwrap_or(0);
                Some(Drop { parent, index })
            }
            TextureKind::Color | TextureKind::Displacement => {
                candidate.group_of(selected).map(|g| Drop {
                    parent: Some(g),
                    index: 0,
                })
            }
        };
        if let Some(drop) = drop {
            candidate.shader_tree.move_to(&[id], drop)?;
        }
        let name = candidate
            .images
            .iter()
            .find(|i| i.id == image)
            .map(|i| i.name.clone())
            .unwrap_or_default();
        // The mesh's first UV map (PNT-10).
        let uv_map = item
            .or(candidate.selection.item)
            .and_then(|i| candidate.item(i))
            .filter(|i| i.is_mesh())
            .and_then(|i| i.mesh.uv_maps.keys().next().cloned());
        let mut next = candidate.next_id.max(self.next_item_id);
        let mut settings_l = crate::Scene::starting_locator(&TextureSource::ImageMap {
            image: None,
            settings: Box::default(),
        });
        settings_l.uv_map = uv_map;
        let locator = candidate.add_texture_locator(&mut next, &name, settings_l)?;
        let mut node = candidate.shader_tree.get(id).expect("just added").clone();
        node.name = candidate.shader_tree.unique_name(&name);
        if let NodeKind::Texture(t) = &mut node.kind {
            t.effect = kind.effect();
            t.locator = Some(locator);
            t.source = TextureSource::ImageMap {
                image: Some(image),
                settings: Box::new(crate::shader_tree::ImageSettings {
                    antialiasing,
                    ..Default::default()
                }),
            };
        }
        candidate.shader_tree.replace(node)?;
        candidate.shader_tree.validate()?;
        candidate.validate()?;
        self.paint.last_layer = Some(id);
        self.commit_paint(candidate, kind.label().into())
    }

    /// PNT-27: the layers, bottom to top in tree order, into one image.
    fn flatten_layers(&mut self, layers: &[NodeId]) -> Result<()> {
        let tree = self.scene.render_shader_tree().into_owned();
        let mut picked = Vec::new();
        for &id in layers {
            let node = tree.get(id).ok_or("Missing shader tree layer")?;
            let NodeKind::Texture(TextureLayer {
                source:
                    TextureSource::ImageMap {
                        image: Some(img), ..
                    },
                effect,
                locator,
                ..
            }) = &node.kind
            else {
                return Err(FLATTEN_REFUSAL.into());
            };
            picked.push((id, *img, *effect, *locator));
        }
        if picked.len() < 2 || picked.iter().any(|p| p.2 != picked[0].2) {
            return Err(FLATTEN_REFUSAL.into());
        }
        let mut images = Vec::new();
        for p in &picked {
            images.push(self.image_pixels(p.1)?);
        }
        let (w, h) = (images[0].width, images[0].height);
        if images.iter().any(|i| i.width != w || i.height != h) {
            return Err(FLATTEN_REFUSAL.into());
        }
        // Panel order is top first; the stack composites bottom first.
        let order: Vec<NodeId> = tree
            .rows(&|_| false)
            .into_iter()
            .map(|(n, _)| n.id)
            .collect();
        let mut stack: Vec<usize> = (0..picked.len()).collect();
        stack.sort_by_key(|&k| std::cmp::Reverse(order.iter().position(|&id| id == picked[k].0)));
        let depth = images
            .iter()
            .map(|i| i.depth)
            .max_by_key(|d| d.bits())
            .unwrap_or(Depth::U8);
        let sources: Vec<operandi_paint::Source<'_>> = stack
            .iter()
            .map(|&k| {
                let layer = &tree.get(picked[k].0).expect("checked").layer;
                operandi_paint::Source {
                    image: &images[k],
                    blend: blend_of(layer.blend),
                    opacity: layer.opacity as f32,
                    invert: layer.invert,
                }
            })
            .collect();
        let flat = operandi_paint::composite(
            &sources,
            Layout {
                width: w,
                height: h,
                format: Format::Rgba,
                depth,
            },
        )?;
        let top = picked[*stack.last().expect("two or more")];
        let mut candidate = self.scene.clone();
        candidate.shader_tree = tree;
        let image = candidate.add_paint_image(&NewImage {
            name: Some("Flattened".into()),
            resolution: 64,
            ..NewImage::default()
        })?;
        if let Some(i) = candidate.images.iter_mut().find(|i| i.id == image) {
            i.width = w;
            i.height = h;
        }
        candidate.paint.buffers.insert(image, Arc::new(flat));
        let id = candidate.shader_tree.add(NewLayer::ImageMap, Some(top.0))?;
        // Directly above the top flattened layer, in its group.
        let parent = candidate.shader_tree.get(top.0).expect("checked").parent;
        let index = candidate
            .shader_tree
            .children(parent)
            .position(|n| n.id == top.0)
            .unwrap_or(0);
        candidate
            .shader_tree
            .move_to(&[id], Drop { parent, index })?;
        let name = candidate
            .images
            .iter()
            .find(|i| i.id == image)
            .expect("added")
            .name
            .clone();
        let mut node = candidate.shader_tree.get(id).expect("just added").clone();
        node.name = candidate.shader_tree.unique_name(&name);
        if let NodeKind::Texture(t) = &mut node.kind {
            t.effect = top.2;
            // The same placement as the top layer: share its locator.
            t.locator = top.3;
            t.source = TextureSource::ImageMap {
                image: Some(image),
                settings: Box::default(),
            };
        }
        candidate.shader_tree.replace(node)?;
        // The flattened layers are turned off, so the look doesn't change
        // (provisional, PNT-Q04).
        for p in &picked {
            let mut n = candidate.shader_tree.get(p.0).expect("checked").clone();
            n.layer.enable = false;
            candidate.shader_tree.replace(n)?;
        }
        candidate.shader_tree.validate()?;
        // The new image isn't undone with the step (PNT-11).
        self.scene
            .images
            .push(candidate.images.last().expect("added").clone());
        self.scene
            .paint
            .buffers
            .insert(image, candidate.paint.buffers[&image].clone());
        self.paint.last_image = Some(image);
        self.paint.last_layer = Some(id);
        self.commit_paint(candidate, "Flatten Layers".into())
    }
}

/// The shader tree's blend mode as the compositor's.
pub fn blend_of(mode: crate::shader_tree::BlendMode) -> operandi_paint::Blend {
    use crate::shader_tree::BlendMode as B;
    use operandi_paint::Blend as P;
    match mode {
        B::Normal => P::Normal,
        B::Add => P::Add,
        B::Subtract => P::Subtract,
        B::Difference => P::Difference,
        B::NormalMultiply => P::NormalMultiply,
        B::Divide => P::Divide,
        B::Multiply => P::Multiply,
        B::Screen => P::Screen,
        B::Overlay => P::Overlay,
        B::SoftLight => P::SoftLight,
        B::HardLight => P::HardLight,
        B::Lighten => P::Lighten,
        B::Darken => P::Darken,
        B::ColorDodge => P::ColorDodge,
        B::ColorBurn => P::ColorBurn,
        B::NormalMapBlend => P::NormalMapBlend,
    }
}
