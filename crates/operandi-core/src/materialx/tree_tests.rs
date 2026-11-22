use super::*;
use crate::procedural::{Procedural, ProceduralKind};
use crate::shader_tree::Filtering;
use crate::texture_locator::{FalloffType, Tiling};

fn boxed(e: Expr) -> Box<Expr> {
    Box::new(e)
}

/// One of every expression node, with awkward numbers (thirds, negative
/// zero, tiny and huge values) so the text has to keep every bit.
fn every_node() -> Vec<Expr> {
    let uv = Expr::MeshUv {
        map: Some("Texture 2".into()),
        repeat: [1. / 3., 2.5],
        rotation: -0.,
    };
    let uv_first = Expr::MeshUv {
        map: None,
        repeat: [1., 1.],
        rotation: 1e-300,
    };
    let locator = Expr::Locator(Box::new([
        0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1e20,
    ]));
    let normal = Expr::Normal(Box::new([1., 0., 0., 0., 1., 0., 0., 0., 1. / 7.]));
    let lookup = |pattern: ProceduralKind| PatternLookup {
        pattern: Procedural::new(pattern).pattern,
        axis: 2,
        vector: locator.clone(),
    };
    let image = |alpha: bool| {
        let l = Box::new(ImageLookup {
            image: 7,
            filtering: Filtering::Bicubic,
            vector: Expr::Tile(boxed(uv.clone()), [Tiling::Mirror, Tiling::Reset]),
            extension: Tiling::Edge,
        });
        if alpha {
            Expr::ImageAlpha(l)
        } else {
            Expr::ImageColor(l)
        }
    };
    let mut out = vec![
        image(false),
        image(true),
        Expr::Planar {
            of: boxed(locator.clone()),
            axis: 1,
            centre: 0.5,
            repeat: [3., 4.],
            mirror: 3,
        },
        Expr::Wrap {
            of: boxed(locator.clone()),
            axis: 0,
            spherical: true,
            centre: -0.25,
            repeat: [1., 2.],
        },
        Expr::Probe {
            of: boxed(locator.clone()),
            axis: 2,
            repeat: [1., 1.],
        },
        Expr::FaceWeight {
            normal: boxed(normal.clone()),
            face: 4,
            kind: FaceKind::Signed,
        },
        Expr::FaceWeight {
            normal: boxed(normal.clone()),
            face: 1,
            kind: FaceKind::Dominant,
        },
        Expr::FaceWeight {
            normal: boxed(normal.clone()),
            face: 2,
            kind: FaceKind::Power(3.5),
        },
        Expr::Facing(boxed(normal.clone()), 1),
        Expr::Falloff {
            of: boxed(locator.clone()),
            kind: FalloffType::LinearZ,
            amount: [0.1, 0.2, 0.3],
        },
        Expr::Inside(boxed(uv_first.clone()), [true, false]),
        Expr::Pattern(Box::new(lookup(ProceduralKind::Noise))),
        Expr::Pattern(Box::new(lookup(ProceduralKind::Checker))),
        Expr::CellRandom(Box::new(lookup(ProceduralKind::Cellular))),
        Expr::BiasGain(boxed(uv_first.clone()), 0.3, 0.7),
        Expr::HsvVary {
            color: boxed(image(false)),
            random: boxed(Expr::CellRandom(Box::new(lookup(ProceduralKind::Dots)))),
            amounts: [0.1, 0.2, 0.3],
        },
        Expr::Luma(boxed(image(false))),
        Expr::Channel(boxed(image(false)), 2),
        Expr::Gamma(boxed(image(false)), 2.2),
        Expr::DistanceTo([1., -2., 3.]),
        Expr::LocatorIncidence([0., 1e-12, 0.]),
        Expr::Hsv {
            of: boxed(image(false)),
            hue_scale: 1.,
            hue_shift: 120.,
            saturation_scale: 0.5,
        },
        Expr::Attribute {
            name: "Weight \"map\" <1> & co".into(),
            alpha: true,
        },
        Expr::Ramp {
            input: boxed(Expr::Geometry(GeometryInput::Slope)),
            lo: 0.,
            hi: 180.,
            stops: Arc::new(vec![[0., 0., 0., 1.], [1., 0.5, 0.25, 0.125]]),
            alpha: false,
        },
    ];
    for (g, _) in GEOMETRY {
        out.push(Expr::Geometry(g));
    }
    for (toward, _) in TOWARD {
        out.push(Expr::Occlusion {
            rays: 16,
            distance: 0.75,
            same_surface: true,
            inside: false,
            toward,
        });
    }
    for (mode, _) in MIXES {
        out.push(Expr::Mix {
            mode,
            fac: boxed(Expr::scalar(0.5)),
            a: boxed(image(false)),
            b: boxed(Expr::Const([0.1, 0.2, 1. / 3.])),
        });
    }
    out
}

fn surface_with(exprs: &[Expr]) -> Principled {
    let shader = crate::shader_tree::ShaderChannels {
        alpha_value: 0.25,
        visible_to_reflection: false,
        ..Default::default()
    };
    let pick = |i: usize| exprs[i % exprs.len()].clone();
    Principled {
        inputs: INPUTS
            .iter()
            .enumerate()
            .rev()
            .map(|(i, (input, _))| (*input, pick(i)))
            .collect(),
        shader,
        bump: Some(pick(3)),
        normal_map: Some(pick(4)),
        displacement: Some(pick(5)),
        coat_bump: 0.6,
        see_through: Some(SeeThrough {
            kind: SeeThroughKind::Glass,
            amount: pick(6),
            color: Expr::Const([0.9, 1., 1.]),
            ior: Expr::scalar(1.45),
            roughness: pick(7),
        }),
        rounded_edge: Some(0.002),
        absorption: Some(([0.5, 0.6, 0.7], 3.)),
        scatter: Some(([0.1, 0.1, 0.2], 0.5, -0.3)),
        thin_walled: true,
        glass_principled: true,
        node: Some(NodeUse {
            layer: 12,
            graph: 3,
            coverage: pick(8),
            values: [("Base Color".to_owned(), vec![0.1, 0.2, 0.3])].into(),
            drivers: vec![("Roughness".into(), pick(9)), ("Tint".into(), pick(10))],
        }),
        values: MaterialValue::ALL
            .into_iter()
            .enumerate()
            .map(|(i, v)| (v, pick(i + 11)))
            .collect(),
    }
}

#[test]
fn the_input_table_is_in_input_order() {
    for (i, (input, name)) in INPUTS.iter().enumerate() {
        assert_eq!(*input as usize, i, "{name}");
    }
}

#[test]
fn every_tree_node_reads_back_exactly() {
    let exprs = every_node();
    for e in &exprs {
        let p = Principled {
            inputs: vec![(Input::BaseColor, e.clone())],
            ..surface_with(&[Expr::scalar(0.)])
        };
        let text = write(&p, &Scene::default(), "One");
        assert_eq!(read(&text).as_ref(), Ok(&p), "{e:?}\n{text}");
    }
}

#[test]
fn a_whole_surface_reads_back_exactly() {
    let p = surface_with(&every_node());
    let text = write(&p, &Scene::default(), "Every Layer");
    assert!(
        text.contains("<surfacematerial name=\"Every_Layer\""),
        "{text}"
    );
    assert_eq!(read(&text), Ok(p));
}

#[test]
fn a_plain_surface_has_no_optional_parts() {
    let p = Principled {
        inputs: vec![(Input::BaseColor, Expr::Const([0.8, 0.8, 0.8]))],
        shader: Default::default(),
        bump: None,
        normal_map: None,
        displacement: None,
        coat_bump: 0.,
        see_through: None,
        rounded_edge: None,
        absorption: None,
        scatter: None,
        thin_walled: false,
        glass_principled: false,
        node: None,
        values: Vec::new(),
    };
    let text = write(&p, &Scene::default(), "Plain");
    assert!(!text.contains("see_through"), "{text}");
    assert!(!text.contains(NODE_MATERIAL), "{text}");
    assert_eq!(read(&text), Ok(p));
}

#[test]
fn a_shared_expression_is_one_node() {
    let shared = every_node().swap_remove(0);
    let p = Principled {
        inputs: vec![
            (Input::BaseColor, shared.clone()),
            (Input::EmissionColor, shared.clone()),
        ],
        ..surface_with(&[Expr::scalar(0.)])
    };
    let text = write(&p, &Scene::default(), "Shared");
    assert_eq!(text.matches("<operandi_image ").count(), 1, "{text}");
    assert_eq!(read(&text), Ok(p));
}

#[test]
fn an_image_names_its_file() {
    let mut scene = Scene::default();
    scene.images.push(crate::backdrop::SceneImage {
        id: 7,
        name: "wood".into(),
        path: "project:textures/wood.png".into(),
        width: 4,
        height: 4,
    });
    let p = Principled {
        inputs: vec![(Input::BaseColor, every_node().swap_remove(0))],
        ..surface_with(&[Expr::scalar(0.)])
    };
    let text = write(&p, &scene, "Wood");
    assert!(
        text.contains("name=\"file\" type=\"filename\" value=\"project:textures/wood.png\""),
        "{text}"
    );
    assert_eq!(read(&text), Ok(p));
}

#[test]
fn broken_graphs_are_refused_with_the_node_named() {
    assert!(
        read("<materialx version=\"1.39\"></materialx>")
            .unwrap_err()
            .contains(SURFACE)
    );
    let unknown = "<materialx version=\"1.39\">
  <mystery name=\"n1\" type=\"color3\" />
  <operandi_surface name=\"surface\" type=\"surfaceshader\">
    <input name=\"base_color\" type=\"color3\" nodename=\"n1\" />
  </operandi_surface>
</materialx>";
    let e = read(unknown).unwrap_err();
    assert!(e.contains("n1 (mystery)"), "{e}");
    let missing = unknown.replace("nodename=\"n1\"", "nodename=\"gone\"");
    assert!(read(&missing).unwrap_err().contains("gone"));
    let bad_value = unknown.replace("nodename=\"n1\"", "value=\"1, 2\"");
    assert!(read(&bad_value).unwrap_err().contains("3 numbers"));
}

#[test]
fn the_library_declares_every_node_the_form_writes() {
    use super::super::tree_library::{declares, library};
    let lib = doc::parse(&library()).unwrap();
    let text = write(&surface_with(&every_node()), &Scene::default(), "All");
    let d = doc::parse(&text).unwrap();
    for el in d
        .elements
        .iter()
        .filter(|e| e.category != "surfacematerial")
    {
        assert!(
            lib.element(&format!("ND_{}", el.category)).is_some(),
            "no nodedef for {}",
            el.category
        );
        for p in &el.inputs {
            assert!(
                declares(&el.category, &p.name, &p.ty),
                "{} doesn't declare {} ({})",
                el.category,
                p.name,
                p.ty
            );
        }
    }
}

/// The standard-node implementations do the same sums as the renderer:
/// MaterialX's Cycles translation folds them, with constants, to what
/// `Mix::apply`, luma and channel give.
#[test]
fn standard_implementations_match_the_tree_sums() {
    use super::super::cycles::translate;
    use super::super::tree_library::library;
    use crate::node_graph::FlatValue;
    let lib = library();
    let lib = lib.trim_end().trim_end_matches("</materialx>");
    let a = [0.2, 0.6, 0.9];
    let b = [0.7, 0.3, 0.5];
    let base_color = |node: &str| -> [f64; 3] {
        let text = format!(
            r#"{lib}  {node}
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodename="n" />
  </open_pbr_surface>
  <surfacematerial name="M" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="s" />
  </surfacematerial>
</materialx>"#
        );
        let d = doc::parse(&text).unwrap();
        let t = translate(&d, None, std::path::Path::new("/")).unwrap_or_else(|r| panic!("{r}"));
        let p = t
            .graph
            .nodes
            .iter()
            .find(|n| n.cycles == "principled_bsdf")
            .unwrap();
        match p
            .values
            .iter()
            .find(|(s, _)| s == "Base Color")
            .map(|(_, v)| v)
        {
            Some(FlatValue::Vec3(v)) => *v,
            other => panic!("{node}: {other:?}"),
        }
    };
    let close = |x: [f64; 3], y: [f64; 3], what: &str| {
        assert!(
            (0..3).all(|i| (x[i] - y[i]).abs() < 1e-9),
            "{what}: {x:?} vs {y:?}"
        );
    };
    for (mode, name) in MIXES {
        let implemented = matches!(
            mode,
            Mix::Blend
                | Mix::Add
                | Mix::Subtract
                | Mix::Multiply
                | Mix::Screen
                | Mix::Difference
                | Mix::Darken
                | Mix::Lighten
        );
        if !implemented {
            continue;
        }
        for t in [0.3, 1.7, -0.5] {
            let node = format!(
                r#"<operandi_mix_{name} name="n" type="color3">
    <input name="bg" type="color3" value="{}" />
    <input name="fg" type="color3" value="{}" />
    <input name="mix" type="color3" value="{t}, 0.9, 0.1" />
  </operandi_mix_{name}>"#,
                nums(&a),
                nums(&b)
            );
            let want = [0, 1, 2].map(|i| mode.apply(a[i], b[i], t));
            close(base_color(&node), want, &format!("{name} at {t}"));
        }
    }
    let luma = 0.2126 * a[0] + 0.7152 * a[1] + 0.0722 * a[2];
    close(
        base_color(&format!(
            r#"<operandi_luma name="n" type="color3"><input name="in" type="color3" value="{}" /></operandi_luma>"#,
            nums(&a)
        )),
        [luma; 3],
        "luma",
    );
    close(
        base_color(&format!(
            r#"<operandi_channel name="n" type="color3"><input name="in" type="color3" value="{}" /><input name="channel" type="integer" value="1" /></operandi_channel>"#,
            nums(&a)
        )),
        [a[1]; 3],
        "channel",
    );
    close(
        base_color(
            r#"<operandi_constant name="n" type="color3"><input name="value" type="vector3" value="0.1, 0.2, 0.3" /></operandi_constant>"#,
        ),
        [0.1, 0.2, 0.3],
        "constant",
    );
}

#[test]
fn unusual_planar_numbers_keep_the_general_node() {
    let e = Expr::Planar {
        of: boxed(Expr::DistanceTo([1., 2., 3.])),
        axis: 7,
        centre: 0.,
        repeat: [1., 1.],
        mirror: 9,
    };
    let p = Principled {
        inputs: vec![(Input::BaseColor, e)],
        ..surface_with(&[Expr::scalar(0.)])
    };
    let text = write(&p, &Scene::default(), "Odd");
    assert!(text.contains("<operandi_planar name="), "{text}");
    assert_eq!(read(&text), Ok(p));
}

#[test]
fn unusual_projection_numbers_keep_the_general_nodes() {
    let n = Expr::Normal(Box::new([1., 0., 0., 0., 1., 0., 0., 0., 1.]));
    for (e, general) in [
        (
            Expr::Wrap {
                of: boxed(n.clone()),
                axis: 5,
                spherical: true,
                centre: 0.,
                repeat: [1., 1.],
            },
            "<operandi_wrap name=",
        ),
        (
            Expr::Probe {
                of: boxed(n.clone()),
                axis: 4,
                repeat: [1., 1.],
            },
            "<operandi_probe name=",
        ),
        (
            Expr::FaceWeight {
                normal: boxed(n.clone()),
                face: 9,
                kind: FaceKind::Power(2.),
            },
            "<operandi_face_weight name=",
        ),
    ] {
        let p = Principled {
            inputs: vec![(Input::BaseColor, e)],
            ..surface_with(&[Expr::scalar(0.)])
        };
        let text = write(&p, &Scene::default(), "Odd");
        assert!(text.contains(general), "{text}");
        assert_eq!(read(&text), Ok(p));
    }
}
