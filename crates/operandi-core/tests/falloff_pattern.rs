//! Falloff batch 5 (falloff spec 2026-11-15): Noise (FO-50), Vertex Map
//! colour and morph maps (FO-49) and Image (FO-54). Original test data.
use glam::{DVec2, DVec3, DVec4};
use operandi_core::falloff_pattern::{self as fp, GrayImage};
use operandi_core::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// A unit quad: points 0..4 at the corners, one polygon.
fn quad() -> Mesh {
    Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(0., 1., 0.),
        ],
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    }
}

/// FO-50: measured default Scale 1 m; repeatable, smooth, 0..1, 0.5 on
/// every lattice point; Scale stretches it.
#[test]
fn noise_is_repeatable_and_bounded() {
    let n = NoiseFalloff::default();
    assert_eq!((n.scale, n.world), (1., false));
    assert!(close(n.weight(DVec3::new(3., -2., 7.)), 0.5));
    let p = DVec3::new(0.37, 1.21, -2.6);
    assert_eq!(n.weight(p), n.weight(p));
    let mut spread = (1f64, 0f64);
    for i in 0..500 {
        let q = DVec3::new(i as f64 * 0.173, i as f64 * 0.071, i as f64 * 0.029);
        let w = n.weight(q);
        assert!((0. ..=1.).contains(&w));
        spread = (spread.0.min(w), spread.1.max(w));
    }
    assert!(spread.1 - spread.0 > 0.4, "{spread:?}");
    // Nearby points weigh nearly the same (smooth).
    assert!((n.weight(p) - n.weight(p + DVec3::splat(1e-4))).abs() < 1e-3);
    let big = NoiseFalloff { scale: 2., ..n };
    assert!(close(big.weight(p * 2.), n.weight(p)));
    assert!(NoiseFalloff { scale: 0., ..n }.validate().is_err());
}

/// FO-49: a morph map reads offset lengths (Magnitude) or one axis
/// (Component); a colour map reads brightness or one channel.
#[test]
fn vertex_map_reads_morph_and_colour_maps() {
    let mut mesh = quad();
    mesh.attributes
        .relative_morphs
        .insert("Bulge".into(), [(1, DVec3::new(0., 3., 4.))].into());
    mesh.attributes.corner_colors.insert(
        "Paint".into(),
        vec![vec![
            Some(DVec4::new(0.3, 0.6, 0.9, 1.)),
            None,
            Some(DVec4::new(1., 1., 1., 1.)),
            None,
        ]],
    );
    let mag = VertexMapFalloff::default();
    let comp = |index| VertexMapFalloff {
        mode: VertexMapMode::Component,
        index,
    };
    assert_eq!(
        fp::vertex_map_values(&mesh, "Bulge", mag).unwrap(),
        [0., 5., 0., 0.]
    );
    assert_eq!(
        fp::vertex_map_values(&mesh, "Bulge", comp(2)).unwrap(),
        [0., 4., 0., 0.]
    );
    let c = fp::vertex_map_values(&mesh, "Paint", mag).unwrap();
    assert!(close(c[0], 0.6) && close(c[2], 1.) && c[1] == 0.);
    let g = fp::vertex_map_values(&mesh, "Paint", comp(1)).unwrap();
    assert!(close(g[0], 0.6));
    // A weight map keeps its own path.
    mesh.attributes
        .point_weights
        .insert("W".into(), [(0, 0.5)].into());
    assert!(fp::vertex_map_values(&mesh, "W", mag).is_none());
}

/// The stack reads the selected morph map through the Vertex Map falloff.
#[test]
fn the_stack_reads_a_selected_morph_map() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut mesh = quad();
    mesh.attributes
        .relative_morphs
        .insert("Bulge".into(), [(2, DVec3::new(0.5, 0., 0.))].into());
    e.scene.active_mut().unwrap().mesh = mesh.into();
    let id = e.scene.active().unwrap().id;
    let mut stack = ActiveFalloff::new(Falloff::VertexMap(Default::default()));
    stack.weight_map = Some("Bulge".into());
    assert_eq!(stack.weights(&e.scene, id).unwrap(), [0., 0., 0.5, 0.]);
}

/// FO-54: brightness with Pos, Scale, Rotation, Repeat and Soft Border;
/// through a UV map with Use Selected UV Map.
#[test]
fn image_weights_follow_the_picture() {
    let d = ImageFalloff::default();
    assert_eq!(
        (
            d.repeat,
            d.soft_border,
            d.use_uv,
            d.scale,
            d.pos,
            d.rotation,
            d.world
        ),
        (true, 5., false, 100., DVec2::ZERO, 0., false)
    );
    // Left half black, right half white.
    let img = GrayImage {
        width: 4,
        height: 2,
        pixels: vec![0., 0., 1., 1., 0., 0., 1., 1.],
    };
    let plain = ImageFalloff {
        soft_border: 0.,
        ..d
    };
    assert_eq!(plain.weight_at(&img, DVec2::new(0.1, 0.5)), 0.);
    assert_eq!(plain.weight_at(&img, DVec2::new(0.9, 0.5)), 1.);
    // Turned half a turn, the sides swap.
    let turned = ImageFalloff {
        rotation: 180.,
        ..plain
    };
    assert!(close(turned.weight_at(&img, DVec2::new(0.1, 0.5)), 1.));
    // Repeat off: outside the picture weighs 0; on, it tiles.
    let small = ImageFalloff {
        scale: 50.,
        ..plain
    };
    assert_eq!(
        ImageFalloff {
            repeat: false,
            ..small
        }
        .weight_at(&img, DVec2::new(0.15, 0.5)),
        0.
    );
    assert_eq!(small.weight_at(&img, DVec2::new(0.15, 0.5)), 1.);
    // Soft Border fades the edge.
    let soft = ImageFalloff {
        soft_border: 50.,
        ..plain
    };
    assert!(soft.weight_at(&img, DVec2::new(0.99, 0.5)) < 0.2);
    // Through UVs: corner 1's UV (1, 0) is the bottom-right of the picture.
    let mut mesh = quad();
    mesh.uv_maps = std::collections::BTreeMap::from([(
        "Texture".to_owned(),
        vec![vec![
            Some(DVec2::new(0., 0.)),
            Some(DVec2::new(1., 0.)),
            Some(DVec2::new(1., 1.)),
            Some(DVec2::new(0., 1.)),
        ]],
    )])
    .into();
    let edge = ImageFalloff {
        repeat: false,
        ..plain
    };
    let w = fp::image_uv_weights(&mesh, "Texture", &img, edge);
    assert_eq!(w, [0., 1., 1., 0.]);
}

#[test]
fn commands_take_their_options() {
    let parsed = |line: &str| match command_line::parse_sculpt(line) {
        Some(command_line::SculptLine::Falloff { falloff, .. }) => falloff,
        other => panic!("{line}: {other:?}"),
    };
    assert_eq!(
        parsed("falloff.noise scale:0.25"),
        Falloff::Noise(NoiseFalloff {
            scale: 0.25,
            ..Default::default()
        })
    );
    assert_eq!(
        parsed("falloff.image useUV:true rotation:90"),
        Falloff::Image(ImageFalloff {
            use_uv: true,
            rotation: 90.,
            ..Default::default()
        })
    );
}
