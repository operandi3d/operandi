//! The viewport's faster drawing (perf task 3, 2026-11-13) makes the same
//! pictures: an edge two polygons share is drawn once at the opacity of
//! two strokes, and a whole selected item takes its selection as a uniform
//! instead of selected geometry. Each scene is drawn both ways on a real
//! graphics device and the pictures compared; blending one stroke at the
//! doubled opacity rounds at most one step of 255 differently.
use super::*;

fn wavy(n: u32) -> operandi_core::Mesh {
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (x, y) = (i as f64 / n as f64 - 0.5, j as f64 / n as f64 - 0.5);
            mesh.vertices
                .push(DVec3::new(x, y, 0.15 * (x * 9.).sin() * (y * 7.).cos()));
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

/// The picture of the first view after a few frames, drawn the new or the
/// reference way.
fn picture(setup: &dyn Fn(&mut Operandi), reference: bool) -> Option<Vec<u8>> {
    picture_of(16, setup, reference, false)
}

/// `picture` of an `n` × `n` grid, with Fade Wireframe on Small Polygons
/// on or off.
fn picture_of(
    n: u32,
    setup: &dyn Fn(&mut Operandi),
    reference: bool,
    fade: bool,
) -> Option<Vec<u8>> {
    let state = super::performance_tests::headless_gpu()?;
    crate::gpu_scene::set_reference_drawing(reference);
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.viewport_preferences.fade_small_wire = fade;
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = wavy(n).into();
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    a.camera.yaw += 0.4;
    a.camera.pitch -= 0.3;

    setup(&mut a);
    for _ in 0..3 {
        super::tests::draw(&mut a, &ctx, vec2(900., 600.), vec![]);
    }
    crate::gpu_scene::set_reference_drawing(false);
    let views = a.viewport_gpu.as_ref().unwrap().read_views();
    views.into_iter().next().map(|(_, _, rgba)| rgba)
}

#[test]
fn faster_drawing_makes_the_same_pictures() {
    let lock = |a: &mut Operandi| {
        let item = a.editor.scene.active_mut().unwrap();
        item.elements.locked.faces = (40..60).collect();
    };
    type Setup = Box<dyn Fn(&mut Operandi)>;
    let scenes: Vec<(&str, Setup)> = vec![
        (
            "polygon mode",
            Box::new(|a| a.run(Command::Mode(Mode::Polygon))),
        ),
        (
            "polygons selected",
            Box::new(|a| {
                a.run(Command::Mode(Mode::Polygon));
                a.editor.scene.selection.faces = vec![100, 101, 117, 5];
            }),
        ),
        (
            "item selected",
            Box::new(|a| a.run(Command::Mode(Mode::Item))),
        ),
        (
            "locked polygons",
            Box::new(move |a| {
                lock(a);
                a.run(Command::Mode(Mode::Polygon));
            }),
        ),
        (
            "locked, item selected",
            Box::new(move |a| {
                lock(a);
                a.run(Command::Mode(Mode::Item));
            }),
        ),
        (
            "subdivided, item selected",
            Box::new(|a| {
                a.run(Command::ToggleSubdivision(
                    operandi_core::subdivision::Toggle::CatmullClark,
                ));
                a.run(Command::Mode(Mode::Item));
            }),
        ),
    ];
    for (name, setup) in &scenes {
        let (Some(new), Some(old)) = (
            picture(setup.as_ref(), false),
            picture(setup.as_ref(), true),
        ) else {
            println!("no graphics device");
            return;
        };
        assert_eq!(new.len(), old.len(), "{name}");
        let most = new.iter().zip(&old).map(|(p, q)| p.abs_diff(*q)).max();
        assert!(most <= Some(1), "{name}: pictures differ by {most:?}/255");
        assert!(old.chunks(4).any(|p| p[3] > 0), "{name}: nothing drawn");
    }
}

/// Fade Wireframe on Small Polygons, seen square on: a coarse grid (edges
/// far longer than the fade's few points) draws exactly as without it; a
/// dense one loses most of its wire.
#[test]
fn wire_fades_only_where_polygons_are_tiny() {
    let top = |a: &mut Operandi| {
        a.run(Command::Mode(Mode::Polygon));
        // The grid lies in X/Y: Front looks straight at it.
        a.camera.view = crate::camera::View::Front;
        a.frame(false);
    };
    let (Some(coarse_on), Some(coarse_off)) = (
        picture_of(16, &top, false, true),
        picture_of(16, &top, false, false),
    ) else {
        println!("no graphics device");
        return;
    };
    let coarse_changed = coarse_on
        .chunks(4)
        .zip(coarse_off.chunks(4))
        .filter(|(p, q)| p != q)
        .count();
    let most = coarse_on
        .iter()
        .zip(&coarse_off)
        .map(|(p, q)| p.abs_diff(*q))
        .max();

    assert!(
        coarse_changed == 0,
        "a coarse mesh's wire changed: {coarse_changed} pixels, by up to {most:?}"
    );
    // 200 × 200 quads: each a few pixels, well under the fade (was 400,
    // four times the work for the same answer; test speed 2026-11-16).
    let dense_on = picture_of(200, &top, false, true).unwrap();
    let dense_off = picture_of(200, &top, false, false).unwrap();
    let changed = dense_on
        .chunks(4)
        .zip(dense_off.chunks(4))
        .filter(|(p, q)| p != q)
        .count();
    assert!(
        changed > 1000,
        "the dense wire didn't fade ({changed} pixels changed)"
    );
}

/// The fade is a preference like any other (UIC-165): on by default, set
/// and read with `pref.value`.
#[test]
fn wire_fade_preference_is_on_by_default_and_scriptable() {
    let mut a = Operandi::empty();
    assert!(a.viewport_preferences.fade_small_wire);
    a.execute("pref.value display.fade_small_wire false");
    assert!(!a.viewport_preferences.fade_small_wire, "{}", a.status);
    a.execute("pref.value display.fade_small_wire ?");
    assert_eq!(a.status, "false");
    a.execute("pref.value display.fade_small_wire true");
    assert!(a.viewport_preferences.fade_small_wire, "{}", a.status);
}

/// Show Material Transparency (viewport transparency, VT-Q05): on by
/// default, set and read with `pref.value`.
#[test]
fn material_transparency_preference_is_on_by_default_and_scriptable() {
    let mut a = Operandi::empty();
    assert!(a.viewport_preferences.material_transparency);
    a.execute("pref.value display.material_transparency false");
    assert!(
        !a.viewport_preferences.material_transparency,
        "{}",
        a.status
    );
    a.execute("pref.value display.material_transparency ?");
    assert_eq!(a.status, "false");
    a.execute("pref.value display.material_transparency true");
    assert!(a.viewport_preferences.material_transparency, "{}", a.status);
}

/// A Wireframe-only view never fades: a dense mesh drawn only as wire
/// would vanish.
#[test]
fn wireframe_style_never_fades() {
    let wire_only = |a: &mut Operandi| {
        a.run(Command::Mode(Mode::Polygon));
        a.display.settings.active.style = crate::view_display::Style::Wireframe;
        a.display.settings.inactive.style = crate::view_display::Style::Wireframe;
        a.camera.view = crate::camera::View::Front;
        a.frame(false);
    };
    let (Some(on), Some(off)) = (
        picture_of(200, &wire_only, false, true),
        picture_of(200, &wire_only, false, false),
    ) else {
        println!("no graphics device");
        return;
    };
    assert!(on == off, "the wireframe-only view faded");
}

#[test]
fn primitive_projected_preview_excludes_hidden_flat_and_smooth_faces() {
    use operandi_core::PolygonRepresentation::{Face as Flat, Subdivs};

    let state = super::performance_tests::headless_gpu();
    if state.is_none() {
        println!("no graphics device; checking submitted wire and fill");
    }
    use crate::viewport::PROJECTED_VERTEX_COUNTS;
    let picture = |omit_hidden: bool, style| {
        let mut mesh = operandi_core::Mesh::default();
        for k in (0..4).filter(|k| !omit_hidden || k % 2 == 0) {
            let offset = mesh.vertices.len() as u32;
            let center = DVec3::new((k as f64 - 1.5) * 1.3, 0., 0.);
            let quad = wavy(1);
            mesh.vertices
                .extend(quad.vertices.iter().map(|v| center + *v));
            let mut face = quad.faces[0].clone();
            face.vertices.iter_mut().for_each(|v| *v += offset);
            face.smooth = k >= 2;
            mesh.faces.push(face);
            mesh.attributes
                .representations
                .push(if k < 2 { Flat } else { Subdivs });
        }
        let mut a = Operandi::empty();
        a.viewport_gpu = state.clone().map(gpu::ViewportGpu::new);
        a.run(Command::NewMesh);
        let item = a.editor.scene.active_mut().unwrap();
        item.mesh = mesh.into();
        if !omit_hidden {
            item.elements.hidden.faces.extend([1, 3]);
        }
        a.camera.view = crate::camera::View::Front;
        a.camera.distance = 7.;
        a.display.settings.active.style = style;
        a.run(Command::Activate(Tool::Box));
        assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Box);
        let ctx = Context::default();
        theme::install(&ctx);
        PROJECTED_VERTEX_COUNTS.set(Some([0, 0]));
        for _ in 0..3 {
            super::tests::draw(&mut a, &ctx, vec2(900., 600.), vec![]);
        }
        let counts = PROJECTED_VERTEX_COUNTS.replace(None).unwrap();
        let pixels = a
            .viewport_gpu
            .as_ref()
            .map(|gpu| gpu.read_views().remove(0).2);
        (counts, pixels)
    };

    use crate::view_display::Style;
    for style in [Style::Wireframe, Style::Solid] {
        let (expected, expected_pixels) = picture(true, style);
        let (actual, actual_pixels) = picture(false, style);
        let pass = usize::from(style == Style::Wireframe);
        assert!(
            expected[pass] > 0,
            "{style:?}: visible neighbours were not drawn"
        );
        assert_eq!(
            actual, expected,
            "{style:?}: submitted fill and wire vertices"
        );
        assert!(
            actual_pixels == expected_pixels,
            "{style:?}: the preview differs from the visible-neighbour control"
        );
    }
}
