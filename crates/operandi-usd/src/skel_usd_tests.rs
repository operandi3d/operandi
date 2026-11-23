//! UsdSkel export (BEHAVIOR-RIG-PHASE5 §7): a bound arm writes a
//! Skeleton, SkelAnimation and binding, and OpenUSD's own skinning (from
//! Blender's bundled Python, when installed) gives the same points as
//! Operandi's skin at every checked frame.
use operandi_core::interchange::ExportFormat;
use operandi_core::*;
use std::path::PathBuf;

fn ok(e: &mut Editor, text: &str) {
    if let Some(Err(err)) = command_line::run_line(e, text) {
        panic!("{text}: {err}");
    }
}

fn last(e: &Editor) -> u64 {
    e.scene.items.last().unwrap().id
}

/// An arm along X bound with Heat, its elbow keyed from 0 to 90° about
/// Z over frames 0..24 (dual quaternion when `dq`).
pub(crate) fn arm(dq: bool) -> (Editor, u64) {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let mesh = last(&e);
    e.scene.item_mut(mesh).unwrap().name = "Arm".into();
    e.scene.item_mut(mesh).unwrap().mesh = primitive::segmented_cube(
        glam::DVec3::new(0.25, 0., 0.),
        glam::DVec3::new(1.5, 0.2, 0.2),
        [30, 2, 2],
    )
    .unwrap()
    .into();
    ok(
        &mut e,
        "operandi.skel.add position:-0.4,0,0 name:Shoulder preserveAlignment:true",
    );
    let a = last(&e);
    ok(
        &mut e,
        &format!(
            "operandi.skel.add position:0.25,0,0 parent:{a} name:Elbow preserveAlignment:true"
        ),
    );
    let b = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0.9,0,0 parent:{b} name:Wrist preserveAlignment:true"),
    );
    ok(&mut e, "operandi.anim.setup on:true");
    ok(
        &mut e,
        &format!("operandi.skin.bind items:{mesh} root:{a} dualQuaternion:{dq}"),
    );
    ok(&mut e, "operandi.anim.setup on:false");
    ok(&mut e, "operandi.time.set frame:0");
    ok(&mut e, &format!("operandi.key.set channels:{b}:rot.Z"));
    ok(&mut e, "operandi.time.set frame:24");
    ok(
        &mut e,
        &format!(
            "transform.channel name:rot.Z value:{} item:{b}",
            std::f64::consts::FRAC_PI_2
        ),
    );
    ok(&mut e, &format!("operandi.key.set channels:{b}:rot.Z"));
    e.scene.time.start = e.scene.time.rate.tick_of(0);
    e.scene.time.end = e.scene.time.rate.tick_of(24);
    (e, mesh)
}

fn out_dir() -> PathBuf {
    let dir = std::env::var_os("OPERANDI_SKEL_CHECK")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("operandi-skel-usd-{}", std::process::id()))
        });
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_bound_arm_writes_usdskel() {
    let (e, _) = arm(true);
    let path = out_dir().join("arm.usda");
    let scene = e.evaluated(evaluate::Purpose::Final);
    crate::export(&path, &scene, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    for want in [
        "def SkelRoot \"root\"",
        "def Skeleton \"Shoulder1_Skeleton\"",
        "\"Shoulder1/Elbow1/Wrist1\"",
        "def SkelAnimation \"Animation\"",
        "primvars:skel:jointWeights",
        "primvars:skel:jointIndices",
        "uniform token primvars:skel:skinningMethod = \"dualQuaternion\"",
        "rel skel:skeleton",
        "rel skel:animationSource",
    ] {
        assert!(text.contains(want), "{want} missing");
    }
    // Joint items aren't Xforms of their own.
    assert!(!text.contains("def Xform \"Shoulder1\""));
}

/// OpenUSD skins the exported arm the way Operandi does (§8 check 3's
/// export half; Blender's own Python, skipped where it isn't installed).
#[test]
fn openusd_skins_the_arm_like_operandi() {
    let python =
        PathBuf::from("/Applications/Blender.app/Contents/Resources/5.2/python/bin/python3.13");
    for dq in [false, true] {
        let (mut e, mesh) = arm(dq);
        let dir = out_dir();
        let path = dir.join(format!("arm-{dq}.usd"));
        crate::export(
            &path,
            &e.evaluated(evaluate::Purpose::Final),
            ExportFormat::Usd,
        )
        .unwrap();
        if !python.is_file() {
            eprintln!("no Blender Python here; OpenUSD skinning check skipped");
            return;
        }
        let script = dir.join("skin_arm.py");
        std::fs::write(
            &script,
            r#"import sys
from pxr import Usd, UsdGeom, UsdSkel, Gf
stage = Usd.Stage.Open(sys.argv[1])
cache = UsdSkel.Cache()
root = UsdSkel.Root(stage.GetPrimAtPath("/root"))
cache.Populate(root, Usd.PrimDefaultPredicate)
for binding in cache.ComputeSkelBindings(root, Usd.PrimDefaultPredicate):
    skel = cache.GetSkelQuery(binding.GetSkeleton())
    for q in binding.GetSkinningTargets():
        mesh = UsdGeom.Mesh(q.GetPrim())
        for f in (0, 12, 24):
            t = Usd.TimeCode(f)
            xf = skel.ComputeSkinningTransforms(t)
            pts = mesh.GetPointsAttr().Get(t)
            q.ComputeSkinnedPoints(xf, pts, t)
            # Skinned points are in skeleton space; bring them into the
            # mesh's own space.
            to_mesh = mesh.ComputeLocalToWorldTransform(t).GetInverse()
            for i in range(0, len(pts), 37):
                p = to_mesh.Transform(Gf.Vec3d(pts[i]))
                print(f, i, p[0], p[1], p[2])
"#,
        )
        .unwrap();
        let out = std::process::Command::new(&python)
            .arg("-I")
            .arg(&script)
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let mut seen = 0;
        for line in text.lines() {
            let v: Vec<f64> = line
                .split_whitespace()
                .map(|w| w.parse().unwrap())
                .collect();
            ok(&mut e, &format!("operandi.time.set frame:{}", v[0]));
            let s = e.evaluated(evaluate::Purpose::Final);
            let ours = s.item(mesh).unwrap().mesh.open().vertices[v[1] as usize];
            let theirs = glam::DVec3::new(v[2], v[3], v[4]);
            assert!(
                ours.distance(theirs) < 1e-4,
                "dq {dq} frame {} point {}: {ours} vs {theirs}",
                v[0],
                v[1]
            );
            seen += 1;
        }
        assert!(seen > 6, "{text}");
    }
}

/// §7 import: the exported arm reads back as the same joints, weights and
/// skin, and deforms the same at frames 0, 12 and 24.
#[test]
fn the_arm_reads_back_the_same() {
    for dq in [false, true] {
        let (mut e, mesh) = arm(dq);
        let path = out_dir().join(format!("arm-back-{dq}.usd"));
        crate::export(
            &path,
            &e.evaluated(evaluate::Purpose::Final),
            ExportFormat::Usd,
        )
        .unwrap();
        let back = crate::import(&path, &Default::default()).unwrap();
        let mut b = Editor {
            scene: back.scene,
            ..Default::default()
        };
        let joints: Vec<String> = b
            .scene
            .items
            .iter()
            .filter(|i| i.is_joint())
            .map(|i| i.name.clone())
            .collect();
        assert_eq!(
            joints,
            ["Shoulder1", "Elbow1", "Wrist1"],
            "{:?}",
            back.notes
        );
        let m2 = b
            .scene
            .items
            .iter()
            .find(|i| i.is_mesh() && i.mesh_ops.is_some())
            .expect("a skinned mesh")
            .id;
        let (_, v) = skin_commands::skin_op(&b.scene, m2).unwrap();
        assert_eq!(v.interpolation == skin::DUAL_QUATERNION, dq);
        let w1 = &e
            .scene
            .item(mesh)
            .unwrap()
            .mesh
            .open()
            .attributes
            .point_weights;
        let w2 = &b
            .scene
            .item(m2)
            .unwrap()
            .mesh
            .open()
            .attributes
            .point_weights;
        for name in ["Shoulder1", "Elbow1", "Wrist1"] {
            for (p, w) in &w1[name] {
                let got = w2[name].get(p).copied().unwrap_or(0.);
                assert!((got - w).abs() < 1e-5, "{name} {p}: {got} vs {w}");
            }
        }
        for f in [0, 12, 24] {
            ok(&mut e, &format!("operandi.time.set frame:{f}"));
            ok(&mut b, &format!("operandi.time.set frame:{f}"));
            let ours = e
                .evaluated(evaluate::Purpose::Final)
                .item(mesh)
                .unwrap()
                .mesh
                .open()
                .vertices
                .clone();
            let theirs = b
                .evaluated(evaluate::Purpose::Final)
                .item(m2)
                .unwrap()
                .mesh
                .open()
                .vertices
                .clone();
            assert_eq!(ours.len(), theirs.len());
            for (p, (a, c)) in ours.iter().zip(&theirs).enumerate() {
                assert!(
                    a.distance(*c) < 1e-4,
                    "dq {dq} frame {f} point {p}: {a} vs {c}"
                );
            }
        }
    }
}

const BLENDER: &str = "/Applications/Blender.app/Contents/MacOS/Blender";

/// The Blender script: import argv[0], print the deformed mesh's world
/// points at frames 0, 12, 24, export the scene again to argv[2].
const BLENDER_SCRIPT: &str = r#"import bpy, sys
argv = sys.argv[sys.argv.index("--") + 1:]
src, out, again = argv[0], argv[1], argv[2]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.wm.usd_import(filepath=src)
scene = bpy.context.scene
lines = []
mesh = next(o for o in bpy.data.objects if o.type == 'MESH')
arm = next(o for o in bpy.data.objects if o.type == 'ARMATURE')
lines.append("BONES " + " ".join(b.name for b in arm.data.bones))
mod = next(m for m in mesh.modifiers if m.type == 'ARMATURE')
lines.append(f"VOLUME {mod.use_deform_preserve_volume}")
for f in (0, 12, 24):
    scene.frame_set(f)
    dg = bpy.context.evaluated_depsgraph_get()
    ev = mesh.evaluated_get(dg)
    me = ev.to_mesh()
    for i in range(0, len(me.vertices), 37):
        p = mesh.matrix_world @ me.vertices[i].co
        lines.append(f"P {f} {i} {p.x} {p.y} {p.z}")
    ev.to_mesh_clear()
open(out, "w").write("\n".join(lines) + "\n")
scene.frame_set(0)
bpy.ops.wm.usd_export(filepath=again, export_animation=True, export_armatures=True)
"#;

/// §8 milestone check 3: animate the arm, export USD, open it in Blender:
/// the same deformation; Blender's own export reads back into Operandi
/// with the same joints and the same deformation. Skipped where Blender
/// isn't installed.
#[test]
fn blender_round_trip() {
    if !std::path::Path::new(BLENDER).is_file() {
        eprintln!("no Blender here; round trip skipped");
        return;
    }
    let (mut e, mesh) = arm(false);
    let dir = out_dir();
    let path = dir.join("arm-to-blender.usd");
    crate::export(
        &path,
        &e.evaluated(evaluate::Purpose::Final),
        ExportFormat::Usd,
    )
    .unwrap();
    let script = dir.join("blender_round_trip.py");
    std::fs::write(&script, BLENDER_SCRIPT).unwrap();
    let report = dir.join("blender.txt");
    let again = dir.join("from-blender.usda");
    let out = std::process::Command::new(BLENDER)
        .args(["--background", "--factory-startup", "--python"])
        .arg(&script)
        .arg("--")
        .args([&path, &report, &again])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&report).unwrap();
    assert!(text.contains("BONES Shoulder1 Elbow1 Wrist1"), "{text}");
    // Blender is Z up: our (x, y, z) is its (x, -z, y).
    let mut seen = 0;
    for line in text.lines().filter(|l| l.starts_with("P ")) {
        let v: Vec<f64> = line[2..]
            .split_whitespace()
            .map(|w| w.parse().unwrap())
            .collect();
        ok(&mut e, &format!("operandi.time.set frame:{}", v[0]));
        let ours = e
            .evaluated(evaluate::Purpose::Final)
            .item(mesh)
            .unwrap()
            .mesh
            .open()
            .vertices[v[1] as usize];
        let theirs = glam::DVec3::new(v[2], v[4], -v[3]);
        assert!(
            ours.distance(theirs) < 1e-4,
            "frame {} point {}: {ours} vs Blender {theirs}",
            v[0],
            v[1]
        );
        seen += 1;
    }
    assert!(seen > 6, "{text}");
    // Back from Blender.
    let back = crate::import(&again, &Default::default()).unwrap();
    let mut b = Editor {
        scene: back.scene,
        ..Default::default()
    };
    let joints: Vec<String> = b
        .scene
        .items
        .iter()
        .filter(|i| i.is_joint())
        .map(|i| i.name.clone())
        .collect();
    assert_eq!(
        joints,
        ["Shoulder1", "Elbow1", "Wrist1"],
        "{:?}",
        back.notes
    );
    let m2 = b
        .scene
        .items
        .iter()
        .find(|i| i.is_mesh() && i.mesh_ops.is_some())
        .expect("a skinned mesh")
        .id;
    for f in [0, 12, 24] {
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
        ok(&mut b, &format!("operandi.time.set frame:{f}"));
        let s1 = e.evaluated(evaluate::Purpose::Final);
        let s2 = b.evaluated(evaluate::Purpose::Final);
        let w1 = s1.world_matrix(mesh).unwrap();
        let w2 = s2.world_matrix(m2).unwrap();
        let ours = &s1.item(mesh).unwrap().mesh.open().vertices;
        let theirs = &s2.item(m2).unwrap().mesh.open().vertices;
        assert_eq!(ours.len(), theirs.len());
        for (p, (a, c)) in ours.iter().zip(theirs.iter()).enumerate() {
            let (a, c) = (w1.transform_point3(*a), w2.transform_point3(*c));
            assert!(
                a.distance(c) < 1e-4,
                "back from Blender, frame {f} point {p}: {a} vs {c}"
            );
        }
    }
}

/// Phase 6 §7: an arm moved by two-bone IK exports its solved pose as
/// the SkelAnimation (no solver, goal or task in the file), and reads
/// back deforming the same.
#[test]
fn an_ik_arm_exports_its_solved_pose() {
    let (mut e, mesh) = arm(false);
    // Clear the elbow keys; drive the arm with IK instead.
    e.scene
        .animation
        .channels
        .retain(|k, _| !k.channel.starts_with("rot."));
    let joints: Vec<u64> = e
        .scene
        .items
        .iter()
        .filter(|i| i.is_joint())
        .map(|i| i.id)
        .collect();
    // Bend the elbow a little in Setup so the chain has a side.
    ok(&mut e, "operandi.anim.setup on:true");
    ok(&mut e, &format!("operandi.skel.move joint:{} position:0.25,0,-0.05 compensate:true preserveAlignment:true", joints[1]));
    ok(
        &mut e,
        &format!(
            "operandi.ik.applyTwoBone joints:{},{},{}",
            joints[0], joints[1], joints[2]
        ),
    );
    ok(&mut e, "operandi.anim.setup on:false");
    let goal = e
        .scene
        .items
        .iter()
        .find(|i| i.name.ends_with("_IKGoal"))
        .unwrap()
        .id;
    ok(&mut e, "operandi.time.set frame:0");
    ok(
        &mut e,
        &format!("operandi.key.set channels:{goal}:pos.X,{goal}:pos.Y"),
    );
    ok(&mut e, "operandi.time.set frame:24");
    ok(
        &mut e,
        &format!("transform.channel name:pos.X value:0.4 item:{goal}"),
    );
    ok(
        &mut e,
        &format!("transform.channel name:pos.Y value:0.6 item:{goal}"),
    );
    ok(
        &mut e,
        &format!("operandi.key.set channels:{goal}:pos.X,{goal}:pos.Y"),
    );
    let path = out_dir().join("ik-arm.usda");
    crate::export(
        &path,
        &e.evaluated(evaluate::Purpose::Final),
        ExportFormat::Usda,
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("IKGoal\""), "no goal prim in the file");
    let back = crate::import(&path, &Default::default()).unwrap();
    let mut b = Editor {
        scene: back.scene,
        ..Default::default()
    };
    assert!(
        b.scene.animation.rig.pieces.is_empty(),
        "no solver read back"
    );
    let m2 = b
        .scene
        .items
        .iter()
        .find(|i| i.is_mesh() && i.mesh_ops.is_some())
        .unwrap()
        .id;
    for f in [0, 12, 24] {
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
        ok(&mut b, &format!("operandi.time.set frame:{f}"));
        let ours = e
            .evaluated(evaluate::Purpose::Final)
            .item(mesh)
            .unwrap()
            .mesh
            .open()
            .vertices
            .clone();
        let theirs = b
            .evaluated(evaluate::Purpose::Final)
            .item(m2)
            .unwrap()
            .mesh
            .open()
            .vertices
            .clone();
        for (p, (x, y)) in ours.iter().zip(&theirs).enumerate() {
            assert!(x.distance(*y) < 1e-4, "frame {f} point {p}: {x} vs {y}");
        }
    }
}
