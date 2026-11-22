//! Symmetry in the viewport (symmetry-2026-11-13): the seam snap for
//! placed points (SYM3-23) and the status-bar notices (SYM3-25, SYM3-40,
//! SS-SYM-04/08). The geometry rules live in `operandi_core::symmetry`.
use super::*;

/// Past this |cos| between the way to the symmetry plane and the view ray
/// (about 11°), the plane faces the camera and the seam snap is skipped.
const SEAM_FACING: f64 = 0.98;

impl Operandi {
    /// `point` (world) moved onto the active mesh's symmetry plane when it
    /// is within Symmetry Seam Snap (snapping preferences, 8 px by default)
    /// of it on screen (the snap service's `Role::SymmetrySeam`); unchanged
    /// with Symmetry off or farther away.
    pub(super) fn symmetry_seam_snap(&self, point: DVec3, r: Rect) -> DVec3 {
        let symmetry = self.editor.active_symmetry();
        if symmetry.axis.is_none() {
            return point;
        }
        let Some(item) = self.editor.scene.active() else {
            return point;
        };
        let Ok(world) = self.editor.scene.world_matrix(item.id) else {
            return point;
        };
        if world.determinant().abs() < 1e-18 {
            return point;
        }
        let local = world.inverse().transform_point3(point);
        let on = world.transform_point3(symmetry.onto_plane(local));
        // Provisional (SYM3-Q-OP01): when the way to the plane runs along
        // the view (the plane faces the camera, as X symmetry does in the
        // Left view), every point sits "on" the seam on screen, so the
        // screen test says nothing. Keep the point; the plane is only
        // snapped to where its distance shows on screen.
        let offset = on - point;
        if offset.length_squared() > 1e-24
            && let Some((a, _)) = self.camera.project(point, r)
        {
            let (_, ray) = self.camera.ray(a, r);
            if offset.normalize().dot(ray).abs() > SEAM_FACING {
                return point;
            }
        }
        let projector = self.camera.projector(r);
        let seam = crate::snap_service::Role::SymmetrySeam;
        if crate::snap_service::within(&projector, &self.snapping, seam, point, on) {
            on
        } else {
            point
        }
    }

    /// Show Symmetry messages in the status bar: a one-off note from the
    /// last creation (crossing the plane), and the open tool's notice when
    /// it changes ("<Tool> ignores Symmetry for now", a symmetric sketch's
    /// plane), so no tool ignores the setting silently.
    pub(super) fn sync_symmetry_notice(&mut self) {
        if let Some(note) = self.editor.symmetry_note.take() {
            self.status = note;
        }
        let notice = self.editor.symmetry_notice();
        if notice != self.symmetry_notice {
            if let Some(text) = &notice {
                self.status = text.clone();
            }
            self.symmetry_notice = notice;
        }
        // Selection inference says what a tool started from another mode
        // works on, never silently (SI-08).
        let inference = self.editor.inference_note().map(str::to_owned);
        if inference != self.inference_notice {
            if let Some(text) = &inference {
                self.status = text.clone();
            }
            self.inference_notice = inference;
        }
        // BN-45 (BH-Q12, provisional): Bend with its axis along the spine.
        // A script's plain "Ready" doesn't hide it.
        let bend = self.editor.bend_notice();
        if bend != self.bend_notice || bend.is_some() && self.status == "Ready" {
            if let Some(text) = bend {
                self.status = text.into();
            }
            self.bend_notice = bend;
        }
    }

    /// The seam guide (section 8, Q1): a square on the active mesh's
    /// symmetry plane, centred on the mesh and a little larger than it, in
    /// world space. Empty with Symmetry off, no visible mesh, or this
    /// view's Show Symmetry Plane off.
    pub(super) fn symmetry_seam_guide(&self) -> Vec<[DVec3; 4]> {
        if !self.display.settings.drawing.symmetry_plane {
            return Vec::new();
        }
        let symmetry = self.editor.active_symmetry();
        let Some(n) = symmetry.normal() else {
            return Vec::new();
        };
        let Some(item) = self
            .editor
            .scene
            .active()
            .filter(|i| self.editor.scene.shows(i))
        else {
            return Vec::new();
        };
        let Ok(world) = self.editor.scene.world_matrix(item.id) else {
            return Vec::new();
        };
        // Remembered per mesh allocation, so no per-frame point scan.
        let Some((lo, hi)) = item
            .mesh
            .shared()
            .bounds()
            .filter(|(lo, hi)| (*hi - *lo).is_finite())
        else {
            return Vec::new();
        };
        let centre = symmetry.onto_plane((lo + hi) / 2.);
        let half = ((hi - lo).length() * 0.55).max(0.05);
        let u = n.any_orthonormal_vector();
        let v = n.cross(u);
        let corner = |a: f64, b: f64| world.transform_point3(centre + (u * a + v * b) * half);
        vec![[
            corner(-1., -1.),
            corner(1., -1.),
            corner(1., 1.),
            corner(-1., 1.),
        ]]
    }
}
