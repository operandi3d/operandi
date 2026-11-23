use operandi_core::rig_expression::Expr;
use std::cell::RefCell;
use std::f64::consts::{E, FRAC_PI_2, FRAC_PI_4, PI};

fn eval(source: &str) -> f64 {
    Expr::parse(source).unwrap().eval(&|_| None)
}

fn assert_value(source: &str, expected: f64) {
    let actual = eval(source);
    assert!(
        (actual - expected).abs() <= 1e-9,
        "{source}: expected {expected}, got {actual}"
    );
}

#[test]
fn numbers_and_operator_precedence() {
    for (source, expected) in [
        ("12 + 1.5 + .5 + 2e-3 + 3E+1", 44.002),
        (" 2 +\n3 * 4 ", 14.0),
        ("(2 + 3) * 4", 20.0),
        ("10 - 3 + 2", 9.0),
        ("24 / 6 / 2", 2.0),
        ("-2^2", -4.0),
        ("(-2)^2", 4.0),
        ("2^3^2", 512.0),
        ("2^-2", 0.25),
        ("2 * -+3^2", -18.0),
        ("!2^0", 0.0),
        ("1 < 2 + 1", 1.0),
        ("2 == 3 < 4", 0.0),
        ("1 || 0 && 0", 1.0),
        ("0 || 2 < 3 && 4 == 4 ? 7 : 8", 7.0),
        ("1 ? 0 : 1 ? 2 : 3", 0.0),
        ("0 ? 1 : 0 ? 2 : 3", 3.0),
        ("1 ? 0 ? 2 : 3 : 4", 3.0),
    ] {
        assert_value(source, expected);
    }
}

#[test]
fn comparisons_and_logic_return_numeric_flags() {
    for (source, expected) in [
        ("2 == 2", 1.0),
        ("2 == 3", 0.0),
        ("2 != 3", 1.0),
        ("2 != 2", 0.0),
        ("2 < 3", 1.0),
        ("2 < 2", 0.0),
        ("2 <= 2", 1.0),
        ("3 <= 2", 0.0),
        ("3 > 2", 1.0),
        ("2 > 2", 0.0),
        ("2 >= 2", 1.0),
        ("2 >= 3", 0.0),
        ("-2 && 3", 1.0),
        ("-2 && 0", 0.0),
        ("0 || -3", 1.0),
        ("0 || 0", 0.0),
        ("!(-0)", 1.0),
        ("!(-3)", 0.0),
    ] {
        assert_value(source, expected);
    }
}

#[test]
fn conditional_and_logic_only_read_needed_variables() {
    for (source, expected, reads) in [
        ("yes ? chosen : skipped", 5.0, &["yes", "chosen"][..]),
        ("no ? skipped : chosen", 5.0, &["no", "chosen"][..]),
        ("no && skipped || chosen", 1.0, &["no", "chosen"][..]),
        ("yes || skipped && skipped_too", 1.0, &["yes"][..]),
        ("yes && chosen", 1.0, &["yes", "chosen"][..]),
        ("no || chosen", 1.0, &["no", "chosen"][..]),
    ] {
        let accessed = RefCell::new(Vec::new());
        let result = Expr::parse(source).unwrap().eval(&|name| {
            accessed.borrow_mut().push(name.to_owned());
            match name {
                "yes" => Some(-1.0),
                "chosen" => Some(5.0),
                _ => None,
            }
        });
        assert_eq!(result, expected, "{source}");
        assert_eq!(accessed.into_inner(), reads, "{source}");
    }
}

#[test]
fn unary_functions() {
    for (source, expected) in [
        ("sin(pi/2)", 1.0),
        ("cos(pi)", -1.0),
        ("tan(pi/4)", 1.0),
        ("asin(1)", FRAC_PI_2),
        ("acos(-1)", PI),
        ("atan(1)", FRAC_PI_4),
        ("sinh(1)", 1.175_201_193_643_801_4),
        ("cosh(1)", 1.543_080_634_815_243_7),
        ("tanh(1)", 0.761_594_155_955_764_9),
        ("asinh(1)", 0.881_373_587_019_543),
        ("acosh(2)", 1.316_957_896_924_816_6),
        ("atanh(.5)", 0.549_306_144_334_054_8),
        ("log2(8)", 3.0),
        ("log10(100)", 2.0),
        ("log(1000)", 3.0),
        ("ln(e)", 1.0),
        ("exp(1)", E),
        ("sqrt(9)", 3.0),
        ("sign(-3)", -1.0),
        ("sign(0)", 0.0),
        ("sign(3)", 1.0),
        ("rint(2.5)", 2.0),
        ("rint(3.5)", 4.0),
        ("rint(-2.5)", -2.0),
        ("rint(-3.5)", -4.0),
        ("abs(-3)", 3.0),
        ("floor(-1.2)", -2.0),
        ("ceil(-1.2)", -1.0),
    ] {
        assert_value(source, expected);
    }
}

#[test]
fn functions_with_multiple_or_variable_arguments() {
    for (source, expected) in [
        ("min(-4)", -4.0),
        ("min(3, -4, 2)", -4.0),
        ("max(3)", 3.0),
        ("max(2, -4, 3)", 3.0),
        ("sum(-4)", -4.0),
        ("sum(3, -4, 2)", 1.0),
        ("avg(-4)", -4.0),
        ("avg(3, -4, 2)", 1.0 / 3.0),
        ("mod(5, 3)", 2.0),
        ("mod(-5, 3)", 1.0),
        ("mod(5, -3)", -1.0),
        ("mod(-5, -3)", -2.0),
        ("clamp(-3, -2, 2)", -2.0),
        ("clamp(3, -2, 2)", 2.0),
        ("clamp(1, -2, 2)", 1.0),
        ("clamp(1, 0, 0)", 0.0),
    ] {
        assert_value(source, expected);
    }
}

#[test]
fn variables_constants_and_wheel_formula() {
    let expr = Expr::parse("z2 + sign(frame) + time + pi + e + _r + z2 + (0 ? A : é)").unwrap();
    assert_eq!(expr.variables(), ["A", "_r", "frame", "time", "z2", "é"]);
    assert_eq!(
        Expr::parse("pi + e").unwrap().variables(),
        Vec::<String>::new()
    );
    assert_value("pi", PI);
    assert_value("e", E);
    assert_value("missing + 4", 4.0);
    assert_eq!(
        Expr::parse("time + frame + Time")
            .unwrap()
            .eval(&|name| match name {
                "time" => Some(0.5),
                "frame" => Some(12.0),
                _ => None,
            }),
        12.5,
    );

    let wheel = Expr::parse("dist / (2*pi*r) * 360").unwrap();
    let result = wheel.eval(&|name| match name {
        "dist" => Some(PI),
        "r" => Some(0.5),
        _ => None,
    });
    assert!((result - 360.0).abs() <= 1e-9);
}

#[test]
fn reports_syntax_errors_at_character_positions() {
    for (source, position, message) in [
        ("", 0, "Expected a value"),
        ("  ", 2, "Expected a value"),
        ("1 +", 3, "Expected a value"),
        ("(1", 2, "Missing )"),
        ("1)", 1, "Unexpected token"),
        ("1 2", 2, "Unexpected token"),
        ("foo(1)", 0, "Unknown function foo"),
        ("SIN(1)", 0, "Unknown function SIN"),
        ("sin(1,2)", 0, "Wrong number of arguments for sin"),
        ("1 + avg()", 4, "Wrong number of arguments for avg"),
        ("mod(1)", 0, "Wrong number of arguments for mod"),
        ("clamp(1,2)", 0, "Wrong number of arguments for clamp"),
        ("noise()", 0, "Wrong number of arguments for noise"),
        ("noise(1,2,3,4)", 0, "Wrong number of arguments for noise"),
        ("a ? b", 5, "Missing :"),
        ("a ? b : ", 8, "Expected a value"),
        ("#", 0, "Unknown character"),
        ("é + #", 4, "Unknown character"),
        ("1e+", 3, "Expected an exponent digit"),
    ] {
        let error = Expr::parse(source).unwrap_err();
        assert_eq!(error.position, position, "{source}: {error:?}");
        assert_eq!(error.message, message, "{source}");
    }
}

#[test]
fn limits_nesting_and_accepts_long_flat_chains() {
    let nested = format!("{}1{}", "(".repeat(256), ")".repeat(256));
    assert_value(&nested, 1.0);
    for (source, position) in [
        (format!("{}1{}", "(".repeat(4000), ")".repeat(4000)), 257),
        (format!("{}1", "-".repeat(4000)), 257),
        (format!("{}1", "1^".repeat(4000)), 514),
        (format!("{}1", "0?0:".repeat(4000)), 1026),
    ] {
        let error = Expr::parse(&source).unwrap_err();
        assert_eq!(error.position, position);
        assert_eq!(error.message, "Expression is nested too deeply");
    }
    assert_value(&format!("{}1", "1+".repeat(4000)), 4001.0);
}

#[test]
fn noise_is_deterministic_bounded_and_continuous() {
    for (source, nearby, across_boundary, other_side) in [
        (
            "noise(0.5)",
            "noise(0.5001)",
            "noise(-0.0001)",
            "noise(0.0001)",
        ),
        (
            "noise(0.5, -1.25)",
            "noise(0.5001, -1.2499)",
            "noise(1.3, 0.9999)",
            "noise(1.3, 1.0001)",
        ),
        (
            "noise(0.5, -1.25, 3.125)",
            "noise(0.5001, -1.2499, 3.1251)",
            "noise(-1.3, 0.4, -1.0001)",
            "noise(-1.3, 0.4, -0.9999)",
        ),
    ] {
        let actual = eval(source);
        assert_eq!(actual, eval(source));
        assert!((-1.0..=1.0).contains(&actual), "{source} = {actual}");
        assert!((actual - eval(nearby)).abs() < 0.001);
        assert!((eval(across_boundary) - eval(other_side)).abs() < 1e-6);
    }
    assert_value("noise(-0)", eval("noise(0)"));
    assert!((eval("noise(0)") - eval("noise(1)")).abs() > 1e-6);
}

#[test]
fn nonfinite_results_and_invalid_bounds_are_safe() {
    assert_eq!(eval("1/0"), f64::INFINITY);
    assert!(eval("0/0").is_nan());
    assert!(eval("sqrt(-1)").is_nan());
    assert!(eval("mod(1, 0)").is_nan());
    assert!(eval("clamp(1, 2, 0)").is_nan());
    assert!(eval("clamp(1, 0/0, 2)").is_nan());
    assert!(eval("noise(1/0)").is_nan());
}
