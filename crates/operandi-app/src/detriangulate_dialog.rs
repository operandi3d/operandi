//! Detriangulate... (BEHAVIOR-DETRIANGULATE.md, DT-10): a command with an
//! options dialog built with the shared dialog (UIC-80..99). OK runs
//! `Command::Detriangulate` once (one undo step, UIC-96); the values are
//! remembered for next time, across restarts (UIC-95), with the Subdivide
//! settings.
use super::*;
use operandi_core::DetriangulateOptions;

/// The open Detriangulate dialog.
pub(super) struct DetriangulateDialog {
    pub(super) form: form::Dialog,
    /// The edges OK would remove now, (low, high) point of the active
    /// mesh, drawn in the viewport while the dialog is open.
    pub(super) preview: Vec<(u32, u32)>,
    /// What `preview` was worked out for: scene revision, active item and
    /// the dialog's options. It is redone only when one of them changes.
    preview_key: Option<(u64, Option<u64>, DetriangulateOptions)>,
}

impl DetriangulateDialog {
    /// DT section 3 rows, top to bottom.
    fn rows(o: &DetriangulateOptions) -> Vec<form::Row> {
        use form::{Row, Unit};
        vec![
            Row::number("flatness", "Flatness", Unit::Percent, o.flatness)
                .range(0., 1000.)
                .tip("How far the pair may fold: the fourth corner's height off the first triangle, as a share of the shared edge (0 % joins only flat pairs)"),
            Row::number("max_angle", "Max Angle", Unit::Angle, o.max_angle)
                .range(1., 180.)
                .tip("Every corner of a new quad must be below this, so near-triangles stay apart"),
            Row::check(
                "keep_material",
                "Keep Material Boundary",
                o.keep_material,
            )
            .tip("Don't join triangles with different materials"),
            Row::check("keep_part", "Keep Part Boundary", o.keep_part)
                .tip("Don't join triangles in different parts"),
            Row::check(
                "keep_smoothing",
                "Keep Smoothing Group Boundary",
                o.keep_smoothing,
            )
            .tip("Don't join triangles with different smoothing"),
            Row::check(
                "keep_sets",
                "Keep Selection Set Boundary",
                o.keep_sets,
            )
            .tip("Don't join a triangle in a polygon selection set with one outside it"),
            Row::check("keep_uv_seams", "Keep UV Boundary", o.keep_uv_seams)
                .tip("Don't remove an edge that is a UV seam"),
        ]
        .into_iter()
        .map(form::Row::fresh)
        .collect()
    }

    pub(super) fn options(&self) -> DetriangulateOptions {
        let v = &self.form.values;
        DetriangulateOptions {
            flatness: v.number("flatness"),
            max_angle: v.number("max_angle"),
            keep_material: v.bool("keep_material"),
            keep_part: v.bool("keep_part"),
            keep_smoothing: v.bool("keep_smoothing"),
            keep_sets: v.bool("keep_sets"),
            keep_uv_seams: v.bool("keep_uv_seams"),
        }
    }
}

impl Operandi {
    /// Detriangulate... opens its dialog (the toolbox row, the menu entry,
    /// the shortcut and the command line).
    pub(super) fn open_detriangulate(&mut self) {
        if self.detriangulate.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog before Detriangulate".into();
            return;
        }
        let rows = DetriangulateDialog::rows(&self.editor.detriangulate_preset);
        let mut dialog = DetriangulateDialog {
            form: self.form_dialog("Detriangulate", rows),
            preview: Vec::new(),
            preview_key: None,
        };
        self.update_detriangulate_preview(&mut dialog);
        self.detriangulate = Some(dialog);
    }

    /// Detriangulate with the remembered options.
    pub(super) fn run_detriangulate(&mut self) {
        let options = self.editor.detriangulate_preset.clone();
        let before = self.editor.scene.active().map_or(0, |i| i.mesh.faces.len());
        if self.run_ok(Command::Detriangulate(options)) {
            let after = self.editor.scene.active().map_or(0, |i| i.mesh.faces.len());
            self.status = format!("Detriangulate: {} triangle pairs joined", before - after);
        }
    }

    /// The dialog, once per frame.
    pub(super) fn detriangulate_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.detriangulate.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "Detriangulate closed; nothing changed".into();
            return;
        }
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => {
                self.status = "Detriangulate canceled; nothing changed".into();
            }
            form::Outcome::Ok => {
                let options = dialog.options();
                match options.validate() {
                    Ok(()) => {
                        self.editor.detriangulate_preset = options;
                        self.run_detriangulate();
                    }
                    Err(e) => {
                        self.status = e.clone();
                        dialog.form.error = Some(e);
                        self.detriangulate = Some(dialog);
                    }
                }
            }
            _ => {
                self.update_detriangulate_preview(&mut dialog);
                self.detriangulate = Some(dialog);
            }
        }
    }

    /// Works out the edges OK would remove with the dialog's current
    /// options, by the same pairing as the edit itself
    /// (`Editor::detriangulate_preview`). Options that can't run, or
    /// nothing to join, preview nothing.
    fn update_detriangulate_preview(&self, dialog: &mut DetriangulateDialog) {
        let options = dialog.options();
        let key = (
            self.editor.revision,
            self.editor.scene.active().map(|i| i.id),
            options.clone(),
        );
        if dialog.preview_key.as_ref() == Some(&key) {
            return;
        }
        dialog.preview = self
            .editor
            .detriangulate_preview(&options)
            .unwrap_or_default();
        dialog.preview_key = Some(key);
    }

    /// While the dialog is open, the edges it is about to remove, over the
    /// active mesh in the view.
    pub(super) fn draw_detriangulate_preview(&self, painter: &egui::Painter, r: Rect) {
        let Some(dialog) = &self.detriangulate else {
            return;
        };
        let Some(item) = self.editor.scene.active() else {
            return;
        };
        let Ok(matrix) = self.editor.scene.world_matrix(item.id) else {
            return;
        };
        let projector = self.camera.projector(r);
        let points = &item.mesh.vertices;
        let stroke = Stroke::new(2.5f32, theme::error());
        for &(a, b) in &dialog.preview {
            let (Some(&pa), Some(&pb)) = (points.get(a as usize), points.get(b as usize)) else {
                continue;
            };
            let ends = [pa, pb].map(|p| projector.project(matrix.transform_point3(p)));
            if let [Some((qa, _)), Some((qb, _))] = ends {
                painter.line_segment([qa, qb], stroke);
            }
        }
    }
}
