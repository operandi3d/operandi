//! Painting stage 5's inks and Curve Constraint (PNT-170..200), acceptance
//! 25–28. Painting builds only.
#![cfg(feature = "painting")]
use glam::{DVec2, DVec3};
use operandi_core::paint::*;
use operandi_core::paint_inks::*;
use operandi_core::paint_stroke::*;
use operandi_core::*;
use operandi_paint::{Depth, Format, Layout, PaintImage};
use std::sync::Arc;

const VIEW: DVec2 = DVec2::new(400., 400.);

fn textured(mesh: Option<Mesh>) -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    match mesh {
        Some(m) => e.scene.active_mut().unwrap().mesh = m.into(),
        None => e
            .dispatch(Command::InsertUnit {
                tool: Tool::Sphere,
                destination: UnitDestination::ExistingMesh,
            })
            .unwrap(),
    }
    e.dispatch(Command::Paint(PaintEdit::AddTexture {
        kind: TextureKind::Color,
        image: NewImage {
            resolution: 256,
            color: Some([0., 0., 0.]),
            ..NewImage::default()
        },
        selected: None,
        item: None,
    }))
    .unwrap();
    let image = e.paint.last_image.unwrap();
    (e, image)
}

fn plane() -> (Editor, u64) {
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(-1., -1., 0.),
            DVec3::new(1., -1., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(-1., 1., 0.),
        ],
        ..Mesh::default()
    };
    m.faces = vec![Face {
        vertices: vec![0, 1, 2, 3],
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }]
    .into();
    let uv = [(0., 0.), (1., 0.), (1., 1.), (0., 1.)].map(|(u, v)| Some(DVec2::new(u, v)));
    m.uv_maps.insert("Texture".into(), vec![uv.to_vec()]);
    textured(Some(m))
}

fn view() -> PaintView {
    PaintView::looking(DVec3::new(0., 0., 3.), DVec3::ZERO, VIEW)
}

fn stroke(e: &Editor, image: u64, view: PaintView, s: PaintSettings, inks: InkRun) -> PaintStroke {
    let mut st = PaintStroke::begin(StrokeSetup {
        scene: &e.scene,
        item: e.scene.selection.item.unwrap(),
        image,
        pixels: e.scene.paint.buffers[&image].clone(),
        uv_map: None,
        view,
        tool: PaintTool::Paintbrush,
        settings: s,
        symmetry: Symmetry::default(),
        falloff: None,
        lasso: None,
    })
    .unwrap();
    st.inks = inks;
    st
}

fn settings(size: f64) -> PaintSettings {
    let mut s = PaintSettings::default();
    s.tip.size = size;
    s
}

/// A "photo": left half red, right half green.
fn photo() -> Arc<PaintImage> {
    let mut p = PaintImage::new(Layout {
        width: 64,
        height: 32,
        format: Format::Rgba,
        depth: Depth::U8,
    })
    .unwrap();
    for y in 0..32 {
        for x in 0..64 {
            let c = if x < 32 {
                [1., 0., 0., 1.]
            } else {
                [0., 1., 0., 1.]
            };
            p.set((y * 64 + x) as usize, c);
        }
    }
    Arc::new(p)
}

fn colour_at(st: &mut PaintStroke, q: DVec2) -> [f32; 4] {
    let p = st.pixel_at(q).expect("a texel there");
    st.pixels().get(p as usize)
}

fn image_ink(ink: ImageInk) -> InkRun {
    InkRun {
        inks: Inks(vec![Ink::Image(ink)]),
        image: Some(photo()),
        view: VIEW,
        ..InkRun::default()
    }
}

#[test]
fn acceptance_25_image_ink_reveals_the_photo_follows_the_widget_and_repeat_off_stops_at_its_edge() {
    let (e, image) = plane();
    // Centred, fitted to the view's height: 800 wide, so x 160 is the red
    // half and x 240 the green half.
    let mut st = stroke(
        &e,
        image,
        view(),
        settings(20.),
        image_ink(ImageInk::default()),
    );
    for x in [160., 240.] {
        st.dab(DVec2::new(x, 200.), 20., None, 1.);
    }
    let left = colour_at(&mut st, DVec2::new(160., 200.));
    let right = colour_at(&mut st, DVec2::new(240., 200.));
    assert!(left[0] > 0.9 && left[1] < 0.1, "{left:?}");
    assert!(right[1] > 0.9 && right[0] < 0.1, "{right:?}");

    // The widget moved 100 px right: x 240 now shows the red half.
    let moved = ImageInk {
        pos: Some(DVec2::new(300., 200.)),
        ..ImageInk::default()
    };
    let mut st = stroke(&e, image, view(), settings(20.), image_ink(moved));
    st.dab(DVec2::new(240., 200.), 20., None, 1.);
    let c = colour_at(&mut st, DVec2::new(240., 200.));
    assert!(c[0] > 0.9 && c[1] < 0.1, "{c:?}");

    // A quarter size (100 px tall, 200 wide) with Repeat off: nothing lands
    // outside it, the inside still paints.
    let small = ImageInk {
        scale: 0.25,
        repeat: false,
        ..ImageInk::default()
    };
    let mut st = stroke(&e, image, view(), settings(10.), image_ink(small));
    st.dab(DVec2::new(200., 120.), 10., None, 1.);
    st.dab(DVec2::new(150., 200.), 10., None, 1.);
    assert_eq!(
        colour_at(&mut st, DVec2::new(200., 120.))[0..3],
        [0., 0., 0.]
    );
    assert!(colour_at(&mut st, DVec2::new(150., 200.))[0] > 0.9);
    // With Repeat on the same dab tiles the picture.
    let tiled = ImageInk {
        scale: 0.25,
        ..ImageInk::default()
    };
    let mut st = stroke(&e, image, view(), settings(10.), image_ink(tiled));
    st.dab(DVec2::new(200., 120.), 10., None, 1.);
    assert!(colour_at(&mut st, DVec2::new(200., 120.))[0..3] != [0., 0., 0.]);
}

#[test]
fn image_ink_stamp_lays_the_whole_picture_in_one_go() {
    let (e, image) = plane();
    let mut st = stroke(
        &e,
        image,
        view(),
        settings(10.),
        image_ink(ImageInk {
            stamp: true,
            ..ImageInk::default()
        }),
    );
    st.ink_stamp(None);
    for (x, red) in [(130., true), (180., true), (220., false), (270., false)] {
        let c = colour_at(&mut st, DVec2::new(x, 200.));
        assert_eq!(c[0] > 0.9, red, "{x}: {c:?}");
    }
}

#[test]
fn acceptance_26_parametric_slope_paints_the_top_red_and_the_sides_blue() {
    let (e, image) = textured(None);
    let mut s = settings(400.);
    s.fg.color = [1., 0., 0.];
    s.bg.color = [0., 0., 1.];
    let inks = InkRun {
        inks: Inks(vec![Ink::Parametric(ParametricInk {
            parameter: Parameter::Slope,
            use_as_mask: false,
            ..ParametricInk::default()
        })]),
        ..InkRun::default()
    };
    let v = PaintView::looking(DVec3::new(0., 0., 4.), DVec3::ZERO, VIEW);
    let mut st = stroke(&e, image, v, s, inks);
    st.dab(DVec2::new(200., 200.), 400., None, 1.);
    let r = e
        .scene
        .active()
        .unwrap()
        .mesh
        .vertices
        .iter()
        .map(|p| p.length())
        .fold(0., f64::max);
    let at = |p: DVec3| v.project(p).unwrap().0;
    // Near the top (10° from straight up) and on the side facing us.
    let top = colour_at(&mut st, at(DVec3::new(0., r * 0.98, r * 0.17)));
    let side = colour_at(&mut st, at(DVec3::new(0., 0., r)));
    assert!(top[0] > 0.8 && top[2] < 0.2, "top {top:?}");
    assert!(side[2] > 0.8 && side[0] < 0.2, "side {side:?}");
}

#[test]
fn parametric_use_as_mask_paints_less_where_the_parameter_is_high() {
    let (e, image) = textured(None);
    let inks = InkRun {
        inks: Inks(vec![Ink::Parametric(ParametricInk::default())]),
        ..InkRun::default()
    };
    let v = PaintView::looking(DVec3::new(0., 0., 4.), DVec3::ZERO, VIEW);
    let mut st = stroke(&e, image, v, settings(400.), inks);
    st.dab(DVec2::new(200., 200.), 400., None, 1.);
    let r = e
        .scene
        .active()
        .unwrap()
        .mesh
        .vertices
        .iter()
        .map(|p| p.length())
        .fold(0., f64::max);
    let at = |p: DVec3| v.project(p).unwrap().0;
    // Incidence Angle: facing full, edge-on nearly nothing.
    let facing = colour_at(&mut st, at(DVec3::new(0., 0., r)))[0];
    let edge = colour_at(&mut st, at(DVec3::new(r * 0.97, 0., r * 0.24)))[0];
    assert!(facing > 0.9, "{facing}");
    assert!(edge < facing * 0.5, "{edge} vs {facing}");
}

#[test]
fn acceptance_27_random_ink_brightness_varies_each_dab() {
    let (e, image) = plane();
    let mut s = settings(12.);
    s.fg.color = [0.5, 0.5, 0.5];
    let inks = InkRun {
        inks: Inks(vec![Ink::Random(RandomInk {
            brightness: 0.3,
            ..RandomInk::default()
        })]),
        seed: 7,
        ..InkRun::default()
    };
    let mut st = stroke(&e, image, view(), s, inks);
    let spots: Vec<DVec2> = (0..8)
        .map(|k| DVec2::new(110. + f64::from(k) * 25., 200.))
        .collect();
    for &q in &spots {
        st.dab(q, 8., None, 1.);
    }
    let values: Vec<f32> = spots.iter().map(|&q| colour_at(&mut st, q)[0]).collect();
    let (lo, hi) = values
        .iter()
        .fold((1f32, 0f32), |(a, b), &v| (a.min(v), b.max(v)));
    assert!(hi - lo > 0.05, "{values:?}");
    for v in &values {
        assert!(
            (0.5 * 0.7 - 0.01..=0.5 * 1.3 + 0.01).contains(v),
            "{values:?}"
        );
    }
    let mut distinct = values.clone();
    distinct.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
    assert!(distinct.len() >= 6, "{values:?}");
}

#[test]
fn projection_ink_paints_from_the_camera_picture() {
    let (mut e, image) = plane();
    let mesh_item = e.scene.selection.item.unwrap();
    e.dispatch(Command::NewMesh).unwrap();
    let cam = e.scene.selection.item.unwrap();
    {
        let it = e.scene.item_mut(cam).unwrap();
        let mut c = camera_light::CameraSettings::default();
        c.background.image = Some(999);
        it.camera = Some(c);
        it.transform.translation = DVec3::new(0., 0., 3.);
    }
    let src = CameraSource::of(&e.scene, cam, |id| (id == 999).then(photo)).unwrap();
    e.scene.selection.item = Some(mesh_item);
    let inks = InkRun {
        inks: Inks(vec![Ink::Projection(ProjectionInk::default())]),
        camera: Some(src),
        ..InkRun::default()
    };
    let mut st = stroke(&e, image, view(), settings(10.), inks);
    let (l, r) = (
        view().project(DVec3::new(-0.3, 0., 0.)).unwrap().0,
        view().project(DVec3::new(0.3, 0., 0.)).unwrap().0,
    );
    st.dab(l, 8., None, 1.);
    st.dab(r, 8., None, 1.);
    let (lc, rc) = (colour_at(&mut st, l), colour_at(&mut st, r));
    assert!(lc[0] > 0.9 && lc[1] < 0.1, "{lc:?}");
    assert!(rc[1] > 0.9 && rc[0] < 0.1, "{rc:?}");
    // No background image: refused with the spec's reason.
    e.scene
        .item_mut(cam)
        .unwrap()
        .camera
        .as_mut()
        .unwrap()
        .background
        .image = None;
    assert_eq!(
        CameraSource::of(&e.scene, cam, |_| None).unwrap_err(),
        NO_CAMERA_IMAGE
    );
}

/// A background mesh holding one straight curve along y = `y` at z = 0.
fn add_curve(e: &mut Editor, y: f64) -> u64 {
    let painted = e.scene.selection.item.unwrap();
    e.dispatch(Command::NewMesh).unwrap();
    let id = e.scene.selection.item.unwrap();
    let mut m = Mesh {
        vertices: (0..5)
            .map(|k| DVec3::new(-0.8 + f64::from(k) * 0.4, y, 0.))
            .collect(),
        ..Mesh::default()
    };
    m.faces = vec![Face {
        vertices: vec![0, 1, 2, 3, 4],
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }]
    .into();
    m.attributes.representations = vec![PolygonRepresentation::Polyline {
        closed: false,
        curve: Default::default(),
    }];
    e.scene.active_mut().unwrap().mesh = m.into();
    e.scene.selection.item = Some(painted);
    e.scene.selection.also.clear();
    id
}

#[test]
fn acceptance_28_background_2d_a_stroke_near_a_background_curve_follows_it() {
    let (mut e, image) = plane();
    add_curve(&mut e, 0.3);
    let guide = CurveGuide::background(&e.scene);
    assert_eq!(guide.curves.len(), 1);
    let v = view();
    let curve_y = v.project(DVec3::new(0., 0.3, 0.)).unwrap().0.y;
    let mut st = stroke(&e, image, v, settings(4.), InkRun::default());
    // A wobbly stroke 10 px either side of the curve.
    for k in 0..30 {
        let q = DVec2::new(
            130. + f64::from(k) * 5.,
            curve_y + if k % 2 == 0 { 10. } else { -10. },
        );
        let snapped = guide.snap_screen(&v, q).unwrap();
        assert!((snapped.y - curve_y).abs() < 1e-6 && (snapped.x - q.x).abs() < 1e-6);
        st.dab(snapped, 3., None, 1.);
    }
    let on = colour_at(&mut st, DVec2::new(200., curve_y))[0];
    let off = colour_at(&mut st, DVec2::new(200., curve_y + 10.))[0];
    assert!(on > 0.9 && off < 0.01, "on {on} off {off}");
    // A selected or hidden curve isn't a background curve.
    let id = e.scene.items.last().unwrap().id;
    e.scene.selection.also.push(id);
    assert!(CurveGuide::background(&e.scene).is_empty());
    e.scene.selection.also.clear();
    e.scene.item_mut(id).unwrap().visible = false;
    assert!(CurveGuide::background(&e.scene).is_empty());
}

#[test]
fn background_3d_only_reaches_surface_within_the_brush_of_the_curve() {
    let (mut e, image) = plane();
    add_curve(&mut e, 0.3);
    let guide = CurveGuide::background(&e.scene);
    let v = view();
    let p = guide.snap_world(DVec3::new(0.1, 0.5, 0.2)).unwrap();
    assert!((p - DVec3::new(0.1, 0.3, 0.)).length() < 1e-9);
    let mut st = stroke(&e, image, v, settings(40.), InkRun::default());
    let q = v.project(p).unwrap().0;
    st.dab_near(q, 40., p, 0.05, 1.);
    assert!(colour_at(&mut st, q)[0] > 0.9);
    // 0.1 world units away is inside the 40 px tip but beyond the reach.
    let far = v.project(p + DVec3::new(0., 0.1, 0.)).unwrap().0;
    assert!(far.distance(q) < 30.);
    assert_eq!(colour_at(&mut st, far)[0], 0.);
}

#[test]
fn inks_toggle_into_the_stack_in_order() {
    let mut k = Inks::default();
    assert!(k.toggle(InkKind::Random));
    assert!(k.toggle(InkKind::Image));
    assert_eq!(
        k.0.iter().map(|i| i.kind()).collect::<Vec<_>>(),
        [InkKind::Random, InkKind::Image]
    );
    assert!(!k.toggle(InkKind::Random));
    assert_eq!(k.0.len(), 1);
    assert_eq!(InkKind::named("Projection"), Some(InkKind::Projection));
    assert_eq!(
        Parameter::named("incidence angle"),
        Some(Parameter::IncidenceAngle)
    );
    assert_eq!(
        CurveConstraint::named("background 2D"),
        Some(CurveConstraint::Background2D)
    );
}
