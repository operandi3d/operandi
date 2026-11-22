// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later

//! UV unwrapping and island packing for Operandi.
//!
//! This crate is a Rust port of Blender's UV parametrizer and packer
//! (`source/blender/geometry/intern/uv_parametrizer.cc`, `uv_pack.cc`, and
//! the SLIM solver in `intern/slim`), at Blender commit
//! b4eed6fd498e76a7e93404bac2326e9f84deba22. Blender's code is
//! GPL-2.0-or-later; the SLIM files are MPL-2.0 and keep that licence here.
//! Each file says what it was adapted from.
//!
//! It knows nothing about Operandi's scene: callers hand it points and
//! polygons ([`UvMesh`]) and get UVs back per corner. The tools that decide
//! which polygons, seams and maps to use live in `operandi-core`.

// The ported solvers keep Blender's index loops and argument lists so they
// can be compared with the original line by line.
#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]

mod abf;
mod boxpack;
mod chart;
mod geom;
mod lscm;
pub mod pack;
mod slim;
mod sparse;
mod stretch;

pub use chart::{ConstructOptions, Param};
pub use glam::{DVec2, DVec3};
pub use slim::SlimOptions;

/// One polygon corner: which point it uses, its current UV, and its flags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    pub point: u32,
    pub uv: DVec2,
    /// Pinned corners keep their UV; with two or more pins in a chart the
    /// solve is fitted to them.
    pub pin: bool,
    /// Selection, used by live unwrap and stretch minimisation (only selected
    /// vertices move).
    pub select: bool,
}

impl Corner {
    pub fn new(point: u32, uv: DVec2) -> Self {
        Self {
            point,
            uv,
            pin: false,
            select: true,
        }
    }
}

/// The surface to unwrap.
#[derive(Clone, Debug, Default)]
pub struct UvMesh {
    pub points: Vec<DVec3>,
    /// Optional per-point weights for weighted minimum stretch.
    pub weights: Option<Vec<f64>>,
    /// Polygons as corner lists, counter-clockwise seen from outside.
    pub faces: Vec<Vec<Corner>>,
    /// Edges to cut along, as point pairs (either order).
    pub seams: Vec<[u32; 2]>,
}

/// Why an unwrap or pack couldn't start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UvError {
    /// A face has fewer than three corners.
    FaceTooSmall { face: usize },
    /// A corner or seam names a point that doesn't exist.
    BadPoint { point: u32 },
    /// Weights were given but not one per point.
    BadWeights,
    /// A UV coordinate isn't finite.
    BadUv { face: usize },
}

impl std::fmt::Display for UvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UvError::FaceTooSmall { face } => {
                write!(f, "polygon {face} has fewer than three corners")
            }
            UvError::BadPoint { point } => write!(f, "point {point} doesn't exist"),
            UvError::BadWeights => write!(f, "there must be one weight per point"),
            UvError::BadUv { face } => write!(f, "polygon {face} has a UV that isn't a number"),
        }
    }
}

impl std::error::Error for UvError {}

impl UvMesh {
    pub fn validate(&self) -> Result<(), UvError> {
        let n = self.points.len();
        if let Some(w) = &self.weights
            && w.len() != n
        {
            return Err(UvError::BadWeights);
        }
        for (fi, face) in self.faces.iter().enumerate() {
            if face.len() < 3 {
                return Err(UvError::FaceTooSmall { face: fi });
            }
            for c in face {
                if c.point as usize >= n {
                    return Err(UvError::BadPoint { point: c.point });
                }
                if !c.uv.is_finite() {
                    return Err(UvError::BadUv { face: fi });
                }
            }
        }
        for s in &self.seams {
            for &p in s {
                if p as usize >= n {
                    return Err(UvError::BadPoint { point: p });
                }
            }
        }
        Ok(())
    }
}

/// Which flattening solver to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnwrapMethod {
    /// Least squares conformal maps.
    Conformal,
    /// Angle based flattening (ABF++), then LSCM from those angles.
    AngleBased,
    /// Scalable locally injective maps: least stretch, never flips.
    MinimumStretch,
}

/// Options for [`unwrap`].
#[derive(Clone, Copy, Debug)]
pub struct UnwrapOptions {
    pub method: UnwrapMethod,
    pub construct: ConstructOptions,
    /// Used by [`UnwrapMethod::MinimumStretch`].
    pub slim: SlimOptions,
    /// Scale islands so their UV sizes match their 3D sizes relative to each
    /// other.
    pub average_scale: bool,
}

impl Default for UnwrapOptions {
    fn default() -> Self {
        Self {
            method: UnwrapMethod::AngleBased,
            construct: ConstructOptions::default(),
            slim: SlimOptions::default(),
            average_scale: true,
        }
    }
}

/// The result of [`unwrap`].
#[derive(Clone, Debug)]
pub struct Unwrapped {
    /// New UV of every corner, face by face, in input order.
    pub uvs: Vec<Vec<DVec2>>,
    /// Island of every face (`None` for faces in islands that were dropped).
    pub islands: Vec<Option<usize>>,
    pub island_count: usize,
    /// Islands the solver couldn't flatten (closed pieces with no seam, or
    /// singular systems).
    pub failed: usize,
}

/// Unwraps `mesh` in one go: build charts, flatten each one, even out island
/// scale. Islands are not packed; see [`pack`].
pub fn unwrap(mesh: &UvMesh, options: &UnwrapOptions) -> Result<Unwrapped, UvError> {
    let mut construct = options.construct;
    if options.method == UnwrapMethod::MinimumStretch {
        // SLIM needs every chart to have a single border.
        construct.fill_holes = true;
        let mut p = Param::new(mesh, construct)?;
        let (_, failed) = p.slim_solve(&options.slim, false);
        return finish(p, options, failed);
    }
    let mut p = Param::new(mesh, construct)?;
    p.lscm_begin_all(false, options.method == UnwrapMethod::AngleBased, false);
    let (_, failed) = p.lscm_solve();
    p.lscm_end();
    finish(p, options, failed)
}

fn finish(mut p: Param, options: &UnwrapOptions, failed: usize) -> Result<Unwrapped, UvError> {
    if options.average_scale {
        p.average(true, false, false);
    }
    p.flush();
    Ok(Unwrapped {
        uvs: p.corner_uvs(),
        islands: p.face_charts(),
        island_count: p.chart_count(),
        failed: failed + p.failed_chart_count(),
    })
}

#[cfg(test)]
mod tests;
