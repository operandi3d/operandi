//! LXO-UI-002/003/009, A-T03/A-T04 and the refusal/cancellation part of
//! A-T08, through the real application route: `request_open_path` →
//! `do_file(Open)` → `open` → `begin_lxo` worker →
//! `poll_import` (called by `import_dialogs` in each frame) → refusal, cancel
//! or the stale guard in `commit_import`. With several open scenes, Open
//! no longer asks to save first: a result opens beside the old document,
//! which stays as it was in its own tab (scenes.rs).
//!
//! The old document is a reachable dirty native document built with ordinary
//! commands: a saved native file, later edits, nonempty undo and redo, item and
//! polygon selection, a fixed workplane, an action center, a live Move preview,
//! an unconfirmed field draft and name draft, and a written recovery journal.
//!
//! The only seam is `WorkerSeam`, handed to the next `begin_lxo` on this test
//! thread. It can lower the worker's limits and pause the worker inside a named
//! real progress callback, or after `load` returns and before the result is
//! sent. Cancellation itself is always the job's real flag, set by Escape in
//! the real progress modal; the worker's real callback returns it.
//!
//! These are headless frames. They do not establish native focus, pointer or
//! keyboard routing, progress display or Cancel on screen.

use super::*;
use operandi_core::lxo::Limits;
use std::cell::{Cell, RefCell};
use std::sync::mpsc::{Receiver, Sender};

/// Where the worker pauses: the n-th call of a named progress phase, or after
/// `load` returns and before the result is sent. The tests that set a pause
/// used removed LXO fixtures (2026-11-05); the
/// seam stays because the import worker calls it.
#[allow(dead_code)]
enum Pause {
    Phase(String, usize),
    BeforeSend,
}

struct Gate {
    at: Pause,
    seen: Cell<usize>,
    reached: Sender<String>,
    resume: Receiver<()>,
}

impl Gate {
    /// Report the pause and wait for the test. If the test has ended (sender
    /// dropped), the worker simply continues.
    fn pause(&self, at: &str) {
        let _ = self.reached.send(at.to_owned());
        let _ = self.resume.recv();
    }
}

pub(super) struct WorkerSeam {
    limits: Option<Limits>,
    gate: Option<Gate>,
}

thread_local! {
    // Per test thread, so parallel tests cannot hand a seam to each other.
    static NEXT_SEAM: RefCell<Option<WorkerSeam>> = const { RefCell::new(None) };
}

impl WorkerSeam {
    pub(super) fn take() -> Self {
        NEXT_SEAM.with(|s| s.borrow_mut().take()).unwrap_or(Self {
            limits: None,
            gate: None,
        })
    }
    pub(super) fn limit(&self, limits: &mut Limits) {
        if let Some(l) = &self.limits {
            *limits = l.clone();
        }
    }
    pub(super) fn at_phase(&self, phase: &str) {
        if let Some(g) = &self.gate
            && let Pause::Phase(name, n) = &g.at
            && name == phase
        {
            g.seen.set(g.seen.get() + 1);
            if g.seen.get() == *n {
                g.pause(phase);
            }
        }
    }
    pub(super) fn before_send(&self) {
        if let Some(g) = &self.gate
            && matches!(g.at, Pause::BeforeSend)
        {
            g.pause("before send");
        }
    }
}

/// Lifecycle fact behind the old document's command order: polygon selection
/// is a history step and clears redo, while item selection, mode, workplane,
/// action context, Move activation and preview keep it.
#[test]
fn component_selection_clears_redo() {
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.activate(Tool::Box);
    app.run(Command::Apply);
    app.run(Command::Drop);
    app.run(Command::Rename("A".into()));
    app.run(Command::Rename("B".into()));
    app.run(Command::Undo);
    let redo = |app: &Operandi| app.editor.redo.len();
    assert_eq!(redo(&app), 1);
    let item = app.editor.scene.active().unwrap().id;
    app.run(Command::SelectItem(Some(item)));
    app.run(Command::Mode(Mode::Polygon));
    assert_eq!(redo(&app), 1, "item selection and mode keep redo");
    app.run(Command::Select {
        item,
        hit: Component::Polygon(0),
        replace: true,
        remove: false,
    });
    assert_eq!(redo(&app), 0, "polygon selection clears redo");
    app.run(Command::Rename("C".into()));
    app.run(Command::Undo);
    app.run(Command::Workplane(Workplane {
        fixed: true,
        plane: 1,
        center: DVec3::new(0., 0.5, 0.),
    }));
    app.run(Command::ActionContext(ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    }));
    app.activate(Tool::Move);
    app.run(Command::Preview(Parameters {
        delta: DVec3::new(0.25, 0.5, 0.75),
        ..Default::default()
    }));
    assert_eq!(
        redo(&app),
        1,
        "workplane, context, Move and preview keep redo"
    );
}
