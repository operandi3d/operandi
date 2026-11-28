//! The UV tab's stage 3 buttons (core uv_ops.rs): their option dialogs,
//! built with the shared dialog (UIC-80..99). A plain click runs a
//! button's command line with its usual values; `operandi.uv.dialog
//! <kind>` (Ctrl/Cmd+click, or the "..." buttons) opens the dialog first
//! (UVT-08), and OK runs `Command::UvOp` once (one undo step, UIC-96).
use super::*;
use operandi_core::uv_ops::{
    AxisChoice, ConnectTo, ErrorTests, PasteMode, SPLIT_LARGE, SPLIT_MEDIUM, TexelAction,
    TexelOptions, UvOp,
};

/// Which stage 3 dialog is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum OpDialog {
    ToOrigin,
    Texel,
    Split,
    Sew,
    MoveSew,
    Errors,
    SetValue,
    Paste,
    QuickAlign,
    Orient,
    /// Cut Map's UV Seam field: which seam map the buttons use.
    SeamMap,
    Symmetrize,
    SelectTile,
}

impl OpDialog {
    fn named(word: &str) -> Option<Self> {
        Some(match word.to_ascii_lowercase().as_str() {
            "toorigin" => Self::ToOrigin,
            "texel" => Self::Texel,
            "split" => Self::Split,
            "sew" => Self::Sew,
            "movesew" => Self::MoveSew,
            "errors" => Self::Errors,
            "setvalue" => Self::SetValue,
            "paste" => Self::Paste,
            "quickalign" => Self::QuickAlign,
            "orient" => Self::Orient,
            "seammap" => Self::SeamMap,
            "symmetrize" => Self::Symmetrize,
            "selecttile" => Self::SelectTile,
            _ => return None,
        })
    }
    fn title(&self) -> &'static str {
        match self {
            Self::ToOrigin => "To Origin",
            Self::Texel => "Texel Density",
            Self::Split => "Split UVs",
            Self::Sew => "Sew UVs",
            Self::MoveSew => "Move and Sew",
            Self::Errors => "Select Error UVs",
            Self::SetValue => "Set UV Value",
            Self::Paste => "Paste UVs",
            Self::QuickAlign => "Quick Align",
            Self::Orient => "Orient Pieces",
            Self::SeamMap => "UV Seam",
            Self::Symmetrize => "Symmetrize UVs",
            Self::SelectTile => "Select UVs in Tile",
        }
    }
}

const CONNECT: [&str; 3] = ["Average", "Center of Selected", "Center of Unselected"];
const CONNECT_VALUES: [ConnectTo; 3] = [
    ConnectTo::Average,
    ConnectTo::Selected,
    ConnectTo::Unselected,
];
const AXIS_CHOICES: [&str; 3] = ["Auto", "Horizontal", "Vertical"];
const AXIS_VALUES: [AxisChoice; 3] = [
    AxisChoice::Auto,
    AxisChoice::Horizontal,
    AxisChoice::Vertical,
];

fn rows(kind: &OpDialog, texel: &TexelOptions, seam: &str, seams: Vec<String>) -> Vec<form::Row> {
    use form::{Row, Unit, When};
    match kind {
        OpDialog::ToOrigin => vec![
            Row::number("udim", "UDIM", Unit::Count, 1001.)
                .range(1001., 1999.)
                .tip("The tile whose lower-left corner the islands move to"),
        ],
        OpDialog::Texel => vec![
            Row::choice(
                "action",
                "Apply",
                &["Individually", "Overall", "Normalize UVs"],
                0,
            )
            .tip("Each island on its own, all together, or all to their combined average"),
            Row::number("image", "Image Size", Unit::Suffix(" px"), texel.image_size)
                .range(1., 65536.)
                .tip("The texture resolution the density is measured for"),
            Row::number("density", "Texel Density", Unit::Suffix(" px/m"), texel.density)
                .range(0.0001, 1e9)
                .when(When::ChoiceIn("action", vec![0, 1]))
                .tip("Target pixels per metre"),
            Row::check("world", "World Space", texel.world_space)
                .tip("Item scale counts when measuring 3D size"),
            Row::check("upright", "Turn Upright", false)
                .tip("Turn each island upright first (Alt+click on the buttons)"),
        ],
        OpDialog::Split => vec![
            Row::choice("size", "Split", &["Plain", "Medium", "Large", "Custom"], 3)
                .tip("Plain splits without moving; Medium and Large push the sides apart"),
            Row::number("distance", "Distance", Unit::Scalar, SPLIT_MEDIUM)
                .range(0., 10.)
                .when(When::ChoiceIn("size", vec![3]))
                .tip("How far apart the two sides move, in UV units"),
        ],
        OpDialog::Sew => vec![
            Row::choice("connect", "Connect To", &CONNECT, 0)
                .tip("Where the two sides meet"),
            Row::check("selected_only", "Move Selected Only", false)
                .tip("Keep the unselected side still"),
        ],
        OpDialog::MoveSew => vec![
            Row::choice("connect", "Connect To", &["Average", "Selected", "Unselected"], 0)
                .tip("Which island moves: both halfway, the other one to the selected, or the selected one"),
            Row::check("scale", "Scale", true).tip("Scale the moving island so the seam fits"),
        ],
        OpDialog::Errors => {
            let d = ErrorTests::default();
            vec![
                Row::check("cross", "Cross Intersection", d.cross)
                    .tip("UV polygons overlapping another"),
                Row::check("self", "Self Intersection", d.self_intersection)
                    .tip("Twisted (bow-tie) UV polygons"),
                Row::check("flipped", "Flipped", d.flipped).tip("UV polygons wound clockwise"),
                Row::check("stacked", "Stacked", d.stacked)
                    .tip("UV polygons exactly on top of another"),
                Row::check("udim", "Crossing UDIM Borders", d.udim_border)
                    .tip("UV polygons crossing a tile edge"),
                Row::check("zero", "Zero Area", d.zero_area)
                    .tip("UV polygons squashed to a line or point"),
            ]
        }
        OpDialog::SetValue => vec![
            Row::check("set_u", "Set U", true),
            Row::number("u", "U", Unit::Scalar, 0.).when(When::Checked("set_u")),
            Row::check("set_v", "Set V", false),
            Row::number("v", "V", Unit::Scalar, 0.).when(When::Checked("set_v")),
        ],
        OpDialog::Paste => vec![
            Row::choice("mode", "Mode", &["Connectivity", "Selection"], 0).tip(
                "Connectivity matches by topology from the first selected polygon; Selection by selection order",
            ),
        ],
        OpDialog::Symmetrize => vec![
            Row::choice("axis", "Symmetry Axis", &["X", "Y", "Z"], 0)
                .tip("The axis the mesh is symmetric across in 3D"),
            Row::choice("direction", "Direction", &["Positive to Negative", "Negative to Positive"], 0)
                .tip("Which side's UVs are copied to the other"),
            Row::choice("mirror", "Mirror Axis", &["U", "V"], 0)
                .tip("The UV axis mirrored, about UV Symmetry's Center"),
        ],
        OpDialog::SelectTile => vec![
            Row::number("udim", "UDIM Number", Unit::Count, 1001.)
                .range(1001., 1999.)
                .tip("Select the polygons whose UVs lie wholly inside this tile"),
        ],
        OpDialog::SeamMap => vec![
            Row::text("name", "UV Seam", seam)
                .suggestions(seams)
                .tip("The seam map Add, Clear, Select, Seam Marker and the Unwrap buttons use; a new name makes a new map on the first Add"),
        ],
        OpDialog::QuickAlign | OpDialog::Orient => vec![
            Row::choice("mode", "Direction", &AXIS_CHOICES, 0)
                .tip("Auto picks the nearer axis; Horizontal lies along U, Vertical along V"),
        ],
    }
}

fn op_of(kind: &OpDialog, v: &form::Values) -> UvOp {
    match kind {
        OpDialog::ToOrigin => UvOp::ToOrigin {
            udim: v.number("udim").round().clamp(1001., 1999.) as u32,
        },
        OpDialog::Texel => UvOp::Texel(
            [
                TexelAction::Individually,
                TexelAction::Overall,
                TexelAction::Normalize,
            ][v.choice("action").min(2)],
            TexelOptions {
                image_size: v.number("image"),
                density: v.number("density"),
                world_space: v.bool("world"),
                upright: v.bool("upright"),
            },
        ),
        OpDialog::Split => UvOp::Split {
            distance: match v.choice("size") {
                0 => 0.,
                1 => SPLIT_MEDIUM,
                2 => SPLIT_LARGE,
                _ => v.number("distance"),
            },
        },
        OpDialog::Sew => UvOp::Sew {
            connect: CONNECT_VALUES[v.choice("connect").min(2)],
            selected_only: v.bool("selected_only"),
        },
        OpDialog::MoveSew => UvOp::MoveSew {
            connect: CONNECT_VALUES[v.choice("connect").min(2)],
            scale: v.bool("scale"),
        },
        OpDialog::Errors => UvOp::SelectErrors(ErrorTests {
            cross: v.bool("cross"),
            self_intersection: v.bool("self"),
            flipped: v.bool("flipped"),
            stacked: v.bool("stacked"),
            udim_border: v.bool("udim"),
            zero_area: v.bool("zero"),
        }),
        OpDialog::SetValue => UvOp::SetValue {
            u: v.bool("set_u").then(|| v.number("u")),
            v: v.bool("set_v").then(|| v.number("v")),
        },
        OpDialog::Paste => UvOp::Paste(if v.choice("mode") == 1 {
            PasteMode::Selection
        } else {
            PasteMode::Connectivity
        }),
        OpDialog::QuickAlign => UvOp::QuickAlign(AXIS_VALUES[v.choice("mode").min(2)]),
        OpDialog::Orient => UvOp::Orient(AXIS_VALUES[v.choice("mode").min(2)]),
        OpDialog::SelectTile => UvOp::SelectTile {
            udim: v.number("udim").round().clamp(1001., 1999.) as u32,
        },
        OpDialog::SeamMap | OpDialog::Symmetrize => unreachable!("handled before"),
    }
}

impl Operandi {
    /// `operandi.uv.dialog <kind>`: a stage 3 button's options first.
    pub(super) fn open_uv_op_dialog(&mut self, word: &str) {
        let Some(kind) = OpDialog::named(word) else {
            self.status = "Usage: operandi.uv.dialog toOrigin|texel|split|sew|moveSew|errors|setValue|paste|quickAlign|orient".into();
            return;
        };
        if self.uv_dialog.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog first".into();
            return;
        }
        if self.editor.session.live.is_some() {
            self.status = crate::unavailable::TOOL_ACTIVE.into();
            return;
        }
        let seams = self
            .editor
            .scene
            .active()
            .map(|i| operandi_core::uv_seams::seam_maps(&i.mesh))
            .unwrap_or_default();
        let rows = rows(
            &kind,
            &self.editor.uv_texel,
            &self.editor.uv_seam_name(),
            seams,
        );
        self.uv_dialog = Some(uv_dialogs::UvDialog::Op(
            self.form_dialog(kind.title(), rows),
            kind,
        ));
    }

    /// OK in a stage 3 dialog: run it once; Texel Density keeps its values.
    pub(super) fn uv_op_dialog_ok(
        &mut self,
        kind: &OpDialog,
        values: &form::Values,
    ) -> std::result::Result<(), String> {
        if *kind == OpDialog::Symmetrize {
            let o = operandi_core::uv_symmetry::SymmetrizeOptions {
                axis: values.choice("axis").min(2),
                positive_to_negative: values.choice("direction") == 0,
                mirror_axis: if values.choice("mirror") == 1 {
                    operandi_core::uv_edit::UvAxis::V
                } else {
                    operandi_core::uv_edit::UvAxis::U
                },
            };
            if self.run_ok(Command::UvSymmetrize(o)) {
                self.status = "Symmetrize UVs done".into();
            }
            return Ok(());
        }
        if *kind == OpDialog::SeamMap {
            let name = values.text("name").trim().to_owned();
            self.editor
                .dispatch(Command::UvSeam(operandi_core::uv_seams::SeamEdit::Use(
                    name.clone(),
                )))?;
            self.status = format!("Seam map: {name}");
            return Ok(());
        }
        let op = op_of(kind, values);
        if let UvOp::Texel(_, o) = &op {
            self.run(Command::UvTexel(*o));
        }
        let label = op.label();
        if self.run_ok(Command::UvOp(op)) {
            self.status = format!("{label} done");
        }
        Ok(())
    }
}

impl Operandi {
    /// Texture > Export UVs to SVG... / EPS... (UVT-120): asks where.
    pub(super) fn ask_uv_layout(&mut self, eps: bool) {
        let Some(item) = self.editor.scene.active() else {
            self.status = "Select a mesh to export its UVs".into();
            return;
        };
        if operandi_core::uv_maps::current_map(&item.mesh).is_none() {
            self.status = "The mesh has no UV map yet".into();
            return;
        }
        let name = format!("{} UVs.{}", item.name, if eps { "eps" } else { "svg" });
        let request = crate::file_chooser::Choose::SaveUvLayout {
            folder: self.texture_folder(),
            name,
            eps,
        };
        if !self.choose(request, crate::file_chooser::Purpose::UvLayout(eps)) {
            self.status = "No file panel here; type operandi.uv.export filename:<file.svg>".into();
        }
    }

    /// Writes the active mesh's UV layout to `path`.
    pub(super) fn export_uv_layout(&mut self, path: &std::path::Path, eps: bool) {
        let Some(item) = self.editor.scene.active() else {
            return;
        };
        let Some(map) = operandi_core::uv_maps::current_map(&item.mesh) else {
            return;
        };
        let format = if eps {
            operandi_core::uv_export::LayoutFormat::Eps
        } else {
            operandi_core::uv_export::LayoutFormat::Svg
        };
        self.status = match operandi_core::uv_export::uv_layout(&item.mesh, &map, format, 1024.)
            .and_then(|text| std::fs::write(path, text).map_err(|e| e.to_string()))
        {
            Ok(()) => format!("UV layout saved to {}", path.display()),
            Err(e) => format!("Couldn't export the UV layout: {e}"),
        };
    }
}

/// The Split and Sew brush (UVT-63), as the UV view uses it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct UvBrush {
    /// Brush radius in pixels (right-drag sizes it).
    pub size: f32,
    /// Sew instead of Split (Shift held flips it for a stroke).
    pub sew: bool,
    pub distance: f64,
    pub connect: ConnectTo,
}
impl Default for UvBrush {
    fn default() -> Self {
        Self {
            size: 20.,
            sew: false,
            distance: SPLIT_MEDIUM,
            connect: ConnectTo::Average,
        }
    }
}

impl Operandi {
    /// `operandi.uv.brush [split|sew|off]`: the Split and Sew brush on in
    /// that mode, or off.
    pub(super) fn uv_brush_command(&mut self, word: &str) {
        match word.to_ascii_lowercase().as_str() {
            "off" => self.uv_brush = None,
            // A second click on the same mode's button stops the brush.
            "sew" | "split" => {
                let sew = word.eq_ignore_ascii_case("sew");
                self.uv_brush = match self.uv_brush {
                    Some(b) if b.sew == sew => None,
                    other => Some(UvBrush {
                        sew,
                        ..other.unwrap_or_default()
                    }),
                }
            }
            _ => {
                self.uv_brush = if self.uv_brush.is_some() {
                    None
                } else {
                    Some(UvBrush::default())
                }
            }
        }
        self.status = match self.uv_brush {
            Some(b) => format!(
                "{} brush: brush over UV points in a UV view (Shift: {}), right-drag sizes it",
                if b.sew { "Sew" } else { "Split" },
                if b.sew { "split" } else { "sew" }
            ),
            None => "Split and Sew brush off".into(),
        };
    }

    /// A finished brush stroke: one Split or Sew of the points it passed.
    pub(super) fn uv_brush_stroke(&mut self, points: Vec<u32>, flip: bool) {
        let Some(b) = self.uv_brush else {
            return;
        };
        if points.is_empty() {
            return;
        }
        let op = UvOp::Brush {
            points,
            sew: b.sew != flip,
            distance: b.distance,
            connect: b.connect,
        };
        let label = op.label();
        if self.run_ok(Command::UvOp(op)) {
            self.status = format!("{label} done");
        }
    }
}
