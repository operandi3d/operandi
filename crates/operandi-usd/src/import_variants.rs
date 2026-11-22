//! Variants and payloads on import (usd-gaps-3): OpenUSD composes the
//! stage with each variant set at its selection (the file's default) and
//! `UsdStage::Open` loads every payload, so the import takes exactly what
//! a USD viewer shows first. This only reports it: which variant sets
//! came in at which selection, and how many prims brought payloads. There
//! is no variant picker yet; other selections need the file edited.
use crate::stage::StageData;

/// Report lines for the stage's variant sets and payloads.
pub fn notes(stage: &StageData) -> Vec<String> {
    let mut out = Vec::new();
    let sets: Vec<String> = stage
        .prims
        .iter()
        .filter(|p| !p.variants.is_empty())
        .map(|p| {
            let chosen: Vec<String> = p
                .variants
                .iter()
                .map(|(set, sel)| {
                    if sel.is_empty() {
                        format!("{set}: none")
                    } else {
                        format!("{set} = {sel}")
                    }
                })
                .collect();
            format!("{} ({})", p.name, chosen.join(", "))
        })
        .collect();
    if !sets.is_empty() {
        const SHOWN: usize = 8;
        let more = sets.len().saturating_sub(SHOWN);
        let mut line = format!(
            "Variant sets at their selections: {}",
            sets[..sets.len().min(SHOWN)].join("; ")
        );
        if more > 0 {
            line.push_str(&format!(" and {more} more"));
        }
        out.push(line);
    }
    let payloads = stage.prims.iter().filter(|p| p.payload).count();
    if payloads > 0 {
        out.push(format!("Payloads loaded: {payloads}"));
    }
    out
}

#[cfg(all(test, feature = "usd"))]
mod tests {
    use std::path::Path;

    /// Variants and payloads (usd-gaps-3): the default selection's geometry
    /// and the payload's come in, and the report names both.
    #[test]
    fn the_default_variant_and_payloads_come_in() {
        let file =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/usd/variants/variants.usda");
        let back = crate::import(&file, &Default::default()).unwrap();
        let notes = back.notes.join("\n");
        assert!(
            notes.contains("Variant sets at their selections: Shelf (size = tall)"),
            "{notes}"
        );
        assert!(notes.contains("Payloads loaded: 1"), "{notes}");
        let tallest = back
            .scene
            .items
            .iter()
            .flat_map(|i| i.mesh.vertices.iter().map(|v| v.y))
            .fold(f64::MIN, f64::max);
        assert!((tallest - 3.0).abs() < 1e-6, "the tall box: {tallest}");
        let triangles = back
            .scene
            .items
            .iter()
            .flat_map(|i| i.mesh.faces.iter())
            .filter(|f| f.vertices.len() == 3)
            .count();
        assert_eq!(triangles, 1, "the payload's lid");
    }
}
