//! Drags on big meshes rewrite only what moved (Jason, 2026-11-13): the
//! points' copy shares its polygons, validation and the GPU geometry patch
//! the moved neighbourhood. These tests check that the patched result is
//! exactly what a full rebuild makes, frame by frame, after the drop, and
//! after undo and redo.
use super::*;

/// `n` x `n` quads on a wave, so neighbouring normals differ and smooth
/// shading averages them.
fn wavy(n: u32) -> operandi_core::Mesh {
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
                smooth: true,
                part: None,
                material_label: None,
            });
        }
    }
    mesh
}

/// The cached GPU geometry of the active item equals a fresh full build of
/// it as it is now, and was drawn from the scene's current mesh.
fn assert_matches_full_build(a: &Operandi, when: &str) {
    let item = a.editor.scene.active().unwrap().clone();
    let (cached, selected) = a.resident_cache.cached(item.id).expect("drawn");
    let smooth = operandi_core::subdivision::has_smooth(&item.mesh).then(|| {
        crate::subdivision_view::SubdivisionCache::default()
            .get(&item)
            .unwrap()
    });
    let matrix = a.editor.scene.world_matrix(item.id).unwrap();
    let fresh = crate::gpu_scene::Resident::default()
        .item(
            &item,
            matrix,
            selected,
            smooth.as_deref(),
            &a.editor.scene.smoothing,
        )
        .0;
    assert!(
        cached.fill == fresh.fill,
        "{when}: fill differs from a full build"
    );
    assert!(
        cached.edges == fresh.edges,
        "{when}: wire differs from a full build"
    );
    assert!(
        cached.highlight == fresh.highlight,
        "{when}: highlight differs from a full build"
    );
    assert_eq!(cached.fill_ranges, fresh.fill_ranges, "{when}");
    assert_eq!(cached.edge_ranges, fresh.edge_ranges, "{when}");
}

fn bits(mesh: &operandi_core::Mesh) -> Vec<[u64; 3]> {
    mesh.vertices
        .iter()
        .map(|p| p.to_array().map(f64::to_bits))
        .collect()
}

/// Move drags of polygons and of points, plain and subdivided: each frame
/// is a patch (no full rebuild), the patched geometry equals a full build,
/// the polygon list stays shared with the pre-drag mesh, and undo and redo
/// restore the exact points and geometry.
#[test]
fn drag_patches_match_a_full_rebuild_through_undo_and_redo() {
    let n = 30u32;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(800., 600.);
    for subdivided in [false, true] {
        for mode in [Mode::Polygon, Mode::Vertex] {
            let when = |what: &str| format!("{what} ({mode:?}, subdivided {subdivided})");
            let mut a = Operandi::empty();
            a.run(Command::NewMesh);
            a.editor.scene.active_mut().unwrap().mesh = wavy(n).into();
            if subdivided {
                a.run(Command::ToggleSubdivision(
                    operandi_core::subdivision::Toggle::CatmullClark,
                ));
            }
            a.force_resident = true;
            a.camera.view = crate::camera::View::Perspective;
            a.frame(false);
            a.run(Command::Mode(mode));
            let mid = (n / 2) * n + n / 2;
            match mode {
                Mode::Polygon => {
                    a.editor.scene.selection.faces = vec![mid, mid + 1, mid + n, mid + n + 1]
                }
                _ => {
                    let p = (n / 2) * (n + 1) + n / 2;
                    a.editor.scene.selection.vertices = vec![p, p + 1, p + n + 1];
                }
            }
            super::tests::draw(&mut a, &ctx, size, vec![]);
            let before = a.editor.scene.active().unwrap().mesh.shared().clone();
            assert_matches_full_build(&a, &when("before the drag"));
            a.activate(Tool::Move);
            let base = a
                .editor
                .session
                .live
                .as_ref()
                .expect("Move active")
                .params
                .clone();
            let builds = a.resident_cache.builds();
            let patches = a.resident_cache.patches();
            for k in 1..=5 {
                let mut p = base.clone();
                p.delta = DVec3::new(0.013 * k as f64, -0.007 * k as f64, 0.021);
                assert!(a.run_ok(Command::Preview(p)), "{}", a.status);
                super::tests::draw(&mut a, &ctx, size, vec![]);
                let now = a.editor.scene.active().unwrap().mesh.shared().clone();
                assert!(
                    operandi_core::Shared::same(&now.faces, &before.faces),
                    "{}",
                    when("the drag copied the polygons")
                );
                assert_matches_full_build(&a, &when(&format!("drag frame {k}")));
            }
            assert_eq!(
                a.resident_cache.builds(),
                builds,
                "{}",
                when("a drag frame rebuilt the GPU geometry in full")
            );
            assert!(
                a.resident_cache.patches() >= patches + 5,
                "{}",
                when("no patches")
            );
            let dragged = bits(&a.editor.scene.active().unwrap().mesh);
            assert_ne!(dragged, bits(&before), "{}", when("nothing moved"));
            a.run(Command::Drop);
            super::tests::draw(&mut a, &ctx, size, vec![]);
            assert_matches_full_build(&a, &when("after the drop"));
            assert_eq!(bits(&a.editor.scene.active().unwrap().mesh), dragged);

            a.run(Command::Undo);
            super::tests::draw(&mut a, &ctx, size, vec![]);
            let undone = &a.editor.scene.active().unwrap().mesh;
            assert_eq!(bits(undone), bits(&before), "{}", when("undo"));
            assert!(**undone.shared() == *before, "{}", when("undo"));
            assert_matches_full_build(&a, &when("after undo"));

            a.run(Command::Redo);
            super::tests::draw(&mut a, &ctx, size, vec![]);
            assert_eq!(
                bits(&a.editor.scene.active().unwrap().mesh),
                dragged,
                "{}",
                when("redo")
            );
            assert_matches_full_build(&a, &when("after redo"));
        }
    }
}

/// Astra P6-002: a polygon's own look changing (here Smooth turned off,
/// as Set Part or Set Material change a tag) away from moved points still
/// rewrites it: the quick update equals a full build.
#[test]
fn a_polygon_look_change_updates_like_a_full_build() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = wavy(12).into();
    let item = a.editor.scene.active().unwrap().clone();
    let matrix = a.editor.scene.world_matrix(item.id).unwrap();
    let smoothing = a.editor.scene.smoothing.clone();
    let mut resident = crate::gpu_scene::Resident::default();
    resident.item(&item, matrix, None, None, &smoothing);
    // One edit that moves a point in one corner and changes a polygon's
    // look in the other: the quick path patches around the point.
    let mut changed = item.clone();
    changed.mesh.vertices[0].z += 0.05;
    changed.mesh.faces[140].smooth = false;
    let (quick, _, patch) = resident.item(&changed, matrix, None, None, &smoothing);
    let fresh = crate::gpu_scene::Resident::default()
        .item(&changed, matrix, None, None, &smoothing)
        .0;
    assert!(
        quick.fill == fresh.fill,
        "stale face shading (patched: {})",
        patch.is_some()
    );
}
