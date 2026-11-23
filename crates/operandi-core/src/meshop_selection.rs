//! Selection operations (MO-50..54, PG-xx): an op's Selection input,
//! re-answered on every evaluation. A stack of them sits on each op after
//! its Select by Index (MO-20): each picks polygons of the mesh below and
//! combines with what's under it by its Blend Mode (MO-51).
//!
//! Select by Previous Operation (PG-xx) reads named groups an op lower in
//! the stack produced, such as a primitive's `caps` or an Array's `clone2`.
//! A producing op writes each group as a reserved polygon set on its
//! result ([`OpContext::tag`](crate::meshop_types::OpContext::tag)), so the
//! group follows the polygons through every op above that carries
//! selection sets along. The runtime strips reserved sets from what it
//! hands out (viewport, render, Freeze, links).
//!
//! Provisional (MOQ-21..24): selection ops pick polygons only; Range and
//! Pattern channel names and Select by Selection Set's type id are
//! Operandi's own.
use crate::{Mesh, Result};
use serde::{Deserialize, Serialize};

/// Reserved polygon set names start with this (not typeable).
const GROUP_PREFIX: &str = "\u{1}meshop:";

/// The reserved set that holds op `op`'s group `group` (PG-03 short id).
pub fn group_set(op: u64, group: &str) -> String {
    format!("{GROUP_PREFIX}{op}:{group}")
}

/// `mesh` without reserved group sets; borrowed when it has none.
pub fn strip_groups(mesh: &crate::Shared<Mesh>) -> crate::Shared<Mesh> {
    if !mesh
        .attributes
        .polygon_sets
        .keys()
        .any(|k| k.starts_with(GROUP_PREFIX))
    {
        return mesh.clone();
    }
    let mut m = mesh.to_owned_value();
    m.attributes
        .polygon_sets
        .retain(|k, _| !k.starts_with(GROUP_PREFIX));
    let out = crate::Shared::new(m);
    out.record_checked();
    out
}

/// `mesh` without its group sets, and those sets. Kernels never see the
/// sets (some refuse any selection set); [`carry_groups`] puts them back.
pub fn split_groups(mesh: &Mesh) -> (std::borrow::Cow<'_, Mesh>, Vec<(String, Vec<u32>)>) {
    if !mesh
        .attributes
        .polygon_sets
        .keys()
        .any(|k| k.starts_with(GROUP_PREFIX))
    {
        return (std::borrow::Cow::Borrowed(mesh), Vec::new());
    }
    let mut clean = mesh.clone();
    let mut groups = Vec::new();
    clean.attributes.polygon_sets.retain(|k, v| {
        if k.starts_with(GROUP_PREFIX) {
            groups.push((k.clone(), v.iter().copied().collect()));
            false
        } else {
            true
        }
    });
    (std::borrow::Cow::Owned(clean), groups)
}

/// Puts `groups` of `input` onto `output`: a polygon of the output that
/// has exactly the corners of a grouped input polygon (same point numbers
/// in the same order) is in its groups. Kernels keep untouched polygons
/// that way, so a group follows the polygons an op leaves alone; a
/// polygon an op rebuilt leaves its groups (provisional, MOQ-22).
pub fn carry_groups(groups: &[(String, Vec<u32>)], input: &Mesh, output: &mut Mesh) {
    if groups.is_empty() {
        return;
    }
    let mut at: std::collections::HashMap<&[u32], u32> = std::collections::HashMap::new();
    for (f, face) in input.faces.iter().enumerate() {
        at.entry(face.vertices.as_slice()).or_insert(f as u32);
    }
    let mut map: Vec<Option<u32>> = vec![None; input.faces.len()];
    for (f, face) in output.faces.iter().enumerate() {
        if let Some(&old) = at.get(face.vertices.as_slice())
            && map[old as usize].is_none()
        {
            map[old as usize] = Some(f as u32);
        }
    }
    for (name, faces) in groups {
        if output.attributes.polygon_sets.contains_key(name) {
            continue;
        }
        let set: std::collections::BTreeSet<u32> = faces
            .iter()
            .filter_map(|&f| map.get(f as usize).copied().flatten())
            .collect();
        output.attributes.polygon_sets.insert(name.clone(), set);
    }
}

/// MO-51 Blend Mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Blend {
    /// Replaces what's below it.
    #[default]
    Override,
    Add,
    Subtract,
    /// Keeps only what both select.
    Multiply,
}
impl Blend {
    pub const ALL: [Blend; 4] = [Self::Override, Self::Add, Self::Subtract, Self::Multiply];
    pub fn label(self) -> &'static str {
        match self {
            Self::Override => "Override",
            Self::Add => "Add",
            Self::Subtract => "Subtract",
            Self::Multiply => "Multiply",
        }
    }
}

/// What a selection op picks (MO-53).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SelKind {
    /// Stored polygon numbers, `0-9,12` form.
    #[serde(rename = "index.selop.item")]
    Index { index: String },
    /// Every `step`th polygon from `start` to `end` (inclusive; 0 = the
    /// last polygon).
    #[serde(rename = "selbyrange.selop.item")]
    Range { start: u32, end: u32, step: u32 },
    /// A repeating on/off list: `1,0` = every other polygon. Accepts
    /// 1/0, True/False, Yes/No.
    #[serde(rename = "pattern.selop.item")]
    Pattern { pattern: String },
    /// A group an op lower in the stack produced (PG-01..04): `source` is
    /// that op's id, `name` its short group id (case-sensitive).
    #[serde(rename = "previous.selop")]
    Previous { source: u64, name: String },
    /// The polygons of a selection set on the mesh below.
    #[serde(rename = "operandi.selset.selop")]
    SelectionSet { name: String },
}
impl SelKind {
    /// The kinds Add Selection offers, with their labels.
    pub fn menu() -> [(&'static str, SelKind); 5] {
        [
            (
                "Select by Index",
                SelKind::Index {
                    index: String::new(),
                },
            ),
            (
                "Select by Range",
                SelKind::Range {
                    start: 0,
                    end: 0,
                    step: 1,
                },
            ),
            (
                "Select by Pattern",
                SelKind::Pattern {
                    pattern: "1,0".into(),
                },
            ),
            (
                "Select by Previous Operation",
                SelKind::Previous {
                    source: 0,
                    name: String::new(),
                },
            ),
            (
                "Select by Selection Set",
                SelKind::SelectionSet {
                    name: String::new(),
                },
            ),
        ]
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Index { .. } => "Select by Index",
            Self::Range { .. } => "Select by Range",
            Self::Pattern { .. } => "Select by Pattern",
            Self::Previous { .. } => "Select by Previous Operation",
            Self::SelectionSet { .. } => "Select by Selection Set",
        }
    }
}

/// One selection op (MO-51 common settings + its kind).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelOp {
    pub enable: bool,
    pub invert: bool,
    #[serde(rename = "blendmode")]
    pub blend: Blend,
    #[serde(flatten)]
    pub kind: SelKind,
}
impl SelOp {
    pub fn new(kind: SelKind) -> Self {
        Self {
            enable: true,
            invert: false,
            blend: Blend::Override,
            kind,
        }
    }
}

fn truthy(word: &str) -> Option<bool> {
    match word.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

impl SelOp {
    /// The polygons of `mesh` this op picks, before Invert and Blend.
    fn picks(&self, mesh: &Mesh) -> Result<Vec<bool>> {
        let n = mesh.faces.len();
        let mut out = vec![false; n];
        match &self.kind {
            SelKind::Index { index } => {
                let s = crate::mesh_stack::SelectByIndex::parse(index)?;
                if let Some(&bad) = s.polygons.iter().find(|&&f| f as usize >= n) {
                    return Err(format!(
                        "Select by Index names polygon {bad}, but the mesh below has {n} polygons"
                    ));
                }
                for f in s.polygons {
                    out[f as usize] = true;
                }
            }
            SelKind::Range { start, end, step } => {
                if *step == 0 {
                    return Err("Select by Range needs a Step of at least 1".into());
                }
                if n > 0 {
                    let end = if *end == 0 {
                        n - 1
                    } else {
                        (*end as usize).min(n - 1)
                    };
                    let mut i = *start as usize;
                    while i <= end {
                        out[i] = true;
                        i += *step as usize;
                    }
                }
            }
            SelKind::Pattern { pattern } => {
                let bits: Vec<bool> = pattern
                    .split(',')
                    .filter(|w| !w.trim().is_empty())
                    .map(|w| {
                        truthy(w).ok_or_else(|| {
                            format!(
                                "Select by Pattern: “{}” isn't 1/0, True/False or Yes/No",
                                w.trim()
                            )
                        })
                    })
                    .collect::<Result<_>>()?;
                if !bits.is_empty() {
                    for (i, o) in out.iter_mut().enumerate() {
                        *o = bits[i % bits.len()];
                    }
                }
            }
            SelKind::Previous { source, name } => {
                // PG-04: an unknown or empty name selects nothing.
                if let Some(set) = mesh.attributes.polygon_sets.get(&group_set(*source, name)) {
                    for &f in set {
                        if let Some(o) = out.get_mut(f as usize) {
                            *o = true;
                        }
                    }
                }
            }
            SelKind::SelectionSet { name } => {
                if let Some(set) = mesh.attributes.polygon_sets.get(name) {
                    for &f in set {
                        if let Some(o) = out.get_mut(f as usize) {
                            *o = true;
                        }
                    }
                }
            }
        }
        Ok(out)
    }
}

/// An op's Selection input on `mesh`: `None` when it has none (the op
/// works on the whole mesh, PG-05), otherwise the picked polygons, which
/// may be none at all (PG-04).
pub fn resolve(
    index: Option<&crate::mesh_stack::SelectByIndex>,
    selops: &[SelOp],
    mesh: &Mesh,
) -> Result<Option<Vec<u32>>> {
    let index = index.filter(|s| !s.polygons.is_empty());
    // Turned-off selection ops act as if they weren't there.
    if index.is_none() && !selops.iter().any(|s| s.enable) {
        return Ok(None);
    }
    let n = mesh.faces.len();
    let mut sel = vec![false; n];
    if let Some(s) = index {
        if let Some(&bad) = s.polygons.iter().find(|&&f| f as usize >= n) {
            return Err(format!(
                "Select by Index names polygon {bad}, but the mesh below has {n} polygons"
            ));
        }
        for &f in &s.polygons {
            sel[f as usize] = true;
        }
    }
    for s in selops.iter().filter(|s| s.enable) {
        let mut mine = s.picks(mesh)?;
        if s.invert {
            mine.iter_mut().for_each(|b| *b = !*b);
        }
        for (a, b) in sel.iter_mut().zip(mine) {
            *a = match s.blend {
                Blend::Override => b,
                Blend::Add => *a || b,
                Blend::Subtract => *a && !b,
                Blend::Multiply => *a && b,
            };
        }
    }
    Ok(Some(
        sel.iter()
            .enumerate()
            .filter(|(_, b)| **b)
            .map(|(i, _)| i as u32)
            .collect(),
    ))
}
