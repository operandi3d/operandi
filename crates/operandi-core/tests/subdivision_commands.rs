//! Subdivision commands through the editor: toggle, settings, Freeze, undo,
//! save and reopen (specifications/subdivision-2026-11-03).
use glam::DVec3;
use operandi_core::subdivision::{SubdivisionSettings, Toggle, has_smooth};
use operandi_core::*;

fn cube_editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    e
}

#[test]
fn toggle_acts_on_selected_polygons_and_undoes() {
    let mut e = cube_editor();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![2];
    e.dispatch(Command::ToggleSubdivision(Toggle::CatmullClark))
        .unwrap();
    let reps = &e.scene.active().unwrap().mesh.attributes.representations;
    assert_eq!(reps[2], PolygonRepresentation::CatmullClark);
    assert_eq!(
        reps.iter()
            .filter(|r| **r != PolygonRepresentation::Face)
            .count(),
        1
    );
    e.dispatch(Command::Undo).unwrap();
    assert!(!has_smooth(&e.scene.active().unwrap().mesh));
    e.dispatch(Command::Redo).unwrap();
    assert!(has_smooth(&e.scene.active().unwrap().mesh));
}

#[test]
fn settings_and_kinds_survive_save_and_reopen() {
    let mut e = cube_editor();
    e.dispatch(Command::ToggleSubdivision(Toggle::Subdivision))
        .unwrap();
    let settings = SubdivisionSettings {
        subdivision_level: 1,
        ..Default::default()
    };
    e.dispatch(Command::SetSubdivision(settings.clone()))
        .unwrap();
    assert_eq!(
        e.scene.active().unwrap().subdivision,
        Some(settings.clone())
    );
    let bytes = persistence::encode(&e.scene).unwrap();
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back, e.scene);
    // Levels are limited and invalid values change nothing.
    let too_high = SubdivisionSettings {
        catmull_clark_level: 99,
        ..Default::default()
    };
    assert!(e.dispatch(Command::SetSubdivision(too_high)).is_err());
    assert_eq!(e.scene.active().unwrap().subdivision, Some(settings));
}

#[test]
fn freeze_bakes_the_display_level_and_undoes() {
    let mut e = cube_editor();
    e.dispatch(Command::ToggleSubdivision(Toggle::CatmullClark))
        .unwrap();
    e.dispatch(Command::SetSubdivision(SubdivisionSettings {
        catmull_clark_level: 1,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Freeze(Default::default())).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    assert_eq!((mesh.vertices.len(), mesh.faces.len()), (26, 24));
    assert!(!has_smooth(mesh));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 6);
    // Nothing to freeze on a plain mesh.
    let mut plain = cube_editor();
    assert!(plain.dispatch(Command::Freeze(Default::default())).is_err());
}

/// The display surface carries the cage's UV maps (each piece's corners
/// inside its cage polygon's UVs), so image textures map onto Sub-D
/// meshes drawn from the stored surface.
#[test]
fn the_display_surface_keeps_the_uv_maps() {
    let mut e = cube_editor();
    e.dispatch(Command::ToggleSubdivision(Toggle::Subdivision))
        .unwrap();
    let mut mesh: Mesh = (*e.scene.active().unwrap().mesh).clone();
    // One map: every polygon covers the unit square.
    let rows: Vec<Vec<Option<glam::DVec2>>> = mesh
        .faces
        .iter()
        .map(|f| {
            (0..f.vertices.len())
                .map(|c| {
                    let a = c as f64 / f.vertices.len() as f64 * std::f64::consts::TAU;
                    Some(glam::DVec2::new(0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()))
                })
                .collect()
        })
        .collect();
    mesh.uv_maps = std::collections::BTreeMap::from([("Texture".to_string(), rows)]).into();
    let settings = SubdivisionSettings::default();
    let (surface, sources, _) =
        operandi_core::subdivision::display_surface_edges(&mesh, &settings).unwrap();
    let map = surface.uv_maps.get("Texture").expect("the map carried");
    assert_eq!(map.len(), surface.faces.len());
    assert!(sources.len() > mesh.faces.len(), "refined");
    for (piece, row) in map.iter().enumerate() {
        assert_eq!(row.len(), surface.faces[piece].vertices.len());
        for uv in row {
            let uv = uv.expect("every corner mapped");
            assert!((-1e-9..=1. + 1e-9).contains(&uv.x) && (-1e-9..=1. + 1e-9).contains(&uv.y));
        }
    }
    // Without UV maps the surface stays without.
    mesh.uv_maps = Default::default();
    let (bare, _, _) = operandi_core::subdivision::display_surface_edges(&mesh, &settings).unwrap();
    assert!(bare.uv_maps.is_empty());
}
