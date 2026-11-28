//! The table of every command (`COMMANDS`) and the argument lists its
//! entries share.
use super::*;

mod animation;
mod app_history;
mod curves;
mod edges_vertices;
mod files;
mod general;
mod graph_editor;
mod item_transforms;
mod items;
mod kits;
mod libraries;
mod nodes;
mod phrases;
mod pies;
mod polygon_uv;
mod presets_layouts;
mod remesh_bake;
mod render_app;
mod rigs;
mod sculpt;
mod selection;
mod selection_sets;
mod statistics;
mod tool_starters;
mod view;
mod viewport_options;

/// scene.saveAs export formats (Operandi's own values: the map names no
/// format strings).
pub const EXPORT_FORMATS: ArgType = ArgType::Choice(&[
    "obj", "stl", "stlAscii", "gltf", "glb", "geo", "svg", "lxo", "ply", "plyAscii", "abc", "usd",
    "usda", "usdc", "usdz", "fbx", "3mf", "step", "iges",
]);
const ON_OFF_QUERY: ArgType = ArgType::Choice(&["on", "off", "?"]);
const AXES: ArgType = ArgType::Choice(&["x", "y", "z", "xy", "yz", "zx", "all"]);
const ITEM_TYPES: ArgType = ArgType::Choice(&[
    "mesh",
    "camera",
    "backdrop",
    "environment",
    "light.area",
    "light.cylinder",
    "light.directional",
    "light.dome",
    "light.mesh",
    "light.photometric",
    "light.point",
    "light.portal",
    "light.spot",
]);
const CLEANUP_ARGS: &[Arg] = &[
    arg("floatingVertex", BOOL, N),
    arg("onePointPolygon", BOOL, N),
    arg("twoPointPolygon", BOOL, N),
    arg("dupPointPolygon", BOOL, N),
    arg("colinear", BOOL, N),
    arg("faceNormal", BOOL, N),
    arg("mergeVertex", BOOL, N),
    arg("mergeDisco", BOOL, N),
    arg("unifyPolygon", BOOL, N),
    arg("forceUnify", BOOL, N),
    arg("removeDiscoWeight", BOOL, N),
    arg("fixGaps", BOOL, N),
];
const SMOOTH_SHIFT_ATTR: ArgType = ArgType::Choice(&[
    "offset",
    "scale",
    "angle",
    "thicken",
    "sharp",
    "angle_weight",
    "both_sides",
]);
const BRIDGE_ATTR: ArgType = ArgType::Choice(&[
    "segments", "twist", "tension", "mode", "remove", "flip", "reverse",
]);

use Area::*;

/// Every command `execute` accepts. Names are unique across entries and
/// aliases (case-insensitive); the app's registry tests hold the parser
/// and this table to each other.
pub static COMMANDS: &[Entry] = &ALL;

/// The table's sections in table order (the `command.list` and help
/// order): one file each under `commands/`.
const SECTIONS: &[&[Entry]] = &[
    general::COMMANDS,
    polygon_uv::COMMANDS,
    remesh_bake::COMMANDS,
    curves::COMMANDS,
    edges_vertices::COMMANDS,
    viewport_options::COMMANDS,
    items::COMMANDS,
    animation::COMMANDS,
    rigs::COMMANDS,
    graph_editor::COMMANDS,
    item_transforms::COMMANDS,
    nodes::COMMANDS,
    presets_layouts::COMMANDS,
    pies::COMMANDS,
    libraries::COMMANDS,
    kits::COMMANDS,
    statistics::COMMANDS,
    selection_sets::COMMANDS,
    sculpt::COMMANDS,
    files::COMMANDS,
    app_history::COMMANDS,
    tool_starters::COMMANDS,
    phrases::COMMANDS,
    selection::COMMANDS,
    view::COMMANDS,
    render_app::COMMANDS,
];

static ALL: [Entry; count(SECTIONS)] = join(SECTIONS);

const fn count(parts: &[&[Entry]]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < parts.len() {
        n += parts[i].len();
        i += 1;
    }
    n
}

/// The sections one after another, unchanged.
const fn join<const N: usize>(parts: &[&[Entry]]) -> [Entry; N] {
    let mut out = [parts[0][0]; N];
    let (mut i, mut k) = (0, 0);
    while i < parts.len() {
        let mut j = 0;
        while j < parts[i].len() {
            out[k] = parts[i][j];
            k += 1;
            j += 1;
        }
        i += 1;
    }
    assert!(k == N);
    out
}
