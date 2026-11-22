//! USD import, stage U2: bound materials become Material Groups for their
//! polygon tags.
//!
//! A material's shading network (the shaders its `outputs:surface` or
//! `outputs:mtlx:surface` reaches, with values and connections) is written
//! out as a MaterialX document and read by Operandi's own MaterialX reader
//! (`operandi_core::materialx`), which already knows MaterialX's
//! open_pbr_surface / standard_surface networks and UsdPreviewSurface with
//! UsdUVTexture, UsdPrimvarReader and UsdTransform2d. So both kinds of
//! network map the same way a .mtlx file does, Texture Locators included.
//! The preferred network (the import option) is tried first, the other
//! one when the reader can't take it; a material neither gives stays a
//! plain polygon tag, and the report says why.
use crate::import::Assets;
use crate::stage::{MaterialData, ShaderNode, StageData};
use operandi_core::interchange::{UsdImportOptions, UsdMaterials};
use operandi_core::materialx::doc::{Document, Element, Port};
use operandi_core::{Result, Scene};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// What the materials became, for the report.
#[derive(Debug, Default)]
pub struct Summary {
    pub materialx: usize,
    pub preview: usize,
    pub plain: usize,
    pub notes: Vec<String>,
}
impl Summary {
    pub fn total(&self) -> usize {
        self.materialx + self.preview + self.plain
    }
}

/// The two networks a material can carry.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Network {
    MaterialX,
    Preview,
}
impl Network {
    fn label(self) -> &'static str {
        match self {
            Network::MaterialX => "MaterialX",
            Network::Preview => "Preview Surface",
        }
    }
}

/// Adds a Material Group for each material in `used` (indices into the
/// stage's materials; `tags` names their polygon tags).
pub fn add(
    scene: &mut Scene,
    stage: &StageData,
    tags: &[String],
    used: &BTreeSet<usize>,
    pictures: &mut Pictures,
    options: &UsdImportOptions,
) -> Result<Summary> {
    let mut summary = Summary::default();
    let folder = pictures.layer_folder.clone();
    // The first material ends up on top of the shader tree.
    for &m in used.iter().rev() {
        let (Some(material), Some(tag)) = (stage.materials.get(m), tags.get(m)) else {
            continue;
        };
        let order = match options.materials {
            UsdMaterials::PreviewSurface => [Network::Preview, Network::MaterialX],
            _ => [Network::MaterialX, Network::Preview],
        };
        let mut failures = Vec::new();
        let mut done = false;
        // A network Operandi wrote is read as it was written (NodeGraphs,
        // interfaces, spare nodes), else as the resolved network.
        let mut attempts = Vec::new();
        for network in order {
            let terminal = match network {
                Network::MaterialX => &material.mtlx,
                Network::Preview => &material.surface,
            };
            if terminal.is_empty() {
                continue;
            }
            if network == Network::MaterialX && !material.graph.is_empty() {
                match crate::import_mtlx_graph::document(material, pictures) {
                    Ok(doc) => attempts.push((network, doc)),
                    Err(e) => failures.push(format!("its MaterialX graph as written: {e}")),
                }
            }
            attempts.push((network, document(material, terminal, pictures)));
        }
        for (network, doc) in attempts {
            let size = |p: &Path| pictures.size(p);
            match operandi_core::materialx::import::preset(
                &doc,
                Some(&material.name),
                &folder,
                &size,
            ) {
                Ok((preset, notes)) => {
                    operandi_core::preset::add_material_group(scene, &preset.scene, tag)?;
                    match network {
                        Network::MaterialX => summary.materialx += 1,
                        Network::Preview => summary.preview += 1,
                    }
                    for f in &failures {
                        summary
                            .notes
                            .push(format!("{tag}: {f}; used {}", network.label()));
                    }
                    let mut notes = notes;
                    notes.sort();
                    notes.dedup();
                    summary
                        .notes
                        .extend(notes.into_iter().map(|n| format!("{tag}: {n}")));
                    done = true;
                    break;
                }
                Err(e) => failures.push(format!("its {} network: {e}", network.label())),
            }
        }
        if !done {
            summary.plain += 1;
            summary.notes.push(if failures.is_empty() {
                format!("{tag}: no surface shader; left as a plain polygon tag")
            } else {
                format!(
                    "{tag}: {}; left as a plain polygon tag",
                    failures.join("; ")
                )
            });
        }
    }
    Ok(summary)
}

/// A MaterialX node category and output type from a shader's info:id:
/// "ND_image_color3" → ("image", "color3"), "UsdPrimvarReader_float2" →
/// ("UsdPrimvarReader", "vector2"), "UsdPreviewSurface" as it is.
pub fn category(id: &str) -> (String, String) {
    const TYPES: [&str; 22] = [
        "float",
        "integer",
        "boolean",
        "string",
        "filename",
        "color3",
        "color4",
        "vector2",
        "vector3",
        "vector4",
        "matrix33",
        "matrix44",
        "surfaceshader",
        "displacementshader",
        "volumeshader",
        "lightshader",
        "BSDF",
        "EDF",
        "VDF",
        "float2",
        "float3",
        "float4",
    ];
    let is_type = |t: &str| TYPES.contains(&t.strip_suffix("FA").unwrap_or(t));
    if let Some(rest) = id.strip_prefix("ND_") {
        let parts: Vec<&str> = rest.split('_').collect();
        if let Some(i) = (1..parts.len()).find(|&i| is_type(parts[i])) {
            let ty = parts[i].strip_suffix("FA").unwrap_or(parts[i]);
            return (parts[..i].join("_"), mx_type(ty).to_string());
        }
        return (rest.to_string(), String::new());
    }
    if let Some(ty) = id.strip_prefix("UsdPrimvarReader_") {
        return ("UsdPrimvarReader".into(), mx_type(ty).to_string());
    }
    (id.to_string(), String::new())
}

/// MaterialX's name for a USD or MaterialX value type.
pub(crate) fn mx_type(ty: &str) -> &str {
    let ty = ty.strip_suffix("[]").unwrap_or(ty);
    match ty {
        "float" | "double" | "half" => "float",
        "int" | "integer" => "integer",
        "bool" | "boolean" => "boolean",
        "color3f" | "color3d" | "color3h" | "color3" => "color3",
        "color4f" | "color4d" | "color4" => "color4",
        "float2" | "double2" | "texCoord2f" | "vector2" => "vector2",
        "float3" | "double3" | "normal3f" | "vector3f" | "point3f" | "vector3" => "vector3",
        "float4" | "double4" | "vector4" => "vector4",
        "asset" | "filename" => "filename",
        "token" | "string" => "string",
        other => other,
    }
}

/// MaterialX's colour-space name for a USD one (OpenUSD's ColorSpaceAPI
/// names, UsdUVTexture's sourceColorSpace, or MaterialX's own).
pub(crate) fn mx_colour_space(space: &str) -> Option<String> {
    Some(
        match space {
            "" | "auto" => return None,
            "sRGB" | "srgb_rec709_display" | "srgb_texture" | "sRGB - Texture" => "srgb_texture",
            "raw" | "data" | "Raw" | "Non-Color" => "raw",
            "lin_rec709_scene" | "lin_rec709" | "Linear" | "linear" => "lin_rec709",
            "lin_ap1_scene" | "acescg" => "acescg",
            other => other,
        }
        .to_string(),
    )
}

/// The inputs whose default MaterialX already computes (the shading
/// normal and tangent): a network that only rebuilds them from the
/// geometry, as Blender writes, is the same as leaving them unset.
const GEOMETRY_INPUTS: [&str; 4] = [
    "geometry_normal",
    "geometry_tangent",
    "geometry_coat_normal",
    "geometry_coat_tangent",
];

/// Nodes that only derive geometric directions.
const GEOMETRY_NODES: [&str; 9] = [
    "normal",
    "tangent",
    "bitangent",
    "normalize",
    "rotate3d",
    "position",
    "transformvector",
    "transformnormal",
    "crossproduct",
];

/// The MaterialX document of `material`'s network from `terminal`.
fn document(material: &MaterialData, terminal: &str, pictures: &mut Pictures) -> Document {
    let by_name: BTreeMap<&str, &ShaderNode> = material
        .nodes
        .iter()
        .map(|n| (n.name.as_str(), n))
        .collect();
    let mut doc = Document {
        version: "1.39".into(),
        colorspace: Some("lin_rec709".into()),
        elements: Vec::new(),
    };
    for node in &material.nodes {
        let (category, from_id) = category(&node.id);
        let unused = unused_lobes(&category, node);
        // The output's own type (an extract node's is float, whatever its
        // id says it reads).
        let ty = node
            .outputs
            .first()
            .map(|o| mx_type(&o.ty).to_string())
            .unwrap_or(from_id);
        let node_space = mx_colour_space(&node.colorspace);
        // UsdUVTexture names its pictures' colour space in an input.
        let source_space = node
            .inputs
            .iter()
            .find(|i| i.name == "sourceColorSpace")
            .and_then(|i| i.value.as_deref())
            .and_then(mx_colour_space);
        let mut inputs = Vec::new();
        for input in &node.inputs {
            if GEOMETRY_INPUTS.contains(&input.name.as_str())
                && input
                    .node
                    .as_deref()
                    .is_some_and(|n| only_geometry(n, &by_name, 0))
            {
                continue;
            }
            if input.unsupported.is_some()
                || (input.node.is_some()
                    && unused.iter().any(|prefix| input.name.starts_with(prefix)))
            {
                continue;
            }
            let ty = mx_type(&input.ty).to_string();
            let mut port = Port {
                name: input.name.clone(),
                ty: ty.clone(),
                nodename: input.node.clone(),
                output: input.output.clone(),
                ..Port::default()
            };
            if input.node.is_none() {
                port.value = if ty == "filename" {
                    Some(pictures.file(
                        input.value.as_deref().unwrap_or_default(),
                        input.resolved.as_deref().unwrap_or_default(),
                        input.anchored.as_deref().unwrap_or_default(),
                    ))
                } else {
                    input.value.clone()
                };
            }
            port.colorspace = if ty == "filename" && category == "UsdUVTexture" {
                source_space.clone().or(node_space.clone())
            } else {
                input
                    .colorspace
                    .as_deref()
                    .and_then(mx_colour_space)
                    .or(node_space.clone())
            };
            inputs.push(port);
        }
        doc.elements.push(Element {
            category,
            name: node.name.clone(),
            ty,
            inputs,
            outputs: node
                .outputs
                .iter()
                .map(|o| Port {
                    name: o.name.clone(),
                    ty: mx_type(&o.ty).to_string(),
                    ..Port::default()
                })
                .collect(),
            colorspace: node_space,
            ..Element::default()
        });
    }
    doc.elements.push(Element {
        category: "surfacematerial".into(),
        name: material.name.clone(),
        ty: "material".into(),
        inputs: vec![Port {
            name: "surfaceshader".into(),
            ty: "surfaceshader".into(),
            nodename: Some(terminal.to_string()),
            ..Port::default()
        }],
        ..Element::default()
    });
    doc
}

/// The input-name prefixes of a surface shader's lobes that are switched
/// off (their weight is a constant 0, or unset with a 0 default). Their
/// connected inputs change nothing, so they are left out rather than
/// refusing the material (Blender wires the base colour picture into
/// subsurface_color whatever the subsurface weight).
fn unused_lobes(category: &str, node: &ShaderNode) -> Vec<&'static str> {
    let lobes: &[(&str, &str)] = match category {
        "open_pbr_surface" => &[
            ("subsurface_weight", "subsurface_"),
            ("coat_weight", "coat_"),
            ("transmission_weight", "transmission_"),
            ("fuzz_weight", "fuzz_"),
            ("thin_film_weight", "thin_film_"),
            ("emission_luminance", "emission_"),
        ],
        "standard_surface" => &[
            ("subsurface", "subsurface_"),
            ("coat", "coat_"),
            ("transmission", "transmission_"),
            ("sheen", "sheen_"),
            ("thin_film_thickness", "thin_film_"),
            ("emission", "emission_"),
        ],
        _ => return Vec::new(),
    };
    lobes
        .iter()
        .filter(
            |(weight, _)| match node.inputs.iter().find(|i| i.name == *weight) {
                None => true,
                Some(i) => {
                    i.node.is_none()
                        && i.value
                            .as_deref()
                            .and_then(|v| v.trim().parse::<f64>().ok())
                            .is_some_and(|v| v == 0.0)
                }
            },
        )
        .map(|(_, prefix)| *prefix)
        .collect()
}

/// True when node `name` and everything feeding it only derive geometric
/// directions.
fn only_geometry(name: &str, nodes: &BTreeMap<&str, &ShaderNode>, depth: usize) -> bool {
    let Some(node) = nodes.get(name) else {
        return false;
    };
    if depth > 32 || !GEOMETRY_NODES.contains(&category(&node.id).0.as_str()) {
        return false;
    }
    node.inputs.iter().all(|i| match &i.node {
        Some(n) => only_geometry(n, nodes, depth + 1),
        None => true,
    })
}

/// Picture files: where each comes from, unpacking .usdz contents, and
/// which are missing.
pub struct Pictures<'a> {
    assets: &'a dyn Assets,
    /// The USD file's folder.
    layer_folder: PathBuf,
    /// Where packaged pictures are unpacked.
    folder: PathBuf,
    unpacked: BTreeMap<String, PathBuf>,
    missing: BTreeSet<String>,
    failed: Vec<String>,
}

impl<'a> Pictures<'a> {
    pub fn new(file: &Path, options: &UsdImportOptions, assets: &'a dyn Assets) -> Self {
        let folder = options.texture_folder.clone().unwrap_or_else(|| {
            let stem = file
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "scene".into());
            file.with_file_name(format!("{stem}_textures"))
        });
        Self {
            assets,
            layer_folder: file.parent().map(Path::to_path_buf).unwrap_or_default(),
            folder,
            unpacked: BTreeMap::new(),
            missing: BTreeSet::new(),
            failed: Vec::new(),
        }
    }

    /// The picture file an asset input names, as an absolute path. A
    /// picture inside a .usdz is unpacked first; a missing one keeps its
    /// written path, made absolute against its layer, so it shows the
    /// missing-file look and can be found again later.
    pub fn file(&mut self, written: &str, resolved: &str, anchored: &str) -> String {
        let resolved = resolved.to_string();
        let written = written.to_string();
        if resolved.ends_with(']')
            && let Some(open) = resolved.find('[')
        {
            if let Some(done) = self.unpacked.get(&resolved) {
                return done.to_string_lossy().into_owned();
            }
            let inner = &resolved[open + 1..resolved.len() - 1];
            let name = Path::new(inner)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "picture".into());
            let target = self.free_target(&name);
            let copied = std::fs::create_dir_all(&self.folder)
                .map_err(|e| format!("Cannot make {}: {e}", self.folder.display()))
                .and_then(|()| self.assets.copy_packaged(&resolved, &target));
            if let Err(e) = copied {
                self.failed.push(format!("{name}: {e}"));
            }
            self.unpacked.insert(resolved, target.clone());
            return target.to_string_lossy().into_owned();
        }
        if !resolved.is_empty() {
            return resolved;
        }
        let path = if anchored.is_empty() {
            written
        } else {
            anchored.to_string()
        };
        self.missing.insert(
            Path::new(&path)
                .file_name()
                .map_or(path.clone(), |n| n.to_string_lossy().into_owned()),
        );
        path
    }

    /// `name` in the unpack folder, numbered when another picture of
    /// this import already took it.
    fn free_target(&self, name: &str) -> PathBuf {
        let taken: BTreeSet<&PathBuf> = self.unpacked.values().collect();
        let path = Path::new(name);
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let mut target = self.folder.join(name);
        let mut n = 2;
        while taken.contains(&target) {
            target = self.folder.join(format!("{stem}_{n}{ext}"));
            n += 1;
        }
        target
    }

    /// A picture's pixel size. A missing picture still gets a layer (a
    /// one-pixel size until the file is found), so it shows the
    /// missing-file look instead of vanishing.
    pub fn size(&self, path: &Path) -> Option<(u32, u32)> {
        self.assets
            .image_size(path)
            .or_else(|| (!path.is_file()).then_some((1, 1)))
    }

    pub fn notes(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !self.missing.is_empty() {
            let names: Vec<&str> = self.missing.iter().map(String::as_str).collect();
            out.push(format!(
                "Missing picture(s), shown with the missing-file look: {}",
                names.join(", ")
            ));
        }
        if !self.unpacked.is_empty() && self.failed.is_empty() {
            out.push(format!(
                "{} picture(s) unpacked from the .usdz into {}",
                self.unpacked.len(),
                self.folder.display()
            ));
        }
        for f in &self.failed {
            out.push(format!("Couldn't unpack {f}"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_ids_become_materialx_categories() {
        for (id, cat, ty) in [
            ("ND_image_color3", "image", "color3"),
            (
                "ND_open_pbr_surface_surfaceshader",
                "open_pbr_surface",
                "surfaceshader",
            ),
            (
                "ND_standard_surface_surfaceshader",
                "standard_surface",
                "surfaceshader",
            ),
            ("ND_convert_color4_color3", "convert", "color4"),
            ("ND_multiply_color3FA", "multiply", "color3"),
            ("ND_normalmap_float", "normalmap", "float"),
            ("ND_texcoord_vector2", "texcoord", "vector2"),
            ("UsdPrimvarReader_float2", "UsdPrimvarReader", "vector2"),
            ("UsdPreviewSurface", "UsdPreviewSurface", ""),
            ("UsdUVTexture", "UsdUVTexture", ""),
        ] {
            assert_eq!(category(id), (cat.to_string(), ty.to_string()), "{id}");
        }
    }

    #[test]
    fn colour_spaces_take_materialx_names() {
        assert_eq!(mx_colour_space("sRGB").as_deref(), Some("srgb_texture"));
        assert_eq!(
            mx_colour_space("srgb_rec709_display").as_deref(),
            Some("srgb_texture")
        );
        assert_eq!(mx_colour_space("raw").as_deref(), Some("raw"));
        assert_eq!(mx_colour_space("data").as_deref(), Some("raw"));
        assert_eq!(
            mx_colour_space("lin_rec709_scene").as_deref(),
            Some("lin_rec709")
        );
        assert_eq!(mx_colour_space("auto"), None);
    }
}
