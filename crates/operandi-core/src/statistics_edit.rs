//! What the Statistics panel's +/− and row menu do (spec
//! BEHAVIOR-STATISTICS.md §6..8), as named commands (UIC-160): the panel,
//! the command line, scripts and the MCP bridge run the same lines with
//! the same undo steps. Operandi-only names under `operandi.statistics.*`
//! (UIC-165).
//!
//! - `operandi.statistics.select row:<row> [action:add|remove|only]`: the
//!   panel's plus adds the row's elements, its minus removes them, Select
//!   only replaces them (§6). Plus and Select only switch to the row's
//!   selection mode (Q2); minus in another mode edits that mode's stored
//!   selection without switching. One undo step each.
//! - `operandi.statistics.hide row: [action:hide|unhide]` (Q11).
//! - `operandi.statistics.rename row: name:`, `.delete row:`,
//!   `.paste row: from:` (sets: add members, Q8; maps: overwrite values),
//!   `.clear row:` (vertex maps: every value zero, map kept) — §7.
//! - `operandi.statistics.count row:` answers "n (m selected)" (UIC-163).
//!
//! Rows are named as `statistics::Row` prints them (`polygons.vertices:3`,
//! `edges.border:flat`, `polygons.part:Wheel`).
use crate::statistics::{
    EvalOptions, Kind, MapKind, MeshEval, Need, Row, compute, snapshot, target_meshes,
};
use crate::{Command, Editor, Mode, Result};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

/// How + / − / Select only change the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Add,
    Remove,
    Only,
}

fn text<'a>(a: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    a.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}
fn row_arg(a: &Map<String, Value>, key: &str) -> Result<Row> {
    text(a, key)
        .ok_or_else(|| format!("Missing {key}: (a statistics row, e.g. polygons.vertices:3)"))?
        .parse()
}
/// The handler of every `operandi.statistics.*` registry entry.
pub fn run_command(
    e: &mut Editor,
    name: &str,
    a: &Map<String, Value>,
) -> std::result::Result<Option<Command>, String> {
    let row = row_arg(a, "row")?;
    match name {
        "operandi.statistics.select" => {
            let action = match text(a, "action")
                .unwrap_or("add")
                .to_ascii_lowercase()
                .as_str()
            {
                "add" | "+" => Action::Add,
                "remove" | "-" | "−" => Action::Remove,
                "only" | "replace" => Action::Only,
                other => return Err(format!("action must be add, remove or only, not {other}")),
            };
            select(e, &row, action)?;
        }
        "operandi.statistics.hide" => {
            let hide = match text(a, "action")
                .unwrap_or("hide")
                .to_ascii_lowercase()
                .as_str()
            {
                "hide" => true,
                "unhide" | "show" => false,
                other => return Err(format!("action must be hide or unhide, not {other}")),
            };
            hide_row(e, &row, hide)?;
        }
        "operandi.statistics.rename" => {
            let to = text(a, "name").ok_or("Missing name: (the new name)")?;
            rename(e, &row, to)?;
        }
        "operandi.statistics.delete" => delete(e, &row)?,
        "operandi.statistics.clear" => clear_map(e, &row)?,
        "operandi.statistics.paste" => {
            let from = row_arg(a, "from")?;
            paste(e, &row, &from)?;
        }
        "operandi.statistics.count" => {
            let need = Need {
                vertices: row.kind() == Kind::Vertex,
                edges: row.kind() == Kind::Edge,
                polygons: row.kind() == Kind::Polygon,
                checks: row.expensive(),
            };
            let report = compute(&snapshot(&e.scene, need));
            let t = report.get(&row).unwrap_or_default();
            e.command_note = Some(format!("{} ({} selected)", t.count, t.selected));
        }
        other => return Err(format!("Unknown statistics command {other}")),
    }
    Ok(None)
}

/// What a row's elements are on one mesh.
struct Members {
    id: u64,
    vertices: Vec<u32>,
    edges: Vec<(u32, u32)>,
    faces: Vec<u32>,
}

fn need_for(row: &Row) -> Need {
    Need {
        vertices: row.kind() == Kind::Vertex,
        edges: row.kind() == Kind::Edge,
        polygons: row.kind() == Kind::Polygon,
        checks: row.expensive(),
    }
}

/// The row's elements on every counted mesh. `include_hidden` for Unhide.
fn members(e: &Editor, row: &Row, include_hidden: bool) -> Vec<Members> {
    let scene = &e.scene;
    target_meshes(scene)
        .into_iter()
        .filter_map(|id| scene.item(id))
        .map(|item| {
            let eval = MeshEval::new(
                &item.mesh,
                &EvalOptions {
                    hidden: &item.elements.hidden,
                    need: need_for(row),
                    include_hidden,
                },
            );
            let list = eval.members(row);
            let mut m = Members {
                id: item.id,
                vertices: Vec::new(),
                edges: Vec::new(),
                faces: Vec::new(),
            };
            match row.kind() {
                Kind::Vertex => m.vertices = list.into_iter().map(|v| v as u32).collect(),
                Kind::Edge => m.edges = list.into_iter().map(|i| eval.edge_points(i)).collect(),
                _ => m.faces = list.into_iter().map(|f| f as u32).collect(),
            }
            m
        })
        .collect()
}

/// Adds `add` to `list` (new ones at the end, in order), removes them, or
/// replaces `list` with them.
fn apply<T: Copy + Ord>(list: &mut Vec<T>, with: &[T], action: Action) {
    match action {
        Action::Add => {
            let have: BTreeSet<T> = list.iter().copied().collect();
            let mut seen = BTreeSet::new();
            list.extend(
                with.iter()
                    .copied()
                    .filter(|x| !have.contains(x) && seen.insert(*x)),
            );
        }
        Action::Remove => {
            let drop: BTreeSet<T> = with.iter().copied().collect();
            list.retain(|x| !drop.contains(x));
        }
        Action::Only => *list = with.to_vec(),
    }
}

fn label(row: &Row, action: Action) -> String {
    let verb = match action {
        Action::Add => "Add",
        Action::Remove => "Remove",
        Action::Only => "Select",
    };
    format!("{verb} {row} (Statistics)")
}

/// +, − and Select only (spec §6).
pub fn select(e: &mut Editor, row: &Row, action: Action) -> Result<()> {
    if e.selection_start.is_some() {
        return Err("Finish the selection first".into());
    }
    let mark = e.history_mark();
    let kind = row.kind();
    let mode = kind.mode();
    let before = e.scene.clone();
    // Q2: + (and Select only) switch to the row's mode.
    if action != Action::Remove && e.scene.selection.mode != mode {
        e.dispatch(Command::Mode(mode))?;
    }
    // The step starts before the mode switch, unless the switch recorded
    // one of its own (then the two are folded into one below).
    let before = if e.recorded_since(mark) {
        e.scene.clone()
    } else {
        before
    };
    if kind == Kind::Item {
        let ids = item_members(e, row);
        let mut now: Vec<u64> = e
            .scene
            .selection
            .item
            .into_iter()
            .chain(e.scene.selection.also.iter().copied())
            .collect();
        apply(&mut now, &ids, action);
        let changed = now
            != e.scene
                .selection
                .item
                .into_iter()
                .chain(e.scene.selection.also.iter().copied())
                .collect::<Vec<_>>();
        if changed {
            e.dispatch(Command::SelectItems(now))?;
        }
        if e.scene != before && !e.recorded_since(mark) {
            e.record_selection(label(row, action), before);
        }
        crate::command_exec::fold_steps(e, mark, &label(row, action));
        return Ok(());
    }
    let all = members(e, row, false);
    // With no active mesh in a component mode every shown mesh counts
    // (statistics::target_meshes); + makes the meshes it selects on active.
    let was_active = e.scene.active_meshes();
    let mut touched = Vec::new();
    for m in &all {
        let mut c = e.scene.selection.take_components(m.id);
        let had = c.clone();
        match kind {
            Kind::Vertex => apply(&mut c.vertices, &m.vertices, action),
            Kind::Edge => apply(&mut c.edges, &m.edges, action),
            _ => apply(&mut c.faces, &m.faces, action),
        }
        if c != had {
            touched.push(m.id);
        }
        e.scene.selection.put_components(m.id, c);
    }
    if was_active.is_empty() && action != Action::Remove && !touched.is_empty() {
        let ids: Vec<u64> = all.iter().map(|m| m.id).collect();
        let parked: Vec<_> = ids
            .iter()
            .map(|&id| (id, e.scene.selection.take_components(id)))
            .collect();
        e.scene.selection.item = ids.first().copied();
        e.scene.selection.also = ids.iter().skip(1).copied().collect();
        for (id, c) in parked {
            e.scene.selection.put_components(id, c);
        }
    }
    if e.scene != before {
        e.record_selection(label(row, action), before);
    }
    crate::command_exec::fold_steps(e, mark, &label(row, action));
    Ok(())
}

/// The items an Items row covers.
fn item_members(e: &Editor, row: &Row) -> Vec<u64> {
    let scene = &e.scene;
    match row {
        Row::All(Kind::Item) => scene.items.iter().map(|i| i.id).collect(),
        Row::ItemType(t) => scene
            .items
            .iter()
            .filter(|i| crate::statistics::ItemType::of(i) == *t)
            .map(|i| i.id)
            .collect(),
        Row::ItemGroup(g) => scene
            .groups
            .iter()
            .find(|x| x.id == *g)
            .map(|x| {
                x.members
                    .iter()
                    .copied()
                    .filter(|m| scene.item(*m).is_some())
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Hide and Unhide a row's elements (Q11). Items rows hide the items.
pub fn hide_row(e: &mut Editor, row: &Row, hide: bool) -> Result<()> {
    e.require_idle()?;
    let before = e.scene.clone();
    if row.kind() == Kind::Item {
        for id in item_members(e, row) {
            if let Some(i) = e.scene.item_mut(id) {
                i.visible = !hide;
                if hide {
                    i.hides_children = false;
                }
            }
        }
    } else {
        let all = members(e, row, !hide);
        for m in all {
            let mesh = e.scene.item(m.id).map(|i| i.mesh.shared().clone());
            let Some(item) = e.scene.item_mut(m.id) else {
                continue;
            };
            let mask = &mut item.elements.hidden;
            if hide {
                mask.vertices.extend(&m.vertices);
                mask.edges.extend(&m.edges);
                mask.faces.extend(&m.faces);
            } else {
                // Unhide also shows what the elements hang on: a polygon's
                // points and edges, an edge's points.
                let mut points: BTreeSet<u32> = m.vertices.iter().copied().collect();
                let mut edges: BTreeSet<(u32, u32)> = m.edges.iter().copied().collect();
                for &(a, b) in &m.edges {
                    points.extend([a, b]);
                }
                if let Some(mesh) = &mesh {
                    for &f in &m.faces {
                        if let Some(face) = mesh.faces.get(f as usize) {
                            points.extend(&face.vertices);
                        }
                        if (f as usize) < mesh.faces.len() {
                            for (a, b) in mesh.element_spans(f as usize) {
                                edges.insert((a.min(b), a.max(b)));
                            }
                        }
                    }
                }
                let faces: BTreeSet<u32> = m.faces.iter().copied().collect();
                mask.vertices.retain(|v| !points.contains(v));
                mask.edges.retain(|x| !edges.contains(x));
                mask.faces.retain(|f| !faces.contains(f));
            }
        }
        if hide {
            e.scene.drop_protected_from_selection();
        }
    }
    if e.scene == before {
        return Err(if hide {
            "Nothing to hide".into()
        } else {
            "Nothing hidden there".into()
        });
    }
    e.record(
        format!(
            "{} {row} (Statistics)",
            if hide { "Hide" } else { "Unhide" }
        ),
        before,
    );
    Ok(())
}

/// The mesh items a named row's rename or delete reaches: every mesh for
/// parts and materials (scene-wide names, with the Shader Tree's masks),
/// the counted meshes for per-mesh sets, maps and smoothing groups.
fn named_targets(e: &Editor, row: &Row) -> Vec<u64> {
    match row {
        Row::Part(_) | Row::Material(_) => e
            .scene
            .items
            .iter()
            .filter(|i| i.is_mesh())
            .map(|i| i.id)
            .collect(),
        _ => target_meshes(&e.scene),
    }
}

/// Renames a part, material, selection set, vertex map, smoothing group or
/// item group everywhere (spec §7). Refuses a name already in use.
pub fn rename(e: &mut Editor, row: &Row, to: &str) -> Result<()> {
    e.require_idle()?;
    let before = e.scene.clone();
    let to = to.to_owned();
    if let Row::ItemGroup(id) = row {
        if e.scene.groups.iter().any(|g| g.name == to && g.id != *id) {
            return Err(format!("A group named {to} already exists"));
        }
        let g = e
            .scene
            .groups
            .iter_mut()
            .find(|g| g.id == *id)
            .ok_or("Missing group")?;
        g.name = to;
    } else {
        let from = row
            .name()
            .ok_or("Only named rows can be renamed")?
            .to_owned();
        if from == to {
            return Ok(());
        }
        if matches!(row, Row::Material(m) if m == "Default") {
            return Err("The default material keeps its name".into());
        }
        for id in named_targets(e, row) {
            let Some(item) = e.scene.item_mut(id) else {
                continue;
            };
            rename_on_mesh(&mut item.mesh, row, &from, &to)?;
        }
        rename_in_shader_tree(&mut e.scene, row, &from, &to);
    }
    finish(e, before, format!("Rename {row} (Statistics)"))
}

fn rename_key<V>(
    map: &mut std::collections::BTreeMap<String, V>,
    from: &str,
    to: &str,
) -> Result<()> {
    if map.contains_key(from) {
        if map.contains_key(to) {
            return Err(format!("{to} already exists"));
        }
        let v = map.remove(from).expect("checked");
        map.insert(to.to_owned(), v);
    }
    Ok(())
}

fn rename_on_mesh(mesh: &mut crate::MeshSlot, row: &Row, from: &str, to: &str) -> Result<()> {
    match row {
        Row::Part(_) => {
            if !mesh.faces.iter().any(|f| f.part.as_deref() == Some(from)) {
                return Ok(());
            }
            if mesh.faces.iter().any(|f| f.part.as_deref() == Some(to)) {
                return Err(format!("A part named {to} already exists"));
            }
            for f in mesh.faces.iter_mut() {
                if f.part.as_deref() == Some(from) {
                    f.part = Some(to.to_owned());
                }
            }
        }
        Row::Material(_) => {
            let has = |m: &crate::Mesh, n: &str| {
                m.faces
                    .iter()
                    .any(|f| crate::statistics::material_name(&f.material) == n)
            };
            if !has(mesh, from) {
                return Ok(());
            }
            if has(mesh, to) {
                return Err(format!("A material named {to} already exists"));
            }
            for f in mesh.faces.iter_mut() {
                if crate::statistics::material_name(&f.material) == from {
                    f.material = to.to_owned();
                }
            }
        }
        Row::Smoothing(_) => {
            let labels = &mesh.attributes.smoothing_labels;
            if !labels.iter().any(|l| l.as_deref() == Some(from)) {
                return Ok(());
            }
            if labels.iter().any(|l| l.as_deref() == Some(to)) {
                return Err(format!("A smoothing group named {to} already exists"));
            }
            for l in mesh.attributes.smoothing_labels.iter_mut() {
                if l.as_deref() == Some(from) {
                    *l = Some(to.to_owned());
                }
            }
        }
        Row::VertexSet(_) => rename_key(&mut mesh.attributes.vertex_sets, from, to)?,
        Row::EdgeSet(_) => rename_key(&mut mesh.attributes.edge_sets, from, to)?,
        Row::PolygonSet(_) => rename_key(&mut mesh.attributes.polygon_sets, from, to)?,
        Row::VertexMap(kind, _) => {
            if !crate::statistics::map_names(mesh)
                .iter()
                .any(|(k, n)| k == kind && n == from)
            {
                return Ok(());
            }
            let m: &mut crate::Mesh = mesh;
            let a = &mut *m.attributes;
            match kind {
                MapKind::Weight => {
                    rename_key(&mut a.point_weights, from, to)?;
                    rename_key(&mut a.corner_weights, from, to)?;
                }
                MapKind::Color => {
                    rename_key(&mut a.point_colors, from, to)?;
                    rename_key(&mut a.corner_colors, from, to)?;
                }
                MapKind::Uv => {
                    rename_key(&mut m.uv_maps, from, to)?;
                    let a = &mut *m.attributes;
                    rename_key(&mut a.point_uvs, from, to)?;
                    rename_key(&mut a.uv_records, from, to)?;
                    if a.current_uv_map.as_deref() == Some(from) {
                        a.current_uv_map = Some(to.to_owned());
                    }
                }
                MapKind::Morph => rename_key(&mut a.relative_morphs, from, to)?,
                MapKind::Normal => {
                    rename_key(&mut a.corner_normals, from, to)?;
                    for n in a.normal_map_order.iter_mut() {
                        if n == from {
                            *n = to.to_owned();
                        }
                    }
                }
                MapKind::Other => {}
            }
            let a = &mut *m.attributes;
            if a.typed_maps
                .get(from)
                .is_some_and(|t| MapKind::of_typed(&t.kind) == *kind)
            {
                rename_key(&mut a.typed_maps, from, to)?;
            }
        }
        _ => return Err("Only named rows can be renamed".into()),
    }
    Ok(())
}

/// Parts and materials: the Shader Tree's Material Group masks follow the
/// new name, and so does the material's entry.
fn rename_in_shader_tree(scene: &mut crate::Scene, row: &Row, from: &str, to: &str) {
    use crate::shader_tree::{NodeKind, TagMask, TagType};
    let tag_type = match row {
        Row::Part(_) => TagType::Part,
        Row::Material(_) => TagType::Material,
        Row::PolygonSet(_) => TagType::SelectionSet,
        _ => return,
    };
    for node in &mut scene.shader_tree.nodes {
        if let NodeKind::Group(g) = &mut node.kind
            && g.tag_type == tag_type
            && g.tag == TagMask::Name(from.to_owned())
        {
            g.tag = TagMask::Name(to.to_owned());
        }
    }
    if tag_type == TagType::Material
        && !scene.materials.contains_key(to)
        && let Some(m) = scene.materials.remove(from)
    {
        scene.materials.insert(to.to_owned(), m);
    }
}

/// Validates and records an edit, or puts the scene back.
fn finish(e: &mut Editor, before: crate::Scene, label: String) -> Result<()> {
    if e.scene == before {
        return Err("Nothing to change".into());
    }
    if let Err(err) = e.scene.validate() {
        e.scene = before;
        return Err(err);
    }
    e.record(label, before);
    Ok(())
}

/// Deletes a tag, set, map or group from every element; the elements stay
/// (spec §7).
pub fn delete(e: &mut Editor, row: &Row) -> Result<()> {
    e.require_idle()?;
    let before = e.scene.clone();
    if let Row::ItemGroup(id) = row {
        e.scene.groups.retain(|g| g.id != *id);
        return finish(e, before, format!("Delete {row} (Statistics)"));
    }
    let name = row
        .name()
        .ok_or("Only named rows can be deleted")?
        .to_owned();
    for id in named_targets(e, row) {
        let Some(item) = e.scene.item_mut(id) else {
            continue;
        };
        let mesh = &mut item.mesh;
        match row {
            Row::Part(_) => {
                if mesh.faces.iter().any(|f| f.part.as_deref() == Some(&name)) {
                    for f in mesh.faces.iter_mut() {
                        if f.part.as_deref() == Some(&name) {
                            f.part = None;
                        }
                    }
                }
            }
            Row::Material(_) => {
                if name == "Default" {
                    return Err("The default material can't be removed".into());
                }
                if mesh.faces.iter().any(|f| f.material == name) {
                    for f in mesh.faces.iter_mut() {
                        if f.material == name {
                            f.material = "Default".into();
                        }
                    }
                }
            }
            Row::Smoothing(_) => {
                if mesh
                    .attributes
                    .smoothing_labels
                    .iter()
                    .any(|l| l.as_deref() == Some(&name))
                {
                    for l in mesh.attributes.smoothing_labels.iter_mut() {
                        if l.as_deref() == Some(&name) {
                            *l = None;
                        }
                    }
                }
            }
            Row::VertexSet(_) if mesh.attributes.vertex_sets.contains_key(&name) => {
                mesh.attributes.vertex_sets.remove(&name);
            }
            Row::EdgeSet(_) if mesh.attributes.edge_sets.contains_key(&name) => {
                mesh.attributes.edge_sets.remove(&name);
            }
            Row::PolygonSet(_) if mesh.attributes.polygon_sets.contains_key(&name) => {
                mesh.attributes.polygon_sets.remove(&name);
            }
            Row::VertexMap(kind, _) => {
                if !crate::statistics::map_names(mesh)
                    .iter()
                    .any(|(k, n)| k == kind && *n == name)
                {
                    continue;
                }
                let m: &mut crate::Mesh = mesh;
                match kind {
                    MapKind::Weight => {
                        m.attributes.point_weights.remove(&name);
                        m.attributes.corner_weights.remove(&name);
                    }
                    MapKind::Color => {
                        m.attributes.point_colors.remove(&name);
                        m.attributes.corner_colors.remove(&name);
                    }
                    MapKind::Uv => {
                        m.uv_maps.remove(&name);
                        let a = &mut *m.attributes;
                        a.point_uvs.remove(&name);
                        a.uv_records.remove(&name);
                        if a.current_uv_map.as_deref() == Some(&name) {
                            a.current_uv_map = None;
                        }
                    }
                    MapKind::Morph => {
                        m.attributes.relative_morphs.remove(&name);
                    }
                    MapKind::Normal => {
                        let a = &mut *m.attributes;
                        a.corner_normals.remove(&name);
                        a.normal_map_order.retain(|n| *n != name);
                    }
                    MapKind::Other => {}
                }
                if m.attributes
                    .typed_maps
                    .get(&name)
                    .is_some_and(|t| MapKind::of_typed(&t.kind) == *kind)
                {
                    m.attributes.typed_maps.remove(&name);
                }
            }
            _ => {}
        }
    }
    finish(e, before, format!("Delete {row} (Statistics)"))
}

/// Clear (vertex maps): every value becomes zero, the map stays.
pub fn clear_map(e: &mut Editor, row: &Row) -> Result<()> {
    e.require_idle()?;
    let Row::VertexMap(kind, name) = row else {
        return Err("Clear works on vertex maps".into());
    };
    let before = e.scene.clone();
    for id in target_meshes(&e.scene) {
        let Some(item) = e.scene.item_mut(id) else {
            continue;
        };
        let m: &mut crate::Mesh = &mut item.mesh;
        match kind {
            MapKind::Weight => {
                if let Some(w) = m.attributes.point_weights.get_mut(name) {
                    w.values_mut().for_each(|v| *v = 0.);
                }
                if let Some(w) = m.attributes.corner_weights.get_mut(name) {
                    w.iter_mut().flatten().flatten().for_each(|v| *v = 0.);
                }
            }
            MapKind::Color => {
                if let Some(c) = m.attributes.point_colors.get_mut(name) {
                    c.values_mut().for_each(|v| *v = glam::DVec4::ZERO);
                }
                if let Some(c) = m.attributes.corner_colors.get_mut(name) {
                    c.iter_mut()
                        .flatten()
                        .flatten()
                        .for_each(|v| *v = glam::DVec4::ZERO);
                }
            }
            MapKind::Uv => {
                if let Some(uv) = m.uv_maps.get_mut(name) {
                    uv.iter_mut()
                        .flatten()
                        .flatten()
                        .for_each(|v| *v = glam::DVec2::ZERO);
                }
                if let Some(uv) = m.attributes.point_uvs.get_mut(name) {
                    uv.values_mut().for_each(|v| *v = glam::DVec2::ZERO);
                }
            }
            MapKind::Morph => {
                if let Some(d) = m.attributes.relative_morphs.get_mut(name) {
                    d.values_mut().for_each(|v| *v = glam::DVec3::ZERO);
                }
            }
            MapKind::Normal | MapKind::Other => {
                return Err("Clear works on weight, UV, morph and colour maps".into());
            }
        }
        if let Some(t) = m.attributes.typed_maps.get_mut(name)
            && MapKind::of_typed(&t.kind) == *kind
        {
            t.points.values_mut().flatten().for_each(|v| *v = 0.);
            t.corners
                .iter_mut()
                .flatten()
                .flatten()
                .flatten()
                .for_each(|v| *v = 0.);
        }
    }
    finish(e, before, format!("Clear {row} (Statistics)"))
}

/// Paste: a copied selection set's members join this set (Q8); a copied
/// vertex map's values overwrite this map's (spec §7).
pub fn paste(e: &mut Editor, row: &Row, from: &Row) -> Result<()> {
    e.require_idle()?;
    let before = e.scene.clone();
    let same_kind = matches!(
        (row, from),
        (Row::VertexSet(_), Row::VertexSet(_))
            | (Row::EdgeSet(_), Row::EdgeSet(_))
            | (Row::PolygonSet(_), Row::PolygonSet(_))
    ) || matches!((row, from), (Row::VertexMap(a, _), Row::VertexMap(b, _)) if a == b);
    if !same_kind {
        return Err(format!("{from} can't be pasted onto {row}"));
    }
    for id in target_meshes(&e.scene) {
        let Some(item) = e.scene.item_mut(id) else {
            continue;
        };
        let m: &mut crate::Mesh = &mut item.mesh;
        match (row, from) {
            (Row::VertexSet(to), Row::VertexSet(src)) => {
                if let Some(s) = m.attributes.vertex_sets.get(src).cloned()
                    && let Some(t) = m.attributes.vertex_sets.get_mut(to)
                {
                    t.extend(s);
                }
            }
            (Row::EdgeSet(to), Row::EdgeSet(src)) => {
                if let Some(s) = m.attributes.edge_sets.get(src).cloned()
                    && let Some(t) = m.attributes.edge_sets.get_mut(to)
                {
                    t.extend(s);
                }
            }
            (Row::PolygonSet(to), Row::PolygonSet(src)) => {
                if let Some(s) = m.attributes.polygon_sets.get(src).cloned()
                    && let Some(t) = m.attributes.polygon_sets.get_mut(to)
                {
                    t.extend(s);
                }
            }
            (Row::VertexMap(kind, to), Row::VertexMap(_, src)) => {
                paste_map(m, *kind, src, to)?;
            }
            _ => {}
        }
    }
    finish(e, before, format!("Paste {from} onto {row} (Statistics)"))
}

fn paste_map(m: &mut crate::Mesh, kind: MapKind, src: &str, to: &str) -> Result<()> {
    fn points<V: Clone>(
        map: &mut std::collections::BTreeMap<String, std::collections::BTreeMap<u32, V>>,
        src: &str,
        to: &str,
    ) {
        if let Some(s) = map.get(src).cloned()
            && let Some(t) = map.get_mut(to)
        {
            t.extend(s);
        }
    }
    fn corners<V: Clone>(
        map: &mut std::collections::BTreeMap<String, Vec<Vec<Option<V>>>>,
        src: &str,
        to: &str,
    ) {
        if let Some(s) = map.get(src).cloned()
            && let Some(t) = map.get_mut(to)
        {
            for (tf, sf) in t.iter_mut().zip(&s) {
                for (tc, sc) in tf.iter_mut().zip(sf) {
                    if sc.is_some() {
                        *tc = sc.clone();
                    }
                }
            }
        }
    }
    let a = &mut *m.attributes;
    match kind {
        MapKind::Weight => {
            points(&mut a.point_weights, src, to);
            corners(&mut a.corner_weights, src, to);
        }
        MapKind::Color => {
            points(&mut a.point_colors, src, to);
            corners(&mut a.corner_colors, src, to);
        }
        MapKind::Morph => points(&mut a.relative_morphs, src, to),
        MapKind::Uv => {
            points(&mut a.point_uvs, src, to);
            corners(&mut m.uv_maps, src, to);
        }
        MapKind::Normal => corners(&mut a.corner_normals, src, to),
        MapKind::Other => {
            return Err("Paste works on weight, UV, morph, colour and normal maps".into());
        }
    }
    Ok(())
}

/// The mode a row's + would switch to (Q2), for the panel's tooltips.
pub fn mode_of(row: &Row) -> Mode {
    row.kind().mode()
}
