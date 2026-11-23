//! Rig phase 4 §8/§9 in USD: the robot arm hand-off (a part dynamically
//! parented to World, the gripper at 30, the conveyor at 60) exports its
//! evaluated motion as time samples, rigged and baked alike, and OpenUSD
//! itself (Blender's Python, when installed) reads the same positions.
use glam::DVec3;
use operandi_core::interchange::ExportFormat;
use operandi_core::*;
use std::path::PathBuf;

fn ok(e: &mut Editor, text: &str) {
    if let Some(Err(err)) = command_line::run_line(e, text) {
        panic!("{text}: {err}");
    }
}

/// A unit-cube item called `name` at `at`.
fn cube(e: &mut Editor, name: &str, at: DVec3) -> u64 {
    e.scene.new_mesh().unwrap();
    let id = e.scene.items.last().unwrap().id;
    e.scene.item_mut(id).unwrap().name = name.into();
    let h = 0.25;
    let p: Vec<[f64; 3]> = (0..8)
        .map(|i| {
            [
                if i & 1 == 0 { -h } else { h },
                if i & 2 == 0 { -h } else { h },
                if i & 4 == 0 { -h } else { h },
            ]
        })
        .collect();
    let f = vec![
        vec![0, 2, 3, 1],
        vec![4, 5, 7, 6],
        vec![0, 1, 5, 4],
        vec![2, 6, 7, 3],
        vec![1, 3, 7, 5],
        vec![0, 4, 6, 2],
    ];
    let mesh = Mesh {
        vertices: p.into_iter().map(DVec3::from_array).collect(),
        faces: Shared::new(
            f.into_iter()
                .map(|vertices| Face {
                    vertices,
                    uv: None,
                    material: "Default".into(),
                    smooth: true,
                    part: None,
                    material_label: None,
                })
                .collect(),
        ),
        ..Default::default()
    };
    e.scene.item_mut(id).unwrap().mesh = MeshSlot::from(mesh);
    e.scene.item_mut(id).unwrap().transform.translation = at;
    id
}

fn hand_off() -> (Editor, u64) {
    let mut e = Editor::default();
    let part = cube(&mut e, "Part", DVec3::new(1., 0., 0.));
    let gripper = cube(&mut e, "Gripper", DVec3::new(1., 2., 0.));
    let conveyor = cube(&mut e, "Conveyor", DVec3::new(0., -1., 0.));
    ok(
        &mut e,
        &format!("operandi.key.add channel:{gripper}:pos.X frame:30 value:1"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{gripper}:pos.X frame:60 value:5"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{conveyor}:pos.Z frame:60 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{conveyor}:pos.Z frame:90 value:6"),
    );
    ok(
        &mut e,
        &format!(
            "operandi.rig.dynamicParentAdd item:{part} parents:{gripper},{conveyor} compensate:true"
        ),
    );
    for (f, parent) in [(0, 0), (30, 1), (60, 2)] {
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
        ok(
            &mut e,
            &format!(
                "operandi.rig.dynamicParentSet item:{part} parent:{parent} compensate:true key:true"
            ),
        );
    }
    ok(&mut e, "operandi.time.settings end:90");
    ok(&mut e, "operandi.time.set frame:0");
    (e, part)
}

fn out_dir() -> PathBuf {
    let dir = std::env::var_os("OPERANDI_RIG_CHECK")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("operandi-rig-usd-{}", std::process::id()))
        });
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The part's evaluated position at each frame.
fn expected(e: &mut Editor, part: u64, frames: &[i64]) -> Vec<DVec3> {
    frames
        .iter()
        .map(|f| {
            ok(e, &format!("operandi.time.set frame:{f}"));
            e.evaluated(evaluate::Purpose::Final)
                .world_matrix(part)
                .unwrap()
                .w_axis
                .truncate()
        })
        .collect()
}

/// OpenUSD's own reading of the part's position at `frames`; `None`
/// without Blender's Python.
fn read_back(path: &std::path::Path, frames: &[i64]) -> Option<Vec<DVec3>> {
    let python =
        PathBuf::from("/Applications/Blender.app/Contents/Resources/5.2/python/bin/python3.13");
    if !python.is_file() {
        eprintln!("no Blender Python here; read-back skipped");
        return None;
    }
    let script = out_dir().join("read_part.py");
    std::fs::write(
        &script,
        r#"import sys
from pxr import Usd, UsdGeom
stage = Usd.Stage.Open(sys.argv[1])
part = next(p for p in stage.Traverse() if p.GetName() == "Part")
x = UsdGeom.Xformable(part)
for f in sys.argv[2:]:
    t = x.ComputeLocalToWorldTransform(Usd.TimeCode(float(f))).ExtractTranslation()
    print(t[0], t[1], t[2])
"#,
    )
    .unwrap();
    let out = std::process::Command::new(&python)
        .arg("-I")
        .arg(&script)
        .arg(path)
        .args(frames.iter().map(|f| f.to_string()))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| {
                let v: Vec<f64> = l.split_whitespace().map(|w| w.parse().unwrap()).collect();
                DVec3::new(v[0], v[1], v[2])
            })
            .collect(),
    )
}

const FRAMES: [i64; 8] = [0, 15, 29, 30, 45, 60, 75, 90];

#[test]
fn milestone4_the_hand_off_exports_its_evaluated_motion_rigged_and_baked() {
    let (mut e, part) = hand_off();
    let want = expected(&mut e, part, &FRAMES);
    ok(&mut e, "operandi.time.set frame:0");
    let dir = out_dir();
    // Rigged: the pieces aren't written; the evaluated transforms are.
    let rigged = dir.join("hand-off.usda");
    let scene = e.evaluated(evaluate::Purpose::Final);
    crate::export(&rigged, &scene, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&rigged).unwrap();
    assert!(text.contains("xformOp:translate.timeSamples"), "{text}");
    assert!(text.contains("endTimeCode = 90"));
    // Baked: plain keys, the same motion.
    ok(
        &mut e,
        &format!("operandi.rig.bake items:{part} start:0 end:90"),
    );
    ok(&mut e, "operandi.time.set frame:0");
    let baked = dir.join("hand-off-baked.usd");
    let scene = e.evaluated(evaluate::Purpose::Final);
    crate::export(&baked, &scene, ExportFormat::Usd).unwrap();
    let rigged_usd = dir.join("hand-off.usd");
    let (mut e2, _) = hand_off();
    ok(&mut e2, "operandi.time.set frame:0");
    let scene = e2.evaluated(evaluate::Purpose::Final);
    crate::export(&rigged_usd, &scene, ExportFormat::Usd).unwrap();
    for path in [&rigged_usd, &baked] {
        let Some(got) = read_back(path, &FRAMES) else {
            return;
        };
        for ((f, a), b) in FRAMES.iter().zip(&want).zip(&got) {
            assert!(
                (*a - *b).length() < 1e-4,
                "{} frame {f}: {a} vs {b}",
                path.display()
            );
        }
    }
}
