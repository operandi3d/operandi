//! The bridge's C interface (bridge/operandi_usd.h) and the writer that
//! walks a [`Plan`] through it.
//!
//! Safety: every call passes pointers into Rust-owned vectors and strings
//! that outlive the call (the header says all arrays are borrowed for the
//! call only), plus fixed-size buffers for the path and error text. The
//! struct layouts are `#[repr(C)]` copies of the header's, checked against
//! the library's own sizes before the first call.
use crate::lights::{CameraPrim, LightPrim};
use crate::materials::{MaterialPrim, TextureUse};
use crate::plan::{MeshPrim, Plan, XformPrim};
use operandi_core::Result;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char};
use std::path::Path;
use std::sync::OnceLock;

/// Must equal OU_ABI_VERSION in bridge/operandi_usd.h. Bump both whenever a
/// shared struct, enum value or function changes.
pub const OU_ABI_VERSION: u32 = 9;

#[repr(C)]
pub struct OuXform {
    pub name: *const c_char,
    pub translate: [f64; 3],
    pub rotate: [f64; 3],
    pub rotate_order: u32,
    pub scale: [f64; 3],
    pub use_matrix: u32,
    pub matrix: [f64; 16],
    pub visible: u32,
}

#[repr(C)]
pub struct OuUvSet {
    pub name: *const c_char,
    pub corner_uvs: *const f32,
    pub corner_count: usize,
}

#[repr(C)]
pub struct OuSubset {
    pub name: *const c_char,
    pub faces: *const i32,
    pub face_count: usize,
    pub material: *const c_char,
}

#[repr(C)]
pub struct OuMesh {
    pub name: *const c_char,
    pub points: *const f32,
    pub point_count: usize,
    pub face_counts: *const i32,
    pub face_count: usize,
    pub face_indices: *const i32,
    pub index_count: usize,
    pub normals: *const f32,
    pub subdivision: u32,
    pub boundary: u32,
    pub uv_interpolation: u32,
    pub crease_points: *const i32,
    pub crease_sharpness: *const f32,
    pub crease_count: usize,
    pub uv_sets: *const OuUvSet,
    pub uv_set_count: usize,
    pub subsets: *const OuSubset,
    pub subset_count: usize,
    pub double_sided: u32,
    pub visible: u32,
    pub material: *const c_char,
}

#[repr(C)]
pub struct OuTexture {
    pub file: *const c_char,
    pub uv: *const c_char,
    pub channel: u32,
    pub scale: [f32; 4],
    pub bias: [f32; 4],
    pub wrap: u32,
    pub raw: u32,
    pub has_transform: u32,
    pub uv_scale: [f32; 2],
    pub uv_rotation: f32,
}

#[repr(C)]
pub struct OuSurfaceInput {
    pub name: *const c_char,
    pub kind: u32,
    pub value: [f32; 3],
    pub texture: *const OuTexture,
}

#[repr(C)]
pub struct OuMaterial {
    pub name: *const c_char,
    pub inputs: *const OuSurfaceInput,
    pub input_count: usize,
}

#[repr(C)]
pub struct OuCamera {
    pub name: *const c_char,
    pub orthographic: u32,
    pub focal_length: f32,
    pub horizontal_aperture: f32,
    pub vertical_aperture: f32,
    pub horizontal_offset: f32,
    pub vertical_offset: f32,
    pub clipping: [f32; 2],
    pub f_stop: f32,
    pub focus_distance: f32,
}

#[repr(C)]
pub struct OuLight {
    pub name: *const c_char,
    pub kind: u32,
    pub intensity: f32,
    pub color: [f32; 3],
    pub diffuse: f32,
    pub specular: f32,
    pub normalize: u32,
    pub radius: f32,
    pub width: f32,
    pub height: f32,
    pub length: f32,
    pub angle: f32,
    pub cone_angle: f32,
    pub cone_softness: f32,
    pub ies_file: *const c_char,
    pub texture_file: *const c_char,
}

#[repr(C)]
pub struct OuStage {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OuReader {
    _private: [u8; 0],
}

/// Reads the USD file at `path` into a [`StageData`] (the bridge walks the
/// stage once; its description and buffers are copied out, then freed).
pub fn read(path: &Path) -> Result<crate::stage::StageData> {
    ready()?;
    let c_path = c_string(&path.to_string_lossy());
    let mut error = [0 as c_char; TEXT_LEN];
    // SAFETY: valid C string and error buffer for the call.
    let reader = unsafe { ou_read_open(c_path.as_ptr(), error.as_mut_ptr(), TEXT_LEN) };
    if reader.is_null() {
        return Err(text(&error));
    }
    // SAFETY: `reader` is a live reader until ou_read_free below; the JSON
    // is NUL-terminated and owned by it, and each buffer pointer/length
    // pair it returns stays valid until then. Everything is copied before
    // the free.
    unsafe {
        let json = CStr::from_ptr(ou_read_json(reader))
            .to_string_lossy()
            .into_owned();
        let mut buffers = Vec::new();
        loop {
            let mut data: *const u8 = std::ptr::null();
            let len = ou_read_buffer(reader, buffers.len(), &mut data);
            if data.is_null() {
                break;
            }
            buffers.push(if len == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(data, len).to_vec()
            });
        }
        ou_read_free(reader);
        crate::stage::StageData::parse(&json, buffers)
    }
}

/// Writes the asset OpenUSD resolved to `resolved` (a plain file, or one
/// inside a .usdz) to `destination`.
pub fn copy_asset(resolved: &str, destination: &Path) -> Result<()> {
    ready()?;
    let from = c_string(resolved);
    let to = c_string(&destination.to_string_lossy());
    let mut error = [0 as c_char; TEXT_LEN];
    // SAFETY: valid C strings and error buffer for the call.
    if unsafe { ou_copy_asset(from.as_ptr(), to.as_ptr(), error.as_mut_ptr(), TEXT_LEN) } == 0 {
        return Err(text(&error));
    }
    Ok(())
}

const STRUCT_XFORM: u32 = 0;
const STRUCT_UV_SET: u32 = 1;
const STRUCT_SUBSET: u32 = 2;
const STRUCT_MESH: u32 = 3;
const STRUCT_TEXTURE: u32 = 4;
const STRUCT_SURFACE_INPUT: u32 = 5;
const STRUCT_MATERIAL: u32 = 6;
const STRUCT_CAMERA: u32 = 7;
const STRUCT_LIGHT: u32 = 8;

unsafe extern "C" {
    fn ou_abi_version() -> u32;
    fn ou_struct_size(which: u32) -> usize;
    fn ou_usd_version() -> *const c_char;
    fn ou_init(plugin_dir: *const c_char, error: *mut c_char, error_len: usize) -> i32;
    fn ou_stage_create(
        path: *const c_char,
        text: u32,
        error: *mut c_char,
        error_len: usize,
    ) -> *mut OuStage;
    fn ou_stage_open(path: *const c_char, error: *mut c_char, error_len: usize) -> *mut OuStage;
    fn ou_stage_free(stage: *mut OuStage);
    fn ou_stage_set_metadata(
        stage: *mut OuStage,
        up_axis: c_char,
        meters_per_unit: f64,
        default_prim: *const c_char,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_define_xform(
        stage: *mut OuStage,
        parent: *const c_char,
        xform: *const OuXform,
        path_out: *mut c_char,
        path_len: usize,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_define_mesh(
        stage: *mut OuStage,
        parent: *const c_char,
        mesh: *const OuMesh,
        path_out: *mut c_char,
        path_len: usize,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_define_material(
        stage: *mut OuStage,
        parent: *const c_char,
        material: *const OuMaterial,
        path_out: *mut c_char,
        path_len: usize,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_define_camera(
        stage: *mut OuStage,
        parent: *const c_char,
        camera: *const OuCamera,
        path_out: *mut c_char,
        path_len: usize,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_define_light(
        stage: *mut OuStage,
        parent: *const c_char,
        light: *const OuLight,
        path_out: *mut c_char,
        path_len: usize,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_define_network(
        stage: *mut OuStage,
        material: *const c_char,
        json: *const c_char,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_define_render_settings(
        stage: *mut OuStage,
        json: *const c_char,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_define_mesh_extras(
        stage: *mut OuStage,
        mesh: *const c_char,
        json: *const c_char,
        buffers: *const *const u8,
        sizes: *const usize,
        buffer_count: usize,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_define_curves(
        stage: *mut OuStage,
        parent: *const c_char,
        json: *const c_char,
        buffers: *const *const u8,
        sizes: *const usize,
        buffer_count: usize,
        path_out: *mut c_char,
        path_len: usize,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_define_point_instancer(
        stage: *mut OuStage,
        parent: *const c_char,
        json: *const c_char,
        buffers: *const *const u8,
        sizes: *const usize,
        buffer_count: usize,
        path_out: *mut c_char,
        path_len: usize,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_make_class(
        stage: *mut OuStage,
        path: *const c_char,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_make_instance(
        stage: *mut OuStage,
        path: *const c_char,
        prototype: *const c_char,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_stage_save(stage: *mut OuStage, error: *mut c_char, error_len: usize) -> i32;
    fn ou_package_usdz(
        source: *const c_char,
        usdz_path: *const c_char,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn ou_stage_text(stage: *mut OuStage) -> *mut c_char;
    fn ou_string_free(text: *mut c_char);
    fn ou_world_matrix(stage: *mut OuStage, path: *const c_char, out: *mut f64) -> i32;
    fn ou_read_open(path: *const c_char, error: *mut c_char, error_len: usize) -> *mut OuReader;
    fn ou_read_free(reader: *mut OuReader);
    fn ou_read_json(reader: *const OuReader) -> *const c_char;
    fn ou_read_buffer(reader: *const OuReader, index: usize, data: *mut *const u8) -> usize;
    fn ou_copy_asset(
        resolved: *const c_char,
        destination: *const c_char,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
}

/// Error and path buffers.
const TEXT_LEN: usize = 2048;

fn text(buffer: &[c_char]) -> String {
    // SAFETY: the bridge always NUL-terminates what it writes, and the
    // buffer starts zeroed, so there is a NUL inside it.
    unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

fn c_string(s: &str) -> CString {
    // Interior NULs can't cross the C boundary; names never need them.
    CString::new(s.replace('\0', "_")).expect("NULs replaced")
}

/// Checks the loaded library matches this build and loads OpenUSD's
/// plugins, once.
pub fn ready() -> Result<()> {
    static READY: OnceLock<std::result::Result<(), String>> = OnceLock::new();
    READY
        .get_or_init(|| {
            // SAFETY: plain value queries with no arguments to get wrong.
            let abi = unsafe { ou_abi_version() };
            if abi != OU_ABI_VERSION {
                return Err(format!(
                    "The USD library is version {abi}, this build needs {OU_ABI_VERSION}: \
                     rerun scripts/build-usd.sh"
                ));
            }
            let sizes = [
                (STRUCT_XFORM, size_of::<OuXform>(), "OuXform"),
                (STRUCT_UV_SET, size_of::<OuUvSet>(), "OuUvSet"),
                (STRUCT_SUBSET, size_of::<OuSubset>(), "OuSubset"),
                (STRUCT_MESH, size_of::<OuMesh>(), "OuMesh"),
                (STRUCT_TEXTURE, size_of::<OuTexture>(), "OuTexture"),
                (
                    STRUCT_SURFACE_INPUT,
                    size_of::<OuSurfaceInput>(),
                    "OuSurfaceInput",
                ),
                (STRUCT_MATERIAL, size_of::<OuMaterial>(), "OuMaterial"),
                (STRUCT_CAMERA, size_of::<OuCamera>(), "OuCamera"),
                (STRUCT_LIGHT, size_of::<OuLight>(), "OuLight"),
            ];
            for (which, ours, name) in sizes {
                // SAFETY: as above.
                let theirs = unsafe { ou_struct_size(which) };
                if theirs != ours {
                    return Err(format!(
                        "The USD library's {name} is {theirs} bytes, Rust's is {ours}: \
                         OU_ABI_VERSION needs bumping"
                    ));
                }
            }
            let mut error = [0 as c_char; TEXT_LEN];
            let folder = plugin_folder().map(|f| c_string(&f.to_string_lossy()));
            // SAFETY: a null plugin folder means "next to the library"; the
            // folder string and the TEXT_LEN error buffer outlive the call.
            let dir = folder.as_ref().map_or(std::ptr::null(), |f| f.as_ptr());
            if unsafe { ou_init(dir, error.as_mut_ptr(), TEXT_LEN) } == 0 {
                return Err(text(&error));
            }
            Ok(())
        })
        .clone()
}

/// OpenUSD's plugin descriptions in an app bundle: Contents/Resources/usd
/// (code signing keeps Frameworks for code only). Elsewhere (tests, a
/// development build) OpenUSD finds them next to its own library.
fn plugin_folder() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let folder = exe.parent()?.parent()?.join("Resources/usd");
    folder.join("plugInfo.json").is_file().then_some(folder)
}

/// "0.26.8".
pub fn usd_version() -> String {
    // SAFETY: returns a static NUL-terminated string.
    unsafe { CStr::from_ptr(ou_usd_version()) }
        .to_string_lossy()
        .into_owned()
}

/// An open stage, freed on drop.
pub struct Stage(*mut OuStage);
impl Drop for Stage {
    fn drop(&mut self) {
        // SAFETY: the pointer came from ou_stage_create/open and is freed once.
        unsafe { ou_stage_free(self.0) }
    }
}
impl Stage {
    pub fn create(path: &Path, text_format: bool) -> Result<Self> {
        ready()?;
        let c_path = c_string(&path.to_string_lossy());
        let mut error = [0 as c_char; TEXT_LEN];
        // SAFETY: valid C string and error buffer.
        let stage = unsafe {
            ou_stage_create(
                c_path.as_ptr(),
                text_format as u32,
                error.as_mut_ptr(),
                TEXT_LEN,
            )
        };
        if stage.is_null() {
            return Err(text(&error));
        }
        Ok(Self(stage))
    }

    pub fn open(path: &Path) -> Result<Self> {
        ready()?;
        let c_path = c_string(&path.to_string_lossy());
        let mut error = [0 as c_char; TEXT_LEN];
        // SAFETY: valid C string and error buffer.
        let stage = unsafe { ou_stage_open(c_path.as_ptr(), error.as_mut_ptr(), TEXT_LEN) };
        if stage.is_null() {
            return Err(text(&error));
        }
        Ok(Self(stage))
    }

    fn call(&self, f: impl FnOnce(*mut c_char) -> i32) -> Result<()> {
        let mut error = [0 as c_char; TEXT_LEN];
        if f(error.as_mut_ptr()) == 0 {
            return Err(text(&error));
        }
        Ok(())
    }

    pub fn set_metadata(
        &self,
        up_axis: char,
        meters_per_unit: f64,
        default_prim: &str,
    ) -> Result<()> {
        let prim = c_string(default_prim);
        // SAFETY: live stage, valid C string, TEXT_LEN error buffer.
        self.call(|error| unsafe {
            ou_stage_set_metadata(
                self.0,
                up_axis as u8 as c_char,
                meters_per_unit,
                prim.as_ptr(),
                error,
                TEXT_LEN,
            )
        })
    }

    pub fn define_xform(
        &self,
        parent: &str,
        xform: &XformPrim,
        matrix: Option<[f64; 16]>,
    ) -> Result<String> {
        let name = c_string(&xform.name);
        let parent = c_string(parent);
        let desc = OuXform {
            name: name.as_ptr(),
            translate: xform.translate,
            rotate: xform.rotate,
            rotate_order: xform.order as u32,
            scale: xform.scale,
            use_matrix: matrix.is_some() as u32,
            matrix: matrix.unwrap_or([0.0; 16]),
            visible: xform.visible as u32,
        };
        let mut path = [0 as c_char; TEXT_LEN];
        // SAFETY: `desc` and the strings it points to live until the call
        // returns; buffers are TEXT_LEN long.
        self.call(|error| unsafe {
            ou_define_xform(
                self.0,
                parent.as_ptr(),
                &desc,
                path.as_mut_ptr(),
                TEXT_LEN,
                error,
                TEXT_LEN,
            )
        })?;
        Ok(text(&path))
    }

    pub fn define_mesh(
        &self,
        parent: &str,
        mesh: &MeshPrim,
        materials: &[String],
    ) -> Result<String> {
        let name = c_string(&mesh.name);
        let material_path =
            |m: Option<usize>| m.and_then(|m| materials.get(m)).map(|p| c_string(p));
        let mesh_material = material_path(mesh.material);
        let subset_materials: Vec<Option<CString>> = mesh
            .subsets
            .iter()
            .map(|s| material_path(s.material))
            .collect();
        let parent = c_string(parent);
        let uv_names: Vec<CString> = mesh.uv_sets.iter().map(|(n, _)| c_string(n)).collect();
        let uv_sets: Vec<OuUvSet> = mesh
            .uv_sets
            .iter()
            .zip(&uv_names)
            .map(|((_, values), name)| OuUvSet {
                name: name.as_ptr(),
                corner_uvs: values.as_ptr().cast(),
                corner_count: values.len(),
            })
            .collect();
        let subset_names: Vec<CString> = mesh.subsets.iter().map(|s| c_string(&s.name)).collect();
        let subsets: Vec<OuSubset> = mesh
            .subsets
            .iter()
            .zip(&subset_names)
            .zip(&subset_materials)
            .map(|((subset, name), material)| OuSubset {
                name: name.as_ptr(),
                faces: subset.faces.as_ptr(),
                face_count: subset.faces.len(),
                material: material.as_ref().map_or(std::ptr::null(), |m| m.as_ptr()),
            })
            .collect();
        let desc = OuMesh {
            name: name.as_ptr(),
            points: mesh.points.as_ptr().cast(),
            point_count: mesh.points.len(),
            face_counts: mesh.face_counts.as_ptr(),
            face_count: mesh.face_counts.len(),
            face_indices: mesh.face_indices.as_ptr(),
            index_count: mesh.face_indices.len(),
            normals: mesh
                .normals
                .as_ref()
                .filter(|n| n.len() == mesh.face_indices.len())
                .map_or(std::ptr::null(), |n| n.as_ptr().cast()),
            subdivision: mesh.subdivision as u32,
            boundary: mesh.boundary.map_or(0, |b| b as u32),
            uv_interpolation: mesh.uv_interpolation.map_or(0, |u| u as u32),
            crease_points: mesh.creases.as_ptr().cast(),
            crease_sharpness: mesh.crease_sharpness.as_ptr(),
            crease_count: mesh.creases.len().min(mesh.crease_sharpness.len()),
            uv_sets: uv_sets.as_ptr(),
            uv_set_count: uv_sets.len(),
            subsets: subsets.as_ptr(),
            subset_count: subsets.len(),
            double_sided: 1,
            visible: mesh.visible as u32,
            material: mesh_material
                .as_ref()
                .map_or(std::ptr::null(), |m| m.as_ptr()),
        };
        let mut path = [0 as c_char; TEXT_LEN];
        // SAFETY: every pointer in `desc` refers to a vector or string above
        // that lives until the call returns; [f32; 3] / [f32; 2] / [i32; 2]
        // arrays are laid out as the flat float/int lists the header asks for.
        self.call(|error| unsafe {
            ou_define_mesh(
                self.0,
                parent.as_ptr(),
                &desc,
                path.as_mut_ptr(),
                TEXT_LEN,
                error,
                TEXT_LEN,
            )
        })?;
        Ok(text(&path))
    }

    /// Defines a Material; `files` gives each image id's asset path.
    pub fn define_material(
        &self,
        parent: &str,
        material: &MaterialPrim,
        files: &BTreeMap<u64, String>,
    ) -> Result<String> {
        let name = c_string(&material.name);
        let parent = c_string(parent);
        let input_names: Vec<CString> = material.inputs.iter().map(|i| c_string(i.name)).collect();
        // Strings each texture points to, kept alive until the call returns.
        let texture_strings: Vec<Option<(CString, CString)>> = material
            .inputs
            .iter()
            .map(|i| {
                i.texture.as_ref().map(|t| {
                    let file = files.get(&t.image).map(String::as_str).unwrap_or("");
                    (c_string(file), c_string(&t.uv))
                })
            })
            .collect();
        let textures: Vec<Option<OuTexture>> = material
            .inputs
            .iter()
            .zip(&texture_strings)
            .map(|(i, strings)| {
                let (t, (file, uv)): (&TextureUse, _) = (i.texture.as_ref()?, strings.as_ref()?);
                Some(OuTexture {
                    file: file.as_ptr(),
                    uv: uv.as_ptr(),
                    channel: t.channel as u32,
                    scale: t.scale,
                    bias: t.bias,
                    wrap: t.wrap as u32,
                    raw: t.raw as u32,
                    has_transform: t.transform.is_some() as u32,
                    uv_scale: t.transform.map_or([1.0, 1.0], |(s, _)| s),
                    uv_rotation: t.transform.map_or(0.0, |(_, r)| r),
                })
            })
            .collect();
        let inputs: Vec<OuSurfaceInput> = material
            .inputs
            .iter()
            .zip(&input_names)
            .zip(&textures)
            .map(|((i, name), texture)| OuSurfaceInput {
                name: name.as_ptr(),
                kind: i.kind as u32,
                value: i.value,
                texture: texture
                    .as_ref()
                    .map_or(std::ptr::null(), |t| t as *const OuTexture),
            })
            .collect();
        let desc = OuMaterial {
            name: name.as_ptr(),
            inputs: inputs.as_ptr(),
            input_count: inputs.len(),
        };
        let mut path = [0 as c_char; TEXT_LEN];
        // SAFETY: `desc`, its inputs, textures and strings all live in the
        // vectors above until the call returns; buffers are TEXT_LEN long.
        self.call(|error| unsafe {
            ou_define_material(
                self.0,
                parent.as_ptr(),
                &desc,
                path.as_mut_ptr(),
                TEXT_LEN,
                error,
                TEXT_LEN,
            )
        })?;
        Ok(text(&path))
    }

    /// Adds the MaterialX network `json` (`materialx_out::json`) to the
    /// Material at `material`.
    /// Writes a mesh's maps, morphs and sets (plan_extras.rs) on the Mesh
    /// at `mesh`.
    pub fn define_mesh_extras(
        &self,
        mesh: &str,
        extras: &crate::plan_extras::MeshExtras,
    ) -> Result<()> {
        let (json, buffers) = extras.json_and_buffers();
        let mesh = c_string(mesh);
        let json = c_string(&json);
        let pointers: Vec<*const u8> = buffers.iter().map(|b| b.as_ptr()).collect();
        let sizes: Vec<usize> = buffers.iter().map(Vec::len).collect();
        // SAFETY: the strings and every buffer the pointers and sizes
        // describe live until the call returns.
        self.call(|error| unsafe {
            ou_define_mesh_extras(
                self.0,
                mesh.as_ptr(),
                json.as_ptr(),
                pointers.as_ptr(),
                sizes.as_ptr(),
                buffers.len(),
                error,
                TEXT_LEN,
            )
        })
    }

    /// Writes a BasisCurves or NurbsCurves prim under `parent`.
    pub fn define_curves(
        &self,
        parent: &str,
        curves: &crate::curves::CurvesPrim,
        materials: &[String],
        visible: bool,
    ) -> Result<String> {
        let material = curves
            .material
            .and_then(|m| materials.get(m))
            .map(String::as_str);
        let (json, buffers) = curves.json_and_buffers(material, visible);
        let parent = c_string(parent);
        let json = c_string(&json);
        let pointers: Vec<*const u8> = buffers.iter().map(|b| b.as_ptr()).collect();
        let sizes: Vec<usize> = buffers.iter().map(Vec::len).collect();
        let mut path = [0 as c_char; TEXT_LEN];
        // SAFETY: the strings, buffers and the path buffer live until the
        // call returns; the bridge writes at most TEXT_LEN bytes of path.
        self.call(|error| unsafe {
            ou_define_curves(
                self.0,
                parent.as_ptr(),
                json.as_ptr(),
                pointers.as_ptr(),
                sizes.as_ptr(),
                buffers.len(),
                path.as_mut_ptr(),
                TEXT_LEN,
                error,
                TEXT_LEN,
            )
        })?;
        Ok(text(&path))
    }

    /// Writes a PointInstancer under `parent` whose one prototype is
    /// `prototype` (a prim path in this stage).
    pub fn define_point_instancer(
        &self,
        parent: &str,
        instancer: &crate::point_instancer::InstancerPrim,
        prototype: &str,
    ) -> Result<String> {
        let (json, buffers) = instancer.json_and_buffers(prototype);
        let parent = c_string(parent);
        let json = c_string(&json);
        let pointers: Vec<*const u8> = buffers.iter().map(|b| b.as_ptr()).collect();
        let sizes: Vec<usize> = buffers.iter().map(Vec::len).collect();
        let mut path = [0 as c_char; TEXT_LEN];
        // SAFETY: the strings, buffers and the path buffer live until the
        // call returns; the bridge writes at most TEXT_LEN bytes of path.
        self.call(|error| unsafe {
            ou_define_point_instancer(
                self.0,
                parent.as_ptr(),
                json.as_ptr(),
                pointers.as_ptr(),
                sizes.as_ptr(),
                buffers.len(),
                path.as_mut_ptr(),
                TEXT_LEN,
                error,
                TEXT_LEN,
            )
        })?;
        Ok(text(&path))
    }

    /// Makes the prim at `path` a class (abstract, not drawn).
    pub fn make_class(&self, path: &str) -> Result<()> {
        let path = c_string(path);
        // SAFETY: live stage, valid C string, TEXT_LEN error buffer.
        self.call(|error| unsafe { ou_make_class(self.0, path.as_ptr(), error, TEXT_LEN) })
    }

    /// Makes the prim at `path` an instanceable reference to `prototype`.
    pub fn make_instance(&self, path: &str, prototype: &str) -> Result<()> {
        let path = c_string(path);
        let prototype = c_string(prototype);
        // SAFETY: live stage, valid C strings, TEXT_LEN error buffer.
        self.call(|error| unsafe {
            ou_make_instance(self.0, path.as_ptr(), prototype.as_ptr(), error, TEXT_LEN)
        })
    }

    pub fn define_network(&self, material: &str, json: &str) -> Result<()> {
        let material = c_string(material);
        let json = c_string(json);
        // SAFETY: valid C strings and error buffer for the call.
        self.call(|error| unsafe {
            ou_define_network(self.0, material.as_ptr(), json.as_ptr(), error, TEXT_LEN)
        })
    }

    /// Writes the render settings (`render_usd::json`).
    pub fn define_render_settings(&self, json: &str) -> Result<()> {
        let json = c_string(json);
        // SAFETY: a valid C string and error buffer for the call.
        self.call(|error| unsafe {
            ou_define_render_settings(self.0, json.as_ptr(), error, TEXT_LEN)
        })
    }

    pub fn define_camera(&self, parent: &str, name: &str, camera: &CameraPrim) -> Result<String> {
        let name = c_string(name);
        let parent = c_string(parent);
        let desc = OuCamera {
            name: name.as_ptr(),
            orthographic: camera.orthographic as u32,
            focal_length: camera.focal_length,
            horizontal_aperture: camera.horizontal_aperture,
            vertical_aperture: camera.vertical_aperture,
            horizontal_offset: camera.horizontal_offset,
            vertical_offset: camera.vertical_offset,
            clipping: camera.clipping,
            f_stop: camera.f_stop,
            focus_distance: camera.focus_distance,
        };
        let mut path = [0 as c_char; TEXT_LEN];
        // SAFETY: `desc` and its strings live until the call returns.
        self.call(|error| unsafe {
            ou_define_camera(
                self.0,
                parent.as_ptr(),
                &desc,
                path.as_mut_ptr(),
                TEXT_LEN,
                error,
                TEXT_LEN,
            )
        })?;
        Ok(text(&path))
    }

    /// `texture`: the dome picture's asset path, if any.
    pub fn define_light(
        &self,
        parent: &str,
        name: &str,
        light: &LightPrim,
        texture: Option<&str>,
    ) -> Result<String> {
        let name = c_string(name);
        let parent = c_string(parent);
        let ies = (!light.ies_file.is_empty()).then(|| c_string(&light.ies_file));
        let texture = texture.map(c_string);
        let desc = OuLight {
            name: name.as_ptr(),
            kind: light.kind as u32,
            intensity: light.intensity,
            color: light.color,
            diffuse: light.diffuse,
            specular: light.specular,
            normalize: light.normalize as u32,
            radius: light.radius,
            width: light.width,
            height: light.height,
            length: light.length,
            angle: light.angle,
            cone_angle: light.cone_angle,
            cone_softness: light.cone_softness,
            ies_file: ies.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            texture_file: texture.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
        };
        let mut path = [0 as c_char; TEXT_LEN];
        // SAFETY: `desc` and its strings live until the call returns.
        self.call(|error| unsafe {
            ou_define_light(
                self.0,
                parent.as_ptr(),
                &desc,
                path.as_mut_ptr(),
                TEXT_LEN,
                error,
                TEXT_LEN,
            )
        })?;
        Ok(text(&path))
    }

    pub fn save(&self) -> Result<()> {
        // SAFETY: live stage, TEXT_LEN error buffer.
        self.call(|error| unsafe { ou_stage_save(self.0, error, TEXT_LEN) })
    }

    /// The flattened stage as .usda text.
    pub fn text(&self) -> Result<String> {
        // SAFETY: live stage; the returned string is freed with ou_string_free.
        unsafe {
            let raw = ou_stage_text(self.0);
            if raw.is_null() {
                return Err("Couldn't print the USD stage".into());
            }
            let out = CStr::from_ptr(raw).to_string_lossy().into_owned();
            ou_string_free(raw);
            Ok(out)
        }
    }

    /// A prim's local-to-world matrix, row-major, USD's row-vector convention.
    pub fn world_matrix(&self, path: &str) -> Option<[f64; 16]> {
        let path = c_string(path);
        let mut out = [0.0; 16];
        // SAFETY: live stage, valid C string, 16-double output.
        (unsafe { ou_world_matrix(self.0, path.as_ptr(), out.as_mut_ptr()) } != 0).then_some(out)
    }
}

/// Packs the saved USD file `source` and what it references into `usdz`.
pub fn package_usdz(source: &Path, usdz: &Path) -> Result<()> {
    ready()?;
    let source = c_string(&source.to_string_lossy());
    let usdz = c_string(&usdz.to_string_lossy());
    let mut error = [0 as c_char; TEXT_LEN];
    // SAFETY: valid C strings and a TEXT_LEN error buffer.
    if unsafe { ou_package_usdz(source.as_ptr(), usdz.as_ptr(), error.as_mut_ptr(), TEXT_LEN) } == 0
    {
        return Err(text(&error));
    }
    Ok(())
}

/// Writes `plan` to a new file at `path` (which must not exist yet);
/// `files` gives each image's asset path as the file should name it.
pub fn write(
    path: &Path,
    plan: &Plan,
    text_format: bool,
    files: &BTreeMap<u64, String>,
    dome_file: Option<&str>,
) -> Result<()> {
    let stage = Stage::create(path, text_format)?;
    let root = XformPrim {
        name: "root".into(),
        translate: [0.0; 3],
        rotate: [0.0; 3],
        order: crate::plan::RotateOrder::Xyz,
        scale: [1.0; 3],
        visible: true,
        meshes: Vec::new(),
        curves: Vec::new(),
        instance: None,
        camera: None,
        light: None,
        children: Vec::new(),
        instancers: Vec::new(),
    };
    let root_path = stage.define_xform("/", &root, plan.root_matrix)?;
    let scope = format!("{root_path}/_materials");
    let materials = plan
        .materials
        .iter()
        .map(|m| {
            let path = stage.define_material(&scope, m, files)?;
            if let Some(network) = &m.mtlx {
                stage.define_network(&path, &crate::materialx_out::json(network, files))?;
            }
            Ok(path)
        })
        .collect::<Result<Vec<String>>>()?;
    // Mesh Instance prototypes: a class Xform, so only the items that
    // reference them show them.
    let mut prototypes = Vec::new();
    if !plan.prototypes.is_empty() {
        let holder = plain_xform("_prototypes", true);
        let holder_path = stage.define_xform(&root_path, &holder, None)?;
        stage.make_class(&holder_path)?;
        for proto in &plan.prototypes {
            let path = stage.define_xform(&holder_path, proto, None)?;
            for mesh in &proto.meshes {
                write_mesh(&stage, &path, mesh, &materials)?;
            }
            for curves in &proto.curves {
                stage.define_curves(&path, curves, &materials, true)?;
            }
            prototypes.push(path);
        }
    }
    let written = Written {
        materials: &materials,
        prototypes: &prototypes,
        render_camera: std::cell::RefCell::new(None),
    };
    for prim in &plan.prims {
        write_prim(&stage, &root_path, prim, &written, 0)?;
    }
    for instancer in &plan.instancers {
        write_instancer(&stage, &root_path, instancer, &written)?;
    }
    // The environment: a DomeLight in its own Xform turned to the up axis.
    if let Some((dome, rotate)) = &plan.dome {
        let holder = XformPrim {
            rotate: *rotate,
            ..plain_xform("Environment", true)
        };
        let path = stage.define_xform(&root_path, &holder, None)?;
        stage.define_light(&path, "Dome", dome, dome_file)?;
    }
    // The Render item (render_usd.rs), naming the render camera.
    stage.define_render_settings(&crate::render_usd::json(
        &plan.render,
        written.render_camera.borrow().as_deref(),
    ))?;
    // Operandi works in metres.
    stage.set_metadata(plan.up_axis, 1.0, &root_path)?;
    stage.save()
}

/// An Xform with no transform of its own.
fn plain_xform(name: &str, visible: bool) -> XformPrim {
    XformPrim {
        name: name.into(),
        translate: [0.0; 3],
        rotate: [0.0; 3],
        order: crate::plan::RotateOrder::Xyz,
        scale: [1.0; 3],
        visible,
        meshes: Vec::new(),
        curves: Vec::new(),
        instance: None,
        camera: None,
        light: None,
        children: Vec::new(),
        instancers: Vec::new(),
    }
}

/// Prim paths written before the items: materials and prototypes, by
/// their plan index.
struct Written<'a> {
    materials: &'a [String],
    prototypes: &'a [String],
    /// The render camera's prim path, once written.
    render_camera: std::cell::RefCell<Option<String>>,
}

fn write_mesh(stage: &Stage, parent: &str, mesh: &MeshPrim, materials: &[String]) -> Result<()> {
    let path = stage.define_mesh(parent, mesh, materials)?;
    if !mesh.extras.is_empty() {
        stage.define_mesh_extras(&path, &mesh.extras)?;
    }
    Ok(())
}

fn write_prim(
    stage: &Stage,
    parent: &str,
    prim: &XformPrim,
    written: &Written,
    depth: usize,
) -> Result<()> {
    if depth > 10_000 {
        return Err("The item hierarchy is too deep to export".into());
    }
    let path = stage.define_xform(parent, prim, None)?;
    for mesh in &prim.meshes {
        write_mesh(stage, &path, mesh, written.materials)?;
    }
    let visible = prim.meshes.first().is_none_or(|m| m.visible);
    for curves in &prim.curves {
        stage.define_curves(&path, curves, written.materials, visible)?;
    }
    // A Mesh Instance: a child Xform with no transform that references the
    // prototype, so the item's own children stay its own.
    if let Some(instance) = prim.instance {
        let prototype = written
            .prototypes
            .get(instance.prototype)
            .ok_or("An instance names a missing prototype")?;
        let holder = stage.define_xform(&path, &plain_xform(&prim.name, instance.visible), None)?;
        stage.make_instance(&holder, prototype)?;
    }
    if let Some(camera) = &prim.camera {
        let at = stage.define_camera(&path, &prim.name, camera)?;
        if camera.render {
            *written.render_camera.borrow_mut() = Some(at);
        }
    }
    if let Some(light) = &prim.light {
        stage.define_light(&path, &prim.name, light, None)?;
    }
    for child in &prim.children {
        write_prim(stage, &path, child, written, depth + 1)?;
    }
    for instancer in &prim.instancers {
        write_instancer(stage, &path, instancer, written)?;
    }
    Ok(())
}

fn write_instancer(
    stage: &Stage,
    parent: &str,
    instancer: &crate::point_instancer::InstancerPrim,
    written: &Written,
) -> Result<()> {
    let prototype = written
        .prototypes
        .get(instancer.prototype)
        .ok_or("An instancer names a missing prototype")?;
    stage.define_point_instancer(parent, instancer, prototype)?;
    Ok(())
}
