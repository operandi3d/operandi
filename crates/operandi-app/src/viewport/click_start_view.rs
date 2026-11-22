//! Click to start in the view (specifications/tool-click-start-2026-11-16):
//! where a click lands, and the first press with an armed tool.
use super::*;

impl Operandi {
    /// Where a click at `pos` lands (CS-10, BN-30a/31): on a snap target
    /// when snapping finds one, else on the active mesh's surface under the
    /// pointer (also with snapping off, provisional CS-Q02), else on the
    /// work plane. `None` when the view is edge-on to the work plane and
    /// the pointer misses the mesh.
    pub(crate) fn click_point(&mut self, pos: Pos2, r: Rect) -> Option<DVec3> {
        let point = self
            .surface_point_at(pos, r)
            .or_else(|| self.work_point(pos, r, self.actual_plane(), self.workplane_center()))?;
        // The target itself may be snapped to (nothing counts as moving
        // yet); points a tool has already moved are left out by the
        // snapping layer (SN2-17).
        Some(
            self.snap_point(point, Some(Default::default()), r)
                .unwrap_or(point),
        )
    }

    /// The first hit of the pointer's ray on the active mesh, as drawn.
    pub(crate) fn surface_point_at(&self, pos: Pos2, r: Rect) -> Option<DVec3> {
        let id = self.editor.scene.active()?.id;
        let (origin, direction) = self.camera.ray(pos, r);
        operandi_core::drop_action::surface_hit(
            &self.editor.scene,
            id,
            operandi_core::drop_action::Ray { origin, direction },
        )
        .map(|(point, _)| point)
    }

    /// CS-02: a primary press in a 3D view with an armed tool makes it live
    /// at the click (`Command::ToolClick`). Returns false when the press
    /// can't start it (a UV view, a click that lands nowhere, a refusal),
    /// with the reason in the status bar; true lets the press go on as the
    /// tool's own (CS-15).
    pub(crate) fn start_armed_tool(&mut self, pos: Pos2, r: Rect) -> bool {
        if self.is_uv_view(self.active_view) {
            self.status = "Click in a 3D view to start the tool".into();
            return false;
        }
        let Some(point) = self.click_point(pos, r) else {
            self.status = "The view is edge-on to the work plane; click on the mesh".into();
            return false;
        };
        let plane = self.actual_plane();
        self.run_ok(Command::ToolClick { point, plane })
    }
}
