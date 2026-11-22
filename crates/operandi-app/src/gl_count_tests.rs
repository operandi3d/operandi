//! The "GL: n" viewport readout: triangles drawn for the visible scene,
//! subdivision included, counted from the cached GPU geometry when the
//! views hold it and from the polygon list otherwise.
use super::tests::draw;
use super::*;
use crate::defaults_preferences::thousands;

/// A new scene whose active item is a 24-side, 12-segment sphere: 240
/// quads and 48 pole triangles.
fn sphere_app() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let sphere = operandi_core::radial::globe_geometry(
        DVec3::ZERO,
        operandi_core::radial::RadialParameters {
            radius: DVec3::ONE,
            sides: 24,
            segments: 12,
        },
    )
    .unwrap();
    let sides: Vec<usize> = sphere.faces.iter().map(|f| f.vertices.len()).collect();
    assert_eq!(sides.iter().filter(|&&n| n == 4).count(), 240);
    assert_eq!(sides.iter().filter(|&&n| n == 3).count(), 48);
    assert_eq!(sides.len(), 288);
    a.editor.scene.active_mut().unwrap().mesh = sphere.into();
    a.editor.undo.clear();
    a
}

fn gl_line(a: &Operandi) -> String {
    let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
    a.overlay_lines(r)
        .into_iter()
        .find(|l| l.starts_with("GL: "))
        .expect("GL line")
}

fn subdivide(a: &mut Operandi) {
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    let item = a.editor.scene.active().unwrap();
    assert_eq!(
        item.subdivision
            .clone()
            .unwrap_or_default()
            .catmull_clark_level,
        2
    );
}

#[test]
fn sphere_counts_quads_as_two_triangles() {
    let a = sphere_app();
    assert_eq!(a.drawn_triangles(), 240 * 2 + 48);
    assert_eq!(gl_line(&a), "GL: 528");
}

#[test]
fn subdivided_sphere_at_level_2() {
    let mut a = sphere_app();
    subdivide(&mut a);
    // Each quad becomes 16 quads, each pole triangle 12 quads.
    assert_eq!(a.drawn_triangles(), 240 * 16 * 2 + 48 * 12 * 2);
    assert_eq!(gl_line(&a), "GL: 8,832");
}

#[test]
fn gpu_geometry_count_matches_and_follows_changes() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = sphere_app();
    a.force_resident = true;
    draw(&mut a, &ctx, size, vec![]);
    assert_eq!(resident(&a), Some(528));
    assert_eq!(gl_line(&a), "GL: 528");
    // SubD on: before the views draw it, the old fill is not counted.
    subdivide(&mut a);
    assert_eq!(resident(&a), None, "stale fill");
    assert_eq!(gl_line(&a), "GL: 8,832", "from the polygon list");
    // Once drawn, the cached fill is the level-2 surface.
    draw(&mut a, &ctx, size, vec![]);
    assert_eq!(resident(&a), Some(8_832));
    assert_eq!(gl_line(&a), "GL: 8,832");
    // Level 1: n quads per n-gon.
    let mut settings = a
        .editor
        .scene
        .active()
        .unwrap()
        .subdivision
        .clone()
        .unwrap_or_default();
    settings.catmull_clark_level = 1;
    a.editor.scene.active_mut().unwrap().subdivision = Some(settings);
    draw(&mut a, &ctx, size, vec![]);
    assert_eq!(
        gl_line(&a),
        format!("GL: {}", thousands((240 * 4 + 48 * 3) * 2))
    );
    // Hidden: the views drop it, and so does the count.
    a.editor.scene.active_mut().unwrap().visible = false;
    draw(&mut a, &ctx, size, vec![]);
    assert_eq!(resident(&a), None);
    assert_eq!(gl_line(&a), "GL: 0");
}

/// The active item's cached GPU triangle count, when current.
fn resident(a: &Operandi) -> Option<u64> {
    a.resident_cache
        .drawn_triangles(a.editor.scene.active().unwrap())
}

#[test]
fn hidden_items_are_not_counted() {
    let mut a = sphere_app();
    let sphere = a.editor.scene.active().unwrap().id;
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::cube_geometry(DVec3::ZERO, DVec3::ONE)
            .unwrap()
            .into();
    assert_eq!(gl_line(&a), "GL: 540", "sphere 528 + cube 12");
    a.editor
        .scene
        .items
        .iter_mut()
        .find(|i| i.id == sphere)
        .unwrap()
        .visible = false;
    assert_eq!(gl_line(&a), "GL: 12");
}

#[test]
fn polygon_list_count_covers_ngons_and_subdivision_kinds() {
    use operandi_core::subdivision::{SubdivisionSettings, drawn_triangles};
    let settings = SubdivisionSettings::default();
    // A box: 6 quads.
    let mut cube = operandi_core::cube_geometry(DVec3::ZERO, DVec3::ONE).unwrap();
    assert_eq!(drawn_triangles(&cube, &settings), 12);
    // A pentagon is 3 triangles.
    let extra = cube.faces[1].vertices[2];
    cube.faces[0].vertices.push(extra);
    assert_eq!(drawn_triangles(&cube, &settings), 13);
    // Subdivision (L² quads per corner) at level 3: 6 x 4 x 9 quads.
    let mut cube = operandi_core::cube_geometry(DVec3::ZERO, DVec3::ONE).unwrap();
    cube.attributes.representations = vec![operandi_core::PolygonRepresentation::Subdivs; 6];
    let level3 = SubdivisionSettings {
        subdivision_level: 3,
        ..settings.clone()
    };
    assert_eq!(drawn_triangles(&cube, &level3), 6 * 4 * 9 * 2);
    // Level 0 keeps the polygons.
    let level0 = SubdivisionSettings {
        subdivision_level: 0,
        ..settings
    };
    assert_eq!(drawn_triangles(&cube, &level0), 12);
}

#[test]
fn thousands_separators() {
    assert_eq!(thousands(8_832), "8,832");
    assert_eq!(thousands(12_345_678), "12,345,678");
    assert_eq!(thousands(528), "528");
}

/// The GL line counts triangles by subdivision kind and not from the
/// smooth-shading flag, from the polygon list and from the GPU fill alike
/// (PREF-Q21; the graphics memory watch counts the same way).
#[test]
fn gl_line_counts_drawn_triangles() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = sphere_app();
    assert_eq!(a.drawn_triangles(), 528);
    // Smooth shading on every polygon draws nothing extra.
    for f in &mut a.editor.scene.active_mut().unwrap().mesh.faces {
        f.smooth = true;
    }
    assert_eq!(a.drawn_triangles(), 528);
    a.force_resident = true;
    draw(&mut a, &ctx, size, vec![]);
    subdivide(&mut a);
    assert_eq!(a.drawn_triangles(), 8_832, "from the polygon list");
    draw(&mut a, &ctx, size, vec![]);
    assert_eq!(resident(&a), Some(8_832));
    assert_eq!(a.drawn_triangles(), 8_832, "from the GPU fill");
    assert_eq!(gl_line(&a), "GL: 8,832");
    // Hidden items don't count.
    a.editor.scene.active_mut().unwrap().visible = false;
    assert_eq!(a.drawn_triangles(), 0);
}

/// A mesh with no listed polygon kinds counts from the remembered summary
/// (no pass), and agrees with the per-polygon count.
#[test]
fn plain_mesh_triangles_come_from_the_summary() {
    use operandi_core::subdivision::{SubdivisionSettings, drawn_triangles};
    let a = sphere_app();
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert!(mesh.attributes.representations.is_empty());
    assert_eq!(mesh.face_summary().plain_triangles, 528);
    assert_eq!(drawn_triangles(mesh, &SubdivisionSettings::default()), 528);
}
