//! Poly Haul's modes (spec `poly-haul-modes-2026-11-06`). Offset, Extrude
//! and Scale act on the selected polygons; Bevel, Split, Tear and Slide on
//! the edge or point under the pointer when the drag starts; Remove on the
//! clicked polygon.
//!
//! * Offset: the selection moves along its normal (C1, `poly_haul_geometry`).
//! * Extrude: the same move with new side walls (C2).
//! * Bevel: the edge under the pointer is chamfered by Inset (C3).
//! * Scale: each selected polygon scales in its own plane about its centre
//!   (C4).
//! * Split: the quad under the pointer is cut across, parallel to the
//!   picked edge's neighbours, at Position along the picked edge (C5).
//! * Remove: the clicked polygon goes; its points stay while used (C7).
//! * Tear: the picked corner of the picked polygon gets its own point,
//!   moved by the drag (C8).
//! * Slide: the picked point slides along the incident edge best matching
//!   the drag, held exactly to it (decided sane rule for Q-PH-06).
//!
//! Add Loop is open (Q-PH-05) and not built.
use crate::{ExtrudeMode, ExtrudeUvs, Mesh, Result};
use glam::DVec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolyHaulMode {
    #[default]
    Offset,
    Extrude,
    Bevel,
    Scale,
    Split,
    Remove,
    Tear,
    Slide,
}

impl PolyHaulMode {
    pub const ALL: [Self; 8] = [
        Self::Offset,
        Self::Extrude,
        Self::Bevel,
        Self::Scale,
        Self::Split,
        Self::Remove,
        Self::Tear,
        Self::Slide,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Offset => "Offset",
            Self::Extrude => "Extrude",
            Self::Bevel => "Bevel",
            Self::Scale => "Scale",
            Self::Split => "Split",
            Self::Remove => "Remove",
            Self::Tear => "Tear",
            Self::Slide => "Slide",
        }
    }
    /// Whether the mode acts on the selection (else on the picked element).
    pub fn uses_selection(self) -> bool {
        matches!(self, Self::Offset | Self::Extrude | Self::Scale)
    }
}

/// The element under the pointer when the drag started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum HaulTarget {
    #[default]
    None,
    /// An edge, `a`→`b` as picked.
    Edge(u32, u32),
    /// A point, with the polygon it was picked on.
    Point {
        point: u32,
        polygon: u32,
    },
    Polygon(u32),
}

/// The drag's amounts; each mode reads its own.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HaulAmount {
    /// Offset and Extrude distance along the normal.
    pub offset: f64,
    /// Bevel inset.
    pub inset: f64,
    /// Scale factor in the polygon's plane.
    pub scale: f64,
    /// Split position along the picked edge, 0..1 from its first point.
    pub position: f64,
    /// Tear and Slide movement, in the mesh's space.
    pub vector: DVec3,
}

impl Default for HaulAmount {
    /// Settings after Reset (PH-D0): Offset 0, Scale 100 %, Position 50 %.
    fn default() -> Self {
        Self {
            offset: 0.,
            inset: 0.,
            scale: 1.,
            position: 0.5,
            vector: DVec3::ZERO,
        }
    }
}

/// The result and the polygon selection afterwards (`None`: unchanged).
pub struct HaulOutcome {
    pub mesh: Mesh,
    pub faces: Option<Vec<u32>>,
}

fn keep(mesh: Mesh) -> HaulOutcome {
    HaulOutcome { mesh, faces: None }
}

/// One Poly Haul result from `source`. Pure.
pub fn poly_haul_mode_geometry(
    source: &Mesh,
    selected: &[u32],
    mode: PolyHaulMode,
    target: HaulTarget,
    amount: HaulAmount,
) -> Result<HaulOutcome> {
    source.validate()?;
    let need_target = || "Poly Haul: start the drag on an element for this mode".to_string();
    match mode {
        PolyHaulMode::Offset => {
            crate::poly_haul_geometry(source, selected, amount.offset).map(keep)
        }
        PolyHaulMode::Extrude => {
            if selected.is_empty() {
                return Err("Poly Haul requires selected polygons".into());
            }
            if amount.offset == 0. {
                return Ok(keep(source.clone()));
            }
            // Along the selection's summed normal, like Offset.
            let normal: DVec3 = selected
                .iter()
                .map(|&f| source.normal(f as usize))
                .sum::<DVec3>()
                .try_normalize()
                .ok_or("Poly Haul: the selection has no normal")?;
            let r = crate::extrude_geometry(
                source,
                selected,
                normal * amount.offset,
                1,
                ExtrudeMode::Auto,
                ExtrudeUvs::default(),
                None,
            )?;
            Ok(HaulOutcome {
                mesh: r.mesh,
                faces: Some(r.caps),
            })
        }
        PolyHaulMode::Scale => {
            if selected.is_empty() {
                return Err("Poly Haul requires selected polygons".into());
            }
            if !amount.scale.is_finite() {
                return Err("Poly Haul Scale must be finite".into());
            }
            let mut out = source.clone();
            let mut moved = vec![false; source.vertices.len()];
            for &f in selected {
                let f = f as usize;
                if f >= source.faces.len() || source.is_polyline(f) {
                    return Err("Poly Haul supports surface polygons only".into());
                }
                let centre = source.center(f);
                let normal = source.normal(f).normalize_or_zero();
                for &v in &source.faces[f].vertices {
                    if moved[v as usize] {
                        continue;
                    }
                    moved[v as usize] = true;
                    let d = source.vertices[v as usize] - centre;
                    let along = normal * d.dot(normal);
                    out.vertices[v as usize] = centre + along + (d - along) * amount.scale;
                }
            }
            out.validate()?;
            Ok(keep(out))
        }
        PolyHaulMode::Bevel => {
            let HaulTarget::Edge(a, b) = target else {
                return Err(need_target());
            };
            if amount.inset == 0. {
                return Ok(keep(source.clone()));
            }
            let (mesh, _) =
                crate::edge_bevel_geometry(source, &[(a, b)], amount.inset, 1, 100., true)?;
            Ok(keep(mesh))
        }
        PolyHaulMode::Split => split(source, target, amount.position),
        PolyHaulMode::Remove => {
            let HaulTarget::Polygon(f) = target else {
                return Err(need_target());
            };
            if f as usize >= source.faces.len() {
                return Err("Poly Haul: that polygon no longer exists".into());
            }
            let mut out = source.clone();
            out.remove_faces(&[f as usize].into_iter().collect())?;
            out.validate()?;
            Ok(HaulOutcome {
                mesh: out,
                faces: Some(Vec::new()),
            })
        }
        PolyHaulMode::Tear => {
            let HaulTarget::Point { point, polygon } = target else {
                return Err(need_target());
            };
            let (mesh, _) =
                crate::vertex_tear_geometry(source, &[point], &[polygon], amount.vector)?;
            Ok(keep(mesh))
        }
        PolyHaulMode::Slide => {
            let point = match target {
                HaulTarget::Point { point, .. } => point,
                _ => return Err(need_target()),
            };
            let p = *source
                .vertices
                .get(point as usize)
                .ok_or("Poly Haul: that point no longer exists")?;
            // The incident edge best matching the drag; the point moves
            // along it only (Q-PH-06), never past its far end.
            let best = source
                .edges()
                .into_iter()
                .filter_map(|(a, b)| match point {
                    _ if a == point => Some(b),
                    _ if b == point => Some(a),
                    _ => None,
                })
                .map(|other| source.vertices[other as usize] - p)
                .max_by(|x, y| {
                    let score = |d: &DVec3| d.normalize_or_zero().dot(amount.vector);
                    score(x).total_cmp(&score(y))
                })
                .ok_or("Poly Haul: that point has no edges to slide along")?;
            let along = best.normalize_or_zero();
            let distance = along.dot(amount.vector).clamp(0., best.length());
            let mut out = source.clone();
            out.vertices[point as usize] = p + along * distance;
            for f in 0..out.faces.len() {
                if !out.is_polyline(f) && out.normal(f) == DVec3::ZERO {
                    out.attributes.degenerate_faces.insert(f as u32);
                }
            }
            out.validate()?;
            Ok(keep(out))
        }
    }
}

/// Split: cut the quad holding the picked edge `a`→`b` across it, from
/// `position` along `a`→`b` to the same share along the opposite edge.
fn split(source: &Mesh, target: HaulTarget, position: f64) -> Result<HaulOutcome> {
    let HaulTarget::Edge(a, b) = target else {
        return Err("Poly Haul: start the drag on an edge to split".into());
    };
    if !(position > 0. && position < 1.) {
        // At an end the cut would run along an existing edge: nothing.
        return Ok(HaulOutcome {
            mesh: source.clone(),
            faces: None,
        });
    }
    // The quad that has a→b (either winding).
    let (face, ia) = source
        .faces
        .iter()
        .enumerate()
        .filter(|(f, face)| !source.is_polyline(*f) && face.vertices.len() == 4)
        .find_map(|(f, face)| {
            let n = face.vertices.len();
            (0..n)
                .find(|&i| {
                    let (x, y) = (face.vertices[i], face.vertices[(i + 1) % n]);
                    (x, y) == (a, b) || (x, y) == (b, a)
                })
                .map(|i| (f, i))
        })
        .ok_or("Poly Haul Split needs an edge of a quad")?;
    let ring = source.faces[face].vertices.clone();
    // Opposite edge, walked the same way as the picked one.
    let (c, d) = (ring[(ia + 3) % 4], ring[(ia + 2) % 4]);
    // Points at `position` from the picked edge's first point, and the
    // same share along the side opposite it (c is next to the quad's
    // ring[ia], d next to ring[ia + 1]).
    let t = if ring[ia] == a {
        position
    } else {
        1. - position
    };
    let (m1, p) = crate::add_point(source, ring[ia], ring[(ia + 1) % 4], t)?;
    let (m2, q) = crate::add_point(&m1, c, d, t)?;
    let corners = &m2.faces[face].vertices;
    let cp = corners
        .iter()
        .position(|&v| v == p)
        .ok_or("Split lost its cut point")?;
    let cq = corners
        .iter()
        .position(|&v| v == q)
        .ok_or("Split lost its cut point")?;
    let mesh = crate::polygon_split::cut_face(&m2, face, cp, cq)?;
    Ok(HaulOutcome {
        mesh,
        faces: Some(Vec::new()),
    })
}
