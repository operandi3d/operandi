//! Preferences > Defaults > Application > Use OS Color Picker for the
//! shared colour field (colour_field.rs). Off (the default), a click opens
//! Operandi's colour picker. On, it opens the system colour panel (macOS
//! NSColorPanel) and the field follows it while the panel stays open; other
//! platforms keep Operandi's picker.
use crate::native::color_panel;
use eframe::egui::Id;
use std::cell::Cell;

thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    /// The swatch the system panel is open for.
    static OWNER: Cell<Option<Id>> = const { Cell::new(None) };
}

pub fn set_enabled(on: bool) {
    ENABLED.with(|c| c.set(on));
    if !on && OWNER.with(|o| o.take()).is_some() {
        color_panel::close();
    }
}
fn enabled() -> bool {
    ENABLED.with(Cell::get) && cfg!(target_os = "macos")
}

/// The shared colour field's click with the preference on: open the system
/// panel for field `owner` on linear `rgb`. False (nothing opened) when the
/// preference is off, so the field opens its own picker.
pub(crate) fn open_system_for(owner: Id, rgb: [f64; 3]) -> bool {
    if !enabled() {
        return false;
    }
    let srgb = rgb.map(|c| operandi_core::colour_math::linear_to_srgb(c.clamp(0., 1.)) as f32);
    if color_panel::open(srgb) {
        OWNER.with(|o| o.set(Some(owner)));
        true
    } else {
        false
    }
}

/// Whether the system panel is open for `owner`.
pub(crate) fn system_open_for(owner: Id) -> bool {
    OWNER.with(Cell::get) == Some(owner)
}

/// While the system panel is open for `owner`: its colour (linear) when it
/// differs from `rgb`.
pub(crate) fn follow_system(owner: Id, rgb: [f64; 3]) -> Option<[f64; 3]> {
    if OWNER.with(Cell::get) != Some(owner) {
        return None;
    }
    match color_panel::poll() {
        Some(picked) => {
            let linear = picked.map(|c| operandi_core::colour_math::srgb_to_linear(f64::from(c)));
            let now = rgb.map(|c| operandi_core::colour_math::linear_to_srgb(c.clamp(0., 1.)));
            picked
                .iter()
                .zip(now)
                .any(|(&p, n)| (f64::from(p) - n).abs() > 1. / 512.)
                .then_some(linear)
        }
        None => {
            OWNER.with(|o| o.set(None));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_panel_opens_only_with_the_preference() {
        let owner = Id::new("field");
        set_enabled(false);
        assert!(!enabled());
        assert!(!open_system_for(owner, [0.5; 3]));
        assert!(!system_open_for(owner));
        assert_eq!(follow_system(owner, [0.5; 3]), None);
        set_enabled(true);
        assert_eq!(enabled(), cfg!(target_os = "macos"));
        set_enabled(false);
    }
}
