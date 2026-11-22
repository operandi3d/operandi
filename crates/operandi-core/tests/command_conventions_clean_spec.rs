//! Command conventions (`specifications/command-conventions-2026-11-05`):
//! numbers with unit and percent text (decided Q-CV-02).
use operandi_core::numeric::{Quantity, parse_quantity};

#[test]
fn units_and_percent_are_parsed_properly() {
    let near = |a: f64, b: f64| (a - b).abs() < 1e-12;
    // CV06/CV07: "50cm" is 0.5 m and "250mm" is 0.25 m, not 50 and 250.
    assert!(near(
        parse_quantity("50cm", Quantity::Distance).unwrap(),
        0.5
    ));
    assert!(near(
        parse_quantity("250mm", Quantity::Distance).unwrap(),
        0.25
    ));
    assert!(near(parse_quantity("2m", Quantity::Distance).unwrap(), 2.));
    // CV15: "50%" is fifty percent.
    assert!(near(parse_quantity("50%", Quantity::Percent).unwrap(), 50.));
    // Any other trailing text is refused, never dropped.
    for bad in ["50 apples", "3x", "12cmm", "%50"] {
        assert!(parse_quantity(bad, Quantity::Distance).is_err(), "{bad}");
    }
}

use operandi_core::command_args::{bind, boolean, choice};

fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

const SETPOS: [&str; 4] = ["axis", "value", "workPlane", "worldSpace"];

#[test]
fn named_positional_and_reordered_arguments_agree() {
    // CV01–CV03.
    let a = args("axis:y value:0.5 workPlane:false worldSpace:false");
    let b = args("y 0.5 false false");
    let c = args("worldSpace:false value:0.5 axis:y workPlane:false");
    let want = vec![Some("y"), Some("0.5"), Some("false"), Some("false")];
    assert_eq!(bind("vert.set", &a, &SETPOS).unwrap(), want);
    assert_eq!(bind("vert.set", &b, &SETPOS).unwrap(), want);
    assert_eq!(bind("vert.set", &c, &SETPOS).unwrap(), want);
    // Names are case-insensitive; a name given twice is refused.
    assert!(bind("vert.set", &args("AXIS:y axis:x"), &SETPOS).is_err());
    assert!(bind("vert.set", &args("y 0.5 false false extra"), &SETPOS).is_err());
}

#[test]
fn choices_by_name_or_index_and_unknown_choices_are_refused() {
    let axes = ["x", "y", "z"];
    // CV04: index 1 is y.
    assert_eq!(choice("axis", "1", &axes).unwrap(), 1);
    assert_eq!(choice("axis", "Y", &axes).unwrap(), 1);
    // CV05, decided Q-CV-01: refused with the valid choices.
    let error = choice("axis", "w", &axes).unwrap_err();
    assert!(error.contains("x, y, z"), "{error}");
    assert!(choice("axis", "3", &axes).is_err());
}

#[test]
fn booleans_accept_true_on_and_one() {
    // CV08–CV10.
    for (text, want) in [
        ("on", true),
        ("1", true),
        ("true", true),
        ("off", false),
        ("0", false),
        ("false", false),
    ] {
        assert_eq!(boolean("worldSpace", text).unwrap(), want, "{text}");
    }
    assert!(boolean("worldSpace", "maybe").is_err());
}
