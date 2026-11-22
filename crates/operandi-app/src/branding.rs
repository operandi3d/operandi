//! Supplied user-admitted branding; uniform scaling only, no drawn substitutes.
//!
//! The O (rendered in Operandi, 2026-11-10) is Operandi's logo: the toolbar symbol, its hover logo,
//! the About window and the Dock/window icon.
use eframe::egui::*;

/// Dock tile (the O on a black tile): the window icon under cargo run
/// and on Linux/Windows. Inside a macOS .app the bundle's own icon is used.
pub const DOCK_ICON: &[u8] = include_bytes!("../../../assets/branding/operandi-dock-icon.png");
/// The full logo with wordmark (1448 × 1086).
const LOGO: &[u8] = include_bytes!("../../../assets/branding/operandi-logo.png");
/// Uniform 128px (Lanczos) derivative of OP-LOGO-APP-2 (branding-manifest.json).
const SYMBOL: &[u8] = include_bytes!("../assets/operandi-symbol-128.png");

fn texture(ctx: &Context, full: bool) -> Option<TextureHandle> {
    let id = Id::new(if full {
        "operandi-approved-logo"
    } else {
        "operandi-approved-symbol"
    });
    if let Some(texture) = ctx.data_mut(|d| d.get_temp::<TextureHandle>(id)) {
        return Some(texture);
    }
    let image = eframe::icon_data::from_png_bytes(if full { LOGO } else { SYMBOL }).ok()?;
    let image = ColorImage::from_rgba_unmultiplied(
        [image.width as usize, image.height as usize],
        &image.rgba,
    );
    let texture = ctx.load_texture(
        if full {
            "Operandi approved full logo"
        } else {
            "Operandi approved app symbol"
        },
        image,
        TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    Some(texture)
}
/// The full logo at `size`, for the About window (about.rs).
pub fn logo(ui: &mut Ui, size: Vec2) {
    if let Some(logo) = texture(ui.ctx(), true) {
        ui.add(Image::new(&logo).fit_to_exact_size(size));
    }
}
/// The logo (symbol and name) at the top bar's left end. It is the bar's
/// collapse control (UIC-16, Jason 2026-11-18: the logo is the control,
/// no separate button), so it highlights on hover, shows the pointing
/// hand and says "Collapse bar"; the caller runs the collapse on a click.
pub fn identity(ui: &mut Ui) -> Response {
    let name = RichText::new("OPERANDI").strong();
    let galley = WidgetText::from(name).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        TextStyle::Button,
    );
    let symbol = vec2(20., 20.);
    let gap = 4.;
    let pad = vec2(3., 1.);
    let size = vec2(
        pad.x * 2. + symbol.x + gap + galley.size().x,
        pad.y * 2. + symbol.y.max(galley.size().y),
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if response.hovered() || response.has_focus() {
            painter.rect_filled(rect, 3., super::theme::hover());
        }
        let inner = rect.shrink2(pad);
        let symbol_rect =
            Rect::from_min_size(pos2(inner.left(), inner.center().y - symbol.y / 2.), symbol);
        if let Some(texture) = texture(ui.ctx(), false) {
            Image::new(&texture).paint_at(ui, symbol_rect);
        }
        let text_pos = pos2(
            symbol_rect.right() + gap,
            inner.center().y - galley.size().y / 2.,
        );
        painter.galley(text_pos, galley, super::theme::text());
    }
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_ui(|ui| {
            ui.label("Collapse bar");
            if let Some(logo) = texture(ui.ctx(), true) {
                ui.add(Image::new(&logo).fit_to_exact_size(vec2(256., 192.)));
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_assets_decode_at_their_sizes() {
        for (bytes, size) in [
            (DOCK_ICON, (1024, 1024)),
            (LOGO, (1448, 1086)),
            (SYMBOL, (128, 128)),
        ] {
            let image = eframe::icon_data::from_png_bytes(bytes).unwrap();
            assert_eq!((image.width, image.height), size);
        }
    }
}
