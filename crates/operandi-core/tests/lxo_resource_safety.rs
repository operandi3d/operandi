//! LXO-SAFE-001 / LXO-FMT-001 resource budgets (D3, format/01 "Approved
//! resource limits"): catalog M-size-limit and the testable subset of M-budget
//! in `acceptance/tests.json`.
//!
//! Evidence comes in two kinds, and each test says which kind it gives:
//! - real importer evidence: actual `lxo::parse` / `lxo::load` calls on
//!   admitted triangle copies or a task-owned sparse file;
//! - direct accounting arithmetic: public `Control::charge` calls with no
//!   importer involved.
//!
//! The progress callback and `Control::used` are the observable signals. `load`
//! checks the file size before its first "reading" progress call, and `parse`
//! checks it after its first "framing" call; neither charges working memory
//! before that check. No allocator is instrumented, so nothing here measures
//! peak allocation. Per-allocation failure injection (the rest of M-budget) has
//! no seam in the current API and is not exercised.
//!
//! Temporary files live only under this test binary's `CARGO_TARGET_TMPDIR`, in
//! a per-process directory that is removed afterwards.

use operandi_core::lxo;
use std::io::{Read, Seek, SeekFrom, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

const MIB: usize = 1024 * 1024;
/// M-size-limit `declared_file_length`: one byte over the 256 MiB default.
const OVERSIZED: u64 = 268_435_457;

/// One importer call with the progress stages it reported and the working
/// memory it charged.
struct Run {
    result: lxo::Result<lxo::Candidate>,
    stages: Vec<String>,
    used: usize,
}

fn run(
    limits: lxo::Limits,
    call: impl FnOnce(&mut lxo::Control<'_>) -> lxo::Result<lxo::Candidate>,
) -> Run {
    let mut stages = Vec::new();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let mut progress = |stage: &str| {
            stages.push(stage.to_owned());
            true
        };
        let mut control = lxo::Control::new(&mut progress);
        control.limits = limits;
        let result = call(&mut control);
        (result, control.used)
    }));
    let (result, used) = outcome.expect("importer panicked");
    Run {
        result,
        stages,
        used,
    }
}

fn load(path: &Path, limits: lxo::Limits) -> Run {
    run(limits, |c| lxo::load(path, c))
}

fn input_limit(input_bytes: usize) -> lxo::Limits {
    lxo::Limits {
        input_bytes,
        ..lxo::Limits::default()
    }
}

/// The refusal category, with the diagnostic bounds every refusal must meet.
fn refusal(run: &Run, input_len: u64) -> &lxo::Error {
    let e = run.result.as_ref().expect_err("expected a refusal");
    assert!(e.detail.chars().count() <= 768, "unbounded detail: {e:?}");
    assert!(e.to_string().chars().count() <= 1024, "unbounded message");
    assert!(e.offset as u64 <= input_len, "offset outside input: {e:?}");
    e
}

/// A per-process directory under this test binary's temporary area.
struct TaskDir(PathBuf);

impl TaskDir {
    fn new(test: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("lxo-resource-safety")
            .join(format!("{test}-{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn entries(&self) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TaskDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn read_range(path: &Path, at: u64, len: usize) -> Vec<u8> {
    let mut file = std::fs::File::open(path).unwrap();
    file.seek(SeekFrom::Start(at)).unwrap();
    let mut out = vec![0; len];
    file.read_exact(&mut out).unwrap();
    out
}

/// The approved D3 defaults are what `Control::new` uses; nothing below relaxes
/// them except the input limit, which is only ever lowered.
#[test]
fn default_limits_are_the_approved_d3_budgets() {
    let limits = lxo::Limits::default();
    assert_eq!(limits.input_bytes, 256 * MIB);
    assert_eq!(limits.working_bytes, 1024 * MIB);
    let mut progress = |_: &str| true;
    let control = lxo::Control::new(&mut progress);
    assert_eq!(control.limits.input_bytes, 256 * MIB);
    assert_eq!(control.limits.working_bytes, 1024 * MIB);
    assert_eq!(control.used, 0);
    assert_eq!(OVERSIZED, 256 * MIB as u64 + 1);
}

/// M-size-limit, real importer evidence. A sparse file one byte over the
/// default input limit, starting with a FORM/LXOB header whose length matches
/// the file, is refused by `load` before any read block or parse stage runs.
#[test]
fn m_size_limit_sparse_file_is_refused_from_its_size_before_reading() {
    let dir = TaskDir::new("m-size-limit");
    let sentinel_bytes = b"operandi lxo resource-safety sentinel\n";
    let sentinel = dir.write("sentinel.bin", sentinel_bytes);

    let path = dir.0.join("oversized.lxo");
    let mut header = Vec::new();
    header.extend_from_slice(b"FORM");
    header.extend_from_slice(&((OVERSIZED - 8) as u32).to_be_bytes());
    header.extend_from_slice(b"LXOB");
    {
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&header).unwrap();
        file.set_len(OVERSIZED).unwrap();
        file.seek(SeekFrom::Start(OVERSIZED - 1)).unwrap();
        file.write_all(&[0x5a]).unwrap();
    }
    let before = std::fs::metadata(&path).unwrap();
    assert_eq!(before.len(), OVERSIZED);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let allocated = before.blocks() * 512;
        println!(
            "sparse fixture: logical {} bytes, allocated {allocated} bytes ({})",
            before.len(),
            if allocated < OVERSIZED / 16 {
                "sparse"
            } else {
                "NOT sparse on this file system"
            }
        );
    }

    let started = std::time::Instant::now();
    let refused = load(&path, lxo::Limits::default());
    let elapsed = started.elapsed();
    let e = refusal(&refused, OVERSIZED);
    assert_eq!(e.category, "file_size_limit");
    assert_eq!(e.offset, 0);
    assert_eq!(e.detail, "Input exceeds 256 MiB");
    // No "reading" block and no "framing" stage ran, and nothing was charged:
    // the refusal came from the file's metadata, not from buffered bytes.
    assert!(
        refused.stages.is_empty(),
        "stages ran: {:?}",
        refused.stages
    );
    assert_eq!(refused.used, 0);
    println!("sparse load refused in {elapsed:?}: {e}");

    // The fixture and sentinel are untouched; only small ranges are read back.
    let after = std::fs::metadata(&path).unwrap();
    assert_eq!(after.len(), OVERSIZED);
    assert_eq!(after.modified().unwrap(), before.modified().unwrap());
    assert_eq!(read_range(&path, 0, 12), header);
    assert_eq!(read_range(&path, OVERSIZED - 2, 2), [0, 0x5a]);
    assert_eq!(std::fs::read(&sentinel).unwrap(), sentinel_bytes);
    assert_eq!(dir.entries(), ["oversized.lxo", "sentinel.bin"]);
}

/// Stream bound, real importer evidence: a source whose metadata reports no
/// length is still capped while reading. Uses a lowered input limit so the
/// read stops after two 64 KiB blocks.
#[cfg(unix)]
#[test]
fn unbounded_stream_is_capped_while_reading() {
    let zero = Path::new("/dev/zero");
    assert_eq!(std::fs::metadata(zero).unwrap().len(), 0);
    let refused = load(zero, input_limit(100_000));
    let e = refusal(&refused, 100_000);
    assert_eq!(e.category, "file_size_limit");
    assert_eq!(e.detail, "Input grew beyond budget");
    assert_eq!(refused.stages, ["reading", "reading"]);
    // Budget preflight (2026-10-28): the first block's buffer capacity is
    // charged its 8× reserve before the buffer grows; the second block is
    // refused by the input cap before any further growth or charge.
    assert_eq!(refused.used, 8 * 65_536);
}

/// `Control::charge` arithmetic, direct accounting evidence only: zero, exact,
/// over and overflow boundaries. No importer runs and nothing large is
/// allocated; the counts are just numbers.
#[test]
fn charge_arithmetic_boundaries() {
    let mut progress = |_: &str| true;
    let mut c = lxo::Control::new(&mut progress);
    let over = |e: lxo::Error| {
        assert_eq!((e.category, e.offset), ("resource_limit", 0));
        e.detail
    };

    // Zero budget: a zero charge fits, any byte does not.
    c.limits.working_bytes = 0;
    assert!(c.charge(0).is_ok());
    assert_eq!(c.used, 0);
    assert_eq!(
        over(c.charge(1).unwrap_err()),
        "Aggregate import working memory budget exceeded"
    );
    // Observed: a refused charge stays counted. The importer abandons its
    // Control after any error, so this is recorded, not relied on.
    assert_eq!(c.used, 1);

    // Exact budget: reaching the limit is allowed, passing it is not.
    c.used = 0;
    c.limits.working_bytes = 1000;
    assert!(c.charge(999).is_ok());
    assert!(c.charge(1).is_ok());
    assert!(c.charge(0).is_ok());
    assert_eq!(c.used, 1000);
    assert_eq!(
        over(c.charge(1).unwrap_err()),
        "Aggregate import working memory budget exceeded"
    );

    // A huge request under the default budget is a budget refusal, not an
    // overflow, and does not allocate.
    c.used = 0;
    c.limits = lxo::Limits::default();
    assert_eq!(
        over(c.charge(usize::MAX).unwrap_err()),
        "Aggregate import working memory budget exceeded"
    );

    // Checked addition: the sum may reach usize::MAX but not wrap past it, and
    // an overflowing charge leaves the count unchanged.
    c.limits.working_bytes = usize::MAX;
    c.used = usize::MAX - 1;
    assert!(c.charge(1).is_ok());
    assert_eq!(c.used, usize::MAX);
    assert_eq!(over(c.charge(1).unwrap_err()), "Working memory overflow");
    assert_eq!(c.used, usize::MAX);
    c.used = 5;
    assert_eq!(
        over(c.charge(usize::MAX).unwrap_err()),
        "Working memory overflow"
    );
    assert_eq!(c.used, 5);
}
