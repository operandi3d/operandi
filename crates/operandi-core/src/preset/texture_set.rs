//! Texture sets (CONTENT-DIRECTORIES CD-20..27): pictures in one folder
//! that make one material, matched by base name and map suffix, and their
//! Material preset (CD-25).
//!
//! The one texture-set loader: the preset browser (preset/library.rs)
//! lists and loads sets through it, and the material converter
//! (preset/convert.rs) builds its presets with it.
use super::{Preset, PresetHeader, PresetKind};
use crate::shader_tree::{
    BlendMode, GroupMask, ImageSettings, Layer, NewLayer, Node, NodeKind, ShaderTree, Swizzle,
    TextureEffect, TextureLayer, TextureSource,
};
use crate::{Result, Scene};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A texture map's job (CD-21).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MapKind {
    BaseColor,
    Roughness,
    /// Inverted roughness.
    Gloss,
    Metallic,
    /// OpenGL normal (Y up).
    NormalGl,
    /// DirectX normal (Y down): used with green flipped (CD-22).
    NormalDx,
    /// A normal map that doesn't say; treated as OpenGL (CD-22).
    Normal,
    Height,
    AmbientOcclusion,
    Opacity,
    Emission,
    /// Packed AO (R), roughness (G) and metal (B).
    Packed,
}

impl MapKind {
    pub fn label(self) -> &'static str {
        match self {
            MapKind::BaseColor => "Base Color",
            MapKind::Roughness => "Roughness",
            MapKind::Gloss => "Gloss",
            MapKind::Metallic => "Metallic",
            MapKind::NormalGl => "Normal (OpenGL)",
            MapKind::NormalDx => "Normal (DirectX)",
            MapKind::Normal => "Normal",
            MapKind::Height => "Height",
            MapKind::AmbientOcclusion => "Ambient Occlusion",
            MapKind::Opacity => "Opacity",
            MapKind::Emission => "Emission",
            MapKind::Packed => "AO/Roughness/Metal",
        }
    }
}

/// CD-21's suffixes, separators removed. Checked as whole words at the
/// name's end; when several fit, the one spanning the most words wins
/// ("nor_dx" over "dx"), which is CD-21's "longest first".
///
/// A bare "mask" is deliberately not an opacity map (provisional,
/// PB1-Q15): Poly Haven uses it for cut-outs (hessian) and for paint/wear
/// masks alike (wood_peeling_paint_weathered), and wiring a paint mask to
/// Dissolve punches holes in the material. Such files are left unused.
const SUFFIXES: &[(&str, MapKind)] = &[
    ("ambientocclusion", MapKind::AmbientOcclusion),
    ("basecolor", MapKind::BaseColor),
    ("displacement", MapKind::Height),
    ("glossiness", MapKind::Gloss),
    ("roughness", MapKind::Roughness),
    ("metalness", MapKind::Metallic),
    ("occlusion", MapKind::AmbientOcclusion),
    ("emissive", MapKind::Emission),
    ("emission", MapKind::Emission),
    ("normalgl", MapKind::NormalGl),
    ("normaldx", MapKind::NormalDx),
    ("metallic", MapKind::Metallic),
    ("diffuse", MapKind::BaseColor),
    ("opacity", MapKind::Opacity),
    ("albedo", MapKind::BaseColor),
    ("colour", MapKind::BaseColor),
    ("height", MapKind::Height),
    ("normal", MapKind::Normal),
    ("color", MapKind::BaseColor),
    ("gloss", MapKind::Gloss),
    ("norgl", MapKind::NormalGl),
    ("nrmgl", MapKind::NormalGl),
    ("nordx", MapKind::NormalDx),
    ("alpha", MapKind::Opacity),
    ("metal", MapKind::Metallic),
    ("rough", MapKind::Roughness),
    ("diff", MapKind::BaseColor),
    ("disp", MapKind::Height),
    ("col", MapKind::BaseColor),
    ("rgh", MapKind::Roughness),
    ("nor", MapKind::Normal),
    ("nrm", MapKind::Normal),
    ("arm", MapKind::Packed),
    ("orm", MapKind::Packed),
    ("ao", MapKind::AmbientOcclusion),
];

/// Format words that may follow a resolution tag ("4k-jpg", "_1K-PNG").
const FORMAT_WORDS: &[&str] = &[
    "jpg", "jpeg", "png", "exr", "tif", "tiff", "tga", "bmp", "hdr",
];

/// Words that mark a set's own preview picture (CD-24).
const PREVIEW_WORDS: &[&str] = &["preview", "thumb", "thumbnail"];

/// A file name split into words at `_`, `-`, space and `.`, with the
/// byte range of each word in the original name.
fn words(stem: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in stem.char_indices() {
        let separator = matches!(c, '_' | '-' | ' ' | '.');
        match (separator, start) {
            (true, Some(s)) => {
                out.push((s, i));
                start = None;
            }
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, stem.len()));
    }
    out
}

/// A resolution tag such as `1k`, `2K`, `4k` (in pixels/1024).
fn resolution(word: &str) -> Option<u32> {
    let lower = word.to_ascii_lowercase();
    let digits = lower.strip_suffix('k')?;
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

/// What a picture's name says about it within a texture set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapName {
    /// One map of a set.
    Map {
        /// Lower-case base with separators removed: the grouping key.
        key: String,
        /// The base as written in the file name, for the entry's name.
        base: String,
        kind: MapKind,
        /// The resolution tag in thousands of pixels, if the name has one.
        resolution: Option<u32>,
    },
    /// A set's preview picture (CD-24).
    Preview { key: String },
}

/// Read a picture's file name (without extension) as a texture map
/// (CD-20, CD-21). Case and separators don't matter; a trailing
/// resolution/format tag (`1k`, `4k-jpg`) is ignored wherever it sits
/// next to the suffix.
pub fn parse_map_name(stem: &str) -> Option<MapName> {
    let mut spans = words(stem);
    let lower = |s: &(usize, usize)| stem[s.0..s.1].to_ascii_lowercase();
    let mut res = None;
    let strip_tags = |spans: &mut Vec<(usize, usize)>, res: &mut Option<u32>| {
        while let Some(last) = spans.last() {
            let w = lower(last);
            if FORMAT_WORDS.contains(&w.as_str()) {
                spans.pop();
            } else if let Some(r) = resolution(&w) {
                *res = res.or(Some(r));
                spans.pop();
            } else {
                break;
            }
        }
    };
    strip_tags(&mut spans, &mut res);
    if spans.len() < 2 {
        return None;
    }
    let key_of = |spans: &[(usize, usize)]| spans.iter().map(lower).collect::<String>();
    let base_of = |spans: &[(usize, usize)]| {
        let (first, last) = (spans[0], spans[spans.len() - 1]);
        stem[first.0..last.1].to_owned()
    };
    if PREVIEW_WORDS.contains(&lower(spans.last().expect("two or more")).as_str()) {
        let mut base = spans[..spans.len() - 1].to_vec();
        strip_tags(&mut base, &mut res);
        if base.is_empty() {
            return None;
        }
        return Some(MapName::Preview { key: key_of(&base) });
    }
    // The suffix may span up to three words ("nor_gl", "base_color").
    for take in (1..=3.min(spans.len() - 1)).rev() {
        let suffix: String = spans[spans.len() - take..].iter().map(lower).collect();
        if let Some(&(_, kind)) = SUFFIXES.iter().find(|(s, _)| *s == suffix) {
            let mut base = spans[..spans.len() - take].to_vec();
            strip_tags(&mut base, &mut res);
            if base.is_empty() {
                return None;
            }
            return Some(MapName::Map {
                key: key_of(&base),
                base: base_of(&base),
                kind,
                resolution: res,
            });
        }
    }
    None
}

/// One map file of a set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapFile {
    pub kind: MapKind,
    pub resolution: Option<u32>,
    pub path: PathBuf,
}

/// Two or more pictures in one folder that make one material (CD-20).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureSet {
    /// The base name (CD-14).
    pub name: String,
    pub maps: Vec<MapFile>,
    /// A `*preview*` / `*thumb*` picture next to the set (CD-24).
    pub preview: Option<PathBuf>,
}

/// Which resolution a texture set loads (CD-23, BQ-03).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResolutionChoice {
    Lowest,
    #[default]
    Highest,
    /// This many thousand pixels, or the nearest below it (else the
    /// lowest) [chosen].
    Set(u32),
}

impl TextureSet {
    /// Every file of the set, preview included.
    pub fn files(&self) -> impl Iterator<Item = &Path> {
        self.maps
            .iter()
            .map(|m| m.path.as_path())
            .chain(self.preview.as_deref())
    }

    /// The picture that stands for the set until a rendered ball replaces
    /// it (CD-24): its preview, else its base colour map, else its first.
    pub fn representative(&self) -> &Path {
        if let Some(p) = &self.preview {
            return p;
        }
        self.maps
            .iter()
            .find(|m| m.kind == MapKind::BaseColor)
            .or(self.maps.first())
            .map(|m| m.path.as_path())
            .unwrap_or(Path::new(""))
    }

    /// The resolutions found, lowest first (`None` for untagged maps).
    pub fn resolutions(&self) -> Vec<Option<u32>> {
        let mut r: Vec<Option<u32>> = self.maps.iter().map(|m| m.resolution).collect();
        r.sort();
        r.dedup();
        r
    }

    /// The maps to load at `choice` (CD-22, CD-23): one file per map, and
    /// one normal map (OpenGL over DirectX over unknown).
    pub fn pick(&self, choice: ResolutionChoice) -> Vec<&MapFile> {
        let found = self.resolutions();
        let want = match choice {
            ResolutionChoice::Lowest => found.first().copied().flatten(),
            ResolutionChoice::Highest => found.last().copied().flatten(),
            ResolutionChoice::Set(r) => found
                .iter()
                .rev()
                .flatten()
                .find(|&&f| f <= r)
                .copied()
                .or(found.first().copied().flatten()),
        };
        let mut by_kind: BTreeMap<MapKind, &MapFile> = BTreeMap::new();
        for map in &self.maps {
            let better = |old: &MapFile| {
                // The wanted resolution, else the closest one.
                let distance = |m: &MapFile| match (m.resolution, want) {
                    (Some(a), Some(b)) => a.abs_diff(b),
                    (None, None) => 0,
                    _ => u32::MAX / 2,
                };
                distance(map) < distance(old)
            };
            match by_kind.get(&map.kind) {
                Some(old) if !better(old) => {}
                _ => {
                    by_kind.insert(map.kind, map);
                }
            }
        }
        let normal = [MapKind::NormalGl, MapKind::NormalDx, MapKind::Normal]
            .into_iter()
            .find(|k| by_kind.contains_key(k));
        by_kind
            .into_iter()
            .filter(|(k, _)| {
                !matches!(k, MapKind::NormalGl | MapKind::NormalDx | MapKind::Normal)
                    || Some(*k) == normal
            })
            .map(|(_, m)| m)
            .collect()
    }

    /// The set as a Material preset (CD-25): a material group named after
    /// the set, holding a material and one image layer per map, each with
    /// its effect. Pictures are linked by path. Colour maps render as sRGB
    /// and the rest as plain numbers, which the renderers already decide by
    /// effect. `size_of` gives a picture's pixel size (the core can't read
    /// pictures); one it can't read is left out with a note.
    pub fn material_preset(
        &self,
        choice: ResolutionChoice,
        size_of: &dyn Fn(&Path) -> Option<(u32, u32)>,
    ) -> Result<(Preset, Vec<String>)> {
        let mut scene = Scene {
            shader_tree: ShaderTree {
                node_views: Default::default(),
                nodes: Vec::new(),
                next_id: 1,
                outputs: Default::default(),
                node_graphs: Default::default(),
            },
            ..Scene::default()
        };
        let mut notes = Vec::new();
        let mut later_notes: Vec<String> = Vec::new();
        let group = node(1, None, &self.name, NodeKind::Group(GroupMask::default()));
        let mut nodes = vec![group];
        let mut next = 2;
        let mut layer = |nodes: &mut Vec<Node>,
                         scene: &mut Scene,
                         map: &MapFile,
                         effect: TextureEffect,
                         settings: ImageSettings,
                         blend: BlendMode|
         -> Result<()> {
            let Some((w, h)) = size_of(&map.path) else {
                notes.push(format!(
                    "{} left out: the picture can't be read",
                    file_name(&map.path)
                ));
                return Ok(());
            };
            let image = scene.image_for_path(&map.path.to_string_lossy(), w, h)?;
            let mut kind = NewLayer::ImageMap.kind();
            if let NodeKind::Texture(TextureLayer {
                source, effect: e, ..
            }) = &mut kind
            {
                *source = TextureSource::ImageMap {
                    image: Some(image),
                    settings: Box::new(settings),
                };
                *e = effect;
            }
            let mut n = node(
                next,
                Some(1),
                &format!("{} {}", self.name, map.kind.label()),
                kind,
            );
            n.layer = Layer {
                blend,
                ..Layer::default()
            };
            next += 1;
            nodes.push(n);
            Ok(())
        };
        let picked = self.pick(choice);
        // Top of the list first: the AO multiply sits above the colour it
        // darkens [BQ-04].
        let order = [
            MapKind::AmbientOcclusion,
            MapKind::BaseColor,
            MapKind::Packed,
            MapKind::Roughness,
            MapKind::Gloss,
            MapKind::Metallic,
            MapKind::NormalGl,
            MapKind::NormalDx,
            MapKind::Normal,
            MapKind::Height,
            MapKind::Opacity,
            MapKind::Emission,
        ];
        let has_color = picked.iter().any(|m| m.kind == MapKind::BaseColor);
        // AO multiplies the base colour from the top (BQ-04, BMC-Q09).
        if has_color {
            for (map, settings) in ao_maps(&picked) {
                layer(
                    &mut nodes,
                    &mut scene,
                    map,
                    TextureEffect::DiffuseColor,
                    settings,
                    BlendMode::Multiply,
                )?;
            }
        }
        for kind in order {
            for map in picked.iter().filter(|m| m.kind == kind) {
                let plain = ImageSettings::default;
                match kind {
                    // BQ-04 default: multiply onto base colour; with no
                    // colour map there's nothing to darken.
                    // Added above, from `ao_maps`.
                    MapKind::AmbientOcclusion if has_color => {}
                    MapKind::AmbientOcclusion => {
                        later_notes.push("AO map left out: the set has no base colour".into())
                    }
                    MapKind::BaseColor => layer(
                        &mut nodes,
                        &mut scene,
                        map,
                        TextureEffect::DiffuseColor,
                        plain(),
                        BlendMode::Normal,
                    )?,
                    MapKind::Packed => {
                        // R = AO, G = roughness, B = metal. Separate maps
                        // win: the packed one fills only what's missing
                        // [chosen, PB1-Q07].
                        let has = |k: MapKind| picked.iter().any(|m| m.kind == k);
                        let channel = |swizzle| ImageSettings { swizzle, ..plain() };
                        // Its AO (red) went on top, above the base colour.
                        for (effect, swizzle, separate) in [
                            (
                                TextureEffect::Roughness,
                                Swizzle::Green,
                                has(MapKind::Roughness) || has(MapKind::Gloss),
                            ),
                            (
                                TextureEffect::Metallic,
                                Swizzle::Blue,
                                has(MapKind::Metallic),
                            ),
                        ] {
                            if separate {
                                continue;
                            }
                            layer(
                                &mut nodes,
                                &mut scene,
                                map,
                                effect,
                                channel(swizzle),
                                BlendMode::Normal,
                            )?;
                        }
                    }
                    MapKind::Roughness => layer(
                        &mut nodes,
                        &mut scene,
                        map,
                        TextureEffect::Roughness,
                        plain(),
                        BlendMode::Normal,
                    )?,
                    MapKind::Gloss => layer(
                        &mut nodes,
                        &mut scene,
                        map,
                        TextureEffect::Roughness,
                        ImageSettings {
                            invert_red: true,
                            invert_green: true,
                            invert_blue: true,
                            ..plain()
                        },
                        BlendMode::Normal,
                    )?,
                    MapKind::Metallic => layer(
                        &mut nodes,
                        &mut scene,
                        map,
                        TextureEffect::Metallic,
                        plain(),
                        BlendMode::Normal,
                    )?,
                    MapKind::NormalGl | MapKind::Normal => layer(
                        &mut nodes,
                        &mut scene,
                        map,
                        TextureEffect::Normal,
                        plain(),
                        BlendMode::Normal,
                    )?,
                    // CD-22: a DirectX normal has its green flipped.
                    MapKind::NormalDx => layer(
                        &mut nodes,
                        &mut scene,
                        map,
                        TextureEffect::Normal,
                        ImageSettings {
                            invert_green: true,
                            ..plain()
                        },
                        BlendMode::Normal,
                    )?,
                    MapKind::Height => layer(
                        &mut nodes,
                        &mut scene,
                        map,
                        TextureEffect::Displacement,
                        plain(),
                        BlendMode::Normal,
                    )?,
                    // Provisional (PB1-Q03): opacity drives Dissolve, white
                    // = solid, so the picture is inverted.
                    MapKind::Opacity => layer(
                        &mut nodes,
                        &mut scene,
                        map,
                        TextureEffect::Dissolve,
                        ImageSettings {
                            invert_red: true,
                            invert_green: true,
                            invert_blue: true,
                            ..plain()
                        },
                        BlendMode::Normal,
                    )?,
                    MapKind::Emission => layer(
                        &mut nodes,
                        &mut scene,
                        map,
                        TextureEffect::LuminousColor,
                        plain(),
                        BlendMode::Normal,
                    )?,
                }
            }
        }
        notes.extend(later_notes);
        if nodes.len() == 1 {
            return Err(format!("None of {}'s pictures can be read", self.name));
        }
        // The material sits at the bottom, under its textures. A height map
        // displaces 1 cm [chosen, PB1-Q04]; the new-material default is 0.
        let mut material = NewLayer::Material.kind();
        if let NodeKind::Material(m) = &mut material
            && picked.iter().any(|m| m.kind == MapKind::Height)
        {
            m.displacement_distance = 0.01;
        }
        nodes.push(node(
            next,
            Some(1),
            &format!("{} Material", self.name),
            material,
        ));
        next += 1;
        scene.shader_tree = ShaderTree {
            node_views: Default::default(),
            nodes,
            next_id: next,
            outputs: Default::default(),
            node_graphs: Default::default(),
        };
        // Each image layer gets its Texture Locator (UV Map), as a file
        // read from disk would.
        let mut next_item = scene.next_id;
        scene.give_layers_locators(&mut next_item)?;
        scene.next_id = scene.next_id.max(next_item);
        scene.validate()?;
        Ok((
            Preset {
                header: PresetHeader {
                    kind: PresetKind::Material,
                    thumbnail: None,
                    source: None,
                },
                scene,
            },
            notes,
        ))
    }
}

/// The pictures that darken a set's base colour (BQ-04), top first, with
/// their settings: its AO map, else a packed map's red (AO) channel. The
/// one rule for texture sets and for MaterialX files beside one (MX-Q03).
fn ao_maps<'a>(picked: &[&'a MapFile]) -> Vec<(&'a MapFile, ImageSettings)> {
    let separate: Vec<_> = picked
        .iter()
        .filter(|m| m.kind == MapKind::AmbientOcclusion)
        .map(|m| (*m, ImageSettings::default()))
        .collect();
    if !separate.is_empty() {
        return separate;
    }
    picked
        .iter()
        .filter(|m| m.kind == MapKind::Packed)
        .map(|m| {
            (
                *m,
                ImageSettings {
                    swizzle: Swizzle::Red,
                    ..ImageSettings::default()
                },
            )
        })
        .collect()
}

impl TextureSet {
    /// MX-Q03 (Jason, 2026-11-11): a material preset read from a MaterialX
    /// file beside this set gets the set's AO, as the set's own preset
    /// would: a Multiply layer on the base colour at the top of the group.
    /// Only when the preset colours the base with a picture and has no
    /// Multiply layer there yet. Returns notes (pictures it can't read).
    pub fn add_ambient_occlusion(
        &self,
        preset: &mut Preset,
        choice: ResolutionChoice,
        size_of: &dyn Fn(&Path) -> Option<(u32, u32)>,
    ) -> Result<Vec<String>> {
        let tree = &preset.scene.shader_tree;
        let Some(group) = tree
            .nodes
            .first()
            .filter(|n| matches!(n.kind, NodeKind::Group(_)))
        else {
            return Ok(Vec::new());
        };
        let group = (group.id, group.name.clone());
        let base_layers = |blend: BlendMode| {
            tree.nodes.iter().any(|n| {
                n.layer.blend == blend
                    && matches!(
                        &n.kind,
                        NodeKind::Texture(TextureLayer {
                            effect: TextureEffect::DiffuseColor,
                            source: TextureSource::ImageMap { image: Some(_), .. },
                            ..
                        })
                    )
            })
        };
        if !base_layers(BlendMode::Normal) || base_layers(BlendMode::Multiply) {
            return Ok(Vec::new());
        }
        let picked = self.pick(choice);
        let mut notes = Vec::new();
        let scene = &mut preset.scene;
        let mut at = 1;
        for (map, settings) in ao_maps(&picked) {
            let Some((w, h)) = size_of(&map.path) else {
                notes.push(format!(
                    "{} left out: the picture can't be read",
                    file_name(&map.path)
                ));
                continue;
            };
            let image = scene.image_for_path(&map.path.to_string_lossy(), w, h)?;
            let mut kind = NewLayer::ImageMap.kind();
            if let NodeKind::Texture(TextureLayer { source, effect, .. }) = &mut kind {
                *source = TextureSource::ImageMap {
                    image: Some(image),
                    settings: Box::new(settings),
                };
                *effect = TextureEffect::DiffuseColor;
            }
            let id = scene.shader_tree.next_id;
            scene.shader_tree.next_id += 1;
            let mut n = node(
                id,
                Some(group.0),
                &format!("{} {}", group.1, map.kind.label()),
                kind,
            );
            n.layer.blend = BlendMode::Multiply;
            scene.shader_tree.nodes.insert(at, n);
            at += 1;
        }
        let mut next_item = scene.next_id;
        scene.give_layers_locators(&mut next_item)?;
        scene.next_id = scene.next_id.max(next_item);
        scene.validate()?;
        Ok(notes)
    }
}

fn node(id: u64, parent: Option<u64>, name: &str, kind: NodeKind) -> Node {
    Node {
        id,
        parent,
        name: name.into(),
        layer: Layer::default(),
        kind,
        extras: Default::default(),
    }
}

/// Fold a folder's pictures into texture sets (CD-20): two or more maps
/// with one base. Returned sorted by name.
pub fn texture_sets(pictures: &[&Path]) -> Vec<TextureSet> {
    let mut groups: BTreeMap<String, TextureSet> = BTreeMap::new();
    let mut previews: BTreeMap<String, PathBuf> = BTreeMap::new();
    for path in pictures {
        match parse_map_name(&stem(path)) {
            Some(MapName::Map {
                key,
                base,
                kind,
                resolution,
            }) => {
                let set = groups.entry(key).or_insert_with(|| TextureSet {
                    name: base.clone(),
                    maps: Vec::new(),
                    preview: None,
                });
                // The name is written as the base colour map writes it.
                if kind == MapKind::BaseColor {
                    set.name = base;
                }
                set.maps.push(MapFile {
                    kind,
                    resolution,
                    path: path.to_path_buf(),
                });
            }
            Some(MapName::Preview { key }) => {
                previews.insert(key, path.to_path_buf());
            }
            None => {}
        }
    }
    // A picture named just the base ("Bricks113A.png" next to its maps)
    // is the set's preview too (CD-24) [chosen].
    for path in pictures {
        let key = plain_key(&stem(path));
        if !key.is_empty()
            && groups.get(&key).is_some_and(|s| s.maps.len() >= 2)
            && !groups[&key].maps.iter().any(|m| m.path == *path)
        {
            previews.entry(key).or_insert_with(|| path.to_path_buf());
        }
    }
    let mut out: Vec<TextureSet> = groups
        .into_iter()
        .filter(|(_, s)| s.maps.len() >= 2)
        .map(|(key, mut s)| {
            s.preview = previews.remove(&key);
            s.maps.sort_by(|a, b| {
                (a.kind, a.resolution, &a.path).cmp(&(b.kind, b.resolution, &b.path))
            });
            s
        })
        .collect();
    out.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    out
}

/// A whole file name as a grouping key: lower case, separators and
/// trailing resolution/format tags gone.
fn plain_key(stem: &str) -> String {
    let mut spans = words(stem);
    while let Some(last) = spans.last() {
        let w = stem[last.0..last.1].to_ascii_lowercase();
        if FORMAT_WORDS.contains(&w.as_str()) || resolution(&w).is_some() {
            spans.pop();
        } else {
            break;
        }
    }
    spans
        .iter()
        .map(|s| stem[s.0..s.1].to_ascii_lowercase())
        .collect()
}

/// Case-insensitive compare with runs of digits compared as numbers.
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut digits = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        digits.push(c);
                        it.next();
                    }
                    digits
                };
                let (da, db) = (take(&mut a), take(&mut b));
                let (ta, tb) = (da.trim_start_matches('0'), db.trim_start_matches('0'));
                let order = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if order != std::cmp::Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                let order = x.to_lowercase().cmp(y.to_lowercase());
                if order != std::cmp::Ordering::Equal {
                    return order;
                }
                a.next();
                b.next();
            }
        }
    }
}

pub(super) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub(super) fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
