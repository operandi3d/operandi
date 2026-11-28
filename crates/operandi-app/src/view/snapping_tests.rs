//! Snapping (specifications/snapping-2026-11-05): oracle landings, typed
//! values, grid, the hold key and history.
#![cfg_attr(not(operandi_specs), allow(dead_code, unused_imports))]
use super::*;
use crate::snapping::{SnappingPreferences, Targets};
use operandi_core::{Face, Mesh};
use serde_json::Value;

const VERTEX: Targets = Targets {
    vertex: true,
    grid: false,
    ..Targets::NONE
};
const GRID: Targets = Targets {
    vertex: false,
    grid: true,
    ..Targets::NONE
};
const BOTH: Targets = Targets {
    vertex: true,
    grid: true,
    ..Targets::NONE
};

#[cfg(operandi_specs)]
fn oracles() -> BTreeMap<String, Value> {
    let path = std::path::Path::new(env!("OPERANDI_SPECS_DIR"))
        .join("specifications/snapping-2026-11-05/ORACLES-SNAPPING.json");
    let all: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    all["oracles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| (o["id"].as_str().unwrap().to_owned(), o.clone()))
        .collect()
}

fn v3(v: &Value) -> DVec3 {
    DVec3::new(
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    )
}

/// The two quads of the oracles, Front view at 288.3 px per metre, Vertex
/// mode, `vertex` selected, Move active with the handle at the selection.
#[cfg(operandi_specs)]
fn two_quads(vertex: u32, targets: Targets) -> (Operandi, Rect) {
    let o = oracles();
    let input = &o["SNP-O-SN-03"]["input"];
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.editor.scene.active_mut().unwrap().mesh = Mesh {
        vertices: input["points"].as_array().unwrap().iter().map(v3).collect(),
        faces: input["polygons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| Face {
                vertices: p
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect(),
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    }
    .into();
    let rect = Rect::from_min_size(pos2(0., 0.), vec2(1600., 900.));
    app.camera.view = View::Front;
    app.camera.target = DVec3::new(2., 0.5, 0.);
    app.camera.distance = rect.height() as f64 * app.camera.lens / 288.3;
    app.run(Command::Mode(Mode::Vertex));
    app.editor.scene.selection.vertices = vec![vertex];
    app.run(Command::ActionContext(ActionContext {
        center: CenterMode::Selection,
        ..app.editor.action_context
    }));
    app.snapping = SnappingPreferences {
        enabled: true,
        global: Targets::default(),
        component: targets,
        ..Default::default()
    };
    app.activate(Tool::Move);
    (app, rect)
}

fn points(app: &Operandi) -> Vec<DVec3> {
    app.editor.scene.active().unwrap().mesh.vertices.clone()
}

fn screen(app: &Operandi, p: DVec3, rect: Rect) -> Pos2 {
    app.camera.project(p, rect).unwrap().0
}

/// Drag the Move center handle from where it is to `to`.
fn drag_to(app: &mut Operandi, to: Pos2, rect: Rect) {
    let center = app
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .action_frame
        .unwrap()
        .center;
    let start = screen(app, center, rect);
    let base = app.editor.session.live.as_ref().unwrap().params.clone();
    app.update_handle(viewport::FREE_HANDLE, base, start, to, Tool::Move, rect);
}

#[test]
#[cfg(operandi_specs)]
fn snp_o_sn_03_vertex_landings_3_and_15_px_short_are_exact() {
    let o = oracles();
    for (id, short) in [("SNP-O-SN-03", 3.), ("SNP-O-SN-03-15px", 15.)] {
        let (mut app, rect) = two_quads(2, VERTEX);
        let target = screen(&app, DVec3::new(3., 1., 0.), rect);
        drag_to(&mut app, target - vec2(short, 0.), rect);
        assert_eq!(
            app.snap_marker,
            Some((DVec3::new(3., 1., 0.), true)),
            "{id}"
        );
        app.run(Command::Drop);
        let expected = &o[id]["expected"];
        let want: Vec<DVec3> = expected["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(v3)
            .collect();
        let got = points(&app);
        assert_eq!(got.len(), want.len());
        for (g, w) in got.iter().zip(&want) {
            assert!((*g - *w).abs().max_element() < 1e-6, "{id}: {g} vs {w}");
        }
        // SN-10: no weld, same polygons, selection kept.
        assert_eq!(app.editor.scene.active().unwrap().mesh.faces.len(), 2);
        assert_eq!(app.editor.scene.selection.vertices, vec![2]);
    }
}

#[test]
#[cfg(operandi_specs)]
fn outside_inner_range_only_highlights_and_off_does_nothing() {
    // 30 px short: inside Outer (40) but not Inner (24).
    let (mut app, rect) = two_quads(2, VERTEX);
    let target = screen(&app, DVec3::new(3., 1., 0.), rect);
    drag_to(&mut app, target - vec2(30., 0.), rect);
    assert_eq!(app.snap_marker, Some((DVec3::new(3., 1., 0.), false)));
    let x = points(&app)[2].x;
    assert!((x - (3. - 30. / 288.3)).abs() < 1e-6, "{x}");
    // Snapping off: 3 px short stays 3 px short.
    let (mut app, rect) = two_quads(2, VERTEX);
    app.snapping.enabled = false;
    drag_to(&mut app, target - vec2(3., 0.), rect);
    assert_eq!(app.snap_marker, None);
    assert!((points(&app)[2].x - (3. - 3. / 288.3)).abs() < 1e-6);
}

#[test]
#[cfg(operandi_specs)]
fn the_moving_point_is_not_its_own_target() {
    let (mut app, rect) = two_quads(2, VERTEX);
    let start = screen(&app, DVec3::new(1., 1., 0.), rect);
    drag_to(&mut app, start + vec2(5., 0.), rect);
    assert!((points(&app)[2].x - (1. + 5. / 288.3)).abs() < 1e-6);
}

#[test]
#[cfg(operandi_specs)]
fn snp_o_01_02_typed_moves_are_never_snapped() {
    let o = oracles();
    let (mut app, _) = two_quads(2, BOTH);
    app.snapping.global = BOTH;
    for (key, value) in [
        ("delta0", "1.98 m"),
        ("delta1", "0.01 m"),
        ("delta2", "0 m"),
    ] {
        app.fields.insert(key.into(), value.into());
    }
    assert!(app.confirm_fields(), "{}", app.status);
    app.run(Command::Drop);
    let want = v3(&o["SNP-O-01"]["expected"]["point2"]);
    assert!((points(&app)[2] - want).abs().max_element() < 1e-5);
    // SNP-O-02 continues from that result.
    app.editor.scene.selection.vertices = vec![1];
    app.activate(Tool::Move);
    for (key, value) in [
        ("delta0", "0.33 m"),
        ("delta1", "0.27 m"),
        ("delta2", "0 m"),
    ] {
        app.fields.insert(key.into(), value.into());
    }
    assert!(app.confirm_fields(), "{}", app.status);
    app.run(Command::Drop);
    let want = v3(&o["SNP-O-02"]["expected"]["point1"]);
    assert!((points(&app)[1] - want).abs().max_element() < 1e-5);
}

#[test]
#[cfg(operandi_specs)]
fn grid_target_lands_on_a_crossing_and_keeps_depth() {
    let (mut app, rect) = two_quads(2, GRID);
    let plane = app.actual_plane();
    let step = grid::spacing(&app.camera, rect, plane, app.editor.scene.workplane.center);
    let start = screen(&app, DVec3::new(1., 1., 0.), rect);
    drag_to(&mut app, start + vec2(37., -11.), rect);
    let p = points(&app)[2];
    // Front view: the XY plane; X and Y on the grid, Z unchanged.
    assert_eq!(plane, 2);
    for c in [p.x, p.y] {
        assert!(
            ((c / step).round() * step - c).abs() < 1e-9,
            "{p} step {step}"
        );
    }
    assert_eq!(p.z, 0.);
    assert_ne!(p, DVec3::new(1., 1., 0.));
}

#[test]
#[cfg(operandi_specs)]
fn a_near_vertex_beats_a_farther_grid_crossing() {
    let (mut app, rect) = two_quads(2, BOTH);
    let target = screen(&app, DVec3::new(3., 1., 0.), rect);
    drag_to(&mut app, target - vec2(3., 0.), rect);
    assert!((points(&app)[2] - DVec3::new(3., 1., 0.)).length() < 1e-9);
}

#[test]
#[cfg(operandi_specs)]
fn x_taps_toggle_and_holding_through_a_drag_flips_back() {
    let ctx = Context::default();
    let idle = |app: &mut Operandi| {
        ctx.begin_pass(egui::RawInput::default());
        app.update_snap_hold(&ctx);
        let _ = ctx.end_pass();
    };
    let (mut app, rect) = two_quads(2, VERTEX);
    app.snapping.enabled = false;
    let history = app.editor.history.undo.len();
    // Tap: on, and stays on when the key is up.
    app.execute("snapping toggle");
    assert!(app.snapping.enabled);
    idle(&mut app);
    assert!(app.snapping.enabled && app.snap_hold.is_none());
    // SN-14: toggling is not an undo step.
    assert_eq!(app.editor.history.undo.len(), history);
    // Hold through a drag: off for the drag, back on at release.
    app.execute("snapping toggle");
    assert!(!app.snapping.enabled);
    let target = screen(&app, DVec3::new(3., 1., 0.), rect);
    drag_to(&mut app, target - vec2(3., 0.), rect);
    assert!(
        (points(&app)[2].x - (3. - 3. / 288.3)).abs() < 1e-6,
        "not snapped while held"
    );
    idle(&mut app);
    assert!(app.snapping.enabled);
}

#[test]
fn snapping_menu_and_options_are_wired() {
    let mut app = Operandi::empty();
    assert!(!app.snapping.enabled);
    app.execute("snapping options");
    assert!(app.snapping_options.is_some());
    assert_eq!(
        app.bindings.get("Snapping Toggle").map(String::as_str),
        Some("X")
    );
    assert_eq!(
        app.bindings.get("Snapping Options").map(String::as_str),
        Some("F11")
    );
}

#[test]
#[cfg(operandi_specs)]
fn tweak_grab_point_snaps_to_a_vertex() {
    // SN-12: Tweak's grabbed point snaps like a handle.
    let (mut app, rect) = two_quads(2, VERTEX);
    app.run(Command::Drop);
    app.activate(Tool::Tweak);
    let item = app.editor.scene.active().unwrap().id;
    let pick = viewport::tweak_view::TweakPick {
        grab: operandi_core::TweakGrab {
            item,
            element: operandi_core::TweakElement::Vertex(2),
        },
        at: DVec3::new(1., 1., 0.),
    };
    let base = app.editor.session.live.as_ref().unwrap().params.clone();
    let target = screen(&app, DVec3::new(3., 1., 0.), rect);
    let press = screen(&app, DVec3::new(1., 1., 0.), rect);
    app.tweak_drag(pick, &base, press, target - vec2(10., 4.), rect);
    assert!(
        (points(&app)[2] - DVec3::new(3., 1., 0.)).length() < 1e-9,
        "{} {}",
        points(&app)[2],
        app.status
    );
    // A small drag isn't held back by the grabbed point itself.
    let start = screen(&app, DVec3::new(1., 1., 0.), rect);
    app.tweak_drag(pick, &base, start, start + vec2(5., 0.), rect);
    assert!((points(&app)[2].x - (1. + 5. / 288.3)).abs() < 1e-6);
}

#[test]
#[cfg(operandi_specs)]
fn vertex_only_never_snaps_to_the_grid() {
    // Jason's QA: an unticked target must not snap. Far from every vertex,
    // a drag lands exactly where the pointer puts it.
    let (mut app, rect) = two_quads(2, VERTEX);
    app.snapping.global = VERTEX;
    let start = screen(&app, DVec3::new(1., 1., 0.), rect);
    drag_to(&mut app, start + vec2(137., -61.), rect);
    let p = points(&app)[2];
    assert!((p.x - (1. + 137. / 288.3)).abs() < 1e-6, "{p}");
    assert!((p.y - (1. + 61. / 288.3)).abs() < 1e-6, "{p}");
    assert_eq!(app.snap_marker, None);
}

#[test]
fn snapping_options_popover_draws_the_built_targets() {
    let mut app = Operandi::empty();
    app.execute("snapping options");
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..2 {
        ctx.begin_pass(egui::RawInput::default());
        app.snapping_window(&ctx);
        let out = ctx.end_pass();
        let _ = out;
    }
    // Only Grid and Vertex are built; the rest are left out (UIC-117).
    assert!(app.snapping_options.is_some());
}

#[test]
#[cfg(operandi_specs)]
fn points_hidden_behind_a_surface_are_not_targets() {
    // Jason's QA: a point on the far side of the geometry, overlapping on
    // screen, must not win. A quad between the camera and (3, 1, 0) hides it.
    let (mut app, rect) = two_quads(2, VERTEX);
    app.run(Command::Drop);
    let eye = app.camera.basis().0;
    let z = 0.5 * eye.z.signum();
    {
        let mesh: &mut operandi_core::Mesh = &mut app.editor.scene.active_mut().unwrap().mesh;
        let base = mesh.vertices.len() as u32;
        for (x, y) in [(2.5, 0.5), (3.5, 0.5), (3.5, 1.5), (2.5, 1.5)] {
            mesh.vertices.push(DVec3::new(x, y, z));
        }
        mesh.faces.push(Face {
            vertices: vec![base, base + 1, base + 2, base + 3],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
    }
    app.activate(Tool::Move);
    let target = screen(&app, DVec3::new(3., 1., 0.), rect);
    drag_to(&mut app, target - vec2(3., 0.), rect);
    let p = points(&app)[2];
    assert!(
        (p.x - (3. - 3. / 288.3)).abs() < 1e-6,
        "snapped through: {p}"
    );
}

#[test]
#[cfg(operandi_specs)]
fn an_axis_arrow_drag_takes_the_targets_position_along_its_axis() {
    // SN2-30b: drag point 2 (1, 1, 0) by the X arrow with the pointer on
    // the point (4, 0, 0): it lands at x 4 and keeps its own y.
    let (mut app, rect) = two_quads(2, VERTEX);
    let center = app
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .action_frame
        .unwrap()
        .center;
    let start = screen(&app, center, rect);
    let base = app.editor.session.live.as_ref().unwrap().params.clone();
    let target = screen(&app, DVec3::new(4., 0., 0.), rect);
    app.update_handle(0, base, start, target + vec2(2., 0.), Tool::Move, rect);
    let p = points(&app)[2];
    assert!((p - DVec3::new(4., 1., 0.)).length() < 1e-6, "{p}");
    // SN2-43b: the reticle sat where the point would be unsnapped.
    assert!(app.snap_reticle.is_some());
}

/// N06: Option+click on the Snapping button opens the popover below the
/// button; that same click must not count as a click outside and close it
/// again in the frame it opens. A later click outside still closes it.
#[test]
fn n06_the_opening_click_does_not_close_the_popover() {
    use crate::snapping::{HeaderAction, PopoverState, show_popover};
    let ctx = Context::default();
    let click = |at: Pos2| egui::RawInput {
        events: vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::ALT,
            },
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::ALT,
            },
        ],
        ..Default::default()
    };
    let button = pos2(400., 20.);
    let run = |opening: bool| {
        let state = PopoverState {
            at: pos2(380., 44.),
            pinned: false,
            entered: false,
            opening,
        };
        let mut out = None;
        let _ = ctx.run(click(button), |ctx| {
            out = Some(show_popover(ctx, "n06", state, pos2(0., 0.), true, |ui| {
                ui.label("rows");
                HeaderAction::None
            }));
        });
        out.unwrap()
    };
    assert!(run(true).is_some(), "the opening click keeps it open");
    assert!(run(false).is_none(), "a later click outside closes it");
}

/// A quad whose edge 0-1 is slanted 30 degrees in the Front view, and a
/// second quad whose corner (3, 1.2, 0) is a vertex target. Edge mode with
/// that edge selected, a real Local action center, Move active.
fn slanted_edge(targets: Targets) -> (Operandi, Rect) {
    let (c, s) = (30_f64.to_radians().cos(), 30_f64.to_radians().sin());
    let quad = |v: [u32; 4]| Face {
        vertices: v.to_vec(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    };
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.editor.scene.active_mut().unwrap().mesh = Mesh {
        vertices: vec![
            DVec3::ZERO,
            DVec3::new(c, s, 0.),
            DVec3::new(c, s + 1., 0.),
            DVec3::new(0., 1., 0.),
            DVec3::new(3., 1.2, 0.),
            DVec3::new(4., 1.2, 0.),
            DVec3::new(4., 2.2, 0.),
            DVec3::new(3., 2.2, 0.),
        ],
        faces: vec![quad([0, 1, 2, 3]), quad([4, 5, 6, 7])].into(),
        ..Default::default()
    }
    .into();
    let rect = Rect::from_min_size(pos2(0., 0.), vec2(1600., 900.));
    app.camera.view = View::Front;
    app.camera.target = DVec3::new(2., 1., 0.);
    app.camera.distance = rect.height() as f64 * app.camera.lens / 288.3;
    app.run(Command::Mode(Mode::Edge));
    app.editor.scene.selection.edges = vec![(0, 1)];
    app.run(Command::ActionContext(ActionContext::LOCAL));
    app.snapping = SnappingPreferences {
        enabled: true,
        global: Targets::default(),
        component: targets,
        ..Default::default()
    };
    app.activate(Tool::Move);
    (app, rect)
}

/// Drag the slanted edge's X arrow (tilted along the edge by the Local
/// action center) with the pointer on `to`; the handle's new position.
fn drag_tilted_arrow(app: &mut Operandi, to: DVec3, rect: Rect) -> DVec3 {
    let start = screen(app, app.transform_handle_center(), rect);
    let base = app.editor.session.live.as_ref().unwrap().params.clone();
    let target = screen(app, to, rect);
    app.update_handle(0, base, start, target, Tool::Move, rect);
    app.transform_handle_center()
}

/// Jason's QA report (2026-11-22): on a Local action center the arrow
/// follows the edge, and a vertex target pulled the edge past the target
/// along it. The handle must stop at the target's nearest point on the
/// arrow's line, and the edge must travel exactly that far.
#[test]
fn a_tilted_local_arrow_lands_on_the_vertex_without_overshoot() {
    let (mut app, rect) = slanted_edge(VERTEX);
    let start = app.transform_handle_center();
    let axis = app.handle_axis(0);
    // The arrow really is tilted along the edge (not a world axis).
    let edge = DVec3::new(30_f64.to_radians().cos(), 0.5, 0.);
    assert!(axis.dot(edge).abs() > 1. - 1e-9, "arrow {axis}");
    let vertex = DVec3::new(3., 1.2, 0.);
    let want = start + axis * (vertex - start).dot(axis);
    let landed = drag_tilted_arrow(&mut app, vertex, rect);
    assert!((landed - want).length() < 1e-6, "{landed} want {want}");
    let moved = points(&app)[0];
    assert!(
        (moved - axis * (vertex - start).dot(axis)).length() < 1e-6,
        "{moved}"
    );
}
