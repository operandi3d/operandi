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
