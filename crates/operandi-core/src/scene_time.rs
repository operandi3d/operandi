//! The scene's time settings (AR-02, master plan phase 0 door 2): frame
//! rate, start and end of the scene's time range, and the current time.
//!
//! Nothing animates yet. These are saved so that time is part of every
//! scene file from now on, and `evaluate` (evaluate.rs) already takes the
//! current time, so keys, sims and turntables later only add readers.
//! Times are exact ticks and the rate an exact fraction (time_tick.rs), so
//! frame 24 at 29.97 fps is one exact time, never a rounded float. Scripts
//! give and read seconds and fps (`29.97` becomes 30000/1001). Values are
//! Operandi's own (provisional): 24 fps, 0 to 5 seconds (120 frames).
use crate::time_tick::{FrameRate, Tick};
use serde::{Deserialize, Serialize};

/// Frame-rate range accepted.
pub const FPS_RANGE: std::ops::RangeInclusive<f64> = 0.001..=1000.;
/// Largest time (seconds, either sign) accepted: about 11.5 days.
pub const MAX_SECONDS: f64 = 1e6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTime {
    /// Frames per second, as an exact fraction.
    pub rate: FrameRate,
    /// The time range's start.
    pub start: Tick,
    /// The time range's end.
    pub end: Tick,
    /// The current time. May lie outside the range.
    pub current: Tick,
}

impl Default for SceneTime {
    fn default() -> Self {
        let rate = FrameRate::FPS_24;
        Self {
            rate,
            start: Tick::ZERO,
            end: rate.tick_of(120),
            current: Tick::ZERO,
        }
    }
}

impl SceneTime {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
    pub fn validate(&self) -> crate::Result<()> {
        if !self.rate.is_valid() || !FPS_RANGE.contains(&self.rate.as_f64()) {
            return Err(format!(
                "Frame rate must be between {} and {} fps",
                FPS_RANGE.start(),
                FPS_RANGE.end()
            ));
        }
        for (name, t) in [
            ("Start", self.start),
            ("End", self.end),
            ("Current time", self.current),
        ] {
            if t.seconds().abs() > MAX_SECONDS {
                return Err(format!("{name} is out of range"));
            }
        }
        if self.start > self.end {
            return Err("The time range's start is after its end".into());
        }
        Ok(())
    }
    /// The frame the current time lies in.
    pub fn current_frame(&self) -> i64 {
        self.rate.frame_at(self.current)
    }
}

impl crate::Editor {
    /// `Command::SetSceneTime`: rate and range are one "Scene Time" undo
    /// step; the current time alone is not an undo step (TK-05, UIC-225)
    /// and poses the scene there (anim_edit.rs).
    pub(crate) fn set_scene_time(&mut self, time: SceneTime) -> crate::Result<()> {
        time.validate()?;
        if self.scene.time == time {
            return Ok(());
        }
        let now = self.scene.time.current;
        let settings = SceneTime {
            current: now,
            ..time
        };
        if settings != self.scene.time {
            let mut candidate = self.scene.clone();
            candidate.time = settings;
            let before = std::mem::replace(&mut self.scene, candidate);
            self.record("Scene Time".into(), before);
        }
        if time.current != now {
            self.set_current_time(time.current, false)?;
        }
        Ok(())
    }
}

// ------------------------------------------------------------ channels

use crate::channels::{Channel, ChannelType, ChannelValue, Target};

fn is_scene(_: &crate::Scene, target: Target) -> bool {
    target == Target::Scene
}

fn number(v: ChannelValue) -> crate::channels::Result<f64> {
    match v {
        ChannelValue::Float(f) if f.is_finite() => Ok(f),
        ChannelValue::Int(n) => Ok(n as f64),
        _ => Err("Expected a number".into()),
    }
}

/// A time given in seconds, as ticks (refusing what can't be one).
fn ticks(v: ChannelValue) -> crate::channels::Result<Tick> {
    let s = number(v)?;
    if s.abs() > MAX_SECONDS {
        return Err("Time is out of range".into());
    }
    Ok(Tick::from_seconds(s))
}

macro_rules! time_channel {
    ($name:literal, $field:ident) => {
        Channel {
            name: $name,
            ty: ChannelType::Float,
            applies: is_scene,
            get: |s, _| Some(ChannelValue::Float(s.time.$field.seconds())),
            set: |s, _, v| {
                let mut time = s.time;
                time.$field = ticks(v)?;
                Ok(crate::Command::SetSceneTime(time))
            },
        }
    };
}

/// `item.channel ... scene:true`: the scene's time settings (UIC-163/165:
/// read and set through the general setter, no command of their own).
/// Names are Operandi's own (provisional). Times in seconds, stored as
/// exact ticks; `frame` reads and sets the current time as a whole frame.
pub static CHANNELS: &[Channel] = &[
    Channel {
        name: "fps",
        ty: ChannelType::Float,
        applies: is_scene,
        get: |s, _| Some(ChannelValue::Float(s.time.rate.as_f64())),
        set: |s, _, v| {
            let mut time = s.time;
            let fps = number(v)?;
            if !FPS_RANGE.contains(&fps) {
                return Err(format!(
                    "Frame rate must be between {} and {} fps",
                    FPS_RANGE.start(),
                    FPS_RANGE.end()
                ));
            }
            time.rate = FrameRate::from_f64(fps);
            Ok(crate::Command::SetSceneTime(time))
        },
    },
    time_channel!("start", start),
    time_channel!("end", end),
    time_channel!("time", current),
    Channel {
        name: "frame",
        ty: ChannelType::Int,
        applies: is_scene,
        get: |s, _| Some(ChannelValue::Int(s.time.current_frame())),
        set: |s, _, v| {
            let mut time = s.time;
            let ChannelValue::Int(frame) = v else {
                return Err("Expected a whole frame".into());
            };
            time.current = time.rate.tick_of(frame);
            Ok(crate::Command::SetSceneTime(time))
        },
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        let t = SceneTime::default();
        assert!(t.validate().is_ok());
        assert_eq!(t.rate.frame_at(t.end), 120);
        assert_eq!(t.end.seconds(), 5.);
    }

    #[test]
    fn refuses_bad_values() {
        let bad = [
            SceneTime {
                rate: FrameRate { num: 0, den: 1 },
                ..Default::default()
            },
            SceneTime {
                start: Tick::from_seconds(2.),
                end: Tick::from_seconds(1.),
                ..Default::default()
            },
            SceneTime {
                current: Tick::from_seconds(2. * MAX_SECONDS),
                ..Default::default()
            },
        ];
        for t in bad {
            assert!(t.validate().is_err(), "{t:?}");
        }
    }
}
