use super::{Control, Error, Result};

#[derive(Clone, Copy)]
pub struct Chunk<'a> {
    pub id: [u8; 4],
    pub data: &'a [u8],
    pub offset: usize,
}
pub struct Reader<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    pub base: usize,
}
impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], base: usize) -> Self {
        Self { data, pos: 0, base }
    }
    pub fn error(&self, category: &'static str, detail: &str) -> Error {
        Error::new(category, self.base + self.pos, detail)
    }
    pub fn done(&self) -> bool {
        self.pos == self.data.len()
    }
    pub fn end(&self) -> Result<()> {
        if self.done() {
            Ok(())
        } else {
            Err(self.error("unexpected_payload", "Payload was not consumed exactly"))
        }
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| self.error("length_overflow", "Byte length overflow"))?;
        let out = self
            .data
            .get(self.pos..end)
            .ok_or_else(|| self.error("truncated_payload", "Value exceeds enclosing boundary"))?;
        self.pos = end;
        Ok(out)
    }
    pub fn id(&mut self) -> Result<[u8; 4]> {
        Ok(self.take(4)?.try_into().unwrap())
    }
    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn f32(&mut self) -> Result<f64> {
        let n = f32::from_bits(self.u32()?) as f64;
        if !n.is_finite() {
            return Err(self.error(
                "nonfinite_modeling_value",
                "NaN or infinity is not admitted",
            ));
        }
        Ok(n)
    }
    pub fn vx(&mut self) -> Result<u32> {
        if self.data.get(self.pos) == Some(&255) {
            let n = self
                .u32()
                .map_err(|_| self.error("truncated_index", "Incomplete long VX"))?
                & 0xffffff;
            if n < 65280 {
                return Err(self.error("noncanonical_index", "Long VX below 65280"));
            }
            Ok(n)
        } else {
            self.u16()
                .map(u32::from)
                .map_err(|_| self.error("truncated_index", "Incomplete short VX"))
        }
    }
    pub fn string(&mut self, control: &mut Control<'_>) -> Result<String> {
        let bytes = self.string_bytes(control)?;
        let text = std::str::from_utf8(bytes)
            .map_err(|_| self.error("invalid_utf8", "ENCO=2 requires strict UTF-8"))?;
        Ok(text.to_owned())
    }
    /// A name (display text). Strict UTF-8, except that where
    /// `control.ansi_names` is set (LXO-LEG-Q05) one that isn't UTF-8 is
    /// decoded as Windows-1252 and recorded for a note.
    pub fn name(&mut self, control: &mut Control<'_>) -> Result<String> {
        let bytes = self.string_bytes(control)?;
        match std::str::from_utf8(bytes) {
            Ok(text) => Ok(text.to_owned()),
            Err(_) if control.ansi_names => {
                let text = windows_1252(bytes);
                control.ansi_decoded.insert(text.clone());
                Ok(text)
            }
            Err(_) => Err(self.error("invalid_utf8", "ENCO=2 requires strict UTF-8")),
        }
    }
    /// A bounded S0 string's bytes, terminator and padding consumed.
    fn string_bytes(&mut self, control: &mut Control<'_>) -> Result<&'a [u8]> {
        let remaining = &self.data[self.pos..];
        let n = remaining
            .iter()
            .take(1_048_577)
            .position(|b| *b == 0)
            .ok_or_else(|| self.error("unterminated_string", "Missing bounded S0 terminator"))?;
        control.string(n)?;
        let bytes = self.take(n)?;
        self.take(1)?;
        if (n + 1) % 2 != 0 && self.take(1)? != [0] {
            return Err(self.error("invalid_strict_padding", "Nonzero S0 padding"));
        }
        Ok(bytes)
    }
    pub fn chunks(&mut self, sub: bool, control: &mut Control<'_>) -> Result<Vec<Chunk<'a>>> {
        let mut chunks = Vec::new();
        while !self.done() {
            control.check(if sub { "subchunk framing" } else { "framing" })?;
            control.record(sub)?;
            let offset = self.base + self.pos;
            if self.data.len() - self.pos < if sub { 6 } else { 8 } {
                return Err(self.error("truncated_chunk_header", "Incomplete chunk header"));
            }
            let id = self.id()?;
            let n = if sub {
                self.u16()? as usize
            } else {
                self.u32()? as usize
            };
            let data = self.take(n)?;
            // A pad byte can only be proven absent at the enclosing boundary;
            // earlier, the next byte is read as the pad and must be zero.
            if n % 2 != 0 && self.done() {
                return Err(self.error(
                    "missing_padding",
                    "Odd payload ends at the enclosing boundary without its pad byte",
                ));
            }
            if n % 2 != 0 && self.take(1)? != [0] {
                return Err(self.error("invalid_strict_padding", "Nonzero chunk padding"));
            }
            control.charge(std::mem::size_of::<Chunk<'_>>() * 2)?;
            chunks.try_reserve(1).map_err(|_| {
                self.error("allocation_failure", "Chunk inventory allocation failed")
            })?;
            chunks.push(Chunk {
                id,
                data,
                offset: offset + if sub { 6 } else { 8 },
            });
        }
        Ok(chunks)
    }
}

/// Windows-1252 text: Latin-1 except 0x80..0x9F, whose five unassigned
/// bytes keep their C1 code points (as web browsers decode it).
pub fn windows_1252(bytes: &[u8]) -> String {
    const HIGH: [char; 32] = [
        '\u{20AC}', '\u{81}', '\u{201A}', '\u{192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{2C6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8D}', '\u{17D}',
        '\u{8F}', '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}',
        '\u{2014}', '\u{2DC}', '\u{2122}', '\u{161}', '\u{203A}', '\u{153}', '\u{9D}', '\u{17E}',
        '\u{178}',
    ];
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9F => HIGH[usize::from(b - 0x80)],
            _ => char::from(b),
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vx_boundaries_and_truncations() {
        for (data, expected) in [
            (&[0xfe, 0xff][..], 65279),
            (&[0xff, 0, 0xff, 0][..], 65280),
            (&[0xff, 0xff, 0xff, 0xff][..], 16777215),
        ] {
            let mut r = Reader::new(data, 0);
            assert_eq!(r.vx().unwrap(), expected);
            assert!(r.done());
        }
        for data in [&[][..], &[0xff], &[0xff, 0], &[0xff, 0, 0], &[0]] {
            assert_eq!(
                Reader::new(data, 0).vx().unwrap_err().category,
                "truncated_index"
            );
        }
        assert_eq!(
            Reader::new(&[0xff, 0, 0, 1], 0).vx().unwrap_err().category,
            "noncanonical_index"
        );
    }
    #[test]
    fn windows_1252_names() {
        assert_eq!(
            super::windows_1252(&[0x80, 0xE9, b'A', 0x81, 0x9F, 0xFF]),
            "\u{20AC}\u{E9}A\u{81}\u{178}\u{FF}"
        );
    }

    #[test]
    fn string_padding_utf8_and_boundaries() {
        let mut progress = |_: &str| true;
        for data in [&b"a"[..], &[0xff, 0], &[0, 127]] {
            assert!(
                Reader::new(data, 0)
                    .string(&mut Control::new(&mut progress))
                    .is_err()
            );
        }
        let bytes = "é水\0".as_bytes();
        let mut r = Reader::new(bytes, 0);
        assert_eq!(r.string(&mut Control::new(&mut progress)).unwrap(), "é水");
        assert!(r.done());
    }
    #[test]
    fn framing_rejects_every_truncated_header_and_nonzero_padding() {
        let mut progress = |_: &str| true;
        let bytes = [b'P', b'R', b'V', b'W', 0, 0, 0, 1, 0, 0];
        for n in 1..bytes.len() {
            assert!(
                Reader::new(&bytes[..n], 12)
                    .chunks(false, &mut Control::new(&mut progress))
                    .is_err()
            );
        }
        assert_eq!(
            Reader::new(&bytes, 12)
                .chunks(false, &mut Control::new(&mut progress))
                .unwrap()
                .len(),
            1
        );
        let mut changed = bytes;
        changed[9] = 127;
        assert!(
            Reader::new(&changed, 12)
                .chunks(false, &mut Control::new(&mut progress))
                .is_err()
        );
        for n in 1..6 {
            assert!(
                Reader::new(&bytes[..n], 0)
                    .chunks(true, &mut Control::new(&mut progress))
                    .is_err()
            );
        }
    }
}
