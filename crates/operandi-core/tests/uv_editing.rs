//! UV editing stage 1 (specifications/uv-editing-2026-11-16): the UV map
//! list (E1), Move / Rotate / Scale / Transform in UV space (E6) and the
//! island affine engine (E7). Acceptance cases UV-01..UV-07 (UVE-1) and
//! UVT-A04, plus the per-tool criteria (empty selection, cancel, undo/redo,
//! save/reopen).
use glam::{DVec2, DVec3};
use operandi_core::uv_edit::*;
use operandi_core::uv_maps::{UvMapEdit, current_map};
use operandi_core::{Command, Editor, Face, Mesh, Mode, Parameters, Tool};

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

/// A 1 m plane in XZ with UVs (0,0) (1,0) (1,1) (0,1) in "Texture".
fn plane() -> Mesh {
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 0., 1.),
            DVec3::new(0., 0., 1.),
        ],
        faces: vec![face(&[0, 1, 2, 3])].into(),
        ..Default::default()
    };
    m.uv_maps.insert(
        "Texture".into(),
        vec![vec![
            Some(DVec2::new(0., 0.)),
            Some(DVec2::new(1., 0.)),
            Some(DVec2::new(1., 1.)),
            Some(DVec2::new(0., 1.)),
        ]],
    );
    m
}

/// Two quads side by side sharing edge 1-4. `seam`: the right quad's UVs
/// sit apart (a UV seam along the shared edge).
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

fn editor_with(mesh: Mesh) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn uvs(e: &Editor, map: &str) -> Vec<Vec<DVec2>> {
    mesh(e).uv_maps[map]
        .iter()
        .map(|r| r.iter().map(|p| p.unwrap_or(DVec2::NAN)).collect())
        .collect()
}

fn near(a: DVec2, b: DVec2) -> bool {
    (a - b).length() < 1e-9
}

fn assert_row(got: &[DVec2], want: &[(f64, f64)]) {
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(want) {
        assert!(near(*g, DVec2::new(w.0, w.1)), "{got:?} vs {want:?}");
    }
}

/// Start `tool` in UV space on the selected map and preview `edit`.
fn uv_tool(e: &mut Editor, tool: Tool, edit: impl FnOnce(&mut Parameters)) {
    e.dispatch(Command::Activate(tool)).unwrap();
    e.dispatch(Command::UvToolSpace(Some(None))).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    edit(&mut p);
    e.dispatch(Command::Preview(p)).unwrap();
}

// ---------------------------------------------------------------- E6

#[test]
fn uv01_scale_twice_about_the_centre_leaves_3d() {
    let mut e = editor_with(plane());
    let points = mesh(&e).vertices.clone();
    uv_tool(&mut e, Tool::Scale, |p| p.delta = DVec3::new(2., 2., 1.));
    e.dispatch(Command::Drop).unwrap();
    assert_row(
        &uvs(&e, "Texture")[0],
        &[(-0.5, -0.5), (1.5, -0.5), (1.5, 1.5), (-0.5, 1.5)],
    );
    assert_eq!(mesh(&e).vertices, points, "UVT-06: 3D never moves");
}

#[test]
fn uv02_rotate_90_counter_clockwise_moves_each_corner_one_place() {
    let mut e = editor_with(plane());
    uv_tool(&mut e, Tool::Rotate, |p| p.delta = DVec3::new(0., 0., 90.));
    e.dispatch(Command::Drop).unwrap();
    assert_row(
        &uvs(&e, "Texture")[0],
        &[(1., 0.), (1., 1.), (0., 1.), (0., 0.)],
    );
}

#[test]
fn transform_tool_offsets_turns_and_scales_in_uv() {
    let mut e = editor_with(plane());
    uv_tool(&mut e, Tool::Transform, |p| {
        p.delta = DVec3::new(1., 0., 0.);
        p.scale_delta = DVec3::new(0.5, 0.5, 1.);
    });
    e.dispatch(Command::Drop).unwrap();
    assert_row(
        &uvs(&e, "Texture")[0],
        &[(1.25, 0.25), (1.75, 0.25), (1.75, 0.75), (1.25, 0.75)],
    );
}

#[test]
fn uv_move_cancel_restores_and_undo_redo_is_one_step() {
    let mut e = editor_with(plane());
    let before = uvs(&e, "Texture");
    uv_tool(&mut e, Tool::Move, |p| p.delta = DVec3::new(0.25, 0., 0.));
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(uvs(&e, "Texture"), before, "cancel leaves the map");

    let steps = e.history.undo.len();
    uv_tool(&mut e, Tool::Move, |p| p.delta = DVec3::new(0.25, 0., 0.));
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    let moved = uvs(&e, "Texture");
    assert!(near(moved[0][0], DVec2::new(0.25, 0.)));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(uvs(&e, "Texture"), before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(uvs(&e, "Texture"), moved);
}

#[test]
fn uvt02_nothing_selected_moves_every_visible_uv_and_selection_limits() {
    let mut e = editor_with(strip(false));
    uv_tool(&mut e, Tool::Move, |p| p.delta = DVec3::new(0., 1., 0.));
    e.dispatch(Command::Drop).unwrap();
    assert!(uvs(&e, "Texture").iter().flatten().all(|p| p.y >= 1.));

    // Only point 0 selected: just its UV moves.
    let mut e = editor_with(strip(false));
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = vec![0];
    uv_tool(&mut e, Tool::Move, |p| p.delta = DVec3::new(0., 1., 0.));
    e.dispatch(Command::Drop).unwrap();
    let u = uvs(&e, "Texture");
    assert!(near(u[0][0], DVec2::new(0., 1.)));
    assert!(near(u[0][1], DVec2::new(0.5, 0.)));
}

#[test]
fn uvt42_shared_uvs_move_together_but_seams_move_apart() {
    // No seam: selecting the left quad drags the shared UV positions of
    // the right one with it.
    let mut e = editor_with(strip(false));
    e.scene.selection.faces = vec![0];
    uv_tool(&mut e, Tool::Move, |p| p.delta = DVec3::new(0., 0.25, 0.));
    e.dispatch(Command::Drop).unwrap();
    let u = uvs(&e, "Texture");
    assert!(
        near(u[1][0], DVec2::new(0.5, 0.25)),
        "shared position moved"
    );
    assert!(near(u[1][1], DVec2::new(1., 0.)), "the far side stays");

    // With a seam the right quad stays where it is.
    let mut e = editor_with(strip(true));
    e.scene.selection.faces = vec![0];
    uv_tool(&mut e, Tool::Move, |p| p.delta = DVec3::new(0., 0.25, 0.));
    e.dispatch(Command::Drop).unwrap();
    let u = uvs(&e, "Texture");
    assert!(near(u[1][0], DVec2::new(1., 0.)));
}

#[test]
fn uvt41_tear_off_moves_only_the_selected_polygons_corners() {
    let mut e = editor_with(strip(false));
    e.scene.selection.faces = vec![0];
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::UvToolSpace(Some(None))).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.uv.as_mut().unwrap().tear_off = true;
    p.delta = DVec3::new(0., 0.25, 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let u = uvs(&e, "Texture");
    assert!(near(u[0][1], DVec2::new(0.5, 0.25)));
    assert!(
        near(u[1][0], DVec2::new(0.5, 0.)),
        "the neighbour stays: torn off"
    );
}

#[test]
fn only_the_selected_map_changes_and_grid_snap_lands_on_tenths() {
    let mut m = plane();
    let other = m.uv_maps["Texture"].clone();
    m.uv_maps.insert("Other".into(), other.clone());
    let mut e = editor_with(m);
    e.dispatch(Command::UvMap(UvMapEdit::Select("Texture".into())))
        .unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::UvToolSpace(Some(None))).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.uv.as_mut().unwrap().grid_snap = Some(0.1);
    p.delta = DVec3::new(0.234, 0.0, 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    // The centre 0.5 lands on 0.7.
    assert!(near(uvs(&e, "Texture")[0][0], DVec2::new(0.2, 0.)));
    assert_eq!(mesh(&e).uv_maps["Other"], other);
}

#[test]
fn hidden_polygons_are_never_touched() {
    let mut e = editor_with(strip(true));
    e.scene.selection.faces = vec![1];
    e.dispatch(Command::EditElements {
        kind: operandi_core::ElementKind::Hide,
        op: operandi_core::ElementOp::Selected,
    })
    .unwrap();
    assert!(e.scene.active().unwrap().elements.hidden.faces.contains(&1));
    e.scene.selection.faces.clear();
    let before = uvs(&e, "Texture")[1].clone();
    uv_tool(&mut e, Tool::Move, |p| p.delta = DVec3::new(0.5, 0., 0.));
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(uvs(&e, "Texture")[1], before);
    assert!(near(uvs(&e, "Texture")[0][0], DVec2::new(0.5, 0.)));
}

#[test]
fn uv_space_needs_a_map_and_a_transform_tool() {
    let mut m = plane();
    m.uv_maps.clear();
    let mut e = editor_with(m);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert!(e.dispatch(Command::UvToolSpace(Some(None))).is_err());
    e.dispatch(Command::Cancel).unwrap();
    let mut e = editor_with(plane());
    e.dispatch(Command::Activate(Tool::Bevel)).ok();
    if e.session.live.is_some() {
        assert!(e.dispatch(Command::UvToolSpace(Some(None))).is_err());
    }
}

#[test]
fn uv_transform_survives_save_and_reopen() {
    let mut e = editor_with(plane());
    uv_tool(&mut e, Tool::Move, |p| p.delta = DVec3::new(0.5, 0.5, 0.));
    e.dispatch(Command::Drop).unwrap();
    let bytes = operandi_core::persistence::encode(&e.scene).unwrap();
    let back = operandi_core::persistence::decode(&bytes).unwrap();
    assert_eq!(
        back.active().unwrap().mesh.uv_maps,
        e.scene.active().unwrap().mesh.uv_maps
    );
}

// ---------------------------------------------------------------- E7

fn run(e: &mut Editor, op: IslandOp) {
    e.dispatch(Command::UvIsland(op)).unwrap();
}

fn with_row(rows: Vec<Vec<(f64, f64)>>) -> Mesh {
    let mut m = plane();
    m.uv_maps.insert(
        "Texture".into(),
        rows.into_iter()
            .map(|r| r.into_iter().map(|(u, v)| Some(DVec2::new(u, v))).collect())
            .collect(),
    );
    m
}

#[test]
fn uv03_fit_entire_keep_proportion() {
    let mut e = editor_with(with_row(vec![vec![
        (0.2, 0.3),
        (0.4, 0.3),
        (0.4, 0.9),
        (0.2, 0.9),
    ]]));
    run(
        &mut e,
        IslandOp {
            rule: IslandRule::Fit(FitOptions::default()),
            grouping: Grouping::Entire,
        },
    );
    let u = &uvs(&e, "Texture")[0];
    let (lo, hi) = u
        .iter()
        .fold((DVec2::MAX, DVec2::MIN), |(l, h), p| (l.min(*p), h.max(*p)));
    assert!(near(lo, DVec2::ZERO), "{lo}");
    assert!(
        (hi.y - 1.).abs() < 1e-9 && (hi.x - 1. / 3.).abs() < 1e-9,
        "{hi}"
    );
}

#[test]
fn fit_gaps_udim_stretch_and_per_island() {
    // Gaps 10 %, no proportion, tile 1002: fills U 1.1..1.9 and V 0.1..0.9.
    let mut e = editor_with(with_row(vec![vec![
        (0.2, 0.3),
        (0.4, 0.3),
        (0.4, 0.9),
        (0.2, 0.9),
    ]]));
    run(
        &mut e,
        IslandOp {
            rule: IslandRule::Fit(FitOptions {
                keep_proportion: false,
                gaps: 10.,
                udim: 1002,
            }),
            grouping: Grouping::Entire,
        },
    );
    assert_row(
        &uvs(&e, "Texture")[0],
        &[(1.1, 0.1), (1.9, 0.1), (1.9, 0.9), (1.1, 0.9)],
    );
    // Per island: both quads of a seamed strip fill the tile on their own.
    let mut e = editor_with(strip(true));
    run(
        &mut e,
        IslandOp {
            rule: IslandRule::Fit(FitOptions::default()),
            grouping: Grouping::Island,
        },
    );
    for row in uvs(&e, "Texture") {
        assert_row(&row, &[(0., 0.), (1., 0.), (1., 1.), (0., 1.)]);
    }
}

#[test]
fn uv04_mirror_u_about_half() {
    let mut e = editor_with(with_row(vec![vec![
        (0.1, 0.2),
        (0.3, 0.2),
        (0.3, 0.4),
        (0.1, 0.4),
    ]]));
    run(
        &mut e,
        IslandOp {
            rule: IslandRule::Mirror {
                axis: UvAxis::U,
                center: 0.5,
            },
            grouping: Grouping::Entire,
        },
    );
    assert!(near(uvs(&e, "Texture")[0][0], DVec2::new(0.9, 0.2)));
}

#[test]
fn uv05_align_down_flattens_to_the_lowest_v() {
    let mut e = editor_with(with_row(vec![vec![
        (0.0, 0.3),
        (0.5, 0.35),
        (0.6, 0.28),
        (0.1, 0.9),
    ]]));
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = vec![0, 1, 2];
    run(
        &mut e,
        IslandOp {
            rule: IslandRule::Align {
                axis: UvAxis::V,
                to: AlignTo::Low,
            },
            grouping: Grouping::Entire,
        },
    );
    let u = &uvs(&e, "Texture")[0];
    for p in &u[..3] {
        assert!((p.y - 0.28).abs() < 1e-12);
    }
    assert!((u[3].y - 0.9).abs() < 1e-12, "unselected stays");
    assert!((u[1].x - 0.5).abs() < 1e-12, "U unchanged");
}

#[test]
fn uvt_a04_flip_u_about_its_own_centre() {
    let mut e = editor_with(with_row(vec![vec![
        (0.2, 0.),
        (0.25, 0.),
        (0.4, 1.),
        (0.2, 1.),
    ]]));
    run(
        &mut e,
        IslandOp {
            rule: IslandRule::Flip { axis: UvAxis::U },
            grouping: Grouping::Entire,
        },
    );
    assert!((uvs(&e, "Texture")[0][1].x - 0.35).abs() < 1e-12);
}

#[test]
fn align_per_edge_row_flattens_each_row_on_its_own() {
    // A 2 x 1 strip without seams: rows are the bottom edge 0-1-2 and the
    // top edge 3-4-5, each with a small wobble.
    let mut m = strip(false);
    let rows = m.uv_maps.get_mut("Texture").unwrap();
    rows[0][1] = Some(DVec2::new(0.5, 0.05));
    rows[1][0] = Some(DVec2::new(0.5, 0.05));
    let mut e = editor_with(m);
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = vec![(0, 1), (1, 2), (3, 4), (4, 5)];
    run(
        &mut e,
        IslandOp {
            rule: IslandRule::Align {
                axis: UvAxis::V,
                to: AlignTo::Low,
            },
            grouping: Grouping::EdgeRow,
        },
    );
    let u = uvs(&e, "Texture");
    assert!((u[0][1].y).abs() < 1e-12, "bottom row at its low V");
    assert!((u[0][2].y - 0.5).abs() < 1e-12, "top row keeps its own V");
}

#[test]
fn island_run_is_one_undo_step_and_refuses_an_empty_target() {
    let mut e = editor_with(plane());
    let before = uvs(&e, "Texture");
    let steps = e.history.undo.len();
    run(
        &mut e,
        IslandOp {
            rule: IslandRule::Flip { axis: UvAxis::V },
            grouping: Grouping::Entire,
        },
    );
    assert_eq!(e.history.undo.len(), steps + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(uvs(&e, "Texture"), before);
    // A mesh with no map gets an empty "Texture" and has nothing to fit.
    let mut m = plane();
    m.uv_maps.clear();
    let mut e = editor_with(m);
    assert!(
        e.dispatch(Command::UvIsland(IslandOp {
            rule: IslandRule::Fit(FitOptions::default()),
            grouping: Grouping::Entire,
        }))
        .is_err()
    );
}

#[test]
fn preset_buttons_use_the_rule_table_and_remembered_values() {
    let mut e = editor_with(strip(true));
    // Shift+click Fit UVs: per island.
    e.dispatch(Command::UvIslandPreset {
        preset: "uv.fitUVs".into(),
        click: Click::Shift,
    })
    .unwrap();
    for row in uvs(&e, "Texture") {
        assert_row(&row, &[(0., 0.), (1., 0.), (1., 1.), (0., 1.)]);
    }
    // The Mirror button uses the remembered Center.
    e.uv_island_memory.mirror_center = 0.;
    e.dispatch(Command::UvIslandPreset {
        preset: "uv.mirrorU".into(),
        click: Click::Plain,
    })
    .unwrap();
    assert!(near(uvs(&e, "Texture")[0][1], DVec2::new(-1., 0.)));
    // Every E7 row in the table resolves.
    for row in operandi_core::tool_preset::factory()
        .iter()
        .filter(|r| r.actor.operation.as_deref() == Some("uv_island"))
    {
        let preset = row.actor.uv.unwrap();
        for click in [Click::Plain, Click::Shift, Click::Alt] {
            preset.op(click, &IslandMemory::default()).unwrap();
        }
    }
    assert!(
        e.dispatch(Command::UvIslandPreset {
            preset: "Flex".into(),
            click: Click::Plain,
        })
        .is_err()
    );
}

#[test]
fn island_commands_parse_from_the_command_line() {
    let mut e = editor_with(with_row(vec![vec![
        (0.2, 0.3),
        (0.4, 0.3),
        (0.4, 0.9),
        (0.2, 0.9),
    ]]));
    operandi_core::command_exec::run(
        &mut e,
        "operandi.uv.fit",
        &serde_json::json!({"keepProportion": false})
            .as_object()
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert_row(
        &uvs(&e, "Texture")[0],
        &[(0., 0.), (1., 0.), (1., 1.), (0., 1.)],
    );
    operandi_core::command_exec::run(
        &mut e,
        "operandi.uv.align",
        &serde_json::json!({"direction": "up"})
            .as_object()
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert!(
        uvs(&e, "Texture")[0]
            .iter()
            .all(|p| (p.y - 1.).abs() < 1e-12)
    );
    // Remembered: the next plain Fit keeps Keep Proportion off.
    assert!(!e.uv_island_memory.fit.keep_proportion);
}

// ---------------------------------------------------------------- E1

#[test]
fn uv06_new_map_is_selected_and_unwrap_writes_only_into_it() {
    let mut e = editor_with(plane());
    let texture = mesh(&e).uv_maps["Texture"].clone();
    e.dispatch(Command::UvMap(UvMapEdit::New(Some("Iris".into()))))
        .unwrap();
    assert_eq!(mesh(&e).uv_maps.len(), 2);
    assert_eq!(current_map(mesh(&e)).as_deref(), Some("Iris"));
    e.dispatch(Command::UvUnwrap(Default::default())).unwrap();
    assert_eq!(mesh(&e).uv_maps["Texture"], texture);
    assert!(mesh(&e).uv_maps["Iris"][0].iter().all(Option::is_some));
}

#[test]
fn uv07_copy_and_paste_a_whole_map() {
    let mut e = editor_with(plane());
    e.dispatch(Command::UvMap(UvMapEdit::Rename {
        from: "Texture".into(),
        to: "Vest".into(),
    }))
    .unwrap();
    e.dispatch(Command::UvMap(UvMapEdit::Copy("Vest".into())))
        .unwrap();
    e.dispatch(Command::UvMap(UvMapEdit::New(Some("Vest 2".into()))))
        .unwrap();
    e.dispatch(Command::UvMap(UvMapEdit::Paste("Vest 2".into())))
        .unwrap();
    assert_eq!(mesh(&e).uv_maps["Vest 2"], mesh(&e).uv_maps["Vest"]);
    let vest = mesh(&e).uv_maps["Vest"].clone();
    uv_tool(&mut e, Tool::Scale, |p| p.delta = DVec3::new(2., 2., 1.));
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(mesh(&e).uv_maps["Vest"], vest, "scaling Vest 2 leaves Vest");
    assert_ne!(mesh(&e).uv_maps["Vest 2"], vest);
}

#[test]
fn map_list_names_delete_several_and_undo() {
    let mut e = editor_with(plane());
    // Q1 (provisional): repeats of the default name count up.
    e.dispatch(Command::UvMap(UvMapEdit::New(None))).unwrap();
    e.dispatch(Command::UvMap(UvMapEdit::New(None))).unwrap();
    let names: Vec<String> = mesh(&e).uv_maps.keys().cloned().collect();
    assert_eq!(names, ["Texture", "Texture 2", "Texture 3"]);
    assert_eq!(current_map(mesh(&e)).as_deref(), Some("Texture 3"));
    assert!(
        e.dispatch(Command::UvMap(UvMapEdit::New(Some("Texture".into()))))
            .is_err()
    );
    assert!(
        e.dispatch(Command::UvMap(UvMapEdit::Rename {
            from: "Texture 2".into(),
            to: "Texture".into(),
        }))
        .is_err()
    );
    let steps = e.history.undo.len();
    e.dispatch(Command::UvMap(UvMapEdit::Delete(vec![
        "Texture 2".into(),
        "Texture 3".into(),
    ])))
    .unwrap();
    assert_eq!(e.history.undo.len(), steps + 1, "one step for several");
    assert_eq!(mesh(&e).uv_maps.len(), 1);
    assert_eq!(current_map(mesh(&e)).as_deref(), Some("Texture"));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e).uv_maps.len(), 3);
    // Copy is no undo step; pasting with nothing copied is refused.
    let mut fresh = editor_with(plane());
    assert!(
        fresh
            .dispatch(Command::UvMap(UvMapEdit::Paste("Texture".into())))
            .is_err()
    );
}

#[test]
fn selected_map_survives_save_and_reopen_and_vertmap_new_makes_uv_maps() {
    let mut e = editor_with(plane());
    operandi_core::command_exec::run(
        &mut e,
        "vertMap.new",
        &serde_json::json!({"name": "Iris", "type": "uv"})
            .as_object()
            .unwrap()
            .clone(),
    )
    .unwrap();
    operandi_core::command_exec::run(
        &mut e,
        "operandi.uvMap.select",
        &serde_json::json!({"name": "Texture"})
            .as_object()
            .unwrap()
            .clone(),
    )
    .unwrap();
    let bytes = operandi_core::persistence::encode(&e.scene).unwrap();
    let back = operandi_core::persistence::decode(&bytes).unwrap();
    let m = &back.active().unwrap().mesh;
    assert!(m.uv_maps.contains_key("Iris"));
    assert_eq!(current_map(m).as_deref(), Some("Texture"));
    assert_eq!(m.attributes.current_uv_map.as_deref(), Some("Texture"));
}
