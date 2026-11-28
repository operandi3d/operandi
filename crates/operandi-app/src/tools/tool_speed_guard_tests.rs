//! Speed guard for interactive tools (see "Fast drags" in AGENTS.md).
//!
//! A drag changes a tool's value every frame. After the first frames of a
//! drag the topology stays the same, so a tool on the fast path must not
//! re-evaluate the whole subdivision surface, rebuild the item's GPU
//! geometry, rebuild the picking surface or recompute topology results
//! (edge lists and counts) again. This test drags every tool on a
//! subdivided grid and fails when any of those happen, which is how a tool
//! falling back to a slow path shows up long before anyone feels it.
//!
//! `drag_step` matches every tool with no catch-all, so a new tool does not
//! compile until it is given a drag here (or a reason it has none).
use super::*;

type Step = fn(&mut Parameters, f64);

/// The mode a tool is dragged in and how one drag frame changes its value;
/// `None` for tools without a value drag.
fn drag_step(tool: Tool) -> Option<(Mode, Step)> {
    Some(match tool {
        // Primitives and sketches create from clicks; nothing to drag on a
        // large mesh.
        Tool::Box | Tool::Sphere | Tool::Cylinder | Tool::SketchExtrude | Tool::VertexCreate => {
            return None;
        }
        // Needs a bevel whose rows end on side edges; covered by its own
        // tests, not set up on this grid.
        Tool::EdgeRelax => return None,
        // Chamfer Edit works on plain (Face) polygons only, never on a
        // subdivided cage.
        Tool::Chamfer => return None,
        Tool::Move | Tool::Transform => (Mode::Polygon, |p, k| p.delta.z = 0.001 * k),
        Tool::Rotate => (Mode::Polygon, |p, k| p.rotation_delta.z = 0.01 * k),
        Tool::Scale => (Mode::Polygon, |p, k| {
            p.scale_delta = DVec3::splat(1. + 0.01 * k)
        }),
        Tool::Extrude => (Mode::Polygon, |p, k| p.shift = 0.001 * k),
        Tool::SmoothShift | Tool::Thicken => {
            (Mode::Polygon, |p, k| p.smooth_shift.offset = 0.001 * k)
        }
        Tool::Bevel => (Mode::Polygon, |p, k| {
            p.inset = 0.0002 * k;
            p.shift = 0.001 * k
        }),
        // Twist Bevel: a bent, twisted tube growing longer, so every ring
        // moves (a twist too sharp for the length is refused, as it folds
        // the walls).
        Tool::TwistBevel => (Mode::Polygon, |p, k| {
            p.twist_bevel.shift = 0.05 + 0.001 * k;
            p.twist_bevel.inset = 0.0001 * k;
            p.twist_bevel.bend = 30.;
            p.twist_bevel.rotation = 10.;
        }),
        Tool::PolyHaul => (Mode::Polygon, |p, k| p.poly_haul_offset = 0.001 * k),
        Tool::Inset => (Mode::Polygon, |p, k| p.inset = 0.0005 * k),
        Tool::Shift => (Mode::Polygon, |p, k| p.shift = 0.001 * k),
        Tool::EdgeExtend => (Mode::Edge, |p, k| p.delta.z = 0.001 * k),
        Tool::EdgeExtrude => (Mode::Edge, |p, k| {
            p.shift = 0.001 * k;
            p.inset = 0.001
        }),
        Tool::EdgeBevel | Tool::EdgeChamfer => (Mode::Edge, |p, k| p.chamfer.offset = 0.0005 * k),
        Tool::EdgeSlide => (Mode::Edge, |p, k| p.shift = 0.02 * k),
        Tool::EdgeCrease => (Mode::Edge, |p, k| p.shift = 0.05 * k),
        Tool::VertexBevel => (Mode::Vertex, |p, k| p.inset = 0.0002 * k),
        Tool::VertexExtrude => (Mode::Vertex, |p, k| {
            p.shift = 0.001 * k;
            p.inset = 0.001
        }),
        Tool::VertexMerge => (Mode::Vertex, |p, k| p.shift = 0.0001 * k),
        Tool::VertexTear => (Mode::Vertex, |p, k| p.delta.z = 0.001 * k),
        // The grab (the middle point) is set after activation, below.
        Tool::Tweak => (Mode::Polygon, |p, k| p.delta.z = 0.001 * k),
        Tool::VertexSlide => (Mode::Vertex, |p, k| {
            p.shift = 0.05 * k;
            p.delta = DVec3::X
        }),
        Tool::Push => (Mode::Polygon, |p, k| p.shift = 0.001 * k),
        Tool::Smooth => (Mode::Vertex, |p, k| p.smooth.strength = 0.01 * k),
        Tool::Jitter => (Mode::Vertex, |p, k| {
            p.jitter.range = DVec3::splat(0.001 * k)
        }),
        Tool::Quantize => (Mode::Vertex, |p, k| {
            p.quantize.step = DVec3::splat(0.01 + 0.001 * k)
        }),
        Tool::RadialAlign => (Mode::Polygon, |p, k| {
            p.radial_align.weight = (0.05 * k).min(1.)
        }),
        Tool::LinearAlign => (Mode::Edge, |p, k| {
            p.linear_align.weight = (0.05 * k).min(1.)
        }),
        Tool::Bend => (Mode::Polygon, |p, k| p.bend.angle = 5. * k),
        // The Symmetry Tool rebuilds partners over the whole mesh: it is a
        // one-off repair, not a per-element drag (provisional).
        Tool::SymmetryTool => return None,
        // The UV tab's tools solve or project whole UV maps by design
        // (Unwrap and Relax re-solve every island; Project rewrites the
        // selection); they never move points, so the surface isn't
        // evaluated again (uv_editing_tests covers that).
        Tool::UvProject
        | Tool::UvViewProject
        | Tool::UvUnwrap
        | Tool::UvRelax
        | Tool::UvPelt
        | Tool::UvPeeler
        | Tool::UvBoxTransform => {
            return None;
        }
        // The Slice tools re-cut the base mesh each frame; once the cut's
        // topology is set, a drag only moves its new points. Loop Slice's
        // ring is the selected middle edge (set at activation).
        Tool::LoopSlice => (Mode::Edge, |p, k| {
            p.slice.loop_slice.positions = vec![0.3 + 0.02 * k]
        }),
        // Points on two opposite edges of the middle polygon (24 x 24 grid:
        // point (12, 12) is 312), dragged along them.
        Tool::EdgeSlice => (Mode::Polygon, |p, k| {
            let t = 0.3 + 0.02 * k;
            p.slice.edge_slice.points = vec![
                operandi_core::SlicePoint::Edge { a: 312, b: 313, t },
                operandi_core::SlicePoint::Edge { a: 337, b: 338, t },
            ]
        }),
        // A line across the selected middle row (the grid is in XY), moved
        // a little each frame.
        Tool::PolygonSlice => (Mode::Polygon, |p, k| {
            let y = 0.01 + 0.0002 * k;
            p.slice.polygon_slice.cuts = vec![operandi_core::PlaneCut {
                start: DVec3::new(-0.6, y, 0.),
                end: DVec3::new(0.6, y, 0.),
                up: DVec3::Z,
            }]
        }),
        // Waves 2 and 3 (the grid is in XY, so the cutters look along Z): a
        // Pen line and a Curve across the selected polygons, a rectangle
        // over them, and one X cut moved a little each frame.
        Tool::PenSlice => (Mode::Polygon, |p, k| {
            let o = &mut p.slice.pen_slice;
            o.cutter.axis = operandi_core::SliceAxis::Z;
            o.closed = false;
            let y = 0.020833333333333332 + 0.00041666666666666664 * k;
            o.points = vec![
                DVec3::new(-0.020833333333333332, y, 1.),
                DVec3::new(0.10416666666666666, y, 1.),
            ];
        }),
        Tool::CurveSlice => (Mode::Polygon, |p, k| {
            let o = &mut p.slice.curve_slice;
            o.cutter.axis = operandi_core::SliceAxis::Z;
            let y = 0.020833333333333332 + 0.00041666666666666664 * k;
            o.points = vec![
                DVec3::new(-0.020833333333333332, y, 1.),
                DVec3::new(0.041666666666666664, y + 0.010416666666666666, 1.),
                DVec3::new(0.10416666666666666, y, 1.),
            ];
        }),
        Tool::PrimitiveSlice => (Mode::Polygon, |p, k| {
            let o = &mut p.slice.primitive_slice;
            o.place = Some(operandi_core::ShapePlace::on(
                DVec3::new(
                    0.041666666666666664 + 0.00041666666666666664 * k,
                    0.020833333333333332,
                    0.,
                ),
                DVec3::Z,
            ));
            o.half_x = 0.041666666666666664;
            o.half_y = 0.010416666666666666;
        }),
        // Contour cuts on release (Fast Slice on): a drag frame previews
        // nothing, and its cut reads background meshes, not the grid.
        Tool::Contour => return None,
        Tool::AxisSlice => (Mode::Polygon, |p, k| {
            p.slice.axis_slice.offset.x = 0.00041666666666666664 * k
        }),
        // The Duplicate tools copy the selected polygons; a drag moves the
        // placer, so only the copies' points change (no welds: the copies
        // stay clear of the grid).
        Tool::Mirror => (Mode::Polygon, |p, k| {
            p.duplicate.placed = true;
            p.duplicate.mirror.center.x = 0.6 + 0.001 * k
        }),
        Tool::Lathe => (Mode::Polygon, |p, k| {
            p.duplicate.placed = true;
            p.duplicate.lathe.circle.center.x = 2. + 0.001 * k;
            p.duplicate.lathe.circle.end = 180.
        }),
        Tool::Clone => (Mode::Polygon, |p, k| {
            p.duplicate.placed = true;
            p.duplicate.clone.offset.x = 1. + 0.001 * k
        }),
        Tool::Array => (Mode::Polygon, |p, k| {
            p.duplicate.placed = true;
            p.duplicate.array.offset.x = 1. + 0.001 * k
        }),
        Tool::RadialArray => (Mode::Polygon, |p, k| {
            p.duplicate.placed = true;
            p.duplicate.radial.circle.center.x = 2. + 0.001 * k
        }),
        // The path tools: dragging the path's last point moves only the
        // swept or cloned points.
        Tool::CurveExtrude => (Mode::Polygon, |p, k| {
            let t = &mut p.duplicate.curve_extrude;
            t.steps.number = 4;
            t.curve.points = vec![
                DVec3::ZERO,
                DVec3::new(0., 0., 0.2),
                DVec3::new(0.1 + 0.001 * k, 0., 0.4),
            ];
        }),
        Tool::CurveClone => (Mode::Polygon, |p, k| {
            let t = &mut p.duplicate.curve_clone;
            t.steps.number = 3;
            t.curve.points = vec![DVec3::ZERO, DVec3::new(1. + 0.001 * k, 0., 0.)];
        }),
        Tool::ScatterClone => (Mode::Polygon, |p, k| {
            p.duplicate.placed = true;
            p.duplicate.scatter.generator.count = 3;
            p.duplicate.scatter.generator.centre.x = 2. + 0.001 * k
        }),
        // Symmetrize has no drag (DX-41: fields only), and any change of
        // Offset moves the whole mirrored half, so a rebuild is expected.
        Tool::Symmetrize => return None,
    })
}

/// `n` x `n` quads on a gentle wave, all Catmull-Clark.
fn grid(n: u32) -> operandi_core::Mesh {
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (x, y) = (i as f64 / n as f64 - 0.5, j as f64 / n as f64 - 0.5);
            mesh.vertices
                .push(DVec3::new(x, y, 0.1 * (x * 9.).sin() * (y * 7.).cos()));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    mesh
}

#[test]
fn every_tool_drag_stays_on_the_fast_path() {
    let n = 24u32;
    let mid_face = (n / 2) * n + n / 2;
    let mid_point = (n / 2) * (n + 1) + n / 2;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut failures = Vec::new();
    for tool in Tool::ALL {
        let Some((mode, step)) = drag_step(tool) else {
            continue;
        };
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid(n).into();
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::CatmullClark,
        ));
        a.force_resident = true;
        a.camera.view = crate::camera::View::Perspective;
        a.frame(false);
        a.run(Command::Mode(mode));
        let selection = &mut a.editor.scene.selection;
        match (tool, mode) {
            // An edge on the grid's open border.
            (Tool::EdgeExtend, _) => selection.edges = vec![(n / 2, n / 2 + 1)],
            // A polygon and one of its points shared with other polygons.
            (Tool::VertexTear, _) => {
                selection.faces = vec![mid_face];
                selection.vertices = vec![mid_point];
            }
            (_, Mode::Polygon) => selection.faces = vec![mid_face, mid_face + 1, mid_face + n],
            (_, Mode::Edge) => selection.edges = vec![(mid_point, mid_point + 1)],
            _ => selection.vertices = vec![mid_point],
        }
        super::tests::draw(&mut a, &ctx, size, vec![]);
        a.activate(tool);
        let Some(mut base) = a.editor.session.live.as_ref().map(|l| l.params.clone()) else {
            failures.push(format!("{tool:?}: didn't activate ({})", a.status));
            continue;
        };
        if tool == Tool::Tweak {
            base.tweak = Some(operandi_core::TweakGrab {
                item: a.editor.scene.active().unwrap().id,
                element: operandi_core::TweakElement::Polygon(mid_face),
            });
            // The falloff path: weights over a few cells (the grid is 1 m
            // wide), limited to the connected piece.
            base.tweak_options.range = 0.1;
            base.tweak_options.connect = operandi_core::TweakConnect::Connectivity;
        }
        let frame = |a: &mut Operandi, k: f64| {
            let mut p = base.clone();
            step(&mut p, k);
            let ok = a.run_ok(Command::Preview(p));
            super::tests::draw(a, &ctx, size, vec![]);
            ok
        };
        // The first frames may change the topology (a bevel adds polygons).
        if !(frame(&mut a, 1.) && frame(&mut a, 2.)) {
            failures.push(format!("{tool:?}: preview refused ({})", a.status));
            continue;
        }
        let counts = |a: &Operandi| {
            [
                a.subdivision_cache.full_builds(),
                a.resident_cache.builds(),
                a.surface_cache.builds(),
                operandi_core::topology_cache::computed(),
            ]
        };
        let before = counts(&a);
        for k in 3..7 {
            if !frame(&mut a, k as f64) {
                failures.push(format!("{tool:?}: preview refused ({})", a.status));
                break;
            }
        }
        let after = counts(&a);
        for (i, what) in [
            "subdivision surface evaluated in full",
            "GPU geometry rebuilt in full",
            "picking surface rebuilt",
            "topology recomputed",
        ]
        .iter()
        .enumerate()
        {
            if after[i] != before[i] {
                failures.push(format!(
                    "{tool:?}: {what} {} times in 4 drag frames",
                    after[i] - before[i]
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "slow drag paths:\n{}",
        failures.join("\n")
    );
}

/// Flex (a Transform with a Selection falloff): the selection rings are
/// worked out once per drag, so its drag frames stay on the fast path too.
#[test]
fn flex_drag_stays_on_the_fast_path() {
    let n = 24u32;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = grid(n).into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    a.force_resident = true;
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    a.run(Command::Mode(Mode::Polygon));
    // A 6 × 6 patch in the middle of the grid.
    a.editor.scene.selection.faces = (9..15)
        .flat_map(|z| (9..15).map(move |x| z * n + x))
        .collect();
    super::tests::draw(&mut a, &ctx, size, vec![]);
    a.activate_deform(operandi_core::DeformPreset::Flex);
    let base = a
        .editor
        .session
        .live
        .as_ref()
        .expect("Flex is live")
        .params
        .clone();
    let frame = |a: &mut Operandi, k: f64| {
        let mut p = base.clone();
        p.rotation_delta.x = 2. * k;
        assert!(a.run_ok(Command::Preview(p)), "{}", a.status);
        super::tests::draw(a, &ctx, size, vec![]);
    };
    frame(&mut a, 1.);
    frame(&mut a, 2.);
    let counts = |a: &Operandi| {
        [
            a.subdivision_cache.full_builds(),
            a.resident_cache.builds(),
            a.surface_cache.builds(),
            operandi_core::topology_cache::computed(),
        ]
    };
    let before = counts(&a);
    for k in 3..7 {
        frame(&mut a, k as f64);
    }
    assert_eq!(counts(&a), before, "Flex drag left the fast path");
}

/// Flare (a Push with a fitted Linear falloff, tool presets 6b): its drag
/// frames only move points, like Push's.
#[test]
fn flare_drag_stays_on_the_fast_path() {
    let n = 24u32;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = grid(n).into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    a.force_resident = true;
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = (9..15)
        .flat_map(|z| (9..15).map(move |x| z * n + x))
        .collect();
    super::tests::draw(&mut a, &ctx, size, vec![]);
    a.activate_preset("Flare");
    let base = a
        .editor
        .session
        .live
        .as_ref()
        .expect("Flare is live")
        .params
        .clone();
    let frame = |a: &mut Operandi, k: f64| {
        let mut p = base.clone();
        p.shift = 0.01 * k;
        assert!(a.run_ok(Command::Preview(p)), "{}", a.status);
        super::tests::draw(a, &ctx, size, vec![]);
    };
    frame(&mut a, 1.);
    frame(&mut a, 2.);
    let counts = |a: &Operandi| {
        [
            a.subdivision_cache.full_builds(),
            a.resident_cache.builds(),
            a.surface_cache.builds(),
            operandi_core::topology_cache::computed(),
        ]
    };
    let before = counts(&a);
    for k in 3..7 {
        frame(&mut a, k as f64);
    }
    assert_eq!(counts(&a), before, "Flare drag left the fast path");
}

#[test]
fn sculpt_strokes_stay_on_the_fast_path() {
    // Every sculpt brush (sculpt_ui.rs) stroked across the subdivided grid
    // with real pointer events: after the first frames a stroke only moves
    // points, so the surface, GPU geometry and picking are patched, never
    // rebuilt (SC-110/111). Strokes pick with their own lookup.
    let n = 24u32;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut failures = Vec::new();
    for brush in operandi_core::SculptBrush::every() {
        if brush.unavailable().is_some() {
            // Attenuate waits for sculpt layers (SCT-Q12).
            continue;
        }
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid(n).into();
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::CatmullClark,
        ));
        a.force_resident = true;
        a.camera.view = crate::camera::View::Front;
        a.frame(false);
        for _ in 0..2 {
            super::tests::draw(&mut a, &ctx, size, vec![]);
        }
        if brush.is_vertex_map() {
            // A half-painted map, so every tool (Erase too) changes it.
            a.run(Command::NewWeightMap {
                name: "W".into(),
                value: Some(0.5),
            });
            // Uneven, so Smooth has something to blur.
            let mesh = &mut a.editor.scene.active_mut().unwrap().mesh;
            let left: Vec<u32> = (0..mesh.vertices.len() as u32)
                .filter(|&v| mesh.vertices[v as usize].x < 0.)
                .collect();
            let map = mesh.attributes.point_weights.get_mut("W").unwrap();
            for v in left {
                map.insert(v, 1.);
            }
        }
        a.activate_sculpt(brush);
        // Small against the grid: the surface patch falls back to a full
        // evaluation once the moved region is over half the mesh.
        a.sculpt.settings.brush_mut(brush).tip.size = 30.;
        let start = a
            .camera
            .project(DVec3::ZERO, a.viewport_rect)
            .map(|(p, _)| p)
            .unwrap();
        let at = |k: f32| start + vec2(k * 6., -k * 3.);
        super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![
                Event::PointerMoved(at(0.)),
                super::tests::pointer(at(0.), true, Modifiers::NONE),
            ],
        );
        for k in 1..3 {
            super::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(at(k as f32))]);
        }
        let counts = |a: &Operandi| {
            [
                a.subdivision_cache.full_builds(),
                a.resident_cache.builds(),
                a.surface_cache.builds(),
                operandi_core::topology_cache::computed(),
            ]
        };
        let mesh_before = a.editor.scene.active().unwrap().mesh.vertices.clone();
        let weights_before = a
            .editor
            .scene
            .active()
            .unwrap()
            .mesh
            .attributes
            .point_weights
            .clone();
        let before = counts(&a);
        for k in 3..10 {
            super::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(at(k as f32))]);
        }
        let after = counts(&a);
        let mesh_now = &a.editor.scene.active().unwrap().mesh;
        let unchanged = if brush.is_vertex_map() {
            mesh_now.attributes.point_weights == weights_before
        } else {
            mesh_now.vertices == mesh_before
        };
        if unchanged {
            failures.push(format!(
                "{brush:?}: the stroke moved nothing ({})",
                a.status
            ));
        }
        for (i, what) in [
            "subdivision surface evaluated in full",
            "GPU geometry rebuilt in full",
            "picking surface rebuilt",
            "topology recomputed",
        ]
        .iter()
        .enumerate()
        {
            if after[i] != before[i] {
                failures.push(format!(
                    "{brush:?} stroke: {what} {} times in 7 frames",
                    after[i] - before[i]
                ));
            }
        }
        super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![super::tests::pointer(at(10.), false, Modifiers::NONE)],
        );
        if a.editor.history.undo.last().map(|c| c.label.as_str())
            != Some(&if brush.is_vertex_map() {
                format!("Vertex Map {} Stroke", brush.label())
            } else {
                format!("{} Stroke", brush.label())
            })
        {
            failures.push(format!("{brush:?}: no stroke step ({})", a.status));
        }
    }
    assert!(
        failures.is_empty(),
        "slow sculpt paths:\n{}",
        failures.join("\n")
    );
}

#[test]
fn paint_selection_drags_stay_on_the_fast_path() {
    // Left-drag painting in each component mode, with fast pointer moves.
    let n = 24u32;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut failures = Vec::new();
    for mode in [Mode::Polygon, Mode::Edge, Mode::Vertex] {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid(n).into();
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::CatmullClark,
        ));
        a.force_resident = true;
        a.camera.view = crate::camera::View::Perspective;
        a.frame(false);
        a.run(Command::Mode(mode));
        for _ in 0..2 {
            super::tests::draw(&mut a, &ctx, size, vec![]);
        }
        let start = a.viewport_rect.center() - vec2(150., 60.);
        let at = |k: f32| start + vec2(k * 30., k * 11.);
        super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![
                Event::PointerMoved(at(0.)),
                super::tests::pointer(at(0.), true, Modifiers::NONE),
            ],
        );
        for k in 1..3 {
            super::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(at(k as f32))]);
        }
        let painted = |a: &Operandi| {
            let s = &a.editor.scene.selection;
            s.faces.len() + s.edges.len() + s.vertices.len()
        };
        let counts = |a: &Operandi| {
            [
                a.subdivision_cache.full_builds(),
                a.resident_cache.builds(),
                a.surface_cache.builds(),
                operandi_core::topology_cache::computed(),
            ]
        };
        let (before, painted_before) = (counts(&a), painted(&a));
        for k in 3..10 {
            super::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(at(k as f32))]);
        }
        let after = counts(&a);
        if painted(&a) <= painted_before {
            failures.push(format!("{mode:?}: the drag painted nothing new"));
        }
        for (i, what) in [
            "subdivision surface evaluated in full",
            "GPU geometry rebuilt in full",
            "picking surface rebuilt",
            "topology recomputed",
        ]
        .iter()
        .enumerate()
        {
            if after[i] != before[i] {
                failures.push(format!(
                    "{mode:?} paint: {what} {} times in 7 drag frames",
                    after[i] - before[i]
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "slow selection paths:\n{}",
        failures.join("\n")
    );
}

#[test]
fn item_transform_drags_stay_on_the_fast_path() {
    // Item mode: Move, Rotate and Scale change only the item's placement,
    // so after the first frames the cached GPU geometry is drawn through a
    // matrix (gpu_scene.rs `delta`) instead of being rebuilt every frame.
    let n = 24u32;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut failures = Vec::new();
    // In Item mode each tool reads `delta`: metres, degrees, factors.
    let steps: [(Tool, Step); 3] = [
        (Tool::Move, |p, k| p.delta.z = 0.01 * k),
        (Tool::Rotate, |p, k| p.delta.z = 5. * k),
        (Tool::Scale, |p, k| p.delta = DVec3::splat(1. + 0.05 * k)),
    ];
    for (tool, step) in steps {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid(n).into();
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::CatmullClark,
        ));
        a.force_resident = true;
        a.camera.view = crate::camera::View::Perspective;
        a.frame(false);
        a.run(Command::Mode(Mode::Item));
        super::tests::draw(&mut a, &ctx, size, vec![]);
        a.activate(tool);
        let Some(base) = a.editor.session.live.as_ref().map(|l| l.params.clone()) else {
            failures.push(format!("{tool:?}: didn't activate ({})", a.status));
            continue;
        };
        let placement = a.editor.scene.active().unwrap().transform.clone();
        let frame = |a: &mut Operandi, k: f64| {
            let mut p = base.clone();
            step(&mut p, k);
            let ok = a.run_ok(Command::Preview(p));
            super::tests::draw(a, &ctx, size, vec![]);
            ok
        };
        if !(frame(&mut a, 1.) && frame(&mut a, 2.)) {
            failures.push(format!("{tool:?}: preview refused ({})", a.status));
            continue;
        }
        let counts = |a: &Operandi| {
            [
                a.subdivision_cache.full_builds(),
                a.resident_cache.builds(),
                a.surface_cache.builds(),
                operandi_core::topology_cache::computed(),
            ]
        };
        let before = counts(&a);
        for k in 3..7 {
            if !frame(&mut a, k as f64) {
                failures.push(format!("{tool:?}: preview refused ({})", a.status));
                break;
            }
        }
        let after = counts(&a);
        if a.editor.scene.active().unwrap().transform == placement {
            failures.push(format!("{tool:?}: the item didn't move"));
        }
        for (i, what) in [
            "subdivision surface evaluated in full",
            "GPU geometry rebuilt in full",
            "picking surface rebuilt",
            "topology recomputed",
        ]
        .iter()
        .enumerate()
        {
            if after[i] != before[i] {
                failures.push(format!(
                    "{tool:?} (Item mode): {what} {} times in 4 drag frames",
                    after[i] - before[i]
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "slow item drag paths:\n{}",
        failures.join("\n")
    );
}

#[test]
fn scene_changes_under_a_resting_pointer_keep_the_picking_surface() {
    // The item moved every frame without a pointer drag (a numeric scrub,
    // typed values, the MCP bridge) while the pointer rests over the view:
    // the rollover must not rebuild the whole picking surface each frame.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = grid(24).into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    a.force_resident = true;
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    a.run(Command::Mode(Mode::Item));
    super::tests::draw(&mut a, &ctx, size, vec![]);
    let over = a.viewport_rect.center();
    super::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(over)]);
    a.activate(Tool::Move);
    let base = a.editor.session.live.as_ref().unwrap().params.clone();
    let frame = |a: &mut Operandi, k: f64| {
        let mut p = base.clone();
        p.delta.x = 0.01 * k;
        a.run(Command::Preview(p));
        super::tests::draw(a, &ctx, size, vec![Event::PointerMoved(over)]);
    };
    frame(&mut a, 1.);
    frame(&mut a, 2.);
    let before = a.surface_cache.builds();
    for k in 3..7 {
        frame(&mut a, k as f64);
    }
    assert_eq!(a.surface_cache.builds(), before, "picking surface rebuilt");
    // Once the scene holds still the rollover picks again.
    for _ in 0..2 {
        super::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(over)]);
    }
    assert_eq!(a.surface_cache.builds(), before + 1);
}

/// Bug report "Lag after modeling operation on a large model": after each
/// Transform drag in a scene of many meshes, the first still frame rebuilt
/// the picking surface of every mesh (1-2 s on a 400-mesh import). Only
/// the edited mesh's part may rebuild; the rest are reused, and picking
/// still matches a surface built from scratch.
#[test]
fn a_drag_release_rebuilds_picking_for_the_edited_mesh_only() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut a = Operandi::empty();
    for k in 0..12 {
        a.run(Command::NewMesh);
        let mut mesh = grid(6);
        for v in &mut mesh.vertices {
            v.x += (k % 4) as f64 * 1.5;
            v.y += (k / 4) as f64 * 1.5;
        }
        a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    }
    // Draw as with a graphics card (overlays don't read the surface).
    a.force_resident = true;
    let edited = a.editor.scene.items[5].id;
    a.editor.scene.selection.item = Some(edited);
    a.editor.scene.selection.also.clear();
    a.camera.view = crate::camera::View::Top;
    a.frame(false);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = (10..14).collect();
    a.activate(Tool::Transform);
    super::tests::draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    let start = (0..150)
        .flat_map(|y| (0..200).map(move |x| (x, y)))
        .map(|(x, y)| r.left_top() + vec2(x as f32 * 4. + 10., y as f32 * 4. + 10.))
        .find(|&p| a.handle_hit(p, r).is_some())
        .expect("a Transform handle on screen");
    let over = start + vec2(1., 1.);
    // The rollover has a surface before the drag.
    super::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(start)]);
    let _ = a.pick_surface();
    let (builds, parts) = (a.surface_cache.builds(), a.surface_cache.part_builds());
    let before = a.editor.scene.item(edited).unwrap().mesh.clone();
    super::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![super::tests::pointer(start, true, Modifiers::NONE)],
    );
    let mut end = start;
    for k in 1..6 {
        end = start + vec2(k as f32 * 3., k as f32 * -2.);
        super::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(end)]);
    }
    super::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![super::tests::pointer(end, false, Modifiers::NONE)],
    );
    for _ in 0..3 {
        super::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(over)]);
    }
    let surface = a.pick_surface();
    assert_ne!(
        a.editor.scene.item(edited).unwrap().mesh,
        before,
        "the drag moved the polygons"
    );
    assert!(
        a.surface_cache.builds() > builds,
        "picking follows the edit"
    );
    assert_eq!(
        a.surface_cache.part_builds() - parts,
        1,
        "only the edited mesh's picking part rebuilds"
    );
    // Reused parts pick exactly as a fresh surface does.
    let scene = a
        .editor
        .evaluated(operandi_core::evaluate::Purpose::Pick)
        .into_owned();
    let fresh = crate::picking::Surface::with_smooth(&scene, Some(&a.subdivision_cache));
    for y in (0..600).step_by(37) {
        for x in (0..800).step_by(41) {
            let p = r.left_top() + vec2(x as f32, y as f32);
            assert_eq!(
                surface.pick(&a.camera, p, r),
                fresh.pick(&a.camera, p, r),
                "at {p:?}"
            );
        }
    }
}

#[test]
fn many_pointer_moves_in_a_frame_run_one_tool_update() {
    // A real mouse reports several moves per frame; a handle drag updates
    // the tool once, at the latest position, and lands where one move
    // there would.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let setup = || {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid(8).into();
        a.camera.view = crate::camera::View::Perspective;
        a.frame(false);
        a.run(Command::Mode(Mode::Item));
        a.activate(Tool::Move);
        super::tests::draw(&mut a, &ctx, size, vec![]);
        let r = a.viewport_rect;
        let start = (0..200)
            .flat_map(|y| (0..200).map(move |x| (x, y)))
            .map(|(x, y)| r.center() + vec2(x as f32 * 2. - 200., y as f32 * 2. - 200.))
            .find(|&p| a.handle_hit(p, r).is_some())
            .expect("a Move handle");
        super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![
                Event::PointerMoved(start),
                super::tests::pointer(start, true, Modifiers::NONE),
            ],
        );
        (a, start)
    };
    let (mut many, start) = setup();
    let publication = many.editor.identities.publication;
    let moves = (1..=20)
        .map(|k| Event::PointerMoved(start + vec2(k as f32 * 2., k as f32)))
        .collect();
    super::tests::draw(&mut many, &ctx, size, moves);
    let updates = many.editor.identities.publication - publication;
    assert!(updates <= 2, "{updates} tool updates for one frame");
    let (mut one, start) = setup();
    super::tests::draw(
        &mut one,
        &ctx,
        size,
        vec![Event::PointerMoved(start + vec2(40., 20.))],
    );
    assert!(many.navigation.is_some());
    assert_eq!(
        many.editor.scene.active().unwrap().transform,
        one.editor.scene.active().unwrap().transform
    );
    assert_ne!(
        many.editor.scene.active().unwrap().transform,
        Default::default()
    );
}

#[test]
fn whole_mesh_component_drags_draw_the_cached_mesh() {
    // Polygon mode with nothing selected: Move, Rotate and Scale move every
    // point of the mesh together (an imported scene starts in Polygon mode,
    // so this is how a whole mesh gets dragged). After the first frames
    // nothing about the mesh is rebuilt, patched or re-evaluated: the
    // cached mesh is drawn through the drag's map.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut failures = Vec::new();
    let steps: [(Tool, Step); 3] = [
        (Tool::Move, |p, k| {
            p.delta = DVec3::new(0.01 * k, 0.005 * k, 0.)
        }),
        // Rotate and Scale read `delta` too (degrees, factors).
        (Tool::Rotate, |p, k| p.delta.z = 3. * k),
        (Tool::Scale, |p, k| p.delta = DVec3::splat(1. + 0.02 * k)),
    ];
    for (tool, step) in steps {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid(24).into();
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::CatmullClark,
        ));
        a.force_resident = true;
        a.camera.view = crate::camera::View::Perspective;
        a.frame(false);
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces.clear();
        super::tests::draw(&mut a, &ctx, size, vec![]);
        a.activate(tool);
        let Some(base) = a.editor.session.live.as_ref().map(|l| l.params.clone()) else {
            failures.push(format!("{tool:?}: didn't activate ({})", a.status));
            continue;
        };
        let points = a.editor.scene.active().unwrap().mesh.vertices.clone();
        let frame = |a: &mut Operandi, k: f64| {
            let mut p = base.clone();
            step(&mut p, k);
            let ok = a.run_ok(Command::Preview(p));
            super::tests::draw(a, &ctx, size, vec![]);
            ok
        };
        if !(frame(&mut a, 1.) && frame(&mut a, 2.)) {
            failures.push(format!("{tool:?}: preview refused ({})", a.status));
            continue;
        }
        let counts = |a: &Operandi| {
            [
                a.resident_cache.builds(),
                a.resident_cache.patches(),
                a.subdivision_cache.full_builds(),
                a.subdivision_cache.updates(),
                a.surface_cache.builds(),
            ]
        };
        let before = counts(&a);
        for k in 3..7 {
            frame(&mut a, k as f64);
        }
        let after = counts(&a);
        if a.editor.scene.active().unwrap().mesh.vertices == points {
            failures.push(format!("{tool:?}: the mesh didn't move"));
        }
        for (i, what) in [
            "GPU geometry rebuilt",
            "GPU geometry patched",
            "subdivision evaluated in full",
            "subdivision re-evaluated",
            "picking surface rebuilt",
        ]
        .iter()
        .enumerate()
        {
            if after[i] != before[i] {
                failures.push(format!(
                    "{tool:?} (whole mesh): {what} {} times in 4 frames",
                    after[i] - before[i]
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "slow whole-mesh drags:\n{}",
        failures.join("\n")
    );
}

#[test]
fn whole_mesh_map_finds_one_shared_map_or_none() {
    let mesh = grid(6);
    let from = &mesh.vertices;
    let map = glam::DMat4::from_scale_rotation_translation(
        DVec3::new(1.5, 0.5, 2.),
        glam::DQuat::from_rotation_z(0.4),
        DVec3::new(0.3, -1., 2.),
    );
    let to: Vec<DVec3> = from.iter().map(|&p| map.transform_point3(p)).collect();
    let found = super::viewport::whole_mesh_map(from, &to).expect("a map");
    assert!(found.abs_diff_eq(map, 1e-9), "{found}");
    // One point moved on its own: no single map.
    let mut partial = to.clone();
    partial[7].z += 0.01;
    assert!(super::viewport::whole_mesh_map(from, &partial).is_none());
    // A mirror is refused (the cached triangles would face away).
    let mirrored: Vec<DVec3> = from.iter().map(|&p| DVec3::new(-p.x, p.y, p.z)).collect();
    assert!(super::viewport::whole_mesh_map(from, &mirrored).is_none());
}

/// Loop Slice moves a whole ring of points every frame, so its cost is the
/// width of that ring. On a dense subdivided grid, each drag frame must
/// re-subdivide only the ring's polygons and one ring of context around
/// them, compute each rewritten piece's normal once, and leave the same
/// surface and GPU geometry a full rebuild makes.
#[test]
fn loop_slice_drag_work_stays_near_the_ring() {
    let n = 48u32;
    let mid_point = (n / 2) * (n + 1) + n / 2;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = grid(n).into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    a.force_resident = true;
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(mid_point, mid_point + 1)];
    super::tests::draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::LoopSlice);
    let base = a
        .editor
        .session
        .live
        .as_ref()
        .expect("Loop Slice active")
        .params
        .clone();
    let frame = |a: &mut Operandi, k: f64| {
        let mut p = base.clone();
        p.slice.loop_slice.positions = vec![0.3 + 0.02 * k];
        assert!(a.run_ok(Command::Preview(p)), "{}", a.status);
        super::tests::draw(a, &ctx, size, vec![]);
    };
    // The first frames set the cut's topology.
    frame(&mut a, 1.);
    frame(&mut a, 2.);
    let resubdivided = operandi_core::subdivision::resubdivided_polygons();
    let (pieces, normals) = a.resident_cache.patch_piece_work();
    let frames = 4;
    let mut previous = a.editor.scene.active().unwrap().clone();
    for k in 3..3 + frames {
        previous = a.editor.scene.active().unwrap().clone();
        frame(&mut a, k as f64);
    }
    // The ring cuts n polygons into 2n; their pieces change along with one
    // ring of neighbours either side (4n), re-subdivided with one more ring
    // of context (6n). Three rings of context were 10n.
    let per_frame =
        (operandi_core::subdivision::resubdivided_polygons() - resubdivided) / frames as u64;
    assert!(
        per_frame <= 7 * n as u64,
        "{per_frame} polygons re-subdivided per frame for a {n}-cell ring"
    );
    let (pieces, normals) = {
        let (p, q) = a.resident_cache.patch_piece_work();
        (p - pieces, q - normals)
    };
    assert!(pieces > 0, "the drag patched no smooth pieces");
    assert!(
        normals * 2 <= pieces * 3,
        "{normals} piece normals computed for {pieces} rewritten pieces"
    );

    // The partial surface is the full rebuild, point for point.
    let item = a.editor.scene.active().unwrap().clone();
    let partial = a.subdivision_cache.get(&item).unwrap();
    let full = crate::subdivision_view::SubdivisionCache::default()
        .get(&item)
        .unwrap();
    assert_eq!(partial.mesh.faces, full.mesh.faces);
    assert!(
        partial
            .mesh
            .vertices
            .iter()
            .zip(&full.mesh.vertices)
            .all(|(p, f)| p.to_array().map(f64::to_bits) == f.to_array().map(f64::to_bits)),
        "the partial surface differs from a full rebuild"
    );

    // And one frame's GPU patch is the full build's geometry.
    let smoothing = operandi_core::shading::SmoothingSettings::default();
    let cache = crate::subdivision_view::SubdivisionCache::default();
    let before = cache.get(&previous).unwrap();
    let mut resident = crate::gpu_scene::Resident::default();
    resident.item(
        &previous,
        glam::DMat4::IDENTITY,
        None,
        Some(&before),
        &smoothing,
    );
    let after = cache.get(&item).unwrap();
    assert_eq!(cache.updates(), 1, "the surface wasn't updated in place");
    let (patched, _, patch) =
        resident.item(&item, glam::DMat4::IDENTITY, None, Some(&after), &smoothing);
    assert!(patch.is_some(), "a rebuild, not a patch");
    let fresh = crate::gpu_scene::Resident::default()
        .item(&item, glam::DMat4::IDENTITY, None, Some(&full), &smoothing)
        .0;
    assert!(
        patched.fill == fresh.fill,
        "patched fill differs from a full build"
    );
    assert!(
        patched.edges == fresh.edges,
        "patched wireframe differs from a full build"
    );
}

#[test]
fn image_layer_drags_compile_and_upload_nothing() {
    // An image map drawn per pixel (material_gpu.rs): component and item
    // drags must not read the picture again, make a new material snapshot
    // (which would re-upload textures or recompile the fill shader) or
    // rebuild the geometry.
    use operandi_core::shader_tree::{NewLayer, NodeKind, TextureEffect, TextureSource};
    let dir = std::env::temp_dir().join(format!("operandi-vit-guard-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ramp.png");
    let pixels: Vec<u8> = (0..64u32 * 64)
        .flat_map(|i| [(i % 64 * 4) as u8, (i / 64 * 4) as u8, 90])
        .collect();
    image::save_buffer(&path, &pixels, 64, 64, image::ExtendedColorType::Rgb8).unwrap();
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut failures = Vec::new();
    for (tool, mode) in [(Tool::Move, Mode::Polygon), (Tool::Move, Mode::Item)] {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = grid(24).into();
        let scene = &mut a.editor.scene;
        let base = scene.shader_tree.material_for_tag("Default").unwrap();
        let layer = scene
            .shader_tree
            .add(NewLayer::Constant, Some(base))
            .unwrap();
        let mut node = scene.shader_tree.get(layer).unwrap().clone();
        if let NodeKind::Texture(t) = &mut node.kind {
            t.effect = TextureEffect::DiffuseColor;
            t.source = TextureSource::ImageMap {
                image: Some(77),
                settings: Default::default(),
            };
        }
        scene.shader_tree.replace(node).unwrap();
        scene.images.push(operandi_core::backdrop::SceneImage {
            id: 77,
            name: "ramp".into(),
            path: path.to_str().unwrap().into(),
            width: 64,
            height: 64,
        });
        a.force_resident = true;
        a.camera.view = crate::camera::View::Perspective;
        a.frame(false);
        a.run(Command::Mode(mode));
        if mode == Mode::Polygon {
            a.editor.scene.selection.faces = vec![300, 301];
        } else {
            // A selected item draws in the selection fill unless Show
            // Selected Filling is off; then its image shows while it moves.
            a.display.settings.drawing.selected_filling = false;
        }
        super::tests::draw(&mut a, &ctx, size, vec![]);
        a.activate(tool);
        let Some(base) = a.editor.session.live.as_ref().map(|l| l.params.clone()) else {
            failures.push(format!("{tool:?} {mode:?}: didn't activate"));
            continue;
        };
        let frame = |a: &mut Operandi, k: f64| {
            let mut p = base.clone();
            p.delta.z = 0.01 * k;
            a.run(Command::Preview(p));
            super::tests::draw(a, &ctx, size, vec![]);
        };
        frame(&mut a, 1.);
        frame(&mut a, 2.);
        let counts = |a: &Operandi| {
            [
                a.resident_cache.builds(),
                a.resident_cache.image_reads(),
                a.resident_cache.material_generation(),
            ]
        };
        let before = counts(&a);
        if before[2] == 0 {
            failures.push(format!(
                "{tool:?} {mode:?}: the image isn't drawn per pixel"
            ));
        }
        for k in 3..7 {
            frame(&mut a, k as f64);
        }
        let after = counts(&a);
        for (i, what) in [
            "GPU geometry rebuilt in full",
            "image file read",
            "material snapshot (upload/compile) made",
        ]
        .iter()
        .enumerate()
        {
            if after[i] != before[i] {
                failures.push(format!(
                    "{tool:?} {mode:?}: {what} {} times in 4 drag frames",
                    after[i] - before[i]
                ));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        failures.is_empty(),
        "slow image drags:\n{}",
        failures.join("\n")
    );
}

/// Live patches (G7 PC-7): dragging a boundary curve's point redraws the
/// patch every frame, but its cells keep their topology, so the drag stays
/// on the fast path (no full GPU rebuild, no topology recompute).
#[test]
fn live_patch_redraw_stays_on_the_fast_path() {
    use operandi_core::PolygonRepresentation;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let mut m = operandi_core::Mesh {
        vertices: vec![
            DVec3::new(-1., 0., -1.),
            DVec3::new(1., 0., -1.),
            DVec3::new(1., 0., 1.),
            DVec3::new(-1., 0., 1.),
        ],
        ..Default::default()
    };
    let face = |vertices: Vec<u32>| Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    };
    for k in 0..4u32 {
        let (p, q) = (m.vertices[k as usize], m.vertices[((k + 1) % 4) as usize]);
        m.vertices.push((p + q) / 2. + DVec3::Y * 0.2);
        m.faces.push(face(vec![k, 4 + k, (k + 1) % 4]));
        m.attributes
            .representations
            .push(PolygonRepresentation::Polyline {
                closed: false,
                curve: operandi_core::CurveKind::Curve {
                    start_control: false,
                    end_control: false,
                },
            });
    }
    m.faces.push(face(vec![0, 1, 2, 3]));
    m.attributes
        .representations
        .push(PolygonRepresentation::Patch);
    a.editor.scene.active_mut().unwrap().mesh = m.into();
    a.force_resident = true;
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![4];
    super::tests::draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::Move);
    let base = a.editor.session.live.as_ref().unwrap().params.clone();
    let frame = |a: &mut Operandi, k: f64| {
        let mut p = base.clone();
        p.delta.y = 0.01 * k;
        assert!(a.run_ok(Command::Preview(p)), "{}", a.status);
        super::tests::draw(a, &ctx, size, vec![]);
    };
    frame(&mut a, 1.);
    frame(&mut a, 2.);
    let counts = |a: &Operandi| {
        [
            a.resident_cache.builds(),
            operandi_core::topology_cache::computed(),
        ]
    };
    let before = counts(&a);
    for k in 3..7 {
        frame(&mut a, k as f64);
    }
    let after = counts(&a);
    assert_eq!(after, before, "[GPU full rebuilds, topology recomputes]");
    // And the drawn patch did follow the curve.
    let shown = a
        .editor
        .evaluated(operandi_core::evaluate::Purpose::Display { base_edit: None });
    let s = &shown.items[0].mesh;
    assert_eq!(
        (0..s.faces.len()).filter(|&f| !s.is_polyline(f)).count(),
        256
    );
}
