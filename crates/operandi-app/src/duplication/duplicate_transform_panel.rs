//! Tool Properties of the Duplicate tab's Transform buttons
//! (duplicate-panel-2026-11-20 DX-20..26; geometry in
//! `operandi_core::duplicate_transform`). DX-24: the placer's rows and
//! handles are the whole form; the copier sections (Clone Effector and
//! any item-copy rows) don't apply and are left out (UIC-117).
use super::*;
use crate::tool_form::{self as tf, ToolForm};
use operandi_core::duplicate_transform as dt;

type Form = ToolForm<Parameters>;

/// The placer sections a Transform form keeps.
const PLACER_SECTIONS: &[&str] = &[
    "Array Generator",
    "Helix Generator",
    "Scatter Generator",
    "Curve Path",
    "Bezier Path",
    "B-Spline Path",
    "Pen Path",
    "Path Steps",
];

/// Whether the live Duplicate tool runs as a Transform button.
pub(crate) fn is_transform(p: &Parameters) -> bool {
    p.duplicate.copier == Some(operandi_core::duplicate_copier::Copier::Transform)
}

impl Operandi {
    pub(super) fn duplicate_transform_form(&self, tool: Tool, p: &Parameters, f: &mut Form) {
        // The ordinary tool's form, less its copier: one place draws each
        // placer's rows.
        let mut full = Form::new();
        if tool == Tool::CurveClone {
            self.path_tool_form(tool, p, &mut full);
        } else {
            self.duplicate_form(tool, p, &mut full);
        }
        let path = tool == Tool::CurveClone;
        for item in full.items {
            match item {
                tf::Item::Section { name, .. } if !PLACER_SECTIONS.contains(&name) => {}
                // The copy tools' notes and Apply speak of copies.
                tf::Item::Note(_) | tf::Item::Error(_) => {}
                tf::Item::Buttons(_) if !path => {}
                other => {
                    f.push(other);
                }
            }
        }
        let placed = p.duplicate.placed;
        if !path {
            // DS-31: Apply places the items without a click in the view.
            f.buttons(vec![
                tf::Button::change("Apply", |p: &mut Parameters| p.duplicate.placed = true)
                    .tip("Move the items onto the spots with these values (or click in the view)")
                    .enabled_if(!placed, "The items are placed; values now move them live"),
            ]);
        }
        let scene = &self.editor.scene;
        let live = self.editor.session.live.as_ref();
        if let Some(error) = live.and_then(|l| l.preview_error.as_ref()) {
            f.error(error.clone());
        } else if dt::nothing_to_move(scene) {
            f.note(dt::NOTHING_TO_MOVE);
        }
        // DX-25: locked items stay put.
        match live.map_or(0, |l| dt::locked_count(&l.base)) {
            0 => {}
            1 => {
                f.note("1 locked item stays where it is.");
            }
            n => {
                f.note(format!("{n} locked items stay where they are."));
            }
        }
        f.note(transform_hint(tool, p));
    }
}

/// What to do next, shown under the rows.
fn transform_hint(tool: Tool, p: &Parameters) -> &'static str {
    let d = &p.duplicate;
    match tool {
        Tool::CurveClone if !d.curve_clone.has_path() => {
            "Click in the view to place path points; the selected items move onto the path, one per place, top of the item list first."
        }
        Tool::CurveClone => {
            "Drag points to reshape the path; the items follow. Drop (Space) keeps them there."
        }
        _ if !d.placed => {
            "Click in the view (or Apply): the selected items move onto the spots, one per spot, top of the item list first."
        }
        _ => {
            "Change the values or drag the handles: the items follow. Drop (Space) keeps them; Undo puts them back."
        }
    }
}
