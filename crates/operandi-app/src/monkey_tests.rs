//! Monkey-user sessions (testing plan item 4, Jason 2026-11-18): a seeded
//! random "user" picks selections, registry commands, undo, redo and saves
//! on a box, and after every step checks what must always hold:
//!
//! - nothing panics;
//! - every mesh passes `Mesh::validate`;
//! - the scene saves and reopens to the same bytes;
//! - at the end, undoing everything gives back the starting geometry.
//!
//! It finds bugs nobody thought to write a test for: the ones that come
//! from sequences. A failure prints the seed and every step, so it can be
//! replayed (`OPERANDI_MONKEY_SEED=<seed>`) and turned into an ordinary
//! regression test.
//!
//! The normal test run tries a few seeds; the nightly run tries many
//! (`OPERANDI_MONKEY_SEEDS=2000 cargo test ... monkey -- --ignored`).
use super::command_contract_tests::{SLOW, Setup, close_result_windows, commands, differences};
use super::*;
use operandi_core::persistence;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Steps in one session.
const STEPS: usize = 40;
/// Past this many polygons a session undoes instead of growing the mesh
/// further, so repeated subdivides don't make sessions slow.
const MAX_POLYGONS: usize = 4_000;

/// Commands left out: slow in debug builds, or they need files, the
/// network or a window to mean anything.
const LEFT_OUT: &[&str] = &[
    "sculpt.remesh",
    "operandi.remesh",
    "mesh.retopo",
    "operandi.environment.loadImage",
    "operandi.materialx.import",
    "operandi.nodeMaterial.import",
    "scene.importReference",
    "clip.addStill",
    "backdrop.image",
];

/// SplitMix64: small, fast and the same on every machine, so a seed
/// replays exactly.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Picks a random selection on the active mesh: 1 to 4 points, edges or
/// polygons. Returns what it did, or `None` when there's no mesh to pick on.
fn random_selection(a: &mut Operandi, rng: &mut Rng) -> Option<String> {
    let mesh = &a.editor.scene.active()?.mesh;
    // Real polygons only: curves and one- or two-point polygons have no
    // edges to pick the way a person would.
    let real: Vec<usize> = (0..mesh.faces.len())
        .filter(|&f| mesh.faces[f].vertices.len() >= 3)
        .collect();
    if real.is_empty() {
        return None;
    }
    let count = 1 + rng.below(4);
    let faces: Vec<usize> = (0..count).map(|_| real[rng.below(real.len())]).collect();
    let (mode, label, picks) = match rng.below(3) {
        0 => (Mode::Polygon, "polygons", {
            let mut f: Vec<u32> = faces.iter().map(|&f| f as u32).collect();
            f.sort_unstable();
            f.dedup();
            Picks::Faces(f)
        }),
        1 => (Mode::Edge, "edges", {
            let mut e: Vec<(u32, u32)> = faces
                .iter()
                .map(|&f| {
                    let v = &mesh.faces[f].vertices;
                    let i = rng.below(v.len());
                    let (p, q) = (v[i], v[(i + 1) % v.len()]);
                    (p.min(q), p.max(q))
                })
                .collect();
            e.sort_unstable();
            e.dedup();
            Picks::Edges(e)
        }),
        _ => (Mode::Vertex, "points", {
            let mut p: Vec<u32> = faces
                .iter()
                .map(|&f| {
                    let v = &mesh.faces[f].vertices;
                    v[rng.below(v.len())]
                })
                .collect();
            p.sort_unstable();
            p.dedup();
            Picks::Points(p)
        }),
    };
    let text = format!("select {label} {picks:?}");
    a.run(Command::Mode(mode));
    let s = &mut a.editor.scene.selection;
    s.faces.clear();
    s.edges.clear();
    s.vertices.clear();
    match picks {
        Picks::Faces(f) => s.faces = f,
        Picks::Edges(e) => s.edges = e,
        Picks::Points(p) => s.vertices = p,
    }
    Some(text)
}

#[derive(Debug)]
enum Picks {
    Faces(Vec<u32>),
    Edges(Vec<(u32, u32)>),
    Points(Vec<u32>),
}

fn polygons(a: &Operandi) -> usize {
    a.editor
        .scene
        .items
        .iter()
        .map(|i| i.mesh.faces.len())
        .sum()
}

/// One session from `seed`. `Err` holds the steps and what broke.
fn session(seed: u64, count: usize, lines: &[String]) -> std::result::Result<(), String> {
    session_in(None, seed, count, lines)
}

/// `session`, starting from `fixed` instead of a setup the seed picks
/// (the live patch sessions, patch_safety_tests.rs).
pub(super) fn session_in(
    fixed: Option<Setup>,
    seed: u64,
    count: usize,
    lines: &[String],
) -> std::result::Result<(), String> {
    let mut rng = Rng(seed);
    let picked = Setup::ALL[rng.below(Setup::ALL.len())];
    let setup = fixed.unwrap_or(picked);
    let mut a = setup.app();
    let start = a.editor.scene.clone();
    // Making the box left undo steps of its own; undo back to here only.
    let depth = a.editor.undo.len();
    let mut steps = vec![format!("start: box, {setup:?}")];
    let fail = |steps: &[String], what: String| {
        Err(format!(
            "seed {seed}: {what}\n  steps:\n    {}",
            steps.join("\n    ")
        ))
    };
    for _ in 0..count {
        let roll = rng.below(100);
        // Never undo past the box: its own steps aren't the session's.
        let can_undo = a.editor.undo.len() > depth;
        let step = if can_undo && (polygons(&a) > MAX_POLYGONS || roll < 10) {
            "Undo".to_owned()
        } else if roll < 15 {
            "Redo".to_owned()
        } else if roll < 40 {
            let kept = a.editor.scene.selection.clone();
            match random_selection(&mut a, &mut rng) {
                Some(text) => {
                    if persistence::encode(&a.editor.scene).is_ok() {
                        steps.push(text);
                    } else {
                        // A pick the scene can't hold here (the mode
                        // switch was refused, say): a person couldn't make
                        // it either, so it's put back and skipped.
                        a.editor.scene.selection = kept;
                    }
                    continue;
                }
                None => lines[rng.below(lines.len())].clone(),
            }
        } else {
            lines[rng.below(lines.len())].clone()
        };
        steps.push(step.clone());
        let ran = catch_unwind(AssertUnwindSafe(|| {
            a.execute(&step);
            close_result_windows(&mut a);
            a
        }));
        a = match ran {
            Ok(a) => a,
            Err(_) => return fail(&steps, "panics".into()),
        };
        for item in &a.editor.scene.items {
            if let Err(e) = item.mesh.validate() {
                return fail(&steps, format!("invalid mesh on {:?}: {e}", item.name));
            }
        }
        let bytes = match persistence::encode(&a.editor.scene) {
            Ok(bytes) => bytes,
            Err(e) => return fail(&steps, format!("can't save: {e}")),
        };
        match persistence::decode(&bytes).and_then(|s| persistence::encode(&s)) {
            Ok(again) if again == bytes => {}
            Ok(_) => return fail(&steps, "saves and reopens differently".into()),
            Err(e) => return fail(&steps, format!("doesn't reopen: {e}")),
        }
    }
    // Undo everything: the geometry must be the box we started with.
    let mut guard = 0;
    while a.editor.undo.len() > depth && guard < STEPS * 2 {
        a.execute("Undo");
        guard += 1;
    }
    if a.editor.scene.items != start.items {
        let what = differences(&start, &a.editor.scene);
        return fail(
            &steps,
            format!("undoing everything doesn't give back the start ({what})"),
        );
    }
    Ok(())
}

/// The lines a monkey can type: every contract command's example.
pub(super) fn lines() -> Vec<String> {
    commands()
        .into_iter()
        .filter(|(name, _)| !SLOW.contains(name) && !LEFT_OUT.contains(name))
        .map(|(_, line)| line)
        .collect()
}

/// A failing session cut down to its shortest failing length, so the
/// last step listed is the one that broke it. Selections don't count:
/// they only set up the next step.
fn shortest(seed: u64, lines: &[String]) -> Option<String> {
    session(seed, STEPS, lines).err()?;
    (1..=STEPS).find_map(|n| session(seed, n, lines).err())
}

/// Seeds that fail for a known reason, skipped until it's fixed; the run
/// says when one passes again so it can come off the list.
const KNOWN_SEEDS: &[(u64, &str)] = &[(
    4966,
    "edge.addPoint records 2 undo steps (command_contract_tests KNOWN)",
)];

/// Runs `count` seeds from `first`; panics listing every failing seed.
fn run_seeds(first: u64, count: u64) {
    let lines = lines();
    let mut failures = Vec::new();
    for seed in first..first + count {
        let known = KNOWN_SEEDS.iter().any(|(s, _)| *s == seed);
        match (shortest(seed, &lines), known) {
            (Some(f), false) => failures.push(f),
            (None, true) => failures.push(format!(
                "seed {seed} passes now: delete it from KNOWN_SEEDS"
            )),
            _ => {}
        }
    }
    assert!(
        failures.is_empty(),
        "\n{} of {count} monkey sessions failed:\n\n{}\n",
        failures.len(),
        failures.join("\n\n")
    );
}

/// The short run in every test pass, or one seed to replay
/// (`OPERANDI_MONKEY_SEED=<seed>`).
#[test]
fn monkey_sessions_keep_the_scene_sound() {
    match std::env::var("OPERANDI_MONKEY_SEED") {
        Ok(seed) => run_seeds(seed.parse().expect("a number"), 1),
        Err(_) => run_seeds(1, 24),
    }
}

/// The long run, for the nightly script: `OPERANDI_MONKEY_SEEDS` seeds
/// (default 500) after the short run's.
#[test]
#[ignore = "long run: OPERANDI_MONKEY_SEEDS=2000 cargo test --locked -p operandi-app monkey -- --ignored"]
fn monkey_sessions_long_run() {
    let count = std::env::var("OPERANDI_MONKEY_SEEDS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(500);
    run_seeds(1_000, count);
}

/// Regression (monkey seeds 2141, 2869): Unmerge copied the locked
/// elements' old numbers onto every piece, so the scene saved but
/// wouldn't reopen.
#[test]
fn unmerge_renumbers_locked_elements_so_the_scene_reopens() {
    let mut a = Setup::Edges.app();
    for line in ["edges.toCurve", "lock.sel", "layer.unmerge"] {
        a.execute(line);
    }
    assert!(a.editor.scene.items.len() > 1, "{}", a.status);
    let bytes = persistence::encode(&a.editor.scene).unwrap();
    persistence::decode(&bytes).expect("the unmerged scene reopens");
    let locked: usize = a
        .editor
        .scene
        .items
        .iter()
        .map(|i| i.elements.locked.edges.len())
        .sum();
    assert_eq!(locked, 2, "both locked edges are kept, on their pieces");
}

/// Regression (monkey seed 24 of 2026-11-21): after Instance, hide and
/// Center, the instance's hidden points stayed put but its source took the
/// moved ones, so the two saved different points and a reopen re-shared
/// them. The restored points now reach the whole instance group.
#[test]
fn center_after_instance_and_hide_keeps_the_group_one_shape() {
    let mut a = Setup::Polygons.app();
    for line in ["Edge Split", "poly.haul offset:0.2"] {
        a.execute(line);
    }
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(5, 12), (6, 13), (11, 13)];
    for line in [
        "Instance",
        "hide.sel",
        "Smooth Shift Reset",
        "select.loop",
        "vert.center all",
    ] {
        a.execute(line);
        let bytes = persistence::encode(&a.editor.scene).unwrap();
        let again = persistence::encode(&persistence::decode(&bytes).unwrap()).unwrap();
        assert!(again == bytes, "saves and reopens the same after {line}");
    }
    let items = &a.editor.scene.items;
    assert_eq!(items.len(), 2);
    assert_eq!(items[1].instance_of, Some(items[0].id));
    assert_eq!(
        items[0].mesh.vertices, items[1].mesh.vertices,
        "the source and its instance show one shape"
    );
}
