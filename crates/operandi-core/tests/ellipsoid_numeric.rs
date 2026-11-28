//! EA-T01..12 and numeric branches of PT-SWEEP-ELL/PT-T-ELL-01..03.
use glam::{DVec2, DVec3, DVec4};
use operandi_core::*;
use std::collections::BTreeSet;
fn point_id(label: &str, n: u32, m: u32) -> u32 {
    match label {
        "P-" => 0,
        "P+" => 1 + n * (m - 1),
        _ => {
            let q: Vec<u32> = label
                .trim_start_matches("R[")
                .trim_end_matches(']')
                .split(',')
                .map(|s| s.parse().unwrap())
                .collect();
            1 + (q[0] - 1) * n + q[1]
        }
    }
}
fn explicit() -> EllipsoidParameters {
    EllipsoidParameters {
        position: DVec3::new(1., -2., 3.),
        radius: DVec3::new(2., 3., 4.),
        sides: 8,
        segments: 4,
        make_uvs: false,
        ..Default::default()
    }
}
#[test]
fn ea_t03_t04_pt_ell01_resolution_axis_type_surface_and_signed_cycles() {
    for n in [1, 2, 3, 8, 9, 24] {
        for m in [0, 1, 4, 12] {
            for axis in 0..3 {
                for polygon in [
                    PolygonRepresentation::Face,
                    PolygonRepresentation::Subdivs,
                    PolygonRepresentation::CatmullClark,
                ] {
                    let p = EllipsoidParameters {
                        sides: n,
                        segments: m,
                        axis,
                        polygon,
                        ..explicit()
                    }
                    .normalized();
                    let mesh = ellipsoid_geometry(&p).unwrap();
                    assert_eq!(
                        mesh.vertices.len(),
                        (2 + p.sides * (p.segments - 1)) as usize
                    );
                    assert_eq!(mesh.faces.len(), (p.sides * p.segments) as usize);
                    assert_eq!(
                        mesh.edges().len(),
                        (p.sides * (2 * p.segments - 1)) as usize
                    );
                    assert!(mesh.is_closed());
                    assert_eq!(
                        mesh.vertices.len() + mesh.faces.len(),
                        mesh.edges().len() + 2
                    );
                    for (i, v) in mesh.vertices.iter().enumerate() {
                        assert!(
                            (((v - p.position) / p.radius).length_squared() - 1.).abs() < 1e-12
                        );
                        if i > 0 && i + 1 < mesh.vertices.len() {
                            assert!((v[p.axis] - p.position[p.axis]).abs() < p.radius[p.axis]);
                        }
                    }
                    for fi in 0..mesh.faces.len() {
                        assert!(mesh.normal(fi).dot(mesh.center(fi) - p.position) > 0.);
                    }
                    for component in 0..3 {
                        let mut signed = p.clone();
                        signed.radius[component] *= -1.;
                        let reflected = ellipsoid_geometry(&signed).unwrap();
                        assert_eq!(reflected.faces, mesh.faces); // No reflected-normal repair.
                        for (a, b) in mesh.vertices.iter().zip(&reflected.vertices) {
                            let mut expected = *a;
                            expected[component] = 2. * p.position[component] - a[component];
                            assert!((expected - b).length() < 1e-12);
                        }
                    }
                }
            }
        }
    }
    let mesh = ellipsoid_geometry(&EllipsoidParameters {
        sides: 9,
        segments: 5,
        ..explicit()
    })
    .unwrap();
    assert!(
        mesh.vertices
            .iter()
            .map(|v| v.x)
            .fold(f64::NEG_INFINITY, f64::max)
            < 3.
    );
}
#[test]
fn ea_t05_bulge_independence_clamps_extremes_and_no_artificial_ceiling() {
    let base = ellipsoid_geometry(&explicit()).unwrap();
    // Independent elementary samples: a top bulge changes the meridian;
    // a side bulge moves diagonal equatorial points without changing height.
    let top_only = ellipsoid_geometry(&EllipsoidParameters {
        bulge_top: 1.,
        ..explicit()
    })
    .unwrap();
    assert!((top_only.vertices[1].y - (-4.381101577952299)).abs() < 1e-12);
    assert_ne!(top_only.vertices[1], base.vertices[1]);
    let side_only = ellipsoid_geometry(&EllipsoidParameters {
        bulge_side: 1.,
        ..explicit()
    })
    .unwrap();
    let equator_diagonal = point_id("R[2,1]", 8, 4) as usize;
    assert!((side_only.vertices[equator_diagonal].x - (-0.5874010519681996)).abs() < 1e-12);
    assert_eq!(
        side_only.vertices[equator_diagonal].y,
        base.vertices[equator_diagonal].y
    );
    assert_ne!(
        side_only.vertices[equator_diagonal],
        base.vertices[equator_diagonal]
    );
    for bt in [-100., -2., -1.5, 0., 0.25, 0.5, 1., 4., 100., 1000.] {
        for bs in [-3., -1.5, 0., 0.25, 0.5, 1., 4., 100.] {
            let p = EllipsoidParameters {
                bulge_top: bt,
                bulge_side: bs,
                ..explicit()
            };
            let mesh = ellipsoid_geometry(&p).unwrap();
            assert_eq!(mesh.faces, base.faces);
            if bt == 0. && bs != 0. {
                assert_eq!(mesh.vertices[1].y, base.vertices[1].y);
            }
            if bt < -1.5 || bs < -1.5 {
                assert_eq!(mesh, ellipsoid_geometry(&p.clone().normalized()).unwrap());
            }
        }
    }
    let p = EllipsoidParameters {
        bulge_side: 100.,
        ..explicit()
    };
    let mesh = ellipsoid_geometry(&p).unwrap();
    assert!((mesh.vertices[point_id("R[2,0]", 8, 4) as usize].x - 0.0249425173).abs() < 2e-6);
    assert!((mesh.vertices[point_id("R[2,2]", 8, 4) as usize].z - 4.923789978).abs() < 2e-6);
}
#[test]
fn ea_t06_numeric_bounds_crossing_zero_and_degenerate_storage() {
    let mut p = explicit();
    p.set_min(DVec3::new(0., -5., -1.));
    assert_eq!((p.position.x, p.radius.x, p.max().x), (1.5, 1.5, 3.));
    let mut max = p.max();
    max.y = 2.;
    p.set_max(max);
    assert_eq!((p.position.y, p.radius.y, p.min().y), (-1.5, 3.5, -5.));
    for (min, center, radius) in [(7., 7., 0.), (9., 8., -1.)] {
        let mut bound = p.min();
        bound.z = min;
        p.set_min(bound);
        assert_eq!((p.position.z, p.radius.z, p.max().z), (center, radius, 7.));
        ellipsoid_geometry(&p).unwrap().validate().unwrap();
    }
    p.radius.x = -2.;
    assert_eq!((p.min().x, p.max().x), (3.5, -0.5));
    for axis in 0..3 {
        p.radius[axis] = 0.;
        let mesh = ellipsoid_geometry(&p).unwrap();
        assert_eq!(mesh.faces.len(), 32);
        mesh.validate().unwrap();
        for fi in 0..mesh.faces.len() {
            let _ = mesh.surface_triangles(fi);
        }
    }
    assert_eq!(
        ellipsoid_geometry(&p)
            .unwrap()
            .attributes
            .degenerate_faces
            .len(),
        32
    );
}
#[test]
fn ea_t07_t08_t09_uv_corner_identity_gaps_and_retained_compensation() {
    let p = EllipsoidParameters {
        make_uvs: true,
        ..explicit()
    };
    let mesh = ellipsoid_geometry(&p).unwrap();
    let uv = &mesh.uv_maps["Texture"];
    let ids = &mesh.attributes.uv_records["Texture"];
    // Decided (ellipsoid-2026-11-05 Q-EL-01): the top fan's ring corner
    // keeps its own column, like the adjoining band.
    assert_eq!(mesh.faces[24].vertices[1], mesh.faces[16].vertices[3]);
    assert_eq!(uv[24][1], Some(DVec2::new(0., 0.75)));
    assert_eq!(uv[16][3], Some(DVec2::new(0., 0.75)));
    assert_eq!(ids[24][1], ids[16][3]);
    // Continuous ordinary band corners share identity, the seam does not.
    assert_eq!(ids[8][2], ids[9][3]);
    assert_ne!(ids[8][0], ids[15][1]);
    for gaps in [-10., 0., 7., 50., 60., 100.] {
        let q = EllipsoidParameters { gaps, ..p.clone() };
        let result = ellipsoid_geometry(&q).unwrap();
        assert_eq!(result.vertices, mesh.vertices);
        assert_eq!(result.attributes.uv_records, mesh.attributes.uv_records);
        let g = gaps.clamp(0., 50.) / 100.;
        for (got, original) in result.uv_maps["Texture"]
            .iter()
            .flatten()
            .zip(uv.iter().flatten())
        {
            assert!(
                (got.unwrap() - (DVec2::splat(g) + (1. - 2. * g) * original.unwrap())).length()
                    < 1e-12
            );
        }
    }
    for axis in 0..3 {
        for n in [7, 8, 11] {
            for m in [4, 5, 7] {
                let q = EllipsoidParameters {
                    axis,
                    sides: n,
                    segments: m,
                    make_uvs: true,
                    compensation: true,
                    gaps: 7.,
                    ..explicit()
                };
                let a = ellipsoid_geometry(&q).unwrap();
                let b = ellipsoid_geometry(&EllipsoidParameters {
                    bulge_top: 0.37,
                    bulge_side: 0.81,
                    ..q.clone()
                })
                .unwrap();
                assert_eq!(a.uv_maps, b.uv_maps);
                let off = EllipsoidParameters {
                    make_uvs: false,
                    ..q
                };
                assert!(ellipsoid_geometry(&off).unwrap().uv_maps.is_empty());
                assert_eq!(off.gaps, 7.);
                assert!(off.compensation);
            }
        }
    }
    for axis in 0..3 {
        for value in [0., -2.] {
            let mut q = p.clone();
            q.radius[axis] = value;
            ellipsoid_geometry(&q).unwrap();
            q.compensation = true;
            assert!(
                ellipsoid_geometry(&q)
                    .unwrap_err()
                    .contains("Compensated UVs")
            );
            q.make_uvs = false;
            ellipsoid_geometry(&q).unwrap();
        }
    }
}

fn mapped_fixture() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0).unwrap();
    mesh.attributes.representations = vec![PolygonRepresentation::Subdivs; 6];
    for (i, f) in mesh.faces.iter_mut().enumerate() {
        f.material = format!("material{i}");
        f.part = Some(format!("part{i}"));
        f.smooth = i % 2 == 0;
    }
    mesh.uv_maps.insert(
        "ELL_ExistingUV".into(),
        mesh.faces
            .iter()
            .map(|f| {
                f.vertices
                    .iter()
                    .map(|v| Some(DVec2::new(f64::from(*v), 0.2)))
                    .collect()
            })
            .collect(),
    );
    mesh.uv_maps.insert(
        "ELL_SparseUV".into(),
        mesh.faces
            .iter()
            .map(|f| {
                f.vertices
                    .iter()
                    .map(|&v| (v == 0).then_some(DVec2::new(0.3, 0.4)))
                    .collect()
            })
            .collect(),
    );
    mesh.attributes.uv_records.insert(
        "ELL_SparseUV".into(),
        mesh.faces
            .iter()
            .map(|f| f.vertices.iter().map(|&v| (v == 0).then_some(42)).collect())
            .collect(),
    );
    mesh.attributes
        .corner_normals
        .insert("original".into(), vec![vec![Some(DVec3::Z); 4]; 6]);
    mesh.attributes
        .point_weights
        .entry("weights".into())
        .or_default()
        .insert(0, 0.75);
    mesh.attributes
        .point_colors
        .entry("colors".into())
        .or_default()
        .insert(1, DVec4::ONE);
    e.scene.active_mut().unwrap().mesh = mesh.clone().into();
    let first = Some(e.scene.active().unwrap().id);
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.select_item(first);
    e.scene.selection.mode = Mode::Item;
    e.scene.validate().unwrap();
    e
}
#[test]
fn ea_t10_t11_sparse_source_selected_destination_replacement_history_and_reopen() {
    for make_uvs in [false, true] {
        let mut e = mapped_fixture();
        let base = e.scene.clone();
        e.dispatch(Command::ArmEllipsoid).unwrap();
        let p = EllipsoidParameters {
            position: DVec3::new(5., 2., -1.),
            sides: 9,
            segments: 5,
            polygon: PolygonRepresentation::CatmullClark,
            bulge_top: 0.37,
            bulge_side: 0.81,
            make_uvs,
            compensation: true,
            gaps: 7.,
            selected_uv_map: Some("ELL_SparseUV".into()),
            ..explicit()
        };
        for bt in [0.37, 0.7, 0.37] {
            e.dispatch(Command::ConfigureEllipsoid(EllipsoidParameters {
                bulge_top: bt,
                ..p.clone()
            }))
            .unwrap();
            assert_eq!(e.scene, base);
        }
        let expected = e.ellipsoid_preview_scene().unwrap().unwrap();
        let mesh = &expected.active().unwrap().mesh;
        assert_eq!((mesh.vertices.len(), mesh.faces.len()), (46, 51));
        assert_eq!(&mesh.vertices[..8], base.active().unwrap().mesh.vertices);
        assert_eq!(&mesh.faces[..6], base.active().unwrap().mesh.faces);
        assert_eq!(expected.items[1], base.items[1]);
        assert_eq!(expected.selection, base.selection);
        for name in ["ELL_ExistingUV", "ELL_SparseUV"] {
            assert_eq!(
                &mesh.uv_maps[name][..6],
                base.active().unwrap().mesh.uv_maps[name]
            );
        }
        assert!(
            mesh.uv_maps["ELL_ExistingUV"][6..]
                .iter()
                .flatten()
                .all(Option::is_none)
        );
        assert_eq!(
            mesh.uv_maps["ELL_SparseUV"][6..]
                .iter()
                .flatten()
                .any(Option::is_some),
            make_uvs
        );
        assert_eq!(
            mesh.attributes.corner_normals["original"][..6],
            base.active().unwrap().mesh.attributes.corner_normals["original"]
        );
        assert_eq!(
            mesh.attributes.point_weights,
            base.active().unwrap().mesh.attributes.point_weights
        );
        assert_eq!(
            mesh.attributes.point_colors,
            base.active().unwrap().mesh.attributes.point_colors
        );
        let identities: BTreeSet<_> = mesh.attributes.uv_records["ELL_SparseUV"][6..]
            .iter()
            .flatten()
            .flatten()
            .collect();
        if make_uvs {
            assert!(identities.iter().all(|&&id| id > 42));
        }
        e.dispatch(Command::Apply).unwrap();
        assert_eq!(e.scene, expected);
        assert_eq!(e.history.undo.len(), 1);
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(e.history.undo.len(), 1);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, base);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, expected);
        assert!(e.session.ellipsoid.is_none());
        assert_eq!(
            persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap(),
            expected
        );
        let path = std::env::temp_dir().join(format!(
            "ellipsoid-ea10-{}-{make_uvs}.opscene",
            std::process::id()
        ));
        e.save_committed(&path).unwrap();
        let reopened = persistence::load(&path).unwrap();
        assert_eq!(reopened, expected);
        std::fs::remove_file(path).unwrap();
        let mut edit = Editor {
            scene: reopened,
            ..Default::default()
        };
        edit.dispatch(Command::ArmEllipsoid).unwrap();
        edit.dispatch(Command::ConfigureEllipsoid(p)).unwrap();
        edit.dispatch(Command::Cancel).unwrap();
        assert_eq!(edit.scene, expected);
    }
}
#[test]
fn ea_t01_reset_units_destinations_and_single_checkpoint() {
    let p = EllipsoidParameters::default();
    assert_eq!(p.radius, DVec3::splat(0.5));
    assert_eq!((p.sides, p.segments), (24, 12));
    assert_eq!(p.polygon, PolygonRepresentation::Face);
    assert!(p.make_uvs);
    assert!(!p.compensation);
    for destination in [UnitDestination::ExistingMesh, UnitDestination::NewMesh] {
        let mut e = mapped_fixture();
        let before = e.scene.clone();
        e.dispatch(Command::InsertEllipsoidUnit(destination))
            .unwrap();
        let result = e.scene.clone();
        let mesh = &e.scene.active().unwrap().mesh;
        assert_eq!(
            mesh.vertices.len(),
            if destination == UnitDestination::NewMesh {
                266
            } else {
                274
            }
        );
        if destination == UnitDestination::NewMesh {
            assert_eq!(e.scene.active().unwrap().name, "Ellipsoid");
            assert_eq!(e.scene.items[0], before.items[0]);
        }
        assert!(e.session.ellipsoid.is_none());
        assert_eq!(e.history.undo.len(), 1);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, result);
        assert!(e.session.ellipsoid.is_none());
        e.dispatch(Command::ArmEllipsoid).unwrap();
        e.dispatch(Command::ConfigureEllipsoid(explicit())).unwrap();
        e.dispatch(Command::ResetEllipsoid).unwrap();
        assert_eq!(e.session.ellipsoid.as_ref().unwrap().requested, p);
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.scene, result);
    }
}
#[test]
fn ea_t11_invalid_recovery_cancellation_drop_save_and_context_guards() {
    let mut e = mapped_fixture();
    let base = e.scene.clone();
    e.dispatch(Command::ArmEllipsoid).unwrap();
    e.dispatch(Command::ConfigureEllipsoid(explicit())).unwrap();
    let valid = e.session.ellipsoid.as_ref().unwrap().preview.clone();
    for p in [
        EllipsoidParameters {
            sides: u32::MAX,
            ..explicit()
        },
        EllipsoidParameters {
            bulge_top: f64::NAN,
            ..explicit()
        },
        EllipsoidParameters {
            axis: 3,
            ..explicit()
        },
        EllipsoidParameters {
            make_uvs: true,
            compensation: true,
            radius: DVec3::new(0., 3., 4.),
            ..explicit()
        },
    ] {
        assert!(e.dispatch(Command::ConfigureEllipsoid(p)).is_err());
        assert_eq!(e.scene, base);
        assert_eq!(e.session.ellipsoid.as_ref().unwrap().preview, valid);
        assert!(e.dispatch(Command::Drop).is_err());
        assert!(e.dispatch(Command::Apply).is_err());
        e.dispatch(Command::ConfigureEllipsoid(explicit())).unwrap();
        assert!(e.session.ellipsoid.as_ref().unwrap().error.is_none());
    }
    for cmd in [Command::Redo, Command::SelectItem(None), Command::Continue] {
        assert!(e.dispatch(cmd).is_err());
        assert_eq!(e.scene, base);
    }
    let failed = e.clone();
    assert!(
        e.save_committed(std::path::Path::new(
            "/nonexistent-ellipsoid-directory/scene.opscene"
        ))
        .is_err()
    );
    assert!(e == failed);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, base);
    assert!(e.history.undo.is_empty());
    e.dispatch(Command::ArmEllipsoid).unwrap();
    e.dispatch(Command::ConfigureEllipsoid(explicit())).unwrap();
    let expected = e.ellipsoid_preview_scene().unwrap().unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.scene, expected);
    assert_eq!(e.history.undo.len(), 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, base);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, expected);
    assert!(e.session.ellipsoid.is_none());
    // A moved mesh takes the Ellipsoid now (primitive_moved_mesh.rs); an
    // empty selection, a hidden mesh and a non-default action center don't.
    for setup in 0..3 {
        let mut e = mapped_fixture();
        match setup {
            0 => e.scene.select_item(None),
            1 => e.scene.active_mut().unwrap().visible = false,
            _ => e.action_context.center = CenterMode::Origin,
        };
        let before = e.clone();
        assert!(e.dispatch(Command::ArmEllipsoid).is_err());
        assert!(e == before);
    }
}

#[test]
fn ea_t12_numeric_plane_independence_and_bounded_reflected_lobes() {
    let base = ellipsoid_geometry(&explicit()).unwrap();
    for plane in 0..3 {
        for fixed in [false, true] {
            let mut e = mapped_fixture();
            e.scene.workplane = Workplane {
                fixed,
                plane,
                center: DVec3::new(4., 5., 6.),
            };
            e.dispatch(Command::ArmEllipsoid).unwrap();
            e.dispatch(Command::ConfigureEllipsoid(explicit())).unwrap();
            assert_eq!(
                e.session
                    .ellipsoid
                    .as_ref()
                    .unwrap()
                    .preview
                    .as_ref()
                    .unwrap(),
                &base
            );
        }
    }
    for axis in 0..3 {
        let p = EllipsoidParameters {
            symmetry_axis: Some(axis),
            ..explicit()
        };
        let mesh = ellipsoid_geometry(&p).unwrap();
        assert_eq!(
            (mesh.vertices.len(), mesh.faces.len(), mesh.edges().len()),
            (52, 64, 112)
        );
        assert_eq!(&mesh.vertices[..26], base.vertices);
        assert_eq!(&mesh.faces[..32], base.faces);
        for (v, q) in base.vertices.iter().zip(&mesh.vertices[26..]) {
            let mut expected = *v;
            expected[axis] *= -1.;
            assert_eq!(*q, expected);
        }
        for (f, q) in base.faces.iter().zip(&mesh.faces[32..]) {
            assert_eq!(
                q.vertices,
                f.vertices.iter().rev().map(|v| v + 26).collect::<Vec<_>>()
            );
        }
        let mut e = mapped_fixture();
        e.dispatch(Command::ArmEllipsoid).unwrap();
        e.dispatch(Command::ConfigureEllipsoid(p.clone())).unwrap();
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(
            persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap(),
            e.scene
        );
        assert!(
            ellipsoid_geometry(&EllipsoidParameters {
                position: DVec3::ZERO,
                ..p
            })
            .unwrap_err()
            .contains("tested setup")
        );
    }
}
