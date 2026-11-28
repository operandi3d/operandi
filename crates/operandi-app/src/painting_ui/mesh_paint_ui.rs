//! Mesh Paint and Mesh Paint Instance in the app (missing-tools-g6
//! G6-MESH-PAINT v1): the placer. A Create-tab utility like Retopo
//! Sketch: presses and drags on the target (the active mesh) become
//! surface points, and each click or stroke is one core `MeshPaint`
//! command (one undo step, MP-16; operandi-core mesh_paint.rs places and
//! copies).
//!
//! Paint modes (MP-03..08): Slide (press, drag to slide, release; Ctrl
//! first: the drag sizes it), Strokes (copies scattered in the brush
//! circle along the stroke), Drop / Drop (no Scale) (click; drag scales and
//! turns it about the normal), Linear (A to B), Array (a rectangle's grid)
//! and Radial (a ring from the centre out to the drag point). Shift snaps
//! Linear and Radial to 45° steps and makes Array's rectangle square.
use super::*;
use crate::create_utility::{Utility, UtilityDrag};
use crate::tool_form::{self as tf, ToolForm};
use operandi_core::duplicate_items::ParentMode;
use operandi_core::mesh_paint::{
    GeometrySource, MeshPaintSettings, MeshPaintStroke, PaintEffect, PaintMode, PaintParameter,
    PaintPoint, Rng, RotationMode, SizeMode,
};

/// A Mesh Paint gesture in progress.
#[derive(Clone, Debug, Default)]
pub(super) struct Gesture {
    /// Where the press was, on screen.
    pub start: Pos2,
    /// The pointer now.
    pub at: Pos2,
    /// Ctrl was held at the press (Slide: the drag sizes the copy).
    pub sizing: bool,
    pub shift: bool,
    /// Strokes: the points so far and the last dab's screen place.
    pub points: Vec<PaintPoint>,
    pub last_dab: Option<Pos2>,
    pub seed: u64,
}

/// A fresh seed per stroke; it is stored in the command, so undo, redo and
/// reopen replay the same copies.
fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64)
}

impl Operandi {
    fn mesh_paint_settings(&self) -> Option<MeshPaintSettings> {
        match self.utility {
            Some(Utility::MeshPaint(s)) => Some(s),
            _ => None,
        }
    }

    /// The target's surface under `pos` (MP-02): point and normal.
    fn mesh_paint_hit(&self, pos: Pos2, r: Rect) -> Option<(DVec3, DVec3)> {
        let target = self.editor.scene.active()?.id;
        let (origin, dir) = self.camera.ray(pos, r);
        self.editor.scene.surface_hit(target, origin, dir)
    }

    /// The brush radius in world units at `at` (Adaptive size).
    fn brush_world(&self, s: &MeshPaintSettings, pos: Pos2, at: DVec3, r: Rect) -> f64 {
        let (o, a) = self.camera.ray(pos, r);
        let (_, b) = self.camera.ray(pos + vec2(s.brush_size as f32, 0.), r);
        let depth = (at - o).length();
        (a - b).length() * depth
    }

    /// A press with Mesh Paint on.
    pub(super) fn mesh_paint_press(
        &mut self,
        pos: Pos2,
        r: Rect,
        mods: Modifiers,
    ) -> input::GestureKind {
        let Some(s) = self.mesh_paint_settings() else {
            return input::GestureKind::Blocked;
        };
        if self.editor.scene.mesh_paint_sources().is_empty() {
            self.status = operandi_core::mesh_paint::NO_SOURCE.into();
            return input::GestureKind::Blocked;
        }
        self.mesh_paint = Gesture {
            start: pos,
            at: pos,
            sizing: mods.ctrl || mods.mac_cmd,
            shift: mods.shift,
            points: Vec::new(),
            last_dab: None,
            seed: seed(),
        };
        if s.mode == PaintMode::Strokes {
            self.mesh_paint_dab(pos, r);
        }
        input::GestureKind::Utility(UtilityDrag::MeshPaint)
    }

    pub(super) fn mesh_paint_drag(&mut self, pos: Pos2, r: Rect) {
        self.mesh_paint.at = pos;
        if self.mesh_paint_settings().map(|s| s.mode) == Some(PaintMode::Strokes) {
            self.mesh_paint_dab(pos, r);
        }
    }

    /// Strokes (MP-04): once the pointer has gone a brush radius, scatter
    /// copies at random in the brush circle (Brush Density: up to one per
    /// 10 % per dab, a reading); each point's falloff is 1 at the centre,
    /// 0 at the edge; the heading is the stroke's direction.
    fn mesh_paint_dab(&mut self, pos: Pos2, r: Rect) {
        let Some(s) = self.mesh_paint_settings() else {
            return;
        };
        let radius = s.brush_size.max(1.) as f32;
        if let Some(last) = self.mesh_paint.last_dab
            && last.distance(pos) < radius
        {
            return;
        }
        let heading = self.mesh_paint.last_dab.and_then(|last| {
            let a = self.mesh_paint_hit(last, r)?.0;
            let b = self.mesh_paint_hit(pos, r)?.0;
            (b - a).try_normalize()
        });
        self.mesh_paint.last_dab = Some(pos);
        let mut rng = Rng(self.mesh_paint.seed ^ (self.mesh_paint.points.len() as u64 + 1));
        let count = ((s.brush_density / 10.).round() as usize).max(1);
        for _ in 0..count {
            // Even over the circle's area.
            let d = rng.unit().sqrt();
            let a = rng.unit() * std::f64::consts::TAU;
            let at = pos + vec2((a.cos() * d) as f32, (a.sin() * d) as f32) * radius;
            if let Some((p, n)) = self.mesh_paint_hit(at, r) {
                let mut pt = PaintPoint::at(p, n);
                pt.falloff = 1. - d;
                pt.heading = heading;
                self.mesh_paint.points.push(pt);
            }
        }
    }

    /// Release: the click or stroke becomes one command.
    pub(super) fn mesh_paint_release(&mut self, r: Rect) {
        let Some(s) = self.mesh_paint_settings() else {
            return;
        };
        let g = std::mem::take(&mut self.mesh_paint);
        let (from, to) = (g.start, g.at);
        let screen_points: Vec<(Pos2, Option<Vec2>)> = match s.mode {
            PaintMode::Strokes => Vec::new(),
            PaintMode::Slide | PaintMode::Drop | PaintMode::DropNoScale => {
                let at = if s.mode == PaintMode::Slide && !g.sizing {
                    to
                } else {
                    from
                };
                vec![(at, None)]
            }
            PaintMode::Linear => {
                let to = if g.shift { snap45(from, to) } else { to };
                let n = s.linear_count.max(1);
                (0..n)
                    .map(|i| {
                        let t = if n == 1 {
                            0.
                        } else {
                            i as f32 / (n - 1) as f32
                        };
                        (from + (to - from) * t, Some(to - from))
                    })
                    .collect()
            }
            PaintMode::Array => {
                let mut to = to;
                if g.shift {
                    let d = to - from;
                    let side = d.x.abs().max(d.y.abs());
                    to = from + vec2(side * d.x.signum(), side * d.y.signum());
                }
                let [nx, ny] = s.array_count.map(|c| c.max(1));
                let mut out = Vec::new();
                for j in 0..ny {
                    for i in 0..nx {
                        let fx = if nx == 1 {
                            0.5
                        } else {
                            i as f32 / (nx - 1) as f32
                        };
                        let fy = if ny == 1 {
                            0.5
                        } else {
                            j as f32 / (ny - 1) as f32
                        };
                        out.push((
                            pos2(from.x + (to.x - from.x) * fx, from.y + (to.y - from.y) * fy),
                            None,
                        ));
                    }
                }
                out
            }
            PaintMode::Radial => {
                let to = if g.shift { snap45(from, to) } else { to };
                let v = to - from;
                let n = s.radial_count.max(1);
                let start = v.y.atan2(v.x);
                (0..n)
                    .map(|i| {
                        let a = start + std::f32::consts::TAU * i as f32 / n as f32;
                        (from + vec2(a.cos(), a.sin()) * v.length(), None)
                    })
                    .collect()
            }
        };
        let mut points = g.points;
        for (at, along) in screen_points {
            let Some((p, n)) = self.mesh_paint_hit(at, r) else {
                continue;
            };
            let mut pt = PaintPoint::at(p, n);
            if let Some(along) = along
                && let (Some(a), Some(b)) = (
                    self.mesh_paint_hit(from, r),
                    self.mesh_paint_hit(from + along, r),
                )
            {
                pt.heading = (b.0 - a.0).try_normalize();
            }
            let drag = to - from;
            match s.mode {
                // MP-05: the drag scales (by its length) and turns it.
                PaintMode::Drop | PaintMode::DropNoScale if drag.length() > 2. => {
                    pt.spin = -f64::from(drag.y.atan2(drag.x));
                    if s.mode == PaintMode::Drop {
                        pt.scale = 1. + f64::from(drag.length()) / 100.;
                    }
                }
                // MP-03: Ctrl first, the drag sizes it.
                PaintMode::Slide if g.sizing => {
                    pt.scale = (1. + f64::from(drag.x - drag.y) / 100.).max(0.01);
                }
                _ => {}
            }
            points.push(pt);
        }
        if points.is_empty() {
            self.status = "Mesh Paint: paint on the selected mesh's surface".into();
            return;
        }
        let (_, _, up, _) = self.camera.basis();
        let brush_world = self.brush_world(&s, from, points[0].position, r);
        let count = points.len();
        if self.run_ok(Command::MeshPaint(Box::new(MeshPaintStroke {
            settings: s,
            points,
            seed: g.seed,
            view_up: up,
            brush_world,
        }))) {
            self.status = format!(
                "{}: {count} placed",
                if s.replicas {
                    "Mesh Paint Instancer"
                } else if s.instances {
                    "Mesh Paint Instance"
                } else {
                    "Mesh Paint"
                }
            );
        }
    }

    /// The brush circle (Strokes) or the gesture's line, rectangle or ring.
    pub(super) fn draw_mesh_paint(&self, painter: &egui::Painter, hover: Option<Pos2>, _r: Rect) {
        let Some(s) = self.mesh_paint_settings() else {
            return;
        };
        let stroke = egui::Stroke::new(1.5_f32, theme::viewport::selection());
        let g = &self.mesh_paint;
        match s.mode {
            PaintMode::Strokes | PaintMode::Slide | PaintMode::Drop | PaintMode::DropNoScale => {
                if let Some(at) = hover {
                    painter.circle_stroke(at, s.brush_size as f32, stroke);
                }
            }
            PaintMode::Linear if g.seed != 0 => {
                painter.line_segment([g.start, g.at], stroke);
            }
            PaintMode::Array if g.seed != 0 => {
                painter.rect_stroke(
                    Rect::from_two_pos(g.start, g.at),
                    0.,
                    stroke,
                    egui::StrokeKind::Middle,
                );
            }
            PaintMode::Radial if g.seed != 0 => {
                painter.circle_stroke(g.start, g.start.distance(g.at), stroke);
            }
            _ => {}
        }
    }
}

/// The end turned to the nearest 45° on screen (Shift).
fn snap45(from: Pos2, to: Pos2) -> Pos2 {
    let v = to - from;
    let step = std::f32::consts::FRAC_PI_4;
    let a = (v.y.atan2(v.x) / step).round() * step;
    from + vec2(a.cos(), a.sin()) * v.length()
}

/// Mesh Paint's Tool Properties (the spec's settings).
pub(super) fn mesh_paint_form(f: &mut ToolForm<Utility>, s: &MeshPaintSettings) {
    fn read(u: &Utility) -> MeshPaintSettings {
        match u {
            Utility::MeshPaint(s) => *s,
            _ => MeshPaintSettings::default(),
        }
    }
    fn edit(u: &mut Utility, change: impl FnOnce(&mut MeshPaintSettings)) {
        if let Utility::MeshPaint(s) = u {
            change(s);
        }
    }
    fn count(v: f64, what: &str) -> std::result::Result<u32, String> {
        if v.fract() != 0. || !(1. ..=1000.).contains(&v) {
            return Err(format!("{what} must be a whole number from 1 to 1000"));
        }
        Ok(v as u32)
    }
    use crate::form::Unit;
    f.field(tf::choice(
        "mp-mode",
        "Paint Mode",
        &[
            (PaintMode::Slide, "Slide"),
            (PaintMode::Strokes, "Strokes"),
            (PaintMode::Drop, "Drop"),
            (PaintMode::DropNoScale, "Drop (no Scale)"),
            (PaintMode::Linear, "Linear"),
            (PaintMode::Array, "Array"),
            (PaintMode::Radial, "Radial"),
        ],
        |u: &Utility| read(u).mode,
        |u, m| edit(u, |s| s.mode = m),
    ))
    .field(tf::number(
        "mp_brush_size",
        "Brush Size",
        Unit::Suffix(" px"),
        |u: &Utility| read(u).brush_size,
        |u, v| {
            if v.is_nan() || v <= 0. {
                return Err("Brush Size must be more than 0".into());
            }
            edit(u, |s| s.brush_size = v);
            Ok(())
        },
    ))
    .field(tf::number(
        "mp_density",
        "Brush Density",
        Unit::Percent,
        |u: &Utility| read(u).brush_density,
        |u, v| {
            if !(0. ..=100.).contains(&v) {
                return Err("Brush Density is 0 to 100 %".into());
            }
            edit(u, |s| s.brush_density = v);
            Ok(())
        },
    ))
    .separator()
    .field(tf::choice(
        "mp-size-mode",
        "Size Mode",
        &[
            (SizeMode::Uniform, "Uniform"),
            (SizeMode::Adaptive, "Adaptive"),
            (SizeMode::Random, "Random"),
            (SizeMode::AdaptiveRandom, "Adaptive + Random"),
            (SizeMode::Locked, "Locked"),
        ],
        |u: &Utility| read(u).size_mode,
        |u, m| edit(u, |s| s.size_mode = m),
    ))
    .fields_from(tf::vector(
        "mp_size",
        "Size",
        Unit::Percent,
        |u: &Utility| read(u).size,
        |u, a, v| {
            edit(u, |s| s.size[a] = v);
            Ok(())
        },
    ))
    .field(tf::number(
        "mp_scale_increment",
        "Scale Increment",
        Unit::Percent,
        |u: &Utility| read(u).scale_increment,
        |u, v| {
            edit(u, |s| s.scale_increment = v.max(0.));
            Ok(())
        },
    ))
    .separator()
    .field(tf::choice(
        "mp-rotation-mode",
        "Rotation Mode",
        &[
            (RotationMode::Uniform, "Uniform"),
            (RotationMode::SurfaceAlign, "Surface Align"),
            (RotationMode::Random, "Random"),
            (RotationMode::SurfaceAlignRandom, "Surface Align + Random"),
            (RotationMode::SurfaceAlignTablet, "Surface Align + Tablet"),
            (
                RotationMode::SurfaceAlignTabletNoPitch,
                "Surface Align + Tablet (no Pitch)",
            ),
            (RotationMode::ScreenAlignTablet, "Screen Align + Tablet"),
            (
                RotationMode::ScreenAlignTabletNoPitch,
                "Screen Align + Tablet (no Pitch)",
            ),
        ],
        |u: &Utility| read(u).rotation_mode,
        |u, m| edit(u, |s| s.rotation_mode = m),
    ))
    .fields_from(tf::vector(
        "mp_rotation",
        "Rotation",
        Unit::Angle,
        |u: &Utility| read(u).rotation,
        |u, a, v| {
            edit(u, |s| s.rotation[a] = v);
            Ok(())
        },
    ))
    .field(tf::check(
        "mp_rotation_snap",
        "Rotation Snapping",
        |u: &Utility| read(u).rotation_snap,
        |u, on| edit(u, |s| s.rotation_snap = on),
    ))
    .field(
        tf::number(
            "mp_snap_angle",
            "Angle",
            Unit::Angle,
            |u: &Utility| read(u).snap_angle,
            |u, v| {
                if v.is_nan() || v <= 0. {
                    return Err("Angle must be more than 0°".into());
                }
                edit(u, |s| s.snap_angle = v);
                Ok(())
            },
        )
        .enabled_if(s.rotation_snap, "Turn Rotation Snapping on first"),
    )
    .field(tf::number(
        "mp_offset",
        "Offset",
        Unit::Distance,
        |u: &Utility| read(u).offset,
        |u, v| {
            edit(u, |s| s.offset = v);
            Ok(())
        },
    ))
    .separator();
    let parameters = [
        (PaintParameter::Falloff, "Falloff"),
        (PaintParameter::Pressure, "Pressure"),
        (PaintParameter::Slope, "Slope"),
        (PaintParameter::Altitude, "Altitude"),
        (PaintParameter::InverseSlope, "1.0 − Slope"),
        (PaintParameter::InverseAltitude, "1.0 − Altitude"),
    ];
    let effects = [
        (PaintEffect::None, "(none)"),
        (PaintEffect::Density, "Density"),
        (PaintEffect::Size, "Size"),
        (PaintEffect::Rotation, "Rotation"),
        (PaintEffect::SizeDensity, "Size + Density"),
    ];
    f.field(tf::choice(
        "mp-param1",
        "Parameter 1",
        &parameters,
        |u: &Utility| read(u).parameter1.0,
        |u, p| edit(u, |s| s.parameter1.0 = p),
    ))
    .field(tf::choice(
        "mp-effect1",
        "Effect 1",
        &effects,
        |u: &Utility| read(u).parameter1.1,
        |u, e| edit(u, |s| s.parameter1.1 = e),
    ))
    .field(tf::choice(
        "mp-param2",
        "Parameter 2",
        &parameters,
        |u: &Utility| read(u).parameter2.0,
        |u, p| edit(u, |s| s.parameter2.0 = p),
    ))
    .field(tf::choice(
        "mp-effect2",
        "Effect 2",
        &effects,
        |u: &Utility| read(u).parameter2.1,
        |u, e| edit(u, |s| s.parameter2.1 = e),
    ))
    .separator()
    .field(tf::number(
        "mp_linear",
        "Linear Count",
        Unit::Count,
        |u: &Utility| f64::from(read(u).linear_count),
        |u, v| {
            let n = count(v, "Linear Count")?;
            edit(u, |s| s.linear_count = n);
            Ok(())
        },
    ))
    .field(tf::number(
        "mp_array_x",
        "Array Count X",
        Unit::Count,
        |u: &Utility| f64::from(read(u).array_count[0]),
        |u, v| {
            let n = count(v, "Array Count X")?;
            edit(u, |s| s.array_count[0] = n);
            Ok(())
        },
    ))
    .field(tf::number(
        "mp_array_y",
        "Array Count Y",
        Unit::Count,
        |u: &Utility| f64::from(read(u).array_count[1]),
        |u, v| {
            let n = count(v, "Array Count Y")?;
            edit(u, |s| s.array_count[1] = n);
            Ok(())
        },
    ))
    .field(tf::number(
        "mp_radial",
        "Radial Count",
        Unit::Count,
        |u: &Utility| f64::from(read(u).radial_count),
        |u, v| {
            let n = count(v, "Radial Count")?;
            edit(u, |s| s.radial_count = n);
            Ok(())
        },
    ))
    .separator()
    .field(tf::check(
        "mp_3d",
        "3D Brush",
        |u: &Utility| read(u).brush_3d,
        |u, on| edit(u, |s| s.brush_3d = on),
    ))
    .field(tf::choice(
        "mp-source",
        "Geometry Source",
        &[
            (GeometrySource::AllLayers, "All BG Layers"),
            (GeometrySource::RandomLayer, "Random BG Layer"),
        ],
        |u: &Utility| read(u).source,
        |u, g| edit(u, |s| s.source = g),
    ))
    .field(
        tf::check(
            "mp_instances",
            "Create Instances",
            |u: &Utility| read(u).instances,
            |u, on| edit(u, |s| s.instances = on),
        )
        // #46.8: it had no tooltip, unlike Create Instancer below.
        .tip("Each copy is a new instance item of its background mesh instead of geometry added to the selected mesh"),
    )
    .field(
        tf::check(
            "mp_replicas",
            "Create Instancer",
            |u: &Utility| read(u).replicas,
            |u, on| edit(u, |s| s.replicas = on),
        )
        .tip(
            "Each stroke makes one Instancer item, with a point per copy, instead of separate copies (G8 RP-7)",
        ),
    )
    .field(
        tf::choice(
            "mp-parent",
            "Parent",
            &[
                (ParentMode::Off, "Off"),
                (ParentMode::Hierarchy, "Hierarchy"),
                (ParentMode::Root, "Root"),
                (ParentMode::Primary, "Primary"),
            ],
            |u: &Utility| read(u).parent,
            |u, p| edit(u, |s| s.parent = p),
        )
        .enabled_if(
            s.instances,
            "Parent is for instances: turn Create Instances on",
        ),
    );
    f.note(match s.mode {
        PaintMode::Slide => "Press on the selected mesh, drag to slide the copy, release to place it. Ctrl first: the drag sizes it.",
        PaintMode::Strokes => "Drag over the selected mesh: copies scatter inside the brush along the stroke.",
        PaintMode::Drop => "Click to place; drag from it to size it and turn it about the surface.",
        PaintMode::DropNoScale => "Click to place; drag from it to turn it about the surface.",
        PaintMode::Linear => "Drag from A to B: Linear Count copies along the line. Shift: 45° steps.",
        PaintMode::Array => "Drag a rectangle: Array Count X × Y copies on its grid. Shift: a square.",
        PaintMode::Radial => "Click the centre and drag out: Radial Count copies in a ring. Shift: 45° start.",
    });
}
