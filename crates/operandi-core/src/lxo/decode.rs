use super::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Float(f64),
    Text(String),
}
impl Value {
    pub fn text(&self) -> Option<&str> {
        if let Self::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
    pub fn number(&self) -> Option<f64> {
        match self {
            Self::Int(n) => Some(*n as f64),
            Self::Float(n) => Some(*n),
            _ => None,
        }
    }
}
#[derive(Debug)]
pub struct Item {
    pub id: u32,
    pub kind: String,
    pub name: String,
    pub visible_name: Option<String>,
    pub channels: BTreeMap<String, Value>,
    pub links: Vec<(String, u32, i32)>,
    pub gradients: Vec<Gradient>,
    pub unsupported: Vec<String>,
    pub has_layer: bool,
    pub tags: Vec<super::metadata::Tag>,
}
#[derive(Debug)]
pub struct Gradient {
    pub name: String,
    pub envelope: u32,
    pub profile: bool,
}
#[derive(Debug)]
pub struct ActionString {
    pub owner: u32,
    pub name: String,
    pub value: String,
    pub profile: bool,
}
#[derive(Debug)]
pub struct Action {
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub parent: Option<u32>,
    pub values: Vec<(u32, String, u16, u32, Value)>,
    pub gradients: Vec<(u32, Gradient)>,
    pub strings: Vec<ActionString>,
    pub owners: BTreeSet<u32>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct EnvelopeReference {
    pub record: &'static str,
    pub owner: u32,
    pub action: Option<u32>,
    pub channel: String,
    pub envelope: u32,
    pub offset: usize,
}
pub struct Raw<'a> {
    pub profile: String,
    pub names: Vec<String>,
    pub tags: Vec<String>,
    pub items: BTreeMap<u32, Item>,
    pub layers: Vec<Vec<Chunk<'a>>>,
    pub actions: Vec<Action>,
    pub envelopes: BTreeMap<u32, &'a [u8]>,
    pub envelope_references: Vec<EnvelopeReference>,
    pub static_envelope_fields: usize,
    pub sizes: Vec<super::metadata::Size>,
    pub report: Report,
    pub unknown_item_records: BTreeSet<[u8; 4]>,
    /// Format 4.x: action values Operandi can't read, by owner item. They
    /// refuse only a modeled owner's group (convert); other owners' are
    /// left out with a note.
    pub unreadable: Vec<(u32, String)>,
}
pub fn value(r: &mut Reader<'_>, kind: u16, c: &mut Control<'_>) -> Result<Value> {
    match kind {
        1 | 0x21 | 0x11 => Ok(Value::Int(r.u32()? as i32)),
        2 | 0x22 | 0x12 => Ok(Value::Float(r.f32()?)),
        3 | 0x23 | 0x13 => Ok(Value::Text(r.string(c)?)),
        _ => Err(r.error(
            "unsupported_channel_encoding",
            &format!("Channel encoding {kind:#x}"),
        )),
    }
}
fn named(names: &[String], index: u32, offset: usize, c: &mut Control<'_>) -> Result<String> {
    let name = names.get(index as usize).ok_or_else(|| {
        Error::new(
            "invalid_channel_reference",
            offset,
            "CHNM index is out of range",
        )
    })?;
    c.charge(name.len() * 4 + 128)?;
    Ok(name.clone())
}

fn insert(
    channels: &mut BTreeMap<String, Value>,
    key: String,
    v: Value,
    offset: usize,
) -> Result<()> {
    if channels.insert(key, v).is_some() {
        Err(Error::new(
            "duplicate_channel",
            offset,
            "Repeated scalar/component channel",
        ))
    } else {
        Ok(())
    }
}
/// Format 4.x: a channel stored twice (e.g. as CHAN and in a CHNV) keeps
/// its first value when both agree; values that disagree are left out of
/// the item's import (`unsupported`), refusing only its group.
fn insert_legacy(item: &mut Item, key: String, v: Value) {
    let agree = |a: &Value, b: &Value| match (a.number(), b.number()) {
        (Some(a), Some(b)) => (a - b).abs() <= 1e-6 * (1.0 + a.abs()),
        _ => a == b,
    };
    match item.channels.get(&key) {
        None => {
            item.channels.insert(key, v);
        }
        Some(old) if agree(old, &v) => {}
        Some(_) => item
            .unsupported
            .push(format!("conflicting values for channel {key}")),
    }
}
fn one<'a>(chunks: &'a [Chunk<'a>], id: [u8; 4], required: bool) -> Result<Option<Chunk<'a>>> {
    let mut matches = chunks.iter().filter(|c| c.id == id);
    let first = matches.next().copied();
    if matches.next().is_some() || required && first.is_none() {
        return Err(Error::new(
            "invalid_table_cardinality",
            0,
            &format!(
                "Require {} {}",
                if required { "one" } else { "at most one" },
                String::from_utf8_lossy(&id)
            ),
        ));
    }
    Ok(first)
}
pub fn decode<'a>(chunks: &[Chunk<'a>], c: &mut Control<'_>) -> Result<Raw<'a>> {
    let vrsn = one(chunks, *b"VRSN", true)?.unwrap();
    let mut r = Reader::new(vrsn.data, vrsn.offset);
    let version = (r.u32()?, r.u32()?);
    // LEG-01: formats 4.0 and 4.1 are Modo 8-10. Formats 2.x and 3.x
    // (older Modo) go through the same lenient reader, mesh content only
    // (LEG-OLD-01, provisional).
    let old = matches!(version.0, 2 | 3);
    let legacy = matches!(version, (4, 0) | (4, 1)) || old;
    if version != (16, 0) && !legacy {
        return Err(r.error("unsupported_format_version", "Require VRSN=(16,0)"));
    }
    // Format 4.x files may lack ENCO or name another encoding; strings are
    // still read as strict UTF-8, so a string that isn't refuses the file.
    let mut encoding_note = None;
    match one(chunks, *b"ENCO", !legacy)? {
        Some(enco) => {
            let mut r = Reader::new(enco.data, enco.offset);
            let n = r.u32()?;
            if n != 2 && !legacy {
                return Err(r.error("unsupported_encoding", "Require ENCO=2 UTF-8"));
            }
            r.end()?;
            if n != 2 {
                encoding_note = Some(format!("encoding {n} read as UTF-8"));
            }
        }
        None => encoding_note = Some("no encoding record; read as UTF-8".to_string()),
    }
    // LXO-LEG-Q05: then names that aren't UTF-8 are read as Windows-1252.
    c.ansi_names = legacy && encoding_note.is_some();
    let _version_text = r.string(c)?;
    r.end()?;
    let (profile, producer) = if legacy {
        // LEG-02: APPV and STAG are information only, never acceptance.
        let era = if old {
            "Modo older than 8"
        } else {
            "Modo 8-10"
        };
        let mut producer = format!("{era} (LXO format {}.{})", version.0, version.1);
        if let Some(appv) = one(chunks, *b"APPV", false)? {
            let mut r = Reader::new(appv.data, appv.offset);
            let words = [r.u32()?, r.u32()?, r.u32()?, r.u32()?];
            producer = format!("{producer}, APPV {words:?}");
        }
        (format!("Legacy {}", version.0), producer)
    } else {
        let appv = one(chunks, *b"APPV", true)?.unwrap();
        let mut r = Reader::new(appv.data, appv.offset);
        let words = [r.u32()?, r.u32()?, r.u32()?, r.u32()?];
        let producer = r.string(c)?;
        r.end()?;
        let profile = match (words, producer.as_str()) {
            ([2000, 2000, 0, 662987], "Modo 16.0v4") => "Modo 16",
            ([17, 4000, 8, 957049], "Modo 17.0v8") => "Modo 17",
            _ => {
                return Err(r.error(
                    "untested_producer_profile",
                    &format!("APPV {words:?} {producer}; only Modo 16.0v4 / 17.0v8 admitted"),
                ));
            }
        };
        (profile.to_string(), producer)
    };
    let mut names = Vec::new();
    if let Some(ch) = one(chunks, *b"CHNM", false)? {
        let mut r = Reader::new(ch.data, ch.offset);
        let n = r.u32()? as usize;
        if n > 1_000_000 || n > r.data.len() / 2 {
            return Err(r.error("resource_limit", "CHNM count exceeds payload/budget"));
        }
        for _ in 0..n {
            c.check("channel table")?;
            names.push(r.string(c)?);
        }
        r.end()?;
    }
    let mut tags = Vec::new();
    if let Some(ch) = one(chunks, *b"TAGS", false)? {
        let mut r = Reader::new(ch.data, ch.offset);
        while !r.done() {
            c.record(true)?;
            tags.push(r.name(c)?);
        }
    }
    let mut raw = Raw {
        profile,
        names,
        tags,
        items: BTreeMap::new(),
        layers: Vec::new(),
        actions: Vec::new(),
        envelopes: BTreeMap::new(),
        envelope_references: Vec::new(),
        static_envelope_fields: 0,
        sizes: Vec::new(),
        unknown_item_records: BTreeSet::new(),
        unreadable: Vec::new(),
        report: Report {
            producer,
            ..Default::default()
        },
    };
    if let Some(note) = encoding_note {
        raw.report.omit(note);
    }
    for ch in chunks {
        c.check("inventory")?;
        let mut r = Reader::new(ch.data, ch.offset);
        match &ch.id {
            b"VRSN" | b"ENCO" | b"APPV" | b"CHNM" | b"TAGS" => {}
            // LEG-11: cached simulation and item index data, not modelling.
            b"STAG" | b"DATA" | b"BAKE" | b"VVEC" | b"3GRP" | b"3SRF" | b"VRTS" if legacy => {
                raw.report.omit(format!(
                    "{} cache/information chunk",
                    String::from_utf8_lossy(&ch.id)
                ))
            }
            b"IASS" if ch.data.is_empty() => {}
            b"AANI" if ch.data == [0x41, 0x41, 0x53, 0x45, 0, 10, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0] => {
            }
            // Format 4.x: sub-scene, assembly and reference records point at
            // other files Operandi doesn't open; left out with a note.
            b"IASS" | b"AANI" | b"REFS" | b"SUBS" | b"XREF" if legacy => raw.report.omit(format!(
                "{} sub-scene/reference record(s) not imported (other files aren't opened)",
                String::from_utf8_lossy(&ch.id)
            )),
            b"IASS" | b"AANI" | b"REFS" => {
                return Err(r.error(
                    "unsupported_external_or_assembly_state",
                    "Unverified global dependency/action profile; no external resolution",
                ));
            }
            b"PRVW" => raw.report.omit(format!(
                "preview, unparsed declared payload {} bytes",
                ch.data.len()
            )),
            b"LAYR" => {
                if raw.layers.len() >= crate::MAX_SCENE_ITEMS {
                    return Err(r.error("resource_limit", "Mesh layer count"));
                }
                raw.layers.push(vec![*ch]);
            }
            b"PNTS" | b"POLS" | b"VMAP" | b"VMAD" | b"VMED" | b"VMPA" | b"PTAG" | b"BBOX" => {
                raw.layers
                    .last_mut()
                    .ok_or_else(|| r.error("missing_geometry_scope", "Geometry without LAYR"))?
                    .push(*ch);
            }
            b"ITEM" => {
                // The same item count as the other Modo reader (phase1).
                if raw.items.len() >= 4 * crate::MAX_SCENE_ITEMS {
                    return Err(r.error("resource_limit", "Item count limit"));
                }
                let kind = r.string(c)?;
                let name = r.name(c)?;
                let id = r.u32()?;
                let mut item = Item {
                    id,
                    kind,
                    name,
                    visible_name: None,
                    channels: BTreeMap::new(),
                    links: vec![],
                    gradients: vec![],
                    unsupported: vec![],
                    has_layer: false,
                    tags: Vec::new(),
                };
                let mut single = BTreeSet::new();
                let mut gradient_names = BTreeSet::new();
                let mut tag_keys = BTreeSet::new();
                for sub in r.chunks(true, c)? {
                    let mut s = Reader::new(sub.data, sub.offset);
                    match &sub.id {
                        b"UNIQ" | b"UIDX" | b"VNAM" | b"LAYR" | b"BBOX" => {
                            if !single.insert(sub.id) {
                                return Err(
                                    s.error("duplicate_item_metadata", "Repeated ITEM metadata")
                                );
                            }
                            match &sub.id {
                                b"UNIQ" => {
                                    s.string(c)?;
                                }
                                b"VNAM" => item.visible_name = Some(s.name(c)?),
                                b"UIDX" => {
                                    s.u32()?;
                                }
                                b"LAYR" => {
                                    s.take(12)?;
                                    item.has_layer = true;
                                }
                                _ => {
                                    s.take(24)?;
                                    raw.report.omit("cached item bounds");
                                }
                            }
                            s.end()?;
                        }
                        b"LINK" => {
                            let graph = s.string(c)?;
                            let target = s.u32()?;
                            let ordinal = s.u32()? as i32;
                            if target > i32::MAX as u32 && !legacy {
                                return Err(s.error(
                                    "invalid_item_reference",
                                    "Negative/unsupported LINK target",
                                ));
                            }
                            s.end()?;
                            if target > i32::MAX as u32 {
                                // Format 4.x writes links whose target is
                                // no item (e.g. 0xFFFFFFFF); they connect
                                // nothing, so leave them out with a count.
                                raw.report.omit("links with no usable target, left out");
                                continue;
                            }
                            item.links.push((graph, target, ordinal));
                        }
                        b"CHAN" | b"CHNL" if sub.id == *b"CHAN" || legacy => {
                            // CHNL (older formats) names the channel in place
                            // of a CHNM index; the rest is CHAN's layout.
                            let key = if sub.id == *b"CHNL" {
                                s.string(c)?
                            } else {
                                named(&raw.names, s.vx()?, sub.offset, c)?
                            };
                            let kind = s.u16()?;
                            let v = value(&mut s, kind, c)?;
                            s.end()?;
                            if matches!(kind, 17..=19) {
                                // LEG-08: an envelope-driven channel refuses
                                // only its own dependency group.
                                if legacy {
                                    item.unsupported.push(format!("animated channel {key}"));
                                    continue;
                                }
                                return Err(s.error(
                                    "unknown_reference_semantics",
                                    "Associated ITEM.CHAN requires reviewed action resolution",
                                ));
                            }
                            if key == "size" {
                                c.charge(128)?;
                                raw.sizes.push(super::metadata::Size {
                                    owner: id,
                                    action: None,
                                    kind,
                                    bits: match &v {
                                        Value::Float(value) => Some((*value as f32).to_bits()),
                                        _ => None,
                                    },
                                    offset: sub.offset,
                                });
                            }
                            if legacy {
                                insert_legacy(&mut item, key, v);
                            } else {
                                insert(&mut item.channels, key, v, sub.offset)?;
                            }
                        }
                        b"CHNV" => {
                            let base = s.string(c)?;
                            let kind = s.u16()?;
                            let n = s.u16()?;
                            if legacy && (!matches!(kind, 1..=3 | 33..=35) || n > 4) {
                                // A vector type Operandi can't read: the
                                // item's channel is unknown, so only its
                                // group is refused.
                                item.unsupported.push(format!(
                                    "unreadable channel vector {base} (type {kind:#x})"
                                ));
                                raw.report.omit("unreadable channel vectors left out");
                                continue;
                            }
                            if !matches!(kind, 1..=3 | 33..=35) {
                                return Err(s.error(
                                    "unknown_reference_semantics",
                                    "Unadmitted CHNV type prevents reference closure",
                                ));
                            }
                            if n > 4 {
                                return Err(s.error(
                                    "unsupported_channel_vector",
                                    "Vector component count",
                                ));
                            }
                            for _ in 0..n {
                                let component = s.string(c)?;
                                let v = value(&mut s, kind, c).map_err(|mut e| {
                                    if e.category == "nonfinite_modeling_value"
                                        && matches!(component.as_str(), "X" | "Y" | "Z")
                                        && matches!(
                                            (item.kind.as_str(), base.as_str()),
                                            ("translation", "pos")
                                                | ("rotation", "rot")
                                                | ("scale", "scl")
                                        )
                                    {
                                        e.category = "nonfinite_transform";
                                    }
                                    e
                                })?;
                                let key = format!("{base}.{component}");
                                if legacy {
                                    insert_legacy(&mut item, key, v);
                                } else {
                                    insert(&mut item.channels, key, v, sub.offset)?;
                                }
                            }
                            s.end()?;
                        }
                        b"ITAG" => {
                            c.check("item tag decoding")?;
                            let id = s.id()?;
                            let value = s.string(c)?;
                            // LEG-OLD-05 (provisional): mesh presets mark
                            // their items with PRST/PPRT; information only.
                            if legacy && s.done() && matches!(&id, b"PRST" | b"PPRT") {
                                raw.report.omit("preset item tags not imported");
                                continue;
                            }
                            if !s.done() || !super::metadata::tag_allowed(&item.kind, id, &value) {
                                // Format 4.x: an unknown tag only matters on
                                // an item Operandi imports.
                                if legacy {
                                    item.unsupported
                                        .push(format!("ITAG {}", String::from_utf8_lossy(&id)));
                                    continue;
                                }
                                return Err(s.error("unsupported_itag_profile", "Unadmitted owner/tag/value or trailing bytes prevent reference closure"));
                            }
                            if !tag_keys.insert(id) {
                                return Err(s.error("duplicate_item_tag", "Repeated owner/tag ID4"));
                            }
                            c.charge(192)?;
                            item.tags.push(super::metadata::Tag {
                                id,
                                value,
                                offset: sub.offset,
                            });
                        }
                        b"CHNS" => {
                            let name = s.string(c)?;
                            let value = Value::Text(s.string(c)?);
                            s.end()?;
                            if matches!(
                                item.kind.as_str(),
                                "mesh"
                                    | "translation"
                                    | "rotation"
                                    | "scale"
                                    | "transform"
                                    | "meshInst"
                            ) {
                                item.unsupported
                                    .push(format!("unsupported modeling ITEM.CHNS {name}"));
                            }
                            if legacy {
                                insert_legacy(&mut item, name, value);
                            } else {
                                insert(&mut item.channels, name, value, sub.offset)?;
                            }
                        }
                        b"GRAD" => {
                            let name = s.string(c)?;
                            let gradient = gradient(&mut s, name, false)?;
                            c.charge(256)?;
                            if !gradient_names.insert(gradient.name.clone()) {
                                return Err(s.error(
                                    "duplicate_gradient",
                                    "Repeated ITEM gradient channel",
                                ));
                            }
                            raw.envelope_references.push(EnvelopeReference {
                                record: "ITEM.GRAD",
                                owner: id,
                                action: None,
                                channel: gradient.name.clone(),
                                envelope: gradient.envelope,
                                offset: sub.offset,
                            });
                            item.gradients.push(gradient);
                        }
                        // Format 4.x: unknown item records (packages, rig and
                        // display data) don't change geometry; left out.
                        _ if legacy => raw.report.omit(format!(
                            "ITEM.{} records not imported",
                            String::from_utf8_lossy(&sub.id)
                        )),
                        _ => {
                            raw.unknown_item_records.insert(sub.id);
                            item.unsupported.push(format!(
                                "{} unparsed, declared payload {} bytes",
                                String::from_utf8_lossy(&sub.id),
                                sub.data.len()
                            ));
                        }
                    }
                }
                if let Some(first) = raw.items.get_mut(&id) {
                    // Older formats and mesh presets can write the same item
                    // twice; the records are one item (LEG-OLD-03).
                    if legacy && first.kind == item.kind && first.name == item.name {
                        merge_repeated_item(first, item);
                        raw.report.omit("repeated item records merged");
                        continue;
                    }
                    return Err(r.error(
                        "duplicate_item_id",
                        &format!(
                            "ITEM IDs must be unique ({} '{}' and {} '{}')",
                            first.kind, first.name, item.kind, item.name
                        ),
                    ));
                }
                raw.items.insert(id, item);
            }
            b"ENVL" => {
                let id = r.vx()?;
                let body = &ch.data[r.pos..];
                r.u32()?;
                if raw.envelopes.insert(id, body).is_some() {
                    return Err(r.error("duplicate_envelope_id", "Repeated envelope ID"));
                }
                r.chunks(true, c)?;
            }
            b"ACTN" => {
                let name = r.string(c)?;
                let kind = r.string(c)?;
                let id = r.u32()?;
                let mut action = Action {
                    id,
                    name,
                    kind,
                    parent: None,
                    values: vec![],
                    gradients: vec![],
                    strings: vec![],
                    owners: BTreeSet::new(),
                };
                let mut owner = None;
                let mut gradient_keys = BTreeSet::new();
                let mut string_keys = BTreeSet::new();
                for sub in r.chunks(true, c)? {
                    let mut s = Reader::new(sub.data, sub.offset);
                    match &sub.id {
                        b"ITEM" => {
                            let selected = s.u32()?;
                            action.owners.insert(selected);
                            owner = Some(selected);
                            s.end()?;
                        }
                        b"PRNT" => {
                            let parent = s.u32()?;
                            if legacy && action.parent.is_some() {
                                // Keep the first parent.
                                raw.report.omit("repeated action parents (first kept)");
                                continue;
                            }
                            if action.parent.replace(parent).is_some() {
                                return Err(
                                    s.error("duplicate_action_parent", "Repeated action PRNT")
                                );
                            }
                            s.end()?;
                        }
                        b"CHAN" => {
                            let key = named(&raw.names, s.vx()?, sub.offset, c)?;
                            let kind = s.u16()?;
                            // LEG-08: flagged and unflagged types are the same.
                            let kind = if legacy && matches!(kind, 0x21..=0x23 | 0x31..=0x33) {
                                kind - 0x20
                            } else {
                                kind
                            };
                            let env = s.vx()?;
                            if legacy
                                && (!matches!(kind, 1..=3 | 17..=19)
                                    || matches!(kind, 1..=3) && env != 0)
                            {
                                let owner = owner.ok_or_else(|| {
                                    s.error("missing_action_owner", "ACTN.CHAN before ITEM")
                                })?;
                                raw.unreadable.push((
                                    owner,
                                    format!("unreadable action value {key} (type {kind:#x})"),
                                ));
                                continue;
                            }
                            if !matches!(kind, 1..=3 | 17..=19) {
                                return Err(s.error(
                                    "unsupported_channel_encoding",
                                    "Unadmitted ACTN channel type prevents reference closure",
                                ));
                            }
                            if matches!(kind, 1..=3) && env != 0 {
                                return Err(s.error(
                                    "unsupported_static_envelope_field",
                                    "Static ACTN.CHAN requires the observed zero field",
                                ));
                            }
                            let v = value(&mut s, kind, c)?;
                            s.end()?;
                            if matches!(kind, 17..=19) {
                                c.charge(key.len() + 128)?;
                                raw.envelope_references.push(EnvelopeReference {
                                    record: "ACTN.CHAN",
                                    owner: owner.ok_or_else(|| {
                                        s.error("missing_action_owner", "ACTN.CHAN before ITEM")
                                    })?,
                                    action: Some(id),
                                    channel: key.clone(),
                                    envelope: env,
                                    offset: sub.offset,
                                });
                            } else {
                                raw.static_envelope_fields += 1;
                            }
                            if key == "size" {
                                c.charge(128)?;
                                raw.sizes.push(super::metadata::Size {
                                    owner: owner.ok_or_else(|| {
                                        s.error("missing_action_owner", "Size before ITEM")
                                    })?,
                                    action: Some(id),
                                    kind,
                                    bits: match &v {
                                        Value::Float(value) => Some((*value as f32).to_bits()),
                                        _ => None,
                                    },
                                    offset: sub.offset,
                                });
                            }
                            action.values.push((
                                owner.ok_or_else(|| {
                                    s.error("missing_action_owner", "ACTN.CHAN before ITEM")
                                })?,
                                key,
                                kind,
                                env,
                                v,
                            ));
                        }
                        b"GRAD" => {
                            let owner = owner.ok_or_else(|| {
                                s.error("missing_action_owner", "ACTN.GRAD before ITEM")
                            })?;
                            let name = named(&raw.names, s.vx()?, sub.offset, c)?;
                            let g = gradient(&mut s, name, true)?;
                            c.charge(256)?;
                            if !gradient_keys.insert((owner, g.name.clone())) {
                                return Err(s.error(
                                    "duplicate_gradient",
                                    "Repeated ACTN gradient channel",
                                ));
                            }
                            raw.envelope_references.push(EnvelopeReference {
                                record: "ACTN.GRAD",
                                owner,
                                action: Some(id),
                                channel: g.name.clone(),
                                envelope: g.envelope,
                                offset: sub.offset,
                            });
                            action.gradients.push((owner, g));
                        }
                        b"CHNS" => {
                            let owner = owner.ok_or_else(|| {
                                s.error("missing_action_owner", "ACTN.CHNS before ITEM")
                            })?;
                            let legacy_name = s.string(c)?;
                            let name = named(&raw.names, s.vx()?, sub.offset, c)?;
                            let value = s.string(c)?;
                            let profile = legacy_name.is_empty() && s.done();
                            s.take(s.data.len() - s.pos)?;
                            c.charge(256)?;
                            if !string_keys.insert((owner, name.clone())) {
                                return Err(s.error(
                                    "duplicate_action_channel",
                                    "Repeated ACTN string channel",
                                ));
                            }
                            action.strings.push(ActionString {
                                owner,
                                name,
                                value,
                                profile,
                            });
                        }
                        _ if legacy => raw.report.omit(format!(
                            "ACTN.{} records not imported",
                            String::from_utf8_lossy(&sub.id)
                        )),
                        _ => {
                            return Err(s.error(
                                "unknown_action_state",
                                &format!(
                                    "ACTN.{} has unclassified dependencies",
                                    String::from_utf8_lossy(&sub.id)
                                ),
                            ));
                        }
                    }
                }
                raw.actions.push(action);
            }
            // Format 4.x: anything else top-level is left out with a note.
            _ if legacy => raw.report.omit(format!(
                "{} chunk not imported",
                String::from_utf8_lossy(&ch.id)
            )),
            _ => {
                return Err(r.error(
                    "unknown_dependencies",
                    &format!(
                        "Top-level {} has unknown ownership",
                        String::from_utf8_lossy(&ch.id)
                    ),
                ));
            }
        }
    }
    c.check("reference resolution")?;
    if legacy {
        drop_references_to_missing_items(&mut raw);
    }
    let mut aids = BTreeSet::new();
    for a in &raw.actions {
        if !aids.insert(a.id) {
            return Err(Error::new("duplicate_action_id", 0, "Repeated ACTN ID"));
        }
    }
    for a in &raw.actions {
        if a.parent.is_some_and(|p| !aids.contains(&p)) {
            return Err(Error::new(
                "invalid_action_reference",
                0,
                "Dangling ACTN parent",
            ));
        }
        for owner in &a.owners {
            if !raw.items.contains_key(owner) {
                return Err(Error::new(
                    "invalid_item_reference",
                    0,
                    &format!("Dangling ACTN selector (action '{}' {})", a.name, a.kind),
                ));
            }
        }
        for (_, g) in &a.gradients {
            if !raw.envelopes.contains_key(&g.envelope) {
                return Err(Error::new(
                    "invalid_envelope_reference",
                    0,
                    "Dangling ACTN gradient envelope",
                ));
            }
        }
        for (owner, _, kind, env, _) in &a.values {
            if !raw.items.contains_key(owner) {
                return Err(Error::new(
                    "invalid_item_reference",
                    0,
                    "Dangling ACTN owner",
                ));
            }
            if matches!(kind, 17..=19) && !raw.envelopes.contains_key(env) {
                return Err(Error::new(
                    "invalid_envelope_reference",
                    0,
                    "Dangling animated channel envelope",
                ));
            }
        }
    }
    for i in raw.items.values() {
        for (graph, target, _) in &i.links {
            if !raw.items.contains_key(target) {
                return Err(Error::new(
                    "invalid_item_reference",
                    0,
                    &format!("Dangling LINK target ({} '{}' {graph})", i.kind, i.name),
                ));
            }
        }
        for g in &i.gradients {
            if !raw.envelopes.contains_key(&g.envelope) {
                return Err(Error::new(
                    "invalid_envelope_reference",
                    0,
                    "Dangling GRAD envelope",
                ));
            }
        }
    }
    Ok(raw)
}

/// LEG-OLD-03 (provisional): a repeated ITEM with the same id, kind and name
/// adds its records to the first. Channels go through `insert_legacy`, so
/// values that disagree leave the item out of the import (its group only).
fn merge_repeated_item(first: &mut Item, repeat: Item) {
    first.has_layer |= repeat.has_layer;
    if first.visible_name.is_none() {
        first.visible_name = repeat.visible_name;
    }
    for (key, v) in repeat.channels {
        insert_legacy(first, key, v);
    }
    for link in repeat.links {
        if !first.links.contains(&link) {
            first.links.push(link);
        }
    }
    for g in repeat.gradients {
        if !first.gradients.iter().any(|old| old.name == g.name) {
            first.gradients.push(g);
        }
    }
    for tag in repeat.tags {
        if !first.tags.iter().any(|old| old.id == tag.id) {
            first.tags.push(tag);
        }
    }
    first.unsupported.extend(repeat.unsupported);
}

/// LEG-OLD-04 (provisional): mesh presets and older files keep links and
/// action values aimed at items they didn't save (the render item, the
/// scene's setup action). Those connect nothing in this file, so they are
/// left out with a note; references between items that are present are
/// still checked as before.
fn drop_references_to_missing_items(raw: &mut Raw<'_>) {
    let present: BTreeSet<u32> = raw.items.keys().copied().collect();
    let mut links = 0;
    for item in raw.items.values_mut() {
        let before = item.links.len();
        item.links.retain(|(_, target, _)| present.contains(target));
        links += before - item.links.len();
    }
    for _ in 0..links {
        raw.report
            .omit("links to items not saved in the file, left out");
    }
    let mut values = 0;
    for a in &mut raw.actions {
        let before = a.owners.len();
        a.owners.retain(|owner| present.contains(owner));
        values += before - a.owners.len();
        a.values.retain(|(owner, ..)| present.contains(owner));
        a.gradients.retain(|(owner, _)| present.contains(owner));
        a.strings.retain(|s| present.contains(&s.owner));
    }
    for _ in 0..values {
        raw.report
            .omit("action values for items not saved in the file, left out");
    }
    raw.sizes.retain(|size| present.contains(&size.owner));
    raw.envelope_references
        .retain(|reference| present.contains(&reference.owner));
    raw.unreadable.retain(|(owner, _)| present.contains(owner));
}

/// Consume bounded opaque variants without assigning semantics to their tails.
fn gradient(s: &mut Reader<'_>, name: String, action: bool) -> Result<Gradient> {
    let envelope = s.vx()?;
    let flags = s.u32()?;
    let tail = s.take(s.data.len() - s.pos)?;
    let expected: &[u8] = if action {
        b"\0\0float\0float\0"
    } else {
        b"float\0float\0"
    };
    Ok(Gradient {
        name,
        envelope,
        profile: flags == 0 && tail == expected,
    })
}

#[cfg(test)]
mod legacy_channel_tests {
    use super::*;

    #[test]
    fn a_repeated_channel_keeps_agreeing_values_and_flags_conflicts() {
        let mut item = Item {
            id: 1,
            kind: "translation".into(),
            name: String::new(),
            visible_name: None,
            channels: BTreeMap::new(),
            links: vec![],
            gradients: vec![],
            unsupported: vec![],
            has_layer: false,
            tags: vec![],
        };
        insert_legacy(&mut item, "pos.X".into(), Value::Float(1.0));
        insert_legacy(&mut item, "pos.X".into(), Value::Float(1.0 + 1e-9));
        assert!(item.unsupported.is_empty());
        insert_legacy(&mut item, "pos.X".into(), Value::Float(2.0));
        assert_eq!(item.unsupported, ["conflicting values for channel pos.X"]);
        assert_eq!(item.channels["pos.X"], Value::Float(1.0));
    }
}
