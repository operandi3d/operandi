//! QUAD-B01/I01 Quadruple (Convert to Quadrangles). It
//! takes a polygon laid down as two opposing rows of vertices (for example
//! with Pen) and adds the edges spanning between opposite vertices, making a
//! row of quads.
//!
//! Spec `quadruple-2026-11-05`: each selected n-gon (n ≥ 5) is read as two
//! rows of corners joined by straight rungs; no points are added. An odd
//! count ends with a triangle. Of the possible strip ends, the pair giving
//! the shortest total new edges is used, never joining points of the same
//! row (decided Q-QD-01): such a join makes a quad with no area, which the
//! validity check refuses. Ties: the straighter rows. Every piece must stay
//! simple with positive area in the polygon's best-fit plane; the polygon
//! need not be planar or convex. Quads and triangles are left alone.
use crate::{
    Editor, Mode, Result,
    polygon_triple::{
        FaceSplitOrigin, TRIPLE_AREA_TOLERANCE, TRIPLE_LENGTH_TOLERANCE, chart_projected,
        remap_mesh, require_simple,
    },
};
use std::collections::{BTreeMap, BTreeSet};

/// Corner ordinals of each quad for one polygon, or a refusal.
fn plan(
    points: &[glam::DVec3],
    vertices: &[u32],
    edges: &BTreeSet<(u32, u32)>,
) -> Result<Vec<Vec<usize>>> {
    let n = points.len();
    // Rows of k + odd and k corners; an odd count ends with a triangle.
    let odd = n % 2;
    let k = n / 2;
    let plane = chart_projected(points, false)
        .map_err(|e| format!("Quadruple cannot read this polygon: {e}"))?;
    let q = &plane.points;
    let eps = TRIPLE_LENGTH_TOLERANCE * plane.extent;
    require_simple(q, eps).map_err(|e| format!("Quadruple needs a simple polygon: {e}"))?;
    let limit = TRIPLE_AREA_TOLERANCE * plane.extent * plane.extent;
    // Bend at a corner: |sin| of its turn (0 for a straight row corner).
    let turn = |i: usize| {
        let (a, b, c) = (q[(i + n - 1) % n], q[i], q[(i + 1) % n]);
        let (u, v) = ((b - a).normalize_or_zero(), (c - b).normalize_or_zero());
        u.perp_dot(v).abs()
    };
    let mut best: Option<(f64, f64, Vec<Vec<usize>>)> = None;
    // Row A runs a_m = c(s + m), row B back from the other end,
    // b_m = c(s + n − 1 − m); rungs join a_m and b_m.
    for s in 0..n {
        let a = |m: usize| (s + m) % n;
        let b = |m: usize| (s + n - 1 - m) % n;
        // Interior rungs a_m–b_m (the last quad of an even count closes on
        // a boundary edge).
        let rungs = 1..k - 1 + odd;
        let row_bend: f64 = rungs.clone().map(|m| turn(a(m)) + turn(b(m))).sum();
        // k − 1 quads, then for an odd count the closing triangle.
        let mut quads: Vec<Vec<usize>> = (1..k)
            .map(|m| vec![a(m - 1), a(m), b(m), b(m - 1)])
            .collect();
        if odd == 1 {
            quads.push(vec![a(k - 1), a(k), b(k - 1)]);
        }
        let valid = quads.iter().all(|quad| {
            let pts: Vec<_> = quad.iter().map(|&c| q[c]).collect();
            let corners = pts.len();
            let area: f64 = (0..corners)
                .map(|i| pts[i].perp_dot(pts[(i + 1) % corners]))
                .sum();
            area > 2. * limit && require_simple(&pts, eps).is_ok()
        }) && rungs.clone().all(|m| {
            let (x, y) = (vertices[a(m)], vertices[b(m)]);
            !edges.contains(&(x.min(y), x.max(y)))
        });
        if !valid {
            continue;
        }
        let span: f64 = rungs.map(|m| q[a(m)].distance(q[b(m)])).sum();
        let scale = plane.extent.max(1e-300);
        let better = best.as_ref().is_none_or(|(bend, len, _)| {
            span < len - 1e-9 * scale || ((span - len).abs() <= 1e-9 * scale && row_bend < *bend)
        });
        if better {
            best = Some((row_bend, span, quads));
        }
    }
    best.map(|(_, _, quads)| quads)
        .ok_or_else(|| "Quadruple cannot split this polygon into a valid row of quads".into())
}

impl Editor {
    /// Quadruple would change nothing: idle, Polygon mode, and every selected
    /// polygon a quad or triangle (CMD-P-02 / CMD-B3-03).
    pub(crate) fn quadruple_is_noop(&self) -> bool {
        let Some(item) = self.scene.active().filter(|i| self.scene.shows(i)) else {
            return false;
        };
        let faces = &self.scene.selection.faces;
        !self.i2_active()
            && self.session.pen_draft.is_none()
            && self.session.live.is_none()
            && self.selection_start.is_none()
            && self.scene.selection.mode == Mode::Polygon
            && !faces.is_empty()
            && faces.iter().all(|&f| {
                item.mesh.faces.get(f as usize).is_some_and(|face| {
                    face.vertices.len() <= 4
                        && !item.mesh.is_polyline(f as usize)
                        && !item.mesh.attributes.degenerate_faces.contains(&f)
                })
            })
    }

    pub(crate) fn quadruple_polygon(&mut self) -> Result<()> {
        if self.i2_active() {
            return Err("Finish or Cancel Extrude before Quadruple".into());
        }
        if self.session.pen_draft.is_some() {
            return Err("Finish or Cancel Pen before Quadruple".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish selection before Quadruple".into());
        }
        if self.scene.selection.mode != Mode::Polygon {
            return Err("Need to directly select one or more polygons.".into());
        }
        self.scene.validate()?;
        // Nothing selected converts every polygon and selects nothing after
        // (quadruple-2026-11-05 rules 6 and 8).
        let everything = self.scene.selection.faces.is_empty();
        let selection: BTreeSet<u32> = if everything {
            let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
            (0..mesh.faces.len() as u32)
                .filter(|&f| {
                    !mesh.is_polyline(f as usize) && !mesh.attributes.degenerate_faces.contains(&f)
                })
                .collect()
        } else {
            self.scene.selection.faces.iter().copied().collect()
        };
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh before Quadruple")?;
        let item_id = item.id;
        let source = &item.mesh;
        let edges: BTreeSet<(u32, u32)> = source.edges().into_iter().collect();
        let mut plans: Vec<(u32, Vec<Vec<usize>>)> = Vec::new();
        for &selected in &selection {
            let face = source
                .faces
                .get(selected as usize)
                .ok_or("Selected polygon no longer exists")?;
            if source.is_polyline(selected as usize)
                || source.attributes.degenerate_faces.contains(&selected)
            {
                return Err("Quadruple requires nondegenerate surface polygons".into());
            }
            if face.vertices.len() <= 4 {
                continue;
            }
            let points: Vec<_> = face
                .vertices
                .iter()
                .map(|&v| source.vertices[v as usize])
                .collect();
            let quads = plan(&points, &face.vertices, &edges)
                .map_err(|e| format!("Polygon {selected}: {e}"))?;
            plans.push((selected, quads));
        }
        if plans.is_empty() {
            // CMD-P-02 / CMD-B3-03: quads and triangles are left as they are,
            // with no message and the selection unchanged.
            return Ok(());
        }
        let mut origins: Vec<_> = (0..source.faces.len())
            .map(|face| FaceSplitOrigin {
                face,
                corners: None,
            })
            .collect();
        let mut pieces = BTreeMap::new();
        let mut new_selection: Vec<u32> = Vec::new();
        for (selected, quads) in plans {
            let mut indices = vec![selected];
            let mut quads = quads.into_iter();
            origins[selected as usize].corners = quads.next();
            for quad in quads {
                if origins.len() >= crate::MAX_MESH_ELEMENTS {
                    return Err("Quadruple exceeds the mesh face budget".into());
                }
                let appended =
                    u32::try_from(origins.len()).map_err(|_| "Mesh face budget exceeded")?;
                origins.push(FaceSplitOrigin {
                    face: selected as usize,
                    corners: Some(quad),
                });
                indices.push(appended);
            }
            new_selection.extend(&indices);
            pieces.insert(selected, indices);
        }
        let mesh = remap_mesh(source, &origins, &pieces)?;
        mesh.validate()?;
        let ids = self
            .identities
            .current
            .meshes
            .get(&item_id)
            .ok_or("Missing mesh identity owner")?
            .clone();
        let next_ids = self
            .identities
            .with_ids(|next| ids.split_faces(source, &mesh, &origins, next))?;
        let before = self.scene.clone();
        let identity_before = self.identities.current.clone();
        self.scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
        self.scene.selection.faces = if everything {
            Vec::new()
        } else {
            new_selection
        };
        self.identities
            .current
            .meshes
            .insert(item_id, next_ids.into());
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.observe(&self.scene))
        {
            self.scene = before;
            self.identities.current = identity_before;
            return Err(e);
        }
        self.record("Quadruple".into(), before);
        Ok(())
    }
}
