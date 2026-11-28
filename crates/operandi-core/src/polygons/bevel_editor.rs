//! Bevel configuration and recipes own parameters, never geometry or history.
use crate::{BevelOptions, Command, Editor, Falloff, MixMode, Parameters, Result, Tool};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BevelRecipe {
    pub shift: f64,
    pub inset: f64,
    pub grouped: bool,
    pub options: BevelOptions,
}
impl BevelRecipe {
    pub fn parameters(&self, mut p: Parameters) -> Parameters {
        p.shift = self.shift;
        p.inset = self.inset;
        p.group_polygons = self.grouped;
        p.bevel = self.options.clone();
        p
    }
    pub fn validate(&self) -> Result<()> {
        if [
            self.shift,
            self.inset,
            self.options.shift_range,
            self.options.inset_range,
            self.options.max_angle,
            self.options.weld_range,
        ]
        .iter()
        .any(|x| !x.is_finite())
            || self.options.weld_range < 0.
            || self.options.max_angle < 0.
            || !(1..=4096).contains(&self.options.segments)
            || self.options.material.len() > 1_048_576
        {
            return Err("Invalid Bevel recipe parameters".into());
        }
        Ok(())
    }
}
impl From<&Parameters> for BevelRecipe {
    fn from(p: &Parameters) -> Self {
        Self {
            shift: p.shift,
            inset: p.inset,
            grouped: p.group_polygons,
            options: p.bevel.clone(),
        }
    }
}
fn name_valid(name: &str) -> Result<()> {
    if name.trim().is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        Err("Choose a nonempty tool-state name (up to 256 bytes)".into())
    } else {
        Ok(())
    }
}
impl Editor {
    fn remap_falloff_stamps(&mut self, added: &[(Falloff, MixMode)]) {
        let old = &self.added_falloffs;
        if self.falloff_stamps.is_empty() || old == added {
            return;
        }
        if old.len() == added.len() {
            let mut changed = old.iter().zip(added).filter(|(a, b)| a != b);
            if let Some((a, b)) = changed.next()
                && std::mem::discriminant(&a.0) == std::mem::discriminant(&b.0)
                && changed.next().is_none()
            {
                // An in-place settings edit keeps the layer's paint.
                return;
            }
        }

        let mut new_indices = vec![None; old.len()];
        let mut claimed = vec![false; added.len()];
        // Match full entries first so Mix Mode can distinguish otherwise
        // identical falloffs. A second pass also allows Mix Mode edits.
        for match_mix in [true, false] {
            for (previous, new_index) in old.iter().zip(&mut new_indices) {
                if new_index.is_none()
                    && let Some(index) = added.iter().enumerate().position(|(i, layer)| {
                        !claimed[i]
                            && previous.0 == layer.0
                            && (!match_mix || previous.1 == layer.1)
                    })
                {
                    *new_index = Some(index);
                    claimed[index] = true;
                }
            }
        }
        use crate::uv_pipe::UV_STAMP_LAYER;
        self.falloff_stamps = std::mem::take(&mut self.falloff_stamps)
            .into_iter()
            .filter_map(|((layer, item), weights)| {
                let (offset, layer) = match layer.checked_sub(UV_STAMP_LAYER) {
                    Some(uv_layer) => (UV_STAMP_LAYER, uv_layer),
                    None => (0, layer),
                };
                let new_layer = if layer == 0 {
                    0
                } else {
                    new_indices.get(layer - 1).copied().flatten()? + 1
                };
                Some(((offset + new_layer, item), weights))
            })
            .collect();
    }

    pub(crate) fn dispatch_bevel(&mut self, cmd: Command) -> Result<()> {
        if let Command::SpatialFalloff(field) = cmd {
            if let Some(field) = field {
                field.validate()?;
            }
            if self
                .session
                .live
                .as_ref()
                .is_some_and(|l| !l.tool.card().layers.falloff)
            {
                return Err(crate::tool_card::falloff_refusal());
            }
            let mut next = self.clone();
            // VMF-10: picking another type replaces the whole stack; editing
            // the active falloff's fields keeps what was added on top.
            let same_kind = matches!((self.spatial_falloff, field), (Some(a), Some(b))
                if std::mem::discriminant(&a) == std::mem::discriminant(&b));
            if !same_kind {
                next.added_falloffs.clear();
                next.spatial_falloff_mix = crate::MixMode::default();
            }
            next.spatial_falloff = field;
            if !same_kind {
                next.falloff_element = None;
                next.falloff_stamps.clear();
            }
            // FAL-M-03: each type keeps its last settings for reactivation.
            match field {
                Some(crate::Falloff::Linear(f)) => next.linear_falloff_preset = f,
                Some(crate::Falloff::Radial(f)) => next.radial_falloff_preset = f,
                _ => {}
            }
            if let Some(l) = next.session.live.as_ref().filter(|l| l.applied) {
                next.dispatch(Command::Preview(l.params.clone()))?;
            }
            next.refresh_falloff_primitives();
            *self = next;
            return Ok(());
        }
        if let Command::FalloffPath(points) = cmd {
            if points.iter().any(|p| !p.is_finite()) {
                return Err("Path points must be numbers".into());
            }
            let mut next = self.clone();
            next.falloff_path = points;
            if let Some(l) = next.session.live.as_ref().filter(|l| l.applied) {
                next.dispatch(Command::Preview(l.params.clone()))?;
            }
            next.refresh_falloff_primitives();
            *self = next;
            return Ok(());
        }
        if let Command::FalloffStamps(stamps) = cmd {
            if self.spatial_falloff.is_none() && !stamps.is_empty() {
                return Err("Pick a falloff first".into());
            }
            if stamps.values().flatten().any(|w| !w.is_finite()) {
                return Err("Falloff weights must be numbers".into());
            }
            let mut next = self.clone();
            next.falloff_stamps = stamps
                .into_iter()
                .map(|(k, w)| (k, std::sync::Arc::new(w)))
                .collect();
            if let Some(l) = next.session.live.as_ref().filter(|l| l.applied) {
                next.dispatch(Command::Preview(l.params.clone()))?;
            }
            next.refresh_falloff_primitives();
            *self = next;
            return Ok(());
        }
        if let Command::FalloffElementPick(pick) = cmd {
            // FO-45: the pick only counts while an Element layer is stacked.
            if pick.is_some()
                && !self.active_falloff().is_some_and(|f| {
                    std::iter::once(f.first)
                        .chain(f.added.iter().map(|a| a.0))
                        .any(|l| matches!(l, crate::Falloff::Element(_)))
                })
            {
                return Err("Pick the Element falloff first".into());
            }
            let mut next = self.clone();
            next.falloff_element = pick;
            if let Some(l) = next.session.live.as_ref().filter(|l| l.applied) {
                next.dispatch(Command::Preview(l.params.clone()))?;
            }
            next.refresh_falloff_primitives();
            *self = next;
            return Ok(());
        }
        if let Command::SpatialFalloffMix(mix) = cmd {
            if self.spatial_falloff.is_none() {
                return Err("Pick a falloff first".into());
            }
            // FO-32: no effect on the first layer, so nothing re-applies.
            self.spatial_falloff_mix = mix;
            return Ok(());
        }
        if let Command::AddedFalloffs(added) = cmd {
            if self.spatial_falloff.is_none() && !added.is_empty() {
                return Err("Pick a falloff first; added falloffs stack on top of it".into());
            }
            for (f, _) in &added {
                f.validate()?;
            }
            let mut next = self.clone();
            next.remap_falloff_stamps(&added);
            next.added_falloffs = added;
            if let Some(l) = next.session.live.as_ref().filter(|l| l.applied) {
                next.dispatch(Command::Preview(l.params.clone()))?;
            }
            next.refresh_falloff_primitives();
            *self = next;
            return Ok(());
        }
        let l = self
            .session
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::Bevel)
            .ok_or("Arm Bevel first")?;
        match cmd {
            Command::ConfigureBevel(p) => {
                BevelRecipe::from(&p).validate()?;
                if p.bevel.weld_range != l.params.bevel.weld_range && !p.bevel.auto_weld {
                    return Err("Enable Auto Weld before changing Weld Range".into());
                }
                // TA-01/TA-02 (tool-activation v1): a panel edit is live from
                // the moment the tool turns on and keeps adjusting the same
                // bevel. An "edit" that changes nothing (the panel re-sending
                // the current values) stays a store, so switching on alone
                // changes nothing (TA-03).
                if l.applied || p != l.params {
                    self.dispatch(Command::Preview(p))?;
                } else {
                    let l = self.session.live.as_mut().unwrap();
                    l.params = p;
                    l.preview_error = None;
                }
            }
            Command::ResetBevel => {
                let mut p = l.params.clone();
                p.shift = 0.;
                p.inset = 0.;
                p.group_polygons = true;
                p.bevel = Default::default();
                // Reset may restore a disabled retained value atomically.
                if l.applied {
                    self.dispatch(Command::Preview(p))?;
                } else {
                    let l = self.session.live.as_mut().unwrap();
                    l.params = p;
                    l.preview_error = None;
                }
            }
            Command::BevelProfile(profile) => {
                let mut next = self.clone();
                next.bevel_profile = profile;
                if l.applied {
                    next.dispatch(Command::Preview(l.params.clone()))?;
                }
                *self = next;
            }
            Command::BevelStateSave(name) => {
                name_valid(&name)?;
                let recipe = BevelRecipe::from(&l.params);
                recipe.validate()?;
                self.bevel_states.insert(name.clone(), recipe);
                self.bevel_state = Some(name);
            }
            Command::BevelStateRecall(None) => self.bevel_state = None,
            Command::BevelStateRecall(Some(name)) => {
                let recipe = self.bevel_states.get(&name).ok_or("Unknown Bevel state")?;
                let p = recipe.parameters(l.params.clone());
                // Recall is atomic, including disabled retained attributes.
                recipe.validate()?;
                if l.applied {
                    self.dispatch(Command::Preview(p))?;
                } else {
                    let l = self.session.live.as_mut().unwrap();
                    l.params = p;
                    l.preview_error = None;
                }
                self.bevel_state = Some(name);
            }
            Command::ResetBevelState => {
                if let Some(name) = self.bevel_state.clone() {
                    self.dispatch_bevel(Command::BevelStateRecall(Some(name)))?;
                }
            }
            Command::BevelStateRename(name) => {
                name_valid(&name)?;
                let old = self.bevel_state.clone().ok_or("Select a Bevel state")?;
                if old != name && self.bevel_states.contains_key(&name) {
                    return Err("A Bevel state already has that name".into());
                }
                let recipe = self
                    .bevel_states
                    .remove(&old)
                    .ok_or("Unknown Bevel state")?;
                self.bevel_states.insert(name.clone(), recipe);
                self.bevel_state = Some(name);
            }
            Command::BevelStateDelete => {
                if let Some(name) = self.bevel_state.take() {
                    self.bevel_states.remove(&name);
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
