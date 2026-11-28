//! Element and Screen Action Centers in the views (AC-RULES v6: C-ELEM,
//! A-ELEM, C-SCREEN, A-SCREEN). Choosing one puts it in force at once;
//! after that every press off the tool's handle picks (Element) or places
//! (Screen): `element_press` and `place_center_press` in
//! `viewport/handle_drag.rs`. What a press does under each Action Center
//! is core's `ActionContext::press`; the transform tools ask it in
//! `pointer_input.rs`.
use super::*;
use glam::DMat3;

impl Operandi {
    /// Element chosen by name (the Action Center menu, a pie). It is in
    /// force at once: the world origin with World axes until an element is
    /// picked, not whatever Action Center was on before. Choosing it again
    /// keeps the last pick.
    pub(super) fn choose_element_center(&mut self) {
        self.preset_center_click = false;
        if !matches!(self.editor.action_context.center, CenterMode::Element(_))
            && !self.run_ok(Command::ActionContext(ActionContext::ELEMENT_UNPICKED))
        {
            return;
        }
        self.status =
            "Element: with Move, Rotate or Scale on, press a polygon, edge or point to center on it"
                .into();
    }

    /// Screen chosen by name. It is in force at once: the centre where
    /// Automatic's is now (the middle of the box round what the tool
    /// moves), with the view's axes as they are now (A-SCREEN). Every
    /// press off the handle then places it again.
    pub(super) fn choose_screen_center(&mut self) {
        self.preset_center_click = false;
        let scene = &self.editor.scene;
        let center = scene
            .active()
            .zip(operandi_core::transform_vertices(scene).ok())
            .and_then(|(item, moved)| {
                let world = scene.world_matrix(item.id).ok()?;
                let c = operandi_core::box_centre(
                    moved
                        .iter()
                        .map(|&v| world.transform_point3(item.mesh.vertices[v as usize])),
                );
                c.is_finite().then_some(c)
            })
            .unwrap_or(self.camera.target);
        if self.run_ok(Command::ActionContext(self.screen_context(center))) {
            self.status =
                "Screen: with Move, Rotate or Scale on, press in the view to place the center"
                    .into();
        }
    }

    /// Screen at world point `center`, with this view's axes: X to the
    /// right, Y up, Z toward the viewer.
    pub(super) fn screen_context(&self, center: DVec3) -> ActionContext {
        let (_, right, up, forward) = self.camera.basis();
        ActionContext {
            center: CenterMode::Screen(operandi_core::PickedPoint::new(center)),
            axis: AxisOrientation::Screen(operandi_core::PickedFrame::from_basis(
                DMat3::from_cols(right, up, -forward),
            )),
        }
    }
}
