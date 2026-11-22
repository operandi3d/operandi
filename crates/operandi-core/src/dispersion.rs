//! Physical dispersion figures for Material presets (MAT-ED-Q01, Jason
//! 2026-11-12). A gem's "dispersion" is the B−G figure: n at 430.8 nm
//! minus n at 686.7 nm. OpenPBR takes an Abbe number,
//! V = (n_d − 1) / (n_F − n_C), with transmission_dispersion_scale 1 for
//! the physical amount (the effective Abbe number is V ÷ scale).
//!
//! The F−C to B−G ratio comes from a two-term Cauchy fit,
//! n(λ) = A + B / λ²: a difference n(λ1) − n(λ2) is B (1/λ1² − 1/λ2²), so
//! (n_F − n_C) / (n_B − n_G) = (1/λF² − 1/λC²) / (1/λB² − 1/λG²) ≈ 0.585.
//! Check: diamond (n 2.417, B−G 0.044) gives V ≈ 55, quartz (n 1.544,
//! B−G 0.013) V ≈ 72; published values are about 55 and 68–70.

/// Fraunhofer F (486.13 nm) and C (656.27 nm) lines.
const F_NM: f64 = 486.13;
const C_NM: f64 = 656.27;
/// The gemological B (686.7 nm) and G (430.8 nm) lines.
const B_NM: f64 = 686.7;
const G_NM: f64 = 430.8;
/// Abbe Number's range on the Material.
pub const ABBE_RANGE: (f64, f64) = (1., 1000.);

/// (n_F − n_C) ÷ (n_G − n_B) under a Cauchy fit (≈ 0.585).
pub fn fc_per_bg() -> f64 {
    let k = |nm: f64| 1. / (nm * nm);
    (k(F_NM) - k(C_NM)) / (k(G_NM) - k(B_NM))
}

/// The Abbe number for refractive index `ior` and B−G figure `figure`,
/// kept in the Material's range. None when there is no dispersion to
/// show (figure 0 or less, or an index of 1 or less).
pub fn abbe_from_figure(ior: f64, figure: f64) -> Option<f64> {
    if !(figure > 0. && ior > 1. && ior.is_finite() && figure.is_finite()) {
        return None;
    }
    let abbe = (ior - 1.) / (fc_per_bg() * figure);
    Some(abbe.clamp(ABBE_RANGE.0, ABBE_RANGE.1))
}

/// The B−G figure a material shows now: from its index, Abbe number and
/// Dispersion scale (the effective Abbe number is `abbe / scale`).
pub fn figure_of(ior: f64, abbe: f64, scale: f64) -> f64 {
    if !(ior > 1. && abbe > 0. && scale > 0.) {
        return 0.;
    }
    (ior - 1.) * scale / (fc_per_bg() * abbe)
}

/// Apply B−G figure `figure` to material `m` (a Dispersion preset):
/// Dispersion to 100 % and Abbe Number from its Refractive Index. An
/// error (and no change) when the index is 1 or less.
pub fn apply_figure(
    m: &mut crate::shader_tree::MaterialChannels,
    figure: f64,
) -> crate::Result<()> {
    let abbe = abbe_from_figure(m.refractive_index, figure)
        .ok_or("Dispersion needs a Refractive Index above 1")?;
    m.more.dispersion = 1.;
    m.openpbr.dispersion_abbe_number = abbe;
    Ok(())
}

/// The B−G figure material `m` shows now (what (new) saves).
pub fn figure_of_material(m: &crate::shader_tree::MaterialChannels) -> f64 {
    figure_of(
        m.refractive_index,
        m.openpbr.dispersion_abbe_number,
        m.more.dispersion,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preset_sets_full_scale_and_the_abbe_number() {
        let mut m = crate::shader_tree::MaterialChannels {
            refractive_index: 2.42,
            ..Default::default()
        };
        apply_figure(&mut m, 0.044).unwrap();
        assert_eq!(m.more.dispersion, 1.);
        let v = m.openpbr.dispersion_abbe_number;
        assert!((52.0..60.0).contains(&v), "{v}");
        assert!((figure_of_material(&m) - 0.044).abs() < 1e-12);
        // Index 1 (Air): refused, nothing changes.
        let mut air = crate::shader_tree::MaterialChannels {
            refractive_index: 1.,
            ..Default::default()
        };
        let before = air.clone();
        assert!(apply_figure(&mut air, 0.044).is_err());
        assert_eq!(air, before);
    }

    #[test]
    fn ratio_is_the_cauchy_one() {
        let r = fc_per_bg();
        assert!((0.58..0.59).contains(&r), "{r}");
    }

    #[test]
    fn diamond_and_quartz_land_near_their_published_abbe_numbers() {
        let diamond = abbe_from_figure(2.417, 0.044).unwrap();
        assert!((52.0..60.0).contains(&diamond), "diamond {diamond}");
        let quartz = abbe_from_figure(1.544, 0.013).unwrap();
        assert!((65.0..76.0).contains(&quartz), "quartz {quartz}");
        let silica = abbe_from_figure(1.458, 0.01).unwrap();
        assert!((70.0..90.0).contains(&silica), "silica {silica}");
    }

    #[test]
    fn figure_round_trips_and_follows_the_scale() {
        let abbe = abbe_from_figure(2.42, 0.044).unwrap();
        assert!((figure_of(2.42, abbe, 1.) - 0.044).abs() < 1e-12);
        assert!((figure_of(2.42, abbe, 0.5) - 0.022).abs() < 1e-12);
        assert_eq!(figure_of(2.42, abbe, 0.), 0.);
        assert_eq!(figure_of(1., abbe, 1.), 0.);
    }

    #[test]
    fn no_dispersion_without_an_index_or_a_figure() {
        assert_eq!(abbe_from_figure(1.0, 0.044), None);
        assert_eq!(abbe_from_figure(1.5, 0.), None);
        assert_eq!(abbe_from_figure(f64::NAN, 0.01), None);
        // Tiny figures stop at the field's top.
        assert_eq!(abbe_from_figure(1.5, 1e-9), Some(ABBE_RANGE.1));
    }
}
