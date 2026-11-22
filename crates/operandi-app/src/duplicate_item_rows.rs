//! The Duplicate tools' Item section (BEHAVIOR-DUPLICATE-PANEL DX-31/32):
//! Parent, Instance and Hierarchy, shared by the plain tools in Item mode
//! and their Instance versions, so both use one form (UIC-110..117).
//! Bounding Box and Source's other choices are left out until built
//! (UIC-117). Geometry in `operandi_core::duplicate_items`.
use super::*;
use crate::live_fields::{check, pick};
use crate::tool_form as tf;
use operandi_core::Copies;
use operandi_core::duplicate_copier::Copier;
use operandi_core::duplicate_items::ParentMode;

type Form = tf::ToolForm<Parameters>;
pub(super) type CopiesGet = fn(&Parameters) -> &Copies;
pub(super) type CopiesGetMut = fn(&mut Parameters) -> &mut Copies;

/// The Item section's rows. `item_mode`: the selection is in Item mode,
/// where the plain tools make item copies too.
pub(super) fn item_rows(
    get: CopiesGet,
    get_mut: CopiesGetMut,
    instancing: bool,
    item_mode: bool,
) -> Vec<tf::Item<Parameters>> {
    let makes_items = item_mode || instancing;
    let why = "In a component mode the copies go into the mesh; turn on Instance or pick Item mode";
    let mut f = Form::new();
    f.field(
        check(
            "copies_instance",
            "Instance",
            |p| operandi_core::duplicate_items::instancing(&p.duplicate),
            |p, on| p.duplicate.copier = on.then_some(Copier::Instance),
        )
        .tip("Each copy is an instance: it shares its source's mesh, so editing one changes all"),
    )
    .field(
        pick(
            "copies-parent",
            "Parent",
            &[
                (ParentMode::Off, ParentMode::Off.label()),
                (ParentMode::Hierarchy, ParentMode::Hierarchy.label()),
                (ParentMode::Root, ParentMode::Root.label()),
                (ParentMode::Primary, ParentMode::Primary.label()),
            ],
            move |p| get(p).item.parent,
            move |p, m| get_mut(p).item.parent = m,
        )
        .tip("Off: copies sit beside the source. Hierarchy: a chain, each under the one before. Root: all under the source. Primary: all under a new Group Locator under the source")
        .enabled_if(makes_items, why),
    )
    .field(
        check(
            "copies_hierarchy",
            "Hierarchy",
            move |p| get(p).item.hierarchy,
            move |p, on| get_mut(p).item.hierarchy = on,
        )
        .tip("Copy the source's child items too, under each copy")
        .enabled_if(makes_items, why),
    );
    f.items
}
