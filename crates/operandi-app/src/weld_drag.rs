//! Weld drag (WD-01..05): with Weld armed, press on a point and drag it.
//! Within the Weld snap radius of another point of the same mesh it snaps
//! there (highlighted); releasing then merges the dragged point into that
//! one. Released over empty space it just moves, keeping the view-plane
//! depth it started at. Each drag is one undo step; the tool stays armed.
//!
//! The scene isn't touched until the release: the drag is previewed as an
//! overlay (the point and its edges following the pointer), so Escape or a
//! refused weld leaves the scene exactly as it was.
use super::*;
use theme::viewport as palette;

/// WD-02 snap distance, in logical (egui) points (the snap service's
/// table holds it).
#[cfg(test)]
pub(crate) const WELD_SNAP_RADIUS: f32 = match crate::snap_service::Role::Weld.fixed_radius() {
    Some(r) => r,
    None => 0.,
};

/// One Weld drag in progress, from the press to the release.
pub(crate) struct WeldDragState {
    item: u64,
    /// The point being dragged.
    source: u32,
    /// Its world position at the press: the drag stays at this depth.
    start: DVec3,
    /// World positions of the points sharing an edge with `source`, for
    /// the rubber-band preview.
    neighbours: Vec<DVec3>,
    pointer: Pos2,
    /// The point the drag is snapped onto now, if any, and where it is.
    snap: Option<(u32, DVec3)>,
}

impl Operandi {
    /// The press with Weld armed: pick the nearest point under the pointer
    /// (whatever the selection) and start dragging it.
    pub(super) fn weld_press(&mut self, pos: Pos2, r: Rect) -> input::GestureKind {
        let Some(source) = self.pick_vertex(pos, r) else {
            self.status = "Weld: press on a point and drag it".into();
            return input::GestureKind::Blocked;
        };
        let Some(item) = self.editor.scene.active() else {
            return input::GestureKind::Blocked;
        };
        let Ok(matrix) = self.editor.scene.world_matrix(item.id) else {
            return input::GestureKind::Blocked;
        };
        let mesh = &item.mesh;
        let mut neighbours: Vec<u32> = Vec::new();
        for (fi, f) in mesh.faces.iter().enumerate() {
            let n = f.vertices.len();
            let Some(i) = f.vertices.iter().position(|&v| v == source) else {
                continue;
            };
            // An open polyline's ends have one neighbour; polygons wrap.
            let around = if mesh.is_polyline(fi) {
                [i.checked_sub(1), (i + 1 < n).then_some(i + 1)]
            } else {
                [Some((i + n - 1) % n), Some((i + 1) % n)]
            };
            for v in around.into_iter().flatten().map(|j| f.vertices[j]) {
                if v != source && !neighbours.contains(&v) {
                    neighbours.push(v);
                }
            }
        }
        self.weld_drag = Some(WeldDragState {
            item: item.id,
            source,
            start: matrix.transform_point3(mesh.vertices[source as usize]),
            neighbours: neighbours
                .iter()
                .map(|&v| matrix.transform_point3(mesh.vertices[v as usize]))
                .collect(),
            pointer: pos,
            snap: None,
        });
        self.status = "Weld: drag onto another point to weld, or release to move it".into();
        input::GestureKind::WeldDrag
    }

    /// The pointer moved during a Weld drag: follow it and find a snap.
    pub(super) fn weld_move(&mut self, pos: Pos2, r: Rect) {
        let snap = self.weld_snap(pos, r);
        if let Some(state) = &mut self.weld_drag {
            state.pointer = pos;
            state.snap = snap;
        }
    }

    /// WD-02: the nearest visible other point of the same mesh within the
    /// snap radius. Points hidden behind the surface don't snap, as they
    /// don't pick.
    fn weld_snap(&self, pos: Pos2, r: Rect) -> Option<(u32, DVec3)> {
        let state = self.weld_drag.as_ref()?;
        self.editor.scene.active().filter(|i| i.id == state.item)?;
        use crate::snap_service::Owner;
        let other = |owner: &Owner| !matches!(*owner, Owner::Point { item, v } if item == state.item && v != state.source);
        let found = self.snap_scene_point(pos, crate::snap_service::Role::Weld, r, &other)?;
        match found.owner {
            Owner::Point { v, .. } => Some((v, found.hit.point)),
            _ => None,
        }
    }

    /// WD-05: where the pointer meets the plane facing the view through
    /// the dragged point's starting position.
    fn weld_view_point(&self, start: DVec3, pos: Pos2, r: Rect) -> Option<DVec3> {
        let (_, _, _, forward) = self.camera.basis();
        let (o, d) = self.camera.ray(pos, r);
        let denom = d.dot(forward);
        if denom.abs() < 1e-12 {
            return None;
        }
        let t = (start - o).dot(forward) / denom;
        (t > 0.).then(|| o + d * t).filter(|p| p.is_finite())
    }

    /// The release: one Weld edit (merge or move), and Weld stays armed.
    pub(super) fn weld_release(&mut self, pos: Pos2, start: Pos2, r: Rect) {
        self.weld_move(pos, r);
        let Some(state) = self.weld_drag.take() else {
            return;
        };
        let drop = match state.snap {
            Some((target, _)) => WeldDrop::Point(target),
            // A press and release in place is a click: nothing to move.
            None if pos.distance(start) < 1. => {
                self.status = "Weld: drag the point onto another one".into();
                return;
            }
            None => match self.weld_view_point(state.start, pos, r) {
                Some(world) => WeldDrop::Position(world),
                None => {
                    self.status = "Weld: release in front of the camera".into();
                    return;
                }
            },
        };
        let merged = matches!(drop, WeldDrop::Point(_));
        if self.run_ok(Command::WeldDrag {
            source: state.source,
            drop,
        }) {
            self.status = if merged {
                "Weld done; drag another point to weld again".into()
            } else {
                "Weld: point moved; drag onto another point to weld".into()
            };
        }
    }

    #[cfg(test)]
    pub(super) fn weld_drag_snap(&self) -> Option<u32> {
        self.weld_drag.as_ref().and_then(|s| s.snap).map(|s| s.0)
    }

    #[cfg(test)]
    pub(super) fn weld_drag_source(&self) -> Option<u32> {
        self.weld_drag.as_ref().map(|s| s.source)
    }

    /// The drag preview: the point's edges and the point following the
    /// pointer (or the snap target), and the snap target highlighted.
    pub(super) fn paint_weld_drag(&self, painter: &Painter, r: Rect) {
        let Some(state) = &self.weld_drag else {
            return;
        };
        let snapped = state
            .snap
            .and_then(|(_, p)| self.camera.project(p, r))
            .map(|q| q.0);
        let at = snapped
            .or_else(|| {
                self.weld_view_point(state.start, state.pointer, r)
                    .and_then(|p| self.camera.project(p, r).map(|q| q.0))
            })
            .unwrap_or(state.pointer);
        let stroke = Stroke::new(1.5_f32, palette::selection());
        for &n in &state.neighbours {
            if let Some((q, _)) = self.camera.project(n, r) {
                painter.line_segment([q, at], stroke);
            }
        }
        if let Some(q) = snapped {
            painter.circle_stroke(q, 8., Stroke::new(2_f32, palette::rollover()));
        }
        painter.circle_filled(at, 4., palette::selection());
    }
}
