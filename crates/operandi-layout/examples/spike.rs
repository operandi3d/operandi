//! P06 workspace feasibility check: run with
//! `cargo run -p operandi-layout --example spike`.
//!
//! Shows the operations the architecture asks egui to prove before the app
//! adopts them: splits and nested tabs (drag a tab onto another pane's edge
//! or tab bar), tearing a pane off into its own OS window (right-click its
//! tab > Tear Off), docking it back (Dock Back, or close the window), a
//! second viewport with its own camera (+ Viewport), and the layout coming
//! back after a restart (Save Layout, then quit and run again).
//!
//! The "document" is a counter with an undo history, standing in for the
//! scene: it must survive every layout change. Each viewport's camera is an
//! angle you change by dragging in it; each keeps its own.
use std::collections::BTreeMap;

use eframe::egui;
use egui_tiles::{Behavior, TileId, Tiles, UiResponse};
use operandi_layout::{Layout, Pane};

/// Stands in for the scene and its history.
#[derive(Default)]
struct Document {
    value: i32,
    history: Vec<i32>,
}

struct Spike {
    layout: Layout,
    document: Document,
    /// Each viewport's camera (an orbit angle in degrees), by viewport key.
    cameras: BTreeMap<u32, f32>,
    path: std::path::PathBuf,
    status: String,
}

/// Requests a pane makes while it is drawn; applied after the frame's trees.
#[derive(Default)]
struct Requests {
    tear_off: Option<Pane>,
}

struct Panes<'a> {
    document: &'a mut Document,
    cameras: &'a mut BTreeMap<u32, f32>,
    requests: &'a mut Requests,
}

impl Behavior<Pane> for Panes<'_> {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        pane.title().into()
    }

    fn pane_ui(&mut self, ui: &mut egui::Ui, _tile: TileId, pane: &mut Pane) -> UiResponse {
        match *pane {
            Pane::Viewport(n) => {
                let angle = self.cameras.entry(n).or_insert(0.);
                let (rect, response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
                *angle += response.drag_delta().x * 0.5;
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 0., egui::Color32::from_gray(48));
                // A square seen from the camera's angle.
                let c = rect.center();
                let r = rect.size().min_elem() * 0.25;
                let a = angle.to_radians();
                let corners: Vec<egui::Pos2> = (0..4)
                    .map(|k| {
                        let t = a + k as f32 * std::f32::consts::FRAC_PI_2;
                        c + egui::vec2(t.cos() * r, t.sin() * r * 0.5)
                    })
                    .collect();
                painter.add(egui::Shape::closed_line(
                    corners,
                    egui::Stroke::new(2_f32, egui::Color32::LIGHT_GRAY),
                ));
                painter.text(
                    rect.left_top() + egui::vec2(8., 8.),
                    egui::Align2::LEFT_TOP,
                    format!(
                        "{} · camera {:.0}° (drag to turn) · document {}",
                        pane.title(),
                        angle,
                        self.document.value
                    ),
                    egui::FontId::proportional(13.),
                    egui::Color32::WHITE,
                );
            }
            Pane::Toolbox => {
                ui.heading("Toolbox");
                if ui.button("Add 1 to the document").clicked() {
                    self.document.history.push(self.document.value);
                    self.document.value += 1;
                }
                if ui
                    .add_enabled(!self.document.history.is_empty(), egui::Button::new("Undo"))
                    .clicked()
                {
                    self.document.value = self.document.history.pop().unwrap_or(0);
                }
            }
            Pane::ToolTabs => {
                ui.heading(pane.title());
            }
            Pane::Items => {
                ui.heading("Items");
                ui.label(format!("Document value: {}", self.document.value));
            }
            Pane::Properties | Pane::ToolProperties => {
                ui.heading(pane.title());
                ui.label(format!("Undo steps: {}", self.document.history.len()));
            }
            Pane::Uv(_) | Pane::Missing(_) | Pane::DockTab(_) => {
                ui.heading("UV");
            }
        }
        UiResponse::None
    }

    fn on_tab_button(
        &mut self,
        tiles: &Tiles<Pane>,
        tile_id: TileId,
        response: egui::Response,
    ) -> egui::Response {
        if let Some(&pane) = tiles.get_pane(&tile_id) {
            response.context_menu(|ui| {
                if ui.button("Tear Off").clicked() {
                    self.requests.tear_off = Some(pane);
                    ui.close();
                }
            });
        }
        response
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
}

impl eframe::App for Spike {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut requests = Requests::default();
        let mut dock = None;
        egui::TopBottomPanel::top("spike-bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("+ Viewport").clicked() {
                    let pane = self.layout.add_viewport();
                    self.status = format!("Added {}", pane.title());
                }
                if ui.button("Save Layout").clicked() {
                    self.status = match self
                        .layout
                        .to_json()
                        .and_then(|t| std::fs::write(&self.path, t).map_err(|e| e.to_string()))
                    {
                        Ok(()) => format!("Saved to {}", self.path.display()),
                        Err(e) => format!("Couldn't save: {e}"),
                    };
                }
                if ui.button("Reset Layout").clicked() {
                    self.layout = Layout::default();
                    self.status = "Default layout".into();
                }
                ui.label(&self.status);
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut panes = Panes {
                document: &mut self.document,
                cameras: &mut self.cameras,
                requests: &mut requests,
            };
            self.layout.main.ui(&mut panes, ui);
        });
        for window in &mut self.layout.windows {
            let id = window.id;
            let title = operandi_layout::panes_title(&window.tree);
            let mut panes = Panes {
                document: &mut self.document,
                cameras: &mut self.cameras,
                requests: &mut requests,
            };
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of(("operandi-window", id)),
                egui::ViewportBuilder::default()
                    .with_title(title)
                    .with_inner_size(window.size),
                |ctx, _class| {
                    egui::TopBottomPanel::top("window-bar").show(ctx, |ui| {
                        if ui.button("Dock Back").clicked() {
                            dock = Some(id);
                        }
                    });
                    egui::CentralPanel::default().show(ctx, |ui| {
                        window.tree.ui(&mut panes, ui);
                    });
                    if let Some(size) = ctx.input(|i| i.viewport().inner_rect).map(|r| r.size()) {
                        window.size = [size.x, size.y];
                    }
                    if ctx.input(|i| i.viewport().close_requested()) {
                        dock = Some(id);
                    }
                },
            );
        }
        if let Some(pane) = requests.tear_off {
            self.status = match self.layout.tear_off(pane, [420., 320.]) {
                Ok(_) => format!("{} torn off", pane.title()),
                Err(e) => e,
            };
        }
        if let Some(id) = dock {
            let _ = self.layout.dock(id);
            self.status = "Docked back".into();
        }
    }
}

fn main() -> eframe::Result {
    let path = std::env::temp_dir().join("operandi-layout-spike.json");
    let (layout, status) = match std::fs::read_to_string(&path) {
        Ok(text) => match Layout::from_json(&text) {
            Ok(layout) => (layout, "Restored the saved layout".to_owned()),
            Err(e) => (Layout::default(), format!("Saved layout refused: {e}")),
        },
        Err(_) => (Layout::default(), "Default layout".to_owned()),
    };
    let cameras = layout.viewports().into_iter().map(|v| (v, 0.)).collect();
    eframe::run_native(
        "Operandi layout check",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1100., 700.]),
            ..Default::default()
        },
        Box::new(move |_| {
            Ok(Box::new(Spike {
                layout,
                document: Document::default(),
                cameras,
                path,
                status,
            }))
        }),
    )
}
