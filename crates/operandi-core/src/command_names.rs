//! Command and tool name registry (architecture wave 3, items #5 and #12).
//!
//! Operandi's command, tool and argument names follow the reference name map
//! (`target/command-names-2026-11-06`, Jason 2026-11-06) so existing scripts
//! port. Old Operandi names stay accepted as deprecated aliases: every entry
//! point (command line, menus, pie entries, saved bindings, the MCP bridge)
//! passes its text or names through here before parsing, so the parsers only
//! ever see the new names. Names compare case-insensitively, like `tool.set`.
use crate::Mode;
use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::Mutex;

/// `tool.set` ids: (old, new).
///
/// Kept in sorted order (case-insensitive), so parallel branches add
/// lines in different places (AGENTS.md, "Code"); a test checks it.
pub const TOOL_RENAMES: [(&str, &str); 10] = [
    ("deform.push", "xfrm.push"),
    ("deform.smooth", "xfrm.smooth"),
    ("prim.torus", "prim.toroid"),
    ("slice.edge", "edge.knife"),
    ("slice.loop", "poly.loopSlice"),
    ("slice.polygon", "poly.knife"),
    ("tweak", "ElementMove"),
    ("vert.create", "prim.makeVertex"),
    // `tool.set vert.merge` is the tool; bare `vert.merge` stays the command.
    ("vert.mergeTool", "vert.merge"),
    ("vert.weld", "DragWeld"),
];

/// Old tool ids that are still current command verbs, so `help <id>`
/// keeps showing the command (`vert.weld target:<n>` is a command too).
const STILL_COMMANDS: [&str; 1] = ["vert.weld"];

/// Command verbs: (old, new). Several come from registry stage 2 (the
/// name map subset in specifications/command-names).
///
/// Kept in sorted order (case-insensitive), so parallel branches add
/// lines in different places (AGENTS.md, "Code"); a test checks it.
pub const COMMAND_RENAMES: [(&str, &str); 17] = [
    ("backdrop.attr", "backdrop.edit"),
    ("backdrop.autoFit", "backdrop.fit"),
    ("backdrop.create", "backdrop.add"),
    ("edge.spin", "edge.spinQuads"),
    ("edge.toCurves", "edges.toCurve"),
    ("image.reload", "clip.reload"),
    ("item.add", "item.create"),
    // The MCP bridge's old name.
    ("item.rename", "item.name"),
    // PD-Q12 (UIC-162): the map's Select Item.
    ("item.select", "select.item"),
    ("poly.flattenNgons", "@ppi_flatten_ngons.py"),
    ("preset.load", "preset.do"),
    ("preset.save", "preset.saveAs"),
    ("selset.assign", "select.editSet"),
    ("selset.delete", "select.deleteSet"),
    ("selset.group", "select.convertGroup"),
    ("selset.use", "select.useSet"),
    ("view.showVertices", "view3d.showVertices"),
];

/// Old verbs that became a new verb plus fixed arguments: (old, new verb,
/// arguments appended after any the caller gave).
///
/// Kept in sorted order (case-insensitive), so parallel branches add
/// lines in different places (AGENTS.md, "Code"); a test checks it.
pub const COMMAND_EXPANSIONS: [(&str, &str, &str); 3] = [
    (
        "@ppi_PolyScale_Offset.py detach",
        "@ppi_PolyScale_Offset_Detach.py",
        "",
    ),
    ("edge.joinAverage", "edge.join", "average:true"),
    ("selset.deleteAll", "select.deleteSet", "all:true"),
];

/// Argument names: (owner under its new name, old, new). The owner is the
/// `tool.attr` tool or the command whose `name:value` arguments these are.
///
/// Kept in sorted order (case-insensitive), so parallel branches add
/// lines in different places (AGENTS.md, "Code"); a test checks it.
pub const ARG_RENAMES: [(&str, &str, &str); 9] = [
    ("chamfer.edit", "sourceSegments", "sourceSegs"),
    ("edge.bevel", "material", "matName"),
    ("edge.bevel", "useMaterial", "useMat"),
    ("edge.chamfer", "material", "matName"),
    ("edge.chamfer", "useMaterial", "useMat"),
    ("poly.reduct", "legacy", "method"),
    ("preset.do", "path", "preset"),
    ("preset.saveAs", "path", "filename"),
    // The MCP bridge's old JSON name.
    ("vert.merge", "distance", "dist"),
];

/// `*.x` tool names pick their vert/edge/poly/item variant by selection
/// mode: (name, vertex, edge, polygon, item). `None` = no such variant yet.
pub const STAR_TOOLS: [(&str, [Option<&str>; 4]); 1] = [(
    "*.Slide",
    // Provisional: Operandi has no Polygon or Item Slide.
    [Some("vert.slide"), Some("edge.slide"), None, None],
)];

fn lookup<'a>(table: &'a [(&'a str, &'a str)], name: &str) -> Option<&'a str> {
    table
        .iter()
        .find(|(old, _)| old.eq_ignore_ascii_case(name))
        .map(|(_, new)| *new)
}

/// The current tool id for a deprecated one, or `None` when `name` isn't old.
pub fn renamed_tool(name: &str) -> Option<&'static str> {
    lookup(&TOOL_RENAMES, name)
}

/// The current command verb for a deprecated one.
pub fn renamed_command(name: &str) -> Option<&'static str> {
    lookup(&COMMAND_RENAMES, name)
}

/// The current argument name for a deprecated `arg` of `owner` (either name
/// of the owner works).
pub fn renamed_arg(owner: &str, arg: &str) -> Option<&'static str> {
    let owner = canonical_name(owner);
    ARG_RENAMES
        .iter()
        .find(|(o, old, _)| o.eq_ignore_ascii_case(&owner) && old.eq_ignore_ascii_case(arg))
        .map(|(_, _, new)| *new)
}

/// `name` under its current tool or command name (unchanged when current).
pub fn canonical_name(name: &str) -> String {
    renamed_tool(name)
        .or_else(|| renamed_command(name))
        .map_or_else(|| name.to_owned(), str::to_owned)
}

/// The variant a `*.x` name picks in `mode`: `Ok(None)` when `name` isn't a
/// `*.x` name, an error when that mode has no variant.
pub fn star_tool(name: &str, mode: Mode) -> std::result::Result<Option<&'static str>, String> {
    let Some((star, variants)) = STAR_TOOLS
        .iter()
        .find(|(star, _)| star.eq_ignore_ascii_case(name))
    else {
        return Ok(None);
    };
    let slot = match mode {
        Mode::Vertex => 0,
        Mode::Edge => 1,
        Mode::Polygon => 2,
        Mode::Item => 3,
    };
    variants[slot]
        .map(Some)
        .ok_or_else(|| format!("{star} has no {mode:?} variant; use Vertex or Edge mode"))
}

/// A `percent` argument in the map's units: a bare number is a fraction
/// (0.5 = 50 %), a number with `%` is a percent. Returns percent (0–100),
/// the unit the tools and panels keep (Jason, 2026-11-07: scripts use the
/// map's units; panels keep showing %).
pub fn script_percent(text: &str) -> crate::Result<f64> {
    use crate::numeric::{Quantity, parse_quantity};
    if text.contains('%') {
        parse_quantity(text, Quantity::Percent)
    } else {
        Ok(parse_quantity(text, Quantity::Scalar)? * 100.)
    }
}

/// An `angle` argument in the map's units: a bare number is radians; a
/// number with `deg`, `°` or `rad` keeps its unit. Returns degrees, the
/// unit the tools and panels keep.
pub fn script_angle(text: &str) -> crate::Result<f64> {
    use crate::numeric::{Quantity, parse_quantity};
    if text.chars().any(|c| c.is_alphabetic() || c == '°') {
        parse_quantity(text, Quantity::Angle)
    } else {
        Ok(parse_quantity(text, Quantity::Scalar)?.to_degrees())
    }
}

static NOTED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// The deprecation line for `old`, the first time it is used this run.
pub fn first_deprecation(old: &str, new: &str) -> Option<String> {
    let mut noted = NOTED.lock().unwrap_or_else(|e| e.into_inner());
    noted
        .insert(old.to_ascii_lowercase())
        .then(|| format!("Deprecated command name: {old} is now {new}"))
}

/// A command line with old names replaced, and which ones were.
#[derive(Debug, Default, PartialEq)]
pub struct Rewrite {
    pub text: String,
    /// (old, new) for each replaced name, in order.
    pub renamed: Vec<(String, String)>,
}

/// Byte ranges of the whitespace-separated tokens of `s`, keeping quotes and
/// braces inside their token (the app's tokenizer rules). Unbalanced input
/// just ends the last token; the parser reports it later.
fn token_spans(s: &str) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut start = None;
    let mut quote = false;
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        if start.is_none() {
            if c.is_whitespace() {
                continue;
            }
            start = Some(i);
        }
        match c {
            '"' if depth == 0 => quote = !quote,
            '{' if !quote => depth += 1,
            '}' if !quote => depth = depth.saturating_sub(1),
            c if c.is_whitespace() && !quote && depth == 0 => {
                spans.push(start.take().unwrap()..i);
            }
            _ => {}
        }
    }
    if let Some(start) = start {
        spans.push(start..s.len());
    }
    spans
}

/// A token's text without one pair of surrounding quotes or braces.
fn bare(token: &str) -> &str {
    let t = token.trim();
    for (open, close) in [('"', '"'), ('{', '}')] {
        if t.len() >= 2 && t.starts_with(open) && t.ends_with(close) {
            return &t[1..t.len() - 1];
        }
    }
    t
}

/// Rewrites deprecated names in one command line: the verb, the tool id of
/// `tool.set|attr|reset` and `help`, the `tool.attr` attribute, `name:` keys
/// of renamed arguments, and `*.x` tool names by `mode`. Everything else,
/// quoting included, is kept byte for byte.
pub fn rewrite(line: &str, mode: Mode) -> std::result::Result<Rewrite, String> {
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let mut renamed = Vec::new();
    let spans = token_spans(line);
    let Some(first) = spans.first().cloned() else {
        return Ok(Rewrite {
            text: line.to_owned(),
            renamed,
        });
    };
    let verb = &line[first.clone()];
    // Two-token expansion (`@ppi_PolyScale_Offset.py detach`).
    if spans.len() == 2 {
        let two = format!("{verb} {}", &line[spans[1].clone()]);
        if let Some((old, new, _)) = COMMAND_EXPANSIONS
            .iter()
            .find(|(old, _, _)| old.eq_ignore_ascii_case(&two))
        {
            return Ok(Rewrite {
                text: (*new).to_owned(),
                renamed: vec![((*old).to_owned(), (*new).to_owned())],
            });
        }
    }
    let mut owner = verb.to_owned();
    let mut appended = "";
    if let Some(new) = renamed_command(verb) {
        renamed.push((verb.to_owned(), new.to_owned()));
        edits.push((first.clone(), new.to_owned()));
        owner = new.to_owned();
    } else if let Some((old, new, args)) = COMMAND_EXPANSIONS
        .iter()
        .find(|(old, _, _)| !old.contains(' ') && old.eq_ignore_ascii_case(verb))
    {
        renamed.push(((*old).to_owned(), format!("{new} {args}").trim().to_owned()));
        edits.push((first.clone(), (*new).to_owned()));
        owner = (*new).to_owned();
        appended = args;
    }
    let lowered = verb.to_ascii_lowercase();
    let names_tool = matches!(
        lowered.as_str(),
        "tool.set" | "tool.attr" | "tool.reset" | "help"
    );
    let mut rest = spans[1..].iter().cloned().enumerate();
    if names_tool && let Some((_, span)) = rest.next() {
        // `tool.set preset:{id} mode:on` names the tool in a `preset:` value.
        let token = &line[span.clone()];
        let (value_span, value) = match token.split_once(':') {
            Some((key, value)) if key.eq_ignore_ascii_case("preset") => {
                (span.start + key.len() + 1..span.end, value)
            }
            _ => (span.clone(), token),
        };
        let id = bare(value);
        let star = if lowered == "help" {
            None
        } else {
            star_tool(id, mode)?
        };
        let new = star
            .map(str::to_owned)
            .or_else(|| {
                let still =
                    lowered == "help" && STILL_COMMANDS.iter().any(|c| c.eq_ignore_ascii_case(id));
                (!still).then(|| renamed_tool(id).map(str::to_owned))?
            })
            .or_else(|| {
                (lowered == "help").then(|| {
                    renamed_command(id).map(str::to_owned).or_else(|| {
                        COMMAND_EXPANSIONS
                            .iter()
                            .find(|(old, _, _)| old.eq_ignore_ascii_case(id))
                            .map(|(_, new, _)| (*new).to_owned())
                    })
                })?
            });
        if let Some(new) = new {
            if star.is_none() {
                renamed.push((id.to_owned(), new.clone()));
            }
            edits.push((value_span, new.clone()));
            owner = new;
        } else {
            owner = id.to_owned();
        }
        // `tool.attr <tool> <attr> <value>`: the attribute is positional.
        if lowered == "tool.attr"
            && let Some((_, span)) = rest.next()
        {
            let attr = &line[span.clone()];
            if let Some(new) = renamed_arg(&owner, attr) {
                renamed.push((attr.to_owned(), new.to_owned()));
                edits.push((span, new.to_owned()));
            }
        }
    }
    // `name:value` arguments of the command (or the tool.set tool).
    for (_, span) in rest {
        let token = &line[span.clone()];
        if let Some((key, _)) = token.split_once(':')
            && let Some(new) = renamed_arg(&owner, key)
        {
            renamed.push((key.to_owned(), new.to_owned()));
            edits.push((span.start..span.start + key.len(), new.to_owned()));
        }
    }
    let mut text = line.to_owned();
    for (range, new) in edits.into_iter().rev() {
        text.replace_range(range, &new);
    }
    if !appended.is_empty() {
        text = format!("{} {appended}", text.trim_end());
    }
    Ok(Rewrite { text, renamed })
}

/// `text` without a leading `key:` (any case), if it has one.
pub fn strip_key<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.split_once(':')
        .filter(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &str) -> String {
        rewrite(line, Mode::Polygon).unwrap().text
    }

    /// The first entry out of case-insensitive order, as (index, entry,
    /// the entry before it).
    fn out_of_order(keys: &[String]) -> Option<(usize, &str, &str)> {
        keys.windows(2)
            .position(|w| w[0].to_lowercase() > w[1].to_lowercase())
            .map(|i| (i + 1, keys[i + 1].as_str(), keys[i].as_str()))
    }

    /// Contributor rule (the maintainer, 2026-11-19): shared tables take one
    /// added line per new name, in sorted order, so branches don't clash.
    #[test]
    fn name_tables_stay_sorted() {
        let tables: [(&str, Vec<String>); 4] = [
            (
                "TOOL_RENAMES",
                TOOL_RENAMES.iter().map(|(o, _)| o.to_string()).collect(),
            ),
            (
                "COMMAND_RENAMES",
                COMMAND_RENAMES.iter().map(|(o, _)| o.to_string()).collect(),
            ),
            (
                "COMMAND_EXPANSIONS",
                COMMAND_EXPANSIONS
                    .iter()
                    .map(|(o, _, _)| o.to_string())
                    .collect(),
            ),
            (
                "ARG_RENAMES",
                ARG_RENAMES
                    .iter()
                    .map(|(w, o, _)| format!("{w} {o}"))
                    .collect(),
            ),
        ];
        for (name, keys) in &tables {
            if let Some((at, entry, before)) = out_of_order(keys) {
                panic!(
                    "command_names.rs: {name} must stay in sorted order (case-insensitive); \
                     entry {at} `{entry}` belongs before `{before}`"
                );
            }
        }
    }

    #[test]
    fn every_alias_resolves_to_its_new_name() {
        for (old, new) in TOOL_RENAMES {
            assert_eq!(renamed_tool(old), Some(new));
            assert_eq!(renamed_tool(&old.to_ascii_uppercase()), Some(new));
            assert_eq!(renamed_tool(new), None, "{new} is current");
            assert_eq!(
                text(&format!("tool.set {old} on")),
                format!("tool.set {new} on")
            );
        }
        for (old, new) in COMMAND_RENAMES {
            assert_eq!(renamed_command(old), Some(new));
            assert_eq!(renamed_command(new), None, "{new} is current");
            assert_eq!(text(&format!("{old} a:1")), format!("{new} a:1"));
            assert_eq!(text(&format!("help {old}")), format!("help {new}"));
        }
        for (owner, old, new) in ARG_RENAMES {
            assert_eq!(renamed_arg(owner, old), Some(new));
            assert_eq!(renamed_arg(owner, new), None);
        }
    }

    #[test]
    fn expansions_add_their_arguments() {
        assert_eq!(text("edge.joinAverage"), "edge.join average:true");
        assert_eq!(text("selset.deleteAll"), "select.deleteSet all:true");
        assert_eq!(
            text("@ppi_polyscale_offset.py DETACH"),
            "@ppi_PolyScale_Offset_Detach.py"
        );
        assert_eq!(text("@ppi_PolyScale_Offset.py"), "@ppi_PolyScale_Offset.py");
        assert_eq!(text("help edge.joinAverage"), "help edge.join");
        // vert.weld is still a command; only tool.set renames it.
        assert_eq!(text("help vert.weld"), "help vert.weld");
        assert_eq!(text("tool.set vert.weld on"), "tool.set DragWeld on");
    }

    #[test]
    fn arguments_and_quoting_survive() {
        assert_eq!(
            text("chamfer.edit offset:0.1 segments:2 sourceSegments:1 depth:50"),
            "chamfer.edit offset:0.1 segments:2 sourceSegs:1 depth:50"
        );
        assert_eq!(
            text("tool.attr edge.chamfer useMaterial on"),
            "tool.attr edge.chamfer useMat on"
        );
        assert_eq!(
            text("tool.attr poly.reduct legacy 1"),
            "tool.attr poly.reduct method 1"
        );
        assert_eq!(
            text("tool.set preset:{slice.loop} mode:on"),
            "tool.set preset:poly.loopSlice mode:on"
        );
        assert_eq!(
            text("selset.use {Front Rim}  add"),
            "select.useSet {Front Rim}  add"
        );
        assert_eq!(text("poly.setPart \"a b\""), "poly.setPart \"a b\"");
        let r = rewrite("tool.set tweak on", Mode::Vertex).unwrap();
        assert_eq!(r.renamed, [("tweak".to_owned(), "ElementMove".to_owned())]);
        assert!(
            rewrite("tool.set ElementMove on", Mode::Vertex)
                .unwrap()
                .renamed
                .is_empty()
        );
    }

    #[test]
    fn star_names_pick_by_mode() {
        let set = |mode| rewrite("tool.set \"*.Slide\" on", mode).map(|r| r.text);
        assert_eq!(set(Mode::Vertex).unwrap(), "tool.set vert.slide on");
        assert_eq!(set(Mode::Edge).unwrap(), "tool.set edge.slide on");
        assert!(set(Mode::Polygon).unwrap_err().contains("Vertex or Edge"));
        // Picking a variant isn't a deprecated use.
        assert!(
            rewrite("tool.set *.slide on", Mode::Edge)
                .unwrap()
                .renamed
                .is_empty()
        );
    }

    #[test]
    fn script_units_are_the_maps() {
        assert_eq!(script_percent("0.5").unwrap(), 50.);
        assert_eq!(script_percent("1").unwrap(), 100.);
        assert_eq!(script_percent("50%").unwrap(), 50.);
        assert_eq!(script_percent("50 %").unwrap(), 50.);
        assert!(script_percent("abc").is_err());
        let half_pi = std::f64::consts::FRAC_PI_2;
        assert!((script_angle(&half_pi.to_string()).unwrap() - 90.).abs() < 1e-9);
        assert_eq!(script_angle("45deg").unwrap(), 45.);
        assert_eq!(script_angle("45°").unwrap(), 45.);
        assert!((script_angle("1rad").unwrap() - 1f64.to_degrees()).abs() < 1e-9);
    }

    #[test]
    fn deprecation_is_noted_once() {
        assert!(first_deprecation("test.onlyOnce", "x").is_some());
        assert!(first_deprecation("TEST.onlyonce", "x").is_none());
    }
}
