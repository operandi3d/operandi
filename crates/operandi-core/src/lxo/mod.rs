//! Bounded LXO modeling import. Modo scene files follow the clean phase-1
//! spec (`phase1.rs`, lxo-import-phase1-2026-11-05); Modo 8–10 files (format
//! 4.x, lxo-legacy-import-2026-11-03) are read by `decode`/`convert`.
//! Original Rust implementation; no runtime, SDK, third-party parser or GPU API.
mod convert;
mod decode;
pub mod export;
mod lxe;
mod metadata;
mod phase1;
pub mod preset_items;
mod q001;
pub(crate) mod wire;
use crate::Scene;
/// A channel value as the readers store it.
pub use decode::Value;
use std::{fmt, path::Path};
use wire::{Chunk, Reader};

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    pub category: &'static str,
    pub offset: usize,
    pub detail: String,
}
impl Error {
    pub fn new(category: &'static str, offset: usize, detail: &str) -> Self {
        Self {
            category,
            offset,
            detail: detail.chars().take(768).collect(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A wrong format version is the common case for older Modo files;
        // say so plainly instead of leading with the internal category.
        // Procedural modelling refusals are already a plain sentence.
        if self.category == "unsupported_procedural_modelling" {
            return f.write_str(&self.detail);
        }
        if self.category == "unsupported_format_version" {
            return write!(
                f,
                "This LXO is from a Modo version Operandi can't read yet. Operandi reads Modo 8–10, 16 and 17 modeling scenes. ({}, byte {})",
                self.detail, self.offset
            );
        }
        write!(
            f,
            "{} at byte {}: {}. Bounded modeling import only.",
            self.category, self.offset, self.detail
        )
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Debug)]
pub struct Limits {
    pub input_bytes: usize,
    pub working_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            input_bytes: 256 * 1024 * 1024,
            working_bytes: 1024 * 1024 * 1024,
        }
    }
}
/// Returning false requests cancellation. Invoked in all parse/conversion phases.
pub struct Control<'a> {
    pub limits: Limits,
    pub used: usize,
    strings: usize,
    chunks: usize,
    records: usize,
    pub progress: &'a mut dyn FnMut(&str) -> bool,
    /// LXO-LEG-Q05: a format 4.x file with no ENCO or a non-UTF-8 one reads
    /// a name that isn't UTF-8 as Windows-1252 (`Reader::name`).
    pub ansi_names: bool,
    /// Names decoded that way, for the report's note.
    pub ansi_decoded: std::collections::BTreeSet<String>,
}
impl<'a> Control<'a> {
    pub fn new(progress: &'a mut dyn FnMut(&str) -> bool) -> Self {
        Self {
            limits: Limits::default(),
            used: 0,
            strings: 0,
            chunks: 0,
            records: 0,
            progress,
            ansi_names: false,
            ansi_decoded: Default::default(),
        }
    }
    pub fn check(&mut self, stage: &str) -> Result<()> {
        if (self.progress)(stage) {
            Ok(())
        } else {
            Err(Error::new("cancelled", 0, stage))
        }
    }
    pub fn charge(&mut self, n: usize) -> Result<()> {
        self.used = self
            .used
            .checked_add(n)
            .ok_or_else(|| Error::new("resource_limit", 0, "Working memory overflow"))?;
        if self.used > self.limits.working_bytes {
            Err(Error::new(
                "resource_limit",
                0,
                "Aggregate import working memory budget exceeded",
            ))
        } else {
            Ok(())
        }
    }
    fn string(&mut self, n: usize) -> Result<()> {
        self.strings = self
            .strings
            .checked_add(n)
            .ok_or_else(|| Error::new("resource_limit", 0, "String budget overflow"))?;
        if n > 1_048_576 || self.strings > 16 * 1024 * 1024 {
            return Err(Error::new("resource_limit", 0, "String budget exceeded"));
        }
        self.charge(n * 4 + 64)
    }
    fn record(&mut self, sub: bool) -> Result<()> {
        if sub {
            self.records += 1;
        } else {
            self.chunks += 1;
        }
        if self.records > 1_000_000 || self.chunks > 100_000 {
            Err(Error::new(
                "resource_limit",
                0,
                "Metadata record count limit",
            ))
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct AuxiliaryOmissions {
    pub records: usize,
    pub settings: usize,
    pub envelopes: usize,
    pub owners: std::collections::BTreeMap<String, usize>,
    pub channels: std::collections::BTreeMap<String, usize>,
}
#[derive(Clone, Debug, Default)]
pub struct UnreferencedEnvelopes {
    pub ids: Vec<u32>,
    pub profiles: std::collections::BTreeMap<String, usize>,
    pub payload_bytes: usize,
    pub chunk_bytes: usize,
}
#[derive(Clone, Debug, Default)]
pub struct MetadataOmissions {
    pub tag_records: usize,
    pub tag_samples: Vec<String>,
    pub tag_settings: usize,
    pub tag_owners: std::collections::BTreeMap<String, usize>,
    pub tags: std::collections::BTreeMap<String, usize>,
    pub size_records: usize,
    pub size_settings: usize,
}
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub producer: String,
    pub meshes: usize,
    pub points: usize,
    pub polygons: usize,
    pub corners: usize,
    pub uv_maps: Vec<String>,
    pub materials: Vec<String>,
    pub parts: Vec<String>,
    pub hidden: usize,
    pub auxiliary: AuxiliaryOmissions,
    pub metadata: MetadataOmissions,
    pub unreferenced_envelopes: UnreferencedEnvelopes,
    pub envelope_records: usize,
    pub active_envelope_references: usize,
    pub static_envelope_fields: usize,
    pub omissions: std::collections::BTreeMap<String, usize>,
    pub rejected_groups: Vec<String>,
    /// Per-mesh notes on what an import left out or changed.
    pub notes: Vec<String>,
}
impl Report {
    pub fn summary(&self) -> String {
        format!(
            "{} · FORM/LXOB 16.0, UTF-8, Phase 1 ordinary FACE profile\n{} meshes, {} vertices, {} polygons, {} corners; {} hidden\nUV maps ({}): {}\nMaterials ({}): {}\nParts ({}): {}\nOmissions: {}\n{}{}\nMaterial memberships retained; shader appearance unavailable. Saved editing/display state omitted; neutral editing context. Save separately as .opscene; use File > Export for LXO.",
            self.producer,
            self.meshes,
            self.points,
            self.polygons,
            self.corners,
            self.hidden,
            self.uv_maps.len(),
            sample(&self.uv_maps),
            self.materials.len(),
            sample(&self.materials),
            self.parts.len(),
            sample(&self.parts),
            self.omissions
                .iter()
                .map(|(k, v)| format!("{v} {k}"))
                .collect::<Vec<_>>()
                .join(", "),
            self.notes
                .iter()
                .map(|n| format!("Note: {n}\n"))
                .collect::<String>(),
            format_args!(
                "{}\nUnused default envelope records omitted: {} ({} payload bytes / {} chunk bytes); IDs {:?}, profiles {:?}; original owner/lifecycle unresolved.\nAuxiliary omissions: {} serialized records, {} distinct owner/channel settings, {} distinct envelopes; owners {:?}; channels {:?}. Unused default curve profiles and color/render settings are not restored.\nScene/render tags omitted: {} records / {} settings, owners {:?}, tag counts {:?}. Mesh display size omitted: {} settings / {} records; geometry and item scale retained.",
                sample(&self.rejected_groups),
                self.unreferenced_envelopes.ids.len(),
                self.unreferenced_envelopes.payload_bytes,
                self.unreferenced_envelopes.chunk_bytes,
                self.unreferenced_envelopes
                    .ids
                    .iter()
                    .take(16)
                    .collect::<Vec<_>>(),
                self.unreferenced_envelopes.profiles,
                self.auxiliary.records,
                self.auxiliary.settings,
                self.auxiliary.envelopes,
                self.auxiliary.owners,
                self.auxiliary.channels,
                self.metadata.tag_records,
                self.metadata.tag_settings,
                self.metadata.tag_owners,
                self.metadata.tags,
                self.metadata.size_settings,
                self.metadata.size_records
            )
        )
    }
    fn omit(&mut self, kind: impl Into<String>) {
        *self.omissions.entry(kind.into()).or_default() += 1;
    }
}
fn sample(names: &[String]) -> String {
    let text = names
        .iter()
        .take(16)
        // Long enough for a whole reason; cut ones say so.
        .map(|s| {
            if s.chars().count() > 400 {
                format!("{}…", s.chars().take(400).collect::<String>())
            } else {
                s.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    if names.len() > 16 {
        format!("{text} … ({} total)", names.len())
    } else {
        text
    }
}
/// A candidate never mutates a current Editor. Rejected groups require explicit UI choice.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub scene: Scene,
    pub report: Report,
}
impl Candidate {
    pub fn strict(self) -> Result<Self> {
        if self.report.rejected_groups.is_empty() {
            Ok(self)
        } else {
            Err(Error::new(
                "unsupported_modeling_group",
                0,
                &sample(&self.report.rejected_groups),
            ))
        }
    }
}
/// Working memory charged per source byte: the source buffer itself plus a
/// conservative allowance for decoded and candidate expansion, in addition to
/// explicit record charges (format/01 "Approved resource limits").
fn source_reserve(bytes: usize) -> Result<usize> {
    bytes
        .checked_mul(8)
        .ok_or_else(|| Error::new("resource_limit", 0, "Input accounting overflow"))
}
pub fn parse(bytes: &[u8], control: &mut Control<'_>) -> Result<Candidate> {
    control.check("framing")?;
    if bytes.len() > control.limits.input_bytes {
        return Err(Error::new("file_size_limit", 0, "Input exceeds 256 MiB"));
    }
    control.charge(source_reserve(bytes.len())?)?;
    parse_reserved(bytes, control, SCENE_FORMS)
}
/// `FORM` types a scene file may have.
const SCENE_FORMS: &[[u8; 4]] = &[*b"LXOB"];
/// `FORM` types a Modo mesh preset (`.lxl`) may have (MC-11). The SDK docs
/// name `LXPM` for mesh presets without describing its body, so the body
/// is read exactly like a scene's; `LXOB` is accepted too, since the docs
/// don't say every `.lxl` uses `LXPM` (MC-Q03, provisional).
const MESH_PRESET_FORMS: &[[u8; 4]] = &[*b"LXPM", *b"LXOB"];
/// Parse a source whose reserve has already been charged: by `parse` for the
/// caller's slice, or by `load` for every byte of its buffer's capacity.
fn parse_reserved(bytes: &[u8], control: &mut Control<'_>, forms: &[[u8; 4]]) -> Result<Candidate> {
    if bytes.len() < 12 {
        return Err(Error::new(
            "truncated_header",
            0,
            "Expected 12-byte FORM/LXOB header",
        ));
    }
    let mut r = Reader::new(bytes, 0);
    if r.id()? != *b"FORM" {
        return Err(r.error("wrong_form_type", "Expected FORM container"));
    }
    let n = r.u32()?;
    if n == 0 {
        return Err(r.error("unsupported_extended_size", "Zero FORM length is deferred"));
    }
    if u64::from(n) + 8 != bytes.len() as u64 {
        return Err(r.error("form_length_mismatch", "FORM length must match entire file"));
    }
    let form = r.id()?;
    if !forms.contains(&form) {
        return Err(r.error(
            "wrong_form_type",
            &if forms == SCENE_FORMS {
                "Expected LXOB, not LWO/preset".to_owned()
            } else {
                format!(
                    "Expected a mesh preset (LXPM), not {}",
                    String::from_utf8_lossy(&form)
                )
            },
        ));
    }
    let chunks = r.chunks(false, control)?;
    // Operandi's own LXO export profile first; anything else gets the Modo
    // profile rules unchanged (LXE-IMP-01).
    if let Some(candidate) = lxe::read(&chunks, control)? {
        return Ok(candidate);
    }
    // Format 4.x (Modo 8–10) and older go to the legacy reader, which also
    // refuses 2.x and 3.x; everything else is read to the phase-1 spec.
    let vrsn = chunks
        .iter()
        .find(|c| &c.id == b"VRSN")
        .and_then(|c| c.data.get(..8))
        .map(|b| {
            (
                u32::from_be_bytes(b[..4].try_into().unwrap()),
                u32::from_be_bytes(b[4..].try_into().unwrap()),
            )
        });
    let result = if matches!(vrsn, Some((2..=4, _))) {
        decode::decode(&chunks, control)
            .and_then(|raw| convert::convert(raw, control, form == *b"LXPM"))
    } else {
        phase1::read(&chunks, control, form == *b"LXPM")
    };
    // Every refusal names the file's format version, so a folder of
    // refusals can be sorted by version from the message alone.
    result.map_err(|mut e| {
        if let Some((major, minor)) = vrsn {
            e.detail = format!("{} [LXO format {major}.{minor}]", e.detail);
        }
        e
    })
}
/// A source buffer whose every byte of capacity was charged its reserve before
/// it was allocated. Capacity, not length, is charged, because capacity is
/// what is allocated.
struct SourceBuffer {
    bytes: Vec<u8>,
    charged: usize,
}
impl SourceBuffer {
    /// Grow capacity to at least `capacity`, charging the added capacity first.
    fn reserve(&mut self, capacity: usize, control: &mut Control<'_>) -> Result<()> {
        if capacity > self.charged {
            control.charge(source_reserve(capacity - self.charged)?)?;
            self.charged = capacity;
        }
        let additional = capacity.saturating_sub(self.bytes.len());
        self.bytes
            .try_reserve_exact(additional)
            .map_err(|_| Error::new("allocation_failure", 0, "Source buffer"))?;
        // `try_reserve_exact` normally gives exactly the requested capacity;
        // charge any extra so the reserve still covers all of it.
        if self.bytes.capacity() > self.charged {
            control.charge(source_reserve(self.bytes.capacity() - self.charged)?)?;
            self.charged = self.bytes.capacity();
        }
        Ok(())
    }
}
/// Read a whole source under both limits. A stated size (file metadata) that
/// passed the input cap is reserved and charged before the first read. Each
/// block beyond the current capacity is checked against the input cap, then
/// capacity grows (doubling where the working budget allows, otherwise exactly)
/// with its reserve charged before the buffer grows. The stated size is only a
/// hint: the result is what was actually read, which may be shorter or longer.
fn read_source(
    mut source: impl std::io::Read,
    stated: u64,
    control: &mut Control<'_>,
) -> Result<SourceBuffer> {
    let input = control.limits.input_bytes;
    let stated = usize::try_from(stated)
        .ok()
        .filter(|&n| n <= input)
        .ok_or_else(|| Error::new("file_size_limit", 0, "Input exceeds 256 MiB"))?;
    let mut buffer = SourceBuffer {
        bytes: Vec::new(),
        charged: 0,
    };
    buffer.reserve(stated, control)?;
    let mut block = [0u8; 65536];
    loop {
        control.check("reading")?;
        let n = source
            .read(&mut block)
            .map_err(|e| Error::new("io", 0, &e.to_string()))?;
        if n == 0 {
            return Ok(buffer);
        }
        let len = buffer
            .bytes
            .len()
            .checked_add(n)
            .filter(|&len| len <= input)
            .ok_or_else(|| Error::new("file_size_limit", 0, "Input grew beyond budget"))?;
        if len > buffer.bytes.capacity() {
            let doubled = buffer.bytes.capacity().saturating_mul(2).clamp(len, input);
            let affordable = source_reserve(doubled - buffer.charged.min(doubled))
                .ok()
                .and_then(|n| control.used.checked_add(n))
                .is_some_and(|total| total <= control.limits.working_bytes);
            buffer.reserve(if affordable { doubled } else { len }, control)?;
        }
        buffer.bytes.extend_from_slice(&block[..n]);
    }
}
pub fn load(path: &Path, control: &mut Control<'_>) -> Result<Candidate> {
    load_forms(path, control, SCENE_FORMS)
}
/// Read a Modo mesh preset (`.lxl`, MC-11) through the scene reader.
/// Unknown chunks are skipped by size or refuse the file, as for scenes
/// (Q-MC1, black-box only). A preset with no mesh Operandi can read is
/// refused, so a load never adds an empty item.
pub fn load_mesh_preset(path: &Path, control: &mut Control<'_>) -> Result<Candidate> {
    let candidate = load_forms(path, control, MESH_PRESET_FORMS)?;
    if candidate.report.meshes == 0 {
        return Err(Error::new(
            "no_modeling_geometry",
            0,
            "The mesh preset holds no mesh Operandi can read",
        ));
    }
    Ok(candidate)
}
fn load_forms(path: &Path, control: &mut Control<'_>, forms: &[[u8; 4]]) -> Result<Candidate> {
    let file = std::fs::File::open(path).map_err(|e| Error::new("io", 0, &e.to_string()))?;
    let meta = file
        .metadata()
        .map_err(|e| Error::new("io", 0, &e.to_string()))?;
    if meta.len() > control.limits.input_bytes as u64 {
        return Err(Error::new("file_size_limit", 0, "Input exceeds 256 MiB"));
    }
    // The buffer lives until `load` returns, as before this preflight.
    let buffer = read_source(file, meta.len(), control)?;
    control.check("framing")?;
    let mut candidate = parse_reserved(&buffer.bytes, control, forms)?;
    let mut source = crate::ImportSource {
        path: path
            .canonicalize()
            .unwrap_or_else(|_| path.to_owned())
            .display()
            .to_string(),
        producer: candidate.report.producer.clone(),
        device: None,
        inode: None,
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        source.device = Some(meta.dev());
        source.inode = Some(meta.ino());
    }
    candidate.scene.import_source = Some(source);
    Ok(candidate)
}

/// `read_source` with deterministic in-memory readers: short reads, growth
/// past the stated size, the doubling fallback, both caps and read errors.
#[cfg(test)]
mod source_tests {
    use super::*;

    /// Yields `data` in reads of at most `chunk` bytes.
    struct Chunks<'a> {
        data: &'a [u8],
        chunk: usize,
    }
    impl std::io::Read for Chunks<'_> {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            let n = self.data.len().min(self.chunk).min(out.len());
            out[..n].copy_from_slice(&self.data[..n]);
            self.data = &self.data[n..];
            Ok(n)
        }
    }

    /// (result, final used, "reading" callbacks)
    fn read(
        data: &[u8],
        chunk: usize,
        stated: u64,
        limits: Limits,
    ) -> (Result<SourceBuffer>, usize, usize) {
        let mut reads = 0;
        let mut progress = |_: &str| {
            reads += 1;
            true
        };
        let mut control = Control::new(&mut progress);
        control.limits = limits;
        let result = read_source(Chunks { data, chunk }, stated, &mut control);
        let used = control.used;
        (result, used, reads)
    }

    fn limits(input_bytes: usize, working_bytes: usize) -> Limits {
        Limits {
            input_bytes,
            working_bytes,
        }
    }

    #[test]
    fn short_read_keeps_the_stated_capacity_charge() {
        let (r, used, reads) = read(&[7; 10], 4, 30, Limits::default());
        let b = r.unwrap();
        assert_eq!((b.bytes.len(), b.bytes.capacity(), b.charged), (10, 30, 30));
        assert_eq!((used, reads), (240, 4));
    }

    #[test]
    fn growth_past_the_stated_size_is_charged_per_capacity() {
        let data = vec![1u8; 100_000];
        let (r, used, _) = read(&data, 40_000, 10, Limits::default());
        let b = r.unwrap();
        assert_eq!(b.bytes, data);
        assert_eq!(b.charged, b.bytes.capacity());
        assert_eq!(used, 8 * b.bytes.capacity());
    }

    #[test]
    fn doubling_falls_back_to_exact_growth_within_the_budget() {
        // 1000, 2000, then 4000 would cost 32000; exact 3000 fits 24000.
        let (r, used, _) = read(&[2; 3000], 1000, 0, limits(1 << 20, 24_000));
        let b = r.unwrap();
        assert_eq!(
            (b.bytes.len(), b.bytes.capacity(), used),
            (3000, 3000, 24_000)
        );
        // One byte less budget refuses the exact growth.
        let (r, used, _) = read(&[2; 3000], 1000, 0, limits(1 << 20, 23_999));
        assert_eq!(r.err().unwrap().category, "resource_limit");
        assert_eq!(used, 24_000);
    }

    #[test]
    fn growth_past_the_input_cap_refuses_before_growing() {
        let (r, used, reads) = read(&[3; 2500], 1000, 0, limits(2000, 1 << 20));
        let e = r.err().unwrap();
        assert_eq!(
            (e.category, e.detail.as_str()),
            ("file_size_limit", "Input grew beyond budget")
        );
        assert_eq!((used, reads), (8 * 2000, 3));
    }

    #[test]
    fn stated_size_over_the_cap_charges_nothing() {
        let (r, used, reads) = read(&[], 1, 5000, limits(4999, 1 << 20));
        assert_eq!(r.err().unwrap().category, "file_size_limit");
        assert_eq!((used, reads), (0, 0));
    }

    #[test]
    fn read_errors_are_reported() {
        struct Failing;
        impl std::io::Read for Failing {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("device gone"))
            }
        }
        let mut progress = |_: &str| true;
        let mut control = Control::new(&mut progress);
        let e = read_source(Failing, 8, &mut control).err().unwrap();
        assert_eq!((e.category, e.detail.as_str()), ("io", "device gone"));
        assert_eq!(control.used, 64);
    }
}

/// MC-11: Modo mesh presets go through the scene reader.
#[cfg(test)]
mod mesh_preset_tests {
    use super::*;
    use glam::DVec3;

    /// A one-triangle scene written by Operandi's LXO export, its FORM
    /// type set to `form` and `extra` chunks put first (as a preset's
    /// DESC and PRVW might be).
    fn preset_bytes(form: &[u8; 4], extra: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        let item = scene.items.last_mut().unwrap();
        item.name = "Fish".into();
        item.mesh = crate::Mesh {
            vertices: vec![DVec3::ZERO, DVec3::X, DVec3::Y],
            faces: vec![crate::Face {
                vertices: vec![0, 1, 2],
                uv: None,
                material: "Scales".into(),
                smooth: false,
                part: None,
                material_label: None,
            }]
            .into(),
            ..Default::default()
        }
        .into();
        let bytes = export::write(&scene, &mut Vec::new()).unwrap();
        let mut body = form.to_vec();
        for (id, data) in extra {
            body.extend_from_slice(*id);
            body.extend_from_slice(&(data.len() as u32).to_be_bytes());
            body.extend_from_slice(data);
            if data.len() % 2 == 1 {
                body.push(0);
            }
        }
        body.extend_from_slice(&bytes[12..]);
        let mut out = b"FORM".to_vec();
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend(body);
        out
    }

    fn load(bytes: &[u8], name: &str) -> Result<Candidate> {
        let dir = std::env::temp_dir().join(format!("operandi-lxl-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.lxl"));
        std::fs::write(&path, bytes).unwrap();
        let mut keep_going = |_: &str| true;
        let result = load_mesh_preset(&path, &mut Control::new(&mut keep_going));
        std::fs::remove_dir_all(&dir).unwrap();
        result
    }

    #[test]
    fn a_mesh_preset_form_reads_like_a_scene_and_skips_preset_chunks() {
        let png = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n', 1, 2, 3];
        let bytes = preset_bytes(b"LXPM", &[(b"DESC", b"mesh\0Fish\0"), (b"PRVW", &png)]);
        let c = load(&bytes, "lxpm").unwrap();
        let mesh = c.scene.items.iter().find(|i| i.name == "Fish").unwrap();
        assert_eq!(mesh.mesh.faces.len(), 1);
        assert_eq!(mesh.mesh.vertices.len(), 3);
        // The same file as a scene is refused: scenes are LXOB only.
        let mut keep_going = |_: &str| true;
        let e = parse(&bytes, &mut Control::new(&mut keep_going)).unwrap_err();
        assert_eq!(e.category, "wrong_form_type");
    }

    #[test]
    fn an_lxob_mesh_preset_reads_too() {
        let c = load(&preset_bytes(b"LXOB", &[]), "lxob").unwrap();
        assert_eq!(c.report.meshes, 1);
    }

    #[test]
    fn other_preset_kinds_are_refused_without_reading_them() {
        let e = load(&preset_bytes(b"LXPR", &[]), "lxpr").unwrap_err();
        assert_eq!(e.category, "wrong_form_type");
        assert!(e.detail.contains("LXPR"), "{}", e.detail);
    }

    #[test]
    fn a_preset_with_no_mesh_is_refused() {
        // Only the header chunks: nothing to add.
        let mut body = b"LXPM".to_vec();
        body.extend_from_slice(b"DESC\0\0\0\x06mesh\0\0");
        let mut bytes = b"FORM".to_vec();
        bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
        bytes.extend(body);
        let e = load(&bytes, "empty").unwrap_err();
        assert!(
            matches!(e.category, "no_modeling_geometry" | "truncated_payload")
                || e.category.starts_with("missing"),
            "{e:?}"
        );
    }
}
