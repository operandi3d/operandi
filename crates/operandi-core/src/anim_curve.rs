//! Key curves (BEHAVIOR-TIME-KEYS TK-10..17, BEHAVIOR-TIMELINE TL-45/46):
//! the values one scalar channel takes over time.
//!
//! A curve is a list of keys sorted by time. Between two keys the value is
//! a cubic Hermite segment whose end slopes come from each key's slope type
//! (TL-46); before the first key the curve holds the first value, after the
//! last it holds the last (TK-13). Stepped curves (choices, on/off, TK-14)
//! hold each key's value until the next key.
//!
//! **Smooth Flat** (TK-12, the default for new keys): the first and last
//! keys are flat, and an inner key's slope is 1.5 × the slope of the line
//! through its two neighbours, zero at a peak or dip, and never more than
//! three times either side's own slope (so a segment with one flat end
//! can't overshoot its neighbour). This reproduces every measured value of
//! TK-13 (keys 0/1/3 at frames 0/10/20 give 0.225 per frame). The
//! near-equal-neighbour rule is provisional (TK-Q01) until research probes
//! more cases.
use crate::time_tick::Tick;
use serde::{Deserialize, Serialize};

/// How one side of a key shapes its segment (TL-46).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Slope {
    /// Smooth through the neighbours (may overshoot).
    Auto,
    /// TK-12: smooth, flat at the ends and at peaks, no overshoot.
    #[default]
    SmoothFlat,
    /// Auto, but flat where a neighbour has the same value.
    AutoFlat,
    /// Zero slope.
    Flat,
    /// Points straight at the neighbour key on this side.
    Linear,
    /// Hold the value until the next key (an out side's meaning; an in
    /// side marked Stepped is read as flat).
    Stepped,
    /// The key's own handle slope (phase 3 edits it).
    Manual,
}

impl Slope {
    /// Menu order and labels of TL-45 (Linear In / Out are per side).
    pub const NAMES: [(&'static str, Slope); 7] = [
        ("auto", Slope::Auto),
        ("smoothFlat", Slope::SmoothFlat),
        ("autoFlat", Slope::AutoFlat),
        ("flat", Slope::Flat),
        ("linear", Slope::Linear),
        ("stepped", Slope::Stepped),
        ("manual", Slope::Manual),
    ];
    pub fn parse(word: &str) -> Option<Slope> {
        Self::NAMES
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(word))
            .map(|(_, s)| *s)
    }
    pub fn name(self) -> &'static str {
        Self::NAMES
            .iter()
            .find(|(_, s)| *s == self)
            .map_or("smoothFlat", |(n, _)| n)
    }
}

fn is_default_slope(s: &Slope) -> bool {
    *s == Slope::SmoothFlat
}
fn is_zero(v: &f64) -> bool {
    *v == 0.
}
fn is_one(v: &f64) -> bool {
    *v == 1.
}
fn one() -> f64 {
    1.
}

/// One key (TK-11): a value pinned to a time, and how the curve leaves and
/// arrives at it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Key {
    pub time: Tick,
    pub value: f64,
    #[serde(default, skip_serializing_if = "is_default_slope")]
    pub slope_in: Slope,
    #[serde(default, skip_serializing_if = "is_default_slope")]
    pub slope_out: Slope,
    /// Manual handle slopes, value per second (used by `Slope::Manual`).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub manual_in: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub manual_out: f64,
    /// Handle weights (phase 3; kept so files don't change then).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub weight_in: f64,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub weight_out: f64,
    /// In and out handles move separately.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub broken: bool,
}

impl Key {
    /// A new key with the default Smooth Flat slopes (TK-12).
    pub fn new(time: Tick, value: f64) -> Key {
        Key {
            time,
            value,
            slope_in: Slope::SmoothFlat,
            slope_out: Slope::SmoothFlat,
            manual_in: 0.,
            manual_out: 0.,
            weight_in: 1.,
            weight_out: 1.,
            broken: false,
        }
    }
}

/// The keys of one scalar channel, sorted by time, no two at one time.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Curve {
    pub keys: Vec<Key>,
    /// A choice or on/off channel: always stepped (TK-14).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stepped: bool,
}

impl Curve {
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
    /// The key exactly at `t`.
    pub fn key_at(&self, t: Tick) -> Option<usize> {
        self.keys.binary_search_by(|k| k.time.cmp(&t)).ok()
    }
    /// Sets a key at `t`: replaces the value of one already there,
    /// keeping its slopes (TK-17), else adds a Smooth Flat key.
    pub fn set(&mut self, t: Tick, value: f64) {
        match self.keys.binary_search_by(|k| k.time.cmp(&t)) {
            Ok(i) => self.keys[i].value = value,
            Err(i) => self.keys.insert(i, Key::new(t, value)),
        }
    }
    /// Inserts `key` as is (replacing one at its time).
    pub fn put(&mut self, key: Key) {
        match self.keys.binary_search_by(|k| k.time.cmp(&key.time)) {
            Ok(i) => self.keys[i] = key,
            Err(i) => self.keys.insert(i, key),
        }
    }
    /// Removes the key at `t`; whether there was one.
    pub fn remove(&mut self, t: Tick) -> bool {
        match self.key_at(t) {
            Some(i) => {
                self.keys.remove(i);
                true
            }
            None => false,
        }
    }
    /// Re-sorts after keys' times changed; of keys landing on one time the
    /// last one listed wins.
    pub fn sort(&mut self) {
        self.keys.sort_by_key(|k| k.time);
        let mut out: Vec<Key> = Vec::with_capacity(self.keys.len());
        for k in self.keys.drain(..) {
            match out.last_mut() {
                Some(last) if last.time == k.time => *last = k,
                _ => out.push(k),
            }
        }
        self.keys = out;
    }

    /// The value at time `t` (seconds as ticks).
    pub fn value_at(&self, t: Tick) -> f64 {
        let keys = &self.keys;
        let Some(first) = keys.first() else {
            return 0.;
        };
        if t <= first.time {
            return first.value;
        }
        let last = keys.last().unwrap_or(first);
        if t >= last.time {
            return last.value;
        }
        // The segment [i, i+1] holding t.
        let i = match keys.binary_search_by(|k| k.time.cmp(&t)) {
            Ok(i) => return keys[i].value,
            Err(i) => i - 1,
        };
        let (a, b) = (&keys[i], &keys[i + 1]);
        if self.stepped || a.slope_out == Slope::Stepped {
            return a.value;
        }
        let h = (b.time - a.time).seconds();
        let s = (t - a.time).seconds() / h;
        let (m0, m1) = self.segment_slopes(i);
        hermite(a.value, b.value, m0 * h, m1 * h, s)
    }

    /// The slopes (value per second) leaving key `i` and arriving at key
    /// `i + 1`, after the no-overshoot limit.
    fn segment_slopes(&self, i: usize) -> (f64, f64) {
        let keys = &self.keys;
        let (a, b) = (&keys[i], &keys[i + 1]);
        let d = (b.value - a.value) / (b.time - a.time).seconds();
        let mut m0 = self.side_slope(i, a.slope_out, false);
        let mut m1 = self.side_slope(i + 1, b.slope_in, true);
        // Smooth Flat never overshoots inside the segment either: when both
        // ends lean in, scale them back onto the monotone region
        // (Fritsch-Carlson). Never touches a segment with a flat end.
        let limited = |s: Slope| s == Slope::SmoothFlat;
        if limited(a.slope_out) && limited(b.slope_in) && d != 0. {
            let (alpha, beta) = (m0 / d, m1 / d);
            let r = alpha * alpha + beta * beta;
            if r > 9. {
                let tau = 3. / r.sqrt();
                m0 *= tau;
                m1 *= tau;
            }
        }
        (m0, m1)
    }

    /// The slope at key `i` on its in side (`incoming`) or out side.
    fn side_slope(&self, i: usize, slope: Slope, incoming: bool) -> f64 {
        let keys = &self.keys;
        let k = &keys[i];
        let prev = i.checked_sub(1).map(|j| &keys[j]);
        let next = keys.get(i + 1);
        let rate = |a: &Key, b: &Key| (b.value - a.value) / (b.time - a.time).seconds();
        match slope {
            Slope::Flat | Slope::Stepped => 0.,
            Slope::Manual => {
                if incoming {
                    k.manual_in
                } else {
                    k.manual_out
                }
            }
            Slope::Linear => match (incoming, prev, next) {
                (true, Some(p), _) => rate(p, k),
                (false, _, Some(n)) => rate(k, n),
                _ => 0.,
            },
            Slope::SmoothFlat => match (prev, next) {
                (Some(p), Some(n)) => {
                    let (d1, d2) = (rate(p, k), rate(k, n));
                    if d1 * d2 <= 0. {
                        0.
                    } else {
                        let m = 1.5 * rate(p, n);
                        let cap = 3. * d1.abs().min(d2.abs());
                        m.clamp(-cap, cap)
                    }
                }
                _ => 0.,
            },
            Slope::Auto | Slope::AutoFlat => {
                let flat_here = slope == Slope::AutoFlat
                    && (prev.is_some_and(|p| same(p.value, k.value))
                        || next.is_some_and(|n| same(n.value, k.value)));
                if flat_here {
                    return 0.;
                }
                match (prev, next) {
                    (Some(p), Some(n)) => rate(p, n),
                    // Ends point at their one neighbour (provisional, TK-Q02).
                    (Some(p), None) => rate(p, k),
                    (None, Some(n)) => rate(k, n),
                    (None, None) => 0.,
                }
            }
        }
    }

    /// Every key time.
    pub fn times(&self) -> impl Iterator<Item = Tick> + '_ {
        self.keys.iter().map(|k| k.time)
    }
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.)
}

/// Cubic Hermite from `p0` to `p1` with end tangents `m0`, `m1` (already
/// scaled to the segment), at `s` in 0..1.
fn hermite(p0: f64, p1: f64, m0: f64, m1: f64, s: f64) -> f64 {
    let s2 = s * s;
    let s3 = s2 * s;
    (2. * s3 - 3. * s2 + 1.) * p0
        + (s3 - 2. * s2 + s) * m0
        + (-2. * s3 + 3. * s2) * p1
        + (s3 - s2) * m1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_tick::FrameRate;

    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-4, "{a} vs {b}");
    }

    #[test]
    fn tk13_two_keys_are_a_flat_ended_s_curve() {
        let mut c = Curve::default();
        c.set(Tick::from_seconds(0.), 0.);
        c.set(Tick::from_seconds(1.), 1.);
        near(c.value_at(Tick::from_seconds(0.25)), 0.15625);
        near(c.value_at(Tick::from_seconds(0.5)), 0.5);
        near(c.value_at(Tick::from_seconds(0.75)), 0.84375);
    }

    #[test]
    fn tk13_three_keys_at_30_fps_match_every_measured_value() {
        let r = FrameRate::FPS_30;
        let mut c = Curve::default();
        c.set(r.tick_of(0), 0.);
        c.set(r.tick_of(10), 1.);
        c.set(r.tick_of(20), 3.);
        for (frame, value) in [
            (2, 0.032),
            (5, 0.21875),
            (8, 0.608),
            (10, 1.0),
            (12, 1.496),
            (15, 2.28125),
            (18, 2.864),
            (-5, 0.),
            (25, 3.),
        ] {
            near(c.value_at(r.tick_of(frame)), value);
        }
        // The inner key's slope: 0.225 per frame.
        let per_second = c.side_slope(1, Slope::SmoothFlat, false);
        near(per_second / 30., 0.225);
    }

    #[test]
    fn smooth_flat_is_flat_at_a_peak_and_never_overshoots() {
        let r = FrameRate::FPS_24;
        let mut c = Curve::default();
        c.set(r.tick_of(0), 0.);
        c.set(r.tick_of(10), 5.);
        c.set(r.tick_of(20), 0.);
        assert_eq!(c.side_slope(1, Slope::SmoothFlat, false), 0.);
        // A near-flat step then a rise: nothing goes below 0 or above 1.
        let mut c = Curve::default();
        c.set(r.tick_of(0), 0.);
        c.set(r.tick_of(10), 0.999);
        c.set(r.tick_of(11), 1.);
        c.set(r.tick_of(30), 10.);
        for f in 0..300 {
            let v = c.value_at(Tick(r.tick_of(0).0 + f * r.frame_duration().0 / 10));
            assert!((-1e-9..=10. + 1e-9).contains(&v), "{f}: {v}");
        }
        for f in 0..100 {
            let v = c.value_at(Tick(f * r.frame_duration().0 / 10));
            assert!(v <= 0.999 + 1e-9, "segment 1 overshoots at {f}: {v}");
        }
    }

    #[test]
    fn linear_stepped_and_choice_curves() {
        let r = FrameRate::FPS_24;
        let mut c = Curve::default();
        c.set(r.tick_of(0), 2.);
        c.set(r.tick_of(12), 0.);
        c.set(r.tick_of(24), 2.);
        c.keys[1].slope_in = Slope::Linear;
        c.keys[1].slope_out = Slope::Linear;
        // Linear into the contact: the value just before is on the line's
        // tangent, so the curve is sharp at f12 (in slope -2/12 per frame).
        let m_in = c.side_slope(1, Slope::Linear, true);
        near(m_in, (0. - 2.) / 0.5);
        c.keys[0].slope_out = Slope::Stepped;
        near(c.value_at(r.tick_of(11)), 2.);
        near(c.value_at(r.tick_of(12)), 0.);
        let mut choice = Curve {
            stepped: true,
            ..Default::default()
        };
        choice.set(r.tick_of(0), 1.);
        choice.set(r.tick_of(10), 3.);
        near(choice.value_at(r.tick_of(9)), 1.);
        near(choice.value_at(r.tick_of(10)), 3.);
    }

    #[test]
    fn setting_an_existing_key_keeps_its_slopes() {
        let mut c = Curve::default();
        c.set(Tick(0), 1.);
        c.keys[0].slope_out = Slope::Linear;
        c.set(Tick(0), 4.);
        assert_eq!(c.keys.len(), 1);
        assert_eq!(c.keys[0].value, 4.);
        assert_eq!(c.keys[0].slope_out, Slope::Linear);
    }
}
