//! The Python scripting switch: scripting's UI stays hidden from normal
//! users until it is ready (the anim_switch.rs pattern). One function
//! decides it: [`enabled`]. Launch day = flip [`DEFAULT_ON`].
//!
//! Off hides the Script Console, the System menu's script entries and
//! refuses `operandi.python.*` lines and the bridge's `python_run`. The
//! commands stay registered either way, so the switch's refusal is what a
//! script or a typed line sees.
//!
//! On: Preferences > Defaults > Application > Experimental > Python
//! scripting (preview), or `OPERANDI_PYTHON=1` in the environment (it wins
//! over the preference; `OPERANDI_PYTHON=0` forces it off).
use std::cell::Cell;

/// Whether scripting shows when neither the preference nor the
/// environment says otherwise.
pub(crate) const DEFAULT_ON: bool = false;

/// The environment variable that overrides everything.
#[cfg_attr(test, allow(dead_code))]
pub(crate) const ENV: &str = "OPERANDI_PYTHON";

/// What a refused line says.
pub(crate) const OFF: &str = "Python scripting is off: turn on Preferences > Defaults > Application > Experimental > Python scripting (preview), or start Operandi with OPERANDI_PYTHON=1";

thread_local! {
    static PREFERENCE: Cell<Option<bool>> = const { Cell::new(None) };
    /// Tests: on unless a test turns it off.
    #[cfg(test)]
    static TEST: Cell<bool> = const { Cell::new(true) };
}

/// Whether scripting's UI shows and its commands run.
pub(crate) fn enabled() -> bool {
    #[cfg(test)]
    {
        TEST.with(Cell::get)
    }
    #[cfg(not(test))]
    {
        from(
            std::env::var(ENV).ok().as_deref(),
            PREFERENCE.with(Cell::get),
        )
    }
}

/// The rule: the environment wins, then the preference, then the default.
pub(crate) fn from(env: Option<&str>, preference: Option<bool>) -> bool {
    match env.map(str::trim) {
        Some("1" | "true" | "on" | "yes") => true,
        Some("0" | "false" | "off" | "no") => false,
        _ => preference.unwrap_or(DEFAULT_ON),
    }
}

/// The preference changed (or was loaded).
pub(crate) fn set_preference(on: bool) {
    PREFERENCE.with(|p| p.set(Some(on)));
}

/// Tests: turn scripting on or off for this thread.
#[cfg(test)]
pub(crate) fn set_for_test(on: bool) {
    TEST.with(|t| t.set(on));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_environment_wins_then_the_preference_then_the_default() {
        assert!(from(Some("1"), Some(false)));
        assert!(!from(Some("0"), Some(true)));
        assert!(from(None, Some(true)));
        assert!(!from(None, Some(false)));
        assert_eq!(from(None, None), DEFAULT_ON);
    }
}
