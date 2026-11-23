//! The command registry: one table of every command Operandi accepts
//! (extensibility plan, phase 0; UI conventions bible UIC-160..164).
//!
//! Every line the command line, a menu, a pie entry, a saved key binding
//! or a script sends goes through `execute` in the app. This table is the
//! authoritative list of the names `execute` understands, with their
//! arguments, units and flags. Dispatch still flows through the app's
//! parsers for now (stage 1); later stages move parsing here and have the
//! MCP bridge read this table instead of keeping its own.
//!
//! Two kinds of entry:
//! - [`Kind::Script`]: dotted script names from the reference name map
//!   (`poly.flip`, `tool.set`). Old spellings resolve through
//!   [`command_names`](crate::command_names) and are reported as
//!   deprecated (plan L5).
//! - [`Kind::Action`]: the plain phrases the Input Map, menus and pies use
//!   (`Grow`, `Top View`). Matched case-insensitively, like the parser.
//!
//! Arguments use the map's units: distances in metres, percents as 0–1
//! (or `50%`), angles in radians (or `30deg`). Panels convert for display.
use crate::command_names;

/// The scripting surface's version. Bumped when a command is removed or an
/// argument changes meaning; adding commands or optional arguments doesn't.
pub const API_VERSION: u32 = 1;

/// Script command or Input Map action phrase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Script,
    Action,
}

/// What a command is about, for `command.list <area>` and reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Area {
    Modeling,
    Selection,
    Tools,
    Sculpt,
    View,
    ViewportOptions,
    Items,
    Shading,
    Presets,
    Files,
    Render,
    Preferences,
    Help,
    Debug,
}

impl Area {
    pub const ALL: [Area; 14] = [
        Area::Modeling,
        Area::Selection,
        Area::Tools,
        Area::Sculpt,
        Area::View,
        Area::ViewportOptions,
        Area::Items,
        Area::Shading,
        Area::Presets,
        Area::Files,
        Area::Render,
        Area::Preferences,
        Area::Help,
        Area::Debug,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Area::Modeling => "modeling",
            Area::Selection => "selection",
            Area::Tools => "tools",
            Area::Sculpt => "sculpt",
            Area::View => "view",
            Area::ViewportOptions => "viewport",
            Area::Items => "items",
            Area::Shading => "shading",
            Area::Presets => "presets",
            Area::Files => "files",
            Area::Render => "render",
            Area::Preferences => "preferences",
            Area::Help => "help",
            Area::Debug => "debug",
        }
    }
}

/// An argument's value type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgType {
    Bool,
    Int,
    Float,
    Text,
    /// One of these names (or its index).
    Choice(&'static [&'static str]),
    Path,
    Json,
}

/// The unit a number is given in (the map's script units).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    None,
    /// Metres, or any length unit typed with it (`50cm`).
    Distance,
    /// 0–1, or a value with `%`.
    Percent,
    /// Radians, or degrees typed with a unit (`30deg`).
    Angle,
    /// A plain multiplier (1 = unchanged).
    Factor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arg {
    pub name: &'static str,
    pub ty: ArgType,
    pub unit: Unit,
    /// Shown in help; `None` when the command has no single default.
    pub default: Option<&'static str>,
    pub optional: bool,
}

/// A required argument.
pub const fn arg(name: &'static str, ty: ArgType, unit: Unit) -> Arg {
    Arg {
        name,
        ty,
        unit,
        default: None,
        optional: false,
    }
}

/// An optional argument and the value used when it's left out.
pub const fn opt(
    name: &'static str,
    ty: ArgType,
    unit: Unit,
    default: Option<&'static str>,
) -> Arg {
    Arg {
        name,
        ty,
        unit,
        default,
        optional: true,
    }
}

const BOOL: ArgType = ArgType::Bool;
const INT: ArgType = ArgType::Int;
const FLOAT: ArgType = ArgType::Float;
const TEXT: ArgType = ArgType::Text;
const N: Unit = Unit::None;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    /// Changes the scene as one undo step when it succeeds.
    pub undoable: bool,
    /// Answers `?` (or reports a value) instead of changing anything.
    pub query: bool,
    /// Only meaningful with the app's window: opens a dialog, panel or
    /// window, or changes how a view looks. Scripts without a window can't
    /// use it.
    pub ui_only: bool,
    /// Starts an interactive tool; its edit is one undo step when dropped.
    pub tool: bool,
    /// Runs with no window through `command_exec` (the MCP bridge's
    /// `command_run` and typed lines alike); its JSON arguments and
    /// handler are the entry's `json`.
    pub headless: bool,
    /// An Operandi name with no match in the reference name map (UIC-165):
    /// kept, but marked.
    pub operandi: bool,
    /// Typed at the keyboard, joins a live Move/Rotate/Scale/Transform in
    /// Items mode instead of being refused by it (PD-Q12).
    pub allowed_during_transform_tool: bool,
    /// Acts outside the scene and can't be undone: save, export, quit,
    /// clearing a log (CMD-01 class "side effect").
    pub side_effect: bool,
    /// Plumbing that shows only in the History tab (CMD-01 class "quiet").
    pub quiet: bool,
}

/// One JSON argument of a headless command: what the MCP bridge's
/// `command_run` and a typed line's words become (`command_exec`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsonArg {
    pub name: &'static str,
    /// JSON type, e.g. `integer`, `number`, `[x, y, z]`, `string`.
    pub kind: &'static str,
    pub required: bool,
    /// Meaning, unit and default.
    pub about: &'static str,
}

pub const fn jopt(name: &'static str, kind: &'static str, about: &'static str) -> JsonArg {
    JsonArg {
        name,
        kind,
        required: false,
        about,
    }
}
pub const fn jreq(name: &'static str, kind: &'static str, about: &'static str) -> JsonArg {
    JsonArg {
        name,
        kind,
        required: true,
        about,
    }
}

/// Item id argument that defaults to the active item.
const JSON_ITEM: JsonArg = jopt("item", "integer", "Item id; default the active item.");

/// Runs a headless command on the editor alone: the command's current
/// name and its checked JSON arguments. `Ok(Some)` is the one `Command`
/// for the caller to dispatch; `Ok(None)` means it already ran.
pub type EditorHandler = fn(
    &mut crate::Editor,
    &str,
    &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<crate::Command>, String>;

/// What a command that runs without a window takes and what runs it
/// (extensibility plan stage 2, L6).
#[derive(Clone, Copy, Debug)]
pub struct Headless {
    /// The script description (MCP `command_help`, docs/MCP_BRIDGE.md).
    pub about: &'static str,
    pub args: &'static [JsonArg],
    pub run: EditorHandler,
}

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub name: &'static str,
    /// Other names the parser takes for this command (same behaviour).
    pub aliases: &'static [&'static str],
    pub kind: Kind,
    pub area: Area,
    pub args: &'static [Arg],
    pub flags: Flags,
    /// One line of help.
    pub help: &'static str,
    /// A line that runs it (tests run every example through the parser).
    pub example: &'static str,
    /// The phrase the app's parser takes for this Modo-named command
    /// (`app.undo` runs as `Undo`): see [`parser_spelling`].
    pub spelled: Option<&'static str>,
    /// For a phrase: the Modo invocation of the same button (UIC-165,
    /// specifications/command-names/MODO-BUTTON-INVOCATIONS.txt), e.g.
    /// `tool.set prim.cube on` for `Cube`. Runs through the parser too.
    pub modo: Option<&'static str>,
    /// Set for a command that runs without a window: its JSON arguments
    /// and handler. `flags.headless` says the same.
    pub json: Option<Headless>,
}

const fn entry(kind: Kind, name: &'static str, area: Area, help: &'static str) -> Entry {
    Entry {
        name,
        aliases: &[],
        kind,
        area,
        args: &[],
        flags: Flags {
            undoable: false,
            query: false,
            ui_only: false,
            tool: false,
            headless: false,
            operandi: false,
            allowed_during_transform_tool: false,
            side_effect: false,
            quiet: false,
        },
        spelled: None,
        modo: None,
        json: None,
        help,
        example: name,
    }
}

/// A dotted script command.
pub const fn script(name: &'static str, area: Area, help: &'static str) -> Entry {
    entry(Kind::Script, name, area, help)
}

/// An Input Map action or typed phrase.
pub const fn action(name: &'static str, area: Area, help: &'static str) -> Entry {
    entry(Kind::Action, name, area, help)
}

impl Entry {
    pub const fn aliases(mut self, aliases: &'static [&'static str]) -> Self {
        self.aliases = aliases;
        self
    }
    pub const fn args(mut self, args: &'static [Arg]) -> Self {
        self.args = args;
        self
    }
    pub const fn example(mut self, example: &'static str) -> Self {
        self.example = example;
        self
    }
    pub const fn undo(mut self) -> Self {
        self.flags.undoable = true;
        self
    }
    pub const fn query(mut self) -> Self {
        self.flags.query = true;
        self
    }
    pub const fn ui(mut self) -> Self {
        self.flags.ui_only = true;
        self
    }
    pub const fn tool(mut self) -> Self {
        self.flags.tool = true;
        self
    }
    /// Class "side effect" (CMD-01): outside the scene, not undoable.
    pub const fn side_effect(mut self) -> Self {
        self.flags.side_effect = true;
        self
    }
    /// Class "quiet" (CMD-01): only in the History tab.
    pub const fn quiet(mut self) -> Self {
        self.flags.quiet = true;
        self
    }
    /// The command's class (CMD-01..03, `command_record::class`).
    pub fn class(&self) -> Option<crate::command_record::CommandClass> {
        crate::command_record::class(self)
    }
    pub const fn during_transform_tool(mut self) -> Self {
        self.flags.allowed_during_transform_tool = true;
        self
    }
    pub const fn operandi(mut self) -> Self {
        self.flags.operandi = true;
        self
    }
    /// A Modo-named command the app parser knows by `phrase`, which also
    /// becomes an alias (UIC-165: the phrase is never the only name).
    pub const fn spelled(mut self, phrase: &'static str) -> Self {
        self.spelled = Some(phrase);
        self
    }
    pub const fn modo(mut self, invocation: &'static str) -> Self {
        self.modo = Some(invocation);
        self
    }
    /// Runs on the editor alone (no window) with these JSON arguments and
    /// this handler: a kit's headless command.
    pub const fn on_editor(
        mut self,
        about: &'static str,
        args: &'static [JsonArg],
        run: EditorHandler,
    ) -> Self {
        self.flags.headless = true;
        self.json = Some(Headless { about, args, run });
        self
    }
    /// Runs without a window through `command_exec`, with these JSON
    /// arguments and the built-in handler.
    const fn headless(mut self, about: &'static str, args: &'static [JsonArg]) -> Self {
        self.flags.headless = true;
        self.json = Some(Headless {
            about,
            args,
            run: crate::command_exec::built_in,
        });
        self
    }

    /// Every name this entry answers to.
    pub fn names(&self) -> impl Iterator<Item = &'static str> {
        std::iter::once(self.name)
            .chain(self.spelled)
            .chain(self.aliases.iter().copied())
    }

    /// `name arg:type ...` with optional arguments in brackets.
    pub fn usage(&self) -> String {
        let mut line = self.name.to_owned();
        if self.args.is_empty()
            && let Some(spec) = crate::command_exec::spec(self.name)
        {
            for a in spec.args {
                let one = format!("{}:{}", a.name, a.kind.replace(' ', ""));
                line.push(' ');
                line.push_str(&if a.required { one } else { format!("[{one}]") });
            }
            return line;
        }
        for a in self.args {
            let ty = match a.ty {
                ArgType::Bool => "bool".to_owned(),
                ArgType::Int => "int".to_owned(),
                ArgType::Float => match a.unit {
                    Unit::Distance => "distance".to_owned(),
                    Unit::Percent => "percent".to_owned(),
                    Unit::Angle => "angle".to_owned(),
                    Unit::Factor => "factor".to_owned(),
                    Unit::None => "number".to_owned(),
                },
                ArgType::Text => "text".to_owned(),
                ArgType::Choice(c) => c.join("|"),
                ArgType::Path => "path".to_owned(),
                ArgType::Json => "json".to_owned(),
            };
            let mut one = format!("{}:{ty}", a.name);
            if let Some(d) = a.default {
                one.push_str(&format!("={d}"));
            }
            if a.optional {
                one = format!("[{one}]");
            }
            line.push(' ');
            line.push_str(&one);
        }
        line
    }

    /// The usage line, flags and help, for `help <name>`.
    pub fn describe(&self) -> String {
        let mut flags = Vec::new();
        if self.flags.undoable {
            flags.push("undoable");
        }
        if self.flags.tool {
            flags.push("starts a tool");
        }
        if self.flags.query {
            flags.push("query");
        }
        if self.flags.ui_only {
            flags.push("needs the window");
        }
        if self.flags.headless {
            flags.push("runs without a window");
        }
        if self.flags.operandi {
            flags.push("Operandi-only name");
        }
        if let Some(class) = self.class() {
            flags.push(class.label());
        }
        let mut text = format!("Help: {}", self.usage());
        if !self.aliases.is_empty() {
            text.push_str(&format!(" (also {})", self.aliases.join(", ")));
        }
        text.push_str(&format!(" — {}", self.help));
        if !flags.is_empty() {
            text.push_str(&format!(" [{}]", flags.join(", ")));
        }
        if let Some(modo) = self.modo {
            text.push_str(&format!(" Script form: {modo}."));
        }
        text
    }
}

// ------------------------------------------------------------------ tables

/// `tool.set` ids (the reference name map's tool names). Old ids resolve
/// through `command_names::TOOL_RENAMES`; `*.Slide` picks by mode.
pub const TOOL_IDS: &[&str] = &[
    "prim.pen",
    "prim.cube",
    "prim.sphere",
    "prim.cylinder",
    "prim.ellipsoid",
    "prim.cone",
    "prim.toroid",
    "prim.capsule",
    "prim.arc",
    "prim.tube",
    "solid.sketch",
    "prim.makeVertex",
    "poly.bevel",
    "poly.twistBevel",
    "poly.extrude",
    "poly.smshift",
    "Thicken",
    "poly.spikey",
    "poly.haul",
    "poly.sweep",
    "poly.reduct",
    "poly.inset",
    "poly.shift",
    "poly.loopSlice",
    "poly.knife",
    "poly.julienne",
    "chamfer.edit",
    "Transform",
    "TransformMove",
    "TransformRotate",
    "TransformScale",
    "edge.bridge",
    "edge.extend",
    "edge.extrude",
    "edge.slide",
    "edge.crease",
    "edge.bevel",
    "edge.chamfer",
    "edge.relax",
    "edge.knife",
    "edge.addLoop",
    "edge.addPoint",
    "vert.bevel",
    "vert.extrude",
    "vert.merge",
    "vert.tear",
    "vert.slide",
    "*.Slide",
    "DragWeld",
    "ElementMove",
    "xfrm.push",
    "xfrm.smooth",
    "xfrm.jitter",
    "xfrm.quantize",
    "xfrm.radialAlign",
    "xfrm.linearAlign",
    "xfrm.bend",
    "operandi.symmetry.tool",
    "Curve Slice",
    "PenKnife",
    "Primitive Slice",
    // Topology tab Contour (missing-tools-g4): Operandi's own id.
    "Contour",
    "*.mirror",
    "Radial Sweep",
    "*.clone",
    "*.array",
    "*.Radial Array",
    "curve.extrude",
    "curve.clone",
    "scatter.clone",
    "operandi.symmetrize.tool",
    "operandi.uv.project",
    "operandi.uv.viewProject",
    "uv.unwrap",
    "uv.relax",
    "uv.pelt",
    "operandi.uv.peeler",
    "operandi.uv.boxTransform",
    "sculpt.smooth",
    "sculpt.push",
    "sculpt.inflate",
    "sculpt.move",
    "sculpt.fold",
    "sculpt.clay",
    "sculpt.smudge",
    "sculpt.carve",
    "sculpt.scar",
    "sculpt.flatten",
    "sculpt.tangentPinch",
    "sculpt.centerPinch",
    "sculpt.emboss",
    "sculpt.shrink",
    "sculpt.spin",
    "sculpt.vmapPaint",
    "sculpt.vmapSmooth",
    "sculpt.vmapScale",
    "sculpt.vmapErase",
    "sculpt.vmapSetVal",
    "falloff.vertexMap",
    "falloff.invert",
    "falloff.selection",
    "falloff.softSelection",
    "falloff.element",
    "falloff.coplanar",
    "falloff.screen",
    "falloff.lasso",
    "falloff.airbrush",
    "falloff.noise",
    "falloff.image",
    "falloff.path",
    "falloff.curvature",
];

/// Modo `tool.set` ids of tools Operandi has under a phrase (UIC-165,
/// MODO-BUTTON-INVOCATIONS.txt): `tool.set util.ruler on` runs `Ruler`.
///
/// Kept in sorted order (case-insensitive), so parallel branches add
/// lines in different places (AGENTS.md, "Code"); a test checks it.
pub const TOOL_PHRASES: &[(&str, &str)] = &[
    ("AxisRotate", "Axis Rotate"),
    ("Bend", "Bend"),
    ("BezierClone", "Bezier Clone"),
    ("BezierExtrude", "Bezier Extrude"),
    ("BezierInstance", "Bezier Instance"),
    ("BezierReplica", "Bezier Replica"),
    ("BezierTransform", "Bezier Transform"),
    ("BSplineClone", "B-Spline Clone"),
    ("BSplineExtrude", "B-Spline Extrude"),
    ("BSplineInstance", "B-Spline Instance"),
    ("BSplineReplica", "B-Spline Replica"),
    ("BSplineTransform", "B-Spline Transform"),
    ("const.bg", "Constrain to Background"),
    ("CurveClone", "Curve Clone"),
    ("CurveExtrude", "Curve Extrude"),
    ("CurveInstance", "Curve Instance"),
    ("CurveReplica", "Curve Replica"),
    ("CurveTransform", "Curve Transform"),
    ("ElementPush", "Element Push"),
    ("Flare", "Flare"),
    ("Flex", "Flex"),
    ("GroundAlign", "Ground Align"),
    ("InstanceArray", "Instance Array"),
    ("InstanceClone", "Instance Clone"),
    ("InstanceMirror", "Instance Mirror"),
    ("InstanceRadialArray", "Instance Radial Array"),
    ("InstanceScatter", "Instance Scatter"),
    ("mesh.topology", "Topology Pen"),
    ("paste.tool", "Paste Tool"),
    ("patch.draw", "Draw Patch"),
    ("PenClone", "Pen Clone"),
    ("PenExtrude", "Pen Extrude"),
    ("PenInstance", "Pen Instance"),
    ("PenReplicate", "Pen Replicate"),
    ("PenTransform", "Transform Pen"),
    ("prim.bezier", "Bezier Curve"),
    ("prim.bspline", "B-Spline"),
    ("prim.curve", "Curve"),
    ("prim.sketch", "Sketch"),
    ("prim.text", "Text"),
    ("QuickAlign", "Quick Align"),
    ("ReplicaArray", "Replica Array"),
    ("ReplicaClone", "Replica Clone"),
    ("ReplicaMirror", "Replica Mirror"),
    ("ReplicaRadialArray", "Replica Radial Array"),
    ("ReplicaScatter", "Replica Scatter"),
    ("seam.marker", "Conditional Loop Tool"),
    ("select.polygonCoplanar", "Coplanar Polygons Tool"),
    ("SoftSelectionMove", "Soft Selection Move"),
    ("SoftSelectionRotate", "Soft Selection Rotate"),
    ("SoftSelectionScale", "Soft Selection Scale"),
    ("SoftSelectionTransform", "Soft Selection Transform"),
    ("Swirl", "Swirl"),
    ("TransformArray", "Transform Array"),
    ("TransformRadialArray", "Transform Radial Array"),
    ("TransformScatter", "Transform Scatter"),
    ("util.bounds", "Dimensions"),
    ("util.protractor", "Protractor"),
    ("util.ruler", "Ruler"),
    ("xfrm.jitter", "Jitter"),
    ("xfrm.linearAlign", "Linear Align"),
    ("xfrm.magnet", "Soft Move"),
    ("xfrm.pole", "Bulge"),
    ("xfrm.quantize", "Quantize"),
    ("xfrm.radialAlign", "Radial Align"),
    ("xfrm.shear", "Shear"),
    ("xfrm.softDrag", "Soft Drag"),
    ("xfrm.taper", "Taper"),
    ("xfrm.twist", "Twist"),
    ("xfrm.vortex", "Vortex"),
];

/// The phrase that starts the tool Modo calls `id`, if Operandi has it.
pub fn tool_phrase(id: &str) -> Option<&'static str> {
    TOOL_PHRASES
        .iter()
        .find(|(m, _)| m.eq_ignore_ascii_case(id))
        .map(|(_, p)| *p)
}

/// scene.saveAs export formats (Operandi's own values: the map names no
/// format strings).
pub const EXPORT_FORMATS: ArgType = ArgType::Choice(&[
    "obj", "stl", "stlAscii", "gltf", "glb", "geo", "svg", "lxo", "ply", "plyAscii", "abc", "usd",
    "usda", "usdc", "usdz", "fbx", "3mf",
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
pub static COMMANDS: &[Entry] = &[
    // ---------------------------------------------------------- help
    script(
        "cmds.showHelp",
        Help,
        "Lists help topics, or shows one command's arguments and help.",
    )
    .spelled("help")
    .args(&[opt("command", TEXT, N, None)])
    .query()
    .example("cmds.showHelp poly.flip"),
    script(
        "command.list",
        Help,
        "Lists every command (optionally those whose name or area contains a word) in the log.",
    )
        .operandi()
    .args(&[opt("filter", TEXT, N, None)])
    .query(),
    // ---------------------------------------------------------- history
    // Command History (HIS-11, HIS-22, HIS-50, CMD-40). Argument names
    // are provisional (CH-Q03).
    script("cmds.batch", Help, "Runs up to 9 command lines (c1:..c9:) as one undo step named label:, its lines kept as sub-commands.")
        .args(&[
            opt("label", TEXT, N, Some("Batch")),
            opt("c1", TEXT, N, None),
            opt("c2", TEXT, N, None),
            opt("c3", TEXT, N, None),
            opt("c4", TEXT, N, None),
            opt("c5", TEXT, N, None),
            opt("c6", TEXT, N, None),
            opt("c7", TEXT, N, None),
            opt("c8", TEXT, N, None),
            opt("c9", TEXT, N, None),
        ])
        .undo()
        .example("cmds.batch label:Two c1:poly.flip c2:poly.flip"),
    script("cmds.undoTo", Help, "Undoes or redoes until index: steps are done (0 undoes everything the undo list holds); the Undos tab's drag row runs it.")
        .args(&[arg("index", INT, N)])
        .quiet()
        .example("cmds.undoTo 0"),
    script("history.clear", Help, "Empties the Command History's History tab (results:true empties the Results tab instead).")
        .args(&[opt("results", BOOL, N, Some("false"))])
        .side_effect(),
    script("history.max", Help, "Sets or asks (?) how many lines the History tab keeps; the oldest go first.")
        .args(&[arg("count", TEXT, N)])
        .ui()
        .query()
        .example("history.max 1000"),
    script("history.resultsMax", Help, "Sets or asks (?) how many lines the Results tab keeps.")
        .args(&[arg("count", TEXT, N)])
        .ui()
        .query()
        .example("history.resultsMax 200"),
    // ---------------------------------------------------------- tools
    script("tool.set", Tools, "Starts (on), stops (off) or queries (?) a tool by id.")
        .args(&[
            arg("preset", ArgType::Choice(TOOL_IDS), N),
            arg("mode", ON_OFF_QUERY, N),
        ])
        .tool()
        .query()
        .example("tool.set prim.cube ?"),
    script("tool.attr", Tools, "Sets or queries (?) one setting of the live tool (Spike, Smooth Shift, Bridge, Edge Bevel/Chamfer, Polygon Reduction).")
        .args(&[arg("tool", TEXT, N), arg("attr", TEXT, N), arg("value", TEXT, N)])
        .query()
        .example("tool.attr poly.spikey factor ?").undo(),
    script("tool.reset", Tools, "Resets the active tool's settings.")
        .args(&[opt("tool", TEXT, N, None)]).undo(),
    // MN-05: Edit > Reset Tool Attributes and its key (SC-083).
    action("Reset Tool Attributes", Tools, "Puts the active tool's values back to the factory ones; the tool stays active.")
        .modo("tool.reset").undo(),
    script("tool.apply", Tools, "Applies the active tool and keeps it active.")
        .aliases(&["tool.doApply"])
        .undo(),
    script("tool.drop", Tools, "Drops the active tool, keeping its edit.").undo(),
    // MP-01: opens a form as a pop-over; Space runs it for Mini Properties.
    script("attr.formPopover", View, "Opens a form as a pop-over at the pointer (form:MiniProps is Mini Properties); again closes it.")
        .args(&[
            arg("form", TEXT, N),
            opt("proficiency", TEXT, N, None),
            opt("pin", BOOL, N, Some("true")),
            opt("lockProficiency", BOOL, N, Some("false")),
            opt("focusFirstField", BOOL, N, Some("false")),
            opt("open", BOOL, N, None),
        ])
        .example("attr.formPopover form:MiniProps proficiency:mini lockProficiency:true pin:true")
        .ui(),
    // ---------------------------------------------------------- polygon
    script("operandi.pen.create", Modeling, "Creates Pen geometry in one step from JSON positions.")
        .operandi()
        .aliases(&["pen"])
        .args(&[arg("json", ArgType::Json, N)])
        .undo()
        .example("operandi.pen.create {}"),
    script("poly.setPart", Modeling, "Assigns a part name to the selected polygons; bare opens the dialog.")
        .headless(
            "Set the Part tag of the selected polygons.",
            &[jreq("name", "string", "Part name.")],
        )
        .args(&[opt("name", TEXT, N, None)])
        .undo()
        .example("poly.setPart Part"),
    // UV names from the registry thread (2026-11-11). uv.unwrap is a tool
    // in the reference (`tool.set uv.unwrap on`); until Operandi has a UV
    // viewport it runs one-shot with the tool's attributes (UV-Q20).
    script("uv.unwrap", Modeling, "UV Unwrap of the visible polygons into the UV map (one-shot for now); unnamed attributes keep the last values.")
        .headless(
            "UV Unwrap of the visible polygons into the UV map (one-shot for now); unnamed attributes keep the last values.",
            &[
            jopt(
                "iter",
                "integer",
                "0 keeps the Initial Projection (default 100).",
            ),
            jopt("gaps", "number", "Relative Gap Size, 0-1 (default 0.2)."),
            jopt("matr", "boolean", "Segment by Material."),
            jopt("part", "boolean", "Segment by Part."),
            jopt(
                "project",
                "integer|string",
                "0 planar, 1 cylindrical, 2 spherical, 3 current, 4 groupNormal.",
            ),
            jopt("pinning", "integer|string", "0 u, 1 v."),
            jopt("axis", "integer|string", "Projection Axis: 0 x, 1 y, 2 z."),
            jopt("angle", "number", "Offset Angle in radians."),
            jopt("seal", "boolean", "Seal Holes."),
            jopt("layout", "boolean", "Update Layout."),
            jopt("mode", "integer|string", "0 conformal, 1 angleBased."),
            jopt(
                "symmetrize",
                "boolean",
                "Symmetrize (not built yet; must be false).",
            ),
            jopt("symmAxis", "integer|string", "0 x, 1 y, 2 z."),
            jopt("symmComp", "integer|string", "Mirror Axis: 0 u, 1 v."),
            jopt(
                "name",
                "string",
                "UV map; made when missing (default Texture).",
            ),
            jopt(
                "seam",
                "integer|string",
                "0 selectedEdges, 1 seamMap, 2 selectionSet.",
            ),
            jopt("seamName", "string", "Seam map name."),
            jopt("pickName", "string", "Edge selection set name."),
        ],
        )
        .args(&[
            opt("iter", INT, N, Some("100")),
            opt("gaps", FLOAT, N, Some("0.2")),
            opt("matr", BOOL, N, Some("false")),
            opt("part", BOOL, N, Some("false")),
            opt("project", ArgType::Choice(&["planar", "cylindrical", "spherical", "current", "groupNormal"]), N, Some("planar")),
            opt("pinning", ArgType::Choice(&["u", "v"]), N, Some("u")),
            opt("axis", ArgType::Choice(&["x", "y", "z"]), N, Some("y")),
            opt("angle", FLOAT, N, Some("0")),
            opt("seal", BOOL, N, Some("true")),
            opt("layout", BOOL, N, Some("true")),
            opt("mode", ArgType::Choice(&["conformal", "angleBased"]), N, Some("conformal")),
            opt("symmetrize", BOOL, N, Some("false")),
            opt("symmAxis", ArgType::Choice(&["x", "y", "z"]), N, Some("x")),
            opt("symmComp", ArgType::Choice(&["u", "v"]), N, Some("u")),
            opt("name", TEXT, N, Some("Texture")),
            opt("seam", ArgType::Choice(&["selectedEdges", "seamMap", "selectionSet"]), N, Some("selectedEdges")),
            opt("seamName", TEXT, N, None),
            opt("pickName", TEXT, N, None),
        ])
        .undo()
        .example("uv.unwrap mode:angleBased"),
    // BRQ-110: UV Relax's command form, the tool's Tool Properties as
    // arguments over its Reset values (UVS-1 §3; names from the
    // reference's uv.relax tool attributes).
    script("uv.relax", Modeling, "UV Relax of the UV map on the visible polygons, one undo step; unnamed attributes take the tool's Reset values.")
        .headless(
            "UV Relax of the active mesh's UV map on its visible polygons, one undo step. A polygon selection doesn't limit it; with noEdge false, islands holding a selected edge stay still. Unnamed attributes take the tool's Reset values; iter 0 (the Reset value) changes nothing.",
            &[
            jopt("mode", "integer|string", "0 spring, 1 conformal (default), 2 angleBased, 3 adaptive."),
            jopt("iter", "integer", "Iterations, the amount of relaxing (default 0: no change)."),
            jopt("seal", "boolean", "Seal Holes (Conformal and Angle Based only; default false)."),
            jopt("boundary", "integer|string", "0 smooth (default), 1 locked, 2 maintain, 3 straighten; not used in Spring mode."),
            jopt("area", "number", "Area Weight, 0-1 (Adaptive only; default 0.1)."),
            jopt("uniform", "boolean", "Uniform Spans (Straighten only; default false)."),
            jopt("noEdge", "boolean", "Ignore Edge Selection (default true); false holds islands with a selected edge still."),
            jopt("name", "string", "UV map (default: the selected one)."),
        ],
        )
        .args(&[
            opt("mode", ArgType::Choice(&["spring", "conformal", "angleBased", "adaptive"]), N, Some("conformal")),
            opt("iter", INT, N, Some("0")),
            opt("seal", BOOL, N, Some("false")),
            opt("boundary", ArgType::Choice(&["smooth", "locked", "maintain", "straighten"]), N, Some("smooth")),
            opt("area", FLOAT, N, Some("0.1")),
            opt("uniform", BOOL, N, Some("false")),
            opt("noEdge", BOOL, N, Some("true")),
            opt("name", TEXT, N, None),
        ])
        .undo()
        .example("uv.relax iter:10"),
    script("operandi.uv.atlas", Modeling, "Atlas Unwrap: an automatic UV layout of the visible polygons with no seams to mark (charts of polygons within angle: of each other, packed at one density). One undo step.")
        .operandi()
        .headless(
            "Atlas Unwrap of the active mesh's visible polygons into the UV map: charts of neighbouring polygons whose normals stay within angle of the chart's first, packed into 0-1 at one texel density. One undo step. Unnamed attributes keep the last values.",
            &[
            jopt("name", "string", "UV map; made when missing (default Texture)."),
            jopt("angle", "number", "Most a chart bends, radians (default 1.0472, 60°)."),
            jopt("gaps", "number", "Space around each chart in texels at resolution (default 4)."),
            jopt("resolution", "integer", "Texture size the gaps are measured at (default 1024)."),
        ],
        )
        .args(&[
            opt("name", TEXT, N, Some("Texture")),
            opt("angle", FLOAT, Unit::Angle, Some("1.0472")),
            opt("gaps", FLOAT, N, Some("4")),
            opt("resolution", INT, N, Some("1024")),
        ])
        .undo()
        .example("operandi.uv.atlas angle:0.7854"),
    script("uv.pack", Modeling, "Pack UVs of the UV map; only the selected polygons' islands move when polygons are selected.")
        .headless(
            "Pack UVs of the UV map; only the selected polygons' islands move when polygons are selected.",
            &[
            jopt("pack", "boolean", "Islands may move."),
            jopt(
                "stretch",
                "boolean",
                "Match each island's UV area to its 3D area first.",
            ),
            jopt("orient", "boolean", "Islands may rotate."),
            jopt("keepSize", "boolean", "Islands keep their size."),
            jopt(
                "direction",
                "integer|string",
                "0 horizontal, 1 vertical, 2 auto.",
            ),
            jopt("gaps", "number", "Gaps, 0-1 (default 0.2)."),
            jopt("byPixel", "boolean", "Set Gaps by Pixel."),
            jopt(
                "gapsByPixel",
                "number",
                "Pixel Size (gap in pixels of a 1024 texture).",
            ),
            jopt("bbox", "boolean", "Based on Bounding Box."),
            jopt("stack", "boolean", "Lock Stacked."),
            jopt(
                "region",
                "integer|string",
                "0 normalized, 1 nearest, 2 udim, 3 manual, 4 tiles, 5 polygonTag, 6 byItem.",
            ),
            jopt("udim", "integer", "Target UDIM tile (from 1001)."),
            jopt("regionX", "number", "Manual region corner U."),
            jopt("regionY", "number", "Manual region corner V."),
            jopt("regionW", "number", "Manual region width."),
            jopt("regionH", "number", "Manual region height."),
            jopt("tileU", "integer", "Tiles wide."),
            jopt("tileV", "integer", "Tiles high."),
            jopt(
                "polygonTag",
                "integer|string",
                "0 material, 1 part, 2 smoothingGroup, 3 selectionSet: the Polygon Tag region's groups.",
            ),
            jopt(
                "background",
                "boolean",
                "Background Constraint: other visible meshes' islands stay put and are packed round.",
            ),
            jopt(
                "writeNew",
                "boolean",
                "Pack UVs to New Map: keep the original map.",
            ),
            jopt("newName", "string", "The new map's name (with writeNew)."),
            jopt("bestrot", "boolean", "Find Best Box Rotation."),
        ],
        )
        .args(&[
            opt("pack", BOOL, N, Some("true")),
            opt("stretch", BOOL, N, Some("true")),
            opt("orient", BOOL, N, Some("false")),
            opt("keepSize", BOOL, N, Some("false")),
            opt("direction", ArgType::Choice(&["horizontal", "vertical", "auto"]), N, Some("auto")),
            opt("gaps", FLOAT, N, Some("0.2")),
            opt("byPixel", BOOL, N, Some("false")),
            opt("gapsByPixel", FLOAT, N, Some("10.24")),
            opt("bbox", BOOL, N, Some("false")),
            opt("stack", BOOL, N, Some("false")),
            opt("region", ArgType::Choice(&["normalized", "nearest", "udim", "manual", "tiles", "polygonTag", "byItem"]), N, Some("normalized")),
            opt("udim", INT, N, Some("1001")),
            opt("regionX", FLOAT, N, Some("0")),
            opt("regionY", FLOAT, N, Some("0")),
            opt("regionW", FLOAT, N, Some("1")),
            opt("regionH", FLOAT, N, Some("1")),
            opt("tileU", INT, N, Some("1")),
            opt("tileV", INT, N, Some("1")),
            opt("polygonTag", ArgType::Choice(&["material", "part", "smoothingGroup", "selectionSet"]), N, Some("material")),
            opt("background", BOOL, N, Some("false")),
            opt("writeNew", BOOL, N, Some("false")),
            opt("newName", TEXT, N, None),
            opt("bestrot", BOOL, N, Some("true")),
        ])
        .undo()
        .example("uv.pack orient:true"),
    // UV editing stage 1 (uv-editing-2026-11-16). The map has no names for
    // these yet: `operandi.*` placeholders (docs/NAMES-NEEDED.md).
    script("operandi.uvMap.select", Modeling, "Selects a UV map of the active mesh: the one the UV view, UV tools and Unwrap use.")
        .operandi()
        .headless(
            "Select a UV map of the active mesh (the UV view, the UV tools and Unwrap use it), one undo step.",
            &[jreq("name", "string", "The map's name.")],
        )
        .args(&[arg("name", TEXT, N)])
        .undo()
        .example("operandi.uvMap.select Iris"),
    script("operandi.uvMap.rename", Modeling, "Renames a UV map of the active mesh.")
        .operandi()
        .headless(
            "Rename a UV map of the active mesh, one undo step.",
            &[
                jreq("name", "string", "The map's name now."),
                jreq("newName", "string", "Its new name."),
            ],
        )
        .args(&[arg("name", TEXT, N), arg("newName", TEXT, N)])
        .undo()
        .example("operandi.uvMap.rename Texture Iris"),
    script("operandi.uvMap.delete", Modeling, "Deletes UV maps of the active mesh (names separated by commas).")
        .operandi()
        .headless(
            "Delete one or several UV maps of the active mesh in one undo step.",
            &[jreq("name", "string", "The map's name, or several separated by commas.")],
        )
        .args(&[arg("name", TEXT, N)])
        .undo()
        .example("operandi.uvMap.delete \"Texture 2,Texture 3\""),
    script("operandi.uvMap.copy", Modeling, "Copies a whole UV map of the active mesh to the UV map clipboard.")
        .operandi()
        .headless(
            "Copy a whole UV map's values to the UV map clipboard (no undo step).",
            &[jreq("name", "string", "The map's name.")],
        )
        .args(&[arg("name", TEXT, N)])
        .example("operandi.uvMap.copy Vest").side_effect(),
    script("operandi.uvMap.paste", Modeling, "Replaces a UV map's values with the copied map's.")
        .operandi()
        .headless(
            "Replace a UV map's values with the copied map's (same polygons needed), one undo step.",
            &[jreq("name", "string", "The map pasted into.")],
        )
        .args(&[arg("name", TEXT, N)])
        .undo()
        .example("operandi.uvMap.paste \"Vest 2\""),
    script("operandi.uv.fit", Modeling, "Fit UVs: scales and moves the selected UVs to fill a UDIM tile.")
        .operandi()
        .headless(
            "Fit UVs: scale and move the selected UVs (all visible ones with nothing selected) to fill a UDIM tile, one undo step. Unnamed attributes keep the last values.",
            &[
                jopt("fit", "string", "entire (default), island or polygon."),
                jopt("keepProportion", "boolean", "Scale uniformly (default true)."),
                jopt("gaps", "number", "Margin inside the tile, percent (default 0)."),
                jopt("gapsByPixel", "number", "The margin in pixels of a 1024 texture instead."),
                jopt("udim", "integer", "The tile (default 1001)."),
            ],
        )
        .args(&[
            opt("fit", ArgType::Choice(&["entire", "island", "polygon"]), N, Some("entire")),
            opt("keepProportion", BOOL, N, Some("true")),
            opt("gaps", FLOAT, N, Some("0")),
            opt("gapsByPixel", FLOAT, N, None),
            opt("udim", INT, N, Some("1001")),
        ])
        .undo()
        .example("operandi.uv.fit fit:island"),
    script("operandi.uv.flip", Modeling, "Flip UVs: mirrors the selected UVs about their own centre.")
        .operandi()
        .headless(
            "Flip U or V: mirror the selected UVs about their bounding-box centre, or each polygon about its own, one undo step.",
            &[
                jopt("axis", "string", "u (default, left to right) or v (top to bottom)."),
                jopt("perPolygon", "boolean", "Each polygon about its own centre."),
            ],
        )
        .args(&[
            opt("axis", ArgType::Choice(&["u", "v"]), N, Some("u")),
            opt("perPolygon", BOOL, N, Some("false")),
        ])
        .undo()
        .example("operandi.uv.flip axis:v"),
    // UV tab stage 3 (uv_ops.rs; BEHAVIOR-UV-TOOLS §7.5-7.9, §9, §11, §12).
    script("operandi.uv.quickAlign", Modeling, "Quick Align: turns each island so its first selected edge lies along U or V.")
        .operandi()
        .headless(
            "Quick Align (UVT-54): turn each island holding the selection so its first selected edge (or the line between its first two selected points) lies along an axis, one undo step.",
            &[jopt("mode", "string", "auto (default: the nearer axis), horizontal or vertical.")],
        )
        .args(&[opt("mode", ArgType::Choice(&["auto", "horizontal", "vertical"]), N, Some("auto"))])
        .undo()
        .example("operandi.uv.quickAlign mode:horizontal"),
    script("operandi.uv.islandAlign", Modeling, "Island Align: lines the selected islands up by their sides or centres.")
        .operandi()
        .headless(
            "Island Align (UVT-55): move each selected island so their left sides, right sides or centres (u) and bottoms, tops or middles (v) line up, one undo step.",
            &[
                jopt("u", "string", "min, max, center or unchanged (default)."),
                jopt("v", "string", "min, max, center or unchanged (default)."),
            ],
        )
        .args(&[
            opt("u", ArgType::Choice(&["min", "max", "center", "unchanged"]), N, Some("unchanged")),
            opt("v", ArgType::Choice(&["min", "max", "center", "unchanged"]), N, Some("unchanged")),
        ])
        .undo()
        .example("operandi.uv.islandAlign u:min"),
    script("operandi.uv.orient", Modeling, "Orient Pieces: turns each selected island to sit squarely.")
        .operandi()
        .headless(
            "Orient Pieces (UVT-56): turn each island in its smallest box (auto), or with its longer side along U (horizontal) or V (vertical), one undo step.",
            &[jopt("mode", "string", "auto (default), horizontal or vertical.")],
        )
        .args(&[opt("mode", ArgType::Choice(&["auto", "horizontal", "vertical"]), N, Some("auto"))])
        .undo()
        .example("operandi.uv.orient mode:horizontal"),
    script("operandi.uv.toOrigin", Modeling, "To Origin: moves the selected islands to a UDIM tile's corner.")
        .operandi()
        .headless(
            "To Origin (UVT-57): move each selected island so its lower-left corner sits on the tile's lower-left corner, one undo step.",
            &[jopt("udim", "integer", "The tile (default 1001).")],
        )
        .args(&[opt("udim", INT, N, Some("1001"))])
        .undo()
        .example("operandi.uv.toOrigin udim:1002"),
    script("operandi.uv.texel", Modeling, "Texel Density: scales the selected islands to a texel density.")
        .operandi()
        .headless(
            "Texel Density (UVT-58/59): density = Image Size x sqrt(UV area / 3D area). Apply individually or overall, or normalize, one undo step.",
            &[
                jopt("action", "string", "individually (default), overall or normalize."),
                jopt("imageSize", "number", "Texture size in pixels (the panel's value)."),
                jopt("density", "number", "Target pixels per metre (the panel's value)."),
                jopt("worldSpace", "boolean", "Item scale counts (the panel's value)."),
                jopt("upright", "boolean", "Turn islands upright first (Alt)."),
            ],
        )
        .args(&[
            opt("action", ArgType::Choice(&["individually", "overall", "normalize"]), N, Some("individually")),
            opt("imageSize", FLOAT, N, None),
            opt("density", FLOAT, N, None),
            opt("worldSpace", BOOL, N, None),
            opt("upright", BOOL, N, Some("false")),
        ])
        .undo()
        .example("operandi.uv.texel action:overall density:512"),
    script("operandi.uv.realWorldSize", Modeling, "Real World Size: one UV unit stands for a length.")
        .operandi()
        .headless(
            "Real World Size: scale the selected islands together so one UV unit covers size metres, one undo step.",
            &[
                jopt("size", "number", "Metres one UV unit stands for (default 1)."),
                jopt("worldSpace", "boolean", "Item scale counts (default true)."),
            ],
        )
        .args(&[opt("size", FLOAT, N, Some("1")), opt("worldSpace", BOOL, N, Some("true"))])
        .undo()
        .example("operandi.uv.realWorldSize size:2"),
    script("operandi.uv.split", Modeling, "Split UVs: splits the selected UVs from the rest.")
        .operandi()
        .headless(
            "Split UVs (UVT-60): split along the selected edges, round a polygon selection, or at selected points, the sides moved apart by distance, one undo step.",
            &[
                jopt("size", "string", "plain (default, invisibly apart), medium or large."),
                jopt("distance", "number", "A custom distance in UV units instead."),
            ],
        )
        .args(&[
            opt("size", ArgType::Choice(&["plain", "medium", "large"]), N, Some("plain")),
            opt("distance", FLOAT, N, None),
        ])
        .undo()
        .example("operandi.uv.split size:large"),
    script("operandi.uv.sew", Modeling, "Sew UVs: joins UV seams at the selected edges or points.")
        .operandi()
        .headless(
            "Sew UVs (UVT-61): join each selected seam's two sides into one UV position, one undo step.",
            &[
                jopt("connect", "string", "average (default), selected or unselected."),
                jopt("selectedOnly", "boolean", "Keep the unselected side still (Move Selected Only)."),
            ],
        )
        .args(&[
            opt("connect", ArgType::Choice(&["average", "selected", "unselected"]), N, Some("average")),
            opt("selectedOnly", BOOL, N, Some("false")),
        ])
        .undo()
        .example("operandi.uv.sew connect:selected"),
    script("operandi.uv.moveSew", Modeling, "Move and Sew: moves an island to the seam, then joins it.")
        .operandi()
        .headless(
            "Move and Sew (UVT-62): move (turn, and scale) the island across each selected seam so the seam lines up, then join it, one undo step.",
            &[
                jopt("connect", "string", "average (default), selected or unselected."),
                jopt("scale", "boolean", "Scale the island to fit (default true)."),
            ],
        )
        .args(&[
            opt("connect", ArgType::Choice(&["average", "selected", "unselected"]), N, Some("average")),
            opt("scale", BOOL, N, Some("true")),
        ])
        .undo()
        .example("operandi.uv.moveSew connect:selected"),
    script("operandi.uv.selectBorder", Modeling, "Select Border: selects every UV island's outline edges.")
        .operandi()
        .headless("Select UV Border (UVT-80): select the edges on every island's outline.", &[])
        .undo()
        .example("operandi.uv.selectBorder"),
    script("operandi.uv.selectErrors", Modeling, "Select Error UVs: selects polygons failing the ticked tests.")
        .operandi()
        .headless(
            "Select Error UVs (UVT-81): select the polygons that fail any ticked test.",
            &[
                jopt("cross", "boolean", "Overlapping another UV polygon."),
                jopt("self", "boolean", "Twisted (bow-tie) UV polygons."),
                jopt("flipped", "boolean", "Wound clockwise in UV."),
                jopt("stacked", "boolean", "Exactly on top of another."),
                jopt("udim", "boolean", "Crossing a UDIM tile border."),
                jopt("zero", "boolean", "Zero UV area."),
            ],
        )
        .args(&[
            opt("cross", BOOL, N, Some("true")),
            opt("self", BOOL, N, Some("true")),
            opt("flipped", BOOL, N, Some("true")),
            opt("stacked", BOOL, N, Some("true")),
            opt("udim", BOOL, N, Some("false")),
            opt("zero", BOOL, N, Some("true")),
        ])
        .undo()
        .example("operandi.uv.selectErrors flipped:true cross:false"),
    script("operandi.uv.selectUnmapped", Modeling, "Select Unmapped: selects polygons with no UVs in the map.")
        .operandi()
        .headless(
            "Select Unmapped (UVT-82): select the polygons with no UVs in the selected map, or the mapped ones with invert.",
            &[jopt("invert", "boolean", "Select the mapped polygons instead.")],
        )
        .args(&[opt("invert", BOOL, N, Some("false"))])
        .undo()
        .example("operandi.uv.selectUnmapped"),
    script("operandi.uv.copy", Modeling, "Copy UVs: puts the selected polygons' UVs on the UV clipboard.")
        .operandi()
        .headless("Copy UVs (UVT-90): the selected polygons' UVs onto the UV clipboard.", &[])
        .example("operandi.uv.copy").side_effect(),
    script("operandi.uv.cut", Modeling, "Cut UVs: copies the selected polygons' UVs, then removes them.")
        .operandi()
        .headless("Cut UVs (UVT-90): copy, then remove the selected polygons' UVs, one undo step.", &[])
        .undo()
        .example("operandi.uv.cut"),
    script("operandi.uv.paste", Modeling, "Paste UVs: writes the UV clipboard onto the selected polygons.")
        .operandi()
        .headless(
            "Paste UVs (UVT-91): write the clipboard onto the selected polygons, matched by topology or by selection order, one undo step.",
            &[jopt("mode", "string", "connectivity (default) or selection.")],
        )
        .args(&[opt("mode", ArgType::Choice(&["connectivity", "selection"]), N, Some("connectivity"))])
        .undo()
        .example("operandi.uv.paste mode:selection"),
    script("operandi.uv.delete", Modeling, "Delete UVs: removes the selected polygons' UVs from the map.")
        .operandi()
        .headless("Delete UVs (UVT-93): the selected polygons become unmapped, one undo step.", &[])
        .undo()
        .example("operandi.uv.delete"),
    script("operandi.uv.setValue", Modeling, "Set UV Value: sets U or V of the selected points.")
        .operandi()
        .headless(
            "Set UV Value (UVT-94): set U, V or both of the selected UVs, one undo step.",
            &[jopt("u", "number", "The U value."), jopt("v", "number", "The V value.")],
        )
        .args(&[opt("u", FLOAT, N, None), opt("v", FLOAT, N, None)])
        .undo()
        .example("operandi.uv.setValue u:0.5"),
    script("operandi.uv.rotatePolygons", Modeling, "Rotate Polygons: shifts each selected polygon's UVs one corner along.")
        .operandi()
        .headless("Rotate Polygon UVs (UVT-95): each selected polygon's corners take the next corner's UV, one undo step.", &[])
        .undo()
        .example("operandi.uv.rotatePolygons"),
    script("operandi.uv.mergeMaps", Modeling, "Merge UV Maps: merges maps into the first, deleting the others.")
        .operandi()
        .headless(
            "Merge UV Maps (UVT-96): fill the first map from the others where it has no UVs, then delete the others, one undo step.",
            &[jopt("names", "string", "The maps, comma separated, the target first.")],
        )
        .args(&[opt("names", TEXT, N, None)])
        .undo()
        .example("operandi.uv.mergeMaps names:Texture,Texture 2"),
    script("operandi.uv.seam", Modeling, "Cut Map: adds, clears, selects or deletes a UV seam map's edges; marker grows a Conditional Loop into it.")
        .operandi()
        .headless(
            "UV seam maps (UVT-100/101, UVS-1 §1.6): add the selected edges (all with none selected), clear them, select the map's edges, delete the map, or mark the Conditional Loop from the selected edges, one undo step.",
            &[
                jopt("action", "string", "add (default), clear, select, delete, marker, or use (pick the map the buttons use)."),
                jopt("name", "string", "The seam map (default: the one in use, UV Seam to start)."),
            ],
        )
        .args(&[
            opt("action", ArgType::Choice(&["add", "clear", "select", "delete", "marker", "use"]), N, Some("add")),
            opt("name", TEXT, N, None),
        ])
        .undo()
        .example("operandi.uv.seam action:add name:Body"),
    script("operandi.uv.symmetry", Modeling, "UV Symmetry: off, or mirror UV edits across a U or V line.")
        .operandi()
        .headless(
            "The UV Symmetry layer (UVT-70): in a UV view, an edit to a UV is copied, mirrored, to its partner across the line.",
            &[
                jopt("axis", "string", "off (default), u or v."),
                jopt("center", "number", "The line's position (default 0.5, the last used)."),
            ],
        )
        .args(&[
            opt("axis", ArgType::Choice(&["off", "u", "v"]), N, Some("off")),
            opt("center", FLOAT, N, None),
        ])
        .quiet()
        .example("operandi.uv.symmetry axis:u center:0.5"),
    script("operandi.uv.symmetryCenter", Modeling, "UV Symmetry Center: welds the UVs lying on the symmetry line.")
        .operandi()
        .headless("Center (UVT-70): UVs on the UV symmetry line land on it, and one point's UVs there become one, one undo step.", &[])
        .undo()
        .example("operandi.uv.symmetryCenter"),
    script("operandi.uv.symmetrize", Modeling, "Symmetrize UVs: copies UVs from 3D mirror polygons, mirrored across the UV line.")
        .operandi()
        .headless(
            "Symmetrize (UVT-71): each polygon on the target side gets its 3D mirror partner's UVs, mirrored across the UV symmetry line, one undo step.",
            &[
                jopt("axis", "string", "The 3D mirror axis: x (default), y or z."),
                jopt("positiveToNegative", "boolean", "Copy from the + side to the - side (default true)."),
                jopt("mirror", "string", "The UV axis mirrored: u (default) or v."),
            ],
        )
        .args(&[
            opt("axis", ArgType::Choice(&["x", "y", "z"]), N, Some("x")),
            opt("positiveToNegative", BOOL, N, Some("true")),
            opt("mirror", ArgType::Choice(&["u", "v"]), N, Some("u")),
        ])
        .undo()
        .example("operandi.uv.symmetrize axis:x mirror:u"),
    script("operandi.uv.selectTile", Modeling, "UDIM Indicator: selects the polygons whose UVs lie inside a tile.")
        .operandi()
        .headless(
            "Select UVs in the Region (UVT-113): select the polygons whose UVs lie wholly inside the UDIM tile.",
            &[jopt("udim", "integer", "The tile (default 1001).")],
        )
        .args(&[opt("udim", INT, N, Some("1001"))])
        .undo()
        .example("operandi.uv.selectTile udim:1002"),
    script("operandi.uv.nudgeTile", Modeling, "UDIM Indicator arrows: moves the selected islands whole tiles.")
        .operandi()
        .headless(
            "Move the selected islands whole tiles over (UVT-113 arrows), one undo step.",
            &[
                jopt("u", "integer", "Tiles right (negative left)."),
                jopt("v", "integer", "Tiles up (negative down)."),
            ],
        )
        .args(&[opt("u", INT, N, Some("0")), opt("v", INT, N, Some("0"))])
        .undo()
        .example("operandi.uv.nudgeTile u:1"),
    script("operandi.uv.export", Modeling, "Export UVs: the UV map's edges as an SVG or EPS drawing of the 0-1 tile.")
        .operandi()
        .headless(
            "Export UVs to SVG or EPS (UVT-120): the current map's edges, the 0-1 tile size units square, V up.",
            &[
                jopt("filename", "string", "Where to write it."),
                jopt("format", "string", "svg (default) or eps."),
                jopt("size", "number", "The drawing's width and height (default 1024)."),
            ],
        )
        .args(&[
            arg("filename", TEXT, N),
            opt("format", ArgType::Choice(&["svg", "eps"]), N, Some("svg")),
            opt("size", FLOAT, N, Some("1024")),
        ])
        .side_effect()
        .example("operandi.uv.export filename:layout.svg"),
    script("operandi.uv.toMesh", Modeling, "Convert UVs to Mesh: a flat mesh item shaped like the UV layout.")
        .operandi()
        .headless("Convert UVs to Mesh (UVT-120): a new mesh with a point per UV position (U to X, V to Y) and a polygon per mapped polygon, one undo step.", &[])
        .undo()
        .example("operandi.uv.toMesh"),
    script("operandi.uv.transfer", Modeling, "Transfer UVs: copies the primary mesh's UV map to the other selected meshes of the same topology.")
        .operandi()
        .headless("Transfer UVs (UVT-92): the primary active mesh's current map onto every other active mesh with the same polygons, one undo step.", &[])
        .undo()
        .example("operandi.uv.transfer"),
    script("operandi.uv.mirror", Modeling, "Mirror UVs across a line on U or V.")
        .operandi()
        .headless(
            "Mirror the selected UVs across the line axis = center, one undo step.",
            &[
                jopt("axis", "string", "u (default) or v."),
                jopt("center", "number", "The line's position (default 0.5, the last used)."),
            ],
        )
        .args(&[
            opt("axis", ArgType::Choice(&["u", "v"]), N, Some("u")),
            opt("center", FLOAT, N, Some("0.5")),
        ])
        .undo()
        .example("operandi.uv.mirror axis:u center:0.5"),
    script("operandi.uv.align", Modeling, "Align UVs: moves the selected UVs onto one line.")
        .operandi()
        .headless(
            "Align UVs: set the selected UVs' U (left, right) or V (up, down) to one value of their bounding box, one undo step.",
            &[
                jopt("direction", "string", "up, down, left or right."),
                jopt("alignTo", "string", "low, average, center or high (default the direction's far side)."),
                jopt("perRow", "boolean", "Each run of connected selected edges on its own."),
            ],
        )
        .args(&[
            opt("direction", ArgType::Choice(&["up", "down", "left", "right"]), N, Some("down")),
            opt("alignTo", ArgType::Choice(&["low", "average", "center", "high"]), N, None),
            opt("perRow", BOOL, N, Some("false")),
        ])
        .undo()
        .example("operandi.uv.align direction:down"),
    // Layouts stage 3 (UIC-210): the toolbox's E7 island buttons as one
    // command line each, so a form can hold them. `click` is the button's
    // modifier: shift and alt pick the rule's other groupings, options
    // opens its dialog first (uv_edit_ui.rs).
    // The UV tab's stage 3 dialogs (uv_ops_ui.rs): a button's options
    // first (UVT-08), then one run.
    script("operandi.uv.dialog", Modeling, "Opens a UV button's options: toOrigin, texel, split, sew, moveSew, errors, setValue, paste, quickAlign or orient.")
        .operandi()
        .args(&[arg("kind", ArgType::Choice(&["toOrigin", "texel", "split", "sew", "moveSew", "errors", "setValue", "paste", "quickAlign", "orient", "seamMap", "symmetrize", "selectTile"]), N)])
        .ui()
        .example("operandi.uv.dialog texel"),
    script("operandi.uv.unwrapSeams", Modeling, "Cut Map's Unwrap Tool (or tool:pelt, Unwrap & Relax) with the seam map in use as its seams.")
        .operandi()
        .args(&[opt("tool", ArgType::Choice(&["unwrap", "pelt"]), N, Some("unwrap"))])
        .ui()
        .example("operandi.uv.unwrapSeams pelt"),
    script("operandi.uv.brush", Modeling, "The Split and Sew brush in a UV view: split, sew or off; bare toggles it.")
        .operandi()
        .args(&[opt("mode", ArgType::Choice(&["split", "sew", "off"]), N, None)])
        .ui()
        .example("operandi.uv.brush sew"),
    script("operandi.uv.preset", Modeling, "Runs a UV island button (uv.fitUVs, uv.flipU, uv.alignUp ...) with the remembered values; click: shift or alt picks its other groupings, options opens its dialog.")
        .operandi()
        .args(&[
            arg("id", TEXT, N),
            opt("click", ArgType::Choice(&["plain", "shift", "alt", "options"]), N, Some("plain")),
        ])
        .undo()
        .example("operandi.uv.preset uv.fitUVs shift"),
    // Remesh names from the registry thread (2026-11-11): mesh.retopo and
    // sculpt.remesh are the reference's; Blocks / Smooth / Sharp have no
    // reference command, so they are Operandi's. Bare opens the dialog.
    script("mesh.retopo", Modeling, "Automatic Retopology: rebuilds the active mesh as even quads that follow its shape; bare opens the dialog, unnamed attributes keep the last values.")
        .headless(
            "Automatic Retopology: rebuilds the active mesh as even quads that follow its shape (QuadriFlow); unnamed attributes keep the last values.",
            &[
            jopt(
                "countMode",
                "string",
                "number or percent (polyTargetNum or polyTargetPct).",
            ),
            jopt(
                "polyTargetNum",
                "integer",
                "About how many quads (default 5000).",
            ),
            jopt(
                "polyTargetPct",
                "number",
                "About this share of the polygon count, 0-10 (default 0.25).",
            ),
            jopt(
                "creaseAngle",
                "number",
                "Radians; new edges follow sharper creases, 0 is off (default 0).",
            ),
            jopt(
                "boundaryAlign",
                "boolean",
                "New edges follow open borders (default true).",
            ),
            jopt(
                "vertNormals",
                "boolean",
                "Smooth shade the result (default true).",
            ),
            jopt(
                "newMesh",
                "boolean",
                "Put the result in a new mesh item (default false).",
            ),
            jopt(
                "seed",
                "integer",
                "Operandi only: random start; the same seed gives the same quads.",
            ),
        ],
        )
        .args(&[
            opt("countMode", ArgType::Choice(&["number", "percent"]), N, Some("number")),
            opt("polyTargetNum", INT, N, Some("5000")),
            opt("polyTargetPct", FLOAT, Unit::Percent, Some("0.25")),
            opt("creaseAngle", FLOAT, Unit::Angle, Some("0")),
            opt("boundaryAlign", BOOL, N, Some("true")),
            opt("vertNormals", BOOL, N, Some("true")),
            opt("newMesh", BOOL, N, Some("false")),
            // Operandi only (docs/COMMAND_REGISTRY.md).
            opt("seed", INT, N, Some("0")),
        ])
        .undo()
        .example("mesh.retopo polyTargetNum:2000 creaseAngle:30deg"),
    script("sculpt.remesh", Sculpt, "Voxel Remesh: rebuilds the active mesh as an even quad skin on a voxel grid, joining overlaps and closing holes; bare opens the dialog.")
        .headless(
            "Voxel Remesh: rebuilds the active mesh as an even quad skin on a voxel grid; joins overlaps and closes holes. Unnamed attributes keep the last values.",
            &[
            jopt("density", "number", "0-1, higher is finer (default 0.5)."),
            jopt(
                "voxelSize",
                "number",
                "Operandi only: voxel size in metres; above 0 it replaces density.",
            ),
            jopt(
                "keepMaps",
                "boolean",
                "Operandi only: weight and colour maps carry over (default true).",
            ),
            jopt(
                "vertNormals",
                "boolean",
                "Operandi only: smooth shade the result (default true).",
            ),
        ],
        )
        .args(&[
            opt("density", FLOAT, Unit::Percent, Some("0.5")),
            // Operandi only (docs/COMMAND_REGISTRY.md).
            opt("voxelSize", FLOAT, Unit::Distance, Some("0")),
            opt("keepMaps", BOOL, N, Some("true")),
            opt("vertNormals", BOOL, N, Some("true")),
        ])
        .undo()
        .example("sculpt.remesh density:0.6"),
    // Bakes from TextureCraft (texturecraft-mining-2026-11-17): Operandi's
    // own names (no map name), placeholders in docs/NAMES-NEEDED.md.
    script("operandi.bake.curvature", Shading, "Bakes the active mesh's Curvature map (convex edges light, cavities dark, flat 50 % grey) into its UVs as a 16-bit greyscale PNG, with padding. Not undoable; bare opens the dialog.")
        .operandi()
        .headless(
            "Bake the active mesh's Curvature map into its UV layout and write it as a 16-bit greyscale PNG: convex edges light, cavities dark, flat 50 % grey. The scene doesn't change.",
            &[
            jreq("filename", "string", "The .png to write."),
            jopt("size", "integer", "Width and height in pixels, 16-8192 (default 1024)."),
            jopt("padding", "integer", "Pixels of padding around each UV island (default 16)."),
            jopt("fill", "boolean", "Fill the rest of the image smoothly (default true)."),
            jopt("uvMap", "string", "UV map baked into (default the mesh's Texture map)."),
            jopt("rays", "integer", "Rays per pixel, 1-1024 (default 32)."),
            jopt("radius", "number", "Sampling radius as a share of the mesh's size (default 0.001)."),
        ],
        )
        .args(&[
            opt("filename", ArgType::Path, N, None),
            opt("size", INT, N, Some("1024")),
            opt("padding", INT, N, Some("16")),
            opt("fill", BOOL, N, Some("true")),
            opt("uvMap", TEXT, N, None),
            opt("rays", INT, N, Some("32")),
            opt("radius", FLOAT, N, Some("0.001")),
        ])
        .side_effect()
        .example("operandi.bake.curvature filename:/tmp/curvature.png size:2048"),
    script("operandi.bake.thickness", Shading, "Bakes the active mesh's Thickness map (thin parts dark, thick parts light) into its UVs as a 16-bit greyscale PNG, with padding. Not undoable; bare opens the dialog.")
        .operandi()
        .headless(
            "Bake the active mesh's Thickness map into its UV layout and write it as a 16-bit greyscale PNG: thin parts dark, thick parts light. The scene doesn't change.",
            &[
            jreq("filename", "string", "The .png to write."),
            jopt("size", "integer", "Width and height in pixels, 16-8192 (default 1024)."),
            jopt("padding", "integer", "Pixels of padding around each UV island (default 16)."),
            jopt("fill", "boolean", "Fill the rest of the image smoothly (default true)."),
            jopt("uvMap", "string", "UV map baked into (default the mesh's Texture map)."),
            jopt("rays", "integer", "Rays per pixel, 1-1024 (default 64)."),
            jopt("distance", "number", "Longest distance measured, as a share of the mesh's size (default 0.1)."),
        ],
        )
        .args(&[
            opt("filename", ArgType::Path, N, None),
            opt("size", INT, N, Some("1024")),
            opt("padding", INT, N, Some("16")),
            opt("fill", BOOL, N, Some("true")),
            opt("uvMap", TEXT, N, None),
            opt("rays", INT, N, Some("64")),
            opt("distance", FLOAT, N, Some("0.1")),
        ])
        .side_effect()
        .example("operandi.bake.thickness filename:/tmp/thickness.png"),
    script("operandi.bake.padImage", Shading, "Pads a PNG texture around the active mesh's UV islands so seams don't show background when filtered; writes output: or the file itself. Not undoable; bare opens the dialog.")
        .operandi()
        .headless(
            "Pad a PNG texture around the active mesh's UV islands (seam padding), writing output or the file itself. The scene doesn't change.",
            &[
            jreq("filename", "string", "The .png to pad."),
            jopt("output", "string", "Where to write the result (default: replace filename)."),
            jopt("padding", "integer", "Pixels of padding (default 16)."),
            jopt("uvMap", "string", "UV map whose islands are used (default the mesh's Texture map)."),
        ],
        )
        .args(&[
            opt("filename", ArgType::Path, N, None),
            opt("output", ArgType::Path, N, None),
            opt("padding", INT, N, Some("16")),
            opt("uvMap", TEXT, N, None),
        ])
        .side_effect()
        .example("operandi.bake.padImage filename:/tmp/color.png padding:8"),
    script("operandi.remesh", Modeling, "Remesh Blocks, Smooth or Sharp: rebuilds the active mesh on an octree; bare opens the dialog.")
        .operandi()
        .headless(
            "Remesh Blocks, Smooth or Sharp: rebuilds the active mesh on an octree-sized grid. Unnamed attributes keep the last values.",
            &[
            jopt(
                "mode",
                "string",
                "blocks, smooth or sharp (default smooth).",
            ),
            jopt("depth", "integer", "Octree Depth 2-12 (default 6)."),
            jopt(
                "scale",
                "number",
                "How much of the octree the mesh fills, 0-1 (default 0.9).",
            ),
            jopt(
                "sharpness",
                "number",
                "Sharp mode: higher keeps more edges sharp (default 1).",
            ),
            jopt(
                "removeDisconnected",
                "boolean",
                "Drop small loose pieces (default false).",
            ),
            jopt(
                "threshold",
                "number",
                "With removeDisconnected: smallest piece kept, as a share of the biggest, 0-1 (default 1).",
            ),
            jopt(
                "vertNormals",
                "boolean",
                "Smooth shade the result (default true).",
            ),
        ],
        )
        .args(&[
            opt("mode", ArgType::Choice(&["blocks", "smooth", "sharp"]), N, Some("smooth")),
            opt("depth", INT, N, Some("6")),
            opt("scale", FLOAT, Unit::Percent, Some("0.9")),
            opt("sharpness", FLOAT, N, Some("1")),
            opt("removeDisconnected", BOOL, N, Some("false")),
            opt("threshold", FLOAT, Unit::Percent, Some("1")),
            opt("vertNormals", BOOL, N, Some("true")),
        ])
        .undo()
        .example("operandi.remesh mode:sharp depth:7"),
    script("poly.boolean", Modeling, "Boolean of the active mesh with its drivers (Union, Intersect, Subtract, Add); bare opens the dialog.")
        .headless(
            "Boolean of the active mesh with its drivers (BOOL-25); the drivers are untouched.",
            &[
            jopt(
                "mode",
                "integer|string",
                "0 union (default), 1 intersect, 2 subtract, 3 add.",
            ),
            jopt(
                "cutmesh",
                "integer|string",
                "Drivers: 0 background (default: visible unselected meshes), 1 lastSelected (the selected pieces of the active mesh), 2 item.",
            ),
            jopt(
                "item",
                "integer",
                "Driver item id for cutmesh item (a group drives with its meshes).",
            ),
            jopt(
                "children",
                "boolean",
                "Add the drivers' child meshes; default false.",
            ),
        ],
        )
        .args(&[
            opt("mode", ArgType::Choice(&["union", "intersect", "subtract", "add"]), N, Some("union")),
            opt("cutmesh", ArgType::Choice(&["background", "lastSelected", "item"]), N, Some("background")),
            opt("item", TEXT, N, None),
            opt("children", BOOL, N, Some("false")),
        ])
        .undo()
        .example("poly.boolean mode:subtract"),
    script("poly.solidDrill", Modeling, "Solid Drill: cuts the active mesh's polygons with a closed driver (Core, Tunnel, Stencil, Slice); bare opens the dialog.")
        .headless(
            "Solid Drill: cut the active mesh's polygons with a closed driver (DRILL-06); no driver faces are added.",
            &[
            jreq(
                "mode",
                "integer|string",
                "0 core, 1 tunnel, 2 stencil, 3 slice (or the word).",
            ),
            jopt(
                "material",
                "string",
                "Stencil's material for the inside pieces; default Stencil, created if missing.",
            ),
            jopt(
                "cutmesh",
                "integer|string",
                "Drivers: 0 background (default), 1 lastSelected, 2 item.",
            ),
            jopt("item", "integer", "Driver item id for cutmesh item."),
            jopt(
                "children",
                "boolean",
                "Add the drivers' child meshes; default false.",
            ),
        ],
        )
        .args(&[
            arg("mode", ArgType::Choice(&["core", "tunnel", "stencil", "slice"]), N),
            opt("material", TEXT, N, Some("Stencil")),
            opt("cutmesh", ArgType::Choice(&["background", "lastSelected", "item"]), N, Some("background")),
            opt("item", TEXT, N, None),
            opt("children", BOOL, N, Some("false")),
        ])
        .undo()
        .example("poly.solidDrill mode:tunnel"),
    script("poly.drill", Modeling, "Axis Drill: cuts the active mesh's polygons with the driver outline pushed along X, Y or Z; bare opens the dialog.")
        .headless(
            "Axis Drill: cut the active mesh's polygons with the driver outline's prism along an axis (DRILL-06).",
            &[
            jreq(
                "mode",
                "integer|string",
                "0 core, 1 tunnel, 2 stencil, 3 slice (or the word).",
            ),
            jreq("axis", "integer|string", "0 x, 1 y, 2 z (or the letter)."),
            jopt(
                "material",
                "string",
                "Stencil's material for the inside pieces; default Stencil, created if missing.",
            ),
            jopt(
                "cutmesh",
                "integer|string",
                "Drivers: 0 background (default), 1 lastSelected, 2 item.",
            ),
            jopt("item", "integer", "Driver item id for cutmesh item."),
            jopt(
                "children",
                "boolean",
                "Add the drivers' child meshes; default false.",
            ),
        ],
        )
        .args(&[
            arg("mode", ArgType::Choice(&["core", "tunnel", "stencil", "slice"]), N),
            arg("axis", ArgType::Choice(&["x", "y", "z"]), N),
            opt("material", TEXT, N, Some("Stencil")),
            opt("cutmesh", ArgType::Choice(&["background", "lastSelected", "item"]), N, Some("background")),
            opt("item", TEXT, N, None),
            opt("children", BOOL, N, Some("false")),
        ])
        .undo()
        .example("poly.drill mode:core axis:y"),
    script("poly.setMaterial", Modeling, "Tags the selected polygons with a material, creating it if new; bare opens the dialog.")
        .headless(
            "Set the material tag of the selected polygons (a new material when the name is new).",
            &[jreq("name", "string", "Material name.")],
        )
        .args(&[opt("name", TEXT, N, None)])
        .undo()
        .example("poly.setMaterial Clay"),
    script("poly.unify", Modeling, "Removes duplicate polygons; bare opens the options.")
        .headless(
            "Remove polygons that duplicate another (same points, either winding), keeping the lowest of each group; one undo step.",
            &[jopt("force", "boolean", "Also remove duplicates whose tags or corner data differ (default false).")],
        )
        .args(&[opt("force", BOOL, N, None)])
        .undo()
        .example("poly.unify false"),
    script("mesh.cleanup", Modeling, "Mesh Cleanup with twelve explicit switches (only floatingVertex is supported).")
        .headless(
            "Mesh Cleanup: run the checked operations on the selected polygons (the whole active mesh when none), one undo step (BRQ-60). Each switch not given takes the dialog's Reset value, on; all off is refused.",
            &[
            jopt("floatingVertex", "boolean", "Remove Floating Vertices (default true)."),
            jopt("onePointPolygon", "boolean", "Remove 1 Point Polygons (default true)."),
            jopt("twoPointPolygon", "boolean", "Remove 2 Point Polygons (default true)."),
            jopt("dupPointPolygon", "boolean", "Fix Duplicate Points in Polygons (default true)."),
            jopt("colinear", "boolean", "Remove Colinear Vertices (default true)."),
            jopt("faceNormal", "boolean", "Make Face Normals Consistent (default true)."),
            jopt("mergeVertex", "boolean", "Merge Overlapping Vertices (default true)."),
            jopt("mergeDisco", "boolean", "Merge Discontinuous UVs (default true)."),
            jopt("unifyPolygon", "boolean", "Unify Polygons (default true)."),
            jopt("forceUnify", "boolean", "Force Unify (default true)."),
            jopt("removeDiscoWeight", "boolean", "Remove Discontinuous Weights (default true)."),
            jopt("fixGaps", "boolean", "Fix Gaps (default true)."),
        ],
        )
        .args(CLEANUP_ARGS)
        .undo(),
    script("chamfer.edit", Modeling, "Applies Chamfer Edit once.")
        .headless(
            "Chamfer Edit once on the selected polygons; one undo step. Options not given take the Reset values.",
            &[
                jopt("offset", "number", "Metres (default 0)."),
                jopt("segments", "integer", "Default 1."),
                jopt("sourceSegs", "integer", "Default 1."),
                jopt("depth", "number", "Factor, 1 = 100 % (default 1)."),
                jopt("mode", "string", "inset (default) or width: what Offset changes."),
                jopt("absSize", "boolean", "Absolute Size: Offset is the inset or width itself (default false)."),
                jopt("swapSide", "boolean", "Swap Side: the strip runs the other way; Offset moves its open ends (default false)."),
                jopt("reverseX", "boolean", "Reverse Direction; no effect without a profile (default false)."),
                jopt("reverseY", "boolean", "Reverse Depth; no effect without a profile (default false)."),
            ],
        )
        .args(&[
            arg("offset", FLOAT, Unit::Distance),
            arg("segments", INT, N),
            arg("sourceSegs", INT, N),
            arg("depth", FLOAT, Unit::Percent),
        ])
        .undo()
        .example("chamfer.edit offset:0.1 segments:2 sourceSegs:1 depth:0.5"),
    script("poly.haul", Modeling, "Applies Poly Haul once.")
        .headless(
            "Poly Haul once on the selected polygons; one undo step.",
            &[jreq("offset", "number", "Metres.")],
        )
        .args(&[arg("offset", FLOAT, Unit::Distance)])
        .undo()
        .example("poly.haul offset:0.2"),
    script("operandi.sketch.sweep", Modeling, "Sketch Extrude along an explicit world path.")
        .operandi()
        .args(&[arg("path", ArgType::Json, N)])
        .undo()
        .example("operandi.sketch.sweep {\"path\":[[0,0,0],[1,0,0]]}"),
    script("@ppi_PolyScale_Offset.py", Modeling, "Opens Enter Inset Amount for Scale Offset.").ui(),
    script("@ppi_PolyScale_Offset_Detach.py", Modeling, "Scale Offset on new, unconnected points (opens its dialog).").ui(),
    script("@ppi_flatten_ngons.py", Modeling, "Flattens nonplanar polygons of the active mesh.")
        .headless(
            "Flatten the selected polygons of more than four sides.",
            &[],
        ).undo(),
    script("operandi.mesh.fixBlackDots", Modeling, "Fix Black Dots: merges polygon pairs that share a run of edges into one polygon, on the whole active mesh.")
        .operandi()
        .headless(
            "Fix Black Dots (MT2): every pair of polygons sharing two or more edges in a row (inner points used by those two only) becomes one polygon; the inner points are kept unused and selected with the run's ends, Vertex mode, one undo step. 'No black dots found.' when there are none.",
            &[],
        ).undo(),
    script("operandi.mesh.setData", Modeling, "Set Mesh Data: replaces a mesh item's points, and its polygons when given, as one undo step (scripts, K9).")
        .on_editor(
            "Set Mesh Data (K9): sets a mesh item's points (a list of [x, y, z] in item space, metres) and, when polygons is given, its polygons (lists of point indices; the mesh is rebuilt and its UV maps, vertex maps and component selection go). Points alone must keep the point count. One undo step; a bad mesh is refused and nothing changes.",
            &[
            jopt("item", "integer", "Mesh item id; default the active item."),
            jreq("points", "array", "Points as [x, y, z] lists."),
            jopt("polygons", "array", "Polygons as lists of point indices."),
        ],
            crate::script_mesh::run,
        )
        .undo()
        .operandi()
        .example("operandi.mesh.setData points:[[0,0,0],[1,0,0],[0,1,0]] polygons:[[0,1,2]]"),
    script("operandi.edge.zip", Modeling, "Edge Zip: welds the edge rows either side of the selected edges onto them, removing the quads between.")
        .operandi()
        .headless(
            "Edge Zip (MT3): each selected edge needs a quad on each side; its points and the points across those quads weld at their average, the quads collapse, one undo step, nothing selected afterwards.",
            &[],
        ).undo(),
    script("operandi.poly.diagonalRedirect", Modeling, "Diagonal Re-Direct: turns the edge flow through two diagonal quads 45 degrees.")
        .operandi()
        .headless(
            "Diagonal Re-Direct (MT4): two selected quads touching at one corner split along the diagonal through it; each side quad off the border gets a centre point and is rebuilt so the diagonal is an edge loop, one undo step.",
            &[],
        ).undo(),
    script("@ppi_vert_tear.py", Tools, "Starts the Tear tool.").tool(),
    script("poly.split", Modeling, "Connects pairs of selected vertices of one polygon with new edges.")
        .headless(
            "Split the selected polygons along selected vertices.",
            &[],
        ).undo(),
    script("poly.quadruple", Modeling, "Turns polygons laid out as two rows into a row of quads.")
        .headless(
            "Turn selected triangle pairs into quads.",
            &[],
        ).undo(),
    script("poly.spinQuads", Modeling, "Spins the shared edge of two selected polygons.")
        .headless(
            "Spin the edge shared by two selected quads.",
            &[],
        ).undo(),
    script("poly.align", Modeling, "Makes the selected polygons face the same way.")
        .headless(
            "Make the selected polygons face the same way as the first.",
            &[],
        ).undo(),
    script("poly.triple", Modeling, "Splits the selected polygons into triangles.")
        .headless(
            "Triangulate the selected polygons.",
            &[],
        ).undo(),
    script("operandi.solidSketch.strip", Modeling, "Removes the Solid Sketch data from the selected meshes, leaving plain polygons the tool can't pick up again.")
        .operandi()
        .undo(),
    script("poly.flip", Modeling, "Reverses the winding of the selected polygons.")
        .headless(
            "Flip the selected polygons' facing.",
            &[],
        ).undo(),
    script("poly.remove", Modeling, "Removes the selected polygons, keeping their vertices.")
        .headless(
            "Remove the selected polygons.",
            &[],
        ).undo(),
    script("poly.merge", Modeling, "Merges each connected region of selected polygons into one.")
        .headless(
            "Merge the selected polygons into one.",
            &[],
        ).undo(),
    script("poly.collapse", Modeling, "Collapses each selected region to a point.")
        .headless(
            "Collapse each selected polygon to a point.",
            &[],
        ).undo(),
    script("poly.make", Modeling, "Builds one quad from two selected boundary edges.")
        .headless(
            "Make a polygon from the selected vertices, in selection order.",
            &[],
        )
        .args(&[arg("type", ArgType::Choice(&["auto"]), N)])
        .undo()
        .example("poly.make auto"),
    // Curve tab commands (CURVE-COMMANDS-2D-SHAPES v1, curve_commands.rs).
    script("poly.makeCurveOpen", Modeling, "Makes an open Curve through the selected vertices, in selection order (CC-01).")
        .headless("Make an open Curve through the selected vertices in selection order; Polygon mode with the curve selected after.", &[])
        .undo().operandi(),
    script("poly.makeCurveClosed", Modeling, "Makes a closed Curve through the selected vertices, in selection order (CC-01).")
        .headless("Make a closed Curve through the selected vertices in selection order; Polygon mode with the curve selected after.", &[])
        .undo().operandi(),
    script("poly.makeCurveFill", Modeling, "Adds a flat Curve Fill polygon to each selected closed curve, keeping the curve (CC-10).")
        .headless("Add a Curve Fill polygon to each selected closed curve.", &[])
        .undo().operandi(),
    script("curve.winding", Modeling, "Makes the selected closed curves run the same way round (CC-20).")
        .headless(
            "Align the selected curves' winding: counter-clockwise seen from the automatic axis (clockwise with reverse).",
            &[
                jopt("merge", "boolean", "Join open curves that share end points first (default true)."),
                jopt("reverse", "boolean", "Wind the other way (default false)."),
            ],
        )
        .args(&[opt("merge", BOOL, N, Some("true")), opt("reverse", BOOL, N, Some("false"))])
        .undo().operandi(),
    // Batch 5 commands (generator_ops.rs; names provisional, GO-Q5).
    script("operandi.uv.rectangle", Modeling, "Straightens quad-grid UV islands into exact rectangles (GO-20).")
        .headless(
            "UV Rectangle on the selected polygons (all when none): each island that is a full grid of quads becomes a grid of rectangles.",
            &[
                jopt("name", "string", "UV map (default Texture)."),
                jopt("pack", "boolean", "Pack the islands into 0..1 after (default false)."),
                jopt("gaps", "number", "Pack gap, 0-1 (default 0.2)."),
                jopt("uniformX", "boolean", "Equal spans along U (default false)."),
                jopt("uniformY", "boolean", "Equal spans along V (default false)."),
                jopt("orient", "boolean", "Line each island up with U (default false)."),
            ],
        )
        .undo().operandi(),
    // Hands-on curve commands (curve_convert.rs, CC-40..64; names provisional).
    script("operandi.topology.tool", Tools, "Starts a Topology tab tool with Constrain to Background on (and snapping, for the drawing tools) (TB-11).")
        .args(&[opt("tool", TEXT, N, None)])
        .ui().operandi(),
    script("curve.boolean", Modeling, "Slices, unites, intersects or subtracts the selected closed curves with the other curves (CB-xx); bare opens its dialog.")
        .args(&[
            opt("operation", TEXT, N, Some("slice")),
            opt("axis", TEXT, N, Some("auto")),
            opt("clip", BOOL, N, Some("false")),
            opt("merge", BOOL, N, Some("true")),
        ])
        .undo().operandi(),
    script("curve.convert", Modeling, "Converts the selected curves to another curve type in a new item Converted_<type>, then edits it with that curve tool (CC-50).")
        .args(&[opt("from", TEXT, N, None), opt("to", TEXT, N, None)])
        .undo().operandi(),
    script("ppi.curveOffset", Modeling, "Adds an offset copy of the selected curves (CC-40); without offset: opens Enter Offset Amount.")
        .args(&[
            opt("closed", BOOL, N, Some("true")),
            opt("trio", BOOL, N, Some("false")),
            opt("offset", FLOAT, Unit::Distance, None),
        ])
        .undo().operandi(),
    script("bezier.tangent", Modeling, "Sharpens, smooths or evens the handles of the selected Bezier knots (CC-60); bare opens its dialog.")
        .args(&[
            opt("operation", TEXT, N, Some("sharp")),
            opt("tangent", TEXT, N, Some("both")),
            opt("setLength", BOOL, N, Some("false")),
            opt("length", FLOAT, Unit::Distance, Some("1")),
        ])
        .undo().operandi(),
    script("operandi.dicer", Modeling, "Smashes the active mesh into chunks with random planes (Dicer).")
        .headless(
            "Dicer: cut the active (closed) mesh with random planes into closed chunks; the same seed gives the same chunks.",
            &[
                jopt("seed", "integer", "Random Seed (default 40)."),
                jopt("planes", "integer", "Max. Cutting Planes, 1-16 (default 6)."),
                jopt("gap", "number", "Space between chunks (default 0.001)."),
                jopt("innerMaterial", "string", "Material of the new inside faces."),
                jopt("scaleX", "number", "Fragment Scale X (default 1)."),
                jopt("scaleY", "number", "Fragment Scale Y (default 1)."),
                jopt("scaleZ", "number", "Fragment Scale Z (default 1)."),
            ],
        )
        .undo().operandi(),
    script("operandi.vertMap.transfer", Modeling, "Copies a vertex map from the background mesh by sampling its surface (GO-30).")
        .headless(
            "Transfer Vertex Map from the first shown background mesh to the active mesh's selected points (all when none), sampling the source surface (values interpolate across its polygons).",
            &[
                jopt("source", "string", "The map's name (Position needs none)."),
                jopt("type", "string", "position, weight, uv, morph, rgb or rgba (default position)."),
                jopt("space", "string", "world (the items' placements count, default) or local (GO-33)."),
                jopt("method", "string", "distance (nearest spot, default) or raycast (along each point's normal; misses get no value, GO-34)."),
            ],
        )
        .undo().operandi(),
    script("curves.setColor", View, "Sets the display colour of curves in the viewports (CC-30): a view setting, not saved in the scene.")
        .args(&[opt("override", BOOL, N, Some("true")), opt("color", TEXT, N, None)])
        .ui().operandi(),
    script("vert.set", Modeling, "Gives every affected point one coordinate; bare opens Set Vertex Position.")
        .headless(
            "Set Position: give every selected point (or the points of the selected edges or polygons) one coordinate; one undo step.",
            &[
                jreq("axis", "string", "x, y or z."),
                jreq("value", "number", "Metres."),
                jopt("workPlane", "boolean", "Along the work plane's axis from its centre (default false)."),
                jopt("worldSpace", "boolean", "A world coordinate (default false)."),
            ],
        )
        .args(&[
            arg("axis", ArgType::Choice(&["x", "y", "z"]), N),
            arg("value", FLOAT, Unit::Distance),
            opt("workPlane", BOOL, N, Some("false")),
            opt("worldSpace", BOOL, N, Some("false")),
        ])
        .undo()
        .example("vert.set axis:y value:0"),
    script("poly.subdivide", Modeling, "Subdivides the selected polygons once; bare opens the options.")
        .headless(
            "Subdivide the selected polygons (all when none), one undo step (BRQ-10). Options not given take the Subdivide dialog's Reset values (Catmull-Clark, fractal 0, max angle 89.5°, Smooth All), not the last used.",
            &[
            jopt("type", "string", "Method: \"ccsds\" (Catmull-Clark, the default; the SDS Subdivide button), \"flat\" (Faceted), \"smooth\" or \"sds\" (the dialog's SDS Subdivide)."),
            jopt("fractal", "number", "Fractal, 0-1 (default 0)."),
            jopt("maxangle", "number", "Max Smoothing Angle, radians (default 89.5°)."),
            jopt("linear", "boolean", "Linear UVs (default false)."),
            jopt("bound", "string", "Boundary rule (Catmull-Clark): \"smooth\" (Smooth All, default), \"crease\" (Crease All) or \"creaseEdges\"."),
            jopt("noEdgeWeight", "boolean", "Ignore edge weights (default false)."),
            jopt("iterations", "integer", "How many times to subdivide, 1-8 (default 1; an Operandi extra: the button's repeat count)."),
        ],
        )
        .args(&[
            arg("type", ArgType::Choice(&["ccsds", "flat", "smooth", "sds"]), N),
            opt("fractal", FLOAT, Unit::Percent, None),
            opt("maxangle", FLOAT, Unit::Angle, None),
            opt("linear", BOOL, N, None),
            opt("bound", ArgType::Choice(&["smooth", "crease", "creaseEdges"]), N, None),
            opt("noEdgeWeight", BOOL, N, None),
        ])
        .undo()
        .example("poly.subdivide flat"),
    script("poly.unsubdiv", Modeling, "Merges groups of four quads back into one; bare opens the dialog.")
        .headless(
            "UnSubdivide: merge groups of four quads back into one, one undo step (BRQ-10). Options not given take the dialog's Reset values (iteration 1, uv and material hints on).",
            &[
            jopt("iteration", "integer", "Iterations, 0-16 (default 1)."),
            jopt("flat", "boolean", "Flat (default false)."),
            jopt("uv", "boolean", "UV hint (default true)."),
            jopt("material", "boolean", "Material hint (default true)."),
            jopt("part", "boolean", "Part hint (default false)."),
            jopt("smoothingGroup", "boolean", "Smoothing group hint (default false)."),
            jopt("selectionSet", "boolean", "Selection set hint (default false)."),
            jopt("selectionSetName", "string", "The selection set the hint reads."),
        ],
        )
        .args(&[
            opt("iteration", INT, N, None),
            opt("flat", BOOL, N, None),
            opt("uv", BOOL, N, None),
            opt("material", BOOL, N, None),
            opt("part", BOOL, N, None),
            opt("smoothingGroup", BOOL, N, None),
            opt("selectionSet", BOOL, N, None),
            opt("selectionSetName", TEXT, N, None),
        ])
        .undo()
        .example("poly.unsubdiv iteration:1"),
    script("tool.handleStyle", Tools, "Sets how tool handles draw: invisible, basic, advanced or default (basic).")
        .args(&[arg("style", ArgType::Choice(&["invisible", "basic", "advanced", "default"]), N)])
        .ui()
        .example("tool.handleStyle advanced"),
    script("scale.absolute", Modeling, "Absolute Scaling: Grab Size (mode:grab) reads the selection's size; uniform, explicit and reference scale it to typed sizes. Values left out come from the Absolute Scaling window.")
        .headless(
            "Absolute Scaling: scale the selection (or the active mesh) to typed sizes, one undo step. mode grab only reads: it answers the selection's size and changes nothing. Values not given take the Absolute Scaling window's defaults (0 sizes leave that axis alone, centre Center).",
            &[
            jreq(
                "mode",
                "string",
                "\"grab\", \"uniform\", \"explicit\" or \"reference\".",
            ),
            jopt(
                "absolute_size_uniform",
                "number",
                "uniform: the length, metres, the Uniform Axis becomes.",
            ),
            jopt(
                "absolute_size_uniform_pref",
                "string",
                "Uniform Axis: \"longestAxis\" (default), \"x\", \"y\" or \"z\".",
            ),
            jopt(
                "orient_to_selection",
                "boolean",
                "Measure along the selection's own sides; default false.",
            ),
            jopt(
                "absolute_size_x",
                "number",
                "explicit / reference: X size, metres (0 leaves X alone).",
            ),
            jopt(
                "absolute_size_y",
                "number",
                "explicit / reference: Y size, metres (0 leaves Y alone).",
            ),
            jopt(
                "absolute_size_z",
                "number",
                "explicit / reference: Z size, metres (0 leaves Z alone).",
            ),
            jopt(
                "absolute_size_center_x",
                "string",
                "What stays put along X: \"center\" (default), \"high\", \"low\" or \"origin\" (the item's origin).",
            ),
            jopt(
                "absolute_size_center_y",
                "string",
                "Along Y: \"center\", \"high\", \"low\" or \"origin\".",
            ),
            jopt(
                "absolute_size_center_z",
                "string",
                "Along Z: \"center\", \"high\", \"low\" or \"origin\".",
            ),
        ],
        )
        .args(&[
            arg("mode", ArgType::Choice(&["grab", "uniform", "explicit", "reference"]), N),
            opt("absolute_size_uniform", FLOAT, Unit::Distance, None),
            opt("absolute_size_uniform_pref", ArgType::Choice(&["longestAxis", "x", "y", "z"]), N, None),
            opt("orient_to_selection", BOOL, N, None),
            opt("absolute_size_x", FLOAT, Unit::Distance, None),
            opt("absolute_size_y", FLOAT, Unit::Distance, None),
            opt("absolute_size_z", FLOAT, Unit::Distance, None),
            opt("absolute_size_center_x", ArgType::Choice(&["center", "high", "low", "origin"]), N, None),
            opt("absolute_size_center_y", ArgType::Choice(&["center", "high", "low", "origin"]), N, None),
            opt("absolute_size_center_z", ArgType::Choice(&["center", "high", "low", "origin"]), N, None),
        ])
        .undo()
        .example("scale.absolute mode:explicit absolute_size_x:2"),
    script("vert.center", Modeling, "Centres the selection (or mesh) on the origin along the given axes.")
        .headless(
            "Center: move the selected points, edges or polygons (the whole mesh when none) so their world bounding-box centre is 0 on the given axes, one undo step (BRQ-10).",
            &[jreq("axis", "string", "\"all\", \"x\", \"y\", \"z\", \"xy\", \"yz\" or \"zx\".")],
        )
        .args(&[arg("axis", AXES, N)])
        .undo()
        .example("vert.center all"),
    script("poly.freeze", Modeling, "Freezes curves and subdivision surfaces into plain geometry; bare opens the Freeze dialog. face 0 = Faces, 1 = Polylines, 2 = 2 Point Polygons (2 is Operandi-only); drill, axis, disp, deform, morphSave and morphName are accepted and ignored.")
        .headless(
            "Freeze curves and subdivision surfaces of the active mesh into plain polygons (the selected polygons in Polygon mode, else all), one undo step. Arguments not given keep the last Freeze's values (defaults: faces, tess true, fixed true, 5°). Never opens the dialog here.",
            &[
            jopt(
                "face",
                "string",
                "Curves become 0 / \"faces\", 1 / \"polylines\" or 2 / \"twoPointPolygons\" (2 is Operandi's).",
            ),
            jopt(
                "tess",
                "boolean",
                "Freeze subdivision surfaces too (Tessellation); default true.",
            ),
            jopt(
                "fixed",
                "boolean",
                "Fixed spans per curve segment; false uses cangle instead.",
            ),
            jopt(
                "cangle",
                "number",
                "Curve Refinement Angle, radians (above 0, at most π), used when fixed is false.",
            ),
            jopt("drill", "boolean", "Accepted and ignored (not built yet)."),
            jopt("axis", "integer", "Accepted and ignored (not built yet)."),
            jopt("disp", "boolean", "Accepted and ignored (not built yet)."),
            jopt("deform", "boolean", "Accepted and ignored (not built yet)."),
            jopt(
                "morphSave",
                "boolean",
                "Accepted and ignored (not built yet).",
            ),
            jopt(
                "morphName",
                "string",
                "Accepted and ignored (not built yet).",
            ),
        ],
        )
        .args(&[
            // 0 and 1 are the reference's values; 2 (twoPointPolygons) is
            // an Operandi-only addition (registry thread, 2026-11-11).
            opt("face", ArgType::Choice(&["faces", "polylines", "twoPointPolygons"]), N, None),
            opt("drill", BOOL, N, None),
            opt("axis", INT, N, None),
            opt("disp", BOOL, N, None),
            opt("deform", BOOL, N, None),
            opt("tess", BOOL, N, None),
            opt("fixed", BOOL, N, None),
            opt("cangle", FLOAT, Unit::Angle, None),
            opt("morphSave", BOOL, N, None),
            opt("morphName", TEXT, N, None),
        ])
        .undo()
        .example("poly.freeze face:2 fixed:false cangle:5deg"),
    // ---------------------------------------------------------- edges
    script("edge.addLoop", Modeling, "Adds a loop across each selected edge's polygon band.")
        .headless(
            "Add an edge loop across the edge a–b.",
            &[
            jreq("a", "integer", "Edge start vertex index."),
            jreq("b", "integer", "Edge end vertex index."),
            jreq("t", "number", "Position along a→b, 0–1."),
        ],
        )
        .args(&[opt("position", FLOAT, Unit::Percent, Some("0.5"))])
        .undo(),
    script("edge.addPoint", Modeling, "Splits each selected edge with a new point.")
        .headless(
            "Add a point on the edge a–b.",
            &[
            jreq("a", "integer", "Edge start vertex index."),
            jreq("b", "integer", "Edge end vertex index."),
            jreq("t", "number", "Position along a→b, 0–1."),
        ],
        )
        .args(&[opt("position", FLOAT, Unit::Percent, Some("0.5"))])
        .undo(),
    script("edge.growQuads", Modeling, "Grow Quads on the selected edges.")
        .headless(
            "Grow quads from the selected boundary edges.",
            &[],
        ).undo(),
    script("edge.fillQuads", Modeling, "Fill Quads on the selected edges.")
        .headless(
            "Fill the hole bounded by the selected edges with quads.",
            &[],
        ).undo(),
    script("edge.split", Modeling, "Splits the mesh along the selected edges.")
        .headless(
            "Split the mesh along the selected edges.",
            &[],
        ).undo(),
    script("edge.splitNormals", Modeling, "Splits normals along the selected edges.").undo(),
    script("edge.spinQuads", Modeling, "Spins the selected edges.")
        .headless(
            "Spin the selected edges.",
            &[],
        ).undo(),
    script("edge.join", Modeling, "Joins the selected edges (average:true averages them).")
        .headless(
            "Join each selected edge's ends.",
            &[jopt(
            "average",
            "boolean",
            "At their average position (true) or not (false, default).",
        )],
        )
        .args(&[opt("average", BOOL, N, Some("false"))])
        .undo(),
    script("edge.collapse", Modeling, "Collapses the selected edges.")
        .headless(
            "Collapse the selected edges.",
            &[],
        ).undo(),
    script("edge.remove", Modeling, "Removes the selected edges.")
        .headless(
            "Remove the selected edges.",
            &[],
        ).undo(),
    script("edges.toCurve", Modeling, "Makes curves from the selected edges.").undo(),
    script("edge.unbevel", Modeling, "Unbevels the selected edges.")
        .headless(
            "Undo a bevel on the selected edges.",
            &[],
        ).undo(),
    script("edge.subdivide", Modeling, "Subdivides the selected edges.")
        .headless(
            "Subdivide the selected edges.",
            &[
            jopt(
                "count",
                "integer",
                "New points per edge; omit for the plain one-cut Subdivide.",
            ),
            jopt(
                "percent",
                "number",
                "With count 1: where the point goes along the edge, 0–1 (0.5 = middle).",
            ),
        ],
        ).undo(),
    // ---------------------------------------------------------- vertices
    script("vert.new", Modeling, "Adds one point at a world position; bare opens Create New Vertex.")
        .headless(
            "Add a vertex to the active mesh.",
            &[
            jopt(
                "position",
                "[x, y, z]",
                "Item-space position, metres (or give x, y and z).",
            ),
            jopt("x", "number", "Metres, with y and z."),
            jopt("y", "number", "Metres."),
            jopt("z", "number", "Metres."),
        ],
        )
        .args(&[
            arg("x", FLOAT, Unit::Distance),
            arg("y", FLOAT, Unit::Distance),
            arg("z", FLOAT, Unit::Distance),
        ])
        .undo()
        .example("vert.new 0 0 0"),
    script("vert.split", Modeling, "Gives each polygon its own copy of the selected points.")
        .headless(
            "Split the selected vertices.",
            &[],
        ).undo(),
    script("vert.splitEdge", Modeling, "Splits the selected points along edges.")
        .headless(
            "Split the edges at the selected vertices.",
            &[],
        ).operandi()
        .undo(),
    script("vert.align", Modeling, "Aligns the selected points.")
        .headless(
            "Align the selected vertices.",
            &[
            jopt(
                "axes",
                "[bool, bool, bool]",
                "Which of X, Y, Z to align; default all.",
            ),
            jopt("uniform", "boolean", "Uniform spacing; default false."),
        ],
        )
        .args(&[
            opt("axis", AXES, N, Some("all")),
            opt("uniform", BOOL, N, None),
        ])
        .undo(),
    script("vert.merge", Modeling, "Merges the selected points (auto or by fixed distance).")
        .headless(
            "Merge selected vertices closer than a distance.",
            &[jopt(
            "dist",
            "number",
            "Metres; default 0 (coincident only).",
        )],
        )
        .args(&[
            opt("range", ArgType::Choice(&["auto", "fixed"]), N, Some("auto")),
            opt("dist", FLOAT, Unit::Distance, None),
            opt("keep", BOOL, N, Some("false")),
        ])
        .undo(),
    script("vert.join", Modeling, "Joins the selected points into one.")
        .headless(
            "Join the selected vertices into one.",
            &[jopt(
            "average",
            "boolean",
            "At their average position (true) or the first (false, default).",
        )],
        )
        .args(&[opt("average", BOOL, N, None)])
        .undo(),
    script("vert.collapse", Modeling, "Collapses the selected points.")
        .headless(
            "Collapse the selected vertices.",
            &[],
        ).undo(),
    script("vert.remove", Modeling, "Removes the selected points.")
        .headless(
            "Remove the selected vertices (and their polygons).",
            &[],
        ).undo(),
    script("vert.removeEdge", Modeling, "Removes the selected points and their edges.")
        .headless(
            "Remove the selected vertices, keeping polygons.",
            &[],
        ).operandi()
        .undo(),
    script("delete", Modeling, "Deletes the selection of the current mode: points take their polygons, edges dissolve, polygons go, leaving no unused points; items are deleted.")
        .headless(
            "Delete the selection of the current mode: points take their polygons, edges dissolve, polygons go; unused points go too; items are deleted (the Delete and Backspace keys).",
            &[],
        )
        .undo(),
    script("vert.reorder", Modeling, "Reorders point indices; bare opens the dialog.")
        .args(&[arg("mode", ArgType::Choice(&["swap", "top", "bottom"]), N)])
        .undo()
        .example("vert.reorder mode:swap"),
    script("vert.syncOrder", Modeling, "Syncs point order to polygon order.").undo(),
    script("vert.weld", Modeling, "Welds the selected points onto one point index.")
        .operandi()
        .headless(
            "Weld vertices onto a target vertex.",
            &[
            jreq("sources", "array of integers", "Vertex indices to weld."),
            jreq("target", "integer", "Vertex index they move to."),
        ],
        )
        .args(&[arg("target", INT, N)])
        .undo()
        .example("vert.weld target:0"),
    // Viewport Properties rows (viewport_properties.rs VIEW3D); bgmesh:1
    // is the Inactive Meshes group (provisional).
    script("view3d.controls", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.controls ?"),
    script("view3d.showHUD", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showHUD ?"),
    script("view3d.showWireframeItemMode", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showWireframeItemMode ?"),
    script("view3d.bgEnvironment", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.bgEnvironment ?"),
    script("view3d.showLights", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showLights ?"),
    script("view3d.showCameras", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showCameras ?"),
    script("view3d.showMeshes", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showMeshes ?"),
    script("view3d.showGrid", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showGrid ?"),
    script("view3d.showWorkPlane", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showWorkPlane ?"),
    script("view3d.fadedGrid", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.fadedGrid ?"),
    script("view3d.fadedGridAxes", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.fadedGridAxes ?"),
    script("view3d.showLocators", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showLocators ?"),
    script("view3d.showTextureLocators", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showTextureLocators ?"),
    script("view3d.showLightIllumination", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showLightIllumination ?"),
    script("view3d.showSelections", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showSelections ?"),
    script("view3d.selectedNormals", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.selectedNormals ?"),
    script("view3d.fillSelected", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.fillSelected ?"),
    script("view3d.outlineSelected", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.outlineSelected ?"),
    script("view3d.showBackdrop", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showBackdrop ?"),
    script("view3d.useTrackball", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.useTrackball ?"),
    script("view3d.orbitSelected", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.orbitSelected ?"),
    script("view3d.gnzAmbientIntensity", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.gnzAmbientIntensity ?"),
    script("operandi.view3d.showSymmetryPlane", ViewportOptions, "Show Symmetry Plane of the active view: the mirror plane's outline while Symmetry is on (off hides only the outline); ? reads it.")
        .operandi()
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("operandi.view3d.showSymmetryPlane false"),
    script("operandi.view3d.matcap", ViewportOptions, "The active view's matcap (the Matcap style): a built-in name, or custom with image:<path>; name:custom without image: opens the file chooser; all:true sets it in every 3D view, each switched to the Matcap style (the Preset Browser's Use in All 3D Views); ? reads it.")
        .operandi()
        .args(&[
            arg("name", TEXT, N),
            opt("image", TEXT, N, None),
            opt("bgmesh", INT, N, Some("0")),
            opt("all", BOOL, N, Some("false")),
        ])
        .query()
        .ui()
        .example("operandi.view3d.matcap name:fullmetal")
        .example("operandi.view3d.matcap ?"),
    script("view3d.gnzAmbientColor", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.gnzAmbientColor ?"),
    script("view3d.indPointSize", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.indPointSize ?"),
    script("view3d.pointSize", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.pointSize ?"),
    script("view3d.showSelectionRollover", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showSelectionRollover ?"),
    script("view3d.meshSmoothing", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.meshSmoothing ?"),
    script("view3d.showGnzShadows", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showGnzShadows ?"),
    script("view3d.showUVBounds", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showUVBounds ?"),
    // HT-5/HT-6: the view header's Weightmaps and UV Distortion toggles.
    script("view3d.showweights", ViewportOptions, "Show Weightmaps in the active view: meshes tinted by the selected weight map; ? reads it.")
        // In the name map since HT-Q5 (state only); bgmesh is accepted like
        // the other view3d settings and changes nothing.
        .args(&[arg("state", BOOL, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showweights true"),
    script("view.topologyMode", ViewportOptions, "Topology Mode in the active view: the active meshes draw over the background meshes, see-through (missing-tools-g3 MT7); bare toggles.")
        .operandi()
        .args(&[opt("state", BOOL, N, None)])
        .ui()
        .example("view.topologyMode on"),
    script("view3d.showDistortion", ViewportOptions, "Show UV Distortion in the active view: polygons tinted by UV stretch; ? reads it.")
        // In the name map since HT-Q5 (enabled, bgmesh). Provisional:
        // bgmesh:1 is accepted but treated as 0 (the toggle is one per view,
        // not split into active/inactive meshes; HT-Q5).
        .args(&[arg("enabled", BOOL, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.showDistortion true"),
    script("view3d.sameAsActive", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.sameAsActive ?"),
    script("view3d.inactiveInvisible", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.inactiveInvisible ?"),
    script("view3d.projection", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.projection ?"),
    script("view3d.shadingStyle", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.shadingStyle ?"),
    script("view3d.wireframeOverlay", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.wireframeOverlay ?"),
    script("view3d.wireframeAlpha", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.wireframeAlpha ?"),
    script("view3d.wireframeViewColor", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.wireframeViewColor ?"),
    script("view3d.TransparencyOverride", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.TransparencyOverride ?"),
    script("view3d.TransparencyOverrideSolidColorFront", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.TransparencyOverrideSolidColorFront ?"),
    script("view3d.TransparencyOverrideSolidAlphaFront", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.TransparencyOverrideSolidAlphaFront ?"),
    script("view3d.TransparencyOverrideSolidColorBack", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.TransparencyOverrideSolidColorBack ?"),
    script("view3d.TransparencyOverrideSolidAlphaBack", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.TransparencyOverrideSolidAlphaBack ?"),
    script("view3d.TransparencyOverrideOverlayMode", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.TransparencyOverrideOverlayMode ?"),
    script("view3d.TransparencyOverrideFlatShade", ViewportOptions, "Viewport Properties setting of the active view; ? reads it.")
        .args(&[arg("value", TEXT, N), opt("bgmesh", INT, N, Some("0"))])
        .query()
        .ui()
        .example("view3d.TransparencyOverrideFlatShade ?"),
    script("view3d.showVertices", ViewportOptions, "Shows or hides vertices in the views.")
        .args(&[opt("state", ON_OFF_QUERY, N, None)])
        .query()
        .ui(),
    // ---------------------------------------------------------- items
    script("item.create", Items, "Adds one item after the selected item.")
        .headless(
            "Add an item from the Items list's Add menu (camera, a light, a mesh...), alone selected.",
            &[jreq(
            "type",
            "string",
            "Item type key; see the `types` list in command_help item.create.",
        )],
        )
        .args(&[arg("type", ITEM_TYPES, N)])
        .undo()
        .example("item.create mesh"),
    script("item.pattern", Items, "Selects, unselects or hides items whose names match a pattern.")
        .operandi()
        .headless(
            "Select, unselect or hide the items whose names match a pattern (Items list header).",
            &[
            jreq("action", "string", "\"select\", \"unselect\" or \"hide\"."),
            jopt(
                "pattern",
                "string",
                "Name pattern with * and ? wildcards; default \"*\".",
            ),
            jopt(
                "limit_to_children",
                "boolean",
                "Only descendants of the selected items; default false.",
            ),
        ],
        )
        .args(&[
            arg("action", ArgType::Choice(&["select", "unselect", "hide"]), N),
            opt("pattern", TEXT, N, Some("*")),
        ])
        .undo()
        .example("item.pattern select"),
    script("item.editorColor", Items, "Sets the selected items' Editor Color; bare reports it.")
        .operandi()
        .args(&[opt("color", TEXT, N, None)])
        .undo()
        .query(),
    // The MCP bridge's commands that had no typed form (extensibility
    // stage 2): their arguments are the bridge's JSON ones.
    script("mesh.new", Items, "Adds an empty mesh item and makes it active.")
        .operandi()
        .undo()
        .headless(
            "Add an empty mesh item and make it active.",
            &[jopt(
            "name",
            "string",
            "Name for the new mesh; default \"Mesh\" (numbered when taken).",
        )],
        ),
    // The map's Select Item (PD-Q12); item.select is its deprecated alias
    // (command_names::COMMAND_RENAMES).
    script("select.item", Selection, "Makes an item the active item (none clears the item selection); mode and mask are accepted, not used yet.")
        .during_transform_tool()
        .undo()
        .headless(
            "Make an item the active (selected) item (the map's Select Item; was item.select, still accepted). Doesn't change the component mode; the select tool's item argument does the same in any mode.",
            &[
            jopt(
                "item",
                "integer",
                "Item id; omit to clear the item selection.",
            ),
            jopt(
                "mode",
                "integer",
                "Accepted, not used yet (the map's mode).",
            ),
            jopt("mask", "string", "Accepted, not used yet (the map's mask)."),
        ],
        )
        .example("select.item item:2"),
    // N11 (Jason, 2026-11-16): Ctrl+G groups the selected items under a
    // new Group Locator (GL-04).
    action("Parent to Group Locator", Items, "Parents the selected items to a new Group Locator, in place.")
        .undo()
        .operandi(),
    script("item.delete", Items, "Deletes the active item; children:delete|root deletes every selected item.")
        .undo()
        .headless(
            "Delete the active item; with `children`, every selected item.",
            &[
            jopt(
                "children",
                "string",
                "\"delete\" deletes their child items too; \"root\" moves them to the scene root in place (default).",
            ),
            jopt(
                "item",
                "string",
                "A mesh operation to delete instead, written op:<item>:<op> (Mesh Stack).",
            ),
        ],
        ),
    script("item.name", Items, "Renames the active item.")
        .undo()
        .headless(
            "Rename the active item, or a shader-tree layer with layer.",
            &[
            jreq("name", "string", "New name, 1–1024 bytes."),
            jopt(
                "layer",
                "integer",
                "A shader-tree layer id to rename instead (Operandi's own argument).",
            ),
        ],
        )
        .example("item.name Base"),
    script("item.visibility", Items, "Toggles an item's visibility.")
        .operandi()
        .undo()
        .headless(
            "Toggle an item's visibility.",
            &[JSON_ITEM],
        ),
    script("item.parent", Items, "Parents an item to another, or to the scene root.")
        .undo()
        .headless(
            "Parent an item to another (or to the scene root).",
            &[
            JSON_ITEM,
            jopt(
                "parent",
                "integer",
                "New parent's id; omit for the scene root.",
            ),
            jopt(
                "keep_position",
                "boolean",
                "Keep the item where it is in the world; default true.",
            ),
            jopt(
                "position",
                "integer",
                "With item:op:<item>:<op>, the op's new place in evaluation order (0 runs first).",
            ),
        ],
        ),
    // MeshOps (meshops_editor.rs), reference names from the registry
    // thread (MESHOPS-NAMES 2026-11-11). item.delete, item.parent,
    // item.channel and select.item take an op as item:op:<item>:<op>.
    script("meshop.create", Modeling, "Adds a mesh operation to the selected mesh, above the selected row of its stack.")
        .undo()
        .headless(
            "Add a mesh operation to the selected mesh, directly above the selected row of its Mesh Operations list (on top when none is selected). Polygons selected on the Base Mesh become its Select by Index.",
            &[
            jreq(
                "type",
                "string",
                "Op type, such as prim.cube.item, poly.bevel.item, pmodel.meshmerge (Merge Meshes), boolean.meshop.item, solidDrill.meshop.item or axisDrill.meshop.item; command_help meshop.create lists every type (types).",
            ),
            jopt("mode", "integer", "Accepted, not used yet."),
            jopt("category", "string", "Accepted, not used yet."),
            jopt("applyDefaultPreset", "boolean", "Accepted, not used yet."),
        ],
        )
        .example("meshop.create type:poly.bevel.item"),
    script("deformer.enable", Modeling, "Turns a mesh operation on or off.")
        .undo()
        .headless(
            "Turn a mesh operation on or off (its eye); off passes its input through.",
            &[
            jopt(
                "deformer",
                "string",
                "The op, op:<item>:<op>; default the selected one.",
            ),
            jopt("enable", "boolean", "Default true."),
        ],
        ),
    script("deformer.freeze", Modeling, "Freezes a mesh operation and those below it into the Base Mesh; duplicate:true makes a new mesh item.")
        .undo()
        .headless(
            "Freeze a mesh operation and every op below it into the Base Mesh; the ops above stay live. With duplicate, the frozen copy is a new mesh item.",
            &[
            jopt(
                "duplicate",
                "boolean",
                "Freeze into a new mesh item; default false.",
            ),
            jopt(
                "deformer",
                "string",
                "The op, op:<item>:<op>; default the selected one.",
            ),
            jopt("mesh", "integer", "Accepted, not used yet."),
        ],
        ),
    script("mesh.freeze", Modeling, "Freezes the selected mesh's whole Mesh Stack.")
        .undo()
        .headless(
            "Freeze the selected mesh's whole Mesh Stack into a plain mesh.",
            &[],
        ),
    script("mirror.clone.create", Modeling, "Adds a Mirror mesh operation to the selected mesh.")
        .undo()
        .headless(
            "Add a Mirror mesh operation (a Clone Effector with a pmodel.mirror generator) to the selected mesh, as meshop.create does.",
            &[],
        ),
    script("array.clone.create", Modeling, "Adds an Array mesh operation to the selected mesh.")
        .undo()
        .headless(
            "Add an Array mesh operation (a Clone Effector with a pmodel.array generator) to the selected mesh, as meshop.create does.",
            &[],
        ),
    script("deformer.selectBaseMesh", Modeling, "Edit Base Mesh on or off for a mesh with a Mesh Stack.")
        .headless(
            "Edit Base Mesh on or off: direct tools then edit the mesh's Base Mesh, with the stack's result drawn as a ghost.",
            &[JSON_ITEM, jopt("select", "boolean", "Default true.")],
        ).undo(),
    script("selop.indexcreate", Selection, "Gives the selected mesh operation a Select by Index of the selected polygons.")
        .undo()
        .headless(
            "Give the selected mesh operation a Select by Index of the polygons selected on its Base Mesh.",
            &[],
        ),
    script("selop.selection", Selection, "Sets, clears, adds to or removes from a mesh operation's Select by Index.")
        .undo()
        .headless(
            "Edit a mesh operation's Select by Index from the polygons selected on its Base Mesh.",
            &[
            jopt(
                "mode",
                "string",
                "set (default), clear, add or remove; or 0 to 3.",
            ),
            jopt(
                "item",
                "string",
                "The op, op:<item>:<op>; default the selected one.",
            ),
        ],
        ),
    script("selop.create", Selection, "Adds a selection operation to a mesh operation's Selection input.")
        .operandi()
        .undo()
        .headless(
            "Add a selection operation (index.selop.item, selbyrange.selop.item, pattern.selop.item, previous.selop, operandi.selset.selop) on top of a mesh operation's Selection input.",
            &[
                jreq("type", "string", "The selection operation's type."),
                jopt("item", "string", "The op, op:<item>:<op>; default the selected one."),
            ],
        ),
    script("selop.edit", Selection, "Changes or removes one of a mesh operation's selection operations.")
        .operandi()
        .undo()
        .headless(
            "Change values of, or remove, a mesh operation's selection operation (0 = bottom).",
            &[
                jreq("index", "integer", "Which selection operation, from the bottom (0)."),
                jopt("values", "object", "Values to set: enable, invert, blendmode (override, add, subtract, multiply), and the kind's own (index, start, end, step, pattern, source, name)."),
                jopt("remove", "boolean", "Remove it instead."),
                jopt("item", "string", "The op, op:<item>:<op>; default the selected one."),
            ],
        ),
    script("item.drop", Items, "Runs a Drop Action from the sources onto a destination item.")
        .operandi()
        .undo()
        .headless(
            "Run a Drop Action, as when dragging items onto another in the Items list.",
            &[
            jreq(
                "action",
                "string",
                "Drop action key; see the `actions` list in command_help item.drop.",
            ),
            jreq("destination", "integer", "The item dropped onto."),
            jopt(
                "sources",
                "array of integers",
                "Items dropped; default the item selection in order.",
            ),
            jopt(
                "ray_origin",
                "[x, y, z]",
                "Place / Place and Align only: world-space ray start, metres.",
            ),
            jopt(
                "ray_direction",
                "[x, y, z]",
                "Place / Place and Align only: world-space ray direction.",
            ),
        ],
        )
        .example("item.drop action:match_position destination:1"),
    script("item.transform", Items, "Sets an item's translation, rotation (radians) or scale.")
        .during_transform_tool()
        .operandi()
        .undo()
        .headless(
            "Set an item's transform (one undo step). Values are absolute; channels not given keep their value.",
            &[
            JSON_ITEM,
            jopt(
                "translation",
                "[x, y, z]",
                "Position relative to the parent, metres, absolute.",
            ),
            jopt(
                "rotation",
                "[x, y, z]",
                "Euler angles, radians, absolute (X then Y then Z, as scene_info reports).",
            ),
            jopt(
                "scale",
                "[x, y, z]",
                "Scale factors, absolute (1 = unscaled).",
            ),
        ],
        ),
    script("symmetry.set", Tools, "Sets symmetry: axis off|x|y|z and offset.")
        .operandi()
        .undo()
        .headless(
            "Set modelling symmetry.",
            &[
            jopt(
                "axis",
                "string",
                "\"off\" (default), \"x\", \"y\" or \"z\".",
            ),
            jopt(
                "offset",
                "number",
                "Symmetry plane offset along the axis, metres; default 0.",
            ),
        ],
        ),
    script("select.symmetrical", Selection, "Selects the symmetrical counterparts of the selection.")
        .undo()
        .headless(
            "Add the symmetrical counterparts of the selection.",
            &[],
        ),
    script("pref.value", Preferences, "Sets (or with ? reads) one preference by name; saves it like the Preferences window. Names are Operandi's own for now.")
        .args(&[arg("name", TEXT, N), arg("value", TEXT, N)])
        .query()
        .example("pref.value input.trackball ?"),
    // Animation (BEHAVIOR-TIME-KEYS, BEHAVIOR-TIMELINE; anim_commands.rs).
    // Operandi names (no reference names in the map; UIC-165 provisional).
    script("operandi.anim.query", Items, "Reads the scene's time settings, playback, pending edits, markers and animated channels with their keys and values (UIC-163), as JSON.")
        .on_editor(
            "Read animation state as JSON: current frame, fps, scene and current range, time display, Auto Key, snap, real time, end behavior, key time, playing, pending edits, markers, and each animated channel's keys (frame, value, slopes), value now and values at frames.",
            &[
            jopt("channels", "string", "Only these channels, item:name,..."),
            jopt("item", "integer", "Only this item's channels."),
            jopt("frames", "string", "Frames to read each curve at, comma separated."),
        ],
            crate::anim_commands::run,
        )
        .query()
        .quiet()
        .operandi()
        .example("operandi.anim.query frames:0,12,24"),
    script("operandi.render.frameRange", Render, "Renders a frame range with Cycles, one numbered image per frame, into the project's renders folder (TL-70); needs the running app.")
        .on_editor(
            "Render frames start..end (default the current range) with Cycles into folder (default the project's renders/) as <name>_0000.png; needs the running app. operandi.anim.query reports progress (job).",
            &[
            jopt("start", "number", "First frame (default the current range's)."),
            jopt("end", "number", "Last frame (default the current range's)."),
            jopt("range", "string", "current (default) or scene: which range start and end default to."),
            jopt("folder", "string", "Folder to write to."),
            jopt("name", "string", "File name before the frame number (default Render)."),
            jopt("samples", "integer", "Samples per pixel (default the render settings')."),
            jopt("width", "integer", "Width in pixels (with height)."),
            jopt("height", "integer", "Height in pixels (with width)."),
            jopt("camera", "integer", "Camera item (default the render camera)."),
        ],
            crate::anim_commands::run,
        )
        .side_effect()
        .operandi()
        .example("operandi.render.frameRange start:0 end:24"),
    script("operandi.render.cancelRange", Render, "Stops a running frame render or playblast (TL-70).")
        .on_editor(
            "Stop the running frame range render or playblast.",
            &[],
            crate::anim_commands::run,
        )
        .side_effect()
        .operandi()
        .example("operandi.render.cancelRange"),
    script("operandi.view.playblast", Render, "Records the active 3D view over a frame range, HUD hidden, to a movie or an image sequence and opens it (TL-71); needs the running app.")
        .on_editor(
            "Record the active 3D view for frames start..end (default the current range) at scale 1, 0.5 or 0.25 to an image sequence in folder (default the project's renders/), then open it unless open is false; needs the running app.",
            &[
            jopt("start", "number", "First frame."),
            jopt("end", "number", "Last frame."),
            jopt("range", "string", "current (default) or scene."),
            jopt("scale", "number", "1, 0.5 or 0.25 of the view's size (default 1)."),
            jopt("folder", "string", "Folder to write to."),
            jopt("name", "string", "File name before the frame number (default Playblast)."),
            jopt("open", "boolean", "Open the result in the system player after (default true)."),
            jopt("format", "string", "movie (default: an .mp4, through ffmpeg when it is installed; else the images) or images (a PNG sequence)."),
        ],
            crate::anim_commands::run,
        )
        .side_effect()
        .operandi()
        .example("operandi.view.playblast scale:0.5"),
    // Graph editor (BEHAVIOR-GRAPH-EDITOR; anim_graph.rs, anim_commands.rs).
    script("operandi.key.offset", Items, "Moves the selected keys by frames and by a value at once, one undo step (GE-40/45: a graph editor drag's result).")
        .on_editor(
            "Move the selected keys by frames in time and by value, one undo step (GE-40/45).",
            &[
            jopt("frames", "number", "Frames to move by (default 0)."),
            jopt("value", "number", "Value to add (default 0)."),
            jopt("link", "boolean", "Related keys (X/Y/Z, R/G/B) at the same times move in time too (GE-42)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.offset frames:2 value:0.5"),
    script("operandi.key.setValue", Items, "Sets, adds to or multiplies the selected keys' values (GE-71 Set Value).")
        .on_editor(
            "Sets, adds to or multiplies the selected keys' values (GE-71 Set Value).",
            &[
            jreq("value", "number", "The value."),
            jopt("mode", "string", "set (default), add or multiply."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.setValue value:2"),
    script("operandi.key.scaleValue", Items, "Scales the selected keys' values around a value (GE-43).")
        .on_editor(
            "Scales the selected keys' values around a value (GE-43).",
            &[
            jreq("factor", "number", "Factor."),
            jopt("pivot", "number", "The value to scale around (default 0)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.scaleValue factor:1.5"),
    script("operandi.key.setFrame", Items, "Puts the selected keys at a frame (the earliest there, spacing kept), or moves them by frames (GE-71 Set Frame).")
        .on_editor(
            "Puts the selected keys at a frame (the earliest there, spacing kept), or moves them by frames (GE-71 Set Frame).",
            &[
            jreq("frame", "number", "Frame (or frames to move by with relative)."),
            jopt("relative", "boolean", "Move by frame instead of to it."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.setFrame frame:10"),
    script("operandi.key.handle", Items, "Sets the selected keys' in or out handle: slope (value per frame) and/or weight; that side becomes Manual, a unified key's other side follows the slope (GE-50/53).")
        .on_editor(
            "Sets the selected keys' in or out handle: slope (value per frame) and/or weight; that side becomes Manual, a unified key's other side follows the slope (GE-50/53).",
            &[
            jopt("side", "string", "in or out (default out)."),
            jopt("slope", "number", "Slope, value per frame."),
            jopt("weight", "number", "Weight 0.01-3 (1 is the plain length)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.handle side:out slope:0.1"),
    script("operandi.key.break", Items, "Breaks the selected keys' handles so each side moves alone (GE-51).")
        .on_editor(
            "Breaks the selected keys' handles so each side moves alone (GE-51).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.break"),
    script("operandi.key.unify", Items, "Unifies the selected keys' handles into one line (GE-51).")
        .on_editor(
            "Unifies the selected keys' handles into one line (GE-51).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.unify"),
    script("operandi.key.autoHandles", Items, "Puts the selected keys' handle weights back to Auto; with slopes:true their slopes too (GE-51/52).")
        .on_editor(
            "Puts the selected keys' handle weights back to Auto; with slopes:true their slopes too (GE-51/52).",
            &[
            jopt("slopes", "boolean", "Slopes back to Auto as well."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.autoHandles slopes:true"),
    script("operandi.key.add", Items, "Adds a key on a channel at a time and value (GE-31); the curve's own value when value is left out.")
        .on_editor(
            "Adds a key on a channel at a time and value (GE-31); the curve's own value when value is left out.",
            &[
            jreq("channel", "string", "item:name."),
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
            jopt("value", "number", "The key's value."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.add channel:1:pos.Y frame:6 value:1"),
    script("operandi.key.keepShape", Items, "Keys channels at a time without changing their curves (GE-32 Key Keeping Shape).")
        .on_editor(
            "Keys channels at a time without changing their curves (GE-32 Key Keeping Shape).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.keepShape channels:1:pos.Y frame:6"),
    script("operandi.curve.behavior", Items, "Sets curves' behaviour before the first key and/or after the last (GE-61).")
        .on_editor(
            "Sets curves' behaviour before the first key and/or after the last (GE-61).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jopt("pre", "string", "constant, constantSmooth, linear, repeat, oscillate, oscillateSmooth, offsetRepeat, offsetRepeatSmooth or reset."),
            jopt("post", "string", "As pre."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.behavior post:repeat"),
    script("operandi.curve.interpolation", Items, "Sets curves' interpolation: every key's slopes (GE-60).")
        .on_editor(
            "Sets curves' interpolation: every key's slopes (GE-60).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jreq("type", "string", "curve, linear or stepped."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.interpolation type:linear"),
    script("operandi.curve.cycle", Items, "Turns the cycle option on or off: the last key matches the first (GE-62).")
        .on_editor(
            "Turns the cycle option on or off: the last key matches the first (GE-62).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jopt("on", "boolean", "On (default true)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.cycle on:true"),
    script("operandi.curve.shadow", Items, "Copy to Shadow, Replace with Shadow, Swap or Clear Shadow (GE-80).")
        .on_editor(
            "Copy to Shadow, Replace with Shadow, Swap or Clear Shadow (GE-80).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jopt("action", "string", "copy (default), replace, swap or clear."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.shadow action:copy"),
    script("operandi.curve.eulerFilter", Items, "Unwraps rotation curves that jump by whole turns (GE-X1).")
        .on_editor(
            "Unwraps rotation curves that jump by whole turns (GE-X1).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.eulerFilter"),
    script("operandi.curve.simplify", Items, "Removes keys that change the curve by no more than a tolerance (GE-X2).")
        .on_editor(
            "Removes keys that change the curve by no more than a tolerance (GE-X2).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jopt("tolerance", "number", "Largest change allowed (default 0.001)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.simplify tolerance:0.01"),
    script("operandi.time.set", Items, "Sets the current time (TK-01); not an undo step (TK-05). Snaps to a frame with Time Snap on.")
        .on_editor(
            "Sets the current time (TK-01); not an undo step (TK-05). Snaps to a frame with Time Snap on.",
            &[
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.time.set frame:24"),
    script("operandi.time.step", Items, "Steps the current time by frames (TK-63; Shift+Left/Right, comma/period).")
        .on_editor(
            "Steps the current time by frames (TK-63; Shift+Left/Right, comma/period).",
            &[
            jopt("frames", "number", "Frames to step (default 1; negative steps back)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.time.step frames:-1"),
    script("operandi.time.go", Items, "Goes to the current range's first or last frame, or the previous, next, first or last key of the selected items or channels (TK-63).")
        .on_editor(
            "Goes to the current range's first or last frame, or the previous, next, first or last key of the selected items or channels (TK-63).",
            &[
            jopt("to", "string", "first, last, prevKey, nextKey, firstKey or lastKey (default first)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.time.go to:last"),
    script("operandi.time.play", Items, "Plays or stops playback through the current range (TK-60; /).")
        .on_editor(
            "Plays or stops playback through the current range (TK-60; /).",
            &[
            jopt("backwards", "boolean", "Play backwards (default false)."),
            jopt("stop", "boolean", "true stops, false starts; default toggles."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.time.play stop:true"),
    script("operandi.time.keyTime", Items, "Sets the key time alone, where new keys land (TL-20), or resets it to follow the current time.")
        .on_editor(
            "Sets the key time alone, where new keys land (TL-20), or resets it to follow the current time.",
            &[
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
            jopt("reset", "boolean", "Follow the current time again."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.time.keyTime reset:true"),
    script("operandi.channel.select", Items, "Selects a channel for keying (UIC-119), or clears the channel selection.")
        .on_editor(
            "Selects a channel for keying (UIC-119), or clears the channel selection.",
            &[
            jopt("channel", "string", "item:name, or none."),
            jopt("add", "boolean", "Toggle it in the selection (default false)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.channel.select channel:none"),
    script("operandi.key.select", Items, "Selects keys in the track row (TL-41).")
        .on_editor(
            "Selects keys in the track row (TL-41).",
            &[
            jopt("keys", "string", "Keys as item:name@frame,...."),
            jopt("mode", "string", "replace, add, remove, all or clear (default replace)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.key.select mode:clear"),
    script("operandi.marker.select", Items, "Selects markers (TL-52).")
        .on_editor(
            "Selects markers (TL-52).",
            &[
            jopt("ids", "string", "Marker ids, comma separated."),
            jopt("add", "boolean", "Add to the selection (default false)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.marker.select ids:1"),
    script("operandi.key.copy", Items, "Copies the selected keys (or the selected items' keys at the key time) to the key clipboard (TL-45/47); not an undo step.")
        .on_editor(
            "Copies the selected keys (or the selected items' keys at the key time) to the key clipboard (TL-45/47); not an undo step.",
            &[],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.key.copy"),
    script("operandi.key.set", Items, "Sets keys at the key time with the channels' current values (TK-20/21; S keys the selected channels).")
        .on_editor(
            "Sets keys at the key time with the channels' current values (TK-20/21; S keys the selected channels).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.set channels:1:pos.Y"),
    script("operandi.key.toggle", Items, "A status-light click: keys the channels, or removes their keys at the key time when every one has one (TK-20).")
        .on_editor(
            "A status-light click: keys the channels, or removes their keys at the key time when every one has one (TK-20).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.toggle channels:1:pos.X"),
    script("operandi.key.remove", Items, "Removes the channels' keys at the key time or the given time (TK-24 Remove Key).")
        .on_editor(
            "Removes the channels' keys at the key time or the given time (TK-24 Remove Key).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.remove channels:1:pos.Y"),
    script("operandi.key.transforms", Items, "Keys position, rotation, scale or all three of the selected items (TK-22; Shift+W/E/R/Y).")
        .on_editor(
            "Keys position, rotation, scale or all three of the selected items (TK-22; Shift+W/E/R/Y).",
            &[
            jopt("type", "string", "position, rotation, scale or all (default all)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.transforms type:position"),
    script("operandi.key.items", Items, "Keys every animated channel of the selected items (TK-23; Shift+S).")
        .on_editor(
            "Keys every animated channel of the selected items (TK-23; Shift+S).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.items"),
    script("operandi.key.removeAnimation", Items, "Removes every key of the channels, keeping their current values (TK-24).")
        .on_editor(
            "Removes every key of the channels, keeping their current values (TK-24).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.removeAnimation channels:1:pos.Y"),
    script("operandi.key.reset", Items, "Removes every key of the channels and puts back their default values (TK-24).")
        .on_editor(
            "Removes every key of the channels and puts back their default values (TK-24).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.reset channels:1:pos.Y"),
    script("operandi.key.bake", Items, "Keys the channels on every frame of the current range (TK-24).")
        .on_editor(
            "Keys the channels on every frame of the current range (TK-24).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.bake channels:1:pos.Y"),
    script("operandi.key.apply", Items, "Keys every edited-but-not-keyed channel at the key time (TK-41 Apply).")
        .on_editor(
            "Keys every edited-but-not-keyed channel at the key time (TK-41 Apply).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.apply"),
    script("operandi.key.discard", Items, "Drops every edited-but-not-keyed value back to its curve (TK-41 Discard).")
        .on_editor(
            "Drops every edited-but-not-keyed value back to its curve (TK-41 Discard).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.discard"),
    script("operandi.key.autoKey", Items, "Sets Auto Key: off, animated or all (TK-30).")
        .on_editor(
            "Sets Auto Key: off, animated or all (TK-30).",
            &[
            jreq("mode", "string", "off, animated or all."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.autoKey mode:animated"),
    script("operandi.key.move", Items, "Moves the selected keys by frames (TL-42, Shift...).")
        .on_editor(
            "Moves the selected keys by frames (TL-42, Shift...).",
            &[
            jreq("frames", "number", "Frames to move (negative moves earlier)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.move frames:2"),
    script("operandi.key.scale", Items, "Scales the selected keys' timing around a pivot (TL-43, Scale...).")
        .on_editor(
            "Scales the selected keys' timing around a pivot (TL-43, Scale...).",
            &[
            jreq("factor", "number", "Timing factor, above 0."),
            jopt("frame", "number", "Pivot frame (default the key time)."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.scale factor:2"),
    script("operandi.key.invert", Items, "Reverses the selected keys' timing within their span (TL-45 Invert...).")
        .on_editor(
            "Reverses the selected keys' timing within their span (TL-45 Invert...).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.invert"),
    script("operandi.key.delete", Items, "Deletes the selected keys (TL-45 Delete / Delete Related).")
        .on_editor(
            "Deletes the selected keys (TL-45 Delete / Delete Related).",
            &[
            jopt("related", "boolean", "Also the sibling X/Y/Z keys at the same times."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.delete related:true"),
    script("operandi.key.slope", Items, "Sets the selected keys' slope type (TL-45/46).")
        .on_editor(
            "Sets the selected keys' slope type (TL-45/46).",
            &[
            jreq("type", "string", "auto, smoothFlat, autoFlat, flat, linear, linearIn, linearOut, stepped or manual."),
            jopt("side", "string", "in, out or both (default both; linearIn/Out pick theirs)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.slope type:linear"),
    script("operandi.key.cut", Items, "Copies the selected keys to the key clipboard, then deletes them (TL-45).")
        .on_editor(
            "Copies the selected keys to the key clipboard, then deletes them (TL-45).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.cut"),
    script("operandi.key.paste", Items, "Pastes the key clipboard at the key time (TL-45): merge, insert (later keys move along) or replace (keys in the pasted span go).")
        .on_editor(
            "Pastes the key clipboard at the key time (TL-45): merge, insert (later keys move along) or replace (keys in the pasted span go).",
            &[
            jopt("mode", "string", "merge, insert or replace (default merge)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.paste mode:replace"),
    script("operandi.time.insertFrame", Items, "Moves the selected items' keys after the key time one frame later (TL-21).")
        .on_editor(
            "Moves the selected items' keys after the key time one frame later (TL-21).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.time.insertFrame"),
    script("operandi.time.removeFrame", Items, "Moves the selected items' keys after the key time one frame earlier, never deleting keys (TL-21).")
        .on_editor(
            "Moves the selected items' keys after the key time one frame earlier, never deleting keys (TL-21).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.time.removeFrame"),
    script("operandi.time.settings", Items, "Sets the scene's time settings (TK-03/04): frame rate, scene and current range (frames), time display, Auto Key, Time Snap, Play Real Time, End Behavior, Auto Update Existing Keys.")
        .on_editor(
            "Sets the scene's time settings (TK-03/04): frame rate, scene and current range (frames), time display, Auto Key, Time Snap, Play Real Time, End Behavior, Auto Update Existing Keys.",
            &[
            jopt("fps", "number", "Frames per second."),
            jopt("start", "number", "Scene range start, frames."),
            jopt("end", "number", "Scene range end, frames."),
            jopt("rangeStart", "number", "Current range start, frames."),
            jopt("rangeEnd", "number", "Current range end, frames."),
            jopt("display", "string", "frames, seconds, smpte or film."),
            jopt("autoKey", "string", "off, animated or all."),
            jopt("snap", "boolean", "Time Snap."),
            jopt("realTime", "boolean", "Play Real Time."),
            jopt("endBehavior", "string", "hold, repeat or wrap."),
            jopt("autoUpdate", "boolean", "Auto Update Existing Keys."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.time.settings fps:24 start:0 end:120"),
    script("operandi.time.range", Items, "Sets the current range in frames (TL-13), or resets it to the scene range.")
        .on_editor(
            "Sets the current range in frames (TL-13), or resets it to the scene range.",
            &[
            jopt("start", "number", "Start frame."),
            jopt("end", "number", "End frame."),
            jopt("reset", "boolean", "Back to the scene range."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.time.range start:0 end:48"),
    script("operandi.time.fit", Items, "Fits the current range to the selected items' or channels' keys, or every key (TL-14; A / Shift+A).")
        .on_editor(
            "Fits the current range to the selected items' or channels' keys, or every key (TL-14; A / Shift+A).",
            &[
            jopt("all", "boolean", "Every key in the scene."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.time.fit all:true"),
    script("operandi.marker.add", Items, "Adds a time marker at the current or given time, owned by the selected item or the scene (TL-50/51; M).")
        .on_editor(
            "Adds a time marker at the current or given time, owned by the selected item or the scene (TL-50/51; M).",
            &[
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
            jopt("scene", "boolean", "Owned by the scene even with items selected."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.marker.add frame:12"),
    script("operandi.marker.edit", Items, "Changes markers (TL-52/53): time, shift, length, end, label, shape, colour, Show Label, Frame Limits, Snap to Frames.")
        .on_editor(
            "Changes markers (TL-52/53): time, shift, length, end, label, shape, colour, Show Label, Frame Limits, Snap to Frames.",
            &[
            jopt("ids", "string", "Marker ids; default the selected markers."),
            jopt("frame", "number", "New start frame."),
            jopt("shift", "number", "Frames to move by."),
            jopt("length", "number", "Range length, frames (0: a plain marker)."),
            jopt("endFrame", "number", "Range end frame."),
            jopt("label", "string", "Label."),
            jopt("shape", "string", "triangle, square, diamond or bar."),
            jopt("color", "string", "r,g,b 0-1."),
            jopt("showLabel", "boolean", "Show the label above the marker."),
            jopt("frameLimits", "boolean", "Limit playback and stepping to the range."),
            jopt("snapToFrames", "boolean", "Move start and end onto whole frames."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.marker.edit ids:1 label:Contact"),
    script("operandi.marker.delete", Items, "Deletes markers (TL-52).")
        .on_editor(
            "Deletes markers (TL-52).",
            &[
            jopt("ids", "string", "Marker ids; default the selected markers."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.marker.delete ids:1"),
    // The map's Transform Channel (PD-Q12): on a live item tool's Texture
    // Locator it is one step inside the tool.
    script("transform.channel", Items, "Sets (or with ? reads) one transform channel: pos.X/Y/Z, rot.X/Y/Z, scl.X/Y/Z; one undo step.")
        .during_transform_tool()
        .undo()
        .query()
        .headless(
            "Set one transform channel of an item (default the active item); value \"?\" answers the current value. One undo step; on the Texture Locator a live Move/Rotate/Scale/Transform holds, one step inside that tool, as a drag of its handle (PD-Q12).",
            &[
            jreq(
                "name",
                "string",
                "pos.X/Y/Z (metres), rot.X/Y/Z (radians, or with deg) or scl.X/Y/Z (a Texture Locator's Size, at least 0.001).",
            ),
            jreq("value", "any", "The new value; \"?\" reads it."),
            jopt(
                "mode",
                "integer",
                "Not supported yet (the map's mode); leave it out.",
            ),
            JSON_ITEM,
            jopt(
                "filter",
                "string",
                "Not supported yet (the map's filter); leave it out.",
            ),
            jopt(
                "replace",
                "string",
                "Not supported yet (the map's replace); leave it out.",
            ),
        ],
        )
        .example("transform.channel name:scl.X value:2"),
    // Spec freeze-transforms-2026-11-14 section 8 (item_transform_ops.rs).
    script("transform.reset", Items, "Puts the selected items' translation, rotation, scale or all of them back to identity; the items move, their geometry doesn't.")
        .undo()
        .headless(
            "Reset part of every selected item's transform to identity (position 0, rotation 0, scale 100%). Children follow; geometry is untouched; locked items are skipped. One undo step, none when nothing changes.",
            &[jopt(
                "type",
                "string",
                "translation, rotation, scale or all; default all.",
            )],
        )
        .example("transform.reset rotation"),
    script("transform.freeze", Items, "Bakes the selected mesh items' translation, rotation, scale or whole transform into their geometry; nothing moves.")
        .undo()
        .headless(
            "Bake part of every selected mesh item's transform into its points, then set that part to identity, so nothing moves on screen. Children are compensated; mirrored scale reverses polygons; locked items and non-mesh items are skipped. One undo step, none when nothing changes.",
            &[jopt(
                "type",
                "string",
                "translation, rotation, scale or all; default all.",
            )],
        )
        .example("transform.freeze scale"),
    script("item.setType", Items, "Changes a light's type, keeping what both types share.")
        .undo()
        .headless(
            "Change a light's type, keeping what both types share (Light Properties > Type).",
            &[
            jreq(
                "type",
                "string",
                "Light type: Directional, Point, Spot, Area, Cylinder, Photometric, Mesh, Dome or Portal.",
            ),
            jopt(
                "mask",
                "string",
                "Not supported yet (the map's mask); leave it out.",
            ),
            JSON_ITEM,
        ],
        )
        .example("item.setType Spot"),
    script("light.temperature", Items, "Sets a light's colour from a colour temperature (kelvins).")
        .undo()
        .headless(
            "Set a light's colour from a colour temperature (Light Material > Set Temperature).",
            &[
            jreq("kelvins", "number", "Colour temperature, kelvins."),
            jopt(
                "gamma",
                "number",
                "Not supported yet (the map's gamma); leave it out.",
            ),
            JSON_ITEM,
        ],
        )
        .example("light.temperature 5600"),
    script("light.mesh", Items, "Sets a Mesh Light's Prototype mesh, or none.")
        .undo()
        .headless(
            "Set a Mesh Light's Prototype mesh (or none).",
            &[
            jopt(
                "light",
                "integer",
                "The Mesh Light's item id; default the active item.",
            ),
            jreq("mesh", "string", "The mesh item id, or \"none\"."),
        ],
        )
        .example("light.mesh mesh:none"),
    script("texture.setIMap", Shading, "Sets an Image Map layer's Image (scene image name or path, or none).")
        .undo()
        .headless(
            "Set an Image Map layer's Image: a scene image by name or file path, or none.",
            &[
            jreq(
                "image",
                "string",
                "Scene image name or file path, or \"none\".",
            ),
            jopt(
                "name",
                "string",
                "Not supported yet (the map's name); leave it out.",
            ),
            jreq("layer", "integer", "The Image Map layer's id."),
        ],
        ),
    script("texture.delete", Shading, "Deletes texture layer item: (a layer id) from the Shader Tree or an environment, with the Texture Locator no other layer uses (Clear Image on an environment). One undo step.")
        .args(&[arg("item", INT, N)])
        .undo()
        .headless(
            "Delete a texture layer (Shader Tree or environment) and its unshared Texture Locator.",
            &[jreq("item", "integer", "The texture layer's id.")],
        )
        .example("texture.delete item:4294967298"),
    script("operandi.environment.loadImage", Shading, "Load Image…: the picture filename: becomes environment item:'s image (default the top environment, made when there is none); it replaces the picture of the environment's image layer, keeping its rotation and opacity, or adds an Image Map layer above the Environment Material. One undo step. Without a filename opens the file panel.")
        .operandi()
        .args(&[opt("filename", ArgType::Path, N, None), opt("item", INT, N, None)])
        .undo()
        .example("operandi.environment.loadImage filename:/hdri/studio.hdr"),
    script("texture.setLocator", Shading, "Sets a texture layer's Texture Locator item (or none).")
        .undo()
        .headless(
            "Set a texture layer's Texture Locator item (or none).",
            &[
            jreq("locator", "string", "Locator item id, or \"none\"."),
            jreq("layer", "integer", "The texture layer's id."),
        ],
        ),
    script("texture.setVMap", Shading, "Sets a Vertex Map texture layer's map (or none).")
        .undo()
        .headless(
            "Set a Vertex Map texture layer's map by name (or none).",
            &[
            jreq("name", "string", "Vertex map name, or \"none\"."),
            jreq("layer", "integer", "The Vertex Map layer's id."),
        ],
        ),
    script("mask.setPTag", Shading, "Sets a Group's Polygon Tag ((all), (none) or a tag name).")
        .undo()
        .headless(
            "Set a Group's Polygon Tag: a tag name, \"(all)\" or \"(none)\".",
            &[
            jreq("name", "string", "Tag name, (all) or (none)."),
            jreq("layer", "integer", "The Group layer's id."),
        ],
        ),
    script("mask.setPTagType", Shading, "Sets a Group's Polygon Tag Type; the tag goes back to (none).")
        .undo()
        .headless(
            "Set a Group's Polygon Tag Type (Material, Part or Selection Set); the tag goes back to (none).",
            &[
            jreq("name", "string", "Tag type."),
            jreq("layer", "integer", "The Group layer's id."),
        ],
        ),
    script("item.channel", Items, "Sets (or with ? reads) one channel of an item or shader layer; one undo step. Channel names are Operandi's own for now.")
        .undo()
        .query()
        .headless(
            "Set one channel of an item (default the active item) or of a shader layer; value \"?\" answers the current value. Channel names are Operandi's own for now (docs/COMMAND_REGISTRY.md lists them).",
            &[
            jreq(
                "name",
                "string",
                "Channel name, e.g. intensity, color (color.r for one part), opacity.",
            ),
            jreq(
                "value",
                "any",
                "The value: number, true/false, text, r,g,b, an item id or none; \"?\" reads it. Percents 0–1, distances metres, angles radians.",
            ),
            jopt(
                "mode",
                "integer",
                "Not supported yet (the map's mode); leave it out.",
            ),
            JSON_ITEM,
            jopt(
                "layer",
                "integer",
                "A shader-tree layer id instead of an item (Operandi's own argument).",
            ),
            jopt(
                "scene",
                "boolean",
                "true: the scene's own time channels fps, start, end, time (seconds) instead of an item (Operandi's own argument).",
            ),
        ],
        )
        .example("item.channel intensity ?"),
    script("environment.light", Shading, "Set Physical Sun: picks the directional light that places a Physically-based Daylight environment's sun (sunLight:<id>|none; envMaterial:<layer id>, default the active environment).")
        .undo()
        .headless(
            "Set Physical Sun: the directional light whose direction places the sun of a Physically-based Daylight environment (environments-2026-11-09 §3). One undo step.",
            &[
            jopt(
                "envMaterial",
                "integer",
                "The Environment Material's layer id; default the active environment's.",
            ),
            jreq("sunLight", "any", "A directional light's item id, or none."),
        ],
        )
        .example("environment.light sunLight:none"),
    script("smoothing.set", Modeling, "Sets the scene's smoothing amount, angle, crease by angle or groups.")
        .operandi()
        .undo()
        .headless(
            "Set the smooth-shading rule (one for every mesh: the default material's smoothing). Fields not given keep their value. Viewport, exports and renders all use it.",
            &[
            jopt(
                "amount",
                "number",
                "How far corners bend toward the averaged normal: 0 flat, 1 fully smooth (the default); -1..2 allowed.",
            ),
            jopt(
                "angle",
                "number",
                "Smoothing angle, radians: neighbouring polygons within this angle blend (default 40° = 0.698).",
            ),
            jopt(
                "crease_by_angle",
                "boolean",
                "Off blends every connected polygon (creases only from groups); default true.",
            ),
            jopt(
                "groups",
                "string",
                "Smoothing groups: \"off\", \"crease\" (default) or \"force\".",
            ),
        ],
        ),
    script("backdrop.add", Items, "Adds a Backdrop item at the origin; with an image it shows that picture, facing project (what dropping a picture on a view does, BD-60).")
        .args(&[
            opt("name", TEXT, N, None),
            opt("image", ArgType::Path, N, None),
            opt(
                "project",
                ArgType::Choice(&["front", "back", "top", "bottom", "left", "right", "camera"]),
                N,
                Some("front"),
            ),
        ])
        .undo()
        .example("backdrop.add name:Plan project:top"),
    script("backdrop.image", Items, "Shows a picture on the selected Backdrop (none clears); bare opens the file panel.")
        .operandi()
        .args(&[opt("path", ArgType::Path, N, None)])
        .undo(),
    script("backdrop.edit", Items, "Sets options of the selected Backdrop (map names, or the old backdrop.attr <name> <value>).")
        .args(&[
            opt("image", ArgType::Path, N, None),
            opt("project", TEXT, N, None),
            opt("aspect", BOOL, N, None),
            opt("flip", BOOL, N, None),
            opt("invert", BOOL, N, None),
            opt("blend", BOOL, N, None),
            opt("contrast", FLOAT, Unit::Percent, None),
            opt("brightness", FLOAT, Unit::Percent, None),
            opt("transparency", FLOAT, Unit::Percent, None),
        ])
        .undo()
        .example("backdrop.edit flip:true"),
    script("backdrop.fit", Items, "Fits the selected Backdrop to the active meshes' combined box (BD-61).").undo(),
    script("backdrop.size", Items, "Sizes the selected Backdrop's picture.")
        .operandi()
        .args(&[
            arg("width", FLOAT, Unit::Distance),
            opt("height", FLOAT, Unit::Distance, None),
        ])
        .undo()
        .example("backdrop.size 1"),
    script("clip.reload", Shading, "Reads a scene image's file again (Reload Image; all when bare) and updates the viewport; changes no scene data.")
        .args(&[opt("image", TEXT, N, Some("all"))]).side_effect(),
    script("clip.addStill", Shading, "Loads a picture file as a scene image entry only (what dropping a picture outside a 3D view does, BD-60).")
        .args(&[arg("filename", ArgType::Path, N)])
        .undo(),
    script("image.list", Shading, "Reports the scene's images: name, file path and pixel size.")
        .operandi().query(),
    // Nodes N1 (NM spec). Names confirmed by the registry owner
    // 2026-11-11; Operandi-only, the map has no node-graph commands. Shared
    // graphs are scene data, not items, so their defaults stay here rather
    // than in the item.channel table.
    script("operandi.nodeMaterial.import", Shading, "Imports a node graph file (JSON) as Node Material layers, all or nothing (NM-05).")
        .operandi()
        .headless(
            "Import a node graph file (JSON) as Node Material layers, placed as Add Layer places one; all or nothing per material. One undo step.",
            &[
            jreq("filename", "string", "The node graph file."),
            jopt(
                "layer",
                "integer",
                "A shader-tree layer the new layers go next to; default as Add Layer with nothing selected.",
            ),
        ],
        )
        .args(&[arg("filename", ArgType::Path, N), opt("layer", INT, N, None)])
        .undo(),
    script("operandi.nodeMaterial.showGraph", Shading, "Shows a Node Material's graph read-only in the Node View panel, docking it under the views if it isn't open (NM-15); bare, the selected Node Material layer's. A Material Group shows its own MaterialX graph (Show as Nodes).")
        .operandi()
        .ui()
        .args(&[opt("layer", INT, N, None), opt("graph", INT, N, None)]),
    script("operandi.nodeMaterial.setDefault", Shading, "Sets (or with ? reads) a node graph's shared exposed default; layers without their own value follow (NM-04). Graphs aren't items, so this isn't an item.channel.")
        .operandi()
        .headless(
            "Set a node graph's shared exposed default (layers without their own value follow), one undo step; value \"?\" answers the current default.",
            &[
            jreq(
                "graph",
                "integer",
                "The graph's id (shader_tree_get lists node graphs).",
            ),
            jreq("input", "string", "The exposed input's name, e.g. Tint."),
            jreq(
                "value",
                "any",
                "A number, r,g,b text or [r, g, b] (0..1 components); \"?\" reads it.",
            ),
        ],
        )
        .args(&[arg("graph", INT, N), arg("input", TEXT, N), arg("value", TEXT, N)])
        .undo()
        .example("operandi.nodeMaterial.setDefault graph:1 input:Tint value:1,0.5,0.5"),
    // Node editing (node_view_commands.rs; placeholder names, NAMES-NEEDED.md).
    script("operandi.nodes.add", Shading, "Adds a free MaterialX node from the node view's Add menu to a Material Group (NE-02). One undo step.")
        .operandi()
        .headless("Add a free MaterialX node (noise3d, multiply... with its output type) to a Material Group's node view.", &[
            jreq("group", "integer", "The Material Group's layer id."),
            jreq("node", "string", "The MaterialX category, e.g. noise3d."),
            jopt("type", "string", "Its output type, e.g. color3 (the default) or float."),
            jopt("x", "number", "Canvas x."),
            jopt("y", "number", "Canvas y."),
        ])
        .args(&[arg("group", INT, N), arg("node", TEXT, N), opt("type", TEXT, N, None), opt("x", FLOAT, N, None), opt("y", FLOAT, N, None)])
        .undo()
        .example("operandi.nodes.add group:5 node:noise3d type:color3"),
    script("operandi.nodes.connect", Shading, "Wires a free node's output into a free node's input or a Material layer's channel, which it then drives (NE-03). One undo step.")
        .operandi()
        .headless("Wire a free node into <node>.<input> or layer:<id>.<channel>; the input's own value is kept but stops applying.", &[
            jreq("group", "integer", "The Material Group's layer id."),
            jreq("from", "string", "The free node."),
            jopt("output", "string", "Its output (out)."),
            jreq("to", "string", "<node>.<input> or layer:<id>.<channel>, e.g. layer:4.diffuse_color."),
        ])
        .args(&[arg("group", INT, N), arg("from", TEXT, N), opt("output", TEXT, N, None), arg("to", TEXT, N)])
        .undo()
        .example("operandi.nodes.connect group:5 from:Noise to:layer:4.diffuse_color"),
    script("operandi.nodes.delete", Shading, "Deletes node-view boxes: a free node with its wires, or a layer's box (deleting the layer); mixing nodes are locked (NE-05, NE-06). One undo step.")
        .operandi()
        .headless("Delete free nodes or layer boxes (layer:<id>) from a Material Group's node view; mixing nodes are refused.", &[
            jreq("group", "integer", "The Material Group's layer id."),
            jreq("node", "string", "Box names, comma-separated."),
        ])
        .args(&[arg("group", INT, N), arg("node", TEXT, N)])
        .undo(),
    script("operandi.nodes.disconnect", Shading, "Removes the wire into an input: its own value applies again (NE-03). One undo step.")
        .operandi()
        .headless("Remove the wire into <node>.<input> or layer:<id>.<channel>.", &[
            jreq("group", "integer", "The Material Group's layer id."),
            jreq("to", "string", "<node>.<input> or layer:<id>.<channel>."),
        ])
        .args(&[arg("group", INT, N), arg("to", TEXT, N)])
        .undo(),
    script("operandi.nodes.hide", Shading, "Hides node-view boxes in the view only (NE-07). One undo step.")
        .operandi()
        .headless("Hide boxes in a Material Group's node view; renders don't change.", &[
            jreq("group", "integer", "The Material Group's layer id."),
            jreq("node", "string", "Box names, comma-separated."),
        ])
        .args(&[arg("group", INT, N), arg("node", TEXT, N)])
        .undo(),
    script("operandi.nodes.move", Shading, "Places a node-view box (NE-09, NE-16). One undo step per drag; renders don't change.")
        .operandi()
        .headless("Place a box in a Material Group's node view.", &[
            jreq("group", "integer", "The Material Group's layer id."),
            jreq("node", "string", "The box."),
            jreq("x", "number", "Canvas x."),
            jreq("y", "number", "Canvas y."),
        ])
        .args(&[arg("group", INT, N), arg("node", TEXT, N), arg("x", FLOAT, N), arg("y", FLOAT, N)])
        .undo(),
    script("operandi.nodes.setValue", Shading, "Sets a free node's input value (NE-01). One undo step; a drag is one step on release.")
        .operandi()
        .headless("Set a free node's input value (a number, or comma-separated numbers).", &[
            jreq("group", "integer", "The Material Group's layer id."),
            jreq("node", "string", "The free node."),
            jreq("input", "string", "Its input."),
            jreq("value", "any", "A number or comma-separated numbers."),
        ])
        .args(&[arg("group", INT, N), arg("node", TEXT, N), arg("input", TEXT, N), arg("value", TEXT, N)])
        .undo(),
    script("operandi.nodes.show", Shading, "Shows hidden node-view boxes (NE-07). One undo step.")
        .operandi()
        .headless("Show hidden boxes in a Material Group's node view.", &[
            jreq("group", "integer", "The Material Group's layer id."),
            jreq("node", "string", "Box names, comma-separated."),
        ])
        .args(&[arg("group", INT, N), arg("node", TEXT, N)])
        .undo(),
    script("operandi.nodes.showAll", Shading, "Shows every hidden node-view box (NE-07). One undo step.")
        .operandi()
        .headless("Show every hidden box in a Material Group's node view.", &[jreq("group", "integer", "The Material Group's layer id.")])
        .args(&[arg("group", INT, N)])
        .undo(),
    script("preset.saveAs", Presets, "Saves a preset; without a filename opens the save panel. mtlx:true (an Operandi extra, default off; the Preset Browser gear toggle sets it) also writes a .mtlx beside a material preset. kind:environment or item:<environment> saves an environment preset (.openv) of every environment; lights:, meshes: and cameras: (Operandi extras, default false) add every light, the selected meshes and every camera; centerOnGround:true (an Operandi extra, default off) saves an item preset's copy centred on X/Z and resting on Y = 0; bare, its options dialog opens.")
        .args(&[
            opt(
                "kind",
                ArgType::Choice(&["item", "assembly", "environment", "renderSettings", "material"]),
                N,
                None,
            ),
            opt("filename", ArgType::Path, N, None),
            opt("mtlx", BOOL, N, Some("false")),
            opt("item", INT, N, None),
            opt("lights", BOOL, N, Some("false")),
            opt("meshes", BOOL, N, Some("false")),
            opt("cameras", BOOL, N, Some("false")),
            opt("centerOnGround", BOOL, N, Some("false")),
        ]).side_effect(),
    script("preset.do", Presets, "Loads a preset as one undo step; also what the Preset Browser shows (texture sets, HDRs, pictures, scenes). target: (an Operandi extra) names the material group a material goes into; without a preset path opens the file panel. mode: (an Operandi extra) applies an environment preset as replace (default) or add.")
        .args(&[
            opt("preset", ArgType::Path, N, None),
            opt("handler", TEXT, N, None),
            opt("target", TEXT, N, None),
            opt("mode", ArgType::Choice(&["replace", "add", "0", "1"]), N, Some("replace")),
        ])
        .undo(),
    // MaterialX (docs/specs/openpbr-materialx): Operandi's own names
    // (registry ruling 2026-11-12).
    script("operandi.materialx.import", Presets, "Imports a MaterialX material as a Material and Image Map layers, directly above layer: or at the top of the shader tree without one; one that needs Node Material is refused whole. One undo step. Without a filename opens the file panel.")
        .operandi()
        .args(&[opt("filename", ArgType::Path, N, None), opt("layer", INT, N, None)])
        .undo(),
    script("operandi.materialx.export", Presets, "Writes material or material group item: (id or name; without one, the selected one) as an OpenPBR MaterialX material, listing what it couldn't write. Not undoable. Without a filename opens the save panel.")
        .operandi()
        .args(&[opt("filename", ArgType::Path, N, None), opt("item", TEXT, N, None)])
        .side_effect(),
    // ------------------------------------------------------ preset browser
    // Names from the registry thread (2026-11-11), Stage 1 PB-100..122.
    // Layouts stage 6 (LAY-50, UIC-205): every palette.
    script("layout.window", Presets, "Opens or closes a palette (its own window): presetBrowser, uvEditor, uvView, toolbox, items, properties, shading, groups, vertexMaps, commandHistory or vertexNormalTools. Bare open: toggles; open:true on an open one brings it forward (LAY-50).")
        .args(&[
            arg(
                "hash",
                ArgType::Choice(&[
                    "presetBrowser",
                    "uvEditor",
                    "uvView",
                    "toolbox",
                    "items",
                    "properties",
                    "shading",
                    "groups",
                    "vertexMaps",
                    "commandHistory",
                    "formEditor",
                    "vertexNormalTools",
                ]),
                N,
            ),
            opt("open", BOOL, N, None),
        ])
        .example("layout.window hash:uvEditor")
        .ui(),
    script("operandi.layout.togglePalettes", View, "Hides every open palette, or shows them all again (Layout > Windows/Palettes > Toggle Palettes, `; LAY-53).")
        .operandi()
        .ui(),
    // Layouts stage 1 (layouts-2026-11-16, LAY-30): every way of switching
    // layouts runs this.
    script("layout.restore", View, "Shows the layout called name: (Layout > Layouts lists them). The layout being left keeps its state; the scene, undo and selection don't change.")
        .args(&[arg("name", TEXT, N)])
        .example("layout.restore UV")
        .ui(),
    // Layouts stage 2 (layouts-2026-11-16 stage 2). Placeholders under
    // operandi.* until the registry thread names them (docs/NAMES-NEEDED.md).
    // The menu, the layout bar, the keys and the pie all run these (LAY-30).
    script("operandi.layout.create", View, "Makes a new layout called name: at the right-hand end of the layout bar and shows it. copy:true copies the current layout; without it the view area is one empty panel to build from (LAY-43). Without name: opens the New Layout dialog (CMD-12).")
        .operandi()
        .args(&[opt("name", TEXT, N, None), opt("copy", BOOL, N, Some("false"))])
        .example("operandi.layout.create Sketch")
        .ui(),
    script("operandi.layout.save", View, "Saves the current layout's file now (changes are also remembered automatically, UIC-199).")
        .operandi()
        .ui(),
    script("operandi.layout.saveAs", View, "Saves the current arrangement as a new layout called name: and shows it (LAY-44). Without name: opens the Save Layout As dialog (CMD-12).")
        .operandi()
        .args(&[opt("name", TEXT, N, None)])
        .example("operandi.layout.saveAs {My Model}")
        .ui(),
    script("operandi.layout.rename", View, "Renames layout: (the current one without it) to name:. Layouts you made only; shipped ones keep their names (LAY-45). Without name: opens the Rename Layout dialog (CMD-12).")
        .operandi()
        .args(&[opt("name", TEXT, N, None), opt("layout", TEXT, N, None)])
        .example("operandi.layout.rename Sculpting layout:Sketch")
        .ui(),
    script("operandi.layout.delete", View, "Deletes layout: (the current one without it). Layouts you made only; shipped ones can be reverted but not deleted (LAY-46). ask:true asks first (Layout > Delete Layout...).")
        .operandi()
        .args(&[opt("layout", TEXT, N, None), opt("ask", BOOL, N, Some("false"))])
        .ui(),
    script("operandi.layout.revert", View, "Puts layout: (the current one without it) back to its shipped arrangement and drops your copy (LAY-42). Its own keys stay unless keys:true (LAY-Q56). ask:true asks first (the tab's double-click, Layout > Reset Layout...).")
        .operandi()
        .aliases(&["operandi.layout.reset"])
        .args(&[opt("layout", TEXT, N, None), opt("keys", BOOL, N, Some("false")), opt("ask", BOOL, N, Some("false"))])
        .ui(),
    // Layouts stage 5 (LAY-20, UIC-201): each layout's key map context.
    script("operandi.layout.setKey", View, "Sets action:'s key (an Input Map action) in layout:'s own key map (the current layout without it): key:none unbinds it there, no key: gives it back to the global map. Other layouts keep the global key (LAY-20).")
        .operandi()
        .args(&[arg("action", TEXT, N), opt("key", TEXT, N, None), opt("layout", TEXT, N, None)])
        .example("operandi.layout.setKey action:Move key:Ctrl+M")
        .ui(),
    script("operandi.layout.clearKeys", View, "Clears layout:'s own keys (the current layout without it): it uses the global key map again (LAY-20).")
        .operandi()
        .args(&[opt("layout", TEXT, N, None)])
        .ui(),
    // Layouts stage 3 (LAY-87, UIC-216): a toolbox form back to shipped.
    script("operandi.form.revert", View, "Puts form: (a toolbox form id, e.g. toolbox.uv) back to its shipped version and drops your copy (LAY-87).")
        .operandi()
        .args(&[arg("form", TEXT, N)])
        .example("operandi.form.revert toolbox.uv")
        .ui(),
    // Layouts stage 4 (LAY-82, LAY-86, LAY-87, UIC-212): workbench edit
    // mode. form: defaults to the workbench being edited, else the
    // toolbox's (toolbox.workbench); index: counts the form's controls
    // from 0. Edits are the user's copy of the form (Revert drops it).
    script("operandi.workbench.edit", View, "Shows or hides the edit tools of workbench form: (Edit Workbench, LAY-82); bare toggles. Only forms marked as a workbench can be edited.")
        .operandi()
        .args(&[opt("on", BOOL, N, None), opt("form", TEXT, N, None)])
        .example("operandi.workbench.edit form:toolbox.workbench")
        .ui(),
    script("operandi.workbench.pick", View, "Pick to Add (LAY-82): while on, clicking a button, a Tool Properties row, a preset or a Command History line adds it to the workbench being edited instead of running it; bare toggles. Esc stops.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.workbench.add", View, "Adds a button running line: to the end of workbench form: (what Pick to Add does for a button).")
        .operandi()
        .args(&[arg("line", TEXT, N), opt("label", TEXT, N, None), opt("form", TEXT, N, None)])
        .example("operandi.workbench.add {tool.set poly.bevel on} label:Bevel")
        .ui(),
    script("operandi.workbench.addProperty", View, "Adds a tool's Tool Properties row (property: <tool>/<key>, e.g. Bevel/inset) to workbench form:; it shows while that tool is open.")
        .operandi()
        .args(&[arg("property", TEXT, N), opt("label", TEXT, N, None), opt("form", TEXT, N, None)])
        .example("operandi.workbench.addProperty Bevel/inset")
        .ui(),
    script("operandi.workbench.addDivider", View, "Adds a divider to workbench form:; with label: it starts a collapsible section (Add Divider with Label).")
        .operandi()
        .args(&[opt("label", TEXT, N, None), opt("form", TEXT, N, None)])
        .example("operandi.workbench.addDivider label:Bevels")
        .ui(),
    script("operandi.workbench.addPipe", View, "Adds the current tool pipe (action centre, falloff, symmetry and the open tool) to workbench form: as a one-click custom tool (as:preset). as:macro and as:script aren't in Operandi yet.")
        .operandi()
        .args(&[opt("as", ArgType::Choice(&["preset", "macro", "script"]), N, Some("preset")), opt("label", TEXT, N, None), opt("form", TEXT, N, None)])
        .example("operandi.workbench.addPipe label:{Soft Bevel}")
        .ui(),
    script("operandi.workbench.move", View, "Moves control from: of workbench form: to index to: (drag to reorder).")
        .operandi()
        .args(&[arg("from", INT, N), arg("to", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.workbench.move 3 0")
        .ui(),
    script("operandi.workbench.set", View, "Sets one property of control index: of workbench form: (its right-click menu): attr: label, tooltip, popup (default, popup, buttons), width (normal, wide, full), level (basic, standard, advanced), command, toggle (default, checkbox, toggleButton) or iconFile (a PNG or JPEG path; empty clears it).")
        .operandi()
        .args(&[
            arg("index", INT, N),
            arg("attr", ArgType::Choice(&["label", "tooltip", "popup", "width", "level", "command", "toggle", "iconFile"]), N),
            arg("value", TEXT, N),
            opt("form", TEXT, N, None),
        ])
        .example("operandi.workbench.set 0 width full")
        .ui(),
    script("operandi.workbench.chooseIcon", View, "Opens a file panel to pick a PNG or JPEG for control index: of workbench form: (Custom Icon...).")
        .operandi()
        .args(&[arg("index", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.workbench.chooseIcon 0")
        .ui(),
    script("operandi.workbench.duplicate", View, "Copies control index: of workbench form: just after it.")
        .operandi()
        .args(&[arg("index", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.workbench.duplicate 0")
        .ui(),
    script("operandi.workbench.delete", View, "Removes control index: from workbench form:.")
        .operandi()
        .args(&[arg("index", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.workbench.delete 0")
        .ui(),
    script("operandi.workbench.undo", View, "Takes back the last workbench edit of form: made this session (workbench edits aren't scene undo steps).")
        .operandi()
        .args(&[opt("form", TEXT, N, None)])
        .ui(),
    script("operandi.workbench.topBar", View, "Shows or hides the workbench row along the top of the window (toolbar.workbench); bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    // Layouts stage 7 (LAY-83, UIC-213): the Form Editor's edits. form:
    // defaults to the form selected in the Form Editor; index: counts the
    // form's controls from 0. Edits are the user's copy of the form
    // (operandi.form.revert drops it; operandi.workbench.undo takes one
    // back).
    script("operandi.form.newGroup", View, "Adds an empty group called name: to the Form Editor's tree (groups are for tidiness only).")
        .operandi()
        .args(&[arg("name", TEXT, N)])
        .example("operandi.form.newGroup {My Forms}")
        .ui(),
    script("operandi.form.new", View, "Makes a new, empty form called id: (your own form, kept in the Forms folder).")
        .operandi()
        .args(&[arg("id", TEXT, N), opt("label", TEXT, N, None), opt("style", TEXT, N, None), opt("group", TEXT, N, None)])
        .example("operandi.form.new my.tools label:{My Tools}")
        .ui(),
    script("operandi.form.add", View, "Adds a button running line: to form: (at index:, else the end).")
        .operandi()
        .args(&[arg("line", TEXT, N), opt("label", TEXT, N, None), opt("form", TEXT, N, None), opt("index", INT, N, None)])
        .example("operandi.form.add {tool.set poly.bevel on} form:my.tools")
        .ui(),
    script("operandi.form.addForm", View, "Places form sub: inside form: as a sub-form (at index:, else the end).")
        .operandi()
        .args(&[arg("sub", TEXT, N), opt("form", TEXT, N, None), opt("index", INT, N, None)])
        .example("operandi.form.addForm sub:toolbox.uv.ops form:toolbox.uv")
        .ui(),
    script("operandi.form.addDivider", View, "Adds a divider (a labelled one starts a section) at the end of form:.")
        .operandi()
        .args(&[opt("label", TEXT, N, None), opt("form", TEXT, N, None)])
        .ui(),
    script("operandi.form.move", View, "Moves control from: of form: to place to: (the Form Editor's drag).")
        .operandi()
        .args(&[arg("from", INT, N), arg("to", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.form.move from:0 to:1 form:toolbox.workbench")
        .ui(),
    script("operandi.form.set", View, "Sets property attr: to value: on control index: of form:, or on the form itself without index: (label, style, icons, iconSize, workbench, showWhen, slot, slotAt, slotSort, columns, group).")
        .operandi()
        .args(&[arg("attr", TEXT, N), opt("value", TEXT, N, None), opt("index", INT, N, None), opt("form", TEXT, N, None)])
        .example("operandi.form.set attr:workbench value:true form:my.tools")
        .ui(),
    script("operandi.form.delete", View, "Deletes control index: from form:.")
        .operandi()
        .args(&[arg("index", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.form.delete index:0 form:toolbox.workbench")
        .ui(),
    script("operandi.form.select", View, "Opens the Form Editor on form: (and its control index:): Show in Form Editor.")
        .operandi()
        .args(&[arg("form", TEXT, N), opt("index", INT, N, None)])
        .example("operandi.form.select form:toolbox.uv")
        .ui(),
    script("operandi.form.find", View, "Find Form: the next button clicked shows its form in the Form Editor instead of running; bare toggles. Esc stops.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.form.preview", View, "Shows (or hides) the selected form, or form:, drawn in the Form Editor; bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None), opt("form", TEXT, N, None)])
        .ui(),
    script("operandi.form.export", View, "Saves form: to its own file: filename:, else the data folder's Saved Forms folder.")
        .operandi()
        .args(&[opt("form", TEXT, N, None), opt("filename", ArgType::Path, N, None)])
        .ui(),
    // Layouts stage 3 (UIC-221/223): a toolbox placeholder for a command
    // Operandi doesn't have yet. Its state answer greys it with that
    // reason, so the button needs no UI-side logic.
    script("operandi.mode.is", View, "Available only while the selection mode is one of these (item, vertex, edge, polygon); a form's showWhen names it. Running it says which mode is on.")
        .operandi()
        .args(&[arg("modes", TEXT, N)])
        .example("operandi.mode.is vertex edge")
        .quiet(),
    script("operandi.notBuilt", View, "A placeholder for a command Operandi doesn't have yet (name:, e.g. \"Shortest Path\"); does nothing but say so.")
        .operandi()
        .args(&[arg("name", TEXT, N)])
        .example("operandi.notBuilt \"Shortest Path\"")
        .quiet(),
    script("operandi.layout.clear", View, "Empties the current layout's view area to one empty panel (LAY-47).")
        .operandi()
        .ui(),
    script("operandi.layout.star", View, "Stars or unstars layout: (the current one without it); bare toggles (UIC-14).")
        .operandi()
        .args(&[opt("layout", TEXT, N, None), opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.layout.starredOnly", View, "Filters the Layouts pie to starred layouts and the current layout; bare toggles (UIC-14).")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.bar.collapse", View, "Collapses bar: (top, the default; left; right) to a line at its window edge, or brings it back; bare on: toggles. width: reopens a side bar that wide (UIC-16; the logo click, a double-click on a side bar's resize line, and the line's double-click or drag run it).")
        .operandi()
        .args(&[opt("bar", TEXT, N, None), opt("on", BOOL, N, None), opt("width", FLOAT, N, None)])
        .ui(),
    script("operandi.layout.move", View, "Moves layout: to place index: on the layout bar (0 is the left end, UIC-13).")
        .operandi()
        .args(&[arg("layout", TEXT, N), arg("index", INT, N)])
        .example("operandi.layout.move UV 0")
        .ui(),
    script("operandi.layout.switcher", View, "Opens the quick layout switcher: layouts in most-recently-used order (UIC-18). Layout > Recent Layouts runs it.")
        .operandi()
        .ui(),
    // Since 2026-11-20 panels move only in UI build mode, so this lock no
    // longer changes anything; kept so scripts using the name still run.
    script("layout.lockThumbs", View, "Lock Thumb Dragging (LAY-48), kept for scripts: panels now move only in UI build mode, so the lock has no effect; bare toggles.")
        .args(&[opt("lock", BOOL, N, None)])
        .ui(),
    script("layout.lockDividers", View, "Lock Divider Dragging for the current layout: dividers don't resize (LAY-48); bare toggles.")
        .args(&[opt("lock", BOOL, N, None)])
        .ui(),
    // UI build mode (Jason, 2026-11-20): the only time panels move.
    script("operandi.layout.buildMode", View, "UI build mode on:true or off; bare toggles. Panels, tabs and the mode bar move only in this mode, and every panel shows a drag pad to move or dock it by (Layout > UI Build Mode, Ctrl+Shift+L).")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.viewport.saveAs", View, "Saves the active view-area panel (group:false) or the whole view area (group:true) as a named piece under category:, reusable in any layout (Save Viewport As / Save Viewport Group As, LAY-72).")
        .operandi()
        .args(&[arg("name", TEXT, N), opt("category", TEXT, N, Some("User")), opt("group", BOOL, N, Some("false"))])
        .example("operandi.viewport.saveAs Front category:Views")
        .ui(),
    script("operandi.viewport.save", View, "Saves the piece the active panel (or the view area) shows again; every layout that uses it updates (LAY-72).")
        .operandi()
        .ui(),
    script("operandi.viewport.use", View, "Shows the saved piece name: in panel: (the active one without it): a viewport piece replaces the panel, a group piece fills its place (LAY-72).")
        .operandi()
        .args(&[arg("name", TEXT, N), opt("panel", INT, N, None)])
        .example("operandi.viewport.use Front")
        .ui(),
    script("operandi.viewport.revert", View, "Puts the piece the active panel (or the view area) shows back to its saved version (LAY-72).")
        .operandi()
        .ui(),
    script("operandi.viewport.delete", View, "Deletes the saved piece name:; panels showing it keep what they show (Delete Saved Viewport, LAY-72).")
        .operandi()
        .args(&[arg("name", TEXT, N)])
        .example("operandi.viewport.delete Front")
        .ui(),
    script("operandi.panel.kind", View, "Makes view-area panel: (the active one without it) show kind: (3D View, UV View or Empty), in the same place (the content arrow, LAY-73).")
        .operandi()
        .args(&[arg("kind", TEXT, N), opt("panel", INT, N, None)])
        .example("operandi.panel.kind {UV View}")
        .ui(),
    script("operandi.panel.show", View, "Brings the registered panel kind: to the front, or restores it on the right when missing.")
        .operandi()
        .args(&[arg("kind", TEXT, N)])
        .example("operandi.panel.show kind:{Shader Tree}")
        .ui(),
    script("operandi.toolbox.set", Tools, "Shows the Model, UV or Paint toolbox set without switching layouts (Paint requires a painting build).")
        .operandi()
        .args(&[arg("set", ArgType::Choice(&["model", "uv", "paint"]), N)])
        .example("operandi.toolbox.set uv")
        .ui(),
    // A layout's open command (LAY-02): which toolbox tab opens.
    script("operandi.toolbox.tab", Tools, "Opens a toolbox tab: create, select, deform, duplicate, polygon, edge, vertex, edit, sculpt, uv or workbench (the UV tab then stays in the strip). The Model and UV layouts open with it.")
        .operandi()
        .args(&[arg("tab", ArgType::Choice(&["create", "select", "deform", "duplicate", "polygon", "edge", "vertex", "edit", "sculpt", "uv", "workbench"]), N)])
        .example("operandi.toolbox.tab uv")
        .ui(),
    // The Help menu (Jason 2026-11-16): Report Bug moved off the layout bar.
    // Report Bug's capture (BUG-19): bare opens the dialog; all four
    // options skip it (UIC-162).
    script("operandi.help.reportBug", Help, "Opens the bug report form on GitHub in the browser, filled in with this build and system and, as chosen, the tool and selection, recent commands and recent log; a screenshot of the window is saved in the reports folder and put on the clipboard to paste in. The form links to the idea form. Nothing is sent until you submit it there. Give all four options to skip the dialog.")
        .operandi()
        .args(&[
            opt("screenshot", BOOL, N, Some("true")),
            opt("toolState", BOOL, N, Some("true")),
            opt("commands", BOOL, N, Some("true")),
            opt("log", BOOL, N, Some("true")),
        ])
        .example("operandi.help.reportBug screenshot:false toolState:true commands:true log:true")
        .ui().side_effect(),
    script("operandi.help.documentation", Help, "Opens Operandi's documentation in the browser. Not available until the documentation is published.")
        .operandi()
        .ui(),
    // Menus as forms (follows layouts stage 7, LAY-Q66): a command for
    // every menu entry that had none, so every menu is a form and every
    // menu click is a Command History line (UIC-160). Placeholders until
    // the name map names them (NAMES-NEEDED.md).
    script("operandi.scene.current", Files, "Makes the open scene at index: (from 0, the File > Current list's order) the current one.")
        .operandi()
        .args(&[arg("index", INT, N)])
        .example("operandi.scene.current index:0")
        .ui(),
    script("operandi.project.new", Files, "Makes a project folder at path: called name: and sets it; without path: opens the New Project dialog.")
        .operandi()
        .args(&[opt("path", ArgType::Path, N, None), opt("name", TEXT, N, None)])
        .ui().side_effect(),
    script("operandi.project.set", Files, "Sets the project to the folder path:; without path: opens Set Project.")
        .operandi()
        .args(&[opt("path", ArgType::Path, N, None)])
        .ui(),
    script("operandi.project.openRecent", Files, "Sets the project to Recent Projects' entry index: (from 0, newest first).")
        .operandi()
        .args(&[opt("index", INT, N, None)])
        .example("operandi.project.openRecent index:0")
        .ui(),
    script("operandi.project.loadPreset", Presets, "Loads the project's copy of preset: (layout, viewportDisplay or inputMap); it becomes your own setting.")
        .operandi()
        .args(&[arg("preset", ArgType::Choice(&["layout", "viewportDisplay", "inputMap"]), N)])
        .example("operandi.project.loadPreset preset:layout")
        .ui(),
    script("operandi.project.savePreset", Presets, "Saves your preset: (layout, viewportDisplay or inputMap) into the project for others.")
        .operandi()
        .args(&[arg("preset", ArgType::Choice(&["layout", "viewportDisplay", "inputMap"]), N)])
        .example("operandi.project.savePreset preset:inputMap")
        .ui().side_effect(),
    script("operandi.inputMap", Preferences, "Opens the Input Map: every action and its key.")
        .operandi()
        .ui(),
    script("operandi.select.polygonTagType", Selection, "Sets the polygon tag type Materials mode selects by: material, part, smoothingGroup or selectionSet.")
        .operandi()
        .args(&[arg("type", ArgType::Choice(&["material", "part", "smoothingGroup", "selectionSet"]), N)])
        .example("operandi.select.polygonTagType type:material")
        .ui(),
    script("operandi.lasso.style", Selection, "Sets the lasso's shape: freeform, rectangle, circle or ellipse.")
        .operandi()
        .args(&[arg("style", ArgType::Choice(&["freeform", "rectangle", "circle", "ellipse"]), N)])
        .example("operandi.lasso.style style:rectangle")
        .ui(),
    script("operandi.lasso.partial", Selection, "Partial lasso: also selects what the lasso only touches; bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.view.showLocked", ViewportOptions, "Shows locked mesh items in a colour of their own; bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.view.showFalloffs", ViewportOptions, "Tints points by the falloff's weight (FO-06); bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.viewport.close", View, "Closes the active view (a window keeps its last view).")
        .operandi()
        .ui(),
    script("operandi.help.navigation", Help, "Shows the mouse navigation of the current preset in the status line.")
        .operandi()
        .ui(),
    script("operandi.window.diagnostics", Debug, "Opens the Diagnostics window.")
        .operandi()
        .ui(),
    script("operandi.window.nodeView", Shading, "Opens the node view of the selected Material Group, or of the group the selected layer is in; with nothing suitable selected, the group last shown (Window > Node View). With the node view open and in front, closes it (maximised, first restores its group).")
        .operandi()
        .ui(),
    script("operandi.window.frameTime", Debug, "Shows the frame time over the view; bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.mcp.bridge", Debug, "Turns the MCP bridge (scripting from outside the app) on or off; bare toggles. macOS and Linux only.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui().side_effect(),
    script("operandi.window.size", View, "Sets the main window to a standard size: compact (1100 x 720) or standard (1440 x 900).")
        .operandi()
        .args(&[arg("size", ArgType::Choice(&["compact", "standard"]), N)])
        .example("operandi.window.size size:compact")
        .ui(),
    script("pbview.basePathAdd", Presets, "Adds a folder as a Preset Browser root; basePathKey: is its label.")
        .args(&[
            arg("path", ArgType::Path, N),
            opt("basePathKey", TEXT, N, None),
            opt("set", TEXT, N, None),
        ])
        .ui(),
    script("pbview.basePathRemove", Presets, "Takes a root out of the Preset Browser; its files stay on disk.")
        .args(&[opt("path", ArgType::Path, N, None), opt("basePathKey", TEXT, N, None)])
        .ui(),
    script("pbview.refresh", Presets, "Reads the Preset Browser's folders again.").ui(),
    script("pbview.mode", Presets, "The Preset Browser's view: vertical (0, was thumbnails), list (1) or horizontal (2); ? returns the name. The numbers never change.")
        .args(&[arg("mode", ArgType::Choice(&["vertical", "list", "horizontal"]), N)])
        .ui(),
    script("pbview.sortMode", Presets, "Sorts the Preset Browser's contents: name, type, modified or size.")
        .args(&[arg("mode", ArgType::Choice(&["name", "type", "modified", "size"]), N)])
        .ui(),
    script("pbview.filterString", Presets, "Searches the shown folder and everything below it.")
        .args(&[opt("filter", TEXT, N, None)])
        .ui(),
    script("pbview.filterStringEnable", Presets, "Shows or hides the Preset Browser's search (bare toggles).")
        .args(&[opt("state", BOOL, N, None)])
        .ui(),
    script("operandi.pbview.thumbnailSize", Presets, "The Preset Browser's thumbnail size in points (48 to 192).")
        .operandi()
        .args(&[arg("size", ArgType::Float, N)])
        .ui(),
    // Library pack (F1). Operandi-only: the map has no command for it.
    // Locating a library is pbview.basePathAdd (registry ruling 2026-11-11).
    script("operandi.pbview.libraryGet", Presets, "Opens the free Operandi Library's web page (or url:) in the default web browser; downloads nothing and isn't undoable.")
        .operandi()
        .args(&[opt("url", TEXT, N, Some("https://operandi3d.org/library"))])
        .ui(),
    // Modo content folder (MC-20, MC-02; registry thread 2026-11-12).
    script("operandi.pbview.kindFilter", Presets, "Shows only one kind in the Preset Browser: all, meshes, materials, environments, images or scenes (0 to 5); ? returns it. Not undoable.")
        .operandi()
        .args(&[arg(
            "kind",
            ArgType::Choice(&["all", "meshes", "materials", "environments", "images", "scenes"]),
            N,
        )])
        .ui(),
    script("operandi.pbview.onlyUsable", Presets, "Hides (true) or lists greyed (false) files the Preset Browser can't use; bare toggles, ? returns it. Not undoable.")
        .operandi()
        .args(&[opt("state", BOOL, N, None)])
        .ui(),
    script("dircache.copy", Presets, "Copies folder source: into dest: (default: the library folder) in the background; an existing folder of that name needs overwrite:true. Not undoable.")
        .args(&[
            arg("source", ArgType::Path, N),
            opt("dest", ArgType::Path, N, None),
            opt("overwrite", BOOL, N, Some("false")),
        ])
        .ui(),
    script("file.rename", Presets, "Renames the selected Preset Browser entry (path: for scripts); a texture set's maps together.")
        .args(&[arg("newName", TEXT, N), opt("path", ArgType::Path, N, None)]).side_effect(),
    script("file.duplicate", Presets, "Copies the selected Preset Browser entry under a new name.")
        .args(&[arg("newName", TEXT, N), opt("path", ArgType::Path, N, None)]).side_effect(),
    script("file.delete", Presets, "Deletes the selected Preset Browser entry from disk (the menu asks first).")
        .args(&[opt("path", ArgType::Path, N, None)]).side_effect(),
    // ---------------------------------------------------------- selection sets
    script("select.editSet", Selection, "Saves the selection as a named set; bare opens the dialog.")
        .headless(
            "Store the current selection as a named selection set.",
            &[jreq("name", "string", "Set name.")],
        )
        .args(&[arg("name", TEXT, N), opt("mode", ArgType::Choice(&["add"]), N, None)])
        .undo()
        .example("select.editSet Front"),
    script("select.useSet", Selection, "Replaces, adds or removes a set's members in the selection; bare opens the dialog.")
        .headless(
            "Select a named selection set.",
            &[
            jreq("name", "string", "Set name."),
            jopt(
                "mode",
                "string",
                "\"replace\" (default), \"add\" or \"remove\".",
            ),
        ],
        )
        .args(&[
            arg("name", TEXT, N),
            opt("mode", ArgType::Choice(&["replace", "add", "remove"]), N, Some("replace")),
        ])
        .undo()
        .example("select.useSet Front"),
    script("select.deleteSet", Selection, "Deletes a selection set (all:true deletes every set); bare opens the dialog.")
        .headless(
            "Delete a selection set.",
            &[
            jopt(
                "name",
                "string",
                "Set name; omit to delete every set of the current mode.",
            ),
            jopt(
                "all",
                "boolean",
                "true deletes every set of the current mode.",
            ),
        ],
        )
        .args(&[opt("name", TEXT, N, None), opt("all", BOOL, N, Some("false"))])
        .undo()
        .example("select.deleteSet Front"),
    script("selset.ungroup", Selection, "Deletes a selection set group; its sets stay.")
        .operandi()
        .headless(
            "Delete a selection-set group (the sets stay).",
            &[jreq("name", "string", "Group name.")],
        )
        .args(&[arg("group", TEXT, N)])
        .undo()
        .example("selset.ungroup Group"),
    script("select.convertGroup", Selection, "Makes a selection set group of named sets; bare opens the dialog.")
        .headless(
            "Group selection sets.",
            &[
            jreq("name", "string", "Group name."),
            jopt(
                "sets",
                "array of strings",
                "Set names; default every set of the current mode.",
            ),
        ],
        )
        .args(&[arg("group", TEXT, N), opt("sets", TEXT, N, None)])
        .undo()
        .example("select.convertGroup Group"),
    // ---------------------------------------------------------- sculpt
    script("sculpt.smooth", Sculpt, "Starts the Smooth sculpt brush.").tool(),
    script("sculpt.push", Sculpt, "Starts the Push sculpt brush.").tool(),
    script("sculpt.inflate", Sculpt, "Starts the Inflate sculpt brush.").tool(),
    script("sculpt.move", Sculpt, "Starts the Move sculpt brush.").tool(),
    script("sculpt.fold", Sculpt, "Starts the Fold sculpt brush.").tool(),
    script("sculpt.clay", Sculpt, "Starts the Clay sculpt brush.").tool(),
    script("sculpt.smudge", Sculpt, "Starts the Smudge sculpt brush.").tool(),
    script("sculpt.carve", Sculpt, "Starts the Carve sculpt brush.").tool(),
    script("sculpt.scar", Sculpt, "Starts the Scar sculpt brush.").tool(),
    script("sculpt.flatten", Sculpt, "Starts the Flatten sculpt brush.").tool(),
    script("sculpt.tangentPinch", Sculpt, "Starts the Tangent Pinch sculpt brush.").tool(),
    script("sculpt.centerPinch", Sculpt, "Starts the Center Pinch sculpt brush.").tool(),
    script("sculpt.emboss", Sculpt, "Starts the Emboss sculpt brush.").tool(),
    script("sculpt.shrink", Sculpt, "Starts the Attenuate sculpt brush.").tool(),
    script("sculpt.spin", Sculpt, "Starts the Spin sculpt brush.").tool(),
    script("sculpt.vmapPaint", Sculpt, "Starts the Paint Vertex Map Tool.").tool(),
    script("sculpt.vmapSmooth", Sculpt, "Starts the Smooth Vertex Map Tool.").tool(),
    script("sculpt.vmapScale", Sculpt, "Starts the Scale Down Vertex Map Tool.").tool(),
    script("sculpt.vmapErase", Sculpt, "Starts the Erase Vertex Map Tool.").tool(),
    script("sculpt.vmapSetVal", Sculpt, "Starts the Set Value Vertex Map Tool.").tool(),
    script("vertMap.new", Sculpt, "Adds a vertex map (type weight, or uv for a UV map) to the active meshes and selects it.")
        .headless(
            "Add a weight map to the active meshes and select it (the one the Vertex Map Tools and the Vertex Map falloff use), or with type uv a UV map to the active mesh that becomes its selected UV map; one undo step.",
            &[
            jreq("name", "string", "The new map's name."),
            jopt(
                "type",
                "string",
                "\"weight\" (the default) or \"uv\" (also \"txuv\").",
            ),
            jopt(
                "init",
                "boolean",
                "Fill every point with value (0 when value is left out); false leaves the map empty. Default: filled only when value is given.",
            ),
            jopt("color", "string", "Accepted and ignored."),
            jopt("value", "number", "The weight every point gets, e.g. 0.5."),
        ],
        )
        .args(&[
            arg("name", TEXT, N),
            opt("type", ArgType::Choice(&["weight", "uv", "txuv"]), N, Some("weight")),
            opt("init", BOOL, N, Some("false")),
            opt("color", TEXT, N, None),
            opt("value", FLOAT, N, None),
        ])
        .undo()
        .example("vertMap.new Mask weight true value:0.5"),
    script("select.vertexMap", Sculpt, "Selects the weight map the Vertex Map Tools and falloff use.")
        .headless(
            "Select the weight map the Vertex Map Tools and the Vertex Map falloff use (one at a time).",
            &[
            jopt("name", "string", "The map's name; left out selects none."),
            jopt("type", "string", "\"weight\" (default) or \"wght\"."),
            jopt(
                "mode",
                "string",
                "\"set\" (default) or \"add\" select it; \"remove\" selects none.",
            ),
        ],
        )
        .args(&[
            opt("name", TEXT, N, None),
            opt("type", ArgType::Choice(&["weight", "wght"]), N, Some("weight")),
            opt("mode", ArgType::Choice(&["set", "add", "remove"]), N, Some("set")),
        ])
        .example("select.vertexMap Mask weight set").ui(),
    script("sculpt.liveWeightEdit", Sculpt, "Updates what a weight map drives during a stroke.")
        .args(&[opt("live", BOOL, N, None)])
        .example("sculpt.liveWeightEdit live:false").ui(),
    script("brush.smooth", Sculpt, "Gives the sculpt brush a smooth tip.").ui(),
    script("brush.hard", Sculpt, "Gives the sculpt brush a hard tip.").ui(),
    script("sculpt.borderLock", Sculpt, "Locks mesh borders while sculpting (toggles when bare).")
        .args(&[opt("state", ON_OFF_QUERY, N, None)])
        .query()
        .example("sculpt.borderLock ?"),
    script("sculpt.cornerLock", Sculpt, "Locks mesh corners while sculpting (toggles when bare).")
        .args(&[opt("state", ON_OFF_QUERY, N, None)])
        .query()
        .example("sculpt.cornerLock ?"),
    // ---------------------------------------------------------- debug
    // The app's benchmarks (operandi.bench.*) need its window: the app
    // registers them itself (operandi-app command_handlers.rs).
    // ================================================= typed actions
    // ---------------------------------------------------------- files
    script("scene.open", Files, "Opens a scene file; without a filename opens the file panel.")
        .aliases(&["Open"])
        .args(&[opt("filename", ArgType::Path, N, None)])
        .example("scene.open /nonexistent.opscene").side_effect(),
    script("scene.new", Files, "Opens a new, empty scene in its own tab.").side_effect(),
    script("scene.save", Files, "Saves the current scene; a scene never saved asks for a file name.").side_effect(),
    script("scene.saveAs", Files, "Saves the scene under a file name; a format (or a filename whose extension names one) writes an interchange file instead, leaving the scene's own name alone. Without a filename opens the save panel.")
        .headless(
            "Export the scene to an interchange file, as File > Export does (Mesh Stack results, hidden items as hidden). The document keeps its own file name and unsaved state, and nothing is added to the history. Here it only exports: it needs filename plus a format (format:, or a file extension that names one); to save the .opscene use scene_save.",
            &[
            jreq("filename", "string", "The file to write."),
            jopt(
                "format",
                "string",
                "obj, stl, stlAscii, gltf, glb, geo, svg, lxo, ply, plyAscii, abc, usd, usda, usdc, usdz, fbx or 3mf; default from the file's extension (.stl and .ply mean binary). USD needs the running app's exporter (operandi-mcp --attach).",
            ),
            jopt(
                "export",
                "boolean",
                "Accepted; here scene.saveAs always exports.",
            ),
            jopt(
                "animation",
                "string",
                "USD: on (default; time samples once a frame over the scene range), off, or current (the current frame only).",
            ),
        ],
        )
        .args(&[
            opt("filename", ArgType::Path, N, None),
            opt("format", EXPORT_FORMATS, N, None),
            opt("export", BOOL, N, Some("false")),
            opt("animation", ArgType::Choice(&["on", "off", "current"]), N, None),
        ]).side_effect(),
    script("scene.close", Files, "Closes the current scene (asks first when it has unsaved changes).").side_effect(),
    script("scene.closeAll", Files, "Asks to close every open scene.").side_effect(),
    // USD import (docs/BEHAVIOR-USD-IMPORT.md): Operandi's own name, a
    // placeholder until the registry thread rules on it.
    script("operandi.usd.import", Files, "Opens a USD file (.usd, .usda, .usdc, .usdz) as a new unsaved scene with these import options and shows the import report: upAxis file/y/z, units file/m/cm/mm/in/ft, materials materialx/preview/none, cameras, lights and graphs (the node graph sidecar, graphsFile or <name>_graphs.json beside it) true/false. Without a filename opens the file panel, whose USD files ask the Import USD options.")
        .operandi()
        .args(&[
            opt("filename", ArgType::Path, N, None),
            opt("upAxis", ArgType::Choice(&["file", "y", "z"]), N, Some("file")),
            opt("units", ArgType::Choice(&["file", "m", "cm", "mm", "in", "ft"]), N, Some("file")),
            opt("materials", ArgType::Choice(&["materialx", "preview", "none"]), N, Some("materialx")),
            opt("cameras", BOOL, N, Some("true")),
            opt("lights", BOOL, N, Some("true")),
            opt("graphs", BOOL, N, Some("true")),
            opt("graphsFile", ArgType::Path, N, None),
            opt("instances", ArgType::Choice(&["instances", "copies"]), N, Some("instances")),
        ])
        .side_effect(),
    script("scene.importReference", Files, "Adds a scene or model file's items (Operandi or Modo scene, OBJ, STL, glTF, GEO, PLY, FBX, 3MF, Alembic, USD; picked by extension) to the current scene, selected, as one undo step; meshes/shaders/lights/environments/cameras:false leave those out. Without a filename opens the import panel. scene.open opens a file as its own scene.")
        .args(&[
            opt("filename", ArgType::Path, N, None),
            opt("meshes", BOOL, N, Some("true")),
            opt("shaders", BOOL, N, Some("true")),
            opt("lights", BOOL, N, Some("true")),
            opt("environments", BOOL, N, Some("true")),
            opt("cameras", BOOL, N, Some("true")),
        ]).undo(),
    // ---------------------------------------------------------- history
    script("app.undo", Modeling, "Undoes the last step.").spelled("Undo").quiet(),
    script("app.redo", Modeling, "Redoes the last undone step.").spelled("Redo").quiet(),
    action("Apply", Tools, "Applies the active tool.")
        .modo("tool.apply")
        .aliases(&[
            "Bridge Apply",
            "Cone Apply",
            "Ellipsoid Apply",
            "Spike Apply",
            "Torus Apply",
            "Capsule Apply",
            "Arc Apply",
            "Tube Apply",
            "Smooth Shift Apply",
            "Thicken Apply",
            "Apply Last Cube",
        ])
        .undo(),
    action("Drop", Tools, "Drops the active tool.")
        .modo("tool.drop")
        .aliases(&["Drop Cube", "Smooth Shift Drop", "Thicken Drop"])
        .undo(),
    action("Cancel", Tools, "Cancels the active tool's edit.")
        .operandi()
        .aliases(&["Smooth Shift Cancel", "Thicken Cancel"]).undo(),
    action("Continue", Tools, "Starts the last tool again.")
        .operandi()
        .aliases(&["Repeat"])
        .tool(),
    action("Tool Properties", Tools, "Opens Tool Properties.")
        .operandi().ui(),
    // ---------------------------------------------------------- tool starters
    action("Cube", Tools, "Starts the Cube tool.")
        .modo("tool.set prim.cube on").aliases(&["Box"]).tool(),
    action("Sphere", Tools, "Starts the Sphere tool.")
        .modo("tool.set prim.sphere on").tool(),
    action("Cylinder", Tools, "Starts the Cylinder tool.")
        .modo("tool.set prim.cylinder on").tool(),
    action("Cone", Tools, "Starts the Cone tool.")
        .modo("tool.set prim.cone on").tool(),
    action("Ellipsoid", Tools, "Starts the Ellipsoid tool.")
        .modo("tool.set prim.ellipsoid on").tool(),
    action("Ellipsoid Reset", Tools, "Resets the Ellipsoid tool.")
        .modo("tool.reset").undo(),
    action("Ellipsoid Unit", Modeling, "Adds a unit ellipsoid to the active mesh.")
        .operandi().undo(),
    action("Ellipsoid New Unit", Modeling, "Adds a unit ellipsoid as a new mesh.")
        .operandi().undo(),
    // Layouts stage 3 batch 7 (LAY-Q42): the Create tab's Shift / Ctrl
    // variants as command lines, named like Ellipsoid's.
    action("Cube Unit", Modeling, "Adds a 1 m cube to the active mesh at its origin, no drag (the Create tab's Ctrl-click).")
        .operandi().undo(),
    action("Cube New Unit", Modeling, "Adds a 1 m cube as a new mesh item at the scene origin, no drag (the Create tab's Shift-click).")
        .operandi().undo(),
    action("Sphere Unit", Modeling, "Adds a 1 m sphere to the active mesh at its origin, no drag (the Create tab's Ctrl-click).")
        .operandi().undo(),
    action("Sphere New Unit", Modeling, "Adds a 1 m sphere as a new mesh item at the scene origin, no drag (the Create tab's Shift-click).")
        .operandi().undo(),
    action("Cylinder Unit", Modeling, "Adds a 1 m cylinder to the active mesh at its origin, no drag (the Create tab's Ctrl-click).")
        .operandi().undo(),
    action("Cylinder New Unit", Modeling, "Adds a 1 m cylinder as a new mesh item at the scene origin, no drag (the Create tab's Shift-click).")
        .operandi().undo(),
    action("Cone Unit", Modeling, "Adds a 1 m cone to the active mesh at its origin, no drag (the Create tab's Ctrl-click).")
        .operandi().undo(),
    action("Cone New Unit", Modeling, "Adds a 1 m cone as a new mesh item at the scene origin, no drag (the Create tab's Shift-click).")
        .operandi().undo(),
    action("Torus Unit", Modeling, "Adds a 1 m torus to the active mesh at its origin, no drag (the Create tab's Ctrl-click).")
        .operandi().undo(),
    action("Torus New Unit", Modeling, "Adds a 1 m torus as a new mesh item at the scene origin, no drag (the Create tab's Shift-click).")
        .operandi().undo(),
    action("Capsule Unit", Modeling, "Adds a 1 m capsule to the active mesh at its origin, no drag (the Create tab's Ctrl-click).")
        .operandi().undo(),
    action("Capsule New Unit", Modeling, "Adds a 1 m capsule as a new mesh item at the scene origin, no drag (the Create tab's Shift-click).")
        .operandi().undo(),
    action("Torus", Tools, "Starts the Torus tool.")
        .modo("tool.set prim.toroid on").tool(),
    action("Capsule", Tools, "Starts the Capsule tool.")
        .modo("tool.set prim.capsule on").tool(),
    action("Arc", Tools, "Starts the Arc tool.")
        .modo("tool.set prim.arc on").tool(),
    action("Tube", Tools, "Starts the Tube tool.")
        .modo("tool.set prim.tube on").tool(),
    // Solid Sketch (spec solid-sketch v0.3): the button and menu run the
    // SS Default preset (Subdiv Mode Level 1); `tool.set solid.sketch on`
    // is the bare tool (Level 2).
    action("Solid Sketch", Tools, "Starts Solid Sketch with the SS Default preset: click to drop nodes joined into one closed, subdivided mesh.")
        .modo("tool.set \"SS Default\" on").tool(),
    action("Text", Tools, "Starts the Text tool.")
        .modo("tool.set prim.text on").tool(),
    action("Teapot", Tools, "Starts the Teapot tool: the Utah Teapot, sized by its longest side (missing-tools-g2 MT6).")
        .operandi().tool(),
    action("Teapot Item", Modeling, "Adds a 1 m Utah Teapot as a new mesh item at the scene origin, no drag (MT6-05).")
        .operandi().undo(),
    action("Plane Item", Modeling, "Adds a 1 m square plane facing up as a new mesh item at the scene origin, no drag (the Text button's Shift-click, MT5).")
        .operandi().undo(),
    action("Torus Reset", Tools, "Resets the active Torus, Capsule, Arc, Tube or Text tool.")
        .modo("tool.reset")
        .aliases(&["Capsule Reset", "Arc Reset", "Tube Reset", "Text Reset", "Teapot Reset"]).undo(),
    action("Bevel", Tools, "Starts Bevel.")
        .modo("tool.set poly.bevel on").tool(),
    // Twist Bevel (twist-bevel-2026-11-15): the spec's own id, a plain
    // modelling-tool name rather than an operandi.* one.
    action("Twist Bevel", Tools, "Starts Twist Bevel: bevel polygons out into a tube that can taper, twist and bend.")
        .modo("tool.set poly.twistBevel on").tool(),
    action("Chamfer Edit", Tools, "Starts Chamfer Edit.")
        .modo("tool.set chamfer.edit on").tool(),
    action("Poly Haul", Tools, "Starts Poly Haul.")
        .modo("tool.set poly.haul on").tool(),
    action("Sketch Extrude", Tools, "Starts Sketch Extrude.")
        .modo("tool.set poly.sweep on").tool(),
    action("Extrude", Tools, "Starts Extrude.")
        .modo("tool.set poly.extrude on").tool(),
    action("Smooth Shift", Tools, "Starts Smooth Shift; with an attribute and value, sets it on the live tool.")
        .modo("tool.set poly.smshift on")
        .aliases(&["poly.smshift"])
        .args(&[opt("attr", SMOOTH_SHIFT_ATTR, N, None), opt("value", TEXT, N, None)])
        .tool(),
    action("Thicken", Tools, "Starts Thicken; with an attribute and value, sets it on the live tool.")
        .modo("tool.set Thicken on")
        .args(&[opt("attr", SMOOTH_SHIFT_ATTR, N, None), opt("value", TEXT, N, None)])
        .tool(),
    action("Smooth Shift Reset", Tools, "Resets Smooth Shift or Thicken.")
        .modo("tool.reset").aliases(&["Thicken Reset"]).undo(),
    action("Smooth Shift Clone", Tools, "Clones the Smooth Shift result.")
        .operandi()
        .aliases(&["Thicken Clone"])
        .undo(),
    action("Spike", Tools, "Starts Spike.")
        .modo("tool.set poly.spikey on").tool(),
    action("Spike Reset", Tools, "Resets Spike.")
        .modo("tool.reset").undo(),
    action("Spike Strength", Tools, "Sets the live Spike's strength.")
        .operandi()
        .args(&[arg("value", FLOAT, Unit::Factor)])
        .example("spike strength 1").undo(),
    action("Bridge", Tools, "Starts Bridge; with an attribute and value, sets it on the live tool.")
        .modo("tool.set edge.bridge on")
        .args(&[opt("attr", BRIDGE_ATTR, N, None), opt("value", TEXT, N, None)])
        .tool(),
    action("Bridge Reset", Tools, "Resets Bridge.")
        .modo("tool.reset").undo(),
    action("Polygon Reduction", Tools, "Arms Polygon Reduction.")
        .modo("tool.set poly.reduct on").tool(),
    action("Polygon Pen", Tools, "Starts the interactive Polygon Pen.")
        .modo("tool.set prim.pen on")
        .aliases(&["Pen Interactive"])
        .tool(),
    action("Transform", Tools, "Starts Transform.")
        .operandi().tool(),
    action("Move", Tools, "Starts Move.")
        .operandi().tool(),
    action("Rotate", Tools, "Starts Rotate.")
        .operandi().tool(),
    action("Scale", Tools, "Starts Scale.")
        .operandi().tool(),
    action("Extend", Tools, "Starts Edge Extend.")
        .modo("tool.set edge.extend on").tool(),
    action("Edge Extrude", Tools, "Starts Edge Extrude.")
        .modo("tool.set edge.extrude on").tool(),
    action("Slide", Tools, "Starts Edge Slide.")
        .modo("tool.set edge.slide on").tool(),
    action("Edit Slide", Tools, "The Edit tab's Slide: Edge Slide in Edge mode, Vertex Slide in Vertex mode; unavailable in Polygon and Item mode (Polygon Slide isn't built).")
        .operandi().tool(),
    action("Edge Weight", Tools, "Starts Edge Weight.")
        .operandi().aliases(&["Edge Crease"]).tool(),
    action("Edge Bevel", Tools, "Starts Edge Bevel.")
        .modo("tool.set edge.bevel on").tool(),
    action("Edge Chamfer", Tools, "Starts Edge Chamfer.")
        .modo("tool.set edge.chamfer on").tool(),
    action("Edge Relax", Tools, "Starts Edge Relax.")
        .modo("tool.set edge.relax on").tool(),
    action("Create", Tools, "Starts the Create vertex tool.")
        .modo("tool.set prim.makeVertex on").tool(),
    action("Vertex Bevel", Tools, "Starts Vertex Bevel.")
        .modo("tool.set vert.bevel on").tool(),
    action("Vertex Extrude", Tools, "Starts Vertex Extrude.")
        .modo("tool.set vert.extrude on").tool(),
    action("Merge Tool", Tools, "Starts the vertex Merge tool.")
        .modo("tool.set vert.merge on").tool(),
    action("Tear", Tools, "Starts Tear.")
        .modo("@ppi_vert_tear.py").tool(),
    action("Vertex Slide", Tools, "Starts Vertex Slide.")
        .modo("tool.set vert.slide on").tool(),
    action("Retopo Edge Lock", Modeling, "Adds the selected guide curves to the RetopoEdgeLock selection set (TC-53).")
        .operandi().undo(),
    action("Retopo Edge Flow", Modeling, "Adds the selected guide curves to the RetopoFlow selection set (TC-53).")
        .operandi().undo(),
    action("Topology Pen", Tools, "Starts Topology Pen: drag an element to move it; Shift+drag an edge to pull out a new strip onto the background.")
        .modo("tool.set mesh.topology on").tool(),
    action("Tack Tool", Tools, "Turns the Tack Tool on or off: click a polygon to put the selection on it.")
        .operandi().ui(),
    action("Mesh Paint", Tools, "Turns Mesh Paint on or off: paint copies of a background mesh onto the selected mesh (missing-tools-g6).")
        .operandi().ui(),
    action("Mesh Paint Instance", Tools, "Turns Mesh Paint Instance on or off: Mesh Paint making instance items (missing-tools-g6).")
        .operandi().ui(),
    action("Guides", Tools, "Turns the Guides tool on or off: up to ten construction lines that draw in the views and that points snap to (missing-tools-g9 GD-0).")
        .operandi().ui(),
    action("Mesh Paint Replica", Tools, "Turns Mesh Paint Replica on or off: Mesh Paint making one Replicator per stroke (missing-tools-g8 RP-7).")
        .operandi().ui(),
    action("Retopo Sketch", Tools, "Turns Retopo Sketch on or off: drag across a background mesh to draw a guide curve on it.")
        .operandi().ui(),
    action("Axis Rotate", Tools, "Starts Axis Rotate: Rotate about the work plane's normal only.")
        .operandi().tool(),
    action("Element Move", Tools, "Starts Element Move.")
        .modo("tool.set ElementMove on").aliases(&["Tweak"]).tool(),
    action("Weld", Tools, "Toggles drag-to-weld.")
        .modo("tool.set DragWeld on").tool(),
    action("Drop Weld", Tools, "Turns drag-to-weld off; unavailable while Weld is off.")
        .operandi().tool(),
    action("Add Loop", Tools, "Arms Add Loop, or disarms it: click an edge to add a loop (Ctrl: middle).")
        .operandi().tool(),
    action("Add Point", Tools, "Arms Add Point, or disarms it: click an edge to add a point (Ctrl: middle).")
        .operandi().tool(),
    action("Push", Tools, "Starts Push.")
        .modo("tool.set xfrm.push on").tool(),
    action("Smooth", Tools, "Starts Smooth.")
        .modo("tool.set xfrm.smooth on").tool(),
    action("Jitter", Tools, "Starts Jitter: moves each selected point by a seeded random offset within Range X/Y/Z.")
        .modo("tool.set xfrm.jitter on").tool(),
    action("Quantize", Tools, "Starts Quantize: snaps the selected points to a world grid of Step X/Y/Z.")
        .modo("tool.set xfrm.quantize on").tool(),
    action("Radial Align", Tools, "Starts Radial Align: puts the selected loop, points or polygon border on a circle, arc or regular polygon.")
        .modo("tool.set xfrm.radialAlign on").aliases(&["Radial"]).tool(),
    action("Linear Align", Tools, "Starts Linear Align: lines up each selected run between its two end points.")
        .modo("tool.set xfrm.linearAlign on").aliases(&["Linear"]).tool(),
    action("Soft Move", Tools, "Starts the Soft Move deform preset.")
        .modo("tool.set xfrm.magnet on").tool(),
    action("Shear", Tools, "Starts the Shear deform preset.")
        .modo("tool.set xfrm.shear on").tool(),
    action("Twist", Tools, "Starts the Twist deform preset.")
        .modo("tool.set xfrm.twist on").tool(),
    action("Swirl", Tools, "Starts the Swirl deform preset.")
        .modo("tool.set Swirl on").tool(),
    action("Taper", Tools, "Starts the Taper deform preset.")
        .modo("tool.set xfrm.taper on").tool(),
    action("Bulge", Tools, "Starts the Bulge deform preset.")
        .modo("tool.set xfrm.pole on").tool(),
    action("Flare", Tools, "Starts the Flare deform preset: Push with a Linear falloff fitted to the selection.")
        .modo("tool.set Flare on").tool(),
    action("Bend", Tools, "Starts Bend: curls the selection into an arc along its spine.")
        .modo("tool.set Bend on").tool(),
    action(
        "Symmetry Tool",
        Tools,
        "Starts the Symmetry Tool: drag to raise Threshold; points whose mirror partner is within it snap to match.",
    )
    .operandi()
    .tool(),
    action("Element Push", Tools, "Starts Element Push: Push with an Element falloff; drag the element under the pointer along its normal.")
        .modo("tool.set ElementPush on").tool(),
    action("Soft Drag", Tools, "Starts Soft Drag: Move with a Screen falloff around the pointer.")
        .modo("tool.set xfrm.softDrag on").tool(),
    action("Vortex", Tools, "Starts Vortex: Rotate with a Cylinder falloff; the middle turns most.")
        .modo("tool.set xfrm.vortex on").tool(),
    action("Soft Selection Move", Tools, "Starts Soft Selection Move: Move with a Soft Selection falloff about the selection.")
        .modo("tool.set SoftSelectionMove on").tool(),
    action("Soft Selection Rotate", Tools, "Starts Soft Selection Rotate: Rotate with a Soft Selection falloff about the selection.")
        .modo("tool.set SoftSelectionRotate on").tool(),
    action("Soft Selection Scale", Tools, "Starts Soft Selection Scale: Scale with a Soft Selection falloff about the selection.")
        .modo("tool.set SoftSelectionScale on").tool(),
    action("Soft Selection Transform", Tools, "Starts Soft Selection Transform: Transform with a Soft Selection falloff about the selection.")
        .modo("tool.set SoftSelectionTransform on").tool(),
    action("Flex", Tools, "Starts Flex: Transform with a Selection falloff about the selection's border.")
        .modo("tool.set Flex on").tool(),
    action("Loop Slice", Tools, "Starts Loop Slice.")
        .modo("tool.set poly.loopSlice on").tool(),
    action("Edge Slice", Tools, "Starts Edge Slice.")
        .modo("tool.set edge.knife on").tool(),
    action("Polygon Slice", Tools, "Starts Polygon Slice.")
        .modo("tool.set poly.knife on").tool(),
    action("Inset", Tools, "Starts Inset.")
        .modo("tool.set poly.inset on").tool(),
    action("Shift", Tools, "Starts Shift.")
        .modo("tool.set poly.shift on").tool(),
    action("Curve Slice", Tools, "Starts Curve Slice.")
        .modo("tool.set {Curve Slice} on").tool(),
    action("Pen Slice", Tools, "Starts Pen Slice.")
        .modo("tool.set PenKnife on").aliases(&["PenKnife"]).tool(),
    action("Primitive Slice", Tools, "Starts Primitive Slice.")
        .modo("tool.set {Primitive Slice} on").tool(),
    action("Contour", Tools, "Starts Contour: drag across a background mesh to lay a curve loop on it (missing-tools-g4).")
        .operandi()
        .tool(),
    action("Polygon Axis Slice", Tools, "Starts Polygon Axis Slice.")
        .modo("tool.set poly.julienne on")
        .aliases(&["Axis Slice"])
        .tool(),
    action("Mirror", Tools, "Starts Mirror.")
        .modo("tool.set *.mirror on").tool(),
    action("Radial Sweep", Tools, "Starts Radial Sweep.")
        .modo("tool.set {Radial Sweep} on").aliases(&["Lathe"]).tool(),
    action("Clone", Tools, "Starts Clone.")
        .modo("tool.set *.clone on").tool(),
    action("Array", Tools, "Starts Array.")
        .modo("tool.set *.array on").tool(),
    action("Radial Array", Tools, "Starts Radial Array.")
        .modo("tool.set {*.Radial Array} on").tool(),
    // The path tools' buttons (path_tools.rs, CX-01; ids provisional, CX-Q6).
    action("curve.extrude", Tools, "Starts the bare Curve Extrude tool (the buttons are its presets).")
        .operandi().tool(),
    action("curve.clone", Tools, "Starts the bare Curve Clone tool (the buttons are its presets).")
        .operandi().tool(),
    action("Symmetrize", Tools, "Starts Symmetrize: deletes one side of the mesh and mirrors the other side over it (DX-40).")
        .operandi().tool(),
    // The Duplicate tab's Instance buttons (DX-30, duplicate_items.rs).
    action("Instance Clone", Tools, "Starts Instance Clone: Mesh Instances of the selected items (DX-30).")
        .tool().operandi(),
    action("Instance Array", Tools, "Starts Instance Array: Mesh Instances of the selected items (DX-30).")
        .tool().operandi(),
    action("Instance Radial Array", Tools, "Starts Instance Radial Array: Mesh Instances of the selected items (DX-30).")
        .tool().operandi(),
    action("Instance Mirror", Tools, "Starts Instance Mirror: Mesh Instances of the selected items (DX-30).")
        .tool().operandi(),
    action("Instance Scatter", Tools, "Starts Instance Scatter: Mesh Instances of the selected items (DX-30).")
        .tool().operandi(),
    action("Curve Instance", Tools, "Starts Curve Instance: Mesh Instances of the selected items along a path (DX-30).")
        .tool().operandi(),
    action("Bezier Instance", Tools, "Starts Bezier Instance: Mesh Instances of the selected items along a path (DX-30).")
        .tool().operandi(),
    action("B-Spline Instance", Tools, "Starts B-Spline Instance: Mesh Instances of the selected items along a path (DX-30).")
        .tool().operandi(),
    action("Pen Instance", Tools, "Starts Pen Instance: Mesh Instances of the selected items along a path (DX-30).")
        .tool().operandi(),
    // The Duplicate tab's Replica buttons (missing-tools-g8 RP-6,
    // replicator.rs): the Instance buttons' placement, one Replicator.
    action("Replica Clone", Tools, "Starts Replica Clone: one Replicator of the selected items, a point per copy (missing-tools-g8 RP-6).")
        .tool().operandi(),
    action("Curve Replica", Tools, "Starts Curve Replica: one Replicator of the selected items, a point per copy (missing-tools-g8 RP-6).")
        .tool().operandi(),
    action("Bezier Replica", Tools, "Starts Bezier Replica: one Replicator of the selected items, a point per copy (missing-tools-g8 RP-6).")
        .tool().operandi(),
    action("B-Spline Replica", Tools, "Starts B-Spline Replica: one Replicator of the selected items, a point per copy (missing-tools-g8 RP-6).")
        .tool().operandi(),
    action("Pen Replicate", Tools, "Starts Pen Replicate: one Replicator of the selected items, a point per copy (missing-tools-g8 RP-6).")
        .tool().operandi(),
    action("Replica Scatter", Tools, "Starts Replica Scatter: one Replicator of the selected items, a point per copy (missing-tools-g8 RP-6).")
        .tool().operandi(),
    action("Replica Array", Tools, "Starts Replica Array: one Replicator of the selected items, a point per copy (missing-tools-g8 RP-6).")
        .tool().operandi(),
    action("Replica Radial Array", Tools, "Starts Replica Radial Array: one Replicator of the selected items, a point per copy (missing-tools-g8 RP-6).")
        .tool().operandi(),
    action("Replica Mirror", Tools, "Starts Replica Mirror: one Replicator of the selected items, a point per copy (missing-tools-g8 RP-6).")
        .tool().operandi(),
    action("Scatter Clone", Tools, "Starts Scatter Clone: copies scattered at random in a box (GO-01).")
        .modo("tool.set scatter.clone on").operandi().tool(),
    // The Duplicate tab's Transform buttons (DX-20): each moves the selected
    // items onto its placer's spots (duplicate_transform.rs).
    action("Transform Array", Tools, "Starts Transform Array: moves the selected items onto an Array grid's cells.")
        .modo("tool.set TransformArray on").tool().operandi(),
    action("Transform Radial Array", Tools, "Starts Transform Radial Array: moves the selected items round a circle.")
        .modo("tool.set TransformRadialArray on").tool().operandi(),
    action("Transform Scatter", Tools, "Starts Transform Scatter: moves the selected items onto scattered places.")
        .modo("tool.set TransformScatter on").tool().operandi(),
    action("Curve Transform", Tools, "Starts Curve Transform: moves the selected items along a smooth curve.")
        .modo("tool.set CurveTransform on").tool().operandi(),
    action("Bezier Transform", Tools, "Starts Bezier Transform: moves the selected items along a Bezier path.")
        .modo("tool.set BezierTransform on").tool().operandi(),
    action("B-Spline Transform", Tools, "Starts B-Spline Transform: moves the selected items along a B-Spline path.")
        .modo("tool.set BSplineTransform on").tool().operandi(),
    action("Transform Pen", Tools, "Starts Transform Pen: moves the selected items to the points of a straight-line path.")
        .modo("tool.set PenTransform on").tool().operandi(),
    // UV tab live tools (BEHAVIOR-UV-TOOLS §2-4, UVS-1 §1, §1.5, §3).
    action("UV Project", Tools, "Starts Project: writes the selected polygons' UVs from a Planar, Cylindrical, Spherical, Barycentric or Cubic projection (UVT-10..18).")
        .modo("tool.set operandi.uv.project on").operandi().tool(),
    action("Project from View", Tools, "Starts Project from View: a click in a 3D view writes the selected polygons' screen positions as UVs (UVT-19).")
        .modo("tool.set operandi.uv.viewProject on").operandi().tool(),
    action("Unwrap Tool", Tools, "Starts UV Unwrap as a live tool: unwraps every visible polygon along the seams; drag for Iterations.")
        .modo("tool.set uv.unwrap on").tool(),
    action("UV Relax", Tools, "Starts UV Relax: relaxes the map's islands to take out stretch; drag for Iterations.")
        .modo("tool.set uv.relax on").tool(),
    action("Unwrap & Relax", Tools, "Starts Unwrap & Relax: Unwrap, then Relax the result, in one tool.")
        .modo("tool.set uv.pelt on").aliases(&["Unwrap and Relax"]).tool(),
    action("Peeler", Tools, "Starts Peeler: lays clean quad strips out as a regular grid, cut at the selected edges (UVT-30).")
        .modo("tool.set operandi.uv.peeler on").operandi().tool(),
    action("Box Transform", Tools, "Starts Box Transform (UV view): move, turn and scale the selected UVs by a box round them (UVT-43).")
        .modo("tool.set operandi.uv.boxTransform on").operandi().tool(),
    action("Curve Extrude", Tools, "Starts Curve Extrude: extrude polygons along a smooth curve.")
        .modo("tool.set CurveExtrude on").tool().operandi(),
    action("Bezier Extrude", Tools, "Starts Bezier Extrude: extrude polygons along a Bezier path.")
        .modo("tool.set BezierExtrude on").tool().operandi(),
    action("B-Spline Extrude", Tools, "Starts B-Spline Extrude: extrude polygons along a B-Spline path.")
        .modo("tool.set BSplineExtrude on").tool().operandi(),
    action("Pen Extrude", Tools, "Starts Pen Extrude: extrude polygons along a straight-line path, one section per point.")
        .modo("tool.set PenExtrude on").tool().operandi(),
    action("Curve Clone", Tools, "Starts Curve Clone: copies along a smooth curve.")
        .modo("tool.set CurveClone on").tool().operandi(),
    action("Bezier Clone", Tools, "Starts Bezier Clone: copies along a Bezier path.")
        .modo("tool.set BezierClone on").tool().operandi(),
    action("B-Spline Clone", Tools, "Starts B-Spline Clone: copies along a B-Spline path.")
        .modo("tool.set BSplineClone on").tool().operandi(),
    action("Pen Clone", Tools, "Starts Pen Clone: copies at each point of a straight-line path.")
        .modo("tool.set PenClone on").tool().operandi(),
    action("Curve", Tools, "Starts the Curve draw tool.")
        .modo("tool.set prim.curve on").tool(),
    action("B-Spline", Tools, "Starts the B-Spline draw tool.")
        .modo("tool.set prim.bspline on").aliases(&["BSpline"]).tool(),
    action("Bezier Curve", Tools, "Starts the Bezier draw tool.")
        .modo("tool.set prim.bezier on").aliases(&["Bezier"]).tool(),
    action("Sketch", Tools, "Starts the Sketch draw tool.")
        .modo("tool.set prim.sketch on").tool(),
    action("Draw Patch", Tools, "Starts the Draw Patch tool.")
        .modo("tool.set patch.draw on").tool(),
    action("Quick Align", Tools, "Starts Quick Align.")
        .modo("tool.set QuickAlign on").tool(),
    action("Ground Align", Tools, "Starts Ground Align.")
        .modo("tool.set GroundAlign on").tool(),
    action("Ruler", Tools, "Starts the Ruler.")
        .modo("tool.set util.ruler on").tool(),
    action("Protractor", Tools, "Starts the Protractor.")
        .modo("tool.set util.protractor on").tool(),
    action("Dimensions", Tools, "Starts Dimensions.")
        .modo("tool.set util.bounds on").tool(),
    action("Paste Tool", Tools, "Toggles the Paste tool.")
        .modo("tool.set paste.tool on").tool(),
    action("Sculpt Smooth", Sculpt, "Starts the Smooth sculpt brush.")
        .modo("tool.set sculpt.smooth on").tool(),
    action("Sculpt Push", Sculpt, "Starts the Push sculpt brush.")
        .modo("tool.set sculpt.push on").tool(),
    action("Sculpt Inflate", Sculpt, "Starts the Inflate sculpt brush.")
        .modo("tool.set sculpt.inflate on").tool(),
    action("Sculpt Move", Sculpt, "Starts the Move sculpt brush.")
        .modo("tool.set sculpt.move on").tool(),
    action("Sculpt Fold", Sculpt, "Starts the Fold sculpt brush.")
        .modo("tool.set sculpt.fold on").tool(),
    action("Sculpt Clay", Sculpt, "Starts the Clay sculpt brush.")
        .modo("tool.set sculpt.clay on").tool(),
    action("Sculpt Smudge", Sculpt, "Starts the Smudge sculpt brush.")
        .modo("tool.set sculpt.smudge on").tool(),
    action("Sculpt Carve", Sculpt, "Starts the Carve sculpt brush.")
        .modo("tool.set sculpt.carve on").tool(),
    action("Sculpt Scar", Sculpt, "Starts the Scar sculpt brush.")
        .modo("tool.set sculpt.scar on").tool(),
    action("Sculpt Flatten", Sculpt, "Starts the Flatten sculpt brush.")
        .modo("tool.set sculpt.flatten on").tool(),
    action("Sculpt Tangent Pinch", Sculpt, "Starts the Tangent Pinch sculpt brush.")
        .modo("tool.set sculpt.tangentPinch on").tool(),
    action("Sculpt Center Pinch", Sculpt, "Starts the Center Pinch sculpt brush.")
        .modo("tool.set sculpt.centerPinch on").tool(),
    action("Sculpt Emboss", Sculpt, "Starts the Emboss sculpt brush.")
        .modo("tool.set sculpt.emboss on").tool(),
    action("Sculpt Attenuate", Sculpt, "Starts the Attenuate sculpt brush.")
        .modo("tool.set sculpt.shrink on").tool(),
    action("Sculpt Spin", Sculpt, "Starts the Spin sculpt brush.")
        .modo("tool.set sculpt.spin on").tool(),
    action("Vertex Map Paint", Sculpt, "Starts the Paint Vertex Map Tool.")
        .modo("tool.set sculpt.vmapPaint on").tool(),
    action("Vertex Map Smooth", Sculpt, "Starts the Smooth Vertex Map Tool.")
        .modo("tool.set sculpt.vmapSmooth on").tool(),
    action("Vertex Map Scale Down", Sculpt, "Starts the Scale Down Vertex Map Tool.")
        .modo("tool.set sculpt.vmapScale on").tool(),
    action("Vertex Map Erase", Sculpt, "Starts the Erase Vertex Map Tool.")
        .modo("tool.set sculpt.vmapErase on").tool(),
    action("Vertex Map Set Value", Sculpt, "Starts the Set Value Vertex Map Tool.")
        .modo("tool.set sculpt.vmapSetVal on").tool(),
    // ---------------------------------------------------------- commands
    script("cut", Modeling, "Cuts the selection to the clipboard.").spelled("Cut Selection").undo(),
    script("copy", Modeling, "Copies the selection to the clipboard.").spelled("Copy Selection").side_effect(),
    script("paste", Modeling, "Pastes the clipboard.").spelled("Paste Selection").undo(),
    script("layer.mergeMeshes", Items, "Merges the selected meshes into one.")
        .aliases(&["Merge Meshes"])
        .spelled("Item Merge")
        .undo(),
    script("layer.unmerge", Items, "Splits a mesh into its separate pieces.")
        .aliases(&["Unmerge Mesh"])
        .spelled("Item Unmerge")
        .undo(),
    action("Delete Item", Items, "Deletes the selection (in the Shading tab, the selected layers).")
        .operandi()
        .aliases(&["Delete Mesh"])
        .undo(),
    action("Delete Item Forward", Items, "Deletes the selection (forward delete key).")
        .operandi().undo(),
    action("Duplicate Hierarchy", Items, "Copies each selected item with all its children, keeping the links; the copies are selected.")
        .operandi().undo(),
    action("Duplicate Item", Items, "Copies the selected item in place, named Name (2), and selects the copy.")
        .operandi().undo(),
    action("New Mesh", Items, "Adds an empty mesh.")
        .operandi().undo(),
    action("Merge Instance", Items, "Bakes the selected mesh's one instance back into it, placed where the instance is; the instance item goes (missing-tools-g3 MT12).")
        .operandi().undo(),
    action("Instance", Items, "Adds an instance of each selected mesh item in the same place: it shares its source's mesh (DX-38).")
        .operandi().undo(),
    action("Freeze Replicator", Items, "Turns the selected Replicator into a group of instance items, one per copy, at the same places; the Replicator and its point cloud go (missing-tools-g8 RP-5).")
        .operandi().undo(),
    action("Flip", Modeling, "Reverses the selected polygons.")
        .modo("poly.flip").undo(),
    action("Flip It", Modeling, "Mirrors the selected polygons across an axis through the selection's centre, facing out; with no axis, asks for one.")
        .operandi()
        .args(&[opt("axis", ArgType::Choice(&["x", "y", "z"]), N, None)])
        .example("flip it x")
        .undo(),
    action("Flip It Origin", Modeling, "Mirrors the selected polygons across an axis through the world origin, facing out; with no axis, asks for one.")
        .operandi()
        .args(&[opt("axis", ArgType::Choice(&["x", "y", "z"]), N, None)])
        .example("flip it origin x")
        .undo(),
    action("Toggle Subdivision", Modeling, "Toggles subdivision surfaces on the selection.")
        .operandi().undo(),
    action("Toggle Catmull-Clark", Modeling, "Toggles Catmull-Clark subdivision on the selection.")
        .operandi().undo(),
    action("Freeze", Modeling, "Freezes curves and subdivision surfaces into plain geometry with the last Freeze settings.")
        .operandi().undo(),
    action("SDS Subdivide", Modeling, "Subdivides the selection once (Catmull-Clark).")
        .modo("poly.subdivide ccsds").undo(),
    action("SDS Subdivide 2X", Modeling, "Subdivides the selection twice (Catmull-Clark), as one undo step.")
        .operandi().undo(),
    action("Faceted Subdivide", Modeling, "Subdivides the selection once (faceted).")
        .modo("poly.subdivide flat").undo(),
    action("Smooth Subdivide", Modeling, "Subdivides the selection once (smooth).")
        .modo("poly.subdivide smooth").undo(),
    action("Subdivide Options", Modeling, "Opens the Subdivide options.")
        .modo("poly.subdivide").ui(),
    action("UnSubdivide", Modeling, "Opens the UnSubdivide dialog.")
        .modo("poly.unsubdiv").ui(),
    action("Center X", Modeling, "Centres the selection on X.")
        .modo("vert.center x").undo(),
    action("Center Y", Modeling, "Centres the selection on Y.")
        .modo("vert.center y").undo(),
    action("Center Z", Modeling, "Centres the selection on Z.")
        .modo("vert.center z").undo(),
    action("Center All", Modeling, "Centres the selection on all axes.")
        .modo("vert.center all").undo(),
    action("Set Part", Modeling, "Opens Polygon Set Part.")
        .modo("poly.setPart").ui(),
    action("Set Position", Modeling, "Opens Set Vertex Position.")
        .modo("vert.set").ui(),
    action("Absolute Scaling", Modeling, "Opens or closes the Absolute Scaling window.")
        .operandi().ui(),
    action("Snaps and Precision", Tools, "Opens or closes the Snaps and Precision panel.")
        .operandi().ui(),
    action("Preset Browser", Presets, "Opens or closes the Preset Browser's window (layout.window hash:presetBrowser).")
        .operandi().ui(),
    action("Preset Browser Popover", Presets, "Opens the Preset Browser as a popover under the pointer (F6).")
        .operandi().ui(),
    action("Drag Snap", Tools, "Element Move with snapping: drag an element onto another.")
        .modo("tool.set SnapDrag on").tool(),
    action("Drag Snap Rigid", Tools, "Drag Snap moving everything connected rigidly.")
        .modo("tool.set SnapDragRigid on").tool(),
    action("Planar Rotate Snap", Tools, "Rotate about the clicked element's normal, snapping a rest and a target point.")
        .modo("tool.set PlanarRotateSnap on").tool(),
    action("Planar Rotate Snap Axis", Tools, "Planar Rotate Snap about an axis set from two snapped points.")
        .modo("tool.set PlanarRotateSnapAxis on").tool(),
    action("Scale Absolute", Tools, "Scale by snapping a base handle and a scale handle to points.")
        .modo("tool.set ScaleAbsolute on").tool(),
    action("Size Absolute", Tools, "Scale from a snapped base size to a typed or snapped size.")
        .modo("tool.set SizeAbsolute on").tool(),
    action("Advanced Handles", Tools, "Sets the tool handle style to Advanced.")
        .modo("tool.handleStyle advanced").ui(),
    action("Set Material", Modeling, "Opens Set Material.")
        .modo("poly.setMaterial").ui(),
    action("Unify", Modeling, "Opens Unify Polygons.")
        .modo("poly.unify").ui(),
    action("Scale Offset", Modeling, "Opens Scale Offset.")
        .modo("@ppi_PolyScale_Offset.py").ui(),
    action("Flatten Ngons", Modeling, "Flattens nonplanar polygons.")
        .modo("@ppi_flatten_ngons.py").undo(),
    action("Split", Modeling, "Connects selected vertex pairs with edges.")
        .modo("poly.split").undo(),
    action("Quadruple", Modeling, "Quadruple on the selected polygons.")
        .modo("poly.quadruple").undo(),
    action("Triple Fan", Modeling, "Splits the selected polygons into triangles fanned from the selected vertex.")
        .operandi().undo(),
    action("Collapse Selection", Modeling, "Replaces the selected polygons with triangles around one new point at the middle of their outline.")
        .operandi().undo(),
    action("Detriangulate", Modeling, "Opens Detriangulate.")
        .operandi().ui(),
    action("Relax Mesh", Modeling, "Opens Relax Mesh: evens out the selected points (whole mesh with nothing selected); open borders stay put.")
        .operandi().aliases(&["Relax Mesh..."]).ui(),
    script("sculpt.relaxMesh", Modeling, "Relax Mesh with these values and no dialog; bare, opens the dialog.")
        .args(&[
            opt("iter", INT, N, Some("5")),
            opt("constrain", BOOL, N, Some("false")),
        ])
        .example("sculpt.relaxMesh iter:5 constrain:true")
        .undo(),
    action("UV Unwrap", Modeling, "Opens UV Unwrap (uv.unwrap; a dialog until the UV viewport exists).")
        .operandi().ui(),
    action("UV View", Modeling, "Shows UV space in the active view, or 3D again (UV spec v1.1 §4).")
        .operandi().ui(),
    action("Open UV Editor", Modeling, "Texture > Open UV Editor: the UV Editor palette, the selected UV map in its own window; again closes it (UVT-200, LAY-54).")
        .operandi().ui(),
    action("Export UVs to SVG...", Modeling, "Texture > Export UVs to SVG...: the UV map's edges as an SVG drawing of the 0-1 tile (operandi.uv.export).")
        .operandi().ui(),
    action("Export UVs to EPS...", Modeling, "Texture > Export UVs to EPS...: the UV map's edges as an EPS drawing of the 0-1 tile (operandi.uv.export).")
        .operandi().ui(),
    action("Pack UVs", Modeling, "Opens Pack UVs.")
        .modo("uv.pack").ui(),
    action("Remesh", Modeling, "Opens Remesh: Voxel (sculpt.remesh) or Blocks, Smooth and Sharp (operandi.remesh).")
        .operandi().ui(),
    action("Automatic Retopology", Modeling, "Opens Automatic Retopology (bare mesh.retopo).")
        .modo("mesh.retopo").ui(),
    action("Boolean", Modeling, "Opens Boolean (bare poly.boolean).")
        .modo("poly.boolean").ui(),
    action("Solid Drill", Modeling, "Opens Solid Drill (bare poly.solidDrill).")
        .modo("poly.solidDrill").ui(),
    action("Axis Drill", Modeling, "Opens Axis Drill (bare poly.drill).")
        .modo("poly.drill").ui(),
    action("Spin Quad", Modeling, "Spins the shared edge of two polygons.")
        .modo("poly.spinQuads").undo(),
    action("Polygon Align", Modeling, "Makes the selected polygons face the same way.")
        .modo("poly.align").undo(),
    action("Triple", Modeling, "Triangulates the selected polygons.")
        .modo("poly.triple").undo(),
    action("Remove", Modeling, "Removes the selected polygons.")
        .modo("poly.remove").undo(),
    action("Polygon Merge", Modeling, "Merges the selected polygons.")
        .modo("poly.merge").undo(),
    action("Polygon Collapse", Modeling, "Collapses the selected polygons.")
        .modo("poly.collapse").undo(),
    action("Make", Modeling, "Builds a quad from two boundary edges.")
        .modo("poly.make auto").undo(),
    action("Assign Selection Set", Selection, "Opens Assign Selection Set.")
        .modo("select.editSet").ui(),
    action("Use Selection Set", Selection, "Opens Use Selection Set.")
        .modo("select.useSet").ui(),
    action("Delete Selection Set", Selection, "Opens Delete Selection Set.")
        .modo("select.deleteSet").ui(),
    action("Create Selection Set Group", Selection, "Opens Create Selection Set Group.")
        .modo("select.convertGroup").ui(),
    action("Toggle Symmetry", Modeling, "Turns Symmetry off, or back on with the last axis (X the first time).")
        .operandi().ui(),
    action("Symmetrize Positive", Modeling, "Copies the positive side over the negative.")
        .operandi().undo(),
    action("Symmetrize Negative", Modeling, "Copies the negative side over the positive.")
        .operandi().undo(),
    action("Symmetrize Polygons Positive", Modeling, "Deletes the negative half, mirrors the positive half across and merges the seam.")
        .operandi().undo(),
    action("Symmetrize Polygons Negative", Modeling, "Deletes the positive half, mirrors the negative half across and merges the seam.")
        .operandi().undo(),
    action("Edge Subdivide", Modeling, "Subdivides the selected edges (count and percent when given).")
        .modo("edge.subdivide")
        .args(&[opt("count", INT, N, None), opt("percent", FLOAT, N, None)])
        .undo(),
    action("Grow Quads", Modeling, "Grow Quads.")
        .modo("edge.growQuads").undo(),
    action("Fill Quads", Modeling, "Fill Quads.")
        .modo("edge.fillQuads").undo(),
    action("Edge Split", Modeling, "Splits along the selected edges.")
        .modo("edge.split").undo(),
    action("Split Normals", Modeling, "Splits normals along the selected edges.")
        .modo("edge.splitNormals").undo(),
    // Vertex Normal Tools (missing-tools-g9 VN-0..VN-10, vertex_normals.rs).
    action("Vertex Normal Tools...", Modeling, "Opens the Vertex Normal Tools palette: Smoothing, Tools and Cage (missing-tools-g9 VN-0).")
        .operandi().ui(),
    action("Recalculate Vertex Normals", Modeling, "Rebuilds the vertex normal map of the selected meshes (creating it if missing), smooth except across hard edges (VN-2).")
        .operandi().undo(),
    action("Harden Vertex Normals", Modeling, "Adds edges to the HardEdges set and rebuilds the normal map: mode vertices, edges, polygons, angle, uvAuto, uvEdge or uvPolygon (VN-3).")
        .operandi()
        .args(&[
            opt("mode", ArgType::Choice(&["vertices", "edges", "polygons", "angle", "uvAuto", "uvEdge", "uvPolygon"]), N, Some("edges")),
            opt("softenOthers", BOOL, N, Some("false")),
            opt("connected", BOOL, N, Some("false")),
        ])
        .example("harden vertex normals mode:angle")
        .undo(),
    action("Soften Vertex Normals", Modeling, "Takes edges out of the HardEdges set and rebuilds the normal map: mode vertices, edges, polygons, or all (VN-4).")
        .operandi()
        .args(&[
            opt("mode", ArgType::Choice(&["vertices", "edges", "polygons", "all"]), N, Some("all")),
            opt("connected", BOOL, N, Some("false")),
        ])
        .example("soften vertex normals mode:all")
        .undo(),
    action("Select Hard Edges", Modeling, "Selects the mesh's hard edges: mode set, add or remove (VN-5).")
        .operandi()
        .args(&[opt("mode", ArgType::Choice(&["set", "add", "remove"]), N, Some("set"))])
        .example("select hard edges")
        .undo(),
    action("Select Soft Edges", Modeling, "Selects the mesh's soft edges: mode set, add or remove (VN-5).")
        .operandi()
        .args(&[opt("mode", ArgType::Choice(&["set", "add", "remove"]), N, Some("set"))])
        .example("select soft edges")
        .undo(),
    action("Toggle Vertex Normal Maps", Modeling, "Auto removes the selected meshes' normal maps if they have any, else recreates one; on recreates; off removes (VN-7).")
        .operandi()
        .args(&[opt("mode", ArgType::Choice(&["auto", "on", "off"]), N, Some("auto"))])
        .example("toggle vertex normal maps")
        .undo(),
    action("Transfer Normals", Modeling, "Copies normals from the background mesh onto the active mesh at the nearest surface point (VN-8).")
        .operandi()
        .args(&[opt("world", BOOL, N, Some("true"))])
        .example("transfer normals")
        .undo(),
    action("Convert to Hard Edges", Modeling, "Every edge where shading splits joins the HardEdges set; removeGroups deletes the smoothing groups after (VN-9).")
        .operandi()
        .args(&[opt("removeGroups", BOOL, N, Some("false"))])
        .example("convert to hard edges")
        .undo(),
    action("Rename Normal Maps", Modeling, "Renames the selected meshes' normal maps to the custom name in Vertex Normal Tools options (VN-10).")
        .operandi()
        .args(&[opt("name", TEXT, N, None)])
        .example("rename normal maps name:Custom")
        .undo(),
    action("Spin Polygon Vertex Indices", Modeling, "Turns the stored point order of each selected polygon by offset (negative the other way); shape unchanged, corner data follows (VN-10).")
        .operandi()
        .args(&[opt("offset", INT, N, Some("1"))])
        .example("spin polygon vertex indices offset:1")
        .undo(),
    action("Morph to Normals", Modeling, "Each corner normal points along the morph map's offset at its point (VN-10).")
        .operandi()
        .args(&[opt("morph", TEXT, N, None)])
        .example("morph to normals morph:Morph")
        .undo(),
    action("Edge Spin", Modeling, "Spins the selected edges.")
        .modo("edge.spinQuads").undo(),
    action("Join", Modeling, "Joins the selected edges.")
        .modo("edge.join").undo(),
    action("Join Averaged", Modeling, "Joins the selected edges at their average.")
        .modo("edge.join average:true").undo(),
    action("Edge Collapse", Modeling, "Collapses the selected edges.")
        .modo("edge.collapse").undo(),
    action("Edge Remove", Modeling, "Removes the selected edges.")
        .modo("edge.remove").undo(),
    action("Edges To Curves", Modeling, "Makes curves from the selected edges.")
        .modo("edges.toCurve").undo(),
    action("Unbevel", Modeling, "Unbevels the selected edges.")
        .modo("edge.unbevel").undo(),
    action("New Vertex", Modeling, "Opens Create New Vertex.")
        .modo("vert.new").ui(),
    action("Show Vertices", ViewportOptions, "Toggles vertex display.")
        .modo("view3d.showVertices").ui(),
    action("Replicators", ViewportOptions, "Steps the view's Replicators option: None (every copy a box), Selected (the selected Replicator's copies shaded), All (missing-tools-g8 RP-3).")
        .operandi().ui(),
    action("Vertex Split", Modeling, "Splits the selected points.")
        .modo("vert.split").undo(),
    action("Vertex Split Edge", Modeling, "Splits the selected points along edges.")
        .operandi().undo(),
    action("Vertex Set Position", Modeling, "Opens Set Vertex Position.")
        .modo("vert.set").ui(),
    action("Vertex Align", Modeling, "Opens Vertex Align.")
        .modo("vert.align").ui(),
    action("Vertex Merge", Modeling, "Opens Vertex Merge.")
        .modo("vert.merge").ui(),
    action("Vertex Join", Modeling, "Opens Vertex Join.")
        .modo("vert.join").ui(),
    action("Vertex Join Averaged", Modeling, "Joins the selected points at their average.")
        .modo("vert.join average:true").undo(),
    action("Vertex Collapse", Modeling, "Collapses the selected points.")
        .modo("vert.collapse").undo(),
    action("Vertex Remove", Modeling, "Removes the selected points.")
        .modo("vert.remove").undo(),
    action("Vertex Remove Edge", Modeling, "Removes the selected points and edges.")
        .operandi().undo(),
    action("Reorder Index", Modeling, "Opens Reorder Index.")
        .modo("vert.reorder").ui(),
    action("Sync Index Order", Modeling, "Syncs point order to polygon order.")
        .modo("vert.syncOrder").undo(),
    script("lock.sel", Modeling, "Locks the selected elements.").spelled("Lock Selected").undo(),
    script("lock.unsel", Modeling, "Locks the unselected elements.").spelled("Lock Unselected").undo(),
    script("lock.invert", Modeling, "Inverts which elements are locked.").spelled("Lock Invert").undo(),
    script("unlock", Modeling, "Unlocks every element.").undo(),
    // BRQ-110: Hide and Unhide run headless too (in the selection mode).
    script("hide.sel", Modeling, "Hides the selected elements.").spelled("Hide Selected")
        .headless("Hide the selected elements of the active mesh (everything when nothing is selected), one undo step; hidden polygons drop out of the UV tools.", &[])
        .undo(),
    script("hide.unsel", Modeling, "Hides the unselected elements.").spelled("Hide Unselected")
        .headless("Hide the elements of the active mesh that aren't selected, one undo step.", &[])
        .undo(),
    script("hide.invert", Modeling, "Inverts which elements are hidden.").spelled("Hide Invert")
        .headless("Swap which elements of the active mesh are hidden, one undo step.", &[])
        .undo(),
    script("unhide", Modeling, "Shows every hidden element.")
        .headless("Show every hidden element of the selected meshes (every mesh when none is selected), one undo step.", &[])
        .undo(),
    // ---------------------------------------------------------- selection
    action("Vertices", Selection, "Vertex selection mode.")
        .operandi().undo(),
    action("Edges", Selection, "Edge selection mode.")
        .operandi().undo(),
    action("Polygons", Selection, "Polygon selection mode.")
        .operandi().undo(),
    action("Items", Selection, "Item selection mode.")
        .operandi().undo(),
    action("Materials", Selection, "Toggles the polygon tag (Materials) selection mode.")
        .operandi().undo(),
    // CV-03 / UIC-66: Alt+Shift+Space (Space's old job before Mini
    // Properties). The mode argument isn't supported yet.
    script("select.nextMode", Selection, "Switches to the next selection mode (Vertices, Edges, Polygons).")
        .args(&[opt("mode", INT, N, None)])
        .spelled("Select Next Mode").undo(),
    script("select.more", Selection, "Selects more.").aliases(&["More"]).spelled("Select More").undo(),
    script("select.less", Selection, "Selects less.").aliases(&["Less"]).spelled("Select Less").undo(),
    script("select.between", Selection, "Selects between two selected elements.").aliases(&["Between"]).spelled("Select Between").undo(),
    action("Select Symmetrical", Selection, "Selects the symmetrical elements.")
        .modo("select.symmetrical").undo(),
    action("Selection Fill", Selection, "Explains Selection Fill (a Shift+Cmd click).")
        .operandi().undo(),
    action("Select Through", Selection, "Explains that Select Through is unavailable.")
        .operandi().undo(),
    action("Next Loop", Selection, "Selects the next loop.")
        .modo("select.loop next").undo(),
    action("Previous Loop", Selection, "Selects the previous loop.")
        .modo("select.loop prev").undo(),
    script("select.shortestPath", Selection, "Adds the shortest path between the first and last selected elements.")
        .spelled("Shortest Path").undo(),
    action("Shortest Path Mode", Selection, "Toggles Shortest Path Mode: Shift+click adds the element and the path to it.")
        .operandi().ui(),
    script("select.edgeSharp", Selection, "Selects edges whose polygons meet at a normal angle between min and max; bare, opens the dialog.")
        .aliases(&["Sharp..."])
        .args(&[opt("min", FLOAT, N, Some("89.55")), opt("max", FLOAT, N, Some("180"))])
        .undo(),
    script("select.edgeCoplanar", Selection, "Selects edges whose two polygons are flat within flatness; bare, opens the dialog.")
        .aliases(&["Coplanar Edges..."])
        .args(&[opt("flatness", FLOAT, N, Some("0.005"))])
        .undo(),
    script("select.vertexColinear", Selection, "Selects points on the line through their outline neighbours; bare, opens the dialog.")
        .aliases(&["Colinear Vertices..."])
        .args(&[opt("distance", FLOAT, N, Some("0.000001"))])
        .undo(),
    script("select.edgeDiagonal", Selection, "Selects the edges that would merge triangle pairs into quads; bare, opens the dialog.")
        .aliases(&["Diagonal..."])
        .args(&[opt("flatness", FLOAT, N, Some("20")), opt("max_angle", FLOAT, N, Some("170"))])
        .undo(),
    action("Coplanar Polygons Tool", Selection, "Opens the Coplanar Polygons Tool: grows the selected polygons to neighbours facing the same way.")
        .modo("tool.set select.polygonCoplanar on").ui(),
    script("select.conditionalLoop", Selection, "Grows the last selected edge into a loop until a stop rule ends it.")
        .spelled("Conditional Loop").undo(),
    action("Conditional Loop Tool", Selection, "Opens Conditional Loop with its stop rules.")
        .modo("tool.set seam.marker on").ui(),
    script("select.lazyState", Selection, "Toggles Lazy Selection: a click picks the nearest point or edge from further away.")
        .spelled("Lazy Selection").ui(),
    script("select.closeLoop", Selection, "Closes the selected loop.")
        .spelled("Close Loop").undo(),
    script("select.expand", Selection, "Grows the selection.")
        .spelled("Grow").undo(),
    script("select.contract", Selection, "Shrinks the selection.")
        .spelled("Shrink").undo(),
    script("select.loop", Selection, "Selects loops.")
        .spelled("Loop").undo(),
    script("select.ring", Selection, "Selects rings.")
        .spelled("Ring").undo(),
    script("select.all", Selection, "Selects everything.").spelled("Select All").undo(),
    script("select.drop", Selection, "Drops the selection of the selection mode (Esc with no tool on).")
        .spelled("Drop Selection").undo(),
    script("select.invert", Selection, "Inverts the selection.")
        .spelled("Invert").undo(),
    script("select.connect", Selection, "Selects connected elements.")
        .spelled("Connected").undo(),
    script("@ppi_invert_selection.py", Selection, "Inverts within connected pieces.")
        .spelled("Invert Connected").undo(),
    script("select.convert", Selection, "Converts the selection to vertices, edges or polygons.")
        .aliases(&["Convert to Edges", "Convert to Polygons"])
        .spelled("Convert to Vertices")
        .args(&[arg("type", ArgType::Choice(&["vertex", "edge", "polygon"]), N)])
        .example("select.convert edge").undo(),
    script("select.boundary", Selection, "Selects the selection's boundary.")
        .spelled("Boundary").undo(),
    action("Select Set", Selection, "Selects a polygon set by name.")
        .operandi()
        .args(&[arg("name", TEXT, N)])
        .example("select set Front").undo(),
    action("Falloff", Tools, "Sets the spatial falloff.")
        .operandi()
        .args(&[arg("type", ArgType::Choice(&["none", "linear", "cylinder", "radial", "airbrush", "screen", "element", "noise", "curvature", "vertex map", "selection", "soft selection", "lasso", "path", "image", "coplanar"]), N)])
        .example("falloff none").ui(),
    script("falloff.vertexMap", Tools, "Adds a Vertex Map falloff (tool.set falloff.vertexMap on).")
        .args(&[
            opt("mode", ArgType::Choice(&["magnitude", "component"]), N, Some("magnitude")),
            opt("index", INT, N, Some("0")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.invert", Tools, "Adds an Invert falloff (tool.set falloff.invert on).").tool(),
    script("falloff.selection", Tools, "Adds a Selection falloff: 0 on the selection's border, full Steps rings in (tool.set falloff.selection on).")
        .args(&[
            opt("steps", INT, N, Some("2")),
            opt("shape", ArgType::Choice(&["linear", "easeIn", "easeOut", "smooth", "custom"]), N, Some("smooth")),
            opt("p0", FLOAT, N, Some("0")),
            opt("p1", FLOAT, N, Some("0")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .example("falloff.selection steps:5 shape:easeOut")
        .tool(),
    script("falloff.softSelection", Tools, "Adds a Soft Selection falloff: the selection at full weight, fading out over Radius beyond it (tool.set falloff.softSelection on).")
        .operandi()
        .args(&[
            opt("radius", FLOAT, N, Some("0.1")),
            opt("connectivity", ArgType::Choice(&["true", "false"]), N, Some("true")),
            opt("shape", ArgType::Choice(&["linear", "easeIn", "easeOut", "smooth", "custom"]), N, Some("linear")),
            opt("p0", FLOAT, N, Some("0")),
            opt("p1", FLOAT, N, Some("0")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .example("falloff.softSelection radius:0.5")
        .tool(),
    script("falloff.element", Tools, "Adds an Element falloff: a press picks the element under the pointer and the drag moves it, with Range around it (tool.set falloff.element on).")
        .operandi()
        .args(&[
            opt("mode", ArgType::Choice(&["auto", "autoCenter", "vertex", "edge", "edgeCenter", "polygon", "polygonCenter"]), N, Some("auto")),
            opt("range", FLOAT, N, Some("0")),
            opt("connect", ArgType::Choice(&["ignore", "connectivity", "rigid", "edgeLoops"]), N, Some("ignore")),
            opt("shape", ArgType::Choice(&["linear", "easeIn", "easeOut", "smooth", "custom"]), N, Some("linear")),
            opt("p0", FLOAT, N, Some("0")),
            opt("p1", FLOAT, N, Some("0")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .example("falloff.element mode:polygon range:0.3")
        .tool(),
    script("falloff.coplanar", Tools, "Adds a Coplanar falloff: the selection plus neighbouring polygons facing within Angle of it (tool.set falloff.coplanar on).")
        .operandi()
        .args(&[
            opt("angle", FLOAT, N, Some("0")),
            opt("connectivity", ArgType::Choice(&["true", "false"]), N, Some("true")),
            opt("shape", ArgType::Choice(&["linear", "easeIn", "easeOut", "smooth", "custom"]), N, Some("linear")),
            opt("p0", FLOAT, N, Some("0")),
            opt("p1", FLOAT, N, Some("0")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .example("falloff.coplanar angle:10")
        .tool(),
    script("falloff.screen", Tools, "Adds a Screen falloff: a circle on screen, drawn in the view by press and drag (tool.set falloff.screen on).")
        .operandi()
        .args(&[
            opt("size", FLOAT, N, Some("20")),
            opt("transparent", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.lasso", Tools, "Adds a Lasso falloff: a shape drawn in the view (tool.set falloff.lasso on).")
        .operandi()
        .args(&[
            opt("style", ArgType::Choice(&["lasso", "rectangle", "circle", "ellipse"]), N, Some("rectangle")),
            opt("softBorder", FLOAT, N, Some("0")),
            opt("world", ArgType::Choice(&["true", "false"]), N, Some("true")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.noise", Tools, "Adds a Noise falloff: smooth 3D noise of position over Scale (tool.set falloff.noise on).")
        .operandi()
        .args(&[
            opt("scale", FLOAT, N, Some("1")),
            opt("world", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .example("falloff.noise scale:0.25")
        .tool(),
    script("falloff.image", Tools, "Adds an Image falloff: a picture's brightness, laid over the view or through the selected UV map; choose the picture in its panel (tool.set falloff.image on).")
        .operandi()
        .args(&[
            opt("repeat", ArgType::Choice(&["true", "false"]), N, Some("true")),
            opt("softBorder", FLOAT, N, Some("5")),
            opt("useUV", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("scale", FLOAT, N, Some("100")),
            opt("posX", FLOAT, N, Some("0")),
            opt("posY", FLOAT, N, Some("0")),
            opt("rotation", FLOAT, N, Some("0")),
            opt("world", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.path", Tools, "Adds a Path falloff: a tube of radius Size around a curve clicked in the view (tool.set falloff.path on).")
        .operandi()
        .args(&[
            opt("size", FLOAT, N, Some("1")),
            opt("world", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.curvature", Tools, "Adds a Curvature falloff: weights from how sharply the surface bends (tool.set falloff.curvature on).")
        .operandi()
        .args(&[
            opt("scale", FLOAT, N, Some("1")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.airbrush", Tools, "Adds an Airbrush falloff: weights painted by dragging in the view (tool.set falloff.airbrush on).")
        .operandi()
        .args(&[
            opt("size", FLOAT, N, Some("20")),
            opt("strength", FLOAT, N, Some("0.1")),
            opt("transparent", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    action("Symmetry", Tools, "Sets symmetry.")
        .operandi()
        .args(&[arg("axis", ArgType::Choice(&["off", "x", "y", "z"]), N)])
        .example("symmetry off").ui(),
    action("Action Center", Tools, "Sets the action centre (element and screen wait for a click).")
        .operandi()
        .args(&[arg(
            "center",
            ArgType::Choice(&[
                "automatic",
                "selection",
                "selection auto axis",
                "origin",
                "local",
                "element",
                "screen",
            ]),
            N,
        )])
        .example("action center origin").ui(),
    // ---------------------------------------------------------- view
    action("Top View", View, "Top view (again: bottom).")
        .operandi().ui(),
    action("Front View", View, "Front view (again: back).")
        .operandi().ui(),
    action("Right View", View, "Right view (again: left).")
        .operandi().ui(),
    action("Perspective View", View, "Perspective view.")
        .operandi().ui(),
    action("View", View, "Sets the active view's direction.")
        .operandi()
        .args(&[arg(
            "direction",
            ArgType::Choice(&[
                "top",
                "bottom",
                "front",
                "back",
                "right",
                "left",
                "perspective",
                "camera",
            ]),
            N,
        )])
        .example("view top").ui(),
    script("viewport.fit", View, "Frames everything (selected:true frames the selection).")
        .args(&[opt("selected", BOOL, N, Some("false"))])
        .spelled("Frame All").ui(),
    script("viewport.fitSelected", View, "Frames the selection.").spelled("Frame Selection").ui(),
    action("Zoom In", View, "Zooms in a step.")
        .operandi().ui(),
    action("Zoom Out", View, "Zooms out a step.")
        .operandi().ui(),
    action("Re-centre", View, "Moves the view centre to the point under the pointer.")
        .operandi()
        .aliases(&["Recentre"]).ui(),
    action("Turn", View, "Toggles turntable orbiting.")
        .operandi().ui(),
    action("Turn Upright", View, "Turns the view upright.")
        .operandi().ui(),
    action("Align Selected", View, "Aligns the view to the selection.")
        .operandi().ui(),
    action("Fit and Align Selected", View, "Fits and aligns the view to the selection.")
        .operandi().ui(),
    script("viewport.maximize", View, "Maximises the view (or restores it).")
        .aliases(&["Maximise Viewport (Alternate)"])
        .spelled("Maximise Viewport")
        .ui(),
    // Menus as forms: View > Split View and Tear Off View (name map
    // names; direction:, keep: and fromMouse: aren't read yet).
    script("viewport.split", View, "Splits the active view in two, side by side (View > Split View).")
        .ui(),
    script("viewport.detach", View, "Tears the active view off into a window of its own (View > Tear Off View); a window's only view stays.")
        .ui(),
    action("Viewport Properties", ViewportOptions, "Opens the view's Properties popover.")
        .operandi().ui(),
    action("Shaded View", ViewportOptions, "Shaded drawing style.")
        .operandi().ui(),
    action("Wireframe View", ViewportOptions, "Wireframe drawing style.")
        .operandi().ui(),
    action("Wireframe Overlay None", ViewportOptions, "No wireframe over active meshes.")
        .operandi().ui(),
    action("Wireframe Overlay Uniform", ViewportOptions, "Uniform wireframe over active meshes.")
        .operandi().ui(),
    action("Wireframe Overlay Colored", ViewportOptions, "Coloured wireframe over active meshes.")
        .operandi().ui(),
    // XR-2: the 3D view header's Transparency buttons (Normal, Ghost,
    // X-Ray; Alt+click for inactive meshes) as Input Map phrases.
    action("Transparency Normal", ViewportOptions, "This view's active meshes drawn solid (Transparency Overrides off).")
        .modo("view3d.TransparencyOverride Normal").ui(),
    action("Transparency Ghost", ViewportOptions, "This view's active meshes in Ghost: polygons facing you see-through and tinted.")
        .modo("view3d.TransparencyOverride Ghost").ui(),
    action("Transparency X-Ray", ViewportOptions, "This view's active meshes in X-Ray: polygons facing you clear.")
        .modo("view3d.TransparencyOverride X-Ray").ui(),
    action("Inactive Transparency Normal", ViewportOptions, "This view's inactive meshes drawn solid.")
        .modo("view3d.TransparencyOverride Normal bgmesh:1").ui(),
    action("Inactive Transparency Ghost", ViewportOptions, "This view's inactive meshes in Ghost.")
        .modo("view3d.TransparencyOverride Ghost bgmesh:1").ui(),
    action("Inactive Transparency X-Ray", ViewportOptions, "This view's inactive meshes in X-Ray.")
        .modo("view3d.TransparencyOverride X-Ray bgmesh:1").ui(),
    script("operandi.view3d.toggleXRay", ViewportOptions, "Toggles this view's active meshes between Normal and X-Ray (Ghost goes to X-Ray). No default key.")
        .spelled("Toggle X-Ray")
        .operandi()
        .ui(),
    action("Shading", ViewportOptions, "Sets the active meshes' drawing style.")
        .operandi()
        .args(&[arg(
            "style",
            ArgType::Choice(&[
                "wireframe",
                "shaded",
                "advanced",
                "texture",
                "default",
                "gooch",
                "vertex map",
                "reflection",
                "solid",
                "cel",
            ]),
            N,
        )])
        .ui()
        .example("shading shaded"),
    action("Toggle", ViewportOptions, "Toggles a viewport display option.")
        .operandi()
        .args(&[arg(
            "option",
            ArgType::Choice(&[
                "lights",
                "cameras",
                "wireframe overlay",
                "show verts",
                "grid",
                "background gradient",
            ]),
            N,
        )])
        .ui()
        .example("toggle grid"),
    action("Snapping Toggle", Tools, "Toggles snapping.")
        .operandi().ui(),
    // SPEC-BACKGROUND-CONSTRAINT: the reference tool const.bg (registry
    // thread 2026-11-13); its options are tool.attr const.bg geometry,
    // bkgOffset, negative, dblSided, handle and target.
    action(
        "Constrain to Background",
        Tools,
        "Turns Constrain to Background on or off: points tools move or create land on the background meshes.",
    )
    .modo("tool.set const.bg on").ui(),
    action("Snapping Options", Tools, "Opens Snapping Options.")
        .operandi().ui(),
    // BEHAVIOR-MESH-CONSTRAINT MC-03: Edit > Snapping > Mesh Constraints.
    action("Mesh Constraints", Tools, "Opens the Mesh Constraint popover.")
        .operandi().ui(),
    action("Element Snap", Tools, "Starts the Element Snap tool (Drag Snap) with snapping on.")
        .operandi().ui(),
    action("Pie: View Direction", View, "Opens the View Direction pie.")
        .operandi().ui(),
    action("Pie: Selection & Transforms", Selection, "Opens the Selection & Transforms pie.")
        .operandi().ui(),
    action("Pie: Shading Style", ViewportOptions, "Opens the Shading Style pie.")
        .operandi().ui(),
    action("Pie: Viewport Toggles", ViewportOptions, "Opens the Viewport Toggles pie.")
        .operandi().ui(),
    action("Pie: Falloffs", Tools, "Opens the Falloffs pie.")
        .operandi().ui(),
    action("Pie: Action Centers", Tools, "Opens the Action Centers pie.")
        .operandi().ui(),
    action("Pie: Modeling", Tools, "Opens the Modeling pie.")
        .operandi().ui(),
    // Layouts stage 2 (UIC-18, LAY-32): the real Ctrl key on a Mac.
    action("Pie: Layouts", View, "Opens the Layouts pie: each slot shows a layout (Ctrl+`).")
        .operandi().ui(),
    action("Layout Switcher", View, "The quick layout switcher while Ctrl+Shift are held: each Tab moves the outline on, releasing switches (Ctrl+Shift+Tab, UIC-18).")
        .operandi().ui(),
    // Layouts stage 6 (LAY-53, UIC-206): the backtick key.
    // Animation keys (Items mode; the app's anim_ui.rs runs their lines).
    action("Play/Stop", Items, "Plays or stops playback (TK-60; / in the Animate layout).")
        .operandi().ui(),
    action("Play Backwards", Items, "Plays backwards through the current range (TK-60).")
        .operandi().ui(),
    action("Back One Frame", Items, "Steps back one frame (TK-63, Shift+Left).")
        .operandi().ui(),
    action("Forward One Frame", Items, "Steps forward one frame (TK-63, Shift+Right).")
        .operandi().ui(),
    action("Step Back", Items, "Steps back one frame (TK-63; comma in the Animate layout).")
        .operandi().ui(),
    action("Step Forward", Items, "Steps forward one frame (TK-63; period in the Animate layout).")
        .operandi().ui(),
    action("First Frame", Items, "Goes to the current range's first frame (TK-63, Shift+Up).")
        .operandi().ui(),
    action("Last Frame", Items, "Goes to the current range's last frame (TK-63, Shift+Down).")
        .operandi().ui(),
    action("Previous Key", Items, "Goes to the previous key of the selected items or channels (TK-63, Ctrl+Left).")
        .operandi().ui(),
    action("Next Key", Items, "Goes to the next key of the selected items or channels (TK-63, Ctrl+Right).")
        .operandi().ui(),
    action("First Key", Items, "Goes to the first key of the selected items or channels (TK-63, Ctrl+Up).")
        .operandi().ui(),
    action("Last Key", Items, "Goes to the last key of the selected items or channels (TK-63, Ctrl+Down).")
        .operandi().ui(),
    action("Key Selected Channels", Items, "Keys the selected channels, or the live transform tool's (TK-21, S).")
        .operandi().ui(),
    action("Key Selected Items", Items, "Keys every animated channel of the selected items (TK-23, Shift+S).")
        .operandi().ui(),
    action("Key Position", Items, "Keys the selected items' position (TK-22, Shift+W).")
        .operandi().ui(),
    action("Key Rotation", Items, "Keys the selected items' rotation (TK-22, Shift+E).")
        .operandi().ui(),
    action("Key Scale", Items, "Keys the selected items' scale (TK-22; Shift+R in the Animate layout).")
        .operandi().ui(),
    action("Key Transforms", Items, "Keys the selected items' position, rotation and scale (TK-22; Shift+Y in the Animate layout).")
        .operandi().ui(),
    action("Toggle Palettes", View, "Hides every open palette, or shows them all again (`, LAY-53).")
        .operandi().ui(),
    action("Node View", View, "Opens the node view of the selected Material Group, or the group the selected layer is in (Cmd+Shift+N).")
        .operandi().ui(),
    action("UI Build Mode", View, "Turns UI build mode on or off: drag pads on every panel (Ctrl+Shift+L).")
        .operandi().ui(),
    // ---------------------------------------------------------- render
    script("render", Render, "Renders the scene (or brings the Render window forward).").ui().side_effect(),
    action("Open Render Window", Render, "Opens the Render window.")
        .operandi().ui(),
    action("Preview", Render, "Opens Preview.")
        .operandi().ui(),
    // Render outputs group D (RO-Q27): one Light Path Expressions output per light.
    script(crate::per_light_outputs::NAME, Render, "Add Per-Light Render Outputs: one Light Path Expressions output per renderable light (C.*L'<label>'); a light with no LPE Label gets its name as its label. One undo step.")
        .operandi()
        .undo()
        .on_editor(
            "Add one Light Path Expressions Render Output per renderable light, skipping lights that already have one.",
            &[],
            crate::per_light_outputs::run,
        ),
    // ---------------------------------------------------------- application
    action("Preferences", Preferences, "Opens Preferences.")
        .operandi().ui(),
    action("Reset Preferences", Preferences, "Asks to reset every preference.")
        .operandi().ui(),
];

// ------------------------------------------------------------------ lookup

/// The entry named `name` (case-insensitive), by its name, an alias, or an
/// old spelling from `command_names`.
pub fn find(name: &str) -> Option<&'static Entry> {
    let name = name.trim();
    let exact = |n: &str| {
        COMMANDS
            .iter()
            .find(|e| e.names().any(|m| m.eq_ignore_ascii_case(n)))
    };
    exact(name).or_else(|| {
        command_names::renamed_command(name)
            .or_else(|| {
                command_names::COMMAND_EXPANSIONS
                    .iter()
                    .find(|(old, _, _)| old.eq_ignore_ascii_case(name))
                    .map(|(_, new, _)| *new)
            })
            .and_then(exact)
    })
}

/// The entry a whole command line starts with, and the rest of the line
/// (its arguments). Multi-word names win over shorter ones ("Spike Reset"
/// over "Spike").
pub fn find_line(line: &str) -> Option<(&'static Entry, &str)> {
    let line = line.trim();
    // Word boundaries, longest first.
    let mut ends: Vec<usize> = line
        .char_indices()
        .filter(|(_, c)| c.is_whitespace())
        .map(|(i, _)| i)
        .collect();
    ends.push(line.len());
    ends.into_iter()
        .rev()
        .find_map(|end| find(&line[..end]).map(|e| (e, line[end..].trim())))
}

/// The line the app's parser takes for `line` when it names a Modo-named
/// command by that name (UIC-165: `app.undo` runs as `Undo`, `select.convert
/// edge` as `Convert to Edges`). `None` when the line needs no change;
/// `Some(Err)` for arguments the phrase can't take yet.
pub fn parser_spelling(line: &str) -> Option<std::result::Result<String, String>> {
    let (entry, rest) = find_line(line)?;
    let phrase = entry.spelled?;
    let head = &line.trim()[..line.trim().len() - rest.len()].trim();
    if !head.eq_ignore_ascii_case(entry.name) {
        // Typed as the phrase or another alias already.
        return None;
    }
    let value = |key: &str| {
        let r = rest.trim();
        crate::command_names::strip_key(r, key)
            .unwrap_or(r)
            .to_owned()
    };
    Some(match entry.name {
        "cmds.showHelp" => Ok(format!("help {}", value("command")).trim().to_owned()),
        "select.convert" => match value("type").to_ascii_lowercase().as_str() {
            "vertex" | "0" => Ok("Convert to Vertices".into()),
            "edge" | "1" => Ok("Convert to Edges".into()),
            "polygon" | "2" => Ok("Convert to Polygons".into()),
            other => Err(format!(
                "select.convert: unknown type \"{other}\"; use vertex, edge or polygon"
            )),
        },
        "select.loop" => match value("direction").to_ascii_lowercase().as_str() {
            "" => Ok(phrase.into()),
            "next" => Ok("Next Loop".into()),
            "prev" | "previous" => Ok("Previous Loop".into()),
            other => Err(format!(
                "select.loop: unknown direction \"{other}\"; use next or prev"
            )),
        },
        "viewport.fit" => match rest.trim() {
            "" => Ok(phrase.into()),
            _ => match crate::command_args::boolean("selected", &value("selected")) {
                Ok(true) => Ok("Frame Selection".into()),
                Ok(false) => Ok(phrase.into()),
                Err(e) => Err(e),
            },
        },
        _ if rest.trim().is_empty() => Ok(phrase.into()),
        _ => Err(format!(
            "{}: arguments aren't supported yet; use {} with none",
            entry.name, entry.name
        )),
    })
}

/// "Deprecated command name: old is now new" when `name` is an old
/// spelling (plan L5; the alias table is `command_names`).
pub fn deprecation(name: &str) -> Option<String> {
    let name = name.trim();
    let new = command_names::renamed_command(name)
        .or_else(|| command_names::renamed_tool(name))
        .or_else(|| {
            command_names::COMMAND_EXPANSIONS
                .iter()
                .find(|(old, _, _)| old.eq_ignore_ascii_case(name))
                .map(|(_, new, _)| *new)
        })?;
    Some(format!("Deprecated command name: {name} is now {new}"))
}

/// The commands in `filter`'s area, or whose name or alias contains it
/// (all when empty), sorted by area then name.
pub fn matching(filter: &str) -> Vec<&'static Entry> {
    matching_in(COMMANDS.iter(), filter)
}

/// [`matching`] over any set of entries (the run-time registry's too).
pub fn matching_in<'a>(entries: impl Iterator<Item = &'a Entry>, filter: &str) -> Vec<&'a Entry> {
    let filter = filter.trim().to_ascii_lowercase();
    let mut entries: Vec<&Entry> = entries
        .filter(|e| {
            filter.is_empty()
                || e.area.label() == filter
                || e.names().any(|n| n.to_ascii_lowercase().contains(&filter))
        })
        .collect();
    entries.sort_by_key(|e| (e.area, e.kind == Kind::Action, e.name.to_ascii_lowercase()));
    entries
}

/// [`matching`] as one line each: area, usage and help.
pub fn list(filter: &str) -> Vec<String> {
    matching(filter)
        .into_iter()
        .map(|e| format!("[{}] {} — {}", e.area.label(), e.usage(), e.help))
        .collect()
}

#[cfg(test)]
mod tests {
    /// Contributor rule (the maintainer, 2026-11-19): one added line per new
    /// tool phrase, in sorted order, so branches don't clash.
    #[test]
    fn tool_phrases_stay_sorted() {
        let ids: Vec<String> = super::TOOL_PHRASES
            .iter()
            .map(|(id, _)| id.to_lowercase())
            .collect();
        if let Some(i) = ids.windows(2).position(|w| w[0] > w[1]) {
            panic!(
                "command_registry.rs: TOOL_PHRASES must stay in sorted order \
                 (case-insensitive); `{}` belongs before `{}`",
                super::TOOL_PHRASES[i + 1].0,
                super::TOOL_PHRASES[i].0
            );
        }
    }

    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn names_are_unique_and_examples_start_with_their_entry() {
        let mut seen = BTreeSet::new();
        for e in COMMANDS {
            for n in e.names() {
                assert!(seen.insert(n.to_ascii_lowercase()), "{n} is listed twice");
            }
            let (found, _) = find_line(e.example)
                .unwrap_or_else(|| panic!("{}: example {:?} names nothing", e.name, e.example));
            assert_eq!(found.name, e.name, "example {:?}", e.example);
            for a in e.args {
                assert!(!a.name.is_empty());
                if let (Some(d), ArgType::Choice(c)) = (a.default, a.ty) {
                    assert!(c.contains(&d), "{}: default {d} isn't a choice", e.name);
                }
            }
        }
    }

    /// L5: every old command name in the alias table reaches an entry, and
    /// says it's deprecated.
    #[test]
    fn every_old_command_name_resolves_and_is_deprecated() {
        for (old, new) in command_names::COMMAND_RENAMES {
            let e = find(old).unwrap_or_else(|| panic!("{old} → {new} has no entry"));
            assert!(e.names().any(|n| n.eq_ignore_ascii_case(new)), "{old}");
            assert!(deprecation(old).unwrap().contains(new));
        }
        for (old, new, _) in command_names::COMMAND_EXPANSIONS {
            let e = find(old).unwrap_or_else(|| panic!("{old} → {new} has no entry"));
            assert!(e.names().any(|n| n.eq_ignore_ascii_case(new)), "{old}");
        }
        for (old, new) in command_names::TOOL_RENAMES {
            assert!(
                TOOL_IDS.iter().any(|t| t.eq_ignore_ascii_case(new)),
                "tool {old} → {new} isn't a tool.set id"
            );
            assert!(deprecation(old).is_some(), "{old}");
        }
        for (star, _) in command_names::STAR_TOOLS {
            assert!(TOOL_IDS.contains(&star), "{star}");
        }
        assert_eq!(deprecation("poly.flip"), None);
    }

    /// Every name in the admitted Modo name lists
    /// (specifications/command-names): the subset's command names, the
    /// button invocations' commands and tool ids, and the `view3d.*` names
    /// listed without their prefix.
    fn map_names() -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for text in [
            include_str!("../../../specifications/command-names/MODO-NAMES-SUBSET.txt"),
            include_str!("../../../specifications/command-names/MODO-BUTTON-INVOCATIONS.txt"),
        ] {
            for line in text.lines() {
                let view3d = line.starts_with("view3d.");
                for word in line
                    .split(|c: char| !(c.is_ascii_alphanumeric() || "._@*".contains(c)))
                    .filter(|w| !w.is_empty())
                {
                    names.insert(word.to_ascii_lowercase());
                    if view3d && !word.contains('.') {
                        names.insert(format!("view3d.{}", word.to_ascii_lowercase()));
                        // "view3d.TransparencyOverrideSolidAlphaBack /
                        // SolidAlphaFront" lists the second one short.
                        names.insert(format!(
                            "view3d.transparencyoverride{}",
                            word.to_ascii_lowercase()
                        ));
                    }
                }
            }
        }
        names
    }

    /// Names that pass UIC-165 without a line in
    /// specifications/command-names (the registry thread's names moved
    /// into MODO-NAMES-SUBSET.txt on 2026-11-11).
    /// transform.reset and transform.freeze are named by the admitted spec
    /// freeze-transforms-2026-11-14 (section 8); layout.restore by
    /// layouts-2026-11-16 (LAY-30); the history commands by
    /// ui-architecture-2026-11-16 (CMD-40, HIS-11, HIS-22, HIS-50).
    const RECORDED: &[&str] = &[
        "transform.reset",
        "transform.freeze",
        "layout.restore",
        "cmds.batch",
        "cmds.undoTo",
        "history.clear",
        "history.max",
        "history.resultsMax",
    ];

    /// UIC-165: every command is a Modo name (or a phrase whose Modo
    /// invocation is known), or is marked Operandi-only on purpose. A new
    /// command fails here until it is matched or marked.
    #[test]
    fn every_command_has_a_modo_name_or_is_marked_operandi_only() {
        let map = map_names();
        let known = |name: &str| {
            map.contains(&name.to_ascii_lowercase())
                || RECORDED.iter().any(|r| r.eq_ignore_ascii_case(name))
        };
        let mut unmatched = Vec::new();
        let mut operandi = Vec::new();
        for e in COMMANDS {
            if e.flags.operandi {
                operandi.push(e.name);
                assert!(
                    !(e.kind == Kind::Script && known(e.name)),
                    "{} is a Modo name; don't mark it Operandi-only",
                    e.name
                );
                continue;
            }
            let covered = match (e.kind, e.modo) {
                (Kind::Script, _) => known(e.name),
                (Kind::Action, Some(modo)) => {
                    let mut words = modo.split_whitespace();
                    let head = words.next().unwrap_or("");
                    let id = words
                        .next()
                        .unwrap_or("")
                        .trim_matches(|c| c == '{' || c == '}' || c == '"');
                    known(head) && (head != "tool.set" || known(id) || TOOL_IDS.contains(&id))
                }
                (Kind::Action, None) => false,
            };
            if !covered {
                unmatched.push(e.name);
            }
        }
        println!(
            "Operandi-only ({}): {}",
            operandi.len(),
            operandi.join(", ")
        );
        assert!(
            unmatched.is_empty(),
            "UIC-165: no Modo name and not marked Operandi-only: {unmatched:?}"
        );
    }

    /// Every phrase's Modo invocation names a registered command.
    #[test]
    fn modo_invocations_name_registered_commands() {
        for e in COMMANDS {
            if let Some(modo) = e.modo {
                assert!(find_line(modo).is_some(), "{}: {modo}", e.name);
            }
        }
        assert_eq!(tool_phrase("util.ruler"), Some("Ruler"));
        for (_, phrase) in TOOL_PHRASES {
            assert!(find(phrase).is_some(), "{phrase}");
        }
    }

    /// PD-Q12's three commands, and only they, join a live transform tool.
    #[test]
    fn three_commands_join_a_live_transform_tool() {
        let names: BTreeSet<&str> = COMMANDS
            .iter()
            .filter(|e| e.flags.allowed_during_transform_tool)
            .map(|e| e.name)
            .collect();
        assert_eq!(
            names,
            BTreeSet::from(["select.item", "transform.channel", "item.transform"])
        );
    }

    /// The headless flag and the JSON arguments come together, and every
    /// JSON argument name is unique within its command.
    #[test]
    fn headless_entries_carry_their_arguments() {
        for e in COMMANDS {
            assert_eq!(e.flags.headless, e.json.is_some(), "{}", e.name);
            if let Some(json) = e.json {
                let names: BTreeSet<&str> = json.args.iter().map(|a| a.name).collect();
                assert_eq!(names.len(), json.args.len(), "{}", e.name);
                assert!(!json.about.is_empty(), "{}", e.name);
            }
        }
        assert_eq!(crate::command_exec::specs().count(), 223);
    }

    #[test]
    fn lines_pick_the_longest_name() {
        assert_eq!(find_line("spike reset").unwrap().0.name, "Spike Reset");
        assert_eq!(find_line("Spike").unwrap().0.name, "Spike");
        let (e, rest) = find_line("smooth shift offset 0.1").unwrap();
        assert_eq!((e.name, rest), ("Smooth Shift", "offset 0.1"));
        assert_eq!(find_line("selset.use A").unwrap().0.name, "select.useSet");
        assert!(find_line("no.such.command").is_none());
    }

    #[test]
    fn list_filters_by_name_or_area() {
        assert_eq!(list("").len(), COMMANDS.len());
        assert!(list("sculpt").iter().all(|l| l.contains("sculpt")));
        assert!(list("poly.flip")[0].contains("poly.flip"));
    }
}
