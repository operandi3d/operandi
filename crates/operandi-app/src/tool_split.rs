//! The left panel's split between the tool list (top) and Tool Properties
//! (bottom), with a draggable divider between them (Jason, 2026-11-08).
//!
//! The split is stored as the fraction of the panel body (both areas, not
//! the divider) that Tool Properties takes. Heights are clamped when drawn,
//! so a short window still shows a little of both areas.

/// Tool Properties' share of the panel body when nothing is saved.
pub(super) const DEFAULT_FRACTION: f32 = 0.4;
/// When a tool becomes active, Tool Properties grows to at least this
/// share of the panel body; a larger share is left alone.
pub(super) const TAKE_OVER_FRACTION: f32 = 0.5;
/// The tool list never gets shorter than this (points) while there is room.
pub(super) const LIST_MIN_HEIGHT: f32 = 96.;
/// Tool Properties (header included) never gets shorter than this.
pub(super) const PROPS_MIN_HEIGHT: f32 = 72.;
/// The divider's drawn height, the same as the Scenes/Properties divider.
pub(super) const DIVIDER_HEIGHT: f32 = 4.;

/// Tool Properties' height in a body `total` points tall for a stored
/// `fraction`, with the tool list at least `list_min` tall (the toolbox
/// passes the length its tab strip needs, never below `LIST_MIN_HEIGHT`).
/// Both areas keep their minimums while they fit; a body too short for
/// both splits it evenly.
pub(super) fn props_height(total: f32, fraction: f32, list_min: f32) -> f32 {
    let total = total.max(0.);
    let low = PROPS_MIN_HEIGHT.min(total / 2.);
    let list_min = list_min.max(LIST_MIN_HEIGHT);
    // While both minimums fit, the list keeps its own; otherwise fall back
    // to the even split.
    let high = if total >= low + list_min {
        total - list_min
    } else {
        (total / 2.).max(low)
    };
    (total * fraction.clamp(0., 1.)).clamp(low, high)
}

/// The fraction after dragging the divider `dy` points (down is positive)
/// from `start`, clamped so both areas keep their minimums.
pub(super) fn dragged(start: f32, dy: f32, total: f32, list_min: f32) -> f32 {
    if total <= 0. {
        return start;
    }
    props_height(total, start - dy / total, list_min) / total
}

/// The take-over rule: below `TAKE_OVER_FRACTION` grows to it, above stays.
pub(super) fn take_over(fraction: f32) -> f32 {
    fraction.max(TAKE_OVER_FRACTION)
}

/// A saved fraction, or the default when missing or damaged.
pub(super) fn restored(saved: Option<f64>) -> f32 {
    saved
        .filter(|f| f.is_finite())
        .map_or(DEFAULT_FRACTION, |f| (f as f32).clamp(0., 1.))
}

/// True when `active` names a tool that wasn't the active one last time
/// this ran (`seen`), which is when the take-over applies. Dropping the
/// tool (`None`) only forgets it, so activating it again counts.
pub(super) fn activated(seen: &mut Option<&'static str>, active: Option<&'static str>) -> bool {
    let fresh = active.is_some() && active != *seen;
    *seen = active;
    fresh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn props_height_keeps_both_minimums_and_splits_short_bodies_evenly() {
        assert_eq!(props_height(600., 0.4, 0.), 240.);
        // Too small a share keeps Tool Properties' minimum.
        assert_eq!(props_height(600., 0.0, 0.), PROPS_MIN_HEIGHT);
        // Too large a share keeps the tool list's minimum.
        assert_eq!(props_height(600., 1.0, 0.), 600. - LIST_MIN_HEIGHT);
        // A body shorter than both minimums is split evenly.
        assert_eq!(props_height(100., 0.9, 0.), 50.);
        assert_eq!(props_height(100., 0.1, 0.), 50.);
        assert_eq!(props_height(-5., 0.5, 0.), 0.);
    }

    #[test]
    fn a_taller_list_minimum_holds_the_drawer_open() {
        // The tab strip needs 350: Tool Properties gets the rest at most.
        assert_eq!(props_height(600., 1.0, 350.), 250.);
        assert_eq!(props_height(600., 0.3, 350.), 180.);
        // A smaller minimum than LIST_MIN_HEIGHT changes nothing.
        assert_eq!(props_height(600., 1.0, 10.), 600. - LIST_MIN_HEIGHT);
        // Too short a body still splits evenly.
        assert_eq!(props_height(300., 0.9, 350.), 150.);
        assert!((dragged(0.3, -200., 600., 350.) - 250. / 600.).abs() < 1e-6);
    }

    #[test]
    fn dragging_up_grows_tool_properties_and_clamps() {
        let up = dragged(0.4, -60., 600., 0.);
        assert!((up - 0.5).abs() < 1e-6);
        let down = dragged(0.4, 60., 600., 0.);
        assert!((down - 0.3).abs() < 1e-6);
        assert!((dragged(0.4, 10_000., 600., 0.) - PROPS_MIN_HEIGHT / 600.).abs() < 1e-6);
        assert!((dragged(0.4, -10_000., 600., 0.) - (600. - LIST_MIN_HEIGHT) / 600.).abs() < 1e-6);
        assert_eq!(dragged(0.4, 50., 0., 0.), 0.4);
    }

    #[test]
    fn take_over_grows_below_half_and_leaves_more_alone() {
        assert_eq!(take_over(0.2), TAKE_OVER_FRACTION);
        assert_eq!(take_over(0.5), 0.5);
        assert_eq!(take_over(0.7), 0.7);
    }

    #[test]
    fn activation_fires_on_each_new_tool_and_again_after_a_drop() {
        let mut seen = None;
        assert!(!activated(&mut seen, None));
        assert!(activated(&mut seen, Some("Bevel")));
        assert!(!activated(&mut seen, Some("Bevel")));
        assert!(activated(&mut seen, Some("Extrude")));
        assert!(!activated(&mut seen, None));
        assert!(activated(&mut seen, Some("Extrude")));
    }

    #[test]
    fn restored_fraction_defaults_and_clamps() {
        assert_eq!(restored(None), DEFAULT_FRACTION);
        assert_eq!(restored(Some(f64::NAN)), DEFAULT_FRACTION);
        assert_eq!(restored(Some(0.65)), 0.65);
        assert_eq!(restored(Some(3.)), 1.);
    }
}
