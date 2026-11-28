//! Symmetrize's Tool Properties (BEHAVIOR-DUPLICATE-PANEL DX-41): Axis,
//! Direction, Merge, Offset, Topology, as one section for the shared form
//! (UIC-110..117). Live from the start (DX-44), so there is no Apply.
//! Geometry in `operandi_core::symmetrize_tool`.
use super::*;
use crate::form::Unit;
use crate::live_fields::{check, pick};
use crate::tool_form::{self as tf, ToolForm};
use operandi_core::SymmetrizeSide;

type Form = ToolForm<Parameters>;

impl Operandi {
    pub(super) fn symmetrize_form(&self, p: &Parameters, f: &mut Form) {
        let s = &p.duplicate.symmetrize;
        let mut rows = Form::new();
        rows.field(
            tf::axis(
                |p: &Parameters| p.duplicate.symmetrize.axis,
                |p, a| p.duplicate.symmetrize.axis = a,
            )
            .tip("The plane's normal: X keeps one side of the Y-Z plane"),
        )
        .field(
            pick(
                "symmetrize-direction",
                "Direction",
                &[
                    (SymmetrizeSide::Positive, "Positive to Negative"),
                    (SymmetrizeSide::Negative, "Negative to Positive"),
                ],
                |p| p.duplicate.symmetrize.direction,
                |p, d| p.duplicate.symmetrize.direction = d,
            )
            .tip("Which side is kept and copied over the other"),
        )
        .field(
            check(
                "symmetrize_merge",
                "Merge",
                |p| p.duplicate.symmetrize.merge,
                |p, on| p.duplicate.symmetrize.merge = on,
            )
            .tip("Weld the copy to the kept half on the plane")
            .enabled_if(!s.topology, "Topology moves points; nothing to weld"),
        )
        .field(
            crate::edit_panel::slice_num(
                "symmetrize_offset",
                "Offset",
                Unit::Distance,
                |p| p.duplicate.symmetrize.offset,
                |p, v| {
                    if !v.is_finite() {
                        return Err("Offset must be a finite number".into());
                    }
                    p.duplicate.symmetrize.offset = v;
                    Ok(())
                },
            )
            .tip("Moves the plane along the axis"),
        )
        .field(
            check(
                "symmetrize_topology",
                "Topology",
                |p| p.duplicate.symmetrize.topology,
                |p, on| p.duplicate.symmetrize.topology = on,
            )
            .tip("Move each point to mirror its partner instead of cutting and copying"),
        );
        f.section("Symmetrize", false, rows.items);
        if let Some(error) = self
            .editor
            .session
            .live
            .as_ref()
            .and_then(|l| l.preview_error.as_ref())
        {
            f.error(error.clone());
        }
        // DX-43 (DXQ-3, the maintainer's OK).
        f.note("Works on the whole active mesh; a component selection is ignored.");
    }
}
