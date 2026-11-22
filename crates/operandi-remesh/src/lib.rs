// SPDX-License-Identifier: GPL-2.0-or-later
//! A safe wrapper around QuadriFlow (Huang et al. 2018), through Blender's
//! C API adapted in `cpp/quadriflow_capi.cpp`: all-quad output that follows
//! the surface's curvature, for Automatic Retopology. It takes triangles,
//! returns quads and touches nothing global, so it may run on any thread.

use std::ffi::{c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};

#[repr(C)]
struct QuadriflowRemeshData {
    verts: *const f32,
    faces: *const c_int,
    totfaces: c_int,
    totverts: c_int,
    out_verts: *mut f32,
    out_faces: *mut c_int,
    out_totverts: c_int,
    out_totfaces: c_int,
    target_faces: c_int,
    preserve_sharp: bool,
    sharp_angle: f32,
    preserve_boundary: bool,
    adaptive_scale: bool,
    minimum_cost_flow: bool,
    aggresive_sat: bool,
    rng_seed: c_int,
}

unsafe extern "C" {
    fn QFLOW_quadriflow_remesh(
        qrd: *mut QuadriflowRemeshData,
        update_cb: extern "C" fn(*mut c_void, f32, *mut c_int),
        update_cb_data: *mut c_void,
    );
    fn QFLOW_free_result(qrd: *mut QuadriflowRemeshData);
}

/// A remesh result: points and quads (four point indices each).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuadMesh {
    pub points: Vec<[f32; 3]>,
    pub quads: Vec<[u32; 4]>,
}

/// Why a back end gave nothing back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemeshError {
    /// The input was empty or had a point index out of range.
    BadInput(&'static str),
    /// The cancel flag was raised.
    Canceled,
    /// The solver ran but found no answer.
    Failed(&'static str),
}

impl std::fmt::Display for RemeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemeshError::BadInput(s) | RemeshError::Failed(s) => f.write_str(s),
            RemeshError::Canceled => f.write_str("Remesh canceled"),
        }
    }
}

impl std::error::Error for RemeshError {}

fn check_input(points: &[[f32; 3]], tris: &[[u32; 3]]) -> Result<(), RemeshError> {
    if points.is_empty() || tris.is_empty() {
        return Err(RemeshError::BadInput("Nothing to remesh"));
    }
    if points.len() > i32::MAX as usize / 3 || tris.len() > i32::MAX as usize / 9 {
        return Err(RemeshError::BadInput("The mesh is too big to remesh"));
    }
    let n = points.len() as u32;
    if tris.iter().flatten().any(|&i| i >= n) {
        return Err(RemeshError::BadInput("A triangle points past the mesh"));
    }
    if points.iter().flatten().any(|c| !c.is_finite()) {
        return Err(RemeshError::BadInput(
            "The mesh has a point that isn't a number",
        ));
    }
    Ok(())
}

/// QuadriFlow options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadriflowOptions {
    /// Roughly how many quads to make.
    pub target_faces: u32,
    /// Line quads up with creases whose faces meet at more than this many
    /// radians; 0 is off.
    pub crease_angle: f32,
    /// Line quads up with open borders.
    pub preserve_boundary: bool,
    /// Smaller quads where the surface curves more.
    pub adaptive_scale: bool,
    /// Slower, better-spread quads (minimum-cost flow).
    pub minimum_cost_flow: bool,
    /// Seed for the random start; the same seed gives the same result.
    pub seed: i32,
}

struct Progress<'a> {
    cancel: &'a AtomicBool,
    report: &'a mut dyn FnMut(f32),
}

extern "C" fn quadriflow_progress(data: *mut c_void, progress: f32, cancel: *mut c_int) {
    // SAFETY: `data` is the `Progress` that `quadriflow` passes in and
    // keeps alive for the whole call, and `cancel` is QuadriFlow's own
    // local int. Panics are caught so they never unwind into C++.
    let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        let p = &mut *(data as *mut Progress);
        (p.report)(progress);
        p.cancel.load(Ordering::Relaxed)
    }));
    let stop = ok.unwrap_or(true);
    // SAFETY: see above.
    unsafe { *cancel = stop as c_int };
}

/// Remeshes a closed triangle mesh into quads with QuadriFlow. `cancel` is
/// checked between stages; `report` gets progress from 0 to 1.
pub fn quadriflow(
    points: &[[f32; 3]],
    tris: &[[u32; 3]],
    options: &QuadriflowOptions,
    cancel: &AtomicBool,
    report: &mut dyn FnMut(f32),
) -> Result<QuadMesh, RemeshError> {
    check_input(points, tris)?;
    let target = options.target_faces.clamp(1, i32::MAX as u32 / 4) as c_int;
    let mut qrd = QuadriflowRemeshData {
        verts: points.as_ptr() as *const f32,
        faces: tris.as_ptr() as *const c_int,
        totfaces: tris.len() as c_int,
        totverts: points.len() as c_int,
        out_verts: std::ptr::null_mut(),
        out_faces: std::ptr::null_mut(),
        out_totverts: 0,
        out_totfaces: 0,
        target_faces: target,
        preserve_sharp: options.crease_angle > 0.0,
        sharp_angle: options.crease_angle.to_degrees(),
        preserve_boundary: options.preserve_boundary,
        adaptive_scale: options.adaptive_scale,
        minimum_cost_flow: options.minimum_cost_flow,
        // The aggressive SAT pass shells out to a `minisat` program;
        // Operandi never turns it on.
        aggresive_sat: false,
        rng_seed: options.seed,
    };
    let mut progress = Progress { cancel, report };
    // SAFETY: `points` and `tris` are [f32; 3] / [u32; 3] arrays, so the
    // pointers cover totverts * 3 floats and totfaces * 3 ints, all in
    // range (checked above). The C++ side only reads them, writes the
    // out_* fields, and calls `quadriflow_progress` with `progress`, which
    // lives until the call returns.
    unsafe {
        QFLOW_quadriflow_remesh(
            &mut qrd,
            quadriflow_progress,
            &mut progress as *mut Progress as *mut c_void,
        );
    }
    let out = take_quads(
        qrd.out_verts,
        qrd.out_totverts,
        qrd.out_faces,
        qrd.out_totfaces,
    );
    // SAFETY: frees what QFLOW_quadriflow_remesh allocated (null is fine).
    unsafe { QFLOW_free_result(&mut qrd) };
    if cancel.load(Ordering::Relaxed) {
        return Err(RemeshError::Canceled);
    }
    out.ok_or(RemeshError::Failed(
        "Quad Remesh found no answer for this mesh; try more polygons or close its holes",
    ))
}

/// Copies C output arrays into a [`QuadMesh`], dropping quads that point
/// past the points. None when there's nothing usable.
fn take_quads(
    verts: *const f32,
    totvert: c_int,
    quads: *const c_int,
    totquad: c_int,
) -> Option<QuadMesh> {
    if verts.is_null() || quads.is_null() || totvert <= 0 || totquad <= 0 {
        return None;
    }
    // SAFETY: both back ends allocate exactly 3 * totvert floats and
    // 4 * totquad ints for these pointers.
    let (v, q) = unsafe {
        (
            std::slice::from_raw_parts(verts, totvert as usize * 3),
            std::slice::from_raw_parts(quads, totquad as usize * 4),
        )
    };
    let n = totvert as usize;
    let points: Vec<[f32; 3]> = v.as_chunks::<3>().0.to_vec();
    let quads: Vec<[u32; 4]> = q
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|c| c.iter().all(|&i| i >= 0 && (i as usize) < n))
        .map(|c| [c[0] as u32, c[1] as u32, c[2] as u32, c[3] as u32])
        .collect();
    (!quads.is_empty()).then_some(QuadMesh { points, quads })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit cube as 12 triangles.
    fn cube() -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        let p = (0..8)
            .map(|i| {
                [
                    (i & 1) as f32 - 0.5,
                    ((i >> 1) & 1) as f32 - 0.5,
                    ((i >> 2) & 1) as f32 - 0.5,
                ]
            })
            .collect();
        let quads = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        let t = quads
            .iter()
            .flat_map(|q: &[u32; 4]| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]])
            .collect();
        (p, t)
    }

    #[test]
    fn quadriflow_cube() {
        let (p, t) = cube();
        let cancel = AtomicBool::new(false);
        let mut last = 0.0;
        let m = quadriflow(
            &p,
            &t,
            &QuadriflowOptions {
                target_faces: 200,
                crease_angle: 1.0,
                preserve_boundary: false,
                adaptive_scale: false,
                minimum_cost_flow: false,
                seed: 0,
            },
            &cancel,
            &mut |f| last = f,
        )
        .unwrap();
        assert!(m.quads.len() > 50, "{} quads", m.quads.len());
        assert!(last >= 0.9);
    }

    #[test]
    fn quadriflow_cancel() {
        let (p, t) = cube();
        let cancel = AtomicBool::new(true);
        let r = quadriflow(
            &p,
            &t,
            &QuadriflowOptions {
                target_faces: 200,
                crease_angle: 0.0,
                preserve_boundary: false,
                adaptive_scale: false,
                minimum_cost_flow: false,
                seed: 0,
            },
            &cancel,
            &mut |_| {},
        );
        assert_eq!(r, Err(RemeshError::Canceled));
    }

    #[test]
    fn bad_input() {
        let cancel = AtomicBool::new(false);
        let o = QuadriflowOptions {
            target_faces: 10,
            crease_angle: 0.0,
            preserve_boundary: false,
            adaptive_scale: false,
            minimum_cost_flow: false,
            seed: 0,
        };
        let q = |p: &[[f32; 3]], t: &[[u32; 3]]| quadriflow(p, t, &o, &cancel, &mut |_| {});
        assert!(matches!(q(&[], &[]), Err(RemeshError::BadInput(_))));
        assert!(matches!(
            q(&[[0.0; 3]], &[[0, 1, 2]]),
            Err(RemeshError::BadInput(_))
        ));
    }
}
