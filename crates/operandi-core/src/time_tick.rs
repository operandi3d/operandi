//! Exact time (F4 Time foundation, Jason 2026-11-17): every time is a
//! whole number of [`Tick`]s, and a frame rate is an exact fraction.
//!
//! 254 016 000 000 ticks per second divides evenly into every common frame
//! duration (23.976, 24, 25, 29.97, 30, 48, 50, 59.94, 60, 119.88, 120 fps)
//! and the usual audio rates, so frame `n` at any of them is an exact tick
//! and keys, frames and sim steps never drift by rounding.
//!
//! Adapted from craft-libs `crates/time/src/lib.rs` (github.com/storytold/
//! craft-libs at 42bb9d35), the `Tick` and `FrameRate` part only: MIT OR
//! Apache-2.0, Copyright (c) 2026 ArtCraft Team and the craft-libs
//! contributors (docs/licenses/craft-libs-LICENSE-MIT.txt). Changes: cut to
//! what Operandi uses, `From`/serde forms, Operandi's comments.
use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

/// Clock ticks per second: 254 016 000 000.
pub const TICKS_PER_SECOND: i64 = 254_016_000_000;

/// A point or length of time, in ticks. Saved as the plain integer.
/// Arithmetic saturates at the storage limits; results in range are exact.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Tick(pub i64);

impl Tick {
    pub const ZERO: Tick = Tick(0);
    /// Seconds to ticks, rounded to the nearest tick (saturating).
    pub fn from_seconds(s: f64) -> Tick {
        Tick((s * TICKS_PER_SECOND as f64).round() as i64)
    }
    pub fn seconds(self) -> f64 {
        self.0 as f64 / TICKS_PER_SECOND as f64
    }
}

impl Add for Tick {
    type Output = Tick;
    fn add(self, o: Tick) -> Tick {
        Tick(self.0.saturating_add(o.0))
    }
}

impl Sub for Tick {
    type Output = Tick;
    fn sub(self, o: Tick) -> Tick {
        Tick(self.0.saturating_sub(o.0))
    }
}

/// A frame rate as an exact fraction: `num / den` frames per second.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRate {
    pub num: i64,
    pub den: i64,
}

impl FrameRate {
    pub const FPS_23_976: FrameRate = FrameRate {
        num: 24000,
        den: 1001,
    };
    pub const FPS_24: FrameRate = FrameRate { num: 24, den: 1 };
    pub const FPS_25: FrameRate = FrameRate { num: 25, den: 1 };
    pub const FPS_29_97: FrameRate = FrameRate {
        num: 30000,
        den: 1001,
    };
    pub const FPS_30: FrameRate = FrameRate { num: 30, den: 1 };
    pub const FPS_48: FrameRate = FrameRate { num: 48, den: 1 };
    pub const FPS_50: FrameRate = FrameRate { num: 50, den: 1 };
    pub const FPS_59_94: FrameRate = FrameRate {
        num: 60000,
        den: 1001,
    };
    pub const FPS_60: FrameRate = FrameRate { num: 60, den: 1 };
    pub const FPS_119_88: FrameRate = FrameRate {
        num: 120000,
        den: 1001,
    };
    pub const FPS_120: FrameRate = FrameRate { num: 120, den: 1 };

    /// The standard film, video and game rates.
    pub const COMMON: [FrameRate; 11] = [
        Self::FPS_23_976,
        Self::FPS_24,
        Self::FPS_25,
        Self::FPS_29_97,
        Self::FPS_30,
        Self::FPS_48,
        Self::FPS_50,
        Self::FPS_59_94,
        Self::FPS_60,
        Self::FPS_119_88,
        Self::FPS_120,
    ];

    /// `num / den`, reduced.
    pub fn new(num: i64, den: i64) -> Self {
        let g = gcd(num.unsigned_abs(), den.unsigned_abs()).max(1) as i128;
        FrameRate {
            num: (num as i128 / g) as i64,
            den: (den as i128 / g) as i64,
        }
    }
    /// The standard rate within 0.005 fps of `fps` (so 29.97 is exactly
    /// 30000/1001), otherwise `fps` to a thousandth.
    pub fn from_f64(fps: f64) -> Self {
        for r in Self::COMMON {
            if (r.as_f64() - fps).abs() < 0.005 {
                return r;
            }
        }
        FrameRate::new((fps * 1000.0).round() as i64, 1000)
    }
    pub fn as_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }
    pub fn is_valid(self) -> bool {
        self.num > 0 && self.den > 0
    }
    /// One frame's length (floored for rates that don't divide evenly).
    pub fn frame_duration(self) -> Tick {
        Tick(clamp_i64(
            (TICKS_PER_SECOND as i128 * self.den as i128) / self.num.max(1) as i128,
        ))
    }
    /// The frame `t` lies in (floor).
    pub fn frame_at(self, t: Tick) -> i64 {
        clamp_i64(frame_index(self, t))
    }
    /// The first tick of frame `f`.
    pub fn tick_of(self, f: i64) -> Tick {
        Tick(clamp_i64(frame_boundary(self, f as i128)))
    }
    /// `t` moved down to the start of its frame.
    pub fn snap(self, t: Tick) -> Tick {
        Tick(clamp_i64(frame_boundary(self, frame_index(self, t))))
    }
    /// "24", "29.97".
    pub fn label(self) -> String {
        if self.den == 1 {
            format!("{}", self.num)
        } else {
            let s = format!("{:.3}", self.as_f64());
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
    }
}

fn clamp_i64(value: i128) -> i64 {
    value.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

fn frame_index(rate: FrameRate, t: Tick) -> i128 {
    (t.0 as i128 * rate.num as i128).div_euclid(TICKS_PER_SECOND as i128 * rate.den.max(1) as i128)
}

fn frame_boundary(rate: FrameRate, frame: i128) -> i128 {
    let Some(n) = frame
        .checked_mul(TICKS_PER_SECOND as i128)
        .and_then(|n| n.checked_mul(rate.den as i128))
    else {
        return if frame < 0 { i128::MIN } else { i128::MAX };
    };
    let d = rate.num.max(1) as i128;
    n.div_euclid(d) + i128::from(n.rem_euclid(d) != 0)
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_common_rate_has_whole_tick_frames() {
        for rate in FrameRate::COMMON {
            let d = rate.frame_duration();
            // Exact: rate × duration is one second, no remainder.
            assert_eq!(
                (d.0 as i128 * rate.num as i128) % (rate.den as i128),
                0,
                "{rate:?}"
            );
            for f in [-7, 0, 1, 24, 1001, 86_400] {
                assert_eq!(rate.frame_at(rate.tick_of(f)), f, "{rate:?} {f}");
                assert_eq!(rate.tick_of(f).0, d.0 * f, "no drift at {f}");
            }
        }
    }

    #[test]
    fn fractional_rates_snap_to_the_standard_fraction() {
        assert_eq!(FrameRate::from_f64(29.97), FrameRate::FPS_29_97);
        assert_eq!(FrameRate::from_f64(24.0), FrameRate::FPS_24);
        assert_eq!(FrameRate::from_f64(12.5), FrameRate::new(25, 2));
        assert_eq!(FrameRate::FPS_29_97.label(), "29.97");
    }

    #[test]
    fn seconds_round_trip_and_snap() {
        let t = Tick::from_seconds(2.5);
        assert_eq!(t.seconds(), 2.5);
        let r = FrameRate::FPS_24;
        assert_eq!(r.snap(Tick(r.tick_of(3).0 + 5)), r.tick_of(3));
    }
}
