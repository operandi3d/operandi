//! A USD stage as the bridge's reader describes it (`ou_read_open`,
//! bridge/operandi_usd_read.cpp): plain data, so the import can be built
//! and tested without OpenUSD. The JSON names large arrays by buffer;
//! [`StageData::buffers`] holds their bytes.
use serde::Deserialize;

/// A large array in [`StageData::buffers`].
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
pub struct BufRef {
    pub buffer: usize,
    /// "i32", "f32", "f32x3" or "f64".
    #[serde(rename = "type")]
    pub ty: Elem,
    /// Elements ("f32x3": triples).
    pub count: usize,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub enum Elem {
    #[default]
    #[serde(rename = "i32")]
    I32,
    #[serde(rename = "f32")]
    F32,
    #[serde(rename = "f32x3")]
    F32x3,
    #[serde(rename = "f64")]
    F64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageData {
    /// "Y" or "Z".
    pub up_axis: String,
    pub meters_per_unit: f64,
    /// The default prim's path, or "".
    #[serde(default)]
    pub default_prim: String,
    /// The camera prim the stage's RenderSettings render through, or ""
    /// (bridge reader's `render_camera`).
    #[serde(default)]
    pub render_camera: String,
    /// The RenderSettings prim the stage renders with (usd-gaps-3).
    #[serde(default)]
    pub render_settings: Option<crate::render_usd::RenderSettingsData>,
    /// Depth first, parents before children.
    pub prims: Vec<Prim>,
    pub meshes: Vec<MeshData>,
    pub materials: Vec<MaterialData>,
    /// Prims that are neither geometry nor transforms.
    #[serde(default)]
    pub skipped: Vec<Skipped>,
    /// UsdSkelSkeletons (import_skel.rs).
    #[serde(default)]
    pub skeletons: Vec<SkelData>,
    #[serde(skip)]
    pub buffers: Vec<Vec<u8>>,
}

/// A UsdSkelSkeleton: joint paths, bind (skeleton-space) and rest (local)
/// transforms, row-major in USD's row-vector convention, its world, and
/// its SkelAnimation.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkelData {
    pub path: String,
    pub joints: Vec<String>,
    pub bind: Vec<[f64; 16]>,
    pub rest: Vec<[f64; 16]>,
    pub world: [f64; 16],
    #[serde(default)]
    pub anim: Option<SkelAnimData>,
}

/// A SkelAnimation's samples: per time, per joint in `joints` order.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkelAnimData {
    pub joints: Vec<String>,
    pub times: Vec<f64>,
    pub translations: Vec<Vec<[f64; 3]>>,
    /// w, x, y, z.
    pub rotations: Vec<Vec<[f64; 4]>>,
    pub scales: Vec<Vec<[f64; 3]>>,
}

/// A mesh's UsdSkelBindingAPI skinning.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinData {
    pub skeleton: String,
    pub element_size: usize,
    pub indices: BufRef,
    pub weights: BufRef,
    pub geom_bind: [f64; 16],
    pub method: String,
    /// The mesh's own skel:joints order, if it has one.
    #[serde(default)]
    pub joints: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prim {
    pub path: String,
    pub name: String,
    /// The schema type name: "Xform", "Mesh", "Scope", "Camera",
    /// "SphereLight", "PointInstancer", "" (typeless)...
    #[serde(rename = "type")]
    pub ty: String,
    /// Index into `prims`, -1 at the top (and for prototypes).
    pub parent: i64,
    #[serde(default)]
    pub purpose: String,
    #[serde(default = "yes")]
    pub visible: bool,
    /// A PointInstancer's prototype subtree: the instancer's index.
    pub proto_of: Option<usize>,
    /// A native instance (instanceable, expanded here).
    #[serde(default)]
    pub instance: bool,
    /// What Operandi wrote in the prim's customData ("operandi").
    pub custom: Option<serde_json::Value>,
    /// Its variant sets and their selections, as composed.
    #[serde(default)]
    pub variants: std::collections::BTreeMap<String, String>,
    /// It has payloads (loaded with the stage).
    #[serde(default)]
    pub payload: bool,
    /// The local transform, row-major in USD's row-vector convention
    /// (translation in the last row). Absent for prims that aren't
    /// transformable.
    pub local: Option<[f64; 16]>,
    #[serde(default)]
    pub resets_xform: bool,
    /// The same as plain translate / rotate / scale, when it is one.
    pub trs: Option<Trs>,
    /// Index into `meshes`.
    pub mesh: Option<usize>,
    /// The material bound to the whole mesh (index into `materials`), -1
    /// for none.
    pub material: Option<i64>,
    #[serde(default)]
    pub subsets: Vec<Subset>,
    pub instancer: Option<Instancer>,
    /// A UsdGeomCamera's attributes.
    pub camera: Option<CameraData>,
    /// A UsdLux light's attributes (the prim type says which light).
    pub light: Option<LightData>,
}

/// UsdGeomCamera attributes, as authored or the schema's fallbacks. Lens
/// and film values are in tenths of a scene unit.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraData {
    pub projection: Option<String>,
    pub focal_length: Option<f64>,
    pub horizontal_aperture: Option<f64>,
    pub vertical_aperture: Option<f64>,
    pub horizontal_aperture_offset: Option<f64>,
    pub vertical_aperture_offset: Option<f64>,
    pub clipping_range: Option<[f64; 2]>,
    pub f_stop: Option<f64>,
    pub focus_distance: Option<f64>,
}

/// UsdLux light and ShapingAPI attributes.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct LightData {
    #[serde(rename = "inputs:intensity")]
    pub intensity: Option<f64>,
    #[serde(rename = "inputs:exposure")]
    pub exposure: Option<f64>,
    #[serde(rename = "inputs:color")]
    pub color: Option<[f64; 3]>,
    #[serde(rename = "inputs:enableColorTemperature")]
    pub enable_color_temperature: Option<bool>,
    #[serde(rename = "inputs:colorTemperature")]
    pub color_temperature: Option<f64>,
    #[serde(rename = "inputs:normalize")]
    pub normalize: Option<bool>,
    #[serde(rename = "inputs:diffuse")]
    pub diffuse: Option<f64>,
    #[serde(rename = "inputs:specular")]
    pub specular: Option<f64>,
    #[serde(rename = "inputs:radius")]
    pub radius: Option<f64>,
    #[serde(rename = "inputs:width")]
    pub width: Option<f64>,
    #[serde(rename = "inputs:height")]
    pub height: Option<f64>,
    #[serde(rename = "inputs:length")]
    pub length: Option<f64>,
    /// DistantLight: angular size, degrees.
    #[serde(rename = "inputs:angle")]
    pub angle: Option<f64>,
    /// ShapingAPI: cone half-angle (degrees) and softness.
    #[serde(rename = "inputs:shaping:cone:angle")]
    pub cone_angle: Option<f64>,
    #[serde(rename = "inputs:shaping:cone:softness")]
    pub cone_softness: Option<f64>,
    #[serde(rename = "inputs:shaping:focus")]
    pub focus: Option<f64>,
    #[serde(rename = "inputs:texture:format")]
    pub texture_format: Option<String>,
    #[serde(rename = "treatAsPoint")]
    pub treat_as_point: Option<bool>,
    /// DomeLight: its picture.
    pub texture: Option<Asset>,
    /// ShapingAPI: the IES profile.
    pub ies: Option<Asset>,
}

/// An asset path: as written, resolved by OpenUSD ("" when it couldn't),
/// and the written path made absolute against its layer.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Asset {
    pub value: Option<String>,
    pub resolved: Option<String>,
    pub anchored: Option<String>,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Trs {
    pub t: [f64; 3],
    /// Degrees.
    pub r: [f64; 3],
    pub s: [f64; 3],
    /// "XYZ"... the axis that turns first first, like USD's rotate ops.
    pub order: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Subset {
    pub name: String,
    pub faces: BufRef,
    pub material: i64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instancer {
    /// Prim indices of the prototype roots, in protoIndices order.
    pub prototypes: Vec<usize>,
    pub proto_indices: BufRef,
    /// One row-major row-vector matrix (16 f64) per instance.
    pub matrices: BufRef,
    /// Instances its invisibleIds hide, by index.
    pub hidden: Option<BufRef>,
    /// Instances its inactiveIds remove, by index.
    pub inactive: Option<BufRef>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshData {
    pub points: BufRef,
    pub counts: BufRef,
    pub indices: BufRef,
    /// "rightHanded" or "leftHanded".
    pub orientation: String,
    /// "catmullClark", "loop", "bilinear" or "none".
    pub scheme: String,
    pub boundary: String,
    pub fvli: String,
    pub normals: Option<Primvar>,
    #[serde(default)]
    pub primvars: Vec<Primvar>,
    pub crease_indices: Option<BufRef>,
    pub crease_lengths: Option<BufRef>,
    pub crease_sharpnesses: Option<BufRef>,
    pub corner_indices: Option<BufRef>,
    pub corner_sharpnesses: Option<BufRef>,
    pub holes: Option<BufRef>,
    /// Operandi's parts and selection sets (subsets of its families).
    #[serde(default)]
    pub groups: Vec<Group>,
    /// Blend shapes bound to the mesh.
    #[serde(default)]
    pub blend_shapes: Vec<BlendShape>,
    /// Its skinning (UsdSkelBindingAPI).
    #[serde(default)]
    pub skin: Option<SkinData>,
    /// What Operandi wrote in the mesh prim's customData ("operandi").
    pub custom: Option<serde_json::Value>,
    /// Set when the prim is a BasisCurves or NurbsCurves prim instead of a
    /// Mesh (no polygons then; curves.rs).
    pub curves: Option<CurvesData>,
}

/// A BasisCurves or NurbsCurves prim's attributes.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurvesData {
    /// "BasisCurves", "NurbsCurves" or "HermiteCurves".
    pub schema: String,
    pub vertex_counts: BufRef,
    pub widths: Option<BufRef>,
    #[serde(default)]
    pub widths_interpolation: String,
    /// BasisCurves' type, basis and wrap.
    #[serde(default, rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub basis: String,
    #[serde(default)]
    pub wrap: String,
    /// NurbsCurves' order per curve, knots, point weights (f64).
    pub order: Option<BufRef>,
    pub knots: Option<BufRef>,
    pub point_weights: Option<BufRef>,
    pub ranges: Option<BufRef>,
    /// What Operandi wrote in its customData ("operandi"): `kinds`.
    pub custom: Option<serde_json::Value>,
}

/// A GeomSubset that isn't a material binding: one of Operandi's families
/// (plan_extras.rs) or another program's (Houdini groups).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub name: String,
    pub family: String,
    /// "face", "point" or "edge".
    pub element_type: String,
    pub indices: BufRef,
    pub custom: Option<serde_json::Value>,
}

/// A UsdSkelBlendShape: offsets for all points, or for `point_indices`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendShape {
    pub name: String,
    pub offsets: BufRef,
    pub point_indices: Option<BufRef>,
    pub custom: Option<serde_json::Value>,
}

/// A primvar (or the normals): flat floats, `dims` per element.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Primvar {
    #[serde(default)]
    pub name: String,
    /// "uv", "color", "opacity", "operandi" (a map Operandi wrote) or
    /// "foreign" (another program's map, import_extras.rs).
    #[serde(default)]
    pub kind: String,
    #[serde(default = "three")]
    pub dims: usize,
    /// The USD scalar type name: "float", "int3", "normal3f"...
    #[serde(default, rename = "type")]
    pub ty: String,
    /// The type's role: "Normal", "Point", "Vector", "Color",
    /// "TextureCoordinate" or "".
    #[serde(default)]
    pub role: String,
    /// "constant", "uniform", "vertex", "varying" or "faceVarying".
    pub interpolation: String,
    pub values: BufRef,
    pub indices: Option<BufRef>,
    /// The `unauthoredValuesIndex` of a map Operandi wrote: elements
    /// indexing it have no value.
    pub unauthored: Option<i64>,
    /// What Operandi wrote in the primvar's customData ("operandi"); such
    /// primvars have kind "operandi".
    pub custom: Option<serde_json::Value>,
}

fn three() -> usize {
    3
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct MaterialData {
    pub path: String,
    pub name: String,
    /// The node `outputs:surface` reaches (UsdPreviewSurface), or "".
    pub surface: String,
    /// The node `outputs:mtlx:surface` reaches, or "".
    pub mtlx: String,
    pub nodes: Vec<ShaderNode>,
    /// The MaterialX network as written, for materials Operandi wrote
    /// (customData operandi:materialxGraph): every Shader and NodeGraph
    /// below the material with direct connections. Empty otherwise.
    #[serde(default)]
    pub graph: Vec<GraphPrim>,
    /// The `graph` prim `outputs:mtlx:surface` connects to.
    #[serde(default)]
    pub graph_terminal: String,
}

/// A Shader or NodeGraph of a material's network, by its path below the
/// material ("NG/img").
#[derive(Clone, Debug, Default, Deserialize)]
pub struct GraphPrim {
    pub path: String,
    /// A NodeGraph (else a Shader).
    pub graph: bool,
    /// A shader's info:id.
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub colorspace: String,
    /// UsdUI's ui:nodegraph:node:pos.
    #[serde(default)]
    pub pos: Option<[f64; 2]>,
    pub inputs: Vec<ShaderInput>,
    pub outputs: Vec<ShaderOutput>,
}

/// An input's or output's direct connection.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Source {
    /// The source prim, below the material.
    pub path: String,
    /// "output" or "input" (a NodeGraph's interface).
    pub kind: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ShaderNode {
    pub name: String,
    /// info:id: "UsdPreviewSurface", "ND_image_color3"...
    pub id: String,
    /// The prim's ColorSpaceAPI space, or "".
    #[serde(default)]
    pub colorspace: String,
    pub inputs: Vec<ShaderInput>,
    pub outputs: Vec<ShaderOutput>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ShaderInput {
    pub name: String,
    /// The USD value type: "float", "color3f", "asset", "token"...
    #[serde(rename = "type")]
    pub ty: String,
    /// The value as text ("0.5", "1, 0, 0"); an asset's path as written.
    pub value: Option<String>,
    /// An asset's resolved path ("" when OpenUSD couldn't find it).
    pub resolved: Option<String>,
    /// An asset's written path made absolute against its layer.
    pub anchored: Option<String>,
    /// Connected to this node's `output`.
    pub node: Option<String>,
    pub output: Option<String>,
    pub colorspace: Option<String>,
    /// A value type the reader doesn't turn into text.
    pub unsupported: Option<String>,
    /// Its direct connection (`MaterialData::graph` only).
    #[serde(default)]
    pub source: Option<Source>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ShaderOutput {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    /// A NodeGraph output's connection (`MaterialData::graph` only).
    #[serde(default)]
    pub source: Option<Source>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Skipped {
    pub path: String,
    #[serde(rename = "type")]
    pub ty: String,
}

impl StageData {
    /// Parses the reader's JSON; `buffers` are its buffers in order.
    pub fn parse(json: &str, buffers: Vec<Vec<u8>>) -> Result<Self, String> {
        let mut data: StageData =
            serde_json::from_str(json).map_err(|e| format!("Unreadable USD description: {e}"))?;
        data.buffers = buffers;
        Ok(data)
    }

    fn bytes(&self, r: &BufRef, elem: Elem, width: usize) -> Result<&[u8], String> {
        let bytes = self
            .buffers
            .get(r.buffer)
            .ok_or_else(|| format!("USD buffer {} is missing", r.buffer))?;
        if r.ty != elem || bytes.len() != r.count * width {
            return Err(format!("USD buffer {} isn't what it claims", r.buffer));
        }
        Ok(bytes)
    }

    pub fn i32s(&self, r: &BufRef) -> Result<Vec<i32>, String> {
        Ok(self
            .bytes(r, Elem::I32, 4)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| i32::from_ne_bytes(*c))
            .collect())
    }

    pub fn f32s(&self, r: &BufRef) -> Result<Vec<f32>, String> {
        let width = if r.ty == Elem::F32x3 { 12 } else { 4 };
        Ok(self
            .bytes(r, r.ty, width)
            .and_then(|b| {
                if matches!(r.ty, Elem::F32 | Elem::F32x3) {
                    Ok(b)
                } else {
                    Err(format!("USD buffer {} isn't floats", r.buffer))
                }
            })?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_ne_bytes(*c))
            .collect())
    }

    pub fn f64s(&self, r: &BufRef) -> Result<Vec<f64>, String> {
        Ok(self
            .bytes(r, Elem::F64, 8)?
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| f64::from_ne_bytes(*c))
            .collect())
    }

    /// Adds a buffer (tests build stages by hand).
    pub fn push_i32(&mut self, values: &[i32]) -> BufRef {
        self.push(
            values.iter().flat_map(|v| v.to_ne_bytes()).collect(),
            Elem::I32,
            values.len(),
        )
    }
    pub fn push_f32(&mut self, values: &[f32]) -> BufRef {
        self.push(
            values.iter().flat_map(|v| v.to_ne_bytes()).collect(),
            Elem::F32,
            values.len(),
        )
    }
    pub fn push_points(&mut self, points: &[[f32; 3]]) -> BufRef {
        let bytes = points
            .iter()
            .flatten()
            .flat_map(|v| v.to_ne_bytes())
            .collect();
        self.push(bytes, Elem::F32x3, points.len())
    }
    pub fn push_f64(&mut self, values: &[f64]) -> BufRef {
        self.push(
            values.iter().flat_map(|v| v.to_ne_bytes()).collect(),
            Elem::F64,
            values.len(),
        )
    }
    fn push(&mut self, bytes: Vec<u8>, ty: Elem, count: usize) -> BufRef {
        self.buffers.push(bytes);
        BufRef {
            buffer: self.buffers.len() - 1,
            ty,
            count,
        }
    }
}
