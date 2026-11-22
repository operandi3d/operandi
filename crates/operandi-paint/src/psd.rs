//! Photoshop documents as paintable images (PNT-300..309), on craft-libs'
//! lossless PSD/PSB reader and writer (`craft-psd`, vendored).
//!
//! - A [`PsdDoc`] keeps the whole document as read, so saving writes back
//!   everything Operandi didn't change: other layers, groups, text and
//!   adjustment layers, masks, resources, at the document's own depth and
//!   format (PSD or PSB).
//! - The layer picker ([`PsdDoc::rows`]) lists the Composite first, then
//!   every layer top to bottom, groups as headers with their layers
//!   indented (PNT-302). Only pixel layers can be painted; text,
//!   adjustment, smart object and group rows are greyed (PNT-305).
//! - The Composite is the stored flattened image. When the file has none,
//!   or a layer was painted, it is rebuilt from the visible pixel layers
//!   (PNT-303); text and adjustment layers are left out of a rebuild, with
//!   one note saying so.
//! - Operandi doesn't edit the layer structure (PNT-309), except adding a
//!   pixel layer to paint into ("Operandi Paint 1", PNT-304).
use crate::blend::{Blend, Source, composite};
use crate::image::{Depth, Format, Layout, PaintImage, Result};
use craft_psd::tree::LayerNode;
use craft_psd::{
    BlendMode, ChannelData, ColorMode, Compression, ImageData, LayerSpec, PixelData, PsdBuilder,
    PsdFile, Rect, Version,
};
use std::path::Path;

/// The note after a rebuilt composite that left layers out (PNT-303).
pub const SKIPPED_NOTE: &str =
    "The composite was rebuilt from the pixel layers; text and adjustment layers were left out.";
/// The name of a layer Operandi adds to paint into (PNT-304).
pub const PAINT_LAYER_BASE: &str = "Operandi Paint";
/// Photoshop's limit for a PSD side; larger images need a PSB.
pub const PSD_MAX_SIDE: u32 = 30_000;

/// What a layer is, for the picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerKind {
    Pixel,
    Text,
    Adjustment,
    Smart,
    /// A fill layer (solid colour, gradient, pattern): no pixels of its own.
    Fill,
    Group,
}

impl LayerKind {
    pub fn paintable(self) -> bool {
        self == LayerKind::Pixel
    }
}

/// Which picture of a document an Image Map shows (PNT-302).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Pick {
    /// The flattened image.
    #[default]
    Composite,
    /// The layer record at this index (`PsdFile::layers`, bottom first).
    Layer(usize),
}

/// One row of the PSD Layer picker.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub pick: Pick,
    pub name: String,
    /// Groups it sits in.
    pub indent: usize,
    pub kind: LayerKind,
}

impl Row {
    /// Greyed in "Paint into which layer?" (PNT-305).
    pub fn paintable(&self) -> bool {
        matches!(self.pick, Pick::Layer(_)) && self.kind.paintable()
    }
}

/// A Photoshop document, read whole.
#[derive(Clone, Debug, PartialEq)]
pub struct PsdDoc {
    pub file: PsdFile,
    /// The composite needs rebuilding before it is shown or saved.
    pub composite_stale: bool,
}

fn err(e: impl std::fmt::Display) -> String {
    format!("Photoshop file: {e}")
}

fn depth_of(bits: u16) -> Result<Depth> {
    match bits {
        8 => Ok(Depth::U8),
        16 => Ok(Depth::U16),
        32 => Ok(Depth::F32),
        other => Err(format!(
            "Photoshop files of {other} bits per channel can't be painted (8, 16 or 32)"
        )),
    }
}

fn kind_of(file: &PsdFile, index: usize) -> LayerKind {
    let Some(l) = file.layers().get(index) else {
        return LayerKind::Pixel;
    };
    if l.type_layer().is_some() {
        LayerKind::Text
    } else if l.smart_object().is_some() {
        LayerKind::Smart
    } else if l.adjustment().is_some() {
        LayerKind::Adjustment
    } else if l.solid_color().is_some() || l.block(b"GdFl").is_some() || l.block(b"PtFl").is_some()
    {
        LayerKind::Fill
    } else {
        LayerKind::Pixel
    }
}

impl PsdDoc {
    pub fn read(path: &Path) -> Result<PsdDoc> {
        let bytes =
            std::fs::read(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        PsdDoc::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<PsdDoc> {
        let file = PsdFile::from_bytes(bytes).map_err(err)?;
        let doc = PsdDoc {
            composite_stale: file.has_real_merged_data() == Some(false),
            file,
        };
        doc.check_paintable_mode()?;
        Ok(doc)
    }

    /// Painting reads RGB and Grayscale documents (provisional, PNT-Q07).
    fn check_paintable_mode(&self) -> Result<()> {
        depth_of(self.file.header.depth)?;
        match self.file.header.color_mode {
            ColorMode::Rgb | ColorMode::Grayscale => Ok(()),
            other => Err(format!(
                "Only RGB and Grayscale Photoshop files can be painted (this one is {other:?})"
            )),
        }
    }

    pub fn width(&self) -> u32 {
        self.file.header.width
    }
    pub fn height(&self) -> u32 {
        self.file.header.height
    }
    pub fn depth(&self) -> Depth {
        depth_of(self.file.header.depth).unwrap_or(Depth::U8)
    }
    pub fn is_psb(&self) -> bool {
        self.file.header.version == Version::Psb
    }
    fn format(&self) -> Format {
        if self.file.header.color_mode == ColorMode::Grayscale {
            Format::Gray
        } else {
            Format::Rgba
        }
    }

    /// The picker's rows: Composite, then the layer tree top to bottom.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = vec![Row {
            pick: Pick::Composite,
            name: "Composite".into(),
            indent: 0,
            kind: LayerKind::Pixel,
        }];
        fn walk(doc: &PsdDoc, nodes: &[LayerNode], indent: usize, rows: &mut Vec<Row>) {
            // Children come bottom to top; the picker reads top first.
            for node in nodes.iter().rev() {
                let index = node.index();
                let name = doc
                    .file
                    .layers()
                    .get(index)
                    .map(|l| l.name())
                    .unwrap_or_default();
                match node {
                    LayerNode::Layer { .. } => rows.push(Row {
                        pick: Pick::Layer(index),
                        name,
                        indent,
                        kind: kind_of(&doc.file, index),
                    }),
                    LayerNode::Group { children, .. } => {
                        rows.push(Row {
                            pick: Pick::Layer(index),
                            name,
                            indent,
                            kind: LayerKind::Group,
                        });
                        walk(doc, children, indent + 1, rows);
                    }
                }
            }
        }
        walk(self, &self.file.layer_tree(), 0, &mut rows);
        rows
    }

    /// The pixel layer named `name`, if there is one.
    pub fn layer_named(&self, name: &str) -> Option<usize> {
        self.rows()
            .into_iter()
            .find(|r| r.name == name && r.kind == LayerKind::Pixel && r.pick != Pick::Composite)
            .and_then(|r| match r.pick {
                Pick::Layer(i) => Some(i),
                Pick::Composite => None,
            })
    }

    /// The picture `pick` shows, canvas-sized, at the document's depth.
    /// The Composite is rebuilt first when stale; the note says when that
    /// left layers out.
    pub fn picture(&mut self, pick: Pick) -> Result<(PaintImage, Option<&'static str>)> {
        match pick {
            Pick::Composite => {
                let note = if self.composite_stale {
                    self.rebuild_composite()?
                } else {
                    None
                };
                let f = self.file.composite_f32().map_err(err)?;
                Ok((self.float_picture(&f.data), note))
            }
            Pick::Layer(i) => {
                if !kind_of(&self.file, i).paintable() {
                    return Err("Only pixel layers have pixels to show".into());
                }
                if let Some(exact) = self.raw_float_layer(i)? {
                    return Ok((exact, None));
                }
                let f = self.file.layer_f32(i, true).map_err(err)?;
                Ok((self.float_picture(&f.data), None))
            }
        }
    }

    /// A 32-bit layer without a mask, read sample for sample (craft-psd's
    /// float path re-encodes linear values to sRGB, which wouldn't come
    /// back exactly). `None` for other layers.
    fn raw_float_layer(&self, index: usize) -> Result<Option<PaintImage>> {
        let h = &self.file.header;
        let Some(l) = self.file.layers().get(index) else {
            return Ok(None);
        };
        if h.depth != 32 || l.channels.iter().any(|c| c.id < -1) || l.rect.is_empty() {
            return Ok(None);
        }
        let grey = h.color_mode == ColorMode::Grayscale;
        let (lw, lh) = l.rect.size().map_err(err)?;
        let mut image = PaintImage::new(Layout {
            width: self.width(),
            height: self.height(),
            format: self.format(),
            depth: Depth::F32,
        })?;
        let ids: &[i16] = if grey { &[0, -1] } else { &[0, 1, 2, -1] };
        let mut planes = Vec::new();
        for &id in ids {
            planes.push(match l.channel(id) {
                Some(_) => Some(l.decode_channel(id, 32, h.version).map_err(err)?),
                None => None,
            });
        }
        let sample = |p: &Option<Vec<u8>>, k: usize, missing: f32| -> f32 {
            p.as_ref()
                .and_then(|b| b.get(k * 4..k * 4 + 4))
                .map_or(missing, |s| f32::from_be_bytes([s[0], s[1], s[2], s[3]]))
        };
        let (w, hh) = (self.width() as i64, self.height() as i64);
        for y in 0..lh {
            for x in 0..lw {
                let (cx, cy) = (l.rect.left as i64 + x as i64, l.rect.top as i64 + y as i64);
                if cx < 0 || cy < 0 || cx >= w || cy >= hh {
                    continue;
                }
                let k = y * lw + x;
                let px = if grey {
                    let g = sample(&planes[0], k, 0.);
                    [g, g, g, sample(&planes[1], k, 1.)]
                } else {
                    [
                        sample(&planes[0], k, 0.),
                        sample(&planes[1], k, 0.),
                        sample(&planes[2], k, 0.),
                        sample(&planes[3], k, 1.),
                    ]
                };
                image.set((cy * w + cx) as usize, px);
            }
        }
        Ok(Some(image))
    }

    fn float_picture(&self, data: &[[f32; 4]]) -> PaintImage {
        let mut image = PaintImage::new(Layout {
            width: self.width(),
            height: self.height(),
            format: self.format(),
            depth: self.depth(),
        })
        .expect("the document's own size");
        let linear = self.file.header.depth == 32;
        for (i, p) in data.iter().enumerate().take(image.pixel_count()) {
            let p = if linear {
                [
                    srgb_decode(p[0]),
                    srgb_decode(p[1]),
                    srgb_decode(p[2]),
                    p[3],
                ]
            } else {
                *p
            };
            image.set(i, p);
        }
        // Grey documents keep their transparency in Rgba only; a grey
        // composite with alpha reads as grey here (provisional).
        image
    }

    /// Adds a pixel layer on top, "Operandi Paint 1", "… 2" (PNT-304), and
    /// returns its index.
    pub fn add_paint_layer(&mut self) -> Result<usize> {
        let taken: Vec<String> = self.file.layers().iter().map(|l| l.name()).collect();
        let name = (1..)
            .map(|n| format!("{PAINT_LAYER_BASE} {n}"))
            .find(|n| !taken.contains(n))
            .expect("some number is free");
        let (w, h) = (self.width(), self.height());
        // A one-layer document of the same size gives a well-formed record.
        let mut b = PsdBuilder::new(w, h);
        b.push_layer(LayerSpec::new(
            name,
            0,
            0,
            w,
            h,
            PixelData::Rgba8(vec![0; w as usize * h as usize * 4]),
        ));
        let mut record = b
            .build()
            .map_err(err)?
            .layers()
            .first()
            .cloned()
            .ok_or("Couldn't make a layer")?;
        // Grey documents take a grey layer: alpha and channel 0 only.
        if self.file.header.color_mode == ColorMode::Grayscale {
            record.channels.retain(|c| c.id == -1 || c.id == 0);
        }
        record.blend_mode = BlendMode::Normal;
        let layers = self.file.layers_mut();
        layers.push(record);
        let index = layers.len() - 1;
        // Re-encode at the document's depth.
        let blank = PaintImage::new(Layout {
            width: w,
            height: h,
            format: self.format(),
            depth: self.depth(),
        })?;
        self.set_layer(index, &blank)?;
        Ok(index)
    }

    /// Writes `image` (canvas-sized) into pixel layer `index` at the
    /// document's depth; the layer then covers the whole canvas. Masks and
    /// every other setting stay. Marks the composite stale.
    pub fn set_layer(&mut self, index: usize, image: &PaintImage) -> Result<()> {
        if !kind_of(&self.file, index).paintable() {
            return Err("Only pixel layers can be painted".into());
        }
        let (w, h) = (self.width(), self.height());
        if image.width != w || image.height != h {
            return Err("The picture must be the document's size".into());
        }
        let depth = self.file.header.depth;
        let version = self.file.header.version;
        let grey = self.file.header.color_mode == ColorMode::Grayscale;
        let ids: &[i16] = if grey { &[-1, 0] } else { &[-1, 0, 1, 2] };
        let compression = compression_for(depth);
        let record = self
            .file
            .layers_mut()
            .get_mut(index)
            .ok_or("No such layer")?;
        record.rect = Rect::from_xywh(0, 0, w, h);
        for &id in ids {
            let plane = plane(image, id, depth, grey);
            match record.channels.iter_mut().find(|c| c.id == id) {
                Some(c) => c
                    .set_decoded(compression, &plane, w as usize, h as usize, depth, version)
                    .map_err(err)?,
                None => record.channels.push(
                    ChannelData::encode(
                        id,
                        compression,
                        &plane,
                        w as usize,
                        h as usize,
                        depth,
                        version,
                    )
                    .map_err(err)?,
                ),
            }
        }
        self.composite_stale = true;
        Ok(())
    }

    /// Rebuilds the stored composite from the visible pixel layers
    /// (PNT-303). Returns the note when text or adjustment layers were
    /// left out.
    pub fn rebuild_composite(&mut self) -> Result<Option<&'static str>> {
        let (w, h) = (self.width(), self.height());
        let layout = Layout {
            width: w,
            height: h,
            format: Format::Rgba,
            depth: Depth::F32,
        };
        let mut skipped = false;
        let mut pictures = Vec::new();
        let hidden = self.hidden_by_groups();
        for (i, l) in self.file.layers().iter().enumerate() {
            match kind_of(&self.file, i) {
                LayerKind::Pixel | LayerKind::Fill => {}
                LayerKind::Group => continue,
                LayerKind::Text | LayerKind::Adjustment | LayerKind::Smart => {
                    skipped |= l.is_visible() && !hidden[i];
                    continue;
                }
            }
            if l.section_divider().is_some() || !l.is_visible() || hidden[i] {
                continue;
            }
            let image = match self.raw_float_layer(i)? {
                Some(exact) => crate::io::converted(&exact, Format::Rgba, Depth::F32),
                None => {
                    let f = self.file.layer_f32(i, true).map_err(err)?;
                    crate::io::converted(&self.float_picture(&f.data), Format::Rgba, Depth::F32)
                }
            };
            let opacity = f32::from(l.opacity) / 255. * f32::from(l.fill_opacity()) / 255.;
            pictures.push((image, blend_of(l.blend_mode), opacity));
        }
        let flat = if pictures.is_empty() {
            PaintImage::new(layout)?
        } else {
            let sources: Vec<Source<'_>> = pictures
                .iter()
                .map(|(image, blend, opacity)| Source {
                    image,
                    blend: *blend,
                    opacity: *opacity,
                    invert: false,
                })
                .collect();
            composite(&sources, layout)?
        };
        self.store_composite(&flat)?;
        self.composite_stale = false;
        Ok(skipped.then_some(SKIPPED_NOTE))
    }

    /// Layers inside a hidden group.
    fn hidden_by_groups(&self) -> Vec<bool> {
        let mut hidden = vec![false; self.file.layers().len()];
        fn walk(doc: &PsdDoc, nodes: &[LayerNode], off: bool, hidden: &mut [bool]) {
            for node in nodes {
                if let Some(h) = hidden.get_mut(node.index()) {
                    *h = off;
                }
                if let LayerNode::Group {
                    index,
                    divider,
                    children,
                } = node
                {
                    let visible = doc
                        .file
                        .layers()
                        .get(*index)
                        .is_some_and(|l| l.is_visible());
                    if let Some(d) = divider.and_then(|d| hidden.get_mut(d)) {
                        *d = true;
                    }
                    walk(doc, children, off || !visible, hidden);
                }
            }
        }
        walk(self, &self.file.layer_tree(), false, &mut hidden);
        hidden
    }

    /// Writes `flat` as the merged image, keeping any extra channels.
    fn store_composite(&mut self, flat: &PaintImage) -> Result<()> {
        let header = self.file.header.clone();
        let depth = header.depth;
        let grey = header.color_mode == ColorMode::Grayscale;
        let colour = if grey { 1 } else { 3 };
        let with_alpha = self.file.merged_has_alpha();
        let plane_len = header.row_bytes() * header.height as usize;
        let mut planes = self.file.decode_merged().unwrap_or_default();
        planes.resize(plane_len * usize::from(header.channels), 0);
        let ids: Vec<i16> = (0..colour as i16).chain(with_alpha.then_some(-1)).collect();
        for (k, id) in ids.iter().enumerate() {
            let mut p = plane(flat, *id, depth, grey);
            if *id >= 0 && with_alpha && !grey {
                // RGB composites are stored matted against white.
                matte(&mut p, &plane(flat, -1, depth, grey), depth);
            }
            planes[k * plane_len..(k + 1) * plane_len].copy_from_slice(&p);
        }
        self.file.image_data =
            ImageData::encode(compression_for(depth), &planes, &header).map_err(err)?;
        Ok(())
    }

    /// The whole document, the composite refreshed when stale (PNT-306).
    pub fn to_bytes(&mut self) -> Result<Vec<u8>> {
        if self.composite_stale {
            self.rebuild_composite()?;
        }
        self.file.to_bytes().map_err(err)
    }

    pub fn write(&mut self, path: &Path) -> Result<()> {
        let bytes = self.to_bytes()?;
        crate::io::write_atomic(path, |temp| {
            std::fs::write(temp, &bytes)
                .map_err(|e| format!("Cannot write {}: {e}", path.display()))
        })
    }

    /// A new document with one pixel layer "Layer 1" holding `image`, as
    /// New Image saves to PSD/PSB (PNT-308). Float images become 32-bit
    /// documents, others keep 8 or 16 bits.
    pub fn new_from(image: &PaintImage, psb: bool) -> Result<PsdDoc> {
        let (w, h) = (image.width, image.height);
        if !psb && w.max(h) > PSD_MAX_SIDE {
            return Err(format!(
                "A PSD can be at most {PSD_MAX_SIDE} pixels a side; save as PSB"
            ));
        }
        let grey = image.format == Format::Gray;
        let n = w as usize * h as usize;
        // Built at 8 bits, then re-encoded at the image's depth.
        let blank = if grey {
            PixelData::GrayA8(vec![0; n * 2])
        } else {
            PixelData::Rgba8(vec![0; n * 4])
        };
        let mut b = PsdBuilder::new(w, h);
        if psb {
            b = b.version(Version::Psb);
        }
        b.push_layer(LayerSpec::new("Layer 1", 0, 0, w, h, blank.clone()));
        b.composite(blank);
        let mut file = b.build().map_err(err)?;
        set_depth(&mut file, image.depth.bits() as u16);
        let mut doc = PsdDoc {
            file,
            composite_stale: true,
        };
        let layout = Layout {
            width: w,
            height: h,
            format: if grey { Format::Gray } else { Format::Rgba },
            depth: image.depth,
        };
        let converted = crate::io::converted(image, layout.format, layout.depth);
        doc.set_layer(0, &converted)?;
        doc.rebuild_composite()?;
        Ok(doc)
    }
}

/// Changes a document's depth (its layers must be re-encoded after).
/// 16- and 32-bit documents keep their layers in a global `Lr16` / `Lr32`
/// block, as Photoshop writes them.
fn set_depth(file: &mut PsdFile, bits: u16) {
    file.header.depth = bits;
    file.layer_info_placement = match bits {
        16 | 32 => craft_psd::LayerInfoPlacement::GlobalBlock {
            index: usize::MAX,
            signature: *b"8BIM",
            key: if bits == 16 { *b"Lr16" } else { *b"Lr32" },
            padding: None,
        },
        _ => craft_psd::LayerInfoPlacement::Section,
    };
}

/// The inverse of craft-psd's sRGB encoding of 32-bit (linear) documents.
fn srgb_decode(v: f32) -> f32 {
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// How painted channels are compressed: RLE at 8 and 16 bits, zip with
/// prediction for floats (what Photoshop writes).
fn compression_for(depth: u16) -> Compression {
    if depth == 32 {
        Compression::ZipPrediction
    } else {
        Compression::Rle
    }
}

/// One channel of `image` as big-endian samples at `depth`: id -1 is
/// alpha, 0..2 the colours (0 the grey of a grey document).
fn plane(image: &PaintImage, id: i16, depth: u16, grey: bool) -> Vec<u8> {
    let n = image.pixel_count();
    let bytes = (depth / 8) as usize;
    let mut out = Vec::with_capacity(n * bytes);
    for i in 0..n {
        let p = image.get(i);
        let v = match id {
            -1 => p[3],
            0 if grey => crate::image::luma([p[0], p[1], p[2]]),
            c => p[c as usize],
        };
        match depth {
            8 => out.push((v.clamp(0., 1.) * 255.).round() as u8),
            16 => out.extend(((v.clamp(0., 1.) * 65535.).round() as u16).to_be_bytes()),
            _ => out.extend(v.to_be_bytes()),
        }
    }
    out
}

/// Mattes colour samples against white by alpha, in place.
fn matte(colour: &mut [u8], alpha: &[u8], depth: u16) {
    let bytes = (depth / 8) as usize;
    let read = |s: &[u8]| -> f32 {
        match depth {
            8 => f32::from(s[0]) / 255.,
            16 => f32::from(u16::from_be_bytes([s[0], s[1]])) / 65535.,
            _ => f32::from_be_bytes([s[0], s[1], s[2], s[3]]),
        }
    };
    for (c, a) in colour.chunks_mut(bytes).zip(alpha.chunks(bytes)) {
        let (cv, av) = (read(c), read(a));
        let v = cv * av + (1. - av);
        match depth {
            8 => c[0] = (v.clamp(0., 1.) * 255.).round() as u8,
            16 => c.copy_from_slice(&((v.clamp(0., 1.) * 65535.).round() as u16).to_be_bytes()),
            _ => c.copy_from_slice(&v.to_be_bytes()),
        }
    }
}

/// Photoshop's blend modes as the compositor's; modes it doesn't have
/// rebuild as Normal (provisional, PNT-Q08).
fn blend_of(mode: BlendMode) -> Blend {
    match mode {
        BlendMode::Multiply => Blend::Multiply,
        BlendMode::Screen => Blend::Screen,
        BlendMode::Overlay => Blend::Overlay,
        BlendMode::SoftLight => Blend::SoftLight,
        BlendMode::HardLight => Blend::HardLight,
        BlendMode::Darken => Blend::Darken,
        BlendMode::Lighten => Blend::Lighten,
        BlendMode::ColorDodge => Blend::ColorDodge,
        BlendMode::ColorBurn => Blend::ColorBurn,
        BlendMode::Difference => Blend::Difference,
        BlendMode::Subtract => Blend::Subtract,
        BlendMode::LinearDodge => Blend::Add,
        _ => Blend::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(depth: Depth, format: Format) -> PaintImage {
        let mut i = PaintImage::new(Layout {
            width: 64,
            height: 64,
            format,
            depth,
        })
        .unwrap();
        for p in 0..i.pixel_count() {
            let v = (p % 64) as f32 / 63.;
            i.set(p, [v, 1. - v, 0.25, 1.]);
        }
        i
    }

    /// Two pixel layers in a group, a hidden layer, at `depth`.
    fn layered(depth: u16) -> Vec<u8> {
        use craft_psd::GroupSpec;
        let n = 64 * 64;
        let red: Vec<u16> = (0..n).flat_map(|_| [65535, 0, 0, 65535]).collect();
        let half: Vec<u16> = (0..n).flat_map(|_| [0, 0, 65535, 32768]).collect();
        let mut b = PsdBuilder::new(64, 64).depth(16);
        b.push_layer(LayerSpec::new(
            "Base",
            0,
            0,
            64,
            64,
            PixelData::Rgba16(red.clone()),
        ));
        b.begin_group(GroupSpec::new("Details"));
        b.push_layer(LayerSpec::new(
            "Blue",
            0,
            0,
            64,
            64,
            PixelData::Rgba16(half),
        ));
        b.end_group().unwrap();
        b.composite(PixelData::Rgba16(red));
        let mut doc = PsdDoc {
            file: b.build().unwrap(),
            composite_stale: false,
        };
        if depth != 16 {
            // The same pictures re-encoded at the other depth.
            let target = depth_of(depth).unwrap();
            let ids: Vec<usize> = (0..doc.file.layers().len())
                .filter(|&i| {
                    kind_of(&doc.file, i).paintable()
                        && doc.file.layers()[i].section_divider().is_none()
                })
                .collect();
            let layers: Vec<(usize, PaintImage)> = ids
                .into_iter()
                .map(|i| (i, doc.picture(Pick::Layer(i)).unwrap().0))
                .collect();
            set_depth(&mut doc.file, depth);
            for (i, px) in layers {
                let px = crate::io::converted(&px, Format::Rgba, target);
                doc.set_layer(i, &px).unwrap();
            }
        }
        doc.to_bytes().unwrap()
    }

    #[test]
    fn unmodified_documents_round_trip_byte_for_byte() {
        for depth in [8, 16, 32] {
            let bytes = layered(depth);
            let mut doc = PsdDoc::from_bytes(&bytes).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), bytes, "{depth}-bit");
        }
    }

    #[test]
    fn decode_save_decode_keeps_layers_pixels_and_metadata() {
        for depth in [8u16, 16, 32] {
            let bytes = layered(depth);
            let mut first = PsdDoc::from_bytes(&bytes).unwrap();
            let rows = first.rows();
            let pictures: Vec<PaintImage> = rows
                .iter()
                .filter(|r| r.paintable())
                .map(|r| first.picture(r.pick).unwrap().0)
                .collect();
            // Painting a layer and saving: everything else must come back.
            let base = first
                .layer_named("Base")
                .unwrap_or_else(|| panic!("{depth}: {:?}", first.rows()));
            let (base_px, _) = first.picture(Pick::Layer(base)).unwrap();
            first.set_layer(base, &base_px).unwrap();
            let saved = first.to_bytes().unwrap();
            let mut second = PsdDoc::from_bytes(&saved).unwrap();
            assert_eq!(second.rows(), rows, "{depth}-bit layer list");
            assert_eq!(second.file.header, first.file.header);
            assert_eq!(second.file.resources, first.file.resources);
            let again: Vec<PaintImage> = rows
                .iter()
                .filter(|r| r.paintable())
                .map(|r| second.picture(r.pick).unwrap().0)
                .collect();
            assert_eq!(again, pictures, "{depth}-bit pixels");
        }
    }

    #[test]
    fn rows_list_composite_then_layers_top_first_with_groups_indented() {
        let doc = PsdDoc::from_bytes(&layered(8)).unwrap();
        let rows: Vec<(String, usize, LayerKind)> = doc
            .rows()
            .into_iter()
            .map(|r| (r.name, r.indent, r.kind))
            .collect();
        assert_eq!(rows[0], ("Composite".into(), 0, LayerKind::Pixel));
        assert_eq!(rows[1], ("Details".into(), 0, LayerKind::Group));
        assert_eq!(rows[2], ("Blue".into(), 1, LayerKind::Pixel));
        assert_eq!(rows[3], ("Base".into(), 0, LayerKind::Pixel));
        assert!(!doc.rows()[1].paintable());
        assert!(doc.rows()[2].paintable());
        assert!(!doc.rows()[0].paintable(), "the Composite asks which layer");
    }

    #[test]
    fn painting_a_layer_rebuilds_the_composite_on_save() {
        let mut doc = PsdDoc::from_bytes(&layered(8)).unwrap();
        let base = doc.layer_named("Base").unwrap();
        let green = PaintImage::filled(
            Layout {
                width: 64,
                height: 64,
                format: Format::Rgba,
                depth: Depth::U8,
            },
            Some([0., 1., 0.]),
        )
        .unwrap();
        doc.set_layer(base, &green).unwrap();
        assert!(doc.composite_stale);
        let bytes = doc.to_bytes().unwrap();
        let mut back = PsdDoc::from_bytes(&bytes).unwrap();
        let (c, _) = back.picture(Pick::Composite).unwrap();
        let p = c.get(0);
        // Half-transparent blue over green.
        assert!(p[1] > 0.4 && p[2] > 0.4 && p[0] < 0.05, "{p:?}");
    }

    #[test]
    fn paint_layers_are_numbered_and_on_top() {
        let mut doc = PsdDoc::from_bytes(&layered(16)).unwrap();
        let a = doc.add_paint_layer().unwrap();
        let b = doc.add_paint_layer().unwrap();
        let rows = doc.rows();
        assert_eq!(rows[1].name, "Operandi Paint 2");
        assert_eq!(rows[2].name, "Operandi Paint 1");
        assert_eq!(
            (a, b),
            (doc.file.layers().len() - 2, doc.file.layers().len() - 1)
        );
        let back = PsdDoc::from_bytes(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(back.rows().len(), rows.len());
        assert_eq!(back.file.header.depth, 16);
    }

    #[test]
    fn new_documents_hold_one_layer_at_the_image_depth() {
        for (depth, bits) in [(Depth::U8, 8), (Depth::U16, 16), (Depth::F32, 32)] {
            let image = ramp(depth, Format::Rgba);
            for psb in [false, true] {
                let mut doc = PsdDoc::new_from(&image, psb).unwrap();
                let mut back = PsdDoc::from_bytes(&doc.to_bytes().unwrap()).unwrap();
                assert_eq!(back.is_psb(), psb);
                assert_eq!(back.file.header.depth, bits);
                let rows = back.rows();
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[1].name, "Layer 1");
                let (px, _) = back.picture(rows[1].pick).unwrap();
                assert_eq!(px, image, "{bits}-bit layer pixels");
            }
        }
    }

    #[test]
    fn stale_composites_note_skipped_text_layers_only_when_there_are_some() {
        let mut doc = PsdDoc::from_bytes(&layered(8)).unwrap();
        assert_eq!(doc.rebuild_composite().unwrap(), None);
    }
}
