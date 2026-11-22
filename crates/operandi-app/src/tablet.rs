//! Pen tablet input (pen tablet plan, stage 1): pressure, tilt, rotation and
//! which end of the pen is down, for any tablet the OS drives (Wacom, Huion,
//! XP-Pen and the rest all fill the same standard macOS event fields; no
//! vendor SDKs).
//!
//! winit (and so egui) drops these on macOS, so `native.rs` copies them out
//! of every AppKit mouse and tablet event it already observes and calls
//! [`record`]. The rest of the app reads them here, platform-neutrally:
//!
//! - [`pressure_at`] matches a pointer event to the sample AppKit sent for
//!   it (same order, same place), so each sculpt dab gets the pressure of
//!   its own pointer move;
//! - [`latest`] is the newest sample (debug readout, eraser at a press);
//! - [`seen`] says whether a pen has been near the tablet this session.
//!
//! Elsewhere nothing records, so the pointer is always a mouse at full
//! pressure. A mouse or trackpad on macOS is full pressure too (only events
//! AppKit marks as tablet events carry pen values), so mouse sculpting is
//! unchanged.
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::VecDeque;

/// What is pointing (macOS `NSPointingDeviceType`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PenTool {
    /// A mouse, trackpad, or a tablet puck.
    #[default]
    Mouse,
    /// The pen's tip.
    Tip,
    /// The pen's eraser end.
    Eraser,
}

impl PenTool {
    pub fn label(self) -> &'static str {
        match self {
            PenTool::Mouse => "Mouse",
            PenTool::Tip => "Pen tip",
            PenTool::Eraser => "Eraser",
        }
    }
}

/// One pointer sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PenSample {
    /// 0..1 after the pressure curve; 1 for a mouse.
    pub pressure: f32,
    /// 0..1 as the tablet reported it.
    pub raw_pressure: f32,
    /// −1..1 each (macOS `tilt`); 0 for a mouse. Stored; no brush uses it
    /// yet.
    pub tilt_x: f32,
    pub tilt_y: f32,
    /// Barrel rotation in degrees; 0 for a mouse.
    pub rotation: f32,
    pub tool: PenTool,
    /// The pen is near the tablet.
    pub in_proximity: bool,
    /// Seconds since system start (AppKit's event timestamp), for order.
    pub timestamp: f64,
    /// Where the event happened, in the window's points from its top-left
    /// (egui's frame); `None` when unknown.
    pub pos: Option<[f32; 2]>,
}

impl Default for PenSample {
    fn default() -> Self {
        Self {
            pressure: 1.,
            raw_pressure: 1.,
            tilt_x: 0.,
            tilt_y: 0.,
            rotation: 0.,
            tool: PenTool::Mouse,
            in_proximity: false,
            timestamp: 0.,
            pos: None,
        }
    }
}

/// Preferences > Input > Pen Tablet: Pressure Curve. Soft needs a lighter
/// touch for the same effect, Firm a harder one (provisional exponents,
/// PTQ-02).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressureCurve {
    Soft,
    #[default]
    Linear,
    Firm,
}

impl PressureCurve {
    pub const ALL: [PressureCurve; 3] = [Self::Soft, Self::Linear, Self::Firm];
    pub fn label(self) -> &'static str {
        match self {
            Self::Soft => "Soft",
            Self::Linear => "Linear",
            Self::Firm => "Firm",
        }
    }
    /// The curve applied to a raw 0..1 pressure.
    pub fn apply(self, p: f32) -> f32 {
        let p = if p.is_finite() { p.clamp(0., 1.) } else { 1. };
        match self {
            Self::Soft => p.powf(0.5),
            Self::Linear => p,
            Self::Firm => p.powf(2.),
        }
    }
}

/// Preferences > Input > Pen Tablet: what the eraser end does in a sculpt
/// stroke. Invert (decided 2026-11-09) acts like holding Ctrl; Smooth like
/// holding Shift.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EraserAction {
    #[default]
    Invert,
    Smooth,
}

impl EraserAction {
    pub const ALL: [EraserAction; 2] = [Self::Invert, Self::Smooth];
    pub fn label(self) -> &'static str {
        match self {
            Self::Invert => "Invert",
            Self::Smooth => "Smooth",
        }
    }
}

/// Most samples kept between frames (a 200 Hz tablet over a long stall).
const QUEUE: usize = 512;
/// How far (points) a sample may be from its pointer event and still match.
const MATCH_SLOP: f32 = 2.;

struct Shared {
    latest: PenSample,
    /// Samples not yet matched to a pointer event, oldest first.
    queue: VecDeque<PenSample>,
    /// The pen end last reported near the tablet.
    tool: PenTool,
    in_proximity: bool,
    seen: bool,
    curve: PressureCurve,
}

thread_local! {
    /// AppKit delivers events on the main thread and eframe runs the app
    /// there too, so the state lives with that thread. (Each test runs on
    /// its own thread, so tests don't see each other's pens.)
    static SHARED: RefCell<Shared> = const {
        RefCell::new(Shared {
            latest: PenSample {
                pressure: 1.,
                raw_pressure: 1.,
                tilt_x: 0.,
                tilt_y: 0.,
                rotation: 0.,
                tool: PenTool::Mouse,
                in_proximity: false,
                timestamp: 0.,
                pos: None,
            },
            queue: VecDeque::new(),
            tool: PenTool::Mouse,
            in_proximity: false,
            seen: false,
            curve: PressureCurve::Linear,
        })
    };
}

fn with<R>(f: impl FnOnce(&mut Shared) -> R) -> R {
    SHARED.with(|s| f(&mut s.borrow_mut()))
}

/// Where in a press an event is. Only presses, drags and releases become
/// pointer events a stroke matches; hover only updates the newest sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Down,
    Drag,
    Up,
    Hover,
}

/// What the platform layer saw in one event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Raw {
    /// A pen came near the tablet (`entering`) or left it.
    Proximity { entering: bool, tool: PenTool },
    /// A tablet point: mouse down/drag/up/move from the pen, or a bare
    /// tablet point event (`Hover`). `Up` keeps the last pressure (the lift
    /// reads 0).
    Point {
        pressure: f32,
        tilt: [f32; 2],
        rotation: f32,
        phase: Phase,
        timestamp: f64,
        pos: Option<[f32; 2]>,
    },
    /// A plain mouse or trackpad event: full pressure.
    Mouse {
        phase: Phase,
        timestamp: f64,
        pos: Option<[f32; 2]>,
    },
}

/// Called by the platform layer for each event (main thread, in order).
pub fn record(raw: Raw) {
    with(|s| match raw {
        Raw::Proximity { entering, tool } => {
            s.seen = true;
            s.in_proximity = entering;
            s.tool = if entering { tool } else { PenTool::Mouse };
            s.latest.in_proximity = entering;
            s.latest.tool = if entering { tool } else { PenTool::Mouse };
        }
        Raw::Point {
            pressure,
            tilt,
            rotation,
            phase,
            timestamp,
            pos,
        } => {
            s.seen = true;
            let raw_pressure = if phase == Phase::Up {
                s.latest.raw_pressure
            } else if pressure.is_finite() {
                pressure.clamp(0., 1.)
            } else {
                1.
            };
            // A point without a proximity event first (some drivers): the
            // tip.
            let tool = if s.tool == PenTool::Mouse {
                PenTool::Tip
            } else {
                s.tool
            };
            let sample = PenSample {
                pressure: s.curve.apply(raw_pressure),
                raw_pressure,
                tilt_x: finite(tilt[0]).clamp(-1., 1.),
                tilt_y: finite(tilt[1]).clamp(-1., 1.),
                rotation: finite(rotation),
                tool,
                in_proximity: true,
                timestamp,
                pos,
            };
            push(s, sample, phase);
        }
        Raw::Mouse {
            phase,
            timestamp,
            pos,
        } => {
            let sample = PenSample {
                timestamp,
                pos,
                ..PenSample::default()
            };
            push(s, sample, phase);
        }
    });
}

fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0. }
}

fn push(s: &mut Shared, sample: PenSample, phase: Phase) {
    s.latest = sample;
    match phase {
        Phase::Hover => return,
        // A press starts a new run: older samples belong to events already
        // handled (or never matched).
        Phase::Down => s.queue.clear(),
        Phase::Drag | Phase::Up => {}
    }
    if s.queue.len() >= QUEUE {
        s.queue.pop_front();
    }
    s.queue.push_back(sample);
}

/// The newest sample.
pub fn latest() -> PenSample {
    with(|s| s.latest)
}

/// Whether a pen has been near the tablet this session (SC-62's Low
/// Pressure needs one).
pub fn seen() -> bool {
    with(|s| s.seen)
}

/// Sets the Pressure Curve (applied once, here at the source).
pub fn set_curve(curve: PressureCurve) {
    with(|s| s.curve = curve);
}

/// The sample for a pointer event at `pos` (window points): the oldest
/// queued sample at that place, dropping the older ones it passed (their
/// pointer events came before). Without a match (the event wasn't one
/// AppKit sent through this window, or nothing was recorded at all), the
/// newest sample, or full pressure when none.
pub fn sample_at(pos: [f32; 2]) -> PenSample {
    with(|s| {
        let near = |q: &PenSample| {
            q.pos.is_some_and(|p| {
                (p[0] - pos[0]).abs() <= MATCH_SLOP && (p[1] - pos[1]).abs() <= MATCH_SLOP
            })
        };
        if let Some(i) = s.queue.iter().position(near) {
            let found = s.queue[i];
            s.queue.drain(..=i);
            found
        } else {
            s.latest
        }
    })
}

/// The (curved) pressure for a pointer event at `pos`; see [`sample_at`].
pub fn pressure_at(pos: [f32; 2]) -> f32 {
    sample_at(pos).pressure
}

/// Debug readout (`OPERANDI_PEN_DEBUG=1`): off by default.
pub fn readout_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("OPERANDI_PEN_DEBUG").is_ok_and(|v| !v.is_empty() && v != "0"))
}

/// The readout's text.
pub fn readout() -> String {
    let s = latest();
    format!(
        "Pen: {} · pressure {:.2} (raw {:.2}) · tilt {:+.2}, {:+.2} · rotation {:.0}° · {}{}",
        s.tool.label(),
        s.pressure,
        s.raw_pressure,
        s.tilt_x,
        s.tilt_y,
        s.rotation,
        if s.in_proximity {
            "in proximity"
        } else {
            "out of proximity"
        },
        if seen() { "" } else { " · no pen seen yet" },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(p: f32, x: f32) -> Raw {
        Raw::Point {
            pressure: p,
            tilt: [0.25, -0.5],
            rotation: 30.,
            phase: Phase::Drag,
            timestamp: x as f64,
            pos: Some([x, 10.]),
        }
    }

    #[test]
    fn mouse_is_full_pressure() {
        assert_eq!(pressure_at([5., 5.]), 1.);
        record(Raw::Mouse {
            phase: Phase::Down,
            timestamp: 1.,
            pos: Some([5., 5.]),
        });
        assert_eq!(pressure_at([5., 5.]), 1.);
        assert_eq!(latest().tool, PenTool::Mouse);
        assert!(!seen());
    }

    #[test]
    fn samples_match_their_pointer_events_in_order() {
        record(Raw::Proximity {
            entering: true,
            tool: PenTool::Tip,
        });
        for (i, p) in [0.1, 0.4, 0.8].into_iter().enumerate() {
            record(point(p, 100. + 10. * i as f32));
        }
        assert!(seen());
        assert_eq!(pressure_at([100., 10.]), 0.1);
        assert_eq!(pressure_at([110.5, 10.]), 0.4);
        assert_eq!(pressure_at([120., 10.]), 0.8);
        // Nothing left to match: the newest.
        assert_eq!(pressure_at([500., 10.]), 0.8);
        let s = latest();
        assert_eq!((s.tilt_x, s.tilt_y, s.rotation), (0.25, -0.5, 30.));
        assert_eq!(s.tool, PenTool::Tip);
    }

    #[test]
    fn skipped_samples_are_dropped() {
        for (i, p) in [0.2, 0.3, 0.9].into_iter().enumerate() {
            record(point(p, 10. * i as f32));
        }
        // egui merged the first two moves: the third matches, the rest go.
        assert_eq!(pressure_at([20., 10.]), 0.9);
        assert_eq!(pressure_at([0., 10.]), 0.9);
    }

    #[test]
    fn hover_and_old_presses_never_match() {
        record(Raw::Proximity {
            entering: true,
            tool: PenTool::Tip,
        });
        // An earlier drag left a sample here, then the pen hovered (0).
        record(point(0.7, 50.));
        record(Raw::Point {
            pressure: 0.,
            tilt: [0.; 2],
            rotation: 0.,
            phase: Phase::Hover,
            timestamp: 3.,
            pos: Some([50., 10.]),
        });
        assert_eq!(latest().pressure, 0.);
        // The new press, at the same place.
        record(Raw::Point {
            pressure: 0.3,
            tilt: [0.; 2],
            rotation: 0.,
            phase: Phase::Down,
            timestamp: 4.,
            pos: Some([50., 10.]),
        });
        record(point(0.9, 60.));
        assert_eq!(pressure_at([50., 10.]), 0.3);
        assert_eq!(pressure_at([60., 10.]), 0.9);
    }

    #[test]
    fn curve_is_applied_at_the_source() {
        set_curve(PressureCurve::Firm);
        record(point(0.5, 1.));
        let s = latest();
        assert_eq!(s.raw_pressure, 0.5);
        assert!((s.pressure - 0.25).abs() < 1e-6);
        set_curve(PressureCurve::Soft);
        record(point(0.25, 2.));
        assert!((latest().pressure - 0.5).abs() < 1e-6);
        assert_eq!(PressureCurve::Linear.apply(0.3), 0.3);
        assert_eq!(PressureCurve::Soft.apply(f32::NAN), 1.);
    }

    #[test]
    fn eraser_end_and_lift() {
        record(Raw::Proximity {
            entering: true,
            tool: PenTool::Eraser,
        });
        record(point(0.6, 1.));
        assert_eq!(latest().tool, PenTool::Eraser);
        // The lift reads 0 pressure; keep the last.
        record(Raw::Point {
            pressure: 0.,
            tilt: [0.; 2],
            rotation: 0.,
            phase: Phase::Up,
            timestamp: 2.,
            pos: Some([1., 10.]),
        });
        assert_eq!(latest().raw_pressure, 0.6);
        record(Raw::Proximity {
            entering: false,
            tool: PenTool::Eraser,
        });
        assert_eq!(latest().tool, PenTool::Mouse);
        assert!(!latest().in_proximity);
        assert!(seen());
    }
}
