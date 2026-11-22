//! Render > Add Per-Light Render Outputs (render outputs group D,
//! 2026-11-20, RO-Q27): one Light Path Expressions output per renderable
//! light, expression `C.*L'<label>'`, named "LPE: <expression>". A light
//! with an empty LPE Label gets its item name as its label. Lights whose
//! output already exists are skipped. One undo step for the whole command.
use crate::camera_light::LightEdit;
use crate::light_path_expressions::per_light_expression;
use crate::shader_tree::{NewLayer, NodeKind, OutputEffect, ShaderTreeEdit};
use crate::{Command, Editor, Result};
use serde_json::{Map, Value};

/// The command's name (an Operandi command: not in the reference map).
pub const NAME: &str = "operandi.render.addPerLightOutputs";
/// Its undo label and menu entry.
pub const LABEL: &str = "Add Per-Light Render Outputs";

/// The registry's handler: runs it, nothing left to dispatch.
pub(crate) fn run(
    e: &mut Editor,
    _name: &str,
    _args: &Map<String, Value>,
) -> std::result::Result<Option<Command>, String> {
    add_per_light_outputs(e)?;
    Ok(None)
}

/// Adds the outputs; returns how many it added.
pub fn add_per_light_outputs(e: &mut Editor) -> Result<usize> {
    let scene = &e.scene;
    // (item, label to use, whether the light needs that label set).
    let lights: Vec<(u64, String, bool)> = scene
        .items
        .iter()
        .filter_map(|item| {
            let l = item.light.as_ref()?;
            if !l.renders(scene.shows(item)) || !l.kind.has_visible_rows() {
                return None;
            }
            Some(if l.lpe_label.is_empty() {
                (item.id, item.name.trim().to_owned(), true)
            } else {
                (item.id, l.lpe_label.clone(), false)
            })
        })
        .collect();
    if lights.is_empty() {
        return Err("The scene has no light that renders".into());
    }
    let tree = scene.shading_tree();
    let mut have: Vec<String> = tree
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                n.kind,
                NodeKind::RenderOutput(OutputEffect::LightPathExpressions)
            )
        })
        .map(|n| tree.output_settings(n.id).lpe)
        .collect();
    let start = e.history_mark();
    let mut added = 0;
    for (item, label, set_label) in lights {
        // A label holding a quote can't be written in an expression.
        if label.is_empty() || label.contains('\'') {
            continue;
        }
        let expression = per_light_expression(&label);
        if have.contains(&expression) {
            continue;
        }
        if set_label {
            e.dispatch(Command::EditLight {
                item,
                edit: LightEdit::LpeLabel(label.clone()),
            })?;
        }
        let before: Vec<u64> = output_ids(e);
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
            layer: NewLayer::RenderOutput(OutputEffect::LightPathExpressions),
            selected: None,
        }))?;
        let id = output_ids(e)
            .into_iter()
            .find(|i| !before.contains(i))
            .ok_or("The new output wasn't added")?;
        let mut node = e
            .scene
            .shader_tree
            .get(id)
            .cloned()
            .ok_or("The new output wasn't added")?;
        node.name = format!("LPE: {expression}");
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
            node,
            continuing: false,
        }))?;
        let mut settings = e.scene.shader_tree.output_settings(id);
        settings.lpe = expression.clone();
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetOutput {
            node: id,
            settings: Box::new(settings),
            continuing: false,
        }))?;
        have.push(expression);
        added += 1;
    }
    if added == 0 {
        return Err("Every light already has its Light Path Expressions output".into());
    }
    crate::command_exec::fold_steps(e, start, LABEL);
    Ok(added)
}

fn output_ids(e: &Editor) -> Vec<u64> {
    e.scene
        .shader_tree
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::RenderOutput(_)))
        .map(|n| n.id)
        .collect()
}
