//! The Transform buttons of the Duplicate tab (duplicate-panel-2026-11-20,
//! DX-20..26): Transform Array, Transform Radial Array, Transform Scatter,
//! Curve / Bezier / B-Spline Transform and Transform Pen.
//!
//! Each keeps its tool's placer (grid, circle, scatter cloud, path) and
//! swaps the copier: instead of making copies, the selected items
//! themselves move onto the placer's spots, one item per spot, by their
//! item positions. No new items are made.
//!
//! Provisional readings (docs/SPECIFICATION_QUESTIONS.md, DXQ-1):
//! - Items go to spots in item-list order, top to bottom (DX-21).
//! - The spots are worked out from the first item, as if it were the
//!   ordinary tool's source; its own place is the first spot for the grid,
//!   circle and scatter placers (DX-10), the path's first place for paths.
//! - A spot's turn and size go on top of the item's own, about the item's
//!   position, in the item's own axes (DX-23), so no shear is made.
use crate::duplicate::{DuplicateParameters, array_transforms, radial_transforms};
use crate::editor::Tool;
use crate::{Mesh, Mode, Result, Scene};
use glam::{DMat4, DQuat, DVec3};

/// DX-11: what the panel says when there is nothing to move.
pub const NOTHING_TO_MOVE: &str = "Nothing to move: select items (or a mesh in a component mode).";

/// DX-11 / DX-21 / DX-26: the items a Transform button moves, in item-list
/// order. Item mode: the selected items; a component mode: the active
/// mesh item(s). An item whose parent (at any level) is also picked is
/// left to move with it.
pub fn items_to_move(scene: &Scene) -> Vec<u64> {
    let picked: Vec<u64> = if scene.selection.mode == Mode::Item {
        scene.selected_items()
    } else {
        let ids = scene.active_meshes();
        if ids.is_empty() {
            scene.active().map(|i| i.id).into_iter().collect()
        } else {
            ids
        }
    };
    let set: std::collections::BTreeSet<u64> = picked.iter().copied().collect();
    let under_picked = |id: u64| {
        let mut at = scene.item(id).and_then(|i| i.parent);
        // A parent chain is short; the bound only guards a broken file.
        for _ in 0..scene.items.len() {
            let Some(p) = at else { return false };
            if set.contains(&p) {
                return true;
            }
            at = scene.item(p).and_then(|i| i.parent);
        }
        false
    };
    scene
        .items
        .iter()
        .map(|i| i.id)
        .filter(|id| set.contains(id) && !under_picked(*id))
        .collect()
}

/// Whether a Transform button would find nothing to move.
pub fn nothing_to_move(scene: &Scene) -> bool {
    items_to_move(scene).is_empty()
}

/// The item's world position (its pivot: "by their centres", DX-20).
fn position(scene: &Scene, id: u64) -> Result<DVec3> {
    Ok(scene.world_matrix(id)?.transform_point3(DVec3::ZERO))
}

/// The placer's spots for `tool` (world transforms), the first spot
/// included, worked out with `anchor` (the first item's position) as the
/// source.
pub fn spots(tool: Tool, p: &DuplicateParameters, anchor: DVec3) -> Result<Vec<DMat4>> {
    let mut out = Vec::new();
    match tool {
        Tool::Array => {
            // The first cell is the source's own; Replace Source gives it
            // with the cell's size like every other cell.
            let mut a = p.array;
            a.copies.replace_source = true;
            out = array_transforms(&a, anchor)?;
        }
        Tool::RadialArray => {
            let mut r = p.radial;
            r.copies.replace_source = false;
            out.push(DMat4::IDENTITY);
            out.extend(radial_transforms(&r)?);
        }
        Tool::ScatterClone => {
            out.push(DMat4::IDENTITY);
            out.extend(p.scatter.generator.transforms(anchor, false)?);
        }
        Tool::CurveClone => {
            let mut t = p.curve_clone.clone();
            t.clone.copies.replace_source = true;
            // Size belongs to the copies' effector, which a Transform button
            // doesn't have (DX-24): the items keep their size.
            t.clone.size = 1.;
            // The path tools size and pivot on the source's bounding box;
            // here the source is a point, the anchor (a two-point line
            // stands in for it).
            let mut box_of_anchor = Mesh {
                vertices: vec![anchor, anchor],
                ..Default::default()
            };
            crate::vertex_commands::append_face(&mut box_of_anchor, vec![0, 1], None);
            out = crate::path_tools::clone_transforms(&box_of_anchor, &[0], &t)?;
        }
        _ => return Err("This tool has no Transform version".into()),
    }
    Ok(out)
}

/// Item `id`'s new world matrix on spot `t`: its position goes where the
/// spot takes the anchor, and the spot's turn and size go on top of its
/// own (DX-23).
fn placed(scene: &Scene, id: u64, anchor: DVec3, t: DMat4) -> Result<DMat4> {
    let world = scene.world_matrix(id)?;
    let (scale, turn, _) = world.to_scale_rotation_translation();
    let (spot_scale, spot_turn, _) = t.to_scale_rotation_translation();
    let at = t.transform_point3(anchor);
    let turn: DQuat = (spot_turn * turn).normalize();
    let out = DMat4::from_scale_rotation_translation(scale * spot_scale, turn, at);
    if !out.is_finite() {
        return Err("The item's new place isn't finite".into());
    }
    Ok(out)
}

/// The live preview of a Transform button on the base scene: the items
/// move onto the spots (DX-20). Locked items stay put and are counted
/// (DX-25); more items than spots leave the rest where they are, fewer
/// use the first spots (DX-22).
pub(crate) fn preview(tool: Tool, scene: &mut Scene, p: &DuplicateParameters) -> Result<()> {
    let ready = match tool {
        Tool::CurveClone => p.curve_clone.has_path(),
        _ => p.placed,
    };
    if !ready {
        return Ok(());
    }
    let all = items_to_move(scene);
    if all.is_empty() {
        return Err(NOTHING_TO_MOVE.into());
    }
    let ids: Vec<u64> = all
        .into_iter()
        .filter(|id| !scene.item(*id).is_some_and(|i| i.is_locked()))
        .collect();
    let Some(&first) = ids.first() else {
        return Err("Every selected item is locked".into());
    };
    let anchor = position(scene, first)?;
    let spots = spots(tool, p, anchor)?;
    // Work out every new place before moving any item: a moved parent
    // would change where its (unpicked) children's ancestors are, but not
    // the picked items, which never sit under each other (DX-26).
    let moves: Vec<(u64, DMat4)> = ids
        .iter()
        .zip(&spots)
        .map(|(&id, &t)| Ok((id, placed(scene, id, anchor, t)?)))
        .collect::<Result<_>>()?;
    for (id, world) in moves {
        scene.set_world_matrix(id, world)?;
    }
    Ok(())
}

/// DX-25: how many of the items a Transform button would move are locked
/// (they stay put; the panel says how many).
pub fn locked_count(scene: &Scene) -> usize {
    items_to_move(scene)
        .iter()
        .filter(|id| scene.item(**id).is_some_and(|i| i.is_locked()))
        .count()
}
