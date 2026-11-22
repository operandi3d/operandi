//! Orbit Selected (Viewport Properties > Drawing and Control > Mouse
//! Control, viewport-buttons spec 4.1 "Now"): with it on, orbiting a view
//! turns the eye about the selection's centre instead of the view's centre,
//! so the selection stays put on screen while the view turns around it.
//! The centre is the middle of the selection's bounding box: the selected
//! components' points (the whole active mesh with none selected), or in
//! Item mode the selected items. With nothing selected the view orbits as
//! usual. Provisional details: VP-Q07.
use super::*;
use glam::DMat3;

/// After an orbit that turned the camera from basis `before` (eye, right,
/// up, forward) to its current one about its own target, move the target
/// so the same turn happens about `pivot` instead.
pub(crate) fn turn_about(
    camera: &mut crate::camera::Camera,
    before: (DVec3, DVec3, DVec3, DVec3),
    pivot: DVec3,
) {
    let (_, r0, u0, f0) = before;
    let (_, r1, u1, f1) = camera.basis();
    let old = DMat3::from_cols(r0, u0, f0);
    let new = DMat3::from_cols(r1, u1, f1);
    // The bases are orthonormal: the inverse is the transpose.
    let turn = new * old.transpose();
    let target = pivot + turn * (camera.target - pivot);
    if target.is_finite() {
        camera.target = target;
    }
}

/// The selection's centre in world space, if anything is selected.
pub(crate) fn selection_centre(scene: &operandi_core::Scene) -> Option<DVec3> {
    let (lo, hi) = if scene.selection.mode == Mode::Item {
        let mut lo = DVec3::splat(f64::INFINITY);
        let mut hi = DVec3::splat(f64::NEG_INFINITY);
        for id in scene.selected_items() {
            let item = scene.item(id)?;
            let Ok(world) = scene.world_matrix(id) else {
                continue;
            };
            let points = item
                .mesh
                .vertices
                .iter()
                .map(|&p| world.transform_point3(p));
            // A camera, light or locator (no points): its own place.
            let place = item
                .mesh
                .vertices
                .is_empty()
                .then(|| world.transform_point3(DVec3::ZERO));
            for p in points.chain(place) {
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        (lo, hi)
    } else {
        operandi_core::falloff::selection_bounds(scene).ok()?
    };
    (lo.is_finite() && hi.is_finite()).then(|| (lo + hi) / 2.)
}

/// The last pivot worked out, with what it was worked out from, so an orbit
/// drag finds it once rather than every frame.
pub(crate) struct PivotMemo {
    selection: operandi_core::Selection,
    serial: u64,
    pivot: Option<DVec3>,
}

impl Operandi {
    /// The point Orbit Selected turns about: the selection's centre.
    pub(super) fn orbit_pivot(&mut self) -> Option<DVec3> {
        let scene = &self.editor.scene;
        let serial = self.editor.history_serial;
        if let Some(memo) = &self.orbit_pivot_memo
            && memo.serial == serial
            && memo.selection == scene.selection
        {
            return memo.pivot;
        }
        let pivot = selection_centre(scene);
        self.orbit_pivot_memo = Some(Box::new(PivotMemo {
            selection: scene.selection.clone(),
            serial,
            pivot,
        }));
        pivot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_selected_keeps_the_selection_centre_in_place() {
        let mut a = crate::tests::box_app();
        let r = Rect::from_min_size(Pos2::ZERO, vec2(600., 400.));
        a.camera.view = View::Perspective;
        a.camera.target = DVec3::new(5., 0., 0.);
        // Item mode, the box selected: its centre.
        a.editor.dispatch(Command::Mode(Mode::Item)).unwrap();
        let id = a.editor.scene.items[0].id;
        a.editor.dispatch(Command::SelectItem(Some(id))).unwrap();
        let centre = selection_centre(&a.editor.scene).unwrap();
        let on_screen = |a: &Operandi| a.camera.project(centre, r).unwrap().0;
        let before = on_screen(&a);
        a.display.settings.drawing.orbit_selected = true;
        a.navigate(input::NavigationKind::Orbit, vec2(60., 25.), r);
        let after = on_screen(&a);
        assert!(before.distance(after) < 0.01, "{before:?} → {after:?}");
        // Off: the view turns about its own centre, so the box moves.
        a.display.settings.drawing.orbit_selected = false;
        a.navigate(input::NavigationKind::Orbit, vec2(60., 25.), r);
        assert!(on_screen(&a).distance(after) > 1.);
    }

    #[test]
    fn nothing_selected_orbits_as_usual() {
        let mut a = crate::tests::box_app();
        a.editor.dispatch(Command::Mode(Mode::Item)).unwrap();
        a.editor.dispatch(Command::SelectItem(None)).unwrap();
        assert_eq!(selection_centre(&a.editor.scene), None);
        let r = Rect::from_min_size(Pos2::ZERO, vec2(600., 400.));
        a.display.settings.drawing.orbit_selected = true;
        let target = a.camera.target;
        a.navigate(input::NavigationKind::Orbit, vec2(60., 25.), r);
        assert_eq!(a.camera.target, target);
    }
}
