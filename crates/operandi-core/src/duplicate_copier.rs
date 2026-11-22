//! Which copier a Duplicate tool button runs with
//! (duplicate-panel-2026-11-20, DX-20 / DX-30). The placer (line, grid,
//! circle, scatter cloud, path) is the tool's own; a preset row's
//! `copier = "instance"` or `copier = "transform"` swaps what is put at
//! the spots. No copier (the plain buttons) makes ordinary copies.

/// A preset row's copier (`actor.copier` in factory-tool-presets.toml).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Copier {
    /// DX-30: new items that are instances of their source.
    Instance,
    /// DX-20: the selected items themselves move onto the spots.
    Transform,
    /// G8 RP-6: one Replicator per source, with a point per spot
    /// (replicator.rs).
    Replica,
}
