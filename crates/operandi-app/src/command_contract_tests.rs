//! The command contract (testing plan item 3, Jason 2026-11-18): every
//! registry command that edits the scene keeps the promise AGENTS.md makes
//! for edits ("previews, committed edits, undo/redo and save/reopen must
//! agree"). For each undoable command, on a box in each selection mode and
//! with nothing selected, this runs the command's example line and checks:
//!
//! - it doesn't panic;
//! - every mesh still passes `Mesh::validate`;
//! - it records at most one undo step, and one if it changed the scene;
//! - undo gives back the scene from before, byte for byte as saved;
//! - redo gives back the scene from after;
//! - the scene saves and reopens unchanged.
//!
//! A command added to the registry is covered the day it lands. Failures
//! known today are listed in `KNOWN` with what goes wrong; the test also
//! fails when one of them starts passing, so the list only shrinks.
use super::tests::box_app;
use super::*;
use operandi_core::persistence;
use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// The starting states every command is tried in.
#[derive(Clone, Copy, Debug)]
pub(super) enum Setup {
    Polygons,
    Edges,
    Points,
    Nothing,
    /// The box with a live patch beside it (G7 PC-7), the patch polygon
    /// selected. Not in `ALL`: patch_safety_tests.rs sweeps it.
    Patch,
}

impl Setup {
    pub(super) const ALL: [Setup; 4] =
        [Setup::Polygons, Setup::Edges, Setup::Points, Setup::Nothing];

    fn name(self) -> &'static str {
        match self {
            Setup::Polygons => "polygons",
            Setup::Edges => "edges",
            Setup::Points => "points",
            Setup::Nothing => "nothing",
            Setup::Patch => "patch",
        }
    }

    /// A 2 m box with part of it selected in this setup's mode: two
    /// polygons that share an edge, two opposite edges of one polygon, or
    /// the two ends of one edge, so commands that need neighbours or a
    /// pair have something to work on.
    pub(super) fn app(self) -> Operandi {
        let mut a = box_app();
        if let Setup::Patch = self {
            super::patch_safety_tests::add_live_patch(&mut a);
            return a;
        }
        let mesh = &a.editor.scene.active().unwrap().mesh;
        let f = mesh.faces[0].vertices.clone();
        let neighbour = (1..mesh.faces.len() as u32)
            .find(|&i| {
                let g = &mesh.faces[i as usize].vertices;
                f.iter().filter(|v| g.contains(v)).count() == 2
            })
            .expect("a box polygon has a neighbour");
        let mode = match self {
            Setup::Polygons | Setup::Nothing => Mode::Polygon,
            Setup::Edges => Mode::Edge,
            Setup::Points => Mode::Vertex,
            Setup::Patch => unreachable!("set up above"),
        };
        a.run(Command::Mode(mode));
        let selection = &mut a.editor.scene.selection;
        selection.faces.clear();
        selection.edges.clear();
        selection.vertices.clear();
        match self {
            Setup::Polygons => selection.faces = vec![0, neighbour],
            Setup::Edges => {
                // Edges are stored low index first.
                let edge = |p: u32, q: u32| (p.min(q), p.max(q));
                selection.edges = vec![edge(f[0], f[1]), edge(f[2], f[3])]
            }
            Setup::Points => selection.vertices = vec![f[0], f[1]],
            Setup::Nothing | Setup::Patch => {}
        }
        a
    }
}

/// Closes the result windows some commands leave open (Align, Unify, Set
/// Material, Scale Offset), as a person clicking OK would. While one is
/// open every other command, Undo included, is refused.
pub(super) fn close_result_windows(a: &mut Operandi) {
    a.align_notice = None;
    a.unify = None;
    a.unify_count = None;
    a.set_material = None;
    a.scale_offset = None;
}

/// The scene as it would be saved.
pub(super) fn saved(a: &Operandi) -> Vec<u8> {
    persistence::encode(&a.editor.scene).expect("scene encodes")
}

/// The commands the contract applies to: undoable scene edits that run
/// from a typed line, without a window, dialog or interactive tool.
pub(super) fn commands() -> Vec<(&'static str, String)> {
    command_handlers::registry()
        .iter()
        .map(|c| &c.entry)
        .filter(|e| {
            let f = e.flags;
            f.undoable && !f.ui_only && !f.tool && !f.side_effect && !f.query
        })
        .map(|e| (e.name, e.example.to_owned()))
        .collect()
}

/// Which saved parts of two scenes differ, for the failure message:
/// top-level fields, and per item the fields that changed.
pub(super) fn differences(a: &operandi_core::Scene, b: &operandi_core::Scene) -> String {
    use serde_json::Value;
    let (Ok(Value::Object(x)), Ok(Value::Object(y))) =
        (serde_json::to_value(a), serde_json::to_value(b))
    else {
        return "?".into();
    };
    let mut out = Vec::new();
    for key in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
        match (x.get(key), y.get(key)) {
            (Some(Value::Array(p)), Some(Value::Array(q))) if key == "items" => {
                if p.len() != q.len() {
                    out.push(format!("items ({} vs {})", p.len(), q.len()));
                    continue;
                }
                for (i, (p, q)) in p.iter().zip(q).enumerate() {
                    let (Value::Object(p), Value::Object(q)) = (p, q) else {
                        continue;
                    };
                    for f in p.keys().chain(q.keys().filter(|k| !p.contains_key(*k))) {
                        if p.get(f) != q.get(f) {
                            out.push(format!("items[{i}].{f}"));
                        }
                    }
                }
            }
            (p, q) if p != q => out.push(key.clone()),
            _ => {}
        }
    }
    if out.is_empty() {
        "unsaved state only (hidden elements?)".into()
    } else {
        out.join(", ")
    }
}

/// What kind of promise a command broke. A `KNOWN` line names the kind it
/// expects, so a command listed for one problem can't hide a different,
/// worse one behind it, such as a panic or a broken Undo (Astra P4-001).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Broken {
    Panics,
    InvalidMesh,
    ExtraSteps,
    StepWithoutChange,
    ChangeWithoutStep,
    Reopen,
    Undo,
    Redo,
}

/// One broken promise: its kind and what happened.
#[derive(Debug)]
pub(super) struct Failure {
    pub kind: Broken,
    pub text: String,
}

impl Failure {
    fn new(kind: Broken, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{:?}]", self.text, self.kind)
    }
}

/// How one `name setup` result compares with the known-failure lists.
pub(super) enum Verdict {
    Fine,
    /// A new failure, or a known command failing in a different way.
    Broken(String),
    /// A `KNOWN` line whose command passes now.
    Stale,
}

pub(super) fn judge(
    key: &str,
    line: &str,
    result: std::result::Result<bool, Failure>,
    known: &[(&str, Broken, &str)],
) -> Verdict {
    let expected = known.iter().find(|(k, _, _)| *k == key).map(|(_, b, _)| *b);
    match (result, expected) {
        (Ok(_), None) => Verdict::Fine,
        (Ok(_), Some(_)) => Verdict::Stale,
        (Err(e), None) => Verdict::Broken(format!("{key} ({line:?}): {e}")),
        (Err(e), Some(kind)) if e.kind == kind => Verdict::Fine,
        (Err(e), Some(kind)) => {
            Verdict::Broken(format!("{key} ({line:?}): {e}, but KNOWN expects {kind:?}"))
        }
    }
}

/// What broke, as one `name setup: problem` line. Undo and redo are
/// compared on the whole scene in memory (hidden elements too); save and
/// reopen on the saved bytes.
pub(super) fn check(line: &str, setup: Setup) -> std::result::Result<bool, Failure> {
    let mut a = setup.app();
    let before = a.editor.scene.clone();
    let steps = a.editor.undo.len();
    let ran = catch_unwind(AssertUnwindSafe(|| {
        a.execute(line);
        close_result_windows(&mut a);
        a
    }));
    let mut a = match ran {
        Ok(a) => a,
        Err(_) => return Err(Failure::new(Broken::Panics, "panics")),
    };
    for item in &a.editor.scene.items {
        if let Err(e) = item.mesh.validate() {
            return Err(Failure::new(
                Broken::InvalidMesh,
                format!("leaves an invalid mesh on {:?}: {e}", item.name),
            ));
        }
    }
    let after = a.editor.scene.clone();
    let added = a.editor.undo.len().saturating_sub(steps);
    if added > 1 {
        return Err(Failure::new(
            Broken::ExtraSteps,
            format!("records {added} undo steps"),
        ));
    }
    if after == before {
        // Nothing to undo; that's fine when the setup gives it nothing to
        // work on, and the step count must say so too.
        return match added {
            0 => Ok(false),
            _ => Err(Failure::new(
                Broken::StepWithoutChange,
                "records an undo step without changing the scene",
            )),
        };
    }
    if added == 0 {
        return Err(Failure::new(
            Broken::ChangeWithoutStep,
            format!(
                "changes the scene without an undo step ({})",
                differences(&before, &after)
            ),
        ));
    }
    let bytes = saved(&a);
    match persistence::decode(&bytes) {
        Ok(reopened) => match persistence::encode(&reopened) {
            Ok(again) if again == bytes => {}
            _ => {
                return Err(Failure::new(
                    Broken::Reopen,
                    format!(
                        "saves and reopens differently ({})",
                        differences(&after, &reopened)
                    ),
                ));
            }
        },
        Err(e) => return Err(Failure::new(Broken::Reopen, format!("doesn't reopen: {e}"))),
    }
    a.execute("Undo");
    if a.editor.scene != before {
        return Err(Failure::new(
            Broken::Undo,
            format!(
                "undo doesn't give back the scene from before ({})",
                differences(&before, &a.editor.scene)
            ),
        ));
    }
    a.execute("Redo");
    if a.editor.scene != after {
        return Err(Failure::new(
            Broken::Redo,
            format!(
                "redo doesn't give back the scene from after ({})",
                differences(&after, &a.editor.scene)
            ),
        ));
    }
    Ok(true)
}

/// Known failures as `"<command> <setup>"`, with what goes wrong. Fix the
/// command and delete its line; the test says when a line is stale.
pub(super) const KNOWN: &[(&str, Broken, &str)] = &[
    (
        "Vertices polygons",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Vertices edges",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Vertices nothing",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Edges polygons",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Edges points",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Edges nothing",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Polygons edges",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Polygons points",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Items polygons",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Items edges",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Items points",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Items nothing",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Materials polygons",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Materials edges",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Materials points",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Materials nothing",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "select.nextMode polygons",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "select.nextMode edges",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "select.nextMode points",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "select.nextMode nothing",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "vert.merge polygons",
        Broken::ChangeWithoutStep,
        "changes the selection with no undo step when there are no points to merge",
    ),
    (
        "vert.merge edges",
        Broken::ChangeWithoutStep,
        "changes the selection with no undo step when there are no points to merge",
    ),
    (
        "edge.split points",
        Broken::ChangeWithoutStep,
        "changes the selection with no undo step when the points split nothing",
    ),
    (
        "Edge Split points",
        Broken::ChangeWithoutStep,
        "changes the selection with no undo step when the points split nothing",
    ),
    (
        "edge.addPoint edges",
        Broken::ExtraSteps,
        "records 2 undo steps",
    ),
    (
        "cmds.batch polygons",
        Broken::StepWithoutChange,
        "not a bug: the registry example flips twice, so the scene ends as it began",
    ),
    (
        "cmds.batch nothing",
        Broken::StepWithoutChange,
        "not a bug: the registry example flips twice, so the scene ends as it began",
    ),
];

#[test]
fn saving_twice_gives_the_same_bytes() {
    // The contract compares saved bytes, so saving must be repeatable.
    for setup in Setup::ALL {
        let a = setup.app();
        assert_eq!(saved(&a), saved(&a), "{}", setup.name());
    }
}

/// Commands too slow in a debug build to try in every setup (uv.pack
/// takes ~5 s on the box); they run in the Polygons setup only.
pub(super) const SLOW: &[&str] = &["uv.pack"];

/// Every fourth command from `part`, in every setup.
fn sweep(part: usize) {
    let mut failures = Vec::new();
    let mut stale = Vec::new();
    let mut edited = BTreeSet::new();
    let mut total = 0;
    for (name, line) in commands().into_iter().skip(part).step_by(4) {
        total += 1;
        for setup in Setup::ALL {
            if SLOW.contains(&name) && !matches!(setup, Setup::Polygons) {
                continue;
            }
            let key = format!("{name} {}", setup.name());
            let result = check(&line, setup);
            if let Ok(true) | Err(_) = result {
                edited.insert(name);
            }
            match judge(&key, &line, result, KNOWN) {
                Verdict::Fine => {}
                Verdict::Broken(e) => failures.push(e),
                Verdict::Stale => stale.push(key),
            }
        }
    }
    // Commands whose example changes nothing in any setup aren't really
    // tested here; they need a richer starting scene (later work).
    eprintln!(
        "command contract part {part}: {} of {total} commands edited a setup",
        edited.len()
    );
    if std::env::var_os("OPERANDI_CONTRACT_LIST").is_some() {
        for (name, line) in commands().into_iter().skip(part).step_by(4) {
            if !edited.contains(name) {
                eprintln!("  untested: {line}");
            }
        }
    }
    assert!(
        failures.is_empty() && stale.is_empty(),
        "\nBroken commands:\n{}\n\nKNOWN lines that pass now (delete them):\n{}\n",
        failures.join("\n"),
        stale.join("\n"),
    );
}

// Four parts so they run side by side, like the drag-undo sweep.
#[test]
fn every_command_keeps_the_undo_redo_save_contract() {
    sweep(0);
}
#[test]
fn every_command_keeps_the_undo_redo_save_contract_part_2() {
    sweep(1);
}
#[test]
fn every_command_keeps_the_undo_redo_save_contract_part_3() {
    sweep(2);
}
#[test]
fn every_command_keeps_the_undo_redo_save_contract_part_4() {
    sweep(3);
}

/// A `KNOWN` line covers only the problem it names (Astra P4-001): the
/// same command panicking, or breaking Undo, is still a failure.
#[test]
fn a_known_line_does_not_hide_a_different_failure() {
    let known = &[("x.cmd polygons", Broken::ChangeWithoutStep, "listed")][..];
    let fails = |kind| Err(Failure::new(kind, "boom"));
    assert!(matches!(
        judge(
            "x.cmd polygons",
            "x.cmd",
            fails(Broken::ChangeWithoutStep),
            known
        ),
        Verdict::Fine
    ));
    for kind in [Broken::Panics, Broken::Undo, Broken::InvalidMesh] {
        assert!(
            matches!(
                judge("x.cmd polygons", "x.cmd", fails(kind), known),
                Verdict::Broken(_)
            ),
            "{kind:?} hid behind the KNOWN line"
        );
    }
    assert!(matches!(
        judge("x.cmd polygons", "x.cmd", Ok(true), known),
        Verdict::Stale
    ));
    assert!(matches!(
        judge("y.cmd polygons", "y.cmd", fails(Broken::Redo), known),
        Verdict::Broken(_)
    ));
}
