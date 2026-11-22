//! Starter meshes: a handful of item presets built entirely by this file
//! for the Operandi Library pack (2026.11.5, Jason OK'd "a handful of our
//! own shapes"). Because nothing goes into them but this code, they are
//! our own work and the pack dedicates them to the public domain (CC0 1.0,
//! credit "Operandi contributors").
//!
//! Every shape is quads only, sized like the real thing in metres, standing
//! on the Y axis and resting on the ground (lowest point at
//! y = 0). Polygons carry the default material tag, so a dropped shape
//! takes the scene's base material, and a "Texture" UV map laid out at
//! real-world scale (one UV unit is one metre), so tiling materials keep
//! the same texel size on every shape. *Provisional*: real-world UVs
//! overlap where islands share coordinates; a shape meant for baking
//! would want a packed 0–1 layout (docs/SPECIFICATION_QUESTIONS.md LP-Q05).
//!
//! The pack's copies are written by
//! `operandi-app --write-starter-meshes <folder>`, which also renders each
//! thumbnail on the browser's preview stage; tests here keep the geometry
//! honest.
use super::{Preset, PresetCredit, PresetHeader, PresetKind, PresetSource};
use crate::mesh::Face;
use crate::primitive_uv::PRIMITIVE_UV_MAP;
use crate::{Mesh, Result, Scene};
use glam::{DVec2, DVec3};
use std::collections::HashMap;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

/// The licence the pack ships them under.
pub const LICENCE: &str = super::pack::CC0;
/// Their credit line.
pub const CREDIT: &str = "Operandi contributors";
/// Where they come from: this file.
pub const SOURCE_URL: &str = "https://github.com/operandi3d/Operandi/blob/main/crates/operandi-core/src/preset/starter_meshes.rs";

/// One starter mesh: its category folder, name and builder.
pub struct StarterMesh {
    pub category: &'static str,
    pub name: &'static str,
    build: fn() -> Result<Mesh>,
}

impl StarterMesh {
    /// The shape's mesh, resting on the ground.
    pub fn mesh(&self) -> Result<Mesh> {
        let mut mesh = (self.build)()?;
        rest_on_ground(&mut mesh);
        mesh.validate()?;
        Ok(mesh)
    }

    /// The shape as an Item preset, credited and licensed, with no
    /// thumbnail (the app renders one).
    pub fn preset(&self) -> Result<Preset> {
        let mut scene = Scene::default();
        let id = scene.next_id.max(1);
        scene.next_id = id + 1;
        scene
            .items
            .push(super::ball::mesh_item(id, self.name, self.mesh()?));
        scene.select_item(Some(id));
        let mut preset = PresetSource::Items.build(&scene)?.preset;
        preset.header = PresetHeader {
            kind: PresetKind::Item,
            thumbnail: None,
            source: Some(PresetCredit {
                licence: LICENCE.into(),
                url: SOURCE_URL.into(),
                credit: CREDIT.into(),
            }),
        };
        Ok(preset)
    }

    /// Where it goes below a pack's Meshes/ folder: `<Category>/<Name>.oppreset`.
    pub fn relative_path(&self) -> String {
        format!(
            "{}/{}.{}",
            self.category,
            self.name,
            super::PRESET_EXTENSION
        )
    }
}

/// Every starter mesh, in shelf order.
pub const ALL: [StarterMesh; 13] = [
    StarterMesh {
        category: "Primitives",
        name: "Rounded Cube",
        build: rounded_cube,
    },
    StarterMesh {
        category: "Primitives",
        name: "Sphere",
        build: sphere,
    },
    StarterMesh {
        category: "Primitives",
        name: "Cylinder",
        build: cylinder,
    },
    StarterMesh {
        category: "Primitives",
        name: "Cone",
        build: cone,
    },
    StarterMesh {
        category: "Primitives",
        name: "Torus",
        build: torus,
    },
    StarterMesh {
        category: "Primitives",
        name: "Capsule",
        build: capsule,
    },
    StarterMesh {
        category: "Primitives",
        name: "Tube",
        build: tube,
    },
    StarterMesh {
        category: "Primitives",
        name: "Subdivided Plane",
        build: plane,
    },
    StarterMesh {
        category: "Furniture",
        name: "Table",
        build: table,
    },
    StarterMesh {
        category: "Furniture",
        name: "Stool",
        build: stool,
    },
    StarterMesh {
        category: "Props",
        name: "Mug",
        build: mug,
    },
    StarterMesh {
        category: "Studio",
        name: "Stepped Plinth",
        build: stepped_plinth,
    },
    StarterMesh {
        category: "Studio",
        name: "Backdrop Sweep",
        build: backdrop_sweep,
    },
];

// --- The shapes ------------------------------------------------------------

/// 50 cm cube with 2.5 cm rounded edges.
fn rounded_cube() -> Result<Mesh> {
    let mut b = Builder::default();
    b.rounded_box(DVec3::ZERO, DVec3::splat(0.25), 0.025, 3, 0.075);
    Ok(b.finish())
}

/// 50 cm quad ball (a cube pushed out to a sphere: no poles).
fn sphere() -> Result<Mesh> {
    let mut b = Builder::default();
    b.rounded_box(DVec3::ZERO, DVec3::splat(0.25), 0.25, 8, 1.);
    Ok(b.finish())
}

/// 50 cm across, 50 cm tall, quad-gridded caps.
fn cylinder() -> Result<Mesh> {
    let mut b = Builder::default();
    let profile = column(&[(0., 0.), (0.25, 0.), (0.25, 0.5), (0., 0.5)], 0.05);
    b.lathe(&profile, 10, false);
    Ok(b.finish())
}

/// 50 cm across, 50 cm tall; the tip is a tiny 1 mm cap, so it stays quads.
fn cone() -> Result<Mesh> {
    let mut b = Builder::default();
    let mut profile = vec![(0., 0.), (0.25, 0.)];
    for i in 1..=8 {
        let t = f64::from(i) / 8.;
        profile.push((0.25 + (0.001 - 0.25) * t, 0.5 * t));
    }
    profile.push((0., 0.5));
    b.lathe(&profile, 10, false);
    Ok(b.finish())
}

/// A 50 cm ring, 12 cm thick, lying flat.
fn torus() -> Result<Mesh> {
    let mut b = Builder::default();
    b.lathe(&circle(0.19, 0.06, 0.06, 24), 10, true);
    Ok(b.finish())
}

/// 25 cm across, 75 cm tall, quads all over (a rounded box with fully
/// rounded sides).
fn capsule() -> Result<Mesh> {
    let mut b = Builder::default();
    b.rounded_box(
        DVec3::ZERO,
        DVec3::new(0.125, 0.375, 0.125),
        0.125,
        6,
        0.0625,
    );
    Ok(b.finish())
}

/// A 20 cm pipe, 1 cm wall, 60 cm tall.
fn tube() -> Result<Mesh> {
    let mut b = Builder::default();
    let mut profile = column(&[(0.09, 0.), (0.1, 0.), (0.1, 0.6), (0.09, 0.6)], 0.05);
    // Down the inside back to the start (the profile closes).
    for i in (1..12).rev() {
        profile.push((0.09, 0.6 * f64::from(i) / 12.));
    }
    b.lathe(&profile, 10, true);
    Ok(b.finish())
}

/// 2 m square floor tile, 20 by 20 quads.
fn plane() -> Result<Mesh> {
    let mut b = Builder::default();
    let n = 20;
    let at = |i: u32| -1. + 2. * f64::from(i) / f64::from(n);
    b.grid(
        n,
        n,
        |i, j| DVec3::new(at(i), 0., -at(j)),
        |i, j| DVec2::new(at(i) + 1., at(j) + 1.),
    );
    Ok(b.finish())
}

/// A 120 × 75 cm table, 75 cm tall: a 4 cm top on four square legs.
fn table() -> Result<Mesh> {
    let mut b = Builder::default();
    b.rounded_box(
        DVec3::new(0., 0.73, 0.),
        DVec3::new(0.6, 0.02, 0.375),
        0.006,
        2,
        0.1,
    );
    for (x, z) in [(1., 1.), (-1., 1.), (1., -1.), (-1., -1.)] {
        // The legs reach 5 mm into the top, so no faces lie on each other.
        b.rounded_box(
            DVec3::new(x * 0.52, 0.3575, z * 0.295),
            DVec3::new(0.025, 0.3575, 0.025),
            0.008,
            2,
            0.1,
        );
    }
    Ok(b.finish())
}

/// A 45 cm stool: a round seat with a rounded edge on four splayed legs
/// and a footrest ring.
fn stool() -> Result<Mesh> {
    let mut b = Builder::default();
    // Seat: 36 cm across, 3.5 cm thick, 8 mm round-over top and bottom.
    let (r, y0, y1, round) = (0.18, 0.42, 0.455, 0.008);
    let mut seat = vec![(0., y0), (r - round, y0)];
    seat.extend(arc((r - round, y0 + round), round, -FRAC_PI_2, 0., 3));
    seat.push((r, y0 + round + 0.5 * (y1 - y0 - 2. * round)));
    seat.extend(arc((r - round, y1 - round), round, 0., FRAC_PI_2, 3));
    seat.push((0., y1));
    b.lathe(&seat, 10, false);
    // Legs: 2.8 cm round, from 17 cm out on the floor to 12 cm under the
    // seat, ending 5 mm inside it.
    let (top_r, bottom_r, top_y) = (0.12, 0.17, 0.425);
    for k in 0..4 {
        let angle = FRAC_PI_4 + FRAC_PI_2 * f64::from(k);
        let dir = DVec3::new(angle.cos(), 0., angle.sin());
        let bottom = dir * bottom_r;
        let top = dir * top_r + DVec3::Y * top_y;
        let mut leg = Builder::default();
        let length = (top - bottom).length();
        let profile = column(
            &[(0., 0.), (0.014, 0.), (0.014, length), (0., length)],
            0.06,
        );
        leg.lathe(&profile, 3, false);
        let up = (top - bottom) / length;
        let rotation = glam::DQuat::from_rotation_arc(DVec3::Y, up);
        for v in &mut leg.vertices {
            *v = bottom + rotation * *v;
        }
        b.append(leg);
    }
    // Footrest: a 1 cm ring through the legs at 16 cm.
    let ring_y = 0.16;
    let ring_r = bottom_r + (top_r - bottom_r) * ring_y / top_y;
    let mut ring = Builder::default();
    ring.lathe(&circle(ring_r, ring_y, 0.01, 12), 12, true);
    b.append(ring);
    Ok(b.finish())
}

/// A 33 cl mug: 8.4 cm across, 9.5 cm tall, 4 mm wall, rounded rim and
/// foot, and a handle.
fn mug() -> Result<Mesh> {
    let mut b = Builder::default();
    let (r, h, wall, floor) = (0.042, 0.095, 0.004, 0.007);
    let foot = 0.003;
    let mut profile = vec![(0., 0.), (r - foot, 0.)];
    profile.extend(arc((r - foot, foot), foot, -FRAC_PI_2, 0., 3));
    let rim_y = h - wall / 2.;
    for i in 1..=8 {
        profile.push((r, foot + (rim_y - foot) * f64::from(i) / 8.));
    }
    // The rim: a half round over the wall.
    profile.extend(arc((r - wall / 2., rim_y), wall / 2., 0., PI, 6));
    let fillet = 0.006;
    let inner = r - wall;
    for i in 1..=8 {
        let y = rim_y + (floor + fillet - rim_y) * f64::from(i) / 8.;
        profile.push((inner, y));
    }
    profile.extend(arc(
        (inner - fillet, floor + fillet),
        fillet,
        0.,
        -FRAC_PI_2,
        4,
    ));
    profile.push((0., floor));
    b.lathe(&profile, 8, false);
    // The handle: an oval section swept round a C from the wall and back;
    // its open ends sit inside the wall.
    let steps = 24;
    let path: Vec<DVec3> = (0..=steps)
        .map(|i| {
            let s = PI * f64::from(i) / f64::from(steps);
            DVec3::new(r - 0.002 + 0.03 * s.sin(), 0.05 + 0.026 * s.cos(), 0.)
        })
        .collect();
    b.sweep(&path, 0.0065, 0.0042, 16);
    Ok(b.finish())
}

/// A round three-step display plinth, 1.2 m across and 30 cm tall, with
/// 5 mm bevels.
fn stepped_plinth() -> Result<Mesh> {
    let mut b = Builder::default();
    let bevel = 0.005;
    let mut profile = vec![(0., 0.)];
    let steps = [(0.6, 0.1), (0.45, 0.2), (0.3, 0.3)];
    let mut y = 0.;
    for (radius, top) in steps {
        profile.push((radius - bevel, y));
        profile.push((radius, y + bevel));
        for i in 1..4 {
            profile.push((
                radius,
                y + bevel + (top - y - 2. * bevel) * f64::from(i) / 4.,
            ));
        }
        profile.push((radius, top - bevel));
        profile.push((radius - bevel, top));
        y = top;
    }
    profile.push((0., y));
    b.lathe(&profile, 16, false);
    Ok(b.finish())
}

/// A photo backdrop: 3 m wide, a 1.5 m floor curving up through an 80 cm
/// radius into a 2.5 m wall. One-sided, facing the camera side (+Z).
fn backdrop_sweep() -> Result<Mesh> {
    let mut b = Builder::default();
    let radius = 0.8;
    // (y, z) along the profile, from the front edge of the floor.
    let mut profile = Vec::new();
    for i in 0..8 {
        profile.push((0., 1.5 - (1.5) * f64::from(i) / 8.));
    }
    for i in 0..12 {
        let a = FRAC_PI_2 * f64::from(i) / 12.;
        profile.push((radius - radius * a.cos(), -radius * a.sin()));
    }
    for i in 0..=10 {
        profile.push((radius + (2.5 - radius) * f64::from(i) / 10., -radius));
    }
    let mut along = vec![0.];
    for w in profile.windows(2) {
        let d = DVec2::new(w[1].0 - w[0].0, w[1].1 - w[0].1).length();
        along.push(along.last().unwrap() + d);
    }
    let n = 12;
    let x = |i: u32| -1.5 + 3. * f64::from(i) / f64::from(n);
    let rows = profile.len() as u32 - 1;
    b.grid(
        n,
        rows,
        |i, j| {
            let (py, pz) = profile[j as usize];
            DVec3::new(x(i), py, pz)
        },
        |i, j| DVec2::new(x(i) + 1.5, along[j as usize]),
    );
    Ok(b.finish())
}

// --- Profiles --------------------------------------------------------------

/// A lathe profile through `corners`, with straight runs split into pieces
/// no longer than `step`, skipping runs on the axis (r = 0).
fn column(corners: &[(f64, f64)], step: f64) -> Vec<(f64, f64)> {
    let mut out = vec![corners[0]];
    for w in corners.windows(2) {
        let (a, b) = (w[0], w[1]);
        let length = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let pieces = if a.0 == 0. || b.0 == 0. {
            1
        } else {
            (length / step).ceil().max(1.) as u32
        };
        for i in 1..=pieces {
            let t = f64::from(i) / f64::from(pieces);
            out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
        }
    }
    out
}

/// Points on an arc round `center` from angle `from` to `to` (radians, in
/// the (r, y) plane), leaving out the start point.
fn arc(center: (f64, f64), radius: f64, from: f64, to: f64, pieces: u32) -> Vec<(f64, f64)> {
    (1..=pieces)
        .map(|i| {
            let a = from + (to - from) * f64::from(i) / f64::from(pieces);
            (center.0 + radius * a.cos(), center.1 + radius * a.sin())
        })
        .collect()
}

/// A closed circular profile of `radius` round (r, y) = (`center_r`, `y`),
/// running so the lathe faces outward.
fn circle(center_r: f64, y: f64, radius: f64, pieces: u32) -> Vec<(f64, f64)> {
    (0..pieces)
        .map(|i| {
            // Counter-clockwise in (r, y) from the bottom.
            let a = -FRAC_PI_2 + TAU * f64::from(i) / f64::from(pieces);
            (center_r + radius * a.cos(), y + radius * a.sin())
        })
        .collect()
}

/// Lift `mesh` so its lowest point sits at y = 0. (The shapes are built
/// round the Y axis already; the mug's body stays on it, not its bounds.)
fn rest_on_ground(mesh: &mut Mesh) {
    let low = mesh
        .vertices
        .iter()
        .map(|v| v.y)
        .fold(f64::INFINITY, f64::min);
    for v in &mut mesh.vertices {
        v.y -= low;
        // Keep the files tidy: no -0 or 1e-17 noise.
        *v = (*v * 1e9).round() / 1e9;
    }
}

// --- Building --------------------------------------------------------------

/// Points and quads with their UVs; faces are counter-clockwise seen from
/// outside.
#[derive(Default)]
struct Builder {
    vertices: Vec<DVec3>,
    faces: Vec<[u32; 4]>,
    uvs: Vec<[DVec2; 4]>,
}

impl Builder {
    fn point(&mut self, p: DVec3) -> u32 {
        self.vertices.push(p);
        (self.vertices.len() - 1) as u32
    }

    fn quad(&mut self, v: [u32; 4], uv: [DVec2; 4]) {
        self.faces.push(v);
        self.uvs.push(uv);
    }

    fn append(&mut self, other: Builder) {
        let base = self.vertices.len() as u32;
        self.vertices.extend(other.vertices);
        self.faces
            .extend(other.faces.into_iter().map(|f| f.map(|v| v + base)));
        self.uvs.extend(other.uvs);
    }

    fn finish(self) -> Mesh {
        let faces = self
            .faces
            .iter()
            .map(|f| Face {
                vertices: f.to_vec(),
                uv: None,
                material: crate::material::DEFAULT_MATERIAL.into(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect();
        let uvs = self
            .uvs
            .iter()
            .map(|uv| uv.iter().map(|&p| Some(p)).collect())
            .collect();
        let mut mesh = Mesh {
            vertices: self.vertices,
            faces,
            ..Mesh::default()
        };
        mesh.uv_maps.insert(PRIMITIVE_UV_MAP.into(), uvs);
        mesh
    }

    /// An open (n + 1) × (m + 1) grid of points `at(i, j)`, quads facing
    /// the side from which i runs right and j runs up.
    fn grid(
        &mut self,
        n: u32,
        m: u32,
        at: impl Fn(u32, u32) -> DVec3,
        uv: impl Fn(u32, u32) -> DVec2,
    ) {
        let mut ids = Vec::new();
        for j in 0..=m {
            for i in 0..=n {
                ids.push(self.point(at(i, j)));
            }
        }
        let id = |i: u32, j: u32| ids[(j * (n + 1) + i) as usize];
        for j in 0..m {
            for i in 0..n {
                self.quad(
                    [id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1)],
                    [uv(i, j), uv(i + 1, j), uv(i + 1, j + 1), uv(i, j + 1)],
                );
            }
        }
    }

    /// A box of half-size `half` round `center` with edges rounded to
    /// `radius` (at most the smallest half-size; equal to every half-size
    /// it's a sphere). `steps` quads per 45° of rounding; flat runs are cut
    /// into pieces no longer than `piece`. Quads only; real-world UVs per
    /// side.
    fn rounded_box(&mut self, center: DVec3, half: DVec3, radius: f64, steps: u32, piece: f64) {
        let r = radius.min(half.min_element());
        // Box-surface parameters per axis, as arc lengths: the rounded zone
        // at each end, the flat run between.
        let axis_params = |h: f64| -> Vec<f64> {
            let inner = h - r;
            let mut out = Vec::new();
            for j in (1..=steps).rev() {
                out.push(-(inner + r * FRAC_PI_4 * f64::from(j) / f64::from(steps)));
            }
            if inner > 1e-9 {
                let pieces = (2. * inner / piece).ceil().max(1.) as u32;
                for i in 0..=pieces {
                    out.push(-inner + 2. * inner * f64::from(i) / f64::from(pieces));
                }
            } else {
                out.push(0.);
            }
            for j in 1..=steps {
                out.push(inner + r * FRAC_PI_4 * f64::from(j) / f64::from(steps));
            }
            out
        };
        let params = [
            axis_params(half.x),
            axis_params(half.y),
            axis_params(half.z),
        ];
        // A parameter in the rounded zone maps to the cube-sphere coordinate
        // (h - r) + r·tan(s / r), which spaces the rounding's quads evenly
        // by angle; then the point is pushed out to distance r from the
        // box's flat core.
        let place = |s: DVec3| -> DVec3 {
            let mut p = DVec3::ZERO;
            for a in 0..3 {
                let flat_half = half[a] - r;
                let excess = s[a].abs() - flat_half;
                p[a] = if excess > 0. {
                    s[a].signum() * (flat_half + r * (excess / r).tan())
                } else {
                    s[a]
                };
            }
            let inner = p.clamp(DVec3::splat(r) - half, half - DVec3::splat(r));
            let d = p - inner;
            let len = d.length();
            if len > 1e-12 {
                center + inner + d / len * r
            } else {
                center + p
            }
        };
        // (normal axis, sign, u axis, u sign, v axis, v sign): u right and
        // v up seen from outside.
        let sides: [(usize, f64, usize, f64, usize, f64); 6] = [
            (0, 1., 2, -1., 1, 1.),
            (0, -1., 2, 1., 1, 1.),
            (1, 1., 0, 1., 2, -1.),
            (1, -1., 0, 1., 2, 1.),
            (2, 1., 0, 1., 1, 1.),
            (2, -1., 0, -1., 1, 1.),
        ];
        let mut shared: HashMap<[usize; 3], u32> = HashMap::new();
        for (n_axis, n_sign, u_axis, u_sign, v_axis, v_sign) in sides {
            let count = |a: usize| params[a].len();
            let index = |a: usize, sign: f64, k: usize| {
                if sign > 0. { k } else { count(a) - 1 - k }
            };
            let n_index = if n_sign > 0. { count(n_axis) - 1 } else { 0 };
            let mut id = |this: &mut Self, i: usize, j: usize| -> (u32, DVec2) {
                let mut key = [0; 3];
                key[n_axis] = n_index;
                key[u_axis] = index(u_axis, u_sign, i);
                key[v_axis] = index(v_axis, v_sign, j);
                let s = DVec3::new(params[0][key[0]], params[1][key[1]], params[2][key[2]]);
                let uv = DVec2::new(u_sign * s[u_axis], v_sign * s[v_axis]);
                let v = *shared.entry(key).or_insert_with(|| this.point(place(s)));
                (v, uv)
            };
            for j in 0..count(v_axis) - 1 {
                for i in 0..count(u_axis) - 1 {
                    let a = id(self, i, j);
                    let b = id(self, i + 1, j);
                    let c = id(self, i + 1, j + 1);
                    let d = id(self, i, j + 1);
                    self.quad([a.0, b.0, c.0, d.0], [a.1, b.1, c.1, d.1]);
                }
            }
        }
    }

    /// Spin a profile of (radius, y) points round the Y axis with 4·`n`
    /// sides. An open profile that starts or ends on the axis (r = 0) gets a
    /// quad-gridded cap there; `closed` joins the last point to the first.
    /// Quads only.
    fn lathe(&mut self, profile: &[(f64, f64)], n: u32, closed: bool) {
        let sides = 4 * n;
        let angle = |i: u32| -FRAC_PI_4 + TAU * f64::from(i) / f64::from(sides);
        let starts_on_axis = !closed && profile[0].0 == 0.;
        let ends_on_axis = !closed && profile[profile.len() - 1].0 == 0.;
        let first = usize::from(starts_on_axis);
        let last = profile.len() - 1 - usize::from(ends_on_axis);
        // Arc length along the profile for V.
        let mut along = vec![0.];
        for w in profile.windows(2) {
            let d = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
            along.push(along.last().unwrap() + d);
        }
        let rings: Vec<Vec<u32>> = (first..=last)
            .map(|k| {
                let (r, y) = profile[k];
                (0..sides)
                    .map(|i| self.point(DVec3::new(r * angle(i).cos(), y, r * angle(i).sin())))
                    .collect()
            })
            .collect();
        let ring_uv = |k: usize, i: u32| {
            let a = angle(i) + FRAC_PI_4;
            // U runs the other way round so the texture isn't mirrored.
            DVec2::new((TAU - a) * profile[k].0.max(1e-3), along[k])
        };
        let mut bands: Vec<(usize, usize)> = (0..rings.len() - 1).map(|k| (k, k + 1)).collect();
        if closed {
            bands.push((rings.len() - 1, 0));
        }
        for (lo, hi) in bands {
            let (pk_lo, pk_hi) = (first + lo, first + hi);
            for i in 0..sides {
                let next = (i + 1) % sides;
                let mut uv_hi = ring_uv(pk_hi, i);
                let mut uv_hi_next = ring_uv(pk_hi, i + 1);
                if closed && hi == 0 {
                    // The seam band: V carries on past the last point.
                    let total = along[profile.len() - 1]
                        + ((profile[0].0 - profile[profile.len() - 1].0).powi(2)
                            + (profile[0].1 - profile[profile.len() - 1].1).powi(2))
                        .sqrt();
                    uv_hi.y = total;
                    uv_hi_next.y = total;
                }
                self.quad(
                    [
                        rings[lo][i as usize],
                        rings[hi][i as usize],
                        rings[hi][next as usize],
                        rings[lo][next as usize],
                    ],
                    [ring_uv(pk_lo, i), uv_hi, uv_hi_next, ring_uv(pk_lo, i + 1)],
                );
            }
        }
        if starts_on_axis {
            let up = profile[1].0 < 0.;
            self.disc(&rings[0], n, profile[1], up);
        }
        if ends_on_axis {
            let k = profile.len() - 2;
            let up = profile[k].0 > 0.;
            self.disc(&rings[rings.len() - 1], n, profile[k], up);
        }
    }

    /// Fill a lathe ring of 4·`n` points with an `n` × `n` grid of quads
    /// facing up (+Y) or down. Grid points are a square pulled round to the
    /// circle at its edge; the edge points are the ring's own.
    fn disc(&mut self, ring: &[u32], n: u32, (radius, y): (f64, f64), up: bool) {
        let n = n as usize;
        // tan spacing puts the square's edge points at the ring's angles.
        let s = |a: usize| (-FRAC_PI_4 + FRAC_PI_2 * a as f64 / n as f64).tan();
        let mut ids = vec![vec![0u32; n + 1]; n + 1];
        for (a, row) in ids.iter_mut().enumerate() {
            for (b, slot) in row.iter_mut().enumerate() {
                *slot = if let Some(i) = ring_index(a, b, n) {
                    ring[i]
                } else {
                    let q = DVec2::new(s(a), s(b));
                    let m = q.x.abs().max(q.y.abs());
                    let round = if q.length() > 0. {
                        q * (m / q.length())
                    } else {
                        q
                    };
                    let p = (q + (round - q) * m) * radius;
                    self.point(DVec3::new(p.x, y, p.y))
                };
            }
        }
        let uv_of = |this: &Self, v: u32| {
            let p = this.vertices[v as usize];
            if up {
                DVec2::new(p.x, -p.z)
            } else {
                DVec2::new(p.x, p.z)
            }
        };
        for a in 0..n {
            for b in 0..n {
                // (x, z) counter-clockwise is clockwise seen from +Y.
                let mut q = [ids[a][b], ids[a + 1][b], ids[a + 1][b + 1], ids[a][b + 1]];
                if up {
                    q.reverse();
                }
                let uv = q.map(|v| uv_of(self, v));
                self.quad(q, uv);
            }
        }
    }

    /// Sweep an oval (half-width `width` across the path's plane, half-depth
    /// `depth` in it) along a path in the XY plane, ends left open.
    fn sweep(&mut self, path: &[DVec3], width: f64, depth: f64, sides: u32) {
        let mut along = vec![0.];
        for w in path.windows(2) {
            along.push(along.last().unwrap() + (w[1] - w[0]).length());
        }
        let rings: Vec<Vec<u32>> = (0..path.len())
            .map(|k| {
                let tangent = if k == 0 {
                    path[1] - path[0]
                } else if k == path.len() - 1 {
                    path[k] - path[k - 1]
                } else {
                    path[k + 1] - path[k - 1]
                }
                .normalize();
                let normal = tangent.cross(DVec3::Z);
                (0..sides)
                    .map(|i| {
                        let a = TAU * f64::from(i) / f64::from(sides);
                        self.point(path[k] + normal * depth * a.cos() + DVec3::Z * width * a.sin())
                    })
                    .collect()
            })
            .collect();
        let girth = PI * (width + depth);
        for k in 0..path.len() - 1 {
            for i in 0..sides {
                let next = (i + 1) % sides;
                let u = |i: u32| girth * f64::from(i) / f64::from(sides);
                self.quad(
                    [
                        rings[k][i as usize],
                        rings[k + 1][i as usize],
                        rings[k + 1][next as usize],
                        rings[k][next as usize],
                    ],
                    [
                        DVec2::new(u(i), along[k]),
                        DVec2::new(u(i), along[k + 1]),
                        DVec2::new(u(i + 1), along[k + 1]),
                        DVec2::new(u(i + 1), along[k]),
                    ],
                );
            }
        }
    }
}

/// Where grid point (a, b) of an `n` × `n` disc sits in its 4·`n` ring, if
/// it's on the edge. The ring starts at angle -45° (a = n, b = 0) and runs
/// counter-clockwise in (x, z).
fn ring_index(a: usize, b: usize, n: usize) -> Option<usize> {
    if a == n {
        Some(b)
    } else if b == n {
        Some(n + (n - a))
    } else if a == 0 {
        Some(2 * n + (n - b))
    } else if b == 0 {
        Some((3 * n + a) % (4 * n))
    } else {
        None
    }
}

#[cfg(test)]
#[path = "starter_meshes/tests.rs"]
mod tests;
