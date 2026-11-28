//! The Transform pipe in UV space (specifications/uv-2026-11-16,
//! BRIEF-TRANSFORM-PIPE-IN-UV, UVT-130..136): Move, Element Move, the
//! falloff presets and Smooth run their own 3D code on the UV proxy and
//! write back U and V only.
use glam::{DVec2, DVec3};
use operandi_core::uv_pipe::UvProxy;
use operandi_core::*;

fn face(v: &[u32]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A 2 m cube with the primitive "Texture" UVs: every side its own island.
fn cube() -> Mesh {
    let (position, size) = (DVec3::ZERO, DVec3::splat(2.));
    let mut mesh = primitive::segmented_cube(position, size, [1; 3]).unwrap();
    add_cube_uvs(&mut mesh, position, size).unwrap();
    mesh
}

/// Two quads sharing edge 1-4 in 3D. `seam`: the right quad's UVs sit
/// 0.5 to the right, so the strip is two UV islands.
fn strip(seam: bool) -> Mesh {
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(0., 0., 1.),
            DVec3::new(1., 0., 1.),
            DVec3::new(2., 0., 1.),
        ],
        faces: vec![face(&[0, 1, 4, 3]), face(&[1, 2, 5, 4])].into(),
        ..Default::default()
    };
    let o = if seam { 0.5 } else { 0. };
    let p = |u: f64, v: f64| Some(DVec2::new(u, v));
    m.uv_maps.insert(
        "Texture".into(),
        vec![
            vec![p(0., 0.), p(0.5, 0.), p(0.5, 0.5), p(0., 0.5)],
            vec![
                p(0.5 + o, 0.),
                p(1. + o, 0.),
                p(1. + o, 0.5),
                p(0.5 + o, 0.5),
            ],
        ],
    );
    m
}

/// A 2x2-quad grid in XZ (points 3x3, point 4 in the middle) with UVs
/// (x/2, z/2): one island over the 0-1 tile.
fn grid() -> Mesh {
    let mut m = Mesh::default();
    for z in 0..3 {
        for x in 0..3 {
            m.vertices.push(DVec3::new(x as f64, 0., z as f64));
        }
    }
    let at = |x: u32, z: u32| z * 3 + x;
    let mut faces = Vec::new();
    let mut rows = Vec::new();
    for z in 0..2 {
        for x in 0..2 {
            let v = [at(x, z), at(x + 1, z), at(x + 1, z + 1), at(x, z + 1)];
            faces.push(face(&v));
            rows.push(
                v.iter()
                    .map(|&p| {
                        let q = m.vertices[p as usize];
                        Some(DVec2::new(q.x / 2., q.z / 2.))
                    })
                    .collect(),
            );
        }
    }
    m.faces = faces.into();
    m.uv_maps.insert("Texture".into(), rows);
    m
}

fn editor_with(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn uvs(e: &Editor) -> Vec<Vec<DVec2>> {
    mesh(e).uv_maps["Texture"]
        .iter()
        .map(|r| r.iter().map(|p| p.unwrap()).collect())
        .collect()
}

fn near(a: DVec2, b: DVec2) -> bool {
    (a - b).length() < 1e-9
}

/// Start `tool` (or a preset) in UV space and preview `edit`.
fn uv_preview(e: &mut Editor, start: Command, edit: impl FnOnce(&Editor, &mut Parameters)) {
    e.dispatch(start).unwrap();
    e.dispatch(Command::UvToolSpace(Some(None))).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    edit(e, &mut p);
    e.dispatch(Command::Preview(p)).unwrap();
}

/// The proxy point of polygon `f`'s corner `c` (the tool's own numbering
/// of UV positions, which a UV view picks from).
fn uv_point(e: &Editor, f: usize, c: usize) -> u32 {
    let proxy = UvProxy::build(&e.scene, "Texture", false).unwrap();
    proxy.corners()[f][c]
}

fn grab(e: &Editor, f: usize, c: usize, p: &mut Parameters) {
    p.tweak = Some(TweakGrab {
        item: e.scene.active().unwrap().id,
        element: TweakElement::Vertex(uv_point(e, f, c)),
    });
}

#[test]
fn move_u_on_one_cube_polygon_leaves_its_seam_neighbours_and_3d() {
    let mut e = editor_with(cube(), Mode::Polygon);
    e.scene.selection.faces = vec![0];
    let (before, points) = (uvs(&e), mesh(&e).vertices.clone());
    let m = mesh(&e).clone();
    uv_preview(&mut e, Command::Activate(Tool::Move), |_, p| {
        p.delta = DVec3::new(0.25, 0., 0.)
    });
    e.dispatch(Command::Drop).unwrap();
    let after = uvs(&e);
    // A corner moves when it is one of polygon 0's UV positions: polygon 0
    // itself, or a neighbour joined to it in UV. A neighbour across a seam
    // (same 3D point, other UV) stays.
    let own: Vec<(u32, DVec2)> = m.faces[0]
        .vertices
        .iter()
        .zip(&before[0])
        .map(|(&p, &uv)| (p, uv))
        .collect();
    let mut seam_corners = 0;
    for (f, (a, b)) in after.iter().zip(&before).enumerate() {
        for (c, (x, y)) in a.iter().zip(b).enumerate() {
            let point = m.faces[f].vertices[c];
            let shared = own.iter().any(|&(p, uv)| p == point && near(uv, *y));
            let want = if shared {
                *y + DVec2::new(0.25, 0.)
            } else {
                *y
            };
            if !shared && own.iter().any(|&(p, _)| p == point) {
                seam_corners += 1;
            }
            assert!(near(*x, want), "polygon {f} corner {c}: {x} vs {want}");
        }
    }
    assert!(seam_corners > 0, "the cube's map has seams at polygon 0");
    assert_eq!(mesh(&e).vertices, points, "UVT-06: 3D never moves");
}

#[test]
fn element_move_range_zero_moves_only_the_grabbed_uv_point() {
    let mut e = editor_with(strip(false), Mode::Vertex);
    let before = uvs(&e);
    uv_preview(&mut e, Command::Activate(Tool::Tweak), |e, p| {
        grab(e, 0, 2, p);
        p.tweak_options.range = 0.;
        p.delta = DVec3::new(0., 0.1, 0.);
    });
    e.dispatch(Command::Drop).unwrap();
    let after = uvs(&e);
    // Point 4's one UV position is shared by both quads: both corners move.
    for (f, c) in [(0, 0), (0, 1), (0, 3), (1, 0), (1, 1), (1, 2)] {
        assert!(near(after[f][c], before[f][c]), "{f}/{c} stays");
    }
    for (f, c) in [(0, 2), (1, 3)] {
        assert!(near(after[f][c], before[f][c] + DVec2::new(0., 0.1)));
    }
}

#[test]
fn element_move_at_a_seam_moves_only_the_clicked_uv_position() {
    let mut e = editor_with(strip(true), Mode::Vertex);
    let before = uvs(&e);
    uv_preview(&mut e, Command::Activate(Tool::Tweak), |e, p| {
        grab(e, 0, 1, p); // point 1 on the left island
        p.delta = DVec3::new(-0.1, 0., 0.);
    });
    let after = uvs(&e);
    assert!(near(after[0][1], before[0][1] - DVec2::new(0.1, 0.)));
    assert!(
        near(after[1][0], before[1][0]),
        "UVT-131: the other side stays"
    );
}

#[test]
fn element_move_with_connectivity_keeps_to_the_uv_island() {
    // Range 1.0 reaches the right island, 0.5 away across the seam.
    let run = |connect| {
        let mut e = editor_with(strip(true), Mode::Vertex);
        let before = uvs(&e);
        uv_preview(&mut e, Command::Activate(Tool::Tweak), |e, p| {
            grab(e, 0, 1, p);
            p.tweak_options.range = 1.0;
            p.tweak_options.connect = connect;
            p.delta = DVec3::new(0., 0.2, 0.);
        });
        let after = uvs(&e);
        let right_moved = (0..4).any(|c| !near(after[1][c], before[1][c]));
        let left_moved = (0..4).all(|c| !near(after[0][c], before[0][c]));
        (left_moved, right_moved)
    };
    assert_eq!(run(TweakConnect::Ignore), (true, true));
    assert_eq!(
        run(TweakConnect::Connectivity),
        (true, false),
        "UVT-Q12: the other island doesn't move"
    );
}

#[test]
fn element_move_rigid_moves_the_whole_island() {
    let mut e = editor_with(strip(true), Mode::Vertex);
    let before = uvs(&e);
    uv_preview(&mut e, Command::Activate(Tool::Tweak), |e, p| {
        grab(e, 0, 0, p);
        p.tweak_options.connect = TweakConnect::Rigid;
        p.delta = DVec3::new(0.3, 0., 0.);
    });
    let after = uvs(&e);
    for c in 0..4 {
        assert!(near(after[0][c], before[0][c] + DVec2::new(0.3, 0.)));
        assert!(near(after[1][c], before[1][c]));
    }
}

#[test]
fn rotate_90_about_the_selection_centre_turns_counter_clockwise() {
    let mut e = editor_with(strip(true), Mode::Polygon);
    e.scene.selection.faces = vec![0];
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Selection,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    // Make the left quad the unit square.
    let mut m = mesh(&e).clone();
    let mut rows = m.uv_maps["Texture"].clone();
    rows[0] = [(0., 0.), (1., 0.), (1., 1.), (0., 1.)]
        .map(|(u, v)| Some(DVec2::new(u, v)))
        .to_vec();
    rows[1] = rows[1]
        .iter()
        .map(|p| p.map(|p| p + DVec2::new(2., 0.)))
        .collect();
    m.uv_maps.insert("Texture".into(), rows);
    e.scene.active_mut().unwrap().mesh = m.into();
    uv_preview(&mut e, Command::Activate(Tool::Rotate), |_, p| {
        p.delta = DVec3::new(0., 0., 90.)
    });
    let after = uvs(&e);
    for (got, want) in after[0]
        .iter()
        .zip([(1., 0.), (1., 1.), (0., 1.), (0., 0.)])
    {
        assert!(near(*got, DVec2::new(want.0, want.1)), "{after:?}");
    }
}

#[test]
fn soft_move_falloff_is_fitted_and_measured_in_uv() {
    let mut e = editor_with(grid(), Mode::Polygon);
    e.scene.selection.faces = vec![0, 1, 2, 3];
    let (before, points) = (uvs(&e), mesh(&e).vertices.clone());
    uv_preview(
        &mut e,
        Command::ActivatePreset("xfrm.magnet".into()),
        |_, p| p.delta = DVec3::new(0., 0.1, 0.),
    );
    let Some(Falloff::Radial(r)) = e.spatial_falloff else {
        panic!("Soft Move has a radial falloff");
    };
    assert!(
        (r.center - DVec3::new(0.5, 0.5, 0.)).length() < 1e-9
            && (r.size - DVec3::new(0.5, 0.5, 0.)).length() < 1e-9,
        "fitted to the UV selection: {r:?}"
    );
    let after = uvs(&e);
    // Polygon 0's corner 2 is the middle point: full weight. Its corner 0
    // is the tile's corner: on the falloff's edge, weight 0.
    assert!(
        near(after[0][2], before[0][2] + DVec2::new(0., 0.1)),
        "{after:?}"
    );
    assert!(near(after[0][0], before[0][0]));
    // A side's middle is outside the ellipse's reach too (d = 1).
    assert!(near(after[0][1], before[0][1]));
    assert_eq!(mesh(&e).vertices, points);
}

#[test]
fn smooth_relaxes_uvs_and_leaves_3d() {
    let mut m = strip(false);
    // Pull one interior UV out of line.
    let mut rows = m.uv_maps["Texture"].clone();
    rows[0][1] = Some(DVec2::new(0.5, -0.2));
    rows[1][0] = Some(DVec2::new(0.5, -0.2));
    m.uv_maps.insert("Texture".into(), rows);
    let mut e = editor_with(m, Mode::Vertex);
    e.scene.selection.vertices = vec![1];
    let points = mesh(&e).vertices.clone();
    uv_preview(&mut e, Command::Activate(Tool::Smooth), |_, p| {
        p.smooth.strength = 1.;
        p.smooth.iterations = 1;
        p.smooth.lock_boundary = false;
    });
    let after = uvs(&e);
    assert!(after[0][1].y > -0.2 + 1e-6, "{after:?}");
    assert!(near(after[0][1], after[1][0]), "one UV position");
    assert_eq!(mesh(&e).vertices, points);
}

#[test]
fn uv_tool_session_is_one_undo_step_and_cancel_restores() {
    let mut e = editor_with(strip(true), Mode::Vertex);
    let before = uvs(&e);
    uv_preview(&mut e, Command::Activate(Tool::Tweak), |e, p| {
        grab(e, 0, 0, p);
        p.delta = DVec3::new(0.3, 0., 0.);
    });
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(uvs(&e), before);
    uv_preview(&mut e, Command::Activate(Tool::Tweak), |e, p| {
        grab(e, 0, 0, p);
        p.delta = DVec3::new(0.3, 0., 0.);
    });
    e.dispatch(Command::Drop).unwrap();
    assert_ne!(uvs(&e), before);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(uvs(&e), before);
}

#[test]
fn symmetry_is_off_in_uv_space() {
    let mut e = editor_with(strip(true), Mode::Vertex);
    e.symmetry = Symmetry {
        axis: Some(0),
        ..Symmetry::OFF
    };
    let before = uvs(&e);
    uv_preview(&mut e, Command::Activate(Tool::Tweak), |e, p| {
        grab(e, 0, 0, p);
        p.delta = DVec3::new(0., 0.1, 0.);
    });
    let after = uvs(&e);
    let moved = (0..2)
        .flat_map(|f| (0..4).map(move |c| (f, c)))
        .filter(|&(f, c)| !near(after[f][c], before[f][c]))
        .count();
    assert_eq!(moved, 1, "UVT-136: no mirrored UV moves");
}

#[test]
fn the_same_tools_in_3d_still_move_points() {
    let mut e = editor_with(strip(true), Mode::Polygon);
    e.scene.selection.faces = vec![0];
    let before = uvs(&e);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::new(0., 1., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(mesh(&e).vertices[0], DVec3::new(0., 1., 0.));
    assert_eq!(uvs(&e), before, "a 3D move leaves the UVs");
}

#[test]
fn uvt133_origin_and_element_centres_are_uv_points() {
    let centre = |e: &mut Editor, mode: CenterMode| {
        e.dispatch(Command::ActionContext(ActionContext {
            center: mode,
            axis: AxisOrientation::World,
        }))
        .unwrap();
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center
    };
    let mut e = editor_with(strip(true), Mode::Polygon);
    e.scene.selection.faces = vec![1];
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    e.dispatch(Command::UvToolSpace(Some(None))).unwrap();
    // Selection: the UV selection's centre (the right island).
    let c = e
        .session
        .live
        .as_ref()
        .unwrap()
        .action_frame
        .unwrap()
        .center;
    assert!((c - DVec3::new(1.25, 0.25, 0.)).length() < 1e-9, "{c}");
    assert_eq!(centre(&mut e, CenterMode::Origin), DVec3::ZERO);
    let picked = PickedPoint::new(DVec3::new(1., 0.5, 0.));
    let c = centre(&mut e, CenterMode::Element(picked));
    assert!((c - DVec3::new(1., 0.5, 0.)).length() < 1e-9, "{c}");
    // Scaling by 2 about that corner keeps it and doubles the rest.
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::new(2., 2., 1.);
    e.dispatch(Command::Preview(p)).unwrap();
    let u = uvs(&e);
    assert!(near(u[1][3], DVec2::new(1., 0.5)));
    assert!(near(u[1][2], DVec2::new(2., 0.5)));
    assert!(near(u[1][0], DVec2::new(1., -0.5)), "{u:?}");
}

#[test]
fn uvt137_slide_in_a_uv_view_slides_uvs_along_uv_edges() {
    let mut e = editor_with(strip(false), Mode::Edge);
    e.scene.selection.edges = vec![(1, 4)];
    let (before, points) = (uvs(&e), mesh(&e).vertices.clone());
    uv_preview(&mut e, Command::Activate(Tool::EdgeSlide), |_, p| {
        p.shift = 0.5
    });
    e.dispatch(Command::Drop).unwrap();
    let after = uvs(&e);
    // The shared middle column (U 0.5) slides half way to one side in U.
    let du = after[0][1].x - before[0][1].x;
    assert!((du.abs() - 0.25).abs() < 1e-9, "{after:?}");
    assert!(near(after[0][2] - before[0][2], DVec2::new(du, 0.)));
    assert!(near(after[1][0], after[0][1]), "one UV position");
    assert!(near(after[0][0], before[0][0]));
    assert_eq!(mesh(&e).vertices, points, "UVT-06");
}

#[test]
fn jitter_on_uvs_moves_u_and_v_only_and_keeps_3d() {
    let mut e = editor_with(grid(), Mode::Polygon);
    e.scene.selection.faces = vec![0, 1, 2, 3];
    let (before, points) = (uvs(&e), mesh(&e).vertices.clone());
    uv_preview(&mut e, Command::Activate(Tool::Jitter), |_, p| {
        p.jitter.range = DVec3::new(0.05, 0.05, 5.);
    });
    e.dispatch(Command::Drop).unwrap();
    let after = uvs(&e);
    let mut moved = false;
    for (a, b) in after.iter().flatten().zip(before.iter().flatten()) {
        let d = *a - *b;
        assert!(
            d.x.abs() <= 0.05 + 1e-12 && d.y.abs() <= 0.05 + 1e-12,
            "{d:?}"
        );
        moved |= d.length() > 1e-9;
    }
    assert!(moved, "Jitter moved some UVs");
    // Corner 2 of polygon 0 and corner 3 of polygon 1 are the same UV
    // position (the middle point): it stays one position.
    assert!(near(after[0][2], after[1][3]));
    assert_eq!(mesh(&e).vertices, points, "UVT-06");
}

#[test]
fn rigid_rotate_jitter_turns_uv_islands_in_the_uv_plane() {
    let mut e = editor_with(strip(true), Mode::Polygon);
    let before = uvs(&e);
    uv_preview(&mut e, Command::Activate(Tool::Jitter), |_, p| {
        p.jitter.range = DVec3::new(0.2, 0.2, 0.);
        p.jitter.rigid_rotate = true;
    });
    e.dispatch(Command::Drop).unwrap();
    let after = uvs(&e);
    // A turn keeps each island's edge lengths (a tilt out of the plane
    // would shorten them).
    for f in 0..2 {
        let len = |r: &Vec<Vec<DVec2>>, i: usize| (r[f][i] - r[f][(i + 1) % 4]).length();
        for i in 0..4 {
            assert!((len(&after, i) - len(&before, i)).abs() < 1e-9, "{after:?}");
        }
    }
    assert_ne!(after, before);
}

#[test]
fn quantize_on_uvs_snaps_u_and_v_to_the_step() {
    let mut m = strip(false);
    let mut rows = m.uv_maps["Texture"].clone();
    rows[0][0] = Some(DVec2::new(0.07, 0.03));
    m.uv_maps.insert("Texture".into(), rows);
    let mut e = editor_with(m, Mode::Vertex);
    e.scene.selection.vertices = vec![0];
    uv_preview(&mut e, Command::Activate(Tool::Quantize), |_, p| {
        p.quantize.step = DVec3::new(0.1, 0.1, 0.1);
    });
    e.dispatch(Command::Drop).unwrap();
    assert!(near(uvs(&e)[0][0], DVec2::new(0.1, 0.)), "{:?}", uvs(&e));
}

#[test]
fn soft_drag_in_a_uv_view_moves_by_the_uv_stamped_screen_weights() {
    // Soft Drag = Move + a Screen falloff; the UV view stamps the proxy's
    // points (UV_STAMP_LAYER), so 3D stamps never apply to UVs or back.
    let mut e = editor_with(grid(), Mode::Polygon);
    let (before, points) = (uvs(&e), mesh(&e).vertices.clone());
    e.dispatch(Command::ActivatePreset("xfrm.softDrag".into()))
        .unwrap();
    e.dispatch(Command::UvToolSpace(Some(None))).unwrap();
    let proxy = UvProxy::build(&e.scene, "Texture", false).unwrap();
    let middle = proxy.corners()[0][2] as usize;
    let corner = proxy.corners()[0][0] as usize;
    let mut w = vec![0.; proxy.mesh().vertices.len()];
    w[middle] = 1.;
    w[proxy.corners()[0][1] as usize] = 0.5;
    let id = e.scene.active().unwrap().id;
    let mut stamps = std::collections::BTreeMap::new();
    stamps.insert((operandi_core::uv_pipe::UV_STAMP_LAYER, id), w);
    // A 3D stamp of every point at full weight must not reach the UVs.
    stamps.insert((0, id), vec![1.; points.len()]);
    e.dispatch(Command::FalloffStamps(stamps)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::new(0.1, 0., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let after = uvs(&e);
    assert!(
        near(after[0][2], before[0][2] + DVec2::new(0.1, 0.)),
        "{after:?}"
    );
    assert!(near(after[0][1], before[0][1] + DVec2::new(0.05, 0.)));
    assert!(near(after[0][0], before[0][0]), "{corner}: weight 0 stays");
    assert_eq!(mesh(&e).vertices, points, "UVT-06");
}

#[test]
fn uvt137_slides_uv_map_field_names_the_map_and_pins_the_space() {
    // Slide's UV Map field (UVT-Q16): naming a map puts the tool on that
    // map's UVs from any view, and says so (pinned) so the app keeps it.
    let mut e = editor_with(strip(false), Mode::Edge);
    e.scene.selection.edges = vec![(1, 4)];
    let points = mesh(&e).vertices.clone();
    let before = uvs(&e);
    e.dispatch(Command::Activate(Tool::EdgeSlide)).unwrap();
    e.dispatch(Command::UvToolSpace(Some(Some("Texture".into()))))
        .unwrap();
    let space = e.session.live.as_ref().unwrap().params.uv.clone().unwrap();
    assert!(space.pinned && space.map == "Texture");
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = 0.5;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_ne!(uvs(&e), before);
    assert_eq!(mesh(&e).vertices, points);
    // The view's own switch doesn't pin.
    e.dispatch(Command::Activate(Tool::EdgeSlide)).unwrap();
    e.dispatch(Command::UvToolSpace(Some(None))).unwrap();
    assert!(
        !e.session
            .live
            .as_ref()
            .unwrap()
            .params
            .uv
            .as_ref()
            .unwrap()
            .pinned
    );
}

#[test]
fn tear_off_splits_shared_authored_uv_owners_through_drop_and_reopen() {
    let mut m = strip(false);
    m.attributes.uv_records.insert(
        "Texture".into(),
        vec![
            vec![None, Some(7), None, None],
            vec![Some(7), None, None, None],
        ],
    );
    let mut e = editor_with(m.clone(), Mode::Polygon);
    e.scene.selection.faces = vec![0];
    uv_preview(&mut e, Command::Activate(Tool::Move), |_, p| {
        p.uv.as_mut().unwrap().tear_off = true;
        p.delta = DVec3::new(0.25, 0., 0.);
    });
    let check = |mesh: &MeshSlot| {
        mesh.validate().unwrap();
        mesh.to_owned_value().validate().unwrap();
        let owners = &mesh.attributes.uv_records["Texture"];
        assert_eq!(owners[1][0], Some(7));
        assert!(owners[0][1].is_some());
        assert_ne!(owners[0][1], owners[1][0]);
        assert_eq!(mesh.uv_maps["Texture"][0][1], Some(DVec2::new(0.75, 0.)));
        assert_eq!(mesh.uv_maps["Texture"][1][0], Some(DVec2::new(0.5, 0.)));
    };
    check(&e.scene.active().unwrap().mesh);
    e.dispatch(Command::Drop).unwrap();
    check(&e.scene.active().unwrap().mesh);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), &m);
    e.dispatch(Command::Redo).unwrap();
    check(&e.scene.active().unwrap().mesh);
    let reopened = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    check(&reopened.active().unwrap().mesh);
}
