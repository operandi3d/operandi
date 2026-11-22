//! Text transport for numeric-only panels, independent of tool parameters and
//! transactions. Callers own validation, publication, and error presentation.
use super::*;

impl Operandi {
    /// Publish confirmed display text only after the caller accepts parameters.
    /// Error clearing remains caller-owned; synchronization does not accept a draft.
    pub(super) fn sync_numeric_value(&mut self, key: &str, value: f64, unit: &str) {
        let text = workspace::number_text(value, unit);
        self.fields.insert(key.into(), text.clone());
        self.confirmed_fields.insert(key.into(), text);
    }
}
