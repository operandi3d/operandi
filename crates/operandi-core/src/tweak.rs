//! Tweak (Element Move, key T): grab one vertex, edge or polygon under the
//! pointer and drag it, without selecting it first.
//! Spec: `specifications/tweak-2026-11-03` (v1). Slices TWK-1 (pick and
//! drag) and TWK-2 (Mode, Range with the falloff shapes, Connected
//! Elements), plus Symmetry (provisional). No handle or snapping yet.
use crate::{Mesh, Result, Side, Symmetry};
use glam::DVec3;

/// The element a Tweak press grabbed, by the mesh's own indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TweakElement {
    Vertex(u32),
    /// An edge by its two point indices (either order).
    Edge(u32, u32),
    Polygon(u32),
}

/// What the current Tweak drag holds: the element on item `item`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TweakGrab {
    pub item: u64,
    pub element: TweakElement,
}

impl TweakElement {
    /// TWK-13: the points that move with the element (a vertex: itself; an
    /// edge: both ends; a polygon: all its corners), each once, ascending.
    pub fn points(self, mesh: &Mesh) -> Result<Vec<u32>> {
        let count = mesh.vertices.len() as u32;
        let mut points = match self {
            Self::Vertex(v) => vec![v],
            Self::Edge(a, b) => vec![a, b],
            Self::Polygon(f) => mesh
                .faces
                .get(f as usize)
                .ok_or("Tweak: that polygon no longer exists")?
                .vertices
                .clone(),
        };
        if points.iter().any(|&v| v >= count) {
            return Err("Tweak: that point no longer exists".into());
        }
        points.sort_unstable();
        points.dedup();
        Ok(points)
    }
}

/// TWK-13/14/25: move the grabbed element's points by `offset` (in the
/// mesh's own coordinates). Nothing else changes: no points or polygons are
/// added, removed or renumbered.
pub fn tweak_points(mesh: &mut Mesh, element: TweakElement, offset: DVec3) -> Result<()> {
    if !offset.is_finite() {
        return Err("Tweak offset must be finite".into());
    }
    for v in element.points(mesh)? {
        mesh.vertices[v as usize] += offset;
    }
    Ok(())
}

/// TWK-20 (provisional, TWK-Q13): what Symmetry adds to a tweak, resolved
/// once per drag from the mesh as it was at the press.
#[derive(Clone, Debug, PartialEq)]
pub struct TweakMirror {
    pub symmetry: Symmetry,
    /// The side the element is dragged on (its centre's side; an element
    /// centred on the plane counts as the positive side).
    pub primary: Side,
    /// "On the plane" tolerance of the mesh at the press.
    pub tolerance: f64,
    /// Each grabbed point with its mirror counterpart (itself on the plane).
    pub pairs: Vec<(u32, u32)>,
}

/// The mirror data for tweaking `element` of `mesh` with `symmetry`;
/// `None` when Symmetry is off. Counterparts are matched like Select
/// Symmetrical (nearest mirrored point, or through connectivity with
/// Topology on); a point without one moves alone.
pub fn tweak_mirror(
    mesh: &Mesh,
    element: TweakElement,
    symmetry: Symmetry,
) -> Result<Option<TweakMirror>> {
    tweak_mirror_points(mesh, &element.points(mesh)?, symmetry)
}

/// `tweak_mirror` for a set of grabbed points (an element, or an edge loop).
pub fn tweak_mirror_points(
    mesh: &Mesh,
    points: &[u32],
    symmetry: Symmetry,
) -> Result<Option<TweakMirror>> {
    if symmetry.axis.is_none() {
        return Ok(None);
    }
    let tolerance = crate::symmetry::match_tolerance(mesh);
    let topology = symmetry
        .topology
        .then(|| crate::symmetry::topology_counterparts(mesh, symmetry));
    let pairs = points
        .iter()
        .filter_map(|&v| {
            let counterpart = match &topology {
                Some(map) => map.get(v as usize).copied().flatten(),
                None => crate::symmetry::nearest(
                    mesh,
                    symmetry.mirror(mesh.vertices[v as usize]),
                    tolerance,
                ),
            };
            counterpart.map(|c| (v, c))
        })
        .collect();
    // ACS-01/ACS-03: the primary side every tool uses (the pivot
    // service's): the box centre of the grabbed half, a centre on the
    // plane counting as the − side.
    let half = crate::pivot_service::half_points(mesh, points, symmetry);
    let centre = crate::box_centre(
        half.iter()
            .filter_map(|&v| mesh.vertices.get(v as usize).copied()),
    );
    let primary = crate::pivot_service::primary_side(symmetry, centre, tolerance);
    Ok(Some(TweakMirror {
        symmetry,
        primary,
        tolerance,
        pairs,
    }))
}

/// `tweak_points` with Symmetry (TWK-20, provisional): points on the dragged
/// side move by `offset`, points on the other side (the grabbed element's
/// own, or the counterparts of its points) by the mirrored offset, and
/// points on the plane stay on it. `base` is the mesh at the press.
pub fn tweak_points_mirrored(
    mesh: &mut Mesh,
    base: &Mesh,
    element: TweakElement,
    offset: DVec3,
    mirror: &TweakMirror,
) -> Result<()> {
    let mut moves: Vec<(u32, f64)> = element.points(base)?.into_iter().map(|v| (v, 1.)).collect();
    moves.extend(mirror.pairs.iter().map(|&(_, c)| (c, 1.)));
    moves.sort_unstable_by_key(|m| m.0);
    moves.dedup_by_key(|m| m.0);
    apply_moves(mesh, base, &moves, offset, Some(mirror))
}

/// TWK-03 Mode: which kind of element a press can grab. The Center
/// versions put the grab point (the drag plane and, later, the handle) at
/// the element's center; the falloff is measured from the element's points
/// in every mode (TWK-12).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TweakMode {
    #[default]
    Auto,
    AutoCenter,
    Vertex,
    Edge,
    EdgeCenter,
    Polygon,
    PolygonCenter,
}
impl TweakMode {
    /// TWK-03: the seven entries, in the reference's order.
    pub const ALL: [Self; 7] = [
        Self::Auto,
        Self::AutoCenter,
        Self::Vertex,
        Self::Edge,
        Self::EdgeCenter,
        Self::Polygon,
        Self::PolygonCenter,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::AutoCenter => "Auto Center",
            Self::Vertex => "Vertex",
            Self::Edge => "Edge",
            Self::EdgeCenter => "Edge Center",
            Self::Polygon => "Polygon",
            Self::PolygonCenter => "Polygon Center",
        }
    }
    pub fn grabs_vertices(self) -> bool {
        matches!(self, Self::Auto | Self::AutoCenter | Self::Vertex)
    }
    pub fn grabs_edges(self) -> bool {
        matches!(
            self,
            Self::Auto | Self::AutoCenter | Self::Edge | Self::EdgeCenter
        )
    }
    pub fn grabs_polygons(self) -> bool {
        matches!(
            self,
            Self::Auto | Self::AutoCenter | Self::Polygon | Self::PolygonCenter
        )
    }
    /// The grab point is the element's center rather than the hit point.
    pub fn centered(self) -> bool {
        matches!(
            self,
            Self::AutoCenter | Self::EdgeCenter | Self::PolygonCenter
        )
    }
}

/// TWK-15/16 Connected Elements: which other geometry takes part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TweakConnect {
    /// Distance alone decides, across separate pieces.
    #[default]
    Ignore,
    /// Only the grabbed element's connected piece is weighted.
    Connectivity,
    /// The whole connected piece moves by the full offset; Range is unused.
    Rigid,
    /// An edge pick moves its whole edge loop (TWK-16).
    EdgeLoops,
}
impl TweakConnect {
    pub const ALL: [Self; 4] = [
        Self::Ignore,
        Self::Connectivity,
        Self::Rigid,
        Self::EdgeLoops,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Ignore => "Ignore",
            Self::Connectivity => "Use Connectivity",
            Self::Rigid => "Rigid Connections",
            Self::EdgeLoops => "Edge Loops",
        }
    }
    /// Rigid ignores Range (the reference greys the field, TWK-15).
    pub fn uses_range(self) -> bool {
        self != Self::Rigid
    }
}

/// The Tweak options that decide what moves (TWK-03, TWK-12, TWK-15/16).
/// Defaults: Auto (guide), Range 0, Ignore and Linear (the values seen in
/// the tested session; factory defaults unconfirmed, TWK-Q08).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TweakOptions {
    pub mode: TweakMode,
    /// Falloff radius in world metres; 0 moves only the grabbed element.
    pub range: f64,
    pub connect: TweakConnect,
    pub shape: crate::FalloffShape,
}
impl Default for TweakOptions {
    fn default() -> Self {
        Self {
            mode: TweakMode::Auto,
            range: 0.,
            connect: TweakConnect::Ignore,
            shape: crate::FalloffShape::Linear,
        }
    }
}

/// Everything one drag moves, resolved once at its first frame from the
/// press-time mesh: each moving point with its weight, plus the Symmetry
/// counterparts. Later frames only scale the offset by the stored weights.
#[derive(Clone, Debug, PartialEq)]
pub struct TweakPlan {
    pub grab: TweakGrab,
    pub options: TweakOptions,
    pub symmetry: Symmetry,
    pub mirror: Option<TweakMirror>,
    /// (point, weight) for every point that moves, ascending by point.
    pub moves: crate::Shared<Vec<(u32, f64)>>,
}

/// TWK-16 (observed for a G4 edge): the points of the edge loop through
/// edge (a, b). The loop runs straight through each point: from the edge
/// arriving at a point it continues along the one edge there that shares
/// no polygon with it, so it crosses quad grids and follows open borders.
/// It stops at a point where that edge isn't unique (a pole, a triangle
/// fan, a cube corner) and closes when it comes back to its start.
/// Provisional at poles and borders (TWK-Q11).
pub fn edge_loop_points(mesh: &Mesh, a: u32, b: u32) -> Vec<u32> {
    let faces = mesh.point_faces();
    // The points joined to `v` by a polygon side.
    let neighbours = |v: u32| {
        let mut out: Vec<u32> = Vec::new();
        for &f in faces.of(v) {
            let points = &mesh.faces[f as usize].vertices;
            let n = points.len();
            for i in 0..n {
                if points[i] == v {
                    for u in [points[(i + n - 1) % n], points[(i + 1) % n]] {
                        if u != v && !out.contains(&u) {
                            out.push(u);
                        }
                    }
                }
            }
        }
        out
    };
    // Whether sides v–x and v–y belong to one polygon.
    let share = |v: u32, x: u32, y: u32| {
        faces.of(v).iter().any(|&f| {
            let points = &mesh.faces[f as usize].vertices;
            let n = points.len();
            (0..n).any(|i| {
                points[i] == v && {
                    let (p, q) = (points[(i + n - 1) % n], points[(i + 1) % n]);
                    (p == x && q == y) || (p == y && q == x)
                }
            })
        })
    };
    let mut seen = vec![a, b];
    for (from, start) in [(a, b), (b, a)] {
        let (mut prev, mut at) = (from, start);
        loop {
            let next: Vec<u32> = neighbours(at)
                .into_iter()
                .filter(|&u| u != prev && !share(at, prev, u))
                .collect();
            let [next] = next[..] else { break };
            if seen.contains(&next) {
                break;
            }
            seen.push(next);
            (prev, at) = (at, next);
        }
    }
    seen.sort_unstable();
    seen
}

/// Flags every point on the same connected pieces as `seeds` (points joined
/// through polygons).
pub(crate) fn connected_points_of(mesh: &Mesh, seeds: &[u32]) -> Vec<bool> {
    connected_points(mesh, seeds)
}

fn connected_points(mesh: &Mesh, seeds: &[u32]) -> Vec<bool> {
    let faces = mesh.point_faces();
    let mut point_seen = vec![false; mesh.vertices.len()];
    let mut face_seen = vec![false; mesh.faces.len()];
    let mut pending: Vec<u32> = Vec::new();
    for &v in seeds {
        if !point_seen[v as usize] {
            point_seen[v as usize] = true;
            pending.push(v);
        }
    }
    while let Some(v) = pending.pop() {
        for &f in faces.of(v) {
            if std::mem::replace(&mut face_seen[f as usize], true) {
                continue;
            }
            for &u in &mesh.faces[f as usize].vertices {
                if !std::mem::replace(&mut point_seen[u as usize], true) {
                    pending.push(u);
                }
            }
        }
    }
    point_seen
}

/// TWK-12/13/15/16: what a tweak of `grab` moves on `mesh` (the press-time
/// mesh; `world` is its item's world matrix, since Range is a world
/// distance).
///
/// The grabbed element's points (with Edge Loops and an edge pick, the
/// loop's; with Symmetry, also their counterparts) get weight 1. With
/// Range > 0 every other point gets the Shape's weight of d / Range, d
/// being its straight 3D distance to the nearest of those points, and
/// moves only when that weight is above 0. Use Connectivity limits that to
/// the grabbed pieces; Rigid moves the whole pieces at weight 1.
pub fn tweak_plan(
    mesh: &Mesh,
    world: glam::DMat4,
    grab: TweakGrab,
    options: TweakOptions,
    symmetry: Symmetry,
) -> Result<TweakPlan> {
    tweak_plan_with(mesh, world, grab, options, symmetry, None)
}

/// Flags per point of the pieces joined to the seed points.
pub(crate) type PieceFinder<'a> = &'a dyn Fn(&[u32]) -> Vec<bool>;

/// [`tweak_plan`] with the pieces Use Connectivity and Rigid Connections
/// keep to worked out by `pieces` (flags per point, from the seed points)
/// instead of through `mesh`'s polygons. A UV tool uses it to read
/// connectivity from the 3D mesh (`uv_pipe::CONNECTIVITY`).
pub(crate) fn tweak_plan_with(
    mesh: &Mesh,
    world: glam::DMat4,
    grab: TweakGrab,
    options: TweakOptions,
    symmetry: Symmetry,
    pieces: Option<PieceFinder<'_>>,
) -> Result<TweakPlan> {
    if !(options.range.is_finite() && options.range >= 0.) {
        return Err("Tweak Range must be zero or more".into());
    }
    let mut seeds = grab.element.points(mesh)?;
    if options.connect == TweakConnect::EdgeLoops
        && let TweakElement::Edge(a, b) = grab.element
    {
        seeds = edge_loop_points(mesh, a, b);
    }
    let mirror = tweak_mirror_points(mesh, &seeds, symmetry)?;
    let mut measured = seeds.clone();
    if let Some(m) = &mirror {
        measured.extend(m.pairs.iter().map(|&(_, c)| c));
        measured.sort_unstable();
        measured.dedup();
    }
    let pieces = matches!(
        options.connect,
        TweakConnect::Connectivity | TweakConnect::Rigid
    )
    .then(|| match pieces {
        Some(pieces) => pieces(&measured),
        None => connected_points(mesh, &measured),
    });
    let mut moves: Vec<(u32, f64)> = Vec::new();
    if options.connect == TweakConnect::Rigid {
        let pieces = pieces.as_deref().unwrap_or_default();
        moves.extend(
            pieces
                .iter()
                .enumerate()
                .filter(|&(_, &on)| on)
                .map(|(v, _)| (v as u32, 1.)),
        );
    } else {
        let mut is_seed = vec![false; mesh.vertices.len()];
        for &v in &measured {
            is_seed[v as usize] = true;
            moves.push((v, 1.));
        }
        let range = options.range;
        if range > 0. {
            let centers: Vec<DVec3> = measured
                .iter()
                .map(|&v| world.transform_point3(mesh.vertices[v as usize]))
                .collect();
            // A box around the measured points, grown by Range, rejects
            // most far points before the per-point nearest search.
            let (lo, hi) = centers.iter().fold(
                (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
                |(lo, hi), &c| (lo.min(c), hi.max(c)),
            );
            let (lo, hi) = (lo - DVec3::splat(range), hi + DVec3::splat(range));
            for (v, &local) in mesh.vertices.iter().enumerate() {
                if is_seed[v] || pieces.as_ref().is_some_and(|p| !p[v]) {
                    continue;
                }
                let p = world.transform_point3(local);
                if p.cmplt(lo).any() || p.cmpgt(hi).any() {
                    continue;
                }
                let d = centers
                    .iter()
                    .map(|&c| c.distance_squared(p))
                    .fold(f64::INFINITY, f64::min)
                    .sqrt();
                if d < range {
                    let w = options.shape.weight(d / range);
                    if w > 0. {
                        moves.push((v as u32, w));
                    }
                }
            }
        }
    }
    moves.sort_unstable_by_key(|m| m.0);
    Ok(TweakPlan {
        grab,
        options,
        symmetry,
        mirror,
        moves: crate::Shared::new(moves),
    })
}

/// Places a tweak on `mesh`: each planned point at its press-time position
/// plus weight × `offset` (mesh coordinates). With Symmetry, points on the
/// far side take the mirrored offset and points on the plane stay on it.
pub fn tweak_apply(mesh: &mut Mesh, base: &Mesh, plan: &TweakPlan, offset: DVec3) -> Result<()> {
    apply_moves(mesh, base, &plan.moves, offset, plan.mirror.as_ref())
}

fn apply_moves(
    mesh: &mut Mesh,
    base: &Mesh,
    moves: &[(u32, f64)],
    offset: DVec3,
    mirror: Option<&TweakMirror>,
) -> Result<()> {
    if !offset.is_finite() {
        return Err("Tweak offset must be finite".into());
    }
    if mesh.vertices.len() != base.vertices.len() {
        return Err("Tweak: the mesh changed during the drag".into());
    }
    if moves
        .iter()
        .any(|&(v, _)| v as usize >= base.vertices.len())
    {
        return Err("Tweak: that point no longer exists".into());
    }
    for &(v, w) in moves {
        let start = base.vertices[v as usize];
        let o = offset * w;
        mesh.vertices[v as usize] = match mirror {
            None => start + o,
            Some(m) => {
                let s = m.symmetry;
                match s.side(start, m.tolerance) {
                    Some(Side::Plane) => s.onto_plane(start + o),
                    Some(side) if side == m.primary => start + o,
                    _ => start + s.mirror_vector(o),
                }
            }
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;

    fn quad() -> Mesh {
        Mesh {
            vertices: vec![
                DVec3::new(0., 0., 0.),
                DVec3::new(1., 0., 0.),
                DVec3::new(1., 1., 0.),
                DVec3::new(0., 1., 0.),
                DVec3::new(5., 5., 5.),
            ],
            faces: vec![Face {
                vertices: vec![0, 1, 2, 3],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            }]
            .into(),
            ..Default::default()
        }
    }

    #[test]
    fn each_element_moves_only_its_own_points() {
        let offset = DVec3::new(0.25, -0.5, 1.);
        for (element, moved) in [
            (TweakElement::Vertex(2), vec![2]),
            (TweakElement::Edge(3, 0), vec![0, 3]),
            (TweakElement::Polygon(0), vec![0, 1, 2, 3]),
        ] {
            let before = quad();
            let mut after = before.clone();
            tweak_points(&mut after, element, offset).unwrap();
            for v in 0..before.vertices.len() {
                let expected = if moved.contains(&v) {
                    before.vertices[v] + offset
                } else {
                    before.vertices[v]
                };
                assert_eq!(after.vertices[v], expected, "{element:?} point {v}");
            }
            assert_eq!(after.faces, before.faces);
        }
    }

    /// ACS-01/ACS-03: Tweak's primary side is the pivot service's, so an
    /// element exactly on the plane counts as the − side, as for Move.
    #[test]
    fn tweak_primary_side_agrees_with_the_pivot_service() {
        let mesh = quad();
        let symmetry = Symmetry::x();
        let tolerance = crate::symmetry::match_tolerance(&mesh);
        let moves = |points: &[u32]| {
            let half = crate::pivot_service::half_points(&mesh, points, symmetry);
            let centre = crate::box_centre(half.iter().map(|&v| mesh.vertices[v as usize]));
            crate::pivot_service::primary_side(symmetry, centre, tolerance)
        };
        for (element, expected) in [
            // On the plane (x = 0): the − side.
            (TweakElement::Vertex(0), Side::Negative),
            (TweakElement::Edge(3, 0), Side::Negative),
            (TweakElement::Vertex(1), Side::Positive),
            // On the plane and the + side: the + half decides.
            (TweakElement::Polygon(0), Side::Positive),
        ] {
            let points = element.points(&mesh).unwrap();
            let mirror = tweak_mirror(&mesh, element, symmetry).unwrap().unwrap();
            assert_eq!(mirror.primary, expected, "{element:?}");
            assert_eq!(mirror.primary, moves(&points), "{element:?} vs Move");
        }
    }

    #[test]
    fn missing_elements_and_bad_offsets_are_refused() {
        let mut mesh = quad();
        assert!(tweak_points(&mut mesh, TweakElement::Vertex(9), DVec3::X).is_err());
        assert!(tweak_points(&mut mesh, TweakElement::Polygon(3), DVec3::X).is_err());
        assert!(tweak_points(&mut mesh, TweakElement::Vertex(0), DVec3::NAN).is_err());
        assert_eq!(mesh, quad());
    }
}
