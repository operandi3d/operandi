//! The app's recent log and status lines, kept in memory for bug and crash
//! reports (bug_report.rs). Nothing is written to disk or sent anywhere from
//! here; a report includes the tail only when the user asks for one.
//!
//! `log_line!` prints to stderr like `eprintln!` and also keeps the line.
//! Status-bar messages are recorded once per change by `note_status`.
use std::collections::VecDeque;
use std::sync::Mutex;

/// Lines kept; reports use the last ~30.
const CAPACITY: usize = 200;
/// A single line longer than this is cut, so one huge message can't crowd
/// out the rest of a report.
const MAX_LINE: usize = 400;

struct Ring {
    lines: VecDeque<String>,
    last_status: String,
}

static RING: Mutex<Ring> = Mutex::new(Ring {
    lines: VecDeque::new(),
    last_status: String::new(),
});

/// Prints like `eprintln!` and keeps the line for reports.
macro_rules! log_line {
    ($($arg:tt)*) => {{
        let line = format!($($arg)*);
        eprintln!("{line}");
        $crate::log_ring::push(line);
    }};
}
pub(crate) use log_line;

fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Time of day in UTC; the crash file and report carry the full date.
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600 % 24,
        secs / 60 % 60,
        secs % 60
    )
}

/// Keeps one line (time-stamped). Never panics or blocks for long: a
/// poisoned lock is recovered, since the panic hook reads this too.
pub fn push(line: impl Into<String>) {
    let mut line: String = line.into();
    if line.len() > MAX_LINE {
        let mut cut = MAX_LINE;
        while !line.is_char_boundary(cut) {
            cut -= 1;
        }
        line.truncate(cut);
        line.push('…');
    }
    let stamped = format!("{} {line}", timestamp());
    let mut ring = RING.lock().unwrap_or_else(|e| e.into_inner());
    if ring.lines.len() == CAPACITY {
        ring.lines.pop_front();
    }
    ring.lines.push_back(stamped);
}

/// Records the status-bar message when it changes (called once a frame).
pub fn note_status(status: &str) {
    {
        let mut ring = RING.lock().unwrap_or_else(|e| e.into_inner());
        if ring.last_status == status {
            return;
        }
        ring.last_status = status.to_owned();
    }
    if !status.is_empty() {
        push(format!("status: {status}"));
    }
}

/// The last `n` lines, oldest first.
pub fn tail(n: usize) -> Vec<String> {
    let ring = RING.lock().unwrap_or_else(|e| e.into_inner());
    ring.lines
        .iter()
        .skip(ring.lines.len().saturating_sub(n))
        .cloned()
        .collect()
}

/// For the panic hook: never waits on a lock another thread holds (that
/// thread may be the one panicking).
pub fn try_tail(n: usize) -> Vec<String> {
    match RING.try_lock() {
        Ok(ring) => ring
            .lines
            .iter()
            .skip(ring.lines.len().saturating_sub(n))
            .cloned()
            .collect(),
        Err(std::sync::TryLockError::Poisoned(e)) => {
            let ring = e.into_inner();
            ring.lines
                .iter()
                .skip(ring.lines.len().saturating_sub(n))
                .cloned()
                .collect()
        }
        Err(std::sync::TryLockError::WouldBlock) => vec!["(log busy at crash time)".into()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The ring is process-wide, so one test exercises it end to end.
    #[test]
    fn keeps_the_newest_lines_cuts_long_ones_and_dedupes_status() {
        for i in 0..CAPACITY + 5 {
            push(format!("line {i}"));
        }
        let last = tail(3);
        assert_eq!(last.len(), 3);
        assert!(last[2].ends_with(&format!("line {}", CAPACITY + 4)));
        assert!(last[0].ends_with(&format!("line {}", CAPACITY + 2)));
        assert!(tail(10_000).len() <= CAPACITY);

        push("é".repeat(MAX_LINE));
        let long = tail(1).pop().unwrap();
        assert!(long.ends_with('…') && long.len() < MAX_LINE + 20);

        note_status("Ready to test the ring");
        note_status("Ready to test the ring");
        let statuses = tail(CAPACITY)
            .iter()
            .filter(|l| l.ends_with("status: Ready to test the ring"))
            .count();
        assert_eq!(statuses, 1, "an unchanged status is recorded once");
        assert_eq!(try_tail(1), tail(1));
    }
}
