use glam::{DVec2, DVec3, DVec4};
use operandi_core::*;
fn params() -> ConeParameters {
    ConeParameters {
        position: DVec3::new(1., 2., 3.),
        size: DVec3::new(0.5, 0.75, 1.),
        sides: 8,
        segments: 3,
        ..Default::default()
    }
}
fn close(a: DVec3, b: DVec3) {
    assert!((a - b).abs().max_element() < 2e-6, "{a:?} != {b:?}");
}
fn uv_close(a: DVec2, b: [f64; 2]) {
    assert!(
        (a - DVec2::from(b)).abs().max_element() < 2e-6,
        "{a:?} != {b:?}"
    );
}
fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.history.undo.clear();
    e.dispatch(Command::ArmCone).unwrap();
    e
}
#[test]
fn ca_t01_t02_all_axes_oriented_rings_shared_tip_caps_and_weights() {
    for axis in 0..3 {
        for n in [3, 4, 5, 7, 8, 9, 24] {
            for m in [1, 2, 3, 4, 12] {
                for caps in [ConeCap::None, ConeCap::Single] {
                    for polygon in [
                        PolygonRepresentation::Face,
                        PolygonRepresentation::Subdivs,
                        PolygonRepresentation::CatmullClark,
                    ] {
                        let p = ConeParameters {
                            axis,
                            sides: n,
                            segments: m,
                            bottom_cap: caps,
                            polygon,
                            edge_weight: 50.,
                            ..params()
                        };
                        let mesh = cone_geometry(&p).unwrap();
                        assert_eq!(mesh.vertices.len(), (n * m + 1) as usize);
                        assert_eq!(mesh.edges().len(), (2 * n * m) as usize);
                        assert_eq!(
                            mesh.faces.len(),
                            (n * m + u32::from(caps == ConeCap::Single)) as usize
                        );
                        let mut tip = p.position;
                        tip[axis] += p.size[axis];
                        close(*mesh.vertices.last().unwrap(), tip);
                        assert!(
                            mesh.attributes
                                .representations
                                .iter()
                                .all(|&v| v == polygon)
                        );
                        assert_eq!(
                            mesh.attributes.creases.len(),
                            if polygon == PolygonRepresentation::Face {
                                0
                            } else {
                                n as usize
                            }
                        );
                        for c in &mesh.attributes.creases {
                            assert!(c.a < n && c.b < n);
                            assert_eq!(c.weight, 0.5);
                        }
                        for j in 0..m - 1 {
                            for i in 0..n {
                                assert_eq!(
                                    mesh.faces[(j * n + i) as usize].vertices,
                                    vec![
                                        (j + 1) * n + i,
                                        j * n + i,
                                        j * n + (i + 1) % n,
                                        (j + 1) * n + (i + 1) % n
                                    ]
                                );
                            }
                        }
                        for fi in 0..mesh.faces.len() {
                            let center = mesh.center(fi);
                            let mut inside = p.position;
                            inside[axis] -= p.size[axis] * 0.5;
                            assert!(mesh.normal(fi).dot(center - inside) > 0.);
                        }
                        assert_eq!(mesh.is_closed(), caps == ConeCap::Single);
                    }
                }
            }
        }
    }
}
#[test]
fn ca_t01_signed_linked_endpoints_and_reset() {
    let mut p = ConeParameters::default();
    assert_eq!((p.sides, p.segments), (24, 1));
    p.position.x = 1.;
    p.size.x = 2.;
    p.set_min(DVec3::new(-2., p.min().y, p.min().z));
    assert_eq!((p.position.x, p.size.x), (0.5, 2.5));
    p.set_max(DVec3::new(8., p.max().y, p.max().z));
    assert_eq!((p.position.x, p.size.x), (3., 5.));
    p.size.x = -3.;
    assert_eq!((p.min().x, p.max().x), (6., 0.));
    p.set_min(DVec3::new(10., p.min().y, p.min().z));
    assert_eq!((p.position.x, p.size.x), (5., -5.));
    let m = cone_geometry(&ConeParameters::default()).unwrap();
    assert_eq!(
        (m.vertices.len(), m.edges().len(), m.faces.len()),
        (25, 48, 25)
    );
}
#[test]
fn ca_t04_t13_numeric_uv_oracles_and_seam() {
    let p = params();
    let mesh = cone_geometry(&p).unwrap();
    let map = &mesh.uv_maps["Texture"];
    // First quad cycle is upper0,base0,base1,upper1. Last face is cap.
    uv_close(map[0][1].unwrap(), [0.480969876, 0.845670819]);
    uv_close(map[24][0].unwrap(), [0.980969906, 0.654329181]);
    uv_close(map[16][2].unwrap(), [0.25, 0.75]);
    uv_close(map[0][0].unwrap(), [0.403979927, 0.813780546]);
    for (axis, center, radii) in [
        (0, [0.267631114, 0.6431585], [0.267631114, 0.356841485]),
        (1, [0.292893231, 0.853553355], [0.292893219, 0.146446609]),
        (2, [0.3267976, 0.50980359], [0.3267976, 0.4901964]),
    ] {
        let p = ConeParameters {
            axis,
            compensation: true,
            ..params()
        };
        let m = cone_geometry(&p).unwrap();
        let uv = &m.uv_maps["Texture"];
        uv_close(uv[16][2].unwrap(), center);
        let phi = std::f64::consts::PI / 8.;
        uv_close(
            uv[0][1].unwrap(),
            [
                center[0] + radii[0] * phi.cos(),
                center[1] + radii[1] * phi.sin(),
            ],
        );
    }
    for g in [0., 1., 5., 7., 10., 20., 25., 30., 32.] {
        for comp in [false, true] {
            for caps in [ConeCap::None, ConeCap::Single] {
                let m = cone_geometry(&ConeParameters {
                    gaps: g,
                    compensation: comp,
                    bottom_cap: caps,
                    ..params()
                })
                .unwrap();
                assert!(
                    m.uv_maps["Texture"]
                        .iter()
                        .flatten()
                        .flatten()
                        .all(|v| v.is_finite())
                );
            }
        }
    }
    let off = cone_geometry(&ConeParameters {
        make_uvs: false,
        gaps: 50.,
        ..params()
    })
    .unwrap();
    assert!(off.uv_maps["Texture"].iter().flatten().all(Option::is_none));
}
#[test]
fn ca_d02_preview_retention_every_apply_route_recovery_cancel() {
    let mut e = editor();
    let committed = e.scene.clone();
    let p = params();
    e.dispatch(Command::ConfigureCone(p.clone())).unwrap();
    let last = e.session.cone.as_ref().unwrap().preview.clone();
    let history = e.history.undo.clone();
    let invalid = ConeParameters {
        gaps: 50.,
        ..p.clone()
    };
    assert!(e.dispatch(Command::ConfigureCone(invalid)).is_err());
    assert_eq!(e.session.cone.as_ref().unwrap().requested.gaps, 50.);
    assert_eq!(e.session.cone.as_ref().unwrap().preview, last);
    for _ in 0..2 {
        assert!(e.dispatch(Command::Apply).is_err());
        assert_eq!(e.scene, committed);
        assert!(e.history.undo == history);
    }
    e.dispatch(Command::ConfigureCone(ConeParameters { gaps: 7., ..p }))
        .unwrap();
    assert!(e.session.cone.as_ref().unwrap().error.is_none());
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices.len(), 25);
    let after = e.scene.clone();
    e.dispatch(Command::ConfigureCone(ConeParameters {
        gaps: 50.,
        ..params()
    }))
    .unwrap_err();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, after);
    let mut e = editor();
    e.dispatch(Command::ConfigureCone(ConeParameters {
        gaps: 50.,
        ..params()
    }))
    .unwrap_err();
    assert!(e.session.cone.as_ref().unwrap().preview.is_none());
    assert_eq!(e.scene.active().unwrap().mesh.vertices.len(), 0);
    e.dispatch(Command::ConfigureCone(ConeParameters {
        gaps: 50.,
        make_uvs: false,
        ..params()
    }))
    .unwrap();
    e.dispatch(Command::Apply).unwrap();
}
#[test]
fn ca_t18_numeric_apply_history_is_not_live_draw_history() {
    let mut e = editor();
    assert!(e.scene.active().unwrap().mesh.vertices.is_empty());
    e.dispatch(Command::ConfigureCone(params())).unwrap();
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices.len(), 25);
    e.dispatch(Command::ConfigureCone(ConeParameters {
        sides: 10,
        ..params()
    }))
    .unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices.len(), 25);
    e.dispatch(Command::Apply).unwrap();
    let scene = e.scene.clone();
    assert_eq!(scene.active().unwrap().mesh.vertices.len(), 56);
    // Drop after Apply shows no preview, so it records nothing.
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.cone.is_none());
    assert_eq!(e.scene, scene);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices.len(), 25);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, scene);
}
#[test]
fn ca_t16_world_numeric_frame_and_transformed_target() {
    let mut e = editor();
    e.session.cone = None;
    let id = e.scene.active().unwrap().id;
    e.scene.active_mut().unwrap().transform = scene::Transform {
        translation: DVec3::new(10., 0., 0.),
        rotation: DVec3::new(0., 0., std::f64::consts::FRAC_PI_2),
        scale: DVec3::new(2., 1., 1.),
        ..Default::default()
    };
    e.scene.workplane.center = DVec3::new(4., 5., 6.);
    e.scene.workplane.fixed = true;
    e.scene.workplane.plane = 2;
    let t = e.scene.active().unwrap().transform.clone();
    e.dispatch(Command::ArmCone).unwrap();
    e.dispatch(Command::ConfigureCone(params())).unwrap();
    e.dispatch(Command::Apply).unwrap();
    let item = e.scene.active().unwrap();
    assert_eq!(item.transform, t);
    close(
        *item.mesh.vertices.last().unwrap(),
        DVec3::new(1.375, 9., 3.),
    );
    let expected = cone_geometry(&params()).unwrap();
    for (i, &v) in item.mesh.vertices.iter().enumerate() {
        close(e.scene.world_point(id, v), expected.vertices[i]);
    }
}
#[test]
fn ca_t14_supported_single_subset_source_preservation_and_t15_persistence() {
    let mut e = editor();
    e.session.cone = None;
    let mut src = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    src.attributes.representations = vec![PolygonRepresentation::Face; 6];
    src.attributes.representations[1] = PolygonRepresentation::Subdivs;
    src.attributes.creases.push(MeshCrease {
        a: 0,
        b: 1,
        weight: 0.375,
    });
    src.attributes
        .point_weights
        .entry("OriginalWeight".into())
        .or_default()
        .insert(4, 0.7);
    src.attributes
        .point_colors
        .entry("OriginalRGBA".into())
        .or_default()
        .insert(1, DVec4::new(0.1, 0.2, 0.3, 1.));
    src.attributes.corner_normals.insert(
        "OriginalNormals".into(),
        src.faces
            .iter()
            .map(|f| vec![Some(DVec3::Z); f.vertices.len()])
            .collect(),
    );
    for name in ["Texture", "AltUV", "SparseUV"] {
        src.uv_maps.insert(
            name.into(),
            src.faces
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    f.vertices
                        .iter()
                        .enumerate()
                        .map(|(j, _)| {
                            if name == "SparseUV" && (i != 0 || j > 1) {
                                None
                            } else {
                                Some(DVec2::new(i as f64, j as f64))
                            }
                        })
                        .collect()
                })
                .collect(),
        );
    }
    src.faces[0].material = "OriginalMaterial".into();
    src.faces[0].part = Some("OriginalPart".into());
    src.faces[0].smooth = true;
    src.validate().unwrap();
    e.scene.active_mut().unwrap().mesh = src.clone().into();
    e.scene.new_mesh().unwrap();
    let unrelated = e.scene.active().unwrap().clone();
    e.scene.select_item(Some(1));
    e.dispatch(Command::ArmCone).unwrap();
    e.dispatch(Command::ConfigureCone(ConeParameters {
        polygon: PolygonRepresentation::CatmullClark,
        selected_uv_map: Some("SparseUV".into()),
        edge_weight: 50.,
        ..params()
    }))
    .unwrap();
    e.dispatch(Command::Apply).unwrap();
    let out = &e.scene.active().unwrap().mesh;
    assert_eq!(&out.vertices[..8], &src.vertices);
    assert_eq!(&out.faces[..6], &src.faces);
    assert_eq!(
        &out.attributes.representations[..6],
        &src.attributes.representations
    );
    assert_eq!(out.attributes.point_weights, src.attributes.point_weights);
    assert_eq!(out.attributes.point_colors, src.attributes.point_colors);
    assert_eq!(out.attributes.creases[0], src.attributes.creases[0]);
    for name in ["Texture", "AltUV", "SparseUV"] {
        assert_eq!(&out.uv_maps[name][..6], &src.uv_maps[name]);
    }
    assert!(
        out.uv_maps["Texture"][6..]
            .iter()
            .flatten()
            .all(Option::is_none)
    );
    assert!(
        out.uv_maps["SparseUV"][6..]
            .iter()
            .flatten()
            .all(Option::is_some)
    );
    assert_eq!(
        &out.attributes.corner_normals["OriginalNormals"][..6],
        &src.attributes.corner_normals["OriginalNormals"]
    );
    assert_eq!(e.scene.items[1], unrelated);
    e.dispatch(Command::Drop).unwrap();
    let expected = e.scene.clone();
    let bytes = persistence::encode(&expected).unwrap();
    assert_eq!(persistence::decode(&bytes).unwrap(), expected);
    let path = std::env::temp_dir().join(format!("operandi-cone-{}.opscene", std::process::id()));
    e.save_committed(&path).unwrap();
    assert_eq!(persistence::load(&path).unwrap(), expected);
    std::fs::remove_file(path).unwrap();
    // This Single subset is deliberately not the held N9 Quad Grid CA-T14 oracle.
}

#[test]
fn ca_t02_retained_type_weight_and_selected_map_without_texture() {
    let mut e = editor();
    let mut p = params();
    p.polygon = PolygonRepresentation::Subdivs;
    p.edge_weight = -50.;
    p.selected_uv_map = Some("AltUV".into());
    e.dispatch(Command::ConfigureCone(p.clone())).unwrap();
    assert!(
        e.session
            .cone
            .as_ref()
            .unwrap()
            .preview
            .as_ref()
            .unwrap()
            .attributes
            .creases
            .iter()
            .all(|c| c.weight == -0.5)
    );
    p.polygon = PolygonRepresentation::Face;
    e.dispatch(Command::ConfigureCone(p.clone())).unwrap();
    assert!(
        e.session
            .cone
            .as_ref()
            .unwrap()
            .preview
            .as_ref()
            .unwrap()
            .attributes
            .creases
            .is_empty()
    );
    assert_eq!(e.session.cone.as_ref().unwrap().requested.edge_weight, -50.);
    p.polygon = PolygonRepresentation::CatmullClark;
    p.edge_weight = 150.;
    e.dispatch(Command::ConfigureCone(p)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert!(m.attributes.creases.iter().all(|c| c.weight == 1.5));
    assert!(!m.uv_maps.contains_key("Texture"));
    assert!(m.uv_maps.contains_key("AltUV"));
    let committed = e.scene.clone();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.active().unwrap().mesh.vertices.is_empty());
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, committed);
}
#[test]
fn ca_t15_failed_save_and_invalid_source_are_atomic() {
    let mut e = editor();
    e.dispatch(Command::ConfigureCone(params())).unwrap();
    e.dispatch(Command::Apply).unwrap();
    let before = e.clone();
    let bad = std::env::temp_dir().join(format!(
        "operandi-missing-cone-{}/result.opscene",
        std::process::id()
    ));
    assert!(e.save_committed(&bad).is_err());
    assert!(e == before);
    let mut p = params();
    p.gaps = 50.;
    e.dispatch(Command::ConfigureCone(p)).unwrap_err();
    let before = e.clone();
    assert!(e.save_committed(&bad).is_err());
    assert!(e == before);
    let mut target = Mesh::default();
    let mut invalid = cone_geometry(&params()).unwrap();
    invalid.faces[0].vertices[0] = u32::MAX;
    let before = target.clone();
    assert!(append_mesh(&mut target, invalid).is_err());
    assert_eq!(target, before);
}
