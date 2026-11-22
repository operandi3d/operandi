//! Revision 3's bounded auxiliary profiles. These are byte profiles, not an
//! envelope evaluator or a general interpretation of gradient extensions.
use super::{decode::Raw, *};
use std::collections::{BTreeMap, BTreeSet};

const RAD: &str = "000000005052452000020001504f5354000200014b4559200008000000003f80000054414e4900100040000000000000000000003f80000054414e4f00140000000000400000000000000000000000000000464c4147000400000f00";
const ROT: &str = "000000005052452000020001504f5354000200014b4559200008000000000000000054414e4900100040000000000000000000000000000054414e4f00140000000000400000000000000000000000000000464c4147000400000f00";
fn exact(bytes: &[u8], hex: &str) -> bool {
    bytes.len() * 2 == hex.len()
        && bytes
            .iter()
            .zip(hex.as_bytes().as_chunks::<2>().0.iter())
            .all(|(b, h)| {
                let digit = |v: u8| if v <= b'9' { v - b'0' } else { v - b'a' + 10 };
                *b == digit(h[0]) * 16 + digit(h[1])
            })
}
fn allowed(kind: &str, name: &str) -> Option<&'static str> {
    match (kind, name) {
        ("scene", "ocioConfig") => Some("nuke-default"),
        ("scene", "def8bitColorspace" | "def16bitColorspace") => Some("nuke-default:sRGB"),
        ("scene", "defFloatColorspace") => Some("nuke-default:linear"),
        ("polyRender", "irrLName" | "irrSName")
        | ("environment", "lpeLabel")
        | ("shaderFolder", "effect") => Some(""),
        ("polyRender", "outPat") => Some(".[<pass>.][<output>.][<LR>.]<FFFF>"),
        ("envMaterial", "effect") => Some("envColor"),
        _ => None,
    }
}
pub fn validate(
    raw: &Raw<'_>,
    bad: &mut BTreeMap<u32, Vec<String>>,
    c: &mut Control<'_>,
) -> Result<UnreferencedEnvelopes> {
    if !raw.unknown_item_records.is_empty() {
        return Err(Error::new(
            "unknown_reference_semantics",
            0,
            &format!(
                "Unadmitted ITEM records prevent envelope reference closure: {:?}",
                raw.unknown_item_records
                    .iter()
                    .take(16)
                    .map(|id| String::from_utf8_lossy(id))
                    .collect::<Vec<_>>()
            ),
        ));
    }
    let mut referenced = BTreeSet::new();
    for reference in &raw.envelope_references {
        c.check("envelope reference closure")?;
        if !raw.items.contains_key(&reference.owner)
            || !raw.envelopes.contains_key(&reference.envelope)
        {
            return Err(Error::new(
                "invalid_envelope_reference",
                reference.offset,
                &format!(
                    "{} owner {} action {:?} channel {} references absent ENVL {}",
                    reference.record,
                    reference.owner,
                    reference.action,
                    reference.channel,
                    reference.envelope
                ),
            ));
        }
        referenced.insert(reference.envelope);
    }

    let mut setup_by_owner: BTreeMap<u32, BTreeMap<&str, Vec<&super::decode::Gradient>>> =
        BTreeMap::new();
    let mut string_values = BTreeMap::new();
    for a in &raw.actions {
        for (owner, g) in &a.gradients {
            c.check("envelope-profile comparison")?;
            referenced.insert(g.envelope);
            if a.name != "setup" || a.kind != "setup" {
                bad.entry(*owner)
                    .or_default()
                    .push("unsupported gradient action context".into());
            } else {
                setup_by_owner
                    .entry(*owner)
                    .or_default()
                    .entry(&g.name)
                    .or_default()
                    .push(g);
            }
        }
        for s in &a.strings {
            c.check("omission inventory")?;
            if string_values
                .insert((s.owner, &s.name), &s.value)
                .is_some_and(|v| v != &s.value)
            {
                return Err(Error::new(
                    "unknown_dependencies",
                    0,
                    "Conflicting string channel representations",
                ));
            }
        }
    }
    let mut complete_mesh_pairs = 0;
    for item in raw.items.values() {
        c.check("envelope-profile comparison")?;
        let setup = setup_by_owner.remove(&item.id).unwrap_or_default();
        for g in &item.gradients {
            referenced.insert(g.envelope);
        }
        if item.kind != "mesh" {
            if !item.gradients.is_empty() || !setup.is_empty() {
                return Err(Error::new(
                    "unknown_dependencies",
                    0,
                    "Gradient on an unclassified non-mesh owner",
                ));
            }
            continue;
        }
        let mut valid = item.gradients.len() == 2 && setup.len() == 2;
        for (name, body) in [("radGrad", RAD), ("rotGrad", ROT)] {
            let base = item.gradients.iter().find(|g| g.name == name);
            let action = setup.get(name).filter(|g| g.len() == 1).map(|g| g[0]);
            valid &= [base, action]
                .into_iter()
                .all(|g| g.is_some_and(|g| g.profile && exact(raw.envelopes[&g.envelope], body)));
        }
        if valid {
            complete_mesh_pairs += 1;
        }
        if !valid {
            bad.entry(item.id)
                .or_default()
                .push("unsupported/incomplete default curve gradient profile".into());
        }
    }
    for a in &raw.actions {
        for (owner, key, kind, env, value) in &a.values {
            if kind & 0x10 != 0 {
                referenced.insert(*env);
            }
            // Different serialized channel families must agree even for omitted owners.
            if string_values
                .get(&(*owner, key))
                .is_some_and(|s| value.text() != Some(s.as_str()))
            {
                return Err(Error::new(
                    "unknown_dependencies",
                    0,
                    "Conflicting string channel representations",
                ));
            }
        }
        for s in &a.strings {
            c.check("omission inventory")?;
            let item = &raw.items[&s.owner];
            let modeled = matches!(
                item.kind.as_str(),
                "mesh" | "translation" | "rotation" | "scale" | "meshInst" | "transform"
            );
            let agreed = item
                .channels
                .get(&s.name)
                .is_none_or(|v| v.text() == Some(s.value.as_str()));
            let valid = s.profile
                && a.name == "setup"
                && a.kind == "setup"
                && allowed(&item.kind, &s.name) == Some(s.value.as_str())
                && agreed;
            if !valid {
                if modeled {
                    bad.entry(s.owner)
                        .or_default()
                        .push(format!("unsupported action string {}", s.name));
                } else {
                    return Err(Error::new(
                        "unknown_dependencies",
                        0,
                        "Unsupported auxiliary string owner/value/context",
                    ));
                }
            }
        }
    }
    c.check("envelope reference closure")?;
    let mut inventory = UnreferencedEnvelopes::default();
    for (id, body) in &raw.envelopes {
        c.check("unreferenced envelope profile/count checking")?;
        if referenced.contains(id) {
            continue;
        }
        let profile = if exact(body, RAD) {
            "radGrad_constant"
        } else if exact(body, ROT) {
            "rotGrad_constant"
        } else {
            return Err(Error::new(
                "unclassified_envelope",
                0,
                &format!("ENVL {id}: unknown zero-reference body"),
            ));
        };
        c.charge(128)?;
        inventory.ids.push(*id);
        *inventory.profiles.entry(profile.to_owned()).or_default() += 1;
        let id_width = if *id < 65280 { 2 } else { 4 };
        inventory.payload_bytes += body.len() + id_width;
        inventory.chunk_bytes += body.len() + id_width + 8;
    }
    if inventory
        .profiles
        .get("radGrad_constant")
        .copied()
        .unwrap_or(0)
        != complete_mesh_pairs
        || inventory
            .profiles
            .get("rotGrad_constant")
            .copied()
            .unwrap_or(0)
            != complete_mesh_pairs
    {
        return Err(Error::new(
            "unsupported_envelope_count_profile",
            0,
            &format!(
                "ENV-U1 requires {complete_mesh_pairs} zero-reference records of each exact profile; observed {:?}",
                inventory.profiles
            ),
        ));
    }
    c.check("envelope omission inventory")?;
    Ok(inventory)
}

pub fn account(
    raw: &Raw<'_>,
    rejected: &BTreeSet<u32>,
    report: &mut Report,
    c: &mut Control<'_>,
) -> Result<()> {
    let mut settings = BTreeSet::new();
    let mut owners: BTreeMap<&str, BTreeSet<u32>> = BTreeMap::new();
    let mut envelopes = BTreeSet::new();
    for item in raw.items.values().filter(|i| !rejected.contains(&i.id)) {
        for g in &item.gradients {
            c.check("omission inventory")?;
            settings.insert((item.id, &g.name));
            envelopes.insert(g.envelope);
            *report.auxiliary.channels.entry(g.name.clone()).or_default() += 1;
            report.auxiliary.records += 1;
            owners.entry(&item.kind).or_default().insert(item.id);
        }
    }
    for a in &raw.actions {
        for (owner, g) in a
            .gradients
            .iter()
            .filter(|(owner, _)| !rejected.contains(owner))
        {
            settings.insert((*owner, &g.name));
            envelopes.insert(g.envelope);
            *report.auxiliary.channels.entry(g.name.clone()).or_default() += 1;
            report.auxiliary.records += 1;
            owners
                .entry(&raw.items[owner].kind)
                .or_default()
                .insert(*owner);
        }
        for s in a.strings.iter().filter(|s| !rejected.contains(&s.owner)) {
            settings.insert((s.owner, &s.name));
            owners
                .entry(&raw.items[&s.owner].kind)
                .or_default()
                .insert(s.owner);
            *report.auxiliary.channels.entry(s.name.clone()).or_default() += 1;
            report.auxiliary.records += 1;
        }
    }
    report.auxiliary.settings = settings.len();
    report.auxiliary.envelopes = envelopes.len();
    report.auxiliary.owners = owners
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.len()))
        .collect();
    Ok(())
}
