//! Vertex panel (Milestone 3): Weld (drag, `weld_drag.rs`), Create clicks,
//! the vertex command rows' operations and dialogs, and their `vert.*`
//! commands. The tab itself is drawn from its form,
//! `assets/forms/toolbox.vertex.json` (layouts stage 3 batch 4).
//!
//! Geometry lives in `operandi_core::vertex_commands`; this module only maps
//! panel rows, clicks, dialogs and command text onto Editor commands, so the
//! panel, shortcuts and command line share one operation per row. Spellings
//! not confirmed by a spec are provisional (see the M3 acceptance packet).
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, ToolForm};
use operandi_core::{MergeOptions, ReorderMode, VertexCommand};

/// One-shot rows of the Vertex panel's Commands section, in panel order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VertexRow {
    NewVertex,
    MakePolygon,
    ShowVertices,
    Split,
    SplitEdge,
    SplitPolygon,
    SetPosition,
    Align,
    Merge,
    Join,
    JoinAveraged,
    Collapse,
    Remove,
    RemoveEdge,
    ReorderIndex,
    SyncIndexOrder,
}
impl VertexRow {
    pub(super) const ALL: [Self; 16] = [
        Self::NewVertex,
        Self::MakePolygon,
        Self::ShowVertices,
        Self::Split,
        Self::SplitEdge,
        Self::SplitPolygon,
        Self::SetPosition,
        Self::Align,
        Self::Merge,
        Self::Join,
        Self::JoinAveraged,
        Self::Collapse,
        Self::Remove,
        Self::RemoveEdge,
        Self::ReorderIndex,
        Self::SyncIndexOrder,
    ];
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::NewVertex => "New Vertex...",
            Self::MakePolygon => "Make Polygon",
            Self::ShowVertices => "Show Vertices",
            Self::Split => "Split",
            Self::SplitEdge => "Split Edge",
            Self::SplitPolygon => "Split Polygon",
            Self::SetPosition => "Set Position...",
            Self::Align => "Align...",
            Self::Merge => "Merge...",
            Self::Join => "Join...",
            Self::JoinAveraged => "Join Averaged",
            Self::Collapse => "Collapse",
            Self::Remove => "Remove",
            Self::RemoveEdge => "Remove Edge",
            Self::ReorderIndex => "Reorder Index...",
            Self::SyncIndexOrder => "Sync Index Order",
        }
    }
    /// Input-map action. Make Polygon and Split Polygon share the existing
    /// contextual Make (P) and Split (Ctrl+L) actions.
    pub(super) fn action(self) -> &'static str {
        match self {
            Self::NewVertex => "New Vertex",
            Self::MakePolygon => "Make",
            Self::ShowVertices => "Show Vertices",
            Self::Split => "Vertex Split",
            Self::SplitEdge => "Vertex Split Edge",
            Self::SplitPolygon => "Split",
            Self::SetPosition => "Vertex Set Position",
            Self::Align => "Vertex Align",
            Self::Merge => "Vertex Merge",
            Self::Join => "Vertex Join",
            Self::JoinAveraged => "Vertex Join Averaged",
            Self::Collapse => "Vertex Collapse",
            Self::Remove => "Vertex Remove",
            Self::RemoveEdge => "Vertex Remove Edge",
            Self::ReorderIndex => "Reorder Index",
            Self::SyncIndexOrder => "Sync Index Order",
        }
    }
    /// Rows whose dialog only makes sense on points: greyed out and refused
    /// outside Vertex mode instead of opening a dialog that refuses on OK.
    pub(super) fn needs_vertex_mode(self) -> bool {
        // Set Position, Align, Merge and Join also run from Edge and
        // Polygon mode on the points the selection gives (selection
        // inference, spec §4).
        matches!(self, Self::NewVertex | Self::ReorderIndex)
    }
    /// Whether the action is owned by this panel (not a shared action).
    pub(super) fn owns_action(self) -> bool {
        !matches!(self, Self::MakePolygon | Self::SplitPolygon)
    }
}

/// Canonical `vert.*` verbs handled by [`Operandi::vertex_canonical`].
pub(super) const VERBS: [&str; 13] = operandi_core::command_line::VERTEX_VERBS;

/// Option dialogs of the Vertex rows (New Vertex, Align, Merge, Reorder).
pub(super) struct VertexDialog {
    kind: VertexDialogKind,
    form: form::Dialog,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum VertexDialogKind {
    NewVertex,
    Align,
    Merge,
    Join,
    Reorder,
}
impl VertexDialogKind {
    /// The command's name without "…" (UIC-81).
    fn title(self) -> &'static str {
        match self {
            Self::NewVertex => "New Vertex",
            Self::Align => "Align",
            Self::Merge => "Merge",
            Self::Join => "Join",
            Self::Reorder => "Reorder Index",
        }
    }
    /// The dialog's rows (UIC-98).
    fn rows(self) -> Vec<form::Row> {
        use form::{Row, Unit, When};
        let keep_row = || {
            Row::check("keep_one", "Keep 1-Vertex Polygons", false)
                .tip("Polygons squashed to two corners stay as lines")
        };
        match self {
            Self::NewVertex => ["X", "Y", "Z"]
                .iter()
                .map(|a| {
                    Row::number(&a.to_lowercase(), a, Unit::Distance, 0.)
                        .tip(format!("The new vertex's {a} position"))
                })
                .collect(),
            Self::Align => {
                let axes: Vec<&str> = ALIGN_AXES.iter().map(|(n, _)| *n).collect();
                vec![
                    Row::choice("axis", "Axis", &axes, 0)
                        .tip("The axes the points are lined up on"),
                    Row::check("uniform", "Uniform", false).tip(
                        "Also space the points evenly along the line (provisional, VTO-Q10)",
                    ),
                ]
            }
            Self::Merge => {
                // VX-M04/M05: Range, Distance and the three options.
                vec![
                    Row::segments("range", "Range", &["Automatic", "Fixed"], 0)
                        .tip("Automatic merges points that sit on each other; Fixed uses Distance"),
                    // Distance only applies to a Fixed range (UIC-92/93).
                    Row::number("distance", "Distance", Unit::Distance, 0.001)
                        .range(0., f64::INFINITY)
                        .when(When::ChoiceIn("range", vec![1]))
                        .tip("Points closer than this merge"),
                    keep_row(),
                    Row::check("morph", "Test Morph Map Positions", false)
                        .tip("Points must also be within Distance in every morph map"),
                    Row::check("disco", "Merge Discontinuous UV Values", true)
                        .tip("Where merged corners now have the same UV, they share it"),
                ]
            }
            // VX-C01: Join's dialog.
            Self::Join => vec![
                keep_row(),
                Row::check("disco", "Join Discontinuous UVs", true)
                    .tip("Where joined corners now have the same UV, they share it"),
            ],
            // UIC-89: three short options are segments.
            Self::Reorder => vec![
                Row::segments("mode", "Mode", &["Swap", "Top", "Bottom"], 0)
                    .tip("Swap the two selected points' indices, or move the selection to the top or bottom"),
            ],
        }
    }
}
use operandi_core::command_line::{ALIGN_AXES, VertexLine};

impl Operandi {
    /// Run a one-shot Vertex row (panel, shortcut and the row's verb with
    /// default options; rows with options open their dialog).
    pub(super) fn run_vertex_row(&mut self, row: VertexRow) {
        if !self.vertex_row_available(row) {
            self.status = format!(
                "{} works in Vertex mode; switch to Vertex mode first",
                row.label().trim_end_matches('.')
            );
            return;
        }
        let command = match row {
            VertexRow::NewVertex => {
                return self.open_vertex_dialog(VertexDialogKind::NewVertex);
            }
            VertexRow::MakePolygon => return self.polygon_make(),
            VertexRow::ShowVertices => {
                self.display.settings.drawing.show_verts =
                    !self.display.settings.drawing.show_verts;
                self.status = format!(
                    "Show Vertices {}",
                    if self.display.settings.drawing.show_verts {
                        "on"
                    } else {
                        "off"
                    }
                );
                return;
            }
            VertexRow::SplitPolygon => return self.polygon_split(),
            VertexRow::SetPosition => return self.open_set_position(),
            VertexRow::Align => {
                return self.open_vertex_dialog(VertexDialogKind::Align);
            }
            VertexRow::Merge => {
                return self.open_vertex_dialog(VertexDialogKind::Merge);
            }
            VertexRow::ReorderIndex => {
                return self.open_vertex_dialog(VertexDialogKind::Reorder);
            }
            VertexRow::SyncIndexOrder => return self.sync_index_order(),
            VertexRow::Split => VertexCommand::Split,
            VertexRow::SplitEdge => VertexCommand::SplitEdge,
            VertexRow::Join => {
                return self.open_vertex_dialog(VertexDialogKind::Join);
            }
            VertexRow::JoinAveraged => VertexCommand::Join {
                average: true,
                options: MergeOptions::default(),
            },
            VertexRow::Collapse => VertexCommand::Collapse,
            VertexRow::Remove => VertexCommand::Remove,
            VertexRow::RemoveEdge => VertexCommand::RemoveEdge,
        };
        self.run_vertex_command(command);
    }

    pub(super) fn run_vertex_command(&mut self, command: VertexCommand) {
        let mark = self.editor.history_mark();
        let succeeded = self.run_ok(Command::VertexCommand(command));
        if succeeded {
            self.status = if !self.editor.recorded_since(mark) {
                format!("{}: nothing to change", command.label())
            } else {
                format!("{} done", command.label())
            };
        }
    }

    /// MV-C16: the first run captures the source mesh and its two selected
    /// polygons; a run on another mesh with the same two polygons selected
    /// renumbers that mesh's points to match.
    fn sync_index_order(&mut self) {
        let selection = &self.editor.scene.selection;
        let (Some(item), &[a, b]) = (selection.item, &selection.faces[..]) else {
            self.status = "Select two polygons (Polygon mode) on the source mesh, then the same two on the target".into();
            return;
        };
        if selection.mode != Mode::Polygon {
            self.status = "Sync Index Order uses two selected polygons in Polygon mode".into();
            return;
        }
        match self.sync_source {
            Some((source, faces)) if source != item => {
                let mark = self.editor.history_mark();
                let succeeded = self.run_ok(Command::SyncVertexOrder {
                    source_item: source,
                    source_faces: faces,
                });
                if succeeded {
                    self.sync_source = None;
                    self.status = if !self.editor.recorded_since(mark) {
                        "Sync Index Order: the point order already matches".into()
                    } else {
                        "Sync Index Order done".into()
                    };
                }
            }
            _ => {
                self.sync_source = Some((item, [a, b]));
                self.status = "Sync Index Order: source captured; select the same two polygons on the target mesh and run it again".into();
            }
        }
    }

    fn open_vertex_dialog(&mut self, kind: VertexDialogKind) {
        if self.has_modal() {
            self.status = "Close the current dialog first".into();
            return;
        }
        let mut form = self.form_dialog(kind.title(), kind.rows());
        if kind == VertexDialogKind::NewVertex {
            form = form.focus("x");
        }
        self.vertex_dialog = Some(VertexDialog { kind, form });
    }

    pub(super) fn vertex_dialog_window(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.vertex_dialog.take() else {
            return;
        };
        let title = dialog.kind.title();
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => {
                self.status = format!("{title} canceled");
                return;
            }
            form::Outcome::Ok => {}
            _ => {
                self.vertex_dialog = Some(dialog);
                return;
            }
        }
        // UIC-96: OK runs the command once. An operation error keeps the
        // dialog open with its message.
        let v = &dialog.form.values;
        let succeeded = match dialog.kind {
            VertexDialogKind::NewVertex => self.run_ok(Command::NewVertex(DVec3::new(
                v.number("x"),
                v.number("y"),
                v.number("z"),
            ))),
            VertexDialogKind::Align => {
                self.run_vertex_command(VertexCommand::Align {
                    axes: ALIGN_AXES[v.choice("axis").min(ALIGN_AXES.len() - 1)].1,
                    uniform: v.bool("uniform"),
                });
                true
            }
            VertexDialogKind::Merge => {
                let distance = if v.choice("range") == 1 {
                    v.number("distance")
                } else {
                    0.
                };
                self.run_vertex_command(VertexCommand::Merge {
                    distance,
                    options: MergeOptions {
                        keep_small: v.bool("keep_one"),
                        test_morphs: v.bool("morph"),
                        merge_uvs: v.bool("disco"),
                    },
                });
                true
            }
            VertexDialogKind::Join => {
                self.run_vertex_command(VertexCommand::Join {
                    average: false,
                    options: MergeOptions {
                        keep_small: v.bool("keep_one"),
                        merge_uvs: v.bool("disco"),
                        ..Default::default()
                    },
                });
                true
            }
            VertexDialogKind::Reorder => {
                let mode = [ReorderMode::Swap, ReorderMode::Top, ReorderMode::Bottom]
                    [v.choice("mode").min(2)];
                self.run_vertex_command(VertexCommand::Reorder(mode));
                true
            }
        };
        if succeeded {
            self.remember_form(&dialog.form);
        } else {
            dialog.form.error = Some(self.status.clone());
            self.vertex_dialog = Some(dialog);
        }
    }

    /// Arm or disarm Weld (panel row and `tool.set DragWeld`). While armed,
    /// dragging a point welds it (`weld_drag.rs`).
    pub(super) fn toggle_weld(&mut self) {
        if self.vertex_weld {
            self.vertex_weld = false;
            self.weld_drag = None;
            self.status = "Weld off".into();
            return;
        }
        if self.editor.session.has_active_task() {
            self.status = "Drop or Cancel the active tool before Weld".into();
            return;
        }
        if self.editor.scene.selection.mode != Mode::Vertex {
            self.status = "Weld works in Vertex mode".into();
            return;
        }
        self.edge_click = None;
        self.vertex_weld = true;
        self.weld_drag = None;
        self.status = "Weld: drag a point onto another point to weld them".into();
    }

    /// Nearest visible point of the active item under `p`.
    pub(super) fn pick_vertex(&self, p: Pos2, r: Rect) -> Option<u32> {
        let item = self
            .editor
            .scene
            .active()
            .filter(|i| self.editor.scene.shows(i))?;
        let matrix = self.editor.scene.world_matrix(item.id).ok()?;
        let surface = self.pick_surface();
        let mut best: Option<(f32, u32)> = None;
        for (v, &local) in item.mesh.vertices.iter().enumerate() {
            let world = matrix.transform_point3(local);
            if let Some((q, _)) = self.camera.project(world, r) {
                let d = q.distance(p);
                if d < best.map_or(12., |b| b.0) && surface.visible(&self.camera, world, r) {
                    best = Some((d, v as u32));
                }
            }
        }
        best.map(|b| b.1)
    }

    /// Viewport click for the live Create tool. Returns whether the click
    /// was consumed.
    pub(super) fn vertex_click_at(&mut self, p: Pos2, r: Rect) -> bool {
        let creating = self
            .editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.tool == Tool::VertexCreate);
        if !creating {
            return false;
        }
        let plane = (self.actual_plane(), self.workplane_center());
        let Some(point) = self.work_point(p, r, plane.0, plane.1) else {
            self.status = "Create: click where the pointer meets the work plane".into();
            return true;
        };
        let mut params = self.editor.session.live.as_ref().unwrap().params.clone();
        // VX-CR02: a placed point snaps (after Coordinate Rounding); the
        // points placed so far are targets too.
        self.snap_session = Default::default();
        let point = self.snap_placement(point, &params.sketch_path, None, r);
        params.sketch_path.push(point);
        let succeeded = self.run_ok(Command::Preview(params));
        self.clear_fields();
        if succeeded {
            self.status = "Create: click to add points; edit the last point's X/Y/Z; Drop to finish, Esc to cancel".into();
        }
        true
    }

    /// Canonical `vert.*` / `view3d.showVertices` commands, their help, and
    /// `tool.set DragWeld` (old: vert.weld). Returns false when `text` is
    /// not one of them.
    pub(super) fn vertex_canonical(&mut self, text: &str) -> bool {
        let args = match command::tokens(text) {
            Ok(a) if !a.is_empty() => a,
            _ => return false,
        };
        let first = args[0].to_ascii_lowercase();
        if first == "help" && args.len() == 2 {
            let topic = args[1].to_ascii_lowercase();
            if topic == "dragweld" {
                self.status = vertex_help("dragweld", |action| self.shortcut(action));
                return true;
            }
            if VERBS.contains(&topic.as_str()) {
                self.status = vertex_help(&topic, |action| self.shortcut(action));
                return true;
            }
            return false;
        }
        if first == "tool.set"
            && args
                .get(1)
                .is_some_and(|a| a.eq_ignore_ascii_case("DragWeld"))
        {
            match args.get(2).map(|m| m.to_ascii_lowercase()).as_deref() {
                Some("on" | "1") if args.len() == 3 => {
                    if !self.vertex_weld {
                        self.toggle_weld();
                    }
                }
                Some("off" | "0") if args.len() == 3 => {
                    if self.vertex_weld {
                        self.toggle_weld();
                    }
                }
                Some("?") if args.len() == 3 => {
                    self.status = if self.vertex_weld { "1" } else { "0" }.into()
                }
                _ => self.status = "Use tool.set DragWeld on, off or ?".into(),
            }
            return true;
        }
        // Parsing is core's (command_line::parse_vertex); the app runs it.
        let Some(line) = operandi_core::command_line::parse_vertex(&first, &args[1..]) else {
            return false;
        };
        let usage = vertex_usage(&first);
        match line {
            VertexLine::Row(verb) => self.run_vertex_row(match verb {
                "vert.new" => VertexRow::NewVertex,
                "view3d.showvertices" => VertexRow::ShowVertices,
                "vert.reorder" => VertexRow::ReorderIndex,
                "vert.split" => VertexRow::Split,
                "vert.splitedge" => VertexRow::SplitEdge,
                "vert.collapse" => VertexRow::Collapse,
                "vert.remove" => VertexRow::Remove,
                "vert.removeedge" => VertexRow::RemoveEdge,
                _ => VertexRow::SyncIndexOrder,
            }),
            VertexLine::New(at) => {
                if self.run_ok(Command::NewVertex(at)) {
                    self.status = "New Vertex done".into();
                }
            }
            VertexLine::ShowVertices(on) => self.display.settings.drawing.show_verts = on,
            VertexLine::ShowVerticesQuery => {
                self.status = if self.display.settings.drawing.show_verts {
                    "1"
                } else {
                    "0"
                }
                .into();
            }
            VertexLine::Weld(target) => {
                let sources = self.editor.scene.selection.vertices.clone();
                if self.run_ok(Command::WeldVertices { sources, target }) {
                    self.status = "Weld done".into();
                }
            }
            VertexLine::Command(command) => self.run_vertex_command(command),
            VertexLine::Usage => self.status = usage,
            VertexLine::Error(e) => self.status = e,
        }
        true
    }

    pub(super) fn vertex_row_available(&self, row: VertexRow) -> bool {
        !row.needs_vertex_mode() || self.editor.scene.selection.mode == Mode::Vertex
    }
}

fn vertex_usage(verb: &str) -> String {
    match verb {
        "vert.new" => {
            "Use vert.new <x> <y> <z> (distances, or x: y: z:), or vert.new to open the dialog"
        }
        "vert.align" => "Use vert.align [axis:all|x|y|z|xy|yz|zx] [uniform:true|false]",
        "vert.merge" => {
            "Use vert.merge [range:auto] or vert.merge range:fixed dist:<distance> [keep:false]"
        }
        "vert.join" => "Use vert.join [average:true|false]",
        "vert.reorder" => "Use vert.reorder mode:swap|top|bottom (no arguments opens the dialog)",
        "vert.weld" => "Use vert.weld target:<point index> (the selected points weld onto it)",
        "view3d.showvertices" => "Use view3d.showVertices [on|off|?] (or state:on|off)",
        other => return format!("{other} takes no arguments"),
    }
    .into()
}

fn vertex_help(verb: &str, shortcut: impl Fn(&str) -> String) -> String {
    let (text, action) = match verb {
        "vert.new" => (
            "vert.new <x> <y> <z> adds one point at that world position to the active mesh and selects it; without arguments opens Create New Vertex",
            "New Vertex",
        ),
        "vert.split" => (
            "vert.split gives every polygon using a selected point its own coincident copy (Vertex mode)",
            "Vertex Split",
        ),
        "vert.splitedge" => (
            // Spec: M3-Q02 provisional reading
            "vert.splitEdge adds a middle point to every edge whose two endpoints are selected",
            "Vertex Split Edge",
        ),
        "vert.align" => (
            "vert.align [axis:all|x|y|z|xy|yz|zx] [uniform:true|false] lines the ordered selected points up between the first and last",
            "Vertex Align",
        ),
        "vert.merge" => (
            "vert.merge [range:auto] | range:fixed dist:<distance> welds selected points that coincide or lie within the distance, at the most recently selected one; Keep 1-Vertex Polygons is unsupported",
            "Vertex Merge",
        ),
        "vert.join" => (
            "vert.join [average:true] welds the selected points into the last selected (or their average: Join Averaged)",
            "Vertex Join",
        ),
        "vert.collapse" => (
            "vert.collapse removes each selected point and merges its polygons into one (no hole)",
            "Vertex Collapse",
        ),
        "vert.remove" => (
            "vert.remove deletes the selected points and every polygon using them",
            "Vertex Remove",
        ),
        "vert.removeedge" => (
            "vert.removeEdge dissolves edges between two selected points; their polygons merge",
            "Vertex Remove Edge",
        ),
        "vert.reorder" => (
            "vert.reorder mode:swap|top|bottom renumbers the selected points; the shape is unchanged",
            "Reorder Index",
        ),
        "vert.syncorder" => (
            "vert.syncOrder: select two polygons on the source mesh and run it, then the same two on the target mesh and run it again; the target's points are renumbered to match",
            "Sync Index Order",
        ),
        "vert.weld" => (
            "vert.weld target:<index> welds the selected points onto that point; the Weld panel row (tool.set DragWeld on) welds by dragging one point onto another",
            "Weld",
        ),
        "dragweld" => (
            "tool.set DragWeld on|off|? arms or disarms Weld (the Vertex panel row): drag a point onto another to merge them there (old id: vert.weld)",
            "Weld",
        ),
        _ => (
            "view3d.showVertices [on|off|?] turns the Show Vertices checkbox on or off; off, Vertex mode draws only the selected points; no scene or history change",
            "Show Vertices",
        ),
    };
    format!(
        // Spec: M3-Q05
        "Help: {text}. Vertex panel{}; one undo step per edit. Command spelling may change.",
        shortcut(action).trim_end()
    )
}

/// Tool Properties of the Vertex panel's live tools, per
/// `specifications/vertex-tool-options-2026-11-03/OPTIONS-TOOLS.md`.
/// Options whose behaviour was never measured are left out (UIC-117); the
/// records say why.
pub(super) fn vertex_tool_form(tool: Tool, p: &Parameters, f: &mut ToolForm<Parameters>) {
    use crate::live_fields::{check, dist, vec3, vertex_material_fields, whole};
    match tool {
        Tool::VertexCreate => {
            if p.sketch_path.is_empty() {
                f.heading("Click on the work plane to add a point.");
            } else {
                // Create's X/Y/Z fields edit the last clicked point (world).
                vec3(
                    f,
                    "create_point",
                    "Position",
                    Unit::Distance,
                    " m",
                    |p| p.sketch_path.last().copied().unwrap_or_default(),
                    |p, a, v| {
                        if let Some(last) = p.sketch_path.last_mut() {
                            last[a] = v;
                        }
                    },
                );
            }
            f.push(unmeasured(
                "Point Polygon",
                "off",
                "VTO-Q08",
                Basis::Unknown,
            ))
            .note(format!(
                "{} point(s) · Drop keeps them selected in order; Esc cancels.",
                p.sketch_path.len()
            ));
        }
        Tool::VertexBevel => {
            f.field(dist("inset", "Inset", |p| p.inset, |p, v| p.inset = v));
            vertex_material_fields(p, f);
            // Segments, as on Edge Bevel and Edge Chamfer (the maintainer,
            // 2026-11-26): the segments along each side of the cap, 1 being
            // flat. VX-B02's Round Level L gives 2·L, and the cap's grid
            // needs an even count above 1, so an odd count steps on to the
            // next even one in the direction it was changed
            // (`vertex_bevel_segments_to_level`). Scripts can still set
            // `level` (Round Level).
            f.field(
                whole(
                    "vertex_segments",
                    "Segments",
                    (1., 2. * operandi_core::VERTEX_BEVEL_MAX_ROUND_LEVEL as f64),
                    "Segments requires an integer from 1 to 64",
                    |p| vertex_bevel_level_to_segments(p.vertex_round_level) as f64,
                    |p, n| {
                        p.vertex_round_level =
                            vertex_bevel_segments_to_level(n, p.vertex_round_level)
                    },
                )
                .tip("Segments along each side of the cap: 1 is flat, then 2, 4, 6 ..."),
            )
            .note("An Inset longer than an edge stops at its far end; border points keep their open side.");
        }
        Tool::VertexExtrude => {
            f.field(dist("shift", "Extrude", |p| p.shift, |p, v| p.shift = v))
                .field(dist("inset", "Width", |p| p.inset, |p, v| p.inset = v));
            vertex_material_fields(p, f);
            f.note("Nothing selected extrudes every point; Width 0 changes nothing.");
        }
        Tool::VertexMerge => {
            f.field(dist("shift", "Distance", |p| p.shift, |p, v| p.shift = v))
                .field(
                    check(
                        "vertex_merge_keep_small",
                        "Keep 1-Vertex Polygons",
                        |p| p.vertex_merge_keep_small,
                        |p, on| p.vertex_merge_keep_small = on,
                    )
                    .tip("Polygons squashed to two corners stay as lines"),
                )
                .field(
                    check(
                        "vertex_merge_test_morphs",
                        "Test Morph Map Positions",
                        |p| p.vertex_merge_test_morphs,
                        |p, on| p.vertex_merge_test_morphs = on,
                    )
                    .tip("Points must also be within Distance in every morph map"),
                )
                .note("Selected points within Distance (or closer) merge at their average; nothing stays selected.");
        }
        Tool::VertexTear => {
            vec3(
                f,
                "delta",
                "Offset",
                Unit::Distance,
                " m",
                |p| p.delta,
                |p, a, v| p.delta[a] = v,
            );
            f.note("Select polygons, then a point on them: the selected polygons' corner tears off with its own point and follows the offset.")
                .note("Tear's other options aren't known yet (VTO-Q06).");
        }
        Tool::VertexSlide => vertex_slide_form(p, f),
        _ => {}
    }
}

/// VTO-T07 Vertex Slide. Built: Interpolation, Percent/Distance and Merge
/// Vertices. Face Handle keeps its state but nothing uses it yet;
/// Direction is left out until it's measured on points (VTO-Q07).
fn vertex_slide_form(p: &Parameters, f: &mut ToolForm<Parameters>) {
    use crate::live_fields::{check, dist, interpolation, slide_extras};
    for axis in ["X", "Y", "Z"] {
        f.push(unmeasured(
            format!("Direction {axis}"),
            "0",
            "VTO-Q07",
            Basis::Recorded,
        ));
    }
    f.field(
        check(
            "vertex_slide_merge",
            "Merge Vertices",
            |p| p.vertex_slide_merge,
            |p, on| p.vertex_slide_merge = on,
        )
        .tip("A point that reaches its neighbour merges with it"),
    )
    .field(
        check(
            "vertex_slide_face_handle",
            "Face Handle",
            |p| p.vertex_slide_face_handle,
            |p, on| p.vertex_slide_face_handle = on,
        )
        .tip("Kept as a setting; it doesn't change the slide yet (VTO-Q07)"),
    )
    // No UV map is used by Slide yet, so "(none)" is the only entry.
    .field(tf::fixed(
        "UV Map",
        "(none)",
        "No UV map is used by Slide yet",
    ))
    .field(interpolation("vertex-slide-interpolation"));
    if p.edge_slide_distance {
        f.field(dist(
            "edge_slide_distance",
            "Distance",
            |p| p.shift,
            |p, v| p.shift = v,
        ));
    } else {
        f.field(tf::percent(
            "vertex_slide_percent",
            "Percent",
            "%",
            |p: &Parameters| p.shift,
            |p, v| {
                // VTO-T07: −100 % to 100 %; Loop Slide off lets a point
                // carry on past its edge.
                if p.slide_extras.loop_slide && !(-100. ..=100.).contains(&v) {
                    return Err("Slide Percent must be between -100% and 100%".into());
                }
                p.shift = v / 100.;
                Ok(())
            },
        ));
    }
    f.note("Along the connected edge best aligned with the drag; ±100% reaches the neighbouring point.");
    slide_extras(p, f, false);
}

/// Where an unbuilt option's default comes from (spec evidence marks).
#[derive(Clone, Copy)]
enum Basis {
    /// R: read back from Modo's own tool settings.
    Recorded,
    /// No default known.
    Unknown,
}

/// Hover text for an unmeasured option: why it isn't built.
fn unmeasured_note(question: &str, basis: Basis) -> String {
    let default = match basis {
        Basis::Recorded => "Shown at its default value.",
        Basis::Unknown => "Default not recorded.",
    };
    format!("Not built yet: its behaviour hasn't been measured ({question}). {default}")
}

/// An option the specs list whose behaviour was never measured: left out
/// (UIC-117), recorded with why.
fn unmeasured<P>(
    label: impl Into<String>,
    value: &str,
    question: &str,
    basis: Basis,
) -> tf::Item<P> {
    tf::unbuilt(label, value, unmeasured_note(question, basis))
}

/// Vertex Bevel's Segments for Round Level `level`: 2·L, and 1 (flat) for
/// level 0 (VX-B02).
pub(crate) fn vertex_bevel_level_to_segments(level: u32) -> u32 {
    (2 * level).max(1)
}

/// The Round Level for Segments `n`, given the current `level`. Even counts
/// map straight back; an odd count above 1 has no cap grid, so it steps on
/// to the next even count in the direction it moved (3 from 2 gives 4, 3
/// from 4 gives 2), and a field's arrows or a haul never stick.
pub(crate) fn vertex_bevel_segments_to_level(n: u32, level: u32) -> u32 {
    if n <= 1 {
        0
    } else if n.is_multiple_of(2) || n > vertex_bevel_level_to_segments(level) {
        n.div_ceil(2)
    } else {
        n / 2
    }
    .min(operandi_core::VERTEX_BEVEL_MAX_ROUND_LEVEL)
}

#[cfg(test)]
mod vertex_bevel_segments_tests {
    use super::*;

    #[test]
    fn segments_count_the_cap_sides_and_odd_counts_step_on() {
        assert_eq!(vertex_bevel_level_to_segments(0), 1);
        assert_eq!(vertex_bevel_level_to_segments(3), 6);
        assert_eq!(vertex_bevel_segments_to_level(1, 2), 0);
        assert_eq!(vertex_bevel_segments_to_level(4, 0), 2);
        // Up from 2 (level 1) to 3: on to 4. Down from 4 (level 2) to 3: 2.
        assert_eq!(vertex_bevel_segments_to_level(3, 1), 2);
        assert_eq!(vertex_bevel_segments_to_level(3, 2), 1);
        // Up from flat to 2.
        assert_eq!(vertex_bevel_segments_to_level(2, 0), 1);
        assert_eq!(vertex_bevel_segments_to_level(64, 0), 32);
    }
}

#[cfg(test)]
mod vertex_bevel_level_script_tests {
    use crate::Operandi;
    use operandi_core::{Command, Mode};

    /// Scripts still set and read Vertex Bevel's Round Level as `level`,
    /// and `segments` sets the panel's Segments (2026-11-26).
    #[test]
    fn level_still_sets_vertex_bevel_round_level() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
                .unwrap()
                .into();
        a.run(Command::Mode(Mode::Vertex));
        a.editor.scene.selection.vertices = vec![0];
        a.execute("tool.set vert.bevel on");
        a.execute("tool.attr vert.bevel level 2");
        let level = |a: &Operandi| {
            a.editor
                .session
                .live
                .as_ref()
                .unwrap()
                .params
                .vertex_round_level
        };
        assert_eq!(level(&a), 2, "{}", a.status);
        a.execute("tool.attr vert.bevel segments 6");
        assert_eq!(level(&a), 3, "{}", a.status);
        a.execute("tool.attr vert.bevel level ?");
        assert!(a.status.contains('3'), "{}", a.status);
    }
}
