//! Typed command lines without a window (extensibility plan stage 2):
//! the tokenizer every parser shares, and `run_line`, which runs a line
//! naming a headless command (`command_registry`, flag `headless`) through
//! `command_exec` with its arguments turned into the runner's JSON values.
//! Arguments go by `name:value` or by position, in the order the command's
//! table lists them (command conventions, `command_args`).
use crate::Editor;
use crate::backdrop::{BackdropEdit, Projection};
use crate::command_exec::{self, Result};
use crate::command_registry;
use crate::polygon_subdivide::SubdivideBoundary;
#[allow(unused_imports)]
use crate::{
    AbsoluteFields, AbsoluteMode, ChamferParameters, PositionAxis, PositionSpace, ScaleCenter,
    SubdivideMethod, SubdivideOptions, UniformAxis, UnsubdivideOptions, numeric,
};
use serde_json::{Map, Value};

/// Splits a command line into words. Quotes and `{...}` group spaces
/// (`poly.setPart {Road Part}`); braces nest, quotes don't.
pub fn tokens(s: &str) -> std::result::Result<Vec<String>, &'static str> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut token = String::new();
        let mut quote = false;
        let mut depth = 0usize;
        while let Some(&c) = chars.peek() {
            if !quote && depth == 0 && c.is_whitespace() {
                break;
            }
            chars.next();
            match c {
                '"' if depth == 0 => quote = !quote,
                '{' if !quote => {
                    if depth > 0 {
                        token.push(c);
                    }
                    depth += 1;
                }
                '}' if !quote => {
                    if depth == 0 {
                        return Err("Unmatched brace");
                    }
                    depth -= 1;
                    if depth > 0 {
                        token.push(c);
                    }
                }
                _ => token.push(c),
            }
        }
        if quote || depth != 0 {
            return Err("Unterminated quote or brace");
        }
        out.push(token);
    }
    Ok(out)
}

/// The JSON value of one typed argument of `kind` (a `command_exec::Arg`
/// kind). Lists are JSON (`[1,2,3]`) or comma-separated (`1,2,3`).
fn value(name: &str, kind: &str, text: &str) -> Result<Value> {
    let bad = |what: &str| format!("{name} must be {what}, not \"{text}\"");
    let list = |text: &str| -> Vec<String> {
        let t = text.trim().trim_start_matches('[').trim_end_matches(']');
        t.split(',')
            .map(|p| p.trim().to_owned())
            .filter(|p| !p.is_empty())
            .collect()
    };
    let number = |t: &str| -> Result<Value> {
        let n: f64 = t.trim().parse().map_err(|_| bad("a number"))?;
        serde_json::Number::from_f64(n)
            .map(Value::Number)
            .ok_or_else(|| bad("a number"))
    };
    let integer = |t: &str| -> Result<Value> {
        t.trim()
            .parse::<u64>()
            .map(Value::from)
            .map_err(|_| bad("a whole number"))
    };
    let boolean = |t: &str| -> Result<Value> {
        crate::command_args::boolean(name, t.trim()).map(Value::Bool)
    };
    Ok(match kind {
        "integer" => integer(text)?,
        "number" => number(text)?,
        "boolean" => boolean(text)?,
        "[x, y, z]" => {
            let parts = list(text);
            if parts.len() != 3 {
                return Err(bad("three numbers x,y,z"));
            }
            Value::Array(parts.iter().map(|p| number(p)).collect::<Result<_>>()?)
        }
        "[bool, bool, bool]" => {
            let parts = list(text);
            if parts.len() != 3 {
                return Err(bad("three booleans"));
            }
            Value::Array(parts.iter().map(|p| boolean(p)).collect::<Result<_>>()?)
        }
        "array of integers" => Value::Array(
            list(text)
                .iter()
                .map(|p| integer(p))
                .collect::<Result<_>>()?,
        ),
        "array of strings" => Value::Array(list(text).into_iter().map(Value::String).collect()),
        _ => Value::String(text.to_owned()),
    })
}

/// The JSON arguments of a typed line's words (after the command name),
/// by name or position against `command`'s argument table.
pub fn json_args(command: &str, words: &[String]) -> Result<Map<String, Value>> {
    let spec =
        command_exec::spec(command).ok_or_else(|| format!("{command} can't run from a script"))?;
    let names: Vec<&str> = spec.args.iter().map(|a| a.name).collect();
    let bound = crate::command_args::bind(command, words, &names)?;
    let mut map = Map::new();
    for (arg, given) in spec.args.iter().zip(bound) {
        match given {
            Some(text) => {
                map.insert(arg.name.to_owned(), value(arg.name, arg.kind, text)?);
            }
            None if arg.required => {
                return Err(format!("{command}: Missing required argument {}", arg.name));
            }
            None => {}
        }
    }
    Ok(map)
}

/// The one `Command` a typed line naming a headless command builds, for a
/// caller that dispatches it itself. `Ok(None)`: it took several steps
/// and already ran. `None` when the line names no headless command.
pub fn command_for_line(editor: &mut Editor, line: &str) -> Option<Result<Option<crate::Command>>> {
    let words = match tokens(line) {
        Ok(w) if !w.is_empty() => w,
        Ok(_) => return None,
        Err(e) => return Some(Err(e.into())),
    };
    let entry = command_registry::find(&words[0]).filter(|e| e.flags.headless)?;
    Some(
        json_args(entry.name, &words[1..])
            .and_then(|args| command_exec::command_for(editor, &words[0], &args)),
    )
}

/// Runs a typed line naming a headless command on `editor`. `Ok(None)`
/// when the line doesn't name one (another parser's), else the outcome.
pub fn run_line(editor: &mut Editor, line: &str) -> Option<Result<Option<String>>> {
    let words = match tokens(line) {
        Ok(w) if !w.is_empty() => w,
        Ok(_) => return None,
        Err(e) => return Some(Err(e.into())),
    };
    let entry = command_registry::find(&words[0]).filter(|e| e.flags.headless)?;
    Some(json_args(entry.name, &words[1..]).and_then(|args| {
        // A query answers with text and changes nothing (UIC-163).
        if let Some(answer) = command_exec::answer(editor, &words[0], &args) {
            return answer.map(Some);
        }
        command_exec::run(editor, &words[0], &args).map(|()| None)
    }))
}

// ------------------------------------------------- argument parsers
// The typed commands' argument parsers, moved from the app (stage 2):
// they read words into core values and touch no app state.

/// PART-003: zero arguments open the dialog; one positional or `name:` value
/// is the name. Only a leading `name:` key is special: other colons are part
/// of the name, so a literal leading `name:` needs `name:{name:...}`.
pub fn set_part_argument(args: &[String]) -> std::result::Result<Option<String>, &'static str> {
    const USAGE: &str = "Use poly.setPart <name>; group spaces with quotes or braces, e.g. poly.setPart {Road Part}";
    match args {
        [] => Ok(None),
        [arg] => {
            let value = match arg.split_once(':') {
                Some((key, value)) if key.eq_ignore_ascii_case("name") => value,
                _ => arg.as_str(),
            };
            if value == "?" {
                Err("poly.setPart queries are unsupported; use help poly.setPart")
            } else if value.is_empty() {
                Err(USAGE)
            } else {
                Ok(Some(value.to_owned()))
            }
        }
        _ => Err(USAGE),
    }
}

/// `chamfer.edit` arguments, all four required by name. Depth is a map
/// `percent` (0.5 = 50 %, or `50%`); the tool keeps percent.
pub fn chamfer_edit_arguments(
    args: &[String],
) -> std::result::Result<ChamferParameters, &'static str> {
    const USAGE: &str = "Use chamfer.edit offset:<m> segments:<n> sourceSegs:1 depth:<0–1 or %>";
    let mut p = ChamferParameters::default();
    let mut seen = [false; 4];
    for arg in args {
        let (key, value) = arg.split_once(':').ok_or(USAGE)?;
        let index = match key.to_ascii_lowercase().as_str() {
            "offset" => 0,
            "segments" => 1,
            "sourcesegs" => 2,
            "depth" => 3,
            _ => return Err("Unknown Chamfer Edit parameter"),
        };
        if std::mem::replace(&mut seen[index], true) {
            return Err("Duplicate Chamfer Edit parameter");
        }
        let v = if index == 3 {
            crate::command_names::script_percent(value)
                .map_err(|_| "Invalid Chamfer Edit parameter")?
        } else {
            value
                .parse::<f64>()
                .map_err(|_| "Invalid Chamfer Edit parameter")?
        };
        match index {
            0 => p.offset = v,
            1 | 2 if v.is_finite() && v.fract() == 0. && (1. ..=64.).contains(&v) => {
                if index == 1 {
                    p.segments = v as u32
                } else {
                    p.source_segments = v as u32
                }
            }
            3 => p.depth = v,
            _ => return Err("Invalid Chamfer Edit segment count"),
        }
    }
    if seen.iter().any(|v| !v) {
        return Err("Specify offset, segments, sourceSegs and depth explicitly");
    }
    Ok(p)
}

/// `vert.set axis value workPlane worldSpace`, named or by position
/// (command conventions). Axis and Value are required; the switches
/// default to off.
pub fn set_position_arguments(
    args: &[String],
) -> std::result::Result<(PositionAxis, f64, PositionSpace), String> {
    use crate::command_args::{bind, boolean, choice};
    let bound = bind(
        "vert.set",
        args,
        &["axis", "value", "workPlane", "worldSpace"],
    )?;
    let axis = bound[0].ok_or("vert.set: Missing required argument axis")?;
    let axis = [PositionAxis::X, PositionAxis::Y, PositionAxis::Z]
        [choice("axis", axis, &["x", "y", "z"])?];
    let value = bound[1].ok_or("vert.set: Missing required argument value")?;
    let value = numeric::parse_quantity(value, numeric::Quantity::Distance)?;
    let work_plane = bound[2]
        .map(|v| boolean("workPlane", v))
        .transpose()?
        .unwrap_or(false);
    let world_space = bound[3]
        .map(|v| boolean("worldSpace", v))
        .transpose()?
        .unwrap_or(false);
    Ok((
        axis,
        value,
        PositionSpace::from_switches(work_plane, world_space)?,
    ))
}

/// `scale.absolute mode [absolute_size_uniform absolute_size_uniform_pref
/// orient_to_selection absolute_size_x/y/z absolute_size_center_x/y/z]`,
/// named or by position (spec snaps-precision-2026-11-10 D). Values left
/// out keep `base` (the Absolute Scaling window's fields); given ones
/// replace them (provisional, Q8).
pub fn absolute_scale_arguments(
    args: &[String],
    base: AbsoluteFields,
) -> std::result::Result<(AbsoluteMode, AbsoluteFields), String> {
    use crate::command_args::{bind, boolean, choice};
    const NAMES: [&str; 10] = [
        "mode",
        "absolute_size_uniform",
        "absolute_size_uniform_pref",
        "orient_to_selection",
        "absolute_size_x",
        "absolute_size_y",
        "absolute_size_z",
        "absolute_size_center_x",
        "absolute_size_center_y",
        "absolute_size_center_z",
    ];
    let bound = bind("scale.absolute", args, &NAMES)?;
    let mode = bound[0].ok_or("scale.absolute: Missing required argument mode")?;
    let mode =
        AbsoluteMode::ALL[choice("mode", mode, &["grab", "uniform", "explicit", "reference"])?];
    let mut f = base;
    let distance = |v: &str| numeric::parse_quantity(v, numeric::Quantity::Distance);
    if let Some(v) = bound[1] {
        f.uniform = distance(v)?;
    }
    if let Some(v) = bound[2] {
        f.axis = UniformAxis::ALL[choice(NAMES[2], v, &["longestAxis", "x", "y", "z"])?];
    }
    if let Some(v) = bound[3] {
        f.orient = boolean(NAMES[3], v)?;
    }
    for i in 0..3 {
        if let Some(v) = bound[4 + i] {
            f.size[i] = distance(v)?;
        }
        if let Some(v) = bound[7 + i] {
            // The stored names carry the axis ("highX", "lowY"); either works.
            let axis = ["x", "y", "z"][i];
            let lower = v.to_ascii_lowercase();
            let v = lower
                .strip_suffix(axis)
                .filter(|s| !s.is_empty())
                .unwrap_or(&lower);
            f.center[i] =
                ScaleCenter::ALL[choice(NAMES[7 + i], v, &["center", "high", "low", "origin"])?];
        }
    }
    Ok((mode, f))
}

pub fn named(arg: &str) -> (Option<&str>, &str) {
    match arg.split_once(':') {
        Some((k, v)) => (Some(k), v.trim_matches('"')),
        None => (None, arg.trim_matches('"')),
    }
}

/// `poly.subdivide` arguments over the remembered options.
pub fn parse_subdivide(
    base: &SubdivideOptions,
    args: &[&str],
) -> std::result::Result<SubdivideOptions, String> {
    use crate::command_args::boolean;
    use crate::command_names::{script_angle, script_percent};
    let mut o = base.clone();
    for (i, arg) in args.iter().enumerate() {
        let (key, value) = named(arg);
        let key = key.map(str::to_ascii_lowercase);
        match key.as_deref() {
            None if i == 0 => {
                o.method = SubdivideMethod::parse(value).ok_or_else(|| {
                    format!("Unknown subdivision type \"{value}\"; use ccsds, flat, smooth or sds")
                })?
            }
            Some("type") => {
                o.method = SubdivideMethod::parse(value).ok_or_else(|| {
                    format!("Unknown subdivision type \"{value}\"; use ccsds, flat, smooth or sds")
                })?
            }
            Some("fractal") => o.fractal_percent = script_percent(value)?,
            Some("maxangle") => o.max_angle_degrees = script_angle(value)?,
            Some("linear") => o.linear_uvs = boolean("linear", value)?,
            Some("bound") => {
                o.boundary = SubdivideBoundary::parse(value).ok_or_else(|| {
                    format!("Unknown boundary rule \"{value}\"; use smooth, crease or creaseEdges")
                })?
            }
            Some("bgcons") => o.background_constraint = boolean("bgCons", value)?,
            Some("noedgeweight") => o.ignore_edge_weights = boolean("noEdgeWeight", value)?,
            _ => return Err(format!("poly.subdivide: unknown argument \"{arg}\"")),
        }
    }
    o.validate()?;
    Ok(o)
}

/// `poly.unsubdiv` arguments over the remembered options.
pub fn parse_unsubdivide(
    base: &UnsubdivideOptions,
    args: &[&str],
) -> std::result::Result<UnsubdivideOptions, String> {
    use crate::command_args::boolean;
    let mut o = base.clone();
    for arg in args {
        let (key, value) = named(arg);
        let Some(key) = key else {
            return Err(format!(
                "poly.unsubdiv takes name:value arguments, not \"{arg}\""
            ));
        };
        match key.to_ascii_lowercase().as_str() {
            "iteration" | "iterations" => {
                o.iterations = value
                    .parse::<u32>()
                    .ok()
                    .filter(|n| *n <= crate::polygon_subdivide::MAX_UNSUBDIVIDE_ITERATIONS)
                    .ok_or_else(|| {
                        format!(
                            "Iteration must be a whole number from 0 to {}",
                            crate::polygon_subdivide::MAX_UNSUBDIVIDE_ITERATIONS
                        )
                    })?
            }
            "flat" => o.flat = boolean("flat", value)?,
            "uv" => o.uv_hint = boolean("uv", value)?,
            "material" => o.material_hint = boolean("material", value)?,
            "part" => o.part_hint = boolean("part", value)?,
            "smoothinggroup" => o.smoothing_group_hint = boolean("smoothingGroup", value)?,
            "selectionset" => o.selection_set_hint = boolean("selectionSet", value)?,
            "selectionsetname" => o.selection_set_name = value.to_owned(),
            _ => return Err(format!("poly.unsubdiv: unknown argument \"{arg}\"")),
        }
    }
    Ok(o)
}

/// `backdrop.edit <name> <value>` (old `backdrop.attr`) as a panel edit.
pub fn attr_edit(name: &str, value: &str) -> std::result::Result<BackdropEdit, String> {
    let flag = || match value.to_ascii_lowercase().as_str() {
        "true" | "on" | "1" => Ok(true),
        "false" | "off" | "0" => Ok(false),
        _ => Err(format!("{name} takes true or false")),
    };
    let percent = || {
        value
            .trim_end_matches('%')
            .trim()
            .parse::<f64>()
            .map(|v| v / 100.)
            .map_err(|_| format!("{name} takes a percentage, such as 50 or 50%"))
    };
    Ok(match name.to_ascii_lowercase().as_str() {
        "projection" => BackdropEdit::Projection(
            *Projection::ALL
                .iter()
                .find(|p| p.label().eq_ignore_ascii_case(value))
                .ok_or("projection takes front, back, top, bottom, left, right or camera")?,
        ),
        "showinperspective" => BackdropEdit::ShowInPerspective(flag()?),
        "keepaspect" => BackdropEdit::KeepAspect(flag()?),
        "flip" => BackdropEdit::Flip(flag()?),
        "invert" => BackdropEdit::Invert(flag()?),
        "blend" => BackdropEdit::Blend(flag()?),
        "uniformsize" => BackdropEdit::UniformSize(flag()?),
        "contrast" => BackdropEdit::Contrast(percent()?),
        "brightness" => BackdropEdit::Brightness(percent()?),
        "transparency" => BackdropEdit::Transparency(percent()?),
        "pixelsize" => {
            BackdropEdit::PixelSize(numeric::parse_quantity(value, numeric::Quantity::Distance)?)
        }
        _ => {
            return Err(format!(
                "Unknown Backdrop option {name}; see help backdrop.edit"
            ));
        }
    })
}

/// Binds `args` to `names`: `name:value` by a known name, other values by
/// position. Unlike `command_args::bind`, an unknown `key:` is a value, so
/// set names with colons keep working. `None` when a slot is filled twice
/// or there are too many values.
pub fn bind_lenient<'a>(args: &'a [String], names: &[&str]) -> Option<Vec<Option<&'a str>>> {
    let mut bound = vec![None; names.len()];
    let mut next = 0;
    for arg in args {
        let slot = arg.split_once(':').and_then(|(key, value)| {
            names
                .iter()
                .position(|n| n.eq_ignore_ascii_case(key))
                .map(|i| (i, value))
        });
        let (i, value) = match slot {
            Some(named) => named,
            None => {
                while next < names.len() && bound[next].is_some() {
                    next += 1;
                }
                (next, arg.as_str())
            }
        };
        if i >= names.len() || bound[i].is_some() {
            return None;
        }
        bound[i] = Some(value);
    }
    Some(bound)
}

// ------------------------------------------------- tool.set

/// `tool.set`'s `<preset> <mode>`, by `preset:` / `mode:` or in that
/// order; `None` when an argument is unknown, empty or given twice.
pub fn tool_set_args(args: &[String]) -> Option<(&str, &str)> {
    let mut preset: Option<&str> = None;
    let mut mode: Option<&str> = None;
    for arg in args {
        let (slot, value) = if let Some((key, value)) = arg.split_once(':') {
            match key.to_ascii_lowercase().as_str() {
                "preset" => (&mut preset, value),
                "mode" => (&mut mode, value),
                _ => return None,
            }
        } else if preset.is_none() {
            (&mut preset, arg.as_str())
        } else {
            (&mut mode, arg.as_str())
        };
        if slot.is_some() || value.is_empty() {
            return None;
        }
        *slot = Some(value);
    }
    Some((preset?, mode?))
}

// ------------------------------------------------- vertex verbs

/// Vertex Align's axis choices, in the dialog's order.
pub const ALIGN_AXES: [(&str, [bool; 3]); 7] = [
    ("All", [true; 3]),
    ("X", [true, false, false]),
    ("Y", [false, true, false]),
    ("Z", [false, false, true]),
    ("XY", [true, true, false]),
    ("YZ", [false, true, true]),
    ("ZX", [true, false, true]),
];

/// What a typed vertex-panel line asks for (`vert.*`, `view3d.showVertices`).
#[derive(Clone, Debug, PartialEq)]
pub enum VertexLine {
    /// The bare verb: run its panel row (some open their dialog).
    Row(&'static str),
    /// `vert.new <x> <y> <z>`.
    New(glam::DVec3),
    /// `view3d.showVertices on|off`.
    ShowVertices(bool),
    /// `view3d.showVertices ?`.
    ShowVerticesQuery,
    /// `vert.weld target:<n>`, the selected points onto it.
    Weld(u32),
    Command(crate::VertexCommand),
    /// Arguments the verb doesn't take: show its usage.
    Usage,
    Error(String),
}

/// The vertex verbs (lower case), each with the panel row a bare verb runs.
pub const VERTEX_VERBS: [&str; 13] = [
    "vert.new",
    "vert.split",
    "vert.splitedge",
    "vert.align",
    "vert.merge",
    "vert.join",
    "vert.collapse",
    "vert.remove",
    "vert.removeedge",
    "vert.reorder",
    "vert.syncorder",
    "vert.weld",
    "view3d.showvertices",
];

/// Parses a vertex verb's words (`verb` lower case, `args` after it).
/// `None` when `verb` isn't one.
pub fn parse_vertex(verb: &str, args: &[String]) -> Option<VertexLine> {
    use crate::{ReorderMode, VertexCommand};
    use std::collections::BTreeMap;
    let verb: &'static str = VERTEX_VERBS.iter().find(|v| **v == verb)?;
    let mut options: BTreeMap<String, String> = BTreeMap::new();
    let mut positional = Vec::new();
    for arg in args {
        match arg.split_once(':') {
            Some((k, v)) => {
                if options
                    .insert(k.to_ascii_lowercase(), v.to_string())
                    .is_some()
                {
                    return Some(VertexLine::Error(format!("Duplicate {verb} argument {k}")));
                }
            }
            None => positional.push(arg.clone()),
        }
    }
    // The map's argument names for values Operandi takes by position.
    if verb == "vert.new" && !options.is_empty() && positional.is_empty() {
        let xyz: Vec<_> = ["x", "y", "z"]
            .iter()
            .filter_map(|k| options.get(*k))
            .collect();
        if xyz.len() == 3 && options.len() == 3 {
            positional = xyz.into_iter().cloned().collect();
            options.clear();
        }
    }
    if verb == "view3d.showvertices"
        && let Some(state) = options.remove("state")
    {
        positional.insert(0, state);
    }
    let bool_option = |options: &BTreeMap<String, String>, key: &str| -> Option<Option<bool>> {
        match options.get(key).map(|v| v.to_ascii_lowercase()) {
            None => Some(None),
            Some(v) if matches!(v.as_str(), "1" | "true" | "on") => Some(Some(true)),
            Some(v) if matches!(v.as_str(), "0" | "false" | "off") => Some(Some(false)),
            Some(_) => None,
        }
    };
    let known = |allowed: &[&str]| options.keys().all(|k| allowed.contains(&k.as_str()));
    Some(match verb {
        "vert.new" => {
            if positional.is_empty() && options.is_empty() {
                return Some(VertexLine::Row(verb));
            }
            if positional.len() != 3 || !options.is_empty() {
                return Some(VertexLine::Usage);
            }
            let parsed: Result<Vec<f64>> = positional
                .iter()
                .map(|v| numeric::parse_quantity(v, numeric::Quantity::Distance))
                .collect();
            match parsed {
                Ok(v) => VertexLine::New(glam::DVec3::new(v[0], v[1], v[2])),
                Err(e) => VertexLine::Error(e),
            }
        }
        "view3d.showvertices" => match (positional.as_slice(), options.is_empty()) {
            ([], true) => VertexLine::Row(verb),
            ([mode], true) => match mode.to_ascii_lowercase().as_str() {
                "on" | "1" | "true" => VertexLine::ShowVertices(true),
                "off" | "0" | "false" => VertexLine::ShowVertices(false),
                "?" => VertexLine::ShowVerticesQuery,
                _ => VertexLine::Usage,
            },
            _ => VertexLine::Usage,
        },
        "vert.weld" => {
            let target = options.get("target").map(|t| t.parse::<u32>());
            match (target, positional.is_empty(), known(&["target"])) {
                (Some(Ok(target)), true, true) => VertexLine::Weld(target),
                _ => VertexLine::Usage,
            }
        }
        "vert.join" => match (
            positional.is_empty(),
            known(&["average", "keep"]),
            bool_option(&options, "average"),
            bool_option(&options, "keep"),
        ) {
            (true, true, Some(average), Some(keep)) => VertexLine::Command(VertexCommand::Join {
                average: average.unwrap_or(false),
                options: crate::MergeOptions {
                    keep_small: keep.unwrap_or(false),
                    ..Default::default()
                },
            }),
            _ => VertexLine::Usage,
        },
        "vert.align" => {
            let axis = options
                .get("axis")
                .map(|a| ALIGN_AXES.iter().find(|(n, _)| n.eq_ignore_ascii_case(a)))
                .unwrap_or(Some(&ALIGN_AXES[0]));
            match (
                positional.is_empty(),
                known(&["axis", "uniform"]),
                axis,
                bool_option(&options, "uniform"),
            ) {
                (true, true, Some((_, axes)), Some(uniform)) => {
                    VertexLine::Command(VertexCommand::Align {
                        axes: *axes,
                        uniform: uniform.unwrap_or(false),
                    })
                }
                _ => VertexLine::Usage,
            }
        }
        "vert.merge" => {
            let range = options.get("range").map(|r| r.to_ascii_lowercase());
            let distance = match (range.as_deref(), options.get("dist")) {
                (None | Some("auto" | "automatic"), None) => Some(Ok(0.)),
                (Some("fixed"), Some(d)) => {
                    Some(numeric::parse_quantity(d, numeric::Quantity::Distance))
                }
                _ => None,
            };
            let keep = bool_option(&options, "keep");
            match (
                positional.is_empty(),
                known(&["range", "dist", "keep"]),
                distance,
                keep,
            ) {
                (true, true, Some(Ok(distance)), Some(keep)) => {
                    VertexLine::Command(VertexCommand::Merge {
                        distance,
                        options: crate::MergeOptions {
                            keep_small: keep.unwrap_or(false),
                            ..Default::default()
                        },
                    })
                }
                (true, true, Some(Err(e)), _) => VertexLine::Error(e),
                _ => VertexLine::Usage,
            }
        }
        "vert.reorder" => {
            let mode = match options
                .get("mode")
                .map(|m| m.to_ascii_lowercase())
                .as_deref()
            {
                None if positional.is_empty() && options.is_empty() => {
                    return Some(VertexLine::Row(verb));
                }
                Some("swap") => Some(ReorderMode::Swap),
                Some("top") => Some(ReorderMode::Top),
                Some("bottom") => Some(ReorderMode::Bottom),
                _ => None,
            };
            match (mode, positional.is_empty(), known(&["mode"])) {
                (Some(mode), true, true) => VertexLine::Command(VertexCommand::Reorder(mode)),
                _ => VertexLine::Usage,
            }
        }
        _ if !positional.is_empty() || !options.is_empty() => VertexLine::Usage,
        _ => VertexLine::Row(verb),
    })
}

// ------------------------------------------------- sculpt verbs

/// A switch a line asks for: on, off, toggle (bare) or a query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    On,
    Off,
    Toggle,
    Query,
}

/// What a typed sculpt line asks for.
#[derive(Clone, Debug, PartialEq)]
pub enum SculptLine {
    /// Start the brush (`sculpt.push`, or the Input Map's "Sculpt Push").
    Start(crate::SculptBrush),
    /// `tool.set sculpt.push on|off|?`.
    ToolSet(crate::SculptBrush, Switch),
    Tip(crate::sculpt::TipKind),
    /// `sculpt.borderLock` (false) / `sculpt.cornerLock` (true).
    Lock {
        corners: bool,
        switch: Switch,
    },
    /// `vertMap.new <name> [weight] [init] [value:<n>]` (VM-03).
    NewWeightMap {
        name: String,
        value: Option<f64>,
    },
    /// `select.vertexMap [name]`: no name, or mode remove, selects none.
    SelectWeightMap(Option<String>),
    /// `sculpt.liveWeightEdit live:<bool>` (VMT-20).
    LiveDeformers(Switch),
    /// `tool.set falloff.vertexMap|falloff.invert on|off`, or
    /// `falloff.vertexMap mode index mix`: add the falloff on top of the
    /// active one (it becomes the active one when there is none), or
    /// remove that kind; `None` asks whether one is on (`?`).
    Falloff {
        falloff: crate::Falloff,
        mix: crate::MixMode,
        on: Option<bool>,
    },
    /// A refusal to show as is (usage).
    Usage(String),
}

/// Parses a sculpt line. `None` when it isn't one.
pub fn parse_sculpt(text: &str) -> Option<SculptLine> {
    use crate::SculptBrush;
    use crate::sculpt::TipKind;
    let words: Vec<String> =
        tokens(text).unwrap_or_else(|_| text.split_whitespace().map(str::to_owned).collect());
    let lower: Vec<String> = words.iter().map(|w| w.to_ascii_lowercase()).collect();
    let first = lower.first()?;
    // Input-map actions of the Vertex Map Tools ("Vertex Map Paint").
    if lower.len() >= 3 && lower[0] == "vertex" && lower[1] == "map" {
        let name = lower[2..].join(" ");
        return SculptBrush::VERTEX_MAP
            .into_iter()
            .find(|b| b.label().eq_ignore_ascii_case(&name))
            .map(SculptLine::Start);
    }
    // Keyed (`name:Mask`) or positional arguments.
    let keyed = |key: &str, pos: usize| -> Option<String> {
        let prefix = format!("{key}:");
        words
            .iter()
            .skip(1)
            .find_map(|w| {
                w.to_ascii_lowercase()
                    .starts_with(&prefix)
                    .then(|| w[prefix.len()..].to_owned())
            })
            .or_else(|| {
                words
                    .iter()
                    .skip(1)
                    .filter(|w| !w.contains(':'))
                    .nth(pos)
                    .cloned()
            })
    };
    let flag = |v: Option<String>| -> Option<Option<bool>> {
        match v.as_deref().map(str::to_ascii_lowercase).as_deref() {
            None => Some(None),
            Some("true" | "on" | "1") => Some(Some(true)),
            Some("false" | "off" | "0") => Some(Some(false)),
            _ => None,
        }
    };
    // Falloffs added from the Add Falloff menu (tool.set falloff.<kind>).
    let falloff_kind = |name: &str| -> Option<crate::Falloff> {
        match name {
            "falloff.vertexmap" => Some(crate::Falloff::VertexMap(Default::default())),
            "falloff.invert" => Some(crate::Falloff::Invert),
            "falloff.selection" => Some(crate::Falloff::Selection(Default::default())),
            "falloff.softselection" => Some(crate::Falloff::SoftSelection(Default::default())),
            "falloff.element" => Some(crate::Falloff::Element(Default::default())),
            "falloff.coplanar" => Some(crate::Falloff::Coplanar(Default::default())),
            "falloff.screen" => Some(crate::Falloff::Screen(Default::default())),
            "falloff.lasso" => Some(crate::Falloff::Lasso(Default::default())),
            "falloff.airbrush" => Some(crate::Falloff::Airbrush(Default::default())),
            "falloff.noise" => Some(crate::Falloff::Noise(Default::default())),
            "falloff.image" => Some(crate::Falloff::Image(Default::default())),
            "falloff.path" => Some(crate::Falloff::Path(Default::default())),
            "falloff.curvature" => Some(crate::Falloff::Curvature(Default::default())),
            _ => None,
        }
    };
    if first == "tool.set"
        && let Some(f) = lower.get(1).and_then(|n| falloff_kind(n))
    {
        let on = match lower.get(2).map(String::as_str) {
            Some("off" | "0") => Some(false),
            Some("?") => None,
            _ => Some(true),
        };
        return Some(SculptLine::Falloff {
            falloff: f,
            mix: crate::MixMode::Multiply,
            on,
        });
    }
    if let Some(mut f) = falloff_kind(first) {
        let mode = keyed("mode", 0).map(|m| m.to_ascii_lowercase());
        let index = keyed("index", 1).map(|i| i.parse::<u32>());
        let mix = match keyed("mix", 2).map(|m| m.to_ascii_lowercase()).as_deref() {
            None | Some("multiply") => crate::MixMode::Multiply,
            Some("add") => crate::MixMode::Add,
            Some("subtract") => crate::MixMode::Subtract,
            Some("max") => crate::MixMode::Max,
            Some("min") => crate::MixMode::Min,
            Some(_) => {
                return Some(SculptLine::Usage(
                    "mix is multiply, add, subtract, max or min".into(),
                ));
            }
        };
        // falloff.selection steps shape p0 p1 mix (registry 2026-11-13):
        // keyed arguments only, so the mix position stays the Vertex Map one.
        if let crate::Falloff::Selection(sel) = &mut f {
            let key = |k: &str| keyed(k, usize::MAX);
            if let Some(steps) = key("steps") {
                match steps.parse::<u32>() {
                    Ok(n) if (1..=crate::falloff::MAX_SELECTION_STEPS).contains(&n) => {
                        sel.steps = n
                    }
                    _ => {
                        return Some(SculptLine::Usage(
                            "steps is a whole number from 1 to 1000".into(),
                        ));
                    }
                }
            }
            let number = |k: &str| key(k).map(|v| v.parse::<f64>().ok().filter(|v| v.is_finite()));
            let (p0, p1) = (number("p0"), number("p1"));
            if matches!(p0, Some(None)) || matches!(p1, Some(None)) {
                return Some(SculptLine::Usage("p0 and p1 are numbers".into()));
            }
            if let Some(shape) = key("shape") {
                sel.shape = match shape.to_ascii_lowercase().as_str() {
                    "0" | "linear" => crate::FalloffShape::Linear,
                    "1" | "easein" => crate::FalloffShape::EaseIn,
                    "2" | "easeout" => crate::FalloffShape::EaseOut,
                    "3" | "smooth" => crate::FalloffShape::Smooth,
                    "4" | "custom" => crate::FalloffShape::Custom {
                        ease_in: 0.,
                        ease_out: 0.,
                    },
                    _ => {
                        return Some(SculptLine::Usage(
                            "shape is linear, easeIn, easeOut, smooth or custom (0-4)".into(),
                        ));
                    }
                };
            }
            if let crate::FalloffShape::Custom { ease_in, ease_out } = &mut sel.shape {
                if let Some(Some(v)) = p0 {
                    *ease_in = v;
                }
                if let Some(Some(v)) = p1 {
                    *ease_out = v;
                }
            }
        }
        if let Err(e) = batch2_falloff_args(&mut f, &|k| keyed(k, usize::MAX)) {
            return Some(SculptLine::Usage(e));
        }
        if let crate::Falloff::VertexMap(v) = &mut f {
            match mode.as_deref() {
                None | Some("magnitude") => {}
                Some("component") => v.mode = crate::VertexMapMode::Component,
                Some(_) => return Some(SculptLine::Usage("mode is magnitude or component".into())),
            }
            match index {
                None => {}
                Some(Ok(i)) => v.index = i,
                Some(Err(_)) => return Some(SculptLine::Usage("index is a whole number".into())),
            }
        }
        return Some(SculptLine::Falloff {
            falloff: f,
            mix,
            on: Some(true),
        });
    }
    match first.as_str() {
        "vertmap.new" => {
            // vertMap.new name type init color value (VM-03).
            let Some(name) = keyed("name", 0) else {
                return Some(SculptLine::Usage(
                    "Use vertMap.new <name> [weight] [init] [value:<number>]".into(),
                ));
            };
            if keyed("type", 1).is_some_and(|t| !t.eq_ignore_ascii_case("weight")) {
                return Some(SculptLine::Usage("Operandi makes weight maps only".into()));
            }
            let Some(init) = flag(keyed("init", 2)) else {
                return Some(SculptLine::Usage("init is true or false".into()));
            };
            let value = match keyed("value", 4).map(|v| v.parse::<f64>()) {
                None => None,
                Some(Ok(v)) if v.is_finite() => Some(v),
                Some(_) => return Some(SculptLine::Usage("value must be a number".into())),
            };
            // A value fills every point; init alone fills with 0.
            let value = match (init, value) {
                (Some(false), _) => None,
                (Some(true), v) => Some(v.unwrap_or(0.)),
                (None, v) => v,
            };
            return Some(SculptLine::NewWeightMap { name, value });
        }
        "select.vertexmap" => {
            // select.vertexMap name type mode: set or add selects, remove
            // deselects (one map at a time, VM-02).
            let name = keyed("name", 0);
            let mode = keyed("mode", 2).unwrap_or_else(|| "set".into());
            return Some(match mode.to_ascii_lowercase().as_str() {
                "set" | "add" => SculptLine::SelectWeightMap(name),
                "remove" => SculptLine::SelectWeightMap(None),
                _ => SculptLine::Usage("mode is set, add or remove".into()),
            });
        }
        "sculpt.liveweightedit" => {
            let switch = match flag(keyed("live", 0)) {
                Some(None) if lower.get(1).map(String::as_str) == Some("?") => Switch::Query,
                Some(None) => Switch::Toggle,
                Some(Some(true)) => Switch::On,
                Some(Some(false)) => Switch::Off,
                None => {
                    return Some(SculptLine::Usage(
                        "Use sculpt.liveWeightEdit live:true|false".into(),
                    ));
                }
            };
            return Some(SculptLine::LiveDeformers(switch));
        }
        _ => {}
    }
    // Input-map actions ("Sculpt Push", "Sculpt Tangent Pinch").
    if lower.len() >= 2 && first == "sculpt" {
        let name = lower[1..].join(" ");
        return SculptBrush::ALL
            .into_iter()
            .find(|b| b.label().eq_ignore_ascii_case(&name))
            .map(SculptLine::Start);
    }
    if first == "tool.set"
        && let Some(name) = lower.get(1)
        && let Some(brush) = SculptBrush::from_command(name.trim_start_matches("preset:"))
    {
        let mode = lower
            .get(2)
            .map(|m| m.trim_start_matches("mode:").to_string());
        return Some(match mode.as_deref() {
            Some("on" | "1") => SculptLine::ToolSet(brush, Switch::On),
            Some("off" | "0") => SculptLine::ToolSet(brush, Switch::Off),
            Some("?") => SculptLine::ToolSet(brush, Switch::Query),
            _ => SculptLine::Usage(format!("Use tool.set {} on, off or ?", brush.command())),
        });
    }
    if let Some(brush) = SculptBrush::from_command(first) {
        return Some(if lower.len() == 1 {
            SculptLine::Start(brush)
        } else {
            SculptLine::Usage(format!("{} takes no arguments", brush.command()))
        });
    }
    for kind in [TipKind::Smooth, TipKind::Hard] {
        if first.eq_ignore_ascii_case(kind.command()) {
            return Some(SculptLine::Tip(kind));
        }
    }
    let corners = match first.as_str() {
        "sculpt.borderlock" => false,
        "sculpt.cornerlock" => true,
        _ => return None,
    };
    let switch = match lower.get(1).map(String::as_str) {
        None => Switch::Toggle,
        Some("on" | "true" | "1") => Switch::On,
        Some("off" | "false" | "0") => Switch::Off,
        Some("?") => Switch::Query,
        _ => return Some(SculptLine::Usage(format!("Use {} [on|off|?]", words[0]))),
    };
    Some(SculptLine::Lock { corners, switch })
}

/// Keyed arguments of `falloff.softSelection`, `falloff.element` and
/// `falloff.coplanar` (falloff batch 2; names provisional, FO-Q11).
fn batch2_falloff_args(
    f: &mut crate::Falloff,
    key: &dyn Fn(&str) -> Option<String>,
) -> std::result::Result<(), String> {
    let number = |k: &str| -> std::result::Result<Option<f64>, String> {
        match key(k) {
            None => Ok(None),
            Some(v) => v
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .map(Some)
                .ok_or(format!("{k} is a number")),
        }
    };
    let flag = |k: &str| -> std::result::Result<Option<bool>, String> {
        match key(k).map(|v| v.to_ascii_lowercase()).as_deref() {
            None => Ok(None),
            Some("true" | "on" | "1") => Ok(Some(true)),
            Some("false" | "off" | "0") => Ok(Some(false)),
            Some(_) => Err(format!("{k} is true or false")),
        }
    };
    let shape = |current: crate::FalloffShape| -> std::result::Result<crate::FalloffShape, String> {
        let mut s = match key("shape").map(|v| v.to_ascii_lowercase()).as_deref() {
            None => current,
            Some("0" | "linear") => crate::FalloffShape::Linear,
            Some("1" | "easein") => crate::FalloffShape::EaseIn,
            Some("2" | "easeout") => crate::FalloffShape::EaseOut,
            Some("3" | "smooth") => crate::FalloffShape::Smooth,
            Some("4" | "custom") => crate::FalloffShape::Custom {
                ease_in: 0.,
                ease_out: 0.,
            },
            Some(_) => {
                return Err("shape is linear, easeIn, easeOut, smooth or custom (0-4)".into());
            }
        };
        if let crate::FalloffShape::Custom { ease_in, ease_out } = &mut s {
            if let Some(v) = number("p0")? {
                *ease_in = v;
            }
            if let Some(v) = number("p1")? {
                *ease_out = v;
            }
        }
        Ok(s)
    };
    match f {
        crate::Falloff::SoftSelection(s) => {
            if let Some(r) = number("radius")? {
                s.radius = r;
            }
            if let Some(c) = flag("connectivity")? {
                s.connectivity = c;
            }
            s.shape = shape(s.shape)?;
        }
        crate::Falloff::Coplanar(c) => {
            if let Some(a) = number("angle")? {
                c.angle = a;
            }
            if let Some(on) = flag("connectivity")? {
                c.connectivity = on;
            }
            c.shape = shape(c.shape)?;
        }
        crate::Falloff::Element(e) => {
            if let Some(m) = key("mode") {
                e.mode = crate::TweakMode::ALL
                    .into_iter()
                    .find(|x| x.label().replace(' ', "").eq_ignore_ascii_case(&m))
                    .ok_or("mode is auto, autoCenter, vertex, edge, edgeCenter, polygon or polygonCenter")?;
            }
            if let Some(r) = number("range")? {
                e.range = r;
            }
            if let Some(c) = key("connect") {
                e.connect = match c.to_ascii_lowercase().as_str() {
                    "ignore" => crate::TweakConnect::Ignore,
                    "connectivity" => crate::TweakConnect::Connectivity,
                    "rigid" => crate::TweakConnect::Rigid,
                    "edgeloops" => crate::TweakConnect::EdgeLoops,
                    _ => return Err("connect is ignore, connectivity, rigid or edgeLoops".into()),
                };
            }
            e.shape = shape(e.shape)?;
        }
        // Batch 6. The Path's points are clicked in the view.
        crate::Falloff::Path(f) => {
            if let Some(v) = number("size")? {
                f.size = v;
            }
            if let Some(on) = flag("world")? {
                f.world = on;
            }
        }
        crate::Falloff::Curvature(f) => {
            if let Some(v) = number("scale")? {
                f.scale = v;
            }
        }
        // Batch 5. The Image falloff's picture is chosen in the app.
        crate::Falloff::Noise(f) => {
            if let Some(v) = number("scale")? {
                f.scale = v;
            }
            if let Some(on) = flag("world")? {
                f.world = on;
            }
        }
        crate::Falloff::Image(f) => {
            if let Some(on) = flag("repeat")? {
                f.repeat = on;
            }
            if let Some(v) = number("softborder")? {
                f.soft_border = v;
            }
            if let Some(on) = flag("useuv")? {
                f.use_uv = on;
            }
            if let Some(v) = number("scale")? {
                f.scale = v;
            }
            if let Some(v) = number("posx")? {
                f.pos.x = v;
            }
            if let Some(v) = number("posy")? {
                f.pos.y = v;
            }
            if let Some(v) = number("rotation")? {
                f.rotation = v;
            }
            if let Some(on) = flag("world")? {
                f.world = on;
            }
        }
        // Batch 4: the drawn shape itself comes from the view.
        crate::Falloff::Screen(s) => {
            if let Some(v) = number("size")? {
                s.size = v;
            }
            if let Some(on) = flag("transparent")? {
                s.transparent = on;
            }
        }
        crate::Falloff::Airbrush(a) => {
            if let Some(v) = number("size")? {
                a.size = v;
            }
            if let Some(v) = number("strength")? {
                a.strength = v;
            }
            if let Some(on) = flag("transparent")? {
                a.transparent = on;
            }
        }
        crate::Falloff::Lasso(l) => {
            if let Some(s) = key("style") {
                l.style = crate::LassoStyle::ALL
                    .into_iter()
                    .find(|x| x.label().eq_ignore_ascii_case(&s))
                    .ok_or("style is lasso, rectangle, circle or ellipse")?;
            }
            if let Some(v) = number("softborder")? {
                l.soft_border = v;
            }
            if let Some(on) = flag("world")? {
                l.world = on;
            }
        }
        _ => return Ok(()),
    }
    f.validate()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_arguments_become_the_runners_json() {
        let w = |s: &str| tokens(s).unwrap();
        let a = json_args("item.transform", &w("translation:1,2,3 item:4")).unwrap();
        assert_eq!(a["translation"], serde_json::json!([1.0, 2.0, 3.0]));
        assert_eq!(a["item"], serde_json::json!(4));
        let a = json_args("item.name", &w("{Road Sign}")).unwrap();
        assert_eq!(a["name"], "Road Sign");
        assert!(json_args("item.name", &w("")).unwrap_err().contains("name"));
        assert!(json_args("item.transform", &w("translation:1,2")).is_err());
        assert!(json_args("item.select", &w("item:x")).is_err());
    }

    #[test]
    fn headless_lines_run_without_a_window() {
        let mut e = Editor::default();
        assert!(run_line(&mut e, "mesh.new name:Base").unwrap().is_ok());
        assert_eq!(
            run_line(&mut e, "item.channel name ?").map(|r| r.is_err()),
            Some(true)
        );
        assert_eq!(e.scene.active().unwrap().name, "Base");
        assert!(run_line(&mut e, "item.rename Top").unwrap().is_ok());
        assert_eq!(e.scene.active().unwrap().name, "Top");
        assert!(run_line(&mut e, "Top View").is_none());
        assert!(run_line(&mut e, "no.such.command").is_none());
    }

    #[test]
    fn vertex_lines_parse_without_the_app() {
        let w = |s: &str| tokens(s).unwrap();
        assert_eq!(
            parse_vertex("vert.new", &w("")),
            Some(VertexLine::Row("vert.new"))
        );
        assert_eq!(
            parse_vertex("vert.new", &w("x:1 y:2 z:50cm")),
            Some(VertexLine::New(glam::DVec3::new(1., 2., 0.5)))
        );
        assert_eq!(
            parse_vertex("vert.weld", &w("target:3")),
            Some(VertexLine::Weld(3))
        );
        assert_eq!(parse_vertex("vert.weld", &w("3")), Some(VertexLine::Usage));
        assert_eq!(
            parse_vertex("vert.merge", &w("keep:true")),
            Some(VertexLine::Command(crate::VertexCommand::Merge {
                distance: 0.,
                options: crate::MergeOptions {
                    keep_small: true,
                    ..Default::default()
                },
            }))
        );
        assert_eq!(
            parse_vertex("view3d.showvertices", &w("state:?")),
            Some(VertexLine::ShowVerticesQuery)
        );
        assert!(matches!(
            parse_vertex("vert.join", &w("average:1 average:0")),
            Some(VertexLine::Error(_))
        ));
        assert_eq!(parse_vertex("vert.split", &w("x")), Some(VertexLine::Usage));
        assert_eq!(parse_vertex("poly.flip", &w("")), None);
    }

    #[test]
    fn sculpt_lines_parse_without_the_app() {
        use crate::SculptBrush;
        assert_eq!(
            parse_sculpt("Sculpt Push"),
            Some(SculptLine::Start(SculptBrush::Push))
        );
        assert_eq!(
            parse_sculpt("tool.set {sculpt.push} ?"),
            Some(SculptLine::ToolSet(SculptBrush::Push, Switch::Query))
        );
        assert!(matches!(
            parse_sculpt("sculpt.push x"),
            Some(SculptLine::Usage(_))
        ));
        assert_eq!(
            parse_sculpt("sculpt.cornerLock"),
            Some(SculptLine::Lock {
                corners: true,
                switch: Switch::Toggle
            })
        );
        assert_eq!(parse_sculpt("poly.flip"), None);
        assert_eq!(parse_sculpt("sculpt nothing"), None);
    }

    #[test]
    fn tool_set_binds_by_name_or_order() {
        let w = |s: &str| tokens(s).unwrap();
        assert_eq!(tool_set_args(&w("prim.cube on")), Some(("prim.cube", "on")));
        assert_eq!(
            tool_set_args(&w("mode:? preset:poly.bevel")),
            Some(("poly.bevel", "?"))
        );
        assert_eq!(tool_set_args(&w("prim.cube")), None);
        assert_eq!(tool_set_args(&w("prim.cube on extra")), None);
        assert_eq!(tool_set_args(&w("preset:a preset:b")), None);
        assert_eq!(tool_set_args(&w("tool:x on")), None);
    }
}
