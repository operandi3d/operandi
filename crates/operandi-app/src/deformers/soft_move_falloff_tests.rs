//! TP-19: a tool row that brings its own falloff (Soft Move) starts the
//! falloff at the tool's spot; in the view the two stay separate parts,
//! the falloff sized by its own Size boxes (FO-41) while the tool's
//! handle keeps the shared centre.
use super::tests::box_app;
use super::*;
use crate::viewport::falloff_view::FalloffHandle;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}

/// Soft Move armed, then placed by a click at `at`.
fn placed(at: DVec3) -> Operandi {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Vertex));
    a.click_to_start = true;
    a.camera.view = View::Front;
    a.run(Command::ActivatePreset("xfrm.magnet".into()));
    assert!(
        a.editor.is_armed(),
        "{} {:?}",
        a.status,
        a.editor.session.live.as_ref().map(|l| l.tool)
    );
    a.run(Command::ToolClick {
        point: at,
        plane: 2,
    });
    assert!(!a.editor.is_armed());
    a
}

#[test]
fn tp19_falloff_and_move_handle_start_together() {
    let at = DVec3::new(-0.4, 0.3, 0.);
    let a = placed(at);
    let centre = a.transform_handle_center();
    assert!((centre - at).length() < 1e-9, "{centre}");
    let Some(Falloff::Radial(f)) = a.editor.spatial_falloff else {
        panic!("Soft Move brings a Radial falloff");
    };
    assert!((a.falloff_space().transform_point3(f.center) - at).length() < 1e-9);
}

/// The press on the shared centre goes to the Move handle; the falloff's
/// X box takes its own press and sizes only X.
#[test]
fn tp19_the_x_box_sizes_the_falloff_and_the_centre_stays_the_tools() {
    let r = rect();
    let mut a = placed(DVec3::new(-0.4, 0.3, 0.));
    let centre = a.camera.project(a.transform_handle_center(), r).unwrap().0;
    assert!(
        a.handle_hit(centre, r).is_some(),
        "the Move handle is there"
    );
    assert_eq!(a.falloff_hit(centre, r), None, "the tool's handle wins");

    let handles = a.falloff_handles(r);
    let (i, &(at, kind)) = handles
        .iter()
        .enumerate()
        .find(|(_, h)| h.1 == FalloffHandle::Point(1))
        .unwrap();
    assert!(a.is_size_box(kind));
    let q = a.camera.project(at, r).unwrap().0;
    assert_eq!(a.falloff_hit(q, r), Some(i), "the X box takes the press");
    let Some(start) = a.editor.spatial_falloff else {
        panic!()
    };
    let Falloff::Radial(before) = start else {
        panic!()
    };
    a.drag_falloff(i, start, q, q - vec2(30., 0.), r);
    let Some(Falloff::Radial(after)) = a.editor.spatial_falloff else {
        panic!()
    };
    assert!(after.size.x < before.size.x, "narrower on X");
    assert_eq!((after.size.y, after.size.z), (before.size.y, before.size.z));
    assert_eq!(after.center, before.center, "the centre stays");
    // The move handle is where it was: the parts are separate.
    let still = a.camera.project(a.transform_handle_center(), r).unwrap().0;
    assert_eq!(still, centre);
}

/// Without a live tool nothing competes, so the Centre still takes the
/// press (falloff chosen from the menu, FO-41 as before).
#[test]
fn tp19_with_no_tool_the_centre_is_the_falloffs() {
    let r = rect();
    let mut a = box_app();
    a.run(Command::SpatialFalloff(Some(
        RadialFalloff {
            center: DVec3::ZERO,
            size: DVec3::splat(0.5),
            ..Default::default()
        }
        .into(),
    )));
    a.camera.view = View::Front;
    if a.editor.session.live.is_some() {
        a.run(Command::Drop);
    }
    let q = a.camera.project(DVec3::ZERO, r).unwrap().0;
    assert_eq!(a.falloff_hit(q, r), Some(0));
}
