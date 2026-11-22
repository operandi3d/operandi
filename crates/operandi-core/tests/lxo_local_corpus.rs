//! Local check against a folder of LXO files that can't be in the repo:
//! `OPERANDI_LXO_DIR=/path cargo test -p operandi-core --test all lxo_local_corpus:: -- --ignored --nocapture`.
//! `.lxl` mesh presets are read with `load_mesh_preset`.
//! Prints one line per file: opened (meshes, polygons, skipped meshes and
//! notes) or the refusal.
use operandi_core::lxo;

#[test]
#[ignore]
fn local_lxo_folder() {
    let Ok(dir) = std::env::var("OPERANDI_LXO_DIR") else {
        return;
    };
    let mut files = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("lxo") || e.eq_ignore_ascii_case("lxl"))
            {
                files.push(p);
            }
        }
    }
    files.sort();
    for path in files {
        let mut go = |_: &str| true;
        // The full path, so a refused file can be found again.
        let name = path.display().to_string();
        let preset = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("lxl"));
        let loaded = if preset {
            lxo::load_mesh_preset(&path, &mut lxo::Control::new(&mut go))
        } else {
            lxo::load(&path, &mut lxo::Control::new(&mut go))
        };
        match loaded {
            Ok(c) => {
                let polygons: usize = c.scene.items.iter().map(|i| i.mesh.faces.len()).sum();
                println!(
                    "OK   {name}: {} meshes, {polygons} polygons, {} skipped",
                    c.scene.items.len(),
                    c.report.rejected_groups.len()
                );
                for g in &c.report.rejected_groups {
                    println!("       skip: {g}");
                }
            }
            Err(e) => println!("FAIL {name}: {e}"),
        }
    }
}

/// Layer pivot acceptance (answered by the spec side 2026-11-15): world
/// bounding boxes the spec gives for four local mesh presets, in Modo
/// coordinates. Same folder variable as above; files that aren't there are
/// skipped. Compared as given, axis for axis: the shelf's uneven Z range
/// matches only that way (2026-11-15).
#[test]
#[ignore]
fn local_layer_pivot_boxes() {
    let Ok(dir) = std::env::var("OPERANDI_LXO_DIR") else {
        return;
    };
    let cases: [(&str, [f64; 3], [f64; 3]); 4] = [
        (
            "Assets/Meshes/Interior/Furniture/Sofas And Lounges/9bs_027_Sofa_c683.lxl",
            [-0.55568, -0.00372, -0.43717],
            [0.55568, 0.81642, 0.43717],
        ),
        (
            "Assets/Meshes/Jewelry/Pendant Heart 01.lxl",
            [-0.02508, -0.0273, -0.0117],
            [0.02508, 0.04271, 0.0117],
        ),
        (
            "Assets/Meshes/Human/Hand 01.lxl",
            [-0.11804, -0.03858, -0.05874],
            [0.11804, 0.03858, 0.05874],
        ),
        (
            "Assets/Meshes/Interior/Furniture/Shelves/9bs_046_Shelving_Enam.lxl",
            [-0.70805, -0.0054, -0.07912],
            [0.65205, 2.05539, 0.15112],
        ),
    ];
    for (file, lo, hi) in cases {
        let path = std::path::Path::new(&dir).join(file);
        if !path.exists() {
            println!("SKIP {file}: not found");
            continue;
        }
        let mut go = |_: &str| true;
        let c = match lxo::load_mesh_preset(&path, &mut lxo::Control::new(&mut go)) {
            Ok(c) => c,
            Err(e) => {
                println!("FAIL {file}: {e}");
                continue;
            }
        };
        let (mut min, mut max) = (glam::DVec3::splat(f64::MAX), glam::DVec3::splat(f64::MIN));
        for item in &c.scene.items {
            let m = c.scene.world_matrix(item.id).unwrap();
            for p in &item.mesh.vertices {
                let w = m.transform_point3(*p);
                min = min.min(w);
                max = max.max(w);
            }
        }
        let want_min = glam::DVec3::from_array(lo);
        let want_max = glam::DVec3::from_array(hi);
        let ok = min.abs_diff_eq(want_min, 1e-4) && max.abs_diff_eq(want_max, 1e-4);
        println!(
            "{} {file}: got {min:.5} .. {max:.5}, want {want_min:.5} .. {want_max:.5}",
            if ok { "MATCH" } else { "MISMATCH" }
        );
    }
}
