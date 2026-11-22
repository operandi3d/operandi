//! Vertex Map Tools (spec vertex-map-tools-2026-11-11, VMT-01..20): Paint,
//! Smooth, Scale Down, Erase and Set Value paint the selected weight map
//! with the sculpt brush machinery (tip, pressure, symmetry, falloff,
//! locks), and a finished stroke is one undo step (`Command::WeightStroke`).
//! Paint strength is applied per stroke (N32); the other formulas follow
//! the spec's build guesses (VM-Q12).
use super::*;
use std::collections::BTreeMap;

/// How much Scale Down pulls toward 0 per dab at full Scale Amount and
/// weight (VMT-13 "a small per-dab k", provisional).
pub const SCALE_DOWN_RATE: f64 = 0.1;

/// Dense per-point values of one map (`None`: no value).
pub(super) type Dense = Vec<Option<f64>>;
/// One map's changed points and their new values.
pub type MapChanges = (String, Vec<(u32, Option<f64>)>);

/// The weight maps one stroke paints on one mesh: the selected map and,
/// for Normalize Weights, the mesh's other weight maps (VMT-11).
#[derive(Clone, Debug)]
pub(super) struct WeightLayer {
    pub name: String,
    pub values: Vec<Option<f64>>,
    pub base: Vec<Option<f64>>,
    /// Strongest Paint dab so far at each touched point (N32).
    paint_strength: FxHashMap<u32, f64>,
    /// The other maps: name, current values, values at the press.
    pub others: Vec<(String, Dense, Dense)>,
    /// Points changed since the last flush.
    pub dirty: Vec<u32>,
    pub is_dirty: Vec<bool>,
}

fn dense(map: Option<&BTreeMap<u32, f64>>, n: usize) -> Vec<Option<f64>> {
    let mut v = vec![None; n];
    if let Some(map) = map {
        for (&i, &w) in map {
            if let Some(x) = v.get_mut(i as usize) {
                *x = Some(w);
            }
        }
    }
    v
}

impl WeightLayer {
    pub(super) fn new(mesh: &Mesh, name: &str) -> Self {
        let n = mesh.vertices.len();
        let maps = &mesh.attributes.point_weights;
        let values = dense(maps.get(name), n);
        let others = maps
            .iter()
            .filter(|(k, _)| k.as_str() != name)
            .map(|(k, m)| {
                let d = dense(Some(m), n);
                (k.clone(), d.clone(), d)
            })
            .collect();
        Self {
            name: name.to_owned(),
            base: values.clone(),
            values,
            paint_strength: FxHashMap::default(),
            others,
            dirty: Vec::new(),
            is_dirty: vec![false; n],
        }
    }

    fn mark(&mut self, v: u32) {
        if !self.is_dirty[v as usize] {
            self.is_dirty[v as usize] = true;
            self.dirty.push(v);
        }
    }

    /// Sets point `v`'s value and, with `normalize`, lowers the other maps
    /// there so the total is at most 100 % (VMT-11 build guess: only ever
    /// lowers, and does nothing with a single map).
    fn set(&mut self, v: u32, value: Option<f64>, normalize: bool) {
        let i = v as usize;
        let value = value.filter(|w| w.is_finite());
        if self.values[i] == value {
            return;
        }
        self.values[i] = value;
        self.mark(v);
        if !normalize || self.others.is_empty() {
            return;
        }
        let own = value.unwrap_or(0.);
        let rest: f64 = self
            .others
            .iter()
            .filter_map(|(_, m, _)| m[i])
            .filter(|w| *w > 0.)
            .sum();
        if rest > 0. && own + rest > 1. {
            let scale = (1. - own).max(0.) / rest;
            for (_, m, _) in &mut self.others {
                if let Some(w) = m[i].as_mut()
                    && *w > 0.
                {
                    *w *= scale;
                }
            }
        }
    }
}

/// A finished weight stroke's changes on one mesh: per map, the points and
/// their new values (`None` removes the value).
#[derive(Clone, Debug, PartialEq)]
pub struct WeightItemEdit {
    pub item: u64,
    pub maps: Vec<MapChanges>,
}

/// `Command::WeightStroke`'s payload: one undo step.
#[derive(Clone, Debug, PartialEq)]
pub struct WeightEdit {
    pub label: String,
    pub items: Vec<WeightItemEdit>,
}

impl WeightEdit {
    pub fn is_empty(&self) -> bool {
        self.items
            .iter()
            .all(|i| i.maps.iter().all(|(_, p)| p.is_empty()))
    }
}

impl Target {
    /// The weight map values of the point's edge neighbours (no value
    /// reads as 0).
    fn neighbour_weight(&self, layer: &WeightLayer, v: u32) -> Option<f64> {
        let mut sum = 0.;
        let mut n = 0usize;
        for &f in self.point_faces.of(v) {
            let face = &self.base.faces[f as usize].vertices;
            let k = face.len();
            for (i, &u) in face.iter().enumerate() {
                if u != v {
                    continue;
                }
                for w in [face[(i + k - 1) % k], face[(i + 1) % k]] {
                    sum += layer.values[w as usize].unwrap_or(0.);
                    n += 1;
                }
            }
        }
        (n > 0).then(|| sum / n as f64)
    }

    /// Writes the weight values changed since the last flush into the
    /// scene's mesh, so the display follows the stroke (VMT-20).
    pub(super) fn flush_weights(&mut self, mesh: &mut Mesh) {
        let Some(layer) = self.weights.as_mut() else {
            return;
        };
        if layer.dirty.is_empty() {
            return;
        }
        let dirty = std::mem::take(&mut layer.dirty);
        let maps = &mut mesh.attributes.point_weights;
        let own = maps.entry(layer.name.clone()).or_default();
        for &v in &dirty {
            layer.is_dirty[v as usize] = false;
            match layer.values[v as usize] {
                Some(w) => own.insert(v, w),
                None => own.remove(&v),
            };
        }
        for (name, m, _) in &layer.others {
            if let Some(map) = maps.get_mut(name) {
                for &v in &dirty {
                    match m[v as usize] {
                        Some(w) => map.insert(v, w),
                        None => map.remove(&v),
                    };
                }
            }
        }
    }

    /// The changes against the press.
    pub(super) fn weight_changes(&self) -> Option<WeightItemEdit> {
        let layer = self.weights.as_ref()?;
        let diff = |now: &[Option<f64>], base: &[Option<f64>]| -> Vec<(u32, Option<f64>)> {
            now.iter()
                .zip(base)
                .enumerate()
                .filter(|(_, (a, b))| a != b)
                .map(|(i, (a, _))| (i as u32, *a))
                .collect()
        };
        let mut maps = vec![(layer.name.clone(), diff(&layer.values, &layer.base))];
        for (name, now, base) in &layer.others {
            let d = diff(now, base);
            if !d.is_empty() {
                maps.push((name.clone(), d));
            }
        }
        maps.retain(|(_, p)| !p.is_empty());
        (!maps.is_empty()).then_some(WeightItemEdit {
            item: self.item,
            maps,
        })
    }
}

impl Stroke {
    /// One dab of a Vertex Map tool on target `i` (VMT-10..15).
    pub(super) fn weight_dab(&mut self, i: usize, touches: &[Touch], acting: SculptBrush) {
        let settings = self.setup.settings;
        let ctrl = self.setup.ctrl;
        let amount = settings.offset_amount;
        let normalize = settings.normalize_weights;
        let t = &mut self.targets[i];
        let Some(mut layer) = t.weights.take() else {
            return;
        };
        match acting {
            SculptBrush::VmPaint => {
                let target = if ctrl { 0. } else { 1. };
                for touch in touches {
                    let k = (amount * touch.weight).clamp(0., 1.);
                    if !k.is_finite() || k == 0. {
                        continue;
                    }
                    let strongest = layer.paint_strength.entry(touch.v).or_default();
                    if k <= *strongest {
                        continue;
                    }
                    *strongest = k;
                    let start = layer.base[touch.v as usize].unwrap_or(0.);
                    let new = start + (target - start) * k;
                    layer.set(touch.v, Some(new), normalize);
                }
            }
            SculptBrush::VmSmooth => {
                // Shift-smoothing uses Smooth Amount; the tool its own.
                let strength = if acting != self.setup.brush {
                    settings.smooth_amount
                } else {
                    amount
                };
                let moves: Vec<(u32, f64)> = touches
                    .iter()
                    .filter_map(|touch| {
                        let avg = t.neighbour_weight(&layer, touch.v)?;
                        let old = layer.values[touch.v as usize].unwrap_or(0.);
                        let k = (strength * touch.weight).clamp(0., 1.);
                        // Ctrl sharpens: away from the neighbours' average.
                        let step = (avg - old) * k;
                        Some((touch.v, if ctrl { old - step } else { old + step }))
                    })
                    .collect();
                for (v, w) in moves {
                    layer.set(v, Some(w), normalize);
                }
            }
            SculptBrush::VmScaleDown => {
                for touch in touches {
                    let Some(old) = layer.values[touch.v as usize] else {
                        continue;
                    };
                    let k = (amount * touch.weight * SCALE_DOWN_RATE).clamp(0., 1.);
                    let new = if ctrl { old * (1. + k) } else { old * (1. - k) };
                    layer.set(touch.v, Some(new), normalize);
                }
            }
            SculptBrush::VmErase => {
                for touch in touches {
                    layer.set(touch.v, None, false);
                }
            }
            SculptBrush::VmSetValue => {
                // VMT-15: exactly Value everywhere under the brush, no fade;
                // Ctrl erases.
                for touch in touches {
                    let value = (!ctrl).then_some(amount);
                    layer.set(touch.v, value, normalize && value.is_some());
                }
            }
            _ => {}
        }
        self.targets[i].weights = Some(layer);
    }

    /// The finished weight stroke (Vertex Map Tools), or `None` for a
    /// sculpt stroke.
    pub fn weight_edit(&self) -> Option<WeightEdit> {
        if !self.acting.is_vertex_map() {
            return None;
        }
        Some(WeightEdit {
            label: self.label(),
            items: self
                .targets
                .iter()
                .filter_map(|t| t.weight_changes())
                .collect(),
        })
    }
}

impl crate::editor::Editor {
    /// `Command::WeightStroke`: write the values a weight stroke set, as one
    /// undo step named after the tool (VMT-19). Maps it names that don't
    /// exist yet are created.
    pub(crate) fn weight_stroke(&mut self, edit: WeightEdit) -> Result<()> {
        self.require_idle()?;
        let before = self.scene.clone();
        let mut changed = false;
        for e in &edit.items {
            let item = self
                .scene
                .items
                .iter_mut()
                .find(|i| i.id == e.item && i.is_mesh())
                .ok_or("A painted mesh no longer exists")?;
            let n = item.mesh.vertices.len() as u32;
            if e.maps.iter().any(|(name, pts)| {
                name.trim().is_empty()
                    || pts
                        .iter()
                        .any(|&(v, w)| v >= n || w.is_some_and(|w| !w.is_finite() || w.abs() > 1e9))
            }) {
                self.scene = before;
                return Err("Painted weights must be on existing points and finite".into());
            }
            let mesh = &mut *item.mesh;
            for (name, pts) in &e.maps {
                let map = mesh
                    .attributes
                    .point_weights
                    .entry(name.clone())
                    .or_default();
                for &(v, w) in pts {
                    let was = map.get(&v).copied();
                    if was != w {
                        changed = true;
                        match w {
                            Some(w) => map.insert(v, w),
                            None => map.remove(&v),
                        };
                    }
                }
            }
        }
        if !changed {
            self.scene = before;
            return Ok(());
        }
        let label = if edit.label.trim().is_empty() {
            "Vertex Map Stroke".to_string()
        } else {
            edit.label
        };
        self.record(label, before);
        Ok(())
    }

    /// `Command::NewWeightMap` (VM-03): a weight map named `name` on every
    /// active mesh, with no values or every point at `value`; it becomes
    /// the selected map. One undo step.
    pub(crate) fn new_weight_map(&mut self, name: String, value: Option<f64>) -> Result<()> {
        self.require_idle()?;
        let name = name.trim().to_owned();
        if name.is_empty() {
            return Err("A weight map needs a name".into());
        }
        if value.is_some_and(|v| !v.is_finite()) {
            return Err("Value must be a number".into());
        }
        let ids = self.scene.active_meshes();
        if ids.is_empty() {
            return Err("Select a mesh to add a weight map to".into());
        }
        let before = self.scene.clone();
        for id in ids {
            let Some(item) = self.scene.item_mut(id) else {
                continue;
            };
            if item.mesh.attributes.point_weights.contains_key(&name) {
                self.scene = before;
                return Err(format!("A weight map named {name} already exists"));
            }
            let n = item.mesh.vertices.len() as u32;
            let map: BTreeMap<u32, f64> = match value {
                Some(v) => (0..n).map(|i| (i, v)).collect(),
                None => BTreeMap::new(),
            };
            item.mesh.attributes.point_weights.insert(name.clone(), map);
        }
        self.selected_weight_map = Some(name);
        self.record("New Weight Map".into(), before);
        Ok(())
    }
}
