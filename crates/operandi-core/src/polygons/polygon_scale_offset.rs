//! PolyScale Offset and PolyScale Offset Detach (spec
//! `polyscale-offset-2026-11-06`; earlier MP-C13 / SO-C01..04).
//!
//! Each selected polygon shrinks in its own plane: every edge moves inward
//! by the Inset value (negative grows). PolyScale Offset moves the
//! polygons' own points, so nothing is added and neighbouring polygons
//! slant to follow; the selection is kept. Several selected polygons are
//! inset as one region (shared points move along the shared edges). Detach
//! rebuilds the region on new points so it comes away, and clears the
//! selection. Both go through the Bevel core (grouped, Shift 0, local
//! units); a single isolated polygon keeps the exact profile below.
//! Provisional: collapsing values (Q-PO-01), two adjacent faces detach as
//! one piece (Q-PO-02).
//!
//! Isolated-polygon profile:
//!
//! Every boundary line of the selected face moves by the same signed
//! perpendicular distance inside the face's own plane: positive contracts,
//! negative expands. This is an equal edge-distance offset, not a uniform
//! scale about a center, so a right triangle's corners move by different
//! amounts.
//!
//! Supported domain: exactly one selected ordinary surface face of the active
//! visible mesh whose points are used by no other face, with at least three
//! distinct finite corners forming a planar, simple, strictly convex loop.
//! Only the face's existing points move. Topology, corner order, identities,
//! selection and every face, corner and point record stay exactly as they
//! were; nothing is recomputed or averaged. Other geometry and unused points
//! in the mesh are not read, so their extent never affects eligibility.
//!
//! Corner `i` of the result is the intersection of the offset lines of its
//! two incident edges. With unit in-plane inward edge normals `a` (previous
//! edge) and `b` (next edge), the displacement `d (a + b) / (1 + a·b)` has a
//! component of exactly `d` along both normals; strict convexity keeps
//! `1 + a·b` positive. The result must again be a nondegenerate, strictly
//! convex polygon with every edge in its original direction, otherwise the
//! distance is refused as unsafe.
//!
//! Distances are physical scene units (SO-C01-A1). The loop is mapped through
//! the composed item/parent world transform, offset there, and the result is
//! mapped back to local points, so finite invertible scale, reflection and
//! the shear such compositions produce are honored. Inward follows the
//! scene-space winding, which a reflection reverses together with the
//! normal. Transforms and every other record stay unchanged. Each loop is
//! checked with tolerances relative to its own size both in scene space and
//! locally; a singular transform or a result that does not map back to the
//! scene-space lines is refused.
use crate::{Editor, Mesh, Mode, PolygonRepresentation, Result};
use glam::{DMat3, DMat4, DVec2, DVec3, DVec4};

/// Relative tolerance for planarity, edge length and strict turns, measured
/// against the selected polygon's own size.
const RELATIVE_TOLERANCE: f64 = 1e-9;

/// Typed point/corner data whose meaning depends on in-plane geometry. The
/// admitted contract preserves authored data verbatim and forbids
/// recomputation, so such data on the affected face is refused instead of
/// being left stale.
const GEOMETRY_DERIVED_KINDS: [&str; 2] = ["tangent_basis6", "position_derivative6"];

fn refuse(reason: &str) -> String {
    format!("Scale Offset cannot apply to {reason}")
}

/// Plane frame `(normal, u, w)` with `u × w = normal`, so a loop wound
/// counter-clockwise about the normal is counter-clockwise in `(u, w)`.
fn frame(normal: DVec3) -> (DVec3, DVec3) {
    let u = normal.any_orthonormal_vector();
    (u, normal.cross(u))
}

/// Size of the loop: the largest distance between its first point and any
/// other. Tolerances scale with it so distant geometry never matters.
fn loop_size(points: &[DVec3]) -> f64 {
    points
        .iter()
        .map(|p| p.distance(points[0]))
        .fold(0., f64::max)
}

/// Check that `points` form a planar, simple, strictly convex polygon wound
/// counter-clockwise about `normal`, with no short edges.
fn require_strictly_convex(points: &[DVec3], normal: DVec3, what: &str) -> Result<()> {
    let size = loop_size(points);
    if !(size.is_finite() && size > 0.) {
        return Err(refuse(&format!("a degenerate {what}")));
    }
    let tolerance = RELATIVE_TOLERANCE * size;
    if points
        .iter()
        .any(|p| (*p - points[0]).dot(normal).abs() > tolerance)
    {
        return Err(refuse(&format!("a nonplanar {what}")));
    }
    let (u, w) = frame(normal);
    let flat: Vec<DVec2> = points
        .iter()
        .map(|p| DVec2::new((*p - points[0]).dot(u), (*p - points[0]).dot(w)))
        .collect();
    let n = flat.len();
    let edges: Vec<DVec2> = (0..n).map(|i| flat[(i + 1) % n] - flat[i]).collect();
    if edges.iter().any(|e| e.length() <= tolerance) {
        return Err(refuse(&format!(
            "a {what} with repeated or degenerate corners"
        )));
    }
    let mut turning = 0.;
    for i in 0..n {
        let (a, b) = (edges[(i + n - 1) % n], edges[i]);
        let sine = a.perp_dot(b) / (a.length() * b.length());
        if sine <= RELATIVE_TOLERANCE {
            return Err(refuse(&format!("a {what} that is not strictly convex")));
        }
        turning += a.angle_to(b);
    }
    // All left turns alone also admit star loops; one full turn is simple.
    if (turning - std::f64::consts::TAU).abs() > 1e-6 {
        return Err(refuse(&format!("a self-intersecting {what}")));
    }
    Ok(())
}

/// Newell normal of a loop; zero when the loop has no area.
fn loop_normal(points: &[DVec3]) -> DVec3 {
    let n = points.len();
    (0..n)
        .map(|i| (points[i] - points[0]).cross(points[(i + 1) % n] - points[0]))
        .sum::<DVec3>()
        .normalize_or_zero()
}

/// Equal signed boundary-line offset of a strictly convex loop wound
/// counter-clockwise about `normal`, refusing collapse or inversion.
fn offset_loop(source: &[DVec3], normal: DVec3, distance: f64) -> Result<Vec<DVec3>> {
    let n = source.len();
    let inward: Vec<DVec3> = (0..n)
        .map(|i| normal.cross(source[(i + 1) % n] - source[i]).normalize())
        .collect();
    let result: Vec<DVec3> = (0..n)
        .map(|i| {
            let (a, b) = (inward[(i + n - 1) % n], inward[i]);
            source[i] + distance * (a + b) / (1. + a.dot(b))
        })
        .collect();
    let tolerance = RELATIVE_TOLERANCE * loop_size(source);
    let reversed = (0..n).any(|i| {
        let before = source[(i + 1) % n] - source[i];
        (result[(i + 1) % n] - result[i]).dot(before) / before.length() <= tolerance
    });
    if reversed || result.iter().any(|p| !p.is_finite()) {
        return Err(refuse("a distance that collapses or inverts the polygon"));
    }
    require_strictly_convex(&result, normal, "result")
        .map_err(|_| refuse("a distance that collapses or inverts the polygon"))?;
    Ok(result)
}

/// Inverse of an affine, finite, nonsingular world transform.
fn world_inverse(world: DMat4) -> Result<DMat4> {
    let determinant = DMat3::from_mat4(world).determinant();
    let inverse = world.inverse();
    if world.row(3) != DVec4::W
        || !determinant.is_finite()
        || determinant == 0.
        || !inverse.is_finite()
    {
        return Err(refuse(
            "an item with a singular or non-affine scene transform",
        ));
    }
    Ok(inverse)
}

/// Distance-independent local eligibility of `face`: its corner positions
/// in order and its local normal, or a refusal. Pure.
fn eligible_loop(mesh: &Mesh, face: usize) -> Result<(Vec<DVec3>, DVec3)> {
    let loop_ = &mesh
        .faces
        .get(face)
        .ok_or("Selected polygon no longer exists")?
        .vertices;
    if mesh.is_polyline(face)
        || mesh.attributes.degenerate_faces.contains(&(face as u32))
        || mesh
            .attributes
            .representations
            .get(face)
            .is_some_and(|r| matches!(r, PolygonRepresentation::Polyline { .. }))
    {
        return Err(refuse("a polyline or degenerate element"));
    }
    if loop_.len() < 3 {
        return Err(refuse("a polygon with fewer than three corners"));
    }
    let mut sorted = loop_.clone();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != loop_.len() {
        return Err(refuse("a polygon with repeated corners"));
    }
    if mesh
        .faces
        .iter()
        .enumerate()
        .any(|(fi, f)| fi != face && f.vertices.iter().any(|v| sorted.binary_search(v).is_ok()))
    {
        return Err(refuse("a polygon sharing points with other polygons"));
    }
    let typed_derived = mesh.attributes.typed_maps.values().any(|map| {
        GEOMETRY_DERIVED_KINDS.contains(&map.kind.as_str())
            && ((!map.corners.is_empty() && map.corners[face].iter().any(Option::is_some))
                || loop_.iter().any(|v| map.points.contains_key(v)))
    });
    if typed_derived {
        return Err(refuse("authored tangent or position-derivative data"));
    }
    let source: Vec<DVec3> = loop_.iter().map(|&v| mesh.vertices[v as usize]).collect();
    let normal = mesh.normal(face);
    if normal == DVec3::ZERO {
        return Err(refuse("a degenerate polygon"));
    }
    require_strictly_convex(&source, normal, "polygon")?;
    Ok((source, normal))
}

/// The loop mapped into scene space with its scene normal, checked there
/// with tolerances relative to its own scene-space size. Pure.
fn scene_loop(source: &[DVec3], world: DMat4) -> Result<(Vec<DVec3>, DVec3)> {
    let scene_source: Vec<DVec3> = source.iter().map(|&p| world.transform_point3(p)).collect();
    let scene_normal = loop_normal(&scene_source);
    if scene_normal == DVec3::ZERO {
        return Err(refuse("a polygon that is degenerate in scene space"));
    }
    require_strictly_convex(&scene_source, scene_normal, "polygon in scene space")?;
    Ok((scene_source, scene_normal))
}

/// Every distance-independent refusal for `face` under `world`: the local
/// loop, an invertible transform and the scene-space loop. Pure; this is
/// what the read-only offer check runs, without computing an offset.
pub(crate) fn scale_offset_static_eligibility(
    mesh: &Mesh,
    face: usize,
    world: DMat4,
) -> Result<()> {
    let (source, _) = eligible_loop(mesh, face)?;
    world_inverse(world)?;
    scene_loop(&source, world).map(|_| ())
}

/// Offset corner positions for `face`, in its corner order, or a refusal.
/// `world` is the item's composed scene transform; the distance is measured
/// in scene space. Pure: reads only the face's own points and records.
pub(crate) fn scale_offset_positions(
    mesh: &Mesh,
    face: usize,
    distance: f64,
    world: DMat4,
) -> Result<Vec<DVec3>> {
    if !distance.is_finite() || distance.abs() > 1e9 {
        return Err("Scale Offset requires a finite distance within the scene range".into());
    }
    let (source, normal) = eligible_loop(mesh, face)?;
    let inverse = world_inverse(world)?;
    if distance == 0. {
        // Exact no-change; never round-trip points through the transform.
        return Ok(source);
    }
    let (scene_source, scene_normal) = scene_loop(&source, world)?;
    let scene_result = offset_loop(&scene_source, scene_normal, distance)?;
    let result: Vec<DVec3> = scene_result
        .iter()
        .map(|&q| inverse.transform_point3(q))
        .collect();
    // Local safeguards: the mapped-back loop is itself a valid strictly
    // convex polygon and reproduces the scene-space corners.
    if result.iter().any(|p| !p.is_finite()) {
        return Err(refuse("a result that cannot be represented locally"));
    }
    require_strictly_convex(&result, normal, "result")
        .map_err(|_| refuse("a distance that collapses or inverts the polygon"))?;
    let tolerance = RELATIVE_TOLERANCE * loop_size(&scene_result);
    if result
        .iter()
        .zip(&scene_result)
        .any(|(&p, q)| world.transform_point3(p).distance(*q) > tolerance)
    {
        return Err(refuse("a result that cannot be represented locally"));
    }
    Ok(result)
}

/// Whether `face` shares no point with any other face (the isolated profile).
fn isolated(mesh: &Mesh, face: usize) -> bool {
    let Some(f) = mesh.faces.get(face) else {
        return false;
    };
    mesh.faces
        .iter()
        .enumerate()
        .all(|(fi, g)| fi == face || !g.vertices.iter().any(|v| f.vertices.contains(v)))
}

/// PolyScale Offset on polygons that share points with others (spec
/// `polyscale-offset-2026-11-06`): the selected polygons are inset as one
/// region (the Bevel core, grouped, Shift 0, local units), then the frame
/// polygons are dropped. Offset moves the region's own points onto the
/// inset positions, so no point or polygon is added and the neighbours
/// slant to follow (B1–B3). Detach keeps the inset copies as new points, so
/// the region floats free and the original points stay with the
/// neighbours (B4). Returns the mesh; the selected polygons keep their
/// slots.
pub fn polyscale_offset_geometry(
    mesh: &Mesh,
    faces: &[u32],
    distance: f64,
    detach: bool,
) -> Result<Mesh> {
    if !distance.is_finite() || distance.abs() > 1e9 {
        return Err("Scale Offset requires a finite distance within the scene range".into());
    }
    let inset = crate::bevel_geometry(
        mesh,
        faces,
        0.,
        distance,
        true,
        &crate::BevelOptions::default(),
        None,
    )
    .map_err(|e| format!("Scale Offset: {e}"))?;
    let mut out = inset.mesh;
    // The frame polygons are the ones Bevel appended.
    let frame: std::collections::BTreeSet<usize> = (mesh.faces.len()..out.faces.len()).collect();
    if detach {
        out.remove_faces(&frame)?;
        // Region points no neighbour used are left floating: drop them.
        let used = out.used_points();
        let survivors: Vec<usize> = (0..out.vertices.len()).filter(|&v| used[v]).collect();
        if survivors.len() != out.vertices.len() {
            out = crate::mesh_cleanup::remap_points(&out, &survivors)?;
        }
        out.validate()?;
        return Ok(out);
    }
    // Offset: each selected corner's original point takes its inset copy's
    // position; the copies and the frame go away again.
    let mut result = mesh.clone();
    for &f in faces {
        let before = &mesh.faces[f as usize].vertices;
        let after = &out.faces[f as usize].vertices;
        if before.len() != after.len() {
            return Err("Scale Offset: the inset changed a polygon's corners".into());
        }
        for (&v, &w) in before.iter().zip(after) {
            result.vertices[v as usize] = out.vertices[w as usize];
        }
    }
    for f in 0..result.faces.len() {
        if result.normal(f) == DVec3::ZERO && !result.is_polyline(f) {
            result.attributes.degenerate_faces.insert(f as u32);
        }
    }
    result
        .validate()
        .map_err(|e| format!("Scale Offset produced an invalid mesh ({e}); nothing changed"))?;
    Ok(result)
}

impl Editor {
    /// Read-only Scale Offset eligibility of the current context: the active
    /// item id and its selected faces. Geometry is checked at dispatch.
    pub fn scale_offset_polygon_target(&self) -> Result<(u64, Vec<u32>)> {
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before Scale Offset".into());
        }
        if self.session.pen_draft.is_some() {
            return Err("Drop or Cancel Pen before Scale Offset".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Scale Offset".into());
        }
        if self.scene.selection.mode != Mode::Polygon {
            return Err("Scale Offset requires Polygon selection mode".into());
        }
        if self.scene.workplane.fixed
            || self.action_context != crate::ActionContext::default()
            || self.spatial_falloff.is_some()
        {
            return Err("Scale Offset requires the neutral construction and action context".into());
        }
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh for Scale Offset")?;
        let faces = self.scene.selection.faces.clone();
        if faces.is_empty() {
            return Err("Select one polygon or more for Scale Offset".into());
        }
        if faces.iter().any(|&f| f as usize >= item.mesh.faces.len()) {
            return Err("Selected polygon no longer exists".into());
        }
        Ok((item.id, faces))
    }

    /// Read-only offer check (SO-U01): the target context plus every
    /// distance-independent geometric, data and transform refusal, so a
    /// statically unsupported polygon is refused before a distance is asked
    /// for. Distance-dependent refusals (collapse, inversion, range) are
    /// decided only by the edit itself. Never mutates or records anything.
    pub fn scale_offset_polygon_eligibility(&self) -> Result<(u64, Vec<u32>)> {
        let (item_id, faces) = self.scale_offset_polygon_target()?;
        self.scene.validate()?;
        let world = self.scene.world_matrix(item_id)?;
        let mesh = &self.scene.active().ok_or("Missing active mesh")?.mesh;
        match faces[..] {
            [face] if isolated(mesh, face as usize) => {
                scale_offset_static_eligibility(mesh, face as usize, world)?;
            }
            _ => {
                mesh.validate_extrude_selection(&faces)?;
            }
        }
        Ok((item_id, faces))
    }

    /// One history edit that moves the selected face's points. A zero or
    /// otherwise position-identical result is a successful no-change.
    pub(crate) fn scale_offset_polygon(&mut self, distance: f64) -> Result<()> {
        let (item_id, faces) = self.scale_offset_polygon_target()?;
        self.scene.validate()?;
        let world = self.scene.world_matrix(item_id)?;
        let mesh = &self.scene.active().ok_or("Missing active mesh")?.mesh;
        let face = match faces[..] {
            [face] if isolated(mesh, face as usize) => face,
            _ => {
                if distance == 0. {
                    return Ok(());
                }
                // Points move, nothing is added (B1–B3); selection kept.
                let result = polyscale_offset_geometry(mesh, &faces, distance, false)?;
                let mut candidate = self.scene.clone();
                candidate
                    .active_mut()
                    .filter(|i| i.id == item_id)
                    .ok_or("Missing active mesh")?
                    .mesh = result.into();
                candidate.validate()?;
                let before = std::mem::replace(&mut self.scene, candidate);
                if let Err(error) = self.identities.observe(&self.scene) {
                    self.scene = before;
                    return Err(error);
                }
                self.record("Scale Offset".into(), before);
                return Ok(());
            }
        };
        let positions = scale_offset_positions(mesh, face as usize, distance, world)?;
        let loop_ = mesh.faces[face as usize].vertices.clone();
        if loop_
            .iter()
            .zip(&positions)
            .all(|(&v, p)| mesh.vertices[v as usize] == *p)
        {
            return Ok(());
        }
        let mut candidate = self.scene.clone();
        let target = &mut candidate
            .active_mut()
            .filter(|i| i.id == item_id)
            .ok_or("Missing active mesh")?
            .mesh;
        for (&v, p) in loop_.iter().zip(positions) {
            target.vertices[v as usize] = p;
        }
        candidate.validate()?;
        // Only coordinates change, so every component identity survives.
        let before = std::mem::replace(&mut self.scene, candidate);
        if let Err(error) = self.identities.observe(&self.scene) {
            self.scene = before;
            return Err(error);
        }
        self.record("Scale Offset".into(), before);
        Ok(())
    }

    /// PolyScale Offset Detach (B4): the same inset, rebuilt on new points
    /// so the polygons float free; the selection is cleared afterwards.
    pub(crate) fn scale_offset_detach(&mut self, distance: f64) -> Result<()> {
        let (item_id, faces) = self.scale_offset_polygon_target()?;
        self.scene.validate()?;
        let mesh = &self.scene.active().ok_or("Missing active mesh")?.mesh;
        let result = polyscale_offset_geometry(mesh, &faces, distance, true)?;
        let mut candidate = self.scene.clone();
        candidate
            .active_mut()
            .filter(|i| i.id == item_id)
            .ok_or("Missing active mesh")?
            .mesh = result.into();
        candidate.selection.faces.clear();
        candidate.validate()?;
        let before = std::mem::replace(&mut self.scene, candidate);
        let ids_before = self.identities.current.clone();
        if let Err(error) = self.identities.invalidate_mesh(&self.scene, item_id) {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(error);
        }
        self.record("Scale Offset Detach".into(), before);
        Ok(())
    }
}
