//! Basic primitive UVs (Jason decision 2026-11-01, SP-Q13/CYL-Q16). The
//! record counts come from the M1.6 reference samples (reference-oracles.json);
//! the layouts themselves are provisional.
use glam::{DVec2, DVec3};
use operandi_core::*;
use std::collections::{BTreeMap, BTreeSet};

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e
}

fn radial(sides: u32, segments: u32) -> RadialParameters {
    RadialParameters {
        radius: DVec3::new(1., 1.5, 2.),
        sides,
        segments,
    }
}

fn texture(mesh: &Mesh) -> &Vec<Vec<Option<DVec2>>> {
    mesh.uv_maps.get(PRIMITIVE_UV_MAP).expect("Texture map")
}

/// Distinct (point, UV) records, and points carrying more than one UV.
fn records(mesh: &Mesh) -> (usize, usize) {
    let mut by_point = BTreeMap::<u32, BTreeSet<(u64, u64)>>::new();
    for (face, uvs) in mesh.faces.iter().zip(texture(mesh)) {
        for (&v, uv) in face.vertices.iter().zip(uvs) {
            let uv = uv.expect("every corner has a UV");
            by_point
                .entry(v)
                .or_default()
                .insert((uv.x.to_bits(), uv.y.to_bits()));
        }
    }
    let total = by_point.values().map(BTreeSet::len).sum();
    let multi = by_point.values().filter(|s| s.len() > 1).count();
    (total, multi)
}

/// Twice the signed area of a face's UV polygon.
fn uv_area(uvs: &[Option<DVec2>]) -> f64 {
    let p: Vec<DVec2> = uvs.iter().map(|u| u.unwrap()).collect();
    (0..p.len())
        .map(|i| p[i].perp_dot(p[(i + 1) % p.len()]))
        .sum()
}

fn assert_unit_square_and_unmirrored(mesh: &Mesh, what: &str) {
    for (fi, uvs) in texture(mesh).iter().enumerate() {
        for uv in uvs.iter().flatten() {
            assert!(
                (-1e-12..=1. + 1e-12).contains(&uv.x) && (-1e-12..=1. + 1e-12).contains(&uv.y),
                "{what} face {fi}: {uv}"
            );
        }
        // Outward-wound faces map counterclockwise, so textures aren't mirrored.
        assert!(uv_area(uvs) > 0., "{what} face {fi} is flipped or empty");
    }
}

#[test]
fn sphere_matches_the_reference_record_counts() {
    let mut mesh = globe_geometry(DVec3::new(1., 2., -3.), radial(8, 4)).unwrap();
    add_globe_uvs(&mut mesh, 8, 4).unwrap();
    assert_eq!(records(&mesh), (43, 5), "R16-SP-01 / R17-SP-01");
    assert_unit_square_and_unmirrored(&mesh, "sphere");
}

#[test]
fn cylinder_matches_the_reference_record_counts() {
    let mut mesh = cylinder_geometry(DVec3::new(1., 2., -3.), radial(8, 3)).unwrap();
    add_cylinder_uvs(&mut mesh, 8, 3).unwrap();
    assert_eq!(records(&mesh), (52, 18), "R16-CY-01 / R17-CY-01");
    assert_unit_square_and_unmirrored(&mesh, "cylinder");
}

#[test]
fn cube_faces_fill_their_own_tiles() {
    for segments in [[1; 3], [2, 3, 1]] {
        let (position, size) = (DVec3::new(1., -2., 3.), DVec3::new(2., 3., 4.));
        let mut mesh = primitive::segmented_cube(position, size, segments).unwrap();
        add_cube_uvs(&mut mesh, position, size).unwrap();
        assert_unit_square_and_unmirrored(&mesh, "cube");
        // Each of the six sides covers exactly one 1/3 x 1/2 tile.
        let mut area = BTreeMap::<(usize, bool), f64>::new();
        for (fi, uvs) in texture(&mesh).iter().enumerate() {
            let n = mesh.normal(fi);
            let axis = n.abs().max_position();
            *area.entry((axis, n[axis] > 0.)).or_default() += uv_area(uvs) / 2.;
        }
        assert_eq!(area.len(), 6);
        for (side, a) in area {
            assert!((a - 1. / 6.).abs() < 1e-12, "{segments:?} {side:?}: {a}");
        }
    }
}

/// Draw one primitive through the editor and Drop it.
fn draw(e: &mut Editor, tool: Tool, uvs: bool) {
    e.dispatch(Command::Activate(tool)).unwrap();
    e.dispatch(Command::PrimitiveUvs(uvs)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.size = DVec3::splat(2.);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
}

#[test]
fn primitives_make_uvs_by_default_and_can_turn_them_off() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut e = editor();
        draw(&mut e, tool, true);
        let mesh = &e.scene.active().unwrap().mesh;
        assert!(!mesh.faces.is_empty(), "{tool:?}");
        assert!(
            texture(mesh).iter().flatten().all(Option::is_some),
            "{tool:?}"
        );

        let mut e = editor();
        draw(&mut e, tool, false);
        let mesh = &e.scene.active().unwrap().mesh;
        assert!(
            !mesh.faces.is_empty() && mesh.uv_maps.is_empty(),
            "{tool:?}"
        );
    }
}

#[test]
fn make_uvs_is_kept_between_uses_and_reset_turns_it_back_on() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut e = editor();
        assert!(e.dispatch(Command::Activate(tool)).is_ok());
        assert!(
            e.session.live.as_ref().unwrap().params.make_uvs,
            "{tool:?} default on"
        );
        e.dispatch(Command::Cancel).unwrap();
        draw(&mut e, tool, false);
        e.dispatch(Command::Activate(tool)).unwrap();
        assert!(
            !e.session.live.as_ref().unwrap().params.make_uvs,
            "{tool:?} kept off"
        );
        e.dispatch(Command::ResetPrimitive).unwrap();
        assert!(
            e.session.live.as_ref().unwrap().params.make_uvs,
            "{tool:?} reset on"
        );
        e.dispatch(Command::Cancel).unwrap();
    }
}

#[test]
fn toggling_before_drawing_creates_no_geometry() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut e = editor();
        let scene = e.scene.clone();
        let history = e.history.undo.len();
        e.dispatch(Command::Activate(tool)).unwrap();
        e.dispatch(Command::PrimitiveUvs(false)).unwrap();
        e.dispatch(Command::PrimitiveUvs(true)).unwrap();
        assert_eq!(e.scene, scene, "{tool:?}");
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.history.undo.len(), history);
    }
}

#[test]
fn existing_maps_are_kept_and_undo_save_reopen_agree() {
    let mut e = editor();
    draw(&mut e, Tool::Box, false);
    let old_faces = e.scene.active().unwrap().mesh.faces.len();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    let seam = vec![vec![Some(DVec2::new(0.125, 0.875)); 4]; old_faces];
    mesh.uv_maps.insert("seam".into(), seam.clone());
    let before = e.scene.clone();

    draw(&mut e, Tool::Sphere, true);
    let mesh = &e.scene.active().unwrap().mesh;
    assert_eq!(
        mesh.uv_maps["seam"][..old_faces],
        seam[..],
        "old corners kept"
    );
    assert!(
        mesh.uv_maps["seam"][old_faces..]
            .iter()
            .flatten()
            .all(Option::is_none)
    );
    assert!(
        texture(mesh)[..old_faces]
            .iter()
            .flatten()
            .all(Option::is_none)
    );
    assert!(
        texture(mesh)[old_faces..]
            .iter()
            .flatten()
            .all(Option::is_some)
    );

    let reopened = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert_eq!(reopened.active().unwrap().mesh.uv_maps, mesh.uv_maps);
    let after = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
}

#[test]
fn unit_insertion_follows_make_uvs() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut e = editor();
        e.dispatch(Command::InsertUnit {
            tool,
            destination: UnitDestination::ExistingMesh,
        })
        .unwrap();
        assert!(
            texture(&e.scene.active().unwrap().mesh)
                .iter()
                .flatten()
                .all(Option::is_some)
        );

        let mut e = editor();
        draw(&mut e, tool, false);
        let faces = e.scene.active().unwrap().mesh.faces.len();
        e.dispatch(Command::InsertUnit {
            tool,
            destination: UnitDestination::ExistingMesh,
        })
        .unwrap();
        let mesh = &e.scene.active().unwrap().mesh;
        assert!(
            mesh.faces.len() > faces && mesh.uv_maps.is_empty(),
            "{tool:?}"
        );
    }
}
