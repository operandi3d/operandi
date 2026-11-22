//! Preferences > Images & Painting and Defaults > Default Tools (PNT-214),
//! painting builds only. Every row is a `pref.value painting.<name>`
//! preference, so rows and commands are one path (UIC-164). Not saved
//! between sessions yet (provisional, PNT-Q06).
use super::*;
use crate::preferences::Page;
use crate::prefs_commands::Pref;
use operandi_core::channels::{ChannelType, ChannelValue};
use operandi_core::paint_tips::TipChoice;

/// Default Foreground Color (PNT-214).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultFg {
    White,
    Black,
}

impl DefaultFg {
    pub(crate) const ALL: [DefaultFg; 2] = [DefaultFg::White, DefaultFg::Black];
    pub(crate) fn label(self) -> &'static str {
        match self {
            DefaultFg::White => "White",
            DefaultFg::Black => "Black",
        }
    }
    pub(crate) fn rgb(self) -> [f64; 3] {
        match self {
            DefaultFg::White => [1.; 3],
            DefaultFg::Black => [0.; 3],
        }
    }
}

/// Default Brush (PNT-41, 214): the tip a paint tool starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultBrush {
    Tip(TipChoice),
    LastSelected,
}

impl DefaultBrush {
    pub(crate) const ALL: [DefaultBrush; 7] = [
        DefaultBrush::Tip(TipChoice::Smooth),
        DefaultBrush::Tip(TipChoice::Hard),
        DefaultBrush::Tip(TipChoice::Procedural),
        DefaultBrush::Tip(TipChoice::Image),
        DefaultBrush::Tip(TipChoice::Sphere),
        DefaultBrush::Tip(TipChoice::Text),
        DefaultBrush::LastSelected,
    ];
    pub(crate) fn label(self) -> &'static str {
        match self {
            // The toolbox calls the Image tip "Texture".
            DefaultBrush::Tip(TipChoice::Image) => "Image",
            DefaultBrush::Tip(t) => t.label(),
            DefaultBrush::LastSelected => "Last Selected",
        }
    }
}

const FG_NAMES: &[&str] = &["White", "Black"];
const BRUSH_NAMES: &[&str] = &[
    "Smooth",
    "Hard",
    "Procedural",
    "Image",
    "Sphere",
    "Text",
    "Last Selected",
];

fn as_bool(v: ChannelValue) -> std::result::Result<bool, String> {
    match v {
        ChannelValue::Bool(b) => Ok(b),
        _ => Err("Expected true or false".into()),
    }
}
fn as_float(v: ChannelValue) -> std::result::Result<f64, String> {
    match v {
        ChannelValue::Float(f) => Ok(f),
        ChannelValue::Int(i) => Ok(i as f64),
        _ => Err("Expected a number".into()),
    }
}
fn as_index(v: ChannelValue) -> std::result::Result<usize, String> {
    match v {
        ChannelValue::Enum(i) => Ok(i),
        _ => Err("Expected a choice".into()),
    }
}

/// Nothing to save yet (PNT-Q06).
fn keep(_: &mut Operandi) {}

pub(crate) static PAINTING_PREFS: &[Pref] = &[
    Pref {
        key: "painting.antialiasing",
        page: Page::ImagesPainting,
        label: "Enable Image Antialiasing by Default",
        ty: ChannelType::Bool,
        get: |a| ChannelValue::Bool(a.painting.prefs.antialiasing),
        set: |a, v| {
            a.painting.prefs.antialiasing = as_bool(v)?;
            Ok(())
        },
        after: keep,
    },
    Pref {
        key: "painting.newImageResolution",
        page: Page::ImagesPainting,
        label: "Default New Image Resolution",
        ty: ChannelType::Float,
        get: |a| ChannelValue::Float(f64::from(a.painting.prefs.new_resolution)),
        set: |a, v| {
            let r = as_float(v)?.round();
            operandi_core::paint::pixels::check_side(r as u32)?;
            a.painting.prefs.new_resolution = r as u32;
            Ok(())
        },
        after: keep,
    },
    Pref {
        key: "painting.imagesAsIcons",
        page: Page::ImagesPainting,
        label: "Always Display Images as Icons",
        ty: ChannelType::Bool,
        get: |a| ChannelValue::Bool(a.painting.prefs.images_as_icons),
        set: |a, v| {
            a.painting.prefs.images_as_icons = as_bool(v)?;
            Ok(())
        },
        after: keep,
    },
    Pref {
        key: "painting.autoCreate",
        page: Page::ImagesPainting,
        label: "Auto Create Textures",
        ty: ChannelType::Bool,
        get: |a| ChannelValue::Bool(a.painting.prefs.auto_create),
        set: |a, v| {
            a.painting.prefs.auto_create = as_bool(v)?;
            Ok(())
        },
        after: keep,
    },
    Pref {
        key: "painting.defaultForeground",
        page: Page::ImagesPainting,
        label: "Default Foreground Color",
        ty: ChannelType::Enum(FG_NAMES),
        get: |a| {
            ChannelValue::Enum(
                DefaultFg::ALL
                    .iter()
                    .position(|&f| f == a.painting.prefs.default_fg)
                    .unwrap_or(0),
            )
        },
        set: |a, v| {
            let fg = *DefaultFg::ALL.get(as_index(v)?).ok_or("White or Black")?;
            a.painting.prefs.default_fg = fg;
            a.painting.brush.set_every_fg(fg.rgb());
            Ok(())
        },
        after: keep,
    },
    Pref {
        key: "painting.maxBrushSize",
        page: Page::ImagesPainting,
        label: "Maximum Brush Size",
        ty: ChannelType::Float,
        get: |a| ChannelValue::Float(a.painting.prefs.max_brush),
        set: |a, v| {
            let m = as_float(v)?;
            if !(1. ..=10_000.).contains(&m) {
                return Err("Maximum Brush Size is 1 to 10000 px".into());
            }
            a.painting.prefs.max_brush = m;
            a.painting.brush.clamp_size(m);
            Ok(())
        },
        after: keep,
    },
    Pref {
        key: "painting.defaultBrush",
        page: Page::DefaultTools,
        label: "Default Brush",
        ty: ChannelType::Enum(BRUSH_NAMES),
        get: |a| {
            ChannelValue::Enum(
                DefaultBrush::ALL
                    .iter()
                    .position(|&b| b == a.painting.prefs.default_brush)
                    .unwrap_or(0),
            )
        },
        set: |a, v| {
            a.painting.prefs.default_brush =
                *DefaultBrush::ALL.get(as_index(v)?).ok_or("No such brush")?;
            Ok(())
        },
        after: keep,
    },
];

impl Operandi {
    pub(super) fn images_painting_page(&mut self, ui: &mut egui::Ui) {
        use crate::form::{Unit, rows};
        let d = crate::painting::PaintingPrefs::default();
        let p = &mut self.painting.prefs;
        rows::section(ui, "Images & Painting", false, |ui| {
            rows::check_d(
                ui,
                "Enable Image Antialiasing by Default",
                &mut p.antialiasing,
                d.antialiasing,
                "New Image Map layers from Add Texture start with Antialiasing on",
            );
            rows::number_d(
                ui,
                "Default New Image Resolution",
                &mut p.new_resolution,
                d.new_resolution,
                Unit::Suffix(" px"),
                operandi_core::paint::pixels::MIN_SIZE..=operandi_core::paint::pixels::MAX_SIZE,
                "New Image's and Add Texture's starting size",
            );
            rows::check_d(
                ui,
                "Always Display Images as Icons",
                &mut p.images_as_icons,
                d.images_as_icons,
                "The Images window shows thumbnails instead of a list",
            );
            rows::check_d(
                ui,
                "Auto Create Textures",
                &mut p.auto_create,
                d.auto_create,
                "Painting a mesh with no texture offers to add one",
            );
            let fgs: Vec<_> = DefaultFg::ALL.iter().map(|&f| (f, f.label())).collect();
            rows::choice_d(
                ui,
                "Default Foreground Color",
                &mut p.default_fg,
                &d.default_fg,
                &fgs,
                "The paint tools' FG Color",
            );
            rows::number_d(
                ui,
                "Maximum Brush Size",
                &mut p.max_brush,
                d.max_brush,
                Unit::Suffix(" px"),
                1. ..=10_000.,
                "The largest a paint tip can be",
            );
        });
    }

    pub(super) fn default_tools_page(&mut self, ui: &mut egui::Ui) {
        let d = crate::painting::PaintingPrefs::default();
        let p = &mut self.painting.prefs;
        crate::form::rows::section(ui, "Default Tools", false, |ui| {
            let brushes: Vec<_> = DefaultBrush::ALL.iter().map(|&b| (b, b.label())).collect();
            crate::form::rows::choice_d(
                ui,
                "Default Brush",
                &mut p.default_brush,
                &d.default_brush,
                &brushes,
                "The tip a paint tool starts with; Last Selected keeps the last one",
            );
        });
    }

    /// Reset (a page's Reset button and Reset Preferences…).
    pub(super) fn reset_painting_prefs(&mut self, page: Page) {
        let d = crate::painting::PaintingPrefs::default();
        let p = &mut self.painting.prefs;
        match page {
            Page::DefaultTools => p.default_brush = d.default_brush,
            _ => {
                let (brush, border, also) = (p.default_brush, p.uv_border, p.also_save_images);
                *p = crate::painting::PaintingPrefs {
                    default_brush: brush,
                    uv_border: border,
                    also_save_images: also,
                    ..d
                };
                self.painting.brush.set_every_fg(d.default_fg.rgb());
            }
        }
    }
}
