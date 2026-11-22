//! The animation commands' handlers (TK-20..71, TL-13..54): every button,
//! menu entry and shortcut of time, keys and markers runs one of these
//! lines (UIC-160/164). Registered in `command_registry::COMMANDS` under
//! `operandi.time.*`, `operandi.key.*`, `operandi.channel.select` and
//! `operandi.marker.*`: the reference name map has no animation names, so
//! these are Operandi's own (PLAN "Command history and undo"; UIC-165,
//! provisional until Jason approves them).
//!
//! Times are given in frames (`frame:`) or seconds (`time:`); channels as
//! `item:name` lists (`3:pos.Y,3:scl.Y`).
use crate::anim_curve::Behavior;
use crate::anim_curve::Slope;
use crate::anim_edit::{KeyJump, KeySelect, PasteMode, Side};
use crate::anim_graph::{Interpolation, ShadowAction, ValueMode};
use crate::animation::{self, AutoKey, ChannelKey, EndBehavior, MarkerShape, TimeDisplay};
use crate::time_tick::{FrameRate, Tick};
use crate::{Command, Editor};
use serde_json::{Map, Value};

type Args = Map<String, Value>;
type Out = Result<Option<Command>, String>;

fn text<'a>(a: &'a Args, name: &str) -> Option<&'a str> {
    a.get(name).and_then(Value::as_str)
}
fn number(a: &Args, name: &str) -> Result<Option<f64>, String> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_f64()
            .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
            .filter(|f| f.is_finite())
            .map(Some)
            .ok_or_else(|| format!("{name} must be a number")),
    }
}
fn flag(a: &Args, name: &str) -> Result<Option<bool>, String> {
    match a.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(Value::String(s)) => crate::command_args::boolean(name, s).map(Some),
        Some(_) => Err(format!("{name} must be true or false")),
    }
}

/// A time from `frame:` (frames) or `time:` (seconds).
fn time_arg(e: &Editor, a: &Args) -> Result<Option<Tick>, String> {
    let rate = e.scene.time.rate;
    if let Some(f) = number(a, "frame")? {
        return Ok(Some(animation::at_frames(rate, f)));
    }
    if let Some(s) = number(a, "time")? {
        return Ok(Some(Tick::from_seconds(s)));
    }
    Ok(None)
}

fn channels_arg(a: &Args) -> Result<Option<Vec<ChannelKey>>, String> {
    let Some(list) = text(a, "channels") else {
        return Ok(None);
    };
    list.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ChannelKey::parse)
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

/// `channels:` or else the selected channels (UIC-119), else none.
fn channels_or_selected(e: &Editor, a: &Args) -> Result<Vec<ChannelKey>, String> {
    let list = match channels_arg(a)? {
        Some(list) => list,
        None => e.anim.selected_channels.iter().cloned().collect(),
    };
    if list.is_empty() {
        return Err("Give channels:item:name,... or select channels first".into());
    }
    Ok(list)
}

fn ids_arg(e: &Editor, a: &Args) -> Result<Vec<u64>, String> {
    match text(a, "ids") {
        Some(list) => list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse()
                    .map_err(|_| format!("ids are numbers, not \"{s}\""))
            })
            .collect(),
        None => Ok(e.anim.selected_markers.iter().copied().collect()),
    }
}

/// A key list `item:name@frame,...`.
fn keys_arg(e: &Editor, a: &Args) -> Result<Vec<(ChannelKey, Tick)>, String> {
    let rate = e.scene.time.rate;
    let Some(list) = text(a, "keys") else {
        return Ok(Vec::new());
    };
    list.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let (k, f) = s
                .rsplit_once('@')
                .ok_or_else(|| format!("A key is item:name@frame, not \"{s}\""))?;
            let f: f64 = f
                .trim()
                .parse()
                .map_err(|_| format!("Bad frame in \"{s}\""))?;
            Ok((ChannelKey::parse(k)?, animation::at_frames(rate, f)))
        })
        .collect()
}

fn frames_tick(rate: FrameRate, f: f64) -> Tick {
    let t = animation::at_frames(rate, f.abs());
    if f < 0. { Tick(-t.0) } else { t }
}

/// The handler of every animation entry (`Entry::json.run`).
pub fn run(e: &mut Editor, name: &str, a: &Args) -> Out {
    let rate = e.scene.time.rate;
    match name {
        // ---------------------------------------------------- time (UI)
        "operandi.time.set" => {
            let t = time_arg(e, a)?.ok_or("Give frame: or time:")?;
            e.set_current_time(t, false)?;
        }
        "operandi.time.step" => {
            let n = number(a, "frames")?.unwrap_or(1.);
            e.step_time(n.round() as i64)?;
        }
        "operandi.time.go" => match text(a, "to").unwrap_or("first") {
            "first" => e.go_to_range_end(false)?,
            "last" => e.go_to_range_end(true)?,
            "prevKey" => e.jump_to_key(KeyJump::Previous)?,
            "nextKey" => e.jump_to_key(KeyJump::Next)?,
            "firstKey" => e.jump_to_key(KeyJump::First)?,
            "lastKey" => e.jump_to_key(KeyJump::Last)?,
            other => {
                return Err(format!(
                    "to is first, last, prevKey, nextKey, firstKey or lastKey, not {other}"
                ));
            }
        },
        "operandi.time.play" => {
            let backwards = flag(a, "backwards")?.unwrap_or(false);
            let stop = flag(a, "stop")?;
            let playing = e.anim.playing.is_some();
            if stop == Some(true) || (stop.is_none() && playing) {
                e.anim.playing = None;
                e.anim.play_start = None;
            } else {
                e.anim.playing = Some(backwards);
                e.anim.play_start = Some(e.scene.time.current);
            }
        }
        "operandi.time.keyTime" => {
            if flag(a, "reset")? == Some(true) {
                e.set_key_time(None);
            } else {
                let t = time_arg(e, a)?.ok_or("Give frame:, time: or reset:true")?;
                e.set_key_time(Some(t));
            }
        }
        "operandi.channel.select" => {
            let add = flag(a, "add")?.unwrap_or(false);
            match text(a, "channel") {
                None | Some("none") => e.select_channel(None, add),
                Some(k) => e.select_channel(Some(ChannelKey::parse(k)?), add),
            }
        }
        "operandi.key.select" => {
            let how = match text(a, "mode").unwrap_or("replace") {
                "replace" => KeySelect::Replace,
                "add" => KeySelect::Add,
                "remove" => KeySelect::Remove,
                "clear" => {
                    e.anim.selected_keys.clear();
                    return Ok(None);
                }
                "all" => {
                    let all: Vec<(ChannelKey, Tick)> = e
                        .key_scope()
                        .into_iter()
                        .flat_map(|k| {
                            let times: Vec<Tick> =
                                e.scene.animation.channels[&k].curve.times().collect();
                            times.into_iter().map(move |t| (k.clone(), t))
                        })
                        .collect();
                    e.select_keys(all, KeySelect::Replace);
                    return Ok(None);
                }
                other => {
                    return Err(format!(
                        "mode is replace, add, remove, all or clear, not {other}"
                    ));
                }
            };
            let keys = keys_arg(e, a)?;
            e.select_keys(keys, how);
        }
        "operandi.marker.select" => {
            let ids = ids_arg(e, a)?;
            if !flag(a, "add")?.unwrap_or(false) {
                e.anim.selected_markers.clear();
            }
            e.anim.selected_markers.extend(ids);
        }
        "operandi.key.copy" => {
            e.copy_keys()?;
        }
        // ---------------------------------------------------- keying
        "operandi.key.set" => match channels_arg(a)? {
            Some(list) => {
                e.key_channels(&list, "Set Key")?;
            }
            None => {
                e.key_selected_channels()?;
            }
        },
        "operandi.key.toggle" => {
            let list = channels_or_selected(e, a)?;
            e.toggle_keys(&list)?;
        }
        "operandi.key.remove" => {
            let list = channels_or_selected(e, a)?;
            let t = time_arg(e, a)?.unwrap_or_else(|| e.key_time());
            e.remove_keys_at(&list, t)?;
        }
        "operandi.key.transforms" => {
            let groups: &[&str] = match text(a, "type").unwrap_or("all") {
                "position" => &["pos"],
                "rotation" => &["rot"],
                "scale" => &["scl"],
                "all" => &["pos", "rot", "scl"],
                other => {
                    return Err(format!(
                        "type is position, rotation, scale or all, not {other}"
                    ));
                }
            };
            e.key_transforms(groups)?;
        }
        "operandi.key.items" => {
            e.key_selected_items()?;
        }
        "operandi.key.removeAnimation" => {
            let list = channels_or_selected(e, a)?;
            e.remove_animation(&list, false)?;
        }
        "operandi.key.reset" => {
            let list = channels_or_selected(e, a)?;
            e.remove_animation(&list, true)?;
        }
        "operandi.key.bake" => {
            let list = channels_or_selected(e, a)?;
            e.bake_channels(&list)?;
        }
        "operandi.key.apply" => e.apply_pending()?,
        "operandi.key.discard" => e.discard_pending()?,
        "operandi.key.autoKey" => {
            let word = text(a, "mode").ok_or("Give mode:off, animated or all")?;
            let mode = AutoKey::parse(word)
                .ok_or_else(|| format!("mode is off, animated or all, not {word}"))?;
            e.set_auto_key(mode)?;
        }
        // ---------------------------------------------------- key edits
        "operandi.key.move" => {
            let f = number(a, "frames")?.ok_or("Give frames:")?;
            e.move_selected_keys(frames_tick(rate, f))?;
        }
        "operandi.key.scale" => {
            let factor = number(a, "factor")?.ok_or("Give factor:")?;
            let pivot = match time_arg(e, a)? {
                Some(t) => t,
                None => e.key_time(),
            };
            e.scale_selected_keys(factor, pivot)?;
        }
        "operandi.key.invert" => e.invert_selected_keys()?,
        "operandi.key.delete" => {
            e.delete_selected_keys(flag(a, "related")?.unwrap_or(false))?;
        }
        "operandi.key.slope" => {
            let word = text(a, "type").ok_or("Give type:")?;
            let (slope, implied) = match word {
                "linearIn" => (Slope::Linear, Some(Side::In)),
                "linearOut" => (Slope::Linear, Some(Side::Out)),
                w => (
                    Slope::parse(w).ok_or_else(|| {
                        format!("type is auto, smoothFlat, autoFlat, flat, linear, linearIn, linearOut, stepped or manual, not {w}")
                    })?,
                    None,
                ),
            };
            let side = match text(a, "side") {
                None => implied.unwrap_or(Side::Both),
                Some("in") => Side::In,
                Some("out") => Side::Out,
                Some("both") => Side::Both,
                Some(o) => return Err(format!("side is in, out or both, not {o}")),
            };
            e.set_selected_slopes(slope, side)?;
        }
        "operandi.key.cut" => e.cut_keys()?,
        "operandi.key.paste" => {
            let mode = match text(a, "mode").unwrap_or("merge") {
                "merge" => PasteMode::Merge,
                "insert" => PasteMode::Insert,
                "replace" => PasteMode::Replace,
                o => return Err(format!("mode is merge, insert or replace, not {o}")),
            };
            e.paste_keys(mode)?;
        }
        "operandi.time.insertFrame" => e.insert_frames(1)?,
        "operandi.time.removeFrame" => e.insert_frames(-1)?,
        // ---------------------------------------------------- settings
        "operandi.time.settings" => {
            let mut time = e.scene.time;
            let mut s = e.scene.animation.settings.clone();
            if let Some(fps) = number(a, "fps")? {
                if !crate::scene_time::FPS_RANGE.contains(&fps) {
                    return Err("fps must be between 0.001 and 1000".into());
                }
                time.rate = FrameRate::from_f64(fps);
            }
            let rate = time.rate;
            if let Some(f) = number(a, "start")? {
                time.start = frames_tick(rate, f);
            }
            if let Some(f) = number(a, "end")? {
                time.end = frames_tick(rate, f);
            }
            let (mut r0, mut r1) = s.current_range.unwrap_or((time.start, time.end));
            let mut range = s.current_range.is_some();
            if let Some(f) = number(a, "rangeStart")? {
                r0 = frames_tick(rate, f);
                range = true;
            }
            if let Some(f) = number(a, "rangeEnd")? {
                r1 = frames_tick(rate, f);
                range = true;
            }
            s.current_range = (range && (r0, r1) != (time.start, time.end)).then_some((r0, r1));
            if let Some(w) = text(a, "display") {
                s.display = TimeDisplay::parse(w)
                    .ok_or_else(|| format!("display is frames, seconds, smpte or film, not {w}"))?;
            }
            if let Some(w) = text(a, "autoKey") {
                s.auto_key = AutoKey::parse(w)
                    .ok_or_else(|| format!("autoKey is off, animated or all, not {w}"))?;
            }
            if let Some(b) = flag(a, "snap")? {
                s.snap = b;
            }
            if let Some(b) = flag(a, "realTime")? {
                s.real_time = b;
            }
            if let Some(w) = text(a, "endBehavior") {
                s.end = EndBehavior::parse(w)
                    .ok_or_else(|| format!("endBehavior is hold, repeat or wrap, not {w}"))?;
            }
            if let Some(b) = flag(a, "autoUpdate")? {
                s.auto_update_keys = b;
            }
            e.set_time_settings(time, s)?;
        }
        "operandi.time.range" => {
            if flag(a, "reset")? == Some(true) {
                e.set_current_range(None)?;
            } else {
                let (r0, r1) = animation::current_range(&e.scene);
                let start = number(a, "start")?.map_or(r0, |f| frames_tick(rate, f));
                let end = number(a, "end")?.map_or(r1, |f| frames_tick(rate, f));
                if start > end {
                    return Err("The range's start is after its end".into());
                }
                e.set_current_range(Some((start, end)))?;
            }
        }
        "operandi.time.fit" => e.fit_range_to_keys(flag(a, "all")?.unwrap_or(false))?,
        // ---------------------------------------------------- markers
        "operandi.marker.add" => {
            let t = time_arg(e, a)?.unwrap_or(e.scene.time.current);
            let id = e.add_marker(t, flag(a, "scene")?.unwrap_or(false))?;
            e.anim.selected_markers = [id].into();
        }
        "operandi.marker.edit" => {
            let ids = ids_arg(e, a)?;
            let at = number(a, "frame")?.map(|f| animation::at_frames(rate, f));
            let shift = number(a, "shift")?.map(|f| frames_tick(rate, f));
            let length = number(a, "length")?.map(|f| frames_tick(rate, f.max(0.)));
            let end = number(a, "endFrame")?.map(|f| animation::at_frames(rate, f));
            let label = text(a, "label").map(str::to_owned);
            let shape = match text(a, "shape") {
                Some(w) => Some(MarkerShape::parse(w).ok_or_else(|| {
                    format!("shape is triangle, square, diamond or bar, not {w}")
                })?),
                None => None,
            };
            let color = match text(a, "color") {
                Some(c) => {
                    let parts: Vec<f32> = c
                        .split(',')
                        .map(|p| p.trim().parse::<f32>())
                        .collect::<Result<_, _>>()
                        .map_err(|_| "color is r,g,b (0-1)".to_string())?;
                    match parts.as_slice() {
                        [r, g, b] => Some([*r, *g, *b]),
                        _ => return Err("color is r,g,b (0-1)".into()),
                    }
                }
                None => None,
            };
            let show = flag(a, "showLabel")?;
            let limits = flag(a, "frameLimits")?;
            let snap = flag(a, "snapToFrames")?.unwrap_or(false);
            e.edit_markers(&ids, "Edit Marker", |m| {
                if let Some(t) = at {
                    m.time = t;
                }
                if let Some(s) = shift {
                    m.time = m.time + s;
                }
                if let Some(l) = length {
                    m.length = l;
                }
                if let Some(t) = end {
                    if t < m.time {
                        return Err("The end is before the marker".into());
                    }
                    m.length = t - m.time;
                }
                if snap {
                    let end = animation::nearest_frame(rate, m.end());
                    m.time = animation::nearest_frame(rate, m.time);
                    m.length = end - m.time;
                }
                if let Some(l) = &label {
                    m.label = l.clone();
                }
                if let Some(s) = shape {
                    m.shape = s;
                }
                if let Some(c) = color {
                    m.color = c;
                }
                if let Some(b) = show {
                    m.show_label = b;
                }
                if let Some(b) = limits {
                    m.frame_limits = b;
                }
                Ok(())
            })?;
        }
        "operandi.marker.delete" => {
            let ids = ids_arg(e, a)?;
            e.delete_markers(&ids)?;
        }
        "operandi.render.frameRange" | "operandi.view.playblast" => {
            let (r0, r1) = animation::current_range(&e.scene);
            let whole = |t: Tick| animation::frames(rate, t).round() as i64;
            let scene_range = text(a, "range") == Some("scene");
            let (d0, d1) = if scene_range {
                (whole(e.scene.time.start), whole(e.scene.time.end))
            } else {
                (whole(r0), whole(r1))
            };
            let start = number(a, "start")?.map_or(d0, |f| f.round() as i64);
            let end = number(a, "end")?.map_or(d1, |f| f.round() as i64);
            if start > end {
                return Err("The range's start is after its end".into());
            }
            if end - start > 100_000 {
                return Err("That range is too long".into());
            }
            let folder = text(a, "folder").map(str::to_owned);
            if name == "operandi.render.frameRange" {
                let whole_arg = |k: &str| -> Result<Option<u32>, String> {
                    Ok(number(a, k)?.map(|f| f.round().clamp(1., 1e6) as u32))
                };
                let size = match (whole_arg("width")?, whole_arg("height")?) {
                    (Some(w), Some(h)) => Some([w, h]),
                    (None, None) => None,
                    _ => return Err("Give both width and height, or neither".into()),
                };
                e.anim.requests.push(animation::AnimRequest::RenderRange {
                    start,
                    end,
                    folder,
                    name: text(a, "name").unwrap_or("Render").to_owned(),
                    samples: whole_arg("samples")?,
                    size,
                    camera: number(a, "camera")?.map(|f| f as u64),
                });
            } else {
                let scale = number(a, "scale")?.unwrap_or(1.);
                if ![1., 0.5, 0.25].contains(&scale) {
                    return Err("scale is 1, 0.5 or 0.25".into());
                }
                e.anim.requests.push(animation::AnimRequest::Playblast {
                    start,
                    end,
                    scale: scale as f32,
                    folder,
                    name: text(a, "name").unwrap_or("Playblast").to_owned(),
                    open: flag(a, "open")?.unwrap_or(true),
                    movie: match text(a, "format").unwrap_or("movie") {
                        "movie" | "mp4" => true,
                        "images" | "png" => false,
                        other => return Err(format!("format is movie or images, not {other}")),
                    },
                });
            }
        }
        "operandi.render.cancelRange" => e.anim.requests.push(animation::AnimRequest::Cancel),
        // ---------------------------------------------------- graph editor
        "operandi.key.offset" => {
            let dt = frames_tick(rate, number(a, "frames")?.unwrap_or(0.));
            e.offset_selected_keys(dt, number(a, "value")?.unwrap_or(0.))?;
        }
        "operandi.key.setValue" => {
            let v = number(a, "value")?.ok_or("Give value:")?;
            let mode = match text(a, "mode").unwrap_or("set") {
                "set" => ValueMode::Set,
                "add" => ValueMode::Add,
                "multiply" => ValueMode::Multiply,
                o => return Err(format!("mode is set, add or multiply, not {o}")),
            };
            e.set_key_values(v, mode)?;
        }
        "operandi.key.scaleValue" => {
            let factor = number(a, "factor")?.ok_or("Give factor:")?;
            e.scale_key_values(factor, number(a, "pivot")?.unwrap_or(0.))?;
        }
        "operandi.key.setFrame" => {
            let relative = flag(a, "relative")?.unwrap_or(false);
            let f = number(a, "frame")?.ok_or("Give frame:")?;
            e.set_key_frames(frames_tick(rate, f), relative)?;
        }
        "operandi.key.handle" => {
            let incoming = match text(a, "side").unwrap_or("out") {
                "in" => true,
                "out" => false,
                o => return Err(format!("side is in or out, not {o}")),
            };
            // Slopes are given per frame, as the graph shows them.
            let slope = number(a, "slope")?.map(|s| s * rate.as_f64());
            let weight = number(a, "weight")?;
            if slope.is_none() && weight.is_none() {
                return Err("Give slope: and/or weight:".into());
            }
            e.set_key_handles(incoming, slope, weight)?;
        }
        "operandi.key.break" => e.set_keys_broken(true)?,
        "operandi.key.unify" => e.set_keys_broken(false)?,
        "operandi.key.autoHandles" => e.reset_key_handles(flag(a, "slopes")?.unwrap_or(false))?,
        "operandi.key.add" => {
            let k = ChannelKey::parse(text(a, "channel").ok_or("Give channel:item:name")?)?;
            let t = time_arg(e, a)?.unwrap_or(e.scene.time.current);
            e.add_key_at(&k, t, number(a, "value")?)?;
        }
        "operandi.key.keepShape" => {
            let list = channels_or_selected(e, a)?;
            let t = time_arg(e, a)?.unwrap_or(e.scene.time.current);
            e.key_keeping_shape(&list, t)?;
        }
        "operandi.curve.behavior" => {
            let word = |k: &str| -> Result<Option<Behavior>, String> {
                text(a, k)
                    .map(|w| {
                        Behavior::parse(w).ok_or_else(|| {
                            format!("{k} is constant, constantSmooth, linear, repeat, oscillate, oscillateSmooth, offsetRepeat, offsetRepeatSmooth or reset, not {w}")
                        })
                    })
                    .transpose()
            };
            let (pre, post) = (word("pre")?, word("post")?);
            if pre.is_none() && post.is_none() {
                return Err("Give pre: and/or post:".into());
            }
            let list = e.curve_scope(channels_arg(a)?);
            e.set_curve_behavior(list, pre, post)?;
        }
        "operandi.curve.interpolation" => {
            let kind = match text(a, "type").ok_or("Give type:")? {
                "curve" => Interpolation::Curve,
                "linear" => Interpolation::Linear,
                "stepped" => Interpolation::Stepped,
                o => return Err(format!("type is curve, linear or stepped, not {o}")),
            };
            let list = e.curve_scope(channels_arg(a)?);
            e.set_curve_interpolation(list, kind)?;
        }
        "operandi.curve.cycle" => {
            let list = e.curve_scope(channels_arg(a)?);
            let on = flag(a, "on")?.unwrap_or(true);
            e.set_curve_cycle(list, on)?;
        }
        "operandi.curve.shadow" => {
            let action = match text(a, "action").unwrap_or("copy") {
                "copy" => ShadowAction::Copy,
                "replace" => ShadowAction::Replace,
                "swap" => ShadowAction::Swap,
                "clear" => ShadowAction::Clear,
                o => return Err(format!("action is copy, replace, swap or clear, not {o}")),
            };
            let list = e.curve_scope(channels_arg(a)?);
            e.shadow(list, action)?;
        }
        "operandi.curve.eulerFilter" => {
            let list = e.curve_scope(channels_arg(a)?);
            e.euler_filter(list)?;
        }
        "operandi.curve.simplify" => {
            let list = e.curve_scope(channels_arg(a)?);
            e.simplify_curves(list, number(a, "tolerance")?.unwrap_or(0.001))?;
        }
        // Answered by `answer` (a query changes nothing).
        "operandi.anim.query" => {}
        other => return Err(format!("Unknown animation command {other}")),
    }
    Ok(None)
}

/// `operandi.anim.query` (UIC-163): the scene's time settings, playback,
/// pending edits and markers, and every animated channel (or the ones in
/// `channels:` / of `item:`) with its keys, its value now and its values
/// at `frames:` (a list), as JSON. Other names: `None`.
pub fn answer(e: &Editor, name: &str, a: &Args) -> Option<Result<String, String>> {
    if name != "operandi.anim.query" {
        return None;
    }
    Some(query(e, a))
}

fn query(e: &Editor, a: &Args) -> Result<String, String> {
    use serde_json::json;
    let s = &e.scene;
    let rate = s.time.rate;
    let fr = |t: Tick| animation::frames(rate, t);
    let set = &s.animation.settings;
    let (r0, r1) = animation::current_range(s);
    let frames: Vec<f64> = match a.get("frames") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(list)) => list
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(|p| {
                p.parse::<f64>()
                    .map_err(|_| format!("frames is a list of numbers, not {p}"))
            })
            .collect::<Result<_, _>>()?,
        Some(Value::Array(list)) => list.iter().filter_map(Value::as_f64).collect(),
        Some(Value::Number(n)) => n.as_f64().into_iter().collect(),
        Some(_) => return Err("frames is a list of numbers".into()),
    };
    let wanted = channels_arg(a)?;
    let item = number(a, "item")?.map(|f| f as u64);
    let channels: Vec<Value> = s
        .animation
        .channels
        .iter()
        .filter(|(k, _)| wanted.as_ref().is_none_or(|w| w.contains(k)))
        .filter(|(k, _)| item.is_none_or(|id| k.item == id))
        .map(|(k, c)| {
            let keys: Vec<Value> = c
                .curve
                .keys
                .iter()
                .map(|key| {
                    json!({
                        "frame": fr(key.time),
                        "seconds": key.time.seconds(),
                        "value": key.value,
                        "in": key.slope_in.name(),
                        "out": key.slope_out.name(),
                        "inSlope": c.curve.key_at(key.time).map(|i| c.curve.slope(i, true) / rate.as_f64()),
                        "outSlope": c.curve.key_at(key.time).map(|i| c.curve.slope(i, false) / rate.as_f64()),
                        "inWeight": key.weight_in,
                        "outWeight": key.weight_out,
                        "broken": key.broken,
                    })
                })
                .collect();
            let values: Map<String, Value> = frames
                .iter()
                .map(|f| {
                    (
                        f.to_string(),
                        json!(c.curve.value_at(animation::at_frames(rate, *f))),
                    )
                })
                .collect();
            json!({
                "channel": k.text(),
                "rest": c.rest,
                "stepped": c.curve.stepped,
                "pre": c.curve.pre.name(),
                "post": c.curve.post.name(),
                "cycle": c.curve.cycle,
                "shadow": s.animation.shadows.contains_key(k),
                "now": animation::read(s, k),
                "pending": animation::is_pending(s, k),
                "keys": keys,
                "values": values,
            })
        })
        .collect();
    let markers: Vec<Value> = s
        .animation
        .markers
        .iter()
        .map(|m| {
            json!({
                "id": m.id,
                "frame": fr(m.time),
                "length": fr(m.length),
                "owner": m.owner,
                "label": m.label,
                "shape": m.shape.name(),
                "frameLimits": m.frame_limits,
                "showLabel": m.show_label,
                "color": m.color,
                "selected": e.anim.selected_markers.contains(&m.id),
            })
        })
        .collect();
    let out = json!({
        "frame": fr(s.time.current),
        "seconds": s.time.current.seconds(),
        "fps": rate.as_f64(),
        "start": fr(s.time.start),
        "end": fr(s.time.end),
        "rangeStart": fr(r0),
        "rangeEnd": fr(r1),
        "display": set.display.name(),
        "timeText": animation::time_text(set.display, rate, s.time.current),
        "autoKey": set.auto_key.name(),
        "snap": set.snap,
        "realTime": set.real_time,
        "endBehavior": set.end.name(),
        "autoUpdate": set.auto_update_keys,
        "keyTime": fr(e.key_time()),
        "playing": e.anim.playing.map(|back| if back { "backwards" } else { "forwards" }),
        "pending": animation::pending(s).iter().map(ChannelKey::text).collect::<Vec<_>>(),
        "selectedChannels": e.anim.selected_channels.iter().map(ChannelKey::text).collect::<Vec<_>>(),
        "selectedKeys": e.live_selected_keys().iter().map(|(k, t)| format!("{}@{}", k.text(), fr(*t))).collect::<Vec<_>>(),
        "markers": markers,
        "channels": channels,
        "job": e.anim.progress.as_ref().map(|(done, all, folder)| json!({"done": done, "frames": all, "folder": folder})),
    });
    Ok(out.to_string())
}

/// Every animation command name, for the app's lists and tests.
pub const NAMES: &[&str] = &[
    "operandi.anim.query",
    "operandi.channel.select",
    "operandi.curve.behavior",
    "operandi.curve.cycle",
    "operandi.curve.eulerFilter",
    "operandi.curve.interpolation",
    "operandi.curve.shadow",
    "operandi.curve.simplify",
    "operandi.key.add",
    "operandi.key.apply",
    "operandi.key.autoHandles",
    "operandi.key.break",
    "operandi.key.handle",
    "operandi.key.keepShape",
    "operandi.key.scaleValue",
    "operandi.key.setFrame",
    "operandi.key.setValue",
    "operandi.key.unify",
    "operandi.key.autoKey",
    "operandi.key.bake",
    "operandi.key.copy",
    "operandi.key.cut",
    "operandi.key.delete",
    "operandi.key.discard",
    "operandi.key.invert",
    "operandi.key.items",
    "operandi.key.move",
    "operandi.key.offset",
    "operandi.key.paste",
    "operandi.key.remove",
    "operandi.key.removeAnimation",
    "operandi.key.reset",
    "operandi.key.scale",
    "operandi.key.select",
    "operandi.key.set",
    "operandi.key.slope",
    "operandi.key.toggle",
    "operandi.key.transforms",
    "operandi.marker.add",
    "operandi.marker.delete",
    "operandi.marker.edit",
    "operandi.marker.select",
    "operandi.render.cancelRange",
    "operandi.render.frameRange",
    "operandi.time.fit",
    "operandi.time.go",
    "operandi.time.insertFrame",
    "operandi.time.keyTime",
    "operandi.time.play",
    "operandi.time.range",
    "operandi.time.removeFrame",
    "operandi.time.set",
    "operandi.time.settings",
    "operandi.time.step",
    "operandi.view.playblast",
];
