//! The Replica buttons' Replica section (G8 RP-6): Source (what gets
//! replicated) and Parent (where the new group goes), in place of the
//! Instance buttons' Item section. Source choices not built yet stay in
//! the list greyed with the reason (UIC-117).
use super::*;
use crate::live_fields::pick;
use crate::tool_form as tf;
use operandi_core::replicator::{ReplicaParent, ReplicaSource};

pub(super) fn replica_rows() -> Vec<tf::Item<Parameters>> {
    let sources: Vec<(ReplicaSource, &str)> =
        ReplicaSource::ALL.iter().map(|s| (*s, s.label())).collect();
    let parents: Vec<(ReplicaParent, &str)> =
        ReplicaParent::ALL.iter().map(|p| (*p, p.label())).collect();
    let mut source = pick(
        "replica-source",
        "Source",
        &sources,
        |p| p.duplicate.replica.source,
        |p, s| p.duplicate.replica.source = s,
    )
    .tip("What gets replicated: the selected item (each selected item makes its own Replicator)");
    for (i, s) in ReplicaSource::ALL.iter().enumerate() {
        if !s.built() {
            source = source.greyed_option(i, "Not built yet (RP-Q4); Active Item works");
        }
    }
    let mut f = tf::ToolForm::<Parameters>::new();
    f.field(source).field(
        pick(
            "replica-parent",
            "Parent",
            &parents,
            |p| p.duplicate.replica.parent,
            |p, m| p.duplicate.replica.parent = m,
        )
        .tip("Scene: the Replicator's group goes at the root. Selected: under the replicated item"),
    );
    f.items
}
