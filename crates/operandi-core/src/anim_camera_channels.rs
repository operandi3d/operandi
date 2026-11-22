//! Camera channels for `item.channel` (so a camera's lens can be keyed,
//! PLAN phase 1 "animate a camera"; TL-81 exports them). Names are
//! Operandi's own (provisional, to be matched to the name map): metres for
//! lengths, like the Camera panel's values.
use crate::Command;
use crate::Scene;
use crate::camera_light::{CameraEdit, CameraSettings};
use crate::channels::{Channel, ChannelType, ChannelValue, Result, Target};

fn camera(scene: &Scene, target: Target) -> Option<&CameraSettings> {
    match target {
        Target::Item(id) => scene.item(id)?.camera.as_ref(),
        _ => None,
    }
}

fn is_camera(scene: &Scene, target: Target) -> bool {
    camera(scene, target).is_some()
}

fn edit(target: Target, edit: CameraEdit) -> Result<Command> {
    match target {
        Target::Item(item) => Ok(Command::EditCamera { item, edit }),
        _ => Err("Not a camera".into()),
    }
}

fn float(v: ChannelValue) -> Result<f64> {
    match v {
        ChannelValue::Float(f) => Ok(f),
        ChannelValue::Int(n) => Ok(n as f64),
        _ => Err("Expected a number".into()),
    }
}

macro_rules! camera_channel {
    ($name:literal, $ty:ident, |$c:ident| $get:expr, $edit:path) => {
        Channel {
            name: $name,
            ty: ChannelType::$ty,
            applies: is_camera,
            get: |s, t| camera(s, t).map(|$c| ChannelValue::Float($get)),
            set: |_, t, v| edit(t, $edit(float(v)?)),
        }
    };
}

/// Camera item channels.
pub static CAMERA: &[Channel] = &[
    camera_channel!(
        "focalLength",
        Distance,
        |c| c.focal_length,
        CameraEdit::FocalLength
    ),
    camera_channel!(
        "focusDistance",
        Distance,
        |c| c.focus_distance,
        CameraEdit::FocusDistance
    ),
    camera_channel!(
        "targetDistance",
        Distance,
        |c| c.target_distance,
        CameraEdit::TargetDistance
    ),
    camera_channel!("fStop", Float, |c| c.dof.f_stop, CameraEdit::FStop),
];
