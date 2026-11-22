//! Turning a 3D mouse's raw HID input reports into axis and button state.
//!
//! What is known about the reports is the public HID behaviour of these
//! devices (no vendor SDK is used): each report starts with its report id;
//! id 1 carries the three translation axes, id 2 the three rotation axes,
//! and newer devices send all six in one longer id 1 report; id 3 carries
//! the buttons as a bit mask. Every axis is a signed 16-bit little-endian
//! value, full deflection about ±350.

/// The device's own axes, in its HID frame: X right, Y toward the user,
/// Z down; rotations about those axes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RawAxes {
    /// Translation X, Y, Z.
    pub t: [i16; 3],
    /// Rotation about X, Y, Z.
    pub r: [i16; 3],
}

impl RawAxes {
    pub fn is_zero(&self) -> bool {
        self.t == [0; 3] && self.r == [0; 3]
    }
}

/// The input state the reports so far leave behind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeviceState {
    pub axes: RawAxes,
    /// Bit n set: button n is down.
    pub buttons: u32,
}

/// Which part of the state a report changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Axes,
    Buttons,
}

/// Full deflection, in report units (provisional: the devices' nominal
/// range; harder pushes are clipped to it).
pub const FULL_SCALE: f32 = 350.;

fn three(bytes: &[u8]) -> [i16; 3] {
    let v = |i: usize| i16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]]);
    [v(0), v(1), v(2)]
}

/// Apply one input report (report id first) to `state`. Reports that are
/// too short or of an unknown id are ignored.
pub fn apply_report(state: &mut DeviceState, report: &[u8]) -> Option<Change> {
    let (&id, body) = report.split_first()?;
    match id {
        1 if body.len() >= 12 => {
            state.axes.t = three(&body[..6]);
            state.axes.r = three(&body[6..12]);
            Some(Change::Axes)
        }
        1 if body.len() >= 6 => {
            state.axes.t = three(body);
            Some(Change::Axes)
        }
        2 if body.len() >= 6 => {
            state.axes.r = three(body);
            Some(Change::Axes)
        }
        3 if !body.is_empty() => {
            let mut bytes = [0u8; 4];
            let n = body.len().min(4);
            bytes[..n].copy_from_slice(&body[..n]);
            state.buttons = u32::from_le_bytes(bytes);
            Some(Change::Buttons)
        }
        _ => None,
    }
}

/// Vendor ids 3D mice report: 3Dconnexion's own, and the older devices
/// that were sold under Logitech's id.
pub const VENDOR_3DCONNEXION: u16 = 0x256F;
pub const VENDOR_LOGITECH: u16 = 0x046D;

/// Whether a HID device is a 3D mouse we can read. `usage` is the HID
/// usage page and usage when the platform reports them: a device can
/// expose several collections, and only the multi-axis controller one
/// (Generic Desktop 0x01, usage 0x08) carries the axes.
pub fn is_space_mouse(vendor: u16, product: u16, usage: Option<(u16, u16)>) -> bool {
    let known = match vendor {
        VENDOR_3DCONNEXION => true,
        // Logitech-era 3Dconnexion devices (SpaceNavigator, SpaceExplorer,
        // SpacePilot ...) use this block of product ids.
        VENDOR_LOGITECH => (0xC603..=0xC62F).contains(&product),
        _ => false,
    };
    known && usage.is_none_or(|(page, usage)| page == 0x01 && usage == 0x08)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le(v: [i16; 3]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    #[test]
    fn split_reports_set_translation_then_rotation() {
        let mut s = DeviceState::default();
        let mut t = vec![1];
        t.extend(le([10, -20, 350]));
        assert_eq!(apply_report(&mut s, &t), Some(Change::Axes));
        assert_eq!(s.axes.t, [10, -20, 350]);
        assert_eq!(s.axes.r, [0; 3]);
        let mut r = vec![2];
        r.extend(le([-1, 2, -350]));
        assert_eq!(apply_report(&mut s, &r), Some(Change::Axes));
        assert_eq!(s.axes.t, [10, -20, 350], "rotation leaves translation");
        assert_eq!(s.axes.r, [-1, 2, -350]);
    }

    #[test]
    fn a_combined_report_sets_all_six_axes() {
        let mut s = DeviceState::default();
        let mut b = vec![1];
        b.extend(le([1, 2, 3]));
        b.extend(le([-4, -5, -6]));
        assert_eq!(apply_report(&mut s, &b), Some(Change::Axes));
        assert_eq!(
            s.axes,
            RawAxes {
                t: [1, 2, 3],
                r: [-4, -5, -6]
            }
        );
    }

    #[test]
    fn buttons_are_a_little_endian_bit_mask() {
        let mut s = DeviceState::default();
        assert_eq!(apply_report(&mut s, &[3, 0b10, 0]), Some(Change::Buttons));
        assert_eq!(s.buttons, 2, "right button of a two-button puck");
        apply_report(&mut s, &[3, 0, 1]);
        assert_eq!(s.buttons, 1 << 8);
        apply_report(&mut s, &[3, 0, 0]);
        assert_eq!(s.buttons, 0, "release");
    }

    #[test]
    fn short_empty_and_unknown_reports_change_nothing() {
        let mut s = DeviceState::default();
        apply_report(&mut s, &[1, 5, 0, 6, 0, 7, 0]);
        let before = s;
        for report in [
            &[][..],
            &[1, 1, 2, 3],
            &[2, 9],
            &[3],
            &[23, 1, 2, 3, 4, 5, 6],
        ] {
            assert_eq!(apply_report(&mut s, report), None, "{report:?}");
            assert_eq!(s, before);
        }
    }

    #[test]
    fn only_3d_mice_on_the_multi_axis_collection_are_picked() {
        // SpaceMouse Compact.
        assert!(is_space_mouse(0x256F, 0xC635, Some((1, 8))));
        assert!(is_space_mouse(0x256F, 0xC635, None));
        // Its other collections, and a Logitech mouse.
        assert!(!is_space_mouse(0x256F, 0xC635, Some((1, 2))));
        assert!(!is_space_mouse(0x256F, 0xC635, Some((0xFF00, 1))));
        assert!(!is_space_mouse(0x046D, 0xB034, Some((1, 8))));
        // SpaceNavigator under the old id.
        assert!(is_space_mouse(0x046D, 0xC626, Some((1, 8))));
    }
}
