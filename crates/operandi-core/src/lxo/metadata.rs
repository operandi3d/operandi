//! Revision 5's exact non-modeling metadata profiles. No tag is executed, and
//! mesh display size is never used as geometry or item-transform scale.
use super::{decode::Raw, *};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug)]
pub struct Tag {
    pub id: [u8; 4],
    pub value: String,
    pub offset: usize,
}
#[derive(Debug)]
pub struct Size {
    pub owner: u32,
    pub action: Option<u32>,
    pub kind: u16,
    pub bits: Option<u32>,
    pub offset: usize,
}
pub fn tag_allowed(owner: &str, id: [u8; 4], value: &str) -> bool {
    // Editor Color (note 10, IC-Q1): an item tag on any item type, value a
    // colour name; unknown names read as none.
    if &id == b"COLR" {
        return true;
    }
    matches!(
        (owner, &id, value),
        ("scene", b"LOCK", "locked")
            | ("scene", b"PRST" | b"PPRT", "DeleteMe")
            | ("polyRender" | "shaderFolder", b"LOCK", "locked")
            | (
                "shaderFolder",
                b"CUE ",
                "envShaders" | "library" | "nodes" | "lightShaders" | "bakeItems",
            )
    )
}
pub fn validate_sizes(
    raw: &Raw<'_>,
    bad: &mut BTreeMap<u32, Vec<String>>,
    c: &mut Control<'_>,
) -> Result<()> {
    let mut by_owner: BTreeMap<u32, Vec<&Size>> = BTreeMap::new();
    for size in &raw.sizes {
        by_owner.entry(size.owner).or_default().push(size);
    }
    for item in raw.items.values().filter(|item| item.kind == "mesh") {
        c.check("mesh display size consensus")?;
        let records = by_owner.remove(&item.id).unwrap_or_default();
        let base = records.iter().find(|record| record.action.is_none());
        let valid = records.len() == 2
            && base.is_some_and(|base| {
                let allowed = matches!(
                    (raw.profile.as_str(), base.bits),
                    ("Modo 16", Some(0x3f800000)) | ("Modo 17", Some(0 | 0x3f800000))
                );
                allowed
                    && records.iter().all(|record| {
                        record.kind == 2
                            && record.bits == base.bits
                            && record.action.is_none_or(|id| {
                                raw.actions
                                    .iter()
                                    .find(|action| action.id == id)
                                    .is_some_and(|action| {
                                        action.name == "setup" && action.kind == "setup"
                                    })
                            })
                    })
                    && records
                        .iter()
                        .filter(|record| record.action.is_some())
                        .count()
                        == 1
            });
        if !valid {
            bad.entry(item.id).or_default().push(format!(
                "unsupported/missing/conflicting exact mesh display size profile at payloads {:?}",
                records
                    .iter()
                    .take(4)
                    .map(|record| record.offset)
                    .collect::<Vec<_>>()
            ));
        }
    }
    Ok(())
}
pub fn account(raw: &Raw<'_>, rejected: &BTreeSet<u32>, report: &mut Report) {
    let mut owners: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    for item in raw
        .items
        .values()
        .filter(|item| !rejected.contains(&item.id))
    {
        for tag in &item.tags {
            if report.metadata.tag_samples.len() < 16 {
                report.metadata.tag_samples.push(format!(
                    "ITEM {} {}={} at byte {}",
                    item.id,
                    String::from_utf8_lossy(&tag.id),
                    tag.value,
                    tag.offset
                ));
            }
            report.metadata.tag_records += 1;
            report.metadata.tag_settings += 1; // Duplicate owner/tag keys were refused during decoding.
            *report
                .metadata
                .tags
                .entry(String::from_utf8_lossy(&tag.id).into_owned())
                .or_default() += 1;
            owners.entry(item.kind.clone()).or_default().insert(item.id);
        }
        if item.kind == "mesh" {
            report.metadata.size_settings += 1;
            report.metadata.size_records += 2;
        }
    }
    report.metadata.tag_owners = owners
        .into_iter()
        .map(|(kind, ids)| (kind, ids.len()))
        .collect();
}

#[cfg(test)]
mod colr_tests {
    use super::tag_allowed;

    /// IC-Q1: the Editor Color tag is admitted on every item type (so a
    /// file whose locators or materials carry one isn't refused), whatever
    /// the name; other unknown tags still are.
    #[test]
    fn colr_is_admitted_on_any_item() {
        for owner in ["mesh", "locator", "advancedMaterial", "camera", "light"] {
            assert!(tag_allowed(owner, *b"COLR", "red"));
            assert!(tag_allowed(owner, *b"COLR", "teal"));
        }
        assert!(!tag_allowed("locator", *b"XXXX", "red"));
    }
}
