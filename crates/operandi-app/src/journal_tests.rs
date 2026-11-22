//! The crash-recovery journal forces each write to disk (~100 ms on macOS),
//! so it must not write every frame of a drag (Jason, 2026-11-01).
use super::tests::{draw, pointer};
use super::*;
use std::time::{Duration, Instant};

/// Runs the journal as if the interval had passed, after any write
/// still running in the background has finished (one writer at a time, so
/// a running write would otherwise make this call wait instead).
fn journal_due_now(a: &mut Operandi, ctx: &Context) {
    if let Some(job) = a.journal_job.take() {
        job.join().unwrap().unwrap();
    }
    a.journal_written = Some(Instant::now() - JOURNAL_INTERVAL - Duration::from_secs(1));
    a.journal_when_due(ctx);
}

#[test]
fn journal_writes_are_at_least_15_seconds_apart_and_the_last_edit_still_lands() {
    assert_eq!(JOURNAL_INTERVAL, Duration::from_secs(15));
    let (mut a, ctx, _) = sphere_with_move();
    a.run(Command::Cancel);
    a.run(Command::NewMesh);
    journal_due_now(&mut a, &ctx);
    assert_eq!(a.journaled.as_ref(), Some(&a.editor.scene));
    if let Some(job) = a.journal_job.take() {
        job.join().unwrap().unwrap();
    }
    // A change 14 s after the last write waits, and asks to be woken when
    // the 15 s are up, so it is written even if nothing else happens.
    a.run(Command::NewMesh);
    a.journal_written = Some(Instant::now() - Duration::from_secs(14));
    // A fresh context of its own, settled, so only the journal books a repaint.
    let ctx = Context::default();
    for _ in 0..3 {
        let _ = ctx.run(Default::default(), |_| {});
    }
    let out = ctx.run(Default::default(), |ctx| a.journal_when_due(ctx));
    assert!(a.journaled.as_ref() != Some(&a.editor.scene), "waits");
    let wake = out.viewport_output[&ViewportId::ROOT].repaint_delay;
    assert!(
        wake <= Duration::from_secs(1) && wake > Duration::from_millis(500),
        "woken when the 15 s are up: {wake:?}"
    );
    // No further edits: at 15 s the pending change is written.
    a.journal_written = Some(Instant::now() - Duration::from_secs(15));
    a.journal_when_due(&ctx);
    assert_eq!(a.journaled.as_ref(), Some(&a.editor.scene));
}

/// A unit sphere with Move active, and a point on one of its handles.
fn sphere_with_move() -> (Operandi, Context, Pos2) {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: UnitDestination::ExistingMesh,
    });
    a.camera.view = View::Perspective;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    for _ in 0..3 {
        draw(&mut a, &ctx, size, vec![]);
    }
    a.activate(Tool::Move);
    for _ in 0..2 {
        draw(&mut a, &ctx, size, vec![]);
    }
    a.journal_when_due(&ctx);
    let r = a.viewport_rect;
    let handle = (0..200)
        .flat_map(|y| (0..200).map(move |x| (x, y)))
        .map(|(x, y)| r.center() + vec2(x as f32 * 2. - 200., y as f32 * 2. - 200.))
        .find(|&p| a.handle_hit(p, r).is_some())
        .expect("a Move handle on screen");
    (a, ctx, handle)
}

#[test]
fn no_journal_writes_during_a_drag_and_one_after_release() {
    let (mut a, ctx, start) = sphere_with_move();
    let size = vec2(1440., 900.);
    let written = a.journaled.clone();
    for i in 0..=20 {
        let pos = start + vec2(i as f32 * 3., i as f32 * 1.5);
        let events = if i == 0 {
            vec![
                Event::PointerMoved(start),
                pointer(start, true, Modifiers::NONE),
            ]
        } else {
            vec![Event::PointerMoved(pos)]
        };
        draw(&mut a, &ctx, size, events);
        a.journal_when_due(&ctx);
        assert!(
            a.journaled == written,
            "frame {i}: journal written mid-drag"
        );
    }
    let end = start + vec2(60., 30.);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(end, false, Modifiers::NONE)],
    );
    assert!(a.navigation.is_none());
    // Release the rate limit so the result is due now.
    journal_due_now(&mut a, &ctx);
    assert_eq!(
        a.journaled.as_ref(),
        Some(&a.editor.scene),
        "written after release"
    );
}

#[test]
fn rapid_changes_wait_for_the_interval_then_are_written() {
    let (mut a, ctx, _) = sphere_with_move();
    a.activate(Tool::Move);
    a.run(Command::Cancel);
    a.run(Command::NewMesh);
    journal_due_now(&mut a, &ctx);
    assert_eq!(a.journaled.as_ref(), Some(&a.editor.scene));
    // A second change inside the interval waits...
    a.run(Command::NewMesh);
    a.journal_when_due(&ctx);
    assert!(
        a.journaled.as_ref() != Some(&a.editor.scene),
        "rate-limited"
    );
    // ...and is written once the interval has passed (and the previous
    // background write has finished).
    journal_due_now(&mut a, &ctx);
    assert_eq!(a.journaled.as_ref(), Some(&a.editor.scene));
}

#[test]
fn journal_writes_run_off_the_frame_and_land_on_disk() {
    let (mut a, ctx, _) = sphere_with_move();
    a.run(Command::Cancel);
    a.run(Command::NewMesh);
    journal_due_now(&mut a, &ctx);
    let job = a.journal_job.take().expect("written in the background");
    job.join().unwrap().unwrap();
    let saved = persistence::load(&a.environment.paths().recovery()).unwrap();
    assert_eq!(saved, a.editor.scene);
}

#[test]
fn a_failing_recovery_copy_shows_its_dialog_once_until_a_write_succeeds() {
    let mut a = Operandi::empty();
    a.journal_result(Err("disk full".into()));
    let (title, message) = *a.file_error.take().expect("first failure: a dialog");
    assert_eq!(title, "Crash Recovery");
    assert!(message.contains("disk full"), "{message}");
    a.journal_result(Err("disk full".into()));
    assert!(a.file_error.is_none(), "then the status line only");
    assert!(a.status.contains("disk full"), "{}", a.status);
    a.journal_result(Ok(()));
    a.journal_result(Err("disk full".into()));
    assert!(a.file_error.is_some(), "shown again after a success");
}
