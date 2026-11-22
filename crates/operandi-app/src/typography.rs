//! Portable UI boundary: inspect the installed macOS face, then feed its bytes
//! into egui's existing text path. No Apple font is embedded or redistributed.
use eframe::egui::*;
use serde::Serialize;
use std::path::Path;

#[cfg(target_os = "macos")]
pub const STANDARD_SIZE: f32 = 10.;
#[cfg(not(target_os = "macos"))]
pub const STANDARD_SIZE: f32 = 11.;
pub const MACOS_FONT: &str = "/System/Library/Fonts/LucidaGrande.ttc";
const FONT_KEY: &str = "Operandi installed UI Regular";

pub fn standard_font() -> FontId {
    FontId::proportional(STANDARD_SIZE)
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub family: String,
    pub face: String,
    pub postscript_name: String,
    pub source: String,
    pub face_index: Option<u32>,
    pub standard_size: f32,
    pub sfnt_weight: Option<u16>,
    pub fallback: Option<String>,
}
fn fallback(reason: String) -> Report {
    Report {
        family: "Ubuntu".into(),
        face: "Light (egui default)".into(),
        postscript_name: "Ubuntu-Light".into(),
        source: "egui bundled default_fonts".into(),
        face_index: None,
        standard_size: STANDARD_SIZE,
        sfnt_weight: None,
        fallback: Some(reason),
    }
}
fn face_name(face: &ttf_parser::Face<'_>, id: u16) -> Option<String> {
    // Prefer English records. Lucida's family/PostScript names are Macintosh
    // Roman ASCII; ttf-parser's UTF-16 helper deliberately doesn't decode those.
    let decode = |n: ttf_parser::name::Name<'_>| {
        n.to_string().or_else(|| {
            (n.platform_id == ttf_parser::PlatformId::Macintosh
                && n.encoding_id == 0
                && n.name.is_ascii())
            .then(|| String::from_utf8(n.name.to_vec()).ok())
            .flatten()
        })
    };
    face.names()
        .into_iter()
        .filter(|n| {
            n.name_id == id
                && ((n.platform_id == ttf_parser::PlatformId::Macintosh && n.language_id == 0)
                    || (n.platform_id == ttf_parser::PlatformId::Windows && n.language_id == 0x409))
        })
        .find_map(decode)
        .or_else(|| {
            face.names()
                .into_iter()
                .filter(|n| n.name_id == id)
                .find_map(decode)
        })
}
fn load(path: &Path, expected_family: &str) -> Result<(FontData, Report), String> {
    let length = std::fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if length > 16 * 1024 * 1024 {
        return Err("Installed UI font exceeds 16 MiB limit".into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let count = ttf_parser::fonts_in_collection(&bytes).unwrap_or(1);
    for index in 0..count {
        let Ok(face) = ttf_parser::Face::parse(&bytes, index) else {
            continue;
        };
        let family = face_name(&face, ttf_parser::name_id::FAMILY).unwrap_or_default();
        let subfamily = face_name(&face, ttf_parser::name_id::SUBFAMILY).unwrap_or_default();
        if family != expected_family || subfamily != "Regular" || face.is_italic() || face.is_bold()
        {
            continue;
        }
        if !(32u8..=126).all(|c| face.glyph_index(c as char).is_some()) {
            return Err("Installed Regular face lacks standard UI ASCII glyphs".into());
        }
        let report = Report {
            family,
            face: subfamily,
            postscript_name: face_name(&face, ttf_parser::name_id::POST_SCRIPT_NAME)
                .unwrap_or_default(),
            source: path.display().to_string(),
            face_index: Some(index),
            standard_size: STANDARD_SIZE,
            sfnt_weight: Some(face.weight().to_number()),
            fallback: None,
        };
        let mut data = FontData::from_owned(bytes);
        data.index = index;
        return Ok((data, report));
    }
    Err(format!(
        "{}: no verified {expected_family} Regular non-bold face",
        path.display()
    ))
}

pub fn install(ctx: &Context) {
    if cfg!(target_os = "macos") {
        install_from(ctx, Path::new(MACOS_FONT), "Lucida Grande");
    } else {
        set(
            ctx,
            None,
            fallback("macOS font choice is not applied on this platform".into()),
        );
    }
}
// Explicit injection for missing-file tests and same-size, family-only QA.
// Production always calls this with the fixed installed macOS path above.
pub(crate) fn install_from(ctx: &Context, path: &Path, family: &str) {
    match load(path, family) {
        Ok((font, report)) => set(ctx, Some(font), report),
        Err(reason) => {
            crate::log_ring::log_line!(
                "Operandi UI font fallback: {reason}; using egui Ubuntu-Light"
            );
            set(ctx, None, fallback(reason));
        }
    }
}
fn set(ctx: &Context, font: Option<FontData>, report: Report) {
    let mut fonts = FontDefinitions::default();
    if let Some(font) = font {
        fonts.font_data.insert(FONT_KEY.into(), font.into());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, FONT_KEY.into());
    }
    ctx.set_fonts(fonts);
    ctx.data_mut(|d| d.insert_temp(Id::new("ui-font-report"), report));
}
pub fn report(ctx: &Context) -> Report {
    ctx.data(|d| d.get_temp(Id::new("ui-font-report")))
        .unwrap_or_else(|| fallback("Font setup has not run".into()))
}
pub fn diagnostics(ctx: &Context) -> serde_json::Value {
    let advances: Vec<_> = ["Create", "Polygon", "Primitives"].into_iter().map(|text| {
        let galley = ctx.fonts_mut(|f| f.layout_no_wrap(text.into(), standard_font(), Color32::WHITE));
        serde_json::json!({"text":text,"advance":galley.rows[0].glyphs.last().map(|g|g.pos.x+g.advance_width).unwrap_or(0.),"layout_width":galley.size().x,"height":galley.size().y})
    }).collect();
    serde_json::json!({"font":report(ctx),"pixels_per_point":ctx.pixels_per_point(),
        "native_pixels_per_point":ctx.input(|i|i.viewport().native_pixels_per_point),
        "executable":std::env::current_exe().ok(),"advances":advances,
        "numeric_monospace_size":11,"item_numeric_proportional_size":11,"command_proportional_size":11,"shortcut_overlay_size":9,"heading_size":11})
}
