//! Replicators (G8) against every command: the command contract's checks
//! (no panic, valid meshes, one undo step, undo/redo and save/reopen agree)
//! and seeded random sessions, on a box with a Replicator of it, the
//! Replicator selected.
use super::command_contract_tests::{SLOW, close_result_windows, commands, differences, saved};
use super::*;
use operandi_core::persistence;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// A box, a Replica Array of it (two copies) and the Replicator selected
/// in Item mode.
fn replica_app() -> Operandi {
    let mut a = super::tests::box_app();
    a.run(Command::Mode(operandi_core::Mode::Item));
    a.activate(operandi_core::Tool::Array);
    let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
    p.duplicate.copier = Some(operandi_core::duplicate_copier::Copier::Replica);
    p.duplicate.array.count = [3, 1, 1];
    p.duplicate.placed = true;
    a.run(Command::Preview(p));
    a.run(Command::Drop);
    let rep = a
        .editor
        .scene
        .items
        .iter()
        .find(|i| i.replicator.is_some())
        .expect("an Instancer")
        .id;
    a.editor.scene.select_item(Some(rep));
    a.editor.history.undo.clear();
    a
}

fn sound(a: &Operandi) -> std::result::Result<(), String> {
    for item in &a.editor.scene.items {
        item.mesh
            .validate()
            .map_err(|e| format!("invalid mesh on {:?}: {e}", item.name))?;
    }
    // The evaluated scene builds (copies and all) without a panic.
    let _ = a
        .editor
        .evaluated(operandi_core::evaluate::Purpose::Final)
        .items
        .len();
    let bytes = saved(a);
    let back = persistence::decode(&bytes).map_err(|e| format!("doesn't reopen: {e}"))?;
    if persistence::encode(&back).ok().as_deref() != Some(bytes.as_slice()) {
        return Err(format!(
            "saves and reopens differently ({})",
            differences(&a.editor.scene, &back)
        ));
    }
    Ok(())
}

fn check(line: &str) -> std::result::Result<(), String> {
    let mut a = replica_app();
    let before = a.editor.scene.clone();
    let ran = catch_unwind(AssertUnwindSafe(|| {
        a.execute(line);
        close_result_windows(&mut a);
        a
    }));
    let mut a = ran.map_err(|_| "panics".to_string())?;
    sound(&a)?;
    let after = a.editor.scene.clone();
    let steps = a.editor.history.undo.len();
    if steps > 1 {
        return Err(format!("records {steps} undo steps"));
    }
    if after == before || steps == 0 {
        return Ok(());
    }
    a.execute("Undo");
    if a.editor.scene != before {
        return Err(format!(
            "undo doesn't give back the scene ({})",
            differences(&before, &a.editor.scene)
        ));
    }
    a.execute("Redo");
    if a.editor.scene != after {
        return Err(format!(
            "redo doesn't give back the scene ({})",
            differences(&after, &a.editor.scene)
        ));
    }
    Ok(())
}

#[test]
fn every_command_keeps_the_contract_on_a_replicator() {
    let failures: Vec<String> = commands()
        .into_iter()
        .filter(|(name, _)| !SLOW.contains(name))
        .filter_map(|(name, line)| {
            check(&line)
                .err()
                .map(|e| format!("{name} ({line:?}): {e}"))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "\nBroken on an Instancer:\n{}\n",
        failures.join("\n")
    );
}

/// Seeded random sessions of contract commands, undo and redo; the scene
/// stays sound throughout and undoing everything gives back the start.
#[test]
fn random_sessions_on_a_replicator_keep_the_scene_sound() {
    let lines: Vec<String> = commands()
        .into_iter()
        .filter(|(name, _)| !SLOW.contains(name))
        .map(|(_, line)| line)
        .collect();
    let mut failures = Vec::new();
    'seeds: for seed in 1..=8u64 {
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mut next = |n: usize| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % n as u64) as usize
        };
        let mut a = replica_app();
        let start = a.editor.scene.clone();
        let mut steps = Vec::new();
        for _ in 0..30 {
            let roll = next(100);
            let line = if roll < 10 {
                "Undo".to_owned()
            } else if roll < 15 {
                "Redo".to_owned()
            } else {
                lines[next(lines.len())].clone()
            };
            steps.push(line.clone());
            let ran = catch_unwind(AssertUnwindSafe(|| {
                a.execute(&line);
                close_result_windows(&mut a);
                a
            }));
            match ran {
                Ok(b) => a = b,
                Err(_) => {
                    failures.push(format!("seed {seed}: panics at {steps:?}"));
                    continue 'seeds;
                }
            }
            if let Err(e) = sound(&a) {
                failures.push(format!("seed {seed}: {e} after {steps:?}"));
                break;
            }
        }
        let mut guard = 0;
        while !a.editor.history.undo.is_empty() && guard < 100 {
            a.execute("Undo");
            guard += 1;
        }
        if failures.is_empty() && a.editor.scene.items != start.items {
            failures.push(format!(
                "seed {seed}: undoing everything doesn't give back the start ({}) after {steps:?}",
                differences(&start, &a.editor.scene)
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}\n", failures.join("\n\n"));
}
