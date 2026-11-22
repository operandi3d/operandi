// SPDX-FileCopyrightText: 2023 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `source/blender/geometry/intern/uv_pack.cc` at commit
// b4eed6fd498e76a7e93404bac2326e9f84deba22. Job-system cancellation and
// progress reporting were left out.

//! Packs UV islands into a square: several strategies (an "L" strip packer,
//! box packing, known optimal square packings and an xatlas-style bitmap
//! packer that follows island shapes) are tried and the smallest layout wins.

// The packing tables and 0.7071 bounds are Blender's own literals.
#![allow(clippy::excessive_precision, clippy::approx_constant)]

use crate::boxpack::{BoxPack, box_pack_2d};
use crate::geom;
use glam::DVec2;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

/// Which rotations islands may take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationMethod {
    None,
    /// Rotated to a minimal rectangle, either vertical or horizontal.
    AxisAligned,
    /// Aligned along U (wide islands).
    AxisAlignedX,
    /// Aligned along V (tall islands).
    AxisAlignedY,
    /// Only 90° turns.
    Cardinal,
    Any,
}

/// The shape packing follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeMethod {
    /// Bounding boxes.
    Aabb,
    /// Convex hulls.
    Convex,
    /// The islands' own triangles.
    Concave,
}

/// How `margin` is measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarginMethod {
    /// Scaled by the size of the existing UVs.
    Scaled,
    /// Added as is.
    Add,
    /// A fraction of the final layout (the gap stays the same however the
    /// islands are scaled).
    Fraction,
}

/// How islands with pinned UVs are treated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinMethod {
    /// Pins don't matter.
    None,
    LockRotation,
    LockRotationScale,
    LockScale,
    /// Pinned islands stay exactly where they are; others pack around them.
    LockAll,
}

/// Packing options (`UVPackIsland_Params`).
#[derive(Clone, Copy, Debug)]
pub struct PackParams {
    pub rotate_method: RotationMethod,
    /// Resize islands to fill the target.
    pub scale_to_fit: bool,
    pub pin_method: PinMethod,
    /// Islands that overlap move as one.
    pub merge_overlap: bool,
    pub margin: f64,
    pub margin_method: MarginMethod,
    /// Added to every island after packing (the UDIM tile corner).
    pub udim_base_offset: DVec2,
    /// Size of the target square in V; 1 for the unit tile.
    pub target_extent: f64,
    pub target_aspect_y: f64,
    pub shape_method: ShapeMethod,
}

impl Default for PackParams {
    fn default() -> Self {
        Self {
            rotate_method: RotationMethod::None,
            scale_to_fit: true,
            pin_method: PinMethod::None,
            merge_overlap: false,
            margin: 0.001,
            margin_method: MarginMethod::Scaled,
            udim_base_offset: DVec2::ZERO,
            target_extent: 1.0,
            target_aspect_y: 1.0,
            shape_method: ShapeMethod::Aabb,
        }
    }
}

/// A 2x2 matrix stored by columns, like Blender's `float[2][2]`.
#[derive(Clone, Copy, Debug)]
pub struct Mat2(pub [[f64; 2]; 2]);

impl Mat2 {
    pub fn mul(&self, v: DVec2) -> DVec2 {
        let m = &self.0;
        DVec2::new(m[0][0] * v.x + m[1][0] * v.y, m[0][1] * v.x + m[1][1] * v.y)
    }
}

/// Where an island goes: translation of its pivot and rotation.
#[derive(Clone, Copy, Debug)]
struct Phi {
    translation: DVec2,
    rotation: f64,
}

impl Default for Phi {
    fn default() -> Self {
        Self {
            translation: DVec2::splat(-1.0),
            rotation: 0.0,
        }
    }
}

impl Phi {
    fn is_valid(&self) -> bool {
        self.translation.x != -1.0
    }
}

/// One island to pack (`PackIsland`). Add its triangles, pack, then map
/// each of its UVs with [`PackIsland::transform`].
#[derive(Clone, Debug)]
pub struct PackIsland {
    pub aspect_y: f64,
    pub pinned: bool,
    /// Output: added to UVs before the transform.
    pub pre_translate: DVec2,
    /// Output: rotation in radians.
    pub angle: f64,
    /// Caller's own index, untouched.
    pub caller_index: usize,
    triangles: Vec<DVec2>,
    pivot: DVec2,
    half_diagonal: DVec2,
    pre_rotate_sincos: DVec2,
}

impl Default for PackIsland {
    fn default() -> Self {
        Self {
            aspect_y: 1.0,
            pinned: false,
            pre_translate: DVec2::ZERO,
            angle: 0.0,
            caller_index: 0,
            triangles: Vec::new(),
            pivot: DVec2::ZERO,
            half_diagonal: DVec2::ZERO,
            pre_rotate_sincos: DVec2::new(0.0, 1.0),
        }
    }
}

/// Signed squared distance from `probe` to the line through `a` and `b`.
fn dist_signed_squared_to_edge(probe: DVec2, a: DVec2, b: DVec2) -> f64 {
    let edge = b - a;
    let side = probe - a;
    let len2 = edge.length_squared();
    if len2 < 1e-40 {
        return side.length_squared();
    }
    let num = edge.x * side.y - edge.y * side.x;
    let ssq = if num >= 0.0 { num * num } else { -num * num };
    ssq / len2
}

fn sincos_rotate_90(sc: DVec2) -> DVec2 {
    DVec2::new(sc.y, -sc.x)
}

fn sincos_wrap(sc: DVec2) -> DVec2 {
    if sc.y <= 0.0 { -sc } else { sc }
}

fn sincos_plusminus_90(sc: DVec2) -> DVec2 {
    let r = sincos_wrap(sc);
    let eps = 0.1_f64.to_radians();
    if r.x.abs() < eps {
        DVec2::new(0.0, 1.0)
    } else if r.y.abs() < eps {
        DVec2::new(if r.x < 0.0 { -1.0 } else { 1.0 }, 0.0)
    } else {
        r
    }
}

fn transformation_from_sincos(sc: DVec2, aspect_y: f64) -> Mat2 {
    Mat2([[sc.y, -sc.x * aspect_y], [sc.x / aspect_y, sc.y]])
}

fn inverse_transformation_from_sincos(sc: DVec2, aspect_y: f64) -> Mat2 {
    Mat2([[sc.y, sc.x * aspect_y], [-sc.x / aspect_y, sc.y]])
}

impl PackIsland {
    pub fn new(aspect_y: f64, pinned: bool, caller_index: usize) -> Self {
        Self {
            aspect_y,
            pinned,
            caller_index,
            ..Default::default()
        }
    }

    /// Adds a triangle (any winding).
    pub fn add_triangle(&mut self, a: DVec2, b: DVec2, c: DVec2) {
        if dist_signed_squared_to_edge(a, b, c) < 0.0 {
            self.triangles.extend([a, b, c]);
        } else {
            self.triangles.extend([a, c, b]);
        }
    }

    /// Adds a polygon, triangulated.
    pub fn add_polygon(&mut self, uvs: &[DVec2]) {
        if uvs.len() < 3 {
            return;
        }
        if uvs.len() == 3 {
            self.add_triangle(uvs[0], uvs[1], uvs[2]);
            return;
        }
        let co: Vec<glam::DVec3> = uvs.iter().map(|p| p.extend(0.0)).collect();
        for t in geom::triangulate(&co) {
            self.add_triangle(uvs[t[0]], uvs[t[1]], uvs[t[2]]);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.len() < 3
    }

    /// The matrix that, applied to `uv + pre_translate`, places the island.
    pub fn build_transformation(&self, scale: f64, angle: f64) -> Mat2 {
        let (s, c) = angle.sin_cos();
        Mat2([
            [c * scale, -s * scale * self.aspect_y],
            [s * scale / self.aspect_y, c * scale],
        ])
    }

    fn build_inverse_transformation(&self, scale: f64, angle: f64) -> Mat2 {
        let (s, c) = angle.sin_cos();
        Mat2([
            [c / scale, s / scale * self.aspect_y],
            [-s / scale / self.aspect_y, c / scale],
        ])
    }

    /// Where `uv` goes once packed, with `scale` the value [`pack_islands`]
    /// returned (use 1 for islands that may not scale) and `offset` the
    /// params' `udim_base_offset`.
    pub fn transform(&self, uv: DVec2, scale: f64, offset: DVec2) -> DVec2 {
        self.build_transformation(scale, self.angle)
            .mul(uv + self.pre_translate)
            + offset
    }

    fn calculate_pre_rotation(&mut self, params: &PackParams) {
        self.pre_rotate_sincos = DVec2::new(0.0, 1.0);
        if params.rotate_method == RotationMethod::Cardinal {
            return;
        }
        if !self.can_rotate_before_pack(params) {
            return;
        }
        let mut coords: Vec<DVec2> = self
            .triangles
            .iter()
            .map(|p| DVec2::new(p.x * self.aspect_y, p.y))
            .collect();
        let fit = geom::aabb_fit_sincos(&coords);
        let mut sc = DVec2::new(-fit.x, fit.y);
        // "Stand up" islands.
        let m = Mat2([[fit.y, fit.x], [-fit.x, fit.y]]);
        for p in &mut coords {
            *p = m.mul(*p);
        }
        let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
        for p in &coords {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        let d = hi - lo;
        match params.rotate_method {
            RotationMethod::AxisAlignedX => {
                if d.x < d.y {
                    sc = sincos_rotate_90(sc);
                }
                sc = sincos_wrap(sc);
            }
            RotationMethod::AxisAlignedY => {
                if d.x > d.y {
                    sc = sincos_rotate_90(sc);
                }
                sc = sincos_wrap(sc);
            }
            _ => {
                if d.y < d.x {
                    sc = sincos_rotate_90(sc);
                }
                sc = sincos_plusminus_90(sc);
            }
        }
        if sc == DVec2::new(0.0, 1.0) {
            return;
        }
        self.pre_rotate_sincos = sc;
        let m = transformation_from_sincos(sc, self.aspect_y);
        for p in &mut self.triangles {
            *p = m.mul(*p);
        }
    }

    fn finalize_geometry(&mut self, params: &PackParams) {
        self.calculate_pre_rotation(params);
        if params.shape_method == ShapeMethod::Convex && self.triangles.len() > 3 {
            let hull = geom::convex_hull(&self.triangles);
            if hull.len() >= 3 {
                let poly: Vec<DVec2> = hull.iter().map(|&i| self.triangles[i]).collect();
                self.triangles.clear();
                self.add_polygon(&poly);
            }
        }
        self.calculate_pivot();
    }

    fn calculate_pivot(&mut self) {
        let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
        for p in &self.triangles {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        if self.triangles.is_empty() {
            lo = DVec2::ZERO;
            hi = DVec2::ZERO;
        }
        self.pivot = (lo + hi) * 0.5;
        self.half_diagonal = (hi - lo) * 0.5;
    }

    fn place(&mut self, scale: f64, phi: Phi) {
        self.angle = phi.rotation + self.pre_rotate_sincos.x.atan2(self.pre_rotate_sincos.y);
        let inv = self.build_inverse_transformation(scale, phi.rotation);
        self.pre_translate = inv.mul(phi.translation) - self.pivot;
        if self.pre_rotate_sincos != DVec2::new(0.0, 1.0) {
            let inv = inverse_transformation_from_sincos(self.pre_rotate_sincos, self.aspect_y);
            self.pre_translate = inv.mul(self.pre_translate);
        }
    }

    fn can_rotate_with(&self, params: &PackParams, method: RotationMethod) -> bool {
        if matches!(
            method,
            RotationMethod::None | RotationMethod::AxisAlignedX | RotationMethod::AxisAlignedY
        ) {
            return false;
        }
        if !self.pinned {
            return true;
        }
        !matches!(
            params.pin_method,
            PinMethod::LockAll | PinMethod::LockRotation | PinMethod::LockRotationScale
        )
    }

    fn can_rotate_before_pack(&self, params: &PackParams) -> bool {
        let m = match params.rotate_method {
            RotationMethod::AxisAlignedX | RotationMethod::AxisAlignedY => {
                RotationMethod::AxisAligned
            }
            m => m,
        };
        self.can_rotate_with(params, m)
    }

    fn can_rotate(&self, params: &PackParams) -> bool {
        self.can_rotate_with(params, params.rotate_method)
    }

    /// Whether this island takes the packing scale.
    pub fn can_scale(&self, params: &PackParams) -> bool {
        if !params.scale_to_fit {
            return false;
        }
        if !self.pinned {
            return true;
        }
        !matches!(
            params.pin_method,
            PinMethod::LockAll | PinMethod::LockScale | PinMethod::LockRotationScale
        )
    }

    fn can_translate(&self, params: &PackParams) -> bool {
        !self.pinned || params.pin_method != PinMethod::LockAll
    }

    fn diagonal_support(&self, scale: f64, rotation: f64, margin: f64) -> DVec2 {
        if rotation == -PI || rotation == 0.0 || rotation == PI {
            return self.half_diagonal * scale + DVec2::splat(margin);
        }
        if rotation == -FRAC_PI_2 || rotation == FRAC_PI_2 || rotation == 1.5 * PI {
            return DVec2::new(
                self.half_diagonal.y / self.aspect_y,
                self.half_diagonal.x * self.aspect_y,
            ) * scale
                + DVec2::splat(margin);
        }
        let d = self
            .build_transformation(scale, rotation)
            .mul(self.half_diagonal);
        let (sx, sy) = (d.x.abs(), d.y.abs());
        DVec2::new(sx + sy * 0.7071 + margin, sx * 0.7071 + sy + margin)
    }
}

/// An axis-aligned rectangle (`rctf`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
}

impl Rect {
    fn new(xmin: f64, xmax: f64, ymin: f64, ymax: f64) -> Self {
        Self {
            xmin,
            xmax,
            ymin,
            ymax,
        }
    }
    fn width(&self) -> f64 {
        self.xmax - self.xmin
    }
    fn height(&self) -> f64 {
        self.ymax - self.ymin
    }
    fn include(&mut self, p: DVec2) {
        self.xmin = self.xmin.min(p.x);
        self.xmax = self.xmax.max(p.x);
        self.ymin = self.ymin.min(p.y);
        self.ymax = self.ymax.max(p.y);
    }
}

fn aspect_extent(r: &Rect, p: &PackParams) -> f64 {
    (r.width() / p.target_aspect_y).max(r.height())
}

fn aspect_area(r: &Rect, p: &PackParams) -> f64 {
    (r.width() / p.target_aspect_y) * r.height()
}

/// `compare_ff_relative(a, b, FLT_EPSILON, 64)`.
fn nearly_equal(a: f64, b: f64) -> bool {
    if (a - b).abs() <= f32::EPSILON as f64 {
        return true;
    }
    let (ia, ib) = ((a as f32).to_bits() as i32, (b as f32).to_bits() as i32);
    if (ia < 0) != (ib < 0) {
        return false;
    }
    (ia - ib).abs() <= 64
}

/// True when `b` is a better layout than `a`.
fn is_larger(a: &Rect, b: &Rect, p: &PackParams) -> bool {
    let (ea, eb) = (aspect_extent(a, p), aspect_extent(b, p));
    if nearly_equal(ea, eb) {
        return aspect_area(b, p) < aspect_area(a, p);
    }
    eb < ea
}

#[derive(Clone, Copy, Debug)]
struct Aabb {
    uv_diagonal: DVec2,
    index: usize,
    aspect_y: f64,
}

/// "Alpaca" strip packer, no rotation.
fn alpaca_turbo(
    exclude_index: usize,
    exclude: &Rect,
    islands: &[Aabb],
    target_aspect_y: f64,
    phis: &mut [Phi],
) -> Rect {
    let mut next_u1 = exclude.xmax;
    let mut next_v1 = exclude.ymax;
    let mut zigzag = next_u1 < next_v1 * target_aspect_y;
    let mut u0 = if zigzag { next_u1 } else { 0.0 };
    let mut v0 = if zigzag { 0.0 } else { next_v1 };
    for island in &islands[exclude_index.min(islands.len())..] {
        let (du, dv) = (island.uv_diagonal.x, island.uv_diagonal.y);
        let restart = if zigzag {
            next_v1 < v0 + dv
        } else {
            next_u1 < u0 + du
        };
        if restart {
            zigzag = next_u1 < next_v1 * target_aspect_y;
            u0 = if zigzag { next_u1 } else { 0.0 };
            v0 = if zigzag { 0.0 } else { next_v1 };
        }
        let phi = &mut phis[island.index];
        phi.rotation = 0.0;
        phi.translation = DVec2::new(u0 + du * 0.5, v0 + dv * 0.5);
        if zigzag {
            v0 += dv;
            next_u1 = next_u1.max(u0 + du);
            next_v1 = next_v1.max(v0);
        } else {
            u0 += du;
            next_v1 = next_v1.max(v0 + dv);
            next_u1 = next_u1.max(u0);
        }
    }
    Rect::new(0.0, next_u1, 0.0, next_v1)
}

fn update_hole_rotate(
    hole: &mut DVec2,
    diag: &mut DVec2,
    rotate: &mut bool,
    u0: f64,
    v0: f64,
    u1: f64,
    v1: f64,
) {
    let hole_area = diag.x * diag.y;
    let quad_area = (u1 - u0) * (v1 - v0);
    if quad_area <= hole_area {
        return;
    }
    *hole = DVec2::new(u0, v0);
    *diag = DVec2::new(u1 - u0, v1 - v0);
    if diag.y < diag.x {
        std::mem::swap(&mut diag.x, &mut diag.y);
        *rotate = true;
    } else {
        *rotate = false;
    }
}

/// "Alpaca" strip packer with 90° turns and hole filling.
fn alpaca_rotate(
    exclude_index: usize,
    exclude: &Rect,
    islands: &[Aabb],
    target_aspect_y: f64,
    phis: &mut [Phi],
) -> Rect {
    let mut next_u1 = exclude.xmax;
    let mut next_v1 = exclude.ymax;
    let mut zigzag = next_u1 / target_aspect_y < next_v1;
    let mut hole = DVec2::ZERO;
    let mut hole_diag = DVec2::ZERO;
    let mut hole_rotate = false;
    let mut u0 = if zigzag { next_u1 } else { 0.0 };
    let mut v0 = if zigzag { 0.0 } else { next_v1 };
    for island in &islands[exclude_index.min(islands.len())..] {
        let phi = &mut phis[island.index];
        let a = island.aspect_y;
        let uvdiag_x = island.uv_diagonal.x * a;
        let min_dsm = uvdiag_x.min(island.uv_diagonal.y);
        let max_dsm = uvdiag_x.max(island.uv_diagonal.y);

        if min_dsm < hole_diag.x && max_dsm < hole_diag.y {
            if hole_rotate == (min_dsm == island.uv_diagonal.x) {
                phi.rotation = FRAC_PI_2;
                phi.translation = DVec2::new(
                    hole.x + island.uv_diagonal.y * 0.5 / a,
                    hole.y + island.uv_diagonal.x * 0.5 * a,
                );
            } else {
                phi.rotation = 0.0;
                phi.translation = hole + island.uv_diagonal * 0.5;
            }
            let p0 = hole.x;
            let p1 = hole.y;
            let p2 = hole.x + if hole_rotate { max_dsm } else { min_dsm } / a;
            let p3 = hole.y + if hole_rotate { min_dsm } else { max_dsm };
            let p4 = hole.x
                + if hole_rotate {
                    hole_diag.y
                } else {
                    hole_diag.x
                };
            let p5 = hole.y
                + if hole_rotate {
                    hole_diag.x
                } else {
                    hole_diag.y
                };
            hole_diag.x = 0.0;
            update_hole_rotate(&mut hole, &mut hole_diag, &mut hole_rotate, p0, p3, p4, p5);
            update_hole_rotate(&mut hole, &mut hole_diag, &mut hole_rotate, p2, p1, p4, p5);
            continue;
        }

        let restart = if zigzag {
            next_v1 < v0 + min_dsm
        } else {
            next_u1 < u0 + min_dsm / a
        };
        if restart {
            update_hole_rotate(
                &mut hole,
                &mut hole_diag,
                &mut hole_rotate,
                u0,
                v0,
                next_u1,
                next_v1,
            );
            zigzag = next_u1 / target_aspect_y < next_v1;
            u0 = if zigzag { next_u1 } else { 0.0 };
            v0 = if zigzag { 0.0 } else { next_v1 };
        }
        if zigzag == (min_dsm == uvdiag_x) {
            phi.rotation = FRAC_PI_2;
            phi.translation = DVec2::new(
                u0 + island.uv_diagonal.y * 0.5 / a,
                v0 + island.uv_diagonal.x * 0.5 * a,
            );
        } else {
            phi.rotation = 0.0;
            phi.translation = DVec2::new(u0, v0) + island.uv_diagonal * 0.5;
        }
        if zigzag {
            v0 += min_dsm;
            next_u1 = next_u1.max(u0 + max_dsm / a);
            next_v1 = next_v1.max(v0);
        } else {
            u0 += min_dsm / a;
            next_v1 = next_v1.max(v0 + max_dsm);
            next_u1 = next_u1.max(u0);
        }
    }
    Rect::new(0.0, next_u1, 0.0, next_v1)
}

fn pack_fast(
    exclude_index: usize,
    exclude: &Rect,
    aabbs: &[Aabb],
    rotate: bool,
    target_aspect_y: f64,
    phis: &mut [Phi],
) -> Rect {
    if rotate {
        alpaca_rotate(exclude_index, exclude, aabbs, target_aspect_y, phis)
    } else {
        alpaca_turbo(exclude_index, exclude, aabbs, target_aspect_y, phis)
    }
}

/// Frits Göbel, 1979.
fn pack_gobel(aabbs: &[Aabb], scale: f64, m: usize, phis: &mut [Phi]) {
    let h = 0.5_f64.sqrt();
    for (i, a) in aabbs.iter().enumerate() {
        let phi = &mut phis[a.index];
        phi.rotation = 0.0;
        if i == 0 {
            phi.translation = DVec2::splat(0.5 * scale);
            continue;
        }
        let xx = (i - 1) % m;
        let yy = (i - 1) / m;
        phi.translation = DVec2::new((xx as f64 + 0.5) * scale, (yy as f64 + 0.5) * scale);
        if xx >= yy {
            phi.translation.x += (1.0 + h) * scale;
        } else {
            phi.translation.y += h * scale;
        }
        if i == m * (m + 1) + 1 {
            phi.translation.x += (m as f64 + h) * scale;
            phi.translation.y -= scale;
        } else if i > m * (m + 1) + 1 {
            phi.rotation = FRAC_PI_4;
            let t = (((i - m * (m + 1)) as f64 - 1.5) * phi.rotation.cos() + 1.0) * scale;
            phi.translation = DVec2::splat(t);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn optimal_pack_table(
    table_count: usize,
    max_extent: f64,
    optimal: &[f64],
    island_count: usize,
    large_uv: f64,
    aabbs: &[Aabb],
    params: &PackParams,
    phis: &mut [Phi],
    extent: &mut Rect,
) {
    if table_count < island_count {
        return;
    }
    let e = Rect::new(0.0, large_uv * max_extent, 0.0, large_uv * max_extent);
    if is_larger(&e, extent, params) {
        return;
    }
    *extent = e;
    for (i, a) in aabbs.iter().enumerate().take(island_count) {
        let phi = &mut phis[a.index];
        phi.translation = DVec2::new(optimal[i * 3], optimal[i * 3 + 1]) * large_uv;
        phi.rotation = optimal[i * 3 + 2];
    }
}

/// Known optimal packings of equal squares (Walter Trump, Pertti Hämäläinen,
/// Robert Wainwright, Erich Friedman, Frits Göbel).
fn optimal_pack(aabbs: &[Aabb], params: &PackParams, phis: &mut [Phi], extent: &mut Rect) {
    if params.shape_method == ShapeMethod::Aabb
        || params.target_aspect_y != 1.0
        || params.rotate_method != RotationMethod::Any
    {
        return;
    }
    let mut large_uv: f64 = 0.0;
    for a in aabbs {
        large_uv = large_uv.max(a.uv_diagonal.x).max(a.uv_diagonal.y);
    }
    let mut n = aabbs.len();
    const OPT_11: [f64; 33] = [
        2.6238700165660708840676,
        2.4365065643739085565755,
        0.70130710554829878145,
        1.9596047386700836678841,
        1.6885655318806973568257,
        0.70130710554829878145,
        1.9364970731945949644626,
        3.1724566890997589752033,
        0.70130710554829878145,
        1.2722458068219282267819,
        2.4245322476118422727609,
        0.70130710554829878145,
        3.1724918301381124230431,
        1.536261617698265524723,
        0.70130710554829878145,
        3.3770999999999999907629,
        3.3770999999999999907629,
        0.0,
        0.5,
        1.5,
        0.0,
        2.5325444557069398676674,
        0.5,
        0.0,
        0.5,
        3.3770999999999999907629,
        0.0,
        1.5,
        0.5,
        0.0,
        0.5,
        0.5,
        0.0,
    ];
    optimal_pack_table(
        11,
        3.8770999999999999907629,
        &OPT_11,
        n,
        large_uv,
        aabbs,
        params,
        phis,
        extent,
    );
    const OPT_18: [f64; 54] = [
        2.4700161985907582717914,
        2.4335783708246112588824,
        0.42403103949074028022892,
        1.3528594569415370862941,
        2.3892972847076845432923,
        0.42403103949074028022892,
        2.0585783708246108147932,
        1.5221405430584633577951,
        0.42403103949074028022892,
        1.7642972847076845432923,
        3.3007351124738324443797,
        0.42403103949074028022892,
        3.3228756555322949139963,
        1.5,
        0.0,
        3.3228756555322949139963,
        3.3228756555322949139963,
        0.0,
        0.5,
        1.5,
        0.0,
        2.3228756555322949139963,
        4.3228756555322949139963,
        0.0,
        0.5,
        3.3228756555322949139963,
        0.0,
        1.5,
        0.5,
        0.0,
        3.3228756555322949139963,
        0.5,
        0.0,
        3.3228756555322949139963,
        4.3228756555322949139963,
        0.0,
        4.3228756555322949139963,
        1.5,
        0.0,
        4.3228756555322949139963,
        3.3228756555322949139963,
        0.0,
        0.5,
        0.5,
        0.0,
        0.5,
        4.3228756555322949139963,
        0.0,
        4.3228756555322949139963,
        0.5,
        0.0,
        4.3228756555322949139963,
        4.3228756555322949139963,
        0.0,
    ];
    optimal_pack_table(
        18,
        4.8228756555322949139963,
        &OPT_18,
        n,
        large_uv,
        aabbs,
        params,
        phis,
        extent,
    );
    const OPT_19: [f64; 57] = [
        2.1785113019775792508881,
        1.9428090415820631342569,
        0.78539816339744827899949,
        1.4714045207910317891731,
        2.6499158227686105959719,
        0.78539816339744827899949,
        2.9428090415820640224354,
        2.7071067811865479058042,
        0.78539816339744827899949,
        2.2357022603955165607204,
        3.4142135623730953675192,
        0.78539816339744827899949,
        1.4428090415820635783462,
        1.2642977396044836613243,
        0.78539816339744827899949,
        3.3856180831641271566923,
        1.5,
        0.0,
        0.73570226039551600560884,
        1.9714045207910311230393,
        0.78539816339744827899949,
        3.6213203435596432733234,
        3.4428090415820635783462,
        0.78539816339744827899949,
        2.9142135623730958116084,
        4.1499158227686105959719,
        0.78539816339744827899949,
        2.3856180831641271566923,
        0.5,
        0.0,
        0.5,
        3.3856180831641271566923,
        0.0,
        1.5,
        4.3856180831641271566923,
        0.0,
        4.3856180831641271566923,
        2.5,
        0.0,
        3.3856180831641271566923,
        0.5,
        0.0,
        4.3856180831641271566923,
        1.5,
        0.0,
        0.5,
        0.5,
        0.0,
        0.5,
        4.3856180831641271566923,
        0.0,
        4.3856180831641271566923,
        0.5,
        0.0,
        4.3856180831641271566923,
        4.3856180831641271566923,
        0.0,
    ];
    optimal_pack_table(
        19,
        4.8856180831641271566923,
        &OPT_19,
        n,
        large_uv,
        aabbs,
        params,
        phis,
        extent,
    );
    const OPT_26: [f64; 78] = [
        2.3106601717798209705279,
        2.8106601717798214146171,
        0.78539816339744827899949,
        1.6035533905932735088129,
        2.1035533905932739529021,
        0.78539816339744827899949,
        3.0177669529663684322429,
        2.1035533905932739529021,
        0.78539816339744827899949,
        2.3106601717798209705279,
        1.3964466094067264911871,
        0.78539816339744827899949,
        1.6035533905932735088129,
        3.5177669529663688763321,
        0.78539816339744827899949,
        0.89644660940672593607559,
        2.8106601717798214146171,
        0.78539816339744827899949,
        3.0177669529663684322429,
        3.5177669529663688763321,
        0.78539816339744827899949,
        3.7248737341529158939579,
        2.8106601717798214146171,
        0.78539816339744827899949,
        2.3106601717798209705279,
        4.2248737341529167821363,
        0.78539816339744827899949,
        0.5,
        1.5,
        0.0,
        1.5,
        0.5,
        0.0,
        3.1213203435596419410558,
        0.5,
        0.0,
        4.1213203435596419410558,
        1.5,
        0.0,
        0.5,
        4.1213203435596419410558,
        0.0,
        0.5,
        0.5,
        0.0,
        4.1213203435596419410558,
        4.1213203435596419410558,
        0.0,
        4.1213203435596419410558,
        0.5,
        0.0,
        1.5,
        5.1213203435596419410558,
        0.0,
        3.1213203435596419410558,
        5.1213203435596419410558,
        0.0,
        5.1213203435596419410558,
        2.5,
        0.0,
        5.1213203435596419410558,
        1.5,
        0.0,
        0.5,
        5.1213203435596419410558,
        0.0,
        4.1213203435596419410558,
        5.1213203435596419410558,
        0.0,
        5.1213203435596419410558,
        4.1213203435596419410558,
        0.0,
        5.1213203435596419410558,
        0.5,
        0.0,
        5.1213203435596419410558,
        5.1213203435596419410558,
        0.0,
    ];
    optimal_pack_table(
        26,
        5.6213203435596419410558,
        &OPT_26,
        n,
        large_uv,
        aabbs,
        params,
        phis,
        extent,
    );

    n = match n {
        37 => 38,
        50 | 51 => 52,
        65 | 66 => 67,
        n => n,
    };
    for a in 1..20usize {
        let count = a * a + a + 3 + ((a - 1) as f64 * 2.0_f64.sqrt()).floor() as usize;
        if n == count {
            let max_uv = large_uv * (a as f64 + 1.0 + 0.5_f64.sqrt());
            let e = Rect::new(0.0, max_uv, 0.0, max_uv);
            if is_larger(extent, &e, params) {
                *extent = e;
                pack_gobel(aabbs, large_uv, a, phis);
            }
            return;
        }
    }
}

fn pack_box_pack_2d(aabbs: &[Aabb], params: &PackParams, phis: &mut [Phi], extent: &mut Rect) {
    let mut boxes: Vec<BoxPack> = aabbs
        .iter()
        .map(|a| BoxPack {
            w: a.uv_diagonal.x / params.target_aspect_y,
            h: a.uv_diagonal.y,
            ..Default::default()
        })
        .collect();
    let (mut mu, mv) = box_pack_2d(&mut boxes);
    mu *= params.target_aspect_y;
    let e = Rect::new(0.0, mu, 0.0, mv);
    if is_larger(extent, &e, params) {
        *extent = e;
        for (a, b) in aabbs.iter().zip(&boxes) {
            let phi = &mut phis[a.index];
            phi.rotation = 0.0;
            phi.translation =
                DVec2::new((b.x + b.w * 0.5) * params.target_aspect_y, b.y + b.h * 0.5);
        }
    }
}

/// Bitmap of signed distances to the islands placed so far, for the xatlas
/// strategy.
struct Occupancy {
    radix: usize,
    scale_reciprocal: f64,
    bitmap: Vec<f32>,
    witness: std::cell::Cell<DVec2>,
    witness_distance: std::cell::Cell<f64>,
    triangle_hint: std::cell::Cell<usize>,
}

const TERMINAL: f64 = 1048576.0;

impl Occupancy {
    fn new(initial_scale: f64) -> Self {
        let radix = 800;
        let mut o = Occupancy {
            radix,
            scale_reciprocal: 1.0,
            bitmap: vec![TERMINAL as f32; radix * radix],
            witness: std::cell::Cell::new(DVec2::splat(-1.0)),
            witness_distance: std::cell::Cell::new(0.0),
            triangle_hint: std::cell::Cell::new(0),
        };
        o.increase_scale();
        o.scale_reciprocal = radix as f64 / initial_scale;
        o
    }

    fn increase_scale(&mut self) {
        self.scale_reciprocal *= 0.5;
        self.clear();
    }

    fn clear(&mut self) {
        self.bitmap.fill(TERMINAL as f32);
        self.witness.set(DVec2::splat(-1.0));
        self.witness_distance.set(0.0);
        self.triangle_hint.set(0);
    }

    fn trace_triangle(
        &mut self,
        uv0: DVec2,
        uv1: DVec2,
        uv2: DVec2,
        margin: f64,
        write: bool,
    ) -> f64 {
        let x0 = uv0.x.min(uv1.x).min(uv2.x);
        let y0 = uv0.y.min(uv1.y).min(uv2.y);
        let x1 = uv0.x.max(uv1.x).max(uv2.x);
        let y1 = uv0.y.max(uv1.y).max(uv2.y);
        let spread = if write { margin * 2.0 } else { 0.0 };
        let r = self.scale_reciprocal;
        let radix = self.radix as i64;
        let ix0 = (((x0 - spread) * r).floor() as i64).max(0);
        let iy0 = (((y0 - spread) * r).floor() as i64).max(0);
        let ix1 = (((x1 + spread) * r + 2.0).floor() as i64).min(radix);
        let iy1 = (((y1 + spread) * r + 2.0).floor() as i64).min(radix);
        let tri = FatTriangle::new(uv0 * r, uv1 * r, uv2 * r);
        let epsilon = 0.7071_f64.max(2.0 * margin * r);

        if !write {
            let w = self.witness.get();
            if (ix0 as f64) <= w.x && w.x < ix1 as f64 && (iy0 as f64) <= w.y && w.y < iy1 as f64 {
                let d = tri.distance(w);
                let extent = epsilon - d - self.witness_distance.get();
                if extent > -0.1 {
                    return extent.max(0.0);
                }
            }
        }
        if ix0 >= ix1 || iy0 >= iy1 {
            return -1.0;
        }
        let (x0, x1) = (ix0 as usize, ix1 as usize);
        if write {
            for y in (iy0..iy1).rev() {
                for x in (ix0..ix1).rev() {
                    let idx = (y * radix + x) as usize;
                    let d = tri.distance(DVec2::new(x as f64, y as f64));
                    self.bitmap[idx] = (d as f32).min(self.bitmap[idx]);
                }
            }
            return -1.0;
        }
        // Blender tests every occupied pixel of the triangle's bounding box.
        // The bitmap holds distances to the islands already placed, which
        // change by at most one per pixel, so a pixel near the triangle can
        // only be in conflict if one inside (or within `epsilon` of) the
        // triangle is too. Only that span of each row is tested, top-right
        // first as in Blender; this keeps the decisions and makes reads far
        // cheaper on big islands.
        let degenerate = tri.inv_len2.iter().any(|v| v.is_nan());
        for y in (iy0..iy1).rev() {
            let yf = y as f64;
            let (mut lo, mut hi) = (x0 as f64, (x1 - 1) as f64);
            if !degenerate {
                for k in 0..3 {
                    // Signed edge distance (positive outside) is a + b x.
                    let (e, p) = (tri.e[k], tri.p[k]);
                    let len = 1.0 / tri.inv_len2[k].sqrt();
                    let a = (e.x * (yf - p.y) + e.y * p.x) / len;
                    let bx = -e.y / len;
                    // Keep x where a + bx x <= epsilon.
                    if bx > 1e-12 {
                        hi = hi.min((epsilon - a) / bx);
                    } else if bx < -1e-12 {
                        lo = lo.max((epsilon - a) / bx);
                    } else if a > epsilon {
                        hi = lo - 1.0;
                    }
                }
            }
            if hi < lo {
                continue;
            }
            let (sx0, sx1) = (
                lo.ceil().max(x0 as f64) as usize,
                (hi.floor() as usize + 1).min(x1),
            );
            if sx0 >= sx1 {
                continue;
            }
            let base = (y * radix) as usize;
            let row = &self.bitmap[base + sx0..base + sx1];
            let mut end = row.len();
            while let Some(k) = row[..end].iter().rposition(|&h| (h as f64) <= epsilon) {
                end = k;
                let hot = row[k] as f64;
                let probe = DVec2::new((sx0 + k) as f64, yf);
                let d = tri.distance(probe);
                let extent = epsilon - d - hot;
                if extent > 0.0 {
                    self.witness.set(probe);
                    self.witness_distance.set(hot);
                    return extent;
                }
            }
        }
        -1.0
    }

    fn trace_island(
        &mut self,
        island: &PackIsland,
        phi: Phi,
        scale: f64,
        margin: f64,
        write: bool,
    ) -> f64 {
        let support = island.diagonal_support(scale, phi.rotation, margin);
        if !write && (phi.translation.x < support.x || phi.translation.y < support.y) {
            return TERMINAL;
        }
        let m = island.build_transformation(scale, phi.rotation);
        let delta = phi.translation - m.mul(island.pivot);
        let n = island.triangles.len();
        let mut i = 0;
        while i < n {
            let j = (i + self.triangle_hint.get()) % n;
            let a = m.mul(island.triangles[j]) + delta;
            let b = m.mul(island.triangles[j + 1]) + delta;
            let c = m.mul(island.triangles[j + 2]) + delta;
            let extent = self.trace_triangle(a, b, c, margin, write);
            if !write && extent >= 0.0 {
                self.triangle_hint.set(j);
                return extent;
            }
            i += 3;
        }
        -1.0
    }
}

/// A triangle prepared for many distance queries: the edge vectors and
/// their inverse squared lengths are worked out once instead of per pixel.
struct FatTriangle {
    p: [DVec2; 3],
    e: [DVec2; 3],
    inv_len2: [f64; 3],
}

impl FatTriangle {
    fn new(a: DVec2, b: DVec2, c: DVec2) -> Self {
        let p = [a, b, c];
        let e = [b - a, c - b, a - c];
        let inv_len2 = e.map(|e| {
            let l = e.length_squared();
            // Below the tolerance Blender falls back to the point distance.
            if l < 1e-40 { f64::NAN } else { 1.0 / l }
        });
        Self { p, e, inv_len2 }
    }

    /// `dist_signed_squared_to_edge` for edge `k`.
    fn edge_dist(&self, probe: DVec2, k: usize) -> f64 {
        let side = probe - self.p[k];
        if self.inv_len2[k].is_nan() {
            return side.length_squared();
        }
        let n = self.e[k].x * side.y - self.e[k].y * side.x;
        n * n.abs() * self.inv_len2[k]
    }

    /// `signed_distance_fat_triangle`: negative inside.
    fn distance(&self, probe: DVec2) -> f64 {
        let r = self
            .edge_dist(probe, 0)
            .max(self.edge_dist(probe, 1))
            .max(self.edge_dist(probe, 2));
        if r < 0.0 {
            return -(-r).sqrt();
        }
        let r = r
            .min((probe - self.p[0]).length_squared())
            .min((probe - self.p[1]).length_squared())
            .min((probe - self.p[2]).length_squared());
        r.sqrt()
    }
}

#[allow(clippy::too_many_arguments)]
fn find_best_fit(
    island: &PackIsland,
    scan_line: i64,
    occ: &mut Occupancy,
    scale: f64,
    angle_90_multiple: i32,
    margin: f64,
    target_aspect_y: f64,
) -> Phi {
    let bitmap_scale = 1.0 / occ.scale_reciprocal;
    let sq = target_aspect_y.sqrt();
    let scan_x = (scan_line as f64 * sq) as i64;
    let scan_y = (scan_line as f64 / sq) as i64;
    let mut phi = Phi {
        rotation: (angle_90_multiple as f64 * 90.0).to_radians(),
        ..Default::default()
    };
    if angle_90_multiple == 1 {
        phi.rotation = FRAC_PI_2;
    } else if angle_90_multiple == 2 {
        phi.rotation = PI;
    } else if angle_90_multiple == 3 {
        phi.rotation = 1.5 * PI;
    }
    let support = island.diagonal_support(scale, phi.rotation, 0.0);
    let mut t = ((2.0 * support.x + margin) * occ.scale_reciprocal).ceil() as i64;
    while t < scan_x {
        phi.translation =
            DVec2::new(t as f64 * bitmap_scale, scan_y as f64 * bitmap_scale) - support;
        let extent = occ.trace_island(island, phi, scale, margin, false);
        if extent < 0.0 {
            return phi;
        }
        t += (extent as i64).max(1);
    }
    let mut t = ((2.0 * support.y + margin) * occ.scale_reciprocal).ceil() as i64;
    while t <= scan_y {
        phi.translation =
            DVec2::new(scan_x as f64 * bitmap_scale, t as f64 * bitmap_scale) - support;
        let extent = occ.trace_island(island, phi, scale, margin, false);
        if extent < 0.0 {
            return phi;
        }
        t += (extent as i64).max(1);
    }
    Phi::default()
}

fn guess_initial_scale(islands: &[PackIsland], scale: f64, margin: f64) -> f64 {
    let mut sum = 1e-40;
    for i in islands {
        sum += i.half_diagonal.x * 2.0 * scale + 2.0 * margin;
        sum += i.half_diagonal.y * 2.0 * scale + 2.0 * margin;
    }
    sum.sqrt() / 6.0
}

struct SquareFinder<'a> {
    margin: f64,
    params: &'a PackParams,
    best_quad: f64,
    best_angle: f64,
    best_bounds: Rect,
    points: Vec<DVec2>,
    indices: Vec<usize>,
}

impl SquareFinder<'_> {
    fn update(&mut self, angle: f64) -> f64 {
        let dir = DVec2::new(angle.cos(), angle.sin());
        let mut b = Rect::new(f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &i in &self.indices {
            let p = self.points[i];
            b.include(DVec2::new(
                p.x * dir.x + p.y * dir.y,
                -p.x * dir.y + p.y * dir.x,
            ));
        }
        b.xmin -= self.margin;
        b.ymin -= self.margin;
        b.xmax += self.margin;
        b.ymax += self.margin;
        let q = aspect_extent(&b, self.params);
        if self.best_quad > q {
            self.best_quad = q;
            self.best_angle = angle;
            self.best_bounds = b;
        }
        q
    }

    fn update_recursive(&mut self, a0: f64, q0: f64, a1: f64, q1: f64) {
        let mid = (a0 + a1) * 0.5;
        let qmid = self.update(mid);
        let sep = a1 - a0;
        if sep < 0.002_f64.to_radians() {
            return;
        }
        let search = 10.0_f64.to_radians() < sep;
        if search || q0 <= q1 {
            self.update_recursive(a0, q0, mid, qmid);
        }
        if search || q1 <= q0 {
            self.update_recursive(mid, qmid, a1, q1);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rotate_inside_square(
    indices: &[Aabb],
    islands: &[PackIsland],
    params: &PackParams,
    scale: f64,
    margin: f64,
    phis: &mut [Phi],
    extent: &mut Rect,
) -> bool {
    if indices.is_empty() || params.rotate_method != RotationMethod::Any {
        return false;
    }
    if params.shape_method == ShapeMethod::Aabb && indices.len() > 1 {
        return false;
    }
    let mut f = SquareFinder {
        margin,
        params,
        best_quad: aspect_extent(extent, params) * 0.999,
        best_angle: 0.0,
        best_bounds: *extent,
        points: Vec::new(),
        indices: Vec::new(),
    };
    for a in indices {
        let island = &islands[a.index];
        if island.aspect_y != 1.0 {
            return false;
        }
        let s = if island.can_scale(params) { scale } else { 1.0 };
        let m = island.build_transformation(s, phis[a.index].rotation);
        let delta = phis[a.index].translation - m.mul(island.pivot);
        for &p in &island.triangles {
            f.points.push(m.mul(p) + delta);
        }
    }
    f.indices = geom::convex_hull(&f.points);
    let q180 = f.update(-PI);
    f.update_recursive(-PI, q180, PI, q180);
    if f.best_angle == 0.0 {
        return false;
    }
    let best_angle = f.best_angle;
    let best_bounds = f.best_bounds;
    for a in indices {
        let m = islands[a.index].build_transformation(1.0, best_angle);
        let phi = &mut phis[a.index];
        phi.rotation += best_angle;
        phi.translation = m.mul(phi.translation) - DVec2::new(best_bounds.xmin, best_bounds.ymin);
    }
    *extent = Rect::new(0.0, best_bounds.width(), 0.0, best_bounds.height());
    true
}

/// The xatlas-style brute force packer over a distance bitmap. Returns how
/// many islands it placed when its layout beat `extent`, otherwise 0.
fn pack_xatlas(
    indices: &[Aabb],
    islands: &[PackIsland],
    scale: f64,
    margin: f64,
    params: &PackParams,
    out_phis: &mut [Phi],
    out_extent: &mut Rect,
) -> usize {
    if params.shape_method == ShapeMethod::Aabb {
        return 0;
    }
    let mut phis = vec![Phi::default(); out_phis.len()];
    let mut occ = Occupancy::new(guess_initial_scale(islands, scale, margin));
    let mut extent = Rect::new(0.0, 0.0, 0.0, 0.0);
    let square_milestone = ((indices.len() as f64).sqrt() / 4.0) as usize + 2;
    let mut scan_line: i64 = 0;
    let mut traced = 0;
    let mut i = 0;
    let mut placed_can_rotate = true;
    let mut increase_remaining = 32;
    let scan_limit = occ.radix as f64
        * params
            .target_aspect_y
            .min(1.0 / params.target_aspect_y)
            .sqrt();

    while i < indices.len() {
        while traced < i {
            let idx = indices[traced].index;
            let island = &islands[idx];
            let s = if island.can_scale(params) { scale } else { 1.0 };
            occ.trace_island(island, phis[idx], s, margin, true);
            traced += 1;
        }
        let idx = indices[i].index;
        let island = &islands[idx];
        if !island.can_translate(params) {
            phis[idx] = Phi {
                translation: island.pivot - params.udim_base_offset,
                rotation: 0.0,
            };
            i += 1;
            placed_can_rotate = false;
            continue;
        }
        let s = if island.can_scale(params) { scale } else { 1.0 };
        let mut max_90 = 1;
        if island.can_rotate(params) {
            if i > 0 && i < 50 {
                max_90 = 4;
            }
        } else {
            placed_can_rotate = false;
        }
        let mut phi = Phi::default();
        for m in 0..max_90 {
            phi = find_best_fit(
                island,
                scan_line,
                &mut occ,
                s,
                m,
                margin,
                params.target_aspect_y,
            );
            if phi.is_valid() {
                break;
            }
        }
        if !phi.is_valid() {
            scan_line += if i < 10 { 1 } else { 2 };
            if (scan_line as f64) < scan_limit {
                continue;
            }
            increase_remaining -= 1;
            if increase_remaining < 0 {
                return 0;
            }
            scan_line = 0;
            occ.increase_scale();
            traced = 0;
            continue;
        }
        phis[idx] = phi;
        i += 1;
        if i == square_milestone
            && placed_can_rotate
            && rotate_inside_square(
                &indices[..i],
                islands,
                params,
                scale,
                margin,
                &mut phis,
                &mut extent,
            )
        {
            scan_line = 0;
            traced = 0;
            occ.clear();
            continue;
        }
        let top_right = island.diagonal_support(s, phi.rotation, margin) + phi.translation;
        extent.xmax = extent.xmax.max(top_right.x);
        extent.ymax = extent.ymax.max(top_right.y);
        if !is_larger(out_extent, &extent, params) && i >= square_milestone {
            return 0;
        }
        if i < 128 || (i & 31) == 16 {
            scan_line = 0;
        } else {
            scan_line = (scan_line - 25).max(0);
        }
    }
    if !is_larger(out_extent, &extent, params) {
        return 0;
    }
    *out_extent = extent;
    for a in &indices[..i] {
        out_phis[a.index] = phis[a.index];
    }
    i
}

/// Packs with every strategy at a fixed `scale` and `margin`; returns the
/// layout's size.
fn pack_scale_margin(
    islands: &[PackIsland],
    scale: f64,
    margin: f64,
    params: &PackParams,
    phis: &mut [Phi],
) -> f64 {
    let all_can_rotate = islands.iter().all(|i| i.can_rotate(params));
    let mut aabbs: Vec<Aabb> = islands
        .iter()
        .enumerate()
        .map(|(i, isl)| {
            let s = if isl.can_scale(params) { scale } else { 1.0 };
            Aabb {
                index: i,
                uv_diagonal: isl.half_diagonal * 2.0 * s + DVec2::splat(2.0 * margin),
                aspect_y: isl.aspect_y,
            }
        })
        .collect();
    // Biggest first, locked islands before everything.
    if all_can_rotate {
        aabbs.sort_by(|a, b| {
            let (ta, tb) = (
                islands[a.index].can_translate(params),
                islands[b.index].can_translate(params),
            );
            if ta != tb {
                return ta.cmp(&tb);
            }
            let (mut au, mut av) = (a.uv_diagonal.x * a.aspect_y, a.uv_diagonal.y);
            let (mut bu, mut bvv) = (b.uv_diagonal.x * b.aspect_y, b.uv_diagonal.y);
            if au > av {
                std::mem::swap(&mut au, &mut av);
            }
            if bu > bvv {
                std::mem::swap(&mut bu, &mut bvv);
            }
            let du = au - bu;
            let dv = (av - bvv) + du * 0.05;
            let a_first = if dv == 0.0 { du > 0.0 } else { dv > 0.0 };
            let b_first = {
                let du2 = bu - au;
                let dv2 = (bvv - av) + du2 * 0.05;
                if dv2 == 0.0 { du2 > 0.0 } else { dv2 > 0.0 }
            };
            match (a_first, b_first) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });
    } else {
        aabbs.sort_by(|a, b| {
            let (ta, tb) = (
                islands[a.index].can_translate(params),
                islands[b.index].can_translate(params),
            );
            if ta != tb {
                return ta.cmp(&tb);
            }
            let aa = a.uv_diagonal.x * a.uv_diagonal.y;
            let ba = b.uv_diagonal.x * b.uv_diagonal.y;
            ba.total_cmp(&aa)
        });
    }

    let mut locked_bounds = Rect::new(0.0, 0.0, 0.0, 0.0);
    let mut locked = 0;
    for (i, a) in aabbs.iter().enumerate() {
        let isl = &islands[a.index];
        if isl.can_translate(params) {
            break;
        }
        let bl = isl.pivot - isl.half_diagonal;
        let tr = isl.pivot + isl.half_diagonal;
        if i == 0 {
            locked_bounds = Rect::new(bl.x, tr.x, bl.y, tr.y);
        } else {
            locked_bounds.include(bl);
            locked_bounds.include(tr);
        }
        phis[a.index] = Phi {
            translation: isl.pivot - params.udim_base_offset,
            rotation: 0.0,
        };
        locked = i + 1;
    }

    let mut cutoff = 1024;
    if params.margin_method == MarginMethod::Fraction && margin > 0.0 {
        cutoff = 81;
    }
    cutoff = cutoff.max(locked);
    let mut slow = cutoff.min(islands.len());
    // The fast packer's layout is the first candidate.
    let mut extent = pack_fast(
        locked,
        &locked_bounds,
        &aabbs[..slow],
        all_can_rotate,
        params.target_aspect_y,
        phis,
    );
    let fast_extent = extent;
    if locked == 0 {
        optimal_pack(&aabbs[..slow], params, phis, &mut extent);
        pack_box_pack_2d(&aabbs[..slow], params, phis, &mut extent);
    }
    let max_xatlas = pack_xatlas(
        &aabbs[..slow],
        islands,
        scale,
        margin,
        params,
        phis,
        &mut extent,
    );
    if max_xatlas > 0 {
        slow = max_xatlas;
    }
    if all_can_rotate && aabbs.len() > slow {
        rotate_inside_square(
            &aabbs[..slow],
            islands,
            params,
            scale,
            margin,
            phis,
            &mut extent,
        );
    }
    if extent == fast_extent {
        slow = locked;
        extent = locked_bounds;
    }
    let final_extent = pack_fast(
        slow,
        &extent,
        &aabbs,
        all_can_rotate,
        params.target_aspect_y,
        phis,
    );
    aspect_extent(&final_extent, params)
}

/// Finds the largest scale that fits the islands into the target with a
/// margin that is a fraction of the result.
fn pack_margin_fraction(
    islands: &mut [PackIsland],
    margin_fraction: f64,
    rescale_margin: bool,
    params: &PackParams,
) -> f64 {
    let (mut scale_low, mut value_low, mut scale_high, mut value_high) = (0.0, 0.0, 0.0, 0.0);
    let n = islands.len();
    let mut phis_a = vec![Phi::default(); n];
    let mut phis_b = vec![Phi::default(); n];
    // Which buffer holds the best fitting layout: 0 none, 1 a, 2 b.
    let mut low: u8 = 0;
    let min_scale = 1e-5;
    for iteration in 0..25 {
        let mut scale = 1.0;
        if iteration == 0 {
        } else if scale_low == 0.0 {
            scale = scale_high * 0.1;
        } else if scale_high == 0.0 {
            scale = scale_low * 10.0;
        } else {
            if scale_high < scale_low * 1.0001 {
                break;
            }
            scale = (f64::sqrt(scale_low) * value_high - f64::sqrt(scale_high) * value_low)
                / (value_high - value_low);
            scale *= scale;
            if iteration & 1 == 1 {
                scale = (scale * (scale_low * scale_high).sqrt()).sqrt();
            }
        }
        scale = scale.max(min_scale);
        let target = if low == 1 { 2 } else { 1 };
        let margin = if rescale_margin {
            margin_fraction * scale
        } else {
            margin_fraction
        };
        let buf = if target == 1 {
            &mut phis_a
        } else {
            &mut phis_b
        };
        let max_uv = pack_scale_margin(islands, scale, margin, params, buf) / params.target_extent;
        let value = max_uv.sqrt() - 1.0;
        if value <= 0.0 {
            scale_low = scale;
            value_low = value;
            low = target;
            if value == 0.0 {
                break;
            }
        } else {
            scale_high = scale;
            value_high = value;
            if scale == min_scale {
                scale_low = scale;
                break;
            }
            if low == 0 {
                low = target;
            }
        }
    }
    if low != 0 {
        let phis = if low == 1 { &phis_a } else { &phis_b };
        for (i, island) in islands.iter_mut().enumerate() {
            let s = if island.can_scale(params) {
                scale_low
            } else {
                1.0
            };
            island.place(s, phis[i]);
        }
    }
    scale_low
}

fn margin_from_aabb_length_sum(islands: &[PackIsland], params: &PackParams) -> f64 {
    let sum: f64 = islands
        .iter()
        .map(|i| (i.half_diagonal.x * 2.0 * i.half_diagonal.y * 2.0).sqrt())
        .sum();
    params.margin * sum * 0.1
}

fn overlap_aabb(pa: DVec2, ha: DVec2, pb: DVec2, hb: DVec2) -> bool {
    !(pa.x + ha.x <= pb.x - hb.x
        || pa.y + ha.y <= pb.y - hb.y
        || pb.x + hb.x <= pa.x - ha.x
        || pb.y + hb.y <= pa.y - ha.y)
}

/// Strict 2D triangle overlap (`isect_tri_tri_v2`): shared edges or corners
/// alone don't count.
fn tri_tri_overlap(a: &[DVec2], b: &[DVec2]) -> bool {
    fn separated(p: &[DVec2], q: &[DVec2]) -> bool {
        for i in 0..3 {
            let e = p[(i + 1) % 3] - p[i];
            let n = DVec2::new(-e.y, e.x);
            let (mut pmin, mut pmax) = (f64::MAX, f64::MIN);
            for v in p {
                let d = n.dot(*v - p[i]);
                pmin = pmin.min(d);
                pmax = pmax.max(d);
            }
            let (mut qmin, mut qmax) = (f64::MAX, f64::MIN);
            for v in q {
                let d = n.dot(*v - p[i]);
                qmin = qmin.min(d);
                qmax = qmax.max(d);
            }
            let tol = 1e-12 * (1.0 + n.length_squared());
            if pmax <= qmin + tol || qmax <= pmin + tol {
                return true;
            }
        }
        false
    }
    !separated(a, b) && !separated(b, a)
}

fn islands_overlap(a: &PackIsland, b: &PackIsland) -> bool {
    if a.aspect_y != b.aspect_y {
        return false;
    }
    if !overlap_aabb(a.pivot, a.half_diagonal, b.pivot, b.half_diagonal) {
        return false;
    }
    a.triangles.as_chunks::<3>().0.iter().any(|ta| {
        b.triangles
            .as_chunks::<3>()
            .0
            .iter()
            .any(|tb| tri_tri_overlap(ta, tb))
    })
}

fn pack_overlap(islands: &mut [PackIsland], params: &PackParams) -> f64 {
    // Arena: inputs first, merged islands after.
    let mut arena: Vec<PackIsland> = islands.to_vec();
    for i in &mut arena {
        i.calculate_pivot();
    }
    let mut sub: Vec<usize> = Vec::new();
    let mut trace: Vec<[usize; 3]> = Vec::new();
    for i in 0..islands.len() {
        let mut cur = i;
        let mut j = sub.len();
        while j > 0 {
            j -= 1;
            if islands_overlap(&arena[cur], &arena[sub[j]]) {
                let (a, b) = (&arena[cur], &arena[sub[j]]);
                let mut merged = PackIsland {
                    aspect_y: (a.aspect_y * b.aspect_y).sqrt(),
                    pinned: a.pinned || b.pinned,
                    caller_index: usize::MAX,
                    ..Default::default()
                };
                for t in a
                    .triangles
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .chain(b.triangles.as_chunks::<3>().0.iter())
                {
                    merged.add_triangle(t[0], t[1], t[2]);
                }
                merged.calculate_pivot();
                arena.push(merged);
                let m = arena.len() - 1;
                trace.push([cur, sub[j], m]);
                cur = m;
                sub.remove(j);
            }
        }
        sub.push(cur);
    }
    let sub_params = PackParams {
        merge_overlap: false,
        ..*params
    };
    let mut packed: Vec<PackIsland> = sub.iter().map(|&i| arena[i].clone()).collect();
    let result = pack_islands(&mut packed, &sub_params);
    for (k, &i) in sub.iter().enumerate() {
        arena[i] = packed[k].clone();
    }
    for &[a, b, m] in trace.iter().rev() {
        let (angle, pre, rot, pinned) = (
            arena[m].angle,
            arena[m].pre_translate,
            arena[m].pre_rotate_sincos,
            arena[m].pinned,
        );
        for x in [a, b] {
            arena[x].angle = angle;
            arena[x].pre_translate = pre;
            arena[x].pre_rotate_sincos = rot;
            if pinned {
                arena[x].pinned = true;
            }
        }
    }
    for (i, island) in islands.iter_mut().enumerate() {
        island.angle = arena[i].angle;
        island.pre_translate = arena[i].pre_translate;
        island.pre_rotate_sincos = arena[i].pre_rotate_sincos;
        island.pinned = arena[i].pinned;
    }
    result
}

/// Packs `islands` and returns the scale islands that may scale must take
/// (pass it to [`PackIsland::transform`]). Each island's `angle` and
/// `pre_translate` are set.
pub fn pack_islands(islands: &mut [PackIsland], params: &PackParams) -> f64 {
    // Islands need a triangle for the maths to stay finite; an empty one
    // becomes a point that takes no room.
    for i in islands.iter_mut() {
        if i.triangles.is_empty() {
            i.triangles.extend([DVec2::ZERO; 3]);
        }
    }
    if islands.is_empty() {
        return 1.0;
    }
    if params.merge_overlap {
        return pack_overlap(islands, params);
    }
    for i in islands.iter_mut() {
        i.finalize_geometry(params);
    }
    let can_scale = islands.iter().filter(|i| i.can_scale(params)).count();
    let can_translate = islands.iter().filter(|i| i.can_translate(params)).count();
    if can_translate == 0 {
        return 1.0;
    }
    if params.margin_method == MarginMethod::Fraction && params.margin > 0.0 && can_scale > 0 {
        return pack_margin_fraction(islands, params.margin, false, params);
    }
    let margin = match params.margin_method {
        MarginMethod::Scaled => margin_from_aabb_length_sum(islands, params),
        _ => params.margin,
    };
    if can_scale > 0 && can_scale != islands.len() {
        return pack_margin_fraction(islands, margin, true, params);
    }
    let mut phis = vec![Phi::default(); islands.len()];
    let max_uv = pack_scale_margin(islands, 1.0, margin, params, &mut phis);
    let result = if can_scale > 0 && max_uv > 1e-14 {
        params.target_extent / max_uv
    } else {
        1.0
    };
    for (i, island) in islands.iter_mut().enumerate() {
        island.place(1.0, phis[i]);
    }
    result
}

/// Groups polygons into UV islands: polygons that share a point with the
/// same UV on both sides are connected. `faces` gives each polygon's
/// (point, uv) corners. Returns island ids per face and the island count.
pub fn uv_islands(faces: &[Vec<(u32, DVec2)>]) -> (Vec<usize>, usize) {
    let mut parent: Vec<usize> = (0..faces.len()).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    // Faces meeting at an edge with matching UVs at both ends join.
    // Edge -> (face, uv at the lower point, uv at the higher point).
    type EdgeUses = std::collections::HashMap<(u32, u32), Vec<(usize, DVec2, DVec2)>>;
    let mut edge_owner = EdgeUses::new();
    for (fi, f) in faces.iter().enumerate() {
        for i in 0..f.len() {
            let (a, ua) = f[i];
            let (b, ub) = f[(i + 1) % f.len()];
            let (key, u0, u1) = if a < b {
                ((a, b), ua, ub)
            } else {
                ((b, a), ub, ua)
            };
            edge_owner.entry(key).or_default().push((fi, u0, u1));
        }
    }
    let close = |a: DVec2, b: DVec2| (a - b).abs().max_element() <= 1e-6;
    for list in edge_owner.values() {
        for i in 0..list.len() {
            for j in i + 1..list.len() {
                let (fa, a0, a1) = list[i];
                let (fb, b0, b1) = list[j];
                if close(a0, b0) && close(a1, b1) {
                    let (ra, rb) = (find(&mut parent, fa), find(&mut parent, fb));
                    parent[ra] = rb;
                }
            }
        }
    }
    let mut ids = vec![usize::MAX; faces.len()];
    let mut map = std::collections::HashMap::new();
    for f in 0..faces.len() {
        let r = find(&mut parent, f);
        let n = map.len();
        ids[f] = *map.entry(r).or_insert(n);
    }
    (ids, map.len())
}

/// Packs polygons' UVs island by island. `movable` says which islands may
/// move (`None`: all); fixed islands keep their UVs and the moved ones pack
/// around them. Returns the new UVs per corner.
pub fn pack_uvs(
    faces: &[Vec<(u32, DVec2)>],
    islands: &[usize],
    island_count: usize,
    movable: Option<&[bool]>,
    params: &PackParams,
) -> Vec<Vec<DVec2>> {
    let mut pack: Vec<PackIsland> = (0..island_count)
        .map(|i| {
            let fixed = movable.is_some_and(|m| !m[i]);
            PackIsland::new(params.target_aspect_y, fixed, i)
        })
        .collect();
    for (fi, f) in faces.iter().enumerate() {
        let uvs: Vec<DVec2> = f.iter().map(|c| c.1).collect();
        pack[islands[fi]].add_polygon(&uvs);
    }
    let mut params = *params;
    if movable.is_some() {
        params.pin_method = PinMethod::LockAll;
    }
    let scale = pack_islands(&mut pack, &params);
    faces
        .iter()
        .enumerate()
        .map(|(fi, f)| {
            let isl = &pack[islands[fi]];
            if isl.pinned && params.pin_method == PinMethod::LockAll {
                return f.iter().map(|c| c.1).collect();
            }
            let s = if isl.can_scale(&params) { scale } else { 1.0 };
            f.iter()
                .map(|c| isl.transform(c.1, s, params.udim_base_offset))
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_faces(rects: &[(f64, f64)]) -> Vec<Vec<(u32, DVec2)>> {
        // Each rectangle is its own island at a random-ish place.
        rects
            .iter()
            .enumerate()
            .map(|(i, &(w, h))| {
                let o = DVec2::new(i as f64 * 3.1, -(i as f64) * 1.7);
                let p = 4 * i as u32;
                vec![
                    (p, o),
                    (p + 1, o + DVec2::new(w, 0.0)),
                    (p + 2, o + DVec2::new(w, h)),
                    (p + 3, o + DVec2::new(0.0, h)),
                ]
            })
            .collect()
    }

    fn check_layout(faces: &[Vec<DVec2>], margin: f64) {
        for f in faces {
            for p in f {
                assert!(
                    p.x >= -1e-6 && p.y >= -1e-6 && p.x <= 1.0 + 1e-6 && p.y <= 1.0 + 1e-6,
                    "{p}"
                );
            }
        }
        // Axis boxes of rotated-by-90 rectangles stay rectangles: check no
        // overlap of the polygons.
        let tris: Vec<Vec<DVec2>> = faces.to_vec();
        for i in 0..tris.len() {
            for j in 0..i {
                let a = [tris[i][0], tris[i][1], tris[i][2]];
                let a2 = [tris[i][0], tris[i][2], tris[i][3]];
                let b = [tris[j][0], tris[j][1], tris[j][2]];
                let b2 = [tris[j][0], tris[j][2], tris[j][3]];
                for x in [&a, &a2] {
                    for y in [&b, &b2] {
                        assert!(!tri_tri_overlap(x, y), "islands {i} and {j} overlap");
                    }
                }
            }
        }
        let _ = margin;
    }

    #[test]
    fn rectangles_fill_the_tile_without_overlap() {
        let rects = [
            (1.0, 0.5),
            (0.3, 0.3),
            (2.0, 0.2),
            (0.5, 0.5),
            (0.7, 1.2),
            (0.1, 0.1),
        ];
        let faces = rect_faces(&rects);
        let ids: Vec<usize> = (0..rects.len()).collect();
        for (rot, shape) in [
            (RotationMethod::None, ShapeMethod::Aabb),
            (RotationMethod::Any, ShapeMethod::Aabb),
            (RotationMethod::Cardinal, ShapeMethod::Concave),
            (RotationMethod::Any, ShapeMethod::Concave),
            (RotationMethod::Any, ShapeMethod::Convex),
        ] {
            let params = PackParams {
                rotate_method: rot,
                shape_method: shape,
                ..Default::default()
            };
            let out = pack_uvs(&faces, &ids, rects.len(), None, &params);
            check_layout(&out, 0.0);
            // Same scale for every island: areas keep their ratios.
            let area = |f: &[DVec2]| crate::tests::signed_area(f).abs();
            let k = area(&out[0]) / (rects[0].0 * rects[0].1);
            for (i, r) in rects.iter().enumerate() {
                assert!((area(&out[i]) / (r.0 * r.1) / k - 1.0).abs() < 1e-6);
            }
            // The layout reaches the far side in at least one direction.
            let mut hi = DVec2::ZERO;
            for p in out.iter().flatten() {
                hi = hi.max(*p);
            }
            assert!(hi.max_element() > 0.99, "{rot:?} {shape:?} {hi}");
        }
    }

    #[test]
    fn fixed_islands_stay_and_others_avoid_them() {
        let rects = [(1.0, 1.0), (1.0, 1.0), (1.0, 1.0)];
        let mut faces = rect_faces(&rects);
        // Island 0 sits in the tile's lower left quarter and is fixed.
        faces[0] = vec![
            (0, DVec2::new(0.0, 0.0)),
            (1, DVec2::new(0.5, 0.0)),
            (2, DVec2::new(0.5, 0.5)),
            (3, DVec2::new(0.0, 0.5)),
        ];
        let ids = [0, 1, 2];
        let movable = [false, true, true];
        let params = PackParams {
            shape_method: ShapeMethod::Concave,
            ..Default::default()
        };
        let out = pack_uvs(&faces, &ids, 3, Some(&movable), &params);
        assert_eq!(out[0], faces[0].iter().map(|c| c.1).collect::<Vec<_>>());
        check_layout(&out, 0.0);
    }

    #[test]
    fn islands_by_uv_connectivity() {
        // Two quads sharing points 1-2; same UVs there in the first case.
        let a = vec![
            (0, DVec2::new(0.0, 0.0)),
            (1, DVec2::new(1.0, 0.0)),
            (2, DVec2::new(1.0, 1.0)),
            (3, DVec2::new(0.0, 1.0)),
        ];
        let b = vec![
            (1, DVec2::new(1.0, 0.0)),
            (4, DVec2::new(2.0, 0.0)),
            (5, DVec2::new(2.0, 1.0)),
            (2, DVec2::new(1.0, 1.0)),
        ];
        assert_eq!(uv_islands(&[a.clone(), b.clone()]).1, 1);
        let mut b2 = b;
        b2[0].1 = DVec2::new(5.0, 0.0);
        assert_eq!(uv_islands(&[a, b2]).1, 2);
    }

    #[test]
    fn overlapping_islands_merge_when_asked() {
        let faces = vec![
            vec![
                (0, DVec2::new(0.0, 0.0)),
                (1, DVec2::new(1.0, 0.0)),
                (2, DVec2::new(0.0, 1.0)),
            ],
            vec![
                (3, DVec2::new(0.2, 0.2)),
                (4, DVec2::new(1.2, 0.2)),
                (5, DVec2::new(0.2, 1.2)),
            ],
        ];
        let params = PackParams {
            merge_overlap: true,
            ..Default::default()
        };
        let out = pack_uvs(&faces, &[0, 1], 2, None, &params);
        // Merged: the offset between them is kept, only scaled.
        let d0 = out[1][0] - out[0][0];
        let d1 = out[1][1] - out[0][1];
        assert!((d0 - d1).length() < 1e-9);
    }

    #[test]
    fn island_bounds_and_area_after_adding_polygon() {
        let mut island = PackIsland::new(1.0, false, 0);
        let polygon = [
            DVec2::new(-3.0, -2.0),
            DVec2::new(1.0, -2.0),
            DVec2::new(1.0, 4.0),
            DVec2::new(-3.0, 4.0),
        ];
        island.add_polygon(&polygon[..2]);
        assert!(island.is_empty());
        island.calculate_pivot();
        assert_eq!(island.pivot, DVec2::ZERO);
        assert_eq!(island.half_diagonal, DVec2::ZERO);

        island.add_polygon(&polygon);
        assert!(!island.is_empty());
        island.calculate_pivot();
        assert!((island.pivot - DVec2::new(-1.0, 1.0)).length() < 1e-12);
        assert!((island.half_diagonal - DVec2::new(2.0, 3.0)).length() < 1e-12);
        let (triangles, remainder) = island.triangles.as_chunks::<3>();
        assert!(remainder.is_empty());
        let area: f64 = triangles
            .iter()
            .map(|triangle| crate::tests::signed_area(triangle).abs())
            .sum();
        assert!((area - 24.0).abs() < 1e-12);
    }

    #[test]
    fn add_margin_separates_unscaled_rectangles_in_offset_tile() {
        let rects = [(0.25, 0.125), (0.125, 0.25)];
        let faces = rect_faces(&rects);
        let params = PackParams {
            scale_to_fit: false,
            margin: 0.05,
            margin_method: MarginMethod::Add,
            udim_base_offset: DVec2::new(3.0, 2.0),
            ..Default::default()
        };
        let out = pack_uvs(&faces, &[0, 1], 2, None, &params);
        let mut bounds = Vec::new();
        for (face, (width, height)) in out.iter().zip(rects) {
            let mut lo = DVec2::splat(f64::INFINITY);
            let mut hi = DVec2::splat(f64::NEG_INFINITY);
            for &uv in face {
                let local = uv - params.udim_base_offset;
                lo = lo.min(local);
                hi = hi.max(local);
            }
            assert!(lo.min_element() >= -1e-9 && hi.max_element() <= 1.0 + 1e-9);
            assert!((hi - lo - DVec2::new(width, height)).length() < 1e-9);
            assert!((crate::tests::signed_area(face) - width * height).abs() < 1e-9);
            bounds.push((lo, hi));
        }
        let gap = (bounds[0].0 - bounds[1].1).max(bounds[1].0 - bounds[0].1);
        assert!(gap.max_element() >= params.margin - 1e-9, "{gap}");
    }

    #[test]
    fn empty_input() {
        let out = pack_uvs(&[], &[], 0, None, &PackParams::default());
        assert!(out.is_empty());
        let mut none: Vec<PackIsland> = Vec::new();
        assert_eq!(pack_islands(&mut none, &PackParams::default()), 1.0);
    }
}
