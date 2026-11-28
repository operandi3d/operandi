//! Tack Tool and Retopo Sketch in the viewport (TOPOLOGY-CLEANUP
//! TC-20..22, TC-50..52; observed hands-on 2026-11-17). Core:
//! `align_tools::Tack`, curves through `Command::SetCurve`. Topology Pen's
//! gestures are in `topology_pen_ui.rs`.
use super::*;
use crate::create_utility::{Utility, UtilityDrag};
use crate::tool_form::{self as tf, ToolForm};
use operandi_core::{CurveEdit, CurveKind, Tack, TackOptions};

/// TC-52: about 3 cm between the points of a Retopo Sketch curve.
pub(crate) const RETOPO_SPACING: f64 = 0.03;

pub(super) fn retopo_sample_is_spaced(last: Option<&DVec3>, hit: DVec3) -> bool {
    last.is_none_or(|last| last.distance(hit) >= RETOPO_SPACING)
}

/// Tack's Tool Properties (TC-21; Copy Geometry, Make Bridge and Mode
/// aren't built yet, UIC-117).
pub(super) fn tack_form(f: &mut ToolForm<Utility>) {
    let read = |u: &Utility| match u {
        Utility::Tack(o) => *o,
        _ => TackOptions::default(),
    };
    fn edit(u: &mut Utility, change: impl Fn(&mut TackOptions)) {
        if let Utility::Tack(o) = u {
            change(o);
        }
    }
    use crate::form::Unit;
    f.field(
        tf::check(
            "tack_align",
            "Align",
            move |u: &Utility| read(u).align,
            |u, on| edit(u, |o| o.align = on),
        )
        .tip("Also turn the selection so its face lies flat on the clicked polygon"),
    )
    .field(tf::number(
        "tack_offset",
        "Offset",
        Unit::Distance,
        move |u: &Utility| read(u).offset,
        |u, v| {
            edit(u, |o| o.offset = v);
            Ok(())
        },
    ))
    .field(tf::percent(
        "tack_size",
        "Size",
        " %",
        move |u: &Utility| read(u).size,
        |u, v| {
            if v <= 0. {
                return Err("Size must be more than 0 %".into());
            }
            edit(u, |o| o.size = v / 100.);
            Ok(())
        },
    ))
    .field(
        tf::number(
            "tack_twist",
            "Rotate",
            Unit::Angle,
            move |u: &Utility| read(u).twist,
            |u, v| {
                edit(u, |o| o.twist = v);
                Ok(())
            },
        )
        .tip("An extra turn about the clicked polygon's normal"),
    )
    .field(
        tf::check(
            "tack_connected",
            "Move Connected Vertices",
            move |u: &Utility| read(u).connected,
            |u, on| edit(u, |o| o.connected = on),
        )
        .tip("Carry everything connected to the selection, not just the selection"),
    )
    .field(
        tf::check(
            "tack_copy",
            "Copy Geometry",
            move |u: &Utility| read(u).copy,
            |u, on| edit(u, |o| o.copy = on),
        )
        .tip("Leave the original and tack a copy (Shift+click does this once)"),
    )
    .field(
        tf::check(
            "tack_bridge",
            "Make Bridge",
            move |u: &Utility| read(u).bridge,
            |u, on| edit(u, |o| o.bridge = on),
        )
        .tip("Bridge onto a polygon of the same mesh; onto another mesh it does nothing (TC-22)"),
    )
    .note("Select the polygon to move, then left-click a polygon (its front side) to put it there. Shift+left-click tacks a copy.");
}

impl Operandi {
    /// TC-22: a left click on a polygon's front side tacks the selection
    /// onto that polygon's centre. Back sides and the active mesh's own
    /// polygons don't count (TC-Q5, provisional).
    pub(super) fn tack_press(&mut self, mut options: TackOptions, pos: Pos2, r: Rect, shift: bool) {
        options.copy |= shift;
        let surface = self.pick_surface();
        let Some((item, face, _)) = surface.pick(&self.camera, pos, r) else {
            self.status = "Tack Tool: click a polygon to tack onto".into();
            return;
        };
        // TC-22: Make Bridge onto another item does nothing at all.
        if options.bridge && self.editor.scene.active().is_some_and(|a| a.id != item) {
            self.status =
                "Tack Tool: Make Bridge only works onto the same mesh; nothing changed".into();
            return;
        }
        if self.editor.scene.active().is_some_and(|a| a.id == item) {
            self.status =
                "Tack Tool: click a polygon of another mesh (the active one is what moves)".into();
            return;
        }
        let Some(it) = self.editor.scene.item(item) else {
            return;
        };
        let Ok(world) = self.editor.scene.world_matrix(item) else {
            return;
        };
        let mesh = &it.mesh;
        if mesh.is_polyline(face as usize) {
            self.status = "Tack Tool: click a polygon, not a curve".into();
            return;
        }
        let normal = world
            .transform_vector3(mesh.normal(face as usize))
            .normalize_or_zero();
        let (_, dir) = self.camera.ray(pos, r);
        if normal.dot(dir) >= 0. {
            self.status = "Tack Tool: that's the polygon's back; click its front side".into();
            return;
        }
        let target = world.transform_point3(mesh.center(face as usize));
        if self.run_ok(Command::Tack(Tack {
            target,
            normal,
            options,
        })) {
            self.status = "Tack Tool: tacked".into();
        }
    }

    /// Retopo Sketch: start a stroke at the press.
    pub(super) fn begin_retopo_sketch(&mut self, pos: Pos2, r: Rect) -> input::GestureKind {
        self.retopo_stroke.clear();
        self.retopo_sketch_drag(pos, r);
        input::GestureKind::Utility(UtilityDrag::RetopoSketch)
    }

    /// TC-52: add the background point under the pointer once it is about
    /// 3 cm from the last one.
    pub(super) fn retopo_sketch_drag(&mut self, pos: Pos2, r: Rect) {
        let (origin, dir) = self.camera.ray(pos, r);
        let Some(hit) = self.editor.background_hit(origin, dir) else {
            return;
        };
        if retopo_sample_is_spaced(self.retopo_stroke.last(), hit) {
            self.retopo_stroke.push(hit);
        }
    }

    /// The stroke becomes one open Curve in the active mesh (TC-52).
    pub(super) fn retopo_sketch_release(&mut self) {
        let points = std::mem::take(&mut self.retopo_stroke);
        if points.len() < 2 {
            self.status = if self
                .editor
                .background_hit(DVec3::ZERO, DVec3::ZERO)
                .is_none()
                && points.is_empty()
            {
                "Retopo Sketch: drag across a background mesh (a shown mesh that isn't selected)"
                    .into()
            } else {
                "Retopo Sketch: drag further to draw a curve".into()
            };
            return;
        }
        let count = points.len();
        if self.run_ok(Command::SetCurve(Box::new(CurveEdit {
            item: None,
            face: None,
            points,
            closed: false,
            kind: CurveKind::Curve {
                start_control: false,
                end_control: false,
            },
            label: "Retopo Sketch".into(),
        }))) {
            self.status = format!("Retopo Sketch: a curve of {count} points");
        }
    }

    /// The stroke so far.
    pub(super) fn draw_retopo_stroke(&self, painter: &egui::Painter, r: Rect) {
        let points: Vec<Pos2> = self
            .retopo_stroke
            .iter()
            .filter_map(|&p| self.camera.project(p, r).map(|(q, _)| q))
            .collect();
        if points.len() >= 2 {
            painter.add(egui::Shape::line(
                points,
                Stroke::new(2_f32, crate::theme::viewport::selection()),
            ));
        }
    }

    /// Whether the open tool preset is Topology Pen.
    pub(super) fn topology_pen_open(&self) -> bool {
        self.editor
            .session
            .preset
            .as_ref()
            .is_some_and(|p| p.row().actor.topology_pen)
    }

    /// Retopo Edge Lock / Retopo Edge Flow (TC-53): add the selected
    /// polygons (the guide curves) to the named polygon selection set.
    pub(super) fn retopo_set(&mut self, name: &str) {
        let s = &self.editor.scene.selection;
        if s.mode != Mode::Polygon || s.faces.is_empty() {
            self.status = format!(
                "{}: select the guide curves first (Polygon mode)",
                if name == "RetopoEdgeFlow" {
                    "Retopo Edge Flow"
                } else {
                    "Retopo Edge Lock"
                }
            );
            return;
        }
        if self.run_ok(Command::AssignSelectionSet(name.into())) {
            self.status = format!("Added to {name}");
        }
    }
}
