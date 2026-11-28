//! Noise falloff Scale changes the noise under a live Scale tool
//! (GitHub #16).
use super::tests::box_app;
use super::*;
use crate::falloff_panel::{FalloffFields, FalloffP};
use crate::tool_form::{Bind, ToolFields};
use operandi_core::NoiseFalloff;

fn scale_key() -> &'static str {
    "falloff0_noise_scale"
}

fn set_scale(a: &mut Operandi, value: f64) -> f64 {
    let p: FalloffP = FalloffFields::params(a).unwrap();
    let form = FalloffFields::form(a, &p);
    let field = form
        .fields()
        .into_iter()
        .find(|f| f.row.key == scale_key())
        .expect("Scale row");
    let Bind::Number { get, set } = &field.bind else {
        panic!("Scale is a number");
    };
    let mut q = p.clone();
    set(&mut q, value).unwrap();
    FalloffFields::publish(a, q).unwrap();
    FalloffFields::published(a);
    let p = FalloffFields::params(a).unwrap();
    get(&p)
}

fn points(a: &Operandi) -> Vec<DVec3> {
    a.editor.scene.active().unwrap().mesh.vertices.clone()
}

#[test]
fn scale_changes_the_noise_under_a_live_scale() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Vertex));
    let mut mesh =
        operandi_core::primitive::segmented_cube(DVec3::ZERO, DVec3::splat(2.), [6; 3]).unwrap();
    mesh.vertices
        .iter_mut()
        .for_each(|v| *v += DVec3::splat(0.013));
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    let n = a.editor.scene.active().unwrap().mesh.vertices.len() as u32;
    a.editor.scene.selection.vertices = (0..n).collect();
    a.run(Command::SpatialFalloff(Some(Falloff::Noise(
        NoiseFalloff::default(),
    ))));
    a.activate(Tool::Scale);
    let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
    p.scale_delta = DVec3::splat(1.);
    a.run(Command::Preview(p));
    let one = points(&a);
    assert_eq!(set_scale(&mut a, 0.37), 0.37);
    let small = points(&a);
    assert_ne!(one, small, "Scale 0.37 changes the noise: {}", a.status);
    assert_eq!(set_scale(&mut a, 3.), 3.);
    assert_ne!(small, points(&a));
}

/// The bug: typed or scrubbed falloff values were never confirmed while a
/// tool's own rows sat above them (only the tool's rows were), so Enter did
/// nothing. Falloff rows now confirm with the tool's.
#[test]
fn typing_a_falloff_value_under_a_live_tool_confirms_it() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Vertex));
    let n = a.editor.scene.active().unwrap().mesh.vertices.len() as u32;
    a.editor.scene.selection.vertices = (0..n).collect();
    a.run(Command::SpatialFalloff(Some(Falloff::Noise(
        NoiseFalloff::default(),
    ))));
    a.activate(Tool::Scale);
    let noise = |a: &Operandi| match a.editor.spatial_falloff {
        Some(Falloff::Noise(n)) => n.scale,
        _ => panic!("Noise is active"),
    };
    a.fields.insert(scale_key().into(), "0.37 m".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert_eq!(noise(&a), 0.37);
    // The same with no tool open: the falloff rows are the whole panel.
    a.run(Command::Drop);
    assert!(a.editor.session.live.is_none());
    a.fields.insert(scale_key().into(), "2 m".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert_eq!(noise(&a), 2.);
    // A refused value is marked on its row and keeps the last good one.
    a.fields.insert(scale_key().into(), "0".into());
    assert!(!a.confirm_fields());
    assert!(a.field_errors.contains_key(scale_key()));
    assert_eq!(noise(&a), 2.);
}
