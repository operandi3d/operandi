//! Mini-slider detents (UIC-54, MN-06.1): while a number field's arrows
//! are dragged, the value stops briefly on "nice" numbers.
//!
//! | Kind | Strong stops | Light stops |
//! |---|---|---|
//! | Angle | 0, every 90° | every 30° and 45° |
//! | Percent | 0, every 100% | every 25% |
//! | Distance | 0, powers of ten | round grid steps (2..9 × each power of ten) |
//! | Plain number | 0, powers of ten | 2, 2.5 and 5 × each power of ten |
//!
//! A strong stop holds the value for 8 px of pointer travel, a light one
//! for 4 px; the travel past the hold carries on, so nothing is lost.
//! Whole-number fields step whole numbers already and have none.
//!
//! Provisional (Operandi's reading, MN-06 note): "round grid steps" for
//! distances are 2..9 × each power of ten; stops closer together on
//! screen than [`MIN_SPACING`] at the drag's current speed are skipped,
//! so fine values near zero don't stick on every pixel.
use crate::scrub::Kind;

/// Stops nearer than this many points of travel to their neighbours at
/// the current drag speed are left out.
pub(crate) const MIN_SPACING: f64 = 12.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Strength {
    Strong,
    Light,
}
impl Strength {
    /// Points of travel the stop holds the value for (UIC-54).
    pub(crate) fn hold(self) -> f32 {
        match self {
            Self::Strong => 8.,
            Self::Light => 4.,
        }
    }
}

/// Every stop of `kind` between `lo` and `hi` (inclusive), for a drag
/// moving `step` units per point.
pub(crate) fn stops(kind: Kind, lo: f64, hi: f64, step: f64) -> Vec<(f64, Strength)> {
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    let step = step.abs().max(1e-12);
    let mut out: Vec<(f64, Strength)> = Vec::new();
    let mut push = |v: f64, s: Strength| {
        if v >= lo - 1e-12 && v <= hi + 1e-12 && !out.iter().any(|(o, _)| (*o - v).abs() < 1e-9) {
            out.push((v, s));
        }
    };
    let wide = |spacing: f64| spacing / step >= MIN_SPACING;
    // Multiples of `every` inside the range, at most a few hundred.
    let multiples = |every: f64| {
        let first = (lo / every).ceil() as i64;
        let last = (hi / every).floor() as i64;
        (first..=last.min(first + 400)).map(move |n| n as f64 * every)
    };
    match kind {
        Kind::Count => {}
        Kind::Angle | Kind::Percent => {
            let (strong, light): (f64, &[f64]) = if kind == Kind::Angle {
                (90., &[30., 45.])
            } else {
                (100., &[25.])
            };
            push(0., Strength::Strong);
            if wide(strong) {
                multiples(strong).for_each(|v| push(v, Strength::Strong));
            }
            for &every in light {
                if wide(every) {
                    multiples(every).for_each(|v| push(v, Strength::Light));
                }
            }
        }
        Kind::Distance | Kind::Scalar => {
            push(0., Strength::Strong);
            let light: &[f64] = if kind == Kind::Distance {
                &[2., 3., 4., 5., 6., 7., 8., 9.]
            } else {
                &[2., 2.5, 5.]
            };
            for k in -6..=6 {
                let decade = 10f64.powi(k);
                if wide(decade * 0.9) {
                    push(decade, Strength::Strong);
                    push(-decade, Strength::Strong);
                }
                // Light stops are a decade (Distance) or half a decade
                // (plain numbers) apart at most.
                let spacing = if kind == Kind::Distance {
                    decade
                } else {
                    decade * 0.5
                };
                if wide(spacing) {
                    for &m in light {
                        push(m * decade, Strength::Light);
                        push(-m * decade, Strength::Light);
                    }
                }
            }
        }
    }
    out
}

/// The first stop a move from `before` to `after` reaches (not `before`
/// itself, so a stop just let go of isn't caught again).
pub(crate) fn first_reached(
    kind: Kind,
    before: f64,
    after: f64,
    step: f64,
) -> Option<(f64, Strength)> {
    if (after - before).abs() < 1e-15 {
        return None;
    }
    stops(kind, before, after, step)
        .into_iter()
        .filter(|(v, _)| (v - before).abs() > 1e-9)
        .min_by(|a, b| (a.0 - before).abs().total_cmp(&(b.0 - before).abs()))
}

/// A stop holding the value, and how much travel it still holds for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Held {
    pub(crate) stop: f64,
    pub(crate) left: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(kind: Kind, lo: f64, hi: f64, step: f64, v: f64) -> Option<Strength> {
        stops(kind, lo, hi, step)
            .into_iter()
            .find(|(s, _)| (s - v).abs() < 1e-9)
            .map(|(_, s)| s)
    }

    #[test]
    fn angle_stops_strong_at_0_and_90s_light_at_30s_and_45s() {
        let s = 0.5;
        assert_eq!(has(Kind::Angle, -200., 200., s, 0.), Some(Strength::Strong));
        assert_eq!(
            has(Kind::Angle, -200., 200., s, 90.),
            Some(Strength::Strong)
        );
        assert_eq!(
            has(Kind::Angle, -200., 200., s, -180.),
            Some(Strength::Strong)
        );
        assert_eq!(has(Kind::Angle, -200., 200., s, 30.), Some(Strength::Light));
        assert_eq!(has(Kind::Angle, -200., 200., s, 45.), Some(Strength::Light));
        assert_eq!(
            has(Kind::Angle, -200., 200., s, 135.),
            Some(Strength::Light)
        );
        assert_eq!(has(Kind::Angle, -200., 200., s, 20.), None);
    }

    #[test]
    fn percent_stops_strong_at_hundreds_light_at_quarters() {
        let s = 0.5;
        assert_eq!(
            has(Kind::Percent, 0., 300., s, 100.),
            Some(Strength::Strong)
        );
        assert_eq!(has(Kind::Percent, 0., 300., s, 25.), Some(Strength::Light));
        assert_eq!(has(Kind::Percent, 0., 300., s, 175.), Some(Strength::Light));
        assert_eq!(has(Kind::Percent, 0., 300., s, 30.), None);
    }

    #[test]
    fn plain_numbers_stop_at_powers_of_ten_and_2_25_5() {
        let s = 0.01;
        assert_eq!(has(Kind::Scalar, -20., 20., s, 1.), Some(Strength::Strong));
        assert_eq!(has(Kind::Scalar, -20., 20., s, 10.), Some(Strength::Strong));
        assert_eq!(has(Kind::Scalar, -20., 20., s, -1.), Some(Strength::Strong));
        for v in [2., 2.5, 5., 20.] {
            assert_eq!(
                has(Kind::Scalar, -30., 30., s, v),
                Some(Strength::Light),
                "{v}"
            );
        }
        assert_eq!(has(Kind::Scalar, -20., 20., s, 3.), None);
        // 0.1 apart at 0.01 per point is 10 points: too close, left out.
        assert_eq!(has(Kind::Scalar, -1., 1., s, 0.25), None);
    }

    #[test]
    fn distances_stop_on_round_grid_steps() {
        let s = 0.01;
        assert_eq!(has(Kind::Distance, 0., 10., s, 1.), Some(Strength::Strong));
        assert_eq!(has(Kind::Distance, 0., 10., s, 3.), Some(Strength::Light));
        assert_eq!(has(Kind::Distance, 0., 10., s, 0.), Some(Strength::Strong));
        assert_eq!(has(Kind::Distance, 0., 10., s, 2.5), None);
    }

    #[test]
    fn counts_have_no_stops() {
        assert!(stops(Kind::Count, -10., 10., 0.2).is_empty());
    }

    #[test]
    fn the_first_stop_in_the_direction_of_travel() {
        assert_eq!(
            first_reached(Kind::Angle, 80., 100., 0.5),
            Some((90., Strength::Strong))
        );
        assert_eq!(
            first_reached(Kind::Angle, 100., 20., 0.5),
            Some((90., Strength::Strong))
        );
        // Leaving a stop doesn't catch it again.
        assert_eq!(first_reached(Kind::Angle, 90., 91., 0.5), None);
    }
}
