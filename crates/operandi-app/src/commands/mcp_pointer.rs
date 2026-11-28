//! Bridge calls that act as the person does (painting test drive,
//! 2026-11-27), so an agent can test painting and other pointer tools in
//! the running app (docs/MCP_BRIDGE.md):
//!
//! - `pointer_stroke`: a press, drag and release along screen points,
//!   one point per frame, through the app's real input path
//!   (`raw_input_hook`), with a mouse button, held keys and, optionally,
//!   pen pressure per point (recorded as tablet samples, as a pen's
//!   events are). Answered once the release has been handled.
//! - `app_command`: one of the app's own command lines (the paint tools,
//!   `operandi.paint.addTexture`, `operandi.image.*` ...), which the
//!   headless `command_run` can't reach. Recorded in the History tab as a
//!   bridge line.
use super::*;
use crate::tablet::{self, Phase, Raw};
use operandi_core::Source;
use serde_json::{Value, json};
use std::collections::VecDeque;

/// The longest stroke one call sends (points; one frame each).
const MAX_POINTS: usize = 4000;

/// A stroke being fed to the app, one frame at a time.
pub(crate) struct StrokeRun {
    frames: VecDeque<StrokeFrame>,
    modifiers: Modifiers,
}

/// One frame's pointer events, and the pen sample that goes with them.
struct StrokeFrame {
    events: Vec<Event>,
    /// Phase and place of this frame's pointer event, and its pressure
    /// (`None`: a mouse, full pressure).
    pen: Option<(Phase, Pos2, Option<f32>)>,
}

fn keys(tool: &str, args: &Value, known: &[&str]) -> std::result::Result<(), String> {
    match args {
        Value::Null => Ok(()),
        Value::Object(m) => operandi_core::command_exec::check_keys(tool, m, known),
        _ => Err(format!("{tool}'s arguments are an object")),
    }
}

fn button(args: &Value) -> std::result::Result<PointerButton, String> {
    match args.get("button").and_then(Value::as_str) {
        None | Some("left") => Ok(PointerButton::Primary),
        Some("right") => Ok(PointerButton::Secondary),
        Some("middle") => Ok(PointerButton::Middle),
        Some(other) => Err(format!("button is left, right or middle, not {other}")),
    }
}

fn modifiers(args: &Value) -> std::result::Result<Modifiers, String> {
    let mut m = Modifiers::NONE;
    let Some(list) = args.get("modifiers").filter(|v| !v.is_null()) else {
        return Ok(m);
    };
    let list = list
        .as_array()
        .ok_or("modifiers is a list: shift, ctrl, alt, cmd")?;
    for key in list {
        match key.as_str() {
            Some("shift") => m.shift = true,
            Some("ctrl") => {
                m.ctrl = true;
                m.command |= !cfg!(target_os = "macos");
            }
            Some("alt") => m.alt = true,
            Some("cmd") => {
                // The window's "Cmd": Command on macOS, Control elsewhere.
                m.command = true;
                if cfg!(target_os = "macos") {
                    m.mac_cmd = true;
                } else {
                    m.ctrl = true;
                }
            }
            _ => return Err(format!("modifiers are shift, ctrl, alt, cmd, not {key}")),
        }
    }
    Ok(m)
}

/// `pressure`: one number for every point, or one per point, 0..1.
fn pressures(args: &Value, n: usize) -> std::result::Result<Option<Vec<f32>>, String> {
    let fraction = |v: &Value| {
        v.as_f64()
            .filter(|p| (0. ..=1.).contains(p))
            .map(|p| p as f32)
            .ok_or_else(|| "pressure is 0 to 1".to_string())
    };
    match args.get("pressure").filter(|v| !v.is_null()) {
        None => Ok(None),
        Some(Value::Array(list)) if list.len() == n => list
            .iter()
            .map(fraction)
            .collect::<std::result::Result<Vec<f32>, String>>()
            .map(Some),
        Some(Value::Array(list)) => Err(format!(
            "pressure has {} values for {n} points; give one per point or a single number",
            list.len()
        )),
        Some(v) => Ok(Some(vec![fraction(v)?; n])),
    }
}

fn numbers(v: &Value, len: usize, what: &str) -> std::result::Result<Vec<f64>, String> {
    let list = v
        .as_array()
        .filter(|l| l.len() == len)
        .ok_or_else(|| format!("each {what} is [{}]", ["x", "y", "z"][..len].join(", ")))?;
    list.iter()
        .map(|c| {
            c.as_f64()
                .ok_or_else(|| format!("{what} coordinates are numbers"))
        })
        .collect()
}

/// Seconds for the tablet samples' order.
fn now_seconds() -> f64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_secs_f64()
}

impl Operandi {
    /// `app_command`: run one app command line as a bridge line. A line
    /// that opens a dialog is reported as an error: the dialog waits in
    /// the app for the person (bridge calls never answer dialogs).
    pub(super) fn mcp_app_command(&mut self, args: &Value) -> std::result::Result<Value, String> {
        keys("app_command", args, &["line"])?;
        let line = args
            .get("line")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .ok_or("app_command needs a line, e.g. \"operandi.paint.airbrush\"")?
            .to_string();
        let result = self.execute_reporting(Source::Bridge, &line);
        if let Some(dialog) = self.open_dialog() {
            return Err(format!(
                "{line} opened the {dialog} in Operandi, which now waits for the person; give the command its values to run it without one"
            ));
        }
        result?;
        let mut out = self.mcp_summary();
        out["line"] = json!(line);
        out["status"] = json!(self.status);
        Ok(out)
    }

    /// The scene echo every editing call answers with.
    fn mcp_summary(&self) -> Value {
        operandi_mcp::Bridge {
            editor: self.editor.clone(),
            ..Default::default()
        }
        .summary()
    }

    /// `pointer_stroke`: check the arguments and queue the stroke's frames.
    pub(super) fn mcp_pointer_stroke(&mut self, args: &Value) -> std::result::Result<(), String> {
        keys(
            "pointer_stroke",
            args,
            &[
                "points",
                "world",
                "viewport",
                "button",
                "modifiers",
                "pressure",
            ],
        )?;
        if self.mcp_stroke_running() {
            return Err("A pointer stroke is still running; wait for its answer".into());
        }
        if self.navigation.is_some() || self.pointer_held {
            return Err("Operandi is in the middle of a mouse drag; try again".into());
        }
        let button = button(args)?;
        let held = modifiers(args)?;
        let given = |k: &str| args.get(k).filter(|v| !v.is_null());
        let (list, world) = match (given("points"), given("world")) {
            (Some(p), None) => (p, false),
            (None, Some(w)) => (w, true),
            _ => {
                return Err(
                    "Give points ([x, y] in the view) or world ([x, y, z]), not both".into(),
                );
            }
        };
        let list = list.as_array().ok_or("points is a list of points")?;
        if list.is_empty() || list.len() > MAX_POINTS {
            return Err(format!("A stroke has 1 to {MAX_POINTS} points"));
        }
        let pressure = pressures(args, list.len())?;
        // The view first, so its rectangle and camera are the ones used.
        if let Some(v) = given("viewport") {
            let views = self.views.viewports();
            let index = v.as_u64().ok_or("viewport is an index, 0 or more")? as usize;
            let view = *views
                .get(index)
                .ok_or_else(|| format!("No viewport {index}; there are {}", views.len()))?;
            self.activate_view(view);
        }
        let rect = self.viewport_rect;
        if rect.width() < 1. || rect.height() < 1. {
            return Err("The viewport hasn't been drawn yet; try again".into());
        }
        let mut at = Vec::with_capacity(list.len());
        for p in list {
            let q = if world {
                let v = numbers(p, 3, "world point")?;
                self.camera
                    .project(glam::DVec3::new(v[0], v[1], v[2]), rect)
                    .map(|(q, _)| q)
                    .ok_or("A world point is behind the view's camera")?
            } else {
                let v = numbers(p, 2, "point")?;
                rect.min + vec2(v[0] as f32, v[1] as f32)
            };
            if !rect.contains(q) {
                return Err(format!(
                    "Point ({:.0}, {:.0}) is outside the view ({:.0} × {:.0} points)",
                    q.x - rect.min.x,
                    q.y - rect.min.y,
                    rect.width(),
                    rect.height()
                ));
            }
            at.push(q);
        }
        let pen = |k: usize| pressure.as_ref().map(|p| p[k]);
        let press = |pos, pressed| Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: held,
        };
        let mut frames = VecDeque::new();
        // Hover first, as a real pointer arrives before it presses.
        frames.push_back(StrokeFrame {
            events: vec![Event::PointerMoved(at[0])],
            pen: Some((Phase::Hover, at[0], pen(0))),
        });
        frames.push_back(StrokeFrame {
            events: vec![press(at[0], true)],
            pen: Some((Phase::Down, at[0], pen(0))),
        });
        for (k, &q) in at.iter().enumerate().skip(1) {
            frames.push_back(StrokeFrame {
                events: vec![Event::PointerMoved(q)],
                pen: Some((Phase::Drag, q, pen(k))),
            });
        }
        let last = at.len() - 1;
        frames.push_back(StrokeFrame {
            events: vec![press(at[last], false)],
            pen: Some((Phase::Up, at[last], pen(last))),
        });
        // A settled frame, then the pointer leaves (the person's own mouse
        // takes over again when it moves).
        frames.push_back(StrokeFrame {
            events: Vec::new(),
            pen: None,
        });
        frames.push_back(StrokeFrame {
            events: vec![Event::PointerGone],
            pen: None,
        });
        self.mcp.stroke = Some(StrokeRun {
            frames,
            modifiers: held,
        });
        Ok(())
    }

    pub(super) fn mcp_stroke_running(&self) -> bool {
        self.mcp.stroke.is_some()
    }

    /// From `raw_input_hook`: the running stroke's events for this frame,
    /// with its keys held and its pen sample recorded.
    pub(super) fn mcp_take_stroke_input(&mut self, input: &mut RawInput) {
        let Some(run) = &mut self.mcp.stroke else {
            return;
        };
        let Some(frame) = run.frames.pop_front() else {
            self.mcp.stroke = None;
            return;
        };
        input.modifiers = run.modifiers;
        if let Some((phase, pos, pressure)) = frame.pen {
            let (timestamp, pos) = (now_seconds(), Some([pos.x, pos.y]));
            tablet::record(match pressure {
                Some(pressure) => Raw::Point {
                    pressure,
                    tilt: [0., 0.],
                    rotation: 0.,
                    phase,
                    timestamp,
                    pos,
                },
                None => Raw::Mouse {
                    phase,
                    timestamp,
                    pos,
                },
            });
        }
        input.events.extend(frame.events);
        if run.frames.is_empty() {
            self.mcp.stroke = None;
        }
    }

    /// A finished stroke's answer: the scene echo and the status line.
    pub(super) fn mcp_stroke_answer(&self) -> Value {
        let mut out = self.mcp_summary();
        out["status"] = json!(self.status);
        out
    }
}
