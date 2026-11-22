//! Light properties Tier 3 in the app: the Dome light's glyph, the
//! Volumetrics section (LP-40..44, LM-30..33), LPE Label (LP-22),
//! Importance (LP-18), the light material's Enable / Blend Mode / Opacity /
//! Affect Subsurface and image layers (LM-01..04, LM-15), and Physical Sun's
//! Gamma and Store Location (LP-SUN-07, LP-SUN-13).
use super::camera_light_ui::{Mark, ring};
use super::*;
use crate::unavailable::Gate;
use operandi_core::camera_light::{
    LayerEdit, LightBlend, LightEdit, LightKind, LightSettings, VolumetricEdit, Volumetrics,
};
use operandi_core::physical_sun::SunEdit;
use serde::{Deserialize, Serialize};

/// A place the user stored with Store Location (LP-SUN-13), saved in
/// `sun-locations.json` and listed after Operandi's own cities.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct StoredLocation {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub time_zone: f64,
}

/// The user's stored places.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SunLocations {
    pub places: Vec<StoredLocation>,
}

impl SunLocations {
    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)
    }
    /// Add or replace the place called `name`.
    pub fn store(&mut self, place: StoredLocation) {
        match self.places.iter_mut().find(|p| p.name == place.name) {
            Some(p) => *p = place,
            None => self.places.push(place),
        }
    }
}

/// Dome light (LP-DM-01): a wire dome (half sphere) above its centre,
/// Display Size metres across.
pub(super) fn dome_marks(world: glam::DMat4, light: &LightSettings) -> Vec<Mark> {
    let r = light.stored.display_size.max(0.05) * 0.5;
    // `ring` draws in local XY: the equator lies flat (XZ), arcs stand up.
    let flat = world * glam::DMat4::from_rotation_x(std::f64::consts::FRAC_PI_2);
    let mut out = ring(flat, r, r, 0.0, 32, false);
    for turn in [0.0, std::f64::consts::FRAC_PI_2] {
        let m = world * glam::DMat4::from_rotation_y(turn);
        let arc: Vec<DVec3> = (0..=16)
            .map(|k| {
                let a = std::f64::consts::PI * k as f64 / 16.0;
                m.transform_point3(DVec3::new(r * a.cos(), r * a.sin(), 0.0))
            })
            .collect();
        out.extend(arc.windows(2).map(|w| Mark::Line([w[0], w[1]])));
    }
    out
}

impl Operandi {
    fn light_edit(&mut self, id: u64, edit: LightEdit) {
        self.run_light(Command::EditLight { item: id, edit });
    }

    /// LP-18 Importance and LP-22 LPE Label (kinds with the Visible rows).
    pub(super) fn importance_and_lpe_rows(&mut self, ui: &mut Ui, id: u64, l: &LightSettings) {
        self.locator_number(
            ui,
            id,
            "importance",
            "Importance",
            l.stored.importance,
            0.0..=1000.0,
            " %",
            100.,
            |v| Command::EditLight {
                item: id,
                edit: LightEdit::Importance(v),
            },
        );
        ui.weak("Importance is saved only: Cycles has no per-light importance");
        let key = egui::Id::new(("light-lpe", id));
        let mut text: String =
            ui.data_mut(|d| d.get_temp(key).unwrap_or_else(|| l.lpe_label.clone()));
        let r = crate::option_row::row(ui, "LPE Label", |ui| {
            ui.add(TextEdit::singleline(&mut text).desired_width(ui.available_width()))
        })
        .inner
        .on_hover_text("Renders this light's share of the picture as its own saved pass");
        self.text_editors.insert(r.id);
        crate::numeric_tab::register(ui, &r, Some(&text));
        if r.changed() {
            ui.data_mut(|d| d.insert_temp(key, text.clone()));
        }
        if r.lost_focus() {
            ui.data_mut(|d| d.remove::<String>(key));
            if text.trim() != l.lpe_label {
                self.light_edit(id, LightEdit::LpeLabel(text));
            }
        }
    }

    /// LP-40..44 and LM-30..33. Rows grey while Volumetrics is off.
    pub(super) fn volumetric_rows(&mut self, ui: &mut Ui, id: u64, l: &LightSettings) {
        let v = l
            .volumetrics
            .clone()
            .unwrap_or_else(|| Volumetrics::new(l.kind));
        let vol = |e: VolumetricEdit| Command::EditLight {
            item: id,
            edit: LightEdit::Volumetric(e),
        };
        let mut on = v.enabled;
        self.probe_light_row(&vol(VolumetricEdit::Enabled(on)));
        if crate::option_row::check(ui, "Volumetrics", &mut on)
            .on_hover_text("A visible beam of light in the air")
            .changed()
        {
            self.run(vol(VolumetricEdit::Enabled(on)));
        }
        ui.gated_ui(
            crate::unavailable::unless(v.enabled, "Turn on Volumetrics first"),
            |ui| {
                match l.kind {
                    LightKind::Directional | LightKind::Spot => {
                        self.locator_number(
                            ui,
                            id,
                            "vol-height",
                            "Height",
                            v.height,
                            0.001..=1e6,
                            " m",
                            1.,
                            |x| vol(VolumetricEdit::Height(x)),
                        );
                        let label = if l.kind == LightKind::Spot {
                            "Base"
                        } else {
                            "Radius"
                        };
                        self.locator_number(
                            ui,
                            id,
                            "vol-radius",
                            label,
                            v.radius,
                            0.0..=1e6,
                            " m",
                            1.,
                            |x| vol(VolumetricEdit::Radius(x)),
                        );
                    }
                    _ => self.locator_number(
                        ui,
                        id,
                        "vol-sphere",
                        "Sphere Radius",
                        v.sphere_radius,
                        0.0..=1e6,
                        " m",
                        1.,
                        |x| vol(VolumetricEdit::SphereRadius(x)),
                    ),
                }
                ui.horizontal(|ui| {
                    crate::option_row::label(ui, "Scattering Color");
                    crate::option_row::fill(ui, |ui| {
                        let mut rgb = v.color;
                        if crate::colour_field::show(
                            ui,
                            &mut rgb,
                            crate::colour_field::Options::ui(),
                        )
                        .changed()
                        {
                            self.run(vol(VolumetricEdit::Color(rgb)));
                        }
                    })
                });
                self.locator_number(
                    ui,
                    id,
                    "vol-density",
                    "Density",
                    v.density,
                    0.0..=1000.0,
                    " %",
                    100.,
                    |x| vol(VolumetricEdit::Density(x)),
                );
                self.locator_number(
                    ui,
                    id,
                    "vol-attenuation",
                    "Light Attenuation",
                    v.attenuation,
                    0.0..=100.0,
                    " %",
                    100.,
                    |x| vol(VolumetricEdit::Attenuation(x)),
                );
                self.locator_number(
                    ui,
                    id,
                    "vol-shift",
                    "Light Shift",
                    v.shift,
                    -100.0..=100.0,
                    " %",
                    100.,
                    |x| vol(VolumetricEdit::Shift(x)),
                );
                ui.weak("Light Shift is saved, not rendered yet");
            },
        );
    }

    /// LM-01..04, LM-15 and the light material's image layers.
    pub(super) fn light_material_extra_rows(&mut self, ui: &mut Ui, id: u64, l: &LightSettings) {
        let mut enabled = l.stored.material_enabled;
        self.probe_light_row(&Command::EditLight {
            item: id,
            edit: LightEdit::MaterialEnabled(enabled),
        });
        if crate::option_row::check(ui, "Enable", &mut enabled)
            .on_hover_text("Off: the light gives no light")
            .changed()
        {
            self.light_edit(id, LightEdit::MaterialEnabled(enabled));
        }
        let blends = LightBlend::ALL.map(|b| (b, b.label()));
        let mut blend = l.blend_mode;
        crate::option_row::choice(
            ui,
            &format!("light-blend-{id}"),
            "Blend Mode",
            &mut blend,
            &blends,
        )
        .response
        .on_hover_text("With nothing below the light material, Normal and the rest look alike");
        if blend != l.blend_mode {
            self.light_edit(id, LightEdit::BlendMode(blend));
        }
        self.locator_number(
            ui,
            id,
            "material-opacity",
            "Opacity",
            l.stored.opacity,
            0.0..=100.0,
            " %",
            100.,
            |v| Command::EditLight {
                item: id,
                edit: LightEdit::Opacity(v),
            },
        );
        // Direct Caustics spec v1: this light's share of the caustics it
        // throws through glass (the Render item's Direct Caustics).
        self.locator_number(
            ui,
            id,
            "affect-caustics",
            "Affect Caustics",
            l.stored.affect_caustics,
            0.0..=100.0,
            " %",
            100.,
            |v| Command::EditLight {
                item: id,
                edit: LightEdit::AffectCaustics(v),
            },
        );
        self.locator_number(
            ui,
            id,
            "affect-subsurface",
            "Affect Subsurface",
            l.stored.affect_subsurface,
            0.0..=1000.0,
            " %",
            100.,
            |v| Command::EditLight {
                item: id,
                edit: LightEdit::AffectSubsurface(v),
            },
        );
        ui.weak("Affect Subsurface is saved only: Cycles has no switch for it");
        // Image layers (gobos), top of the list = top layer.
        ui.separator();
        ui.weak(if l.kind == LightKind::Spot {
            "Image layers (projected through the cone)"
        } else {
            "Image layers (render on spot lights only)"
        });
        for index in (0..l.layers.len()).rev() {
            let layer = l.layers[index].clone();
            let name = std::path::Path::new(&layer.image)
                .file_name()
                .map_or(layer.image.clone(), |n| n.to_string_lossy().into_owned());
            ui.horizontal(|ui| {
                let mut on = layer.enabled;
                if ui.checkbox(&mut on, "").changed() {
                    self.light_edit(id, LightEdit::Layer(LayerEdit::Enabled(index, on)));
                }
                ui.label(name).on_hover_text(&layer.image);
                if ui.small_button("Remove").clicked() {
                    self.light_edit(id, LightEdit::Layer(LayerEdit::Remove(index)));
                }
            });
            let mut b = layer.blend;
            crate::option_row::choice(
                ui,
                &format!("light-layer-blend-{id}-{index}"),
                "Layer Blend",
                &mut b,
                &blends,
            );
            if b != layer.blend {
                self.light_edit(id, LightEdit::Layer(LayerEdit::Blend(index, b)));
            }
            self.locator_number(
                ui,
                id,
                // One draft per layer row would need a key per index; the
                // panel shows few layers, so the first eight get their own.
                ["l0", "l1", "l2", "l3", "l4", "l5", "l6", "l7"][index.min(7)],
                "Layer Opacity",
                layer.opacity,
                0.0..=100.0,
                " %",
                100.,
                move |v| Command::EditLight {
                    item: id,
                    edit: LightEdit::Layer(LayerEdit::Opacity(index, v)),
                },
            );
        }
        ui.horizontal(|ui| {
            crate::option_row::label(ui, "");
            if ui.button("Add Image Layer...").clicked() {
                let folder = crate::backdrop_ui::dirs_home().unwrap_or_default();
                if !self.choose(
                    crate::file_chooser::Choose::Image { folder },
                    crate::file_chooser::Purpose::LightLayerImage(id),
                ) {
                    self.status = "No file panel is available to choose a picture".into();
                }
            }
        });
    }

    /// LP-SUN-07 Gamma (saved; see LPROP-Q23) and LP-SUN-13 Store Location.
    pub(super) fn sun_extra_rows(&mut self, ui: &mut Ui, id: u64, l: &LightSettings) {
        let sun = l.sun.clone();
        self.locator_number(
            ui,
            id,
            "sun-gamma",
            "Gamma",
            sun.gamma,
            0.1..=10.0,
            "",
            1.,
            |v| Command::EditLight {
                item: id,
                edit: LightEdit::Sun(SunEdit::Gamma(v)),
            },
        );
        // Stored places, picked like the built-in cities.
        let places = self.sun_locations.places.clone();
        if !places.is_empty() {
            let current = places.iter().find(|p| {
                p.latitude == sun.latitude
                    && p.longitude == sun.longitude
                    && p.time_zone == sun.time_zone
            });
            let shown = current.map_or("(choose)", |p| p.name.as_str()).to_string();
            let picked = crate::option_row::dropdown(
                ui,
                &format!("sun-stored-{id}"),
                "Stored Location",
                &shown,
                |ui| {
                    let mut picked = None;
                    for p in &places {
                        if ui
                            .selectable_label(current.is_some_and(|c| c.name == p.name), &p.name)
                            .clicked()
                        {
                            picked = Some(p.clone());
                        }
                    }
                    picked
                },
            )
            .inner;
            if let Some(p) = picked {
                self.light_edit(
                    id,
                    LightEdit::Sun(SunEdit::Place(p.latitude, p.longitude, p.time_zone)),
                );
            }
        }
        let key = egui::Id::new(("sun-store-name", id));
        ui.horizontal(|ui| {
            crate::option_row::label(ui, "");
            ui.menu_button("Store Location...", |ui| {
                let mut name: String = ui.data_mut(|d| d.get_temp(key).unwrap_or_default());
                let r = ui.add(TextEdit::singleline(&mut name).hint_text("Name"));
                self.text_editors.insert(r.id);
                crate::numeric_tab::register(ui, &r, Some(&name));
                ui.data_mut(|d| d.insert_temp(key, name.clone()));
                let ok = !name.trim().is_empty();
                if ui
                    .gated(
                        crate::unavailable::unless(ok, "Type a name first"),
                        Button::new("Store"),
                    )
                    .clicked()
                {
                    self.store_sun_location(StoredLocation {
                        name: name.trim().to_string(),
                        latitude: sun.latitude,
                        longitude: sun.longitude,
                        time_zone: sun.time_zone,
                    });
                    ui.data_mut(|d| d.remove::<String>(key));
                    ui.close();
                }
            })
            .response
            .on_hover_text(
                "Save this latitude, longitude and time zone under a name in your preferences",
            );
        });
    }

    /// LP-SUN-13: save a place in the user's preferences (not an undo step).
    pub(super) fn store_sun_location(&mut self, place: StoredLocation) {
        let name = place.name.clone();
        self.sun_locations.store(place);
        self.status = match self
            .sun_locations
            .save(&self.environment.paths().sun_locations())
        {
            Ok(()) => format!("Stored location {name}"),
            Err(e) => format!("Couldn't save the location: {e}"),
        };
    }

    /// An image chosen for a new light-material layer.
    pub(super) fn light_layer_chosen(&mut self, item: u64, path: std::path::PathBuf) {
        let key = path.to_string_lossy().into_owned();
        if self.run_ok(Command::EditLight {
            item,
            edit: LightEdit::Layer(LayerEdit::Add(key)),
        }) {
            self.status = "Image layer added".into();
        }
    }
}
