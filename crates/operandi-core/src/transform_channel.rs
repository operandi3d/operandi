//! `transform.channel` (name map: "Transform Channel"; UIC-160..163,
//! PD-Q12): one item transform channel by name. The channel names are the
//! ones Operandi's LXO files use (`lxo/export.rs`): pos.X/Y/Z (metres),
//! rot.X/Y/Z (the item's stored Euler angles; a bare value is radians, the
//! map's rule for angles, `deg`/`°` keep their unit) and scl.X/Y/Z (a
//! Texture Locator's Size). Provisional: the map's own channel list isn't
//! on hand.
use crate::Result;
use crate::scene::Transform;

/// Every channel name, in the order a report lists them.
pub const CHANNELS: [&str; 9] = [
    "pos.X", "pos.Y", "pos.Z", "rot.X", "rot.Y", "rot.Z", "scl.X", "scl.Y", "scl.Z",
];

/// (part 0 = pos, 1 = rot, 2 = scl; axis 0..3) for a channel name, any case.
fn parse(name: &str) -> Result<(usize, usize)> {
    let k = CHANNELS
        .iter()
        .position(|c| c.eq_ignore_ascii_case(name.trim()))
        .ok_or_else(|| {
            format!(
                "transform.channel: unknown channel {name}; channels are {}",
                CHANNELS.join(", ")
            )
        })?;
    Ok((k / 3, k % 3))
}

/// The channel's value (rotation in radians).
pub fn get(t: &Transform, name: &str) -> Result<f64> {
    let (part, axis) = parse(name)?;
    Ok([t.translation, t.rotation, t.scale][part][axis])
}

/// Set the channel from its command-line text.
pub fn set(t: &mut Transform, name: &str, value: &str) -> Result<()> {
    let (part, axis) = parse(name)?;
    let v = if part == 1 {
        crate::command_names::script_angle(value)?.to_radians()
    } else {
        value
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("transform.channel: {value} isn't a number"))?
    };
    if !v.is_finite() {
        return Err(format!("transform.channel: {value} isn't a number"));
    }
    match part {
        0 => t.translation[axis] = v,
        1 => t.rotation[axis] = v,
        _ => t.scale[axis] = v,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_values_and_units() {
        let mut t = Transform::default();
        set(&mut t, "scl.X", "2").unwrap();
        set(&mut t, "POS.z", "-0.5").unwrap();
        set(&mut t, "rot.Y", "90deg").unwrap();
        assert_eq!(t.scale.x, 2.);
        assert_eq!(t.translation.z, -0.5);
        assert!((t.rotation.y - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        set(&mut t, "rot.X", "1").unwrap();
        assert_eq!(get(&t, "rot.X").unwrap(), 1., "a bare angle is radians");
        assert!(set(&mut t, "size.X", "1").is_err());
        assert!(set(&mut t, "pos.X", "far").is_err());
    }
}
