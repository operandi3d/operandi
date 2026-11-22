//! Test-only record of the controls the shared widgets draw, for the
//! UIC-164 check (every menu entry, button and property row runs a
//! command; command_registry_tests.rs). The shared row component
//! (`form::rows`) and the toolbox buttons report here; nothing is kept
//! unless a test has started recording on its thread.
use std::cell::RefCell;
use std::collections::BTreeSet;

#[derive(Default)]
struct Recorder {
    on: bool,
    /// What the test has on screen (e.g. "lights"), set by the test.
    context: String,
    /// Open `form::rows::scope` names, innermost last.
    scopes: Vec<String>,
    /// (context, surface, label), in the order first drawn. A control
    /// drawn again under a later context (panels that stay on screen)
    /// keeps its first context.
    seen: Vec<(String, String, String)>,
    drawn: BTreeSet<(String, String)>,
}

impl Recorder {
    fn note(&mut self, surface: String, label: &str) {
        let label = label.trim().to_owned();
        if self.drawn.insert((surface.clone(), label.clone())) {
            self.seen.push((self.context.clone(), surface, label));
        }
    }
}

thread_local! {
    static RECORDER: RefCell<Recorder> = RefCell::new(Recorder::default());
}

/// Start recording on this thread, forgetting anything seen before.
pub(crate) fn start() {
    RECORDER.with(|r| {
        *r.borrow_mut() = Recorder {
            on: true,
            ..Default::default()
        }
    });
}

/// What the test is showing now; recorded with each control.
pub(crate) fn set_context(context: &str) {
    RECORDER.with(|r| r.borrow_mut().context = context.to_owned());
}

/// Stop recording and return what was seen.
pub(crate) fn take() -> Vec<(String, String, String)> {
    RECORDER.with(|r| std::mem::take(&mut *r.borrow_mut()).seen)
}

pub(crate) fn push_scope(name: &str) {
    RECORDER.with(|r| r.borrow_mut().scopes.push(name.to_owned()));
}

pub(crate) fn pop_scope() {
    RECORDER.with(|r| {
        r.borrow_mut().scopes.pop();
    });
}

/// A property row (its label) in the innermost open scope.
pub(crate) fn row(label: &str) {
    RECORDER.with(|r| {
        let mut r = r.borrow_mut();
        if r.on {
            let surface = r.scopes.last().cloned().unwrap_or_else(|| "loose".into());
            r.note(surface, label);
        }
    });
}

/// A button on a named surface (the toolbox).
pub(crate) fn button(surface: &str, label: &str) {
    RECORDER.with(|r| {
        let mut r = r.borrow_mut();
        if r.on {
            r.note(surface.to_owned(), label);
        }
    });
}

thread_local! {
    /// Where each row's field was last drawn, by label (the panel-driving
    /// test clicks and drags them).
    static RECTS: RefCell<std::collections::BTreeMap<String, eframe::egui::Rect>> =
        const { RefCell::new(std::collections::BTreeMap::new()) };
    /// The command line a panel edit last ran (replace_layer, outputs).
    static LAST_COMMAND: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub(crate) fn row_rect(label: &str, rect: eframe::egui::Rect) {
    RECTS.with(|r| r.borrow_mut().insert(label.trim().to_owned(), rect));
}

pub(crate) fn take_rects() -> std::collections::BTreeMap<String, eframe::egui::Rect> {
    RECTS.with(|r| std::mem::take(&mut *r.borrow_mut()))
}

pub(crate) fn note_command(line: &str) {
    LAST_COMMAND.with(|c| *c.borrow_mut() = Some(line.to_owned()));
}

pub(crate) fn take_command() -> Option<String> {
    LAST_COMMAND.with(|c| c.borrow_mut().take())
}
