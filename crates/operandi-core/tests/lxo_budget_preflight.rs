//! D3 / LXO-SAFE-001 source-buffer working-budget preflight, through the
//! public `lxo::load` and `lxo::parse` API.
//!
//! `load` must refuse an impossible source reserve (8 bytes of working memory
//! per source byte, format/01 "Approved resource limits") *before* it reserves
//! or reads the source buffer, after the input cap, and must charge every
//! growth of an unknown or growing source before the buffer grows. The shared
//! parser charges the reserve exactly once and never resets an existing
//! `Control::used`.
//!
//! Observables: the progress stages (`load` reports "reading" before each read
//! block, then the parser's "framing"), `Control::used`, the refusal category,
//! and unchanged task-owned copies, mtimes and a sentinel. The heap-order
//! witness for the same refusal is in `lxo_allocation_measurement.rs`.

use operandi_core::lxo;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

struct Run {
    result: lxo::Result<lxo::Candidate>,
    stages: Vec<String>,
    used: usize,
}

/// One importer call with an initial `used`, limits, and a progress policy
/// (`stop_at`: cancel when this 1-based callback is reached).
fn run(
    limits: lxo::Limits,
    used: usize,
    stop_at: Option<usize>,
    call: impl FnOnce(&mut lxo::Control<'_>) -> lxo::Result<lxo::Candidate>,
) -> Run {
    let mut stages = Vec::new();
    let (result, used) = catch_unwind(AssertUnwindSafe(|| {
        let mut progress = |stage: &str| {
            stages.push(stage.to_owned());
            stop_at != Some(stages.len())
        };
        let mut control = lxo::Control::new(&mut progress);
        control.limits = limits;
        control.used = used;
        let result = call(&mut control);
        (result, control.used)
    }))
    .expect("importer panicked");
    Run {
        result,
        stages,
        used,
    }
}

fn load(path: &Path, limits: lxo::Limits, used: usize) -> Run {
    run(limits, used, None, |c| lxo::load(path, c))
}

fn limits(input_bytes: usize, working_bytes: usize) -> lxo::Limits {
    lxo::Limits {
        input_bytes,
        working_bytes,
    }
}

fn working(working_bytes: usize) -> lxo::Limits {
    lxo::Limits {
        working_bytes,
        ..lxo::Limits::default()
    }
}

fn refusal(run: &Run) -> &lxo::Error {
    let e = run.result.as_ref().expect_err("expected a refusal");
    assert!(e.detail.chars().count() <= 768);
    assert!(e.to_string().chars().count() <= 1024);
    e
}

/// An unknown-length stream is charged before each buffer growth: a working
/// budget below two blocks' reserve refuses at the second block, before the
/// buffer grows past one block.
#[cfg(unix)]
#[test]
fn unknown_stream_growth_is_charged_before_the_buffer_grows() {
    let zero = Path::new("/dev/zero");
    assert_eq!(std::fs::metadata(zero).unwrap().len(), 0);
    let block = 65_536;
    let refused = load(zero, working(8 * 2 * block - 1), 0);
    let e = refusal(&refused);
    assert_eq!(e.category, "resource_limit");
    assert_eq!(refused.stages, ["reading", "reading"]);
    // The first block's capacity was charged; the second growth's charge was
    // refused and stays counted.
    assert_eq!(refused.used, 8 * 2 * block);

    // The input cap still wins when growth would pass it.
    let refused = load(zero, limits(100_000, 8 * 4 * block), 0);
    let e = refusal(&refused);
    assert_eq!(
        (e.category, e.detail.as_str()),
        ("file_size_limit", "Input grew beyond budget")
    );
    assert_eq!(refused.stages, ["reading", "reading"]);
    assert_eq!(refused.used, 8 * block);
}
