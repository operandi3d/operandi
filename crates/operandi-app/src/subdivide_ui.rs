//! Edit tab, wave 1 (docs/specs/edit-wave1/SUBDIVIDE.md): the Slide button,
//! the Center row, the Subdivide group, Freeze..., their dialogs, commands
//! and remembered settings.
//!
//! Every button, shortcut and command runs the same core commands:
//! `Command::SubdividePolygons` (each button sets only the Method and reuses
//! the remembered options, SUB-05), `Command::UnsubdividePolygons`,
//! `Command::CenterSelection` and `Command::Freeze`.
use super::*;
pub(super) use operandi_core::command_line::{parse_subdivide, parse_unsubdivide};
use operandi_core::polygon_subdivide::{center_axes, center_axes_word};
use operandi_core::{SubdivideBoundary, SubdivideMethod, SubdivideOptions, UnsubdivideOptions};

/// Why Background Constraint is greyed (SUB-30).
const NO_BACKGROUND: &str = "Not built yet for Subdivide (Edit ▸ Snapping ▸ Constrain to Background works with tools that move points)";

/// Subdivide and UnSubdivide settings saved in the data folder, so the last
/// used ones survive a restart (SUB-05 provisional).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(super) struct SavedSettings {
    pub subdivide: SubdivideOptions,
    pub unsubdivide: UnsubdivideOptions,
    pub freeze: FreezeOptions,
    /// Detriangulate's options (DT-10, UIC-95), kept with these.
    pub detriangulate: operandi_core::DetriangulateOptions,
}

/// SUB-50 Freeze curves choice (core, FRZ-04).
pub(super) use operandi_core::freeze::FreezeCurves;

/// SUB-50 (corrected) Freeze options and their starting values. Curves as,
/// Tessellation, Fixed Spans and Curve Refinement Angle act (FRZ v1); the
/// others are shown greyed (Not available yet).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(super) struct FreezeOptions {
    pub curves_as: FreezeCurves,
    pub make_holes: bool,
    /// 0 X, 1 Y, 2 Z.
    pub drill_axis: u8,
    pub displacement: bool,
    pub deformation: bool,
    pub tessellation: bool,
    pub fixed_spans: bool,
    pub refinement_angle: String,
    pub save_morph: bool,
    pub morph_name: String,
}

impl Default for FreezeOptions {
    fn default() -> Self {
        Self {
            curves_as: FreezeCurves::Faces,
            make_holes: false,
            drill_axis: 1,
            displacement: true,
            deformation: true,
            tessellation: true,
            fixed_spans: true,
            refinement_angle: "5.0 °".into(),
            save_morph: false,
            morph_name: String::new(),
        }
    }
}

impl FreezeOptions {
    /// The options the core Freeze acts on.
    pub(super) fn core(&self) -> operandi_core::freeze::FreezeOptions {
        operandi_core::freeze::FreezeOptions {
            curves_as: self.curves_as,
            tessellation: self.tessellation,
            fixed_spans: self.fixed_spans,
            refinement_degrees: self.refinement_degrees(),
        }
    }
    /// Curve Refinement Angle in degrees, clamped to its range (FRZ-Q5).
    pub(super) fn refinement_degrees(&self) -> f64 {
        use operandi_core::freeze::{MAX_REFINEMENT_DEGREES, MIN_REFINEMENT_DEGREES};
        form::Unit::Angle
            .parse(&self.refinement_angle)
            .unwrap_or(5.)
            .clamp(MIN_REFINEMENT_DEGREES, MAX_REFINEMENT_DEGREES)
    }
}

/// The remembered Curve Refinement Angle text: one decimal, or exact for
/// angles under 0.1° so they don't round to 0.
fn angle_text(degrees: f64) -> String {
    if degrees < 0.1 {
        format!("{degrees} °")
    } else {
        format!("{degrees:.1} °")
    }
}

/// `poly.freeze` arguments (reference names, FRZ "Command") over the
/// remembered options. `face` takes 0 Faces, 1 Polylines, 2 2 Point
/// Polygons (Operandi-only) or those names;
/// `cangle` is radians, or degrees with a unit ("30deg"). Arguments for
/// options that don't act yet are accepted and ignored; the second value
/// lists the ones given.
pub(super) fn parse_freeze(
    base: &FreezeOptions,
    args: &[&str],
) -> Result<(FreezeOptions, Vec<&'static str>)> {
    use operandi_core::command_args::boolean;
    use operandi_core::command_line::named;
    let mut o = base.clone();
    let mut inert = Vec::new();
    for arg in args {
        let (Some(key), value) = named(arg) else {
            return Err(format!(
                "poly.freeze takes name:value arguments, not \"{arg}\""
            ));
        };
        let flag = |name| boolean(name, value).map_err(|e| e.to_string());
        match key.to_ascii_lowercase().as_str() {
            "face" => o.curves_as = FreezeCurves::parse(value).ok_or_else(|| {
                format!(
                    "face takes 0 (Faces), 1 (Polylines) or 2 (2 Point Polygons), not \"{value}\""
                )
            })?,
            "drill" => {
                o.make_holes = flag("drill")?;
                inert.push("drill");
            }
            "axis" => {
                o.drill_axis = match value.to_ascii_lowercase().as_str() {
                    "0" | "x" => 0,
                    "1" | "y" => 1,
                    "2" | "z" => 2,
                    _ => return Err(format!("axis takes 0, 1 or 2 (X, Y, Z), not \"{value}\"")),
                };
                inert.push("axis");
            }
            "disp" => {
                o.displacement = flag("disp")?;
                inert.push("disp");
            }
            "deform" => {
                o.deformation = flag("deform")?;
                inert.push("deform");
            }
            "tess" => o.tessellation = flag("tess")?,
            "fixed" => o.fixed_spans = flag("fixed")?,
            "cangle" => {
                let degrees = operandi_core::command_names::script_angle(value)
                    .map_err(|e| format!("cangle: {e}"))?;
                if !degrees.is_finite() || degrees <= 0. {
                    return Err("cangle must be above 0".into());
                }
                o.refinement_angle =
                    angle_text(degrees.min(operandi_core::freeze::MAX_REFINEMENT_DEGREES));
            }
            "morphsave" => {
                o.save_morph = flag("morphSave")?;
                inert.push("morphSave");
            }
            "morphname" => {
                o.morph_name = value.to_owned();
                inert.push("morphName");
            }
            _ => return Err(format!("poly.freeze: unknown argument \"{arg}\"")),
        }
    }
    Ok((o, inert))
}

/// The Freeze... dialog (shared form, UIC-80..99).
pub(super) struct FreezeDialog {
    form: form::Dialog,
}
impl FreezeDialog {
    const CURVES: [FreezeCurves; 3] = FreezeCurves::ALL;
    /// SUB-50 (corrected) rows, top to bottom. Fields that don't act yet are
    /// greyed at their values with "Not available yet" (UIC-92); Curve
    /// Refinement Angle is greyed while Fixed Spans is on (FRZ options). Freeze's own saved settings are its memory (UIC-95).
    fn rows(o: &FreezeOptions) -> Vec<form::Row> {
        use form::{Row, Unit, When};
        use operandi_core::freeze::{MAX_REFINEMENT_DEGREES, MIN_REFINEMENT_DEGREES};
        let na = "Not available yet";
        let curves = o.curves_as.index();
        let angle = o.refinement_degrees();
        vec![
            Row::choice(
                "curves_as",
                "Freeze Curves as",
                &["2 Point Polygons", "Faces", "Polylines"],
                curves,
            )
            .tip("What each curve becomes: a chain of two-point lines, one Face through its samples, or one straight polyline"),
            Row::check("make_holes", "Make Holes", o.make_holes).when(When::Never).tip(na),
            Row::segments("drill_axis", "Drill Axis", &["X", "Y", "Z"], usize::from(o.drill_axis.min(2)))
                .when(When::Never)
                .tip(na),
            Row::check("displacement", "Freeze Displacement", o.displacement)
                .when(When::Never)
                .tip(na),
            Row::check("deformation", "Freeze Deformation", o.deformation)
                .when(When::Never)
                .tip(na),
            Row::check("tessellation", "Tessellation", o.tessellation).tip(
                "On: subdivision-surface polygons become plain polygons at the display level. Off: nothing freezes, curves included",
            ),
            Row::check("fixed_spans", "Subdivide Curves with Fixed Spans", o.fixed_spans).tip(
                "On: each curve span is cut into the same 36 steps the viewport draws. Off: spans are cut by Curve Refinement Angle",
            ),
            Row::number("refinement_angle", "Curve Refinement Angle", Unit::Angle, angle)
                .range(MIN_REFINEMENT_DEGREES, MAX_REFINEMENT_DEGREES)
                .when(When::Unchecked("fixed_spans"))
                .tip("The most a frozen curve turns from one segment to the next; smaller gives more segments"),
            Row::check("save_morph", "Save Deformation to Morph", o.save_morph)
                .when(When::Never)
                .tip(na),
            Row::text("morph_name", "Morph Map Name", &o.morph_name)
                .when(When::Never)
                .tip(na),
        ]
        .into_iter()
        .map(form::Row::fresh)
        .collect()
    }
    /// The options the rows show.
    pub(super) fn options(&self) -> FreezeOptions {
        let v = &self.form.values;
        FreezeOptions {
            curves_as: Self::CURVES[v.choice("curves_as").min(2)],
            make_holes: v.bool("make_holes"),
            drill_axis: v.choice("drill_axis").min(2) as u8,
            displacement: v.bool("displacement"),
            deformation: v.bool("deformation"),
            tessellation: v.bool("tessellation"),
            fixed_spans: v.bool("fixed_spans"),
            refinement_angle: angle_text(v.number("refinement_angle")),
            save_morph: v.bool("save_morph"),
            morph_name: v.text("morph_name").to_owned(),
        }
    }
    #[cfg(test)]
    pub(super) fn set_tessellation(&mut self, on: bool) {
        self.form.values.set("tessellation", form::Value::Bool(on));
    }
    #[cfg(test)]
    pub(super) fn form(&mut self) -> &mut form::Dialog {
        &mut self.form
    }
}

impl SavedSettings {
    pub(super) fn load(path: &std::path::Path) -> Self {
        std::fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= 8192)
            .and_then(|_| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(|s| s.subdivide.validate().is_ok())
            .unwrap_or_default()
    }
    fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }
}

/// The Subdivide Options dialog (SUB-30), in the shared form.
pub(super) struct SubdivideDialog {
    form: form::Dialog,
}
impl SubdivideDialog {
    /// SUB-30 rows. The remembered Subdivide settings are the memory
    /// (SUB-05, UIC-95), so the rows start from them.
    fn rows(o: &SubdivideOptions) -> Vec<form::Row> {
        use form::{Row, Unit, When};
        let methods: Vec<&str> = SubdivideMethod::ALL.iter().map(|m| m.label()).collect();
        let method = SubdivideMethod::ALL
            .iter()
            .position(|m| *m == o.method)
            .unwrap_or(0);
        let catmull = SubdivideMethod::ALL
            .iter()
            .position(|m| *m == SubdivideMethod::CatmullClark)
            .unwrap_or(0);
        let boundaries: Vec<&str> = SubdivideBoundary::ALL.iter().map(|b| b.label()).collect();
        let boundary = SubdivideBoundary::ALL
            .iter()
            .position(|b| *b == o.boundary)
            .unwrap_or(0);
        vec![
            Row::choice("method", "Subdivision Method", &methods, method).tip(
                "Faceted splits; Smooth rounds new points; SDS Subdivide and Catmull-Clark smooth everything",
            ),
            Row::number("fractal", "Fractal", Unit::Percent, o.fractal_percent)
                .tip("Random offset added to the new points, as a percentage of each polygon's size"),
            Row::number("angle", "Max Smoothing Angle", Unit::Angle, o.max_angle_degrees)
                .tip("Smooth and SDS Subdivide keep edges sharper than this"),
            Row::check("linear_uvs", "Linear UVs", o.linear_uvs)
                .tip("Split the UVs straight instead of smoothing them with the surface"),
            // Catmull-Clark only (SUB-30): greyed for the other methods.
            Row::choice("boundary", "Boundary Rules", &boundaries, boundary)
                .when(When::ChoiceIn("method", vec![catmull]))
                .tip("How open borders are smoothed (Catmull-Clark only)"),
            Row::check(
                "background_constraint",
                "Background Constraint",
                o.background_constraint,
            )
            .when(When::Never)
            .tip(NO_BACKGROUND),
            Row::check(
                "ignore_edge_weights",
                "Ignore Subdivision Edge Weights",
                o.ignore_edge_weights,
            )
            .when(When::ChoiceIn("method", vec![catmull]))
            .tip("Smooth across weighted edges as if they had no weight (Catmull-Clark only)"),
        ]
        .into_iter()
        .map(form::Row::fresh)
        .collect()
    }
    /// The options the rows show.
    pub(super) fn options(&self) -> SubdivideOptions {
        let v = &self.form.values;
        SubdivideOptions {
            method: SubdivideMethod::ALL[v.choice("method").min(SubdivideMethod::ALL.len() - 1)],
            fractal_percent: v.number("fractal"),
            max_angle_degrees: v.number("angle"),
            linear_uvs: v.bool("linear_uvs"),
            boundary: SubdivideBoundary::ALL
                [v.choice("boundary").min(SubdivideBoundary::ALL.len() - 1)],
            background_constraint: v.bool("background_constraint"),
            ignore_edge_weights: v.bool("ignore_edge_weights"),
        }
    }
    #[cfg(test)]
    pub(super) fn set_method(&mut self, method: SubdivideMethod) {
        let i = SubdivideMethod::ALL
            .iter()
            .position(|m| *m == method)
            .unwrap_or(0);
        self.form.values.set("method", form::Value::Choice(i));
    }
}

/// The UnSubdivide dialog (SUB-40 corrected: a click opens it, OK runs).
pub(super) struct UnsubdivideDialog {
    form: form::Dialog,
}
impl UnsubdivideDialog {
    /// SUB-40 (corrected) rows, top to bottom. Hints only break ties
    /// between possible groupings of four quads; with all off a clean mesh
    /// still comes back.
    fn rows(o: &UnsubdivideOptions, sets: Vec<String>) -> Vec<form::Row> {
        use form::{Row, Unit, When};
        let max = f64::from(operandi_core::polygon_subdivide::MAX_UNSUBDIVIDE_ITERATIONS);
        vec![
            // A whole number, with its arrows like every number (UIC-90).
            Row::number(
                "iterations",
                "Iteration",
                Unit::Count,
                f64::from(o.iterations),
            )
            .range(0., max)
            .tip("How many Catmull-Clark steps to undo (0 does nothing)"),
            Row::check("flat", "Flat", o.flat).tip("Merge the quads without moving any point back"),
            Row::check("uv_hint", "UV Corner Hint", o.uv_hint)
                .tip("Prefer groups whose UVs are continuous inside"),
            Row::check("material_hint", "Material Corner Hint", o.material_hint)
                .tip("Prefer groups that share one material"),
            Row::check("part_hint", "Part Corner Hint", o.part_hint)
                .tip("Prefer groups that share one part"),
            Row::check(
                "smoothing_group_hint",
                "Smoothing Group Corner Hint",
                o.smoothing_group_hint,
            )
            .tip("Prefer groups that share one smoothing group"),
            Row::check(
                "selection_set_hint",
                "Selection Set Corner Hint",
                o.selection_set_hint,
            )
            .tip("Prefer groups all inside the named polygon set, or all outside"),
            Row::text(
                "selection_set_name",
                "Selection Set Name",
                &o.selection_set_name,
            )
            .suggestions(sets)
            .when(When::Checked("selection_set_hint"))
            .tip("A polygon selection set of the active mesh: type it or pick it"),
        ]
        .into_iter()
        .map(form::Row::fresh)
        .collect()
    }
    /// The options the rows show.
    pub(super) fn options(&self) -> UnsubdivideOptions {
        let v = &self.form.values;
        UnsubdivideOptions {
            iterations: v.number("iterations").round().max(0.) as u32,
            flat: v.bool("flat"),
            uv_hint: v.bool("uv_hint"),
            material_hint: v.bool("material_hint"),
            part_hint: v.bool("part_hint"),
            smoothing_group_hint: v.bool("smoothing_group_hint"),
            selection_set_hint: v.bool("selection_set_hint"),
            selection_set_name: v.text("selection_set_name").to_owned(),
        }
    }
    #[cfg(test)]
    pub(super) fn set_iterations(&mut self, n: u32) {
        self.form
            .values
            .set("iterations", form::Value::Number(f64::from(n)));
    }
}

#[derive(Default)]
pub(super) struct SubdivideUi {
    pub(super) options: Option<SubdivideDialog>,
    pub(super) unsubdivide: Option<UnsubdivideDialog>,
    /// The Freeze... dialog (SUB-50).
    pub(super) freeze: Option<FreezeDialog>,
    /// Freeze's remembered options.
    pub(super) freeze_options: FreezeOptions,
    /// The open dialog's row rects, for layout tests.
    pub(super) last_rows: Vec<form::RowRects>,
    /// What was last written to the data folder.
    saved: Option<SavedSettings>,
    /// A Subdivide waiting for the big-result question.
    pub(super) large: Option<LargeSubdivide>,
    /// Free memory as the question sees it; `None` reads the system's
    /// (tests set a figure).
    pub(super) free_memory: Option<fn() -> u64>,
}

/// A Subdivide that would take over half of the free memory, asked first.
pub(super) struct LargeSubdivide {
    options: SubdivideOptions,
    times: u32,
    pub(super) estimate: operandi_core::polygon_subdivide::SubdivideEstimate,
    free: u64,
}

impl SubdivideUi {
    pub(super) fn open(&self) -> bool {
        self.options.is_some()
            || self.unsubdivide.is_some()
            || self.freeze.is_some()
            || self.large.is_some()
    }
}

impl Operandi {
    /// Restore the saved settings (at launch).
    pub(super) fn load_subdivide_settings(&mut self) {
        let saved = SavedSettings::load(&self.environment.paths().subdivide_settings());
        self.editor.subdivide_preset = saved.subdivide.clone();
        self.editor.unsubdivide_preset = saved.unsubdivide.clone();
        self.subdivide_ui.freeze_options = saved.freeze.clone();
        self.editor.detriangulate_preset = saved.detriangulate.clone();
        self.subdivide_ui.saved = Some(saved);
    }

    /// Write the settings when a run changed them (once per change).
    pub(super) fn save_subdivide_settings(&mut self) {
        let Some(saved) = &self.subdivide_ui.saved else {
            return;
        };
        let now = SavedSettings {
            subdivide: self.editor.subdivide_preset.clone(),
            unsubdivide: self.editor.unsubdivide_preset.clone(),
            freeze: self.subdivide_ui.freeze_options.clone(),
            detriangulate: self.editor.detriangulate_preset.clone(),
        };
        if *saved != now {
            if let Err(e) = now.save(&self.environment.paths().subdivide_settings()) {
                self.status = format!("Couldn't save the Subdivide settings: {e}");
            }
            self.subdivide_ui.saved = Some(now);
        }
    }

    /// A Subdivide button or shortcut: `method`, the other options as last
    /// used (SUB-05), `times` passes as one undo step.
    pub(super) fn subdivide_with(&mut self, method: SubdivideMethod, times: u32) {
        let options = SubdivideOptions {
            method,
            ..self.editor.subdivide_preset.clone()
        };
        self.run_subdivide(options, times);
    }

    /// Run Subdivide, first asking when the result would take more than
    /// half of the free memory (Jason, 2026-11-12). A result bigger than
    /// all of it is refused by the editor's own check, with the numbers.
    pub(super) fn run_subdivide(&mut self, options: SubdivideOptions, times: u32) {
        let free = self
            .subdivide_ui
            .free_memory
            .unwrap_or(crate::free_memory::free_bytes)();
        if let Ok(estimate) = self.editor.subdivide_estimate(&options, times)
            && estimate.bytes > free / 2
            && estimate.bytes <= free
        {
            self.subdivide_ui.large = Some(LargeSubdivide {
                options,
                times,
                estimate,
                free,
            });
            return;
        }
        self.subdivide_now(options, times);
    }

    fn subdivide_now(&mut self, options: SubdivideOptions, times: u32) {
        let method = options.method;
        if self.run_ok(Command::SubdividePolygons { options, times }) {
            let faces = self.active_face_count();
            self.status = format!("Subdivided ({}): {faces} polygons", method.label());
        }
    }

    /// The question for a Subdivide that needs over half of the free
    /// memory. UIC-97: OK subdivides; Cancel (Escape) changes nothing
    /// (UIC-96). UIC-81: the title is the command's name.
    fn large_subdivide_question(&mut self, ctx: &Context) {
        let Some(large) = self.subdivide_ui.large.take() else {
            return;
        };
        use operandi_core::polygon_subdivide::{gigabytes, group_thousands};
        let message = format!(
            "This makes {} polygons and needs about {} of memory ({} is free now). Continue?",
            group_thousands(large.estimate.polygons),
            gigabytes(large.estimate.bytes),
            gigabytes(large.free)
        );
        match self.show_confirm(ctx, "Subdivide Polygons", &message, &[]) {
            form::Outcome::Ok => self.subdivide_now(large.options, large.times),
            form::Outcome::Cancel => self.status = "Subdivide canceled; nothing changed".into(),
            _ => self.subdivide_ui.large = Some(large),
        }
    }

    fn active_face_count(&self) -> usize {
        self.editor.scene.active().map_or(0, |i| i.mesh.faces.len())
    }

    pub(super) fn open_subdivide_options(&mut self) {
        if self.subdivide_ui.options.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog before Subdivide".into();
            return;
        }
        let rows = SubdivideDialog::rows(&self.editor.subdivide_preset);
        self.subdivide_ui.options = Some(SubdivideDialog {
            form: self.form_dialog("Subdivide", rows),
        });
    }

    /// UnSubdivide with its remembered options (the dialog's OK, scripts).
    pub(super) fn run_unsubdivide(&mut self) {
        let options = self.editor.unsubdivide_preset.clone();
        if self.run_ok(Command::UnsubdividePolygons(options.clone())) {
            self.status = if options.iterations == 0 {
                "UnSubdivide: Iterations is 0; nothing changed".into()
            } else {
                format!("UnSubdivide: {} polygons", self.active_face_count())
            };
        }
    }

    pub(super) fn open_unsubdivide_options(&mut self) {
        if self.subdivide_ui.unsubdivide.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog before UnSubdivide".into();
            return;
        }
        let sets: Vec<String> = self
            .editor
            .scene
            .active()
            .map(|i| i.mesh.attributes.polygon_sets.keys().cloned().collect())
            .unwrap_or_default();
        let rows = UnsubdivideDialog::rows(&self.editor.unsubdivide_preset, sets);
        self.subdivide_ui.unsubdivide = Some(UnsubdivideDialog {
            form: self.form_dialog("UnSubdivide", rows),
        });
    }

    pub(super) fn open_freeze(&mut self) {
        if self.subdivide_ui.freeze.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog before Freeze".into();
            return;
        }
        let rows = FreezeDialog::rows(&self.subdivide_ui.freeze_options);
        self.subdivide_ui.freeze = Some(FreezeDialog {
            form: self.form_dialog("Freeze", rows),
        });
    }

    /// CEN-01..04.
    pub(super) fn center(&mut self, axes: [bool; 3]) {
        if self.run_ok(Command::CenterSelection(axes)) {
            self.status = format!("Centered on {}", center_axes_word(axes).to_uppercase());
        }
    }

    /// SUB-51: Edge Slide in Edge mode, Vertex Slide in Vertex mode.
    pub(super) fn slide_from_tab(&mut self) {
        match self.editor.scene.selection.mode {
            Mode::Edge => self.activate(Tool::EdgeSlide),
            Mode::Vertex => self.activate(Tool::VertexSlide),
            _ => {
                self.status =
                    "Slide works in Vertex or Edge mode (Polygon Slide isn't built yet)".into()
            }
        }
    }

    /// The dialogs, once per frame.
    pub(super) fn subdivide_dialogs(&mut self, ctx: &Context) {
        self.subdivide_options_dialog(ctx);
        self.unsubdivide_dialog(ctx);
        self.freeze_dialog(ctx);
        self.large_subdivide_question(ctx);
    }

    fn subdivide_options_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.subdivide_ui.options.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "Subdivide closed; nothing changed".into();
            return;
        }
        let outcome = self.show_form(ctx, &mut dialog.form);
        self.subdivide_ui.last_rows = dialog.form.row_rects().to_vec();
        match outcome {
            form::Outcome::Cancel => self.status = "Subdivide canceled; nothing changed".into(),
            form::Outcome::Ok => {
                let options = dialog.options();
                match options.validate() {
                    Ok(()) => {
                        self.run_subdivide(options, 1);
                    }
                    Err(e) => {
                        self.status = e.clone();
                        dialog.form.error = Some(e);
                        self.subdivide_ui.options = Some(dialog);
                    }
                }
            }
            _ => self.subdivide_ui.options = Some(dialog),
        }
    }

    fn unsubdivide_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.subdivide_ui.unsubdivide.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "UnSubdivide closed; nothing changed".into();
            return;
        }
        let outcome = self.show_form(ctx, &mut dialog.form);
        self.subdivide_ui.last_rows = dialog.form.row_rects().to_vec();
        match outcome {
            form::Outcome::Cancel => {
                self.status = "UnSubdivide canceled; nothing changed".into();
            }
            form::Outcome::Ok => {
                // The Iteration field keeps itself in 0..=max.
                self.editor.unsubdivide_preset = dialog.options();
                self.run_unsubdivide();
            }
            _ => self.subdivide_ui.unsubdivide = Some(dialog),
        }
    }

    fn freeze_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.subdivide_ui.freeze.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "Freeze closed; nothing changed".into();
            return;
        }
        let outcome = self.show_form(ctx, &mut dialog.form);
        self.subdivide_ui.last_rows = dialog.form.row_rects().to_vec();
        match outcome {
            form::Outcome::Cancel => self.status = "Freeze canceled; nothing changed".into(),
            form::Outcome::Ok => {
                self.subdivide_ui.freeze_options = dialog.options();
                self.run_freeze();
            }
            _ => self.subdivide_ui.freeze = Some(dialog),
        }
    }

    /// Freeze with the remembered options (SUB-50, FRZ-03): curves and,
    /// with Tessellation on, subdivision surfaces, one undo step.
    pub(super) fn run_freeze(&mut self) {
        let options = self.subdivide_ui.freeze_options.core();
        if self.run_ok(Command::Freeze(options)) {
            self.status = format!("Frozen: {} polygons", self.active_face_count());
        }
    }

    /// `poly.subdivide`, `poly.unsubdiv`, `vert.center`, `poly.freeze` and
    /// their help topics. Names from the reference name map; percent
    /// arguments are fractions and angles radians (`command_names`).
    pub(super) fn subdivide_canonical(&mut self, text: &str) -> bool {
        let mut words = text.split_whitespace();
        let first = words.next().unwrap_or("").to_ascii_lowercase();
        if first == "help" {
            let topic = words.next().unwrap_or("").to_ascii_lowercase();
            let help = match topic.as_str() {
                "poly.subdivide" => {
                    "Help: poly.subdivide opens the Subdivide options. poly.subdivide <type> [fractal:<0-1>] [maxangle:<radians>] [linear:<bool>] [bound:smooth|crease|creaseEdges] [noEdgeWeight:<bool>] subdivides the selected polygons (all when none) once. type: ccsds (Catmull-Clark, the SDS Subdivide button), flat (Faceted), smooth, sds (SDS Subdivide). Options not given keep their last values. Shortcuts: D (outside Edge mode); Shift+D opens this dialog in Vertex, Edge and Polygon mode."
                }
                "poly.unsubdiv" => {
                    "Help: poly.unsubdiv opens the UnSubdivide dialog. poly.unsubdiv [iteration:<n>] [flat:<bool>] [uv|material|part|smoothingGroup|selectionSet:<bool>] [selectionSetName:<name>] merges groups of four quads back into one, undoing Catmull-Clark steps. Options not given keep their last values."
                }
                "vert.center" => {
                    "Help: vert.center <x|y|z|xy|yz|zx|all> moves the selected vertices, edges or polygons (the whole mesh when nothing is selected) so their world bounding-box centre is 0 on those axes."
                }
                "poly.freeze" => {
                    "Help: poly.freeze opens the Freeze dialog, which turns curves and subdivision-surface polygons into plain geometry. poly.freeze [face:<0|1|2>] [tess:<bool>] [fixed:<bool>] [cangle:<radians, or 5deg>] freezes without the dialog; face is 0 Faces, 1 Polylines or 2 2 Point Polygons (2 is an Operandi addition), or the names faces, polylines, twoPointPolygons. drill, axis, disp, deform, morphSave and morphName are accepted and ignored (not built yet). Options not given keep their last values. The freeze command runs Freeze with the last values."
                }
                _ => return false,
            };
            self.status = help.into();
            return true;
        }
        let args: Vec<&str> = words.collect();
        match first.as_str() {
            "poly.subdivide" => {
                if args.is_empty() {
                    self.open_subdivide_options();
                    return true;
                }
                match parse_subdivide(&self.editor.subdivide_preset, &args) {
                    Ok(options) => {
                        self.run_subdivide(options, 1);
                    }
                    Err(e) => self.status = e,
                }
            }
            // Bare: the dialog, like the button (SUB-40 corrected).
            "poly.unsubdiv" if args.is_empty() => self.open_unsubdivide_options(),
            "poly.unsubdiv" => match parse_unsubdivide(&self.editor.unsubdivide_preset, &args) {
                Ok(options) => {
                    self.editor.unsubdivide_preset = options;
                    self.run_unsubdivide();
                }
                Err(e) => self.status = e,
            },
            "vert.center" => {
                let word = match args.as_slice() {
                    [w] => w
                        .split_once(':')
                        .filter(|(k, _)| k.eq_ignore_ascii_case("axis"))
                        .map_or(*w, |(_, v)| v),
                    _ => "",
                };
                match center_axes(word) {
                    Some(axes) => self.center(axes),
                    None => self.status = "Use vert.center <x|y|z|xy|yz|zx|all>".into(),
                }
            }
            "poly.freeze" if args.is_empty() => self.open_freeze(),
            "poly.freeze" => match parse_freeze(&self.subdivide_ui.freeze_options, &args) {
                Ok((options, inert)) => {
                    self.subdivide_ui.freeze_options = options;
                    self.run_freeze();
                    if !inert.is_empty() {
                        self.status = format!(
                            "{} ({} accepted and ignored: not built yet)",
                            self.status,
                            inert.join(", ")
                        );
                    }
                }
                Err(e) => self.status = e,
            },
            _ => return false,
        }
        true
    }
}
