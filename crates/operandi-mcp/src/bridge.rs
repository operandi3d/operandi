//! The bridge's command surface: MCP tool names, their input schemas, and how
//! each one maps onto `operandi_core::Command`. Modeling behavior stays in
//! `Editor`; this file only translates JSON in and scene state out.
use glam::DVec3;
use operandi_core::command_exec::{
    as_u32, boolean, num, opt_bool, opt_u64, req_str, u32_list, vec3,
};
use operandi_core::command_names;
use operandi_core::{Command, Component, Editor, Mode, SelectionOperation, Tool, persistence};
use serde_json::{Map, Value, json};

pub type Result<T> = std::result::Result<T, String>;

/// Interactive tools reachable through `tool_run`, by the same ids the app's
/// command line uses (`crates/operandi-app/src/commands/command.rs`). Old ids
/// resolve through `operandi_core::command_names`.
/// Kept in sorted order (case-insensitive), so parallel branches add
/// lines in different places (AGENTS.md, "Code"); a test checks it.
pub const TOOLS: [(&str, Tool); 29] = [
    // BRQ-10: the Edit panel's slices (with Primitive Slice and
    // poly.julienne); their options go in params.slice.
    ("Curve Slice", Tool::CurveSlice),
    ("edge.bevel", Tool::EdgeBevel),
    ("edge.chamfer", Tool::EdgeChamfer),
    ("edge.crease", Tool::EdgeCrease),
    ("edge.extend", Tool::EdgeExtend),
    ("edge.extrude", Tool::EdgeExtrude),
    ("edge.relax", Tool::EdgeRelax),
    ("edge.slide", Tool::EdgeSlide),
    ("ElementMove", Tool::Tweak),
    ("poly.bevel", Tool::Bevel),
    ("poly.extrude", Tool::Extrude),
    // Bridge request 2026-11-20: Inset (params.inset, m).
    ("poly.inset", Tool::Inset),
    ("poly.julienne", Tool::AxisSlice),
    ("poly.smshift", Tool::SmoothShift),
    ("prim.cube", Tool::Box),
    ("prim.cylinder", Tool::Cylinder),
    ("prim.sphere", Tool::Sphere),
    ("Primitive Slice", Tool::PrimitiveSlice),
    ("Thicken", Tool::Thicken),
    ("Transform", Tool::Transform),
    ("TransformMove", Tool::Move),
    ("TransformRotate", Tool::Rotate),
    ("TransformScale", Tool::Scale),
    ("vert.bevel", Tool::VertexBevel),
    ("vert.extrude", Tool::VertexExtrude),
    ("vert.merge", Tool::VertexMerge),
    ("vert.slide", Tool::VertexSlide),
    ("vert.tear", Tool::VertexTear),
    // Bridge request 2026-11-20: params.step [x, y, z], m; 0 leaves that
    // axis alone.
    ("xfrm.quantize", Tool::Quantize),
];

/// tool_run's tools outside [`TOOLS`]: Bridge, a numeric session
/// (BRQ-80, `bridge_run`), and Curve Extrude, run from its button's preset
/// (`curve_extrude_run`). Listed in tools/list's enum with the others.
pub const SESSION_TOOLS: [&str; 2] = ["edge.bridge", "CurveExtrude"];

/// One-shot commands reachable through `command_run`: the command
/// registry's headless entries (extensibility stage 2, L6), named as in the
/// name map; old names resolve through `operandi_core::command_names`.
pub fn commands() -> Vec<&'static str> {
    operandi_core::command_registry::COMMANDS
        .iter()
        .filter(|e| e.flags.headless)
        .map(|e| e.name)
        .collect()
}

/// Tools only the running app can answer (they need its window, GPU or
/// renderer); the headless sidecar refuses them with [`APP_ONLY`].
/// Kept in sorted order (case-insensitive), so parallel branches add
/// lines in different places (AGENTS.md, "Code"); a test checks it.
pub const APP_TOOLS: [&str; 9] = [
    "app_command",
    "online_library",
    "panel_show",
    "pointer_stroke",
    "render",
    "render_status",
    "scene_revert",
    "view_set",
    "viewport_capture",
];

/// The headless sidecar's answer to an app-only tool.
pub const APP_ONLY: &str = "needs the running app: start Operandi with Window > MCP Bridge on and connect with operandi-mcp --attach";

/// One headless editing session. Every call runs on a staged copy of the
/// editor and is published only if every command in it succeeded, so a
/// failed call never leaves a half-armed tool behind.
#[derive(Default)]
pub struct Bridge {
    pub editor: Editor,
    /// The headless session's file and its last saved state.
    pub document: Document,
    /// Inside the running app (attached): app-only jobs such as frame
    /// renders and playblasts are left for the app to run.
    pub attached: bool,
}

/// Which file the headless session is, and the scene as last saved.
#[derive(Default)]
pub struct Document {
    pub path: Option<std::path::PathBuf>,
    pub saved: operandi_core::Scene,
}

pub(crate) fn scene_save_args(args: &Map<String, Value>) -> Result<(&str, bool)> {
    crate::commands::check_keys("scene_save", args, &["path", "copy"])?;
    let path = req_str(args, "path")?;
    let copy = opt_bool(args, "copy")?.unwrap_or(false);
    Ok((path, copy))
}

impl Bridge {
    /// A new scene with one empty mesh, like File > New.
    pub fn new() -> Self {
        let mut editor = Editor::default();
        editor
            .dispatch(Command::NewMesh)
            .expect("an empty editor accepts New Mesh");
        // A new document starts with no history, as an opened one does.
        editor.history.undo.clear();
        editor.history.redo.clear();
        let saved = editor.scene.clone();
        Self {
            editor,
            document: Document { path: None, saved },
            attached: false,
        }
    }

    /// MCP `tools/list` entries.
    pub fn tool_definitions() -> Vec<Value> {
        let vec3 =
            json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3});
        let ids = |v: &[&str]| json!(v);
        let mut tool_ids: Vec<&str> = TOOLS.iter().map(|(id, _)| *id).collect();
        // Tools tool_run runs its own way, not through `TOOLS`.
        tool_ids.extend(SESSION_TOOLS);
        vec![
            def(
                "scene_info",
                "Items (id, name, kind: mesh, camera, light, backdrop or groupLocator; parent, visibility, transform, element counts), the selection, the active tool, symmetry, and undo/redo history labels. Transforms: translation metres, rotation radians (Euler X, Y, Z), scale factors. item_get gives one item's camera, light or backdrop properties.",
                json!({"type": "object", "properties": {}}),
            ),
            def(
                "mesh_get",
                "Vertices, polygons and edges of one mesh item (default: the active item), in item space unless world is true, plus the scene's smooth-shading rule (smoothing). Faces carry vertices, material and part; they no longer carry a per-face smooth flag (it never changed shading): shading comes from smoothing, set with command_run smoothing.set. Lengths are metres.",
                json!({"type": "object", "properties": {
                    "item": {"type": "integer"},
                    "world": {"type": "boolean"},
                    "edges": {"type": "boolean", "description": "Include the edge list (default true)."},
                    "evaluated": {"type": "boolean", "description": "The mesh after its Mesh Operations (item space), as renders and exports read it, with each face's type and mesh_stack_status per op; a mesh without ops returns its own mesh. Default false: the Base Mesh."},
                    "polygon_types": {"type": "boolean", "description": "Give each face its type: face, subdivision, catmullClark, polyline, curve, bspline or bezier (closed_ prefixed when closed). Implied by evaluated."},
                    "uvs": {"type": "boolean", "description": "Include uv_maps: each UV map's name with one [u, v] per polygon corner, in face order (null where the map has none). Default false."},
                    "subdivided": {"description": "true: the mesh as displayed after subdivision (Subdivision and Catmull-Clark polygons refined at the mesh's display levels, Face polygons kept), with source_faces giving each output polygon's cage polygon; a number 0-6: that level for both kinds instead."}
                }, "additionalProperties": false}),
            ),
            def(
                "scene_new",
                "Replace the session with a new scene (one empty mesh unless empty_mesh is false). Clears history.",
                json!({"type": "object", "properties": {"empty_mesh": {"type": "boolean"}}}),
            ),
            def(
                "scene_load",
                "Open an Operandi scene file (.opscene) and clear history.",
                json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}),
            ),
            def(
                "scene_save",
                "Save the committed scene to an .opscene path. In the running app it becomes the document's file (like Save As: the title loses its *), unless copy is true. To export OBJ, STL, glTF, PLY, Alembic, USD and the rest, command_run scene.saveAs {filename, format?} instead: it writes the file as File > Export does and leaves the document's path and saved state alone.",
                json!({"type": "object", "properties": {
                    "path": {"type": "string"},
                    "copy": {"type": "boolean", "description": "Save a copy and leave the document's path and unsaved state alone (default false)."}
                }, "required": ["path"]}),
            ),
            def(
                "scene_revert",
                "Running app only: throw away unsaved changes and reopen the document's file (File > Revert). Refused when the scene was never saved.",
                json!({"type": "object", "properties": {}, "additionalProperties": false}),
            ),
            def(
                "preset_save",
                "Save a preset file (.oppreset; preset.rs). kind: item (the selected items; meshes drop their item transform), assembly (the selected items with everything below them), environment (every light), renderSettings (render outputs and the render camera) or material (shader tree layer `node`, a Material or Material Group). The scene is not changed.",
                json!({"type": "object", "properties": {
                    "path": {"type": "string"},
                    "kind": {"enum": ["item", "assembly", "environment", "renderSettings", "material"]},
                    "node": {"type": "integer", "description": "Shader tree layer id, for kind material."}
                }, "required": ["path", "kind"]}),
            ),
            def(
                "preset_load",
                "Load a preset file as one undo step: items are added, an environment replaces every light, render settings replace the render outputs and add the camera, and a material replaces what is inside Material Group `group`.",
                json!({"type": "object", "properties": {
                    "path": {"type": "string"},
                    "group": {"type": "integer", "description": "Material Group layer id, for a material preset."}
                }, "required": ["path"]}),
            ),
            def(
                "python_run",
                "Run a Python script (Python scripting preview, kits plan K3). It imports operandi: operandi.run(line) runs any command (raises operandi.CommandError with the command's own message on failure), operandi.query(line) answers a query (JSON answers as Python values), with operandi.undo_group(name): folds steps into one, operandi.mesh_get()/mesh_set(points, polygons) read and write a mesh in bulk. The whole script is one undo step (UIC-227). Returns what it printed; a Python exception is an error with its traceback and leaves the scene as it was. In the running app it needs Python scripting (preview) on.",
                json!({"type": "object", "properties": {
                    "code": {"type": "string", "description": "The script's Python source."}
                }, "required": ["code"], "additionalProperties": false}),
            ),
            def(
                "select",
                "Set the selection mode and select components by index, as one undo step. Replaces the selection unless add is true. item makes that item the active one in any mode (the mesh later tools and primitive_add work on). The echoed selection gives the mode and every mode's own list; tools work on the current mode's.",
                json!({"type": "object", "properties": {
                    "mode": {"enum": ["vertex", "edge", "polygon", "item"]},
                    "item": {"type": "integer", "description": "Item id to make active (default the active item)."},
                    "vertices": {"type": "array", "items": {"type": "integer"}},
                    "edges": {"type": "array", "items": {"type": "array", "items": {"type": "integer"}, "minItems": 2, "maxItems": 2}},
                    "faces": {"type": "array", "items": {"type": "integer"}},
                    "add": {"type": "boolean"}
                }, "required": ["mode"], "additionalProperties": false}),
            ),
            def(
                "selection_op",
                "Run a selection operation: grow, shrink, loop, ring, all, invert, connected, invert_connected, boundary, more, less, between, close_loop, or convert (with to).",
                json!({"type": "object", "properties": {
                    "op": {"enum": ["grow", "shrink", "loop", "ring", "all", "invert", "connected", "invert_connected", "boundary", "more", "less", "between", "close_loop", "next_loop", "previous_loop", "convert"]},
                    "to": {"enum": ["vertex", "edge", "polygon"]}
                }, "required": ["op"]}),
            ),
            def(
                "tool_run",
                "Activate an interactive tool, set its parameters, and drop it (one undo step). Parameters not given keep the tool's starting values (for prim.* that is the last-used segment counts; primitive_add starts from Reset values instead). Units: metres, radians, factors. TransformMove delta is an offset in metres; TransformRotate delta is radians about X, Y, Z; TransformScale delta is factors; Transform uses delta (m), rotation_delta (rad) and scale_delta (factors); Extrude uses extent (m); Bevel uses inset and shift (m); vert.bevel uses inset (m) and round_level; prim.cube uses position (centre) and size (FULL edge lengths) and segments [x, y, z]; prim.sphere and prim.cylinder use position (centre), radius (HALF extents [x, y, z]), sides (around the axis) and radial_segments (divisions along the Y axis: latitude bands for the sphere, height rows for the cylinder); Smooth Shift and Thicken use smooth_shift {offset m, scale factor, angle rad, thicken, sharp, angle_weight, both_sides}; edge.bevel/edge.chamfer use chamfer {offset m, segments, depth 0-1}; ElementMove/tweak moves the element in tweak (on the active mesh) by the world offset delta (m), with tweak_options {mode, range (m), connect (Ignore / Use Connectivity / Rigid Connections / Edge Loops), shape (Linear / Ease-In / Ease-Out / Smooth)}. edge.bridge bridges the selection (two edge runs, or two polygons or clusters) with bridge {segments, mode (linear, curve, smooth), twist, tension, remove, flip, reverse}, applied at once; Bridge's other panel options aren't built and are refused. Curve Slice, Primitive Slice and poly.julienne (Polygon Axis Slice, the Edit panel's Dice) take their options in slice {...} and start from Reset values each call. falloffs [{type, ...options, mix?}] runs the tool under that falloff stack for this call only: linear {start, end, shape, in, out, symmetric, world}, radial {center, size, shape, in, out, world}, cylinder {center, size, axis, shape, world}, vertexMap {mode, index}, invert, selection {steps, shape, p0, p1}, softSelection {radius, connectivity, shape}, element {mode, range, connect, shape}, coplanar {angle, connectivity, shape}, noise {scale, world}, curvature {scale}; mix multiply (default), add, subtract, max or min.",
                json!({"type": "object", "properties": {
                    "tool": {"enum": ids(&tool_ids)},
                    "params": {"type": "object", "properties": {
                        "position": described(&vec3, "Primitive centre, metres."),
                        "size": described(&vec3, "prim.cube: FULL edge lengths, metres."),
                        "extent": described(&vec3, "Extrude: offset, metres."),
                        "delta": described(&vec3, "Move/Tweak: offset in metres; TransformRotate: radians about X, Y, Z; TransformScale: factors."),
                        "rotation_delta": described(&vec3, "Transform: rotation, radians about X, Y, Z."),
                        "scale_delta": described(&vec3, "Transform: scale factors."),
                        "radius": described(&vec3, "prim.sphere / prim.cylinder: HALF extents, metres."),
                        "inset": {"type": "number", "description": "Bevel inset, metres."},
                        "shift": {"type": "number", "description": "Bevel shift, metres."},
                        "round_level": {"type": "integer", "minimum": 0, "maximum": 32, "description": "vert.bevel: Round Level; each cap side becomes 2 × level segments."},
                        "segments": {"type": "array", "items": {"type": "integer"}, "minItems": 3, "maxItems": 3, "description": "prim.cube: divisions along X, Y, Z."},
                        "sides": {"type": "integer", "description": "prim.sphere / prim.cylinder: divisions around the axis."},
                        "radial_segments": {"type": "integer", "description": "prim.sphere: latitude bands; prim.cylinder: rows along the height."},
                        "group_polygons": {"type": "boolean"}, "make_uvs": {"type": "boolean"},
                        "chamfer": {"type": "object", "properties": {
                            "offset": {"type": "number", "description": "metres"}, "segments": {"type": "integer"}, "depth": {"type": "number", "description": "0-1 (1 = 100 %)"}
                        }, "additionalProperties": false},
                        "smooth_shift": {"type": "object", "properties": {
                            "offset": {"type": "number", "description": "metres"}, "scale": {"type": "number", "description": "factor"}, "angle": {"type": "number", "description": "radians"},
                            "thicken": {"type": "boolean"}, "sharp": {"type": "boolean"}, "angle_weight": {"type": "boolean"}, "both_sides": {"type": "boolean"}
                        }, "additionalProperties": false},
                        "step": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "description": "xfrm.quantize: the grid step per axis, metres; 0 leaves that axis alone (default [0, 0, 0])"},
                        "slice": crate::slices::schema(),
                        "slide": slide_schema(),
                        "curve_extrude": curve_extrude_schema(),
                        "bridge": {"type": "object", "description": "edge.bridge: segments (default 0), mode (linear, curve (default), smooth), twist (whole number), tension (1 = 100 %), remove (default true), flip, reverse.", "properties": {
                            "segments": {"type": "integer"}, "mode": {"enum": ["linear", "curve", "smooth"]}, "twist": {"type": "integer"},
                            "tension": {"type": "number"}, "remove": {"type": "boolean"}, "flip": {"type": "boolean"}, "reverse": {"type": "boolean"}
                        }},
                        "tweak": {"type": "object", "description": "One of vertex (index), edge ([a, b]) or polygon (index).", "properties": {
                            "vertex": {"type": "integer"},
                            "edge": {"type": "array", "items": {"type": "integer"}, "minItems": 2, "maxItems": 2},
                            "polygon": {"type": "integer"}
                        }}
                    }, "additionalProperties": false},
                    "action_center": {"type": "object", "description": "Run this call under this Action Center (the earlier one comes back after): mode none, auto, selection, selection_auto_axis, border, element or screen (both need center [x, y, z], world metres: a manual centre), origin or local; axis world (default; selection for selection_auto_axis), selection, xz, yz, xy, or element with normal [x, y, z]. Parent and Pivot aren't built and are refused.", "properties": {
                        "mode": {"type": "string"}, "center": vec3, "axis": {"type": "string"}, "normal": vec3
                    }, "required": ["mode"], "additionalProperties": false},
                    "falloffs": {"type": "array", "description": "Run the tool under this falloff stack instead of the current one (the falloff from before comes back after the call): the first is the falloff, the rest stack on top. vertexMap weighs each point by the selected weight map (command_run select.vertexMap / vertMap.new); invert turns the weight so far into 1 - weight. Tools without falloff refuse it.", "items": {"type": "object", "properties": {
                        "type": {"enum": ["linear", "radial", "cylinder", "vertexMap", "invert", "selection", "softSelection", "element", "coplanar", "noise", "curvature"]},
                        "start": described(&vec3, "linear: Start, metres (default 0,0,0)."),
                        "end": described(&vec3, "linear: End, metres (default 0,1,0)."),
                        "center": described(&vec3, "radial / cylinder: Center, metres (default 0,0,0)."),
                        "size": {"description": "radial / cylinder: Size [x, y, z], metres (default 1,1,1); softSelection/path/screen: a number."},
                        "axis": {"enum": ["x", "y", "z"], "description": "cylinder: Axis (default y)."},
                        "shape": {"type": "string", "description": "linear, easeIn, easeOut, smooth or custom (default linear)."},
                        "in": {"type": "number", "description": "custom shape: In."},
                        "out": {"type": "number", "description": "custom shape: Out."},
                        "symmetric": {"enum": ["none", "start", "end"], "description": "linear: Symmetric (default none)."},
                        "world": {"type": "boolean", "description": "Use World Transforms (default false: item space)."},
                        "mode": {"enum": ["magnitude", "component"], "description": "vertexMap only; default magnitude."},
                        "index": {"type": "integer", "description": "vertexMap only: component index (a weight map reads its one value at every index); default 0."},
                        "mix": {"enum": ["multiply", "add", "subtract", "max", "min"], "description": "How an added vertexMap combines with the weight so far; default multiply."}
                    }, "required": ["type"]}}
                }, "required": ["tool"]}),
            ),
            crate::primitives::definition(),
            crate::import::definition(),
            def(
                "command_run",
                &format!(
                    "Run a one-shot command on the current selection, as one undo step. An argument the command doesn't take is an error. Units: metres, radians, fractions 0-1. command_help explains each argument. Commands and their args (? = optional): {}",
                    crate::commands::summary()
                ),
                json!({"type": "object", "properties": {
                    "command": {"enum": ids(&commands())},
                    "args": {"type": "object"}
                }, "required": ["command"], "additionalProperties": false}),
            ),
            def(
                "command_help",
                "What command_run's commands take: each argument's name, type, unit, default and meaning. Pass command for one, or nothing for all.",
                json!({"type": "object", "properties": {"command": {"enum": ids(&commands())}}, "additionalProperties": false}),
            ),
            def(
                "item_get",
                "One item's kind (mesh, camera, light, backdrop), transform and own properties: for a camera its lens, film, clipping, depth of field and render resolution; for a light its type, intensity, colour and shape; for a backdrop its image (file, pixel size), projection, sizes, flip/invert/blend, contrast, brightness, transparency and show in perspective. Every field's unit is listed next to it (camera_units / light_units / backdrop_units).",
                json!({"type": "object", "properties": {"item": {"type": "integer", "description": "Item id; default the active item."}}, "additionalProperties": false}),
            ),
            def(
                "camera_set",
                "Set camera properties (one undo step), as the camera's Properties panel does. Units: metres (focal_length 0.05 = 50 mm, film_width 0.036 = 36 mm), radians, pixels. resolution [w, h] turns on the camera's Resolution Override; render_camera true makes it the camera renders use.",
                crate::properties::schema(crate::properties::CAMERA_FIELDS),
            ),
            def(
                "light_set",
                "Set light properties (one undo step), as the light's Properties panel does. type changes the light's type like the panel's Type dropdown: shared settings and ones both types use stay, the rest start at the new type's defaults. intensity is converted on a type change to keep the brightness 1 m in front; intensity is the panel's number (W/srm²; what it measures depends on the type, see item_get intensity_measures); color is linear RGB 0-1; angles radians; sizes metres.",
                crate::properties::schema(crate::properties::LIGHT_FIELDS),
            ),
            def(
                "backdrop_set",
                "Set Backdrop properties (one undo step), as the Backdrop panel does. Make a Backdrop with command_run item.create type backdrop (or backdrop.add); item_get reads every property back. image: a picture file path (loaded into the scene's images, reused when already there), an image id, or null. Fractions 0-1 for contrast, brightness and transparency; pixel_size metres per pixel (only with uniform_size off; set uniform_size in the same call and it is applied first).",
                crate::properties::schema(crate::properties::BACKDROP_FIELDS),
            ),
            def(
                "meshop_set",
                "Set options of one Mesh Operation (one undo step), as its Properties do: op is op:<item>:<op> (item_get lists a mesh's ops with these addresses and their current values), values maps option names to new values, with the names and units item_get shows. That includes item inputs: Merge Meshes' sources (item ids) and its hierarchy switch, and a Boolean or Drill op's driver item. Values an op doesn't have are refused. Add ops with command_run meshop.create (command_help meshop.create lists every type); turn one on or off with deformer.enable.",
                json!({"type": "object", "properties": {
                    "op": {"type": "string", "description": "op:<item>:<op>"},
                    "values": {"type": "object"}
                }, "required": ["op", "values"], "additionalProperties": false}),
            ),
            def(
                "render_settings_get",
                "The scene's final-render settings: samples, denoise, adaptive_threshold, exposure (stops), the render camera and its picture size. Each field's unit is in units.",
                json!({"type": "object", "properties": {}, "additionalProperties": false}),
            ),
            def(
                "render_settings_set",
                "Set the scene's final-render settings (saved with the scene, one undo step). Fields not given keep their value. resolution sets the render camera's Resolution Override.",
                {
                    let mut s = crate::properties::schema(crate::properties::RENDER_FIELDS);
                    s["properties"]
                        .as_object_mut()
                        .expect("an object")
                        .remove("item");
                    s
                },
            ),
            def(
                "view_set",
                "Running app only: what a viewport shows: a named view, or look through a camera (Camera view; navigating then moves that camera), and its shading style (the view's Display menu / Viewport Properties > Shading Style, the same setting as view3d.shadingStyle). Works on the active viewport, or viewport (an index, 0 = first) which then becomes the active one. Answers the view, shading, viewport index and viewport count. Not an undo step; display settings are per view and not saved with the scene.",
                json!({"type": "object", "properties": {
                    "view": {"enum": VIEW_NAMES, "description": "camera = look through a camera (the camera argument, else the active camera, else the newest)."},
                    "camera": {"type": "integer", "description": "Camera item id to look through (implies view camera). A light item id looks through that light instead (Light view; navigating then moves the light)."},
                    "shading": {"enum": SHADING_STYLES, "description": "Shading style of the view's active meshes: wireframe (edges only), solid (flat unlit), vertex_map, shaded (lit), texture (material colour unlit), default (shaded plus image maps; new views start here), advanced (default plus shadows), gooch, cel, matcap (the view's matcap; its old name reflection works too). The Display menu's names (\"Vertex Map\", \"Gooch Tone Shading\") are accepted too."},
                    "viewport": {"type": "integer", "minimum": 0, "description": "Which viewport, by index in the layout (0 = first; the answer's viewports gives the count). Default the active viewport."}
                }, "additionalProperties": false}),
            ),
            def(
                "online_library",
                "Running app only: the Preset Browser's online libraries (Poly Haven: HDRIs, textures, models; ambientCG: HDRIs, materials; every asset CC0). Each is off until turned on; while off it is never contacted. action status: which are on, catalogue size and age, downloads running, the cache folder. enable / disable with source: as the library's Preferences switch. refresh: fetch catalogues again in the background. list: assets from the cached catalogues (source, kind, search over name, id, category and tags; limit default 50) with id, name, category, resolutions, downloaded resolutions and the cached file. get source kind id [resolution K, default the Download Resolution] [load]: downloads in the background (state downloading, with progress) or answers state ready with the cached file; load true loads it as the browser's double-click does (HDRI: the environment's image; texture: a material in the selected material group; model: imported), once, when it lands. Follow a download with status, not by calling get again.",
                json!({"type": "object", "properties": {
                    "action": {"enum": ["status", "enable", "disable", "refresh", "list", "get"]},
                    "source": {"enum": ["polyhaven", "ambientcg"]},
                    "kind": {"enum": ["hdri", "texture", "model"]},
                    "id": {"type": "string", "description": "The asset's id, from list."},
                    "search": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1},
                    "resolution": {"type": "integer", "description": "Thousands of pixels (1, 2, 4, 8, 16): the nearest offered at or below it, else the lowest."},
                    "load": {"type": "boolean", "description": "get: load it once it's downloaded. Default false."}
                }, "required": ["action"], "additionalProperties": false}),
            ),
            def(
                "panel_show",
                "Running app only: bring a side panel to the front of its tab group so a viewport_capture or a screen shot shows it, as clicking its tab does; a panel that was closed is added back to its group (Properties to the lower group, the others to the upper). Panel layout only: no undo step, no scene change, never opens a dialog. Answers each group's tabs and which is in front.",
                json!({"type": "object", "properties": {
                    "panel": {"enum": PANEL_NAMES, "description": "items (Items list), shader_tree (Shader Tree), groups, properties, mesh_stack (Mesh Stack), preset_browser (Preset Browser), uv_maps (UV Maps), command_history (Command History), vertex_normal_tools (Vertex Normal Tools), statistics (Statistics), node_view (Node View)."}
                }, "required": ["panel"], "additionalProperties": false}),
            ),
            def(
                "app_command",
                "Running app only: run one of the app's own command lines, as typed on its command line: the paint tools (operandi.paint.airbrush, operandi.paint.set size:30 fg:1,1,1), image commands (operandi.paint.addTexture kind:color resolution:1024 color:0,0,0, operandi.image.new resolution:512 name:Skin, operandi.image.saveAs filename:/x/skin.png, operandi.image.save) and every other app command that command_run can't reach. Recorded in the History tab as a bridge line. A line that would open a dialog (a bare operandi.image.new) is an error and the dialog waits in the app for the person: give the command its values. Runs while a tool is on (it may be what turns the tool on). Answers the scene echo, the line and the status line.",
                json!({"type": "object", "properties": {
                    "line": {"type": "string", "description": "The command line, e.g. \"operandi.paint.smudge\"."}
                }, "required": ["line"], "additionalProperties": false}),
            ),
            def(
                "pointer_stroke",
                "Running app only: press, drag and release a mouse button in a viewport through the app's real input path, one point per frame (the first point is pressed, the last released; one point is a click). What it does is what that drag does in the app: paint with a paint tool on, sculpt, select, drag a tool's handle, navigate with alt. Give points in view points from the view's top-left corner, or world points projected through the view's camera. pressure (0-1, one number or one per point) sends the points as a pen's, so the Tablet Nozzle and sculpt brushes read it; without it they are a mouse's (full pressure). Answered after the release has been handled, with the scene echo and the status line. Runs while a tool is on.",
                json!({"type": "object", "properties": {
                    "points": {"type": "array", "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}, "description": "[[x, y], ...] in points from the view's top-left corner; 1 to 4000."},
                    "world": {"type": "array", "items": vec3, "description": "[[x, y, z], ...] world points (metres) instead of points; each must be in front of the camera and inside the view."},
                    "viewport": {"type": "integer", "description": "Which view (index, 0 = first); it becomes the active one. Default the active view."},
                    "button": {"enum": ["left", "right", "middle"], "description": "Default left."},
                    "modifiers": {"type": "array", "items": {"enum": ["shift", "ctrl", "alt", "cmd"]}, "description": "Keys held for the whole stroke; cmd is Command on macOS, Control elsewhere."},
                    "pressure": {"description": "Pen pressure 0-1: one number for every point, or a list with one per point."}
                }, "additionalProperties": false}),
            ),
            def(
                "viewport_capture",
                "Running app only: a picture of a viewport as the person sees it (grid, wireframes, item glyphs; not the tool panels). Saved as PNG and returned as an image unless include_image is false. The view is put back afterwards.",
                json!({"type": "object", "properties": {
                    "view": {"enum": ["current", "perspective", "orthographic", "top", "bottom", "front", "back", "right", "left", "camera"], "description": "current (default) = the active viewport as it is; a named view; or camera = through the scene's render camera."},
                    "camera": {"type": "integer", "description": "Look through this camera item (implies view camera)."},
                    "path": {"type": "string", "description": "Where to save the PNG; default the next 'Viewport NNN.png' in ~/Pictures/Operandi Renders."},
                    "include_image": {"type": "boolean", "description": "Return the picture in the result (default true)."},
                    "max_size": {"type": "integer", "description": "Longest side of the returned image in pixels (default 1024; the saved file keeps full size)."}
                }, "additionalProperties": false}),
            ),
            def(
                "render",
                "Running app only: a final Cycles render (Render > Render Viewport) to an image file, from the scene's render camera (or the given camera). Waits for the picture up to timeout seconds and returns it as an image (scaled to max_size; include_image false leaves it out), or returns while the render goes on; render_status reports it (include_image for its picture). Needs a build with Cycles.",
                json!({"type": "object", "properties": {
                    "path": {"type": "string", "description": "Output file (.png, .jpg); default the next 'Render NNN' in ~/Pictures/Operandi Renders."},
                    "width": {"type": "integer", "description": "Picture width in pixels (default the camera's render size). Give both width and height, or neither."},
                    "height": {"type": "integer", "description": "Picture height in pixels."},
                    "samples": {"type": "integer", "description": "Samples per pixel (default the renderer's)."},
                    "camera": {"type": "integer", "description": "Render through this camera instead of the render camera."},
                    "wait": {"type": "boolean", "description": "Wait for the render (default true)."},
                    "timeout": {"type": "number", "description": "Seconds to wait (default 300, at most 3600)."},
                    "include_image": {"type": "boolean", "description": "Return the finished picture in the result (default true; the saved file keeps full size)."},
                    "max_size": {"type": "integer", "description": "Longest side of the returned image in pixels (default 1024, 16 to 8192)."}
                }, "additionalProperties": false}),
            ),
            def(
                "render_status",
                "Running app only: whether a render is running (with progress) and the last finished render's file, size, samples and time. include_image returns that render's picture too.",
                json!({"type": "object", "properties": {
                    "include_image": {"type": "boolean", "description": "Return the last finished render's picture (default false)."},
                    "max_size": {"type": "integer", "description": "Longest side of the returned image in pixels (default 1024, 16 to 8192)."}
                }, "additionalProperties": false}),
            ),
            def(
                "shader_tree_get",
                "The shader tree (Shading panel), top row first: each layer's id, parent, depth, name, type, effect and eye; Material Groups show their polygon tag, Materials the material tag they shade. Pass layer for one layer in full: Layer block (enable, invert, blend, opacity), channels with their units, and type settings (Gradient input and keys, Vertex Map map, Image Map image).",
                json!({"type": "object", "properties": {"layer": {"type": "integer", "description": "Layer id for one layer in full."}}, "additionalProperties": false}),
            ),
            def(
                "material_get",
                "One Material layer's channels, as the Shading panel's Properties show them: diffuse_color, diffuse_amount, roughness, specular_amount, metallic, clearcoat_amount, anisotropy, transparent_amount, luminous_color, luminous_intensity, subsurface_amount and the rest, each with its unit (fractions 0-1, radians, metres, linear RGB 0-1), plus shading_model. material is the polygon material tag (poly.setMaterial name; default \"Default\") or a Material layer's name; layer is a layer id.",
                json!({"type": "object", "properties": {
                    "material": {"type": "string", "description": "Material tag or Material layer name; default \"Default\"."},
                    "layer": {"type": "integer", "description": "Material layer id (instead of material)."}
                }, "additionalProperties": false}),
            ),
            def(
                "material_set",
                "Set a Material's channels as the Shading panel does (one undo step). Channels go at the top level or under channels, by material_get's names, e.g. {\"material\": \"Red\", \"diffuse_color\": [0.8, 0.05, 0.05], \"roughness\": 0.25, \"metallic\": 1}. Units: fractions 0-1 (the panel's percent), radians, metres, linear RGB 0-1. Also name, enable, opacity, blend, invert, shading_model (Principled, Physically Based, Traditional, Energy Conserving). An unknown channel is an error listing the right names.",
                json!({"type": "object", "properties": {
                    "material": {"type": "string", "description": "Material tag or Material layer name; default \"Default\"."},
                    "layer": {"type": "integer", "description": "Material layer id (instead of material)."},
                    "channels": {"type": "object", "description": "Channel values by name (same as giving them at the top level)."},
                    "shading_model": {"enum": ["Principled", "Physically Based", "Traditional", "Energy Conserving"]}
                }, "additionalProperties": true}),
            ),
            def(
                "shader_layer_add",
                "Add a shader tree layer, as Add Layer does, and set its settings (one undo step); returns its id in layer. type: material, group, folder, shader, constant, image_map, gradient, vertex_map, variation, process, occlusion, or a procedural (checker, grid, dots, noise...). Where: inside parent (a Material Group or Folder id), or with material (a material tag) in that material's group above the Material, else at the top level below the Base Shader. A new texture's effect is Diffuse Color. Settings are shader_layer_set's.",
                {
                    let mut s = layer_settings_schema();
                    let p = s["properties"].as_object_mut().expect("an object");
                    p.insert(
                        "type".into(),
                        json!({"type": "string", "description": "Layer type (above)."}),
                    );
                    p.insert("parent".into(), json!({"type": "integer", "description": "Material Group or Folder layer id to add into."}));
                    p.insert("material".into(), json!({"type": "string", "description": "Add into this material tag's group, above its Material."}));
                    s["required"] = json!(["type"]);
                    s
                },
            ),
            def(
                "shader_layer_set",
                "Change one shader tree layer (one undo step), as its Properties do: name, enable, invert, blend, opacity (0-1); textures: effect (Diffuse Color, Roughness, Bump...); Gradient: input (Incidence, Slope, Distance to Camera...) and value_keys / color_keys [{at, value | color [r,g,b], alpha?, shape?: smooth|linear|stepped}] with at in the input's unit (radians for angles, metres, fractions); Variation: source and the same keys; Vertex Map: map {name, kind: weight|color}; Image Map: image (a PNG/JPEG path, a scene image id, or null); Material Group: tag (polygon material tag, or \"(all)\"); Material: shading_model; channels {name: value} by shader_tree_get's names and units (Constant: color, value).",
                {
                    let mut s = layer_settings_schema();
                    s["properties"].as_object_mut().expect("an object").insert(
                        "layer".into(),
                        json!({"type": "integer", "description": "Layer id (shader_tree_get)."}),
                    );
                    s["required"] = json!(["layer"]);
                    s
                },
            ),
            def(
                "subdivision_set",
                "Subdivision surfaces on a mesh (one undo step): toggle polygons between Face and Subdivision (Tab) or Catmull-Clark (Shift+Tab), and set the mesh's display and render levels and boundary rules. item_get reports the levels and how many polygons of each kind. Fields: item (default the active mesh), toggle, subdivision_level, catmull_clark_level, subdivision_render_level, catmull_clark_render_level (0-6), boundary_rule, uv_boundary_rule, linear_uvs.",
                {
                    let mut s = crate::properties::schema(crate::shading::SUBDIVISION_FIELDS);
                    s["properties"]["toggle"]["enum"] = json!(["subdivision", "catmull_clark"]);
                    s
                },
            ),
            def(
                "undo",
                "Undo the last history step (count times).",
                json!({"type": "object", "properties": {"count": {"type": "integer", "minimum": 1}}}),
            ),
            def(
                "redo",
                "Redo the next history step (count times).",
                json!({"type": "object", "properties": {"count": {"type": "integer", "minimum": 1}}}),
            ),
        ]
    }

    /// MCP `tools/call`. Returns structured JSON, or an error message that
    /// the protocol layer reports with `isError: true`.
    pub fn call(&mut self, name: &str, args: &Value) -> Result<Value> {
        let args = match args {
            Value::Null => &Map::new(),
            Value::Object(m) => m,
            _ => return Err("Arguments must be an object".into()),
        };
        match name {
            "scene_info" => Ok(self.scene_info()),
            "mesh_get" => self.mesh_get(args),
            "scene_new" => {
                *self = if opt_bool(args, "empty_mesh")?.unwrap_or(true) {
                    Self::new()
                } else {
                    Self::default()
                };
                Ok(self.summary())
            }
            "scene_load" => {
                let path = req_str(args, "path")?;
                // BRQ-60: a mesh with polygons Mesh Cleanup fixes opens as-is.
                let scene = persistence::load_for_bridge(std::path::Path::new(path))?;
                self.editor = Editor {
                    scene: scene.clone(),
                    ..Default::default()
                };
                self.document = Document {
                    path: Some(path.into()),
                    saved: scene,
                };
                Ok(self.summary())
            }
            "scene_save" => {
                let (path, copy) = scene_save_args(args)?;
                self.editor.save_committed(std::path::Path::new(path))?;
                if !copy {
                    self.document = Document {
                        path: Some(path.into()),
                        saved: self.editor.scene.clone(),
                    };
                }
                Ok(json!({"saved": path}))
            }
            "preset_save" => {
                use operandi_core::preset::{PresetKind, PresetSource};
                let path = req_str(args, "path")?;
                let kind = PresetKind::from_script_name(req_str(args, "kind")?).ok_or(
                    "kind must be item, assembly, environment, renderSettings or material",
                )?;
                let lights = opt_bool(args, "lights")?.unwrap_or(true);
                let meshes = opt_bool(args, "meshes")?.unwrap_or(false);
                let cameras = opt_bool(args, "cameras")?.unwrap_or(false);
                let node = opt_u64(args, "node")?;
                let source = match kind {
                    PresetKind::Item => PresetSource::Items,
                    PresetKind::Assembly => PresetSource::Assembly,
                    // Lights on unless lights:false; meshes only when asked.
                    PresetKind::Environment => PresetSource::Environment {
                        lights,
                        meshes,
                        cameras,
                    },
                    PresetKind::RenderSettings => PresetSource::RenderSettings,
                    PresetKind::Material => PresetSource::Material(
                        node.ok_or("node is required for a material preset")?,
                    ),
                };
                let notes = self
                    .editor
                    .save_preset(std::path::Path::new(path), source)?;
                Ok(json!({"saved": path, "kind": kind.script_name(), "notes": notes}))
            }
            "preset_load" => {
                let path = req_str(args, "path")?;
                let preset = persistence::load_preset(std::path::Path::new(path))?;
                let group = opt_u64(args, "group")?;
                self.staged(|e| {
                    e.dispatch(Command::LoadPreset {
                        preset: Box::new(preset),
                        group,
                    })
                })
            }
            "python_run" => {
                let code = req_str(args, "code")?.to_owned();
                let mut printed = String::new();
                let mut out = self.staged(|e| {
                    let outcome = operandi_python::run_blocking(
                        &mut operandi_python::EditorHost(e),
                        operandi_python::Source::file_text(code, "<bridge>"),
                    );
                    printed = outcome.output;
                    match outcome.error {
                        None => Ok(()),
                        Some(tb) => Err(format!("{printed}{tb}")),
                    }
                })?;
                out["output"] = json!(printed);
                Ok(out)
            }
            "select" => self.staged(|e| select(e, args)),
            "selection_op" => self.staged(|e| selection_op(e, args)),
            "tool_run" => self.staged(|e| tool_run(e, args)),
            // Queries answer without staging (item.channel value:"?").
            "command_run"
                if let (Some(given), Some(Value::Object(given_args))) = (
                    args.get("command").and_then(Value::as_str),
                    args.get("args"),
                ) && let Some(answer) =
                    operandi_core::command_exec::answer(&self.editor, given, given_args) =>
            {
                answer.map(|value| json!({ "value": value }))
            }
            "command_run" => self.staged(|e| {
                let given = req_str(args, "command")?;
                let empty = Map::new();
                let given_args = match args.get("args") {
                    None | Some(Value::Null) => &empty,
                    Some(Value::Object(m)) => m,
                    _ => return Err("args must be an object".into()),
                };
                operandi_core::command_exec::run(e, given, given_args)
            }),
            "primitive_add" => self.staged(|e| crate::primitives::primitive_add(e, args)),
            "scene_import" => {
                let mut report = Value::Null;
                let mut out = self.staged(|e| {
                    report = crate::import::scene_import(e, args)?;
                    Ok(())
                })?;
                out["imported"] = report["imported"].take();
                out["notes"] = report["notes"].take();
                Ok(out)
            }
            "command_help" => {
                crate::commands::check_keys(name, args, &["command"])?;
                crate::commands::help(args.get("command").and_then(Value::as_str))
            }
            "item_get" => crate::properties::item_get(&self.editor, args),
            "camera_set" => self.staged(|e| crate::properties::camera_set(e, args)),
            "light_set" => self.staged(|e| crate::properties::light_set(e, args)),
            "backdrop_set" => self.staged(|e| crate::properties::backdrop_set(e, args)),
            "meshop_set" => self.staged(|e| crate::properties::meshop_set(e, args)),
            "render_settings_get" => crate::properties::render_settings_get(&self.editor, args),
            "render_settings_set" => {
                self.staged(|e| crate::properties::render_settings_set(e, args))
            }
            "shader_tree_get" => crate::shading::tree_get(&self.editor, args),
            "material_get" => crate::shading::material_get(&self.editor, args),
            "material_set" => self.staged(|e| crate::shading::material_set(e, args)),
            "shader_layer_set" => self.staged(|e| crate::shading::layer_set(e, args)),
            "shader_layer_add" => {
                let mut id = 0;
                let mut out = self.staged(|e| {
                    id = crate::shading::layer_add(e, args)?;
                    Ok(())
                })?;
                out["layer"] = json!(id);
                Ok(out)
            }
            "subdivision_set" => self.staged(|e| crate::shading::subdivision_set(e, args)),
            app if APP_TOOLS.contains(&app) => Err(format!("{app} {APP_ONLY}")),
            "undo" | "redo" => {
                let count = opt_u64(args, "count")?.unwrap_or(1);
                let available = if name == "undo" {
                    self.editor.history.undo.len()
                } else {
                    self.editor.history.redo.len()
                };
                if count as usize > available {
                    return Err(format!("Only {available} step(s) to {name}"));
                }
                let cmd = if name == "undo" {
                    Command::Undo
                } else {
                    Command::Redo
                };
                self.staged(|e| (0..count).try_for_each(|_| e.dispatch(cmd.clone())))
            }
            _ => Err(format!("Unknown tool {name}")),
        }
    }

    fn staged(&mut self, f: impl FnOnce(&mut Editor) -> Result<()>) -> Result<Value> {
        let mut staged = self.editor.clone();
        f(&mut staged)?;
        // Frame renders and playblasts (TL-70/71) need the app's renderer
        // and views.
        if !self.attached && !staged.anim.requests.is_empty() {
            return Err(APP_ONLY.into());
        }
        self.editor = staged;
        Ok(self.summary())
    }

    /// The short state echo returned after every edit.
    pub fn summary(&self) -> Value {
        let e = &self.editor;
        let mesh = e.scene.active().map(|i| {
            json!({"item": i.id, "vertices": i.mesh.vertices.len(),
                   "edges": i.mesh.edges().len(), "faces": i.mesh.faces.len()})
        });
        json!({
            "revision": e.revision,
            "active": mesh,
            "selection": selection_json(e),
            "undo": e.history.undo.len(),
            "redo": e.history.redo.len(),
        })
    }

    pub fn scene_info(&self) -> Value {
        let mut info = self.scene_info_without_document();
        info["document"] = json!({
            "path": self.document.path,
            "name": self.document.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy()),
            "modified": self.editor.scene != self.document.saved,
        });
        info
    }

    /// `scene_info` before the document fields, which the app fills from
    /// its own document.
    pub fn scene_info_without_document(&self) -> Value {
        let e = &self.editor;
        let items: Vec<Value> = e
            .scene
            .items
            .iter()
            .map(|i| {
                json!({
                    "id": i.id,
                    "name": i.name,
                    "kind": item_kind(i),
                    "parent": i.parent,
                    "visible": i.visible,
                    "transform": {
                        "translation": v3(i.transform.translation),
                        "rotation": v3(i.transform.rotation),
                        "scale": v3(i.transform.scale),
                    },
                    "vertices": i.mesh.vertices.len(),
                    "edges": i.mesh.edges().len(),
                    "faces": i.mesh.faces.len(),
                })
            })
            .collect();
        json!({
            "revision": e.revision,
            "items": items,
            "selection": selection_json(e),
            "live_tool": e.live.as_ref().map(|l| l.tool.label()),
            "symmetry": {"axis": e.symmetry.axis, "offset": e.symmetry.offset},
            "undo": e.history.undo.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
            "redo": e.history.redo.iter().rev().map(|c| c.label.as_str()).collect::<Vec<_>>(),
        })
    }

    fn mesh_get(&self, args: &Map<String, Value>) -> Result<Value> {
        let scene = &self.editor.scene;
        let item = match opt_u64(args, "item")? {
            Some(id) => scene
                .items
                .iter()
                .find(|i| i.id == id)
                .ok_or(format!("No item {id}"))?,
            None => scene.active().ok_or("No active item")?,
        };
        crate::commands::check_keys(
            "mesh_get",
            args,
            &[
                "item",
                "world",
                "edges",
                "subdivided",
                "uvs",
                "evaluated",
                "polygon_types",
            ],
        )?;
        // BRQ-91: the result of the item's Mesh Operations, as renders and
        // exports read it (current, never stale).
        let evaluated = opt_bool(args, "evaluated")?.unwrap_or(false);
        let result = if evaluated && item.mesh_stack.is_some() {
            let r = self
                .editor
                .mesh_stack
                .current(scene, item)
                .ok_or("The Mesh Operations could not be evaluated")?;
            Some(r)
        } else {
            None
        };
        let base: &operandi_core::Mesh = match &result {
            Some(r) => &r.mesh,
            None => &item.mesh,
        };
        let world = opt_bool(args, "world")?.unwrap_or(false);
        // BRQ-40: the surface as displayed after subdivision, at the mesh's
        // display levels (true) or at one level for both kinds.
        let mut sources = None;
        let shown: std::borrow::Cow<operandi_core::Mesh> = match args.get("subdivided") {
            None | Some(Value::Null) | Some(Value::Bool(false)) => std::borrow::Cow::Borrowed(base),
            Some(v) => {
                let mut settings = item.subdivision.clone().unwrap_or_default();
                match v {
                    Value::Bool(true) => {}
                    v => {
                        let level = v
                            .as_u64()
                            .filter(|n| *n <= 6)
                            .ok_or("subdivided is true or a level from 0 to 6")?
                            as u8;
                        settings.subdivision_level = level;
                        settings.catmull_clark_level = level;
                    }
                }
                let (mesh, source) = operandi_core::subdivision::display_surface(base, &settings)?;
                sources = Some(source);
                std::borrow::Cow::Owned(mesh)
            }
        };
        let vertices: Vec<Value> = shown
            .vertices
            .iter()
            .map(|&p| {
                v3(if world {
                    scene.world_point(item.id, p)
                } else {
                    p
                })
            })
            .collect();
        let types = evaluated || opt_bool(args, "polygon_types")?.unwrap_or(false);
        let reps = &shown.attributes.representations;
        let faces: Vec<Value> = shown
            .faces
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let mut face =
                    json!({"vertices": f.vertices, "material": f.material, "part": f.part});
                if types {
                    face["type"] = json!(polygon_type(reps.get(i).copied().unwrap_or_default()));
                }
                face
            })
            .collect();
        let mut out = json!({"item": item.id, "name": item.name, "space": if world { "world" } else { "item" },
                             "vertices": vertices, "faces": faces,
                             "smoothing": smoothing_json(&scene.smoothing)});
        // BRQ-70: every UV map, one UV per polygon corner (null where the
        // map has none), on the polygons returned.
        if opt_bool(args, "uvs")?.unwrap_or(false) {
            out["uv_maps"] = Value::Object(
                shown
                    .uv_maps
                    .iter()
                    .map(|(name, faces)| {
                        let faces: Vec<Value> = (0..shown.faces.len())
                            .map(|f| {
                                let corners = shown.faces[f].vertices.len();
                                let given = faces.get(f);
                                json!(
                                    (0..corners)
                                        .map(|c| {
                                            given
                                                .and_then(|uv| uv.get(c).copied().flatten())
                                                .map(|uv| json!([uv.x, uv.y]))
                                                .unwrap_or(Value::Null)
                                        })
                                        .collect::<Vec<_>>()
                                )
                            })
                            .collect();
                        (name.clone(), Value::Array(faces))
                    })
                    .collect(),
            );
        }
        if let Some(r) = &result {
            out["evaluated"] = json!(true);
            out["mesh_stack_status"] = json!(r.statuses.iter().map(op_status).collect::<Vec<_>>());
        }
        if let Some(source) = sources {
            out["subdivided"] = json!(true);
            out["source_faces"] = json!(source);
        }
        if opt_bool(args, "edges")?.unwrap_or(true) {
            out["edges"] = json!(
                shown
                    .edges()
                    .iter()
                    .map(|&(a, b)| [a, b])
                    .collect::<Vec<_>>()
            );
        }
        Ok(out)
    }
}

/// A polygon's type word (BRQ-91).
fn polygon_type(r: operandi_core::PolygonRepresentation) -> String {
    use operandi_core::PolygonRepresentation as P;
    use operandi_core::curves::CurveKind as C;
    match r {
        P::Face => "face".into(),
        P::Subdivs => "subdivision".into(),
        P::CatmullClark => "catmullClark".into(),
        P::Patch => "patch".into(),
        P::Polyline { closed, curve } => {
            let kind = match curve {
                C::Line => "polyline",
                C::Curve { .. } => "curve",
                C::BSpline => "bspline",
                C::Bezier => "bezier",
            };
            if closed {
                format!("closed_{kind}")
            } else {
                kind.into()
            }
        }
    }
}

/// One op's evaluation status (BRQ-91/94): ok, off, or failed with why.
pub fn op_status(s: &operandi_core::mesh_stack::OpStatus) -> Value {
    use operandi_core::mesh_stack::OpStatus as S;
    match s {
        S::Ok => json!("ok"),
        S::Off => json!("off"),
        S::Failed(why) => json!({"failed": why}),
        S::Noted(notes) => json!({"notes": notes
            .iter()
            .map(|n| json!({"item": n.item, "text": n.text, "problem": n.problem}))
            .collect::<Vec<_>>()}),
    }
}

/// An item's kind, as `item_get` names it (BRQ-30).
fn item_kind(i: &operandi_core::Item) -> &'static str {
    if i.camera.is_some() {
        "camera"
    } else if i.light.is_some() {
        "light"
    } else if i.backdrop.is_some() {
        "backdrop"
    } else if i.is_group_locator() {
        "groupLocator"
    } else {
        "mesh"
    }
}

/// Views `view_set` and `viewport_capture` name.
pub const VIEW_NAMES: [&str; 9] = [
    "perspective",
    "orthographic",
    "top",
    "bottom",
    "front",
    "back",
    "right",
    "left",
    "camera",
];

/// Shading styles `view_set shading` names, in the Display menu's order (the
/// app's `view_display::Style::ALL`; a test there keeps the two alike).
pub const SHADING_STYLES: [&str; 10] = [
    "wireframe",
    "solid",
    "vertex_map",
    "shaded",
    "texture",
    "default",
    "advanced",
    "gooch",
    "cel",
    "matcap",
];

/// Side panels `panel_show` names (the app's panel registry; a test there
/// keeps the two alike).
pub const PANEL_NAMES: [&str; 11] = [
    "items",
    "shader_tree",
    "groups",
    "properties",
    "mesh_stack",
    "uv_maps",
    "preset_browser",
    "command_history",
    "vertex_normal_tools",
    "statistics",
    "node_view",
];

/// Settings `shader_layer_set` and `shader_layer_add` share.
fn layer_settings_schema() -> Value {
    let keys = json!({"type": "array", "items": {"type": "object"}});
    json!({"type": "object", "properties": {
        "name": {"type": "string"},
        "enable": {"type": "boolean"},
        "invert": {"type": "boolean"},
        "blend": {"type": "string", "description": "Normal, Add, Subtract, Multiply, Screen, Overlay..."},
        "opacity": {"type": "number", "description": "0-1"},
        "effect": {"type": "string", "description": "Texture effect: Diffuse Color, Diffuse Amount, Roughness, Metallic, Bump, Displacement..."},
        "shading_model": {"type": "string"},
        "tag": {"type": "string", "description": "Material Group: polygon material tag, or (all)."},
        "input": {"type": "string", "description": "Gradient input: Incidence, Slope, Texture Value, Distance to Camera, Texture U..."},
        "value_keys": described(&keys, "Gradient/Variation value curve: [{at, value, shape?}]."),
        "color_keys": described(&keys, "Gradient/Variation colour curve: [{at, color [r, g, b], alpha?, shape?}]."),
        "map": {"type": ["object", "null"], "description": "Vertex Map: {name, kind: weight or color}."},
        "source": {"type": "string", "description": "Variation source: Mesh Part, Item, Particle, Texture Particle."},
        "image": {"type": ["string", "integer", "null"], "description": "Image Map: PNG/JPEG path, scene image id, or null."},
        "channels": {"type": "object", "description": "Channel values by name (shader_tree_get layer lists them with units)."}
    }, "additionalProperties": false})
}

/// `schema` with a description added.
fn described(schema: &Value, description: &str) -> Value {
    let mut s = schema.clone();
    s["description"] = json!(description);
    s
}

fn def(name: &str, description: &str, schema: Value) -> Value {
    json!({"name": name, "description": description, "inputSchema": schema})
}

/// The selection: the mode, and each mode's own list in pick order. Every
/// mode keeps its own list while another mode is current (pressing 1, 2 or
/// 3 shows it again), and tools leave new elements in the other modes'
/// lists too (Make Polygon's new polygon, Edge Subdivide's and Edge
/// Bevel's new points), as the spec oracles record them. The tools work
/// on `mode`'s list.
fn selection_json(e: &Editor) -> Value {
    let s = &e.scene.selection;
    let edges: Vec<[u32; 2]> = s.edges.iter().map(|&(a, b)| [a, b]).collect();
    json!({
        "item": s.item,
        "items": e.scene.selected_items(),
        "mode": mode_name(s.mode),
        "vertices": &s.vertices,
        "edges": edges,
        "faces": &s.faces,
    })
}

/// The smooth-shading rule in the bridge's units (fraction, radians).
fn smoothing_json(s: &operandi_core::shading::SmoothingSettings) -> Value {
    json!({
        "amount": s.amount / 100.,
        "angle": s.angle.to_radians(),
        "crease_by_angle": s.crease_by_angle,
        "groups": format!("{:?}", s.groups).to_lowercase(),
    })
}

fn mode_name(m: Mode) -> &'static str {
    match m {
        Mode::Vertex => "vertex",
        Mode::Edge => "edge",
        Mode::Polygon => "polygon",
        Mode::Item => "item",
    }
}

fn parse_mode(s: &str) -> Result<Mode> {
    Ok(match s {
        "vertex" => Mode::Vertex,
        "edge" => Mode::Edge,
        "polygon" => Mode::Polygon,
        "item" => Mode::Item,
        _ => return Err(format!("Unknown mode {s}")),
    })
}

fn select(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    crate::commands::check_keys(
        "select",
        args,
        &["mode", "item", "vertices", "edges", "faces", "add"],
    )?;
    let mode = parse_mode(req_str(args, "mode")?)?;
    if e.scene.selection.mode != mode {
        e.dispatch(Command::Mode(mode))?;
    }
    let item = match opt_u64(args, "item")? {
        Some(id) => id,
        None => e.scene.selection.item.ok_or("No active item; pass item")?,
    };
    if mode == Mode::Item {
        // add: the item joins the selection as the newest pick (and the
        // primary), so pick-order commands read it last (rig-pick-order).
        if opt_bool(args, "add")?.unwrap_or(false) {
            if e.scene.selected_items().contains(&item) && e.scene.selection.item != Some(item) {
                // Already picked earlier: picking it again makes it newest.
                e.dispatch(Command::ToggleItemSelection(item))?;
            } else if e.scene.selection.item == Some(item) {
                return Ok(());
            }
            return e.dispatch(Command::ToggleItemSelection(item));
        }
        return e.dispatch(Command::SelectItem(Some(item)));
    }
    // The item becomes the active one in component modes too, so later
    // tools and primitive_add work on it.
    if e.scene.selection.item != Some(item) {
        e.dispatch(Command::SelectItem(Some(item)))?;
    }
    let mut hits = Vec::new();
    for v in u32_list(args, "vertices")? {
        hits.push(Component::Vertex(v));
    }
    if let Some(edges) = args.get("edges") {
        for pair in edges.as_array().ok_or("edges must be an array of pairs")? {
            match pair.as_array().map(|p| p.as_slice()) {
                Some([a, b]) => hits.push(Component::Edge(as_u32(a)?, as_u32(b)?)),
                _ => return Err("Each edge is a [a, b] pair".into()),
            }
        }
    }
    for f in u32_list(args, "faces")? {
        hits.push(Component::Polygon(f));
    }
    // SYM-SEL-07: a pick made by script selects only what it names, even
    // with Symmetry on; only pointer picks add the mirror (SYM-B2-10).
    // Symmetry is set aside for the picks and put back after.
    let axis = e.symmetry.axis.take();
    let picked = (|| {
        e.dispatch(Command::BeginSelection)?;
        if !opt_bool(args, "add")?.unwrap_or(false) {
            e.dispatch(Command::ClearSelection)?;
        }
        for hit in hits {
            e.dispatch(Command::Select {
                item,
                hit,
                replace: false,
                remove: false,
            })?;
        }
        e.dispatch(Command::EndSelection { cancel: false })
    })();
    e.symmetry.axis = axis;
    picked
}

fn selection_op(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    use SelectionOperation as S;
    let op = match req_str(args, "op")? {
        "grow" => S::Grow,
        "shrink" => S::Shrink,
        "loop" => S::Loop,
        "ring" => S::Ring,
        "all" => S::All,
        "invert" => S::Invert,
        "connected" => S::Connected,
        "invert_connected" => S::InvertConnected,
        "boundary" => S::Boundary,
        "more" => S::More,
        "less" => S::Less,
        "between" => S::Between,
        "close_loop" => S::CloseLoop,
        "next_loop" => S::NextLoop,
        "previous_loop" => S::PreviousLoop,
        "convert" => S::Convert(parse_mode(req_str(args, "to")?)?),
        other => return Err(format!("Unknown selection op {other}")),
    };
    e.dispatch(Command::Selection(op))
}

/// Logs an old name's first use this run (stderr; stdout is the protocol).
fn note_deprecated(old: &str, new: &str) {
    if let Some(note) = command_names::first_deprecation(old, new) {
        eprintln!("{note}");
    }
}

/// `tool_run`'s falloffs: the Add Falloff menu's Vertex Map, Invert and
/// Selection (`falloff.vertexMap mode index mix`, `falloff.invert`,
/// `falloff.selection steps shape p0 p1 mix`), in stack order.
fn falloff_list(
    v: Option<&Value>,
) -> Result<Vec<(operandi_core::Falloff, operandi_core::MixMode)>> {
    use operandi_core::{Falloff, MixMode, VertexMapFalloff, VertexMapMode};
    let list = match v {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(list)) => list,
        Some(_) => return Err("falloffs is an array of {type, mode?, index?, mix?}".into()),
    };
    list.iter()
        .map(|f| {
            let f = f.as_object().ok_or("each falloff is an object")?;
            let kind = f
                .get("type")
                .and_then(Value::as_str)
                .ok_or("each falloff needs a type: vertexMap, invert or selection")?;
            let text = |k: &str| -> Result<Option<String>> {
                f.get(k)
                    .filter(|v| !v.is_null())
                    .map(|v| {
                        v.as_str()
                            .map(str::to_ascii_lowercase)
                            .ok_or(format!("falloff {k} is a name"))
                    })
                    .transpose()
            };
            let mix = match text("mix")?.as_deref() {
                None | Some("multiply") => MixMode::Multiply,
                Some("add") => MixMode::Add,
                Some("subtract") => MixMode::Subtract,
                Some("max") => MixMode::Max,
                Some("min") => MixMode::Min,
                Some(_) => return Err("falloff mix is multiply, add, subtract, max or min".into()),
            };
            let falloff = match kind {
                k if k.eq_ignore_ascii_case("vertexMap")
                    || k.eq_ignore_ascii_case("falloff.vertexMap") =>
                {
                    crate::commands::check_keys(
                        "falloff vertexMap",
                        f,
                        &["type", "mode", "index", "mix"],
                    )?;
                    let mut v = VertexMapFalloff::default();
                    match text("mode")?.as_deref() {
                        None | Some("magnitude") => {}
                        Some("component") => v.mode = VertexMapMode::Component,
                        Some(_) => return Err("falloff mode is magnitude or component".into()),
                    }
                    if let Some(i) = f.get("index").filter(|v| !v.is_null()) {
                        v.index = as_u32(i)?;
                    }
                    Falloff::VertexMap(v)
                }
                k if k.eq_ignore_ascii_case("invert")
                    || k.eq_ignore_ascii_case("falloff.invert") =>
                {
                    crate::commands::check_keys("falloff invert", f, &["type", "mix"])?;
                    Falloff::Invert
                }
                k if k.eq_ignore_ascii_case("selection")
                    || k.eq_ignore_ascii_case("falloff.selection") =>
                {
                    crate::commands::check_keys(
                        "falloff selection",
                        f,
                        &["type", "steps", "shape", "p0", "p1", "mix"],
                    )?;
                    let mut s = operandi_core::SelectionFalloff::default();
                    if let Some(n) = f.get("steps").filter(|v| !v.is_null()) {
                        s.steps = as_u32(n)?;
                    }
                    let number = |k: &str| -> Result<f64> {
                        match f.get(k).filter(|v| !v.is_null()) {
                            None => Ok(0.),
                            Some(v) => v.as_f64().ok_or(format!("falloff {k} is a number")),
                        }
                    };
                    s.shape = match text("shape")?.as_deref() {
                        None | Some("smooth") => operandi_core::FalloffShape::Smooth,
                        Some("linear") => operandi_core::FalloffShape::Linear,
                        Some("easein") => operandi_core::FalloffShape::EaseIn,
                        Some("easeout") => operandi_core::FalloffShape::EaseOut,
                        Some("custom") => operandi_core::FalloffShape::Custom {
                            ease_in: number("p0")?,
                            ease_out: number("p1")?,
                        },
                        Some(_) => {
                            return Err(
                                "falloff shape is linear, easeIn, easeOut, smooth or custom".into(),
                            );
                        }
                    };
                    let fall = Falloff::Selection(s);
                    fall.validate()?;
                    fall
                }
                // Falloff batch 2: the command line's keyed options, so
                // both entry points agree (falloff.softSelection etc.).
                // BRQ-20: the spatial falloffs, with the panel's options.
                k if ["linear", "radial", "cylinder"].iter().any(|n| {
                    k.eq_ignore_ascii_case(n) || k.eq_ignore_ascii_case(&format!("falloff.{n}"))
                }) =>
                {
                    spatial_falloff(k.trim_start_matches("falloff."), f)?
                }
                k if ["softSelection", "element", "coplanar", "noise", "curvature"].iter().any(|n| {
                    k.eq_ignore_ascii_case(n) || k.eq_ignore_ascii_case(&format!("falloff.{n}"))
                }) =>
                {
                    let name = k.trim_start_matches("falloff.").to_owned();
                    let keys: &[&str] = match name.to_ascii_lowercase().as_str() {
                        "softselection" => &["type", "radius", "connectivity", "shape", "p0", "p1", "mix"],
                        "element" => &["type", "mode", "range", "connect", "shape", "p0", "p1", "mix"],
                        "noise" => &["type", "scale", "world", "mix"],
                        "curvature" => &["type", "scale", "mix"],
                        _ => &["type", "angle", "connectivity", "shape", "p0", "p1", "mix"],
                    };
                    crate::commands::check_keys(&format!("falloff {name}"), f, keys)?;
                    let mut line = format!("falloff.{name}");
                    for &key in &keys[1..keys.len() - 1] {
                        match f.get(key).filter(|v| !v.is_null()) {
                            None => {}
                            Some(serde_json::Value::String(v)) => line += &format!(" {key}:{v}"),
                            Some(v) => line += &format!(" {key}:{v}"),
                        }
                    }
                    match operandi_core::command_line::parse_sculpt(&line) {
                        Some(operandi_core::command_line::SculptLine::Falloff { falloff, .. }) => {
                            falloff
                        }
                        Some(operandi_core::command_line::SculptLine::Usage(e)) => {
                            return Err(format!("falloff {name}: {e}"));
                        }
                        _ => return Err(format!("falloff {name} could not be read")),
                    }
                }
                other => {
                    return Err(format!(
                        "Unknown falloff type {other}; use linear, radial, cylinder, vertexMap, invert, selection, softSelection, element, coplanar, noise or curvature"
                    ));
                }
            };
            Ok((falloff, mix))
        })
        .collect()
}

/// `tool_run`'s `action_center` (BRQ-100): {mode, center?, axis?}.
fn action_center(v: Option<&Value>) -> Result<Option<operandi_core::ActionContext>> {
    use operandi_core::{ActionContext, AxisOrientation, CenterMode, PickedFrame, PickedPoint};
    let o = match v {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::Object(o)) => o,
        Some(_) => return Err("action_center is an object {mode, center?, axis?, normal?}".into()),
    };
    crate::commands::check_keys("action_center", o, &["mode", "center", "axis", "normal"])?;
    let mode = o
        .get("mode")
        .and_then(Value::as_str)
        .ok_or("action_center needs a mode")?
        .to_ascii_lowercase()
        .replace([' ', '-'], "_");
    let point = || -> Result<PickedPoint> {
        o.get("center")
            .ok_or(format!(
                "action_center {mode} needs a center [x, y, z] (world, metres)"
            ))
            .and_then(vec3)
            .map(PickedPoint::new)
    };
    let center = match mode.as_str() {
        "none" => CenterMode::None,
        "auto" | "automatic" => CenterMode::Automatic,
        "selection" => CenterMode::Selection,
        "selection_auto_axis" | "selection_center_auto_axis" => CenterMode::SelectionAutoAxis,
        "border" | "selection_border" => CenterMode::SelectionBorder,
        "origin" => CenterMode::Origin,
        "local" => CenterMode::Local,
        "parent" => CenterMode::Parent,
        "pivot" | "pivot_center_parent_axis" => CenterMode::Pivot,
        "element" => CenterMode::Element(point()?),
        "screen" | "manual" => CenterMode::Screen(point()?),
        "pivot_world_axis" | "parent_center" => {
            return Err(format!("Action Center {mode} isn't built yet"));
        }
        _ => {
            return Err(format!(
                "Unknown action center mode {mode}; use none, auto, selection, selection_auto_axis, border, element, screen, origin, local, parent, pivot or pivot_center_parent_axis"
            ));
        }
    };
    if !matches!(center, CenterMode::Element(_) | CenterMode::Screen(_)) && o.contains_key("center")
    {
        return Err(format!(
            "action_center {mode} takes no center; use element or screen for a given point"
        ));
    }
    let axis = match o
        .get("axis")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        None => match center {
            // AC-21: Selection Border's axes come from the selection;
            // AC-22: Auto Axis keeps World.
            CenterMode::SelectionBorder => AxisOrientation::Selection,
            CenterMode::Parent => AxisOrientation::Parent,
            CenterMode::Pivot if mode == "pivot_center_parent_axis" => AxisOrientation::Parent,
            CenterMode::Pivot => AxisOrientation::Item,
            _ => AxisOrientation::World,
        },
        Some("world") => AxisOrientation::World,
        Some("parent") => AxisOrientation::Parent,
        Some("item") => AxisOrientation::Item,
        Some("selection") => AxisOrientation::Selection,
        Some("workplane_xz") | Some("xz") => AxisOrientation::Workplane(0),
        Some("workplane_yz") | Some("yz") => AxisOrientation::Workplane(1),
        Some("workplane_xy") | Some("xy") => AxisOrientation::Workplane(2),
        Some("element") => AxisOrientation::Element(
            PickedFrame::from_z(
                o.get("normal")
                    .ok_or_else(|| "axis element needs a normal [x, y, z]".to_string())
                    .and_then(vec3)?,
            )
            .ok_or("axis element needs a non-zero normal")?,
        ),
        Some(other) => {
            return Err(format!(
                "Unknown action axis {other}; use world, selection, xz, yz, xy, element, parent or item"
            ));
        }
    };
    if o.contains_key("normal") && !matches!(axis, AxisOrientation::Element(_)) {
        return Err("action_center normal goes with axis element".into());
    }
    Ok(Some(ActionContext { center, axis }))
}

/// A Linear, Radial or Cylinder falloff from `tool_run`'s keys (BRQ-20),
/// over its Reset values. Lengths metres; shape linear, easeIn, easeOut,
/// smooth or custom with in/out (p0/p1).
fn spatial_falloff(kind: &str, f: &Map<String, Value>) -> Result<operandi_core::Falloff> {
    use operandi_core::falloff::{CylinderFalloff, FalloffSymmetric, LinearFalloff, RadialFalloff};
    use operandi_core::{Falloff, FalloffShape};
    let kind = kind.to_ascii_lowercase();
    let keys: &[&str] = match kind.as_str() {
        "linear" => &[
            "type",
            "start",
            "end",
            "shape",
            "in",
            "out",
            "p0",
            "p1",
            "symmetric",
            "world",
            "mix",
        ],
        "radial" => &[
            "type", "center", "size", "shape", "in", "out", "p0", "p1", "world", "mix",
        ],
        _ => &[
            "type", "center", "size", "axis", "shape", "in", "out", "p0", "p1", "world", "mix",
        ],
    };
    crate::commands::check_keys(&format!("falloff {kind}"), f, keys)?;
    let get = |k: &str| f.get(k).filter(|v| !v.is_null());
    let name = |k: &str| -> Result<Option<String>> {
        get(k)
            .map(|v| {
                v.as_str()
                    .map(str::to_ascii_lowercase)
                    .ok_or(format!("falloff {k} is a name"))
            })
            .transpose()
    };
    let number = |a: &str, b: &str| -> Result<f64> { get(a).or(get(b)).map_or(Ok(0.), num) };
    let shape = match name("shape")?.as_deref() {
        None | Some("linear") => FalloffShape::Linear,
        Some("easein") => FalloffShape::EaseIn,
        Some("easeout") => FalloffShape::EaseOut,
        Some("smooth") => FalloffShape::Smooth,
        Some("custom") => FalloffShape::Custom {
            ease_in: number("in", "p0")?,
            ease_out: number("out", "p1")?,
        },
        Some(_) => return Err("falloff shape is linear, easeIn, easeOut, smooth or custom".into()),
    };
    let world = get("world").map(boolean).transpose()?.unwrap_or(false);
    let falloff = match kind.as_str() {
        "linear" => {
            let mut l = LinearFalloff {
                shape,
                world,
                ..Default::default()
            };
            if let Some(v) = get("start") {
                l.start = vec3(v)?;
            }
            if let Some(v) = get("end") {
                l.end = vec3(v)?;
            }
            l.symmetric = match name("symmetric")?.as_deref() {
                None | Some("none") => FalloffSymmetric::None,
                Some("start") => FalloffSymmetric::Start,
                Some("end") => FalloffSymmetric::End,
                Some(_) => return Err("falloff symmetric is none, start or end".into()),
            };
            Falloff::Linear(l)
        }
        "radial" => {
            let mut r = RadialFalloff {
                shape,
                world,
                ..Default::default()
            };
            if let Some(v) = get("center") {
                r.center = vec3(v)?;
            }
            if let Some(v) = get("size") {
                r.size = vec3(v)?;
            }
            Falloff::Radial(r)
        }
        _ => {
            let mut c = CylinderFalloff {
                shape,
                world,
                ..Default::default()
            };
            if let Some(v) = get("center") {
                c.center = vec3(v)?;
            }
            if let Some(v) = get("size") {
                c.size = vec3(v)?;
            }
            c.axis = match name("axis")?.as_deref() {
                None | Some("y") => 1,
                Some("x") => 0,
                Some("z") => 2,
                Some(_) => return Err("falloff axis is x, y or z".into()),
            };
            Falloff::Cylinder(c)
        }
    };
    falloff.validate()?;
    Ok(falloff)
}

/// Bridge's panel options the bridge doesn't have yet: refused when given
/// (BRQ-03), so a check that needs one fails loudly.
const BRIDGE_NOT_BUILT: [&str; 12] = [
    "orient",
    "connect",
    "inset",
    "aspect",
    "steps",
    "auto_step",
    "continuous",
    "guide_curve",
    "array",
    "close_loop",
    "uvs",
    "uv",
];

/// tools/list's description of `tool_run CurveExtrude`'s options.
fn curve_extrude_schema() -> Value {
    json!({"type": "object", "description": "tool CurveExtrude (one undo step on the selected polygons; options left out take the Curve Extrude button's Reset values): points [[x, y, z], ...] (world metres, in order; required), closed (default false), steps (default 24), kind (curve (default), bezier, b_spline, pen), start_control, end_control, by_length, create_curve_polygon (all default false), bank and twist (radians, default 0), automatic (default false), smoothing_angle (radians, default 5°), align_to_path (default true), start_at_source (default true), cap_start (default false), cap_end (default true), invert (default true), size (last section's scale, default 1), corner_scale (pen, default true).", "properties": {
                            "points": {"type": "array", "items": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}}, "closed": {"type": "boolean"}, "steps": {"type": "integer"},
                            "kind": {"enum": ["curve", "bezier", "b_spline", "pen"]}, "start_control": {"type": "boolean"}, "end_control": {"type": "boolean"},
                            "by_length": {"type": "boolean"}, "create_curve_polygon": {"type": "boolean"}, "bank": {"type": "number"}, "twist": {"type": "number"},
                            "automatic": {"type": "boolean"}, "smoothing_angle": {"type": "number"}, "align_to_path": {"type": "boolean"}, "start_at_source": {"type": "boolean"},
                            "cap_start": {"type": "boolean"}, "cap_end": {"type": "boolean"}, "invert": {"type": "boolean"}, "size": {"type": "number"}, "corner_scale": {"type": "boolean"}
                        }, "additionalProperties": false})
}

/// `tool_run CurveExtrude`: Curve Extrude on the current selection along
/// `points` (world metres, in order), from the Curve Extrude button's Reset
/// values (its tool preset: Start at Source on, Cap Start off, Invert on)
/// with the given options over them; one undo step. Angles in radians.
fn curve_extrude_run(e: &mut Editor, params: Option<&Value>) -> Result<()> {
    use operandi_core::path_tools::PathKind;
    let empty = Map::new();
    let params = match params {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(m)) => m,
        _ => return Err("params must be an object".into()),
    };
    // The options go in params.curve_extrude, as Bridge's go in
    // params.bridge.
    if let Some(other) = params.keys().find(|k| *k != "curve_extrude") {
        return Err(format!(
            "Unknown parameter {other}; Curve Extrude's options go in curve_extrude"
        ));
    }
    let params = match params.get("curve_extrude") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(m)) => m,
        _ => return Err("curve_extrude must be an object".into()),
    };
    e.dispatch(Command::ActivatePreset("CurveExtrude".into()))?;
    let mut p = e
        .live
        .as_ref()
        .map(|l| l.params.clone())
        .ok_or("Curve Extrude did not start a live session")?;
    let t = &mut p.duplicate.curve_extrude;
    let degrees_of = |v: &Value| num(v).map(f64::to_degrees);
    for (key, value) in params {
        match key.as_str() {
            "points" => {
                t.curve.points = value
                    .as_array()
                    .ok_or("points is [[x, y, z], ...]")?
                    .iter()
                    .map(vec3)
                    .collect::<Result<_>>()?;
            }
            "closed" => t.curve.closed = boolean(value)?,
            "steps" => t.steps.number = as_u32(value)?,
            "kind" => {
                t.curve.kind = match value.as_str() {
                    Some("curve") => PathKind::Curve,
                    Some("bezier") => PathKind::Bezier,
                    Some("b_spline") => PathKind::BSpline,
                    Some("pen") => PathKind::Pen,
                    _ => return Err("kind is curve, bezier, b_spline or pen".into()),
                }
            }
            "start_control" => t.curve.start_control = boolean(value)?,
            "end_control" => t.curve.end_control = boolean(value)?,
            "by_length" => t.curve.by_length = boolean(value)?,
            "create_curve_polygon" => t.curve.create_curve_polygon = boolean(value)?,
            "bank" => t.curve.bank = degrees_of(value)?,
            "twist" => t.curve.twist = degrees_of(value)?,
            "automatic" => t.steps.automatic = boolean(value)?,
            "smoothing_angle" => t.steps.smoothing_angle = degrees_of(value)?,
            "align_to_path" => t.steps.align_to_path = boolean(value)?,
            "start_at_source" => t.steps.start_at_source = boolean(value)?,
            "cap_start" => t.sweep.cap_start = boolean(value)?,
            "cap_end" => t.sweep.cap_end = boolean(value)?,
            "invert" => t.sweep.invert = boolean(value)?,
            "size" => t.sweep.size = num(value)?,
            "corner_scale" => t.corner_scale = boolean(value)?,
            _ => return Err(format!("Unknown Curve Extrude parameter {key}")),
        }
    }
    if !t.has_path() {
        return Err(format!(
            "Curve Extrude needs a path: at least {} points",
            t.curve.minimum_points()
        ));
    }
    e.dispatch(Command::Preview(p))?;
    e.dispatch(Command::Drop)
}

/// `tool_run edge.bridge`: Bridge on the current selection (two edge runs
/// in Edge mode, two polygons or clusters in Polygon mode) with `params`
/// over the Reset values, applied and dropped as one undo step (BRQ-80).
/// The remembered Bridge options are left as they were.
fn bridge_run(e: &mut Editor, params: Option<&Value>) -> Result<()> {
    use operandi_core::{BridgeMode, BridgeParameters};
    const KEYS: [&str; 7] = [
        "segments", "mode", "twist", "tension", "remove", "flip", "reverse",
    ];
    let empty = Map::new();
    let params = match params {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(m)) => m,
        _ => return Err("params must be an object".into()),
    };
    crate::commands::check_keys("edge.bridge params", params, &["bridge"])?;
    let o = match params.get("bridge") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(m)) => m,
        _ => return Err("params.bridge must be an object".into()),
    };
    if let Some(k) = o.keys().find(|k| BRIDGE_NOT_BUILT.contains(&k.as_str())) {
        return Err(format!("Bridge's {k} option isn't built yet"));
    }
    crate::commands::check_keys("edge.bridge", o, &KEYS)?;
    let mut p = BridgeParameters::default();
    for (key, v) in o {
        match key.as_str() {
            "segments" => p.segments = as_u32(v)?,
            "twist" => {
                p.twist = v
                    .as_i64()
                    .and_then(|n| i32::try_from(n).ok())
                    .ok_or("twist is a whole number")?
            }
            "tension" => p.tension = num(v)?,
            "remove" => p.remove = boolean(v)?,
            "flip" => p.flip = boolean(v)?,
            "reverse" => p.reverse = boolean(v)?,
            "mode" => {
                p.mode = match v.as_str().map(str::to_ascii_lowercase).as_deref() {
                    Some("linear") => BridgeMode::Linear,
                    Some("curve") => BridgeMode::Curve,
                    Some("smooth") => BridgeMode::Smooth,
                    _ => return Err("Bridge mode is linear, curve or smooth".into()),
                }
            }
            _ => unreachable!("keys are checked"),
        }
    }
    let remembered = *e.presets.get::<operandi_core::BridgePreset>();
    let mark = e.history_mark();
    e.dispatch(Command::ArmBridge)?;
    e.dispatch(Command::ConfigureBridge(p))?;
    e.dispatch(Command::Apply)?;
    e.dispatch(Command::Drop)?;
    *e.presets.get_mut::<operandi_core::BridgePreset>() = remembered;
    // Nothing or one polygon selected applies nothing, silently in the
    // panel (BR18/BR19); the bridge says so.
    // By mark (Astra P9-008): a full history keeps the length the same.
    if !e.recorded_since(mark) {
        return Err("Bridge needs two separate selected polygons or clusters".into());
    }
    Ok(())
}

/// `tool_run`'s `slide` schema (its own function: one json! macro would
/// pass the recursion limit).
fn slide_schema() -> Value {
    json!({"type": "object", "description": "edge.slide / vert.slide options over the Reset values: interpolation (percent, default, or distance; only distance given means distance), percent (fraction of each rail, positive toward the chain's left), distance (metres), stop (Stop Negative Slide), loop (Vertex Slide: stay within the edge, default true), merge (Vertex Slide; Edge Slide always merges, false is refused), preserve_curvature, tension (1 = 100 %), slip_uvs, face_handle. mode radial, reverse false and duplicate false are accepted; the others aren't built and are refused. Edges slide in the order they were picked (select keeps pick order).", "additionalProperties": false, "properties": {
        "interpolation": {"enum": ["percent", "distance"]}, "percent": {"type": "number"}, "distance": {"type": "number"},
        "stop": {"type": "boolean"}, "loop": {"type": "boolean"}, "merge": {"type": "boolean"},
        "preserve_curvature": {"type": "boolean"}, "tension": {"type": "number"}, "slip_uvs": {"type": "boolean"},
        "mode": {"type": "string"}, "reverse": {"type": "boolean"}, "duplicate": {"type": "boolean"}, "face_handle": {"type": "boolean"}
    }})
}

/// `tool_run`'s `slide` options for edge.slide and vert.slide (BRQ-50),
/// over the Slide tool's Reset values (ES-D-01). Options Operandi's Slide
/// doesn't have yet are refused unless given at their default.
fn slide_options(tool: Tool, p: &mut operandi_core::Parameters, v: &Value) -> Result<()> {
    const KEYS: [&str; 13] = [
        "interpolation",
        "percent",
        "distance",
        "stop",
        "loop",
        "merge",
        "preserve_curvature",
        "tension",
        "slip_uvs",
        "mode",
        "reverse",
        "duplicate",
        "face_handle",
    ];
    if !matches!(tool, Tool::EdgeSlide | Tool::VertexSlide) {
        return Err(format!("{} takes no slide options", tool.label()));
    }
    let o = v.as_object().ok_or("slide is an object")?;
    crate::commands::check_keys("slide", o, &KEYS)?;
    let get = |k: &str| o.get(k).filter(|v| !v.is_null());
    let flag = |k: &str| get(k).map(boolean).transpose();
    let reset = operandi_core::SlideExtras::default();
    p.slide_extras = reset;
    p.edge_slide_distance = false;
    p.vertex_slide_merge = true;
    p.vertex_slide_face_handle = true;
    p.shift = 0.;
    let by_distance = match get("interpolation")
        .map(|v| {
            v.as_str()
                .map(str::to_ascii_lowercase)
                .ok_or("slide interpolation is percent or distance")
        })
        .transpose()?
        .as_deref()
    {
        Some("percent") => false,
        Some("distance") => true,
        Some(_) => return Err("slide interpolation is percent or distance".into()),
        // Only a distance given: Distance interpolation.
        None => get("distance").is_some() && get("percent").is_none(),
    };
    p.edge_slide_distance = by_distance;
    let amount = if by_distance { "distance" } else { "percent" };
    if let Some(x) = get(amount) {
        p.shift = num(x)?;
    }
    if let Some(on) = flag("stop")? {
        p.slide_extras.stop_negative = on;
    }
    if let Some(on) = flag("loop")? {
        p.slide_extras.loop_slide = on;
    }
    if let Some(on) = flag("preserve_curvature")? {
        p.slide_extras.preserve_curvature = on;
    }
    if let Some(x) = get("tension") {
        p.slide_extras.tension = num(x)?;
    }
    if let Some(on) = flag("slip_uvs")? {
        p.slide_extras.slip_uvs = on;
    }
    if let Some(on) = flag("face_handle")? {
        p.vertex_slide_face_handle = on;
    }
    match (tool, flag("merge")?) {
        (Tool::VertexSlide, Some(on)) => p.vertex_slide_merge = on,
        // Edge Slide always merges a point that lands on its neighbour
        // (ES-04, Radial mode).
        (_, Some(false)) => {
            return Err(
                "Edge Slide's Merge off only applies when Mode is Linear, which isn't built yet"
                    .into(),
            );
        }
        _ => {}
    }
    if let Some(m) = get("mode")
        && !m.as_str().is_some_and(|m| m.eq_ignore_ascii_case("radial"))
    {
        return Err("Slide Mode is Radial only; Linear isn't built yet".into());
    }
    for key in ["reverse", "duplicate"] {
        if flag(key)? == Some(true) {
            return Err(format!("Slide's {key} option isn't built yet"));
        }
    }
    Ok(())
}

/// Activate a tool, set its parameters and drop it. With `falloffs`, the
/// tool runs under that stack instead of the current falloff, and the
/// falloff from before the call comes back afterwards, so one call never
/// changes the next one's.
fn tool_run(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    crate::commands::check_keys(
        "tool_run",
        args,
        &["tool", "params", "falloffs", "action_center"],
    )?;
    let falloffs = falloff_list(args.get("falloffs"))?;
    let context = action_center(args.get("action_center"))?;
    let before = (e.spatial_falloff, e.added_falloffs.clone());
    let context_before = e.action_context;
    // PF-12: a scripted run starts the tool outright; the app's Click to
    // Start (CS-01) would only arm it and nothing would happen.
    let auto_activate = std::mem::replace(&mut e.auto_activate_flags, false);
    // BRQ-100: the call runs under the given Action Center; the setting
    // from before comes back afterwards, as for falloffs.
    let out = match context {
        Some(c) => e
            .dispatch(Command::ActionContext(c))
            .and_then(|()| tool_run_with(e, args, &falloffs)),
        None => tool_run_with(e, args, &falloffs),
    };
    if !falloffs.is_empty() {
        e.spatial_falloff = before.0;
        e.added_falloffs = before.1;
    }
    e.action_context = context_before;
    e.auto_activate_flags = auto_activate;
    out
}

fn tool_run_with(
    e: &mut Editor,
    args: &Map<String, Value>,
    falloffs: &[(operandi_core::Falloff, operandi_core::MixMode)],
) -> Result<()> {
    let given = req_str(args, "tool")?;
    if given.eq_ignore_ascii_case("CurveExtrude") || given.eq_ignore_ascii_case("curve.extrude") {
        if !falloffs.is_empty() {
            return Err("Curve Extrude has no falloff".into());
        }
        return curve_extrude_run(e, args.get("params"));
    }
    if given.eq_ignore_ascii_case("edge.bridge") || given.eq_ignore_ascii_case("bridge") {
        if !falloffs.is_empty() {
            return Err("Bridge has no falloff".into());
        }
        return bridge_run(e, args.get("params"));
    }
    let star = command_names::star_tool(given, e.scene.selection.mode)?;
    let id = match star.or_else(|| command_names::renamed_tool(given)) {
        Some(new) => {
            if star.is_none() {
                note_deprecated(given, new);
            }
            new
        }
        None => given,
    };
    let tool = TOOLS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(id))
        .map(|&(_, t)| t)
        .ok_or(format!("Unknown tool {id}"))?;
    e.dispatch(Command::Activate(tool))?;
    // BRQ-01 / PF-12: options not given take the tool's Reset values, not
    // what an earlier use left behind (MN-05's Reset Tool Attributes).
    e.dispatch(Command::ResetToolAttributes)?;
    // The given stack replaces the falloff for this call: the first is the
    // falloff, the rest are added on top as the Add Falloff menu adds them
    // (VMF-10, VMF-11).
    if !falloffs.is_empty() {
        e.dispatch(Command::SpatialFalloff(None))?;
        let (first, _) = falloffs[0];
        e.dispatch(Command::SpatialFalloff(Some(first)))?;
        e.dispatch(Command::AddedFalloffs(falloffs[1..].to_vec()))?;
    }
    let mut p = e
        .live
        .as_ref()
        .map(|l| l.params.clone())
        .ok_or("The tool did not start a live session")?;
    let empty = Map::new();
    let params = match args.get("params") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(m)) => m,
        _ => return Err("params must be an object".into()),
    };
    // BRQ-01/10: the slices start from their Reset values every call.
    if !crate::slices::keys(tool).is_empty() {
        let none = Value::Object(Map::new());
        crate::slices::apply(tool, &mut p.slice, params.get("slice").unwrap_or(&none))?;
    }
    // BRQ-50: Edge and Vertex Slide options, over their Reset values.
    if let Some(slide) = params.get("slide") {
        slide_options(tool, &mut p, slide)?;
    }
    for (key, value) in params {
        match key.as_str() {
            "slide" => {}
            "slice" if !crate::slices::keys(tool).is_empty() => {}
            "position" => p.position = vec3(value)?,
            "size" => p.size = vec3(value)?,
            "extent" => p.extent = vec3(value)?,
            // The bridge takes radians; Rotate keeps degrees.
            "delta" if tool == Tool::Rotate => p.delta = degrees(vec3(value)?),
            "delta" => p.delta = vec3(value)?,
            "rotation_delta" => p.rotation_delta = degrees(vec3(value)?),
            "scale_delta" => p.scale_delta = vec3(value)?,
            "inset" => p.inset = num(value)?,
            "step" if tool == Tool::Quantize => p.quantize.step = vec3(value)?,
            "shift" => p.shift = num(value)?,
            "round_level" => p.vertex_round_level = as_u32(value)?,
            "group_polygons" => p.group_polygons = boolean(value)?,
            "make_uvs" => p.make_uvs = boolean(value)?,
            "segments" => {
                let s = value
                    .as_array()
                    .filter(|a| a.len() == 3)
                    .ok_or("segments is [x, y, z]")?;
                p.segments = [as_u32(&s[0])?, as_u32(&s[1])?, as_u32(&s[2])?];
            }
            "radius" | "sides" | "radial_segments" => {
                let r = p.radial.as_mut().ok_or(format!("{id} has no {key}"))?;
                match key.as_str() {
                    "radius" => r.radius = vec3(value)?,
                    "sides" => r.sides = as_u32(value)?,
                    _ => r.segments = as_u32(value)?,
                }
            }
            "chamfer" => {
                for (k, v) in value.as_object().ok_or("chamfer is an object")? {
                    match k.as_str() {
                        "offset" => p.chamfer.offset = num(v)?,
                        "segments" => p.chamfer.segments = as_u32(v)?,
                        // 0-1 here; the tool keeps percent.
                        "depth" => p.chamfer.depth = num(v)? * 100.,
                        _ => return Err(format!("Unknown chamfer parameter {k}")),
                    }
                }
            }
            "smooth_shift" => {
                for (k, v) in value.as_object().ok_or("smooth_shift is an object")? {
                    match k.as_str() {
                        "offset" => p.smooth_shift.offset = num(v)?,
                        "scale" => p.smooth_shift.scale = num(v)?,
                        // Radians here; the tool keeps degrees.
                        "angle" => p.smooth_shift.angle = num(v)?.to_degrees(),
                        "thicken" => p.smooth_shift.thicken = boolean(v)?,
                        "sharp" => p.smooth_shift.sharp = boolean(v)?,
                        "angle_weight" => p.smooth_shift.angle_weight = boolean(v)?,
                        "both_sides" => p.smooth_shift.both_sides = boolean(v)?,
                        _ => return Err(format!("Unknown smooth_shift parameter {k}")),
                    }
                }
            }
            "tweak" => {
                let m = value.as_object().filter(|m| m.len() == 1).ok_or(
                    "tweak is one of {\"vertex\": n}, {\"edge\": [a, b]} or {\"polygon\": n}",
                )?;
                let (kind, v) = m.iter().next().expect("one entry");
                let element = match kind.as_str() {
                    "vertex" => operandi_core::TweakElement::Vertex(as_u32(v)?),
                    "polygon" => operandi_core::TweakElement::Polygon(as_u32(v)?),
                    "edge" => {
                        let ends = v
                            .as_array()
                            .filter(|a| a.len() == 2)
                            .ok_or("tweak edge is [a, b]")?;
                        operandi_core::TweakElement::Edge(as_u32(&ends[0])?, as_u32(&ends[1])?)
                    }
                    _ => return Err(format!("Unknown tweak element {kind}")),
                };
                p.tweak = Some(operandi_core::TweakGrab {
                    item: e.scene.selection.item.ok_or("No active item")?,
                    element,
                });
            }
            "tweak_options" => {
                let o = &mut p.tweak_options;
                for (k, v) in value.as_object().ok_or("tweak_options is an object")? {
                    let text = || v.as_str().ok_or(format!("tweak_options.{k} is a name"));
                    match k.as_str() {
                        "range" => o.range = num(v)?,
                        "mode" => {
                            let name = text()?;
                            o.mode = operandi_core::TweakMode::ALL
                                .into_iter()
                                .find(|m| m.label().eq_ignore_ascii_case(name))
                                .ok_or(format!("Unknown Tweak mode {name}"))?;
                        }
                        "connect" => {
                            let name = text()?;
                            o.connect = operandi_core::TweakConnect::ALL
                                .into_iter()
                                .find(|c| c.label().eq_ignore_ascii_case(name))
                                .ok_or(format!("Unknown Connected Elements {name}"))?;
                        }
                        "shape" => {
                            let name = text()?;
                            o.shape = [
                                operandi_core::FalloffShape::Linear,
                                operandi_core::FalloffShape::EaseIn,
                                operandi_core::FalloffShape::EaseOut,
                                operandi_core::FalloffShape::Smooth,
                            ]
                            .into_iter()
                            .find(|s| s.label().eq_ignore_ascii_case(name))
                            .ok_or(format!("Unknown Tweak shape {name}"))?;
                        }
                        _ => return Err(format!("Unknown tweak_options parameter {k}")),
                    }
                }
            }
            _ => return Err(format!("Unknown parameter {key}")),
        }
    }
    // Bevel and Extrude at zero distance build their (flat) polygons only
    // when applied, as the panel's Apply button does (PB-O-B6, PE-O-01): a
    // zero preview is the untouched tool and changes nothing.
    let zero_apply = match tool {
        Tool::Extrude => p.extent == DVec3::ZERO,
        Tool::Bevel => p.shift == 0. && p.inset == 0.,
        // PF-12: Edge Extend at its Reset values adds its zero-width row.
        Tool::EdgeExtend => {
            p.delta == DVec3::ZERO
                && p.inset == 0.
                && p.shift == 0.
                && p.rotation_delta == DVec3::ZERO
                && p.scale_delta == DVec3::ONE
        }
        _ => false,
    };
    if zero_apply {
        e.live
            .as_mut()
            .ok_or("The tool did not start a live session")?
            .params = p;
        e.dispatch(Command::Apply)?;
        return e.dispatch(Command::Drop);
    }
    let configure = if tool.is_smooth_shift() {
        Command::ConfigureSmoothShift(p.smooth_shift)
    } else {
        Command::Preview(p)
    };
    e.dispatch(configure)?;
    // Primitives and Smooth Shift/Thicken commit a numeric stage with Apply;
    // the other tools commit their live preview on Drop.
    if tool.is_primitive() || tool.is_smooth_shift() {
        e.dispatch(Command::Apply)?;
    }
    e.dispatch(Command::Drop)
}

/// Radians from the bridge to the degrees the tools keep.
fn degrees(v: DVec3) -> DVec3 {
    DVec3::new(v.x.to_degrees(), v.y.to_degrees(), v.z.to_degrees())
}
fn v3(v: DVec3) -> Value {
    json!([v.x, v.y, v.z])
}
