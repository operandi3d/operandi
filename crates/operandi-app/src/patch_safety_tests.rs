//! Live patches (G7 PC-7) against every command: the command contract and
//! monkey sessions on a box with a live patch beside it, so a command that
//! meets a patch polygon keeps undo, redo and save/reopen agreeing and the
//! mesh valid.
use super::command_contract_tests::{Broken, KNOWN, SLOW, Setup, Verdict, check, commands, judge};
use super::*;
use operandi_core::PolygonRepresentation;

/// Adds four curves and a live patch on their corners to the box's mesh,
/// beside the box, and selects the patch polygon in Polygon mode.
pub(super) fn add_live_patch(a: &mut Operandi) {
    let corners = [
        DVec3::new(2., 0., -1.),
        DVec3::new(4., 0., -1.),
        DVec3::new(4., 0., 1.),
        DVec3::new(2., 0., 1.),
    ];
    let face = |vertices: Vec<u32>| Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    };
    let mut patch = operandi_core::Mesh {
        vertices: corners.to_vec(),
        ..Default::default()
    };
    for k in 0..4u32 {
        let (p, q) = (corners[k as usize], corners[((k + 1) % 4) as usize]);
        patch.vertices.push((p + q) / 2. + DVec3::Y * 0.3);
        patch.faces.push(face(vec![k, 4 + k, (k + 1) % 4]));
        patch
            .attributes
            .representations
            .push(PolygonRepresentation::Polyline {
                closed: false,
                curve: operandi_core::CurveKind::Curve {
                    start_control: false,
                    end_control: false,
                },
            });
    }
    patch.faces.push(face(vec![0, 1, 2, 3]));
    patch
        .attributes
        .representations
        .push(PolygonRepresentation::Patch);
    let mesh = a.editor.scene.active_mut().unwrap().mesh.shared_mut();
    operandi_core::append_mesh(mesh, patch).unwrap();
    let patch = mesh.faces.len() as u32 - 1;
    mesh.validate().unwrap();
    a.editor.undo.clear();
    a.run(Command::Mode(Mode::Polygon));
    let selection = &mut a.editor.scene.selection;
    selection.edges.clear();
    selection.vertices.clear();
    selection.faces = vec![patch];
}

/// Known failures on the patch setup, the same ones the box setups list in
/// `KNOWN` (command_contract_tests.rs), with what goes wrong.
const PATCH_KNOWN: &[(&str, Broken, &str)] = &[
    (
        "cmds.batch patch",
        Broken::StepWithoutChange,
        "not a bug: the registry example flips twice, so the scene ends as it began",
    ),
    (
        "edge.split patch",
        Broken::ChangeWithoutStep,
        "changes the selection with no undo step when the selection splits nothing",
    ),
    (
        "Edge Split patch",
        Broken::ChangeWithoutStep,
        "changes the selection with no undo step when the selection splits nothing",
    ),
    (
        "vert.merge patch",
        Broken::ChangeWithoutStep,
        "changes the selection with no undo step when there are no points to merge",
    ),
    (
        "Vertices patch",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Edges patch",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Items patch",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "Materials patch",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
    (
        "select.nextMode patch",
        Broken::ChangeWithoutStep,
        "registry marks it undoable, but a mode switch records no undo step",
    ),
];

/// Every registry command, run on the patch setup, keeps the contract.
#[test]
fn every_command_keeps_the_contract_on_a_live_patch() {
    let known: Vec<(&str, Broken, &str)> = KNOWN.iter().chain(PATCH_KNOWN).copied().collect();
    let mut failures = Vec::new();
    let mut stale = Vec::new();
    for (name, line) in commands() {
        if SLOW.contains(&name) {
            continue;
        }
        let key = format!("{name} patch");
        match judge(&key, &line, check(&line, Setup::Patch), &known) {
            Verdict::Fine => {}
            Verdict::Broken(e) => failures.push(e),
            Verdict::Stale => stale.push(key),
        }
    }
    assert!(
        failures.is_empty() && stale.is_empty(),
        "\nBroken on a live patch:\n{}\n\nKNOWN lines that pass now:\n{}\n",
        failures.join("\n"),
        stale.join("\n"),
    );
}

/// Seeded random sessions starting from the patch setup.
#[test]
fn monkey_sessions_on_a_live_patch_keep_the_scene_sound() {
    let lines = super::monkey_tests::lines();
    let failures: Vec<String> = (1..=12u64)
        .filter_map(|seed| {
            super::monkey_tests::session_in(Some(Setup::Patch), seed, 60, &lines).err()
        })
        .collect();
    assert!(
        failures.is_empty(),
        "\n{} patch sessions failed:\n\n{}\n",
        failures.len(),
        failures.join("\n\n")
    );
}
