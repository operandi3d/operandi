//! One way to open picture files (hidden-limits sweep A3).
//!
//! The `image` crate refuses, by default, any picture that decodes to more
//! than 512 MiB: a 16K RGBA texture, an 8K float EXR or a render bigger
//! than about 5800 x 5800. Those failed quietly (a grey viewport texture,
//! "Cannot read" in painting, a render history entry that couldn't be
//! shown). Here the limit is the memory actually free, up to `MOST_BYTES`,
//! so a picture that fits opens and one that doesn't fails with a clear
//! error instead of taking the machine's memory.
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// The most memory one decoded picture may take: a 32K RGBA8 picture or a
/// 16K RGBA float EXR fits.
pub(crate) const MOST_BYTES: u64 = 16 << 30;

/// The decode limits: no size limit beyond the memory one picture may take.
pub(crate) fn limits(free_bytes: u64) -> image::Limits {
    let mut limits = image::Limits::no_limits();
    limits.max_alloc = Some(MOST_BYTES.min(free_bytes));
    limits
}

/// A reader for `path` with its format guessed and the limits set.
pub(crate) fn reader(path: &Path) -> std::io::Result<image::ImageReader<BufReader<File>>> {
    let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
    reader.limits(limits(crate::free_memory::free_bytes()));
    Ok(reader)
}

/// `image::open` with the limits above.
pub(crate) fn open(path: &Path) -> image::ImageResult<image::DynamicImage> {
    reader(path)?.decode()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_pictures_are_allowed_when_memory_is_free() {
        // An 8K float RGBA EXR is 1 GiB decoded, twice the crate's default.
        let need = 8192u64 * 8192 * 16;
        assert!(limits(64 << 30).max_alloc.unwrap() >= need);
        assert_eq!(
            limits(1 << 30).max_alloc,
            Some(1 << 30),
            "free memory caps it"
        );
        assert_eq!(limits(u64::MAX).max_alloc, Some(MOST_BYTES));
    }

    /// Opt-in (about 1 GB of memory and a few seconds): a 16K RGBA PNG,
    /// which the crate's default limit refused, opens.
    /// `cargo test --locked -p operandi-app -- image_decode:: --ignored` (about 30 s)
    #[test]
    #[ignore]
    fn a_16k_rgba_png_opens() {
        let dir = std::env::temp_dir().join(format!("operandi-decode-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("16k.png");
        let side = 16384;
        image::RgbaImage::from_pixel(side, side, image::Rgba([200, 100, 50, 255]))
            .save(&path)
            .unwrap();
        assert!(image::open(&path).is_err(), "the default limit refuses it");
        let picture = open(&path).unwrap();
        assert_eq!(picture.width(), side);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
