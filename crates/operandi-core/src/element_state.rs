//! Geometry lock (LCK-20..33) and quick hide (HID-01..13): per-item sets of
//! vertices, edges and polygons that stay out of selection and editing.
//!
//! Both are kept on the item (`Item::elements`), apart from the item lock
//! (Assembly > Lock, `item_common.rs`) and the Items-list eye. Lock is saved
//! with the scene (provisional, LCK-26); hidden state is not (HID-05).
//!
//! What a set member covers (provisional readings, NLH-Q01..Q04):
//! - Lock reaches down: a locked polygon locks its edges and points, a
//!   locked edge its two points. Selection skips locked elements of the
//!   current mode.
//! - Hide reaches up: a hidden point hides every edge and polygon using it,
//!   a hidden edge the polygons using it. A hidden polygon hides only
//!   itself; its points and edges stay while a shown polygon uses them.
//! - "Not editable": no command moves a point of locked or hidden
//!   geometry, even one shared with editable geometry. Point-only edits
//!   (Move, Rotate, deformers) keep those points where they were; an edit
//!   that changes topology and would alter protected geometry is refused.
//!
//! The editor applies all of this in one place after every command
//! (`Editor::protect_elements`), so every selection method, tool and
//! command obeys it without its own checks.
use crate::{Mesh, Mode, Result, Scene};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Vertices, edges (lower point first) and polygons of one mesh.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementMask {
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub vertices: BTreeSet<u32>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub edges: BTreeSet<(u32, u32)>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub faces: BTreeSet<u32>,
}
impl ElementMask {
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.edges.is_empty() && self.faces.is_empty()
    }
    fn clear(&mut self) {
        *self = Self::default();
    }
}

/// An item's locked and hidden geometry.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementState {
    /// LCK-20: saved with the scene (LCK-26, provisional).
    #[serde(default, skip_serializing_if = "ElementMask::is_empty")]
    pub locked: ElementMask,
    /// HID-05: never saved; a reopened scene shows everything.
    #[serde(skip)]
    pub hidden: ElementMask,
}
impl ElementState {
    pub fn is_empty(&self) -> bool {
        self.locked.is_empty() && self.hidden.is_empty()
    }
    /// For `skip_serializing_if`: nothing that is saved.
    pub fn nothing_saved(&self) -> bool {
        self.locked.is_empty()
    }
    pub fn validate(&self, mesh: &Mesh) -> Result<()> {
        for mask in [&self.locked, &self.hidden] {
            let v = mesh.vertices.len() as u32;
            if mask.vertices.iter().any(|&i| i >= v)
                || mask.edges.iter().any(|&(a, b)| a >= b || b >= v)
                || mask.faces.iter().any(|&f| f as usize >= mesh.faces.len())
            {
                return Err("Locked or hidden geometry refers to missing elements".into());
            }
        }
        Ok(())
    }
}

/// Lock or hide request (LCK-30..33, HID-10..13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementOp {
    /// J / H: the selection; everything when nothing is selected.
    Selected,
    /// Shift+J / Shift+H: everything not selected.
    Unselected,
    /// Cmd+J / Ctrl+H: swap.
    Invert,
    /// I / U: unlock or show everything in the selected items.
    Clear,
}
/// Which of the two sets a request edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementKind {
    Lock,
    Hide,
}
impl ElementKind {
    pub fn label(self, op: ElementOp) -> &'static str {
        match (self, op) {
            (ElementKind::Lock, ElementOp::Selected) => "Lock Selected",
            (ElementKind::Lock, ElementOp::Unselected) => "Lock Unselected",
            (ElementKind::Lock, ElementOp::Invert) => "Lock Invert",
            (ElementKind::Lock, ElementOp::Clear) => "Unlock",
            (ElementKind::Hide, ElementOp::Selected) => "Hide Selected",
            (ElementKind::Hide, ElementOp::Unselected) => "Hide Unselected",
            (ElementKind::Hide, ElementOp::Invert) => "Hide Invert",
            (ElementKind::Hide, ElementOp::Clear) => "Unhide",
        }
    }
}

/// What the sets mean for one mesh, worked out once: per-element flags.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Protection {
    pub vertex_locked: Vec<bool>,
    pub vertex_hidden: Vec<bool>,
    pub face_locked: Vec<bool>,
    pub face_hidden: Vec<bool>,
    /// Points no command may move (locked or hidden geometry uses them).
    pub pinned: Vec<bool>,
    edge_locked: HashSet<(u32, u32)>,
    /// Edges of shown polygons, when anything is hidden.
    edge_shown: Option<HashSet<(u32, u32)>>,
    hidden_edges: BTreeSet<(u32, u32)>,
}
impl Protection {
    /// `None` when nothing in `state` applies.
    pub fn of(mesh: &Mesh, state: &ElementState) -> Option<Self> {
        if state.is_empty() {
            return None;
        }
        let nv = mesh.vertices.len();
        let nf = mesh.faces.len();
        let (locked, hidden) = (&state.locked, &state.hidden);
        let mut p = Protection {
            vertex_locked: vec![false; nv],
            vertex_hidden: vec![false; nv],
            face_locked: vec![false; nf],
            face_hidden: vec![false; nf],
            pinned: vec![false; nv],
            hidden_edges: hidden.edges.clone(),
            ..Default::default()
        };
        let set = |flags: &mut Vec<bool>, i: u32| {
            if let Some(f) = flags.get_mut(i as usize) {
                *f = true;
            }
        };
        // Lock reaches down.
        for &v in &locked.vertices {
            set(&mut p.vertex_locked, v);
        }
        for &(a, b) in &locked.edges {
            p.edge_locked.insert((a, b));
            set(&mut p.vertex_locked, a);
            set(&mut p.vertex_locked, b);
        }
        for &f in &locked.faces {
            let Some(face) = mesh.faces.get(f as usize) else {
                continue;
            };
            p.face_locked[f as usize] = true;
            for (a, b) in mesh.element_spans(f as usize) {
                p.edge_locked.insert((a.min(b), a.max(b)));
            }
            for &v in &face.vertices {
                set(&mut p.vertex_locked, v);
            }
        }
        // Hide reaches up: polygons using a hidden point or edge.
        if !hidden.is_empty() {
            let mut used = vec![false; nv];
            let mut shown_used = vec![false; nv];
            let mut shown_edges = HashSet::new();
            for f in 0..nf {
                let face = &mesh.faces[f];
                let spans: Vec<(u32, u32)> = mesh
                    .element_spans(f)
                    .map(|(a, b)| (a.min(b), a.max(b)))
                    .collect();
                let hide = hidden.faces.contains(&(f as u32))
                    || face.vertices.iter().any(|v| hidden.vertices.contains(v))
                    || spans.iter().any(|e| hidden.edges.contains(e));
                p.face_hidden[f] = hide;
                for &v in &face.vertices {
                    used[v as usize] = true;
                    if !hide {
                        shown_used[v as usize] = true;
                    }
                }
                if !hide {
                    shown_edges.extend(spans);
                }
            }
            for v in 0..nv {
                p.vertex_hidden[v] =
                    hidden.vertices.contains(&(v as u32)) || (used[v] && !shown_used[v]);
            }
            p.edge_shown = Some(shown_edges);
            for f in (0..nf).filter(|&f| p.face_hidden[f]) {
                for &v in &mesh.faces[f].vertices {
                    p.pinned[v as usize] = true;
                }
            }
            for &(a, b) in &hidden.edges {
                set(&mut p.pinned, a);
                set(&mut p.pinned, b);
            }
            for v in 0..nv {
                p.pinned[v] |= p.vertex_hidden[v];
            }
        }
        for v in 0..nv {
            p.pinned[v] |= p.vertex_locked[v];
        }
        Some(p)
    }
    /// Every locked edge, lower point first (LCK-23 draws them).
    pub fn locked_edges(&self) -> &HashSet<(u32, u32)> {
        &self.edge_locked
    }
    pub fn edge_locked(&self, a: u32, b: u32) -> bool {
        self.edge_locked.contains(&(a.min(b), a.max(b)))
    }
    pub fn edge_hidden(&self, a: u32, b: u32) -> bool {
        let e = (a.min(b), a.max(b));
        self.hidden_edges.contains(&e)
            || self.vertex_hidden.get(a as usize).copied().unwrap_or(false)
            || self.vertex_hidden.get(b as usize).copied().unwrap_or(false)
            || self.edge_shown.as_ref().is_some_and(|s| !s.contains(&e))
    }
    /// Whether selection in `mode` must skip this element.
    pub fn vertex_blocked(&self, v: u32) -> bool {
        let i = v as usize;
        self.vertex_locked.get(i).copied().unwrap_or(false)
            || self.vertex_hidden.get(i).copied().unwrap_or(false)
    }
    pub fn edge_blocked(&self, a: u32, b: u32) -> bool {
        self.edge_locked(a, b) || self.edge_hidden(a, b)
    }
    pub fn face_blocked(&self, f: u32) -> bool {
        let i = f as usize;
        self.face_locked.get(i).copied().unwrap_or(false)
            || self.face_hidden.get(i).copied().unwrap_or(false)
    }
    pub fn any_pinned(&self) -> bool {
        self.pinned.iter().any(|&p| p)
    }
}

/// Every element of `mesh` of the kind `mode` selects.
fn all_of(mesh: &Mesh, mode: Mode) -> ElementMask {
    let mut m = ElementMask::default();
    match mode {
        Mode::Vertex => m.vertices = (0..mesh.vertices.len() as u32).collect(),
        Mode::Edge => m.edges = mesh.edges().into_iter().collect(),
        Mode::Polygon => m.faces = (0..mesh.faces.len() as u32).collect(),
        Mode::Item => {
            m.vertices = (0..mesh.vertices.len() as u32).collect();
            m.faces = (0..mesh.faces.len() as u32).collect();
        }
    }
    m
}
fn union(into: &mut ElementMask, add: ElementMask) {
    into.vertices.extend(add.vertices);
    into.edges.extend(add.edges);
    into.faces.extend(add.faces);
}
/// `all` minus `minus`, in the kinds `all` holds.
fn difference(all: &ElementMask, minus: &ElementMask) -> ElementMask {
    ElementMask {
        vertices: all.vertices.difference(&minus.vertices).copied().collect(),
        edges: all.edges.difference(&minus.edges).copied().collect(),
        faces: all.faces.difference(&minus.faces).copied().collect(),
    }
}
/// The current mode's part of `mask`, replaced by `with`.
fn replace_kind(mask: &mut ElementMask, mode: Mode, with: ElementMask) {
    match mode {
        Mode::Vertex => mask.vertices = with.vertices,
        Mode::Edge => mask.edges = with.edges,
        Mode::Polygon => mask.faces = with.faces,
        Mode::Item => {
            mask.vertices = with.vertices;
            mask.faces = with.faces;
            mask.edges.clear();
        }
    }
}
fn kind_of(mask: &ElementMask, mode: Mode) -> ElementMask {
    let mut m = ElementMask::default();
    match mode {
        Mode::Vertex => m.vertices = mask.vertices.clone(),
        Mode::Edge => m.edges = mask.edges.clone(),
        Mode::Polygon => m.faces = mask.faces.clone(),
        Mode::Item => {
            m.vertices = mask.vertices.clone();
            m.faces = mask.faces.clone();
        }
    }
    m
}

impl Scene {
    /// The mesh items a lock/hide request covers: the selected items, or
    /// every mesh item for Unhide with nothing selected (HID-13).
    fn element_targets(&self, kind: ElementKind, op: ElementOp) -> Vec<u64> {
        let mut ids: Vec<u64> = self.selection.item.into_iter().collect();
        ids.extend(self.selection.also.iter().copied());
        ids.retain(|id| self.items.iter().any(|i| i.id == *id && i.is_mesh()));
        if ids.is_empty() && kind == ElementKind::Hide && op == ElementOp::Clear {
            ids = self
                .items
                .iter()
                .filter(|i| i.is_mesh())
                .map(|i| i.id)
                .collect();
        }
        ids
    }
    /// LCK-30..33 / HID-10..13. Component modes work on the active mesh
    /// (where the component selection lives); Item mode and Unlock/Unhide
    /// on every selected mesh item. Errors when nothing would change.
    pub fn edit_elements(&mut self, kind: ElementKind, op: ElementOp) -> Result<()> {
        let mode = self.selection.mode;
        let targets = if op == ElementOp::Clear || mode == Mode::Item {
            self.element_targets(kind, op)
        } else {
            self.selection
                .item
                .filter(|id| self.items.iter().any(|i| i.id == *id && i.is_mesh()))
                .into_iter()
                .collect()
        };
        if targets.is_empty() {
            return Err("Select a mesh item".into());
        }
        let before: Vec<ElementState> = targets
            .iter()
            .filter_map(|id| self.item(*id))
            .map(|i| i.elements.clone())
            .collect();
        // Item mode Unselected: the mesh items that aren't selected.
        if mode == Mode::Item && op == ElementOp::Unselected {
            let others: Vec<u64> = self
                .items
                .iter()
                .filter(|i| i.is_mesh() && !targets.contains(&i.id))
                .map(|i| i.id)
                .collect();
            if others.is_empty() {
                return Err("Every mesh item is selected".into());
            }
            for item in self.items.iter_mut().filter(|i| others.contains(&i.id)) {
                let all = all_of(&item.mesh, Mode::Item);
                let mask = kind.mask_mut(&mut item.elements);
                union(mask, all);
            }
            return Ok(());
        }
        let selected = selected_mask(self);
        for item in self.items.iter_mut().filter(|i| targets.contains(&i.id)) {
            let mesh = item.mesh.clone();
            let mask = kind.mask_mut(&mut item.elements);
            match op {
                ElementOp::Clear => mask.clear(),
                ElementOp::Selected => {
                    let chosen = match (&selected, mode) {
                        (Some(s), m) if m != Mode::Item && !kind_of(s, m).is_empty() => {
                            kind_of(s, m)
                        }
                        _ => all_of(&mesh, mode),
                    };
                    union(mask, chosen);
                }
                ElementOp::Unselected => {
                    let chosen = selected
                        .as_ref()
                        .map(|s| kind_of(s, mode))
                        .unwrap_or_default();
                    union(mask, difference(&all_of(&mesh, mode), &chosen));
                }
                ElementOp::Invert => {
                    let now = kind_of(mask, mode);
                    replace_kind(mask, mode, difference(&all_of(&mesh, mode), &now));
                }
            }
        }
        let after: Vec<ElementState> = targets
            .iter()
            .filter_map(|id| self.item(*id))
            .map(|i| i.elements.clone())
            .collect();
        if before == after {
            return Err(match (kind, op) {
                (ElementKind::Lock, ElementOp::Clear) => "Nothing is locked".into(),
                (ElementKind::Hide, ElementOp::Clear) => "Nothing is hidden".into(),
                _ => "Nothing to change".into(),
            });
        }
        Ok(())
    }

    /// Take locked and hidden elements out of the selection (LCK-22,
    /// HID-04, and LCK-20/HID-03 for every way of selecting).
    pub fn drop_protected_from_selection(&mut self) {
        let Some(item) = self.active() else {
            return;
        };
        if item.elements.is_empty() {
            return;
        }
        let Some(p) = Protection::of(&item.mesh, &item.elements) else {
            return;
        };
        let s = &mut self.selection;
        s.vertices.retain(|&v| !p.vertex_blocked(v));
        s.edges.retain(|&(a, b)| !p.edge_blocked(a, b));
        s.faces.retain(|&f| !p.face_blocked(f));
    }
}

impl ElementKind {
    fn mask_mut(self, state: &mut ElementState) -> &mut ElementMask {
        match self {
            ElementKind::Lock => &mut state.locked,
            ElementKind::Hide => &mut state.hidden,
        }
    }
}

/// The active item's component selection, `None` in Item mode.
fn selected_mask(scene: &Scene) -> Option<ElementMask> {
    let s = &scene.selection;
    (s.mode != Mode::Item).then(|| ElementMask {
        vertices: s.vertices.iter().copied().collect(),
        edges: s.edges.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect(),
        faces: s.faces.iter().copied().collect(),
    })
}

/// Position key for exact lookups: locked and hidden points never move, so
/// a topology edit finds them again where they were.
fn key(p: DVec3) -> [u64; 3] {
    [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
}

/// Keep `state` (made for `old`) valid for `new`, the same item's mesh
/// after a command, and keep protected points where they were.
///
/// Same topology: protected points are put back, so a Move of everything
/// leaves locked geometry in place (LCK-T4). Changed topology: each
/// protected element is found again by its points' exact positions; when
/// one is gone or moved, the edit is refused.
/// Returns the carried state and the points to put back.
type Carried = (ElementState, Vec<(usize, DVec3)>);
pub(crate) fn carry(old: &Mesh, new: &Mesh, state: &ElementState) -> Result<Carried> {
    let Some(p) = Protection::of(old, state) else {
        return Ok((state.clone(), Vec::new()));
    };
    let same_topology = old.vertices.len() == new.vertices.len()
        && old.faces.len() == new.faces.len()
        && old
            .faces
            .iter()
            .zip(&new.faces)
            .all(|(a, b)| a.vertices == b.vertices);
    if same_topology {
        let fixes = p
            .pinned
            .iter()
            .enumerate()
            .filter(|&(i, &pin)| pin && new.vertices[i] != old.vertices[i])
            .map(|(i, _)| (i, old.vertices[i]))
            .collect();
        return Ok((state.clone(), fixes));
    }
    const REFUSED: &str = "This would change locked or hidden geometry";
    let mut by_position: Option<HashMap<[u64; 3], Vec<u32>>> = None;
    let mut find_point = |v: u32| -> Result<u32> {
        let at = old.vertices[v as usize];
        if new.vertices.get(v as usize) == Some(&at) {
            return Ok(v);
        }
        let map = by_position.get_or_insert_with(|| {
            let mut m: HashMap<[u64; 3], Vec<u32>> = HashMap::new();
            for (i, &q) in new.vertices.iter().enumerate() {
                m.entry(key(q)).or_default().push(i as u32);
            }
            m
        });
        map.get(&key(at))
            .and_then(|c| c.first().copied())
            .ok_or_else(|| REFUSED.to_string())
    };
    // Every pinned point must still be there, unmoved.
    let mut point_map: HashMap<u32, u32> = HashMap::new();
    for (i, &pin) in p.pinned.iter().enumerate() {
        if pin {
            point_map.insert(i as u32, find_point(i as u32)?);
        }
    }
    let mut faces_by_points: Option<HashMap<Vec<u32>, u32>> = None;
    let mut find_face = |f: u32| -> Result<u32> {
        let points: Vec<u32> = old.faces[f as usize]
            .vertices
            .iter()
            .map(|v| point_map.get(v).copied().ok_or_else(|| REFUSED.to_string()))
            .collect::<Result<_>>()?;
        if new.faces.get(f as usize).map(|x| &x.vertices) == Some(&points) {
            return Ok(f);
        }
        let map = faces_by_points.get_or_insert_with(|| {
            new.faces
                .iter()
                .enumerate()
                .map(|(i, face)| (face.vertices.clone(), i as u32))
                .collect()
        });
        map.get(&points).copied().ok_or_else(|| REFUSED.to_string())
    };
    let edges: HashSet<(u32, u32)> = new.edges().into_iter().collect();
    let mut carry_mask = |mask: &ElementMask| -> Result<ElementMask> {
        let mut out = ElementMask::default();
        for &v in &mask.vertices {
            out.vertices
                .insert(point_map.get(&v).copied().ok_or(REFUSED)?);
        }
        for &(a, b) in &mask.edges {
            let (a, b) = (
                point_map.get(&a).copied().ok_or(REFUSED)?,
                point_map.get(&b).copied().ok_or(REFUSED)?,
            );
            let e = (a.min(b), a.max(b));
            if !edges.contains(&e) {
                return Err(REFUSED.into());
            }
            out.edges.insert(e);
        }
        for &f in &mask.faces {
            out.faces.insert(find_face(f)?);
        }
        Ok(out)
    };
    let locked = carry_mask(&state.locked)?;
    let hidden = carry_mask(&state.hidden)?;
    // Polygons hidden through a point or edge must survive too.
    for f in (0..old.faces.len()).filter(|&f| p.face_hidden[f] || p.face_locked[f]) {
        find_face(f as u32)?;
    }
    Ok((ElementState { locked, hidden }, Vec::new()))
}

impl crate::Editor {
    /// Run after every command except Undo/Redo (which restore a recorded
    /// scene): keep locked and hidden geometry unchanged in every mesh the
    /// command edited, carry the sets to the new topology, and take
    /// protected elements out of the selection. Folds its own changes into
    /// the step the command just recorded, so Redo replays them.
    pub(crate) fn protect_elements(&mut self, initial: &Scene) -> Result<()> {
        let mut changed = false;
        let mut unprotected = None;
        let mut restored = Vec::new();
        for index in 0..self.scene.items.len() {
            let item = &self.scene.items[index];
            let Some(old) = initial.item(item.id) else {
                continue;
            };
            if old.elements.is_empty()
                || crate::Shared::same(old.mesh.shared(), item.mesh.shared())
                || item.elements != old.elements
            {
                continue;
            }
            let (state, fixes) = carry(&old.mesh, &item.mesh, &old.elements)?;
            if fixes.is_empty() && state == item.elements {
                continue;
            }
            unprotected.get_or_insert_with(|| self.scene.clone());
            let item = &mut self.scene.items[index];
            if !fixes.is_empty() {
                let mesh = &mut *item.mesh;
                for (i, p) in fixes {
                    mesh.vertices[i] = p;
                }
                restored.push(item.id);
            }
            item.elements = state;
            changed = true;
        }
        // A mesh shared through Mesh Instances (mesh_instance.rs) keeps one
        // shape: the restored points reach the source and every other
        // instance too, or the items would save different points and a
        // reopen would re-share them (monkey seed 24, 2026-11-21).
        for id in restored {
            self.scene.share_mesh_of(id);
        }
        if let Some(active) = self.scene.active()
            && !active.elements.is_empty()
        {
            let before = self.scene.selection.clone();
            self.scene.drop_protected_from_selection();
            if self.scene.selection != before {
                if unprotected.is_none() {
                    let mut s = self.scene.clone();
                    s.selection = before;
                    unprotected = Some(s);
                }
                changed = true;
            }
        }
        if changed
            && let Some(unprotected) = unprotected
            && let Some(last) = self.undo.last_mut().map(std::sync::Arc::make_mut)
            && last.after == unprotected
        {
            last.after = self.scene.clone();
        }
        Ok(())
    }
}
