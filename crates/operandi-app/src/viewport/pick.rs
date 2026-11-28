//! Picking in the viewport: the item, point, edge or polygon under the
//! pointer or near its path, and the rollover highlight.
use super::*;

impl Operandi {
    #[cfg(test)]
    pub(crate) fn pick_face(&self, p: Pos2, r: Rect) -> Option<(u64, u32, f64)> {
        self.pick_surface().pick(&self.camera, p, r)
    }
    /// The nearest element of any kind under `p` (Closest rollovers and
    /// Mouse Regions Trigger Selections): a point or edge within the pick
    /// radius, weighed by Point Selection Priority, else the polygon under
    /// the pointer.
    pub(crate) fn pick_closest(&self, p: Pos2, r: Rect) -> Option<(u64, Component)> {
        let surface = self.pick_surface();
        self.pick_closest_on(p, r, &surface)
    }
    pub(super) fn pick_closest_on(
        &self,
        p: Pos2,
        r: Rect,
        surface: &picking::Surface,
    ) -> Option<(u64, Component)> {
        let mut candidates = self.pick_candidates_in(Mode::Vertex, p, p, r);
        candidates.edges = self.pick_candidates_in(Mode::Edge, p, p, r).edges;
        self.pick_among(p, r, surface, &candidates).or_else(|| {
            surface
                .pick_in(&self.camera, p, r, crate::picking::limit(&self.pick_only()))
                .map(|(id, f, _)| (id, Component::Polygon(f)))
        })
    }
    /// The items viewport picking sees; empty means every item. With Pick
    /// Other Meshes in Component Modes off, Vertex, Edge and Polygon picks
    /// see only the active meshes (`Scene::active_meshes`). Other meshes
    /// then neither pick nor hide anything (a ray passes through them), so
    /// a click never switches the active mesh. With no shown active mesh
    /// picking sees every item, so a click can always start somewhere.
    pub(crate) fn pick_only(&self) -> Vec<u64> {
        let scene = &self.editor.scene;
        if self.selection_preferences.pick_other_meshes
            || !matches!(
                scene.selection.mode,
                Mode::Vertex | Mode::Edge | Mode::Polygon
            )
        {
            return Vec::new();
        }
        scene.shown_active_meshes()
    }
    /// What the pointer pre-highlights at `p`: the current mode's element,
    /// or with Closest rollovers in a component mode, the nearest of any
    /// kind.
    pub(super) fn rollover_component_on(
        &self,
        p: Pos2,
        r: Rect,
        surface: &picking::Surface,
    ) -> Option<(u64, Component)> {
        if self.closest_rollovers() {
            self.pick_closest_on(p, r, surface)
        } else {
            self.pick_component_on(p, r, surface)
        }
    }
    pub(crate) fn closest_rollovers(&self) -> bool {
        self.selection_preferences.rollovers == crate::selection_preferences::Rollovers::Closest
            && self.editor.scene.selection.mode != Mode::Item
    }
    /// Whether a picked element is part of the selection.
    pub(crate) fn is_selected_component(&self, id: u64, c: Component) -> bool {
        let s = &self.editor.scene.selection;
        match c {
            Component::Vertex(v) => s.vertices_of(id).contains(&v),
            Component::Edge(a, b) => s.edges_of(id).iter().any(|&e| e == (a, b) || e == (b, a)),
            Component::Polygon(f) => s.faces_of(id).contains(&f),
            Component::Item => s.item == Some(id) || s.also.contains(&id),
        }
    }
    pub(crate) fn pick_component(&self, p: Pos2, r: Rect) -> Option<(u64, Component)> {
        self.pick_component_on(p, r, &self.pick_surface())
    }
    pub(super) fn pick_component_on(
        &self,
        p: Pos2,
        r: Rect,
        surface: &picking::Surface,
    ) -> Option<(u64, Component)> {
        let mode = self.editor.scene.selection.mode;
        if matches!(mode, Mode::Polygon | Mode::Item) {
            let hit = surface.pick_in(&self.camera, p, r, crate::picking::limit(&self.pick_only()));
            // CP-A01: Select No can't be picked as an item in 3D views
            // (still selectable in the Items list).
            if mode == Mode::Item
                && hit.is_some_and(|(id, _, _)| {
                    self.editor
                        .scene
                        .items
                        .iter()
                        .any(|i| i.id == id && !i.is_viewport_selectable())
                })
            {
                return None;
            }
            return hit.map(|(id, f, _)| {
                (
                    id,
                    if mode == Mode::Item {
                        Component::Item
                    } else {
                        Component::Polygon(f)
                    },
                )
            });
        }
        self.pick_among(p, r, surface, &self.pick_candidates(p, p, r))
    }
    /// `item`'s points where they show, in its own coordinates: on the
    /// smooth surface for points whose polygons are all smooth, else on
    /// the cage (`SubdivisionCache::shown_points`). Picking, rollovers and
    /// point dots all use these, so every tool meets a subdivided mesh's
    /// points where they are drawn.
    pub(crate) fn shown_points<'a>(
        &self,
        item: &'a operandi_core::Item,
    ) -> std::borrow::Cow<'a, [DVec3]> {
        match self.subdivision_cache.shown_points(item) {
            Some(points) => std::borrow::Cow::Owned(points),
            None => std::borrow::Cow::Borrowed(&item.mesh.vertices),
        }
    }
    /// The points (Vertex mode) or edges (Edge mode) whose screen position
    /// lies within the 12-point pick radius of the screen segment `from`–`to`,
    /// projected once. A paint drag picks at every pixel along the pointer's
    /// path; gathering candidates once per move instead of projecting the
    /// whole mesh per pixel keeps fast drags on large meshes smooth.
    pub(crate) fn pick_candidates(&self, from: Pos2, to: Pos2, r: Rect) -> PickCandidates {
        self.pick_candidates_in(self.editor.scene.selection.mode, from, to, r)
    }
    /// `pick_candidates` for `mode`'s elements, whatever the current mode.
    pub(super) fn pick_candidates_in(
        &self,
        mode: Mode,
        from: Pos2,
        to: Pos2,
        r: Rect,
    ) -> PickCandidates {
        let mut out = PickCandidates::default();
        if !matches!(mode, Mode::Vertex | Mode::Edge) {
            return out;
        }
        let radius = self.pick_radius();
        let reach = Rect::from_two_pos(from, to).expand(radius);
        let near_path = |q: Pos2| input::segment_distance(q, from, to) < radius;
        // The view basis once, not per point.
        let projector = self.camera.projector(r);
        let display_hidden = self.editor.scene.display_hidden();
        let only = self.pick_only();
        for item in self.editor.scene.items.iter().filter(|i| {
            i.visible
                && !display_hidden.contains(&i.id)
                && (only.is_empty() || only.contains(&i.id))
        }) {
            let Ok(matrix) = self.editor.scene.world_matrix(item.id) else {
                continue;
            };
            self.item_pick_candidates(item, matrix, mode, reach, &near_path, &projector, &mut out);
        }
        out
    }
    /// `item`'s candidates for `pick_candidates_in` (and Tweak): its
    /// points (Vertex mode) or edges (Edge mode) in `reach` near the path,
    /// where they show (on a smooth surface, not its cage).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn item_pick_candidates(
        &self,
        item: &operandi_core::Item,
        matrix: glam::DMat4,
        mode: Mode,
        reach: Rect,
        near_path: &dyn Fn(Pos2) -> bool,
        projector: &crate::camera::Projector,
        out: &mut PickCandidates,
    ) {
        // Points pick where they show: on the smooth surface.
        let shown = self.shown_points(item);
        let point = |v: u32| matrix.transform_point3(shown[v as usize]);
        if mode == Mode::Vertex {
            for v in 0..item.mesh.vertices.len() as u32 {
                let world = point(v);
                if let Some((q, _)) = projector.project(world)
                    && reach.contains(q)
                    && near_path(q)
                {
                    out.points.push((item.id, v, q, world));
                }
            }
        } else {
            // The edge list is cached per mesh; sorting it every frame
            // cost several ms on large meshes.
            let edges = self.mesh_edges(item);
            // Each point projected once, not once per edge end.
            let projected: Vec<Option<(Pos2, f64)>> = (0..item.mesh.vertices.len() as u32)
                .map(|v| projector.project(point(v)))
                .collect();
            // Smooth polygons' edges pick where they are drawn: on the
            // smooth surface (Jason's QA, 11-05), each traced piece a
            // candidate naming its cage edge.
            let mut on_surface = std::collections::BTreeSet::new();
            if let Some(surface) = self.subdivision_cache.smooth(item) {
                for ((a, b), (wa, wb)) in surface.all_edge_paths() {
                    on_surface.insert((a, b));
                    let (wa, wb) = (matrix.transform_point3(wa), matrix.transform_point3(wb));
                    if let (Some((qa, za)), Some((qb, zb))) =
                        (projector.project(wa), projector.project(wb))
                        && Rect::from_two_pos(qa, qb).intersects(reach)
                    {
                        out.edges
                            .push((item.id, a, b, [(qa, za), (qb, zb)], [wa, wb]));
                    }
                }
            }
            for &(a, b) in edges.iter() {
                if on_surface.contains(&(a.min(b), a.max(b))) {
                    continue;
                }
                if let (Some((qa, za)), Some((qb, zb))) =
                    (projected[a as usize], projected[b as usize])
                    && Rect::from_two_pos(qa, qb).intersects(reach)
                {
                    out.edges
                        .push((item.id, a, b, [(qa, za), (qb, zb)], [point(a), point(b)]));
                }
            }
        }
    }
    /// The component nearest `p` among `candidates` (see `pick_candidates`)
    /// that isn't hidden behind a surface.
    pub(crate) fn pick_among(
        &self,
        p: Pos2,
        r: Rect,
        surface: &picking::Surface,
        candidates: &PickCandidates,
    ) -> Option<(u64, Component)> {
        // With picking limited to the active mesh, other meshes don't hide
        // its points and edges either, as they don't block its polygons.
        self.pick_among_items(p, r, surface, candidates, &self.pick_only())
    }
    /// `pick_among` where only the items in `only` can hide a candidate
    /// (every item when it is empty).
    pub(crate) fn pick_among_items(
        &self,
        p: Pos2,
        r: Rect,
        surface: &picking::Surface,
        candidates: &PickCandidates,
        only: &[u64],
    ) -> Option<(u64, Component)> {
        let radius = self.pick_radius();
        // Point Selection Priority: a point counts this much nearer when
        // weighed against edges (it only matters when both are candidates).
        let priority = self.selection_preferences.point_priority as f32;
        let mut best: Option<(f32, u64, Component)> = None;
        let mut nearest_point = radius;
        let only = crate::picking::limit(only);
        for &(id, v, q, world) in &candidates.points {
            let d = q.distance(p);
            if d < nearest_point && surface.visible_in(&self.camera, world, r, only) {
                nearest_point = d;
                best = Some((d - priority, id, Component::Vertex(v)));
            }
        }
        for &(id, a, b, [(qa, za), (qb, zb)], [wa, wb]) in &candidates.edges {
            let d = qb - qa;
            let t = ((p - qa).dot(d) / d.length_sq().max(1e-8)).clamp(0., 1.);
            let distance = input::segment_distance(p, qa, qb);
            if distance < best.as_ref().map_or(radius, |b| b.0)
                && surface.visible_in(
                    &self.camera,
                    wa.lerp(wb, picking::span_world_parameter(&self.camera, t, za, zb)),
                    r,
                    only,
                )
            {
                best = Some((distance, id, Component::Edge(a, b)));
            }
        }
        best.map(|(_, id, c)| (id, c))
    }
}
