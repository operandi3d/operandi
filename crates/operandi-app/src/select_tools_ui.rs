//! Select tab tools (modelling parity batch 4, SELECTION-TOOLS SL-xx): the
//! "..." dialogs (Sharp, Coplanar Edges, Colinear Vertices, Diagonal,
//! Conditional Loop Tool), the live Coplanar Polygons dialog, and the
//! Lazy Selection and Shortest Path Mode toggles. Every run is
//! `Command::SelectTool` (select_tools.rs): one undo step that changes only
//! the selection. Dialogs use the shared dialog (UIC-80..99) and keep their
//! last values (UIC-95).
use super::*;
use operandi_core::DetriangulateOptions;
use operandi_core::select_tools::{
    COLINEAR_DISTANCE, COPLANAR_FLATNESS, ConditionalLoopOptions, CoplanarPolygonOptions,
    SHARP_MAX, SHARP_MIN, SelectRequest,
};

/// Which dialog is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SelectDialogKind {
    Sharp,
    CoplanarEdges,
    Colinear,
    Diagonal,
    ConditionalLoop,
    /// SL-50: live; each change re-runs from the start polygons.
    CoplanarPolygons,
}
impl SelectDialogKind {
    fn title(self) -> &'static str {
        match self {
            Self::Sharp => "Select Sharp Edges",
            Self::CoplanarEdges => "Select Coplanar Edges",
            Self::Colinear => "Select Colinear Vertices",
            Self::Diagonal => "Select Diagonal Edges",
            Self::ConditionalLoop => "Conditional Loop Tool",
            Self::CoplanarPolygons => "Coplanar Polygons Tool",
        }
    }
    fn rows(self) -> Vec<form::Row> {
        use form::{Row, Unit};
        match self {
            Self::Sharp => vec![
                Row::number("min", "Minimum Angle", Unit::Angle, SHARP_MIN).range(0., 180.),
                Row::number("max", "Maximum Angle", Unit::Angle, SHARP_MAX)
                    .range(0., 180.)
                    .tip("Edges whose polygons meet at a normal angle in this range (0° is flat)"),
            ],
            Self::CoplanarEdges => vec![
                Row::number("flatness", "Flatness", Unit::Scalar, COPLANAR_FLATNESS)
                    .range(0., 1.)
                    .tip("How far the pair may bend, as a share of its size (provisional, SL-Q1)"),
            ],
            Self::Colinear => vec![
                Row::number("distance", "Distance", Unit::Distance, COLINEAR_DISTANCE)
                    .range(0., 1e6)
                    .tip("How far off the line through its neighbours a point may sit"),
            ],
            Self::Diagonal => {
                let d = DetriangulateOptions::default();
                vec![
                    Row::number("flatness", "Flatness", Unit::Percent, d.flatness).range(0., 1000.),
                    Row::number("max_angle", "Max Angle", Unit::Angle, d.max_angle).range(1., 180.),
                    Row::check("keep_material", "Keep Material Boundary", d.keep_material),
                    Row::check("keep_part", "Keep Part Boundary", d.keep_part),
                    Row::check(
                        "keep_smoothing",
                        "Keep Smoothing Group Boundary",
                        d.keep_smoothing,
                    ),
                    Row::check("keep_sets", "Keep Selection Set Boundary", d.keep_sets),
                ]
            }
            Self::ConditionalLoop => {
                let d = ConditionalLoopOptions::default();
                vec![
                    Row::check("both", "Loop Both Directions", d.both_directions),
                    Row::number("max_loop", "Max Loop", Unit::Count, d.max_loop as f64)
                        .range(0., 1e6)
                        .tip("0 loops on until a stop rule ends it"),
                    Row::check("shared", "Stop at Shared Polygon", d.stop_shared_polygon),
                    Row::check("sharp", "Stop at Sharp Angles", d.stop_sharp),
                    Row::number("sharp_angle", "Sharp Angle", Unit::Angle, d.sharp_angle)
                        .range(0., 180.)
                        .tip("A turn sharper than this stops the loop (provisional, SL-Q4)"),
                    Row::check("length", "Stop at Length Difference", d.stop_length),
                    Row::number(
                        "length_tolerance",
                        "Length Tolerance",
                        Unit::Percent,
                        d.length_tolerance * 100.,
                    )
                    .range(0., 100_000.),
                    Row::check("selection", "Stop at Selection", d.stop_selection),
                    Row::number(
                        "selection_count",
                        "Selection Count",
                        Unit::Count,
                        d.selection_count as f64,
                    )
                    .range(1., 1e6),
                ]
            }
            Self::CoplanarPolygons => {
                let d = CoplanarPolygonOptions::default();
                vec![
                    Row::number("angle", "Angle", Unit::Angle, d.angle).range(0., 180.),
                    Row::check("connect", "Use Connectivity", d.connectivity)
                        .tip("Only polygons reached from the start through polygons that qualify"),
                    Row::number("range", "Height Range", Unit::Distance, d.range)
                        .range(0., 1e6)
                        .tip("With Use Connectivity off: how far from the start polygon's plane"),
                ]
            }
        }
    }
}

/// The open dialog.
pub(super) struct SelectDialog {
    pub(super) kind: SelectDialogKind,
    pub(super) form: form::Dialog,
    /// Coplanar Polygons: the start polygons, the values last run and how
    /// many undo steps the dialog has added (Cancel takes them back).
    start: Vec<u32>,
    last: Option<CoplanarPolygonOptions>,
    steps: usize,
}

impl SelectDialog {
    /// The request the dialog's values describe.
    fn request(&self) -> SelectRequest {
        let v = &self.form.values;
        match self.kind {
            SelectDialogKind::Sharp => SelectRequest::Sharp {
                min: v.number("min"),
                max: v.number("max"),
            },
            SelectDialogKind::CoplanarEdges => SelectRequest::CoplanarEdges {
                flatness: v.number("flatness"),
            },
            SelectDialogKind::Colinear => SelectRequest::ColinearVertices {
                distance: v.number("distance"),
            },
            SelectDialogKind::Diagonal => SelectRequest::Diagonal(DetriangulateOptions {
                flatness: v.number("flatness"),
                max_angle: v.number("max_angle"),
                keep_material: v.bool("keep_material"),
                keep_part: v.bool("keep_part"),
                keep_smoothing: v.bool("keep_smoothing"),
                keep_sets: v.bool("keep_sets"),
                ..Default::default()
            }),
            SelectDialogKind::ConditionalLoop => {
                SelectRequest::ConditionalLoop(ConditionalLoopOptions {
                    both_directions: v.bool("both"),
                    max_loop: v.number("max_loop").round().max(0.) as u32,
                    stop_shared_polygon: v.bool("shared"),
                    stop_sharp: v.bool("sharp"),
                    sharp_angle: v.number("sharp_angle"),
                    stop_length: v.bool("length"),
                    length_tolerance: v.number("length_tolerance") / 100.,
                    stop_selection: v.bool("selection"),
                    selection_count: v.number("selection_count").round().max(1.) as u32,
                })
            }
            SelectDialogKind::CoplanarPolygons => SelectRequest::CoplanarPolygons {
                start: self.start.clone(),
                options: self.coplanar(),
            },
        }
    }
    fn coplanar(&self) -> CoplanarPolygonOptions {
        let v = &self.form.values;
        CoplanarPolygonOptions {
            angle: v.number("angle"),
            connectivity: v.bool("connect"),
            range: v.number("range"),
        }
    }
}

/// `name:value` arguments over a dialog's starting values; the dialog's
/// row keys are the argument names (Operandi's own, SL spec gives none).
fn parse_args(
    kind: SelectDialogKind,
    values: &mut form::Values,
    rest: &str,
) -> std::result::Result<(), String> {
    let rows = kind.rows();
    for arg in rest.split_whitespace() {
        let (key, value) = operandi_core::command_line::named(arg);
        let Some(key) = key else {
            return Err(format!(
                "{}: arguments are name:value, not \"{arg}\"",
                kind.title()
            ));
        };
        let row = rows
            .iter()
            .find(|r| r.key.eq_ignore_ascii_case(key))
            .ok_or_else(|| format!("{}: unknown argument \"{arg}\"", kind.title()))?;
        let parsed = match values.get(&row.key) {
            Some(form::Value::Bool(_)) => {
                form::Value::Bool(operandi_core::command_args::boolean(key, value)?)
            }
            _ => form::Value::Number(
                value
                    .parse::<f64>()
                    .map_err(|_| format!("{}: {key} must be a number", kind.title()))?,
            ),
        };
        values.set(&row.key, parsed);
    }
    Ok(())
}

impl Operandi {
    /// A "..." button: open its dialog with the last values. With
    /// arguments (a script line) it runs at once with them.
    pub(super) fn select_tool_line(&mut self, kind: SelectDialogKind, rest: &str) {
        if self.select_dialog.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = format!("Close the open dialog before {}", kind.title());
            return;
        }
        let form = self.form_dialog(kind.title(), kind.rows());
        let mut dialog = SelectDialog {
            kind,
            form,
            start: self.editor.scene.selection.faces.clone(),
            last: None,
            steps: 0,
        };
        if !rest.trim().is_empty() {
            match parse_args(kind, &mut dialog.form.values, rest) {
                Ok(()) => {
                    let request = dialog.request();
                    self.run_select_tool(request);
                }
                Err(e) => self.status = e,
            }
            return;
        }
        if kind == SelectDialogKind::CoplanarPolygons {
            if self.editor.scene.selection.mode != Mode::Polygon || dialog.start.is_empty() {
                self.status = "Coplanar Polygons Tool: select a polygon to start from".into();
                return;
            }
            self.coplanar_live(&mut dialog);
        }
        self.select_dialog = Some(dialog);
    }

    fn run_select_tool(&mut self, request: SelectRequest) -> bool {
        let label = request.label();
        let before = self.editor.undo.len();
        let ok = self.run_ok(Command::SelectTool(request));
        if ok {
            let s = &self.editor.scene.selection;
            let count = match s.mode {
                Mode::Vertex => s.vertices.len(),
                Mode::Edge => s.edges.len(),
                Mode::Polygon => s.faces.len(),
                Mode::Item => 0,
            };
            self.status = if self.editor.undo.len() == before {
                format!("{label}: nothing new to select")
            } else {
                format!("{label}: {count} selected")
            };
        }
        ok
    }

    /// SL-50/CP1: re-run from the start polygons when the values changed.
    fn coplanar_live(&mut self, dialog: &mut SelectDialog) {
        let now = dialog.coplanar();
        if dialog.last == Some(now) {
            return;
        }
        dialog.last = Some(now);
        let before = self.editor.undo.len();
        if self.run_select_tool(dialog.request()) && self.editor.undo.len() > before {
            dialog.steps += 1;
        }
    }

    /// The open dialog, once per frame.
    pub(super) fn select_tools_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.select_dialog.take() else {
            return;
        };
        let title = dialog.kind.title();
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => {
                // Coplanar Polygons takes back what it selected.
                for _ in 0..dialog.steps {
                    self.run(Command::Undo);
                }
                self.status = format!("{title} canceled; nothing changed");
            }
            form::Outcome::Ok => {
                self.remember_form(&dialog.form);
                if dialog.kind == SelectDialogKind::CoplanarPolygons {
                    self.coplanar_live(&mut dialog);
                    return;
                }
                let request = dialog.request();
                if !self.run_select_tool(request) {
                    dialog.form.error = Some(self.status.clone());
                    self.select_dialog = Some(dialog);
                }
            }
            _ => {
                if dialog.kind == SelectDialogKind::CoplanarPolygons {
                    self.coplanar_live(&mut dialog);
                }
                self.select_dialog = Some(dialog);
            }
        }
    }

    /// Conditional Loop (SL-60): the button runs with the starting values
    /// and Loop From: Last.
    pub(super) fn conditional_loop(&mut self) {
        self.run_select_tool(SelectRequest::ConditionalLoop(Default::default()));
    }

    /// Shortest Path (SL-01).
    pub(super) fn shortest_path(&mut self) {
        self.run_select_tool(SelectRequest::ShortestPath);
    }

    /// Lazy Selection and Shortest Path Mode: global toggles kept with the
    /// selection preferences.
    pub(super) fn toggle_selection_preference(&mut self, lazy: bool) {
        let p = &mut self.selection_preferences;
        let (name, on) = if lazy {
            p.lazy_selection = !p.lazy_selection;
            ("Lazy Selection", p.lazy_selection)
        } else {
            p.shortest_path_mode = !p.shortest_path_mode;
            ("Shortest Path Mode", p.shortest_path_mode)
        };
        self.status = format!("{name} {}", if on { "on" } else { "off" });
        if let Err(error) = self
            .selection_preferences
            .save(&self.environment.paths().selection_preferences())
        {
            self.status = format!("{name} changed; could not save it: {error}");
        }
    }

    /// SL-03 (provisional, SL-Q6): with Shortest Path Mode on, a Shift+click
    /// that added an element also adds the path to it from the element
    /// selected before. Call after the click's selection change.
    pub(super) fn shortest_path_after_click(&mut self, shift: bool, ctrl: bool) {
        if !self.selection_preferences.shortest_path_mode || !shift || ctrl {
            return;
        }
        let s = &self.editor.scene.selection;
        let count = match s.mode {
            Mode::Vertex => s.vertices.len(),
            Mode::Edge => s.edges.len(),
            Mode::Polygon => s.faces.len(),
            Mode::Item => 0,
        };
        if count < 2 {
            return;
        }
        // The path from the element before the click to the clicked one.
        let _ = self.run_ok(Command::SelectTool(SelectRequest::PathToLast));
    }
}
