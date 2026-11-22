//! M-budget allocation MEASUREMENT (test only): requested heap bytes and
//! allocation counts of real `lxo::parse` / `lxo::load` calls on four admitted
//! fixtures, observed beside `Control::used`.
//!
//! A counting global allocator exists only in this integration-test binary. It
//! delegates every call to `std::alloc::System` with unchanged arguments and
//! returns System's result unchanged. It never injects failures.
//!
//! Measurement boundary:
//! - Only allocations made on a thread while that thread is *armed* (a
//!   const-initialized thread-local flag, set by a drop guard) are tracked.
//!   Harness, output and other threads' new allocations are excluded.
//! - Each tracked block's address and requested size live in a fixed static
//!   table. The block stays tracked through reallocation and is subtracted
//!   when freed, on any thread, during or after the window. Frees of blocks
//!   allocated outside a window are counted separately and never subtracted.
//! - Bytes are `Layout::size()` as requested. Allocator rounding, metadata,
//!   alignment slack, stack, OS mappings, file-system caches and any GPU or
//!   thread activity are not measured. Nothing here is peak RSS.
//!
//! Address-reuse ordering (amendment 1): a tracked entry is removed from the
//! table *before* System may release its address (`dealloc`, and `realloc`,
//! which may move the block), and an entry is inserted only *after* System has
//! handed the address to the caller. Because System's own synchronization
//! orders a release before any reuse of that address, no other thread can
//! free or allocate a reused address while a stale entry for it exists. If
//! `realloc` returns null the old block is unchanged and its entry is put back.
//!
//! Hook safety (amendment 1): hooks touch only atomics and the table. All
//! counter arithmetic is checked; overflow, underflow or a full table sets
//! `error` instead of panicking, and windows check it outside the hooks.
//! Table indices are masked by a power-of-two size, so indexing is in bounds.
//!
//! `parse` receives a source buffer read before its window, so that buffer is
//! excluded and reported as input bytes. `load` reads its own buffer inside the
//! window, so it is included.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};

// ---------------------------------------------------------------------------
// Bookkeeping (no allocation, no panic)

const EMPTY: usize = 0;
/// A removed entry; System never returns `usize::MAX` as a block address.
const TOMBSTONE: usize = usize::MAX;

/// Open-addressed table of tracked block addresses and requested sizes.
struct Table<const N: usize> {
    ptrs: [AtomicUsize; N],
    sizes: [AtomicUsize; N],
}

impl<const N: usize> Table<N> {
    const POWER_OF_TWO: () = assert!(N.is_power_of_two());

    const fn new() -> Self {
        let () = Self::POWER_OF_TWO;
        Self {
            ptrs: [const { AtomicUsize::new(EMPTY) }; N],
            sizes: [const { AtomicUsize::new(0) }; N],
        }
    }

    fn slot(ptr: usize, probe: usize) -> usize {
        (ptr >> 4)
            .wrapping_mul(0x9e37_79b9_7f4a_7c15_u64 as usize)
            .wrapping_add(probe)
            & (N - 1)
    }

    fn insert(&self, ptr: usize, size: usize) -> bool {
        for probe in 0..N {
            let i = Self::slot(ptr, probe);
            let current = self.ptrs[i].load(SeqCst);
            if (current == EMPTY || current == TOMBSTONE)
                && self.ptrs[i]
                    .compare_exchange(current, ptr, SeqCst, SeqCst)
                    .is_ok()
            {
                self.sizes[i].store(size, SeqCst);
                return true;
            }
        }
        false
    }

    fn remove(&self, ptr: usize) -> Option<usize> {
        for probe in 0..N {
            let i = Self::slot(ptr, probe);
            match self.ptrs[i].load(SeqCst) {
                EMPTY => return None,
                p if p == ptr => {
                    let size = self.sizes[i].load(SeqCst);
                    self.ptrs[i].store(TOMBSTONE, SeqCst);
                    return Some(size);
                }
                _ => {}
            }
        }
        None
    }

    /// Live entries. Called only outside hooks.
    fn occupied(&self) -> usize {
        self.ptrs
            .iter()
            .filter(|p| !matches!(p.load(SeqCst), EMPTY | TOMBSTONE))
            .count()
    }

    /// Called only outside hooks, when no block is tracked.
    fn clear(&self) {
        for p in &self.ptrs {
            p.store(EMPTY, SeqCst);
        }
    }
}

/// Checked atomic addition; on overflow sets `error` and leaves `c` unchanged.
fn add(c: &AtomicUsize, n: usize, error: &AtomicBool) -> usize {
    match c.try_update(SeqCst, SeqCst, |v| v.checked_add(n)) {
        Ok(previous) => previous.wrapping_add(n),
        Err(current) => {
            error.store(true, SeqCst);
            current
        }
    }
}

/// Checked atomic subtraction; on underflow sets `error`.
fn sub(c: &AtomicUsize, n: usize, error: &AtomicBool) {
    if c.try_update(SeqCst, SeqCst, |v| v.checked_sub(n)).is_err() {
        error.store(true, SeqCst);
    }
}

struct Counters {
    /// Requested bytes of tracked blocks currently allocated.
    live: AtomicUsize,
    peak: AtomicUsize,
    tracked_blocks: AtomicUsize,
    allocs: AtomicUsize,
    alloc_bytes: AtomicUsize,
    frees: AtomicUsize,
    reallocs: AtomicUsize,
    untracked_frees: AtomicUsize,
    untracked_reallocs: AtomicUsize,
    /// Set by a hook on arithmetic overflow/underflow or a full table.
    error: AtomicBool,
}

impl Counters {
    const fn new() -> Self {
        Self {
            live: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            tracked_blocks: AtomicUsize::new(0),
            allocs: AtomicUsize::new(0),
            alloc_bytes: AtomicUsize::new(0),
            frees: AtomicUsize::new(0),
            reallocs: AtomicUsize::new(0),
            untracked_frees: AtomicUsize::new(0),
            untracked_reallocs: AtomicUsize::new(0),
            error: AtomicBool::new(false),
        }
    }
}

/// The allocator's bookkeeping, separate from `System` so that controls can
/// drive the exact same transitions on a private instance with synthetic
/// addresses (no memory is ever touched through an address).
struct Tracker<const N: usize> {
    table: Table<N>,
    c: Counters,
}

impl<const N: usize> Tracker<N> {
    const fn new() -> Self {
        Self {
            table: Table::new(),
            c: Counters::new(),
        }
    }

    fn bump(&self, counter: &AtomicUsize) {
        add(counter, 1, &self.c.error);
    }

    fn add_live(&self, size: usize) {
        let live = add(&self.c.live, size, &self.c.error);
        self.c.peak.fetch_max(live, SeqCst);
    }

    /// After System returned `ptr` to an armed caller.
    fn on_alloc(&self, ptr: usize, size: usize) {
        if self.table.insert(ptr, size) {
            self.bump(&self.c.allocs);
            add(&self.c.alloc_bytes, size, &self.c.error);
            self.bump(&self.c.tracked_blocks);
            self.add_live(size);
        } else {
            self.c.error.store(true, SeqCst);
        }
    }

    /// Before System may release `ptr`.
    fn on_free(&self, ptr: usize, armed: bool) {
        if let Some(size) = self.table.remove(ptr) {
            sub(&self.c.live, size, &self.c.error);
            sub(&self.c.tracked_blocks, 1, &self.c.error);
            self.bump(&self.c.frees);
        } else if armed {
            self.bump(&self.c.untracked_frees);
        }
    }

    /// Reallocation of `old` through `system`, which may move or release it.
    /// The old entry leaves the table before `system` runs and the new entry
    /// enters only after it returns.
    fn on_realloc(
        &self,
        old: usize,
        new_size: usize,
        armed: bool,
        system: impl FnOnce() -> usize,
    ) -> usize {
        let tracked = self.table.remove(old);
        if let Some(size) = tracked {
            sub(&self.c.live, size, &self.c.error);
        }
        let new = system();
        match tracked {
            // Failure: the old block is unchanged, so its entry returns.
            Some(size) if new == 0 => {
                if !self.table.insert(old, size) {
                    self.c.error.store(true, SeqCst);
                }
                self.add_live(size);
            }
            Some(_) => {
                if !self.table.insert(new, new_size) {
                    self.c.error.store(true, SeqCst);
                }
                self.bump(&self.c.reallocs);
                self.add_live(new_size);
            }
            // A block from before the window resized inside it: counted
            // apart and tracked from here at its full new size.
            None if new != 0 && armed => {
                self.bump(&self.c.untracked_reallocs);
                self.on_alloc(new, new_size);
            }
            None => {}
        }
        new
    }

    fn live(&self) -> usize {
        self.c.live.load(SeqCst)
    }
}

// ---------------------------------------------------------------------------
// Counting allocator

static T: Tracker<{ 1 << 16 }> = Tracker::new();

thread_local! {
    // A const-initialized `Cell<bool>` needs no lazy registration or
    // destructor, so reading it from an allocator hook does not allocate.
    static ARMED: Cell<bool> = const { Cell::new(false) };
}

fn armed() -> bool {
    ARMED.try_with(Cell::get).unwrap_or(false)
}

struct Counting;

// SAFETY: every method forwards the caller's arguments unchanged to `System`,
// which upholds the `GlobalAlloc` contract, and returns its result unchanged
// (including null). The extra work only updates atomics and a static table of
// addresses and sizes: it never dereferences a block, never allocates, never
// formats, and uses only checked or wrapping arithmetic and in-bounds masked
// indices, so it cannot panic.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded unchanged from the caller's contract.
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() && armed() {
            T.on_alloc(p as usize, layout.size());
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded unchanged from the caller's contract.
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() && armed() {
            T.on_alloc(p as usize, layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        T.on_free(ptr as usize, armed());
        // SAFETY: forwarded unchanged from the caller's contract.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new = T.on_realloc(ptr as usize, new_size, armed(), || {
            // SAFETY: forwarded unchanged from the caller's contract.
            unsafe { System.realloc(ptr, layout, new_size) as usize }
        });
        new as *mut u8
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

// ---------------------------------------------------------------------------
// Windows

/// Serializes windows on the global tracker, because live and peak bytes are
/// process-wide. Controls on private trackers do not take it. Nothing waits on
/// another lock while holding it, so it cannot deadlock under the default
/// parallel test harness.
static WINDOWS: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, PartialEq)]
struct Window {
    allocs: usize,
    requested_bytes: usize,
    frees_in_window: usize,
    reallocs: usize,
    peak_live: usize,
    retained_at_return: usize,
    untracked_frees: usize,
    untracked_reallocs: usize,
}

fn counts() -> [usize; 6] {
    let c = &T.c;
    [
        c.allocs.load(SeqCst),
        c.alloc_bytes.load(SeqCst),
        c.frees.load(SeqCst),
        c.reallocs.load(SeqCst),
        c.untracked_frees.load(SeqCst),
        c.untracked_reallocs.load(SeqCst),
    ]
}

/// Disarms on drop, including while a panic unwinds out of the window.
struct ArmGuard;

impl ArmGuard {
    fn arm() -> Self {
        ARMED.with(|a| a.set(true));
        Self
    }
}

impl Drop for ArmGuard {
    fn drop(&mut self) {
        let _ = ARMED.try_with(|a| a.set(false));
    }
}

/// The tracker is consistent and empty: no hook error, no tracked block, no
/// live bytes, and no table entry. Only then may the table be cleared.
fn assert_quiescent() {
    assert!(!T.c.error.load(SeqCst), "allocator hook reported an error");
    assert_eq!(T.c.tracked_blocks.load(SeqCst), 0, "blocks still tracked");
    assert_eq!(T.live(), 0, "live bytes without tracked blocks");
    assert_eq!(
        T.table.occupied(),
        0,
        "table entries without tracked blocks"
    );
}

/// Run `f` armed on this thread. Tracking of its blocks continues after the
/// window, so `T.live()` after dropping the result shows what was released.
/// If `f` panics, the guard disarms and the panic continues unchanged.
fn window<R>(f: impl FnOnce() -> R) -> (R, Window) {
    assert_quiescent();
    T.table.clear();
    T.c.peak.store(0, SeqCst);
    let before = counts();
    let result = {
        let _armed = ArmGuard::arm();
        f()
    };
    let after = counts();
    let d = |i: usize| after[i].wrapping_sub(before[i]);
    let w = Window {
        allocs: d(0),
        requested_bytes: d(1),
        frees_in_window: d(2),
        reallocs: d(3),
        peak_live: T.c.peak.load(SeqCst),
        retained_at_return: T.live(),
        untracked_frees: d(4),
        untracked_reallocs: d(5),
    };
    assert!(!T.c.error.load(SeqCst), "allocator hook reported an error");
    (result, w)
}

fn live_now() -> usize {
    T.live()
}

fn lock() -> std::sync::MutexGuard<'static, ()> {
    WINDOWS.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Instrumentation controls on the real allocator (global tracker)

#[test]
fn instrumentation_controls() {
    let _guard = lock();

    // One allocation retained past the window, released after it.
    let (v, w) = window(|| Vec::<u8>::with_capacity(1000));
    assert_eq!(
        (
            w.allocs,
            w.requested_bytes,
            w.peak_live,
            w.retained_at_return
        ),
        (1, 1000, 1000, 1000)
    );
    drop(v);
    assert_eq!(live_now(), 0);
    println!("CONTROL retained_then_dropped {w:?}");

    // Growth and shrink through realloc.
    let (v, w) = window(|| {
        let mut v = Vec::<u8>::with_capacity(16);
        v.reserve_exact(4096);
        v.shrink_to(100);
        v
    });
    assert_eq!(v.capacity(), 100);
    assert_eq!(
        (w.allocs, w.reallocs, w.peak_live, w.retained_at_return),
        (1, 2, 4096, 100)
    );
    drop(v);
    assert_eq!(live_now(), 0);
    println!("CONTROL realloc_grow_shrink {w:?}");

    // Allocated and freed inside the window.
    let ((), w) = window(|| drop(vec![1u8; 300]));
    assert_eq!(
        (
            w.allocs,
            w.frees_in_window,
            w.peak_live,
            w.retained_at_return
        ),
        (1, 1, 300, 0)
    );
    println!("CONTROL freed_in_window {w:?}");

    // A block from before the window, freed inside it, is not subtracted.
    let pre = vec![0u8; 500];
    let ((), w) = window(|| drop(pre));
    assert_eq!(
        (
            w.allocs,
            w.untracked_frees,
            w.peak_live,
            w.retained_at_return
        ),
        (0, 1, 0, 0)
    );
    println!("CONTROL preexisting_freed {w:?}");

    // A block from before the window, grown inside it, is counted as an
    // untracked realloc and tracked from then on at its full new size.
    let mut pre = Vec::<u8>::with_capacity(8);
    let (grown, w) = window(move || {
        pre.reserve_exact(2048);
        pre
    });
    assert_eq!(
        (
            w.allocs,
            w.reallocs,
            w.untracked_reallocs,
            w.retained_at_return
        ),
        (1, 0, 1, grown.capacity())
    );
    drop(grown);
    assert_eq!(live_now(), 0);
    println!("CONTROL untracked_realloc {w:?}");

    // Another thread allocating during the window is not counted.
    static GO: AtomicBool = AtomicBool::new(false);
    static DONE: AtomicBool = AtomicBool::new(false);
    let worker = std::thread::spawn(|| {
        while !GO.load(SeqCst) {
            std::hint::spin_loop();
        }
        drop(std::hint::black_box(vec![7u8; 777]));
        DONE.store(true, SeqCst);
    });
    let ((), w) = window(|| {
        GO.store(true, SeqCst);
        while !DONE.load(SeqCst) {
            std::hint::spin_loop();
        }
    });
    worker.join().unwrap();
    assert_eq!((w.allocs, w.peak_live, w.untracked_frees), (0, 0, 0));
    println!("CONTROL other_thread_excluded {w:?}");
    assert_quiescent();
}

/// A tracked block reallocated and freed on another thread, after the window,
/// is still followed: its new size replaces the old, and its free subtracts it.
#[test]
fn tracked_block_lifetime_across_threads() {
    let _guard = lock();
    let (v, w) = window(|| Vec::<u8>::with_capacity(64));
    assert_eq!((w.allocs, w.retained_at_return), (1, 64));
    let reallocs = T.c.reallocs.load(SeqCst);
    let frees = T.c.frees.load(SeqCst);
    let capacity = std::thread::spawn(move || {
        let mut v = v;
        v.reserve_exact(10_000);
        let capacity = v.capacity();
        assert_eq!(live_now(), capacity, "regrown size replaces the old one");
        drop(v);
        capacity
    })
    .join()
    .unwrap();
    assert!(capacity >= 10_000);
    assert_eq!(T.c.reallocs.load(SeqCst) - reallocs, 1);
    assert_eq!(T.c.frees.load(SeqCst) - frees, 1);
    assert_eq!(live_now(), 0);
    assert_quiescent();
    println!("CONTROL cross_thread_lifetime realloc->{capacity} then free on another thread");
}

/// A panic inside a window disarms the thread and reaches the caller with its
/// original payload. The payload is allocated before the window and resumed
/// without the panic hook, so the control itself allocates nothing armed.
#[test]
fn panic_in_window_disarms_and_keeps_the_original_payload() {
    let _guard = lock();
    let payload: Box<dyn std::any::Any + Send> = Box::new("original window failure");
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        window(move || {
            std::panic::resume_unwind(payload);
        })
    }));
    let payload = caught.expect_err("the window must propagate the panic");
    assert_eq!(
        payload.downcast_ref::<&str>(),
        Some(&"original window failure")
    );
    assert!(!armed(), "thread still armed after the panic");
    drop(payload);
    assert_quiescent();
    // The next window works normally.
    let ((), w) = window(|| drop(vec![0u8; 10]));
    assert_eq!((w.allocs, w.frees_in_window), (1, 1));
}

// ---------------------------------------------------------------------------
// Bookkeeping controls on private trackers (synthetic addresses, no memory)

/// The address-reuse transition, forced deterministically. Inside `system`
/// (the exact point where System may move the block and release the old
/// address), a second real thread frees a block that was reallocated at the old
/// address, through the same `on_free` path. That free must find no tracked
/// entry, and the moved block must stay tracked at its new size.
///
/// This forces the interleaving with a join inside the system step; it does not
/// claim that uncontrolled allocator scheduling reproduced a race.
#[test]
fn moving_realloc_releases_the_old_address_before_reuse() {
    const OLD: usize = 0x1000;
    const NEW: usize = 0x2000;
    let t: Tracker<8> = Tracker::new();
    t.on_alloc(OLD, 64);
    let new = t.on_realloc(OLD, 256, true, || {
        // Another thread now owns and frees the reused old address.
        std::thread::scope(|s| {
            s.spawn(|| t.on_free(OLD, false));
        });
        NEW
    });
    assert_eq!(new, NEW);
    assert_eq!(
        t.c.frees.load(SeqCst),
        0,
        "reused address freed a tracked entry"
    );
    assert_eq!(t.c.reallocs.load(SeqCst), 1);
    assert_eq!(t.live(), 256);
    assert_eq!(t.table.occupied(), 1);
    t.on_free(NEW, false);
    assert_eq!((t.live(), t.c.frees.load(SeqCst)), (0, 1));
    assert!(!t.c.error.load(SeqCst));

    // The first-freeze order (System first, table second) under the same
    // forced interleaving loses the moved block: the control detects it.
    let t: Tracker<8> = Tracker::new();
    t.on_alloc(OLD, 64);
    let new = {
        std::thread::scope(|s| {
            s.spawn(|| t.on_free(OLD, false));
        });
        let new = NEW;
        if let Some(size) = t.table.remove(OLD) {
            t.table.insert(new, 256);
            sub(&t.c.live, size, &t.c.error);
            t.add_live(256);
        }
        new
    };
    assert_eq!(new, NEW);
    assert_eq!(
        t.c.frees.load(SeqCst),
        1,
        "old order: reuse freed the tracked entry"
    );
    assert_eq!(t.table.occupied(), 0, "old order: moved block untracked");
    println!("CONTROL moving_realloc_order protected; first-freeze order loses the block");
}

/// A failed realloc (System returns null) leaves the old block tracked as it
/// was; an unarmed realloc of an untracked block is ignored.
#[test]
fn failed_and_unarmed_realloc_boundaries() {
    let t: Tracker<8> = Tracker::new();
    t.on_alloc(0x1000, 40);
    assert_eq!(t.on_realloc(0x1000, 4000, true, || 0), 0);
    assert_eq!(
        (t.live(), t.table.occupied(), t.c.reallocs.load(SeqCst)),
        (40, 1, 0)
    );
    assert_eq!(t.c.peak.load(SeqCst), 40);
    assert_eq!(t.on_realloc(0x3000, 50, false, || 0x4000), 0x4000);
    assert_eq!(t.c.untracked_reallocs.load(SeqCst), 0);
    assert_eq!(t.on_realloc(0x5000, 50, true, || 0x6000), 0x6000);
    assert_eq!(t.c.untracked_reallocs.load(SeqCst), 1);
    assert_eq!(t.live(), 90);
    assert!(!t.c.error.load(SeqCst));
}

/// A full table and counter overflow set the error flag instead of panicking
/// inside a hook; windows check that flag outside the hooks.
#[test]
fn table_exhaustion_and_counter_overflow_signal_without_panicking() {
    let t: Tracker<8> = Tracker::new();
    for i in 1..=8 {
        t.on_alloc(i * 0x100, 1);
    }
    assert!(!t.c.error.load(SeqCst));
    t.on_alloc(0x9000, 1);
    assert!(t.c.error.load(SeqCst), "full table must be signalled");
    assert_eq!(t.table.occupied(), 8);

    let t: Tracker<8> = Tracker::new();
    t.c.live.store(usize::MAX - 1, SeqCst);
    t.on_alloc(0x100, 10);
    assert!(t.c.error.load(SeqCst), "live overflow must be signalled");
    assert_eq!(t.live(), usize::MAX - 1, "overflowing add leaves the value");

    let t: Tracker<8> = Tracker::new();
    t.table.insert(0x100, 10); // entry without matching live bytes
    t.on_free(0x100, false);
    assert!(t.c.error.load(SeqCst), "live underflow must be signalled");

    let error = AtomicBool::new(false);
    let c = AtomicUsize::new(usize::MAX);
    assert_eq!(add(&c, 1, &error), usize::MAX);
    assert!(error.load(SeqCst));
}
