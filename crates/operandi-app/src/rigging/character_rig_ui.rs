//! Character rigs in the app (BEHAVIOR-ACS-FEATURES, first milestone):
//! guides drawn in the views and dragged to fit a character (ACS-30/31),
//! while the rig's Guide context is on. The Auto Rig buttons are on the
//! Rig tab (toolbox.rig.json, UIC-190); each runs one `operandi.acs.*`
//! line (UIC-160), and a guide drag folds into one undo step (UIC-161).
use super::*;
use crate::camera_light_ui::Mark;
use operandi_core::character_rig::{self as cr, Side};

/// Screen distance (points) within which a press picks a guide.
const PICK: f32 = 8.;

/// A guide's shape: a wire diamond `size` across, plus a dotted line to
/// its parent guide.
pub(crate) fn guide_marks(at: DVec3, parent: Option<DVec3>, size: f64) -> Vec<Mark> {
    let h = size * 0.5;
    let tips = [
        DVec3::X * h,
        -DVec3::X * h,
        DVec3::Y * h,
        -DVec3::Y * h,
        DVec3::Z * h,
        -DVec3::Z * h,
    ];
    let mut out = Vec::new();
    for a in 0..2 {
        for b in 2..6 {
            out.push(Mark::Line([at + tips[a], at + tips[b]]));
        }
    }
    for (a, b) in [(2, 4), (4, 3), (3, 5), (5, 2)] {
        out.push(Mark::Line([at + tips[a], at + tips[b]]));
    }
    if let Some(p) = parent {
        out.push(Mark::Dotted([at, p]));
    }
    out
}

/// A guide's colour by its side (ACS-11's default scheme, PROVISIONAL):
/// left green, right red, centre blue, like IK goals (the axis colours).
pub(crate) fn guide_color(scene: &operandi_core::Scene, id: u64) -> Color32 {
    let [red, green, blue] = theme::viewport::grid_axes();
    match scene
        .character_rig_of(id)
        .and_then(|c| c.role_of(id))
        .map(cr::side_of)
    {
        Some(Side::Left) => green,
        Some(Side::Right) => red,
        _ => blue,
    }
}

/// The marks for guide `id`, `size` across.
pub(crate) fn guide_item_marks(scene: &operandi_core::Scene, id: u64, size: f64) -> Vec<Mark> {
    let at = cr::world_pos(scene, id);
    let parent = scene
        .item(id)
        .and_then(|i| i.parent)
        .filter(|p| {
            scene
                .item(*p)
                .is_some_and(|i| i.locator == Some(operandi_core::locator::LocatorKind::Guide))
        })
        .map(|p| cr::world_pos(scene, p));
    guide_marks(at, parent, size)
}

impl Operandi {
    /// The shown guide within `PICK` of `pos`, nearest first.
    fn pick_guide(&self, pos: Pos2, r: Rect) -> Option<u64> {
        let scene = &self.editor.scene;
        let projector = self.camera.projector(r);
        scene
            .character_rigs()
            .iter()
            .filter(|c| c.context.shows_guides())
            .flat_map(|c| c.guides.iter().map(|p| p.item))
            .filter(|&g| scene.drawn_in_viewport(g, self.display.settings.drawing.locators))
            .filter_map(|g| {
                let (p, _) = projector.project(cr::world_pos(scene, g))?;
                let d = p.distance(pos);
                (d <= PICK).then_some((d, g))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, g)| g)
    }

    /// ACS-30: with no tool active, a press on a shown guide selects it
    /// and drags it in the plane facing the view; its moves become one
    /// step on release.
    pub(super) fn rig_guide_press(&mut self, pos: Pos2, r: Rect) -> Option<input::GestureKind> {
        if self.editor.session.live.is_some() {
            return None;
        }
        let g = self.pick_guide(pos, r)?;
        let mark = self.editor.history_mark();
        self.editor.scene.select_item(Some(g));
        Some(input::GestureKind::Skeleton(
            crate::skeleton_tool_ui::SkelDrag {
                joint: g,
                mark,
                label: "Move Guide",
                anchor: cr::world_pos(&self.editor.scene, g),
                normal: self.camera.ray(pos, r).1,
                guide: true,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_marks_are_a_diamond_with_a_line_to_the_parent() {
        let marks = guide_marks(DVec3::ZERO, Some(DVec3::Y), 1.);
        let lines = marks.iter().filter(|m| matches!(m, Mark::Line(_))).count();
        assert_eq!(lines, 12, "an octahedron's edges");
        assert!(marks.contains(&Mark::Dotted([DVec3::ZERO, DVec3::Y])));
        assert_eq!(guide_marks(DVec3::ZERO, None, 1.).len(), 12);
    }

    #[test]
    fn a_press_on_a_guide_drags_it_and_the_drag_is_one_step() {
        let mut a = crate::tests::box_app();
        a.execute("operandi.acs.rigAdd");
        let knee = a.editor.scene.character_rigs()[0]
            .guide("leg.L.knee")
            .unwrap();
        let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
        let at = cr::world_pos(&a.editor.scene, knee);
        let (screen, _) = a.camera.projector(r).project(at).unwrap();
        let steps = a.editor.history.undo.len();
        let Some(input::GestureKind::Skeleton(d)) = a.rig_guide_press(screen, r) else {
            panic!("the press should pick the knee guide");
        };
        assert!(d.guide);
        assert_eq!(a.editor.scene.selection.item, Some(knee));
        a.skeleton_drag(&d, screen, screen + vec2(20., 0.), r);
        a.skeleton_drag(&d, screen, screen + vec2(40., -10.), r);
        a.skeleton_release(&d);
        assert_eq!(a.editor.history.undo.len(), steps + 1, "one undo step");
        let moved = cr::world_pos(&a.editor.scene, knee);
        assert!(moved.distance(at) > 1e-3);
        // Symmetry: the right knee followed, mirrored.
        let right = a.editor.scene.character_rigs()[0]
            .guide("leg.R.knee")
            .unwrap();
        let rp = cr::world_pos(&a.editor.scene, right);
        assert!((rp - DVec3::new(-moved.x, moved.y, moved.z)).length() < 1e-9);
        // Not in the Form context: guides are hidden and not picked.
        a.execute("operandi.acs.context context:form");
        assert!(a.rig_guide_press(screen, r).is_none());
    }
}
