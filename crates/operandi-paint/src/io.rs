//! Reading and writing paintable images (PNT-30..35): PNG, TGA, TIFF,
//! JPEG and OpenEXR through the `image` crate. PSD and PSB are `psd.rs`.
//!
//! Files load at their own depth (8-bit, 16-bit or float) and save at the
//! image's depth where the format allows: PNG and TIFF keep 8 or 16 bits
//! (floats are written as 16-bit), OpenEXR is always float, TGA and JPEG
//! are 8-bit. Saving writes a temporary file beside the target and renames
//! it over, so a failed save never leaves half a file.
use crate::image::{Depth, Format, Layout, PaintImage, Pixels, Result};
use image::{DynamicImage, ImageBuffer, ImageFormat};
use std::path::Path;

/// The file types Save Image offers, in the save panel's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileType {
    Png,
    Exr,
    Tga,
    Tiff,
    Jpeg,
    Psd,
    Psb,
}

impl FileType {
    pub const ALL: [FileType; 7] = [
        FileType::Png,
        FileType::Exr,
        FileType::Tga,
        FileType::Tiff,
        FileType::Jpeg,
        FileType::Psd,
        FileType::Psb,
    ];
    pub fn extension(self) -> &'static str {
        match self {
            FileType::Png => "png",
            FileType::Exr => "exr",
            FileType::Tga => "tga",
            FileType::Tiff => "tif",
            FileType::Jpeg => "jpg",
            FileType::Psd => "psd",
            FileType::Psb => "psb",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            FileType::Png => "PNG",
            FileType::Exr => "OpenEXR",
            FileType::Tga => "Targa",
            FileType::Tiff => "TIFF",
            FileType::Jpeg => "JPEG",
            FileType::Psd => "Photoshop PSD",
            FileType::Psb => "Photoshop PSB (large document)",
        }
    }
    pub fn of(path: &Path) -> Option<FileType> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            "png" => FileType::Png,
            "exr" => FileType::Exr,
            "tga" => FileType::Tga,
            "tif" | "tiff" => FileType::Tiff,
            "jpg" | "jpeg" => FileType::Jpeg,
            "psd" => FileType::Psd,
            "psb" => FileType::Psb,
            _ => return None,
        })
    }
    /// A new image's first-save type (PNT-04): OpenEXR for float images,
    /// PNG otherwise.
    pub fn default_for(depth: Depth) -> FileType {
        if depth.is_float() {
            FileType::Exr
        } else {
            FileType::Png
        }
    }
    pub fn is_psd(self) -> bool {
        matches!(self, FileType::Psd | FileType::Psb)
    }
}

/// Reads a flat image file (not PSD/PSB, see `psd::read`).
pub fn load(path: &Path) -> Result<PaintImage> {
    if FileType::of(path).is_some_and(FileType::is_psd) {
        return Err("Photoshop files load through their layers (psd::read)".into());
    }
    let decoded = open_picture(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    Ok(from_dynamic(decoded))
}

/// The most memory one decoded picture may take (as the app's
/// `image_decode::MOST_BYTES`): the `image` crate's own default, 512 MiB,
/// refused 16K textures and 8K float EXRs (hidden-limits sweep A3).
pub const MOST_PICTURE_BYTES: u64 = 16 << 30;

/// `image::open` with the limit above.
pub fn open_picture(path: &Path) -> image::ImageResult<DynamicImage> {
    let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = image::Limits::no_limits();
    limits.max_alloc = Some(MOST_PICTURE_BYTES);
    reader.limits(limits);
    reader.decode()
}

fn from_dynamic(d: DynamicImage) -> PaintImage {
    let (width, height) = (d.width(), d.height());
    let image = |format, depth, pixels| PaintImage {
        width,
        height,
        format,
        depth,
        pixels,
    };
    match d {
        DynamicImage::ImageLuma8(b) => image(Format::Gray, Depth::U8, Pixels::U8(b.into_raw())),
        DynamicImage::ImageRgb8(b) => image(Format::Rgb, Depth::U8, Pixels::U8(b.into_raw())),
        DynamicImage::ImageRgba8(b) => image(Format::Rgba, Depth::U8, Pixels::U8(b.into_raw())),
        DynamicImage::ImageLuma16(b) => image(Format::Gray, Depth::U16, Pixels::U16(b.into_raw())),
        DynamicImage::ImageRgb16(b) => image(Format::Rgb, Depth::U16, Pixels::U16(b.into_raw())),
        DynamicImage::ImageRgba16(b) => image(Format::Rgba, Depth::U16, Pixels::U16(b.into_raw())),
        DynamicImage::ImageRgb32F(b) => image(Format::Rgb, Depth::F32, Pixels::F32(b.into_raw())),
        DynamicImage::ImageRgba32F(b) => image(Format::Rgba, Depth::F32, Pixels::F32(b.into_raw())),
        // Grey with alpha has no format of its own here: RGBA keeps both.
        DynamicImage::ImageLumaA16(_) => {
            let b = d.into_rgba16();
            image(Format::Rgba, Depth::U16, Pixels::U16(b.into_raw()))
        }
        other => {
            let b = other.into_rgba8();
            image(Format::Rgba, Depth::U8, Pixels::U8(b.into_raw()))
        }
    }
}

/// The image converted to `depth` and `format`, sample by sample.
pub fn converted(image: &PaintImage, format: Format, depth: Depth) -> PaintImage {
    if image.format == format && image.depth == depth {
        return image.clone();
    }
    let mut out = PaintImage::new(Layout {
        format,
        depth,
        ..image.layout()
    })
    .expect("same size as an existing image");
    for i in 0..image.pixel_count() {
        out.set(i, image.get(i));
    }
    out
}

fn to_dynamic(image: &PaintImage, file: FileType) -> Result<DynamicImage> {
    let (w, h) = (image.width, image.height);
    let bad = || "The image's pixels don't match its size".to_owned();
    // What the file type can hold, nearest to the image's own storage.
    let (format, depth) = match file {
        FileType::Exr => (
            if image.format.has_alpha() {
                Format::Rgba
            } else {
                Format::Rgb
            },
            Depth::F32,
        ),
        FileType::Png | FileType::Tiff => (
            image.format,
            if image.depth == Depth::U8 {
                Depth::U8
            } else {
                Depth::U16
            },
        ),
        FileType::Jpeg => (
            if image.format == Format::Gray {
                Format::Gray
            } else {
                Format::Rgb
            },
            Depth::U8,
        ),
        FileType::Tga => (image.format, Depth::U8),
        FileType::Psd | FileType::Psb => unreachable!("written by psd.rs"),
    };
    let c = converted(image, format, depth);
    Ok(match (c.format, c.pixels) {
        (Format::Gray, Pixels::U8(v)) => {
            DynamicImage::ImageLuma8(ImageBuffer::from_raw(w, h, v).ok_or_else(bad)?)
        }
        (Format::Rgb, Pixels::U8(v)) => {
            DynamicImage::ImageRgb8(ImageBuffer::from_raw(w, h, v).ok_or_else(bad)?)
        }
        (Format::Rgba, Pixels::U8(v)) => {
            DynamicImage::ImageRgba8(ImageBuffer::from_raw(w, h, v).ok_or_else(bad)?)
        }
        (Format::Gray, Pixels::U16(v)) => {
            DynamicImage::ImageLuma16(ImageBuffer::from_raw(w, h, v).ok_or_else(bad)?)
        }
        (Format::Rgb, Pixels::U16(v)) => {
            DynamicImage::ImageRgb16(ImageBuffer::from_raw(w, h, v).ok_or_else(bad)?)
        }
        (Format::Rgba, Pixels::U16(v)) => {
            DynamicImage::ImageRgba16(ImageBuffer::from_raw(w, h, v).ok_or_else(bad)?)
        }
        (Format::Rgb, Pixels::F32(v)) => {
            DynamicImage::ImageRgb32F(ImageBuffer::from_raw(w, h, v).ok_or_else(bad)?)
        }
        (Format::Rgba, Pixels::F32(v)) => {
            DynamicImage::ImageRgba32F(ImageBuffer::from_raw(w, h, v).ok_or_else(bad)?)
        }
        _ => return Err("That file type can't hold this image".into()),
    })
}

/// Writes `image` to `path` in the type its extension names (flat types
/// only; PSD/PSB go through `psd::write`).
pub fn save(image: &PaintImage, path: &Path) -> Result<()> {
    let file = FileType::of(path).ok_or_else(|| {
        format!(
            "{}: unknown image type; use .png, .exr, .tga, .tif or .jpg",
            path.display()
        )
    })?;
    if file.is_psd() {
        return Err("Photoshop files save through psd::write".into());
    }
    let dynamic = to_dynamic(image, file)?;
    let format = match file {
        FileType::Png => ImageFormat::Png,
        FileType::Exr => ImageFormat::OpenExr,
        FileType::Tga => ImageFormat::Tga,
        FileType::Tiff => ImageFormat::Tiff,
        FileType::Jpeg => ImageFormat::Jpeg,
        FileType::Psd | FileType::Psb => unreachable!(),
    };
    write_atomic(path, |temp| {
        dynamic
            .save_with_format(temp, format)
            .map_err(|e| format!("Cannot write {}: {e}", path.display()))
    })
}

/// Runs `write` on a temporary file beside `path`, then renames it over.
pub fn write_atomic(path: &Path, write: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".into());
    // Keep the extension last: the encoders pick nothing from it, but
    // other tools watching the folder do.
    let temp = path.with_file_name(format!(".{name}.operandi-tmp"));
    if let Err(e) = write(&temp) {
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }
    std::fs::rename(&temp, path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        format!("Cannot write {}: {e}", path.display())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("operandi-paint-io-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn ramp(format: Format, depth: Depth) -> PaintImage {
        let mut i = PaintImage::new(Layout {
            width: 64,
            height: 64,
            format,
            depth,
        })
        .unwrap();
        for p in 0..i.pixel_count() {
            let v = (p % 64) as f32 / 63.;
            i.set(p, [v, 1. - v, 0.5, v]);
        }
        i
    }

    #[test]
    fn png_round_trips_at_8_and_16_bits() {
        for depth in [Depth::U8, Depth::U16] {
            let image = ramp(Format::Rgba, depth);
            let path = temp(&format!("ramp{}.png", depth.bits()));
            save(&image, &path).unwrap();
            assert_eq!(load(&path).unwrap(), image);
        }
    }

    #[test]
    fn float_images_round_trip_through_exr() {
        let image = ramp(Format::Rgba, Depth::F32);
        let path = temp("ramp.exr");
        save(&image, &path).unwrap();
        assert_eq!(load(&path).unwrap(), image);
        assert_eq!(FileType::default_for(Depth::F32), FileType::Exr);
        assert_eq!(FileType::default_for(Depth::U8), FileType::Png);
    }

    #[test]
    fn unknown_types_are_refused_and_leave_no_file() {
        let image = ramp(Format::Rgb, Depth::U8);
        let path = temp("ramp.xyz");
        assert!(save(&image, &path).is_err());
        assert!(!path.exists());
    }
}
