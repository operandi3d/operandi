//! Material distance rows read units and arithmetic (material editing
//! item 1): "25 mm" in Absorption Distance was stored as 25 m.
use operandi_core::numeric::{Quantity, parse_quantity};
use operandi_core::shader_tree::{Channel, MaterialChannels};

fn metres(text: &str) -> f64 {
    parse_quantity(text, Quantity::Distance).expect(text)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

#[test]
fn distance_text_reads_every_unit() {
    assert!(close(metres("25 mm"), 0.025));
    assert!(close(metres("25mm"), 0.025));
    assert!(close(metres("2.5 cm"), 0.025));
    assert!(close(metres("1.5 m"), 1.5));
    assert!(close(metres("2 km"), 2000.));
    assert!(close(metres("40 um"), 40e-6));
    assert!(close(metres("40 µm"), 40e-6));
    assert!(close(metres("1 in"), 0.0254));
    assert!(close(metres("2 ft"), 0.6096));
}

#[test]
fn distance_text_does_arithmetic() {
    assert!(close(metres("10 mm + 5 mm"), 0.015));
    assert!(close(metres("1 m / 4"), 0.25));
    assert!(close(metres("(1 cm + 1 mm) * 2"), 0.022));
}

#[test]
fn plain_number_is_in_the_default_unit() {
    // The default units show metres; a bare number is metres.
    let units = operandi_core::units::current();
    let unit = units.default_unit.metres(&units);
    assert!(close(metres("3"), 3. * unit));
}

#[test]
fn bad_distance_text_is_refused() {
    assert!(parse_quantity("25 parsecs", Quantity::Distance).is_err());
    assert!(parse_quantity("", Quantity::Distance).is_err());
}

#[test]
fn material_distance_rows_are_distance_channels() {
    // Every row these fields drive is a metres channel, so the panel's
    // distance field (units, arithmetic) applies to each.
    let mut m = MaterialChannels::default();
    let names: Vec<&str> = m
        .channels()
        .into_iter()
        .filter(|(_, c)| matches!(c, Channel::Distance(_)))
        .map(|(n, _)| n)
        .collect();
    for want in [
        "Absorption Distance",
        "Subsurface Distance",
        "Bump Amplitude",
        "Displacement Distance",
        "Rounded Edge Width",
    ] {
        assert!(names.contains(&want), "{want} is not a distance row");
    }
}
