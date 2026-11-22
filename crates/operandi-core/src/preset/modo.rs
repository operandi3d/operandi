//! What the preset browser learns from a Modo file without understanding it
//! (BEHAVIOR-MODO-CONTENT-FOLDER MC-03, MC-12, MC-30).
//!
//! Modo's scenes and presets share one published container: an IFF-style,
//! big-endian `FORM` whose type says what the file is (`LXOB` scene, `LXPM`
//! mesh preset, `LXPR` preset, `LXPE` environment). Two chunks are
//! documented for presets: `DESC` (the preset's primary item type and a
//! user-friendly description) and `PRVW` (compressed image data, PNG).
//!
//! Black-box only (Q-MC1, Jason 2026-11-12): this reader looks at the
//! `FORM` header and those chunks and skips every other chunk by its size,
//! seeking past it, so a many-GB content pack lists quickly. Anything that
//! doesn't fit the published layout is ignored; nothing here tries to work
//! out what an unknown chunk holds. Provisional readings (MC-Q01..) are in
//! docs/SPECIFICATION_QUESTIONS.md.
//!
//! The `PRVW` header layout (width, height, pixel format, flags; flags 1 =
//! PNG) is adapted from lxoxide's `Preview` record
//! (<https://github.com/rondreas/lxoxide>, `src/meta.rs` at ea7ede6),
//! Copyright (c) 2026 Andreas Rånman, used under the MIT licence (lxoxide
//! is MIT OR Apache-2.0; see NOTICE and docs/DEPENDENCIES.md).
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Modo files the browser lists by extension (A2). Scripts, macros and
/// config files are never listed ([`NEVER_SHOWN`]).
pub const MESH_PRESET_EXTENSION: &str = "lxl";
/// Listed but greyed (MC-12): generic presets, environment presets, web
/// links. Material presets (`.lxp` naming a material) are usable
/// (LXP-01, `library::is_modo_material`).
pub const UNSUPPORTED_EXTENSIONS: [&str; 3] = ["lxp", "lxe", "lrl"];
/// MC-13: never shown, with "Show only usable files" on or off. Operandi
/// never runs anything from a content folder.
pub const NEVER_SHOWN: [&str; 5] = ["lxm", "py", "pl", "lua", "cfg"];

/// The largest `DESC` read (it is two short strings).
const DESC_LIMIT: u32 = 64 * 1024;
/// The largest `PRVW` read: a thumbnail, not a picture.
const PRVW_LIMIT: u32 = 16 * 1024 * 1024;
/// How many top-level chunks are looked at before giving up: the
/// documented chunks sit among the file's first ones in practice, and a
/// 10 GB scene must not take thousands of seeks per listing.
const CHUNK_LIMIT: usize = 4096;
/// The 8-byte signature every PNG starts with (PNG's own spec).
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
/// `PRVW`'s header: u16 width, u16 height, u32 pixel format, u32 flags.
const PRVW_HEADER: usize = 12;
/// How far into a `PRVW` whose header doesn't say PNG a PNG signature is
/// still looked for (older provisional reading, MC-Q02).
const PNG_SEARCH: usize = 64;

/// What a Modo file's header and documented chunks say.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModoInfo {
    /// The `FORM` type (`LXOB`, `LXPM`, …).
    pub form: [u8; 4],
    /// `DESC`'s primary item type, when it reads as text (MC-Q01).
    pub item_type: Option<String>,
    /// `DESC`'s user-friendly description, when there is one.
    pub description: Option<String>,
    /// Where `PRVW`'s PNG starts and how long the chunk runs from there.
    pub preview: Option<(u64, u32)>,
}

impl ModoInfo {
    /// The `FORM` type as text ("LXOB").
    pub fn form_name(&self) -> String {
        String::from_utf8_lossy(&self.form).into_owned()
    }

    /// The kind word for tooltips and the list view (MC-12, MC-40): from
    /// `DESC`'s item type when it names a kind, else from the `FORM` type.
    /// Lower case, without "Modo" or "preset" ("material", "environment").
    pub fn kind_word(&self) -> &'static str {
        if let Some(t) = &self.item_type {
            let t = t.to_ascii_lowercase();
            // MC-Q01: provisional; DESC's item type names aren't listed
            // anywhere public, so only plain words are matched.
            for (needle, word) in [
                ("material", "material"),
                ("assembly", "assembly"),
                ("environment", "environment"),
                ("render", "render"),
                ("profile", "profile"),
                ("brush", "brush"),
                ("mesh", "mesh"),
                ("light", "light"),
                ("camera", "camera"),
                ("image", "image"),
                ("texture", "texture"),
            ] {
                if t.contains(needle) {
                    return word;
                }
            }
        }
        match &self.form {
            b"LXPM" => "mesh",
            b"LXPE" => "environment",
            b"LXOB" => "scene",
            _ => "",
        }
    }

    /// "Material preset", "Mesh preset", "Preset" (MC-12).
    pub fn kind_label(&self) -> String {
        match self.kind_word() {
            "" => "Preset".into(),
            "scene" => "Scene".into(),
            w => {
                let mut c = w.chars();
                let first = c.next().map(|f| f.to_ascii_uppercase()).unwrap_or(' ');
                format!("{first}{} preset", c.as_str())
            }
        }
    }
}

/// Read `path`'s `FORM` header and its `DESC` and `PRVW` chunks (MC-03).
/// `None` when it isn't a `FORM` file at all.
pub fn read_info(path: &Path) -> Option<ModoInfo> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    read_info_from(&mut file, len)
}

/// [`read_info`] on any seekable source of `len` bytes.
pub fn read_info_from(source: &mut (impl Read + Seek), len: u64) -> Option<ModoInfo> {
    let mut head = [0u8; 12];
    source.read_exact(&mut head).ok()?;
    if &head[..4] != b"FORM" {
        return None;
    }
    let mut info = ModoInfo {
        form: head[8..12].try_into().ok()?,
        ..Default::default()
    };
    // The FORM length bounds the chunks; a short file (still copying, or
    // cut off) is read as far as it goes.
    let form_len = u64::from(u32::from_be_bytes(head[4..8].try_into().ok()?));
    let end = (form_len + 8).min(len);
    let mut at = 12u64;
    for _ in 0..CHUNK_LIMIT {
        if at + 8 > end || (info.item_type.is_some() && info.preview.is_some()) {
            break;
        }
        source.seek(SeekFrom::Start(at)).ok()?;
        let mut header = [0u8; 8];
        if source.read_exact(&mut header).is_err() {
            break;
        }
        let size = u32::from_be_bytes(header[4..8].try_into().ok()?);
        let data_at = at + 8;
        if data_at + u64::from(size) > end {
            break;
        }
        match &header[..4] {
            b"DESC" if size <= DESC_LIMIT && info.item_type.is_none() => {
                let mut data = vec![0; size as usize];
                if source.read_exact(&mut data).is_ok() {
                    let (t, d) = desc_strings(&data);
                    info.item_type = t;
                    info.description = d;
                }
            }
            b"PRVW" if size <= PRVW_LIMIT && info.preview.is_none() => {
                let mut data = vec![0; (size as usize).min(PNG_SEARCH + PNG_SIGNATURE.len())];
                if source.read_exact(&mut data).is_ok() {
                    info.preview =
                        preview_start(&data).map(|i| (data_at + i as u64, size - i as u32));
                }
            }
            _ => {}
        }
        // IFF: odd-sized chunks carry one pad byte.
        at = data_at + u64::from(size) + u64::from(size % 2);
    }
    Some(info)
}

/// Where the PNG starts in a `PRVW` payload's first bytes: right after the
/// 12-byte header when its flags say PNG (1), else wherever PNG's signature
/// appears in the first bytes; `None` without one.
fn preview_start(data: &[u8]) -> Option<usize> {
    let flags = data
        .get(8..PRVW_HEADER)
        .map(|b| u32::from_be_bytes(b.try_into().unwrap()));
    if flags == Some(1) && data[PRVW_HEADER..].starts_with(&PNG_SIGNATURE) {
        return Some(PRVW_HEADER);
    }
    data.windows(8).position(|w| w == PNG_SIGNATURE)
}

/// MC-Q01 provisional: `DESC` holds two zero-terminated strings (LWO2's S0
/// strings, as everywhere else in the container): the item type, then the
/// description. Anything that isn't plain UTF-8 text is dropped.
fn desc_strings(data: &[u8]) -> (Option<String>, Option<String>) {
    let mut parts = data.split(|b| *b == 0).filter(|p| !p.is_empty()).map(|p| {
        std::str::from_utf8(p)
            .ok()
            .map(str::trim)
            .map(str::to_owned)
    });
    let text = |s: Option<Option<String>>| {
        s.flatten()
            .filter(|s| !s.is_empty() && s.chars().all(|c| !c.is_control()))
    };
    let item_type = text(parts.next());
    let description = text(parts.next());
    (item_type, description)
}

/// The PNG bytes of `path`'s `PRVW` chunk (MC-30), if it has one.
pub fn preview_png(path: &Path) -> Option<Vec<u8>> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let (at, size) = read_info_from(&mut file, len)?.preview?;
    file.seek(SeekFrom::Start(at)).ok()?;
    let mut png = vec![0; size as usize];
    file.read_exact(&mut png).ok()?;
    Some(png)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// A FORM file of `form` type holding `chunks`, framed to the
    /// published layout (big-endian sizes, odd chunks padded).
    pub(crate) fn form(form: &[u8; 4], chunks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut body = form.to_vec();
        for (id, data) in chunks {
            body.extend_from_slice(*id);
            body.extend_from_slice(&(data.len() as u32).to_be_bytes());
            body.extend_from_slice(data);
            if data.len() % 2 == 1 {
                body.push(0);
            }
        }
        let mut out = b"FORM".to_vec();
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend(body);
        out
    }

    fn info(bytes: &[u8]) -> Option<ModoInfo> {
        read_info_from(&mut Cursor::new(bytes), bytes.len() as u64)
    }

    #[test]
    fn reads_form_type_desc_and_skips_unknown_chunks_by_size() {
        let bytes = form(
            b"LXPR",
            &[
                (b"VRSN", vec![1, 2, 3]),
                (b"XXXX", vec![0xff; 1001]),
                (b"DESC", b"advancedMaterial\0Red paint\0".to_vec()),
            ],
        );
        let i = info(&bytes).unwrap();
        assert_eq!(i.form_name(), "LXPR");
        assert_eq!(i.item_type.as_deref(), Some("advancedMaterial"));
        assert_eq!(i.description.as_deref(), Some("Red paint"));
        assert_eq!(i.kind_label(), "Material preset");
        assert_eq!(i.preview, None);
    }

    #[test]
    fn finds_the_png_in_prvw() {
        let mut prvw = vec![0, 0, 0, 1];
        prvw.extend_from_slice(&PNG_SIGNATURE);
        prvw.extend_from_slice(b"rest");
        let bytes = form(b"LXPM", &[(b"PRVW", prvw)]);
        let i = info(&bytes).unwrap();
        let (at, size) = i.preview.unwrap();
        assert_eq!(&bytes[at as usize..at as usize + 8], PNG_SIGNATURE);
        assert_eq!(size, 12);
        assert_eq!(i.kind_label(), "Mesh preset");
    }

    #[test]
    fn prvw_header_says_png_after_twelve_bytes() {
        // 64 x 32, pixel format 0, flags 1 (PNG), then the image.
        let mut prvw = vec![0, 64, 0, 32, 0, 0, 0, 0, 0, 0, 0, 1];
        prvw.extend_from_slice(&PNG_SIGNATURE);
        prvw.extend_from_slice(&[9; 40]);
        let bytes = form(b"LXPM", &[(b"PRVW", prvw)]);
        let (at, size) = info(&bytes).unwrap().preview.unwrap();
        assert_eq!(&bytes[at as usize..at as usize + 8], PNG_SIGNATURE);
        assert_eq!(size, 48);
    }

    #[test]
    fn prvw_without_a_png_is_no_preview() {
        let bytes = form(b"LXPR", &[(b"PRVW", vec![7; 300])]);
        assert_eq!(info(&bytes).unwrap().preview, None);
    }

    #[test]
    fn not_a_form_and_cut_off_files() {
        assert_eq!(info(b"PK\x03\x04 not iff"), None);
        assert_eq!(info(b"FORM"), None);
        let bytes = form(b"LXPE", &[(b"DESC", b"environment\0".to_vec())]);
        // Cut inside DESC: the header still says what the file is.
        let cut = info(&bytes[..bytes.len() - 4]).unwrap();
        assert_eq!(cut.form_name(), "LXPE");
        assert_eq!(cut.item_type, None);
        assert_eq!(cut.kind_label(), "Environment preset");
    }

    #[test]
    fn binary_desc_is_ignored() {
        let bytes = form(b"LXPR", &[(b"DESC", vec![1, 2, 3, 0, 0xff, 0xfe, 0])]);
        let i = info(&bytes).unwrap();
        assert_eq!((&i.item_type, &i.description), (&None, &None));
        assert_eq!(i.kind_label(), "Preset");
    }
}
