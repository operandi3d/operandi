//! The drag readout (Jason, 2026-11-21; provisional, not from a spec): while
//! a Move, Rotate, Scale or Transform handle is dragged, a small label next
//! to the handle shows the tool's current values, per axis: Scale in
//! percent ("Y 145.0 %"), Rotate in degrees, Move in the scene's distance
//! unit. Values are written with their unit as fields are (UIC-113). When
//! the drag stops the label fades out over `FADE_SECONDS`. It is drawn on
//! top of the view and takes no input, like the field of view readout
//! (view_fov.rs). New viewport feedback [OP]: not in the UI bible yet.
use crate::Operandi;
use crate::workspace::number_text;
use eframe::egui::{Color32, FontId, Painter, Rect, vec2};
use glam::DVec3;
use operandi_core::{Parameters, Tool};

/// How long the label takes to fade out after the drag stops, in seconds.
pub const FADE_SECONDS: f64 = 1.0;

/// The label of the current or last transform drag.
#[derive(Clone, Debug, PartialEq)]
pub struct DragReadout {
    /// The view the drag is in (only that view draws it).
    pub view: u32,
    pub text: String,
    /// Where the handle is (world); the label sits beside it.
    pub at: DVec3,
    /// When the drag stopped (egui time); `None` while it goes on.
    pub released: Option<f64>,
}

/// The label for `tool`'s values; `family` picks Transform's part (0 Move,
/// 1 Rotate, 2 Scale, the handle that is dragged). `None` for tools that
/// have no readout.
pub fn readout_text(tool: Tool, family: usize, params: &Parameters) -> Option<String> {
    let (kind, values) = match tool {
        Tool::Move => (0, params.delta),
        Tool::Rotate => (1, params.delta),
        Tool::Scale => (2, params.delta),
        Tool::Transform => match family {
            0 => (0, params.delta),
            1 => (1, params.rotation_delta),
            _ => (2, params.scale_delta),
        },
        _ => return None,
    };
    let axis = |name: &str, v: f64| match kind {
        0 => format!("{name} {}", number_text(v, " m")),
        1 => format!("{name} {v:.1}°"),
        _ => format!("{name} {:.1} %", v * 100.),
    };
    Some(
        [("X", values.x), ("Y", values.y), ("Z", values.z)]
            .map(|(n, v)| axis(n, v))
            .join("   "),
    )
}

/// How opaque the label is `since` seconds after the drag stopped.
pub fn fade(since: f64) -> f32 {
    (1. - since / FADE_SECONDS).clamp(0., 1.) as f32
}

impl Operandi {
    /// After a handle drag step: refresh the label from the live tool.
    pub(super) fn note_drag_readout(&mut self, axis: usize, tool: Tool) {
        let family = if crate::viewport::combined_handles(tool) {
            axis / 3
        } else {
            0
        };
        let Some(live) = self.editor.session.live.as_ref().filter(|l| l.tool == tool) else {
            return;
        };
        let Some(text) = readout_text(tool, family, &live.params) else {
            return;
        };
        self.drag_readout = Some(DragReadout {
            view: self.drawing_view,
            text,
            at: self.transform_handle_center(),
            released: None,
        });
    }

    /// The drag stopped: start the fade.
    pub(super) fn release_drag_readout(&mut self, now: f64) {
        if let Some(readout) = &mut self.drag_readout
            && readout.released.is_none()
        {
            readout.released = Some(now);
        }
    }

    pub(super) fn draw_drag_readout(&self, painter: &Painter, r: Rect) {
        let Some(readout) = &self.drag_readout else {
            return;
        };
        // A drag that ended without a release (the tool dropped under it)
        // shows nothing.
        if readout.view != self.drawing_view
            || readout.released.is_none() && self.editor.session.live.is_none()
        {
            return;
        }
        let now = painter.ctx().input(|i| i.time);
        let alpha = readout.released.map_or(1., |at| fade(now - at));
        if alpha <= 0. {
            return;
        }
        let Some((at, _)) = self.camera.project(readout.at, r) else {
            return;
        };
        let text_colour = crate::theme::text().gamma_multiply(alpha);
        let galley =
            painter.layout_no_wrap(readout.text.clone(), FontId::proportional(13.), text_colour);
        // Below and right of the handle centre, kept inside the view.
        let size = galley.size();
        let mut corner = at + vec2(24., 24.);
        corner.x = corner.x.min(r.right() - size.x - 8.).max(r.left() + 8.);
        corner.y = corner.y.min(r.bottom() - size.y - 8.).max(r.top() + 8.);
        let back = Rect::from_min_size(corner, size).expand2(vec2(6., 3.));
        painter.rect_filled(back, 3., Color32::from_black_alpha((150. * alpha) as u8));
        painter.galley(corner, galley, text_colour);
        if readout.released.is_some() {
            painter.ctx().request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::View;
    use crate::tests::{draw, pointer};
    use eframe::egui::{Context, Event, Modifiers};
    use operandi_core::{Command, Mode};

    /// A real drag on the Scale tool's Y handle shows the label while it
    /// goes on, and the release starts the fade.
    #[test]
    fn a_scale_handle_drag_shows_the_label_then_fades() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
                .unwrap()
                .into();
        a.editor.scene.selection.mode = Mode::Polygon;
        a.editor.scene.selection.faces = vec![1];
        a.camera.view = View::Front;
        a.activate(Tool::Scale);
        let ctx = Context::default();
        crate::theme::install(&ctx);
        let size = vec2(1440., 900.);
        draw(&mut a, &ctx, size, vec![]);
        let r = a.viewport_rect;
        let (_, _, handle, _) = a
            .handles(r)
            .into_iter()
            .find(|h| h.0 == 1)
            .expect("Y handle");
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(handle)]);
        draw(
            &mut a,
            &ctx,
            size,
            vec![pointer(handle, true, Modifiers::NONE)],
        );
        let end = handle + vec2(0., -40.);
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(end)]);
        let during = a.drag_readout.clone().expect("label while dragging");
        assert_eq!(during.released, None);
        assert!(
            during.text.starts_with("X 100.0 %   Y 14"),
            "{}",
            during.text
        );
        draw(
            &mut a,
            &ctx,
            size,
            vec![pointer(end, false, Modifiers::NONE)],
        );
        assert!(a.drag_readout.as_ref().unwrap().released.is_some());
    }

    #[test]
    fn scale_reads_in_percent_per_axis() {
        let p = Parameters {
            delta: DVec3::new(1., 1.45, 1.),
            ..Default::default()
        };
        assert_eq!(
            readout_text(Tool::Scale, 0, &p).unwrap(),
            "X 100.0 %   Y 145.0 %   Z 100.0 %"
        );
    }

    #[test]
    fn rotate_reads_in_degrees_and_move_in_distance() {
        let p = Parameters {
            delta: DVec3::new(0., 30., 0.),
            ..Default::default()
        };
        assert_eq!(
            readout_text(Tool::Rotate, 0, &p).unwrap(),
            "X 0.0°   Y 30.0°   Z 0.0°"
        );
        let p = Parameters {
            delta: DVec3::new(0.25, 0., 0.),
            ..Default::default()
        };
        assert!(
            readout_text(Tool::Move, 0, &p)
                .unwrap()
                .starts_with("X 0.25 m")
        );
    }

    #[test]
    fn transform_reads_the_dragged_part_and_other_tools_have_none() {
        let p = Parameters {
            scale_delta: DVec3::new(2., 1., 1.),
            ..Default::default()
        };
        assert!(
            readout_text(Tool::Transform, 2, &p)
                .unwrap()
                .starts_with("X 200.0 %")
        );
        assert_eq!(readout_text(Tool::Bevel, 0, &p), None);
    }

    #[test]
    fn the_label_fades_out_after_the_drag() {
        assert_eq!(fade(0.), 1.);
        assert!((fade(FADE_SECONDS / 2.) - 0.5).abs() < 1e-6);
        assert_eq!(fade(FADE_SECONDS + 0.1), 0.);
    }
}
