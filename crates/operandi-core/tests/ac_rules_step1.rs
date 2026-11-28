//! Action Center rule sheet v6, step 1: the Selection axes (A-SEL),
//! Selection Border's centre (C-BORDER), Symmetry for both (S-01..S-04),
//! and the Pivot and Element checks. Every test goes through the editor's
//! own transform path: choose the Action Center, activate Move, Rotate or
//! Scale, read the handle's frame and apply values.
//!
//! The fixture is the bug report's shape: a box-shaped prong that sticks
//! out at an angle on the + side of the Symmetry plane, and its mirror
//! image on the − side. Selection Border with Symmetry X gave it a tilted
//! gizmo, because the axes were voted by both prongs' polygons.
use glam::{DMat3, DQuat, DVec3, dvec3};
use operandi_core::*;

fn quad(v: [u32; 4]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// How the prong is turned: its length, width and thickness run along
/// `turn() * X`, `* Z` and `* Y`.
fn turn() -> DQuat {
    DQuat::from_rotation_y(0.35) * DQuat::from_rotation_z(0.2)
}
/// Where the prong's first corner sits.
const PLACE: DVec3 = dvec3(4., 1., 0.5);
/// The prong's rings along its length. Its cross-section is 1 thick (Y)
/// and 2 wide (Z), so no two spreads tie.
const RINGS: [f64; 5] = [0., 1.5, 3., 4.5, 6.];
const POINTS: u32 = 20;
const FACES: u32 = 18;

/// A point of the prong from its own coordinates (along, thick, wide).
fn at(along: f64, thick: f64, wide: f64) -> DVec3 {
    PLACE + turn() * dvec3(along, thick, wide)
}

/// The prong: point `ring * 4 + corner`; sleeve polygon `ring * 4 + side`
/// between ring and ring + 1, then the end cap (16) and the start cap (17).
/// With `mirrored`, its mirror image across X follows: points + 20,
/// polygons + 18.
fn prong_mesh(mirrored: bool) -> Mesh {
    let mut m = Mesh::default();
    for along in RINGS {
        for (thick, wide) in [(0., 0.), (1., 0.), (1., 2.), (0., 2.)] {
            m.vertices.push(at(along, thick, wide));
        }
    }
    for ring in 0..4u32 {
        for side in 0..4u32 {
            let (a, b) = (ring * 4 + side, ring * 4 + (side + 1) % 4);
            m.faces.push(quad([a, b, b + 4, a + 4]));
        }
    }
    m.faces.push(quad([16, 17, 18, 19]));
    m.faces.push(quad([3, 2, 1, 0]));
    if mirrored {
        for i in 0..POINTS as usize {
            let p = m.vertices[i];
            m.vertices.push(dvec3(-p.x, p.y, p.z));
        }
        for f in 0..FACES as usize {
            let mut v: Vec<u32> = m.faces[f].vertices.iter().map(|v| v + POINTS).collect();
            v.reverse();
            m.faces.push(quad([v[0], v[1], v[2], v[3]]));
        }
    }
    m
}

/// The polygons of the prong's outer half: the sleeve from ring 2 on and
/// the end cap. Their border is ring 2.
fn end_polygons() -> Vec<u32> {
    (8..=16).collect()
}

fn editor(mesh: Mesh, faces: Vec<u32>, symmetry: Symmetry, center: CenterMode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = faces;
    e.dispatch(Command::Symmetry(symmetry)).unwrap();
    e.dispatch(Command::ActionContext(ActionContext {
        center,
        axis: AxisOrientation::Selection,
    }))
    .unwrap();
    e
}

/// Both prongs with both outer halves selected, as a Symmetry selection is.
fn pair(symmetry: Symmetry, center: CenterMode) -> Editor {
    let mut faces = end_polygons();
    faces.extend(end_polygons().iter().map(|f| f + FACES));
    editor(prong_mesh(true), faces, symmetry, center)
}

fn frame(e: &Editor) -> ActionFrame {
    e.session.live.as_ref().unwrap().action_frame.unwrap()
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.to_vec()
}
fn close(a: DVec3, b: DVec3, what: &str) {
    assert!(a.distance(b) < 1e-9, "{what}: {a} != {b}");
}
fn same_axes(got: DMat3, want: DMat3, what: &str) {
    close(got.x_axis, want.x_axis, &format!("{what}, X"));
    close(got.y_axis, want.y_axis, &format!("{what}, Y"));
    close(got.z_axis, want.z_axis, &format!("{what}, Z"));
}
fn preview(e: &mut Editor, delta: DVec3) {
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = delta;
    e.dispatch(Command::Preview(p)).unwrap();
}

/// The prong's own axes in the world: the frame A-SEL gives its outer
/// half. X along its length (the largest spread), Y across its width, Z
/// through its thickness (the least), signed by A-SEL's rules: X has a
/// positive world-X part; the polygons' normals cancel but for the end
/// cap, which is square to Z, so Z follows X's rule; Y = Z × X.
fn prong_axes() -> DMat3 {
    let (x, z) = (turn() * DVec3::X, turn() * -DVec3::Y);
    assert!(x.x > 0. && z.x > 0.);
    DMat3::from_cols(x, z.cross(x), z)
}

/// Every column of `basis` runs along one of the prong's own directions
/// (turned by `world`), and the frame is right-handed: a gizmo square to
/// the faces.
fn assert_square_to_the_faces(basis: DMat3, world: DMat3) {
    for column in [basis.x_axis, basis.y_axis, basis.z_axis] {
        let best = [DVec3::X, DVec3::Y, DVec3::Z]
            .into_iter()
            .map(|a| (world * (turn() * a)).normalize().dot(column).abs())
            .fold(0., f64::max);
        assert!(best > 1. - 1e-9, "{column} is tilted off the faces");
    }
    close(basis.x_axis.cross(basis.y_axis), basis.z_axis, "handedness");
}

/// Must-pass 1 (the screenshot): Selection Border with Symmetry X on box
/// polygons. The handle sits in the middle of the + side's border ring
/// (C-BORDER, S-01) and its arrows are square to that prong's faces
/// (A-SEL from the + side's points only, S-02). Selection gives the same
/// axes about the + half's box middle.
#[test]
fn selection_border_with_symmetry_is_square_to_the_faces() {
    for tool in [Tool::Move, Tool::Rotate, Tool::Scale] {
        let mut e = pair(Symmetry::x(), CenterMode::SelectionBorder);
        e.dispatch(Command::Activate(tool)).unwrap();
        let f = frame(&e);
        assert_square_to_the_faces(f.basis, DMat3::IDENTITY);
        same_axes(f.basis, prong_axes(), "Selection Border");
        close(f.center, at(3., 0.5, 1.), "the + border ring's middle");

        let mut e = pair(Symmetry::x(), CenterMode::Selection);
        e.dispatch(Command::Activate(tool)).unwrap();
        same_axes(frame(&e).basis, prong_axes(), "Selection");
        close(
            frame(&e).center,
            at(4.5, 0.5, 1.),
            "the + half's box middle",
        );
    }
}

/// Must-pass 1 on a turned, moved item: the Symmetry plane lives in the
/// item's own space (S-08) and the axes are read in the world, so the
/// arrows are still square to the prong's faces where it now is.
#[test]
fn selection_border_with_symmetry_on_a_turned_item() {
    let mut e = pair(Symmetry::x(), CenterMode::SelectionBorder);
    let item = e.scene.active_mut().unwrap();
    item.transform.translation = dvec3(-3., 2., 1.);
    item.transform.rotation = dvec3(0.4, -0.9, 0.25);
    let world = e.scene.world_matrix(e.scene.active().unwrap().id).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let f = frame(&e);
    assert_square_to_the_faces(f.basis, DMat3::from_mat4(world));
    close(
        f.center,
        world.transform_point3(at(3., 0.5, 1.)),
        "the + border ring's middle, in the world",
    );
}

/// Must-pass 2: the same with Symmetry off (one prong's outer half
/// selected) is square to the faces too, with the same frame and centre.
#[test]
fn selection_border_without_symmetry_is_square_to_the_faces() {
    for (center, want) in [
        (CenterMode::SelectionBorder, at(3., 0.5, 1.)),
        (CenterMode::Selection, at(4.5, 0.5, 1.)),
    ] {
        let mut e = editor(prong_mesh(true), end_polygons(), Symmetry::OFF, center);
        e.dispatch(Command::Activate(Tool::Move)).unwrap();
        let f = frame(&e);
        same_axes(f.basis, prong_axes(), center.label());
        close(f.center, want, "centre");
        // A Move along the handle's X slides the selection down the prong.
        let before = points(&e);
        preview(&mut e, dvec3(0.5, 0., 0.));
        let after = points(&e);
        for v in 8..POINTS as usize {
            close(
                after[v],
                before[v] + turn() * dvec3(0.5, 0., 0.),
                "moved point",
            );
        }
        for v in (0..8).chain(POINTS as usize..2 * POINTS as usize) {
            assert_eq!(after[v], before[v], "point {v} isn't selected");
        }
    }
}

/// Must-pass 3: a closed band of polygons round a protrusion is never
/// refused, by Selection or by Selection Border, in the menu check or on
/// any transform tool. Its normals cancel, so the frame comes from its
/// shape: X along the prong for a band longer than the prong is wide.
/// Selection Border's centre is halfway between the band's two border
/// rings.
#[test]
fn a_closed_band_round_a_protrusion_is_never_refused() {
    // Two rows of the sleeve (rings 1 to 3), then one row (rings 1 to 2).
    for (band, long) in [
        ((4..12).collect::<Vec<u32>>(), true),
        ((4..8).collect(), false),
    ] {
        for center in [CenterMode::Selection, CenterMode::SelectionBorder] {
            let context = ActionContext {
                center,
                axis: AxisOrientation::Selection,
            };
            for tool in [Tool::Move, Tool::Rotate, Tool::Scale, Tool::Transform] {
                let mut e = editor(prong_mesh(false), band.clone(), Symmetry::OFF, center);
                e.validate_action_context(context).unwrap();
                e.dispatch(Command::Activate(tool)).unwrap();
                // Also offered while the tool is on.
                e.validate_action_context(context).unwrap();
                let f = frame(&e);
                assert_square_to_the_faces(f.basis, DMat3::IDENTITY);
                let end = if long { 4.5 } else { 3. };
                close(f.center, at((1.5 + end) / 2., 0.5, 1.), "the band's middle");
                if long {
                    close(f.basis.x_axis, turn() * DVec3::X, "X along the prong");
                }
                if tool != Tool::Transform {
                    let delta = if tool == Tool::Scale {
                        dvec3(1.5, 1., 1.)
                    } else {
                        dvec3(0.25, 0., 0.)
                    };
                    let before = points(&e);
                    preview(&mut e, delta);
                    assert_ne!(points(&e), before, "{tool:?} did something");
                    e.dispatch(Command::Drop).unwrap();
                }
            }
        }
    }
}

/// Must-pass 4: with Symmetry the + side is the primary side for a
/// selection spread evenly over both (S-03), its points take the values as
/// entered in the + side's frame, and the − side gets the mirror image
/// (S-04): a Move along the mirror axis flips, a rotation turns the other
/// way, scale factors are the same.
#[test]
fn symmetry_drives_from_the_plus_side_and_mirrors_the_other() {
    let mirror = |p: DVec3| dvec3(-p.x, p.y, p.z);
    for center in [CenterMode::Selection, CenterMode::SelectionBorder] {
        for tool in [Tool::Move, Tool::Rotate, Tool::Scale] {
            let mut e = pair(Symmetry::x(), center);
            e.dispatch(Command::Activate(tool)).unwrap();
            assert_eq!(e.symmetry_primary(&e.scene), Side::Positive);
            let f = frame(&e);
            assert!(f.center.x > 0., "the handle is on the + side");
            let delta = match tool {
                Tool::Move => dvec3(0.5, 0.2, -0.1),
                Tool::Rotate => dvec3(30., -10., 20.),
                _ => dvec3(1.5, 0.5, 2.),
            };
            let before = points(&e);
            preview(&mut e, delta);
            let after = points(&e);
            let turned = DQuat::from_euler(
                glam::EulerRot::XYZ,
                delta.x.to_radians(),
                delta.y.to_radians(),
                delta.z.to_radians(),
            );
            let entered = |p: DVec3| {
                let local = f.basis.transpose() * (p - f.center);
                match tool {
                    Tool::Move => p + f.basis * delta,
                    Tool::Rotate => f.center + f.basis * (turned * local),
                    _ => f.center + f.basis * (local * delta),
                }
            };
            for v in 8..POINTS as usize {
                let twin = v + POINTS as usize;
                close(after[v], entered(before[v]), "+ side, as entered");
                close(after[twin], mirror(after[v]), "− side, the mirror image");
                if tool == Tool::Move {
                    let (plus, minus) = (after[v] - before[v], after[twin] - before[twin]);
                    assert!(plus.x > 0.1 && (plus.x + minus.x).abs() < 1e-9);
                }
            }
            // Unselected points stay; nothing was added (S-06).
            assert_eq!(after.len(), before.len());
            for v in (0..8).chain(POINTS as usize..POINTS as usize + 8) {
                assert_eq!(after[v], before[v]);
            }
        }
    }
}

/// S-02: the Selection axes read only the + side. A selection with
/// nothing there (only the − prong) gets World axes, and its centre comes
/// from the − side (S-01).
#[test]
fn symmetry_with_nothing_on_the_plus_side_is_world() {
    let minus: Vec<u32> = end_polygons().iter().map(|f| f + FACES).collect();
    let mut e = editor(
        prong_mesh(true),
        minus,
        Symmetry::x(),
        CenterMode::SelectionBorder,
    );
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let f = frame(&e);
    assert_eq!(f.basis, DMat3::IDENTITY);
    let ring = at(3., 0.5, 1.);
    close(
        f.center,
        dvec3(-ring.x, ring.y, ring.z),
        "the − ring's middle",
    );
}

/// A centre placed on the − side makes − the primary side (S-07); the
/// Selection axes then come from that side's points, so the gizmo is
/// square to the prong it sits on and centre, axes and the driving side
/// agree. Resetting the centre brings the + side's frame back.
#[test]
fn a_centre_placed_on_the_minus_side_takes_that_sides_axes() {
    let mirror = |p: DVec3| dvec3(-p.x, p.y, p.z);
    let mut e = pair(Symmetry::x(), CenterMode::SelectionBorder);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let plus = frame(&e);
    let placed = mirror(at(3., 0.5, 1.));
    e.dispatch(Command::SetActionCenter(placed)).unwrap();
    let f = frame(&e);
    close(f.center, placed, "as placed");
    assert_eq!(e.symmetry_primary(&e.scene), Side::Negative);
    // The − prong's own directions are the + prong's, mirrored.
    for column in [f.basis.x_axis, f.basis.y_axis, f.basis.z_axis] {
        let best = [DVec3::X, DVec3::Y, DVec3::Z]
            .into_iter()
            .map(|a| mirror(turn() * a).dot(column).abs())
            .fold(0., f64::max);
        assert!(best > 1. - 1e-9, "{column} is tilted off the − prong");
    }
    assert!(f.basis.x_axis.x > 0., "X's sign rule");
    // The − side takes the Move as entered; the + side mirrors it.
    let before = points(&e);
    preview(&mut e, dvec3(0.5, 0., 0.));
    let after = points(&e);
    for v in 8..POINTS as usize {
        let twin = v + POINTS as usize;
        close(
            after[twin],
            before[twin] + f.basis.x_axis * 0.5,
            "− as entered",
        );
        close(after[v], mirror(after[twin]), "+ mirrored");
    }
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::SetActionCenter(placed)).unwrap();
    e.dispatch(Command::ResetActionCenter).unwrap();
    assert_eq!(frame(&e), plus);
}

/// Turning Symmetry on or off while the tool is on moves the handle and
/// its Selection axes to the new rule at once.
#[test]
fn the_live_frame_follows_a_symmetry_change() {
    let mut e = pair(Symmetry::OFF, CenterMode::SelectionBorder);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    // Off: one frame over both prongs, centred between the two rings.
    let ring = at(3., 0.5, 1.);
    close(
        frame(&e).center,
        dvec3(0., ring.y, ring.z),
        "halfway between the rings",
    );
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    same_axes(frame(&e).basis, prong_axes(), "after Symmetry X");
    close(frame(&e).center, ring, "the + ring");
}

/// A flat sheet of quads, `columns` wide (at the given X positions) and two
/// rows deep (Z 0..2): point `row * columns + column`, polygon
/// `row * (columns - 1) + column`.
fn sheet(xs: &[f64]) -> Mesh {
    let n = xs.len() as u32;
    let mut m = Mesh::default();
    for z in 0..3 {
        for &x in xs {
            m.vertices.push(dvec3(x, 0., z as f64));
        }
    }
    for row in 0..2 {
        for column in 0..n - 1 {
            let i = row * n + column;
            m.faces.push(quad([i, i + n, i + n + 1, i + 1]));
        }
    }
    m
}

fn border_centre(mesh: Mesh, mode: Mode, select: impl FnOnce(&mut Selection)) -> DVec3 {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    select(&mut e.scene.selection);
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::SelectionBorder,
        axis: AxisOrientation::Selection,
    }))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    let centre = frame(&e).center;
    // The tool really scales about it.
    let before = points(&e);
    preview(&mut e, DVec3::splat(0.5));
    let moved = transform_vertices(&e.session.live.as_ref().unwrap().base).unwrap();
    for &v in &moved {
        let v = v as usize;
        close(
            points(&e)[v],
            centre + (before[v] - centre) * 0.5,
            "scaled about the centre",
        );
    }
    centre
}

/// C-BORDER: the border edges are the selected polygons' edges that an
/// unselected polygon uses too, and the centre is the middle of the box
/// round their points.
#[test]
fn selection_border_centre_rules() {
    // The near row of an unevenly divided sheet: its border is the line
    // Z = 1, with points at X = 0, 1, 2 and 10. The box middle is X = 5
    // (their average would be 3.25); the sheet's open edges don't count.
    let centre = border_centre(sheet(&[0., 1., 2., 10.]), Mode::Polygon, |s| {
        s.faces = vec![0, 1, 2];
    });
    close(centre, dvec3(5., 0., 1.), "box middle of the border line");
    // One corner polygon: only its two inner edges are border.
    let centre = border_centre(sheet(&[0., 1., 2., 10.]), Mode::Polygon, |s| {
        s.faces = vec![0];
    });
    close(centre, dvec3(0.5, 0., 0.5), "the two inner edges' box");
    // The whole sheet has no border edge: as Selection (the box middle).
    let centre = border_centre(sheet(&[0., 1., 2., 10.]), Mode::Polygon, |s| {
        s.faces = (0..6).collect();
    });
    close(centre, dvec3(5., 0., 1.), "no border: the selection's box");
    // A whole closed mesh: as Selection.
    let centre = border_centre(prong_mesh(false), Mode::Polygon, |s| {
        s.faces = (0..FACES).collect();
    });
    close(centre, at(3., 0.5, 1.), "closed mesh: the selection's box");
    // Edge and Vertex mode: as Selection.
    let centre = border_centre(sheet(&[0., 1., 2., 10.]), Mode::Edge, |s| {
        s.edges = vec![(0, 1), (1, 2)];
    });
    close(centre, dvec3(1., 0., 0.), "edges: their box");
    let centre = border_centre(sheet(&[0., 1., 2., 10.]), Mode::Vertex, |s| {
        s.vertices = vec![0, 3, 7];
    });
    close(centre, dvec3(5., 0., 0.5), "points: their box");
}

/// A-SEL on plain selections, through Move's handle: a flat patch gets Z
/// along its normal and X along its length; a line of edges gets X along
/// the line; one point, or a shape with no longer side (a square's four
/// corners still have a normal, a cube has nothing), falls back by the
/// tie rules.
#[test]
fn selection_axes_follow_the_selections_shape() {
    let axes = |mesh: Mesh, mode: Mode, select: fn(&mut Selection)| {
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        e.scene.active_mut().unwrap().mesh = mesh.into();
        e.dispatch(Command::Mode(mode)).unwrap();
        select(&mut e.scene.selection);
        e.dispatch(Command::ActionContext(ActionContext {
            center: CenterMode::Selection,
            axis: AxisOrientation::Selection,
        }))
        .unwrap();
        e.dispatch(Command::Activate(Tool::Move)).unwrap();
        frame(&e).basis
    };
    // The sheet's polygons face up (+Y). A strip along Z, 1 by 2.
    let strip = axes(sheet(&[0., 1., 2., 3.]), Mode::Polygon, |s| {
        s.faces = vec![0, 3];
    });
    assert_eq!(strip, DMat3::from_cols(DVec3::Z, DVec3::X, DVec3::Y));
    // A square patch: X and Y tie, so X goes nearest world X.
    let square = axes(sheet(&[0., 1., 2., 3.]), Mode::Polygon, |s| {
        s.faces = vec![0];
    });
    assert_eq!(square, DMat3::from_cols(DVec3::X, -DVec3::Z, DVec3::Y));
    // Edges in a row along X.
    let line = axes(sheet(&[0., 1., 2., 3.]), Mode::Edge, |s| {
        s.edges = vec![(0, 1), (1, 2)];
    });
    assert_eq!(line.x_axis, DVec3::X);
    // One point: World.
    let point = axes(sheet(&[0., 1., 2., 3.]), Mode::Vertex, |s| {
        s.vertices = vec![5];
    });
    assert_eq!(point, DMat3::IDENTITY);
}

/// Must-pass 5: Pivot keeps the item's own turned axes (A-PIVOT, kept on
/// purpose) about the item's own position, with Symmetry on or off (S-10).
#[test]
fn pivot_keeps_the_items_turned_axes() {
    for symmetry in [Symmetry::OFF, Symmetry::x()] {
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        let item = e.scene.active_mut().unwrap();
        item.mesh = sheet(&[1., 2., 3., 4.]).into();
        item.transform.translation = dvec3(2., 1., -1.);
        item.transform.rotation = dvec3(0.3, 0.5, -0.2);
        let id = item.id;
        let world = e.scene.world_matrix(id).unwrap();
        e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        e.scene.selection.faces = vec![0, 1];
        e.dispatch(Command::Symmetry(symmetry)).unwrap();
        e.dispatch(Command::ActionContext(ActionContext {
            center: CenterMode::Pivot,
            axis: AxisOrientation::Item,
        }))
        .unwrap();
        e.dispatch(Command::Activate(Tool::Move)).unwrap();
        let f = frame(&e);
        close(f.center, dvec3(2., 1., -1.), "the item's position");
        let turned = DMat3::from_mat4(world);
        assert!(turned.x_axis.distance(DVec3::X) > 0.1, "really turned");
        for (got, want) in [
            (f.basis.x_axis, turned.x_axis),
            (f.basis.y_axis, turned.y_axis),
            (f.basis.z_axis, turned.z_axis),
        ] {
            close(got, want, "the item's own axis");
        }
        if symmetry != Symmetry::OFF {
            // The fixed centre stays on the item's position under
            // Symmetry too (S-10); the moves are checked without it.
            continue;
        }
        // Move X slides the points along the item's own X.
        let before = points(&e);
        preview(&mut e, dvec3(0.7, 0., 0.));
        let after = points(&e);
        for v in [0, 1, 2, 4, 5, 6] {
            close(after[v], before[v] + dvec3(0.7, 0., 0.), "along item X");
        }
        e.dispatch(Command::Cancel).unwrap();
        // Rotate Y turns them about the item's own Y through its position.
        e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
        preview(&mut e, dvec3(0., 90., 0.));
        let after = points(&e);
        for v in [0, 1, 2, 4, 5, 6] {
            close(
                after[v],
                DQuat::from_rotation_y(90f64.to_radians()) * before[v],
                "about item Y",
            );
        }
    }
}

/// Must-pass 6: Element with nothing picked yet is the world origin with
/// World axes (C-ELEM, A-ELEM), also on a moved, turned item.
#[test]
fn element_before_any_pick_is_the_world_origin_and_world_axes() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let item = e.scene.active_mut().unwrap();
    item.mesh = sheet(&[1., 2., 3., 4.]).into();
    item.transform.translation = dvec3(2., 1., -1.);
    item.transform.rotation = dvec3(0.3, 0.5, -0.2);
    let id = item.id;
    let world = e.scene.world_matrix(id).unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0, 1];
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Element(PickedPoint::default()),
        axis: AxisOrientation::World,
    }))
    .unwrap();
    for tool in [Tool::Move, Tool::Rotate, Tool::Scale] {
        e.dispatch(Command::Activate(tool)).unwrap();
        let f = frame(&e);
        assert_eq!(f.center, DVec3::ZERO, "{tool:?}");
        assert_eq!(f.basis, DMat3::IDENTITY, "{tool:?}");
        e.dispatch(Command::Cancel).unwrap();
    }
    // Rotate Z by 90° swings the selection round the world origin.
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    let before = points(&e);
    preview(&mut e, dvec3(0., 0., 90.));
    let after = points(&e);
    for v in [0, 1, 2, 4, 5, 6] {
        let p = world.transform_point3(before[v]);
        close(
            world.transform_point3(after[v]),
            dvec3(-p.y, p.x, p.z),
            "a quarter turn about world Z",
        );
    }
}
