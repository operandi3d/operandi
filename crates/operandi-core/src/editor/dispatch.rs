//! `Editor::dispatch_inner`: runs one `Command` on the editor.
use super::*;

impl Editor {
    pub(crate) fn dispatch_inner(
        &mut self,
        cmd: Command,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        // Tool context: works with any tool open, a Pen draft too.
        if let Command::BackgroundConstraint(c) = cmd {
            return self.set_background_constraint(c);
        }
        // Symmetry too, Solid Sketch and every other session included: the
        // open tool follows the change (tool chain, Jason 2026-11-14).
        match cmd {
            Command::ToggleSymmetry => return self.toggle_symmetry(),
            Command::Symmetry(symmetry) => return self.set_symmetry(symmetry),
            _ => {}
        }
        // Several active meshes: the command runs on each (multi_tools.rs).
        if let Some(targets) = self.command_mesh_targets(&cmd) {
            return self.command_on_meshes(&targets, &cmd);
        }
        // Selection inference for the point and edge commands that take it
        // (selection_inference.rs): run on the converted set, mode kept.
        if let Some(target) = Self::command_takes(&cmd)
            && matches!(
                self.scene.selection.mode,
                Mode::Vertex | Mode::Edge | Mode::Polygon
            )
            && self.scene.selection.mode != target
        {
            return self.dispatch_inferred(target, cmd, report);
        }
        if let Command::CleanupFloating(target) = &cmd {
            return self.cleanup_floating(target.clone());
        }
        if let Command::MeshCleanup { flags, target } = &cmd {
            return self.mesh_cleanup(*flags, target.clone());
        }
        if matches!(&cmd, Command::QuadruplePolygon) {
            return self.quadruple_polygon();
        }
        if let Command::UnifyPolygons(request) = &cmd {
            return self.unify_polygons(request.clone());
        }
        if matches!(&cmd, Command::SpinQuads) {
            return self.spin_quads();
        }
        if matches!(&cmd, Command::AlignPolygons) {
            return self.align_polygons();
        }
        // Like Align, Flatten Ngons is idle-only; its own preflight refuses
        // drafts, live stages and sessions instead of consuming them.
        if matches!(&cmd, Command::FlattenNgons) {
            return self.flatten_ngons();
        }
        if matches!(&cmd, Command::SplitPolygon) {
            return self.split_polygon();
        }
        if matches!(&cmd, Command::MergeItems) {
            return self.merge_items();
        }
        if let Command::SetCurve(edit) = cmd {
            return self.set_curve(*edit);
        }
        if let Command::AddGeometry(add) = cmd {
            return self.add_geometry(*add);
        }
        if matches!(&cmd, Command::UnmergeItem) {
            return self.unmerge_item();
        }
        if matches!(&cmd, Command::MergeInstance) {
            return self.merge_instance();
        }
        if matches!(&cmd, Command::FreezeReplicator) {
            return self.freeze_replicator();
        }
        if let Command::SetReplicator { item, settings } = cmd {
            return self.set_replicator(item, *settings);
        }
        if let Command::VertexNormals(c) = cmd {
            return self.vertex_normals(*c);
        }
        if let Command::SetGuides(g) = cmd {
            return self.set_guides(*g);
        }
        if let Command::MeshPaint(stroke) = cmd {
            return self.mesh_paint(*stroke);
        }
        if matches!(&cmd, Command::DiagonalRedirect) {
            return self.diagonal_redirect();
        }
        if matches!(&cmd, Command::EdgeZip) {
            return self.edge_zip();
        }
        if matches!(&cmd, Command::FixBlackDots) {
            return self.fix_black_dots();
        }
        if matches!(&cmd, Command::TripleFan) {
            return self.triple_fan();
        }
        if let Command::TopologyDuplicate { item, a, b, offset } = cmd {
            return self.topology_duplicate(item, a, b, offset);
        }
        if matches!(&cmd, Command::CollapseSelection) {
            return self.collapse_selection();
        }
        if let Command::TopologyPen(edit) = cmd {
            return self.topology_pen(*edit);
        }
        if let Command::SelectTool(request) = cmd {
            return self.select_tool(request);
        }
        if let Command::RelaxMesh(options) = cmd {
            return self.relax_mesh(options);
        }
        if let Command::Tack(t) = cmd {
            return self.tack(t);
        }
        if let Command::GroundAlign(options) = cmd {
            return self.ground_align(options);
        }
        if let Command::QuickAlign(q) = cmd {
            return self.quick_align(q);
        }
        if matches!(&cmd, Command::CutSelection) {
            return self.cut_selection();
        }
        if matches!(&cmd, Command::PasteSelection) {
            return self.paste(None);
        }
        if let Command::PasteAt(placement) = cmd {
            return self.paste(Some(placement));
        }
        // MK-C03: idle-only like Align; its own preflight refuses a Pen draft,
        // live stage, session or selection gesture instead of consuming it.
        if matches!(&cmd, Command::MakePolygon) {
            return self.make_polygon(crate::MakeType::Auto, false);
        }
        if let Command::MakePolygonAs { kind, close } = cmd {
            return self.make_polygon(kind, close);
        }
        if let Command::MakeCurve { closed } = cmd {
            return self.make_curve_command(closed);
        }
        if matches!(&cmd, Command::MakeCurveFill) {
            return self.make_curve_fill_command();
        }
        if let Command::AlignWinding(options) = cmd {
            return self.align_winding_command(options);
        }
        if let Command::Dicer(o) = cmd {
            return self.cut_pieces("Dicer", |m| crate::dicer::dicer(m, &o));
        }
        if let Command::CurveBoolean(o) = cmd {
            return self.curve_boolean_command(o);
        }
        if let Command::ConvertCurves { from, to } = cmd {
            return self.convert_curves_command(from, to);
        }
        if let Command::OffsetCurves {
            closed,
            offset,
            trio,
        } = cmd
        {
            return self.offset_curves_command(closed, offset, trio);
        }
        if let Command::BezierTangent(o) = cmd {
            return self.bezier_tangent_command(o);
        }
        if let Command::UvRectangle(options) = cmd {
            return self.uv_rectangle_command(options);
        }
        if let Command::TransferVertexMap {
            map,
            kind,
            world,
            sampling,
        } = cmd
        {
            return self.transfer_vertex_map_command(map, kind, world, sampling);
        }
        if let Command::AddEdgePoint { a, b, t } = cmd {
            return self.add_edge_point(a, b, t, &Default::default());
        }
        if let Command::AddEdgePointWith { a, b, t, options } = &cmd {
            return self.add_edge_point(*a, *b, *t, options);
        }
        if let Command::AddEdgeLoopWith { a, b, t, options } = &cmd {
            return self.add_edge_loop(*a, *b, *t, options);
        }
        if let Command::EdgeCommand(command) = cmd {
            return self.edge_command(command);
        }
        if let Command::SubdivideEdges { count, percent } = cmd {
            return self.subdivide_edges(count, percent);
        }
        if let Command::SubdividePolygons { options, times } = cmd {
            return self.subdivide_polygons(options, times);
        }
        if let Command::UnsubdividePolygons(options) = cmd {
            return self.unsubdivide(options);
        }
        if let Command::Detriangulate(options) = cmd {
            return self.detriangulate(options);
        }
        if let Command::UvUnwrap(options) = cmd {
            return self.uv_unwrap(options);
        }
        if let Command::UvAtlas(options) = cmd {
            return self.uv_atlas(options);
        }
        if let Command::UvRelax(options) = cmd {
            return self.uv_relax_once(options);
        }
        if let Command::PackUvs(options) = cmd {
            return self.pack_uvs(options);
        }
        if let Command::Remesh(op) = cmd {
            return self.remesh(op);
        }
        if let Command::SelectUvIslands(map) = cmd {
            return self.select_uv_islands(map);
        }
        if let Command::Boolean(options) = cmd {
            return self.boolean(options);
        }
        if let Command::SolidDrill(options) = cmd {
            return self.drill(crate::DrillKind::Solid, options);
        }
        if let Command::AxisDrill(options) = cmd {
            return self.drill(crate::DrillKind::Axis, options);
        }
        match cmd {
            Command::MeshOpCreate(t) => self.meshop_create(&t),
            Command::MeshBlend(c) => self.meshblend(c),
            Command::MeshOpDelete(ops) => self.meshop_delete(&ops),
            Command::MeshOpMove { op, position } => self.meshop_move(op, position),
            Command::MeshOpEnable { op, on } => self.meshop_enable(op, on),
            Command::MeshOpSet { op, values } => self.meshop_set(op, &values),
            Command::MeshOpSelect(row) => self.meshop_select(row),
            Command::MeshOpIndex { op, edit } => self.meshop_index(op, edit),
            Command::MeshOpSelOp { op, edit } => self.meshop_selop(op, edit),
            Command::MeshOpFreeze { op, duplicate } => self.meshop_freeze(op, duplicate),
            Command::MeshOpFreezeAll => self.meshop_freeze_all(),
            Command::MeshOpEditBase { item, on } => self.meshop_edit_base(item, on),
            cmd => self.dispatch_after_mesh_stack(cmd, report),
        }
    }
    pub(super) fn dispatch_after_mesh_stack(
        &mut self,
        cmd: Command,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        if let Command::CenterSelection(axes) = cmd {
            return self.center_selection(axes);
        }
        if let Command::ResetTransform { items, part } = cmd {
            return self.reset_transform(items, part);
        }
        if let Command::FreezeTransform { items, part } = cmd {
            return self.freeze_transform(items, part);
        }
        if let Command::VertexCommand(command) = cmd {
            return self.vertex_command(command);
        }
        if let Command::NewVertex(world) = cmd {
            return self.new_vertex(world);
        }
        if let Command::WeldVertices { sources, target } = &cmd {
            return self.weld_vertices(sources.clone(), *target);
        }
        if let Command::WeldDrag { source, drop } = cmd {
            return self.weld_drag(source, drop);
        }
        if let Command::SculptStroke(edit) = cmd {
            return self.sculpt_stroke(*edit);
        }
        if let Command::WeightStroke(edit) = cmd {
            return self.weight_stroke(*edit);
        }
        if let Command::NewWeightMap { name, value } = cmd {
            return self.new_weight_map(name, value);
        }
        if let Command::SelectWeightMap(name) = cmd {
            // VM-02: tool context; a live tool under a Vertex Map falloff
            // re-applies with the new map.
            if name.as_ref().is_some_and(|n| n.trim().is_empty()) {
                return Err("A weight map needs a name".into());
            }
            let mut next = self.clone();
            next.selected_weight_map = name;
            if next.spatial_falloff.is_some()
                && let Some(l) = next.session.live.as_ref().filter(|l| l.applied)
            {
                next.dispatch(Command::Preview(l.params.clone()))?;
            }
            *self = next;
            return Ok(());
        }
        if let Command::SyncVertexOrder {
            source_item,
            source_faces,
        } = cmd
        {
            return self.sync_vertex_order(source_item, source_faces);
        }
        if let Command::AddEdgeLoop { a, b, t } = cmd {
            return self.add_edge_loop(a, b, t, &Default::default());
        }
        if let Command::SetPosition(assignment) = &cmd {
            return self.set_position(assignment.clone());
        }
        // Idle-only like Set Position: its preflight refuses live state.
        if let Command::AbsoluteScale(scale) = &cmd {
            return self.absolute_scale(*scale);
        }
        // SO-C03: idle-only like Set Position; its preflight refuses a Pen
        // draft, live stage or session instead of confirming or consuming it.
        if let Command::ScaleOffsetPolygon(distance) = &cmd {
            return self.scale_offset_polygon(*distance);
        }
        if let Command::ScaleOffsetDetach(distance) = &cmd {
            return self.scale_offset_detach(*distance);
        }
        // MAT-001: like Set Position, Set Material is an idle-only modal edit.
        // Route it before Pen, I2 and session dispatch so its own preflight
        // refuses a Pen draft, live stage or Reduction/Bridge session atomically
        // instead of confirming, dropping or consuming that unrelated state.
        if let Command::SetMaterial(assignment) = &cmd {
            return self.set_material(assignment.clone());
        }
        if let Command::ShaderTree(edit) = &cmd {
            return self.edit_shader_tree(edit.clone());
        }
        #[cfg(feature = "painting")]
        if let Command::Paint(edit) = &cmd {
            return self.edit_paint(edit.clone());
        }
        if let Command::ApplyEnvironmentPreset { preset, mode, name } = &cmd {
            return self
                .apply_environment_preset((**preset).clone(), *mode, name)
                .map(|_| ());
        }
        if let Command::LoadPreset { preset, group } = &cmd {
            return self.load_preset((**preset).clone(), *group);
        }
        if let Command::InsertMaterialPreset { preset, beside } = &cmd {
            return self.insert_material_preset((**preset).clone(), *beside);
        }
        if let Command::SetEnvironmentImage {
            path,
            width,
            height,
            item,
        } = &cmd
        {
            return self.set_environment_image(path, *width, *height, *item);
        }
        if let Command::ApplyMaterialPreset {
            preset,
            item,
            faces,
        } = &cmd
        {
            return self.apply_material_preset((**preset).clone(), *item, faces);
        }
        // One rule for picking a tool while another is open (Jason,
        // 2026-11-05): the same tool's button drops it; another tool drops
        // the open one first, keeping its geometry, then starts. Both run on
        // a staged copy, so a Drop that fails (a field error) or a start that
        // fails leaves the open tool, scene and history as they were.
        // Unit insertion while a tool is open drops it first, the same way:
        // its work is one step and the unit another, or nothing changes.
        if matches!(
            &cmd,
            Command::InsertUnit { .. }
                | Command::InsertEllipsoidUnit(_)
                | Command::InsertShapeUnit(..)
        ) && (self.session.has_active_task() || self.i2_active())
        {
            if self.selection_start.is_some() {
                return Err("Finish the selection gesture before unit insertion".into());
            }
            let mut staged = self.clone();
            staged.dispatch(Command::Drop)?;
            if staged.session.has_active_task() || staged.i2_active() {
                return Err("Finish the current tool before unit insertion".into());
            }
            staged.dispatch(cmd)?;
            *self = staged;
            return Ok(());
        }
        if let Some(switch) = self.tool_switch(&cmd) {
            return match switch {
                crate::active_task::ToolSwitch::Toggle => self.dispatch(Command::Drop),
                crate::active_task::ToolSwitch::Replace => {
                    let mut staged = self.clone();
                    staged.dispatch(Command::Drop)?;
                    if staged.session.has_active_task() || staged.i2_active() {
                        return Err("Finish the current tool before starting another".into());
                    }
                    staged.dispatch(cmd)?;
                    *self = staged;
                    Ok(())
                }
            };
        }
        // Clicking another item in the Items tree while Move, Rotate, Scale
        // or Transform is live keeps the tool and re-targets it (Jason,
        // 2026-11-08): the open tool's work is dropped as its own step, the
        // selection changes, and the same tool starts again on the new
        // selection with a fresh zero offset and its handle there. If the
        // tool can't start there (nothing selected, an empty mesh), the
        // selection still stands and the tool stays dropped, as after Undo
        // past a transform's activation. Runs on a staged copy so a Drop or
        // selection error leaves tool, scene and history as they were.
        // Activation may switch a camera/light/backdrop to Item mode, as
        // activating the tool by hand does.
        if matches!(
            &cmd,
            Command::SelectItem(_)
                | Command::SelectItems(_)
                | Command::ToggleItemSelection(_)
                | Command::SelectGroup(_)
        ) && self.selection_keeps_tool()
            && let Some(live) = self.session.live.as_ref()
        {
            let tool = live.tool;
            return self.retarget_transform(tool, cmd);
        }
        // Grow and Shrink (Shift+Up/Down) re-target a live transform tool
        // the same way in component modes (UIC-306, Jason 2026-11-27).
        if matches!(
            &cmd,
            Command::Selection(crate::SelectionOperation::Grow | crate::SelectionOperation::Shrink)
        ) && self.scene.selection.mode != Mode::Item
            && self.selection_keeps_tool()
            && let Some(live) = self.session.live.as_ref()
        {
            let tool = live.tool;
            return self.retarget_transform(tool, cmd);
        }
        // Mode has one shared, ordered structural resolver. It runs before
        // the Pen and session routes so an open tool (a Pen draft too) is
        // dropped first instead of refusing the mode change.
        if let Command::Mode(mode) = &cmd {
            return self.dispatch_mode(*mode);
        }
        if let Command::SetPolygonTagKind(kind) = cmd {
            if let Some(reason) = kind.unavailable_reason() {
                return Err(format!("{}: {reason}", kind.label()));
            }
            self.polygon_tag_kind = kind;
            self.scene.set_tag_kind(kind);
            self.revision = self.revision.wrapping_add(1);
            return Ok(());
        }
        if let Command::EnterTagMode { convert } = cmd {
            let kind = self.polygon_tag_kind;
            if let Some(reason) = kind.unavailable_reason() {
                return Err(format!("{}: {reason}", kind.label()));
            }
            // The shared mode route drops an open tool first.
            let mode = self.scene.selection.mode;
            self.dispatch_mode(Mode::Polygon)?;
            let before = self.scene.clone();
            // Convert reads the selection of the mode we came from.
            self.scene.selection.mode = mode;
            self.scene.enter_tag_mode(kind, convert)?;
            if convert && self.scene.selection.faces != before.selection.faces {
                self.record_selection("Convert to Tags".into(), before);
            }
            return Ok(());
        }
        if let Command::SolidSketchStrip = &cmd {
            return self.strip_solid_sketch();
        }
        if self.session.solid_sketch.is_some()
            || matches!(
                &cmd,
                Command::SolidSketchStart { .. } | Command::SolidSketch { .. }
            )
        {
            return self.solid_sketch_command(cmd, report);
        }
        if self.session.pen_draft.is_some()
            || matches!(
                &cmd,
                Command::PenStart
                    | Command::PenCapture(_)
                    | Command::PenCaptureEnd(_)
                    | Command::PenSelect(_)
                    | Command::PenRefine(_)
            )
        {
            let point_undo = self.session.pen_draft.is_some() && matches!(&cmd, Command::Undo);
            let cancel = self.session.pen_draft.is_some() && matches!(&cmd, Command::Cancel);
            let result = self.interactive_pen_command(cmd);
            if result.is_ok()
                && let Some(report) = report
            {
                if point_undo {
                    report.effect = crate::HistoryEffect::PenPointUndo;
                }
                if cancel {
                    report.effect = crate::HistoryEffect::Cancel;
                }
            }
            return result;
        }
        if matches!(&cmd, Command::FlipPolygons) {
            return self.flip_selected_polygons();
        }
        if let Command::FlipIt { preset, axis } = &cmd {
            return self.flip_it(preset, *axis);
        }
        if let Command::UvToolSpace(space) = &cmd {
            return self.uv_tool_space(space.clone());
        }
        if let Command::UvMap(edit) = &cmd {
            return self.uv_map_edit(edit.clone());
        }
        if let Command::UvIsland(op) = &cmd {
            return self.uv_island(*op);
        }
        if let Command::UvOp(op) = &cmd {
            return self.uv_op(op.clone());
        }
        if let Command::UvTexel(o) = &cmd {
            self.uv_texel = *o;
            return Ok(());
        }
        if let Command::UvSeam(edit) = &cmd {
            return self.uv_seam(edit.clone());
        }
        if let Command::UvSymmetry(s) = &cmd {
            s.validate()?;
            self.uv_symmetry = *s;
            // A live UV tool redoes its result with the new layer.
            if let Some(l) = self
                .session
                .live
                .as_ref()
                .filter(|l| l.applied && l.params.uv.is_some())
            {
                let p = l.params.clone();
                return self.dispatch(Command::Preview(p));
            }
            return Ok(());
        }
        if matches!(cmd, Command::UvSymmetryCenter) {
            return self.uv_symmetry_center();
        }
        if let Command::UvSymmetrize(o) = &cmd {
            return self.uv_symmetrize(*o);
        }
        if matches!(cmd, Command::UvToMesh) {
            return self.uv_to_mesh();
        }
        if matches!(cmd, Command::UvTransfer) {
            return self.uv_transfer();
        }
        if let Command::UvIslandPreset { preset, click } = &cmd {
            return self.uv_island_preset(preset, *click);
        }
        match &cmd {
            Command::ToggleSubdivision(toggle) => return self.toggle_subdivision(*toggle),
            Command::SetSubdivision(settings) => return self.set_subdivision(settings.clone()),
            Command::Freeze(options) => return self.freeze_mesh(options),
            Command::SetSmoothing {
                settings,
                continuing,
            } => return self.set_smoothing(settings.clone(), *continuing),
            Command::SetRenderSettings(settings) => {
                return self.set_render_settings(settings.clone());
            }
            Command::SetSceneTime(time) => return self.set_scene_time(*time),
            _ => {}
        }
        // MN-05: every tool session takes Reset Tool Attributes, so it runs
        // before the sessions' own input guards.
        if matches!(cmd, Command::ResetToolAttributes) {
            return self.reset_tool_attributes();
        }
        if self.session.bridge.is_some() && matches!(&cmd, Command::Pen(_)) {
            return Err("Drop or Cancel Bridge before Pen".into());
        }
        if let Command::Pen(input) = &cmd {
            return self.create_pen(input);
        }
        if self.i2_dispatch(&cmd, report)? {
            self.revision = self.revision.wrapping_add(1);
            return Ok(());
        }
        // RED-001: the Reduction session owns input until Drop or Cancel.
        // Picking another tool is handled by `tool_switch` above.
        if self.session.reduction.is_some()
            && !matches!(
                &cmd,
                Command::InitializeReduction
                    | Command::ConfigureReduction(_)
                    | Command::Apply
                    | Command::Drop
                    | Command::Cancel
                    | Command::Undo
                    | Command::Redo
            )
        {
            return Err(
                "Drop or Cancel Polygon Reduction before changing selection or context".into(),
            );
        }
        if self.session.bridge.is_some()
            && !matches!(
                &cmd,
                Command::ConfigureBridge(_)
                    | Command::ResetBridge
                    | Command::Apply
                    | Command::Drop
                    | Command::Cancel
                    | Command::Undo
                    | Command::Redo
            )
        {
            return Err("Drop or Cancel Bridge before changing selection or context".into());
        }
        if self.session.spike.is_some()
            && !matches!(
                &cmd,
                Command::ConfigureSpike(_)
                    | Command::ResetSpike
                    | Command::Apply
                    | Command::Drop
                    | Command::Cancel
                    | Command::Undo
                    | Command::Redo
            )
        {
            return Err("Drop or Cancel Spike before changing selection or context".into());
        }
        if self.session.ellipsoid.is_some()
            && !matches!(
                &cmd,
                Command::ConfigureEllipsoid(_)
                    | Command::ResetEllipsoid
                    // #35: the falloff shapes the Ellipsoid while it's open.
                    | Command::SpatialFalloff(_)
                    | Command::AddedFalloffs(_)
                    | Command::SpatialFalloffMix(_)
                    | Command::Apply
                    | Command::Drop
                    | Command::Cancel
                    // History refusal is owned by the shared policy below.
                    | Command::Undo
                    | Command::Redo
            )
        {
            return Err(
                "Drop or Cancel numeric Ellipsoid before changing selection/context or history"
                    .into(),
            );
        }
        // Torus/Capsule/Arc/Tube own input like the numeric Ellipsoid.
        // #35: the falloff shapes the Torus and Capsule while they're open.
        let shaped_by_falloff = |kind: crate::ShapeKind| {
            matches!(kind, crate::ShapeKind::Torus | crate::ShapeKind::Capsule)
        };
        if let Some(active) = self.session.shape.as_ref().map(|s| s.kind())
            && !(shaped_by_falloff(active)
                && matches!(
                    &cmd,
                    Command::SpatialFalloff(_)
                        | Command::AddedFalloffs(_)
                        | Command::SpatialFalloffMix(_)
                ))
            && !matches!(
                &cmd,
                Command::ConfigureShape(_)
                    | Command::ResetShape
                    | Command::Apply
                    | Command::Drop
                    | Command::Cancel
                    // History refusal is owned by the shared policy below.
                    | Command::Undo
                    | Command::Redo
            )
        {
            return Err(format!(
                "Drop or Cancel {} before changing selection/context or history",
                active.label()
            ));
        }
        // Retargeting resolves the current surface stage through the existing
        // atomic Drop contract before selection can own it.
        if self
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.tool.is_smooth_shift())
            && matches!(
                &cmd,
                Command::SelectItem(_)
                    | Command::Select { .. }
                    | Command::ClearSelection
                    | Command::Selection(_)
                    | Command::SelectionFill { .. }
                    | Command::SelectEdgeLoop { .. }
                    | Command::SelectConnectedPolygons { .. }
                    | Command::NewMesh
                    | Command::NewMeshBeside
                    | Command::NewNamedMesh(_)
                    | Command::AddItem(_)
                    | Command::ItemPattern { .. }
            )
        {
            let mut staged = self.clone();
            staged.dispatch(Command::Drop)?;
            staged.dispatch(cmd)?;
            *self = staged;
            return Ok(());
        }
        match cmd {
            Command::FlipPolygons | Command::FlipIt { .. } => {
                unreachable!("Flip is handled before session routing")
            }
            Command::UvToolSpace(_)
            | Command::UvMap(_)
            | Command::UvIsland(_)
            | Command::UvOp(_)
            | Command::UvTexel(_)
            | Command::UvSeam(_)
            | Command::UvSymmetry(_)
            | Command::UvSymmetryCenter
            | Command::UvSymmetrize(_)
            | Command::UvToMesh
            | Command::UvTransfer
            | Command::UvIslandPreset { .. } => {
                unreachable!("UV edits are handled before session routing")
            }
            Command::ToggleSubdivision(_)
            | Command::SetSubdivision(_)
            | Command::Freeze(_)
            | Command::SetSmoothing { .. }
            | Command::SetRenderSettings(_)
            | Command::SetSceneTime(_) => {
                unreachable!("Subdivision and smoothing are handled before session routing")
            }
            Command::Pen(_)
            | Command::PenStart
            | Command::PenCapture(_)
            | Command::PenCaptureEnd(_)
            | Command::PenSelect(_)
            | Command::PenRefine(_) => unreachable!("Pen is handled before session routing"),
            Command::SolidSketchStart { .. }
            | Command::SolidSketch { .. }
            | Command::SolidSketchStrip => {
                unreachable!("Solid Sketch is handled before session routing")
            }
            Command::BeginExtrude => unreachable!("handled by I2 boundary"),
            Command::StartExtrude => {
                let live = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::Extrude && !l.applied)
                    .ok_or("Activate Extrude first")?;
                let mut p = live.params.clone();
                p.extent = DVec3::ZERO;
                self.preview(p, true)?;
            }
            cmd @ (Command::ConfigureSmoothShift(_)
            | Command::ResetSmoothShift
            | Command::CloneSmoothShift
            | Command::SurfaceStateSave(_)
            | Command::SurfaceStateRecall(_)
            | Command::SurfaceStateRename(_)
            | Command::SurfaceStateDelete) => self.dispatch_surface(cmd)?,
            Command::ArmBridge => self.arm_bridge()?,
            Command::ConfigureBridge(p) => self.configure_bridge(p)?,
            Command::ResetBridge => self.configure_bridge(Default::default())?,
            Command::ArmSpike => self.arm_spike()?,
            Command::ConfigureSpike(s) => self.configure_spike(s)?,
            Command::ResetSpike => self.configure_spike(0.)?,
            Command::ResetPrimitive => self.reset_primitive()?,
            Command::PrimitiveUvs(on) => {
                let live = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool.is_primitive())
                    .ok_or("Make UVs needs a live Cube, Sphere or Cylinder")?;
                let params = Parameters {
                    make_uvs: on,
                    ..live.params.clone()
                };
                self.set_primitive_params(params)?;
            }
            Command::ArmReduction => self.arm_reduction()?,
            Command::InitializeReduction => self.initialize_reduction()?,
            Command::ConfigureReduction(p) => self.configure_reduction(p)?,
            Command::ArmEllipsoid => self.arm_ellipsoid()?,
            Command::ConfigureEllipsoid(p) => self.configure_ellipsoid(p)?,
            Command::ResetEllipsoid => {
                if self.session.live.is_some() || self.session.cone.is_some() {
                    return Err("Drop or Cancel the other tool before Ellipsoid Reset".into());
                }
                let mut staged = self.clone();
                staged.presets.forget::<crate::EllipsoidPreset>();
                if staged.session.ellipsoid.is_none() {
                    staged.arm_ellipsoid()?;
                }
                staged.configure_ellipsoid(Default::default())?;
                *self = staged;
            }
            Command::InsertEllipsoidUnit(destination) => self.insert_ellipsoid_unit(destination)?,
            Command::InsertShapeUnit(shape, destination) => {
                self.insert_shape_unit(shape, destination)?
            }
            Command::ArmShape(kind) => {
                if self.session.shape.is_some() {
                    return self.dispatch(Command::Drop);
                }
                if self.session.live.is_some()
                    || self.session.cone.is_some()
                    || self.session.ellipsoid.is_some()
                {
                    let mut staged = self.clone();
                    staged.dispatch(Command::Drop)?;
                    staged.dispatch(Command::ArmShape(kind))?;
                    *self = staged;
                    return Ok(());
                }
                self.arm_shape(kind)?;
            }
            Command::ConfigureShape(p) => self.configure_shape(p)?,
            Command::ResetShape => self.reset_shape()?,
            Command::ArmCone => self.arm_cone()?,
            Command::ConfigureCone(p) => self.configure_cone(p)?,
            Command::InsertUnit { tool, destination } => self.insert_unit(tool, destination)?,
            Command::ActionContext(context) => self.handle_action_context(context)?,
            Command::SetActionCenter(point)
                if self
                    .session
                    .live
                    .as_ref()
                    .is_some_and(|l| l.tool == Tool::Bend) =>
            {
                self.set_bend_center(point, None)?;
            }
            Command::ToolClick { point, plane } => {
                if plane > 2 {
                    return Err("The work plane must be 0, 1 or 2".into());
                }
                match self.session.live.as_ref().map(|l| l.tool) {
                    Some(Tool::Bend) => self.set_bend_center(point, Some(plane))?,
                    Some(_) => return Err("This tool has no click setup".into()),
                    None => return Err("No active tool".into()),
                }
            }
            Command::SetActionCenter(point) => {
                let live = self
                    .session
                    .live
                    .as_mut()
                    .ok_or("No active transform tool")?;
                // ACS-07: any tool that pivots through the pivot service
                // takes a placed centre (the transforms, and Smooth Shift
                // and Thicken, ACS-20; the polygon stage's side, ACS-21).
                if live.action_frame.is_none() {
                    return Err("This tool has no center to move".into());
                }
                if live.applied {
                    return Err("Drop the current transform before moving its center".into());
                }
                if !point.is_finite() || point.abs().max_element() > 1e9 {
                    return Err("The center must be a finite point".into());
                }
                let frame = live.action_frame.ok_or("Missing action frame")?;
                // Selection axes under Symmetry follow the side the centre
                // was placed on (S-02), so centre, axes and the side that
                // drives the edit agree; other axes stay as they are.
                let basis = match &self.session.live {
                    Some(live)
                        if self.action_context.axis == crate::AxisOrientation::Selection
                            && live.params.uv.is_none() =>
                    {
                        crate::pivot_service::PivotRequest {
                            placed: Some(point),
                            ..self.pivot_request(&live.base, live.tool)
                        }
                        .frame()
                        .map_or(frame.basis, |f| f.basis)
                    }
                    _ => frame.basis,
                };
                let live = self
                    .session
                    .live
                    .as_mut()
                    .ok_or("No active transform tool")?;
                live.action_frame = Some(crate::ActionFrame {
                    center: point,
                    basis,
                });
                live.placed_centre = Some(point);
            }
            Command::ResetActionCenter => {
                let live = self
                    .session
                    .live
                    .as_ref()
                    .ok_or("No active transform tool")?;
                let frame = live.action_frame.ok_or("This tool has no center to move")?;
                // Resolved against the tool's base, so after a drag the
                // handle (center + delta) still sits on the moved selection.
                let own = self.pivot_frame(&live.base, live.tool)?;
                // Selection axes go back with the centre (S-02, see
                // `SetActionCenter`).
                let basis = if self.action_context.axis == crate::AxisOrientation::Selection
                    && live.params.uv.is_none()
                {
                    own.basis
                } else {
                    frame.basis
                };
                if let Some(live) = &mut self.session.live {
                    live.action_frame = Some(crate::ActionFrame {
                        center: own.center,
                        basis,
                    });
                    live.placed_centre = None;
                }
            }
            Command::CommitTransformStep => self.commit_transform_step()?,
            Command::ContinueTransformStep => self.continue_transform_step()?,
            Command::DuplicateSelectedPolygons => self.duplicate_selected_polygons()?,
            Command::ConfigureChamfer(parameters) => {
                let live = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::Chamfer)
                    .ok_or("Activate Chamfer Edit first")?;
                let mut p = live.params.clone();
                p.chamfer = parameters;
                self.preview(p, false)?;
                *self.presets.get_mut::<crate::ChamferPreset>() = parameters;
            }
            Command::ChamferOnce(parameters) => {
                if self.session.live.is_some() {
                    return Err("Finish or Cancel the active tool first".into());
                }
                let mut staged = self.clone();
                staged.dispatch(Command::Activate(Tool::Chamfer))?;
                staged.dispatch(Command::ConfigureChamfer(parameters))?;
                staged.dispatch(Command::Drop)?;
                *self = staged;
            }
            Command::PolyHaulOnce(offset) => {
                if self.session.live.is_some() {
                    return Err("Finish or Cancel the active tool first".into());
                }
                let mut staged = self.clone();
                staged.dispatch(Command::Activate(Tool::PolyHaul))?;
                let mut params = staged.session.live.as_ref().unwrap().params.clone();
                params.poly_haul_offset = offset;
                staged.dispatch(Command::Preview(params))?;
                staged.dispatch(Command::Drop)?;
                *self = staged;
            }
            Command::SketchExtrudeOnce(path) => self.handle_sketch_extrude_once(path)?,
            cmd @ (Command::ConfigureBevel(_)
            | Command::ResetBevel
            | Command::BevelStateSave(_)
            | Command::BevelStateRecall(_)
            | Command::BevelStateRename(_)
            | Command::BevelStateDelete
            | Command::ResetBevelState
            | Command::BevelProfile(_)
            | Command::SpatialFalloff(_)
            | Command::AddedFalloffs(_)
            | Command::SpatialFalloffMix(_)
            | Command::FalloffElementPick(_)
            | Command::FalloffStamps(_)
            | Command::FalloffPath(_)) => {
                self.dispatch_bevel(cmd)?;
            }
            Command::ConfigureExtrude(p) => self.handle_configure_extrude(p)?,
            Command::ResetPolyHaul => {
                let live = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::PolyHaul)
                    .ok_or("Activate Poly Haul first")?;
                let mut p = live.params.clone();
                p.poly_haul_offset = 0.;
                self.dispatch(Command::Preview(p))?;
            }
            Command::ResetSketchExtrude => {
                let live = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::SketchExtrude)
                    .ok_or("Activate Sketch Extrude first")?;
                let mut p = live.params.clone();
                p.sketch_precision = 30.;
                self.dispatch(Command::Preview(p))?;
            }
            Command::ResetBend => {
                let live = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::Bend)
                    .ok_or("Activate Bend first")?;
                let mut p = live.params.clone();
                p.bend = crate::bend::reset(p.bend, self.bend_axes(&self.scene));
                self.dispatch(Command::Preview(p))?;
            }
            Command::ResetVertexMerge => {
                let live = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::VertexMerge)
                    .ok_or("Activate the Merge Tool first")?;
                let mut p = live.params.clone();
                p.shift = MERGE_TOOL_DISTANCE;
                self.dispatch(Command::Preview(p))?;
            }
            Command::ResetExtrude => {
                let mut p = Parameters::default();
                if let Some(l) = &self.session.live {
                    p.extrude_uv_map = l.params.extrude_uv_map.clone();
                    p.extrude_uv_selected = l.params.extrude_uv_selected;
                }
                // Reset is atomic and can restore U while setters are unavailable.
                let mut staged = self.clone();
                if let Some(l) = &mut staged.session.live {
                    l.params.extrude_uvs = p.extrude_uvs;
                }
                staged.dispatch(Command::ConfigureExtrude(p))?;
                *self = staged;
            }
            // BN-01 / G3: every way of starting Bend (button, Ctrl+E, the
            // command line, scripts) goes through its tool-presets row, so
            // Automatic centre and Auto axis are in place for the life of
            // the tool and go back on drop (TP-12).
            Command::Activate(Tool::Bend)
                if !self
                    .session
                    .preset
                    .as_ref()
                    .is_some_and(|p| p.tool == Tool::Bend) =>
            {
                self.activate_preset(crate::tool_preset::BEND_ROW)?;
                // BN-47: a remembered Angle shows on the mesh at once, so
                // the panel and the preview agree.
                if let Some(l) = self.session.live.as_ref().filter(|l| l.tool == Tool::Bend)
                    && l.params.bend.angle != 0.
                {
                    let p = l.params.clone();
                    self.dispatch(Command::Preview(p))?;
                }
            }
            Command::Activate(t) => {
                self.close_copier_row_for(t);
                let after = self.handle_activate(t)?;
                // Section 8: warn when the points the tool edits lack
                // mirror partners.
                self.note_unpaired_points();
                if let AfterCommand::Done = after {
                    return Ok(());
                }
            }
            Command::ActivateDeform(preset) => self.activate_preset(preset.id())?,
            Command::ActivatePreset(name) => self.activate_preset(&name)?,
            Command::Preview(p) => {
                if let Err(error) = self.preview(p, false) {
                    if let Some(live) = self.session.live.as_mut()
                        && (matches!(live.tool, Tool::Bevel | Tool::TwistBevel | Tool::Extrude)
                            || live.tool.is_smooth_shift())
                    {
                        live.preview_error = Some(error.clone());
                    }
                    return Err(error);
                }
            }
            Command::ToolItemChannel { item, edit } => self.tool_item_channel(item, &edit)?,
            Command::Continue => self.handle_continue()?,
            Command::Apply => {
                if let AfterCommand::Done = self.handle_apply()? {
                    return Ok(());
                }
            }
            Command::Drop => {
                if let AfterCommand::Done = self.handle_drop()? {
                    return Ok(());
                }
            }
            Command::Cancel => self.handle_cancel(report)?,
            Command::Undo => self.handle_undo(report)?,
            Command::Redo => self.handle_redo(report)?,
            Command::Mode(_) => unreachable!("handled by Mode route"),
            Command::BeginSelection => {
                self.require_idle()?;
                if self.selection_start.is_none() {
                    self.selection_start = Some(Box::new(self.scene.clone()));
                }
            }
            Command::EndSelection { cancel } => {
                let partners = std::mem::take(&mut self.session.gesture_partners);
                if !cancel {
                    self.scene.partners_last(&partners);
                }
                if let Some(before) = self.selection_start.take() {
                    if cancel {
                        self.scene.selection = before.selection;
                    } else if self.scene != *before {
                        self.record_selection("Selection".into(), *before);
                    }
                }
            }
            Command::Select {
                item,
                hit,
                replace,
                remove,
            } => {
                self.require_idle()?;
                let before = self.selection_start.is_none().then(|| self.scene.clone());
                self.scene.select_component(item, hit, replace, remove)?;
                // SYM-B2-10: with Symmetry on, a pick also selects the mirror
                // counterpart (measured for polygons; points and edges, and
                // removing, follow the same rule provisionally).
                if let Some(mirror) =
                    self.scene
                        .symmetric_counterpart(item, hit, self.active_symmetry())
                    && mirror != hit
                {
                    self.scene.select_component(item, mirror, false, remove)?;
                    if self.selection_start.is_some() && !remove {
                        self.session.gesture_partners.push((item, mirror));
                    }
                }
                if let Some(before) = before
                    && self.scene != before
                {
                    self.record_selection("Selection".into(), before);
                }
            }
            Command::SelectPolygonSet(name) => self.handle_select_polygon_set(name)?,
            Command::AssignSelectionSet(name) => self.assign_selection_set(name)?,
            Command::UseSelectionSet { name, mode } => self.use_selection_set(name, mode)?,
            Command::DeleteSelectionSet(name) => self.delete_selection_set(name)?,
            Command::CreateSelectionSetGroup { name, sets } => {
                self.create_selection_set_group(name, sets)?
            }
            Command::DeleteSelectionSetGroup(name) => self.delete_selection_set_group(name)?,
            Command::SetPart(assignment) => self.set_part(assignment)?,
            Command::QuadruplePolygon => unreachable!("Quadruple is handled before task routing"),
            Command::SpinQuads => unreachable!("Spin Quad is handled before task routing"),
            Command::AlignPolygons => unreachable!("Align is handled before task routing"),
            Command::MakePolygon | Command::MakePolygonAs { .. } => {
                unreachable!("Make is handled before task routing")
            }
            Command::MakeCurve { .. }
            | Command::MakeCurveFill
            | Command::AlignWinding(_)
            | Command::UvRectangle(_)
            | Command::ConvertCurves { .. }
            | Command::CurveBoolean(_)
            | Command::OffsetCurves { .. }
            | Command::BezierTangent(_)
            | Command::Dicer(_)
            | Command::TransferVertexMap { .. } => {
                unreachable!("curve commands are handled before task routing")
            }
            Command::AddEdgePoint { .. }
            | Command::AddEdgeLoop { .. }
            | Command::AddEdgePointWith { .. }
            | Command::AddEdgeLoopWith { .. }
            | Command::EdgeCommand(_)
            | Command::SubdivideEdges { .. }
            | Command::SubdividePolygons { .. }
            | Command::UnsubdividePolygons(_)
            | Command::Detriangulate(_)
            | Command::UvUnwrap(_)
            | Command::UvAtlas(_)
            | Command::UvRelax(_)
            | Command::PackUvs(_)
            | Command::Remesh(_)
            | Command::SelectUvIslands(_)
            | Command::Boolean(_)
            | Command::MeshOpCreate(_)
            | Command::MeshBlend(_)
            | Command::MeshOpDelete(_)
            | Command::MeshOpMove { .. }
            | Command::MeshOpEnable { .. }
            | Command::MeshOpSet { .. }
            | Command::MeshOpSelect(_)
            | Command::MeshOpIndex { .. }
            | Command::MeshOpSelOp { .. }
            | Command::MeshOpFreeze { .. }
            | Command::MeshOpFreezeAll
            | Command::MeshOpEditBase { .. }
            | Command::SolidDrill(_)
            | Command::AxisDrill(_)
            | Command::CenterSelection(_) => {
                unreachable!("Edge commands are handled before task routing")
            }
            Command::FlattenNgons => unreachable!("Flatten Ngons is handled before task routing"),
            Command::CleanupFloating(_) | Command::MeshCleanup { .. } => {
                unreachable!("Mesh Cleanup is handled before task routing")
            }
            Command::UnifyPolygons(_) => unreachable!("Unify is handled before task routing"),
            Command::SplitPolygon => unreachable!("Split is handled before task routing"),
            Command::SetPosition(_) => unreachable!("Set Position is handled before task routing"),
            Command::AbsoluteScale(_) => {
                unreachable!("Absolute Scaling is handled before task routing")
            }
            Command::ScaleOffsetPolygon(_) | Command::ScaleOffsetDetach(_) => {
                unreachable!("Scale Offset is handled before task routing")
            }
            Command::SetMaterial(_) => unreachable!("Set Material is handled before task routing"),
            Command::ShaderTree(_) => {
                unreachable!("Shader tree edits are handled before task routing")
            }
            Command::LoadPreset { .. } | Command::InsertMaterialPreset { .. } => {
                unreachable!("Preset loads are handled before task routing")
            }
            Command::SetEnvironmentImage { .. } | Command::ApplyEnvironmentPreset { .. } => {
                unreachable!("Environment images and presets are set before task routing")
            }
            Command::ApplyMaterialPreset { .. } => {
                unreachable!("Material drops are handled before task routing")
            }
            Command::Triple => self.triple()?,
            Command::RemovePolygons => self.remove_polygons()?,
            Command::DeleteComponents => self.delete_components()?,
            Command::MergePolygons => self.merge_polygons()?,
            Command::CollapsePolygon => self.collapse_polygon()?,
            Command::Selection(operation) => {
                self.require_idle()?;
                let before = self.scene.clone();
                // SYM-LOOP-01..04: with Symmetry on, seed-reading commands
                // count only the picked side, then add the mirror side.
                let symmetry = self.active_symmetry();
                self.scene
                    .on_active_meshes(|s| s.selection_operation_symmetric(operation, symmetry))?;
                if self.scene != before {
                    self.record_selection(operation.label().into(), before);
                }
            }
            Command::ToggleSymmetry => self.toggle_symmetry()?,
            Command::Symmetry(symmetry) => self.set_symmetry(symmetry)?,
            Command::SelectSymmetrical => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.select_symmetrical(self.active_symmetry())?;
                if self.scene != before {
                    self.record_selection("Select Symmetrical".into(), before);
                }
            }
            Command::SymmetrizePolygons { side, merge } => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene
                    .symmetrize_polygons(self.active_symmetry(), side, merge)?;
                if self.scene != before {
                    self.record("Symmetrize".into(), before);
                }
            }
            Command::Symmetrize(side) => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.symmetrize(self.active_symmetry(), side)?;
                if self.scene != before {
                    self.record("Symmetrize".into(), before);
                }
            }
            Command::SelectionFill { item, face } => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.fill_polygon_selection(item, face)?;
                // SYM3-30: the mirror region is filled too.
                if let Some(crate::Component::Polygon(g)) =
                    self.mirror_pick(item, crate::Component::Polygon(face))
                    && !self.scene.selection.faces_of(item).contains(&g)
                {
                    self.scene.fill_polygon_selection(item, g)?;
                }
                if self.scene != before {
                    self.record_selection("Selection Fill".into(), before);
                }
            }
            Command::SelectEdgeLoop { item, a, b, add } => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.select_edge_loop(item, (a, b), add)?;
                // SYM3-30: the mirror loop is added too.
                if let Some(crate::Component::Edge(x, y)) =
                    self.mirror_pick(item, crate::Component::Edge(a, b))
                    && !self
                        .scene
                        .selection
                        .edges_of(item)
                        .iter()
                        .any(|&(p, q)| (p.min(q), p.max(q)) == (x.min(y), x.max(y)))
                {
                    self.scene.select_edge_loop(item, (x, y), true)?;
                }
                if self.scene != before {
                    self.record_selection("Select Loop".into(), before);
                }
            }
            Command::SelectConnectedPolygons { item, face, add } => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.select_connected_polygons(item, face, add)?;
                // SYM3-30: the mirror piece is added too.
                if let Some(crate::Component::Polygon(g)) =
                    self.mirror_pick(item, crate::Component::Polygon(face))
                    && !self.scene.selection.faces_of(item).contains(&g)
                {
                    self.scene.select_connected_polygons(item, g, true)?;
                }
                if self.scene != before {
                    self.record_selection("Select Connected".into(), before);
                }
            }
            Command::ClearSelection => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.clear_components();
                if self.selection_start.is_none() && self.scene != before {
                    self.record_selection("Clear Selection".into(), before);
                }
            }
            Command::Workplane(plane) => {
                self.require_idle()?;
                if plane.plane > 2 || !plane.center.is_finite() {
                    return Err("Invalid construction plane".into());
                }
                self.scene.workplane = plane;
            }
            Command::SelectItem(id) => self.handle_select_item(id)?,
            Command::SelectGroup(group) => {
                self.require_idle()?;
                let members = self.scene.group_members(group);
                if self.scene.group(group).is_none() {
                    return Err("Missing group".into());
                }
                self.scene.select_item(members.first().copied());
                self.scene.selection.also = members.iter().skip(1).copied().collect();
            }
            Command::SelectItems(ids) => {
                self.require_idle()?;
                if ids.iter().any(|id| !self.scene.item(*id).is_some()) {
                    return Err("Missing item".into());
                }
                let mut ids = ids;
                let mut seen = std::collections::HashSet::new();
                ids.retain(|id| seen.insert(*id));
                self.scene.select_item(ids.first().copied());
                self.scene.selection.also = ids.into_iter().skip(1).collect();
            }
            Command::ToggleItemSelection(id) => {
                self.require_idle()?;
                if !self.scene.item(id).is_some() {
                    return Err("Missing item".into());
                }
                self.scene.toggle_item(id);
            }
            cmd => {
                self.require_idle()?;
                let before = self.scene.clone();
                let label = match cmd {
                    Command::NewMesh | Command::NewMeshBeside | Command::NewNamedMesh(_) => {
                        let name = match &cmd {
                            Command::NewNamedMesh(name) => Some(name.trim().to_owned()),
                            _ => None,
                        };
                        let next = self.scene.next_id.max(self.next_item_id);
                        if next >= u64::MAX - 1 || self.scene.items.len() >= crate::MAX_SCENE_ITEMS
                        {
                            return Err("Scene item/identity budget exhausted".into());
                        }
                        // Item list New: the selected item's parent (NM08).
                        let parent = match cmd {
                            Command::NewMeshBeside => self
                                .scene
                                .selection
                                .item
                                .and_then(|id| self.scene.item(id))
                                .and_then(|i| i.parent),
                            _ => None,
                        };
                        self.scene.next_id = next;
                        self.scene.new_mesh_named(name)?;
                        if let Some(item) = self.scene.items.last_mut() {
                            item.parent = parent;
                        }
                        self.next_item_id = self.scene.next_id;
                        "New Mesh"
                    }
                    Command::NewBackdrop => {
                        let next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        self.scene
                            .new_backdrop(crate::backdrop::Backdrop::new_default())?;
                        self.next_item_id = self.scene.next_id;
                        "New Backdrop"
                    }
                    Command::BackdropLoadImage {
                        item,
                        path,
                        width,
                        height,
                    } => {
                        backdrop_mut(&mut self.scene, item)?;
                        let image = self.scene.image_for_path(&path, width, height)?;
                        backdrop_mut(&mut self.scene, item)?.image = Some(image);
                        "Backdrop Image"
                    }
                    Command::EditBackdrop { item, edit } => {
                        if let crate::backdrop::BackdropEdit::Image(Some(image)) = &edit
                            && self.scene.image(*image).is_none()
                        {
                            return Err("Choose a loaded image".into());
                        }
                        backdrop_mut(&mut self.scene, item)?.apply(edit)?;
                        "Backdrop Setting"
                    }
                    Command::BackdropAutoFit(item) => {
                        self.scene.refuse_locked(&[item])?;
                        // BD-61: the active meshes' combined box.
                        let meshes = self.scene.auto_fit_meshes();
                        let bounds = self
                            .scene
                            .mesh_bounds(&meshes)
                            .ok_or(crate::backdrop::AUTO_FIT_NO_MESH)?;
                        self.scene.auto_fit_backdrop(item, bounds)?;
                        "Auto Fit"
                    }
                    Command::AddBackdrop {
                        name,
                        image,
                        projection,
                    } => {
                        if name.as_deref().is_some_and(|n| n.trim().is_empty()) {
                            return Err("A Backdrop name can't be empty".into());
                        }
                        let next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        let image = match image {
                            Some((path, width, height)) => {
                                Some(self.scene.image_for_path(&path, width, height)?)
                            }
                            None => None,
                        };
                        let settings = crate::backdrop::Backdrop {
                            image,
                            projection,
                            ..crate::backdrop::Backdrop::new_default()
                        };
                        self.scene.new_backdrop(settings)?;
                        if let (Some(name), Some(item)) = (name, self.scene.items.last_mut()) {
                            item.name = name;
                        }
                        self.next_item_id = self.scene.next_id;
                        "Add Backdrop"
                    }
                    Command::LoadImage {
                        path,
                        width,
                        height,
                    } => {
                        self.scene.image_for_path(&path, width, height)?;
                        "Load Image"
                    }
                    Command::BackdropSize {
                        item,
                        width,
                        height,
                    } => {
                        self.scene.refuse_locked(&[item])?;
                        self.scene.size_backdrop(item, width, height)?;
                        "Backdrop Size"
                    }
                    Command::AddItem(kind) => {
                        use crate::item_list::AddItemKind;
                        let next = self.scene.next_id.max(self.next_item_id);
                        if next >= u64::MAX - 1 || self.scene.items.len() >= crate::MAX_SCENE_ITEMS
                        {
                            return Err("Scene item/identity budget exhausted".into());
                        }
                        self.scene.next_id = next;
                        let anchor = self.scene.selection.item;
                        let render_camera = self.scene.render_camera();
                        match kind {
                            AddItemKind::Mesh => self.scene.new_mesh()?,
                            AddItemKind::Backdrop => {
                                self.scene.new_backdrop(Default::default())?;
                            }
                            AddItemKind::Camera => {
                                self.scene
                                    .add_camera(Default::default(), Default::default())?;
                            }
                            AddItemKind::Light(light) => {
                                self.scene.add_light(
                                    crate::camera_light::LightSettings::new(light),
                                    Default::default(),
                                )?;
                            }
                            AddItemKind::GroupLocator => {
                                self.scene.add_group_locator()?;
                            }
                            AddItemKind::Environment => {
                                self.scene.add_environment()?;
                            }
                        }
                        let id = self.scene.items.last().map(|i| i.id);
                        if kind != AddItemKind::Environment {
                            self.scene.place_new_item(anchor, render_camera);
                        }
                        self.scene.select_item(id);
                        self.next_item_id = self.scene.next_id;
                        // The same step names as the older New commands.
                        match kind {
                            AddItemKind::Mesh => "New Mesh",
                            AddItemKind::Backdrop => "New Backdrop",
                            AddItemKind::Camera => "New Camera",
                            AddItemKind::Light(_) => "New Light",
                            AddItemKind::GroupLocator => "New Group Locator",
                            AddItemKind::Environment => "New Environment",
                        }
                    }
                    Command::ItemPattern { query, action } => {
                        use crate::item_list::PatternAction;
                        let matches = self.scene.pattern_matches(&query);
                        if matches.is_empty() {
                            return Err("No items match the pattern".into());
                        }
                        match action {
                            PatternAction::Select => {
                                self.scene.select_item(Some(matches[0]));
                                self.scene.selection.also = matches[1..].to_vec();
                            }
                            PatternAction::Unselect => {
                                let before = self.scene.selected_items();
                                let rest: Vec<u64> = before
                                    .iter()
                                    .copied()
                                    .filter(|id| !matches.contains(id))
                                    .collect();
                                if rest.len() == before.len() {
                                    return Err("No selected items match the pattern".into());
                                }
                                if rest.first() != self.scene.selection.item.as_ref() {
                                    // The active item went: the next one left
                                    // becomes active, components cleared.
                                    self.scene.select_item(rest.first().copied());
                                }
                                self.scene.selection.also = rest.iter().skip(1).copied().collect();
                            }
                            PatternAction::Hide => {
                                let mut changed = false;
                                for item in &mut self.scene.items {
                                    if item.visible && matches.contains(&item.id) {
                                        item.visible = false;
                                        changed = true;
                                    }
                                }
                                if !changed {
                                    return Err("The matching items are hidden already".into());
                                }
                            }
                        }
                        match action {
                            PatternAction::Select => "Select Items",
                            PatternAction::Unselect => "Unselect Items",
                            PatternAction::Hide => "Hide Items",
                        }
                    }
                    Command::EditEnvironmentLayers { item, edit } => {
                        let label = edit.label();
                        let mut next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        self.scene.edit_environment_layers(item, edit)?;
                        next = self.scene.next_id;
                        self.next_item_id = next;
                        label
                    }
                    Command::EditEnvironment { item, edit } => {
                        if let crate::environment::EnvironmentEdit::SunLight(Some(sun)) = &edit {
                            let directional = self.scene.items.iter().any(|i| {
                                i.id == *sun
                                    && i.light.as_ref().is_some_and(|l| {
                                        l.kind == crate::camera_light::LightKind::Directional
                                    })
                            });
                            if !directional {
                                return Err("Sun Light must be a directional light".into());
                            }
                        }
                        let env = self
                            .scene
                            .item_mut(item)
                            .and_then(|i| i.environment.as_mut())
                            .ok_or("Select an environment")?;
                        edit.apply(env)?;
                        edit.label()
                    }
                    Command::NewCamera | Command::NewLight(_) => {
                        let next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        let id = match cmd {
                            Command::NewLight(kind) => self.scene.add_light(
                                crate::camera_light::LightSettings::new(kind),
                                Default::default(),
                            )?,
                            _ => self
                                .scene
                                .add_camera(Default::default(), Default::default())?,
                        };
                        self.next_item_id = self.scene.next_id;
                        self.scene.select_item(Some(id));
                        if matches!(cmd, Command::NewCamera) {
                            "New Camera"
                        } else {
                            "New Light"
                        }
                    }
                    Command::EditCamera { item, edit } => {
                        let label = edit.label();
                        if !self
                            .scene
                            .items
                            .iter()
                            .any(|i| i.id == item && i.camera.is_some())
                        {
                            return Err("Select a camera".into());
                        }
                        if let crate::camera_light::CameraEdit::BackgroundImage(Some(image)) = &edit
                            && self.scene.image(*image).is_none()
                        {
                            return Err("Choose a loaded image".into());
                        }
                        // CP-X3: the other selected cameras follow; all or
                        // nothing.
                        let mut targets = vec![item];
                        let selected = self.scene.selected_items();
                        if selected.contains(&item) {
                            targets.extend(selected);
                        }
                        let mut next = Vec::new();
                        for (index, target) in self.scene.items.iter().enumerate() {
                            if let Some(camera) = target
                                .camera
                                .as_ref()
                                .filter(|_| targets.contains(&target.id))
                            {
                                let mut camera = camera.clone();
                                camera.apply(edit.clone())?;
                                next.push((index, camera));
                            }
                        }
                        for (index, camera) in next {
                            self.scene.items[index].camera = Some(camera);
                        }
                        label
                    }
                    Command::CameraAutofocus(item) => {
                        let d = self.scene.autofocus_distance(item)?;
                        self.scene
                            .item_mut(item)
                            .and_then(|i| i.camera.as_mut())
                            .ok_or("Select a camera")?
                            .apply(crate::camera_light::CameraEdit::FocusDistance(d))?;
                        "Autofocus"
                    }
                    Command::CameraBackgroundLoad {
                        item,
                        path,
                        width,
                        height,
                    } => {
                        if !self
                            .scene
                            .items
                            .iter()
                            .any(|i| i.id == item && i.camera.is_some())
                        {
                            return Err("Select a camera".into());
                        }
                        let image = self.scene.image_for_path(&path, width, height)?;
                        let camera = self
                            .scene
                            .item_mut(item)
                            .and_then(|i| i.camera.as_mut())
                            .ok_or("Select a camera")?;
                        camera.background.image = Some(image);
                        camera.background.enable = true;
                        "Background Image"
                    }
                    Command::SyncCameraToView {
                        item,
                        transform,
                        target_distance,
                    } => {
                        self.scene.refuse_locked(&[item])?;
                        let target = self
                            .scene
                            .items
                            .iter_mut()
                            .find(|i| i.id == item && i.camera.is_some())
                            .ok_or("Select a camera")?;
                        target.transform = transform;
                        target.camera.as_mut().ok_or("Select a camera")?.apply(
                            crate::camera_light::CameraEdit::TargetDistance(target_distance),
                        )?;
                        "Sync to View"
                    }
                    Command::EditLight { item, edit } => {
                        let label = edit.label();
                        if !self
                            .scene
                            .items
                            .iter()
                            .any(|i| i.id == item && i.light.is_some())
                        {
                            return Err("Select a light".into());
                        }
                        // LP-31: an Item Group is one of the scene's groups.
                        if let crate::camera_light::LightEdit::LinkGroup(Some(g)) = &edit
                            && self.scene.group(*g).is_none()
                        {
                            return Err("Choose one of the scene's groups".into());
                        }
                        // LP-MS-01: a prototype is a mesh item.
                        if let crate::camera_light::LightEdit::Prototype(Some(p)) = &edit
                            && !self.scene.items.iter().any(|i| i.id == *p && i.is_mesh())
                        {
                            return Err("Choose a mesh item as the prototype".into());
                        }
                        // LP-51: the other selected lights follow; all or
                        // nothing.
                        let mut targets = vec![item];
                        let selected = self.scene.selected_items();
                        if selected.contains(&item) {
                            targets.extend(selected);
                        }
                        let mut next = Vec::new();
                        for (index, target) in self.scene.items.iter().enumerate() {
                            if let Some(light) = target
                                .light
                                .as_ref()
                                .filter(|_| targets.contains(&target.id))
                            {
                                let mut light = light.clone();
                                light.apply(edit.clone())?;
                                next.push((index, light));
                            }
                        }
                        for (index, light) in next {
                            self.scene.items[index].light = Some(light);
                        }
                        label
                    }
                    Command::NewGroup { name, members } => {
                        let next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        self.scene.new_group(name.as_deref(), &members)?;
                        self.next_item_id = self.scene.next_id;
                        "New Group"
                    }
                    Command::RenameGroup { group, name } => {
                        self.scene.rename_group(group, &name)?;
                        "Rename Group"
                    }
                    Command::DeleteGroup(group) => {
                        self.scene.delete_group(group)?;
                        "Delete Group"
                    }
                    Command::AddToGroup { group, items } => {
                        self.scene.add_to_group(group, &items)?;
                        "Add to Group"
                    }
                    Command::RemoveFromGroup { group, items } => {
                        self.scene.remove_from_group(group, &items)?;
                        "Remove from Group"
                    }
                    Command::LoadPhotometry { item, path } => {
                        let file =
                            crate::photometry::Photometry::load(std::path::Path::new(&path))?;
                        let target = self
                            .scene
                            .item_mut(item)
                            .and_then(|i| i.light.as_mut())
                            .filter(|l| l.kind == crate::camera_light::LightKind::Photometric)
                            .ok_or("Select a photometric light")?;
                        target.apply(crate::camera_light::LightEdit::Photometry {
                            path,
                            intensity: file.radiant_intensity(),
                            width: file.width,
                            height: file.length,
                        })?;
                        "Photometry File"
                    }
                    Command::EditItemCommon { items, edit } => {
                        self.scene.apply_item_common(&items, &edit)?;
                        edit.label()
                    }
                    Command::SetRenderCamera(item) => {
                        self.scene.set_render_camera(item)?;
                        "Render Camera"
                    }
                    Command::CameraView {
                        item,
                        transform,
                        target_distance,
                    } => {
                        let target = self
                            .scene
                            .items
                            .iter_mut()
                            .find(|i| i.id == item && (i.camera.is_some() || i.light.is_some()))
                            .ok_or("Select a camera or light")?;
                        if target.is_locked() {
                            return Err(format!("{} is locked (Assembly > Lock)", target.name));
                        }
                        // Check the Target Distance before moving anything, so
                        // an error leaves the item where it was.
                        let camera = match target.camera.clone() {
                            Some(mut c) => {
                                c.apply(crate::camera_light::CameraEdit::TargetDistance(
                                    target_distance,
                                ))?;
                                Some(c)
                            }
                            None => None,
                        };
                        target.transform = transform;
                        match camera {
                            Some(c) => {
                                target.camera = Some(c);
                                "Camera View"
                            }
                            None => "Light View",
                        }
                    }
                    Command::DeleteItem => {
                        let id = self
                            .scene
                            .selection
                            .item
                            .ok_or("Select an item to delete")?;
                        let locators = self.scene.environment_locators(&[id]);
                        if self.scene.any_has_children(&[id]) {
                            // Scripts' default (provisional, GL-Q6).
                            self.scene.delete_items(&[id], Default::default())?;
                        } else {
                            self.scene.items.retain(|i| i.id != id);
                            self.scene.select_item(None);
                        }
                        self.scene.drop_environment_locators(&locators);
                        "Delete Mesh"
                    }
                    Command::DeleteItems(children) => {
                        let ids = self.scene.selected_items();
                        let locators = self.scene.environment_locators(&ids);
                        let sheared = self.scene.delete_items(&ids, children)?;
                        self.command_note = crate::item_delete::sheared_note(&sheared);
                        self.scene.drop_environment_locators(&locators);
                        "Delete Item"
                    }
                    Command::InstanceItems => {
                        crate::duplicate_items::instance_selected(
                            &mut self.scene,
                            self.next_item_id,
                        )?;
                        self.next_item_id = self.scene.next_id;
                        "Instance"
                    }
                    Command::DuplicateHierarchy => {
                        self.duplicate_hierarchy()?;
                        "Duplicate Hierarchy"
                    }
                    Command::DuplicateItem => {
                        let id = self
                            .scene
                            .selection
                            .item
                            .ok_or("Select an item to duplicate")?;
                        let next = self.scene.next_id.max(self.next_item_id);
                        if next >= u64::MAX - 1 || self.scene.items.len() >= crate::MAX_SCENE_ITEMS
                        {
                            return Err("Scene item/identity budget exhausted".into());
                        }
                        let source = self.scene.item(id).ok_or("Select an item to duplicate")?;
                        let mut copy = source.clone();
                        let base = source.name.clone();
                        let name = (2..)
                            .map(|n| format!("{base} ({n})"))
                            .find(|name| self.scene.items.iter().all(|i| &i.name != name))
                            .unwrap_or(base);
                        copy.id = next;
                        copy.name = name;
                        copy.source_id = None;
                        copy.source_display_name = None;
                        // ENV §6: a copied environment gets its own layer
                        // ids and Environment Material name.
                        if let Some(env) = copy.environment.as_mut() {
                            self.scene.renumber_environment(env)?;
                        }
                        self.scene.items.push(copy);
                        self.scene.next_id = next + 1;
                        self.next_item_id = self.scene.next_id;
                        self.scene.select_item(Some(next));
                        "Duplicate Item"
                    }
                    Command::InsertItems {
                        mut items,
                        parent,
                        images,
                    } => {
                        let next = self.scene.next_id.max(self.next_item_id);
                        if items.is_empty() {
                            return Err("Nothing to insert".into());
                        }
                        if next >= u64::MAX - items.len() as u64
                            || self.scene.items.len() + items.len() > crate::MAX_SCENE_ITEMS
                        {
                            return Err("Scene item/identity budget exhausted".into());
                        }
                        if parent.is_some_and(|p| self.scene.items.iter().all(|i| i.id != p)) {
                            return Err("Choose an existing parent item".into());
                        }
                        // Backdrops keep their pictures: the same file is the
                        // same entry here.
                        let mut scene = self.scene.clone();
                        for item in &mut items {
                            let Some(b) = item.backdrop.as_mut() else {
                                continue;
                            };
                            b.image = match b.image.and_then(|i| images.iter().find(|s| s.id == i))
                            {
                                Some(source) => Some(scene.image_for_path(
                                    &source.path,
                                    source.width,
                                    source.height,
                                )?),
                                None => None,
                            };
                        }
                        self.scene.images = scene.images;
                        let ids: std::collections::BTreeMap<u64, u64> = items
                            .iter()
                            .enumerate()
                            .map(|(k, item)| (item.id, next + k as u64))
                            .collect();
                        let root = ids[&items[0].id];
                        for item in items {
                            let mut copy = item.clone();
                            copy.id = ids[&item.id];
                            copy.parent = item.parent.and_then(|p| ids.get(&p).copied());
                            copy.instance_of = item.instance_of.and_then(|p| ids.get(&p).copied());
                            let base = item.name.clone();
                            if self.scene.items.iter().any(|i| i.name == base) {
                                copy.name = (2..)
                                    .map(|n| format!("{base} ({n})"))
                                    .find(|name| self.scene.items.iter().all(|i| &i.name != name))
                                    .unwrap_or(base);
                            }
                            copy.source_id = None;
                            copy.source_display_name = None;
                            self.scene.items.push(copy);
                        }
                        self.scene.next_id = next + ids.len() as u64;
                        self.next_item_id = self.scene.next_id;
                        if parent.is_some() {
                            // Keep the world position; a parent that would
                            // need shear keeps the transform instead.
                            if self.scene.set_parent(root, parent, true).is_err() {
                                self.scene.set_parent(root, parent, false)?;
                            }
                        }
                        self.scene.select_item(Some(root));
                        "Insert Items"
                    }
                    Command::RemoveBranch(id) => {
                        let branch = self.scene.branch(id)?;
                        let gone: std::collections::BTreeSet<u64> =
                            branch.iter().map(|i| i.id).collect();
                        self.scene.items.retain(|i| !gone.contains(&i.id));
                        if self.scene.selection.item.is_some_and(|i| gone.contains(&i)) {
                            self.scene.select_item(None);
                        }
                        "Move Items Out"
                    }
                    Command::ItemTransform { item, transform } => {
                        self.scene.refuse_locked(&[item])?;
                        let target = self.scene.item_mut(item).ok_or("Select an item")?;
                        // IT-PHY-01: a new rotation order keeps the Euler
                        // values, so the item's orientation changes.
                        target.transform = transform;
                        // Planar spec: a Texture Locator's Size stops at 1 mm.
                        if target.texture_locator().is_some() {
                            target.transform.scale =
                                crate::texture_locator::clamp_size(target.transform.scale);
                        }
                        "Item Transform"
                    }
                    Command::Rename(name) => {
                        if name.trim().is_empty() || name.len() > 1024 {
                            return Err("Name must contain 1–1024 bytes".into());
                        }
                        let item = self.scene.active_mut().ok_or("Select an item")?;
                        let (id, old) = (item.id, item.name.clone());
                        item.name = name.trim().to_owned();
                        // SK-23: a joint's weight maps follow its name.
                        crate::skin_weights::follow_rename(&mut self.scene, id, &old);
                        "Rename Mesh"
                    }
                    Command::SetParent {
                        item,
                        parent,
                        keep_position,
                    } => {
                        // LCK-05: a locked item keeps its links.
                        self.scene.refuse_locked(&[item])?;
                        self.scene.set_parent(item, parent, keep_position)?;
                        if parent.is_some() {
                            "Parent"
                        } else {
                            "Unparent"
                        }
                    }
                    Command::ParentToGroupLocator => {
                        // LCK-05: a locked item keeps its links.
                        self.scene.refuse_locked(&self.scene.selected_items())?;
                        let next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        self.scene.parent_to_group_locator()?;
                        self.next_item_id = self.scene.next_id;
                        crate::locator::PARENT_TO_GROUP_LOCATOR
                    }
                    Command::MoveItems {
                        items,
                        parent,
                        before,
                    } => {
                        if self.scene.move_items(&items, parent, before)? {
                            if parent.is_some() {
                                "Parent"
                            } else {
                                "Unparent"
                            }
                        } else {
                            "Reorder"
                        }
                    }
                    Command::DropAction {
                        action,
                        sources,
                        destination,
                        ray,
                    } => {
                        self.scene.drop_action(action, &sources, destination, ray)?;
                        action.label()
                    }
                    Command::EditElements { kind, op } => {
                        self.scene.edit_elements(kind, op)?;
                        kind.label(op)
                    }
                    Command::Visibility(id) => {
                        // LCK-05: a locked item's eye is refused.
                        self.scene.refuse_locked(&[id])?;
                        let i = self.scene.item_mut(id).ok_or("Missing item")?;
                        i.visible = !i.visible;
                        // A plain toggle hides only the item (GV-03).
                        i.hides_children = false;
                        "Mesh Visibility"
                    }
                    Command::SetItemsVisible {
                        items,
                        visible,
                        hides_children,
                    } => {
                        let free: Vec<u64> = items
                            .iter()
                            .copied()
                            .filter(|id| self.scene.refuse_locked(&[*id]).is_ok())
                            .collect();
                        if free.is_empty() {
                            self.scene.refuse_locked(&items)?;
                            return Err("No items to show or hide".into());
                        }
                        for item in &mut self.scene.items {
                            if free.contains(&item.id) {
                                item.visible = visible;
                                item.hides_children = !visible && hides_children;
                            }
                        }
                        "Mesh Visibility"
                    }
                    Command::VisibilityWithChildren(id) => {
                        self.scene.refuse_locked(&[id])?;
                        let i = self.scene.item_mut(id).ok_or("Missing item")?;
                        i.visible = !i.visible;
                        let visible = i.visible;
                        let mut family = vec![id];
                        let mut at = 0;
                        while let Some(&parent) = family.get(at) {
                            at += 1;
                            for child in
                                self.scene.items.iter().filter(|i| i.parent == Some(parent))
                            {
                                family.push(child.id);
                            }
                        }
                        for item in &mut self.scene.items {
                            if family.contains(&item.id) {
                                item.visible = visible;
                                item.hides_children = false;
                            }
                        }
                        "Mesh Visibility"
                    }
                    _ => return Err("Unsupported command".into()),
                };
                // Deleted items leave every group.
                self.scene.prune_groups();
                if let Err(e) = self.scene.validate() {
                    self.scene = before;
                    return Err(e);
                }
                if before != self.scene {
                    self.record(label.into(), before);
                }
            }
        }
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
}
