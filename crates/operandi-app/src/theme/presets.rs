//! Embedded palettes for Preferences > Display > Colors.
use super::{COLORS, ColorId};
use eframe::egui::Color32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Preset {
    Steel,
    SlateBlue,
    Fog,
    ClassicGrey,
    GradientSky,
    Charcoal,
    Midnight,
    NoneMoreBlack,
}

impl Preset {
    pub(crate) const ALL: [Self; 8] = [
        Self::Steel,
        Self::SlateBlue,
        Self::Fog,
        Self::ClassicGrey,
        Self::GradientSky,
        Self::Charcoal,
        Self::Midnight,
        Self::NoneMoreBlack,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Steel => "Steel",
            Self::SlateBlue => "Slate Blue",
            Self::Fog => "Fog",
            Self::ClassicGrey => "Classic Grey",
            Self::GradientSky => "Gradient Sky",
            Self::Charcoal => "Charcoal",
            Self::Midnight => "Midnight",
            Self::NoneMoreBlack => "None More Black",
        }
    }

    fn color(self, id: ColorId) -> Option<Color32> {
        match self {
            Self::ClassicGrey => (!id.follows_accent_by_default()
                || matches!(id, ColorId::Selection | ColorId::SelectionFill))
            .then_some(id.entry().classic_default),
            Self::Steel => steel_color(id),
            Self::SlateBlue => SLATE_BLUE.color(id),
            Self::Fog => FOG.color(id),
            Self::GradientSky => GRADIENT_SKY.color(id),
            Self::Charcoal => CHARCOAL.color(id),
            Self::Midnight => MIDNIGHT.color(id),
            Self::NoneMoreBlack => NONE_MORE_BLACK.color(id),
        }
    }

    pub(crate) fn colors(self) -> impl Iterator<Item = (ColorId, Color32)> {
        COLORS
            .iter()
            .filter_map(move |entry| self.color(entry.id).map(|color| (entry.id, color)))
    }
}

pub(super) const fn steel_color(id: ColorId) -> Option<Color32> {
    if matches!(id, ColorId::ItemsSelectedText) {
        None
    } else {
        STEEL.color(id)
    }
}

// The 18 roles in the brief, in order. Selected-row text normally has its
// own palette value. Steel leaves it linked to Accent (UIC-01, UIC-33).
struct Palette([u32; 18]);

const STEEL: Palette = Palette([
    0x2E333B, 0x22262C, 0x252A31, 0x3D444F, 0x1B1F24, 0xE9EDF2, 0xAAB3BF, 0x7FB6E6, 0x0E1A26,
    0x333943, 0x373E48, 0x2B4662, 0xFFFFFF, 0x56606D, 0x2C3138, 0x66707C, 0xCDD5DF, 0xFFC24A,
]);
const SLATE_BLUE: Palette = Palette([
    0x2A3140, 0x1E2430, 0x222835, 0x3A4356, 0x181D27, 0xEAEEF6, 0xA9B3C6, 0x8A9EF0, 0x10142A,
    0x2F3747, 0x333C4D, 0x34407A, 0xFFFFFF, 0x525D72, 0x272D39, 0x626D82, 0xCBD3E3, 0xFFC24A,
]);
const FOG: Palette = Palette([
    0x3A4048, 0x2C3137, 0x30353C, 0x4B525C, 0x262A30, 0xEEF1F4, 0xB5BDC7, 0x5B9BD5, 0x0B1724,
    0x3F454E, 0x434A53, 0x38557A, 0xFFFFFF, 0x6A7480, 0x363B42, 0x7A848F, 0xD5DBE2, 0xFFC24A,
]);
const GRADIENT_SKY: Palette = Palette([
    0x444444, 0x373737, 0x2B2B2B, 0x5D5D5D, 0x2A2A2A, 0xDBDBDB, 0xB1B1B1, 0x497C9D, 0xFFFFFF,
    0x2B2B2B, 0x303030, 0x3F6E8C, 0xFFFFFF, 0x8A9BAC, 0x1E2124, 0x6E7680, 0x1A3E8C, 0x43FFA3,
]);
const CHARCOAL: Palette = Palette([
    0x333333, 0x262626, 0x282828, 0x474747, 0x1F1F1F, 0xD4D4D4, 0x9E9E9E, 0x4D8FE8, 0x0B1626,
    0x2D2D2D, 0x313131, 0x3A5F95, 0xFFFFFF, 0x5C5C5C, 0x4A4A4A, 0x6B6B6B, 0x0E0E0E, 0xF7A23B,
]);
const MIDNIGHT: Palette = Palette([
    0x303030, 0x1D1D1D, 0x1D1D1D, 0x545454, 0x242424, 0xE6E6E6, 0xA6A6A6, 0x4772B3, 0xFFFFFF,
    0x282828, 0x2B2B2B, 0x334D80, 0xFFFFFF, 0x3D3D3D, 0x303030, 0x545454, 0x000000, 0xFFA000,
]);
// The selected row uses the muted accent for its label and a darker fill
// than either list stripe (UIC-01, UIC-33).
const NONE_MORE_BLACK: Palette = Palette([
    0x0D0D0D, 0x090909, 0x050505, 0x121212, 0x1A1A1A, 0xD6D6D6, 0xA0A0A0, 0x8AA7A4, 0x090909,
    0x101010, 0x141414, 0x080C0C, 0x8AA7A4, 0x0A0A0A, 0x000000, 0x353535, 0xA6A6A6, 0xFFD470,
]);

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

const fn shade(hex: u32, offset: i8) -> Color32 {
    let color = rgb(hex);
    Color32::from_rgb(
        color.r().saturating_add_signed(offset),
        color.g().saturating_add_signed(offset),
        color.b().saturating_add_signed(offset),
    )
}

impl Palette {
    const fn color(&self, id: ColorId) -> Option<Color32> {
        use ColorId::*;
        let [
            panel,
            tab_strip,
            field,
            control,
            border,
            text,
            weak,
            accent,
            on_accent,
            list,
            stripe,
            selected,
            selected_text,
            viewport_top,
            viewport_bottom,
            grid,
            wire,
            selection,
        ] = self.0;
        Some(match id {
            Panel | Window | ToolboxBackground | ItemsTab => rgb(panel),
            ToolboxHeader => shade(panel, -0x0A),
            TabStrip => rgb(tab_strip),
            Field => rgb(field),
            Control | ToolboxControl => rgb(control),
            Hover | ToolboxHover => shade(control, 0x0A),
            Pressed | ToolboxPressed => shade(control, 0x16),
            Border | ToolboxBorder | ItemsSeparator => rgb(border),
            Text | ToolboxText | ItemsText => rgb(text),
            WeakText | ItemsHeaderText | TabText => rgb(weak),
            Accent | ToolboxActive => rgb(accent),
            OnAccent | ToolboxActiveText => rgb(on_accent),
            ItemsList | ItemsHeader => rgb(list),
            ItemsListStripe => rgb(stripe),
            ItemsSelected => rgb(selected),
            ItemsPrimary => shade(selected, -0x10),
            ItemsSelectedText => rgb(selected_text),
            ViewportGradientTop => rgb(viewport_top),
            ViewportBackground | OrthoBackground => rgb(viewport_bottom),
            Grid | OrthoGrid => rgb(grid),
            Wireframe | OrthoWireframe => rgb(wire),
            Selection | SelectionFill | HandleActive => rgb(selection),
            _ => return None,
        })
    }
}
