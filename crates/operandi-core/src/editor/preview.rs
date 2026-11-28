//! The live tool's preview: `preview` and the per-tool geometry arm
//! `preview_one`.
use super::*;

impl Editor {
    pub(crate) fn preview(&mut self, p: Parameters, apply: bool) -> Result<()> {
        // Bend's centre is the action centre: the frame follows `bend.base`
        // however it changed (panel fields, a click, undo inside the tool).
        let bend_center = (self
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.tool == Tool::Bend))
        .then_some(p.bend.base);
        let result = self.preview_tool(p, apply);
        if result.is_ok()
            && let Some(center) = bend_center
            && let Some(frame) = self
                .session
                .live
                .as_mut()
                .and_then(|l| l.action_frame.as_mut())
        {
            frame.center = center;
        }
        // Back to the mode the selection was made in, with that selection
        // (SI-05/06), whether or not the preview worked.
        self.keep_inferred_source();
        result?;
        // Constrain to Background acts after the tool's own change.
        self.constrain_preview()
    }
    pub(super) fn preview_tool(&mut self, p: Parameters, apply: bool) -> Result<()> {
        let live = self.session.live.as_ref().ok_or("No active tool")?;
        // Started from a UV view: the pipe runs on the map's UVs (UVT-130,
        // uv_pipe.rs).
        if p.uv.is_some() && crate::uv_pipe::runs_on_uvs(live.tool) {
            return self.preview_uv(p, apply);
        }
        // Several active meshes: the tool runs on each (multi_tools.rs).
        if let Some(targets) = Self::tool_mesh_targets(live.tool, &live.base) {
            return self.preview_on_meshes(&targets, p, apply);
        }
        // Tweak grabs an element of any active mesh and edits that mesh.
        if live.tool == Tool::Tweak
            && let Some(grab) = p.tweak
            && live.base.is_other_active_mesh(grab.item)
        {
            return self.preview_on_meshes(&[grab.item], p, apply);
        }
        self.preview_one(p, apply)
    }
    /// One live tool preview on the primary mesh.
    pub(crate) fn preview_one(&mut self, mut p: Parameters, apply: bool) -> Result<()> {
        let inferred = self.inferred_mode();
        let live = self.session.live.as_ref().ok_or("No active tool")?;
        let mut scene = live.base.clone();
        // An inferred start: the tool and every layer see its own mode.
        if let Some(mode) = inferred {
            scene.selection.mode = mode;
        }
        // Twist Bevel's note about a group across the symmetry plane.
        let mut twist_note: Option<String> = None;
        match live.tool {
            Tool::Sphere | Tool::Cylinder => {
                let radial = p.radial.ok_or("Missing primitive Radius/resolution")?;
                // The flat first-draw base is a real disc that Drop keeps
                // (drag-creation-2026-11-06 rules 1 and 4; `flat_disc`).
                if !radial.radius.is_finite() || radial.radius.min_element() < 0. {
                    return Err("Radius must be finite and nonnegative".into());
                }
                let item = scene.active().ok_or("Select a mesh")?;
                if !scene.shows(item) {
                    return Err(
                        "The selected mesh is hidden; show it before creating geometry".into(),
                    );
                }
                // #15: the tool falloff shapes the primitive as it's made.
                let weights = self.primitive_weights();
                let weigh = weights
                    .as_ref()
                    .map(|w| w.weigher(scene.world_to_local(item.id).unwrap_or_default()));
                let weight = weigh.as_ref().map(|f| f as &dyn Fn(DVec3) -> f64);
                let mesh = crate::user_defaults::primitive_symmetry(self.symmetry)
                    .doubled(radial_mesh(live.tool, p.position, radial, &p, weight)?)?;
                // Drawn in world space; the mesh is local.
                let mesh = scene.mesh_world_to_local(item.id, mesh)?;
                crate::append_geometry(&mut scene.active_mut().ok_or("Select a mesh")?.mesh, mesh)?;
            }
            Tool::Box => {
                if !apply && !live.applied && p.cube_shape() == live.params.cube_shape() {
                    self.session
                        .live
                        .as_mut()
                        .ok_or("No active primitive")?
                        .params = p;
                    return Ok(());
                }
                // The Cube's one generator, shared with its MeshOp (#14),
                // shaped by the tool falloff (#15).
                let to_local = scene
                    .active()
                    .map(|i| scene.world_to_local(i.id).unwrap_or_default())
                    .unwrap_or_default();
                let weights = self.primitive_weights();
                let weigh = weights.as_ref().map(|w| w.weigher(to_local));
                let mesh = p
                    .cube_shape()
                    .mesh_weighted(weigh.as_ref().map(|f| f as &dyn Fn(DVec3) -> f64))?;
                // SYM-PR-01/02.
                let mesh = crate::user_defaults::primitive_symmetry(self.active_symmetry())
                    .doubled(mesh)?;
                let item = scene.active().ok_or("Select an item")?;
                if !scene.shows(item) {
                    return Err(
                        "The selected mesh is hidden; show it before creating geometry".into(),
                    );
                }
                // Drawn in world space (UVs too); the mesh is local.
                let mesh = scene.mesh_world_to_local(item.id, mesh)?;
                let item = scene.active_mut().ok_or("Select an item")?;
                crate::primitive::append_geometry(&mut item.mesh, mesh)?;
            }
            Tool::EdgeExtend
                if self.active_symmetry().axis.is_some()
                    && (p.rotation_delta != DVec3::ZERO || p.scale_delta != DVec3::ONE) =>
            {
                // N27 (Jason, 2026-11-17): with Symmetry, Rotate and Scale
                // turn each half about its own centre, the far half
                // mirrored, instead of both about the point between them.
                let frame = live.action_frame.ok_or("Missing action frame")?;
                let at_zero = self.session.extend_from_click;
                let symmetry = self.active_symmetry();
                crate::symmetry_sides::two_pass(&mut scene, symmetry, |scene, mirrored| {
                    let edges = scene.selection.edges.clone();
                    let item = scene.active().ok_or("Select a mesh for Extend")?;
                    let world = scene.world_matrix(item.id)?;
                    let mut points: Vec<u32> = edges.iter().flat_map(|&(a, b)| [a, b]).collect();
                    points.sort_unstable();
                    points.dedup();
                    let centre = points
                        .iter()
                        .filter_map(|&v| item.mesh.vertices.get(v as usize))
                        .map(|&q| world.transform_point3(q))
                        .sum::<DVec3>()
                        / points.len().max(1) as f64;
                    let mirror = crate::symmetry_sides::WorldMirror::new(symmetry, world)
                        .filter(|_| mirrored);
                    let (basis, offset) = match &mirror {
                        // A mirrored basis gives the mirror image of the
                        // same rotate and scale (M·B·R·S·Bᵀ·M).
                        Some(m) => (
                            glam::DMat3::from_mat4(m.matrix) * frame.basis,
                            m.vector(frame.basis * p.delta),
                        ),
                        None => (frame.basis, frame.basis * p.delta),
                    };
                    let shape = crate::ExtendShape {
                        offset,
                        inset: p.inset,
                        shift: p.shift,
                        rotate: p.rotation_delta,
                        scale: p.scale_delta,
                        frame: crate::ActionFrame {
                            center: centre,
                            basis,
                        },
                        segments: p.segments[0],
                        make_uvs: p.edge_extend.make_uvs,
                    };
                    let (mesh, outer) = if at_zero {
                        crate::edge_extend_at_zero(&item.mesh, &edges, world, &shape)?
                    } else {
                        crate::edge_extend_shaped(&item.mesh, &edges, world, &shape)?
                    };
                    scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
                    // PF-12: the new edges' points are the point selection too, so
                    // Vertex mode shows them.
                    scene.selection.vertices = crate::topology::edge_points(&outer);
                    scene.selection.edges = outer;
                    Ok(())
                })?;
            }
            Tool::EdgeExtend => {
                let edges = scene.selection.edges.clone();
                let item = scene.active().ok_or("Select a mesh for Extend")?;
                // Offset is along the action axes (as Move's); the shape
                // works in world space and maps back to the mesh.
                let frame = live.action_frame.ok_or("Missing action frame")?;
                let shape = crate::ExtendShape {
                    offset: frame.basis * p.delta,
                    inset: p.inset,
                    shift: p.shift,
                    rotate: p.rotation_delta,
                    scale: p.scale_delta,
                    frame,
                    segments: p.segments[0],
                    make_uvs: p.edge_extend.make_uvs,
                };
                let world = scene.world_matrix(item.id)?;
                let (mut mesh, outer) = if self.session.extend_from_click {
                    crate::edge_extend_at_zero(&item.mesh, &edges, world, &shape)?
                } else {
                    crate::edge_extend_shaped(&item.mesh, &edges, world, &shape)?
                };
                // SYM3-12: the offset is mirrored for the partner side.
                let symmetry = self.active_symmetry();
                if symmetry.axis.is_some() && shape.offset != DVec3::ZERO {
                    let primary = self.symmetry_primary(&scene);
                    let v = world.inverse().transform_vector3(shape.offset);
                    let moved: Vec<u32> = outer.iter().flat_map(|&(a, b)| [a, b]).collect();
                    let tolerance = crate::symmetry::match_tolerance(&item.mesh);
                    symmetry.mirror_offset(&mut mesh.vertices, &moved, v, primary, tolerance);
                }
                scene.active_mut().unwrap().mesh = mesh.into();
                // PF-12: the new edges' points are the point selection too, so
                // Vertex mode shows them.
                scene.selection.vertices = crate::topology::edge_points(&outer);
                scene.selection.edges = outer;
            }
            Tool::EdgeCrease => {
                let edges = scene.selection.edges.clone();
                let item = scene.active().ok_or("Select a mesh for Edge Weight")?;
                let mesh = crate::edge_crease_geometry(
                    &item.mesh,
                    &edges,
                    p.shift,
                    p.edge_weight_additive,
                )?;
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::EdgeSlide => {
                let edges = scene.selection.edges.clone();
                let item = scene.active().ok_or("Select a mesh for Slide")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                let (mesh, after) = crate::edge_slide_extra(
                    &item.mesh,
                    &edges,
                    p.shift,
                    p.edge_slide_distance,
                    weights.as_deref(),
                    &p.slide_extras,
                )?;
                let merged = mesh.vertices.len() != item.mesh.vertices.len();
                let mut mesh = mesh;
                if !merged {
                    // SYM3-12: the slide direction is mirrored for the
                    // partner side.
                    let symmetry = self.active_symmetry();
                    let primary = self.symmetry_primary(&scene);
                    symmetry.mirror_moved(&item.mesh, &mut mesh, primary);
                }
                scene.active_mut().unwrap().mesh = mesh.into();
                if merged {
                    // ES-04: merged points renumber the mesh.
                    scene.selection.edges = after;
                    scene.selection.vertices.clear();
                    scene.selection.faces.clear();
                }
            }
            Tool::EdgeExtrude => {
                let edges = scene.selection.edges.clone();
                let item = scene.active().ok_or("Select a mesh for Edge Extrude")?;
                // Use Material tags the ridge quads (extrude-2026-11-05, EE08).
                let material = p.vertex_use_material.then_some(p.vertex_material.as_str());
                let (mesh, raised) =
                    crate::edge_extrude_with(&item.mesh, &edges, p.shift, p.inset, material)?;
                scene.active_mut().unwrap().mesh = mesh.into();
                // The ridge edges and their points are selected after.
                scene.selection.vertices = crate::topology::edge_points(&raised);
                scene.selection.edges = raised;
            }
            Tool::EdgeBevel | Tool::EdgeChamfer => {
                let item = scene.active().ok_or("Select a mesh for Edge Bevel")?;
                // TA-06: nothing selected bevels every edge (Edge Bevel only;
                // activation refuses Edge Chamfer without a selection).
                let edges = if live.tool == Tool::EdgeBevel && scene.selection.edges.is_empty() {
                    item.mesh.edges()
                } else {
                    scene.selection.edges.clone()
                };
                // Widths are world distances; the mesh is local (uniform
                // scale assumed for the basic profile).
                let scale = crate::scene::uniform_scale(scene.world_matrix(item.id)?);
                let weights = self.falloff_weights(&scene, item.id)?;
                let settings = crate::EdgeBevelSettings {
                    width: p.chamfer.offset / scale,
                    rows: p.chamfer.segments,
                    depth: p.chamfer.depth,
                    chamfer: live.tool == Tool::EdgeChamfer,
                    options: p.edge_bevel.clone(),
                };
                let result =
                    crate::edge_bevel_with(&item.mesh, &edges, &settings, weights.as_deref())?;
                scene.active_mut().unwrap().mesh = result.mesh.into();
                // EBC-J-01 / EBC-P-02: Edge Bevel selects the strips' edges,
                // Edge Chamfer nothing (the original edges with Square).
                // EBC-B3 oracles: the points selected are the ends of those
                // edges.
                scene.selection.vertices = crate::topology::edge_points(&result.selected_after);
                scene.selection.edges = result.selected_after;
            }
            Tool::VertexCreate => {
                let item = scene.active().ok_or("Select a mesh for Create")?;
                let local = scene.world_matrix(item.id)?.inverse();
                let mut points: Vec<DVec3> = p
                    .sketch_path
                    .iter()
                    .map(|&q| local.transform_point3(q))
                    .collect();
                // Class F, Weld seam (SYM3-22/23): each placed point gets a
                // mirror point; one on the plane (within the matching
                // tolerance) is made once, exactly on it.
                let symmetry = self.active_symmetry();
                if symmetry.axis.is_some() {
                    let tolerance = crate::symmetry::match_tolerance(&item.mesh).max(
                        crate::symmetry::match_tolerance(&crate::Mesh {
                            vertices: points.clone(),
                            ..Default::default()
                        }),
                    );
                    let mut mirrored = Vec::new();
                    for q in &mut points {
                        if symmetry.side(*q, tolerance) == Some(crate::Side::Plane) {
                            *q = symmetry.onto_plane(*q);
                        } else {
                            mirrored.push(symmetry.mirror(*q));
                        }
                    }
                    points.extend(mirrored);
                }
                let (mesh, ids) = crate::add_points(&item.mesh, &points)?;
                scene.active_mut().unwrap().mesh = mesh.into();
                if !ids.is_empty() {
                    scene.selection.vertices = ids;
                }
            }
            Tool::VertexBevel | Tool::VertexExtrude | Tool::VertexMerge | Tool::VertexTear => {
                let mut points = scene.selection.vertices.clone();
                let item = scene.active().ok_or("Select a mesh")?;
                // Vertex Extrude with nothing selected processes every point
                // (extrude-2026-11-05, VE08).
                if points.is_empty() && live.tool == Tool::VertexExtrude {
                    points = (0..item.mesh.vertices.len() as u32).collect();
                }
                let world = scene.world_matrix(item.id)?;
                // Distances are world lengths; the mesh is local (uniform
                // scale assumed for the basic profile).
                let scale = crate::scene::uniform_scale(world);
                let (mesh, selected) = match live.tool {
                    Tool::VertexBevel => crate::vertex_commands::vertex_bevel_round(
                        &item.mesh,
                        &points,
                        p.inset / scale,
                        p.vertex_round_level,
                        self.falloff_weights(&scene, item.id)?.as_deref(),
                    )?,
                    Tool::VertexExtrude => crate::vertex_commands::vertex_extrude_weighted(
                        &item.mesh,
                        &points,
                        p.shift / scale,
                        p.inset / scale,
                        self.falloff_weights(&scene, item.id)?.as_deref(),
                    )?,
                    Tool::VertexMerge => crate::vertex_merge_with(
                        &item.mesh,
                        &points,
                        p.shift / scale,
                        crate::MergeOptions {
                            keep_small: p.vertex_merge_keep_small,
                            test_morphs: p.vertex_merge_test_morphs,
                            ..Default::default()
                        },
                    )?,
                    _ => crate::vertex_tear_geometry(
                        &item.mesh,
                        &points,
                        &scene.selection.faces,
                        world.inverse().transform_vector3(p.delta),
                    )?,
                };
                let mut mesh = mesh;
                // SYM3-12: Tear's new points move by the drag; those torn
                // from the other side take the mirrored drag, seam points
                // keep only the part along the plane (SYM3-13).
                if live.tool == Tool::VertexTear && mesh.vertices.len() > item.mesh.vertices.len() {
                    let symmetry = self.active_symmetry();
                    let primary = self.symmetry_primary(&scene);
                    let torn: Vec<u32> =
                        (item.mesh.vertices.len() as u32..mesh.vertices.len() as u32).collect();
                    let v = world.inverse().transform_vector3(p.delta);
                    let tolerance = crate::symmetry::match_tolerance(&item.mesh);
                    symmetry.mirror_offset(&mut mesh.vertices, &torn, v, primary, tolerance);
                }
                // VTO-T02/T03 Use Material: tag the new polygons (every
                // polygon of a rounded cap, VX-B07).
                if p.vertex_use_material
                    && matches!(live.tool, Tool::VertexBevel | Tool::VertexExtrude)
                {
                    let new = crate::vertex_commands::new_vertex_tool_polygons(&item.mesh, &mesh);
                    crate::vertex_commands::assign_material(&mut mesh, new, &p.vertex_material);
                }
                let same_polygons = item.mesh.faces == mesh.faces;
                scene.active_mut().unwrap().mesh = mesh.into();
                // Tear keeps polygon order, so its polygon selection stays.
                if !same_polygons && live.tool != Tool::VertexTear {
                    scene.selection.edges.clear();
                    scene.selection.faces.clear();
                }
                scene.selection.vertices = selected;
            }
            Tool::LoopSlice
            | Tool::EdgeSlice
            | Tool::PolygonSlice
            | Tool::CurveSlice
            | Tool::PenSlice
            | Tool::PrimitiveSlice
            | Tool::AxisSlice => {
                crate::slice::preview(live.tool, &mut scene, &p.slice, self.active_symmetry())?
            }
            Tool::Contour => crate::contour::preview(&mut scene, &p.contour)?,
            Tool::Mirror
            | Tool::Lathe
            | Tool::Clone
            | Tool::Array
            | Tool::RadialArray
            | Tool::CurveExtrude
            | Tool::CurveClone
            | Tool::ScatterClone
            | Tool::Symmetrize => crate::duplicate::preview(
                live.tool,
                &mut scene,
                &p.duplicate,
                self.next_item_id,
                self.active_symmetry(),
            )?,
            Tool::VertexSlide => {
                let points = scene.selection.vertices.clone();
                let item = scene.active().ok_or("Select a mesh for Slide")?;
                let world = scene.world_matrix(item.id)?;
                let rail = world.inverse().transform_vector3(p.delta);
                // Distance is a world length; the mesh is local (uniform
                // scale assumed, as for the other vertex tools).
                let amount = if p.edge_slide_distance {
                    p.shift / crate::scene::uniform_scale(world)
                } else {
                    p.shift
                };
                let (mesh, selected) = crate::vertex_commands::vertex_slide_extra(
                    &item.mesh,
                    &points,
                    rail,
                    amount,
                    crate::vertex_commands::SlideOptions {
                        by_distance: p.edge_slide_distance,
                        merge: p.vertex_slide_merge,
                    },
                    &p.slide_extras,
                )?;
                let merged = mesh.vertices.len() != item.mesh.vertices.len();
                let mut mesh = mesh;
                if !merged {
                    // SYM3-12: the rail is mirrored for the partner side.
                    let symmetry = self.active_symmetry();
                    let primary = self.symmetry_primary(&scene);
                    symmetry.mirror_moved(&item.mesh, &mut mesh, primary);
                }
                scene.active_mut().unwrap().mesh = mesh.into();
                // Merged points renumber the mesh; edge and polygon picks
                // would point at the wrong components.
                if merged {
                    scene.selection.edges.clear();
                    scene.selection.faces.clear();
                }
                scene.selection.vertices = selected;
            }
            Tool::Push | Tool::Smooth => {
                let points = crate::transform_vertices(&scene)?;
                let item = scene.active().ok_or("Select a mesh")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                let mesh = if live.tool == Tool::Push {
                    // Distance is a world length; the mesh is local (uniform
                    // scale assumed, as for Vertex Extrude).
                    let scale = crate::scene::uniform_scale(scene.world_matrix(item.id)?);
                    let faces = (scene.selection.mode == Mode::Polygon)
                        .then_some(scene.selection.faces.as_slice());
                    // Element Push (hands-on 2026-11-17): with an Element
                    // falloff the picked element's normal is the one
                    // direction, so a polygon moves as one flat piece and
                    // its Range neighbours follow it.
                    let element = self
                        .falloff_element
                        .filter(|g| g.item == item.id)
                        .filter(|_| self.has_element_falloff())
                        .map(|g| crate::deform::element_normal(&item.mesh, g.element));
                    match element {
                        Some(dir) => crate::deform::push_along(
                            &item.mesh,
                            &points,
                            dir,
                            p.shift / scale,
                            weights.as_deref(),
                        )?,
                        None => crate::deform::push_geometry(
                            &item.mesh,
                            &points,
                            p.shift / scale,
                            p.push_normals,
                            faces,
                            weights.as_deref(),
                        )?,
                    }
                } else {
                    crate::deform::smooth_geometry(
                        &item.mesh,
                        &points,
                        &p.smooth,
                        weights.as_deref(),
                    )?
                };
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::UvProject
            | Tool::UvViewProject
            | Tool::UvUnwrap
            | Tool::UvRelax
            | Tool::UvPelt
            | Tool::UvPeeler => {
                let item = scene.active().ok_or("Select a mesh")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                if let Some(mesh) =
                    crate::uv_relax::uv_tool_geometry(live.tool, &scene, &p, weights.as_deref())?
                {
                    scene.active_mut().unwrap().mesh = mesh.into();
                }
            }
            // UVT-43: on the UV proxy (the pipe swapped it in); selected UV
            // positions move by the box's edit, weighted by the falloff.
            Tool::UvBoxTransform => {
                if p.uv.is_none() {
                    return Err("Box Transform works in a UV view".into());
                }
                p.uv_box.validate()?;
                let item = scene.active().ok_or("Select a mesh")?;
                let id = item.id;
                let weights = self.falloff_weights(&scene, id)?;
                let points = crate::transform_vertices(&scene)?;
                let (lo, hi) = crate::uv_box::bounds(&item.mesh.vertices, points.iter().copied())
                    .ok_or("Box Transform: no UVs selected")?;
                let mut mesh: crate::Mesh = (*item.mesh).clone();
                for &v in &points {
                    let q = mesh.vertices[v as usize];
                    let to = p.uv_box.apply(q.truncate(), lo, hi);
                    let w = weights
                        .as_ref()
                        .map_or(1., |w| w.get(v as usize).copied().unwrap_or(0.));
                    let moved = q.truncate().lerp(to, w);
                    mesh.vertices[v as usize] = DVec3::new(moved.x, moved.y, q.z);
                }
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::Jitter | Tool::Quantize | Tool::RadialAlign | Tool::LinearAlign => {
                let item = scene.active().ok_or("Select a mesh")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                let mesh =
                    crate::deform_finish::tool_geometry(live.tool, &scene, &p, weights.as_deref())?;
                // Symmetry: the primary side's result, mirrored (SYM3-12).
                let mut mesh = mesh;
                let symmetry = self.active_symmetry();
                let primary = self.symmetry_primary(&scene);
                let base = &scene.active().unwrap().mesh;
                symmetry.mirror_moved(base, &mut mesh, primary);
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::SymmetryTool => {
                let item = scene
                    .active()
                    .ok_or("Select a mesh for the Symmetry Tool")?;
                // Threshold is a world distance; the mesh is local (uniform
                // scale assumed, as for the other tools).
                let scale = scene
                    .world_matrix(item.id)?
                    .transform_vector3(DVec3::X)
                    .length()
                    .max(1e-12);
                let o = p.symmetry_fix;
                let fixed = crate::symmetry_fix::symmetry_fix(
                    &item.mesh,
                    self.active_symmetry(),
                    o.threshold / scale,
                    o.topology,
                    o.reverse,
                )?;
                self.symmetry_unpaired = fixed.unpaired;
                scene.active_mut().unwrap().mesh = fixed.mesh.into();
            }
            Tool::Bend => {
                let points = crate::transform_vertices(&scene)?;
                let item = scene.active().ok_or("Select a mesh")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                let world = scene.world_matrix(item.id)?;
                let axes = self.bend_axes(&scene);
                let direction = p.bend.direction(axes);
                // BN-45 (BH-Q12, provisional): an axis along the spine can't
                // turn it; the mesh stays as it was (see `bend_notice`).
                let bend = if p.bend.axis_along_spine(axes) {
                    crate::BendOptions {
                        angle: 0.,
                        ..p.bend
                    }
                } else {
                    p.bend
                };
                // SYM3-12: the side the user works on bends as entered, the
                // other side with the mirrored base, spine and lean; seam
                // points stay on the plane (SYM3-13).
                let symmetry = self.active_symmetry();
                let primary = self.symmetry_primary(&scene);
                let list: Vec<u32> = points.iter().copied().collect();
                let split =
                    crate::symmetry_sides::split_points(&item.mesh, symmetry, &list, primary);
                let mirror = crate::symmetry_sides::WorldMirror::new(symmetry, world);
                let mesh = match mirror.filter(|_| split.is_two_sided()) {
                    None => crate::bend::bend_geometry(
                        &item.mesh,
                        &points,
                        world,
                        &bend,
                        direction,
                        weights.as_deref(),
                    )?,
                    Some(m) => {
                        let first: std::collections::BTreeSet<u32> =
                            split.primary.iter().copied().collect();
                        let second: std::collections::BTreeSet<u32> =
                            split.other.iter().copied().collect();
                        let mut mesh = crate::bend::bend_geometry(
                            &item.mesh,
                            &first,
                            world,
                            &bend,
                            direction,
                            weights.as_deref(),
                        )?;
                        let mirrored = crate::BendOptions {
                            base: m.point(bend.base),
                            spine: m.vector(bend.spine),
                            ..bend
                        };
                        mesh = crate::bend::bend_geometry(
                            &mesh,
                            &second,
                            world,
                            &mirrored,
                            m.vector(direction),
                            weights.as_deref(),
                        )?;
                        let tolerance = crate::symmetry::match_tolerance(&item.mesh);
                        for &v in &first {
                            if symmetry.side(item.mesh.vertices[v as usize], tolerance)
                                == Some(crate::Side::Plane)
                            {
                                let q = &mut mesh.vertices[v as usize];
                                *q = symmetry.onto_plane(*q);
                            }
                        }
                        mesh
                    }
                };
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::EdgeRelax => {
                let edges = scene.selection.edges.clone();
                let item = scene.active().ok_or("Select a mesh for Edge Relax")?;
                let mesh = crate::edge_relax_geometry(&item.mesh, &edges, p.shift)?;
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::Chamfer => {
                let selected = scene.selection.faces.clone();
                let item = scene.active().ok_or("Select a mesh for Chamfer Edit")?;
                let output = crate::chamfer_geometry(&item.mesh, &selected, p.chamfer)?;
                // Rule 11: the selection stays when no polygons were added
                // or removed (CE02, CE10); a rebuilt strip clears it.
                let rebuilt = output.faces.len() != item.mesh.faces.len();
                scene.active_mut().unwrap().mesh = output.into();
                if rebuilt {
                    scene.selection.faces.clear();
                }
            }
            Tool::Inset | Tool::Shift => {
                let selected = scene.selection.faces.clone();
                let item = scene.active().ok_or("Select a mesh")?;
                // World distances on the item's own units (uniform scale
                // assumed, as for the vertex tools).
                let scale = crate::scene::uniform_scale(scene.world_matrix(item.id)?);
                let output = if live.tool == Tool::Inset {
                    crate::inset_geometry(&item.mesh, &selected, p.inset / scale)?
                } else {
                    crate::shift_geometry(&item.mesh, &selected, p.shift / scale)?
                };
                scene.active_mut().unwrap().mesh = output.mesh.into();
                scene.selection.faces = output.selected;
            }
            Tool::PolyHaul => {
                let selected = scene.selection.faces.clone();
                let item = scene.active().ok_or("Select a mesh for Poly Haul")?;
                let local = scene.world_matrix(item.id)?.inverse();
                let amount = crate::HaulAmount {
                    offset: p.poly_haul_offset,
                    vector: local.transform_vector3(p.poly_haul_amount.vector),
                    ..p.poly_haul_amount
                };
                let untouched = p.poly_haul_mode.uses_selection() && selected.is_empty()
                    || !p.poly_haul_mode.uses_selection()
                        && p.poly_haul_target == crate::HaulTarget::None;
                // Nothing to act on yet (no selection, or no drag started on
                // an element): the base scene stays.
                if !untouched {
                    let mut output = crate::poly_haul_mode_geometry(
                        &item.mesh,
                        &selected,
                        p.poly_haul_mode,
                        p.poly_haul_target,
                        amount,
                    )?;
                    // SYM3-10/12: a picked edge, point or polygon has its
                    // mirror partner hauled too, with the mirrored drag. The
                    // selection modes act on a symmetric selection already.
                    let symmetry = self.active_symmetry();
                    if !p.poly_haul_mode.uses_selection()
                        && let Some(target) = crate::symmetry_sides::mirrored_haul_target(
                            &item.mesh,
                            &output.mesh,
                            p.poly_haul_target,
                            symmetry,
                        )
                    {
                        let mirrored = crate::HaulAmount {
                            vector: symmetry.mirror_vector(amount.vector),
                            ..amount
                        };
                        let faces = output.faces.clone().unwrap_or_else(|| selected.clone());
                        output = crate::poly_haul_mode_geometry(
                            &output.mesh,
                            &faces,
                            p.poly_haul_mode,
                            target,
                            mirrored,
                        )?;
                    }
                    scene.active_mut().unwrap().mesh = output.mesh.into();
                    if let Some(faces) = output.faces {
                        scene.selection.faces = faces;
                    }
                }
            }
            Tool::SketchExtrude => {
                if !p.sketch_precision.is_finite() || !(1. ..=1000.).contains(&p.sketch_precision) {
                    return Err("Sketch Extrude Precision must be 1–1000 logical pixels".into());
                }
                if !p.sketch_path.is_empty() {
                    // SYM3-12: with Symmetry, the other side's polygons
                    // follow the mirrored path.
                    let symmetry = self.active_symmetry();
                    crate::symmetry_sides::two_pass(&mut scene, symmetry, |scene, mirrored| {
                        let selected = scene.selection.faces.clone();
                        let item = scene.active().ok_or("Select a mesh for Sketch Extrude")?;
                        // The drawn path is in world space; the mesh is local.
                        let local = scene.world_matrix(item.id)?.inverse();
                        let path: Vec<DVec3> = p
                            .sketch_path
                            .iter()
                            .map(|&q| local.transform_point3(q))
                            .map(|q| if mirrored { symmetry.mirror(q) } else { q })
                            .collect();
                        let (mesh, caps) =
                            crate::sketch_extrude_geometry(&item.mesh, &selected, &path)?;
                        scene.active_mut().unwrap().mesh = mesh.into();
                        scene.selection.faces = caps;
                        Ok(())
                    })?;
                }
            }
            Tool::SmoothShift | Tool::Thicken => {
                let selected = scene.selection.faces.clone();
                let item = scene.active().ok_or("Select an editable visible mesh")?;
                if !scene.shows(item) || scene.selection.mode != Mode::Polygon {
                    return Err("Select a visible mesh in Polygon mode".into());
                }
                // Q-SS-04 (Jason, 2026-11-18): Scale follows the selected
                // action centre; with none, the selection's centre. Origin
                // is the item's own origin (as the command oracles); the
                // world points of Element and Screen are taken into the item.
                let to_local = scene.world_matrix(item.id)?.inverse();
                let symmetry = self.active_symmetry();
                let point = |at: DVec3| crate::SmoothShiftPivot::Point {
                    at,
                    mirror: symmetry.axis.is_some().then_some(symmetry),
                };
                // ACS-20 (Jason, 2026-11-18): through the pivot service like
                // a transform tool: a placed centre (ACS-07) and, with
                // Symmetry, the service's primary pivot, the other side
                // scaling about its mirror image (ACS-04) and seam points
                // about its foot on the plane (ACS-05).
                use crate::CenterMode as C;
                let mirrored = symmetry.axis.is_some();
                let pivot = match (live.placed_centre, self.action_context.center) {
                    (Some(at), _) => point(to_local.transform_point3(at)),
                    (None, C::Local) => crate::SmoothShiftPivot::Local,
                    (None, C::None | C::Selection | C::SelectionAutoAxis | C::Automatic)
                        if mirrored =>
                    {
                        crate::SmoothShiftPivot::HalfBox(symmetry)
                    }
                    (None, C::SelectionBorder) if mirrored => {
                        crate::SmoothShiftPivot::HalfBorder(symmetry)
                    }
                    (None, C::None | C::Selection | C::SelectionAutoAxis | C::Automatic) => {
                        crate::SmoothShiftPivot::SelectionBox
                    }
                    (None, C::SelectionBorder) => crate::SmoothShiftPivot::SelectionBorder,
                    (None, C::Origin) => point(DVec3::ZERO),
                    (None, C::Element(at) | C::Screen(at)) => {
                        point(to_local.transform_point3(at.point()))
                    }
                    (None, C::Parent | C::Pivot) => point(
                        to_local
                            .transform_point3(live.action_frame.map_or(DVec3::ZERO, |f| f.center)),
                    ),
                };
                let result = crate::smooth_shift_geometry_about(
                    &item.mesh,
                    &selected,
                    p.smooth_shift,
                    None,
                    pivot,
                )?;
                scene.active_mut().unwrap().mesh = result.mesh.into();
                scene.selection.faces = result.selected;
                scene.selection.vertices.clear();
                scene.selection.edges.clear();
            }
            Tool::Extrude => {
                let item = scene.active().ok_or("Select a mesh")?;
                let source_ids = live
                    .identity_base
                    .as_ref()
                    .and_then(|ids| ids.meshes.get(&item.id))
                    .ok_or("Missing extrusion identity basis")?;
                source_ids.validate_shared(item.mesh.shared())?;
                // Stable ordered targets are resolved once into this private
                // kernel view; neither a coordinate nor a stale dense offset
                // supplies correspondence after the edit.
                // PE-O-03: with nothing selected every polygon is extruded.
                let faces: Vec<u32> = if scene.selection.faces.is_empty() {
                    (0..item.mesh.faces.len() as u32).collect()
                } else {
                    scene.selection.faces.clone()
                };
                let targets = faces
                    .iter()
                    .map(|&f| {
                        source_ids
                            .faces
                            .get(f as usize)
                            .copied()
                            .map(|face| crate::FaceHandle {
                                scope: source_ids.scope,
                                face,
                            })
                            .ok_or_else(|| String::from("Invalid source face target"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let selected = targets
                    .into_iter()
                    .map(|h| source_ids.face_index(h))
                    .collect::<Result<Vec<_>>>()?;
                let world = scene.world_matrix(item.id)?;
                if !item.mesh.attributes.polygon_sets.is_empty()
                    && (world != glam::DMat4::IDENTITY || self.action_context != Default::default())
                {
                    return Err(
                        "I1-S named sets require identity world transform and neutral context"
                            .into(),
                    );
                }
                if !world.is_finite() || world.determinant().abs() < 1e-15 {
                    return Err("Extrude requires an invertible world transform".into());
                }
                if !apply && p.extent == DVec3::ZERO {
                    self.extrude_moved_caps = scene.selection.faces.clone();
                    self.scene = scene;
                    if let Some(ids) = &live.identity_base {
                        self.identities.current = ids.clone();
                        self.identities.observe(&self.scene)?;
                    }
                    let l = self.session.live.as_mut().unwrap();
                    l.identity_preview = None;
                    l.params = p;
                    l.applied = true;
                    l.preview_error = None;
                    return Ok(());
                }
                let mut source = item.mesh.clone();
                // Facing is evaluated in world coordinates, including mirrored
                // transforms; stored attributes remain authored local payloads.
                for v in &mut source.vertices {
                    *v = world.transform_point3(*v);
                }
                let map = p
                    .extrude_uv_map
                    .as_deref()
                    .filter(|n| source.uv_maps.contains_key(*n));
                let mut result = crate::extrude_geometry(
                    &source,
                    &selected,
                    p.extent,
                    p.extrude_sides,
                    p.extrude_mode,
                    p.extrude_uvs,
                    map,
                )?;
                let inverse = world.inverse();
                for (index, v) in result.mesh.vertices.iter_mut().enumerate() {
                    // Preserve untouched local coordinates exactly; a world/local
                    // roundtrip must not introduce drift in unselected geometry.
                    if index < source.vertices.len() && *v == source.vertices[index] {
                        *v = item.mesh.vertices[index];
                    } else {
                        *v = inverse.transform_point3(*v);
                    }
                }
                // SYM3-12: the extrude offset is mirrored for the caps on the
                // partner side (a normal-only extrude is symmetric anyway).
                let symmetry = self.active_symmetry();
                if symmetry.axis.is_some() && p.extent != DVec3::ZERO {
                    let primary = self.symmetry_primary(&scene);
                    let v = inverse.transform_vector3(p.extent);
                    let moved: Vec<u32> = result
                        .moved_caps
                        .iter()
                        .filter_map(|&f| result.mesh.faces.get(f as usize))
                        .flat_map(|f| f.vertices.iter().copied())
                        .collect();
                    let tolerance = crate::symmetry::match_tolerance(&item.mesh);
                    symmetry.mirror_offset(
                        &mut result.mesh.vertices,
                        &moved,
                        v,
                        primary,
                        tolerance,
                    );
                }
                let topology_changed = result.mesh != *item.mesh;
                let source_ids = live
                    .identity_base
                    .as_ref()
                    .and_then(|ids| ids.meshes.get(&item.id))
                    .ok_or("Missing extrusion stage identity basis")?;
                let ids = if let Some((old_plan, ids)) = &live.identity_preview
                    && old_plan == &result.correspondence
                {
                    ids.validate(&result.mesh)?;
                    ids.clone()
                } else {
                    self.identities.with_ids(|next| {
                        source_ids.transported(
                            &item.mesh,
                            &result.mesh,
                            &result.correspondence,
                            next,
                        )
                    })?
                };
                self.identities
                    .current
                    .meshes
                    .insert(item.id, ids.clone().into());
                self.session.live.as_mut().unwrap().identity_preview =
                    Some((result.correspondence.clone(), ids));
                scene.active_mut().unwrap().mesh = result.mesh.into();
                // While the tool is live the moving caps stay selected (a
                // continuation extrudes them again); Apply, and Drop
                // (`select_extrude_caps`), select both caps (PE-M-03).
                scene.selection.faces = if apply {
                    result.caps
                } else {
                    result.moved_caps.clone()
                };
                if topology_changed {
                    scene.selection.vertices.clear();
                    scene.selection.edges.clear();
                }
                scene.validate()?;
                self.extrude_moved_caps = result.moved_caps;
                self.identities.observe(&scene)?;
            }
            Tool::TwistBevel => twist_note = self.preview_twist_bevel(&mut scene, live, &p)?,
            Tool::Bevel => {
                // PB-O-01: nothing selected bevels every polygon and leaves
                // nothing selected. bevel-activation-2026-11-05 BA11 says the
                // opposite; kept as measured in polygon-bevel-2026-11-02 until
                // the spec side settles it (SPECIFICATION_QUESTIONS BA-Q01).
                let everything = scene.selection.faces.is_empty();
                let selected = if everything {
                    (0..scene.active().ok_or("Select an item")?.mesh.faces.len() as u32).collect()
                } else {
                    scene.selection.faces.clone()
                };
                let item = scene.active().ok_or("Select an item")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                let bevel = if apply {
                    crate::bevel_geometry_applied
                } else {
                    crate::bevel_geometry_with_profile
                };
                let result = bevel(
                    &item.mesh,
                    &selected,
                    p.shift,
                    p.inset,
                    p.group_polygons,
                    &p.bevel,
                    weights.as_deref(),
                    self.bevel_profile,
                )
                .map_err(|e| format!("Bevel Shift/Inset: {e}"))?;
                let topology_changed = result.mesh != *item.mesh;
                scene.active_mut().unwrap().mesh = result.mesh.into();
                scene.selection.faces = if everything {
                    Vec::new()
                } else {
                    result.selected
                };
                if topology_changed {
                    scene.selection.vertices.clear();
                    scene.selection.edges.clear();
                }
            }
            Tool::Tweak => {
                // The grabbed element's points move by the world offset
                // `delta`, nearby points by their falloff weight;
                // selection, topology and IDs stay as they were (TWK-22).
                if !p.delta.is_finite() {
                    return Err("Tweak offset must be finite".into());
                }
                if let Some(grab) = p.tweak {
                    let item = scene.active().ok_or("Select a mesh")?;
                    if item.id != grab.item {
                        return Err("Tweak edits the active mesh only".into());
                    }
                    let world = live.base.world_matrix(grab.item)?;
                    let offset = world.inverse().transform_vector3(p.delta);
                    // TWK-12/15/16 and TWK-20 (provisional Symmetry): which
                    // points move and by how much comes from the press-time
                    // mesh, worked out once per drag.
                    let symmetry = self.active_symmetry();
                    let base_mesh = &live.base.active().ok_or("Select a mesh")?.mesh;
                    // In a UV view (UVT-131) Range may be in screen pixels
                    // (UVT-Q11) and connectivity read from the 3D mesh
                    // (UVT-Q12); see uv_pipe.rs.
                    let uv_space = p.uv.as_ref();
                    let mut options = p.tweak_options;
                    if let Some(space) = uv_space
                        && crate::uv_pipe::RANGE_UNITS == crate::uv_pipe::RangeUnits::Pixels
                        && space.pixels_per_uv > 0.
                    {
                        options.range /= space.pixels_per_uv;
                    }
                    let uv_proxy = uv_space.and_then(|s| s.proxy.clone()).filter(|_| {
                        crate::uv_pipe::CONNECTIVITY == crate::uv_pipe::UvConnectivity::Mesh
                    });
                    let pieces =
                        |seeds: &[u32]| uv_proxy.as_ref().expect("checked").connected_in_3d(seeds);
                    // The plan rides on the live parameters from frame to frame.
                    let plan = match p
                        .tweak_plan
                        .take()
                        .or_else(|| live.params.tweak_plan.clone())
                    {
                        Some(plan)
                            if plan.grab == grab
                                && plan.options == options
                                && plan.symmetry == symmetry =>
                        {
                            plan
                        }
                        _ => crate::tweak::tweak_plan_with(
                            base_mesh,
                            world,
                            grab,
                            options,
                            symmetry,
                            uv_proxy
                                .is_some()
                                .then_some(&pieces as crate::tweak::PieceFinder<'_>),
                        )?,
                    };
                    let item = scene.active_mut().ok_or("Select a mesh")?;
                    crate::tweak_apply(&mut item.mesh, base_mesh, &plan, offset)?;
                    p.tweak_plan = Some(plan);
                } else if p.delta != DVec3::ZERO {
                    return Err("Tweak: press on a vertex, edge or polygon and drag".into());
                }
            }
            Tool::Transform | Tool::Move | Tool::Rotate | Tool::Scale => {
                if !p.delta.is_finite()
                    || !p.rotation_delta.is_finite()
                    || !p.scale_delta.is_finite()
                {
                    return Err("Transform must be finite".into());
                }
                let factors = if live.tool == Tool::Transform {
                    Some(p.scale_delta)
                } else {
                    (live.tool == Tool::Scale).then_some(p.delta)
                };
                // IT-04: with Negative Scale on, an item scale may flip sign
                // (never zero: a zero item scale has no inverse). Components
                // may reach 0 %, collapsing onto the centre (GitHub #24),
                // but never go negative.
                let item_mode = scene.selection.mode == Mode::Item;
                let negative_ok = p.negative_scale && item_mode;
                if let Some(f) = factors {
                    if !item_mode && f.min_element() < 0. {
                        return Err(
                            "Scale can't be below 0 %; turn on Negative Scale for items".into()
                        );
                    }
                    if item_mode
                        && (f.abs().min_element() <= 1e-8
                            || (!negative_ok && f.min_element() <= 1e-8))
                    {
                        return Err(if negative_ok {
                            "Scale can't be zero".into()
                        } else {
                            "Scale must be positive; turn on Negative Scale for items".into()
                        });
                    }
                }
                if scene.selection.mode == Mode::Item
                    && let Some(targets) = live.base.item_transform_targets()
                {
                    // Several selected items move together (multi_transform.rs).
                    crate::multi_transform::transform_items(
                        &mut scene, &live.base, &targets, live.tool, &p,
                    )?;
                    // Planar spec: a Texture Locator's Size stops at 1 mm.
                    scene.clamp_locator_sizes();
                } else if scene.selection.mode == Mode::Item {
                    let item = scene
                        .active_mut()
                        .ok_or("Select an item; empty selection does not target all items")?;
                    // CP-A02: a locked item refuses item transforms.
                    if item.is_locked() {
                        return Err(format!("{} is locked (Assembly > Lock)", item.name));
                    }
                    // ENVIRONMENT-PLACEMENT §2: an environment has no
                    // transform; its sky turns on its layers' locators.
                    if item.environment.is_some() {
                        return Err(crate::environment::NO_TRANSFORM.into());
                    }
                    match live.tool {
                        Tool::Transform => {
                            item.transform.translation += p.delta;
                            item.transform.rotation += p.rotation_delta.map(f64::to_radians);
                            item.transform.scale *= p.scale_delta;
                        }
                        Tool::Move => item.transform.translation += p.delta,
                        Tool::Rotate => item.transform.rotation += p.delta.map(f64::to_radians),
                        Tool::Scale => item.transform.scale *= p.delta,
                        _ => {}
                    }
                    // Planar spec: a Texture Locator's Size stops at 1 mm.
                    if item.texture_locator().is_some() {
                        item.transform.scale =
                            crate::texture_locator::clamp_size(item.transform.scale);
                    }
                } else {
                    // The pivot service resolved the drag's base once into
                    // groups (pivot_service.rs): every mesh the tool moves,
                    // its pivots, the points each drives and each point's
                    // Symmetry role. The values are applied once per group;
                    // nothing here works out targets, clusters, mirror
                    // roles or frames.
                    let pivots = self.live_pivots().ok_or("Missing action frame")??;
                    let angles = if live.tool == Tool::Transform {
                        p.rotation_delta
                    } else {
                        p.delta
                    };
                    let rotation = glam::DQuat::from_euler(
                        glam::EulerRot::XYZ,
                        angles.x.to_radians(),
                        angles.y.to_radians(),
                        angles.z.to_radians(),
                    );
                    // FAL-M-02 / FAL-B2-02/03: with a falloff each point moves to
                    // p + w·(T(p) − p), its weight's share of the full
                    // transform. With Symmetry the weight is read at the
                    // point's positive-side position (FAL-B2-04).
                    let falloff = self.active_falloff();
                    // TW-Q1: Twist and Swirl turn each point by weight ×
                    // Angle instead of blending positions (deform.rs).
                    let angle_weighted = live.tool == Tool::Rotate
                        && self
                            .session
                            .preset
                            .as_ref()
                            .is_some_and(|p| p.angle_by_weight);
                    // #24: a scale reaching 0 flattens polygons on
                    // purpose; they are marked as deliberate degenerate
                    // polygons, as Inset 0 and Poly Haul do.
                    let factors = match live.tool {
                        Tool::Scale => Some(p.delta),
                        Tool::Transform => Some(p.scale_delta),
                        _ => None,
                    };
                    let collapses = factors.is_some_and(|f| f.min_element() <= 1e-12);
                    for target_item in &pivots.items {
                        let item_id = target_item.item;
                        // The mirror rules (S-03..S-06): selected points on
                        // the primary side take the values as entered (typed
                        // or dragged alike), the other side's are mirrored,
                        // edited and mirrored back, and seam points go back
                        // onto the plane.
                        let map = &target_item.map;
                        let world = map.world;
                        // VMF-03: a Vertex Map falloff reads the point's own
                        // value (copied out, the mesh is edited below).
                        // The selection-driven falloffs read the drag's
                        // starting mesh and selection (once per drag).
                        let prepared = match falloff.as_ref() {
                            Some(f) => f.prepare(&live.base, item_id)?,
                            None => Default::default(),
                        };
                        let item = scene.item_mut(item_id).ok_or("Select a mesh")?;
                        let mut collapsed: Vec<u32> = Vec::new();
                        for group in &target_item.groups {
                            for target in &group.targets {
                                let i = target.point;
                                let stored = &mut item.mesh.vertices[i as usize];
                                // A point from the other side is edited in
                                // its group's twin frame once it has been
                                // mirrored over (S-04, S-09).
                                let frame = match target.role {
                                    crate::pivot_service::Role::Other => group.twin,
                                    _ => group.frame,
                                };
                                // ACS-13: the weight is read where the point's
                                // primary-side image is.
                                let sample = map.falloff_sample_on(target.side, *stored);
                                let w = falloff.as_ref().map_or(1., |f| {
                                    f.weight_at_local(
                                        world.transform_point3(sample),
                                        sample,
                                        i,
                                        prepared.inputs(),
                                    )
                                });
                                let edit = |start: DVec3| -> DVec3 {
                                    let v = start;
                                    let moved = match live.tool {
                                        Tool::Transform => {
                                            frame.center
                                                + frame.basis
                                                    * (rotation
                                                        * ((frame.basis.transpose()
                                                            * (v - frame.center))
                                                            * p.scale_delta)
                                                        + p.delta)
                                        }
                                        Tool::Move => v + frame.basis * p.delta,
                                        Tool::Rotate if angle_weighted => {
                                            let turned = angles * w;
                                            let rotation = glam::DQuat::from_euler(
                                                glam::EulerRot::XYZ,
                                                turned.x.to_radians(),
                                                turned.y.to_radians(),
                                                turned.z.to_radians(),
                                            );
                                            frame.center
                                                + frame.basis
                                                    * (rotation
                                                        * (frame.basis.transpose()
                                                            * (v - frame.center)))
                                        }
                                        Tool::Rotate => {
                                            frame.center
                                                + frame.basis
                                                    * (rotation
                                                        * (frame.basis.transpose()
                                                            * (v - frame.center)))
                                        }
                                        Tool::Scale => {
                                            frame.center
                                                + frame.basis
                                                    * ((frame.basis.transpose()
                                                        * (v - frame.center))
                                                        * p.delta)
                                        }
                                        _ => v,
                                    };
                                    if angle_weighted {
                                        moved
                                    } else {
                                        start + (moved - start) * w
                                    }
                                };
                                *stored = map.apply_as(target.role, *stored, edit);
                                if collapses {
                                    collapsed.push(i);
                                }
                            }
                        }
                        if collapses {
                            mark_collapsed(&mut item.mesh, &collapsed);
                        }
                    }
                }
            }
        }
        // A drag that only moves points skips re-checking every polygon.
        if let Some(live) = &self.session.live {
            for item in &scene.items {
                if let Some(before) = live.base.item(item.id) {
                    item.mesh.shared().prove_points_moved(before.mesh.shared());
                }
            }
        }
        // ACS-31: a dragged guide's twin follows in the preview too.
        if let Some(live) = &self.session.live {
            crate::character_rig::sync(&mut scene, &live.base);
        }
        scene.validate()?;
        // Item-mode copies take new ids (DS-12).
        self.next_item_id = self.next_item_id.max(scene.next_id);
        self.scene = scene;
        if twist_note.is_some() {
            self.symmetry_note = twist_note;
        }
        let l = self.session.live.as_mut().ok_or("No tool")?;
        l.params = p;
        l.preview_error = None;
        l.applied = true;
        Ok(())
    }

    /// Twist Bevel on the active mesh (twist-bevel-2026-11-15): the shared
    /// pipe's falloff (one weight per tube, read at its base centre),
    /// symmetry (mirror-image tubes) and the action axis (Rotation Axis =
    /// Action Axis) go to `twist_bevel_geometry`. Returns the symmetry
    /// note, if any.
    pub(super) fn preview_twist_bevel(
        &self,
        scene: &mut Scene,
        live: &Live,
        p: &Parameters,
    ) -> Result<Option<String>> {
        // PB-O-01 as Bevel: nothing selected works on every polygon and
        // leaves nothing selected.
        let everything = scene.selection.faces.is_empty();
        let item = scene.active().ok_or("Select an item")?;
        let selected: Vec<u32> = if everything {
            (0..item.mesh.faces.len() as u32).collect()
        } else {
            scene.selection.faces.clone()
        };
        let world = scene.world_matrix(item.id)?;
        // Distances are entered in world units on the item's own scale
        // (uniform scale assumed, as for Inset and Shift).
        let scale = crate::scene::uniform_scale(world);
        let options = crate::TwistBevelOptions {
            shift: p.twist_bevel.shift / scale,
            inset: p.twist_bevel.inset / scale,
            ..p.twist_bevel
        };
        // FAL (spec section 5, TB-Q05): a position falloff is read at each
        // tube's base centre; one that reads points (a weight map, the
        // selection) gives the mean of the tube's point weights.
        let field = self.active_falloff();
        let point_weights = match &field {
            Some(f) if f.reads_map() || f.reads_selection() => Some(f.weights(scene, item.id)?),
            _ => None,
        };
        let weight = |centre: DVec3, points: &[u32]| -> f64 {
            let Some(field) = &field else { return 1. };
            match &point_weights {
                Some(w) => {
                    points.iter().map(|&v| w[v as usize]).sum::<f64>() / points.len().max(1) as f64
                }
                None => field.weight_at_local(
                    world.transform_point3(centre),
                    centre,
                    0,
                    Default::default(),
                ),
            }
        };
        let symmetry = self.active_symmetry();
        let primary = self.symmetry_primary(scene);
        // World axes in the mesh's coordinates: the action axis falls back
        // to world Y and the side axis is world X on a rotated item too.
        let to_local = world.inverse();
        let world_axes = glam::DMat3::from_cols(
            to_local.transform_vector3(DVec3::X),
            to_local.transform_vector3(DVec3::Y),
            to_local.transform_vector3(DVec3::Z),
        );
        let action_axis = live
            .action_frame
            .map(|f| to_local.transform_vector3(self.action_axis(f)))
            .unwrap_or(world_axes.y_axis);
        let pipe = crate::TwistBevelPipe {
            weight: field
                .is_some()
                .then_some(&weight as &dyn Fn(DVec3, &[u32]) -> f64),
            symmetry: symmetry.axis.map(|_| (symmetry, primary)),
            action_axis,
            world_axes,
        };
        let result = crate::twist_bevel_geometry(&item.mesh, &selected, &options, &pipe)?;
        let topology_changed = result.mesh != *item.mesh;
        scene.active_mut().unwrap().mesh = result.mesh.into();
        scene.selection.faces = if everything {
            Vec::new()
        } else {
            result.selected
        };
        if topology_changed {
            scene.selection.vertices.clear();
            scene.selection.edges.clear();
        }
        Ok(result.note)
    }
}

/// Mark the polygons using `moved` points that a scale to 0 left with no
/// area as deliberate degenerate polygons (GitHub #24).
fn mark_collapsed(mesh: &mut crate::Mesh, moved: &[u32]) {
    let mut touched = vec![false; mesh.vertices.len()];
    for &v in moved {
        if let Some(t) = touched.get_mut(v as usize) {
            *t = true;
        }
    }
    for f in 0..mesh.faces.len() {
        if !mesh.is_polyline(f)
            && mesh.faces[f]
                .vertices
                .iter()
                .any(|&v| touched.get(v as usize) == Some(&true))
            && mesh.normal(f) == glam::DVec3::ZERO
        {
            mesh.attributes.degenerate_faces.insert(f as u32);
        }
    }
}
