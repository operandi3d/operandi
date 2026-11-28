//! UNY-B01..03 Unify: remove duplicate polygons.
//!
//! Unify merges polygons that sit in exactly the same
//! position and share the same vertices into one. Every group of surface
//! polygons with the same boundary (any size, either winding) that touches
//! the selection (or every group, with nothing selected) keeps its lowest
//! face and loses the others. A duplicate whose tags or corner data differ
//! from the kept face is left alone unless Force Unify is on, in which case
//! the kept face's data wins.
use crate::{Editor, Mesh, Mode, Result, Scene};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct UnifyTarget {
    document: crate::DocumentToken,
    scene: Scene,
}
#[derive(Clone, Debug, PartialEq)]
pub struct UnifyRequest {
    pub force: bool,
    pub expected: Option<UnifyTarget>,
}

fn equal_rows<T: PartialEq>(rows: &[Vec<T>], a: usize, b: usize, correspondence: &[usize]) -> bool {
    correspondence
        .iter()
        .enumerate()
        .all(|(i, &j)| rows[a][i] == rows[b][j])
}
fn compatible(source: &Mesh, a: usize, b: usize, map: &[usize]) -> bool {
    let x = &source.faces[a];
    let y = &source.faces[b];
    if x.material != y.material || x.material_label != y.material_label || x.smooth != y.smooth {
        return false;
    }
    match (&x.uv, &y.uv) {
        (None, None) => {}
        (Some(u), Some(v)) => {
            if !map.iter().enumerate().all(|(i, &j)| u[i] == v[j]) {
                return false;
            }
        }
        _ => return false,
    }
    let at = &source.attributes;
    if (!at.representations.is_empty() && at.representations[a] != at.representations[b])
        || (!at.smoothing_labels.is_empty() && at.smoothing_labels[a] != at.smoothing_labels[b])
        || at
            .polygon_sets
            .values()
            .any(|set| set.contains(&(a as u32)) != set.contains(&(b as u32)))
    {
        return false;
    }
    if source.uv_maps.values().any(|m| !equal_rows(m, a, b, map))
        || at.uv_records.values().any(|m| !equal_rows(m, a, b, map))
        || at
            .corner_normals
            .values()
            .any(|m| !equal_rows(m, a, b, map))
        || at
            .corner_weights
            .values()
            .any(|m| !equal_rows(m, a, b, map))
        || at.corner_colors.values().any(|m| !equal_rows(m, a, b, map))
        || at
            .typed_maps
            .values()
            .any(|m| !m.corners.is_empty() && !equal_rows(&m.corners, a, b, map))
    {
        return false;
    }
    true
}
fn same_cycle(a: &[u32], b: &[u32]) -> bool {
    let n = a.len();
    n == b.len()
        && (0..n).any(|i| {
            (0..n).all(|j| a[j] == b[(i + j) % n]) || (0..n).all(|j| a[j] == b[(i + n - j) % n])
        })
}
impl Editor {
    pub fn unify_target(&self) -> Result<UnifyTarget> {
        if self.session.pen_draft.is_some() || self.i2_active() {
            return Err("Finish or Cancel the active tool before Unify".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish selection before Unify".into());
        }
        if self.scene.selection.mode != Mode::Polygon {
            return Err("Unify requires Polygon mode".into());
        }
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh before Unify")?;
        self.scene.validate()?;
        let selection = &self.scene.selection.faces;
        if selection.iter().collect::<BTreeSet<_>>().len() != selection.len()
            || selection
                .iter()
                .any(|&f| f as usize >= item.mesh.faces.len())
        {
            return Err("Invalid polygon selection".into());
        }
        if selection.iter().any(|&f| {
            item.mesh.is_polyline(f as usize) || item.mesh.attributes.degenerate_faces.contains(&f)
        }) {
            return Err("Unify supports nondegenerate surface polygons".into());
        }
        Ok(UnifyTarget {
            document: self.identities.document_token(),
            scene: self.scene.clone(),
        })
    }
    pub(crate) fn unify_polygons(&mut self, request: UnifyRequest) -> Result<()> {
        let target = self.unify_target()?;
        if request.expected.as_ref().is_some_and(|v| *v != target) {
            return Err("Unify target changed since the dialog opened".into());
        }
        let item = self.scene.active().ok_or("Select a mesh")?;
        let item_id = item.id;
        let source = &item.mesh;
        let selection = &self.scene.selection.faces;
        let mut groups: BTreeMap<Vec<u32>, Vec<usize>> = BTreeMap::new();
        for (i, face) in source.faces.iter().enumerate() {
            if source.is_polyline(i) || source.attributes.degenerate_faces.contains(&(i as u32)) {
                continue;
            }
            let mut key = face.vertices.clone();
            key.sort_unstable();
            groups.entry(key).or_default().push(i);
        }
        let mut removed: Vec<u32> = Vec::new();
        let mut kept: Vec<u32> = Vec::new();
        for group in groups.values() {
            if group.len() < 2
                || (!selection.is_empty()
                    && !group.iter().any(|i| selection.contains(&(*i as u32))))
            {
                continue;
            }
            let a = group[0];
            let mut any = false;
            for &b in &group[1..] {
                if !same_cycle(&source.faces[a].vertices, &source.faces[b].vertices) {
                    continue;
                }
                let mapping = source.faces[a]
                    .vertices
                    .iter()
                    .map(|v| {
                        source.faces[b]
                            .vertices
                            .iter()
                            .position(|w| w == v)
                            .ok_or_else(|| "Unify corner correspondence missing".to_string())
                    })
                    .collect::<Result<Vec<_>>>()?;
                let same_data = compatible(source, a, b, &mapping)
                    && source.faces[a].part == source.faces[b].part;
                if same_data || request.force {
                    removed.push(b as u32);
                    any = true;
                }
            }
            if any {
                kept.push(a as u32);
            }
        }
        let before = self.scene.clone();
        if !removed.is_empty() {
            removed.sort_unstable();
            let removal = crate::remove_faces(source, &removed)?;
            let ids = self
                .identities
                .current
                .meshes
                .get(&item_id)
                .ok_or("Missing mesh identity owner")?
                .remap_faces(source, &removal.mesh, &removal.survivors)?;
            let new_index = |f: u32| {
                removal
                    .survivors
                    .iter()
                    .position(|&s| s == f as usize)
                    .map(|i| i as u32)
            };
            self.scene.active_mut().ok_or("Missing active mesh")?.mesh = removal.mesh.into();
            self.scene.selection.faces = kept.iter().filter_map(|&f| new_index(f)).collect();
            self.identities.current.meshes.insert(item_id, ids.into());
        } else {
            self.scene.selection.faces.clear();
        }
        self.scene.validate()?;
        self.identities.observe(&self.scene)?;
        if self.scene != before {
            self.record("Unify".into(), before);
        }
        Ok(())
    }
}
