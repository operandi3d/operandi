//! Safe Rust over Operandi's C shim to the Alembic library (BSD-3-Clause).
//! Reads polygon and subdivision meshes at their first sample, world space;
//! writes polygon meshes. Ogawa archives only (HDF5 is not built in).
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;

#[repr(C)]
struct OaMesh {
    name: *const c_char,
    points: *const f64,
    point_count: usize,
    face_counts: *const i32,
    face_count: usize,
    indices: *const i32,
    index_count: usize,
    uvs: *const f32,
}

type OaMeshFn = unsafe extern "C" fn(*mut c_void, *const OaMesh);

unsafe extern "C" {
    fn oa_read(
        path: *const c_char,
        context: *mut c_void,
        on_mesh: OaMeshFn,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn oa_write(
        path: *const c_char,
        meshes: *const OaMesh,
        count: usize,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
}

/// A polygon mesh: points, polygon sizes and corner indices, wound
/// counter-clockwise seen from the front; UVs one pair per corner.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub name: String,
    pub points: Vec<[f64; 3]>,
    pub face_counts: Vec<i32>,
    pub indices: Vec<i32>,
    pub uvs: Option<Vec<[f32; 2]>>,
}

fn c_path(path: &Path) -> Result<CString, String> {
    CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| "The file name has a NUL character".to_owned())
}

fn message(buffer: &[c_char]) -> String {
    // SAFETY: the shim always NUL-terminates inside the buffer, and the
    // buffer starts zeroed, so there is a terminator either way.
    unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

unsafe extern "C" fn collect(context: *mut c_void, mesh: *const OaMesh) {
    // SAFETY: `context` is the `Vec<Mesh>` `read` passed in, alive for the
    // whole call; `mesh` and every array in it are valid for this callback
    // with the lengths the shim gives.
    unsafe {
        let out = &mut *(context as *mut Vec<Mesh>);
        let m = &*mesh;
        let slice = |p: *const f64, n: usize| {
            if n == 0 {
                &[][..]
            } else {
                std::slice::from_raw_parts(p, n)
            }
        };
        let ints = |p: *const i32, n: usize| {
            if n == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(p, n).to_vec()
            }
        };
        let points = slice(m.points, m.point_count * 3)
            .as_chunks::<3>()
            .0
            .to_vec();
        let uvs = (!m.uvs.is_null()).then(|| {
            std::slice::from_raw_parts(m.uvs, m.index_count * 2)
                .as_chunks::<2>()
                .0
                .to_vec()
        });
        out.push(Mesh {
            name: CStr::from_ptr(m.name).to_string_lossy().into_owned(),
            points,
            face_counts: ints(m.face_counts, m.face_count),
            indices: ints(m.indices, m.index_count),
            uvs,
        });
    }
}

/// Every mesh in the archive at `path`.
pub fn read(path: &Path) -> Result<Vec<Mesh>, String> {
    let path = c_path(path)?;
    let mut meshes: Vec<Mesh> = Vec::new();
    let mut error = vec![0 as c_char; 1024];
    // SAFETY: every pointer is valid for the call; `collect` matches the
    // callback type and only touches `meshes` through `context`.
    let failed = unsafe {
        oa_read(
            path.as_ptr(),
            &mut meshes as *mut Vec<Mesh> as *mut c_void,
            collect,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if failed != 0 {
        return Err(message(&error));
    }
    Ok(meshes)
}

/// Write `meshes` to a new archive at `path`. Names must be unique and
/// non-empty; every index must name a point and every count be at least 3.
pub fn write(path: &Path, meshes: &[Mesh]) -> Result<(), String> {
    let mut names = std::collections::BTreeSet::new();
    for m in meshes {
        if m.name.is_empty() || !names.insert(m.name.as_str()) {
            return Err(format!("Alembic object names must be unique: {:?}", m.name));
        }
        let corners: i64 = m.face_counts.iter().map(|&c| c as i64).sum();
        if m.face_counts.iter().any(|&c| c < 3)
            || corners != m.indices.len() as i64
            || m.indices
                .iter()
                .any(|&i| i < 0 || i as usize >= m.points.len())
            || m.uvs.as_ref().is_some_and(|u| u.len() != m.indices.len())
        {
            return Err(format!("{}: inconsistent polygon data", m.name));
        }
    }
    let path = c_path(path)?;
    let names: Vec<CString> = meshes
        .iter()
        .map(|m| CString::new(m.name.replace('\0', "")).expect("NUL removed"))
        .collect();
    let raw: Vec<OaMesh> = meshes
        .iter()
        .zip(&names)
        .map(|(m, name)| OaMesh {
            name: name.as_ptr(),
            points: m.points.as_ptr() as *const f64,
            point_count: m.points.len(),
            face_counts: m.face_counts.as_ptr(),
            face_count: m.face_counts.len(),
            indices: m.indices.as_ptr(),
            index_count: m.indices.len(),
            uvs: m
                .uvs
                .as_ref()
                .map_or(std::ptr::null(), |u| u.as_ptr() as *const f32),
        })
        .collect();
    let mut error = vec![0 as c_char; 1024];
    // SAFETY: `raw` points into `meshes` and `names`, both alive and
    // unchanged for the call; [f64; 3] and [f32; 2] arrays are laid out as
    // the flat arrays the shim reads, and the checks above keep every
    // index inside its array.
    let failed = unsafe {
        oa_write(
            path.as_ptr(),
            raw.as_ptr(),
            raw.len(),
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if failed != 0 {
        return Err(message(&error));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_read_keeps_a_quad_its_winding_and_uvs() {
        let dir = std::env::temp_dir().join(format!("operandi-alembic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("quad.abc");
        let quad = Mesh {
            name: "Quad".into(),
            points: vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
            face_counts: vec![4],
            indices: vec![0, 1, 2, 3],
            uvs: Some(vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]),
        };
        write(&path, std::slice::from_ref(&quad)).unwrap();
        assert_eq!(read(&path).unwrap(), vec![quad]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bad_files_and_bad_meshes_are_errors() {
        let dir = std::env::temp_dir().join(format!("operandi-alembic-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.abc");
        std::fs::write(&path, b"not an archive").unwrap();
        assert!(read(&path).is_err());
        let bad = Mesh {
            name: "Bad".into(),
            points: vec![[0.; 3]; 3],
            face_counts: vec![3],
            indices: vec![0, 1, 7],
            uvs: None,
        };
        assert!(write(&dir.join("out.abc"), &[bad]).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
