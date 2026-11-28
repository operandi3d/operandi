//! Animation edits on the editor (TK-05, TK-20..43, TL-13/14/20/21,
//! TL-41..47, TL-50..54): setting the time, keying, the track row's key
//! edits, the key clipboard, ranges and markers.
//!
//! Every edit here is one named command and one undo step (PLAN "Command
//! history and undo"); changing the current time or the key time is not
//! (UIC-225 class UI). A live Move/Rotate/Scale/Transform is dropped
//! before a keying step and started again after, so its session and the
//! key are separate, ordered undo steps.
use crate::anim_curve::{Curve, Key, Slope};
use crate::animation::{
    self, AnimChannel, AutoKey, ChannelKey, Marker, TRANSFORM_CHANNELS, TimeSettings,
};
use crate::scene_time::SceneTime;
use crate::time_tick::Tick;
use crate::{Command, Editor, Result, Tool};
use std::cell::RefCell;
use std::collections::BTreeSet;

/// Which key a key jump goes to (TK-63).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyJump {
    Previous,
    Next,
    First,
    Last,
}

/// How Paste treats keys already there (TL-45).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasteMode {
    /// Pasted keys replace keys at the same times.
    Merge,
    /// Later keys move along by the pasted span first.
    Insert,
    /// Keys inside the pasted span go first.
    Replace,
}

/// Which side(s) of the selected keys a slope applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    In,
    Out,
    Both,
}

/// Copied keys (TL-47): per app, plain data, never an undo record. Times
/// are relative to the earliest copied key.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyClip {
    pub channels: Vec<(ChannelKey, Vec<Key>)>,
}

// The app edits every scene tab on its one UI thread (and the MCP bridge
// calls arrive there too), so one clipboard per thread is one per app.
thread_local! {
    static CLIPBOARD: RefCell<Option<KeyClip>> = const { RefCell::new(None) };
}

/// The key clipboard's contents.
pub fn clipboard() -> Option<KeyClip> {
    CLIPBOARD.with(|c| c.borrow().clone())
}
fn set_clipboard(clip: KeyClip) {
    CLIPBOARD.with(|c| *c.borrow_mut() = Some(clip));
}

/// The channels a transform tool keys (TK-21/25).
pub fn tool_channels(tool: Tool) -> &'static [&'static str] {
    match tool {
        Tool::Move => &TRANSFORM_CHANNELS[0..3],
        Tool::Rotate => &TRANSFORM_CHANNELS[3..6],
        Tool::Scale => &TRANSFORM_CHANNELS[6..9],
        Tool::Transform => &TRANSFORM_CHANNELS,
        _ => &[],
    }
}

impl Editor {
    // ------------------------------------------------------------ steps

    /// Drops a live item transform tool, returning it to start again.
    pub(crate) fn settle_tool(&mut self) -> Option<Tool> {
        let tool = self.session.live.as_ref().map(|l| l.tool)?;
        if !tool.is_transform() {
            return None;
        }
        self.dispatch(Command::Drop).ok()?;
        Some(tool)
    }
    pub(crate) fn resume_tool(&mut self, tool: Option<Tool>) {
        if let Some(tool) = tool
            && self.session.live.is_none()
        {
            let _ = self.dispatch(Command::Activate(tool));
        }
    }

    /// Runs `edit` as one undo step named `label`. `edit` says whether it
    /// changed anything; an error leaves the scene as it was.
    pub(crate) fn anim_step(
        &mut self,
        label: &str,
        edit: impl FnOnce(&mut Editor) -> Result<bool>,
    ) -> Result<()> {
        let tool = self.settle_tool();
        let before = self.scene.clone();
        self.anim.quiet = true;
        let result = edit(self);
        match result {
            Ok(true) => {
                // Setup mode: what the step changed becomes setup values.
                if self.scene.animation.setup {
                    crate::animation::setup_edit(&before, &mut self.scene);
                }
                if let Err(e) = self.scene.validate() {
                    self.scene = before;
                    self.anim.quiet = false;
                    self.resume_tool(tool);
                    return Err(e);
                }
                self.record(label.into(), before);
            }
            Ok(false) => {}
            Err(e) => {
                self.scene = before;
                self.anim.quiet = false;
                self.resume_tool(tool);
                return Err(e);
            }
        }
        self.anim.quiet = false;
        self.resume_tool(tool);
        Ok(())
    }

    // ------------------------------------------------------------ time

    /// TK-01/05: the current time, everywhere at once; not an undo step.
    /// With Time Snap on it lands on a whole frame. Pending edits are
    /// dropped (the scene is posed at the new time, TK-40), and a live
    /// tool's base scene is posed too so it carries on from the new pose.
    /// The key time follows (TL-20) unless `keep_key_time` (playback).
    pub fn set_current_time(&mut self, t: Tick, keep_key_time: bool) -> Result<()> {
        if t.seconds().abs() > crate::scene_time::MAX_SECONDS {
            return Err("Time is out of range".into());
        }
        let t = if self.scene.animation.settings.snap {
            animation::nearest_frame(self.scene.time.rate, t)
        } else {
            t
        };
        if !keep_key_time {
            self.anim.key_time = None;
        }
        self.scene.time.current = t;
        animation::pose(&mut self.scene, t);
        if let Some(live) = self.session.live.as_mut() {
            live.base.time.current = t;
            animation::pose(&mut live.base, t);
        }
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    /// TL-20: the key time alone (a middle-click in the ruler); `None`
    /// follows the current time again.
    pub fn set_key_time(&mut self, t: Option<Tick>) {
        let snap = self.scene.animation.settings.snap;
        let rate = self.scene.time.rate;
        self.anim.key_time = t
            .map(|t| {
                if snap {
                    animation::nearest_frame(rate, t)
                } else {
                    t
                }
            })
            .filter(|t| *t != self.scene.time.current);
    }

    /// TK-63: the time `n` frames on (wrapping in a Frame Limits range).
    pub fn step_time(&mut self, n: i64) -> Result<()> {
        let t = animation::step_frames(&self.scene, self.scene.time.current, n);
        self.set_current_time(t, false)
    }

    /// TK-63: the first or last frame of the current range.
    pub fn go_to_range_end(&mut self, last: bool) -> Result<()> {
        let (a, b) = animation::current_range(&self.scene);
        self.set_current_time(if last { b } else { a }, false)
    }

    /// The items or channels key jumps and ruler ticks follow (TK-63,
    /// TL-12): the selected channels if any, else the selected items'.
    pub fn key_scope(&self) -> Vec<ChannelKey> {
        let anim = &self.scene.animation;
        let chosen: Vec<ChannelKey> = self
            .anim
            .selected_channels
            .iter()
            .filter(|k| anim.is_animated(k))
            .cloned()
            .collect();
        if !chosen.is_empty() {
            return chosen;
        }
        let items = self.scene.selected_items();
        anim.channels
            .iter()
            .filter(|(k, c)| items.contains(&k.item) && !c.curve.is_empty())
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Key times of `key_scope`, sorted.
    pub fn scope_key_times(&self) -> Vec<Tick> {
        let mut out: Vec<Tick> = self
            .key_scope()
            .iter()
            .filter_map(|k| self.scene.animation.channels.get(k))
            .flat_map(|c| c.curve.times())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// The key a jump goes to from the current time, if any.
    pub fn key_jump_target(&self, jump: KeyJump) -> Option<Tick> {
        let times = self.scope_key_times();
        let now = self.scene.time.current;
        match jump {
            KeyJump::First => times.first().copied(),
            KeyJump::Last => times.last().copied(),
            KeyJump::Previous => times.iter().rev().find(|t| **t < now).copied(),
            KeyJump::Next => times.iter().find(|t| **t > now).copied(),
        }
    }

    /// TK-63: jump to a key of the selected items or channels.
    pub fn jump_to_key(&mut self, jump: KeyJump) -> Result<()> {
        let t = self
            .key_jump_target(jump)
            .ok_or("No key there for the selected items or channels")?;
        self.set_current_time(t, false)
    }

    // ------------------------------------------------------------ keying

    /// Keys `channels` with their current values (TK-20..23) at the key
    /// time (TL-20: values are read at the current time). One undo step.
    pub fn key_channels(&mut self, channels: &[ChannelKey], label: &str) -> Result<usize> {
        if channels.is_empty() {
            return Err("Nothing to key".into());
        }
        for k in channels {
            if !self.scene.owns_channels(k.item) {
                return Err(format!("No item {}", k.item));
            }
            if animation::read(&self.scene, k).is_none()
                || (k.transform_part().is_none()
                    && !animation::keyable(&self.scene, k.item, &k.channel))
            {
                return Err(format!("{} can't hold keys", k.channel));
            }
        }
        let list = channels.to_vec();
        let count = list.len();
        self.anim_step(label, move |e| {
            let at = e.key_time();
            let mut changed = false;
            for k in &list {
                let Some(v) = animation::read(&e.scene, k) else {
                    continue;
                };
                let stepped = animation::is_stepped(&e.scene, k);
                let c = e
                    .scene
                    .animation
                    .channels
                    .entry(k.clone())
                    .or_insert_with(|| AnimChannel {
                        curve: Curve {
                            keys: Vec::new(),
                            stepped,
                            ..Default::default()
                        },
                        rest: v,
                    });
                let had = c.curve.key_at(at).map(|i| c.curve.keys[i].value);
                if had != Some(v) {
                    c.curve.set(at, v);
                    changed = true;
                }
            }
            Ok(changed)
        })?;
        Ok(count)
    }

    /// Removes the keys of `channels` at `t` (a status-light click on a
    /// key, Remove Key). A channel left with no keys keeps its value.
    pub fn remove_keys_at(&mut self, channels: &[ChannelKey], t: Tick) -> Result<()> {
        let list = channels.to_vec();
        self.anim_step("Remove Key", move |e| {
            let mut changed = false;
            for k in &list {
                if let Some(c) = e.scene.animation.channels.get_mut(k) {
                    changed |= c.curve.remove(t);
                    if c.curve.is_empty() {
                        e.scene.animation.channels.remove(k);
                    }
                }
            }
            if !changed {
                return Err("No key there".into());
            }
            animation::repose(&mut e.scene);
            Ok(true)
        })
    }

    /// TK-20: a status-light click: key the channels, or remove their keys
    /// when all of them have one at the key time.
    pub fn toggle_keys(&mut self, channels: &[ChannelKey]) -> Result<()> {
        let at = self.key_time();
        let all_keyed = !channels.is_empty()
            && channels.iter().all(|k| {
                self.scene
                    .animation
                    .channels
                    .get(k)
                    .is_some_and(|c| c.curve.key_at(at).is_some())
                    && !animation::is_pending(&self.scene, k)
            });
        if all_keyed {
            self.remove_keys_at(channels, at)
        } else {
            self.key_channels(channels, "Set Key").map(|_| ())
        }
    }

    /// TK-21 `S`: keys every selected channel; with none selected and a
    /// transform tool live, that tool's channels of the selected items.
    pub fn key_selected_channels(&mut self) -> Result<usize> {
        let mut list: Vec<ChannelKey> = self
            .anim
            .selected_channels
            .iter()
            .filter(|k| self.scene.owns_channels(k.item))
            .cloned()
            .collect();
        if list.is_empty()
            && let Some(tool) = self.session.live.as_ref().map(|l| l.tool)
        {
            for id in self.scene.selected_items() {
                for name in tool_channels(tool) {
                    list.push(ChannelKey::new(id, name));
                }
            }
        }
        if list.is_empty() {
            return Err("Select channels to key (click a property's label), or use Shift+W/E/R/Y to key transforms".into());
        }
        self.key_channels(&list, "Key Selected Channels")
    }

    /// TK-22: keys position, rotation and/or scale of the selected items.
    pub fn key_transforms(&mut self, groups: &[&str]) -> Result<usize> {
        let items = self.scene.selected_items();
        if items.is_empty() {
            return Err("Select items to key".into());
        }
        let list: Vec<ChannelKey> = items
            .iter()
            .filter(|id| {
                self.scene
                    .item(**id)
                    .is_some_and(|i| i.environment.is_none())
            })
            .flat_map(|id| {
                TRANSFORM_CHANNELS
                    .iter()
                    .filter(|n| groups.iter().any(|g| n.starts_with(g)))
                    .map(move |n| ChannelKey::new(*id, n))
            })
            .collect();
        let label = match groups {
            ["pos"] => "Key Position",
            ["rot"] => "Key Rotation",
            ["scl"] => "Key Scale",
            _ => "Key Transforms",
        };
        self.key_channels(&list, label)
    }

    /// TK-23 Shift+S: keys every already-animated channel of the selected
    /// items.
    pub fn key_selected_items(&mut self) -> Result<usize> {
        let items = self.scene.selected_items();
        if items.is_empty() {
            return Err("Select items to key".into());
        }
        let list: Vec<ChannelKey> = self
            .scene
            .animation
            .channels
            .keys()
            .filter(|k| items.contains(&k.item))
            .cloned()
            .collect();
        if list.is_empty() {
            return Err("The selected items have no keys yet; key a channel first (S, Shift+W/E/R/Y or a status light)".into());
        }
        self.key_channels(&list, "Key Selected Items")
    }

    /// TK-24 Remove Animation (keep the value) or Reset (back to the
    /// default value).
    pub fn remove_animation(&mut self, channels: &[ChannelKey], reset: bool) -> Result<()> {
        let list = channels.to_vec();
        let label = if reset {
            "Reset Channel"
        } else {
            "Remove Animation"
        };
        self.anim_step(label, move |e| {
            let mut changed = false;
            for k in &list {
                let default = animation::default_value(&e.scene, k);
                changed |= e.scene.animation.channels.remove(k).is_some();
                if reset {
                    changed |= animation::read(&e.scene, k)
                        .is_some_and(|v| animation::differs(v, default));
                    animation::write(&mut e.scene, k, default)?;
                }
            }
            Ok(changed)
        })
    }

    /// TK-24 Bake: a key on every frame of the current range.
    pub fn bake_channels(&mut self, channels: &[ChannelKey]) -> Result<()> {
        let list = channels.to_vec();
        self.anim_step("Bake", move |e| {
            let rate = e.scene.time.rate;
            let (a, b) = animation::current_range(&e.scene);
            let (fa, fb) = (
                animation::frames(rate, a).ceil() as i64,
                animation::frames(rate, b).floor() as i64,
            );
            if fb - fa > 100_000 {
                return Err("The range is too long to bake".into());
            }
            for k in &list {
                let Some(now) = animation::read(&e.scene, k) else {
                    continue;
                };
                let stepped = animation::is_stepped(&e.scene, k);
                let source = e.scene.animation.channels.get(k).cloned();
                let mut curve = Curve {
                    keys: Vec::new(),
                    stepped,
                    ..Default::default()
                };
                for f in fa..=fb {
                    let t = rate.tick_of(f);
                    let v = source.as_ref().map_or(now, |c| c.curve.value_at(t));
                    curve.set(t, v);
                }
                let rest = source.map_or(now, |c| c.rest);
                e.scene
                    .animation
                    .channels
                    .insert(k.clone(), AnimChannel { curve, rest });
            }
            Ok(true)
        })
    }

    /// TK-41 Apply: keys every pending edit at the key time.
    pub fn apply_pending(&mut self) -> Result<()> {
        let pending = animation::pending(&self.scene);
        if pending.is_empty() {
            return Err("No edits waiting to be keyed".into());
        }
        self.key_channels(&pending, "Apply Edits").map(|_| ())
    }

    /// TK-41 Discard: pending edits go back to their curves.
    pub fn discard_pending(&mut self) -> Result<()> {
        if animation::pending(&self.scene).is_empty() {
            return Err("No edits waiting to be keyed".into());
        }
        self.anim_step("Discard Edits", |e| {
            animation::repose(&mut e.scene);
            Ok(true)
        })
    }

    /// UIC-119: picks (or with `add`, toggles) a channel for `S`.
    pub fn select_channel(&mut self, key: Option<ChannelKey>, add: bool) {
        if !add {
            self.anim.selected_channels.clear();
        }
        if let Some(k) = key
            && !self.anim.selected_channels.remove(&k)
        {
            self.anim.selected_channels.insert(k);
        }
    }

    // ------------------------------------------------------------ key edits

    /// The selected keys that still exist.
    pub fn live_selected_keys(&self) -> Vec<(ChannelKey, Tick)> {
        self.anim
            .selected_keys
            .iter()
            .filter(|(k, t)| {
                self.scene
                    .animation
                    .channels
                    .get(k)
                    .is_some_and(|c| c.curve.key_at(*t).is_some())
            })
            .cloned()
            .collect()
    }

    /// TL-41: replaces (or adds to / takes from) the key selection.
    pub fn select_keys(&mut self, keys: Vec<(ChannelKey, Tick)>, how: KeySelect) {
        match how {
            KeySelect::Replace => {
                self.anim.selected_keys = keys.into_iter().collect();
            }
            KeySelect::Add => self.anim.selected_keys.extend(keys),
            KeySelect::Remove => {
                for k in keys {
                    self.anim.selected_keys.remove(&k);
                }
            }
        }
    }

    /// Changes the selected keys' times with `f`, as one undo step; the
    /// selection follows. Moved keys replace keys already at their new
    /// times.
    fn retime_selected(&mut self, label: &str, f: impl Fn(Tick) -> Tick) -> Result<()> {
        let selected = self.live_selected_keys();
        if selected.is_empty() {
            return Err("Select keys first".into());
        }
        let snap = self.scene.animation.settings.snap;
        let rate = self.scene.time.rate;
        let map = |t: Tick| {
            let n = f(t);
            if snap {
                animation::nearest_frame(rate, n)
            } else {
                n
            }
        };
        let mut moved: Vec<(ChannelKey, Tick)> = Vec::new();
        self.anim_step(label, |e| {
            let mut changed = false;
            let by_channel: std::collections::BTreeMap<ChannelKey, Vec<Tick>> =
                selected.iter().fold(Default::default(), |mut m, (k, t)| {
                    m.entry(k.clone()).or_default().push(*t);
                    m
                });
            for (k, times) in by_channel {
                let Some(c) = e.scene.animation.channels.get_mut(&k) else {
                    continue;
                };
                let (mut taken, kept): (Vec<Key>, Vec<Key>) = c
                    .curve
                    .keys
                    .drain(..)
                    .partition(|key| times.contains(&key.time));
                for key in &mut taken {
                    let to = map(key.time);
                    changed |= to != key.time;
                    key.time = to;
                    moved.push((k.clone(), to));
                }
                // Moved keys win over the ones they land on.
                c.curve.keys = kept;
                c.curve.sort();
                for key in taken {
                    c.curve.put(key);
                }
            }
            if changed {
                animation::repose(&mut e.scene);
            }
            Ok(changed)
        })?;
        self.anim.selected_keys = moved.into_iter().collect();
        Ok(())
    }

    /// TL-42 / Shift…: the selected keys move by `by`.
    pub fn move_selected_keys(&mut self, by: Tick) -> Result<()> {
        self.retime_selected("Move Keys", |t| t + by)
    }

    /// TL-43 / Scale…: the selected keys' timing scaled by `factor` around
    /// `pivot`. Needs two or more keys.
    pub fn scale_selected_keys(&mut self, factor: f64, pivot: Tick) -> Result<()> {
        if !(factor.is_finite() && factor > 0.) {
            return Err("Scale needs a factor above 0".into());
        }
        if self.live_selected_keys().len() < 2 {
            return Err("Select two or more keys to scale their timing".into());
        }
        self.retime_selected("Scale Keys", |t| {
            Tick::from_seconds(pivot.seconds() + (t.seconds() - pivot.seconds()) * factor)
        })
    }

    /// TL-45 Invert…: the selected keys' timing reversed within their span.
    pub fn invert_selected_keys(&mut self) -> Result<()> {
        let sel = self.live_selected_keys();
        let (Some(a), Some(b)) = (
            sel.iter().map(|(_, t)| *t).min(),
            sel.iter().map(|(_, t)| *t).max(),
        ) else {
            return Err("Select keys first".into());
        };
        self.retime_selected("Invert Keys", |t| Tick(a.0 + b.0 - t.0))
    }

    /// TL-45 Delete (and Delete Related: the sibling X/Y/Z keys at the
    /// same times).
    pub fn delete_selected_keys(&mut self, related: bool) -> Result<()> {
        let mut selected = self.live_selected_keys();
        if selected.is_empty() {
            return Err("Select keys first".into());
        }
        if related {
            let extra: Vec<(ChannelKey, Tick)> = selected
                .iter()
                .filter_map(|(k, t)| {
                    let (group, _) = k.transform_part()?;
                    Some(
                        ["X", "Y", "Z"]
                            .map(|a| (ChannelKey::new(k.item, &format!("{group}.{a}")), *t)),
                    )
                })
                .flatten()
                .collect();
            selected.extend(extra);
        }
        let label = if related {
            "Delete Related Keys"
        } else {
            "Delete Keys"
        };
        self.anim_step(label, |e| {
            let mut changed = false;
            for (k, t) in &selected {
                if let Some(c) = e.scene.animation.channels.get_mut(k) {
                    changed |= c.curve.remove(*t);
                    if c.curve.is_empty() {
                        e.scene.animation.channels.remove(k);
                    }
                }
            }
            animation::repose(&mut e.scene);
            Ok(changed)
        })?;
        self.anim.selected_keys.clear();
        Ok(())
    }

    /// TL-45/46: a slope type for the selected keys (`side` picks Linear
    /// In / Out; other types set both sides).
    pub fn set_selected_slopes(&mut self, slope: Slope, side: Side) -> Result<()> {
        let selected = self.live_selected_keys();
        if selected.is_empty() {
            return Err("Select keys first".into());
        }
        let label = match (slope, side) {
            (Slope::Linear, Side::In) => "Linear In".to_string(),
            (Slope::Linear, Side::Out) => "Linear Out".to_string(),
            _ => format!("Slope {}", slope.name()),
        };
        self.anim_step(&label, |e| {
            let mut changed = false;
            for (k, t) in &selected {
                let Some(c) = e.scene.animation.channels.get_mut(k) else {
                    continue;
                };
                let Some(i) = c.curve.key_at(*t) else {
                    continue;
                };
                // Manual keeps today's shape: the handle is set to the
                // slope the key has now.
                let (now_in, now_out) = if slope == Slope::Manual {
                    let eps = Tick(1);
                    let key = &c.curve.keys[i];
                    let rate = |a: Tick, b: Tick| {
                        (c.curve.value_at(b) - c.curve.value_at(a)) / (b - a).seconds()
                    };
                    (
                        rate(key.time - eps, key.time),
                        rate(key.time, key.time + eps),
                    )
                } else {
                    (0., 0.)
                };
                let key = &mut c.curve.keys[i];
                let before = key.clone();
                if matches!(side, Side::In | Side::Both) {
                    key.slope_in = slope;
                    key.manual_in = now_in;
                }
                if matches!(side, Side::Out | Side::Both) {
                    key.slope_out = slope;
                    key.manual_out = now_out;
                }
                changed |= *key != before;
            }
            animation::repose(&mut e.scene);
            Ok(changed)
        })
    }

    /// TL-45 Copy: the selected keys (or, with none, every key of the
    /// selected items at the key time) to the key clipboard.
    pub fn copy_keys(&mut self) -> Result<usize> {
        let mut selected = self.live_selected_keys();
        if selected.is_empty() {
            let at = self.key_time();
            selected = self
                .key_scope()
                .into_iter()
                .filter(|k| self.scene.animation.channels[k].curve.key_at(at).is_some())
                .map(|k| (k, at))
                .collect();
        }
        if selected.is_empty() {
            return Err("Select keys to copy".into());
        }
        let start = selected.iter().map(|(_, t)| *t).min().unwrap_or_default();
        let mut clip = KeyClip::default();
        let mut by: std::collections::BTreeMap<ChannelKey, Vec<Key>> = Default::default();
        for (k, t) in &selected {
            let c = &self.scene.animation.channels[k];
            if let Some(i) = c.curve.key_at(*t) {
                let mut key = c.curve.keys[i].clone();
                key.time = key.time - start;
                by.entry(k.clone()).or_default().push(key);
            }
        }
        let n = by.values().map(Vec::len).sum();
        clip.channels = by.into_iter().collect();
        set_clipboard(clip);
        Ok(n)
    }

    /// TL-45 Cut: Copy, then Delete.
    pub fn cut_keys(&mut self) -> Result<()> {
        if self.live_selected_keys().is_empty() {
            return Err("Select keys to cut".into());
        }
        self.copy_keys()?;
        self.delete_selected_keys(false)
    }

    /// TL-45/47 Paste at the key time. Keys go back to the channels they
    /// came from; copied from one item into a scene where other items are
    /// selected, they go to the first selected item's same channels.
    pub fn paste_keys(&mut self, mode: PasteMode) -> Result<()> {
        let clip = clipboard().ok_or("Copy keys first")?;
        let at = self.key_time();
        let selected = self.scene.selected_items();
        let sources: BTreeSet<u64> = clip.channels.iter().map(|(k, _)| k.item).collect();
        let retarget = sources.len() == 1
            && !selected.is_empty()
            && !selected.iter().any(|id| sources.contains(id));
        let target = |k: &ChannelKey| {
            if retarget {
                ChannelKey::new(selected[0], &k.channel)
            } else {
                k.clone()
            }
        };
        let span = clip
            .channels
            .iter()
            .flat_map(|(_, keys)| keys.iter().map(|k| k.time))
            .max()
            .unwrap_or_default();
        let label = match mode {
            PasteMode::Merge => "Paste Keys",
            PasteMode::Insert => "Paste Insert",
            PasteMode::Replace => "Paste Replace",
        };
        let mut pasted = Vec::new();
        self.anim_step(label, |e| {
            let rate = e.scene.time.rate;
            for (k, keys) in &clip.channels {
                let k = target(k);
                let Some(now) = animation::read(&e.scene, &k) else {
                    continue;
                };
                let stepped = animation::is_stepped(&e.scene, &k);
                let c = e
                    .scene
                    .animation
                    .channels
                    .entry(k.clone())
                    .or_insert_with(|| AnimChannel {
                        curve: Curve {
                            keys: Vec::new(),
                            stepped,
                            ..Default::default()
                        },
                        rest: now,
                    });
                match mode {
                    PasteMode::Merge => {}
                    PasteMode::Replace => c
                        .curve
                        .keys
                        .retain(|key| key.time < at || key.time > at + span),
                    PasteMode::Insert => {
                        let shift = span + rate.frame_duration();
                        for key in c.curve.keys.iter_mut().filter(|key| key.time >= at) {
                            key.time = key.time + shift;
                        }
                    }
                }
                for key in keys {
                    let mut key = key.clone();
                    key.time = at + key.time;
                    pasted.push((k.clone(), key.time));
                    c.curve.put(key);
                }
            }
            if pasted.is_empty() {
                return Err("None of the copied channels is here".into());
            }
            animation::repose(&mut e.scene);
            Ok(true)
        })?;
        self.anim.selected_keys = pasted.into_iter().collect();
        Ok(())
    }

    /// TL-21 Insert Frame (`n` = 1) / Remove Frame (`n` = -1) at the key
    /// time: the selected items' keys after it move one frame. Remove
    /// never deletes keys: it stops where a key would land on another.
    pub fn insert_frames(&mut self, n: i64) -> Result<()> {
        let items = self.scene.selected_items();
        if items.is_empty() {
            return Err("Select the items whose keys move".into());
        }
        let at = self.key_time();
        let label = if n >= 0 {
            "Insert Frame"
        } else {
            "Remove Frame"
        };
        self.anim_step(label, |e| {
            let by = Tick(e.scene.time.rate.frame_duration().0 * n.abs());
            let mut changed = false;
            for (k, c) in e.scene.animation.channels.iter_mut() {
                if !items.contains(&k.item) {
                    continue;
                }
                let keys = &mut c.curve.keys;
                if n < 0 {
                    // The first key after `at` may only move down to just
                    // after the last key at or before it.
                    let first = keys.iter().position(|key| key.time > at);
                    if let Some(i) = first {
                        let floor = if i > 0 {
                            keys[i - 1].time
                        } else {
                            Tick(i64::MIN)
                        };
                        if keys[i].time - by <= floor {
                            continue;
                        }
                    }
                }
                for key in keys.iter_mut().filter(|key| key.time > at) {
                    key.time = if n >= 0 { key.time + by } else { key.time - by };
                    changed = true;
                }
            }
            animation::repose(&mut e.scene);
            Ok(changed)
        })
    }

    // ------------------------------------------------------------ settings

    /// TK-03/04: frame rate, scene range and the other time settings, one
    /// "Time Settings" undo step. The current time stays where it is.
    pub fn set_time_settings(&mut self, time: SceneTime, settings: TimeSettings) -> Result<()> {
        time.validate()?;
        if let Some((a, b)) = settings.current_range
            && a > b
        {
            return Err("The current range's start is after its end".into());
        }
        let mut time = time;
        time.current = self.scene.time.current;
        self.anim_step("Time Settings", move |e| {
            let changed = e.scene.time != time || e.scene.animation.settings != settings;
            e.scene.time = time;
            e.scene.animation.settings = settings;
            Ok(changed)
        })
    }

    /// TL-13: the current range (`None`: back to the scene range).
    pub fn set_current_range(&mut self, range: Option<(Tick, Tick)>) -> Result<()> {
        let mut s = self.scene.animation.settings.clone();
        s.current_range = range.filter(|r| *r != animation::scene_range(&self.scene));
        self.set_time_settings(self.scene.time, s)
    }

    /// TK-30 Auto Key mode.
    pub fn set_auto_key(&mut self, mode: AutoKey) -> Result<()> {
        let mut s = self.scene.animation.settings.clone();
        s.auto_key = mode;
        self.set_time_settings(self.scene.time, s)
    }

    /// TL-14: fit the current range to the selected channels' or items'
    /// keys (none selected: every key), or with `all` every key.
    pub fn fit_range_to_keys(&mut self, all: bool) -> Result<()> {
        let times = if all || self.key_scope().is_empty() {
            self.scene.animation.key_times(None)
        } else {
            self.scope_key_times()
        };
        let (Some(a), Some(b)) = (times.first().copied(), times.last().copied()) else {
            return Err("No keys to fit to".into());
        };
        let b = if a == b {
            a + self.scene.time.rate.frame_duration()
        } else {
            b
        };
        let (s0, s1) = animation::scene_range(&self.scene);
        let mut time = self.scene.time;
        // The scene range grows to hold the keys.
        time.start = s0.min(a);
        time.end = s1.max(b);
        let mut s = self.scene.animation.settings.clone();
        s.current_range = Some((a, b)).filter(|r| *r != (time.start, time.end));
        self.set_time_settings(time, s)
    }

    // ------------------------------------------------------------ markers

    /// TL-50/51: a marker at `t` (nearest frame), owned by the selected
    /// items' first item, or the scene. Returns its id.
    pub fn add_marker(&mut self, t: Tick, scene_owned: bool) -> Result<u64> {
        let rate = self.scene.time.rate;
        let t = animation::nearest_frame(rate, t);
        let owner = if scene_owned {
            None
        } else {
            self.scene.selected_items().first().copied()
        };
        let id = self
            .scene
            .animation
            .markers
            .iter()
            .map(|m| m.id)
            .max()
            .unwrap_or(0)
            + 1;
        self.anim_step("Add Marker", move |e| {
            e.scene.animation.markers.push(Marker {
                id,
                time: t,
                length: Tick::ZERO,
                owner,
                label: String::new(),
                show_label: false,
                shape: Default::default(),
                color: [0.95, 0.75, 0.2],
                frame_limits: false,
            });
            Ok(true)
        })?;
        Ok(id)
    }

    /// TL-52/53: changes markers `ids` with `edit`, one undo step.
    pub fn edit_markers(
        &mut self,
        ids: &[u64],
        label: &str,
        edit: impl Fn(&mut Marker) -> Result<()>,
    ) -> Result<()> {
        if ids.is_empty() {
            return Err("Select markers first".into());
        }
        let ids = ids.to_vec();
        self.anim_step(label, |e| {
            let mut changed = false;
            let mut found = false;
            for m in e.scene.animation.markers.iter_mut() {
                if ids.contains(&m.id) {
                    found = true;
                    let before = m.clone();
                    edit(m)?;
                    if m.length < Tick::ZERO {
                        return Err("A range marker's length can't be negative".into());
                    }
                    changed |= *m != before;
                }
            }
            if !found {
                return Err("No such marker".into());
            }
            Ok(changed)
        })
    }

    /// TL-52 Delete.
    pub fn delete_markers(&mut self, ids: &[u64]) -> Result<()> {
        if ids.is_empty() {
            return Err("Select markers first".into());
        }
        let ids = ids.to_vec();
        self.anim_step("Delete Marker", |e| {
            let n = e.scene.animation.markers.len();
            e.scene.animation.markers.retain(|m| !ids.contains(&m.id));
            Ok(e.scene.animation.markers.len() != n)
        })?;
        self.anim.selected_markers.retain(|id| !ids.contains(id));
        Ok(())
    }

    /// The markers shown now (TL-51): scene markers, and item markers of
    /// selected items.
    pub fn visible_markers(&self) -> Vec<&Marker> {
        let selected = self.scene.selected_items();
        self.scene
            .animation
            .markers
            .iter()
            .filter(|m| m.owner.is_none_or(|o| selected.contains(&o)))
            .collect()
    }
}

/// How a key selection changes (TL-41).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySelect {
    Replace,
    Add,
    Remove,
}
