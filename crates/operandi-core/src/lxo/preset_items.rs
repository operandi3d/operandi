//! The items and channels of a Modo preset (`.lxp`), read for the material
//! converter (BEHAVIOR-LXP-MATERIALS §2 step 2).
//!
//! Black-box only (Q-MC1, BUILD-BRIEF): this reads the published container
//! layout and nothing else: the `FORM` and its top-level chunks; `CHNM`
//! (the channel-name table); `ITEM` (type, name, reference id, then
//! sub-chunks: channels stored by name or by `CHNM` index, `CHNV` vectors,
//! `CHNS` strings, `LINK` graph links and `GRAD` gradient channels); `ACTN`
//! (action values for an `ITEM` owner, the same channel records); and
//! `ENVL` (an envelope: index, type, then `KEY` records of time and value).
//! Every other chunk or record is skipped by its size and counted for the
//! import note. A record that doesn't fit its published layout refuses the
//! preset; nothing here guesses at an unknown record's content.
//!
//! It is more lenient than the scene readers: a preset is never modelled,
//! so producer versions, references to other files and unknown records
//! don't refuse it.
use super::decode::Value;
use super::wire::{Chunk, Reader};
use super::{Control, Error, Result};
use std::collections::BTreeMap;

/// One preset item.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PresetItem {
    pub id: u32,
    pub kind: String,
    pub name: String,
    /// Channels by name; vector components as `base.R` etc. Action values
    /// (`ACTN`) replace the item's own (the setup action is what Modo shows).
    pub channels: BTreeMap<String, Value>,
    /// (graph, target item id, ordinal).
    pub links: Vec<(String, u32, u32)>,
    /// Gradient records, one number curve each (LXP-B07).
    pub gradients: Vec<GradientChannel>,
}

impl PresetItem {
    pub fn number(&self, key: &str) -> Option<f64> {
        self.channels.get(key).and_then(Value::number)
    }
    pub fn text(&self, key: &str) -> Option<&str> {
        self.channels.get(key).and_then(Value::text)
    }
    /// A colour channel (`key.R`, `.G`, `.B`).
    pub fn colour(&self, key: &str) -> Option<[f64; 3]> {
        let c = |s: &str| self.number(&format!("{key}.{s}"));
        Some([c("R")?, c("G")?, c("B")?])
    }
    /// The display name: the item's name, or its type.
    pub fn label(&self) -> &str {
        if self.name.is_empty() {
            &self.kind
        } else {
            &self.name
        }
    }
}

/// One gradient-value record: the channel its curve drives (`value`,
/// `color.R` … `color.A`), the envelope holding the keys, the
/// interpolation flag (0 curve, 1 linear, 2 stepped) and, when the
/// record names them, its key and value types.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GradientChannel {
    pub channel: String,
    pub envelope: u32,
    pub interpolation: Option<u32>,
    pub types: Option<(String, String)>,
}

/// One `ENVL`: its keys as (time, value), in time order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Envelope {
    pub keys: Vec<(f64, f64)>,
}

/// What a preset file holds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PresetItems {
    pub form: [u8; 4],
    /// Items by reference id.
    pub items: BTreeMap<u32, PresetItem>,
    pub envelopes: BTreeMap<u32, Envelope>,
    /// Chunks and records skipped by size, by id, with counts.
    pub skipped: BTreeMap<String, usize>,
}

/// Biggest item count a preset may hold.
const ITEM_LIMIT: usize = 100_000;
/// Biggest key count one envelope may hold.
const KEY_LIMIT: usize = 4096;

/// Read a preset's items from the whole file's bytes.
pub fn read(bytes: &[u8]) -> Result<PresetItems> {
    let mut go = |_: &str| true;
    let mut c = Control::new(&mut go);
    if bytes.len() > c.limits.input_bytes {
        return Err(Error::new("file_size_limit", 0, "Input exceeds 256 MiB"));
    }
    let mut r = Reader::new(bytes, 0);
    if bytes.len() < 12 || r.id()? != *b"FORM" {
        return Err(r.error("wrong_form_type", "Expected a FORM container"));
    }
    let n = r.u32()? as usize;
    // A cut-off file reads as far as it goes; a longer one ends at the FORM.
    let end = (n + 8).min(bytes.len());
    let form = r.id()?;
    let mut body = Reader::new(&bytes[12..end], 12);
    let chunks = body.chunks(false, &mut c)?;
    let mut out = PresetItems {
        form,
        ..Default::default()
    };
    let mut names: Vec<String> = Vec::new();
    // CHNM first: CHAN records refer into it wherever it sits.
    for ch in chunks.iter().filter(|ch| &ch.id == b"CHNM") {
        let mut r = Reader::new(ch.data, ch.offset);
        let n = r.u32()? as usize;
        if n > r.data.len() / 2 {
            return Err(r.error("resource_limit", "CHNM count exceeds its payload"));
        }
        for _ in 0..n {
            names.push(r.string(&mut c)?);
        }
    }
    let mut actions = Vec::new();
    for ch in &chunks {
        match &ch.id {
            b"CHNM" => {}
            b"ITEM" => {
                if out.items.len() >= ITEM_LIMIT {
                    return Err(Error::new("resource_limit", ch.offset, "Item count"));
                }
                let item = item(ch, &names, &mut out.skipped, &mut c)?;
                // A repeated id keeps the first item (older presets write
                // some items twice).
                out.items.entry(item.id).or_insert(item);
            }
            b"ENVL" => {
                let (id, env) = envelope(ch, &mut out.skipped, &mut c)?;
                out.envelopes.entry(id).or_insert(env);
            }
            b"ACTN" => actions.push(*ch),
            other => *out.skipped.entry(id_text(other)).or_default() += 1,
        }
    }
    // Action values after every item is known, parents first.
    let heads = actions
        .iter()
        .map(|ch| action_head(ch, &mut c))
        .collect::<Result<Vec<_>>>()?;
    for h in action_order(heads, &mut out.skipped) {
        action(&h.chunk, &names, &mut out, &mut c)?;
    }
    Ok(out)
}

fn id_text(id: &[u8; 4]) -> String {
    String::from_utf8_lossy(id).trim().to_owned()
}

/// A channel's name from the `CHNM` table; `None` (the record is skipped
/// and counted under `record`) when the index is past the table's end.
fn channel_name(
    names: &[String],
    index: u32,
    skipped: &mut BTreeMap<String, usize>,
    record: &str,
) -> Option<String> {
    let name = names.get(index as usize).cloned();
    if name.is_none() {
        *skipped
            .entry(format!("{record} with an unknown channel name"))
            .or_default() += 1;
    }
    name
}

/// A channel value of `kind`; envelope-driven kinds (17..=19, 0x31..=0x33)
/// carry their static value the same way.
fn channel_value(s: &mut Reader<'_>, kind: u16, c: &mut Control<'_>) -> Result<Option<Value>> {
    let base = match kind {
        0x31..=0x33 => kind - 0x20,
        17..=19 => kind - 16,
        _ => kind,
    };
    match base {
        1 | 2 | 3 | 0x11..=0x13 | 0x21..=0x23 => super::decode::value(s, base, c).map(Some),
        _ => Ok(None),
    }
}

fn item(
    ch: &Chunk<'_>,
    names: &[String],
    skipped: &mut BTreeMap<String, usize>,
    c: &mut Control<'_>,
) -> Result<PresetItem> {
    let mut r = Reader::new(ch.data, ch.offset);
    let kind = r.string(c)?;
    let name = r.name(c)?;
    let id = r.u32()?;
    let mut item = PresetItem {
        id,
        kind,
        name,
        ..Default::default()
    };
    for sub in r.chunks(true, c)? {
        let mut s = Reader::new(sub.data, sub.offset);
        match &sub.id {
            b"CHAN" | b"CHNL" => {
                let key = if sub.id == *b"CHNL" {
                    s.string(c)?
                } else {
                    let index = s.vx()?;
                    let Some(key) = channel_name(names, index, skipped, "ITEM.CHAN") else {
                        continue;
                    };
                    key
                };
                let kind = s.u16()?;
                match channel_value(&mut s, kind, c)? {
                    Some(v) => {
                        item.channels.entry(key).or_insert(v);
                    }
                    None => {
                        *skipped
                            .entry(format!("channel type {kind:#x}"))
                            .or_default() += 1
                    }
                }
            }
            b"CHNV" => {
                let base = s.string(c)?;
                let kind = s.u16()?;
                let n = s.u16()?;
                if n > 16 {
                    return Err(s.error("unsupported_channel_vector", "Vector component count"));
                }
                for _ in 0..n {
                    let component = s.string(c)?;
                    let Some(v) = channel_value(&mut s, kind, c)? else {
                        *skipped
                            .entry(format!("channel type {kind:#x}"))
                            .or_default() += 1;
                        break;
                    };
                    item.channels
                        .entry(format!("{base}.{component}"))
                        .or_insert(v);
                }
            }
            b"CHNS" => {
                let key = s.string(c)?;
                let value = s.string(c)?;
                item.channels.entry(key).or_insert(Value::Text(value));
            }
            b"LINK" => {
                let graph = s.string(c)?;
                let target = s.u32()?;
                let ordinal = s.u32()?;
                item.links.push((graph, target, ordinal));
            }
            b"GRAD" => {
                // LXP-B07a (published spec 7.6.7): S0 channel name, VX
                // envelope, U4 flags. An empty name is the `value` curve.
                let name = s.string(c)?;
                let envelope = s.vx()?;
                let (interpolation, types) = gradient_tail(&mut s, c);
                let channel = if name.is_empty() {
                    "value".into()
                } else {
                    name
                };
                item.gradients.retain(|g| g.channel != channel);
                item.gradients.push(GradientChannel {
                    channel,
                    envelope,
                    interpolation,
                    types,
                });
            }
            other => {
                *skipped
                    .entry(format!("ITEM.{}", id_text(other)))
                    .or_default() += 1
            }
        }
    }
    Ok(item)
}

/// An ITEM-level GRAD record's tail after the envelope: the U4
/// interpolation flag, then the key and value types as strings (scenes
/// write "float", "float"). Missing parts read `None`.
fn gradient_tail(
    s: &mut Reader<'_>,
    c: &mut Control<'_>,
) -> (Option<u32>, Option<(String, String)>) {
    gradient_tail_named(s, false, c).map_or((None, None), |(f, t, _)| (Some(f), t))
}

/// A gradient record's key and value type names.
type KeyTypes = (String, String);

/// A GRAD tail: the U4 flag; in an action, then the S0 fallback name
/// (returned third, empty when absent); then the optional type strings.
fn gradient_tail_named(
    s: &mut Reader<'_>,
    action: bool,
    c: &mut Control<'_>,
) -> Option<(u32, Option<KeyTypes>, String)> {
    let flag = s.u32().ok()?;
    let name = if action && !s.done() {
        s.string(c).unwrap_or_default()
    } else {
        String::new()
    };
    let mut parts = Vec::new();
    while !s.done() && parts.len() < 2 {
        match s.string(c) {
            Ok(p) => parts.push(p),
            Err(_) => break,
        }
    }
    let types = match parts.as_slice() {
        [a, b] if !a.is_empty() && !b.is_empty() => Some((a.clone(), b.clone())),
        _ => None,
    };
    Some((flag, types, name))
}

fn envelope(
    ch: &Chunk<'_>,
    skipped: &mut BTreeMap<String, usize>,
    c: &mut Control<'_>,
) -> Result<(u32, Envelope)> {
    let mut r = Reader::new(ch.data, ch.offset);
    let id = r.vx()?;
    let _kind = r.u32()?;
    let mut env = Envelope::default();
    for sub in r.chunks(true, c)? {
        let mut s = Reader::new(sub.data, sub.offset);
        match &sub.id {
            b"KEY " => {
                if env.keys.len() >= KEY_LIMIT {
                    return Err(s.error("resource_limit", "Envelope key count"));
                }
                let time = s.f32()?;
                let value = s.f32()?;
                env.keys.push((time, value));
            }
            other => {
                *skipped
                    .entry(format!("ENVL.{}", id_text(other)))
                    .or_default() += 1
            }
        }
    }
    env.keys.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok((id, env))
}

/// One `ACTN`'s header and its `PRNT` parent, read before any is applied.
struct ActionHead<'a> {
    id: u32,
    kind: String,
    parent: Option<u32>,
    chunk: Chunk<'a>,
}

fn action_head<'a>(ch: &Chunk<'a>, c: &mut Control<'_>) -> Result<ActionHead<'a>> {
    let mut r = Reader::new(ch.data, ch.offset);
    let _name = r.string(c)?;
    let kind = r.string(c)?;
    let id = r.u32()?;
    let mut parent = None;
    for sub in r.chunks(true, c)? {
        if &sub.id == b"PRNT" {
            let mut s = Reader::new(sub.data, sub.offset);
            parent.get_or_insert(s.u32()?);
        }
    }
    Ok(ActionHead {
        id,
        kind,
        parent,
        chunk: *ch,
    })
}

/// LXP-B07a: the actions to apply, parents before children so a child's
/// values beat its parent's; unrelated actions keep file order. Only
/// `setup` and `edit` actions count (animation is left out); a parent
/// chain that loops or leaves the file just stops there.
fn action_order<'a>(
    heads: Vec<ActionHead<'a>>,
    skipped: &mut BTreeMap<String, usize>,
) -> Vec<ActionHead<'a>> {
    let depth = |h: &ActionHead<'_>| {
        let mut d = 0usize;
        let mut at = h.parent;
        while let Some(p) = at {
            if d > heads.len() {
                break;
            }
            d += 1;
            at = heads.iter().find(|x| x.id == p).and_then(|x| x.parent);
        }
        d
    };
    let mut keyed: Vec<(usize, usize)> = heads
        .iter()
        .enumerate()
        .map(|(i, h)| (depth(h), i))
        .collect();
    keyed.sort();
    let mut heads: Vec<Option<ActionHead<'a>>> = heads.into_iter().map(Some).collect();
    let mut out = Vec::new();
    for (_, i) in keyed {
        let Some(h) = heads[i].take() else { continue };
        if matches!(h.kind.to_ascii_lowercase().as_str(), "setup" | "edit") {
            out.push(h);
        } else {
            *skipped
                .entry(format!("ACTN {} (not applied)", h.kind))
                .or_default() += 1;
        }
    }
    out
}

/// Applies one action's channel records to the items they select. Each
/// `ITEM` sub-chunk (U4 item id) starts that item's records, up to the
/// next `ITEM`. Values replace the item's own (LXP-B07a: action wins).
fn action(
    ch: &Chunk<'_>,
    names: &[String],
    out: &mut PresetItems,
    c: &mut Control<'_>,
) -> Result<()> {
    let mut r = Reader::new(ch.data, ch.offset);
    let _name = r.string(c)?;
    let _kind = r.string(c)?;
    let _id = r.u32()?;
    let mut owner: Option<u32> = None;
    for sub in r.chunks(true, c)? {
        let mut s = Reader::new(sub.data, sub.offset);
        match &sub.id {
            b"ITEM" => owner = Some(s.u32()?),
            b"PRNT" => {}
            b"CHAN" | b"CHNN" => {
                // CHAN names its channel by CHNM index, CHNN by S0 text
                // (provisional, LXP-B07b); then U2 type, VX envelope, value.
                let key = if sub.id == *b"CHNN" {
                    s.string(c)?
                } else {
                    let index = s.vx()?;
                    let Some(key) = channel_name(names, index, &mut out.skipped, "ACTN.CHAN")
                    else {
                        continue;
                    };
                    key
                };
                let kind = s.u16()?;
                let _envelope = s.vx()?;
                let value = channel_value(&mut s, kind, c)?;
                match (value, owner.and_then(|o| out.items.get_mut(&o))) {
                    (Some(v), Some(item)) => {
                        item.channels.insert(key, v);
                    }
                    (None, _) => {
                        *out.skipped
                            .entry(format!("action channel type {kind:#x}"))
                            .or_default() += 1
                    }
                    (Some(_), None) => {
                        *out.skipped
                            .entry("ACTN value for an unknown item".into())
                            .or_default() += 1
                    }
                }
            }
            b"CHNS" => {
                let _legacy = s.string(c)?;
                let index = s.vx()?;
                let Some(key) = channel_name(names, index, &mut out.skipped, "ACTN.CHNS") else {
                    continue;
                };
                let value = s.string(c)?;
                if let Some(item) = owner.and_then(|o| out.items.get_mut(&o)) {
                    item.channels.insert(key, Value::Text(value));
                }
            }
            b"GRAD" => {
                // LXP-B07a (published spec 10.6): VX name index into CHNM,
                // VX envelope, U4 flags, then an S0 name used only when
                // the index is 0.
                let index = s.vx()?;
                let envelope = s.vx()?;
                let (interpolation, types, fallback) = gradient_tail_named(&mut s, true, c)
                    .map_or((None, None, String::new()), |(f, t, n)| (Some(f), t, n));
                let key = if index == 0 && !fallback.is_empty() {
                    fallback
                } else {
                    match channel_name(names, index, &mut out.skipped, "ACTN.GRAD") {
                        Some(k) => k,
                        None => continue,
                    }
                };
                let key = if key.is_empty() { "value".into() } else { key };
                if let Some(item) = owner.and_then(|o| out.items.get_mut(&o)) {
                    item.gradients.retain(|g| g.channel != key);
                    item.gradients.push(GradientChannel {
                        channel: key,
                        envelope,
                        interpolation,
                        types,
                    });
                }
            }
            other => {
                *out.skipped
                    .entry(format!("ACTN.{}", id_text(other)))
                    .or_default() += 1
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Hand-built records in the published layout.
    pub(crate) fn s0(text: &str) -> Vec<u8> {
        let mut v = text.as_bytes().to_vec();
        v.push(0);
        if v.len() % 2 == 1 {
            v.push(0);
        }
        v
    }
    pub(crate) fn sub(id: &[u8; 4], data: Vec<u8>) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(data.len() as u16).to_be_bytes());
        let odd = data.len() % 2 == 1;
        v.extend(data);
        if odd {
            v.push(0);
        }
        v
    }
    pub(crate) fn chunk(id: &[u8; 4], data: Vec<u8>) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let odd = data.len() % 2 == 1;
        v.extend(data);
        if odd {
            v.push(0);
        }
        v
    }
    pub(crate) fn form(form: &[u8; 4], chunks: Vec<Vec<u8>>) -> Vec<u8> {
        let mut body = form.to_vec();
        for ch in chunks {
            body.extend(ch);
        }
        let mut v = b"FORM".to_vec();
        v.extend_from_slice(&(body.len() as u32).to_be_bytes());
        v.extend(body);
        v
    }
    /// A float channel stored by name (CHNL).
    pub(crate) fn float(name: &str, v: f32) -> Vec<u8> {
        let mut d = s0(name);
        d.extend_from_slice(&2u16.to_be_bytes());
        d.extend_from_slice(&v.to_be_bytes());
        sub(b"CHNL", d)
    }
    pub(crate) fn text(name: &str, v: &str) -> Vec<u8> {
        let mut d = s0(name);
        d.extend(s0(v));
        sub(b"CHNS", d)
    }
    pub(crate) fn colour(name: &str, c: [f32; 3]) -> Vec<u8> {
        let mut d = s0(name);
        d.extend_from_slice(&2u16.to_be_bytes());
        d.extend_from_slice(&3u16.to_be_bytes());
        for (k, v) in ["R", "G", "B"].iter().zip(c) {
            d.extend(s0(k));
            d.extend_from_slice(&v.to_be_bytes());
        }
        sub(b"CHNV", d)
    }
    pub(crate) fn link(graph: &str, target: u32, ordinal: u32) -> Vec<u8> {
        let mut d = s0(graph);
        d.extend_from_slice(&target.to_be_bytes());
        d.extend_from_slice(&ordinal.to_be_bytes());
        sub(b"LINK", d)
    }
    pub(crate) fn grad(name: &str, envelope: u16) -> Vec<u8> {
        let mut d = s0(name);
        d.extend_from_slice(&envelope.to_be_bytes());
        d.extend_from_slice(&0u32.to_be_bytes());
        d.extend(s0("float"));
        d.extend(s0("float"));
        sub(b"GRAD", d)
    }
    pub(crate) fn item(kind: &str, name: &str, id: u32, subs: Vec<Vec<u8>>) -> Vec<u8> {
        let mut d = s0(kind);
        d.extend(s0(name));
        d.extend_from_slice(&id.to_be_bytes());
        for s in subs {
            d.extend(s);
        }
        chunk(b"ITEM", d)
    }
    pub(crate) fn envl(id: u16, keys: &[(f32, f32)]) -> Vec<u8> {
        let mut d = id.to_be_bytes().to_vec();
        d.extend_from_slice(&0u32.to_be_bytes());
        for (t, v) in keys {
            let mut k = t.to_be_bytes().to_vec();
            k.extend_from_slice(&v.to_be_bytes());
            d.extend(sub(b"KEY ", k));
        }
        d.extend(sub(b"PRE ", vec![0, 1]));
        chunk(b"ENVL", d)
    }

    #[test]
    fn reads_items_channels_links_gradients_and_envelopes() {
        let bytes = form(
            b"LXPR",
            vec![
                chunk(b"DESC", s0("advancedMaterial")),
                chunk(b"XYZW", vec![1, 2, 3]),
                item(
                    "advancedMaterial",
                    "Gold",
                    7,
                    vec![
                        float("diffAmt", 0.25),
                        colour("diffCol", [1., 0.5, 0.]),
                        text("brdfType", "gtr"),
                        link("parent", 3, 1),
                        sub(b"ZZZZ", vec![9, 9]),
                    ],
                ),
                item("gradient", "", 8, vec![grad("value", 2)]),
                envl(2, &[(1.0, 0.5), (0.0, 0.25)]),
            ],
        );
        let p = read(&bytes).unwrap();
        assert_eq!(&p.form, b"LXPR");
        let m = &p.items[&7];
        assert_eq!(m.number("diffAmt"), Some(0.25));
        assert_eq!(m.colour("diffCol"), Some([1., 0.5, 0.]));
        assert_eq!(m.text("brdfType"), Some("gtr"));
        assert_eq!(m.links, vec![("parent".into(), 3, 1)]);
        assert_eq!(
            p.items[&8].gradients,
            vec![GradientChannel {
                channel: "value".into(),
                envelope: 2,
                interpolation: Some(0),
                types: Some(("float".into(), "float".into())),
            }]
        );
        assert_eq!(p.envelopes[&2].keys, vec![(0.0, 0.25), (1.0, 0.5)]);
        assert_eq!(p.skipped.get("XYZW"), Some(&1));
        assert_eq!(p.skipped.get("DESC"), Some(&1));
        assert_eq!(p.skipped.get("ITEM.ZZZZ"), Some(&1));
        assert_eq!(p.skipped.get("ENVL.PRE"), Some(&1));
    }

    #[test]
    fn action_values_replace_the_items_own() {
        let mut chnm = 2u32.to_be_bytes().to_vec();
        chnm.extend(s0("rough"));
        chnm.extend(s0("effect"));
        let mut actn = s0("setup");
        actn.extend(s0("setup"));
        actn.extend_from_slice(&1u32.to_be_bytes());
        actn.extend(sub(b"ITEM", 5u32.to_be_bytes().to_vec()));
        let mut ch = 0u16.to_be_bytes().to_vec();
        ch.extend_from_slice(&2u16.to_be_bytes());
        ch.extend_from_slice(&0u16.to_be_bytes());
        ch.extend_from_slice(&0.75f32.to_be_bytes());
        actn.extend(sub(b"CHAN", ch));
        let mut st = s0("");
        st.extend_from_slice(&1u16.to_be_bytes());
        st.extend(s0("rough"));
        actn.extend(sub(b"CHNS", st));
        let bytes = form(
            b"LXPR",
            vec![
                chunk(b"CHNM", chnm),
                item("noise", "N", 5, vec![float("rough", 0.1)]),
                chunk(b"ACTN", actn),
            ],
        );
        let p = read(&bytes).unwrap();
        assert_eq!(p.items[&5].number("rough"), Some(0.75));
        assert_eq!(p.items[&5].text("effect"), Some("rough"));
    }

    /// An ACTN of `kind` (id `id`, parent `parent`) holding `subs`.
    fn actn(kind: &str, id: u32, parent: Option<u32>, subs: Vec<Vec<u8>>) -> Vec<u8> {
        let mut d = s0(kind);
        d.extend(s0(kind));
        d.extend_from_slice(&id.to_be_bytes());
        if let Some(p) = parent {
            d.extend(sub(b"PRNT", p.to_be_bytes().to_vec()));
        }
        for s in subs {
            d.extend(s);
        }
        chunk(b"ACTN", d)
    }
    fn owner(id: u32) -> Vec<u8> {
        sub(b"ITEM", id.to_be_bytes().to_vec())
    }
    /// Action GRAD: VX name index, VX envelope, U4 flags, S0 name.
    fn action_grad(index: u16, envelope: u16, flags: u32, name: &str) -> Vec<u8> {
        let mut d = index.to_be_bytes().to_vec();
        d.extend_from_slice(&envelope.to_be_bytes());
        d.extend_from_slice(&flags.to_be_bytes());
        d.extend(s0(name));
        d.extend(s0("percent"));
        d.extend(s0("float"));
        sub(b"GRAD", d)
    }
    /// Action CHAN: VX name index, U2 float type, VX envelope 0, value.
    fn action_float(index: u16, v: f32) -> Vec<u8> {
        let mut d = index.to_be_bytes().to_vec();
        d.extend_from_slice(&2u16.to_be_bytes());
        d.extend_from_slice(&0u16.to_be_bytes());
        d.extend_from_slice(&v.to_be_bytes());
        sub(b"CHAN", d)
    }
    fn chnm(names: &[&str]) -> Vec<u8> {
        let mut d = (names.len() as u32).to_be_bytes().to_vec();
        for n in names {
            d.extend(s0(n));
        }
        chunk(b"CHNM", d)
    }

    /// LXP-B07a: ITEM-level GRADs are named by S0 (empty = `value`);
    /// action GRADs by CHNM index, or their S0 name when the index is 0;
    /// actions merge by channel name, child after parent, `anim` left out;
    /// action channel values reach every item, not only gradients.
    #[test]
    fn gradient_records_from_items_and_actions() {
        let names = chnm(&["", "color.R", "color.G", "color.B", "diffAmt"]);
        let bytes = form(
            b"LXPR",
            vec![
                names,
                item("gradient", "", 8, vec![grad("", 1), grad("color.B", 1)]),
                item("advancedMaterial", "M", 9, vec![float("diffAmt", 0.1)]),
                // The child edit action, written before its setup parent.
                actn(
                    "edit",
                    21,
                    Some(20),
                    vec![
                        owner(8),
                        action_grad(2, 4, 1, ""),
                        owner(9),
                        action_float(4, 0.7),
                    ],
                ),
                actn(
                    "setup",
                    20,
                    None,
                    vec![
                        owner(8),
                        action_grad(1, 2, 0, ""),
                        action_grad(2, 3, 0, ""),
                        action_grad(0, 5, 2, "color.A"),
                        owner(9),
                        action_float(4, 0.5),
                    ],
                ),
                actn("anim", 22, Some(20), vec![owner(9), action_float(4, 0.9)]),
            ],
        );
        let p = read(&bytes).unwrap();
        let g: BTreeMap<_, _> = p.items[&8]
            .gradients
            .iter()
            .map(|g| (g.channel.as_str(), (g.envelope, g.interpolation)))
            .collect();
        assert_eq!(g["value"], (1, Some(0)));
        assert_eq!(g["color.R"], (2, Some(0)));
        assert_eq!(g["color.G"], (4, Some(1)), "child edit beats setup");
        assert_eq!(g["color.B"], (1, Some(0)), "item-level kept");
        assert_eq!(g["color.A"], (5, Some(2)), "S0 name when index is 0");
        assert_eq!(p.items[&9].number("diffAmt"), Some(0.7_f32 as f64));
        assert_eq!(p.skipped.get("ACTN anim (not applied)"), Some(&1));
        let types = &p.items[&8]
            .gradients
            .iter()
            .find(|g| g.channel == "color.R")
            .unwrap()
            .types;
        assert_eq!(types, &Some(("percent".into(), "float".into())));
    }

    #[test]
    fn broken_records_refuse_the_preset() {
        assert!(read(b"PK\x03\x04 not a form").is_err());
        // An ITEM whose CHNL is cut short.
        let mut d = s0("noise");
        d.extend(s0("N"));
        d.extend_from_slice(&1u32.to_be_bytes());
        d.extend(sub(b"CHNL", s0("rough")));
        assert!(read(&form(b"LXPR", vec![chunk(b"ITEM", d)])).is_err());
    }
}
