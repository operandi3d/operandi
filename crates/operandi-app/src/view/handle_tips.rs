//! Hover tooltips for every tool handle (MN-09, UIC-05). Each says what the
//! handle is, what dragging it changes, and any modifier it reads. They
//! show only while the handle is hovered, never during a drag (MN-09.2).
//!
//! Already had their own before MN-09: transform handles, primitive
//! handles, plane circles (`plane_handles.rs`), Bend (`bend_view.rs`) and
//! precision handles. This module adds the live tools' other handle kinds
//! (Twist Bevel, Edge Extend, Bevel-type pairs ...), falloff handles,
//! Tweak's element, an armed Click to Start tool, and camera and light
//! handles. [`ALL_KINDS`] lists every kind for the coverage test.
use crate::camera_light_handles::Grab;
use crate::handle_ids::FalloffHandle;
use operandi_core::tool_card::HandleKind;
use operandi_core::{Falloff, Tool};

/// Every handle kind with a tooltip, for MN-09.3's test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // live handles call `live_tool_tip`
pub(crate) enum TipKind {
    /// A live tool's handle `axis` of layout `kind`.
    Live(HandleKind, usize),
    /// Twist Bevel's spine handles, by id.
    Spine(usize),
    /// A falloff handle: kind 0 Linear, 1 Radial, 2 Cylinder.
    Falloff(u8, FalloffHandle),
    Tweak,
    ClickToStart,
    Camera(Grab),
}

/// Every handle kind: each live layout's axes, Twist Bevel's spine, the
/// falloffs' points and axis handles, Tweak, Click to Start, and every
/// camera and light grab.
#[cfg(test)]
pub(crate) fn all_kinds() -> Vec<TipKind> {
    use HandleKind as K;
    let mut out = Vec::new();
    for (kind, count) in [
        (K::Bounds, 9),
        (K::RadialBounds, 9),
        (K::Axes, 3),
        (K::Rings, 3),
        (K::Combined, 9),
        (K::Pair, 2),
        (K::Normal, 1),
        (K::Single, 1),
    ] {
        out.extend((0..count).map(|a| TipKind::Live(kind, a)));
    }
    use crate::handle_ids as tb;
    for id in [
        tb::TB_SHIFT,
        tb::TB_INSET,
        tb::TB_TWIST,
        tb::TB_BEND,
        tb::TB_BEND_DIRECTION,
    ] {
        out.push(TipKind::Spine(id));
    }
    for kind in 0..3u8 {
        let points = if kind == 0 { 2 } else { 4 };
        for i in 0..points {
            out.push(TipKind::Falloff(kind, FalloffHandle::Point(i)));
        }
        let axis_points: &[usize] = if kind == 0 { &[0, 1] } else { &[0] };
        for &i in axis_points {
            for axis in 0..3 {
                out.push(TipKind::Falloff(kind, FalloffHandle::Axis(i, axis)));
            }
        }
    }
    out.push(TipKind::Tweak);
    out.push(TipKind::ClickToStart);
    for g in [
        Grab::AreaWidth,
        Grab::AreaHeight,
        Grab::SpotCone,
        Grab::SpotEdge,
        Grab::CameraFocus,
        Grab::PointRadius,
    ] {
        out.push(TipKind::Camera(g));
    }
    out
}

const AXES: [&str; 3] = ["X", "Y", "Z"];

/// The tooltip text for `kind`.
pub(crate) fn tip(kind: TipKind) -> String {
    match kind {
        TipKind::Live(kind, axis) => live(kind, axis),
        TipKind::Spine(id) => spine(id).into(),
        TipKind::Falloff(f, h) => falloff(f, h),
        TipKind::Tweak => "Tweak: the point, edge or polygon under the pointer. Drag to move it \
             (with the Range around it). Right-drag: change Range."
            .into(),
        TipKind::ClickToStart => "Click to start: the tool is picked but not started. Click in \
             the view to start it there; drag to start and shape it at once."
            .into(),
        TipKind::Camera(g) => camera(g).into(),
    }
}

fn live(kind: HandleKind, axis: usize) -> String {
    use HandleKind as K;
    let a = AXES[axis % 3];
    match kind {
        K::None => String::new(),
        K::Bounds | K::RadialBounds if axis < 3 => {
            format!("{a} arrow: drag to move the shape along {a}")
        }
        K::Bounds | K::RadialBounds => {
            format!("{a} side: drag to resize the shape along {a}. Ctrl: move it whole")
        }
        K::Axes => format!("{a} arrow: drag to change the tool's amount along {a}"),
        K::Rings => format!("{a} ring: drag around it to turn about {a}. Ctrl: snap steps"),
        K::Combined => match axis / 3 {
            0 => format!("{a} arrow: drag to move along {a}"),
            1 => format!("{a} ring: drag to turn about {a}. Ctrl: snap steps"),
            _ => format!("{a} square: drag to scale along {a}. Ctrl: all three equally"),
        },
        K::Pair if axis == 0 => {
            "Blue arrow along the normal: drag to change Shift (how far it moves out)".into()
        }
        K::Pair => "Red handle sideways: drag to change Inset (how far it moves in)".into(),
        K::Normal => "Arrow along the normal: drag to change the amount".into(),
        K::Single => "Arrow: drag along the tool's direction to change its amount".into(),
        K::Spine => spine(axis).into(),
    }
}

fn spine(id: usize) -> &'static str {
    use crate::handle_ids as tb;
    match id {
        tb::TB_SHIFT => "Shift arrow at the cap: drag to lengthen or shorten the tube",
        tb::TB_INSET => "Inset square at the cap: drag to taper the tube (Inset)",
        tb::TB_TWIST => "Twist ring around the cap: drag around it to twist the tube",
        tb::TB_BEND => "Bend arrow at the base: drag to bend the tube",
        tb::TB_BEND_DIRECTION => {
            "Bend Direction ring at the base: drag around it to turn the way the tube bends"
        }
        _ => "Twist Bevel handle: drag to shape the tube",
    }
}

fn falloff(kind: u8, h: FalloffHandle) -> String {
    let name = ["Linear", "Radial", "Cylinder"][kind.min(2) as usize];
    match h {
        FalloffHandle::Point(0) if kind == 0 => {
            "Linear falloff Start: full effect here. Drag to move it in the view".into()
        }
        FalloffHandle::Point(_) if kind == 0 => {
            "Linear falloff End: no effect past here. Drag to move it in the view".into()
        }
        FalloffHandle::Point(0) => format!("{name} falloff centre: drag to move it in the view"),
        FalloffHandle::Point(_) => {
            format!("{name} falloff size: drag to make it bigger or smaller")
        }
        FalloffHandle::Axis(i, axis) => {
            let what = match (kind, i) {
                (0, 0) => "Start",
                (0, _) => "End",
                _ => "centre",
            };
            format!(
                "{name} falloff {what}, {} arrow: drag to move it along {} only",
                AXES[axis], AXES[axis]
            )
        }
    }
}

fn camera(g: Grab) -> &'static str {
    match g {
        Grab::AreaWidth => "Area light width: drag to make the light wider or narrower",
        Grab::AreaHeight => "Area light height: drag to make the light taller or shorter",
        Grab::SpotCone => "Spot cone: drag to widen or narrow the Cone Angle",
        Grab::SpotEdge => "Spot edge: drag to soften or harden the cone's edge (Edge Angle)",
        Grab::CameraFocus => "Camera focus: drag along the view to set the Focus Distance",
        Grab::PointRadius => "Light radius: drag to set the light's Radius (softer shadows)",
    }
}

/// The falloff kind index `tip` uses, for the active falloff.
pub(crate) fn falloff_kind(f: &Falloff) -> Option<u8> {
    match f {
        Falloff::Linear(_) => Some(0),
        Falloff::Radial(_) => Some(1),
        Falloff::Cylinder(_) => Some(2),
        _ => None,
    }
}

/// A live tool's handle: its own text first, the kind's after.
pub(crate) fn live_tool_tip(tool: Tool, kind: HandleKind, axis: usize) -> String {
    if kind == HandleKind::Spine {
        return format!("{}: {}", tool.label(), spine(axis));
    }
    format!("{}: {}", tool.label(), live(kind, axis))
}

impl crate::Operandi {
    /// The tooltip for whatever non-live-tool handle is under `pos`: a
    /// falloff handle, a camera or light handle, Tweak's element, or the
    /// view while a Click to Start tool is armed.
    pub(crate) fn other_handle_tip(
        &self,
        pos: eframe::egui::Pos2,
        r: eframe::egui::Rect,
    ) -> Option<String> {
        if let Some(i) = self.falloff_hit(pos, r)
            && let Some(f) = self.editor.spatial_falloff.as_ref().and_then(falloff_kind)
            && let Some(&(_, h)) = self.falloff_handles(r).get(i)
        {
            return Some(tip(TipKind::Falloff(f, h)));
        }
        if let Some(g) = self.item_handle_hit(pos, r) {
            return Some(tip(TipKind::Camera(g)));
        }
        let tool = self.editor.session.live.as_ref().map(|l| l.tool);
        if self.editor.is_armed() {
            let name = tool.map(|t| t.label()).unwrap_or("The tool");
            return Some(format!("{name} — {}", tip(TipKind::ClickToStart)));
        }
        if tool == Some(Tool::Tweak) && self.tweak_pick(pos, r).is_some() {
            return Some(tip(TipKind::Tweak));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MN-09.3: every handle kind has tooltip text saying what it is and
    /// what a drag does.
    #[test]
    fn every_handle_kind_has_a_tooltip() {
        let kinds = all_kinds();
        assert!(kinds.len() > 60, "{}", kinds.len());
        for kind in kinds {
            let text = tip(kind);
            assert!(!text.trim().is_empty(), "{kind:?} has no tooltip");
            assert!(
                text.to_lowercase().contains("drag") || text.contains("Click"),
                "{kind:?}: {text}"
            );
        }
    }

    /// Every tool's handle layout is covered (a new layout fails here).
    #[test]
    fn every_tools_layout_is_listed() {
        let kinds = all_kinds();
        for t in Tool::ALL {
            let layout = t.card().handles.kind;
            if layout == HandleKind::None {
                continue;
            }
            let listed = layout == HandleKind::Spine
                || kinds
                    .iter()
                    .any(|k| matches!(k, TipKind::Live(l, _) if *l == layout));
            assert!(listed, "{t:?}'s {layout:?} handles have no tooltips");
        }
    }
}
