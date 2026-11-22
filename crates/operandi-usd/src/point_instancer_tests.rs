//! USD-IQ25: scatters of Mesh Instances as PointInstancers, through
//! OpenUSD.
use crate::plan_tests::{add_box, box_scene};
use glam::DVec3;
use operandi_core::Scene;
use operandi_core::interchange::{ExportFormat, Imported};
use std::path::PathBuf;
use std::time::Instant;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("operandi-usd-instancer")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A source box and `n` Mesh Instances of it under one Group Locator,
/// each moved, turned (in several rotation orders) and scaled; every
/// 97th one hidden.
fn scatter(n: usize) -> Scene {
    use operandi_core::scene::RotationOrder;
    let mut scene = box_scene("Rock");
    let source = scene.items[0].id;
    let shared = scene.items[0].mesh.clone();
    let field = scene.add_group_locator().unwrap();
    scene.item_mut(field).unwrap().name = "Field".into();
    let orders = [RotationOrder::Xyz, RotationOrder::Zxy, RotationOrder::Yzx];
    for i in 0..n {
        let id = add_box(&mut scene, &format!("Rock_{i}"), Some(field));
        let item = scene.item_mut(id).unwrap();
        item.instance_of = Some(source);
        item.mesh = shared.clone();
        let f = i as f64;
        let t = &mut item.transform;
        t.translation = DVec3::new((f * 0.37).sin() * 50.0, f * 0.01, (f * 0.73).cos() * 50.0);
        t.rotation = DVec3::new(f * 0.011, f * 0.029, f * 0.017);
        t.scale = DVec3::splat(0.5 + (i % 7) as f64 * 0.1);
        t.set_rotation_order(orders[i % 3]);
        item.visible = i % 97 != 5;
    }
    scene
}

fn export_and_import(scene: &Scene, path: &std::path::Path) -> (Vec<String>, Imported, u64, f64) {
    let notes = crate::export(path, scene, ExportFormat::Usdc)
        .unwrap()
        .notes;
    let size = std::fs::metadata(path).unwrap().len();
    let started = Instant::now();
    let back = crate::import(path, &Default::default()).unwrap();
    (notes, back, size, started.elapsed().as_secs_f64())
}

/// Every Rock's world placement, eye, parent name and instance source,
/// by name.
fn placements(scene: &Scene) -> Vec<(String, glam::DMat4, bool, String, bool)> {
    let mut out: Vec<_> = scene
        .items
        .iter()
        .filter(|i| i.name.starts_with("Rock_"))
        .map(|i| {
            let parent = i
                .parent
                .and_then(|p| scene.item(p))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            let source = i
                .instance_of
                .and_then(|s| scene.item(s))
                .is_some_and(|s| s.name == "Rock");
            (
                i.name.clone(),
                scene.world_matrix(i.id).unwrap(),
                i.visible,
                parent,
                source,
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn same_places(a: &Scene, b: &Scene) {
    let (pa, pb) = (placements(a), placements(b));
    assert_eq!(pa.len(), pb.len());
    for (x, y) in pa.iter().zip(&pb) {
        assert_eq!((&x.0, x.2, &x.3, x.4), (&y.0, y.2, &y.3, y.4));
        assert!(
            x.1.abs_diff_eq(y.1, 1e-4),
            "{}: {:?} vs {:?}",
            x.0,
            x.1,
            y.1
        );
    }
}

/// A 1,000-copy scatter is one PointInstancer and comes back as the same
/// 1,000 Mesh Instances; compared with instanceable Xforms it is smaller
/// and loads faster (sizes and times are printed: `-- --nocapture`).
#[test]
fn a_thousand_copy_scatter_round_trips_as_a_point_instancer() {
    let scene = scatter(1000);
    let dir = scratch("thousand");
    let (notes, back, size, load) = export_and_import(&scene, &dir.join("instancer.usdc"));
    assert!(
        notes
            .iter()
            .any(|n| n.contains("1000 Mesh Instance(s) written as PointInstancer points")),
        "{notes:?}"
    );
    same_places(&scene, &back.scene);
    // The source keeps its own place; nothing extra arrives (no locator
    // for the instancer).
    assert_eq!(
        back.scene
            .items
            .iter()
            .filter(|i| !i.is_texture_item())
            .count(),
        scene.items.len()
    );
    let (_, back_x, size_x, load_x) = crate::point_instancer::with_threshold(usize::MAX, || {
        export_and_import(&scene, &dir.join("xforms.usdc"))
    });
    same_places(&scene, &back_x.scene);
    println!(
        "1000-copy scatter: PointInstancer {size} bytes, imported in {load:.3} s; \
         instanceable Xforms {size_x} bytes, imported in {load_x:.3} s"
    );
    assert!(size < size_x, "{size} vs {size_x}");
}

/// Below the threshold, or with an instance that has a child or its own
/// subdivision, a scatter stays instanceable Xforms.
#[test]
fn small_or_mixed_scatters_stay_xforms() {
    let few = scatter(crate::point_instancer::DEFAULT_THRESHOLD - 1);
    let path = scratch("few").join("few.usda");
    crate::export(&path, &few, ExportFormat::Usda).unwrap();
    assert!(
        !std::fs::read_to_string(&path)
            .unwrap()
            .contains("PointInstancer")
    );

    let mut mixed = scatter(crate::point_instancer::DEFAULT_THRESHOLD);
    let first = mixed.items.iter().find(|i| i.name == "Rock_0").unwrap().id;
    add_box(&mut mixed, "Pebble", Some(first));
    let path = scratch("mixed").join("mixed.usda");
    crate::export(&path, &mixed, ExportFormat::Usda).unwrap();
    // One left with a child: 15 points, not enough.
    assert!(
        !std::fs::read_to_string(&path)
            .unwrap()
            .contains("PointInstancer")
    );
    let back = crate::import(&path, &Default::default()).unwrap();
    same_places(&mixed, &back.scene);

    let ok = scatter(crate::point_instancer::DEFAULT_THRESHOLD);
    let path = scratch("ok").join("ok.usda");
    crate::export(&path, &ok, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    for want in [
        "def PointInstancer \"Rock_instances\"",
        "quath[] orientations",
        "quatf[] orientationsf",
        "int64[] invisibleIds",
        "instanceable = true",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
    let back = crate::import(&path, &Default::default()).unwrap();
    same_places(&ok, &back.scene);
}
