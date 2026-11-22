//! Painting stage 6 (PNT-216): the Images list's details and the paint
//! Image Viewer. Painting builds only.
//!
//! - Each Images row shows the name, an unsaved mark (*), the resolution
//!   and the format ("RGBA 8-bit"); with Always Display Images as Icons on
//!   (PNT-214) the list is a grid of thumbnails.
//! - A double-click on a row opens the image in the Image Viewer
//!   (`operandi.image.view image:<id>`): its own window, showing the
//!   image's live pixels, so it updates while a stroke paints. A Photoshop
//!   image has a PSD Layer picker there (PNT-307): Composite or one layer.
//!   (Provisional, PNT-Q53: not the Render window's viewer.)
use super::*;
use crate::painting::word;
use operandi_core::command_registry::{self as table, Area, ArgType, Unit};
use operandi_core::command_runtime::{Call, CommandState, HostHandler};
use operandi_core::paint::pixels::{self, PaintImage};
use std::sync::Arc;

/// What the viewer shows: an image and, for a Photoshop image, a layer
/// (`None`: the Composite).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ViewerState {
    pub(crate) image: u64,
    pub(crate) layer: Option<usize>,
}

type State = fn(&Operandi, &Call<'_>) -> CommandState;

const VIEW_ARGS: &[table::Arg] = &[
    table::opt("image", ArgType::Text, Unit::None, None),
    table::opt("layer", ArgType::Text, Unit::None, None),
];

pub(crate) fn commands() -> Vec<(table::Entry, HostHandler<Operandi>, State)> {
    vec![(
        table::script(
            "operandi.image.view",
            Area::Tools,
            "Opens an image in the Image Viewer (PNT-216); it updates live while painting. layer: a Photoshop layer's name, or composite (PNT-307).",
        )
        .operandi()
        .args(VIEW_ARGS)
        .example("operandi.image.view image:3"),
        run_view,
        |a, _| CommandState::lit(a.painting.viewer.is_some()),
    )]
}

pub(crate) fn register(r: &mut crate::command_handlers::Commands) {
    for (entry, run, state) in commands() {
        let name = entry.name;
        r.add_host_command(entry, run)
            .unwrap_or_else(|e| panic!("{e}"));
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
}

fn run_view(a: &mut Operandi, c: &Call<'_>) {
    let Some(image) = a.image_from(c.rest) else {
        return;
    };
    if let Err(e) = a.editor.image_pixels(image) {
        a.status = e;
        return;
    }
    let layer = match word(c.rest, "layer", usize::MAX) {
        None => None,
        Some(l) if l.eq_ignore_ascii_case("composite") => None,
        Some(name) => {
            let doc = match a.editor.psd_doc(image) {
                Ok(Some(d)) => d,
                Ok(None) => {
                    a.status = "Only a Photoshop image has layers".into();
                    return;
                }
                Err(e) => {
                    a.status = e;
                    return;
                }
            };
            match doc.layer_named(&name) {
                Some(i) => Some(i),
                None => {
                    a.status = format!("The Photoshop file has no layer {name}");
                    return;
                }
            }
        }
    };
    a.painting.viewer = Some(ViewerState { image, layer });
    a.status = format!("Image Viewer: {}", a.image_name(image));
}

/// "RGBA 8-bit" for a loaded image.
pub(crate) fn format_text(px: &PaintImage) -> String {
    let depth = match px.depth {
        pixels::Depth::U8 => "8-bit",
        pixels::Depth::U16 => "16-bit",
        pixels::Depth::F32 => "32-bit float",
    };
    format!("{} {depth}", px.format.label())
}

/// An egui texture of `px`, made again only when its pixels change (the
/// key holds the picture's address).
pub(crate) fn texture_of(ctx: &Context, salt: &str, px: &Arc<PaintImage>) -> egui::TextureHandle {
    let key = egui::Id::new((salt, Arc::as_ptr(px) as usize));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return t;
    }
    let (w, h) = (px.width as usize, px.height as usize);
    let rgba: Vec<egui::Color32> = (0..w * h)
        .map(|i| {
            let c = px.get(i).map(|v| (v.clamp(0., 1.) * 255.).round() as u8);
            if px.format.has_alpha() {
                egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
            } else {
                egui::Color32::from_rgb(c[0], c[1], c[2])
            }
        })
        .collect();
    let image = egui::ColorImage {
        size: [w, h],
        pixels: rgba,
        source_size: vec2(w as f32, h as f32),
    };
    let t = ctx.load_texture(salt.to_owned(), image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(key, t.clone()));
    t
}

impl Operandi {
    /// The pixels the viewer shows now.
    pub(crate) fn paint_viewer_picture(&mut self) -> Option<Arc<PaintImage>> {
        let v = self.painting.viewer.clone()?;
        let composite = self.editor.scene.paint.buffers.get(&v.image).cloned()?;
        let Some(index) = v.layer else {
            return Some(composite);
        };
        let doc = self.editor.scene.paint.psd.get(&v.image)?.clone();
        // One layer's pixels, cached against the document they came from.
        let key = (v.image, index, Arc::as_ptr(&doc) as usize);
        if let Some((k, px)) = &self.painting.viewer_layer
            && *k == key
        {
            return Some(px.clone());
        }
        let px = Arc::new(
            (*doc)
                .clone()
                .picture(pixels::psd::Pick::Layer(index))
                .ok()?
                .0,
        );
        self.painting.viewer_layer = Some((key, px.clone()));
        Some(px)
    }

    /// The Image Viewer window, once a frame.
    pub(crate) fn paint_viewer_window(&mut self, ctx: &Context) {
        let Some(v) = self.painting.viewer.clone() else {
            return;
        };
        let name = self.image_name(v.image);
        let rows = self.editor.scene.paint.psd.get(&v.image).map(|d| d.rows());
        let picture = self.paint_viewer_picture();
        let mut open = true;
        let mut line = None;
        egui::Window::new(format!("Image Viewer: {name}"))
            .id(egui::Id::new("paint-image-viewer"))
            .open(&mut open)
            .default_size(vec2(420., 460.))
            .show(ctx, |ui| {
                if let Some(rows) = &rows {
                    // PNT-307: the PSD Layer picker.
                    crate::option_row::field_row(ui, "PSD Layer", |ui| {
                        let shown = v
                            .layer
                            .and_then(|i| {
                                rows.iter().find(|r| r.pick == pixels::psd::Pick::Layer(i))
                            })
                            .map_or("Composite".to_owned(), |r| r.name.clone());
                        egui::ComboBox::from_id_salt("paint-viewer-layer")
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                for r in rows {
                                    let on = match r.pick {
                                        pixels::psd::Pick::Composite => v.layer.is_none(),
                                        pixels::psd::Pick::Layer(i) => v.layer == Some(i),
                                    };
                                    let text = format!("{}{}", "  ".repeat(r.indent), r.name);
                                    if ui.selectable_label(on, text).clicked() {
                                        line = Some(match r.pick {
                                            pixels::psd::Pick::Composite => format!(
                                                "operandi.image.view image:{} layer:composite",
                                                v.image
                                            ),
                                            pixels::psd::Pick::Layer(_) => format!(
                                                "operandi.image.view image:{} layer:\"{}\"",
                                                v.image, r.name
                                            ),
                                        });
                                    }
                                }
                            });
                    });
                }
                let Some(px) = &picture else {
                    ui.weak("The image's pixels aren't loaded");
                    return;
                };
                ui.weak(format!(
                    "{} × {} · {}",
                    px.width,
                    px.height,
                    format_text(px)
                ));
                let t = texture_of(ui.ctx(), "paint-viewer", px);
                let room = ui.available_size();
                let aspect = px.width as f32 / px.height.max(1) as f32;
                let size = if room.x / room.y.max(1.) > aspect {
                    vec2(room.y * aspect, room.y)
                } else {
                    vec2(room.x, room.x / aspect)
                };
                ui.image((t.id(), size));
            });
        if !open {
            self.painting.viewer = None;
        }
        if let Some(l) = line {
            self.execute(&l);
        }
    }
}
