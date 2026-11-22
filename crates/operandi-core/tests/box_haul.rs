//! Box side hauling may cross the opposite side: the box is built on the
//! other side with outward-facing polygons.
use glam::DVec3;
use operandi_core::primitive::{BoxBounds, BoxHandleMode, segmented_cube};

#[test]
fn hauling_flat_box_downward_builds_an_outward_box_below() {
    // Flat box drawn on the ground (zero height), haul +Y below the plane.
    let flat = BoxBounds::new(DVec3::ZERO, DVec3::new(2., 0., 2.)).unwrap();
    let down = flat
        .haul_side(1, true, -1.5, BoxHandleMode::Resize)
        .unwrap();
    assert_eq!(down.min.y, -1.5);
    assert_eq!(down.max.y, 0.);
    let mesh = segmented_cube(down.position(), down.size(), [1; 3]).unwrap();
    mesh.validate().unwrap();
    // Every face points away from the box center.
    let center = down.position();
    for f in 0..mesh.faces.len() {
        assert!(mesh.normal(f).dot(mesh.center(f) - center) > 0.);
    }
}

#[test]
fn hauling_past_the_opposite_side_flips_and_from_center_stays_symmetric() {
    let b = BoxBounds::new(DVec3::ZERO, DVec3::splat(2.)).unwrap();
    let flipped = b.haul_side(2, true, -3., BoxHandleMode::Resize).unwrap();
    assert_eq!((flipped.min.z, flipped.max.z), (-3., -1.));
    let centered = b
        .haul_side(2, true, -3., BoxHandleMode::FromCenter)
        .unwrap();
    assert_eq!((centered.min.z, centered.max.z), (-3., 3.));
}
