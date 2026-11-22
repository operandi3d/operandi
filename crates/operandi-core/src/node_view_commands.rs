//! The node view's commands (node editing, UIC-160): every gesture in a
//! Material Group's node view is one of these lines, so scripts, macros
//! and the bridge do the same with the same undo step. Names are
//! placeholders until the registry thread names them (docs/NAMES-NEEDED.md).
//!
//! Wire targets are written `<free node>.<input>` or
//! `layer:<id>.<channel>` (channel keys as `node_view::CHANNELS`).
use crate::node_view::{NodeEdit, Target};
use crate::shader_tree::ShaderTreeEdit;
use crate::{Command, Editor, Result};
use serde_json::{Map, Value};

/// The commands, for the registry's tests and Help.
pub const NAMES: [&str; 9] = [
    "operandi.nodes.add",
    "operandi.nodes.connect",
    "operandi.nodes.delete",
    "operandi.nodes.disconnect",
    "operandi.nodes.hide",
    "operandi.nodes.move",
    "operandi.nodes.setValue",
    "operandi.nodes.show",
    "operandi.nodes.showAll",
];

fn text<'a>(a: &'a Map<String, Value>, k: &str) -> Result<&'a str> {
    a.get(k)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{k} is required"))
}
fn number(a: &Map<String, Value>, k: &str) -> Result<f64> {
    match a.get(k) {
        Some(Value::Number(n)) => n.as_f64().ok_or_else(|| format!("{k} is a number")),
        Some(Value::String(s)) => s.trim().parse().map_err(|_| format!("{k} is a number")),
        _ => Err(format!("{k} is required")),
    }
}
fn names(a: &Map<String, Value>) -> Result<Vec<String>> {
    Ok(text(a, "node")?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// A wire target as a command writes it.
pub fn target_text(t: &Target) -> String {
    match t {
        Target::Free { node, input } => format!("{node}.{input}"),
        Target::Layer { layer, channel } => format!("layer:{layer}.{channel}"),
    }
}

/// [`target_text`] read back.
pub fn parse_target(s: &str) -> Result<Target> {
    let (node, input) = s
        .rsplit_once('.')
        .ok_or_else(|| format!("'{s}' is <node>.<input> or layer:<id>.<channel>"))?;
    if let Some(id) = node.strip_prefix("layer:") {
        return Ok(Target::Layer {
            layer: id.parse().map_err(|_| format!("'{id}' isn't a layer id"))?,
            channel: input.to_string(),
        });
    }
    Ok(Target::Free {
        node: node.to_string(),
        input: input.to_string(),
    })
}

/// The command `name` builds, `None` when it isn't one of these.
pub fn script_command(e: &Editor, name: &str, a: &Map<String, Value>) -> Result<Option<Command>> {
    if !name.starts_with("operandi.nodes.") {
        return Ok(None);
    }
    let _ = e;
    let group = match a.get("group") {
        Some(Value::Number(n)) => n.as_u64(),
        Some(Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    }
    .ok_or("group is required (a Material Group's layer id)")?;
    let edit = match name {
        "operandi.nodes.add" => {
            let category = text(a, "node")?.to_string();
            let ty = a
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("color3")
                .to_string();
            let at = [number(a, "x").unwrap_or(0.), number(a, "y").unwrap_or(0.)];
            NodeEdit::Add { category, ty, at }
        }
        "operandi.nodes.connect" => NodeEdit::Connect {
            from: text(a, "from")?.to_string(),
            output: a
                .get("output")
                .and_then(Value::as_str)
                .unwrap_or("out")
                .to_string(),
            to: parse_target(text(a, "to")?)?,
        },
        "operandi.nodes.delete" => NodeEdit::Delete(names(a)?),
        "operandi.nodes.disconnect" => NodeEdit::Disconnect(parse_target(text(a, "to")?)?),
        "operandi.nodes.hide" => NodeEdit::Hide(names(a)?),
        "operandi.nodes.show" => NodeEdit::Show(names(a)?),
        "operandi.nodes.showAll" => NodeEdit::ShowAll,
        "operandi.nodes.move" => NodeEdit::Move {
            places: vec![(
                text(a, "node")?.to_string(),
                [number(a, "x")?, number(a, "y")?],
            )],
            continuing: false,
        },
        "operandi.nodes.setValue" => NodeEdit::SetValue {
            node: text(a, "node")?.to_string(),
            input: text(a, "input")?.to_string(),
            value: match a.get("value") {
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::String(s)) => s.clone(),
                _ => return Err("value is required".into()),
            },
            continuing: false,
        },
        other => return Err(format!("Unknown command {other}")),
    };
    Ok(Some(Command::ShaderTree(ShaderTreeEdit::Nodes {
        group,
        edit,
    })))
}

#[cfg(test)]
mod tests {
    use crate::node_view::tests::{MATERIAL, editor};

    fn run(e: &mut crate::Editor, line: &str) -> crate::Result<()> {
        match crate::command_line::run_line(e, line) {
            Some(r) => r.map(|_| ()),
            None => Err(format!("not a headless line: {line}")),
        }
    }

    #[test]
    fn every_gesture_is_a_command_line() {
        let (mut e, group) = editor();
        run(
            &mut e,
            &format!("operandi.nodes.add group:{group} node:noise3d type:color3 x:40 y:-20"),
        )
        .unwrap();
        run(
            &mut e,
            &format!("operandi.nodes.setValue group:{group} node:Noise input:amplitude value:0.5"),
        )
        .unwrap();
        run(
            &mut e,
            &format!(
                "operandi.nodes.connect group:{group} from:Noise to:layer:{MATERIAL}.diffuse_color"
            ),
        )
        .unwrap();
        assert_eq!(
            e.undo.last().unwrap().label,
            "Connect Noise > Diffuse Color"
        );
        run(
            &mut e,
            &format!("operandi.nodes.move group:{group} node:Noise x:5 y:6"),
        )
        .unwrap();
        run(
            &mut e,
            &format!("operandi.nodes.hide group:{group} node:Noise"),
        )
        .unwrap();
        run(&mut e, &format!("operandi.nodes.showAll group:{group}")).unwrap();
        run(
            &mut e,
            &format!("operandi.nodes.disconnect group:{group} to:layer:{MATERIAL}.diffuse_color"),
        )
        .unwrap();
        run(
            &mut e,
            &format!("operandi.nodes.delete group:{group} node:Noise"),
        )
        .unwrap();
        assert_eq!(e.scene.shader_tree.group_nodes(group), None);
        // Refusals come back as errors and change nothing.
        let before = e.scene.clone();
        assert!(
            run(
                &mut e,
                &format!("operandi.nodes.delete group:{group} node:mix:{MATERIAL}")
            )
            .is_err()
        );
        assert!(run(&mut e, "operandi.nodes.showAll").is_err());
        assert_eq!(e.scene, before);
    }
}
