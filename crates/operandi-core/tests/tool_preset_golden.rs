//! Tool presets stage 1 (specifications/tool-presets-2026-11-15, section 8):
//! every preset that moved onto the recipe table must give exactly what it
//! gave before the move. The golden file was written by this test on main
//! 75587ec3, before the table existed, with `OPERANDI_WRITE_PRESET_GOLDEN=1`.
//!
//! Each case: a user falloff and action centre are set, the preset is
//! started, dragged twice (preview, release) and dropped. The case records
//! the points after each drag, the falloff and action centre while the
//! preset is open and after it's dropped, the history labels and the live
//! tool's label. Original test data.
//!
//! One line was changed by hand afterwards, on purpose: Reset under Thicken
//! used to give Smooth Shift's values and now gives Thicken's (TP-15).
use glam::DVec3;
use operandi_core::*;
use std::fmt::Write as _;

/// A 0.5 × 2 × 0.5 column standing on the ground, four rows high.
fn column(mode: Mode, select: &[u32]) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh =
        primitive::segmented_cube(DVec3::new(0., 1., 0.), DVec3::new(0.5, 2., 0.5), [1, 4, 1])
            .unwrap()
            .into();
    e.scene.selection.mode = mode;
    match mode {
        Mode::Polygon => e.scene.selection.faces = select.to_vec(),
        _ => e.scene.selection.vertices = select.to_vec(),
    }
    e.history.undo.clear();
    e
}

fn live(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}

fn record(out: &mut String, e: &Editor, what: &str) {
    let _ = writeln!(out, "  {what}:");
    for p in &e.scene.active().unwrap().mesh.vertices {
        let _ = writeln!(out, "    {:.12} {:.12} {:.12}", p.x, p.y, p.z);
    }
}

fn case(preset: DeformPreset, mode: Mode, select: &[u32], user_falloff: bool) -> String {
    let mut e = column(mode, select);
    if user_falloff {
        let mine = RadialFalloff {
            center: DVec3::new(5., 0., 0.),
            size: DVec3::splat(0.25),
            shape: FalloffShape::EaseIn,
            world: false,
        };
        e.dispatch(Command::SpatialFalloff(Some(mine.into())))
            .unwrap();
    }
    e.action_context = ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    };
    let mut out = format!(
        "case {} mode {mode:?} select {select:?} user_falloff {user_falloff}\n",
        preset.label()
    );
    if let Err(err) = e.dispatch(Command::ActivateDeform(preset)) {
        let _ = writeln!(out, "  refused: {err}");
        return out;
    }
    let _ = writeln!(
        out,
        "  tool {:?} label {:?}",
        e.session.live.as_ref().map(|l| l.tool),
        e.live_tool_label()
    );
    let _ = writeln!(out, "  open falloff {:?}", e.spatial_falloff);
    let _ = writeln!(out, "  open action {:?}", e.action_context);
    for (i, k) in [1.0, 0.6].into_iter().enumerate() {
        if let Err(err) = e
            .dispatch(Command::Preview(Parameters {
                delta: DVec3::new(0.3, 0.7, 0.2) * k,
                rotation_delta: DVec3::new(10., 35., -5.) * k,
                scale_delta: DVec3::new(0.25, 0.5, 0.1) * k,
                ..live(&e)
            }))
            .and_then(|_| e.dispatch(Command::CommitTransformStep))
        {
            let _ = writeln!(out, "  drag {i} refused: {err}");
        }
        record(&mut out, &e, &format!("after drag {i}"));
    }
    e.dispatch(Command::Drop).unwrap();
    let _ = writeln!(
        out,
        "  dropped live {:?}",
        e.session.live.as_ref().map(|l| l.tool)
    );
    let _ = writeln!(out, "  dropped falloff {:?}", e.spatial_falloff);
    let _ = writeln!(out, "  dropped linear {:?}", e.linear_falloff_preset);
    let _ = writeln!(out, "  dropped radial {:?}", e.radial_falloff_preset);
    let _ = writeln!(out, "  dropped action {:?}", e.action_context);
    let _ = writeln!(
        out,
        "  history {:?}",
        e.history
            .undo
            .iter()
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>()
    );
    record(&mut out, &e, "dropped");
    out
}

/// Thicken: Smooth Shift with its own starting values (TP-15 Reset too).
fn thicken_case() -> String {
    let mut e = column(Mode::Polygon, &[1, 2]);
    e.dispatch(Command::Activate(Tool::Thicken)).unwrap();
    let mut out = format!("case Thicken\n  start {:?}\n", live(&e).smooth_shift);
    e.dispatch(Command::ResetSmoothShift).unwrap();
    let _ = writeln!(out, "  reset {:?}", live(&e).smooth_shift);
    out
}

fn all_cases() -> String {
    let mut out = thicken_case();
    for preset in DeformPreset::ALL {
        for (mode, select) in [
            (Mode::Vertex, &[][..]),
            (Mode::Vertex, &[4, 5, 6, 7, 12, 13, 14, 15][..]),
            (Mode::Polygon, &[1, 2][..]),
            (Mode::Item, &[][..]),
        ] {
            for user_falloff in [false, true] {
                out.push_str(&case(preset, mode, select, user_falloff));
            }
        }
    }
    out
}

#[test]
fn migrated_presets_match_main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tool_presets/deform_golden.txt");
    let got = all_cases();
    if std::env::var_os("OPERANDI_WRITE_PRESET_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap();
    if got != want {
        let first = got
            .lines()
            .zip(want.lines())
            .position(|(a, b)| a != b)
            .unwrap_or(0);
        let context: Vec<_> = want.lines().skip(first.saturating_sub(3)).take(6).collect();
        let now: Vec<_> = got.lines().skip(first.saturating_sub(3)).take(6).collect();
        panic!(
            "preset results changed at line {}:\nwas:\n{}\nnow:\n{}",
            first + 1,
            context.join("\n"),
            now.join("\n")
        );
    }
}
