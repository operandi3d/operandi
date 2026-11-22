//! The small part of ZIP that 3MF packages need: read stored and deflated
//! entries, write deflated ones. No ZIP64, encryption or spanning. Written
//! from PKWARE's public APPNOTE; deflate itself is `flate2`.
use crate::Result;
use std::io::{Read, Write};

const LOCAL: u32 = 0x0403_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const END: u32 = 0x0605_4b50;
/// Most entries a package may list, and the largest an entry may inflate
/// to: guards against a hostile archive.
const MAX_ENTRIES: usize = 10_000;
const MAX_ENTRY_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// True when the bytes start like a ZIP archive.
pub(crate) fn is_zip(bytes: &[u8]) -> bool {
    u32_at(bytes, 0) == Some(LOCAL)
}

/// Every file entry of the archive as (name, contents).
pub(crate) fn read(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>> {
    let bad = || "Not a readable ZIP package".to_string();
    // The end record is in the last 64 KiB + 22 bytes (its comment comes after it).
    let lowest = bytes.len().saturating_sub(65_557);
    let end = (lowest..=bytes.len().saturating_sub(22))
        .rev()
        .find(|&i| u32_at(bytes, i) == Some(END))
        .ok_or_else(bad)?;
    let count = u16_at(bytes, end + 10).ok_or_else(bad)? as usize;
    let mut at = u32_at(bytes, end + 16).ok_or_else(bad)? as usize;
    if count > MAX_ENTRIES {
        return Err(format!("The package has more than {MAX_ENTRIES} entries"));
    }
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if u32_at(bytes, at) != Some(CENTRAL) {
            return Err(bad());
        }
        let flags = u16_at(bytes, at + 8).ok_or_else(bad)?;
        let method = u16_at(bytes, at + 10).ok_or_else(bad)?;
        let packed = u32_at(bytes, at + 20).ok_or_else(bad)? as usize;
        let size = u64::from(u32_at(bytes, at + 24).ok_or_else(bad)?);
        let name_len = u16_at(bytes, at + 28).ok_or_else(bad)? as usize;
        let extra_len = u16_at(bytes, at + 30).ok_or_else(bad)? as usize;
        let comment_len = u16_at(bytes, at + 32).ok_or_else(bad)? as usize;
        let local = u32_at(bytes, at + 42).ok_or_else(bad)? as usize;
        let name = bytes.get(at + 46..at + 46 + name_len).ok_or_else(bad)?;
        let name = String::from_utf8_lossy(name).into_owned();
        at += 46 + name_len + extra_len + comment_len;
        if name.ends_with('/') {
            continue;
        }
        if flags & 1 != 0 {
            return Err(format!("{name} is encrypted"));
        }
        if size > MAX_ENTRY_BYTES {
            return Err(format!("{name} is too large"));
        }
        if u32_at(bytes, local) != Some(LOCAL) {
            return Err(bad());
        }
        let data_at = local
            + 30
            + u16_at(bytes, local + 26).ok_or_else(bad)? as usize
            + u16_at(bytes, local + 28).ok_or_else(bad)? as usize;
        let data = bytes.get(data_at..data_at + packed).ok_or_else(bad)?;
        let contents = match method {
            0 => data.to_vec(),
            8 => {
                let mut v = Vec::with_capacity(size.min(64 << 20) as usize);
                flate2::read::DeflateDecoder::new(data)
                    .take(MAX_ENTRY_BYTES)
                    .read_to_end(&mut v)
                    .map_err(|e| format!("{name}: {e}"))?;
                v
            }
            m => {
                return Err(format!(
                    "{name} uses ZIP compression {m}, which Operandi can't read"
                ));
            }
        };
        out.push((name, contents));
    }
    Ok(out)
}

/// An archive of deflated entries, in the order given.
pub(crate) fn write(entries: &[(&str, &[u8])]) -> Result<Vec<u8>> {
    let too_big = || "The package is over 4 GB; ZIP without ZIP64 can't hold it".to_string();
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let mut packed =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        packed.write_all(data).map_err(|e| e.to_string())?;
        let packed = packed.finish().map_err(|e| e.to_string())?;
        let crc = crc32fast::hash(data);
        let offset = u32::try_from(out.len()).map_err(|_| too_big())?;
        let size = u32::try_from(data.len()).map_err(|_| too_big())?;
        let packed_size = u32::try_from(packed.len()).map_err(|_| too_big())?;
        // Version 2.0, no flags, deflate, a fixed 1980-01-01 date so the
        // same scene gives the same bytes.
        let common = |b: &mut Vec<u8>| {
            b.extend(20u16.to_le_bytes());
            b.extend(0u16.to_le_bytes());
            b.extend(8u16.to_le_bytes());
            b.extend(0u16.to_le_bytes());
            b.extend(0x21u16.to_le_bytes());
            b.extend(crc.to_le_bytes());
            b.extend(packed_size.to_le_bytes());
            b.extend(size.to_le_bytes());
            b.extend((name.len() as u16).to_le_bytes());
            b.extend(0u16.to_le_bytes());
        };
        out.extend(LOCAL.to_le_bytes());
        common(&mut out);
        out.extend(name.as_bytes());
        out.extend(&packed);
        central.extend(CENTRAL.to_le_bytes());
        central.extend(20u16.to_le_bytes());
        common(&mut central);
        central.extend([0u8; 8]); // comment length, disk, internal and external attributes
        central.extend([0u8; 2]);
        central.extend(offset.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let central_at = u32::try_from(out.len()).map_err(|_| too_big())?;
    let central_len = central.len() as u32;
    out.extend(central);
    out.extend(END.to_le_bytes());
    out.extend([0u8; 4]);
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend(central_len.to_le_bytes());
    out.extend(central_at.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    Ok(out)
}
