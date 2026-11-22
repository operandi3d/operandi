//! Guides in the app (G9 Part B, GD-0..GD-3): the Guides tool (Edit ▸
//! Snapping ▸ Guides, or the `Guides` command), its Tool Properties
//! (Current Guide and the guide's settings) and the drag that draws a
//! guide, plus the guide lines every view draws.
//!
//! PROVISIONAL (GD-Q1): the Guides tool is a utility like the Ruler, so it
//! takes the viewport while it's on (the guides themselves draw and snap
//! with any tool). A drag draws the current guide as Two Points from where
//! it starts to where it ends, on the work plane or snapped to a point,
//! and turns Enable and Show on; a locked guide refuses the drag. Each
//! change is one undo step.
use super::*;
use crate::create_utility::{Utility, UtilityDrag};
use crate::form::Unit;
use crate::tool_form::{self as tf, ToolForm};
use operandi_core::guides::{COUNT, Guide, GuideMode, GuidePlane};

/// The Guides tool's state: which guide it edits and that guide's values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct GuideEdit {
    pub current: usize,
    pub guide: Guide,
}

impl Operandi {
    /// The Guides tool on the scene's current guide.
    pub(super) fn guides_utility(&self) -> Utility {
        let g = &self.editor.scene.guides;
        Utility::Guides(GuideEdit {
            current: g.current,
            guide: g.guides[g.current.min(COUNT - 1)],
        })
    }

    /// Tool Properties changed the edit: the scene takes it (one undo step),
    /// or a new Current Guide loads that guide.
    pub(super) fn publish_guide(&mut self, edit: GuideEdit) {
        let mut guides = self.editor.scene.guides.clone();
        let edit = if edit.current != guides.current {
            guides.current = edit.current.min(COUNT - 1);
            GuideEdit {
                current: guides.current,
                guide: guides.guides[guides.current],
            }
        } else {
            guides.guides[guides.current] = edit.guide;
            edit
        };
        if guides != self.editor.scene.guides {
            self.run(Command::SetGuides(Box::new(guides)));
        }
        self.utility = Some(Utility::Guides(edit));
    }

    /// A press with the Guides tool: draw the current guide from here.
    pub(super) fn guide_press(
        &mut self,
        edit: GuideEdit,
        pos: Pos2,
        r: Rect,
    ) -> input::GestureKind {
        if edit.guide.lock {
            self.status = format!(
                "Guide {} is locked: untick Lock Guide to move it",
                edit.current
            );
            return input::GestureKind::Blocked;
        }
        let plane = self.actual_plane();
        let start = self
            .snap_scene_point(pos, crate::snap_service::Role::RulerEnd, r, &|_| false)
            .map(|h| h.hit.point)
            .or_else(|| self.work_point(pos, r, plane, self.workplane_center()));
        match start {
            Some(start) => input::GestureKind::Utility(UtilityDrag::Guide(start)),
            None => {
                self.status = "View is parallel to the construction plane".into();
                input::GestureKind::Blocked
            }
        }
    }

    pub(super) fn guide_drag(&mut self, start: DVec3, pos: Pos2, r: Rect) {
        let Some(Utility::Guides(mut edit)) = self.utility else {
            return;
        };
        let plane = self.actual_plane();
        let end = self
            .snap_scene_point(pos, crate::snap_service::Role::RulerEnd, r, &|_| false)
            .map(|h| h.hit.point)
            .or_else(|| self.work_point(pos, r, plane, start));
        let Some(end) = end else {
            return;
        };
        edit.guide.mode = GuideMode::TwoPoints;
        edit.guide.position = start;
        edit.guide.end = end;
        edit.guide.enabled = true;
        edit.guide.show = true;
        self.utility = Some(Utility::Guides(edit));
        self.status = format!(
            "Guide {} · {}",
            edit.current,
            workspace::number_text((end - start).length(), " m")
        );
    }

    pub(super) fn guide_release(&mut self) {
        if let Some(Utility::Guides(edit)) = self.utility {
            self.publish_guide(edit);
        }
    }

    /// GD-1: every guide with Enable and Show on, a thin line in the guide
    /// colour; the one being drawn or edited follows the tool.
    pub(super) fn draw_guides(&self, painter: &egui::Painter, r: Rect) {
        let mut lines = self.editor.scene.guides.drawn();
        if let Some(Utility::Guides(edit)) = self.utility
            && edit.guide.drawn()
        {
            // The tool's own (perhaps not yet committed) version.
            let g = &self.editor.scene.guides.guides[edit.current.min(COUNT - 1)];
            if g.drawn()
                && let Some(i) = lines.iter().position(|s| *s == g.segment())
            {
                lines.remove(i);
            }
            lines.push(edit.guide.segment());
        }
        // A guide colour of its own (provisional: no theme slot yet).
        let colour = Color32::from_rgb(90, 200, 235);
        for (a, b) in lines {
            // A guide runs on past its ends a little, as a construction line.
            if let (Some((p, _)), Some((q, _))) =
                (self.camera.project(a, r), self.camera.project(b, r))
            {
                painter.line_segment([p, q], Stroke::new(1f32, colour));
                painter.circle_filled(p, 2.5, colour);
                painter.circle_filled(q, 2.5, colour);
            }
        }
    }
}

/// The Guides tool's Tool Properties (GD-1).
pub(super) fn guides_form(f: &mut ToolForm<Utility>, current: &GuideEdit) {
    let read = |u: &Utility| match u {
        Utility::Guides(e) => *e,
        _ => GuideEdit {
            current: 0,
            guide: Guide::default(),
        },
    };
    fn edit(u: &mut Utility, f: impl FnOnce(&mut GuideEdit)) {
        if let Utility::Guides(e) = u {
            f(e);
        }
    }
    let choices: Vec<(usize, String)> = (0..COUNT).map(|i| (i, format!("Guide {i}"))).collect();
    let choices: Vec<(usize, &str)> = choices.iter().map(|(i, s)| (*i, s.as_str())).collect();
    f.field(tf::choice(
        "gd_current",
        "Current Guide",
        &choices,
        move |u: &Utility| read(u).current,
        |u, i| edit(u, |e| e.current = i),
    ))
    .field(tf::check(
        "gd_enable",
        "Enable Guide",
        move |u: &Utility| read(u).guide.enabled,
        |u, on| edit(u, |e| e.guide.enabled = on),
    ))
    .field(tf::check(
        "gd_show",
        "Show Guide",
        move |u: &Utility| read(u).guide.show,
        |u, on| edit(u, |e| e.guide.show = on),
    ))
    .field(
        tf::check(
            "gd_snap",
            "Snap to Guide",
            move |u: &Utility| read(u).guide.snap,
            |u, on| edit(u, |e| e.guide.snap = on),
        )
        .tip("Points dragged by any tool snap to this guide while snapping is on"),
    )
    .field(
        tf::check(
            "gd_lock",
            "Lock Guide",
            move |u: &Utility| read(u).guide.lock,
            |u, on| edit(u, |e| e.guide.lock = on),
        )
        .tip("A locked guide can't be moved"),
    )
    .field(tf::choice(
        "gd_mode",
        "Mode",
        &[
            (GuideMode::TwoPoints, "Two Points"),
            (GuideMode::PlanarRay, "Planar Ray"),
        ],
        move |u: &Utility| read(u).guide.mode,
        |u, m| edit(u, |e| e.guide.mode = m),
    ));
    let locked = current.guide.lock;
    let why = "The guide is locked";
    let position = tf::vector(
        "gd_position",
        "Position",
        Unit::Distance,
        move |u: &Utility| read(u).guide.position,
        |u, a, v| {
            edit(u, |e| e.guide.position[a] = v);
            Ok(())
        },
    );
    f.fields_from(
        position
            .into_iter()
            .map(|x| x.enabled_if(!locked, why))
            .collect(),
    );
    if current.guide.mode == GuideMode::TwoPoints {
        let end = tf::vector(
            "gd_end",
            "End",
            Unit::Distance,
            move |u: &Utility| read(u).guide.end,
            |u, a, v| {
                edit(u, |e| e.guide.end[a] = v);
                Ok(())
            },
        );
        f.fields_from(
            end.into_iter()
                .map(|x| x.enabled_if(!locked, why))
                .collect(),
        );
    } else {
        let planes: Vec<(GuidePlane, &str)> =
            GuidePlane::ALL.iter().map(|p| (*p, p.label())).collect();
        f.field(
            tf::choice(
                "gd_plane",
                "Plane",
                &planes,
                move |u: &Utility| read(u).guide.plane,
                |u, p| edit(u, |e| e.guide.plane = p),
            )
            .enabled_if(!locked, why),
        )
        .field(
            tf::number(
                "gd_angle",
                "Angle",
                Unit::Angle,
                move |u: &Utility| read(u).guide.angle,
                |u, v| {
                    edit(u, |e| e.guide.angle = v);
                    Ok(())
                },
            )
            .enabled_if(!locked, why),
        )
        .field(
            tf::number(
                "gd_distance",
                "Distance",
                Unit::Distance,
                move |u: &Utility| read(u).guide.distance,
                |u, v| {
                    if v < 0. {
                        return Err("Distance can't be negative".into());
                    }
                    edit(u, |e| e.guide.distance = v);
                    Ok(())
                },
            )
            .enabled_if(!locked, why),
        );
    }
    f.note("Drag in the viewport to draw the current guide (Two Points). Guides draw and snap with any tool.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snap_service::Role;
    use operandi_core::guides::Guides;

    fn app() -> (Operandi, Rect) {
        let mut a = crate::tests::box_app();
        a.camera.view = crate::camera::View::Front;
        a.camera.target = DVec3::ZERO;
        a.camera.distance = 12.;
        (a, Rect::from_min_size(Pos2::ZERO, vec2(800., 600.)))
    }

    fn guide_line(snap: bool) -> Guides {
        let mut g = Guides::default();
        g.guides[0] = Guide {
            enabled: true,
            show: true,
            snap,
            position: DVec3::new(-4., 3., 0.),
            end: DVec3::new(4., 3., 0.),
            ..Default::default()
        };
        g
    }

    /// GD-2: with Snap to Guide on, a point near the guide lands on it;
    /// off, nothing is there to snap to.
    #[test]
    fn points_snap_to_a_guide_with_snap_on() {
        let (mut a, r) = app();
        let near = a.camera.project(DVec3::new(1., 3., 0.), r).unwrap().0 + vec2(0., 3.);
        a.run(Command::SetGuides(Box::new(guide_line(false))));
        assert!(
            a.snap_scene_point(near, Role::RulerEnd, r, &|_| false)
                .is_none(),
            "Snap to Guide off"
        );
        a.run(Command::SetGuides(Box::new(guide_line(true))));
        let hit = a
            .snap_scene_point(near, Role::RulerEnd, r, &|_| false)
            .expect("the guide is a target");
        assert!((hit.hit.point.y - 3.).abs() < 1e-6, "{}", hit.hit.point);
        assert!((hit.hit.point.x - 1.).abs() < 0.05, "{}", hit.hit.point);
    }

    /// GD-3: guides save with the scene (native version 59) and reopen.
    #[test]
    fn guides_save_and_reopen() {
        let (mut a, _) = app();
        a.run(Command::SetGuides(Box::new(guide_line(true))));
        let bytes = operandi_core::persistence::encode(&a.editor.scene).unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 59"));
        let back = operandi_core::persistence::decode(&bytes).unwrap();
        assert_eq!(back.guides, a.editor.scene.guides);
    }

    /// GD-1: a drag with the Guides tool draws the current guide, one undo
    /// step; a locked guide refuses.
    #[test]
    fn the_guides_tool_draws_the_current_guide() {
        let (mut a, r) = app();
        a.utility_by_name("guides");
        let Some(Utility::Guides(edit)) = a.utility else {
            panic!("the Guides tool is on");
        };
        let from = a.camera.project(DVec3::new(-2., 2., 0.), r).unwrap().0;
        let to = a.camera.project(DVec3::new(2., 2.5, 0.), r).unwrap().0;
        let input::GestureKind::Utility(UtilityDrag::Guide(start)) = a.guide_press(edit, from, r)
        else {
            panic!("a guide drag starts");
        };
        let steps = a.editor.undo.len();
        a.guide_drag(start, to, r);
        a.guide_release();
        let g = a.editor.scene.guides.guides[0];
        assert!(g.enabled && g.show);
        assert!((g.end - g.position).length() > 3.);
        assert_eq!(a.editor.undo.len(), steps + 1);
        a.run(Command::Undo);
        assert!(!a.editor.scene.guides.guides[0].enabled);
        // Locked: refused.
        let mut locked = guide_line(false);
        locked.guides[0].lock = true;
        a.run(Command::SetGuides(Box::new(locked)));
        // (Turning the tool on again would toggle it off.)
        a.utility = Some(a.guides_utility());
        let Some(Utility::Guides(edit)) = a.utility else {
            panic!("the Guides tool is on");
        };
        assert!(matches!(
            a.guide_press(edit, from, r),
            input::GestureKind::Blocked
        ));
    }
}
