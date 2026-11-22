//! MaterialX stage 5: graph layers drawn from their nodes in the viewport.
// Cycles' reference values are kept as it printed them (9 digits).
#![allow(clippy::excessive_precision)]
use super::*;
use crate::cycles_noise as noise;
use crate::node_graph::{FlatLink, FlatNode};
use crate::shader_eval::{Ch, Expr, ShadingPoint};
use crate::shader_func::{Func, MathOp, VectorOp};
use std::path::Path;

/// Values printed by Cycles' own kernel code (src/util/hash.h,
/// src/kernel/svm/noisetex.h and white_noise.h at the pinned commit,
/// compiled on the CPU for these inputs): the port must give the same.
#[test]
fn procedural_textures_match_cycles() {
    assert_eq!(noise::hash(&[1, 2, 3]), 3100736010);
    assert_eq!(noise::hash(&[1, 2, 3, 4]), 92593857);
    // f32 rounding apart (sums done in a slightly different order).
    let close = |a: f32, b: f32| (a - b).abs() < 1e-5;
    assert!(close(noise::snoise(1, [0.37, 0., 0., 0.]), 0.364753783));
    assert!(close(noise::snoise(2, [0.37, -1.2, 0., 0.]), -0.268093646));
    assert!(close(noise::snoise(3, [0.3, 1.7, -2.2, 0.]), 0.260110289));
    assert!(close(noise::snoise(4, [0.3, 1.7, -2.2, 0.9]), -0.300285488));
    let fr = |detail, roughness, offset| noise::Fractal {
        detail,
        roughness,
        lacunarity: 2.,
        offset,
        gain: 1.,
    };
    let v = [0.3, 1.7, -2.2];
    let n3 = noise::noise_texture(
        3,
        noise::NoiseKind::Fbm,
        true,
        true,
        v,
        0.,
        1.5,
        fr(2.5, 0.5, 0.),
        0.3,
    );
    for (a, b) in n3.iter().zip([0.544527292, 0.548702717, 0.466598809]) {
        assert!(close(*a, b), "{n3:?}");
    }
    let n2 = noise::noise_texture(
        2,
        noise::NoiseKind::RidgedMultiFractal,
        true,
        true,
        v,
        0.,
        1.,
        fr(3., 0.6, 0.2),
        0.,
    );
    for (a, b) in n2.iter().zip([0.20145613, 0.0250971392, 0.124579854]) {
        assert!(close(*a, b), "{n2:?}");
    }
    let n4 = noise::noise_texture(
        4,
        noise::NoiseKind::HybridMultiFractal,
        true,
        false,
        v,
        0.4,
        1.,
        fr(1., 0.5, 0.5),
        0.,
    );
    assert!(close(n4[0], 0.719441116), "{n4:?}");
    let n1 = noise::noise_texture(
        1,
        noise::NoiseKind::HeteroTerrain,
        true,
        false,
        v,
        0.77,
        1.,
        fr(2., 0.5, 0.5),
        0.1,
    );
    assert!(close(n1[0], 0.925270915), "{n1:?}");
    let m = noise::noise_texture(
        3,
        noise::NoiseKind::MultiFractal,
        true,
        false,
        v,
        0.,
        1.,
        fr(4.3, 0.5, 0.),
        0.,
    );
    assert!(close(m[0], 1.42733383), "{m:?}");
    let w3 = noise::white_noise(3, true, v, 0.);
    for (a, b) in w3.iter().zip([0.00489179324, 0.45795995, 0.258489609]) {
        assert!(close(*a, b), "{w3:?}");
    }
    let w4 = noise::white_noise(4, true, v, 0.5);
    for (a, b) in w4.iter().zip([0.893423855, 0.128302231, 0.769463241]) {
        assert!(close(*a, b), "{w4:?}");
    }
}

/// Cycles' Voronoi (src/kernel/svm/voronoi.h at the pinned commit, run on
/// the CPU with the node's own set-up) at (0.3, 1.7, −2.2): Distance,
/// Color, Position (and W for 1D/4D) per case.
#[test]
fn voronoi_matches_cycles() {
    use crate::cycles_voronoi::{
        Voronoi, VoronoiFeature as F, VoronoiMetric as M, VoronoiOut as O,
    };
    let close = |a: f32, b: f32| (a - b).abs() < 1e-5;
    #[allow(clippy::too_many_arguments)]
    let mk = |dims, feature, metric, normalize, scale, detail, roughness, exponent, randomness| {
        Voronoi {
            dims,
            feature,
            metric,
            normalize,
            scale,
            detail,
            roughness,
            lacunarity: 2.,
            smoothness: 1.,
            exponent,
            randomness,
        }
    };
    let v = [0.3, 1.7, -2.2];
    // (case, w, distance, colour, position xyz, w or None)
    type Case = (Voronoi, f32, f32, [f32; 3], [f32; 3], Option<f32>);
    let cases: [Case; 7] = [
        (
            mk(3, F::F1, M::Euclidean, false, 5., 0., 0.5, 0.5, 1.),
            0.,
            0.348902643,
            [0.527274132, 0.15260312, 0.982547641],
            [0.30545482, 1.6305207, -2.2034905],
            None,
        ),
        (
            mk(2, F::F2, M::Manhattan, true, 3., 0., 0.5, 0.5, 0.8),
            0.,
            0.256712765,
            [0.881525755, 0.743581653, 0.955568731],
            [0.356949747, 1.4488945, 0.],
            None,
        ),
        (
            mk(3, F::SmoothF1, M::Chebychev, false, 2., 0., 0.5, 0.5, 1.),
            0.,
            0.281765312,
            [0.424765915, 0.581967533, 0.543033361],
            [0.257486194, 1.69911504, -2.28948569],
            None,
        ),
        (
            mk(4, F::F1, M::Minkowski, false, 2., 0., 0.5, 1.5, 1.),
            0.6,
            0.64276737,
            [0.401699901, 0.320088893, 0.551918685],
            [0.20084995, 1.66004443, -2.22404075],
            Some(0.331330925),
        ),
        (
            mk(1, F::F1, M::Euclidean, false, 5., 0., 0.5, 0.5, 1.),
            0.77,
            0.0562672019,
            [0.793732703, 0.627682149, 0.934084058],
            [0.; 3],
            Some(0.758746564),
        ),
        (
            mk(1, F::SmoothF1, M::Euclidean, true, 5., 0., 0.5, 0.5, 1.),
            0.77,
            -0.023328416,
            [0.46813646, 0.632920623, 0.692358911],
            [0.; 3],
            Some(0.76453805),
        ),
        (
            mk(3, F::F1, M::Euclidean, true, 4., 2.5, 0.6, 0.5, 0.9),
            0.,
            0.24152647,
            [0.459925145, 0.499423563, 0.518804073],
            [0.305702567, 1.67632103, -2.19138718],
            None,
        ),
    ];
    for (i, (vo, w, d, c, p, pw)) in cases.iter().enumerate() {
        let got = |o| vo.eval(o, v, *w);
        assert!(
            close(got(O::Distance)[0], *d),
            "case {i}: {:?}",
            got(O::Distance)
        );
        for k in 0..3 {
            assert!(
                close(got(O::Color)[k], c[k]),
                "case {i}: {:?}",
                got(O::Color)
            );
            assert!(
                close(got(O::Position)[k], p[k]),
                "case {i}: {:?}",
                got(O::Position)
            );
        }
        if let Some(pw) = pw {
            assert!(close(got(O::W)[0], *pw), "case {i}: {:?}", got(O::W));
        }
    }
    let edge = mk(
        3,
        F::DistanceToEdge,
        M::Euclidean,
        false,
        5.,
        1.5,
        0.5,
        0.5,
        1.,
    );
    assert!(close(edge.eval(O::Distance, v, 0.)[0], 0.115509182));
    let edge1 = mk(
        1,
        F::DistanceToEdge,
        M::Euclidean,
        true,
        5.,
        0.,
        0.5,
        0.5,
        1.,
    );
    assert!(close(edge1.eval(O::Distance, v, 0.77)[0], 0.0491139889));
    let ns = mk(
        2,
        F::NSphereRadius,
        M::Euclidean,
        false,
        5.,
        0.,
        0.5,
        0.5,
        1.,
    );
    assert!(close(ns.eval(O::Radius, v, 0.)[0], 0.229724914));
    let ns1 = mk(
        1,
        F::NSphereRadius,
        M::Euclidean,
        false,
        5.,
        0.,
        0.5,
        0.5,
        1.,
    );
    assert!(close(ns1.eval(O::Radius, v, 0.77)[0], 0.105381161));
}

#[test]
fn node_sums_follow_cycles_safe_rules() {
    let m = |op, a, b, c| MathOp::apply(op, a, b, c);
    assert_eq!(m(MathOp::Divide, 1., 0., 0.), 0.);
    assert_eq!(m(MathOp::Power, -2., 0.5, 0.), 0.);
    assert_eq!(m(MathOp::Power, -2., 3., 0.), -8.);
    assert_eq!(m(MathOp::Logarithm, -1., 2., 0.), 0.);
    assert_eq!(m(MathOp::Wrap, 5.5, 2., 0.), 1.5);
    assert_eq!(m(MathOp::PingPong, 3., 2., 0.), 1.);
    assert_eq!(m(MathOp::Compare, 1., 1.05, 0.1), 1.);
    assert_eq!(m(MathOp::Arctan2, 0., 0., 0.), 0.);
    assert!((m(MathOp::SmoothMin, 1., 1., 1.) - (1. - 1. / 6.)).abs() < 1e-12);
    let v = |op, a, b| VectorOp::apply(op, a, b, [0.; 3], 1.);
    assert_eq!(
        v(VectorOp::CrossProduct, [1., 0., 0.], [0., 1., 0.]),
        [0., 0., 1.]
    );
    assert_eq!(
        v(VectorOp::DotProduct, [1., 2., 3.], [4., 5., 6.]),
        [32.; 3]
    );
    assert_eq!(v(VectorOp::Project, [1., 1., 0.], [0., 0., 0.]), [0.; 3]);
    assert_eq!(v(VectorOp::Normalize, [0.; 3], [0.; 3]), [0.; 3]);
    // Mapping (point): scale, rotate 90° about Z, then move.
    let p = Func::Mapping(crate::shader_func::MappingKind::Point).eval(&[
        [1., 0., 0.],
        [0., 0., 1.],
        [0., 0., std::f64::consts::FRAC_PI_2],
        [2., 2., 2.],
    ]);
    assert!((p[0]).abs() < 1e-12 && (p[1] - 2.).abs() < 1e-12 && (p[2] - 1.).abs() < 1e-12);
    // Head-on, Fresnel at index 1.5 is 4 %.
    let f = Func::Fresnel.eval(&[[1.5; 3], [0.; 3]])[0];
    assert!((f - 0.04).abs() < 1e-9, "{f}");
    // Constant arguments fold.
    let e = Expr::func(
        Func::Math {
            op: MathOp::Add,
            clamp: true,
        },
        vec![Expr::scalar(0.7), Expr::scalar(0.6), Expr::scalar(0.)],
    );
    assert_eq!(e, Expr::scalar(1.));
}

fn node(cycles: &'static str, values: Vec<(&str, FlatValue)>) -> FlatNode {
    FlatNode {
        cycles,
        name: cycles.into(),
        values: values
            .into_iter()
            .map(|(s, v)| (s.to_string(), v))
            .collect(),
    }
}
fn link(from: usize, fs: &str, to: usize, ts: &str) -> FlatLink {
    FlatLink {
        from,
        from_socket: fs.into(),
        to,
        to_socket: ts.into(),
    }
}
fn channel(v: &ViewGraph, ch: Ch) -> &Expr {
    &v.channels.iter().find(|(c, _)| *c == ch).unwrap().1
}
fn at(p: [f64; 3]) -> ShadingPoint {
    ShadingPoint {
        position: p,
        uv: [p[0], p[1]],
        normal: [0., 0., 1.],
    }
}
fn no_images(_: &str) -> Option<u64> {
    None
}

/// Noise ▸ Color Ramp ▸ Base Color, a Math node on Roughness, a constant
/// Metallic (NM-T1's graph).
fn noise_ramp_graph() -> FlatGraph {
    let table: Vec<[f64; 4]> = (0..256)
        .map(|i| {
            let t = i as f64 / 255.;
            [t, 0., 1. - t, 1.]
        })
        .collect();
    FlatGraph {
        nodes: vec![
            node("output", vec![]),
            node("principled_bsdf", vec![("Metallic", FlatValue::Float(1.))]),
            node("noise_texture", vec![("Scale", FlatValue::Float(4.))]),
            node("rgb_ramp", vec![("Ramp", FlatValue::Ramp(table))]),
            node(
                "math",
                vec![
                    ("Type", FlatValue::Text("multiply".into())),
                    ("Value2", FlatValue::Float(0.5)),
                ],
            ),
        ],
        links: vec![
            link(1, "BSDF", 0, "Surface"),
            link(2, "Fac", 3, "Fac"),
            link(3, "Color", 1, "Base Color"),
            link(2, "Fac", 4, "Value1"),
            link(4, "Value", 1, "Roughness"),
        ],
    }
}

#[test]
fn a_principled_graph_draws_its_inputs() {
    let g = noise_ramp_graph();
    let v = surface(&g, &[], &GraphSpace::default(), &mut no_images).unwrap();
    assert!(v.missing.is_empty(), "{:?}", v.missing);
    assert_eq!(*channel(&v, Ch::Metallic), Expr::scalar(1.));
    assert_eq!(*channel(&v, Ch::DiffuseAmount), Expr::scalar(1.));
    let base = channel(&v, Ch::DiffuseColor);
    let rough = channel(&v, Ch::Roughness);
    assert!(base.varies_by_pattern() && rough.varies_by_pattern());
    // The ramp runs red to blue by the noise, and Roughness is half of it,
    // as Cycles sums them at the same Generated point.
    for p in [[0.1, 0.2, 0.3], [0.7, 0.4, 0.9], [0.5, 0.5, 0.5]] {
        let fac = noise::noise_texture(
            3,
            noise::NoiseKind::Fbm,
            true,
            false,
            p.map(|c| c as f32),
            0.,
            4.,
            noise::Fractal {
                detail: 2.,
                roughness: 0.5,
                lacunarity: 2.,
                offset: 0.,
                gain: 1.,
            },
            0.,
        )[0] as f64;
        let c = base.eval_at(&at(p), &|_, _| [0.; 4]);
        assert!(
            (c[0] - fac).abs() < 1e-2 && (c[0] + c[2] - 1.).abs() < 1e-9,
            "{c:?} {fac}"
        );
        let r = rough.eval_at(&at(p), &|_, _| [0.; 4])[0];
        assert!((r - fac * 0.5).abs() < 1e-6, "{r} {fac}");
    }
    // The GPU program for it builds (per pixel, no images).
    let p = crate::shader_wgsl::translate(base, &mut |_| None).unwrap();
    assert!(p.per_pixel() && p.body.contains("op_g_noise"));
}

#[test]
fn unknown_parts_fall_back_input_by_input() {
    let mut g = noise_ramp_graph();
    // A Magic texture isn't drawn: Base Color takes its own value instead.
    g.nodes[3] = node("magic_texture", vec![]);
    g.links[2] = link(3, "Color", 1, "Base Color");
    g.nodes[1]
        .values
        .push(("Base Color".into(), FlatValue::Vec3([0.1, 0.2, 0.3])));
    let v = surface(&g, &[], &GraphSpace::default(), &mut no_images).unwrap();
    assert_eq!(*channel(&v, Ch::DiffuseColor), Expr::Const([0.1, 0.2, 0.3]));
    assert!(
        v.missing.iter().any(|m| m.contains("Base Color")),
        "{:?}",
        v.missing
    );
    // A surface closure this can't draw: the caller's stand-in colour.
    g.nodes[1] = node("toon_bsdf", vec![]);
    assert!(surface(&g, &[], &GraphSpace::default(), &mut no_images).is_none());
    // Nothing feeding the output at all.
    g.links.clear();
    assert!(surface(&g, &[], &GraphSpace::default(), &mut no_images).is_none());
}

#[test]
fn mixed_and_emitting_closures_blend_their_channels() {
    let g = FlatGraph {
        nodes: vec![
            node("output", vec![]),
            node("mix_closure", vec![("Fac", FlatValue::Float(0.25))]),
            node(
                "principled_bsdf",
                vec![("Base Color", FlatValue::Vec3([1., 0., 0.]))],
            ),
            node(
                "emission",
                vec![
                    ("Color", FlatValue::Vec3([0., 1., 0.])),
                    ("Strength", FlatValue::Float(4.)),
                ],
            ),
        ],
        links: vec![
            link(1, "Closure", 0, "Surface"),
            link(2, "BSDF", 1, "Closure1"),
            link(3, "Emission", 1, "Closure2"),
        ],
    };
    let v = surface(&g, &[], &GraphSpace::default(), &mut no_images).unwrap();
    assert_eq!(*channel(&v, Ch::DiffuseColor), Expr::Const([0.75, 0., 0.]));
    assert_eq!(*channel(&v, Ch::LuminousIntensity), Expr::scalar(1.));
    let lum = channel(&v, Ch::LuminousColor).constant().unwrap();
    assert!((lum[0] - 0.75).abs() < 1e-12 && (lum[1] - 1.).abs() < 1e-12);
}

#[test]
fn generated_coordinates_span_the_mesh_box() {
    let m = GraphSpace::generated_for([-1., 0., 2.], [3., 0., 2.5]);
    let apply = |p: [f64; 3]| {
        [0, 1, 2]
            .map(|r| m[r * 4] * p[0] + m[r * 4 + 1] * p[1] + m[r * 4 + 2] * p[2] + m[r * 4 + 3])
    };
    assert_eq!(apply([-1., 0., 2.]), [0., 0.5, 0.]);
    assert_eq!(apply([3., 0., 2.5]), [1., 0.5, 1.]);
    // A flat axis spans 2 m round its centre, as renders give it.
    assert_eq!(apply([1., 1., 2.25]), [0.5, 1., 0.5]);
}

#[test]
fn driven_inputs_and_pictures_read_their_sources() {
    let g = FlatGraph {
        nodes: vec![
            node("output", vec![]),
            node("principled_bsdf", vec![("Roughness", FlatValue::Driven(0))]),
            node(
                "image_texture",
                vec![
                    ("Filename", FlatValue::Text("/p/rough.png".into())),
                    ("Colorspace", FlatValue::Text("data".into())),
                ],
            ),
        ],
        links: vec![
            link(1, "BSDF", 0, "Surface"),
            link(2, "Color", 1, "Base Color"),
            link(2, "Alpha", 1, "Alpha"),
        ],
    };
    let driver = Expr::Locator(Box::new(GraphSpace::default().generated));
    let mut asked = Vec::new();
    let v = surface(
        &g,
        std::slice::from_ref(&driver),
        &GraphSpace::default(),
        &mut |p| {
            asked.push(p.to_string());
            Some(7)
        },
    )
    .unwrap();
    asked.dedup();
    assert_eq!(asked, ["/p/rough.png"]);
    // A driven float reads the layer value's luminance.
    assert_eq!(*channel(&v, Ch::Roughness), Expr::luma(driver));
    // A data picture gets its numbers back from the viewport's sRGB decode.
    let mut ids = Vec::new();
    channel(&v, Ch::DiffuseColor).images(&mut ids);
    assert_eq!(ids, [7]);
    assert!(matches!(channel(&v, Ch::DiffuseColor), Expr::Func(c) if c.func == Func::SrgbEncode));
    // Alpha 0.25 dissolves 75 %.
    let d = channel(&v, Ch::Dissolve).eval_at(&at([0.; 3]), &|_, _| [0., 0., 0., 0.25]);
    assert!((d[0] - 0.75).abs() < 1e-12);
}

/// MaterialX's own example materials: every graph layer among them draws
/// from its nodes (not the single stand-in colour), with finite values
/// and GPU code the viewport can build.
#[test]
fn materialx_example_graphs_draw_from_their_nodes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/materialx-examples");
    let mut paths = Vec::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "mtlx") {
                paths.push(p);
            }
        }
    }
    paths.sort();
    let (mut graphs, mut drawn) = (0, 0);
    let mut not_drawn = Vec::new();
    for path in paths {
        let Ok(doc) = crate::materialx::read(&path) else {
            continue;
        };
        for name in crate::materialx::material_names(&doc) {
            let folder = path.parent().unwrap();
            let Ok((p, _)) =
                crate::materialx::import::preset(&doc, Some(&name), folder, &|_| Some((2, 2)))
            else {
                continue;
            };
            for g in &p.scene.shader_tree.node_graphs.graphs {
                graphs += 1;
                let flat = g.flatten(&Default::default()).unwrap();
                let Some(v) = surface(&flat, &[], &GraphSpace::default(), &mut |_| Some(1)) else {
                    not_drawn.push(name.clone());
                    continue;
                };
                drawn += 1;
                for m in &v.missing {
                    println!("{name}: {m}");
                }
                for (ch, e) in &v.channels {
                    for q in [[0.2, 0.3, 0.4], [0.9, 0.1, 0.5]] {
                        let x = e.eval_at(&at(q), &|_, _| [0.5; 4]);
                        assert!(x.iter().all(|c| c.is_finite()), "{name} {ch:?} {x:?}");
                    }
                    assert!(
                        crate::shader_wgsl::translate(e, &mut |_| Some(0)).is_some(),
                        "{name} {ch:?}"
                    );
                }
            }
        }
    }
    println!("{drawn}/{graphs} example graphs drawn from their nodes");
    assert!(graphs >= 10, "{graphs} graph materials");
    assert!(
        not_drawn.is_empty(),
        "{drawn}/{graphs} drawn; not: {not_drawn:?}"
    );
}
