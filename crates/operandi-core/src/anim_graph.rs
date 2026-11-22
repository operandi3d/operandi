//! Graph editor edits (BEHAVIOR-GRAPH-EDITOR GE-xx): key values and
//! frames, handles (slope, weight, broken), new keys at a value, Key
//! Keeping Shape, curve behaviours, interpolation and cycle, shadows,
//! the Euler filter and Simplify. Each is one named command and one undo
//! step (PLAN; UIC-160/225); the graph editor's drags record their final
//! result as these lines (GE-45, UIC-161/224).
use crate::anim_curve::{Behavior, Curve, Key, Slope};
use crate::animation::{self, AnimChannel, ChannelKey};
use crate::time_tick::Tick;
use crate::{Editor, Result};

/// GE-71 Set Value: how the typed value applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueMode {
    Set,
    Add,
    Multiply,
}

/// GE-60: a curve's interpolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interpolation {
    Curve,
    Linear,
    Stepped,
}

/// GE-80: what to do with the shadow copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowAction {
    Copy,
    Replace,
    Swap,
    Clear,
}

/// The channels related to `k` (GE-42): its X/Y/Z or R/G/B siblings.
fn related(k: &ChannelKey) -> Vec<ChannelKey> {
    let Some((base, part)) = k.channel.rsplit_once('.') else {
        return Vec::new();
    };
    let parts: &[&str] = match part {
        "X" | "Y" | "Z" => &["X", "Y", "Z"],
        "r" | "g" | "b" => &["r", "g", "b"],
        _ => return Vec::new(),
    };
    parts
        .iter()
        .filter(|p| **p != part)
        .map(|p| ChannelKey::new(k.item, &format!("{base}.{p}")))
        .collect()
}

impl Editor {
    /// The channels a curve-wide edit applies to: `given`, else the
    /// selected channels, else the channels of the selected keys.
    pub fn curve_scope(&self, given: Option<Vec<ChannelKey>>) -> Vec<ChannelKey> {
        let mut out =
            given.unwrap_or_else(|| self.anim.selected_channels.iter().cloned().collect());
        if out.is_empty() {
            out = self
                .live_selected_keys()
                .into_iter()
                .map(|(k, _)| k)
                .collect();
            out.dedup();
        }
        out.retain(|k| self.scene.animation.is_animated(k));
        out
    }

    /// Edits every selected key with `edit` (its channel, the key), as
    /// one undo step; times may change (the selection follows).
    fn edit_selected_keys(
        &mut self,
        label: &str,
        edit: impl Fn(&ChannelKey, &mut Key) -> Result<()>,
    ) -> Result<()> {
        let selected = self.live_selected_keys();
        if selected.is_empty() {
            return Err("Select keys first".into());
        }
        let mut moved = Vec::new();
        self.anim_step(label, |e| {
            let mut changed = false;
            for (k, t) in &selected {
                let Some(c) = e.scene.animation.channels.get_mut(k) else {
                    continue;
                };
                let Some(i) = c.curve.key_at(*t) else {
                    continue;
                };
                let mut key = c.curve.keys[i].clone();
                edit(k, &mut key)?;
                if !key.value.is_finite() {
                    return Err("That gives a value that isn't a number".into());
                }
                if key != c.curve.keys[i] {
                    changed = true;
                    c.curve.keys.remove(i);
                    moved.push((k.clone(), key.time));
                    c.curve.put(key);
                } else {
                    moved.push((k.clone(), *t));
                }
            }
            animation::repose(&mut e.scene);
            Ok(changed)
        })?;
        self.anim.selected_keys = moved.into_iter().collect();
        Ok(())
    }

    /// GE-40: the selected keys moved by `dt` in time and `dv` in value at
    /// once (a graph editor drag's result, one undo step, GE-45). Time
    /// Snap rounds the new times to frames (GE-41).
    pub fn offset_selected_keys(&mut self, dt: Tick, dv: f64) -> Result<()> {
        self.offset_keys(dt, dv, false)
    }

    /// [`Self::offset_selected_keys`]; with `linked` (GE-42) the related
    /// channels' keys (X/Y/Z, R/G/B) at the same times move in time too.
    pub fn offset_keys(&mut self, dt: Tick, dv: f64, linked: bool) -> Result<()> {
        if !dv.is_finite() {
            return Err("Give a number".into());
        }
        let own: std::collections::BTreeSet<(ChannelKey, Tick)> =
            self.live_selected_keys().into_iter().collect();
        if linked {
            for (k, t) in &own {
                for sib in related(k) {
                    if self
                        .scene
                        .animation
                        .channels
                        .get(&sib)
                        .is_some_and(|c| c.curve.key_at(*t).is_some())
                    {
                        self.anim.selected_keys.insert((sib, *t));
                    }
                }
            }
        }
        let snap = self.scene.animation.settings.snap;
        let rate = self.scene.time.rate;
        self.edit_selected_keys("Move Keys", |k, key| {
            let mine = own.contains(&(k.clone(), key.time));
            let t = key.time + dt;
            key.time = if snap {
                animation::nearest_frame(rate, t)
            } else {
                t
            };
            if mine {
                key.value += dv;
            }
            Ok(())
        })
    }

    /// GE-71 Set Value (set / add / multiply) on the selected keys.
    pub fn set_key_values(&mut self, value: f64, mode: ValueMode) -> Result<()> {
        if !value.is_finite() {
            return Err("Give a number".into());
        }
        self.edit_selected_keys("Set Value", |_, key| {
            key.value = match mode {
                ValueMode::Set => value,
                ValueMode::Add => key.value + value,
                ValueMode::Multiply => key.value * value,
            };
            Ok(())
        })
    }

    /// GE-40/43: the selected keys' values scaled by `factor` around
    /// `pivot` (a value).
    pub fn scale_key_values(&mut self, factor: f64, pivot: f64) -> Result<()> {
        if !factor.is_finite() {
            return Err("Give a factor".into());
        }
        self.edit_selected_keys("Scale Values", |_, key| {
            key.value = pivot + (key.value - pivot) * factor;
            Ok(())
        })
    }

    /// GE-71 Set Frame: the selected keys to `t` (or by `t` with
    /// `relative`). With one key per channel each lands on `t`.
    pub fn set_key_frames(&mut self, t: Tick, relative: bool) -> Result<()> {
        let snap = self.scene.animation.settings.snap;
        let rate = self.scene.time.rate;
        let at = |x: Tick| {
            if snap {
                animation::nearest_frame(rate, x)
            } else {
                x
            }
        };
        if relative {
            return self.move_selected_keys(t);
        }
        let selected = self.live_selected_keys();
        let first = selected
            .iter()
            .map(|(_, t)| *t)
            .min()
            .ok_or("Select keys first")?;
        // Absolute: the earliest selected key goes to `t`, the rest keep
        // their spacing.
        self.move_selected_keys(at(t) - first)
    }

    /// GE-50/53: the selected keys' handles. `slope` (value per second)
    /// and `weight` set that side Manual; unified keys' other side follows
    /// the slope.
    pub fn set_key_handles(
        &mut self,
        incoming: bool,
        slope: Option<f64>,
        weight: Option<f64>,
    ) -> Result<()> {
        if slope.is_some_and(|s| !s.is_finite())
            || weight.is_some_and(|w| !(0.01..=3.).contains(&w))
        {
            return Err("A handle's weight is 0.01 to 3, its slope a number".into());
        }
        let label = if incoming { "In Handle" } else { "Out Handle" };
        let snapshot = self.scene.animation.clone();
        self.edit_selected_keys(label, |k, key| {
            // Today's slopes, so a weight change alone keeps the angle.
            let now = |incoming: bool| -> f64 {
                snapshot
                    .channels
                    .get(k)
                    .and_then(|c| c.curve.key_at(key.time).map(|i| c.curve.slope(i, incoming)))
                    .unwrap_or(0.)
            };
            let (now_in, now_out) = (now(true), now(false));
            if let Some(s) = slope {
                if incoming || !key.broken {
                    key.slope_in = Slope::Manual;
                    key.manual_in = s;
                }
                if !incoming || !key.broken {
                    key.slope_out = Slope::Manual;
                    key.manual_out = s;
                }
            }
            if let Some(w) = weight {
                if incoming {
                    key.weight_in = w;
                } else {
                    key.weight_out = w;
                }
                // A weight makes that side Manual at its current angle.
                if incoming && key.slope_in != Slope::Manual {
                    key.slope_in = Slope::Manual;
                    key.manual_in = now_in;
                }
                if !incoming && key.slope_out != Slope::Manual {
                    key.slope_out = Slope::Manual;
                    key.manual_out = now_out;
                }
            }
            Ok(())
        })
    }

    /// GE-51 Break / Unify. Unify lines the out side up with the in side.
    pub fn set_keys_broken(&mut self, broken: bool) -> Result<()> {
        let snapshot = self.scene.animation.clone();
        let label = if broken {
            "Break Handles"
        } else {
            "Unify Handles"
        };
        self.edit_selected_keys(label, |k, key| {
            key.broken = broken;
            if !broken && key.slope_in != key.slope_out {
                let slope = snapshot
                    .channels
                    .get(k)
                    .and_then(|c| c.curve.key_at(key.time).map(|i| c.curve.slope(i, true)))
                    .unwrap_or(0.);
                key.slope_in = Slope::Manual;
                key.slope_out = Slope::Manual;
                key.manual_in = slope;
                key.manual_out = slope;
            }
            Ok(())
        })
    }

    /// GE-52: weights back to Auto (1) on the selected keys; with `auto
    /// slopes` (Shift+double-click, GE-51) the slopes go back to Auto too.
    pub fn reset_key_handles(&mut self, slopes: bool) -> Result<()> {
        self.edit_selected_keys("Auto Handles", |_, key| {
            key.weight_in = 1.;
            key.weight_out = 1.;
            if slopes {
                key.slope_in = Slope::Auto;
                key.slope_out = Slope::Auto;
                key.broken = false;
            }
            Ok(())
        })
    }

    /// GE-31: a key on `channel` at `t` with `value` (the curve's own value
    /// there when `None`).
    pub fn add_key_at(&mut self, channel: &ChannelKey, t: Tick, value: Option<f64>) -> Result<()> {
        if self.scene.item(channel.item).is_none() {
            return Err(format!("No item {}", channel.item));
        }
        let now = animation::read(&self.scene, channel).ok_or("That channel has no value")?;
        if channel.transform_part().is_none()
            && !animation::keyable(&self.scene, channel.item, &channel.channel)
        {
            return Err(format!("{} can't hold keys", channel.channel));
        }
        let rate = self.scene.time.rate;
        let t = if self.scene.animation.settings.snap {
            animation::nearest_frame(rate, t)
        } else {
            t
        };
        let stepped = animation::is_stepped(&self.scene, channel);
        let channel = channel.clone();
        self.anim_step("Add Key", move |e| {
            let c = e
                .scene
                .animation
                .channels
                .entry(channel.clone())
                .or_insert_with(|| AnimChannel {
                    curve: Curve {
                        keys: Vec::new(),
                        stepped,
                        ..Default::default()
                    },
                    rest: now,
                });
            let v = value.unwrap_or_else(|| {
                if c.curve.is_empty() {
                    now
                } else {
                    c.curve.value_at(t)
                }
            });
            c.curve.set(t, v);
            animation::repose(&mut e.scene);
            Ok(true)
        })
    }

    /// GE-32 Key Keeping Shape: keys on `channels` at `t` that leave each
    /// curve exactly as it was (the new key and its neighbours take their
    /// present slopes as Manual handles; a cubic split this way is the
    /// same cubic).
    pub fn key_keeping_shape(&mut self, channels: &[ChannelKey], t: Tick) -> Result<()> {
        let list = channels.to_vec();
        if list.is_empty() {
            return Err("Select channels first".into());
        }
        self.anim_step("Key Keeping Shape", move |e| {
            let mut changed = false;
            for k in &list {
                let Some(c) = e.scene.animation.channels.get_mut(k) else {
                    continue;
                };
                if c.curve.key_at(t).is_some() || c.curve.is_empty() {
                    continue;
                }
                let first = c.curve.keys[0].time;
                let last = c.curve.keys[c.curve.keys.len() - 1].time;
                if t < first || t > last {
                    // Outside the keys the curve is flat (Constant) or a
                    // behaviour; a plain key there keeps a Constant shape.
                    let v = c.curve.value_at(t);
                    c.curve.set(t, v);
                    changed = true;
                    continue;
                }
                let snapshot = c.curve.clone();
                // Neighbours hold today's slopes.
                let i = snapshot
                    .keys
                    .iter()
                    .position(|key| key.time > t)
                    .unwrap_or(0);
                for j in [i - 1, i] {
                    let (s_in, s_out) = (snapshot.slope(j, true), snapshot.slope(j, false));
                    let key = &mut c.curve.keys[j];
                    if key.slope_in != Slope::Stepped {
                        key.slope_in = Slope::Manual;
                        key.manual_in = s_in;
                    }
                    if key.slope_out != Slope::Stepped {
                        key.slope_out = Slope::Manual;
                        key.manual_out = s_out;
                    }
                }
                let slope = snapshot.derivative(t);
                let mut key = Key::new(t, snapshot.value_at(t));
                key.slope_in = Slope::Manual;
                key.slope_out = Slope::Manual;
                key.manual_in = slope;
                key.manual_out = slope;
                if snapshot.keys[i - 1].slope_out == Slope::Stepped {
                    key.slope_out = Slope::Stepped;
                }
                c.curve.put(key);
                changed = true;
            }
            animation::repose(&mut e.scene);
            Ok(changed)
        })
    }

    /// Applies `edit` to the curves of `channels` (one undo step).
    fn edit_curves(
        &mut self,
        label: &str,
        channels: Vec<ChannelKey>,
        edit: impl Fn(&mut Curve) -> Result<()>,
    ) -> Result<()> {
        if channels.is_empty() {
            return Err("Select animated channels or keys first".into());
        }
        self.anim_step(label, |e| {
            let mut changed = false;
            for k in &channels {
                if let Some(c) = e.scene.animation.channels.get_mut(k) {
                    let before = c.curve.clone();
                    edit(&mut c.curve)?;
                    changed |= c.curve != before;
                }
            }
            animation::repose(&mut e.scene);
            Ok(changed)
        })
    }

    /// GE-61: pre and/or post behaviour.
    pub fn set_curve_behavior(
        &mut self,
        channels: Vec<ChannelKey>,
        pre: Option<Behavior>,
        post: Option<Behavior>,
    ) -> Result<()> {
        self.edit_curves("Curve Behavior", channels, |c| {
            if let Some(b) = pre {
                c.pre = b;
            }
            if let Some(b) = post {
                c.post = b;
            }
            Ok(())
        })
    }

    /// GE-60: every key's slopes for the interpolation.
    pub fn set_curve_interpolation(
        &mut self,
        channels: Vec<ChannelKey>,
        kind: Interpolation,
    ) -> Result<()> {
        let slope = match kind {
            Interpolation::Curve => Slope::SmoothFlat,
            Interpolation::Linear => Slope::Linear,
            Interpolation::Stepped => Slope::Stepped,
        };
        self.edit_curves("Interpolation", channels, |c| {
            for k in &mut c.keys {
                k.slope_in = if slope == Slope::Stepped {
                    Slope::Flat
                } else {
                    slope
                };
                k.slope_out = slope;
            }
            Ok(())
        })
    }

    /// GE-62: the cycle option.
    pub fn set_curve_cycle(&mut self, channels: Vec<ChannelKey>, on: bool) -> Result<()> {
        self.edit_curves(if on { "Cycle On" } else { "Cycle Off" }, channels, |c| {
            c.cycle = on;
            Ok(())
        })
    }

    /// GE-80: Copy to Shadow, Replace with Shadow, Swap, Clear.
    pub fn shadow(&mut self, channels: Vec<ChannelKey>, action: ShadowAction) -> Result<()> {
        if channels.is_empty() {
            return Err("Select animated channels first".into());
        }
        let label = match action {
            ShadowAction::Copy => "Copy to Shadow",
            ShadowAction::Replace => "Replace with Shadow",
            ShadowAction::Swap => "Swap with Shadow",
            ShadowAction::Clear => "Clear Shadow",
        };
        self.anim_step(label, |e| {
            let anim = &mut e.scene.animation;
            let mut changed = false;
            for k in &channels {
                match action {
                    ShadowAction::Copy => {
                        if let Some(c) = anim.channels.get(k) {
                            changed |= anim.shadows.insert(k.clone(), c.curve.clone()).as_ref()
                                != Some(&c.curve);
                        }
                    }
                    ShadowAction::Clear => changed |= anim.shadows.remove(k).is_some(),
                    ShadowAction::Replace | ShadowAction::Swap => {
                        let Some(shadow) = anim.shadows.get(k).cloned() else {
                            continue;
                        };
                        if let Some(c) = anim.channels.get_mut(k) {
                            let old = std::mem::replace(&mut c.curve, shadow);
                            changed |= old != c.curve;
                            if action == ShadowAction::Swap {
                                anim.shadows.insert(k.clone(), old);
                            }
                        }
                    }
                }
            }
            if !changed && action != ShadowAction::Copy {
                return Err("No shadow to use: Copy to Shadow first".into());
            }
            animation::repose(&mut e.scene);
            Ok(changed)
        })
    }

    /// GE-X1 Euler filter: rotation curves that jump by a whole turn are
    /// unwrapped (each key moves by whole turns to the nearest of its
    /// neighbour).
    pub fn euler_filter(&mut self, channels: Vec<ChannelKey>) -> Result<()> {
        let rot: Vec<ChannelKey> = channels
            .into_iter()
            .filter(|k| k.transform_part().is_some_and(|(g, _)| g == "rot"))
            .collect();
        if rot.is_empty() {
            return Err("Select rotation channels (or their keys) first".into());
        }
        self.edit_curves("Euler Filter", rot, |c| {
            let turn = std::f64::consts::TAU;
            for i in 1..c.keys.len() {
                let prev = c.keys[i - 1].value;
                let v = &mut c.keys[i].value;
                *v -= ((*v - prev) / turn).round() * turn;
            }
            Ok(())
        })
    }

    /// GE-X2 Simplify: keys whose removal moves the curve by no more than
    /// `tolerance` anywhere (checked each frame) go.
    pub fn simplify_curves(&mut self, channels: Vec<ChannelKey>, tolerance: f64) -> Result<()> {
        if !(tolerance.is_finite() && tolerance >= 0.) {
            return Err("tolerance is 0 or more".into());
        }
        let rate = self.scene.time.rate;
        self.edit_curves("Simplify", channels, |c| {
            let (Some(a), Some(b)) = (
                c.keys.first().map(|k| k.time),
                c.keys.last().map(|k| k.time),
            ) else {
                return Ok(());
            };
            let samples: Vec<Tick> = (rate.frame_at(a)..=rate.frame_at(b))
                .map(|f| rate.tick_of(f))
                .chain(c.keys.iter().map(|k| k.time))
                .collect();
            let reference = c.clone();
            let mut i = 1;
            while i + 1 < c.keys.len() {
                let mut trial = c.clone();
                trial.keys.remove(i);
                let ok = samples
                    .iter()
                    .all(|t| (trial.value_at(*t) - reference.value_at(*t)).abs() <= tolerance);
                if ok {
                    *c = trial;
                } else {
                    i += 1;
                }
            }
            Ok(())
        })
    }
}
