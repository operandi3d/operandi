//! USD export of animation (BEHAVIOR-TIMELINE TL-80..84; PLAN "USD").
//!
//! Operandi keeps time in seconds; USD in time codes. The stage gets
//! `framesPerSecond` = `timeCodesPerSecond` = the scene's frame rate and
//! `startTimeCode` / `endTimeCode` = the scene range in frames (TL-80).
//! Each animated transform op (translate, rotate in the item's order,
//! scale; TL-82) and each animated camera or light value gets one time
//! sample per frame across the scene range, plus one at every key time
//! that falls between frames (TL-81). Unanimated values stay defaults.
//! Curves are re-sampled, so the native .opscene stays the lossless home.
use operandi_core::Scene;
use operandi_core::animation::{self, ChannelKey};
use operandi_core::interchange::UsdAnimation;
use operandi_core::time_tick::Tick;

/// The stage's timing metadata (TL-80), in frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageTime {
    pub fps: f64,
    pub start: f64,
    pub end: f64,
}

/// One item's time samples (TL-81/82): a value per entry of `times`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Samples {
    /// Time codes (frames).
    pub times: Vec<f64>,
    pub translate: Option<Vec<[f64; 3]>>,
    /// Degrees, in the item's rotation order.
    pub rotate: Option<Vec<[f64; 3]>>,
    pub scale: Option<Vec<[f64; 3]>>,
    /// Camera attributes that change: USD name, a value per time.
    pub camera: Vec<(&'static str, Vec<f64>)>,
    /// Light attributes that change: USD name, a value (or colour) per time.
    pub light: Vec<(&'static str, Vec<[f64; 3]>)>,
}

/// The scene posed at every sample time, made once per export.
pub(crate) struct Frames {
    pub(crate) times: Vec<f64>,
    pub(crate) posed: Vec<Scene>,
    /// Items the rig moves: sampled like animated ones (§8).
    pub(crate) rigged: std::collections::HashSet<u64>,
}

/// TL-80/83: the stage's timing; `None` writes none (no animation, or the
/// export option is off).
pub fn stage_time(scene: &Scene, mode: UsdAnimation) -> Option<StageTime> {
    let rate = scene.time.rate;
    let fps = rate.as_f64();
    match mode {
        UsdAnimation::Off => None,
        _ if animated(scene).is_none() => None,
        UsdAnimation::CurrentFrame => {
            let f = animation::frames(rate, scene.time.current);
            Some(StageTime {
                fps,
                start: f,
                end: f,
            })
        }
        UsdAnimation::On => Some(StageTime {
            fps,
            start: animation::frames(rate, scene.time.start),
            end: animation::frames(rate, scene.time.end),
        }),
    }
}

/// The scene to sample: the document behind an evaluated copy, or the
/// scene itself; `None` when nothing in it can move.
fn animated(scene: &Scene) -> Option<&Scene> {
    let s = scene.animation.rig.source.as_deref().unwrap_or(scene);
    (!s.animation.channels.is_empty() || !s.animation.rig.pieces.is_empty()).then_some(s)
}

/// The sample times (frames) and the scene posed at each (TL-81).
pub(crate) fn frames(scene: &Scene, mode: UsdAnimation) -> Option<Frames> {
    if mode != UsdAnimation::On {
        return None;
    }
    // An evaluated copy carries the document scene it came from (§8): the
    // rig is evaluated from that at every sample time.
    let scene = animated(scene)?;
    let rate = scene.time.rate;
    let (a, b) = (scene.time.start, scene.time.end);
    let (fa, fb) = (rate.frame_at(a), rate.frame_at(b));
    let mut ticks: Vec<Tick> = (fa..=fb)
        .map(|f| rate.tick_of(f))
        .filter(|t| *t >= a && *t <= b)
        .collect();
    // Key times between frames.
    ticks.extend(
        scene
            .animation
            .key_times(None)
            .into_iter()
            .filter(|t| *t >= a && *t <= b && rate.snap(*t) != *t),
    );
    ticks.sort();
    ticks.dedup();
    let posed = ticks
        .iter()
        .map(|t| {
            // §8: rigged items export their evaluated transforms.
            let posed = animation::evaluate_at(scene, *t).into_owned();
            if posed.animation.rig.pieces.is_empty() {
                posed
            } else {
                operandi_core::rig_eval::baked_scene(&posed)
            }
        })
        .collect();
    Some(Frames {
        times: ticks.iter().map(|t| animation::frames(rate, *t)).collect(),
        rigged: operandi_core::rig_eval::rigged_items(scene),
        posed,
    })
}

fn group_animated(scene: &Scene, id: u64, group: &str) -> bool {
    ["X", "Y", "Z"].iter().any(|a| {
        scene
            .animation
            .is_animated(&ChannelKey::new(id, &format!("{group}.{a}")))
    })
}

/// Item `id`'s samples, or `None` when nothing of it is animated.
pub(crate) fn item_samples(frames: &Frames, scene: &Scene, id: u64) -> Option<Box<Samples>> {
    let rigged = frames.rigged.contains(&id);
    if !scene.animation.item_is_animated(id) && !rigged {
        return None;
    }
    let mut s = Samples {
        times: frames.times.clone(),
        ..Default::default()
    };
    let transform = |p: &Scene| p.item(id).map(|i| i.transform.clone()).unwrap_or_default();
    let take = |f: &dyn Fn(&operandi_core::scene::Transform) -> [f64; 3]| -> Vec<[f64; 3]> {
        frames.posed.iter().map(|p| f(&transform(p))).collect()
    };
    if rigged || group_animated(scene, id, "pos") {
        s.translate = Some(take(&|t| t.translation.to_array()));
    }
    if rigged || group_animated(scene, id, "rot") {
        let mut r = take(&|t| t.rotation.to_array().map(f64::to_degrees));
        if rigged {
            unwrap_degrees(&mut r);
        }
        s.rotate = Some(r);
    }
    if rigged || group_animated(scene, id, "scl") {
        s.scale = Some(take(&|t| t.scale.to_array()));
    }
    let item = scene.item(id)?;
    if item.camera.is_some() {
        let mut notes = Vec::new();
        let cams: Vec<crate::lights::CameraPrim> = frames
            .posed
            .iter()
            .filter_map(|p| {
                let c = p.item(id)?.camera.as_ref()?;
                Some(crate::lights::camera_prim(p, id, c, &mut notes))
            })
            .collect();
        if cams.len() == frames.posed.len() {
            type Get = fn(&crate::lights::CameraPrim) -> f32;
            let attrs: [(&'static str, Get); 5] = [
                ("focalLength", |c| c.focal_length),
                ("horizontalAperture", |c| c.horizontal_aperture),
                ("verticalAperture", |c| c.vertical_aperture),
                ("fStop", |c| c.f_stop),
                ("focusDistance", |c| c.focus_distance),
            ];
            for (name, get) in attrs {
                let values: Vec<f64> = cams.iter().map(|c| get(c) as f64).collect();
                if values.windows(2).any(|w| w[0] != w[1]) {
                    s.camera.push((name, values));
                }
            }
        }
    }
    if item.light.is_some() {
        let mut notes = Vec::new();
        let lights: Vec<crate::lights::LightPrim> = frames
            .posed
            .iter()
            .filter_map(|p| crate::lights::light_prim(p.item(id)?.light.as_ref()?, &mut notes))
            .collect();
        if lights.len() == frames.posed.len() {
            let intensity: Vec<[f64; 3]> = lights
                .iter()
                .map(|l| [l.intensity as f64, 0., 0.])
                .collect();
            if intensity.windows(2).any(|w| w[0] != w[1]) {
                s.light.push(("inputs:intensity", intensity));
            }
            let color: Vec<[f64; 3]> = lights.iter().map(|l| l.color.map(|c| c as f64)).collect();
            if color.windows(2).any(|w| w[0] != w[1]) {
                s.light.push(("inputs:color", color));
            }
        }
    }
    let any = s.translate.is_some()
        || s.rotate.is_some()
        || s.scale.is_some()
        || !s.camera.is_empty()
        || !s.light.is_empty();
    any.then(|| Box::new(s))
}

/// Rotations decomposed from matrices can jump by whole turns between
/// frames; each angle takes the turn nearest the frame before.
fn unwrap_degrees(r: &mut [[f64; 3]]) {
    for i in 1..r.len() {
        let prev = r[i - 1];
        for (v, p) in r[i].iter_mut().zip(prev) {
            *v += ((p - *v) / 360.).round() * 360.;
        }
    }
}

/// The samples as the bridge's JSON (`ou_set_samples`): the Xform's ops,
/// and attributes of its camera or light prim at `child`.
pub fn json(s: &Samples, order: u32, child: Option<&str>) -> String {
    use serde_json::json;
    let vec3 = |v: &Option<Vec<[f64; 3]>>| v.as_ref().map(|v| json!(v));
    let mut attrs = Vec::new();
    if let Some(child) = child {
        for (name, values) in &s.camera {
            attrs.push(json!({"path": child, "name": name, "type": "float", "values": values}));
        }
        for (name, values) in &s.light {
            if *name == "inputs:color" {
                attrs.push(
                    json!({"path": child, "name": name, "type": "color3f", "values": values}),
                );
            } else {
                let v: Vec<f64> = values.iter().map(|v| v[0]).collect();
                attrs.push(json!({"path": child, "name": name, "type": "float", "values": v}));
            }
        }
    }
    json!({
        "times": s.times,
        "order": order,
        "translate": vec3(&s.translate),
        "rotate": vec3(&s.rotate),
        "scale": vec3(&s.scale),
        "attributes": attrs,
    })
    .to_string()
}
