//! Painting stage 7 in the app (PNT-151, 241..247, 260): the painting
//! parts on sculpt brushes and the sculpt mask. Painting builds only;
//! default builds' sculpting is untouched.
//!
//! - Sculpt tab: Procedural, Sphere, Texture, Stamp and Text tips
//!   (`operandi.paint.sculptTip`, values `operandi.paint.sculptTipSet`),
//!   Save Brush Preset, Nozzle and Inks. Sculpt keeps its own tip values
//!   (PNT-42/Q10); the nozzle and the ink stack are shared with painting.
//! - Each sculpt stroke carries a core `SculptPaint` (paint_sculpt.rs):
//!   the tip, the inks (Projection Ink is paint only) and the mask.
//! - Mask brushes (`operandi.paint.maskBrush kind:…`) run the Vertex Map
//!   Tools' machinery on a hidden weight map the meshes borrow for the
//!   stroke; the release turns it into one `PaintEdit::Mask` undo step and
//!   gives the meshes back unchanged. The mask shows red over the surface.
use super::*;
use crate::painting::word;
use operandi_core::SculptBrush;
use operandi_core::command_registry::{self as table, Area, ArgType, Unit};
use operandi_core::command_runtime::{Call, CommandState, HostHandler};
use operandi_core::paint::PaintEdit;
use operandi_core::paint_inks::InkKind;
use operandi_core::paint_sculpt::{MASK_MAP, SculptPaint};
use operandi_core::paint_tips::{Brush, TipChoice};
use operandi_core::sculpt::Dab;
use std::sync::Arc;

/// The mask's settings (PNT-260): not saved with the scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MaskSettings {
    pub(crate) enable: bool,
    pub(crate) invert: bool,
    /// The red overlay's strength (display only).
    pub(crate) opacity: f64,
}

impl Default for MaskSettings {
    fn default() -> Self {
        MaskSettings {
            enable: true,
            invert: false,
            opacity: 0.6,
        }
    }
}

/// The five mask brushes (PNT-260) and the Vertex Map Tool each runs on.
pub(crate) const MASK_BRUSHES: [(&str, &str, SculptBrush); 5] = [
    ("paint", "Mask Paint", SculptBrush::VmPaint),
    ("smooth", "Mask Smooth", SculptBrush::VmSmooth),
    ("scaleDown", "Mask Scale Down", SculptBrush::VmScaleDown),
    ("erase", "Mask Erase", SculptBrush::VmErase),
    ("setValue", "Mask Set Value", SculptBrush::VmSetValue),
];

/// Projection Ink is paint only (PNT-243).
pub(crate) const PAINT_ONLY_INK: &str = "Projection Ink works with the paint tools only";

type State = fn(&Operandi, &Call<'_>) -> CommandState;

const fn cmd(name: &'static str, help: &'static str) -> table::Entry {
    table::script(name, Area::Tools, help).operandi()
}

const TIP_ARGS: &[table::Arg] = &[table::arg(
    "kind",
    ArgType::Choice(&[
        "smooth",
        "hard",
        "procedural",
        "sphere",
        "image",
        "stamp",
        "text",
    ]),
    Unit::None,
)];
const MASK_ARGS: &[table::Arg] = &[table::arg(
    "kind",
    ArgType::Choice(&["paint", "smooth", "scaleDown", "erase", "setValue"]),
    Unit::None,
)];
const GEOMETRY_ARGS: &[table::Arg] = &[
    table::opt(
        "mode",
        ArgType::Choice(&["height", "vector"]),
        Unit::None,
        Some("height"),
    ),
    table::opt("resolution", ArgType::Int, Unit::None, Some("256")),
    table::opt(
        "axis",
        ArgType::Choice(&["auto", "x", "y", "z"]),
        Unit::None,
        Some("auto"),
    ),
    table::opt("iterations", ArgType::Int, Unit::None, Some("1000")),
];
const MASK_SET_ARGS: &[table::Arg] = &[
    table::opt("enable", ArgType::Bool, Unit::None, None),
    table::opt("invert", ArgType::Bool, Unit::None, None),
    table::opt("opacity", ArgType::Float, Unit::Percent, None),
];

pub(crate) fn commands() -> Vec<(table::Entry, HostHandler<Operandi>, State)> {
    vec![
        (
            cmd("operandi.paint.sculptTip", "The sculpt brushes' tip (PNT-151, 246, 247): Smooth or Hard (the brush's own), Procedural, Sphere, Texture (image), Stamp or Text. Sculpt keeps its own tip values.")
                .args(TIP_ARGS)
                .example("operandi.paint.sculptTip kind:image"),
            |a, c| match word(c.rest, "kind", 0).and_then(|k| TipChoice::named(&k)) {
                Some(choice) => {
                    a.painting.brush.sculpt_tips.choice = choice;
                    if let Some(kind) = basic_kind(choice) {
                        a.set_sculpt_tip(kind);
                    }
                    a.status = format!("Sculpt tip: {}", choice.label());
                }
                None => a.status = "Usage: operandi.paint.sculptTip kind:<smooth|hard|procedural|sphere|image|stamp|text>".into(),
            },
            |a, c| {
                let want = word(c.rest, "kind", 0).and_then(|k| TipChoice::named(&k));
                CommandState::lit(want == Some(a.painting.brush.sculpt_tips.choice))
            },
        ),
        (
            cmd("operandi.paint.sculptTipSet", "The sculpt tip's own values (as operandi.paint.tipSet, for sculpt brushes); the nozzle values are shared with painting (PNT-241).")
                .args(crate::painting_brush::TIPSET_ARGS)
                .example("operandi.paint.sculptTipSet offset:0.5 useAsMask:true"),
            crate::painting_brush::run_tip_set,
            |_, _| CommandState::default(),
        ),
        (
            cmd("operandi.paint.maskBrush", "A sculpt mask brush (PNT-260): paint (Ctrl erases), smooth, scaleDown (Ctrl strengthens), erase, setValue. Masked points move less; fully masked ones don't move.")
                .args(MASK_ARGS)
                .example("operandi.paint.maskBrush kind:paint"),
            |a, c| {
                let kind = word(c.rest, "kind", 0).unwrap_or_default();
                let Some(&(_, label, vm)) = MASK_BRUSHES.iter().find(|(k, _, _)| k.eq_ignore_ascii_case(&kind)) else {
                    a.status = "Usage: operandi.paint.maskBrush kind:<paint|smooth|scaleDown|erase|setValue>".into();
                    return;
                };
                let on = a.painting.brush.mask_brush == Some(vm) && a.sculpt.brush == Some(vm);
                if on {
                    a.painting.brush.mask_brush = None;
                    a.drop_sculpt();
                    return;
                }
                a.drop_paint();
                if a.sculpt.brush != Some(vm) {
                    a.activate_sculpt(vm);
                }
                if a.sculpt.brush == Some(vm) {
                    a.painting.brush.mask_brush = Some(vm);
                    a.status = format!("{label}: left-drag on the mesh (Ctrl inverts)");
                }
            },
            |a, c| {
                let kind = word(c.rest, "kind", 0).unwrap_or_default();
                let on = MASK_BRUSHES
                    .iter()
                    .find(|(k, _, _)| k.eq_ignore_ascii_case(&kind))
                    .is_some_and(|&(_, _, vm)| a.mask_brush_active() == Some(vm));
                CommandState::lit(on)
            },
        ),
        (
            cmd("operandi.paint.maskSet", "The sculpt mask's settings (PNT-260): enable (on), invert (flips the effect, not the red), opacity (the red's strength, display only).")
                .args(MASK_SET_ARGS)
                .example("operandi.paint.maskSet invert:true"),
            |a, c| {
                let mut m = a.painting.brush.mask;
                let r = (|| -> std::result::Result<(), String> {
                    if let Some(v) = word(c.rest, "enable", usize::MAX) {
                        m.enable = operandi_core::command_args::boolean("enable", &v)?;
                    }
                    if let Some(v) = word(c.rest, "invert", usize::MAX) {
                        m.invert = operandi_core::command_args::boolean("invert", &v)?;
                    }
                    if let Some(v) = word(c.rest, "opacity", usize::MAX) {
                        m.opacity = v.trim().parse::<f64>().map_err(|_| "opacity is a number")?.clamp(0., 1.);
                    }
                    Ok(())
                })();
                match r {
                    Ok(()) => a.painting.brush.mask = m,
                    Err(e) => a.status = e,
                }
            },
            |_, _| CommandState::default(),
        ),
        (
            cmd("operandi.paint.geometryToBrush", "Geometry to Brush (PNT-261): the selected open patch (all of the active mesh with nothing selected) becomes the Texture tip's picture. mode height (greyscale) or vector (colour, keeps undercuts), resolution, axis (auto: the patch's facing), iterations. Bare opens the dialog.")
                .args(GEOMETRY_ARGS)
                .side_effect()
                .example("operandi.paint.geometryToBrush mode:height resolution:256"),
            run_geometry_to_brush,
            |a, _| {
                let mesh = a.editor.scene.active().is_some_and(|i| i.is_mesh() && !i.mesh.faces.is_empty());
                CommandState::unless((!mesh).then_some("Select a mesh patch"))
            },
        ),
        (
            cmd("operandi.paint.maskClear", "Clear Mask (PNT-260): removes the active mesh's sculpt mask. One undo step.").undo(),
            |a, _| {
                if let Some(id) = a.editor.scene.selection.item {
                    a.run_mask_edit(id, None, "Clear Mask");
                }
            },
            |a, _| {
                let has = a
                    .editor
                    .scene
                    .selection
                    .item
                    .is_some_and(|id| a.editor.scene.paint.masks.contains_key(&id));
                CommandState::unless((!has).then_some("The active mesh has no mask"))
            },
        ),
        (
            cmd("operandi.paint.maskBakeFalloff", "Bake Falloff (PNT-260): the active falloff's weights become the active mesh's mask. One undo step.").undo(),
            |a, _| a.bake_falloff_to_mask(),
            |a, _| {
                CommandState::unless(a.editor.active_falloff().is_none().then_some("Turn on a falloff first"))
            },
        ),
    ]
}

pub(crate) fn register(r: &mut crate::command_handlers::Commands) {
    for (entry, run, state) in commands() {
        let name = entry.name;
        r.add_host_command(entry, run)
            .unwrap_or_else(|e| panic!("{e}"));
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
}

fn run_geometry_to_brush(a: &mut Operandi, c: &Call<'_>) {
    use operandi_core::paint_geometry_brush::*;
    if c.rest.trim().is_empty() {
        use crate::form::{Row, Unit as U};
        let rows = vec![
            Row::choice("mode", "Mode", &["Height Field", "Vector Displacement"], 0)
                .tip("Height Field: greyscale, height only. Vector Displacement: colour, keeps undercuts"),
            Row::number("resolution", "Brush Resolution", U::Suffix(" px"), 256.).range(16., 4096.),
            Row::choice("axis", "Projection Axis", &["Auto", "X", "Y", "Z"], 0)
                .tip("Auto: the patch's own facing direction"),
            Row::number("iterations", "Iterations", U::Count, 1000.)
                .range(1., 100_000.)
                .tip("More is more accurate and slower"),
        ]
        .into_iter()
        .map(crate::form::Row::fresh)
        .collect();
        a.painting.geometry_brush = Some(a.form_dialog("Geometry to Brush", rows));
        return;
    }
    let mut o = GeometryBrush::default();
    if word(c.rest, "mode", usize::MAX).is_some_and(|m| m.eq_ignore_ascii_case("vector")) {
        o.mode = GeometryBrushMode::VectorDisplacement;
    }
    if let Some(r) =
        word(c.rest, "resolution", usize::MAX).and_then(|r| r.trim().parse::<u32>().ok())
    {
        o.resolution = r;
    }
    if let Some(n) =
        word(c.rest, "iterations", usize::MAX).and_then(|r| r.trim().parse::<u32>().ok())
    {
        o.iterations = n;
    }
    o.axis = match word(c.rest, "axis", usize::MAX)
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "x" => Some(glam::DVec3::X),
        "y" => Some(glam::DVec3::Y),
        "z" => Some(glam::DVec3::Z),
        _ => None,
    };
    let Some(item) = a.editor.scene.active().filter(|i| i.is_mesh()) else {
        a.status = "Select a mesh patch".into();
        return;
    };
    let id = item.id;
    let s = &a.editor.scene.selection;
    let faces = if s.mode == operandi_core::Mode::Polygon && s.item == Some(id) {
        s.faces.clone()
    } else {
        Vec::new()
    };
    let px = match geometry_to_brush(&item.mesh.to_owned_value(), &faces, &o) {
        Ok(px) => px,
        Err(e) => {
            a.paint_notice(&e);
            return;
        }
    };
    if !a.run_ok(Command::Paint(PaintEdit::NewImage(
        operandi_core::paint::NewImage {
            name: Some("Brush from Geometry".into()),
            resolution: px.width,
            format: px.format,
            float: true,
            color: None,
            ..Default::default()
        },
    ))) {
        return;
    }
    let Some(image) = a.editor.paint.last_image else {
        return;
    };
    a.editor.scene.paint.buffers.insert(image, Arc::new(px));
    // The result is the current Texture tip (sculpt's, and the paint
    // tools' when one is on).
    let set = |b: &mut Brush| {
        b.choice = TipChoice::Image;
        b.image.image = Some(image);
        b.image.use_as_mask = true;
        b.image.offset = 0.5;
    };
    set(&mut a.painting.brush.sculpt_tips);
    if a.painting.brush.tool.is_some() {
        set(&mut a.painting.brush.tips);
    }
    a.status = format!("Geometry to Brush: {} tip made", o.mode.label());
}

/// The sculpt brush's own tip kind for Smooth and Hard.
fn basic_kind(choice: TipChoice) -> Option<operandi_core::sculpt::TipKind> {
    match choice {
        TipChoice::Smooth => Some(operandi_core::sculpt::TipKind::Smooth),
        TipChoice::Hard => Some(operandi_core::sculpt::TipKind::Hard),
        _ => None,
    }
}

impl Operandi {
    /// The tips Save/apply Brush Preset and the preset strip act on: the
    /// sculpt brushes' while a sculpt brush is on and no paint tool.
    pub(crate) fn sculpt_context(&self) -> bool {
        self.painting.brush.tool.is_none() && self.sculpt.brush.is_some()
    }

    pub(crate) fn active_tips(&self) -> &Brush {
        if self.sculpt_context() {
            &self.painting.brush.sculpt_tips
        } else {
            &self.painting.brush.tips
        }
    }

    /// A brush preset's tip, values and nozzle onto the active tool's tips
    /// (PNT-244): the tool stays; the nozzle is shared.
    pub(crate) fn apply_brush(&mut self, brush: Brush) {
        let b = &mut self.painting.brush;
        if self.sculpt.brush.is_some() && b.tool.is_none() {
            b.tips.nozzle = brush.nozzle;
            b.tips.nozzle_on = brush.nozzle_on;
            b.sculpt_tips = brush.clone();
            if let Some(kind) = basic_kind(brush.choice) {
                self.set_sculpt_tip(kind);
            }
        } else {
            b.sculpt_tips.nozzle = brush.nozzle;
            b.sculpt_tips.nozzle_on = brush.nozzle_on;
            b.tips = brush;
        }
    }

    /// The mask brush in use, if a mask brush (not its Vertex Map Tool) is
    /// on.
    pub(crate) fn mask_brush_active(&self) -> Option<SculptBrush> {
        self.painting
            .brush
            .mask_brush
            .filter(|&b| self.sculpt.brush == Some(b))
    }

    /// Records mesh `item`'s mask as `values` (one undo step).
    pub(crate) fn run_mask_edit(&mut self, item: u64, values: Option<Arc<Vec<f32>>>, label: &str) {
        if self.run_ok(Command::Paint(PaintEdit::Mask {
            item,
            values,
            label: label.into(),
        })) {
            self.status = label.into();
        }
    }

    fn bake_falloff_to_mask(&mut self) {
        let Some(id) = self.editor.scene.selection.item else {
            self.status = "Select a mesh".into();
            return;
        };
        let Some(f) = self.editor.active_falloff() else {
            self.status = "Turn on a falloff first".into();
            return;
        };
        match f.weights(&self.editor.scene, id) {
            Ok(w) => {
                let values: Vec<f32> = w.iter().map(|v| v.clamp(0., 1.) as f32).collect();
                self.run_mask_edit(id, Some(Arc::new(values)), "Bake Falloff to Mask");
            }
            Err(e) => self.status = e,
        }
    }

    /// The painting parts a sculpt stroke on `item` in view `r` uses.
    pub(crate) fn sculpt_paint(
        &mut self,
        item: u64,
        r: Rect,
    ) -> std::result::Result<SculptPaint, String> {
        let mut p = SculptPaint::new(self.paint_view(r));
        let tips = self.painting.brush.sculpt_tips.clone();
        if basic_kind(tips.choice).is_none() {
            let image = match tips.choice {
                TipChoice::Image | TipChoice::Stamp => tips
                    .image_settings()
                    .image
                    .and_then(|id| self.editor.image_pixels(id).ok()),
                _ => None,
            };
            p.extra = Some(tips.extra(|_| image.clone(), [1.; 3])?);
            p.tip = tips.tip();
            let im = tips.image_settings();
            if matches!(tips.choice, TipChoice::Image | TipChoice::Stamp) && im.use_as_mask {
                // PNT-246: Offset is the zero point.
                p.signed_offset = Some(im.offset);
            }
        }
        // The shared ink stack, Projection Ink left out (paint only).
        let mut run = self.ink_run(operandi_core::paint_stroke::PaintTool::Paintbrush, item, r)?;
        run.inks.0.retain(|k| k.kind() != InkKind::Projection);
        p.inks = run;
        // Constrain to Background on: sculpted points stay on the
        // background meshes (PNT-264).
        if let Some(c) = self.editor.background_constraint {
            let view = self.editor.background_view.0.unwrap_or_default();
            let bg = self
                .editor
                .background_cache
                .get(&self.editor.scene, c.target)?;
            if !bg.is_empty() {
                p.background = Some((bg, c, view));
            }
        }
        let m = self.painting.brush.mask;
        p.masks = self.editor.scene.paint.masks.clone();
        p.mask_enabled = m.enable;
        p.invert_mask = m.invert;
        Ok(p)
    }

    /// A mask stroke borrows the meshes: each gets the hidden mask map
    /// holding its mask, given back on release.
    pub(crate) fn lend_mask_map(&mut self, items: &[u64]) {
        self.painting.brush.mask_lent.clear();
        for &id in items {
            let mask = self.editor.scene.paint.masks.get(&id).cloned();
            let Some(item) = self.editor.scene.item_mut(id) else {
                continue;
            };
            let original = item.mesh.clone();
            let mut mesh: operandi_core::Mesh = item.mesh.to_owned_value();
            let n = mesh.vertices.len();
            let map: std::collections::BTreeMap<u32, f64> = mask
                .iter()
                .flat_map(|m| m.iter().enumerate())
                .filter(|(_, v)| **v > 0.)
                .map(|(i, v)| (i as u32, f64::from(*v)))
                .collect();
            mesh.attributes.point_weights.insert(MASK_MAP.into(), map);
            item.mesh = mesh.into();
            self.painting.brush.mask_lent.push((id, original, n));
        }
    }

    /// The release of a mask stroke: the meshes back as they were, the new
    /// masks as one undo step per mesh. True when this was a mask stroke.
    pub(crate) fn finish_mask_stroke(
        &mut self,
        edit: Option<operandi_core::sculpt::WeightEdit>,
    ) -> bool {
        let lent = std::mem::take(&mut self.painting.brush.mask_lent);
        if lent.is_empty() {
            return false;
        }
        for (id, original, _) in &lent {
            if let Some(item) = self.editor.scene.item_mut(*id) {
                item.mesh = original.clone();
            }
        }
        let Some(edit) = edit else {
            return true;
        };
        let label = MASK_BRUSHES
            .iter()
            .find(|(_, _, vm)| Some(*vm) == self.mask_brush_active())
            .map_or("Mask", |(_, l, _)| l);
        for item_edit in edit.items {
            let Some(&(_, _, n)) = lent.iter().find(|(id, _, _)| *id == item_edit.item) else {
                continue;
            };
            let mut values: Vec<f32> = self
                .editor
                .scene
                .paint
                .masks
                .get(&item_edit.item)
                .map_or_else(|| vec![0.; n], |m| (**m).clone());
            values.resize(n, 0.);
            for (name, changes) in &item_edit.maps {
                if name != MASK_MAP {
                    continue;
                }
                for &(v, w) in changes {
                    if let Some(x) = values.get_mut(v as usize) {
                        *x = w.unwrap_or(0.).clamp(0., 1.) as f32;
                    }
                }
            }
            self.run_mask_edit(item_edit.item, Some(Arc::new(values)), label);
        }
        true
    }

    /// The Tablet Nozzle on a sculpt dab (PNT-241): strength (the dab's
    /// pressure, i.e. Offset Amount per dab), size, and position jitter
    /// across the screen around the dab.
    pub(crate) fn sculpt_nozzle(&self, dab: Dab, seed: u32, n: u32, speed: f64) -> Dab {
        let tips = &self.painting.brush.tips;
        if !tips.nozzle_on {
            return dab;
        }
        let m = tips.nozzle.dab(seed, n, dab.pressure, speed, 0.);
        let (_, right, up, _) = self.camera.basis();
        Dab {
            center: dab.center + (right * m.offset.x - up * m.offset.y) * dab.radius,
            radius: dab.radius * m.size,
            pressure: m.strength,
            ..dab
        }
    }

    /// Ink state answer on the Sculpt tab: Projection Ink is greyed there.
    pub(crate) fn ink_unavailable_for_sculpt(&self, kind: InkKind) -> Option<&'static str> {
        (self.sculpt_context() && kind == InkKind::Projection).then_some(PAINT_ONLY_INK)
    }

    /// Tool Properties' painting rows for a sculpt brush: its tip's values
    /// (when the tip isn't Smooth/Hard), the nozzle, the inks and the mask.
    pub(crate) fn sculpt_painting_properties(&mut self, ui: &mut Ui) {
        let mut lines: Vec<String> = Vec::new();
        if basic_kind(self.painting.brush.sculpt_tips.choice).is_none() {
            let tips = self.painting.brush.sculpt_tips.clone();
            self.tip_rows_for(ui, &mut lines, &tips);
            for l in &mut lines {
                if let Some(rest) = l.strip_prefix("operandi.paint.tipSet") {
                    *l = format!("operandi.paint.sculptTipSet{rest}");
                }
            }
        }
        if self.painting.brush.tips.nozzle_on {
            self.nozzle_rows_for_sculpt(ui, &mut lines);
        }
        if !self.painting.brush.ink.inks.0.is_empty() {
            self.ink_rows(ui, &mut lines);
        }
        ui.separator();
        ui.strong("Mask");
        let m = self.painting.brush.mask;
        crate::option_row::field_row(ui, "Enable Mask", |ui| {
            let mut v = m.enable;
            if ui.checkbox(&mut v, "").changed() {
                lines.push(format!("operandi.paint.maskSet enable:{v}"));
            }
        });
        crate::option_row::field_row(ui, "Invert Mask", |ui| {
            let mut v = m.invert;
            if ui.checkbox(&mut v, "").changed() {
                lines.push(format!("operandi.paint.maskSet invert:{v}"));
            }
        });
        crate::option_row::field_row(ui, "Mask Opacity", |ui| {
            let mut x = m.opacity * 100.;
            if ui
                .add(
                    crate::numeric_tab::DragValue::new(&mut x)
                        .range(0. ..=100.)
                        .suffix(" %"),
                )
                .changed()
            {
                lines.push(format!("operandi.paint.maskSet opacity:{}", x / 100.));
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Clear Mask").clicked() {
                lines.push("operandi.paint.maskClear".into());
            }
            if ui.button("Bake Falloff").clicked() {
                lines.push("operandi.paint.maskBakeFalloff".into());
            }
        });
        for l in lines {
            self.execute(&l);
        }
    }

    /// The mask in red over the active meshes' front-facing polygons: the
    /// stroke's values while a mask stroke runs, else the stored mask.
    pub(crate) fn draw_sculpt_mask(&self, painter: &Painter, r: Rect) {
        let opacity = self.painting.brush.mask.opacity;
        if opacity <= 0. {
            return;
        }
        let view = self.paint_view(r);
        let painter = painter.with_clip_rect(r);
        for item in &self.editor.scene.items {
            let lent_map = item.mesh.attributes.point_weights.get(MASK_MAP);
            let stored = self.editor.scene.paint.masks.get(&item.id);
            if lent_map.is_none() && stored.is_none() {
                continue;
            }
            let value = |v: u32| -> f64 {
                match lent_map {
                    Some(m) => m.get(&v).copied().unwrap_or(0.),
                    None => stored
                        .and_then(|s| s.get(v as usize))
                        .map_or(0., |&x| f64::from(x)),
                }
            };
            let Ok(world) = self.editor.scene.world_matrix(item.id) else {
                continue;
            };
            let mesh = &item.mesh;
            let mut shapes = Vec::new();
            for (f, face) in mesh.faces.iter().enumerate() {
                if mesh.is_polyline(f) || face.vertices.len() < 3 {
                    continue;
                }
                let m: f64 = face.vertices.iter().map(|&v| value(v)).sum::<f64>()
                    / face.vertices.len() as f64;
                if m <= 0.01 {
                    continue;
                }
                let pts: Vec<glam::DVec3> = face
                    .vertices
                    .iter()
                    .map(|&v| world.transform_point3(mesh.vertices[v as usize]))
                    .collect();
                let n = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
                if n.dot(view.eye - pts[0]) <= 0. {
                    continue;
                }
                let screen: Option<Vec<Pos2>> = pts
                    .iter()
                    .map(|p| view.project(*p).map(|(q, _)| pos2(q.x as f32, q.y as f32)))
                    .collect();
                let Some(screen) = screen else {
                    continue;
                };
                let alpha = (m * opacity * 200.).clamp(0., 255.) as u8;
                shapes.push(egui::Shape::convex_polygon(
                    screen,
                    egui::Color32::from_rgba_unmultiplied(235, 40, 40, alpha),
                    egui::Stroke::NONE,
                ));
            }
            painter.extend(shapes);
        }
    }
}
