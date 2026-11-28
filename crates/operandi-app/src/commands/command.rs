//! Finite reviewed command vocabulary. Modeling behavior remains in Editor.
use super::*;
pub(super) use operandi_core::command_line::chamfer_edit_arguments;
use operandi_core::command_line::{set_part_argument, set_position_arguments};
use scale_offset_dialog::SCALE_OFFSET_COMMAND;

/// `tool.set` ids, named as in the name map (command_names.rs, which also
/// turns the old Operandi ids into these). The second field is the
/// command-line name the id starts.
pub(super) const IDS: [(&str, &str); 72] = [
    ("prim.cube", "cube"),
    ("prim.sphere", "sphere"),
    ("prim.cylinder", "cylinder"),
    ("prim.ellipsoid", "ellipsoid"),
    ("prim.cone", "cone"),
    ("poly.bevel", "bevel"),
    ("poly.extrude", "extrude"),
    ("poly.smshift", "smooth shift"),
    ("Thicken", "thicken"),
    ("poly.spikey", "spike"),
    ("Transform", "transform"),
    ("TransformMove", "move"),
    ("TransformRotate", "rotate"),
    ("TransformScale", "scale"),
    ("edge.bridge", "bridge"),
    // RED-001: same session as the Polygon panel row and input action.
    ("poly.reduct", "polygon reduction"),
    ("chamfer.edit", "chamfer edit"),
    ("poly.haul", "poly haul"),
    ("poly.sweep", "sketch extrude"),
    // Tool audit Tier 1. Operandi's own IDs: the gap list doesn't name them.
    ("poly.inset", "inset"),
    ("poly.shift", "shift"),
    // M2 Edge tools.
    ("edge.extend", "extend"),
    ("edge.extrude", "edge extrude"),
    ("edge.slide", "slide"),
    ("edge.crease", "edge weight"),
    ("edge.bevel", "edge bevel"),
    ("edge.chamfer", "edge chamfer"),
    ("edge.relax", "edge relax"),
    // M3 Vertex tools (DragWeld is a click tool handled in vertex_panel;
    // vert.tear and vert.slide have no map tool id: Tear is the
    // @ppi_vert_tear.py command there, Slide is *.Slide).
    ("prim.makeVertex", "create"),
    ("vert.bevel", "vertex bevel"),
    ("vert.extrude", "vertex extrude"),
    ("vert.merge", "merge tool"),
    ("vert.tear", "tear"),
    ("vert.slide", "vertex slide"),
    // Element Move (TWK-01): `tweak` is the old spelling (command_names.rs).
    ("ElementMove", "element move"),
    // NPR primitives.
    ("prim.toroid", "torus"),
    ("prim.capsule", "capsule"),
    ("prim.arc", "arc"),
    ("prim.tube", "tube"),
    // Deform tab.
    ("xfrm.push", "push"),
    ("xfrm.smooth", "smooth"),
    ("xfrm.jitter", "jitter"),
    ("xfrm.quantize", "quantize"),
    ("xfrm.radialAlign", "radial align"),
    ("xfrm.linearAlign", "linear align"),
    ("xfrm.bend", "bend"),
    // Symmetry's repair tool (symmetry-2026-11-13 section 9). Operandi's own
    // ID until the name map gives one (SYM3-Q16).
    ("operandi.symmetry.tool", "symmetry tool"),
    // Edit tab Slice tools.
    ("poly.loopSlice", "loop slice"),
    ("edge.knife", "edge slice"),
    ("poly.knife", "polygon slice"),
    // Edit tab waves 2 and 3 (name map MeshEdit rows).
    ("Curve Slice", "curve slice"),
    ("PenKnife", "penknife"),
    ("Primitive Slice", "primitive slice"),
    ("poly.julienne", "polygon axis slice"),
    ("Contour", "contour"),
    // Duplicate tab (name map Duplicate rows).
    ("*.mirror", "mirror"),
    ("Radial Sweep", "lathe"),
    ("*.clone", "clone"),
    ("*.array", "array"),
    ("*.Radial Array", "radial array"),
    // Twist Bevel (twist-bevel-2026-11-15): the spec's own id.
    ("poly.twistBevel", "twist bevel"),
    // The path tools, bare (their buttons are preset rows; CX-Q6).
    ("curve.extrude", "curve.extrude"),
    ("curve.clone", "curve.clone"),
    ("scatter.clone", "scatter clone"),
    // Symmetrize, the Duplicate tab tool (DX-40): Operandi's own id.
    ("operandi.symmetrize.tool", "symmetrize"),
    // UV tab live tools. uv.unwrap, uv.relax and uv.pelt are the name
    // map's TOOL rows; Project's two are Operandi's own until it names them
    // (NAMES-NEEDED.md).
    ("operandi.uv.project", "uv project"),
    ("operandi.uv.viewProject", "project from view"),
    ("uv.unwrap", "unwrap tool"),
    ("uv.relax", "uv relax"),
    ("uv.pelt", "unwrap & relax"),
    ("operandi.uv.peeler", "peeler"),
    ("operandi.uv.boxTransform", "box transform"),
];
#[cfg(test)]
const ATTRS: [&str; 8] = [
    "factor",
    "shift",
    "scale",
    "maxAngle",
    "thicken",
    "sharp",
    "angleWeight",
    "bothSides",
];

/// The command-line tokenizer moved to core (extensibility stage 2).
pub(super) use operandi_core::command_line::tokens;
pub(super) fn id(s: &str) -> Option<usize> {
    IDS.iter()
        .position(|(name, _)| name.eq_ignore_ascii_case(s))
}
pub(super) fn canonical_pen_payload(text: &str) -> Option<&str> {
    let text = text.trim_start();
    let end = text.find(char::is_whitespace)?;
    if !text[..end].eq_ignore_ascii_case("operandi.pen.create") {
        return None;
    }
    Some(text[end..].trim_start())
}

/// Report a refused command-line entry in the status bar.
fn fail(app: &mut Operandi, message: &str) {
    app.status = message.into();
    app.last_failure = Some(message.into());
}

impl Operandi {
    pub(super) fn polygon_quadruple(&mut self) {
        if self.has_modal() {
            self.status = "Close the open dialog before Quadruple".into();
            return;
        }
        self.run(Command::QuadruplePolygon);
    }
    pub(super) fn polygon_spin(&mut self) {
        if self.has_modal() {
            self.status = "Close the open dialog before Spin Quad".into();
            return;
        }
        self.run(Command::SpinQuads);
    }
    /// MK-U01: the Polygon panel Make row, `poly.make auto` and the "Make"
    /// input action share this one dispatch of `Command::MakePolygon`. Field
    /// drafts of other tools are neither confirmed nor discarded (see `run`).
    pub(super) fn polygon_make(&mut self) {
        if self.has_modal() {
            self.status = "Close the open dialog before Make".into();
            return;
        }
        // Success is the authored Scene actually changing. Undo length is not
        // a success signal: bounded history evicts its oldest entry at 100.
        // Make never succeeds without a change, and a refusal leaves the Scene
        // exactly as it was.
        let before = self.editor.scene.clone();
        let succeeded = self.run_ok(Command::MakePolygon);
        if succeeded && self.editor.scene != before {
            self.status = "Make: added one polygon".into();
        }
    }
    pub(super) fn polygon_split(&mut self) {
        if self.has_modal() {
            self.status = "Close the open dialog before Split".into();
            return;
        }
        self.run(Command::SplitPolygon);
    }
    /// TRI-001: the Polygon panel row, `poly.triple` and the "Triple" input
    /// action share this one dispatch of `Command::Triple`. Field drafts of
    /// other tools are neither confirmed nor discarded (see `run`).
    pub(super) fn polygon_triple(&mut self) {
        let before = self.editor.scene.clone();
        let succeeded = self.run_ok(Command::Triple);
        if !succeeded {
            return;
        }
        self.status = if self.editor.scene == before {
            "Triple: selected polygons are already triangles; nothing changed".into()
        } else {
            format!(
                "Triple: {} selected triangles",
                self.editor.scene.selection.faces.len()
            )
        };
    }
    /// REM-001: the Polygons panel row, `poly.remove` and the remappable
    /// Remove input action all dispatch this one shared core operation.
    /// Field drafts of other panels are neither confirmed nor discarded.
    pub(super) fn remove_polygons(&mut self) {
        let count = self.editor.scene.selection.faces.len();
        let mark = self.editor.history_mark();
        let succeeded = self.run_ok(Command::RemovePolygons);
        if succeeded && self.editor.recorded_since(mark) {
            let plural = if count == 1 { "" } else { "s" };
            self.status = format!("Removed {count} polygon{plural}");
        }
    }
    /// MER-001: the Polygon panel Merge row, `poly.merge` and the "Polygon
    /// Merge" input action share this one dispatch of `Command::MergePolygons`.
    /// Field drafts of other tools are neither confirmed nor discarded.
    pub(super) fn polygon_merge(&mut self) {
        let count = self.editor.scene.selection.faces.len();
        let succeeded = self.run_ok(Command::MergePolygons);
        if succeeded {
            self.status = format!("Merge: {count} polygons merged");
        }
    }
    /// COL-B02: the Polygon panel Collapse row, `poly.collapse` and the
    /// "Polygon Collapse" input action share this one dispatch of
    /// `Command::CollapsePolygon`. Field drafts of other tools are neither
    /// confirmed nor discarded.
    pub(super) fn polygon_collapse(&mut self) {
        let count = self.editor.scene.selection.faces.len();
        let mark = self.editor.history_mark();
        let succeeded = self.run_ok(Command::CollapsePolygon);
        if succeeded && self.editor.recorded_since(mark) {
            let plural = if count == 1 { "" } else { "s" };
            self.status = format!("Collapse: {count} polygon{plural} collapsed");
        }
    }
    /// The row, exact action and remapped key share the same preflight and edit.
    pub(super) fn flatten_ngons(&mut self) {
        if self.has_modal() {
            self.status = "Close the open dialog before Flatten Ngons".into();
            return;
        }
        // Several active meshes: the plan reads the primary only, so the
        // command runs on each mesh and reports per mesh (multi_tools.rs).
        if self.editor.scene.mesh_edit_targets().is_some() {
            if self.run_ok(Command::FlattenNgons) {
                self.status = "Flatten Ngons: done on every selected mesh".into();
            }
            return;
        }
        match self.editor.flatten_ngons_plan() {
            Err(reason) => self.status = reason,
            Ok(plan) => match plan.outcome {
                operandi_core::FlattenOutcome::NoChange => {
                    // Still clears the selection (offset-flatten rule 6).
                    let selection = &self.editor.scene.selection;
                    let selected = !selection.faces.is_empty() || !selection.vertices.is_empty();
                    if !selected || self.run_ok(Command::FlattenNgons) {
                        self.status = "Flatten Ngons: the n-gons are already planar".into();
                    }
                }
                operandi_core::FlattenOutcome::Refused(refusal) => {
                    self.status = refusal.message();
                }
                operandi_core::FlattenOutcome::Change(_) => {
                    let succeeded = self.run_ok(Command::FlattenNgons);
                    if succeeded {
                        self.status = "Flatten Ngons: polygons flattened".into();
                    }
                }
            },
        }
    }
    /// Row of the command table for the open tool or numeric session.
    pub(super) fn command_active(&self) -> Option<usize> {
        let session = if self.editor.session.reduction.is_some() {
            Some("poly.reduct")
        } else if self.editor.session.bridge.is_some() {
            Some("edge.bridge")
        } else if self.editor.session.spike.is_some() {
            Some("poly.spikey")
        } else if self.editor.session.ellipsoid.is_some() {
            Some("prim.ellipsoid")
        } else if let Some(s) = &self.editor.session.shape {
            Some(match s.kind() {
                ShapeKind::Torus => "prim.toroid",
                ShapeKind::Capsule => "prim.capsule",
                ShapeKind::Arc => "prim.arc",
                ShapeKind::Tube => "prim.tube",
                ShapeKind::Text => "prim.text",
                ShapeKind::Teapot => "Teapot",
            })
        } else if self.editor.session.cone.is_some() {
            Some("prim.cone")
        } else {
            None
        };
        let command_id = session.or_else(|| {
            let live = self.editor.session.live.as_ref()?;
            Some(tool_info::info(live.tool).command_id)
        })?;
        id(command_id)
    }
    /// A typed line naming a command that runs without a window (registry
    /// flag `headless`), run through core's `command_line` like the MCP
    /// bridge's `command_run`: refused while a dialog or tool is in use,
    /// staged so a refusal leaves the scene as it was. False when the line
    /// names no such command.
    pub(super) fn headless_command(&mut self, text: &str) -> bool {
        let Some(first) = text.split_whitespace().next() else {
            return false;
        };
        let Some(entry) = operandi_core::command_registry::find(first).filter(|e| e.flags.headless)
        else {
            return false;
        };
        // PD-Q12: typed at the keyboard, these join a live Move/Rotate/
        // Scale/Transform in Items mode (select.item re-targets it; a
        // transform edit of its Texture Locator is one of its tweaks), so
        // the active tool doesn't refuse them. Scripts and the MCP bridge
        // still wait for the tool to close.
        let joins_tool = entry.flags.allowed_during_transform_tool
            && self.editor.scene.selection.mode == Mode::Item
            && self
                .editor
                .session
                .live
                .as_ref()
                .is_some_and(|l| l.tool.transforms_items());
        // An open dialog or a mouse drag still refuses them.
        let only_the_tool = joins_tool
            && self.open_dialog().is_none()
            && self.navigation.is_none()
            && self.editor.selection_start.is_none();
        if !only_the_tool && let Some(refusal) = self.mcp_refusal("command_run") {
            self.status = refusal;
            return true;
        }
        let mut staged = self.editor.clone();
        match operandi_core::command_line::run_line(&mut staged, text) {
            // A query's answer; nothing changed.
            Some(Ok(Some(answer))) => self.status = answer,
            Some(Ok(None)) => {
                self.editor = staged;
                // Numeric field drafts describe the scene before the line.
                self.clear_fields();
                if let Some(i) = self.editor.scene.active() {
                    self.name = i.name.clone();
                }
                self.status = self
                    .editor
                    .command_note
                    .take()
                    .unwrap_or_else(|| "Ready".into());
            }
            Some(Err(e)) => self.status = e,
            None => return false,
        }
        true
    }
    /// A panel's edit as its command line (UIC-164/165): core builds the
    /// one `Command` from the line and the app runs it the usual way, so a
    /// drag or a colour pick still folds into one undo step. Lines that
    /// name no headless command go through `execute`.
    pub(super) fn run_command_line(&mut self, line: &str) {
        match operandi_core::command_line::command_for_line(&mut self.editor, line) {
            Some(Ok(Some(command))) => self.run(command),
            Some(Ok(None)) => self.status = "Ready".into(),
            Some(Err(e)) => self.status = e,
            None => self.execute(line),
        }
    }
    /// Renames the active item through `item.name` (UIC-164/165). A name
    /// with braces in it (the line's grouping marks) goes straight in.
    pub(super) fn rename_active(&mut self, name: String) {
        if name.contains(['{', '}']) {
            self.run(Command::Rename(name));
        } else {
            self.run_command_line(&format!("item.name {{{name}}}"));
        }
    }
    /// Returns false for legacy commands, leaving their established parser intact.
    pub(super) fn canonical_command(&mut self, text: &str) -> bool {
        let trimmed = text.trim();
        if let Some(payload) = canonical_pen_payload(trimmed) {
            match PenInput::parse(payload) {
                Ok(input) => self.run(Command::Pen(input)),
                Err(e) => self.status = e,
            }
            return true;
        }
        // Edit tab Subdivide group, Center and Freeze (subdivide_ui.rs).
        if self.subdivide_canonical(trimmed) {
            return true;
        }
        // M3 Vertex panel verbs, their help topics and tool.set DragWeld.
        if self.vertex_canonical(trimmed) {
            return true;
        }
        // pref.value preferences (prefs_commands.rs).
        if self.pref_value_command(trimmed) {
            return true;
        }
        // view3d.* Viewport Properties settings (viewport_properties.rs).
        if self.view3d_canonical(trimmed) {
            return true;
        }
        // Backdrop item commands and their help topics (backdrop_ui.rs).
        if self.item_list_canonical(trimmed) {
            return true;
        }
        if self.backdrop_canonical(trimmed) {
            return true;
        }
        // image.reload / image.list (shading_panel.rs).
        if self.image_canonical(trimmed) {
            return true;
        }
        // scene.new/save/saveAs/close/closeAll/importReference (file_commands.rs).
        if self.file_canonical(trimmed) {
            return true;
        }
        // preset.save / preset.load and their help topics (preset_ui.rs).
        if self.preset_canonical(trimmed) {
            return true;
        }
        // Selection Sets verbs and help topics (selection_set_dialog.rs).
        if self.selection_set_canonical(trimmed) {
            return true;
        }
        let first = trimmed
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(
            first.as_str(),
            "help"
                | "command.list"
                | "tool.set"
                | "tool.handlestyle"
                | "tool.attr"
                | "tool.reset"
                | "tool.apply"
                | "tool.drop"
                | "tool.doapply"
                | "attr.formpopover"
                | "operandi.pen.create"
                | "poly.setpart"
                | "poly.setmaterial"
                | "poly.unify"
                | "poly.boolean"
                | "mesh.retopo"
                | "sculpt.remesh"
                | "operandi.remesh"
                | "operandi.solidsketch.strip"
                | "poly.soliddrill"
                | "poly.drill"
                | "mesh.cleanup"
                | "chamfer.edit"
                | "poly.haul"
                | "operandi.sketch.sweep"
                | "@ppi_polyscale_offset.py"
                | "@ppi_polyscale_offset_detach.py"
                | "@ppi_flatten_ngons.py"
                | "@ppi_vert_tear.py"
                | "poly.split"
                | "poly.quadruple"
                | "poly.spinquads"
                | "poly.align"
                | "poly.triple"
                | "poly.flip"
                | "poly.remove"
                | "poly.merge"
                | "poly.collapse"
                | "poly.make"
                | "vert.set"
                | "scale.absolute"
                | "edge.addloop"
                | "edge.addpoint"
        ) && !EdgeCommand::ALL
            .iter()
            .any(|c| c.verb().eq_ignore_ascii_case(&first))
        {
            return false;
        }
        let args = match tokens(trimmed) {
            Ok(v) => v,
            Err(e) => {
                self.status = e.into();
                return true;
            }
        };
        match first.as_str() {
            "operandi.sketch.sweep" => {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct WorldPath {
                    path: Vec<DVec3>,
                }
                let json = trimmed[first.len()..].trim();
                match serde_json::from_str::<WorldPath>(json) {
                    Ok(path) => self.run(Command::SketchExtrudeOnce(path.path)),
                    Err(_) => fail(
                        self,
                        "Use operandi.sketch.sweep {\"path\":[[0,0,0],[2,0,0]]}",
                    ),
                }
            }
            "poly.haul" => {
                if args.len() != 2 {
                    fail(self, "Use poly.haul offset:<metres>");
                } else if let Some((key, value)) = args[1].split_once(':') {
                    if key.eq_ignore_ascii_case("offset") {
                        match numeric::parse_quantity(value, numeric::Quantity::Distance) {
                            Ok(offset) => self.run(Command::PolyHaulOnce(offset)),
                            Err(error) => fail(self, &format!("Invalid Poly Haul Offset: {error}")),
                        }
                    } else {
                        fail(self, "Use poly.haul offset:<metres>");
                    }
                } else {
                    fail(self, "Use poly.haul offset:<metres>");
                }
            }
            "chamfer.edit" => self.command_chamfer_edit(&args),
            "scale.absolute" => self.command_absolute_scale(&args[1..]),
            "vert.set" => {
                if args.len() == 1 {
                    self.open_set_position();
                } else {
                    match set_position_arguments(&args[1..]) {
                        Ok((axis, value, space)) => {
                            self.assign_position(axis, value, space, None);
                        }
                        Err(error) => fail(self, &error),
                    }
                }
            }
            "poly.make" => {
                // `type:auto` is the map's name for the positional mode.
                let mode = args.get(1).map(|a| {
                    a.split_once(':')
                        .filter(|(k, _)| k.eq_ignore_ascii_case("type"))
                        .map_or(a.as_str(), |(_, v)| v)
                });
                if args.len() != 2 || !mode.is_some_and(|m| m.eq_ignore_ascii_case("auto")) {
                    fail(
                        self,
                        "Use poly.make auto; other Make modes and arguments are unavailable",
                    );
                } else {
                    self.polygon_make();
                }
            }
            "poly.flip" => {
                if args.len() != 1 {
                    fail(self, "poly.flip takes no arguments");
                } else {
                    self.run(Command::FlipPolygons);
                }
            }
            "operandi.solidsketch.strip" => {
                if args.len() != 1 {
                    fail(self, "operandi.solidSketch.strip takes no arguments");
                } else if self.run_ok(Command::SolidSketchStrip) {
                    self.status = "Solid Sketch data removed".into();
                }
            }
            "help" => self.command_help(&args),
            "command.list" => self.command_list(&args[1..]),
            "tool.set" => self.command_tool_set(&args),
            "tool.handlestyle" => self.command_handle_style(&args[1..]),
            "tool.attr" => self.command_tool_attr(&args),
            "tool.reset" => {
                if args.len() > 2 {
                    fail(self, "Use tool.reset [owner]");
                    return true;
                }
                let active = self.command_active().map(|i| IDS[i].0);
                if args.len() == 2
                    && !(active.is_some_and(|a| a.eq_ignore_ascii_case(&args[1]))
                        || args[1].eq_ignore_ascii_case("poly.smshift")
                            && matches!(active, Some("poly.smshift" | "Thicken")))
                {
                    fail(self, "Reset owner does not match active tool");
                    return true;
                }
                // MN-05.1: every tool resets through one command
                // (`tool_reset.rs` keeps each tool's own reset where it
                // had one).
                if self.editor.session.has_active_task() {
                    self.run(Command::ResetToolAttributes);
                } else {
                    fail(self, "No tool is active, so there is nothing to reset");
                }
            }
            "mesh.cleanup" => self.mesh_cleanup_command(&args[1..]),
            "poly.merge" if args.len() == 1 => self.polygon_merge(),
            "poly.merge" => fail(self, "poly.merge takes no arguments; use help poly.merge"),
            "poly.collapse" if args.len() == 1 => self.polygon_collapse(),
            "poly.collapse" => fail(
                self,
                "poly.collapse takes no arguments; nothing was collapsed. Use help poly.collapse",
            ),
            "poly.triple" if args.len() == 1 => self.polygon_triple(),
            "poly.triple" => fail(self, "poly.triple takes no arguments; use help poly.triple"),
            "poly.remove" if args.len() == 1 => self.remove_polygons(),
            "poly.remove" => fail(
                self,
                "poly.remove takes no arguments; nothing was removed. Use help poly.remove",
            ),
            "edge.addloop" => self.edge_insert_command(edge_panel::EdgeClick::AddLoop, &args[1..]),
            // `edge.join average:true` (or `edge.join 1`) is Join Averaged.
            "edge.join" if args.len() > 1 => {
                use operandi_core::command_args::{bind, boolean};
                match bind("edge.join", &args[1..], &["average"])
                    .and_then(|b| b[0].map(|v| boolean("average", v)).transpose())
                {
                    Ok(Some(true)) => self.run_edge_command(EdgeCommand::JoinAveraged),
                    Ok(_) => self.run_edge_command(EdgeCommand::Join),
                    Err(e) => fail(self, &e),
                }
            }
            verb if EdgeCommand::ALL
                .iter()
                .any(|c| c.verb().eq_ignore_ascii_case(verb)) =>
            {
                let command = *EdgeCommand::ALL
                    .iter()
                    .find(|c| c.verb().eq_ignore_ascii_case(verb))
                    .unwrap();
                if args.len() == 1 {
                    self.run_edge_command(command);
                } else {
                    fail(self, &format!("{} takes no arguments", command.verb()));
                }
            }
            "edge.addpoint" => {
                self.edge_insert_command(edge_panel::EdgeClick::AddPoint, &args[1..])
            }
            "poly.quadruple" if args.len() == 1 => self.polygon_quadruple(),
            "poly.quadruple" => fail(self, "poly.quadruple takes no arguments"),
            "poly.align" if args.len() == 1 => self.polygon_align(),
            "poly.align" => fail(self, "poly.align takes no arguments; nothing was aligned"),
            "poly.spinquads" if args.len() == 1 => self.polygon_spin(),
            "poly.spinquads" => fail(self, "poly.spinQuads takes no arguments"),
            "poly.boolean" => self.boolean_command(&args[1..]),
            "mesh.retopo" | "sculpt.remesh" | "operandi.remesh" => {
                self.remesh_command(trimmed);
            }
            "poly.soliddrill" => self.drill_command(operandi_core::DrillKind::Solid, &args[1..]),
            "poly.drill" => self.drill_command(operandi_core::DrillKind::Axis, &args[1..]),
            "poly.unify" => {
                // Positional or the map's `force:` name.
                let force = |v: &str| {
                    let v = v
                        .split_once(':')
                        .filter(|(k, _)| k.eq_ignore_ascii_case("force"))
                        .map_or(v, |(_, v)| v);
                    operandi_core::command_args::boolean("force", v).ok()
                };
                match &args[1..] {
                    [] => self.open_unify(),
                    [v] if force(v).is_some() => self.execute_unify(force(v).unwrap(), None),
                    _ => fail(self, "Use poly.unify [force:true|false]"),
                }
            }
            // SO-U01: the bare action only opens the dialog; no script host.
            "@ppi_flatten_ngons.py" if args.len() == 1 => self.flatten_ngons(),
            "@ppi_flatten_ngons.py" => fail(
                self,
                "@ppi_flatten_ngons.py takes no arguments; nothing changed",
            ),
            "@ppi_polyscale_offset.py" if args.len() == 1 => self.open_scale_offset(),
            // `@ppi_PolyScale_Offset.py detach` arrives here renamed
            // (command_names.rs).
            "@ppi_polyscale_offset_detach.py" if args.len() == 1 => self.open_scale_offset_detach(),
            "@ppi_polyscale_offset_detach.py" => fail(
                self,
                "@ppi_PolyScale_Offset_Detach.py takes no arguments; nothing changed",
            ),
            // The map's Tear button is this script command; it starts Tear.
            "@ppi_vert_tear.py" if args.len() == 1 => {
                let row = id("vert.tear").expect("vert.tear is a tool id");
                self.execute(IDS[row].1);
            }
            "@ppi_vert_tear.py" => fail(self, "@ppi_vert_tear.py takes no arguments"),
            "@ppi_polyscale_offset.py" => fail(
                self,
                "@ppi_PolyScale_Offset.py takes no arguments; nothing changed. Use help @ppi_PolyScale_Offset.py",
            ),
            "poly.split" if args.len() == 1 => self.polygon_split(),
            "poly.split" => fail(
                self,
                "poly.split takes no arguments; select two opposite vertices",
            ),
            "poly.setmaterial" => match &args[1..] {
                [] => self.open_set_material(),
                [name] if !name.is_empty() && name != "?" => {
                    // A leading `name:` is the map's argument name.
                    let name = match name.split_once(':') {
                        Some((key, value))
                            if key.eq_ignore_ascii_case("name") && !value.is_empty() =>
                        {
                            value
                        }
                        _ => name.as_str(),
                    };
                    self.assign_material(name.to_owned(), None);
                }
                _ => fail(
                    self,
                    "Use poly.setMaterial {Material Name}; exactly one name is supported",
                ),
            },
            "poly.setpart" => match set_part_argument(&args[1..]) {
                Ok(None) => self.open_set_part(),
                Ok(Some(name)) => {
                    self.assign_part(name, None);
                }
                Err(e) => fail(self, e),
            },
            "attr.formpopover" => {
                if let Err(e) = self.form_popover(&args[1..]) {
                    fail(self, &e);
                }
            }
            "tool.apply" | "tool.doapply" | "tool.drop" if args.len() == 1 => {
                self.run(if first != "tool.drop" {
                    Command::Apply
                } else {
                    Command::Drop
                })
            }
            _ => fail(self, "Unsupported command or arguments; use help"),
        }
        true
    }
    /// `chamfer.edit offset:… segments:… sourceSegs:… depth:…`: one-step Chamfer Edit.
    fn command_chamfer_edit(&mut self, args: &[String]) {
        match chamfer_edit_arguments(&args[1..]) {
            Ok(p) => self.run(Command::ChamferOnce(p)),
            Err(e) => fail(self, e),
        }
    }

    /// `command.list [area|word]`: the registry (UIC-160). Bare, a count per
    /// area; with a filter, the matching names. Full lines go to stderr.
    fn command_list(&mut self, args: &[String]) {
        use operandi_core::command_registry::{API_VERSION, Area};
        if args.len() > 1 {
            fail(self, "Use command.list [area|word]");
            return;
        }
        let filter = args.first().map_or("", String::as_str);
        // The run-time registry: built-ins, the app's own and kits'.
        let found = self.commands.matching(filter);
        for e in &found {
            eprintln!("[{}] {} — {}", e.area.label(), e.usage(), e.help);
        }
        self.status = if filter.is_empty() {
            let areas: Vec<String> = Area::ALL
                .iter()
                .map(|&a| {
                    let n = found.iter().filter(|e| e.area == a).count();
                    format!("{} {n}", a.label())
                })
                .collect();
            format!(
                "{} commands (API {API_VERSION}): {}. Use command.list <area|word>, then help <name>.",
                found.len(),
                areas.join(", ")
            )
        } else if found.is_empty() {
            format!("No command matches {filter}")
        } else {
            let names: Vec<&str> = found.iter().map(|e| e.name).collect();
            format!("{} commands: {}", names.len(), names.join(", "))
        };
    }

    /// `help [topic]`: list help topics or show one.
    fn command_help(&mut self, args: &[String]) {
        // A multi-word topic is an action phrase ("help Top View").
        if args.len() > 2
            && let Some(command) = self.commands.find(&args[1..].join(" "))
        {
            self.status = command.entry.describe();
            return;
        }
        if args.len() > 2 {
            fail(
                self,
                "Use help [tool.set|tool.attr|tool.reset|tool.apply|tool.drop|operandi.pen.create|poly.setPart|poly.setMaterial|poly.unify|mesh.cleanup|@ppi_PolyScale_Offset.py|@ppi_flatten_ngons.py|poly.split|poly.quadruple|poly.spinQuads|poly.align|poly.triple|poly.flip|poly.remove|poly.merge|poly.collapse|poly.make|chamfer.edit|poly.haul|operandi.sketch.sweep|vert.set|poly.reduct|edge.addLoop|edge.addPoint|vert.new|vert.split|vert.splitEdge|vert.align|vert.merge|vert.join|vert.collapse|vert.remove|vert.removeEdge|vert.reorder|vert.syncOrder|vert.weld|DragWeld|view3d.showVertices|backdrop.add|backdrop.image|backdrop.attr|backdrop.fit|backdrop.size|select.editSet|select.useSet|select.deleteSet|select.convertGroup|@ppi_PolyScale_Offset_Detach.py|@ppi_vert_tear.py]",
            );
        } else if args.len() == 2
            && ![
                "tool.set",
                "tool.attr",
                "tool.reset",
                "tool.apply",
                "tool.drop",
                "operandi.pen.create",
                "pen",
                "poly.setpart",
                "poly.setmaterial",
                "poly.unify",
                "mesh.cleanup",
                "@ppi_polyscale_offset.py",
                "@ppi_polyscale_offset_detach.py",
                "@ppi_flatten_ngons.py",
                "@ppi_vert_tear.py",
                "poly.split",
                "poly.quadruple",
                "poly.spinquads",
                "poly.align",
                "poly.triple",
                "poly.flip",
                "poly.remove",
                "poly.merge",
                "poly.collapse",
                "poly.make",
                "chamfer.edit",
                "poly.haul",
                "operandi.sketch.sweep",
                "vert.set",
                "poly.reduct",
                "edge.addloop",
                "edge.addpoint",
            ]
            .iter()
            .any(|s| s.eq_ignore_ascii_case(&args[1]))
            && !EdgeCommand::ALL
                .iter()
                .any(|c| c.verb().eq_ignore_ascii_case(&args[1]))
        {
            // Every other command's usage and help come from the registry
            // (UIC-160).
            match self.commands.find(&args[1]) {
                Some(command) => self.status = command.entry.describe(),
                None => fail(self, "Unknown help topic; use help or command.list"),
            }
        } else {
            let topic = args
                .get(1)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if let Some(&command) = EdgeCommand::ALL
                .iter()
                .find(|c| c.verb().eq_ignore_ascii_case(&topic))
            {
                let action = edge_panel::edge_action(command);
                let binding = self
                    .bindings
                    .get(action)
                    .filter(|b| !b.is_empty())
                    .map_or("no shortcut assigned".into(), |b| format!("shortcut {b}"));
                self.status = format!(
                    "Help: {} (no arguments) runs {} on the selected edges in Edge mode, as one undo step. Edge panel row {} ({binding}).",
                    command.verb(),
                    command.label(),
                    command.label()
                );
                return;
            }
            if topic == "chamfer.edit" {
                let binding = self
                    .bindings
                    .get("Chamfer Edit")
                    .filter(|b| !b.is_empty())
                    .map_or("no shortcut assigned".into(), |b| format!("shortcut {b}"));
                self.status = format!(
                    "Help: chamfer.edit offset:0.1 segments:2 sourceSegs:1 depth:0.5 (depth 0–1, or 50%) applies a rounded strip. Polygon panel Chamfer Edit ({binding}) or tool.set chamfer.edit on opens the live tool; use its Offset handle or numeric fields, then tool.drop. One selected simple quad strip, identity transform and neutral context required; unsupported maps and shapes are refused. Drag sensitivity and the rounded shape may change."
                );
                return;
            }
            if topic == "poly.haul" {
                let binding = self
                    .bindings
                    .get("Poly Haul")
                    .filter(|b| !b.is_empty())
                    .map_or("no shortcut assigned".into(), |b| format!("shortcut {b}"));
                self.status = format!(
                    "Help: poly.haul offset:0.2 applies Poly Haul once. Polygon panel Poly Haul ({binding}) or tool.set poly.haul on starts the live tool; drag over a selected polygon (up pulls out, down pushes in) or enter Offset, then Drop. Offset mode moves the selected polygons along their normal without adding edges; default action center only. Drag sensitivity may change."
                );
                return;
            }
            if topic == "operandi.sketch.sweep" {
                let binding = self
                    .bindings
                    .get("Sketch Extrude")
                    .filter(|b| !b.is_empty())
                    .map_or("no shortcut assigned".into(), |b| format!("shortcut {b}"));
                self.status = format!(
                    "Help: operandi.sketch.sweep {{\"path\":[[0,0,0],[2,0,0]]}} applies an explicit world path. Polygon panel Sketch Extrude ({binding}, candidate Alt+B) or tool.set poly.sweep on starts a live stroke that extrudes the selected polygons along the drawn path, one span per Precision step. Fixed-section Sweep only; drag sensitivity may change. Drag to draw, then Drop or Cancel."
                );
                return;
            }
            if topic == "poly.align" {
                let binding = self
                    .bindings
                    .get("Polygon Align")
                    .filter(|b| !b.is_empty())
                    .map_or("no shortcut assigned".into(), |b| format!("shortcut {b}"));
                self.status = format!(
                    "Help: poly.align (Polygon panel Align; {binding}; remap \"Polygon Align\" in the Input Map) makes the selected polygons face the same way, in Polygon mode with no active tool. The first selected polygon is the reference; with one polygon selected, its whole connected piece is aligned. Orientation spreads across shared manifold edges; the flipped count is shown. Nonorientable or authored-normal selections are refused. It takes no arguments."
                );
                return;
            }
            if topic == "@ppi_flatten_ngons.py" {
                let binding = self
                    .bindings
                    .get("Flatten Ngons")
                    .filter(|b| !b.is_empty())
                    .map_or("no shortcut assigned (unassigned by default)".into(), |b| {
                        format!("shortcut {b}")
                    });
                self.status = format!(
                    "Help: @ppi_flatten_ngons.py (Polygon panel Flatten Ngons; {binding}; remap \"Flatten Ngons\" in the Input Map) flattens eligible nonplanar surface polygons throughout the active visible mesh, starting with an explicitly selected polygon. It uses a best-fit plane and refuses unsupported or uncertain geometry without partial changes. Polygon mode, no active tool; takes no arguments."
                );
                return;
            }
            if topic == "@ppi_polyscale_offset.py" {
                let binding = self
                    .bindings
                    .get("Scale Offset")
                    .filter(|b| !b.is_empty())
                    .map_or("no shortcut assigned (unassigned by default)".into(), |b| {
                        format!("shortcut {b}")
                    });
                self.status = format!(
                    "Help: {SCALE_OFFSET_COMMAND} (Polygon panel Scale Offset...; {binding}; remap \"Scale Offset\" in the Input Map) opens Enter Inset Amount. The selected polygons of the active visible mesh shrink by Inset in their own planes, every edge moved inward (negative grows); their points move and the neighbours follow, nothing is added. {SCALE_OFFSET_COMMAND} detach does the same on new points, leaving the polygons unconnected, and clears the selection. 0 changes nothing. Polygon mode with no active tool. Collapsing or folding results are refused."
                );
                return;
            }
            if topic == "poly.make" {
                let binding = self
                    .bindings
                    .get("Make")
                    .filter(|b| !b.is_empty())
                    .map_or("no shortcut assigned".into(), |b| format!("shortcut {b}"));
                self.status = format!(
                    "Help: poly.make auto (Polygon panel Make; {binding}, Edge mode; remap \"Make\" in the Input Map) builds one quad from two selected boundary edges of the active visible mesh, in Edge mode with no active tool. Two edges meeting at a corner get their missing fourth point (an existing opening or coincident point is reused); two separate edges are joined. The quad is wound like its neighbours and takes the tags of the polygon next to the first selected edge; the edge selection is kept. Filled, ambiguous or folded results are refused. Only the auto argument is supported."
                );
                return;
            }
            if topic == "poly.collapse" {
                let binding = self
                    .bindings
                    .get("Polygon Collapse")
                    .filter(|b| !b.is_empty())
                    .map_or("no shortcut assigned".into(), |b| format!("shortcut {b}"));
                self.status = format!(
                    "Help: poly.collapse (Polygon panel Collapse; {binding}, Polygon mode; remap \"Polygon Collapse\" in the Input Map) removes the selected polygons of the active visible mesh without leaving a hole, in Polygon mode with no active tool: each connected piece of the selection becomes one new point at the mean of its points and the neighbouring polygons close up. Pinching or folding results and differing point-map values are refused. It takes no arguments."
                );
                return;
            }
            self.status = match topic.as_str() {
                "" => "Help: topics: tool.set, tool.attr, tool.reset, tool.apply, tool.drop, operandi.pen.create, poly.setPart, poly.setMaterial, poly.unify, mesh.cleanup, @ppi_PolyScale_Offset.py, @ppi_flatten_ngons.py, poly.split, poly.quadruple, poly.spinQuads, poly.align, poly.triple, poly.flip, vert.set, poly.remove, poly.merge, poly.collapse, poly.make, chamfer.edit, poly.haul, operandi.sketch.sweep, poly.reduct, edge.addLoop, edge.addPoint, vert.new, vert.split, vert.splitEdge, vert.align, vert.merge, vert.join, vert.collapse, vert.remove, vert.removeEdge, vert.reorder, vert.syncOrder, vert.weld, DragWeld, view3d.showVertices, select.editSet, select.useSet, select.deleteSet, select.convertGroup, backdrop.add, backdrop.fit, @ppi_PolyScale_Offset_Detach.py, @ppi_vert_tear.py. Old names (selset.*, view.showVertices, edge.spin, tweak…) still work and are deprecated. Use help <topic>; no scripting or history browser. Interactive Polygon Pen: tool.set prim.pen on. command.list lists every command.",
                "poly.reduct" => "Help: poly.reduct (Polygon panel row Polygon Reduction; no default shortcut, remap \"Polygon Reduction\" in the Input Map). tool.set poly.reduct on arms it with Number 0; a stationary click in the viewport initializes from the active visible mesh in Polygon mode from the selected polygons and shows the mesh's current polygon count. Reduction triangulates the selection and removes edges cheapest-first by quadric error, keeping the selection border, mesh boundary and material borders. tool.attr poly.reduct number <integer|?> previews that count from the original (boundaryWeight, materialWeight and legacy are unavailable); Apply records one edit and stays armed, Drop records and exits, Cancel discards only the preview. Boundary Weight, Material Border Weight and Legacy Mode are unavailable in this basic release.",
                "poly.triple" => "Help: poly.triple (Polygon panel Triple, default Shift+T) splits each selected simple planar polygon of the active visible mesh into triangles over its existing vertices, in Polygon mode with no active tool. Existing triangles are kept; an all-triangle selection changes nothing. Nonplanar, self-intersecting, degenerate and Polyline selections are refused. It takes no arguments.",
                "poly.remove" => "Help: poly.remove (no arguments) removes the selected polygons of the active visible mesh in Polygon mode with no active tool. All vertices are kept, including ones left unused; the mesh item remains even with no polygons. No cleanup or capping. The Polygons panel Commands › Remove row and the Input Map action Remove (default Shift+Backspace, Polygon mode) run the same operation. Arguments and queries are unsupported.",
                "edge.addloop" => "Help: edge.addLoop [position:<0–1 or %>] adds a loop through the polygon band crossing each selected edge (Edge mode), at the given position (default 50%); the loop ends at a boundary or triangle. Panel row Add Loop or tool.set edge.addLoop on arms click mode: click an edge (Ctrl: middle).",
                "edge.addpoint" => "Help: edge.addPoint [position:<0–1 or %>] splits each selected edge (Edge mode) with a new point at the given position (default 50%). Panel row Add Point or tool.set edge.addPoint on arms click mode: click an edge (Ctrl: middle).",
                "poly.quadruple" => "Help: poly.quadruple converts each selected polygon laid down as two opposing rows of vertices (an even corner count) into a row of quads, joining opposite row corners.",
                "poly.spinquads" => "Help: poly.spinQuads immediately rotates the shared edge of two selected adjacent polygons with the same number of edges one step around their outline; Spin Quad uses Alt+V by default and supports remapping.",
                "mesh.cleanup" => "Help: mesh.cleanup requires twelve explicit boolean options; only floatingVertex:true is supported. Use the Mesh Cleanup dialog for options.",
                "poly.unify" => "Help: poly.unify opens options; poly.unify false or true removes polygons that duplicate another (same vertices, either winding), keeping the lowest one of each group. Force also removes duplicates whose tags or corner data differ.",
                "poly.split" => "Help: poly.split immediately connects each consecutive pair of selected vertices that are corners of one polygon with a new edge, in Vertex mode. Already connected pairs are skipped; a cut that would leave the polygon is refused.",
                "poly.setmaterial" => "Help: poly.setMaterial {Warm Clay} tags the selected polygons (every polygon when none are selected) with a material, creating it if new; {Default} clears the tag. Bare poly.setMaterial opens the dialog for color and shading too.",
                "poly.setpart" => "Help: poly.setPart <name> (or name:<name>) assigns a part name to the selected polygons of the active visible mesh (every polygon when none are selected) in Polygon mode with no active tool. Group spaces with quotes or braces: poly.setPart {Road Part}. Bare poly.setPart opens the Polygon Set Part dialog. Blank names, clearing, queries and other arguments are unsupported.",
                "poly.flip" => "poly.flip: reverse polygon winding (Sub-D faces too; not curves); no active tool or authored normals.",
                "vert.set" => "Help: vert.set opens Set Vertex Position; vert.set axis:z value:2m [workPlane:false] [worldSpace:false] (named or by position) gives every affected point that coordinate.",
                "poly.merge" => "Help: poly.merge (Polygon panel Merge, no default shortcut) merges each edge-connected region of two or more selected polygons of the active visible mesh into one polygon along its outer boundary, in Polygon mode with no active tool. Internal edges are removed and points left without edges are deleted; the merged polygon takes the lowest selected polygon's tags. Regions with holes or inconsistent winding are refused. It takes no arguments.",
                "tool.set" => "Help: tool.set <preset> <on|off|?>. IDs: prim.pen|cube|sphere|cylinder|ellipsoid|cone|toroid|capsule|arc|tube|makeVertex, poly.bevel|extrude|smshift|spikey|haul|sweep|reduct|inset|shift|loopSlice|knife, chamfer.edit, Thicken, Transform, TransformMove|Rotate|Scale, edge.bridge|extend|extrude|slide|crease|bevel|chamfer|relax|knife|addLoop|addPoint, vert.bevel|extrude|merge|tear|slide, *.Slide (vert.slide or edge.slide by mode), DragWeld, ElementMove, xfrm.push|smooth. Old ids (tweak, prim.torus, vert.create, vert.mergeTool, vert.weld, deform.*, slice.*) still work and are deprecated. Names: box or cube, prim.cube; smooth shift, poly.smshift.",
                "tool.attr" => "Help: tool.attr poly.spikey factor <scalar|?>; tool.attr poly.smshift <shift|scale|maxAngle|thicken|sharp|angleWeight|bothSides> <value|?>. Shift is distance, scale factor (1 = 100 %) or %, maxAngle radians (or 30deg), others booleans. Percents are 0–1 (or 50%) and angles radians, as in scripts. Bridge: tool.attr edge.bridge <segments|twist|mode|remove|flip|orient|tension> <value|?>; tension is factor or percent. Edge Bevel/Chamfer: tool.attr edge.bevel|edge.chamfer <option> <value|?>, options as in the panel (stopAtEdge, useMat, matName…). Polygon Reduction: tool.attr poly.reduct <number|boundaryWeight|materialWeight|method>. Edge Extend: tool.attr edge.extend <offX|offY|offZ|rotX|rotY|rotZ|sclX|sclY|sclZ|segs|inset|shift|moveHandle|rotateHandle|scaleHandle|localHandle|planeHandle none/move/scale|hauling global/local|uvs none/u/v>. Other properties unsupported.",
                "tool.reset" => "Help: tool.reset [owner] resets the active tool's values to the factory ones; the tool stays active (Edit > Reset Tool Attributes, Ctrl+D).",
                "tool.apply" => "Help: tool.apply applies the active tool using its existing lifecycle. tool.doApply is the same.",
                "tool.drop" => "Help: tool.drop drops the active tool using its existing lifecycle.",
                "@ppi_polyscale_offset_detach.py" => "Help: @ppi_PolyScale_Offset_Detach.py opens Enter Inset Amount for Scale Offset on new, unconnected points (the old spelling @ppi_PolyScale_Offset.py detach still works). It takes no arguments.",
                "@ppi_vert_tear.py" => "Help: @ppi_vert_tear.py starts the Tear tool (Vertex panel Tear), like tool.set vert.tear on. It takes no arguments.",
                _ => "Help: operandi.pen.create <JSON> (alias pen <JSON>) creates Pen geometry in one step from four XY positions. Interactive Polygon Pen: tool.set prim.pen on; finish with tool.drop. See command reference for the numeric schema.",
            }.into();
            if topic == "poly.unify" {
                let binding = self
                    .bindings
                    .get("Unify")
                    .filter(|s| !s.is_empty())
                    .map(String::as_str)
                    .unwrap_or("unassigned");
                self.status = format!(
                    "Help: poly.unify opens options; poly.unify false or true removes polygons that duplicate another (same vertices, either winding), keeping the lowest one of each group. Force also removes duplicates whose tags or corner data differ. Current shortcut: {binding}."
                );
            }
        }
    }

    /// `tool.set <id> on|off|?`: start, stop or query a tool.
    fn command_tool_set(&mut self, args: &[String]) {
        // Snaps and Precision presets: `tool.set SnapDrag 1 snap:true` (§6);
        // `snap:true` is what the preset does anyway (snapping on).
        if let Some(id) = args.get(1)
            && SnapPreset::named(id.split_once(':').map_or(id.as_str(), |(_, v)| v)).is_some()
        {
            let id = id.split_once(':').map_or(id.as_str(), |(_, v)| v);
            let mut mode = "on";
            for arg in &args[2..] {
                match arg.split_once(':') {
                    Some((key, _)) if key.eq_ignore_ascii_case("snap") => {}
                    Some((key, value)) if key.eq_ignore_ascii_case("mode") => mode = value,
                    None => mode = arg,
                    _ => return fail(self, "Use tool.set <preset> on|off [snap:true]"),
                }
            }
            match mode.to_ascii_lowercase().as_str() {
                "on" | "1" | "true" => {
                    self.tool_set_snap_preset(id, true);
                }
                "off" | "0" | "false" => {
                    self.tool_set_snap_preset(id, false);
                }
                "?" => {
                    let on = self
                        .snap_preset
                        .is_some_and(|(p, _)| Some(p) == SnapPreset::named(id));
                    self.status = if on { "1" } else { "0" }.into();
                }
                _ => fail(self, "Use tool.set <preset> on|off [snap:true]"),
            }
            return;
        }
        // Binding is core's (command_line::tool_set_args).
        let Some((preset, mode)) = operandi_core::command_line::tool_set_args(&args[1..]) else {
            fail(
                self,
                "Use tool.set <preset> <on|off|?>; bind each argument once",
            );
            return;
        };
        if preset.eq_ignore_ascii_case(background_ui::TOOL_ID) {
            self.const_bg_set(mode);
            return;
        }
        // Solid Sketch: the bare tool (Subdiv Mode Level 2) or the button's
        // `SS Default` preset (Level 1), spec 2 and test 13.
        if preset.eq_ignore_ascii_case("solid.sketch") || preset.eq_ignore_ascii_case("SS Default")
        {
            let on = self.editor.session.solid_sketch.is_some();
            match mode.to_ascii_lowercase().as_str() {
                "on" | "1" if !on => {
                    self.start_solid_sketch(preset.eq_ignore_ascii_case("SS Default"))
                }
                "on" | "1" => {}
                "?" => self.status = if on { "1" } else { "0" }.into(),
                "off" | "0" if on => self.run(Command::Drop),
                "off" | "0" => self.status = "Solid Sketch is not active".into(),
                _ => fail(self, "Use tool.set solid.sketch on, off or ?"),
            }
            return;
        }
        if preset.eq_ignore_ascii_case("prim.pen") {
            match mode.to_ascii_lowercase().as_str() {
                "on" | "1" => self.start_interactive_pen(),
                "?" => {
                    self.status = if self.editor.session.pen_draft.is_some() {
                        "1"
                    } else {
                        "0"
                    }
                    .into()
                }
                "off" | "0" if self.editor.session.pen_draft.is_some() => self.run(Command::Drop),
                "off" | "0" => self.status = "Pen is not active".into(),
                _ => fail(self, "Use tool.set prim.pen on, off or ?"),
            }
            return;
        }
        // M2 Edge click tools: arm/disarm like their panel rows.
        let edge_tool = match preset.to_ascii_lowercase().as_str() {
            "edge.addloop" => Some(edge_panel::EdgeClick::AddLoop),
            "edge.addpoint" => Some(edge_panel::EdgeClick::AddPoint),
            _ => None,
        };
        if let Some(tool) = edge_tool {
            match mode.to_ascii_lowercase().as_str() {
                "on" | "1" if self.edge_click != Some(tool) => self.toggle_edge_click(tool),
                "on" | "1" => {}
                "off" | "0" if self.edge_click == Some(tool) => self.toggle_edge_click(tool),
                "off" | "0" => {}
                "?" => {
                    self.status = if self.edge_click == Some(tool) {
                        "1"
                    } else {
                        "0"
                    }
                    .into()
                }
                _ => fail(self, "Use tool.set edge.addLoop|edge.addPoint on, off or ?"),
            }
            return;
        }
        let Some(i) = id(preset) else {
            // Modo's id for a tool Operandi starts by phrase (UIC-165).
            if let Some(phrase) = operandi_core::command_registry::tool_phrase(preset) {
                match mode.to_ascii_lowercase().as_str() {
                    "on" | "1" => self.execute(phrase),
                    "off" | "0" if self.editor.session.has_active_task() => self.run(Command::Drop),
                    "off" | "0" => self.status = "Ready".into(),
                    _ => fail(
                        self,
                        &format!("tool.set {preset} answers on or off only for now"),
                    ),
                }
                return;
            }
            fail(self, "Unknown tool; use help tool.set");
            return;
        };
        match mode.to_ascii_lowercase().as_str() {
            "?" => {
                self.status = if self.command_active() == Some(i) {
                    "1"
                } else {
                    "0"
                }
                .into()
            }
            "on" | "1" => self.execute(IDS[i].1),
            "off" | "0" => match self.command_active() {
                None => self.status = "Ready".into(),
                Some(active) if active == i => self.run(Command::Drop),
                _ => fail(self, "Another tool is active; targeted off refused"),
            },
            _ => fail(self, "Unsupported tool.set mode; use on, off or ?"),
        }
    }

    /// `tool.attr` through whichever data-driven panel the active tool
    /// has. `owner` must name the active tool. `None` when no tool is
    /// active or its panel isn't data-driven.
    pub(super) fn tool_attr_any(
        &mut self,
        owner: &str,
        attr: &str,
        value: &str,
    ) -> Option<std::result::Result<Option<String>, String>> {
        if let Some(i) = self.command_active()
            && !IDS[i].0.eq_ignore_ascii_case(owner)
            && !IDS[i].1.eq_ignore_ascii_case(owner)
        {
            return Some(Err(format!(
                "{owner} isn't the active tool ({} is)",
                IDS[i].0
            )));
        }
        // Edge Extend's reference names (offX, rotY, sclZ, planeHandle…)
        // name its panel fields.
        let (attr, value) = if self
            .editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.tool == Tool::EdgeExtend)
        {
            live_fields::edge_extend_attr(attr, value)
        } else if let Some(l) = self
            .editor
            .session
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::VertexBevel)
            && ["level", "roundlevel"].contains(&attr.to_ascii_lowercase().as_str())
        {
            // Vertex Bevel's Round Level (VX-B02) is shown as Segments
            // (2026-11-26); `level` still reads and sets it.
            if value == "?" {
                return Some(Ok(Some(l.params.vertex_round_level.to_string())));
            }
            let level = match value.parse::<u32>() {
                Ok(n) if n <= operandi_core::VERTEX_BEVEL_MAX_ROUND_LEVEL => n,
                _ => {
                    return Some(Err(format!(
                        "Round Level is a whole number 0–{}",
                        operandi_core::VERTEX_BEVEL_MAX_ROUND_LEVEL
                    )));
                }
            };
            (
                "vertex_segments".to_owned(),
                crate::vertex_panel::vertex_bevel_level_to_segments(level).to_string(),
            )
        } else {
            (attr.to_owned(), value.to_owned())
        };
        let (attr, value) = (attr.as_str(), value.as_str());
        macro_rules! try_panels {
            ($($t:ty),*) => {
                $(if let Some(r) = self.tool_attr_form::<$t>(attr, value) {
                    return Some(r);
                })*
            };
        }
        try_panels!(
            bridge_panel::BridgeFields,
            create_utility::UtilityFields,
            edge_panel::EdgeInsertFields,
            draw_tools::DrawFields,
            cone_panel::ConeFields,
            ellipsoid_panel::EllipsoidFields,
            pen_interactive_ui::PenFields,
            solid_sketch_ui::SolidSketchFields,
            primitive_panel::PrimitiveFields,
            reduction_panel::ReductionFields,
            sculpt_ui::SculptFields,
            smooth_shift_panel::SurfaceFields,
            shape_panel::ShapeFields,
            spike_panel::SpikeFields,
            live_fields::LiveFields
        );
        None
    }

    /// The active tool's panel values `(key, value)` in script units, from
    /// whichever data-driven panel it has (the Command History's
    /// `tool.attr` lines). `None` when no such panel is open.
    pub(super) fn tool_attr_values(&self) -> Option<Vec<(String, String)>> {
        macro_rules! values_of {
            ($($t:ty),*) => {
                $(if let Some(v) = self.tool_form_values::<$t>() {
                    return Some(v);
                })*
            };
        }
        values_of!(
            bridge_panel::BridgeFields,
            create_utility::UtilityFields,
            edge_panel::EdgeInsertFields,
            draw_tools::DrawFields,
            cone_panel::ConeFields,
            ellipsoid_panel::EllipsoidFields,
            pen_interactive_ui::PenFields,
            solid_sketch_ui::SolidSketchFields,
            primitive_panel::PrimitiveFields,
            reduction_panel::ReductionFields,
            sculpt_ui::SculptFields,
            smooth_shift_panel::SurfaceFields,
            shape_panel::ShapeFields,
            spike_panel::SpikeFields,
            live_fields::LiveFields
        );
        None
    }

    /// The active tool's panel fields (key, label), for tests.
    #[cfg(test)]
    pub(super) fn tool_attr_fields(&self) -> Vec<(String, String)> {
        macro_rules! fields_of {
            ($($t:ty),*) => {
                $(if let Some(p) = <$t as tool_form::ToolFields>::params(self) {
                    let form = <$t as tool_form::ToolFields>::form(self, &p);
                    return form
                        .fields()
                        .iter()
                        .map(|f| (f.row.key.clone(), f.row.label.clone()))
                        .collect();
                })*
            };
        }
        fields_of!(
            bridge_panel::BridgeFields,
            create_utility::UtilityFields,
            edge_panel::EdgeInsertFields,
            draw_tools::DrawFields,
            cone_panel::ConeFields,
            ellipsoid_panel::EllipsoidFields,
            pen_interactive_ui::PenFields,
            solid_sketch_ui::SolidSketchFields,
            primitive_panel::PrimitiveFields,
            reduction_panel::ReductionFields,
            sculpt_ui::SculptFields,
            smooth_shift_panel::SurfaceFields,
            shape_panel::ShapeFields,
            spike_panel::SpikeFields,
            live_fields::LiveFields
        );
        Vec::new()
    }

    /// `tool.attr <id> <attribute> <value>`: change a live tool's setting.
    fn command_tool_attr(&mut self, args: &[String]) {
        if args.len() != 4 {
            fail(self, "Use tool.attr <owner> <attribute> <value|?>");
            return;
        }
        let owner = args[1].to_ascii_lowercase();
        let attr = args[2].to_ascii_lowercase();
        let value = &args[3];
        let active = self.command_active();
        if owner == background_ui::TOOL_ID {
            self.const_bg_attr(&attr, value);
        } else if owner == "edge.bridge" {
            self.bridge_property(&attr, value, false);
        } else if owner == "edge.bevel" || owner == "edge.chamfer" {
            self.edge_bevel_property(owner == "edge.bevel", &attr, value);
        } else if owner == "poly.reduct" {
            self.reduction_property(&attr, value);
        } else if owner == "solid.sketch"
            && let Some(result) = self.solid_sketch_attr(&attr, value)
        {
            match result {
                Ok(Some(answer)) => self.status = answer,
                Ok(None) => self.status = "Ready".into(),
                Err(e) => self.status = e,
            }
        } else if owner == "poly.spikey" && attr == "factor" {
            let Some(spike) = self.editor.session.spike.as_ref() else {
                fail(self, "Activate Spike first");
                return;
            };
            if value == "?" {
                self.status = spike.requested.to_string();
            } else {
                match numeric::parse_quantity(value, numeric::Quantity::Scalar) {
                    Ok(v) => {
                        let mut staged = self.editor.clone();
                        match staged.dispatch(Command::ConfigureSpike(v)) {
                            Ok(()) => {
                                let succeeded = self.run_ok(Command::ConfigureSpike(v));
                                if succeeded {
                                    self.sync_numeric_value("spi_strength", v, "");
                                }
                            }
                            Err(e) => self.status = e,
                        }
                    }
                    Err(e) => self.status = e,
                }
            }
        } else if owner == "poly.smshift"
            && [
                "shift",
                "scale",
                "maxangle",
                "thicken",
                "sharp",
                "angleweight",
                "bothsides",
            ]
            .contains(&attr.as_str())
        {
            if !matches!(active.map(|i| IDS[i].0), Some("poly.smshift" | "Thicken")) {
                fail(self, "Activate Smooth Shift or Thicken first");
                return;
            }
            let mut p = self
                .editor
                .session
                .live
                .as_ref()
                .unwrap()
                .params
                .smooth_shift;
            if value == "?" {
                self.status = match attr.as_str() {
                    "shift" => format!("{} m", p.offset),
                    "scale" => p.scale.to_string(),
                    "maxangle" => format!("{} deg", p.angle),
                    "thicken" => (p.thicken as u8).to_string(),
                    "sharp" => (p.sharp as u8).to_string(),
                    "angleweight" => (p.angle_weight as u8).to_string(),
                    _ => (p.both_sides as u8).to_string(),
                };
                return;
            }
            let parsed = match attr.as_str() {
                "shift" => numeric::parse_quantity(value, numeric::Quantity::Distance)
                    .map(|v| p.offset = v),
                // Script units: radians unless a unit is typed (map angle).
                "maxangle" => {
                    operandi_core::command_names::script_angle(value).map(|v| p.angle = v)
                }
                "scale" => {
                    if value.contains('%') {
                        numeric::parse_quantity(value, numeric::Quantity::Percent)
                            .map(|v| p.scale = v / 100.)
                    } else {
                        numeric::parse_quantity(value, numeric::Quantity::Scalar)
                            .map(|v| p.scale = v)
                    }
                }
                _ => match value.to_ascii_lowercase().as_str() {
                    "1" | "true" | "on" => Ok(true),
                    "0" | "false" | "off" => Ok(false),
                    _ => Err("Use true/on/1 or false/off/0".into()),
                }
                .map(|v| match attr.as_str() {
                    "thicken" => p.thicken = v,
                    "sharp" => p.sharp = v,
                    "angleweight" => p.angle_weight = v,
                    _ => p.both_sides = v,
                }),
            };
            match parsed {
                Ok(()) => {
                    let mut staged = self.editor.clone();
                    if let Err(e) = staged.dispatch(Command::ConfigureSmoothShift(p)) {
                        self.status = e;
                    } else if self.confirm_fields() {
                        let mut current = self
                            .editor
                            .session
                            .live
                            .as_ref()
                            .unwrap()
                            .params
                            .smooth_shift;
                        match attr.as_str() {
                            "shift" => current.offset = p.offset,
                            "scale" => current.scale = p.scale,
                            "maxangle" => current.angle = p.angle,
                            "thicken" => current.thicken = p.thicken,
                            "sharp" => current.sharp = p.sharp,
                            "angleweight" => current.angle_weight = p.angle_weight,
                            _ => current.both_sides = p.both_sides,
                        }
                        let succeeded = self.run_ok(Command::ConfigureSmoothShift(current));
                        if succeeded {
                            match attr.as_str() {
                                "shift" => {
                                    self.sync_numeric_value("sst_offset", current.offset, " m")
                                }
                                "scale" => {
                                    self.sync_numeric_value("sst_scale", current.scale * 100., " %")
                                }
                                "maxangle" => {
                                    self.sync_numeric_value("sst_angle", current.angle, " °")
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Err(e) => self.status = e,
            }
        } else {
            // Any other data-driven Tool Properties field (UIC-164/165).
            match self.tool_attr_any(&owner, &attr, value) {
                Some(Ok(Some(answer))) => self.status = answer,
                Some(Ok(None)) => self.status = "Ready".into(),
                Some(Err(e)) => self.status = e,
                None => fail(self, "Unknown owner or attribute; use help tool.attr"),
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Command conventions on vert.set: named, positional, choice index,
    /// on/off booleans, units, and unknown choices refused with the list.
    #[test]
    fn vert_set_follows_command_conventions() {
        let parse = |s: &str| set_position_arguments(&tokens(s).unwrap());
        assert_eq!(
            parse("axis:y value:50cm workPlane:off worldSpace:on").unwrap(),
            (PositionAxis::Y, 0.5, PositionSpace::World)
        );
        assert_eq!(
            parse("1 250mm 1 0").unwrap(),
            (PositionAxis::Y, 0.25, PositionSpace::WorkPlane)
        );
        assert_eq!(
            parse("value:2 axis:z").unwrap(),
            (PositionAxis::Z, 2., PositionSpace::Local)
        );
        assert!(parse("axis:w value:0.5").unwrap_err().contains("x, y, z"));
        assert!(parse("axis:y value:50apples").is_err());
        assert!(parse("axis:y").unwrap_err().contains("value"));
        assert!(parse("axis:y value:1 workPlane:on worldSpace:on").is_err());
    }
    #[test]
    fn groups_and_rejects_unclosed() {
        assert_eq!(
            tokens("tool.set preset:{prim.cube} mode:on").unwrap(),
            ["tool.set", "preset:prim.cube", "mode:on"]
        );
        assert!(tokens("tool.set {abc").is_err());
    }
    #[test]
    fn set_part_arguments_bind_once() {
        let parse = |s: &str| set_part_argument(&tokens(s).unwrap()[1..]);
        assert_eq!(parse("poly.setPart"), Ok(None));
        assert_eq!(parse("poly.setPart M1PartB"), Ok(Some("M1PartB".into())));
        assert_eq!(
            parse("poly.setPart {Road Part}"),
            Ok(Some("Road Part".into()))
        );
        assert_eq!(
            parse("poly.setPart \"  Body Panel \""),
            Ok(Some("  Body Panel ".into()))
        );
        assert_eq!(
            parse("poly.setPart NAME:{Road Part}"),
            Ok(Some("Road Part".into()))
        );
        assert_eq!(parse("poly.setPart Part:A"), Ok(Some("Part:A".into())));
        assert_eq!(
            parse("poly.setPart name:{name:x}"),
            Ok(Some("name:x".into()))
        );
        for bad in [
            "poly.setPart a b",
            "poly.setPart name:a name:b",
            "poly.setPart name:",
            "poly.setPart ?",
            "poly.setPart {}",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        assert!(tokens("poly.setPart {Road Part").is_err());
        assert!(tokens("poly.setPart \"Road Part").is_err());
    }
    #[test]
    fn every_tool_has_one_table_entry_matching_the_command_table() {
        use std::collections::BTreeSet;
        assert_eq!(tool_info::TOOLS.len(), Tool::ALL.len());
        let mut names = BTreeSet::new();
        for tool in Tool::ALL {
            let info = tool_info::info(tool);
            let row = id(info.command_id)
                .unwrap_or_else(|| panic!("{} is missing from IDS", info.command_id));
            assert!(
                info.names.contains(&IDS[row].1),
                "{}: IDS name {:?} not in {:?}",
                tool.label(),
                IDS[row].1,
                info.names
            );
            for name in info.names {
                assert!(names.insert(*name), "{name} starts two tools");
                assert_eq!(tool_info::by_name(name), Some(tool));
            }
        }
    }
    #[test]
    fn catalog_is_unique_and_complete() {
        assert_eq!(IDS.len(), 72);
        assert_eq!(ATTRS.len(), 8);
        for (i, (name, _)) in IDS.iter().enumerate() {
            assert_eq!(id(name), Some(i));
        }
    }
}
