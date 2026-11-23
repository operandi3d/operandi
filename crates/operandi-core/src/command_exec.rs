//! Commands that run with no window (extensibility plan stage 2, L6):
//! the runner (`run`) that turns a command and its JSON arguments into
//! `Command`s on an `Editor`. The MCP bridge's `command_run` and the app's
//! command line both use it. The arguments themselves live on each
//! command's entry in `command_registry::COMMANDS` (`Entry::json`), so the
//! registry is the one list; [`built_in`] is those entries' handler.
//!
//! Units follow the bridge-wide rule: lengths in metres, angles in radians,
//! fractions 0–1 (1 = 100 %). "absolute" means the value replaces the
//! current one; "offset" means it is added.
use crate::command_names;
use crate::{
    Command, EdgeCommand, Editor, MaterialAssignment, Mode, PartAssignment, Symmetry, UseSetMode,
    VertexCommand,
};
use glam::DVec3;
use serde_json::{Map, Value, json};

pub type Result<T> = std::result::Result<T, String>;

/// Logs an old name's first use this run (stderr; an MCP sidecar's stdout
/// is its protocol).
fn note_deprecated(old: &str, new: &str) {
    if let Some(note) = command_names::first_deprecation(old, new) {
        eprintln!("{note}");
    }
}

/// One JSON argument of a headless command (its entry in
/// `command_registry::COMMANDS` holds the list).
pub use crate::command_registry::JsonArg as Arg;

/// One `command_run` command: a view of a headless registry entry, its
/// script description and JSON arguments.
#[derive(Clone, Copy, Debug)]
pub struct CommandSpec {
    pub name: &'static str,
    pub about: &'static str,
    pub args: &'static [Arg],
}

impl CommandSpec {
    fn of(entry: &crate::command_registry::Entry) -> Option<Self> {
        let json = entry.json?;
        Some(CommandSpec {
            name: entry.name,
            about: json.about,
            args: json.args,
        })
    }
}

/// Every command that runs without a window, in registry order.
pub fn specs() -> impl Iterator<Item = CommandSpec> {
    crate::command_registry::COMMANDS
        .iter()
        .filter_map(CommandSpec::of)
}

/// The headless command named exactly `name`.
pub fn spec(name: &str) -> Option<CommandSpec> {
    specs().find(|s| s.name == name)
}

/// Refuse any argument `name` doesn't take, naming the ones it does.
pub fn check_args(name: &str, args: &Map<String, Value>) -> std::result::Result<(), String> {
    match spec(name) {
        Some(spec) => check_json_args(name, spec.args, args),
        None => Ok(()),
    }
}

/// Refuse any argument not in `takes` (a headless entry's JSON arguments).
pub fn check_json_args(
    name: &str,
    takes: &[Arg],
    args: &Map<String, Value>,
) -> std::result::Result<(), String> {
    if let Some(unknown) = args.keys().find(|k| !takes.iter().any(|a| a.name == *k)) {
        let accepted = if takes.is_empty() {
            "it takes no arguments".to_string()
        } else {
            format!(
                "it takes {}",
                takes.iter().map(|a| a.name).collect::<Vec<_>>().join(", ")
            )
        };
        return Err(format!(
            "{name} has no argument \"{unknown}\"; {accepted} (command_help {name} explains them)"
        ));
    }
    Ok(())
}

/// Refuse any key of an MCP tool's arguments not in `known`.
pub fn check_keys(
    tool: &str,
    args: &Map<String, Value>,
    known: &[&str],
) -> std::result::Result<(), String> {
    match args.keys().find(|k| !known.contains(&k.as_str())) {
        Some(unknown) => Err(format!(
            "{tool} has no argument \"{unknown}\"; it takes {}",
            known.join(", ")
        )),
        None => Ok(()),
    }
}

/// One command's help entry.
pub fn help_entry(spec: &CommandSpec) -> Value {
    let mut entry = json!({
        "command": spec.name,
        "about": spec.about,
        "args": spec.args.iter().map(|a| json!({
            "name": a.name, "type": a.kind, "required": a.required, "about": a.about,
        })).collect::<Vec<_>>(),
    });
    // Lists that come from core, so they stay current.
    match spec.name {
        "item.create" => {
            entry["types"] = json!(
                crate::item_list::AddItemKind::ALL
                    .iter()
                    .map(|k| k.key())
                    .collect::<Vec<_>>()
            );
        }
        // BRQ-92: every op type there is, kits' included.
        "meshop.create" => {
            entry["types"] = json!(
                crate::meshop_types::op_defs()
                    .iter()
                    .map(|d| json!({"type": d.id(), "label": d.label()}))
                    .collect::<Vec<_>>()
            );
        }
        "item.drop" => {
            entry["actions"] = json!(
                crate::DropAction::ALL
                    .iter()
                    .map(|k| k.key())
                    .collect::<Vec<_>>()
            );
        }
        _ => {}
    }
    entry
}

/// `command_help`: one command, or every command.
pub fn help(command: Option<&str>) -> std::result::Result<Value, String> {
    match command {
        // Old names still find their command (`item.add` is `item.create`).
        Some(name) => spec(name)
            .or_else(|| spec(&command_names::canonical_name(name)))
            .as_ref()
            .map(help_entry)
            .ok_or_else(|| format!("Unknown command {name}")),
        None => Ok(json!({
            "units": "lengths metres, angles radians, fractions 0–1; \"absolute\" replaces, \"offset\" adds",
            "commands": specs().map(|s| help_entry(&s)).collect::<Vec<_>>(),
        })),
    }
}

/// The command list for the `command_run` description: name and argument
/// names, one per line.
pub fn summary() -> String {
    specs()
        .map(|s| {
            let args: Vec<String> = s
                .args
                .iter()
                .map(|a| {
                    if a.required {
                        a.name.to_string()
                    } else {
                        format!("{}?", a.name)
                    }
                })
                .collect();
            format!("{} {{{}}}", s.name, args.join(", "))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The argument reference in docs/MCP_BRIDGE.md, between its
/// `<!-- command-args -->` markers (a test keeps the two the same).
pub fn markdown() -> String {
    let mut out = String::new();
    for s in specs() {
        out.push_str(&format!("- **`{}`** — {}", s.name, s.about));
        if s.args.is_empty() {
            out.push_str(" No arguments.\n");
            continue;
        }
        out.push('\n');
        for a in s.args {
            out.push_str(&format!(
                "  - `{}` ({}{}): {}\n",
                a.name,
                a.kind,
                if a.required { ", required" } else { "" },
                a.about
            ));
        }
    }
    out
}

/// A call made of several editor commands stays one undo step: joins the
/// undo steps recorded since `mark` (`Editor::history_mark`) into
/// one step named `label`. Steps are found by their marks, not by a list
/// index, so it still works when Max Undo Level trimmed the history while
/// they were recorded (Astra P4-003).
pub fn fold_steps(e: &mut Editor, mark: u64, label: &str) {
    let from = e.undo.len() - e.steps_since(mark);
    if e.undo.len() <= from + 1 {
        if e.undo.len() == from + 1
            && let Some(last) = e.undo.last_mut().map(std::sync::Arc::make_mut)
        {
            last.label = label.into();
        }
        return;
    }
    use std::sync::Arc;
    let mut steps = e.undo.split_off(from);
    let last = Arc::unwrap_or_clone(steps.pop().expect("more than one step"));
    let mut steps = steps.into_iter();
    let mut first = Arc::unwrap_or_clone(steps.next().expect("more than one step"));
    for step in steps {
        first.records.0.extend(Arc::unwrap_or_clone(step).records.0);
    }
    first.records.0.extend(last.records.0);
    first.after = last.after;
    first.identity_after = last.identity_after;
    first.label = label.into();
    e.undo.push(Arc::new(first));
}

/// The current command name for `given`, with the arguments a renamed
/// spelling implies (`edge.joinAverage` = `edge.join` with average true)
/// and old argument names renamed. Logs deprecated uses.
pub fn resolve_command(given: &str, args: &Map<String, Value>) -> (String, Map<String, Value>) {
    let mut name = given.to_owned();
    let mut args = args.clone();
    if let Some(new) = command_names::renamed_command(given) {
        note_deprecated(given, new);
        name = new.into();
    } else if let Some((old, new, fixed)) = command_names::COMMAND_EXPANSIONS
        .iter()
        .find(|(old, _, _)| old.eq_ignore_ascii_case(given))
    {
        note_deprecated(old, new);
        name = (*new).into();
        for pair in fixed.split_whitespace() {
            if let Some((k, v)) = pair.split_once(':') {
                let value = match v {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    v => Value::String(v.into()),
                };
                args.insert(k.into(), value);
            }
        }
    }
    for key in args.keys().cloned().collect::<Vec<_>>() {
        if let Some(new) = command_names::renamed_arg(&name, &key)
            && !args.contains_key(new)
        {
            note_deprecated(&format!("{name} {key}"), &format!("{name} {new}"));
            let value = args.remove(&key).expect("key is present");
            args.insert(new.into(), value);
        }
    }
    (name, args)
}

/// Runs one command on `e` with JSON arguments, as one undo step (most
/// commands) or a refusal that leaves the scene as it was. `given` may be
/// an old name; deprecated names and arguments are noted once on stderr.
pub fn run(e: &mut Editor, given: &str, given_args: &Map<String, Value>) -> Result<()> {
    if let Some(command) = command_for(e, given, given_args)? {
        e.dispatch(command)?;
    }
    Ok(())
}

/// The one `Command` that runs `given` with these arguments, for callers
/// that dispatch it themselves (the app's panels, so a drag folds into
/// one undo step). Commands that take several steps (`mesh.new name:`)
/// run on `e` here and give `None`.
pub fn command_for(
    e: &mut Editor,
    given: &str,
    given_args: &Map<String, Value>,
) -> Result<Option<Command>> {
    let (name, resolved) = resolve_command(given, given_args);
    // The registry decides what runs without a window (L6), and with what.
    let Some(headless) = crate::command_registry::find(&name).and_then(|e| e.json) else {
        return Err(format!("Unknown command {name}"));
    };
    check_args(&name, &resolved)?;
    (headless.run)(e, &name, &resolved)
}

/// The handler of every built-in headless entry (`Entry::json.run`): the
/// command `name` (current spelling, arguments already checked) on `e`.
/// `Ok(None)` when it took several steps and already ran them.
pub fn built_in(e: &mut Editor, name: &str, a: &Map<String, Value>) -> Result<Option<Command>> {
    let active = |e: &Editor| e.scene.selection.item.ok_or("No active item");
    if let Some(cmd) = crate::meshops_editor::script_command(e, name, a)? {
        return Ok(Some(cmd));
    }
    if let Some(cmd) = crate::uv_ops::script_command(e, name, a)? {
        return Ok(Some(cmd));
    }
    if let Some(cmd) = crate::bridge_commands::script_command(e, name, a)? {
        return Ok(Some(cmd));
    }
    if let Some(cmd) = crate::node_view_commands::script_command(e, name, a)? {
        return Ok(Some(cmd));
    }
    let cmd = match name {
        "mesh.new" => {
            let start = e.history_mark();
            e.dispatch(Command::NewMesh)?;
            if let Some(name) = a.get("name").filter(|v| !v.is_null()) {
                let name = name.as_str().ok_or("name is a string")?;
                e.dispatch(Command::Rename(name.into()))?;
                fold_steps(e, start, "New Mesh");
            }
            return Ok(None);
        }
        "smoothing.set" => {
            let mut s = e.scene.smoothing.clone();
            if let Some(v) = a.get("amount") {
                s.amount = num(v)? * 100.;
            }
            if let Some(v) = a.get("angle") {
                s.angle = num(v)?.to_degrees();
            }
            if let Some(v) = a.get("crease_by_angle") {
                s.crease_by_angle = boolean(v)?;
            }
            if let Some(v) = a.get("groups") {
                use crate::shading::SmoothingGroups as G;
                s.groups = match v.as_str().map(str::to_ascii_lowercase).as_deref() {
                    Some("off") => G::Off,
                    Some("crease") => G::Crease,
                    Some("force") => G::Force,
                    _ => return Err("groups is \"off\", \"crease\" or \"force\"".into()),
                };
            }
            Command::SetSmoothing {
                settings: s,
                continuing: false,
            }
        }
        // Items list header (provisional names, IL-CMD): `type` is an
        // AddItemKind key such as "light.spot"; `action` is select,
        // unselect or hide; `pattern` defaults to "*".
        "item.create" => {
            let kind = req_str(a, "type")?;
            Command::AddItem(
                crate::item_list::AddItemKind::from_key(kind)
                    .ok_or(format!("Unknown item type {kind}"))?,
            )
        }
        "item.pattern" => {
            let action = req_str(a, "action")?;
            Command::ItemPattern {
                action: crate::item_list::PatternAction::from_key(action)
                    .ok_or(format!("Unknown pattern action {action}"))?,
                query: crate::item_list::PatternQuery {
                    pattern: match a.get("pattern") {
                        None | Some(Value::Null) => "*".into(),
                        Some(v) => v.as_str().ok_or("pattern is a string")?.into(),
                    },
                    limit_to_children: opt_bool(a, "limit_to_children")?.unwrap_or(false),
                    ..Default::default()
                },
            }
        }
        // The map's Select Item; mode and mask are accepted, not used yet
        // (PD-Q12).
        "select.item" => Command::SelectItem(opt_u64(a, "item")?),
        // The map's Transform Channel (PD-Q12): one channel, through Item
        // Transform, which the core routes into a live item tool's tweak on
        // its Texture Locator (planar_texture.rs).
        "transform.channel" => {
            let (id, channel, value) = transform_channel_args(e, a)?;
            if value == "?" {
                return Err("transform.channel ? is a query; it changes nothing".into());
            }
            let item = e.scene.item(id).ok_or(format!("No item {id}"))?;
            let mut transform = item.transform.clone();
            crate::transform_channel::set(&mut transform, &channel, &value)?;
            Command::ItemTransform {
                item: id,
                transform,
            }
        }
        // `children`: "delete" deletes them too, "root" moves them to the
        // root in place (the default, GL-Q6); either deletes every
        // selected item. Without it only the current item goes.
        "item.delete" => match opt_str(a, "children")? {
            None => Command::DeleteItem,
            Some(key) => Command::DeleteItems(
                crate::item_delete::ChildrenOnDelete::from_key(key)
                    .ok_or(format!("Unknown children mode {key}; use delete or root"))?,
            ),
        },
        "item.name" => match opt_u64(a, "layer")? {
            None => Command::Rename(req_str(a, "name")?.into()),
            // A shader-tree layer (environment layers too).
            Some(id) => {
                let name = req_str(a, "name")?.trim();
                if name.is_empty() {
                    return Err("A layer name can't be empty".into());
                }
                match e.scene.environment_layer(id) {
                    // An Environment Material: its name.
                    Some((item, None)) => Command::EditEnvironment {
                        item,
                        edit: crate::environment::EnvironmentEdit::MaterialName(name.to_owned()),
                    },
                    // A layer under an environment.
                    Some((item, Some(node))) => {
                        let mut node = node.clone();
                        node.name = name.to_owned();
                        Command::EditEnvironmentLayers {
                            item,
                            edit: crate::environment::EnvironmentLayerEdit::Replace(Box::new(node)),
                        }
                    }
                    None => {
                        let mut node = e
                            .scene
                            .render_shader_tree()
                            .get(id)
                            .cloned()
                            .ok_or(format!("No layer {id}"))?;
                        node.name = name.to_owned();
                        Command::ShaderTree(crate::shader_tree::ShaderTreeEdit::Replace {
                            node,
                            continuing: false,
                        })
                    }
                }
            }
        },
        "environment.light" => {
            let item = match opt_u64(a, "envMaterial")? {
                Some(layer) => match e.scene.environment_layer(layer) {
                    Some((item, None)) => item,
                    _ => return Err(format!("{layer} isn't an Environment Material")),
                },
                None => {
                    let id = active(e)?;
                    e.scene
                        .items
                        .iter()
                        .find(|i| i.id == id && i.environment.is_some())
                        .ok_or("Select an environment, or give envMaterial")?
                        .id
                }
            };
            let sun = match a.get("sunLight") {
                Some(Value::String(s)) if s.eq_ignore_ascii_case("none") => None,
                Some(v) => Some(
                    v.as_u64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                        .ok_or("sunLight takes a light item id or none")?,
                ),
                None => return Err("sunLight is required".into()),
            };
            Command::EditEnvironment {
                item,
                edit: crate::environment::EnvironmentEdit::SunLight(sun),
            }
        }
        "item.visibility" => Command::Visibility(match opt_u64(a, "item")? {
            Some(id) => id,
            None => active(e)?,
        }),
        "item.parent" => Command::SetParent {
            item: match opt_u64(a, "item")? {
                Some(id) => id,
                None => active(e)?,
            },
            parent: opt_u64(a, "parent")?,
            keep_position: opt_bool(a, "keep_position")?.unwrap_or(true),
        },
        // Drop Action (drop-action-2026-11-06): `action` is a key such as
        // "match_position"; `sources` defaults to the item selection in
        // order; Place needs `ray_origin` and `ray_direction` (world).
        "item.drop" => {
            let action = req_str(a, "action")?;
            let action = crate::DropAction::from_key(action)
                .ok_or(format!("Unknown drop action {action}"))?;
            let sources = match a.get("sources") {
                None | Some(Value::Null) => {
                    let sel = &e.scene.selection;
                    sel.item
                        .into_iter()
                        .chain(sel.also.iter().copied())
                        .collect()
                }
                Some(v) => v
                    .as_array()
                    .ok_or("sources is an array of item ids")?
                    .iter()
                    .map(|id| id.as_u64().ok_or("sources is an array of item ids"))
                    .collect::<std::result::Result<Vec<u64>, _>>()?,
            };
            let ray = match (a.get("ray_origin"), a.get("ray_direction")) {
                (Some(o), Some(d)) => Some(crate::drop_action::Ray {
                    origin: vec3(o)?,
                    direction: vec3(d)?,
                }),
                (None, None) => None,
                _ => return Err("Give both ray_origin and ray_direction".into()),
            };
            Command::DropAction {
                action,
                sources,
                destination: opt_u64(a, "destination")?.ok_or("destination is required")?,
                ray,
            }
        }
        "item.transform" => {
            let id = match opt_u64(a, "item")? {
                Some(id) => id,
                None => active(e)?,
            };
            let item = e.scene.item(id).ok_or(format!("No item {id}"))?;
            let mut transform = item.transform.clone();
            if let Some(v) = a.get("translation") {
                transform.translation = vec3(v)?;
            }
            if let Some(v) = a.get("rotation") {
                transform.rotation = vec3(v)?;
            }
            if let Some(v) = a.get("scale") {
                transform.scale = vec3(v)?;
            }
            Command::ItemTransform {
                item: id,
                transform,
            }
        }
        "poly.flip" => Command::FlipPolygons,
        "poly.triple" => Command::Triple,
        "poly.quadruple" => Command::QuadruplePolygon,
        "poly.spinQuads" => Command::SpinQuads,
        "poly.align" => Command::AlignPolygons,
        "poly.split" => Command::SplitPolygon,
        "poly.remove" => Command::RemovePolygons,
        "poly.merge" => Command::MergePolygons,
        "poly.collapse" => Command::CollapsePolygon,
        "@ppi_flatten_ngons.py" => Command::FlattenNgons,
        "operandi.edge.zip" => Command::EdgeZip,
        "operandi.mesh.fixBlackDots" => Command::FixBlackDots,
        "operandi.poly.diagonalRedirect" => Command::DiagonalRedirect,
        "poly.make" => Command::MakePolygon,
        "poly.makeCurveOpen" => Command::MakeCurve { closed: false },
        "poly.makeCurveClosed" => Command::MakeCurve { closed: true },
        "poly.makeCurveFill" => Command::MakeCurveFill,
        "curve.winding" => Command::AlignWinding(crate::curve_commands::WindingOptions {
            merge: opt_bool(a, "merge")?.unwrap_or(true),
            reverse: opt_bool(a, "reverse")?.unwrap_or(false),
        }),
        "operandi.uv.rectangle" => {
            let d = crate::generator_ops::RectangleOptions::default();
            Command::UvRectangle(crate::generator_ops::RectangleOptions {
                map: opt_str(a, "name")?.map_or(d.map, str::to_owned),
                pack: opt_bool(a, "pack")?.unwrap_or(d.pack),
                gaps: a.get("gaps").map(num).transpose()?.unwrap_or(d.gaps),
                uniform_x: opt_bool(a, "uniformX")?.unwrap_or(d.uniform_x),
                uniform_y: opt_bool(a, "uniformY")?.unwrap_or(d.uniform_y),
                orient: opt_bool(a, "orient")?.unwrap_or(d.orient),
            })
        }
        "operandi.dicer" => {
            let d = crate::dicer::DicerOptions::default();
            Command::Dicer(crate::dicer::DicerOptions {
                seed: a.get("seed").map(as_u32).transpose()?.unwrap_or(d.seed),
                planes: a.get("planes").map(as_u32).transpose()?.unwrap_or(d.planes),
                gap: a.get("gap").map(num).transpose()?.unwrap_or(d.gap),
                inner_material: opt_str(a, "innerMaterial")?.unwrap_or_default().to_owned(),
                fragment_scale: DVec3::new(
                    a.get("scaleX").map(num).transpose()?.unwrap_or(1.),
                    a.get("scaleY").map(num).transpose()?.unwrap_or(1.),
                    a.get("scaleZ").map(num).transpose()?.unwrap_or(1.),
                ),
            })
        }
        "operandi.vertMap.transfer" => Command::TransferVertexMap {
            map: opt_str(a, "source")?.unwrap_or_default().to_owned(),
            kind: crate::generator_ops::TransferType::named(
                opt_str(a, "type")?.unwrap_or("position"),
            )?,
            world: match opt_str(a, "space")?
                .unwrap_or("world")
                .to_ascii_lowercase()
                .as_str()
            {
                "world" => true,
                "local" => false,
                other => return Err(format!("space must be local or world, not {other}")),
            },
            sampling: match opt_str(a, "method")?
                .unwrap_or("distance")
                .to_ascii_lowercase()
                .as_str()
            {
                "distance" => crate::generator_ops::TransferSampling::Distance,
                "raycast" => crate::generator_ops::TransferSampling::Raycast,
                other => return Err(format!("method must be raycast or distance, not {other}")),
            },
        },
        "poly.setPart" => Command::SetPart(PartAssignment {
            name: req_str(a, "name")?.into(),
            expected: None,
        }),
        "uv.unwrap" => Command::UvUnwrap(crate::uv_tools::unwrap_args(a, &e.uv_unwrap_preset)?),
        "operandi.uv.atlas" => {
            Command::UvAtlas(crate::uv_atlas::atlas_args(a, &e.uv_atlas_preset)?)
        }
        "uv.relax" => Command::UvRelax(crate::uv_tools::relax_args(a)?),
        // BRQ-110: Hide and Unhide as the H / Shift+H / Ctrl+H / U keys run them.
        "hide.sel" | "hide.unsel" | "hide.invert" | "unhide" => Command::EditElements {
            kind: crate::ElementKind::Hide,
            op: match name {
                "hide.sel" => crate::ElementOp::Selected,
                "hide.unsel" => crate::ElementOp::Unselected,
                "hide.invert" => crate::ElementOp::Invert,
                _ => crate::ElementOp::Clear,
            },
        },
        "uv.pack" => Command::PackUvs(crate::uv_tools::pack_args(a, &e.uv_pack_preset)?),
        // UV editing stage 1 (uv-editing-2026-11-16); `operandi.*` names
        // are placeholders (docs/NAMES-NEEDED.md).
        "operandi.uv.fit" => Command::UvIsland(crate::uv_edit::fit_args(a, &e.uv_island_memory)?),
        "operandi.uv.flip" => Command::UvIsland(crate::uv_edit::flip_args(a, &e.uv_island_memory)?),
        "operandi.uv.mirror" => {
            Command::UvIsland(crate::uv_edit::mirror_args(a, &e.uv_island_memory)?)
        }
        "operandi.uv.align" => {
            Command::UvIsland(crate::uv_edit::align_args(a, &e.uv_island_memory)?)
        }
        "operandi.uvMap.select" => Command::UvMap(crate::uv_maps::UvMapEdit::Select(
            req_str(a, "name")?.into(),
        )),
        "operandi.uvMap.rename" => Command::UvMap(crate::uv_maps::UvMapEdit::Rename {
            from: req_str(a, "name")?.into(),
            to: req_str(a, "newName")?.into(),
        }),
        "operandi.uvMap.delete" => Command::UvMap(crate::uv_maps::UvMapEdit::Delete(
            req_str(a, "name")?
                .split(',')
                .map(|n| n.trim().to_owned())
                .filter(|n| !n.is_empty())
                .collect(),
        )),
        "operandi.uvMap.copy" => {
            Command::UvMap(crate::uv_maps::UvMapEdit::Copy(req_str(a, "name")?.into()))
        }
        "operandi.uvMap.paste" => {
            Command::UvMap(crate::uv_maps::UvMapEdit::Paste(req_str(a, "name")?.into()))
        }
        "mesh.retopo" => Command::Remesh(crate::RemeshOp::Retopo(crate::remesh::args::retopo(
            a,
            &e.retopo_preset,
        )?)),
        "sculpt.remesh" => Command::Remesh(crate::RemeshOp::Voxel(crate::remesh::args::voxel(
            a,
            &e.voxel_remesh_preset,
        )?)),
        "operandi.bake.curvature" | "operandi.bake.thickness" => {
            let map = if name.ends_with("curvature") {
                crate::bake::BakeMap::Curvature
            } else {
                crate::bake::BakeMap::Thickness
            };
            let options = crate::bake::bake_args(a, &crate::bake::BakeOptions::new(map))?;
            e.command_note = Some(crate::bake::run_bake(&e.scene, a, &options)?);
            return Ok(None);
        }
        "operandi.bake.padImage" => {
            e.command_note = Some(crate::bake::run_pad(&e.scene, a)?);
            return Ok(None);
        }
        "operandi.remesh" => Command::Remesh(crate::RemeshOp::Octree(crate::remesh::args::octree(
            a,
            &e.octree_remesh_preset,
        )?)),
        "poly.boolean" => {
            let word = |k: &str| -> Result<Option<String>> {
                Ok(match a.get(k) {
                    None | Some(Value::Null) => None,
                    Some(Value::String(s)) => Some(s.clone()),
                    Some(Value::Number(n)) => Some(n.to_string()),
                    Some(_) => return Err(format!("{k} must be a number or a word")),
                })
            };
            Command::Boolean(crate::BooleanOptions {
                op: word("mode")?
                    .map(|w| crate::BooleanOp::parse(&w))
                    .transpose()?
                    .unwrap_or_default(),
                drive: word("cutmesh")?
                    .map(|w| crate::DriveMesh::parse(&w))
                    .transpose()?
                    .unwrap_or_default(),
                item: opt_u64(a, "item")?,
                children: opt_bool(a, "children")?.unwrap_or(false),
            })
        }
        "poly.solidDrill" | "poly.drill" => {
            let word = |k: &str| -> Result<Option<String>> {
                Ok(match a.get(k) {
                    None | Some(Value::Null) => None,
                    Some(Value::String(s)) => Some(s.clone()),
                    Some(Value::Number(n)) => Some(n.to_string()),
                    Some(_) => return Err(format!("{k} must be a number or a word")),
                })
            };
            let axis_drill = name == "poly.drill";
            let mode = word("mode")?.ok_or(format!("{name} requires mode"))?;
            let mut o = crate::DrillOptions {
                op: crate::DrillOp::parse(&mode)?,
                drive: word("cutmesh")?
                    .map(|w| crate::DriveMesh::parse(&w))
                    .transpose()?
                    .unwrap_or_default(),
                item: opt_u64(a, "item")?,
                children: opt_bool(a, "children")?.unwrap_or(false),
                ..Default::default()
            };
            if let Some(m) = word("material")? {
                o.material = m;
            }
            if axis_drill {
                let axis = word("axis")?.ok_or("poly.drill requires axis")?;
                o.axis = crate::DrillAxis::parse(&axis)?;
                Command::AxisDrill(o)
            } else {
                Command::SolidDrill(o)
            }
        }
        "poly.setMaterial" => Command::SetMaterial(MaterialAssignment {
            name: req_str(a, "name")?.into(),
            expected: None,
            material: None,
        }),
        // Selection Sets: `name` is the set (or group) name; select.useSet
        // takes `mode` replace|add|remove, select.deleteSet without `name`
        // deletes every set of the current mode, select.convertGroup takes `sets` (names;
        // empty or absent = all sets of the current mode).
        "select.editSet" => Command::AssignSelectionSet(req_str(a, "name")?.into()),
        "select.useSet" => Command::UseSelectionSet {
            name: req_str(a, "name")?.into(),
            mode: match a.get("mode").and_then(Value::as_str).unwrap_or("replace") {
                "replace" => UseSetMode::Replace,
                "add" => UseSetMode::Add,
                "remove" => UseSetMode::Remove,
                other => return Err(format!("mode must be replace, add or remove, not {other}")),
            },
        },
        // `all: true` (old: selset.deleteAll) or no name deletes every set.
        "select.deleteSet" => Command::DeleteSelectionSet(match opt_bool(a, "all")? {
            Some(true) => None,
            _ => a.get("name").and_then(Value::as_str).map(str::to_owned),
        }),
        "select.convertGroup" => Command::CreateSelectionSetGroup {
            name: req_str(a, "name")?.into(),
            sets: match a.get("sets") {
                None | Some(Value::Null) => Vec::new(),
                Some(Value::Array(list)) => list
                    .iter()
                    .map(|v| v.as_str().map(str::to_owned).ok_or("sets must be names"))
                    .collect::<std::result::Result<_, _>>()?,
                Some(_) => return Err("sets must be an array of names".into()),
            },
        },
        "selset.ungroup" => Command::DeleteSelectionSetGroup(req_str(a, "name")?.into()),
        "edge.subdivide" => match opt_u64(a, "count")? {
            Some(count) => Command::SubdivideEdges {
                count: u32::try_from(count).map_err(|_| "count is too large")?,
                // The map's percent is 0–1; the command keeps percent.
                percent: a.get("percent").map(num).transpose()?.map(|p| p * 100.),
            },
            None => Command::EdgeCommand(EdgeCommand::Subdivide),
        },
        "edge.collapse" => Command::EdgeCommand(EdgeCommand::Collapse),
        "edge.remove" => Command::EdgeCommand(EdgeCommand::Remove),
        "edge.spinQuads" => Command::EdgeCommand(EdgeCommand::Spin),
        "edge.split" => Command::EdgeCommand(EdgeCommand::Split),
        "edge.join" => Command::EdgeCommand(if opt_bool(a, "average")?.unwrap_or(false) {
            EdgeCommand::JoinAveraged
        } else {
            EdgeCommand::Join
        }),
        "edge.growQuads" => Command::EdgeCommand(EdgeCommand::GrowQuads),
        "edge.fillQuads" => Command::EdgeCommand(EdgeCommand::FillQuads),
        "edge.unbevel" => Command::EdgeCommand(EdgeCommand::Unbevel),
        "edge.addLoop" | "edge.addPoint" => {
            let (a_, b_, t) = (req_u32(a, "a")?, req_u32(a, "b")?, req_num(a, "t")?);
            if name == "edge.addLoop" {
                Command::AddEdgeLoop { a: a_, b: b_, t }
            } else {
                Command::AddEdgePoint { a: a_, b: b_, t }
            }
        }
        // `position` [x, y, z], or the map's x, y and z.
        "vert.new" => Command::NewVertex(match a.get("position") {
            Some(p) => vec3(p)?,
            None => DVec3::new(req_num(a, "x")?, req_num(a, "y")?, req_num(a, "z")?),
        }),
        "vert.weld" => Command::WeldVertices {
            sources: u32_list(a, "sources")?,
            target: req_u32(a, "target")?,
        },
        "vert.split" => Command::VertexCommand(VertexCommand::Split),
        "vert.splitEdge" => Command::VertexCommand(VertexCommand::SplitEdge),
        "vert.join" => Command::VertexCommand(VertexCommand::Join {
            average: opt_bool(a, "average")?.unwrap_or(false),
            options: crate::MergeOptions {
                keep_small: opt_bool(a, "keep")?.unwrap_or(false),
                ..Default::default()
            },
        }),
        "vert.merge" => Command::VertexCommand(VertexCommand::Merge {
            distance: a.get("dist").map(num).transpose()?.unwrap_or(0.),
            options: crate::MergeOptions {
                keep_small: opt_bool(a, "keep")?.unwrap_or(false),
                ..Default::default()
            },
        }),
        "vert.collapse" => Command::VertexCommand(VertexCommand::Collapse),
        "vert.remove" => Command::VertexCommand(VertexCommand::Remove),
        "vert.removeEdge" => Command::VertexCommand(VertexCommand::RemoveEdge),
        // The Delete key's command (name map `delete`, Jason 2026-11-10):
        // removes what is selected in the current mode and every point left
        // unused: points with the polygons using them (a hole), edges
        // dissolved with their leftover points, polygons; items deleted.
        "delete" => match e.scene.selection.mode {
            Mode::Vertex | Mode::Edge | Mode::Polygon => Command::DeleteComponents,
            // N10: every selected item (children move to the root).
            Mode::Item => Command::DeleteItems(Default::default()),
        },
        "vert.align" => {
            let axes = match a.get("axes") {
                None => [true; 3],
                Some(v) => {
                    let list = v
                        .as_array()
                        .filter(|l| l.len() == 3)
                        .ok_or("axes is [x, y, z] booleans")?;
                    [boolean(&list[0])?, boolean(&list[1])?, boolean(&list[2])?]
                }
            };
            Command::VertexCommand(VertexCommand::Align {
                axes,
                uniform: opt_bool(a, "uniform")?.unwrap_or(false),
            })
        }
        "symmetry.set" => Command::Symmetry(Symmetry {
            axis: match a.get("axis").and_then(Value::as_str) {
                None | Some("off") => None,
                Some("x") => Some(0),
                Some("y") => Some(1),
                Some("z") => Some(2),
                Some(other) => return Err(format!("Unknown symmetry axis {other}")),
            },
            offset: a.get("offset").map(num).transpose()?.unwrap_or(0.),
            // Arbitrary axis, Work Plane and Topology came later (SYM-B2-07..09).
            ..Symmetry::OFF
        }),
        "select.symmetrical" => Command::SelectSymmetrical,
        "item.setType" => {
            if a.get("mask").is_some_and(|m| !m.is_null()) {
                return Err("item.setType mask isn't supported yet; leave it out".into());
            }
            let id = match opt_u64(a, "item")? {
                Some(id) => id,
                None => active(e)?,
            };
            let name = req_str(a, "type")?;
            let kind = crate::camera_light::LightKind::ALL
                .iter()
                .copied()
                .find(|k| k.label().eq_ignore_ascii_case(name))
                .ok_or(format!("Unknown light type {name}"))?;
            Command::EditLight {
                item: id,
                edit: crate::camera_light::LightEdit::Kind(kind),
            }
        }
        "light.temperature" => {
            if a.get("gamma").is_some_and(|g| !g.is_null()) {
                return Err("light.temperature gamma isn't supported yet; leave it out".into());
            }
            let id = match opt_u64(a, "item")? {
                Some(id) => id,
                None => active(e)?,
            };
            Command::EditLight {
                item: id,
                edit: crate::camera_light::LightEdit::Temperature(req_num(a, "kelvins")?),
            }
        }
        "light.mesh" => {
            let id = match opt_u64(a, "light")? {
                Some(id) => id,
                None => active(e)?,
            };
            let mesh = match a.get("mesh") {
                Some(Value::String(s)) if s.eq_ignore_ascii_case("none") => None,
                Some(v) => Some(
                    v.as_u64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                        .ok_or("mesh takes a mesh item id or none")?,
                ),
                None => return Err("mesh is required".into()),
            };
            Command::EditLight {
                item: id,
                edit: crate::camera_light::LightEdit::Prototype(mesh),
            }
        }
        "texture.setIMap" => {
            if a.get("name").is_some_and(|n| !n.is_null()) {
                return Err("texture.setIMap name isn't supported yet; leave it out".into());
            }
            let node = opt_u64(a, "layer")?.ok_or("layer is required")?;
            let want = req_str(a, "image")?;
            let image = if want.eq_ignore_ascii_case("none") {
                None
            } else {
                let i = e
                    .scene
                    .images
                    .iter()
                    .find(|i| i.name == want || i.path == want)
                    .ok_or(format!("No scene image {want}"))?;
                Some((i.path.clone(), i.width, i.height))
            };
            Command::ShaderTree(crate::shader_tree::ShaderTreeEdit::SetImage { node, image })
        }
        "texture.delete" => {
            use crate::shader_tree::NodeKind;
            let id = opt_u64(a, "item")?.ok_or("item is required")?;
            if let Some((env, layer)) = e.scene.environment_layer(id) {
                if layer.is_none() {
                    return Err("An Environment Material can't be deleted on its own".into());
                }
                Command::EditEnvironmentLayers {
                    item: env,
                    edit: crate::environment::EnvironmentLayerEdit::Remove(vec![id]),
                }
            } else {
                match e.scene.shader_tree.get(id).map(|n| &n.kind) {
                    Some(NodeKind::Texture(_)) => {
                        Command::ShaderTree(crate::shader_tree::ShaderTreeEdit::Remove(vec![id]))
                    }
                    Some(_) => return Err(format!("Layer {id} isn't a texture layer")),
                    None => return Err(format!("No layer {id}")),
                }
            }
        }
        "texture.setLocator" | "texture.setVMap" | "mask.setPTag" | "mask.setPTagType" => {
            use crate::shader_tree::{NodeKind, TagMask, TagType, TextureLayer, TextureSource};
            let id = opt_u64(a, "layer")?.ok_or("layer is required")?;
            let mut node = e
                .scene
                .shader_tree
                .get(id)
                .cloned()
                .ok_or(format!("No layer {id}"))?;
            match (name, &mut node.kind) {
                ("texture.setLocator", NodeKind::Texture(t)) => {
                    let want = req_str(a, "locator")?;
                    t.locator = if want.eq_ignore_ascii_case("none") {
                        None
                    } else {
                        Some(
                            want.parse()
                                .map_err(|_| "locator takes an item id or none")?,
                        )
                    };
                }
                (
                    "texture.setVMap",
                    NodeKind::Texture(TextureLayer {
                        source: TextureSource::VertexMap { map, .. },
                        ..
                    }),
                ) => {
                    let want = req_str(a, "name")?;
                    *map = if want.eq_ignore_ascii_case("none") {
                        None
                    } else {
                        Some(
                            e.scene
                                .vertex_maps()
                                .into_iter()
                                .find(|m| m.name == want)
                                .ok_or(format!("No vertex map {want}"))?,
                        )
                    };
                }
                ("mask.setPTag", NodeKind::Group(g)) => {
                    let want = req_str(a, "name")?;
                    g.tag = match want {
                        "(all)" => TagMask::All,
                        "(none)" => TagMask::None,
                        other => TagMask::Name(other.to_owned()),
                    };
                }
                ("mask.setPTagType", NodeKind::Group(g)) => {
                    let want = req_str(a, "name")?;
                    let tag_type = TagType::ALL
                        .iter()
                        .copied()
                        .find(|t| t.label().eq_ignore_ascii_case(want))
                        .ok_or(format!("Unknown tag type {want}"))?;
                    // A new tag type starts at "(none)" (as the panel does).
                    if g.tag_type != tag_type {
                        g.tag = TagMask::None;
                    }
                    g.tag_type = tag_type;
                }
                _ => return Err(format!("{name} doesn't apply to layer {id}")),
            }
            Command::ShaderTree(crate::shader_tree::ShaderTreeEdit::Replace {
                node,
                continuing: false,
            })
        }
        "transform.reset" | "transform.freeze" => {
            let part = match opt_str(a, "type")? {
                None => crate::camera_props::TransformReset::All,
                Some(word) => {
                    crate::camera_props::TransformReset::parse(word).ok_or_else(|| {
                        format!("{name}: type is translation, rotation, scale or all, not {word}")
                    })?
                }
            };
            let items = e.scene.selected_items();
            if name == "transform.reset" {
                Command::ResetTransform { items, part }
            } else {
                Command::FreezeTransform { items, part }
            }
        }
        "poly.freeze" => Command::Freeze(freeze_args(a, &e.freeze_preset)?),
        // BRQ-60: every switch defaults on, the dialog's Reset values.
        "mesh.cleanup" => {
            const KEYS: [&str; 12] = [
                "floatingVertex",
                "onePointPolygon",
                "twoPointPolygon",
                "dupPointPolygon",
                "colinear",
                "faceNormal",
                "mergeVertex",
                "mergeDisco",
                "unifyPolygon",
                "forceUnify",
                "removeDiscoWeight",
                "fixGaps",
            ];
            let mut flags = [true; 12];
            for (flag, key) in flags.iter_mut().zip(KEYS) {
                if let Some(v) = opt_bool(a, key)? {
                    *flag = v;
                }
            }
            if flags.iter().all(|f| !f) {
                return Err("No Mesh Cleanup operations selected".into());
            }
            Command::MeshCleanup {
                flags,
                target: None,
            }
        }
        // BRQ-10: options not given take the Reset values, not the last used.
        "poly.subdivide" => {
            let (options, times) = subdivide_args(a)?;
            Command::SubdividePolygons { options, times }
        }
        "poly.unsubdiv" => Command::UnsubdividePolygons(unsubdivide_args(a)?),
        "vert.center" => {
            let word = req_str(a, "axis")?;
            Command::CenterSelection(
                crate::polygon_subdivide::center_axes(word)
                    .ok_or("axis is all, x, y, z, xy, yz or zx")?,
            )
        }
        "scale.absolute" => {
            let scale = absolute_args(a)?;
            if scale.mode == crate::AbsoluteMode::Grab {
                return Err("scale.absolute mode grab only reads the size; it is answered as a query and changes nothing".into());
            }
            Command::AbsoluteScale(scale)
        }
        "vertMap.new" => {
            // UVE-1 section 1: type uv (or txuv) adds a UV map to the active
            // mesh and selects it.
            if let Some(t) = opt_str(a, "type")?
                && (t.eq_ignore_ascii_case("uv") || t.eq_ignore_ascii_case("txuv"))
            {
                return Ok(Some(Command::UvMap(crate::uv_maps::UvMapEdit::New(
                    opt_str(a, "name")?.map(str::to_owned),
                ))));
            }
            if let Some(t) = opt_str(a, "type")?
                && !t.eq_ignore_ascii_case("weight")
            {
                return Err("Operandi makes weight and UV maps only".into());
            }
            let value = match a.get("value").filter(|v| !v.is_null()) {
                None => None,
                Some(v) => Some(
                    num(v)
                        .ok()
                        .filter(|v| v.is_finite())
                        .ok_or("value must be a number")?,
                ),
            };
            // A value fills every point; init alone fills with 0 (VM-03).
            let value = match (opt_bool(a, "init")?, value) {
                (Some(false), _) => None,
                (Some(true), v) => Some(v.unwrap_or(0.)),
                (None, v) => v,
            };
            Command::NewWeightMap {
                name: req_str(a, "name")?.into(),
                value,
            }
        }
        "select.vertexMap" => {
            if let Some(t) = opt_str(a, "type")?
                && !matches!(t.to_ascii_lowercase().as_str(), "weight" | "wght")
            {
                return Err("type is weight (or wght)".into());
            }
            let name = opt_str(a, "name")?.map(str::to_owned);
            Command::SelectWeightMap(match opt_str(a, "mode")?.unwrap_or("set") {
                m if m.eq_ignore_ascii_case("set") || m.eq_ignore_ascii_case("add") => name,
                m if m.eq_ignore_ascii_case("remove") => None,
                _ => return Err("mode is set, add or remove".into()),
            })
        }
        "operandi.nodeMaterial.import" => {
            let path = std::path::Path::new(req_str(a, "filename")?);
            let text = std::fs::read_to_string(path)
                .map_err(|err| format!("Couldn't read {}: {err}", path.display()))?;
            // Refused materials are left out (all or nothing per material).
            let (graphs, _refused) = crate::node_graph::import_report(&text, path.parent())?;
            // Clips list the graphs' pictures (NM-21); a missing or
            // unreadable one keeps its path and renders with the
            // missing-image look, as in the app.
            let images = graphs
                .iter()
                .flat_map(|g| {
                    g.material
                        .nodes
                        .iter()
                        .chain(g.groups.iter().flat_map(|x| &x.nodes))
                })
                .filter_map(|n| n.image.as_ref())
                .filter_map(|img| {
                    crate::node_graph::picture_size(std::path::Path::new(&img.path))
                        .ok()
                        .map(|(w, h)| (img.path.clone(), w, h))
                })
                .collect();
            Command::ShaderTree(crate::shader_tree::ShaderTreeEdit::ImportNodeGraphs {
                graphs,
                images,
                selected: opt_u64(a, "layer")?,
            })
        }
        "operandi.nodeMaterial.setDefault" => {
            let (graph, input, value) = node_default_args(e, a)?;
            if value == "?" {
                return Err(
                    "operandi.nodeMaterial.setDefault value ? is a query; it changes nothing"
                        .into(),
                );
            }
            let value = value
                .split(',')
                .map(|v| v.trim().parse::<f64>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|_| "value is a number or r,g,b")?;
            Command::ShaderTree(crate::shader_tree::ShaderTreeEdit::SetNodeGraphDefault {
                graph,
                input,
                value,
                continuing: false,
            })
        }
        "scene.saveAs" => {
            export_scene(e, a)?;
            return Ok(None);
        }
        "item.channel" => {
            let (target, channel, value) = channel_args(e, a)?;
            if value == "?" {
                return Err("item.channel ? is a query; it changes nothing".into());
            }
            return crate::channels::command_for(&e.scene, target, &channel, &value).map(Some);
        }
        _ => return Err(format!("Unknown command {name}")),
    };
    Ok(Some(cmd))
}

pub fn num(v: &Value) -> Result<f64> {
    v.as_f64()
        .ok_or_else(|| format!("Expected a number, got {v}"))
}
pub fn boolean(v: &Value) -> Result<bool> {
    v.as_bool()
        .ok_or_else(|| format!("Expected a boolean, got {v}"))
}
pub fn as_u32(v: &Value) -> Result<u32> {
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| format!("Expected an index, got {v}"))
}
pub fn req_str<'a>(m: &'a Map<String, Value>, k: &str) -> Result<&'a str> {
    m.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{k} is required"))
}
pub fn req_num(m: &Map<String, Value>, k: &str) -> Result<f64> {
    num(m.get(k).ok_or_else(|| format!("{k} is required"))?)
}
pub fn req_u32(m: &Map<String, Value>, k: &str) -> Result<u32> {
    as_u32(m.get(k).ok_or_else(|| format!("{k} is required"))?)
}
pub fn opt_u64(m: &Map<String, Value>, k: &str) -> Result<Option<u64>> {
    m.get(k)
        .filter(|v| !v.is_null())
        .map(|v| v.as_u64().ok_or_else(|| format!("{k} must be an integer")))
        .transpose()
}
pub fn opt_str<'a>(m: &'a Map<String, Value>, k: &str) -> Result<Option<&'a str>> {
    m.get(k)
        .filter(|v| !v.is_null())
        .map(|v| v.as_str().ok_or_else(|| format!("{k} must be a string")))
        .transpose()
}
pub fn opt_bool(m: &Map<String, Value>, k: &str) -> Result<Option<bool>> {
    m.get(k).filter(|v| !v.is_null()).map(boolean).transpose()
}
pub fn u32_list(m: &Map<String, Value>, k: &str) -> Result<Vec<u32>> {
    match m.get(k) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(list)) => list.iter().map(as_u32).collect(),
        Some(_) => Err(format!("{k} must be an array of indices")),
    }
}
/// `[x, y, z]` as a vector.
pub fn vec3(v: &Value) -> Result<DVec3> {
    match v.as_array().map(|a| a.as_slice()) {
        Some([x, y, z]) => Ok(DVec3::new(num(x)?, num(y)?, num(z)?)),
        _ => Err("Expected [x, y, z]".into()),
    }
}

/// `item.channel`'s target, channel name and value text.
/// transform.channel's (item, channel, value text); the map's mode, filter
/// and replace aren't supported yet.
fn transform_channel_args(e: &Editor, a: &Map<String, Value>) -> Result<(u64, String, String)> {
    for unsupported in ["mode", "filter", "replace"] {
        if a.get(unsupported).is_some_and(|m| !m.is_null()) {
            return Err(format!(
                "transform.channel {unsupported} isn't supported yet; leave it out"
            ));
        }
    }
    let id = match opt_u64(a, "item")? {
        Some(id) => id,
        None => e.scene.selection.item.ok_or("No active item")?,
    };
    let name = req_str(a, "name")?.to_owned();
    let value = match a.get("value") {
        None | Some(Value::Null) => return Err("value is required (\"?\" reads it)".into()),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(_) => return Err("value is a number (or \"?\")".into()),
    };
    Ok((id, name, value))
}

fn channel_args(
    e: &Editor,
    a: &Map<String, Value>,
) -> Result<(crate::channels::Target, String, String)> {
    use crate::channels::Target;
    if a.get("mode").is_some_and(|m| !m.is_null()) {
        return Err("item.channel mode isn't supported yet; leave it out".into());
    }
    let scene = match a.get("scene") {
        None | Some(Value::Null) => false,
        Some(v) => boolean(v)?,
    };
    let target = match (opt_u64(a, "layer")?, opt_u64(a, "item")?) {
        _ if scene && (a.contains_key("layer") || a.contains_key("item")) => {
            return Err("Give scene, item or layer, not more than one".into());
        }
        _ if scene => Target::Scene,
        (Some(layer), None) => Target::Layer(layer),
        (None, Some(item)) => Target::Item(item),
        (None, None) => Target::Item(e.scene.selection.item.ok_or("No active item")?),
        (Some(_), Some(_)) => return Err("Give item or layer, not both".into()),
    };
    let name = req_str(a, "name")?.to_owned();
    let value = match a.get("value") {
        None | Some(Value::Null) => return Err("value is required (\"?\" reads it)".into()),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Array(parts)) => parts
            .iter()
            .map(|p| match p {
                Value::Number(n) => Ok(n.to_string()),
                _ => Err("value lists are numbers (r, g, b)".to_string()),
            })
            .collect::<Result<Vec<_>>>()?
            .join(","),
        Some(_) => return Err("value is a number, true/false, text or a list".into()),
    };
    Ok((target, name, value))
}

/// poly.freeze's options: `base` (the last Freeze's) with the given ones.
/// The arguments for parts not built yet are accepted and ignored.
fn freeze_args(
    a: &Map<String, Value>,
    base: &crate::freeze::FreezeOptions,
) -> Result<crate::freeze::FreezeOptions> {
    let mut o = base.clone();
    match a.get("face").filter(|v| !v.is_null()) {
        None => {}
        Some(v) => {
            let text = match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => return Err("face is 0, 1, 2 or faces, polylines, twoPointPolygons".into()),
            };
            o.curves_as = crate::freeze::FreezeCurves::parse(&text).ok_or_else(|| {
                format!(
                    "face takes 0 (Faces), 1 (Polylines) or 2 (2 Point Polygons), not \"{text}\""
                )
            })?;
        }
    }
    if let Some(v) = opt_bool(a, "tess")? {
        o.tessellation = v;
    }
    if let Some(v) = opt_bool(a, "fixed")? {
        o.fixed_spans = v;
    }
    if let Some(v) = a.get("cangle").filter(|v| !v.is_null()) {
        let degrees = num(v)?.to_degrees();
        if !degrees.is_finite() || degrees <= 0. {
            return Err("cangle must be above 0".into());
        }
        o.refinement_degrees = degrees.min(crate::freeze::MAX_REFINEMENT_DEGREES);
    }
    Ok(o)
}

/// scale.absolute's mode and fields: the Absolute Scaling window's
/// defaults with the given values (the window's own fields are the app's;
/// a script passes what it needs, A3).
/// `poly.subdivide`'s JSON arguments over the dialog's Reset values, and
/// the repeat count.
fn subdivide_args(a: &Map<String, Value>) -> Result<(crate::SubdivideOptions, u32)> {
    use crate::polygon_subdivide::{SubdivideBoundary, SubdivideMethod};
    let mut o = crate::SubdivideOptions::default();
    if let Some(t) = opt_str(a, "type")? {
        o.method = SubdivideMethod::parse(t).ok_or(format!(
            "Unknown subdivision type \"{t}\"; use ccsds, flat, smooth or sds"
        ))?;
    }
    if let Some(v) = a.get("fractal") {
        o.fractal_percent = num(v)? * 100.;
    }
    if let Some(v) = a.get("maxangle") {
        o.max_angle_degrees = num(v)?.to_degrees();
    }
    if let Some(v) = opt_bool(a, "linear")? {
        o.linear_uvs = v;
    }
    if let Some(b) = opt_str(a, "bound")? {
        o.boundary = SubdivideBoundary::parse(b).ok_or(format!(
            "Unknown boundary rule \"{b}\"; use smooth, crease or creaseEdges"
        ))?;
    }
    if let Some(v) = opt_bool(a, "noEdgeWeight")? {
        o.ignore_edge_weights = v;
    }
    o.validate()?;
    let times = match opt_u64(a, "iterations")? {
        None => 1,
        Some(n @ 1..=8) => n as u32,
        Some(_) => return Err("iterations is 1 to 8".into()),
    };
    Ok((o, times))
}

/// `poly.unsubdiv`'s JSON arguments over the dialog's Reset values.
fn unsubdivide_args(a: &Map<String, Value>) -> Result<crate::UnsubdivideOptions> {
    let mut o = crate::UnsubdivideOptions::default();
    if let Some(n) = opt_u64(a, "iteration")? {
        let max = crate::polygon_subdivide::MAX_UNSUBDIVIDE_ITERATIONS;
        o.iterations = u32::try_from(n)
            .ok()
            .filter(|n| *n <= max)
            .ok_or(format!("Iteration must be a whole number from 0 to {max}"))?;
    }
    for (key, flag) in [
        ("flat", &mut o.flat),
        ("uv", &mut o.uv_hint),
        ("material", &mut o.material_hint),
        ("part", &mut o.part_hint),
        ("smoothingGroup", &mut o.smoothing_group_hint),
        ("selectionSet", &mut o.selection_set_hint),
    ] {
        if let Some(v) = opt_bool(a, key)? {
            *flag = v;
        }
    }
    if let Some(name) = opt_str(a, "selectionSetName")? {
        o.selection_set_name = name.to_owned();
    }
    Ok(o)
}

fn absolute_args(a: &Map<String, Value>) -> Result<crate::AbsoluteScale> {
    use crate::{AbsoluteFields, AbsoluteMode, ScaleCenter, UniformAxis};
    let pick = |k: &str, names: &[&str]| -> Result<Option<usize>> {
        match opt_str(a, k)? {
            None => Ok(None),
            Some(v) => names
                .iter()
                .position(|n| n.eq_ignore_ascii_case(v))
                .map(Some)
                .ok_or_else(|| format!("{k} is {}, not \"{v}\"", names.join(", "))),
        }
    };
    let mode =
        pick("mode", &["grab", "uniform", "explicit", "reference"])?.ok_or("mode is required")?;
    let mut f = AbsoluteFields::default();
    if let Some(v) = a.get("absolute_size_uniform").filter(|v| !v.is_null()) {
        f.uniform = num(v)?;
    }
    if let Some(i) = pick(
        "absolute_size_uniform_pref",
        &["longestAxis", "x", "y", "z"],
    )? {
        f.axis = UniformAxis::ALL[i];
    }
    if let Some(v) = opt_bool(a, "orient_to_selection")? {
        f.orient = v;
    }
    for (i, axis) in ["x", "y", "z"].into_iter().enumerate() {
        if let Some(v) = a
            .get(&format!("absolute_size_{axis}"))
            .filter(|v| !v.is_null())
        {
            f.size[i] = num(v)?;
        }
        let key = format!("absolute_size_center_{axis}");
        if let Some(v) = opt_str(a, &key)? {
            // The stored names carry the axis ("highX"); either works.
            let lower = v.to_ascii_lowercase();
            let word = lower
                .strip_suffix(axis)
                .filter(|s| !s.is_empty())
                .unwrap_or(&lower);
            let c = ["center", "high", "low", "origin"]
                .iter()
                .position(|n| *n == word)
                .ok_or_else(|| format!("{key} is center, high, low or origin, not \"{v}\""))?;
            f.center[i] = ScaleCenter::ALL[c];
        }
    }
    Ok(crate::AbsoluteScale {
        mode: AbsoluteMode::ALL[mode],
        fields: f,
    })
}

/// scene.saveAs without a window: File > Export's writer on the scene as
/// MeshOps evaluate it. Changes nothing in the editor (no history step);
/// the document's own file name and saved state are the caller's and stay.
/// A plain .opscene save, or no filename (the save panel), is refused.
fn export_scene(e: &Editor, a: &Map<String, Value>) -> Result<crate::interchange::ExportReport> {
    use crate::interchange;
    let path = opt_str(a, "filename")?
        .ok_or("scene.saveAs needs a filename here (there's no save panel without the window)")?;
    // This compatibility flag never changes headless export behavior.
    opt_bool(a, "export")?;
    let named = match opt_str(a, "format")? {
        None => None,
        Some(f) => Some(interchange::script_export_format(f).ok_or_else(|| {
            format!("format is obj, stl, stlAscii, gltf, glb, geo, svg, lxo, ply, plyAscii, abc, usd, usda, usdc, usdz, fbx or 3mf, not \"{f}\"")
        })?),
    };
    // Both given and disagreeing is an error, as in the app.
    let said = interchange::export_format_of_path(path);
    let format = match (named, said) {
        (Some(n), Some(s)) if n.extension() != s.extension() => {
            return Err(format!(
                "scene.saveAs: format doesn't match the file name's .{}",
                s.extension()
            ));
        }
        (Some(n), _) => n,
        (None, Some(s)) => s,
        (None, None) => {
            return Err("scene.saveAs here only exports: give an interchange format (format:, or a file extension such as .obj); to save the .opscene use scene_save".into());
        }
    };
    if format.is_usd() && !interchange::has_usd_exporter() {
        return Err("USD export needs the running app's exporter: start Operandi with Window > MCP Bridge on and connect with operandi-mcp --attach".into());
    }
    let path = std::path::Path::new(path);
    // TL-83: animation:on (default), off or current, for this export.
    let before = interchange::io_options();
    if let Some(word) = opt_str(a, "animation")? {
        let mode = interchange::UsdAnimation::parse(word)
            .ok_or_else(|| format!("animation is on, off or current, not \"{word}\""))?;
        interchange::set_io_options(interchange::IoOptions {
            usd_animation: mode,
            ..before.clone()
        });
    }
    let result = interchange::export(path, &e.evaluated(crate::evaluate::Purpose::Final), format)
        .map_err(|err| format!("Export {}: {err}", path.display()));
    interchange::set_io_options(before);
    result
}

/// operandi.nodeMaterial.setDefault's graph, exposed input and value text,
/// with the graph and input checked.
fn node_default_args(e: &Editor, a: &Map<String, Value>) -> Result<(u64, String, String)> {
    let graph = opt_u64(a, "graph")?.ok_or("graph is required")?;
    let input = req_str(a, "input")?.to_owned();
    let value = match a.get("value") {
        None | Some(Value::Null) => return Err("value is required (\"?\" reads it)".into()),
        Some(Value::String(s)) => s.trim().to_owned(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Array(parts)) => parts
            .iter()
            .map(|p| match p {
                Value::Number(n) => Ok(n.to_string()),
                _ => Err("value lists are numbers (r, g, b)".to_string()),
            })
            .collect::<Result<Vec<_>>>()?
            .join(","),
        Some(_) => return Err("value is a number, r,g,b or [r, g, b]".into()),
    };
    e.scene
        .shader_tree
        .node_graphs
        .get(graph)
        .ok_or(format!("No node graph {graph}"))?
        .exposed(&input)
        .ok_or(format!(
            "Graph {graph} has no exposed input \u{201c}{input}\u{201d}"
        ))?;
    Ok((graph, input, value))
}

/// A query's answer (`item.channel ... value:?`): `Some` when `given` with
/// these arguments only reads (UIC-163), else `None`.
pub fn answer(e: &Editor, given: &str, given_args: &Map<String, Value>) -> Option<Result<String>> {
    let (name, a) = resolve_command(given, given_args);
    // Grab Size reads the selection's size and changes nothing (A6).
    if name == "scale.absolute"
        && a.get("mode")
            .and_then(Value::as_str)
            .is_some_and(|m| m.eq_ignore_ascii_case("grab"))
    {
        return Some(check_args(&name, &a).and_then(|()| {
            let scale = absolute_args(&a)?;
            let s = e.absolute_fields_after(scale.fields)?.size;
            Ok(format!("Grab Size: {} × {} × {} m", s.x, s.y, s.z))
        }));
    }
    // Animation state and curves (anim_commands.rs), read any time.
    if let Some(answer) = crate::anim_commands::answer(e, &name, &a) {
        return Some(answer);
    }
    if let Some(answer) = crate::rig_schematic::answer(e, &name, &a) {
        return Some(answer);
    }
    // Rig pieces (rig_commands.rs), read any time.
    if let Some(answer) = crate::rig_commands::answer(e, &name, &a) {
        return Some(answer);
    }
    // The evaluation graph (eval_state.rs), read any time.
    if let Some(answer) = crate::eval_state::answer(e, &name, &a) {
        return Some(answer);
    }
    if a.get("value").and_then(Value::as_str).map(str::trim) != Some("?") {
        return None;
    }
    if name == "operandi.nodeMaterial.setDefault" {
        return Some(check_args(&name, &a).and_then(|()| {
            let (graph, input, _) = node_default_args(e, &a)?;
            let exposed = e
                .scene
                .shader_tree
                .node_graphs
                .get(graph)
                .and_then(|g| g.exposed(&input))
                .ok_or("No such input")?;
            Ok(exposed
                .default_value()
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(","))
        }));
    }
    if name == "transform.channel" {
        return Some(check_args(&name, &a).and_then(|()| {
            let (id, channel, _) = transform_channel_args(e, &a)?;
            let item = e.scene.item(id).ok_or(format!("No item {id}"))?;
            let v = crate::transform_channel::get(&item.transform, &channel)?;
            Ok(format!("{} (item {id}): {channel} = {v}", item.name))
        }));
    }
    if name != "item.channel" {
        return None;
    }
    Some(check_args(&name, &a).and_then(|()| {
        let (target, channel, _) = channel_args(e, &a)?;
        crate::channels::get(&e.scene, target, &channel)
    }))
}
