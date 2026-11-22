//! Painting stage 2's regression gate: the sculpt brushes' tips and dab
//! spacing moved into the shared brush engine (brush_engine.rs, PNT-42)
//! and must give exactly what they gave before. The old code is copied
//! here as it was (sculpt.rs and sculpt_ui.rs `lay_dabs` before the move)
//! and compared value for value.
use glam::{DVec3, Vec2};
use operandi_core::brush_engine::{DabPlace, Spacing};
use operandi_core::sculpt::{Tip, TipKind, TipShape};

/// The old `TipShape::weight`.
fn old_shape(shape: TipShape, t: f64) -> f64 {
    use operandi_core::sculpt_blender::curve::Curve;
    let t = t.clamp(0., 1.);
    let curve = match shape {
        TipShape::Linear => Curve::Linear,
        TipShape::Smooth => Curve::Smooth,
        TipShape::Bulge => Curve::Sphere,
        TipShape::Fat => return 1. - t.powi(4),
        TipShape::Sharp => Curve::Sharp,
        TipShape::VerySharp => Curve::Pow4,
        TipShape::Custom => unreachable!("new in stage 2"),
    };
    curve.strength(t, 1.)
}

/// The old Schlick bias.
fn old_bias(b: f64, x: f64) -> f64 {
    let b = b.clamp(0.01, 0.99);
    let x = x.clamp(0., 1.);
    if (b - 0.5).abs() < 1e-12 {
        return x;
    }
    x / ((1. / b - 2.) * (1. - x) + 1.)
}

/// The old `Tip::weight`.
fn old_weight(tip: &Tip, t: f64) -> f64 {
    if !(t.is_finite() && t <= 1.) {
        return 0.;
    }
    let t = t.max(0.);
    let base = match tip.kind {
        TipKind::Smooth => old_shape(tip.shape, t),
        TipKind::Hard => {
            let soft = tip.soft_border.clamp(0., 1.);
            let edge = 1. - soft;
            if soft <= 0. || t <= edge {
                1.
            } else {
                let u = ((t - edge) / soft).clamp(0., 1.);
                1. - u * u * (3. - 2. * u)
            }
        }
    };
    old_bias(tip.bias, base)
}

#[test]
fn sculpt_dab_weights_are_unchanged() {
    let mut checked = 0;
    for kind in [TipKind::Smooth, TipKind::Hard] {
        for shape in TipShape::ALL {
            for bias in [0., 0.2, 0.5, 0.73, 1.] {
                for soft in [0., 0.2, 0.5, 1.] {
                    let tip = Tip {
                        kind,
                        shape,
                        bias,
                        soft_border: soft,
                        ..Tip::default()
                    };
                    for i in -5..=210 {
                        let t = f64::from(i) / 200.;
                        assert_eq!(
                            tip.weight(t).to_bits(),
                            old_weight(&tip, t).to_bits(),
                            "{kind:?} {shape:?} bias {bias} soft {soft} t {t}"
                        );
                        checked += 1;
                    }
                    assert_eq!(tip.weight(f64::NAN), 0.);
                }
            }
        }
    }
    assert!(checked > 10_000);
    // Sculpt's starting tip is unchanged too.
    let d = Tip::default();
    assert_eq!(
        (d.kind, d.shape, d.size, d.soft_border, d.bias),
        (TipKind::Smooth, TipShape::Fat, 10., 0.2, 0.5)
    );
}

/// The old `lay_dabs` loop (centres, normals, pressures and the carry).
#[allow(clippy::type_complexity)]
fn old_spacing(
    from: (Vec2, DVec3, DVec3, f64),
    carry: f64,
    to: (Vec2, DVec3, DVec3, f64),
    gap: f64,
) -> (Vec<(DVec3, DVec3, f64)>, f64) {
    let travel = from.0.distance(to.0) as f64;
    if travel <= 0. {
        return (Vec::new(), carry);
    }
    let mut out = Vec::new();
    let mut s = gap - carry;
    while s <= travel && out.len() < 4000 {
        let t = s / travel;
        out.push((
            from.1.lerp(to.1, t),
            from.2.lerp(to.2, t).normalize_or(to.2),
            from.3 + (to.3 - from.3) * t,
        ));
        s += gap;
    }
    (out, (travel - (s - gap)).max(0.))
}

#[test]
fn sculpt_dab_spacing_is_unchanged() {
    let mut carry = 0.;
    let place = |i: u32| {
        let f = i as f32;
        DabPlace {
            screen: Vec2::new(10. + f * 7.3, 20. + (f * 0.7).sin() * 30.),
            point: DVec3::new(f64::from(i) * 0.11, (f64::from(i) * 0.3).cos(), 0.2),
            normal: DVec3::new(0.1 * f64::from(i), 1., 0.3).normalize(),
            pressure: 0.3 + 0.05 * f64::from(i % 10),
        }
    };
    let (s, first) = Spacing::press(place(0));
    let mut spacer = Spacing { carry: 0., ..s };
    assert_eq!(first, place(0));
    let mut from = place(0);
    for i in 1..60 {
        let to = place(i);
        let gap = 1. + f64::from(i % 7) * 2.5;
        let (old, old_carry) = old_spacing(
            (from.screen, from.point, from.normal, from.pressure),
            carry,
            (to.screen, to.point, to.normal, to.pressure),
            gap,
        );
        let new = spacer.advance(to, || gap, 4000);
        assert_eq!(new.len(), old.len(), "move {i}");
        for (n, o) in new.iter().zip(&old) {
            assert_eq!(
                (n.point, n.normal, n.pressure.to_bits()),
                (o.0, o.1, o.2.to_bits())
            );
        }
        assert_eq!(spacer.carry.to_bits(), old_carry.to_bits());
        carry = old_carry;
        from = to;
    }
    // Off the surface and back: one dab where it came back, carry reset.
    spacer.lift();
    let back = spacer.advance(place(99), || 5., 4000);
    assert_eq!(back, vec![place(99)]);
    assert_eq!(spacer.carry, 0.);
}
