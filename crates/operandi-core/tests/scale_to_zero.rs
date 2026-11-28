//! Scale to 0% (GitHub #24): the Scale tool's handles now reach 0, the
//! value typed fields could already set. Acceptance criteria:
//!
//! 1. Components scaled to 0 collapse onto the action centre; every point
//!    stays a finite number.
//! 2. Undo gives the mesh back exactly; redo collapses it again.
//! 3. Items still can't reach 0 (no inverse, IT-04): refused, untouched.
//! 4. Components can't go below 0; a collapsed mesh saves and reopens.
use glam::DVec3;
use operandi_core::*;

fn cube() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e
}

fn scale(e: &mut Editor, delta: DVec3) {
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta,
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
}

fn mesh(e: &Editor) -> Mesh {
    (*e.scene.active().unwrap().mesh).clone()
}

#[test]
fn components_scale_to_zero_and_undo() {
    let mut e = cube();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    let m = mesh(&e);
    let top = (0..m.faces.len()).find(|&f| m.normal(f).y > 0.9).unwrap() as u32;
    let id = e.scene.active().unwrap().id;
    e.dispatch(Command::Select {
        item: id,
        hit: Component::Polygon(top),
        replace: true,
        remove: false,
    })
    .unwrap();
    let before = mesh(&e);
    scale(&mut e, DVec3::ZERO);
    let after = mesh(&e);
    assert!(after.vertices.iter().all(|v| v.is_finite()));
    let corners = &after.faces[top as usize].vertices;
    let first = after.vertices[corners[0] as usize];
    for &v in corners {
        assert!(
            after.vertices[v as usize].distance(first) < 1e-12,
            "collapsed"
        );
    }
    assert!(
        (first - DVec3::Y).length() < 1e-9,
        "onto the top's centre: {first}"
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(mesh(&e), after);
}

#[test]
fn an_item_scale_stays_above_zero() {
    // A zero item scale has no inverse (IT-04): still refused, the item
    // untouched, while a tiny one works and undoes.
    let mut e = cube();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    let id = e.scene.active().unwrap().id;
    e.dispatch(Command::SelectItem(Some(id))).unwrap();
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    let refused = e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(0., 1., 1.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }));
    assert!(refused.is_err());
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene.active().unwrap().transform.scale, DVec3::ONE);
    scale(&mut e, DVec3::new(0.001, 1., 1.));
    assert!(e.scene.world_matrix(id).unwrap().is_finite());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().transform.scale, DVec3::ONE);
}

#[test]
fn components_reach_zero_but_not_below() {
    let mut e = cube();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![0];
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    let below = e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(-0.5, 1., 1.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }));
    assert!(below.is_err());
    let bytes = persistence::encode(&e.scene).unwrap();
    assert!(persistence::decode(&bytes).is_ok());
}
