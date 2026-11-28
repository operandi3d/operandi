//! MP-C13 SO-C01..04 core acceptance for Polygon Scale Offset.
//!
//! The square and right-triangle inputs and outputs are the admitted original
//! fixture `SO-F03` (`SCALE_OFFSET_ORIGINAL_FIXTURE.json`). Every other case
//! (pentagon with diagonal edge, regular hexagon, rotated/translated planes,
//! different sizes, rich data, refusals) is authored literally here from
//! plane geometry. Expected scenes are the source scene with literal corner
//! positions written in; no expectation is produced by calling the operation.
//! `assert_boundary_distance` is a separate property oracle that measures the
//! perpendicular distance between each original and resulting boundary line.
use glam::{DMat3, DVec2, DVec3, DVec4};
use operandi_core::*;
use std::f64::consts::SQRT_2;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// An editor whose active mesh is `mesh`, in Polygon mode with `selected`.
fn editor_with(mesh: Mesh, selected: Vec<u32>) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    *e.scene.active_mut().unwrap().mesh = mesh;
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = selected;
    e.scene.validate().unwrap();
    e.synchronize_identities().unwrap();
    e
}

fn plain(vertices: Vec<DVec3>, faces: Vec<Vec<u32>>) -> Mesh {
    Mesh {
        vertices,
        faces: faces.into_iter().map(face).collect(),
        ..Default::default()
    }
}

fn offset(e: &mut Editor, distance: f64) -> Result<()> {
    e.dispatch(Command::ScaleOffsetPolygon(distance))
}

/// Full scene equality, allowing only `tolerance` on vertex coordinates.
fn assert_scene(actual: &Scene, expected: &Scene, tolerance: f64) {
    let mut normalized = actual.clone();
    for (a, b) in normalized.items.iter_mut().zip(&expected.items) {
        assert_eq!(a.mesh.vertices.len(), b.mesh.vertices.len());
        for (p, q) in a.mesh.vertices.iter().zip(&b.mesh.vertices) {
            assert!(p.distance(*q) <= tolerance, "{p} != {q}");
        }
        a.mesh.vertices = b.mesh.vertices.clone();
    }
    assert_eq!(&normalized, expected);
}

/// Property oracle: each resulting boundary line lies at signed inward
/// distance `d` from the corresponding original line, in the original plane,
/// with the same corner count and edge directions.
fn assert_boundary_distance(source: &[DVec3], result: &[DVec3], normal: DVec3, d: f64, tol: f64) {
    let n = source.len();
    assert_eq!(result.len(), n);
    for i in 0..n {
        let (p0, p1) = (source[i], source[(i + 1) % n]);
        let (q0, q1) = (result[i], result[(i + 1) % n]);
        let inward = normal.cross(p1 - p0).normalize();
        assert!(((q0 - p0).dot(inward) - d).abs() <= tol, "edge {i} start");
        assert!(((q1 - p0).dot(inward) - d).abs() <= tol, "edge {i} end");
        assert!((q0 - p0).dot(normal).abs() <= tol, "corner {i} left plane");
        assert!((q1 - q0).dot(p1 - p0) > 0., "edge {i} reversed");
    }
}

/// Zero/no-change contract: the editor is identical to `before` (scene,
/// component identities, selection, undo, redo, parked I2 stage, tool state)
/// except the shared runtime ticket counters, which every accepted dispatch
/// advances by one so outstanding identity tickets go stale.
fn assert_no_change(e: &Editor, before: &Editor) {
    assert_eq!(e.identities.publication, before.identities.publication + 1);
    assert_eq!(e.identities.request, before.identities.request + 1);
    let mut normalized = e.clone();
    normalized.identities.publication = before.identities.publication;
    normalized.identities.request = before.identities.request;
    assert!(normalized == *before, "no-change mutated editor state");
}

fn loop_positions(e: &Editor, face: usize) -> Vec<DVec3> {
    let mesh = &e.scene.active().unwrap().mesh;
    mesh.faces[face]
        .vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .collect()
}

// --- Independently authored geometry ------------------------------------

/// Convex pentagon with a 45-degree edge: lines y=0, x=4, x+y=6, y=4, x=0.
const PENTAGON: [[f64; 2]; 5] = [[0., 0.], [4., 0.], [4., 2.], [2., 4.], [0., 4.]];

/// Literal intersections of the offset lines y=d, x=4-d, x+y=6-d*sqrt2,
/// y=4-d, x=d for the pentagon above.
fn pentagon_offset(d: f64) -> Vec<DVec2> {
    let diagonal = 6. - d * SQRT_2;
    vec![
        DVec2::new(d, d),
        DVec2::new(4. - d, d),
        DVec2::new(4. - d, diagonal - (4. - d)),
        DVec2::new(diagonal - (4. - d), 4. - d),
        DVec2::new(d, 4. - d),
    ]
}

/// Rigid placement of the authored 2D shapes, plus a uniform size factor.
struct Placement {
    rotation: DMat3,
    origin: DVec3,
    size: f64,
}
impl Placement {
    fn identity() -> Self {
        Self {
            rotation: DMat3::IDENTITY,
            origin: DVec3::ZERO,
            size: 1.,
        }
    }
    fn tilted(size: f64) -> Self {
        Self {
            rotation: DMat3::from_axis_angle(DVec3::new(1., 2., 3.).normalize(), 0.7),
            origin: DVec3::new(10., -20., 5.5),
            size,
        }
    }
    fn place(&self, p: DVec2) -> DVec3 {
        self.origin + self.rotation * (p.extend(0.) * self.size)
    }
    fn normal(&self) -> DVec3 {
        self.rotation * DVec3::Z
    }
}

#[test]
fn pentagon_inward_and_outward_in_rotated_translated_and_resized_planes() {
    for placement in [
        Placement::identity(),
        Placement::tilted(1.),
        Placement::tilted(10.),
    ] {
        let source: Vec<DVec3> = PENTAGON
            .iter()
            .map(|&p| placement.place(DVec2::from(p)))
            .collect();
        for d in [0.5, -1., 1.2] {
            let mut e = editor_with(plain(source.clone(), vec![vec![0, 1, 2, 3, 4]]), vec![0]);
            let mut expected = e.scene.clone();
            expected.active_mut().unwrap().mesh.vertices = pentagon_offset(d)
                .into_iter()
                .map(|p| placement.place(p))
                .collect();
            offset(&mut e, d * placement.size).unwrap();
            let tol = 1e-12 * 100. * placement.size;
            assert_scene(&e.scene, &expected, tol);
            assert_boundary_distance(
                &source,
                &loop_positions(&e, 0),
                placement.normal(),
                d * placement.size,
                tol,
            );
        }
    }
}

#[test]
fn regular_hexagon_offsets_its_apothem() {
    // Circumradius 2 => apothem sqrt(3). Offset d keeps the corner angles and
    // gives circumradius (sqrt(3) - d) * 2 / sqrt(3).
    let corner = |radius: f64, k: usize| {
        let angle = (15. + 60. * k as f64).to_radians();
        DVec2::new(radius * angle.cos(), radius * angle.sin())
    };
    let placement = Placement::tilted(3.);
    let source: Vec<DVec3> = (0..6).map(|k| placement.place(corner(2., k))).collect();
    for d in [0.5, -0.75] {
        let mut e = editor_with(plain(source.clone(), vec![(0..6).collect()]), vec![0]);
        let mut expected = e.scene.clone();
        let radius = (3f64.sqrt() - d) * 2. / 3f64.sqrt();
        expected.active_mut().unwrap().mesh.vertices =
            (0..6).map(|k| placement.place(corner(radius, k))).collect();
        offset(&mut e, d * placement.size).unwrap();
        assert_scene(&e.scene, &expected, 1e-10);
        assert_boundary_distance(
            &source,
            &loop_positions(&e, 0),
            placement.normal(),
            d * 3.,
            1e-10,
        );
    }
}

// --- Rich data, unrelated geometry, history and persistence --------------

/// Square face 0 with every supported record kind, a disjoint triangle far
/// away (face 1), an unused distant point 7 and a second unrelated item.
fn rich() -> Editor {
    let mut m = plain(
        vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(2., 2., 0.),
            DVec3::new(0., 2., 0.),
            DVec3::new(500., 0., 0.),
            DVec3::new(501., 0., 0.),
            DVec3::new(500., 1., 0.),
            DVec3::new(4e8, -4e8, 2e8),
        ],
        vec![vec![0, 1, 2, 3], vec![4, 5, 6]],
    );
    m.faces[0] = Face {
        vertices: vec![0, 1, 2, 3],
        uv: Some(vec![
            DVec2::new(0., 0.),
            DVec2::new(1., 0.),
            DVec2::new(1., 1.),
            DVec2::new(0., 1.),
        ]),
        material: "Steel".into(),
        smooth: true,
        part: Some("Panel".into()),
        material_label: Some("Brushed".into()),
    };
    m.faces[1].material = "Far".into();
    m.uv_maps.insert(
        "UV".into(),
        vec![
            vec![
                Some(DVec2::new(0.1, 0.2)),
                None,
                Some(DVec2::new(0.5, 0.7)),
                None,
            ],
            vec![None; 3],
        ],
    );
    let a = &mut m.attributes;
    a.uv_records.insert(
        "UV".into(),
        vec![vec![Some(1), None, Some(2), None], vec![None; 3]],
    );
    a.corner_weights.insert(
        "w".into(),
        vec![vec![Some(0.2), None, None, Some(0.8)], vec![None; 3]],
    );
    a.corner_colors.insert(
        "c".into(),
        vec![
            vec![None, Some(DVec4::new(0.2, 0.3, 0.4, 1.)), None, None],
            vec![None; 3],
        ],
    );
    a.corner_normals.insert(
        "n".into(),
        vec![vec![None, None, Some(DVec3::Z), None], vec![None; 3]],
    );
    a.typed_maps.insert(
        "vectors".into(),
        TypedMap {
            kind: "vector3".into(),
            dimension: 3,
            zero_default: false,
            points: [(1, vec![1., 2., 3.]), (7, vec![0., 0., 1.])].into(),
            corners: vec![
                vec![None, None, None, Some(vec![4., 5., 6.])],
                vec![None; 3],
            ],
        },
    );
    a.point_weights
        .insert("pw".into(), [(0, 0.25), (2, 0.5), (7, 1.)].into());
    a.point_colors
        .insert("pc".into(), [(3, DVec4::new(0.9, 0.1, 0.1, 1.))].into());
    a.relative_morphs
        .insert("lift".into(), [(1, DVec3::new(0., 0., 0.5))].into());
    a.smoothing_labels = vec![Some("Group A".into()), None];
    a.representations = vec![PolygonRepresentation::Subdivs, PolygonRepresentation::Face];
    a.creases = vec![MeshCrease {
        a: 0,
        b: 1,
        weight: 0.4,
    }];
    a.polygon_sets.insert("Panels".into(), [0].into());
    a.vertex_sets.insert("Marked".into(), [0, 2, 7].into());
    let mut e = editor_with(m, vec![0]);
    let selection = e.scene.selection.clone();
    e.dispatch(Command::NewMesh).unwrap();
    let other = e.scene.active_mut().unwrap();
    other.name = "Unrelated".into();
    other.transform.translation = DVec3::new(-3., 1., 2.);
    e.scene.selection = selection;
    e.scene.selection.vertices = vec![2, 7];
    e.scene.selection.edges = vec![(0, 1)];
    e.scene.validate().unwrap();
    e.synchronize_identities().unwrap();
    e
}

fn rich_square_result(e: &Editor, d: f64) -> Scene {
    let mut expected = e.scene.clone();
    let v = &mut expected.items[0].mesh.vertices;
    v[0] = DVec3::new(d, d, 0.);
    v[1] = DVec3::new(2. - d, d, 0.);
    v[2] = DVec3::new(2. - d, 2. - d, 0.);
    v[3] = DVec3::new(d, 2. - d, 0.);
    expected
}

#[test]
fn rich_data_identities_undo_redo_and_native_encoding() {
    let mut e = rich();
    let before = e.scene.clone();
    let ids = e.identities.current.clone();
    let undo = e.history.undo.len();
    let expected = rich_square_result(&e, 0.25);
    offset(&mut e, 0.25).unwrap();
    assert_scene(&e.scene, &expected, 0.);
    assert_eq!(e.identities.current, ids);
    assert_eq!(e.history.undo.len(), undo + 1);
    assert!(e.history.redo.is_empty());
    let after = e.scene.clone();
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(persistence::decode(&bytes).unwrap(), after);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    assert_eq!(e.identities.current, ids);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
    assert_eq!(e.identities.current, ids);
    // A repeat is a second edit applied to the current result.
    let mut twice = rich_square_result(&e, 0.25);
    for p in &mut twice.items[0].mesh.vertices[0..4] {
        *p = DVec3::new(
            if p.x < 1. { 0.75 } else { 1.25 },
            if p.y < 1. { 0.75 } else { 1.25 },
            0.,
        );
    }
    offset(&mut e, 0.5).unwrap();
    assert_scene(&e.scene, &twice, 0.);
    assert_eq!(e.history.undo.len(), undo + 2);
    // Outward from the original.
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    let outward = rich_square_result(&e, -0.5);
    offset(&mut e, -0.5).unwrap();
    assert_scene(&e.scene, &outward, 0.);
}

#[test]
fn rigid_item_transform_uses_scene_distance() {
    // SO-C01-A1 superseded the former scaled/mirrored refusal asserted here;
    // scaled, reflected and parent-composed cases are in
    // `polygon_scale_offset_affine.rs`. Rigid placement keeps the local
    // literal result; the scene-space round trip allows rounding only.
    let mut e = rich();
    e.scene.active_mut().unwrap().transform.rotation = DVec3::new(30., 45., 10.);
    e.scene.active_mut().unwrap().transform.translation = DVec3::new(4., 5., 6.);
    let expected = rich_square_result(&e, 0.25);
    offset(&mut e, 0.25).unwrap();
    assert_scene(&e.scene, &expected, 1e-12);
}

#[test]
fn zero_preserves_everything_including_parked_redo() {
    let mut e = rich();
    offset(&mut e, 0.25).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.history.redo.len(), 1);
    let before = e.clone();
    offset(&mut e, 0.).unwrap();
    assert_no_change(&e, &before);
    let before = e.clone();
    offset(&mut e, -0.).unwrap();
    assert_no_change(&e, &before);
    e.dispatch(Command::Redo).unwrap();
    assert_scene(&e.scene, &rich_square_result(&before, 0.25), 0.);
}

#[test]
fn parked_extrude_redo_survives_zero_and_refusal_and_branches_on_edit() {
    // I2 Begin requires a selected "Texture" UV map on the source.
    let mut quad = plain(square(), vec![vec![0, 1, 2, 3]]);
    quad.uv_maps.insert(
        "Texture".into(),
        vec![vec![
            Some(DVec2::new(0., 0.)),
            Some(DVec2::new(1., 0.)),
            Some(DVec2::new(1., 1.)),
            Some(DVec2::new(0., 1.)),
        ]],
    );
    let mut e = editor_with(quad, vec![0]);
    let source = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(Parameters {
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        extrude_uv_map: Some("Texture".into()),
        extrude_uv_selected: true,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::BeginExtrude).unwrap();
    let active = e.clone();
    assert!(
        offset(&mut e, 0.25).is_err(),
        "active Extrude stage owns input"
    );
    assert!(e == active);
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.extent = DVec3::Z * 0.5;
    e.dispatch(Command::ConfigureExtrude(p)).unwrap();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(!e.i2_active() && e.session.live.is_none() && e.i2_can_redo());
    assert_eq!(e.scene.items[0].mesh, source.items[0].mesh);
    let parked = e.clone();
    offset(&mut e, 0.).unwrap();
    assert_no_change(&e, &parked);
    assert!(e.i2_can_redo());
    let parked = e.clone();
    assert!(offset(&mut e, f64::NAN).is_err());
    assert!(e == parked);
    let mut expected = e.scene.clone();
    expected.items[0].mesh.vertices = vec![
        DVec3::new(0.25, 0.25, 0.),
        DVec3::new(1.75, 0.25, 0.),
        DVec3::new(1.75, 1.75, 0.),
        DVec3::new(0.25, 1.75, 0.),
    ];
    offset(&mut e, 0.25).unwrap();
    assert_scene(&e.scene, &expected, 0.);
    assert!(!e.i2_can_redo());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, parked.scene);
}

#[test]
fn unrelated_geometry_and_unrelated_derived_data_do_not_block() {
    // Tangent data on the far triangle only, and points far beyond the
    // square's own size, leave the square eligible.
    let mut e = rich();
    let a = &mut e.scene.items[0].mesh.attributes;
    a.typed_maps.insert(
        "tangents".into(),
        TypedMap {
            kind: "tangent_basis6".into(),
            dimension: 6,
            zero_default: false,
            points: [(5, vec![1., 0., 0., 0., 1., 0.])].into(),
            corners: vec![
                vec![None; 4],
                vec![Some(vec![1., 0., 0., 0., 1., 0.]), None, None],
            ],
        },
    );
    e.scene.validate().unwrap();
    let expected = rich_square_result(&e, 0.5);
    offset(&mut e, 0.5).unwrap();
    assert_scene(&e.scene, &expected, 0.);
}

// --- Atomic refusals -------------------------------------------------------

fn square() -> Vec<DVec3> {
    vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(2., 2., 0.),
        DVec3::new(0., 2., 0.),
    ]
}

fn xy(points: &[[f64; 2]]) -> Vec<DVec3> {
    points.iter().map(|p| DVec3::new(p[0], p[1], 0.)).collect()
}

/// Dispatch, require a refusal containing `reason`, and require the whole
/// editor (scene, identities, selection, history, redo, tool state) intact.
fn assert_refused(mut e: Editor, distance: f64, reason: &str) {
    let before = e.clone();
    let error = offset(&mut e, distance).unwrap_err();
    assert!(error.contains(reason), "expected '{reason}', got '{error}'");
    assert!(e == before, "refusal mutated the editor: {error}");
}

#[test]
fn bad_selection_and_context_refuse_atomically() {
    let square_mesh = || plain(square(), vec![vec![0, 1, 2, 3]]);
    assert_refused(
        editor_with(square_mesh(), vec![]),
        0.25,
        "Select one polygon",
    );
    assert_refused(editor_with(square_mesh(), vec![]), 0., "Select one polygon");
    let two = plain(
        xy(&[[0., 0.], [1., 0.], [0., 1.], [5., 0.], [6., 0.], [5., 1.]]),
        vec![vec![0, 1, 2], vec![3, 4, 5]],
    );
    // Several polygons are inset together since the fix pass.
    let mut e = editor_with(two, vec![0, 1]);
    offset(&mut e, 0.1).unwrap();
    e.scene.validate().unwrap();
    let mut e = editor_with(square_mesh(), vec![0]);
    e.scene.selection.mode = Mode::Vertex;
    assert_refused(e, 0.25, "Polygon selection mode");
    // Empty item selection never means all meshes.
    let mut e = editor_with(square_mesh(), vec![0]);
    e.dispatch(Command::SelectItem(None)).unwrap();
    // No mesh selected since the scene opened, so none stays active (VP-04).
    e.scene.last_mesh = Default::default();
    e.scene.selection.mode = Mode::Polygon;
    assert_refused(e, 0.25, "visible mesh");
    let mut e = editor_with(square_mesh(), vec![0]);
    e.scene.active_mut().unwrap().visible = false;
    assert_refused(e, 0.25, "visible mesh");
    let mut e = editor_with(square_mesh(), vec![0]);
    e.scene.workplane.fixed = true;
    assert_refused(e, 0.25, "neutral");
    let mut e = editor_with(square_mesh(), vec![0]);
    e.spatial_falloff = Some(Falloff::Linear(LinearFalloff {
        start: DVec3::ZERO,
        end: DVec3::X,
        ..Default::default()
    }));
    assert_refused(e, 0.25, "neutral");
    let mut e = editor_with(square_mesh(), vec![0]);
    e.dispatch(Command::BeginSelection).unwrap();
    assert_refused(e, 0.25, "selection gesture");
}

#[test]
fn live_tool_and_pen_owners_refuse_without_consuming_their_state() {
    let square_mesh = || plain(square(), vec![vec![0, 1, 2, 3]]);
    let mut e = editor_with(square_mesh(), vec![0]);
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    assert!(e.session.live.is_some());
    // UIC-307: a live tool no longer refuses; it settles, the offset runs,
    // and the tool starts again.
    offset(&mut e, 0.25).unwrap();
    assert_eq!(e.session.live.as_ref().map(|l| l.tool), Some(Tool::Bevel));
    // Pen drafts on an empty second mesh; the square stays selected data.
    let mut e = editor_with(square_mesh(), vec![0]);
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::PenStart).unwrap();
    assert!(e.session.pen_draft.is_some());
    assert_refused(e, 0.25, "Pen");
}

#[test]
fn invalid_geometry_and_distances_refuse_atomically() {
    let one = |points: Vec<DVec3>| {
        let corners = (0..points.len() as u32).collect();
        editor_with(plain(points, vec![corners]), vec![0])
    };
    for d in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 2e9] {
        assert_refused(one(square()), d, "finite distance");
    }
    assert_refused(
        one(xy(&[[0., 0.], [4., 0.], [4., 4.], [2., 1.], [0., 4.]])),
        0.1,
        "not strictly convex",
    );
    assert_refused(
        one(xy(&[[0., 0.], [1., 0.], [2., 0.], [2., 2.], [0., 2.]])),
        0.1,
        "not strictly convex",
    );
    let mut bent = square();
    bent[2].z = 0.5;
    assert_refused(one(bent), 0.1, "nonplanar");
    assert_refused(
        one(xy(&[[0., 0.], [2., 0.], [2., 0.], [2., 2.], [0., 2.]])),
        0.1,
        "repeated or degenerate",
    );
    let star: Vec<DVec3> = (0..5)
        .map(|k| {
            let a = (90. + 144. * k as f64).to_radians();
            DVec3::new(a.cos(), a.sin(), 0.)
        })
        .collect();
    assert_refused(one(star), 0.05, "self-intersecting");
    // Collapse to a point and inversion, for the square and the 3-4-5
    // triangle (inradius 1).
    assert_refused(one(square()), 1., "collapses or inverts");
    assert_refused(one(square()), 1.5, "collapses or inverts");
    let triangle = xy(&[[0., 0.], [4., 0.], [0., 3.]]);
    assert_refused(one(triangle.clone()), 1., "collapses or inverts");
    let mut e = one(triangle);
    offset(&mut e, 0.99).unwrap();
    // Out of the scene coordinate range.
    assert_refused(one(square()), -1e9 + 1., "Invalid mesh coordinates");
}

#[test]
fn shared_points_unsupported_elements_and_data_refuse_atomically() {
    let pair = plain(
        xy(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.], [4., 0.], [4., 2.]]),
        vec![vec![0, 1, 2, 3], vec![1, 4, 5, 2]],
    );
    // Connected polygons shrink by moving their points; nothing is added
    // (spec polyscale-offset-2026-11-06 B1).
    let mut e = editor_with(pair, vec![0]);
    offset(&mut e, 0.25).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!((m.vertices.len(), m.faces.len()), (6, 2));
    let corner = plain(
        xy(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.], [3., 3.], [2., 3.]]),
        vec![vec![0, 1, 2, 3], vec![2, 4, 5]],
    );
    let mut e = editor_with(corner, vec![0]);
    offset(&mut e, 0.25).unwrap();
    e.scene.validate().unwrap();
    let mut line = plain(square(), vec![vec![0, 1, 2, 3]]);
    line.attributes.representations = vec![PolygonRepresentation::Polyline {
        closed: true,
        curve: Default::default(),
    }];
    assert_refused(editor_with(line, vec![0]), 0.25, "polyline");
    for (corners, points) in [
        (
            vec![vec![None, Some(vec![1., 0., 0., 0., 1., 0.]), None, None]],
            vec![],
        ),
        (vec![], vec![(2u32, vec![1., 0., 0., 0., 1., 0.])]),
    ] {
        for kind in ["tangent_basis6", "position_derivative6"] {
            let mut m = plain(square(), vec![vec![0, 1, 2, 3]]);
            m.attributes.typed_maps.insert(
                "derived".into(),
                TypedMap {
                    kind: kind.into(),
                    dimension: 6,
                    zero_default: false,
                    points: points.clone().into_iter().collect(),
                    corners: corners.clone(),
                },
            );
            assert_refused(
                editor_with(m, vec![0]),
                0.25,
                "tangent or position-derivative",
            );
        }
    }
    let mut e = editor_with(plain(square(), vec![vec![0, 1, 2, 3]]), vec![0]);
    e.scene.selection.faces = vec![9];
    // A stale index is rejected by scene validation before any edit.
    assert_refused(e, 0.25, "Invalid component selection");
}
