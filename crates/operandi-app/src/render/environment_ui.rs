//! Environments in the app (specifications: environments-2026-11-09,
//! "ENV"): their rows in the Shader Tree's Environments section and their
//! Properties (ENV §7). Settings, defaults and undo live in core
//! (operandi_core::environment); every change runs
//! `Command::EditEnvironment`, one undo step named after its row.
//!
//! [chosen] choices (spec silent):
//! - Rows that don't apply to the Environment Type grey out: Constant uses
//!   Zenith only; 2 Color and Overcast Zenith and Nadir; the exponents only
//!   4 Color; Physically-based Daylight greys the gradient rows and is the
//!   only type with the daylight group live (ENV §3).
//! - Fog rows grey by Fog Type: Start and End for Linear, Density for the
//!   others, Base Altitude and Altitude Falloff for Layered.
use super::*;
use crate::unavailable::Gate;
use operandi_core::environment::{
    Environment, EnvironmentEdit, EnvironmentFogType, EnvironmentMaterial, EnvironmentType,
    GradientColor, RayKind,
};
use operandi_core::shader_tree::BlendMode;

/// The Properties tab of an Environment item and of its material.
pub(super) const ENVIRONMENT_TAB: &str = "Environment";
pub(super) const ENVIRONMENT_MATERIAL_TAB: &str = "Environment Material";
pub(super) const ENVIRONMENT_LAYER_TAB: &str = "Environment Layer";
/// The Environment tab's sections under Intensity (HDRI stage 1).
pub(super) const ENVIRONMENT_IMAGE_SECTION: &str = "Environment Image";
pub(super) const ENVIRONMENT_VISIBILITY_SECTION: &str = "Environment Visibility";

/// The `item.channel` line a Properties row of an environment runs
/// (UIC-160/165): (tab, row label) to channel name. The Name rows run
/// `item.name`, the Sun Light row `environment.light`.
#[cfg(test)]
pub(super) fn row_channel(tab: &str, label: &str) -> Option<&'static str> {
    Some(match (tab, label) {
        (ENVIRONMENT_TAB, "Intensity") => "intensity",
        (ENVIRONMENT_TAB, "Visible to Camera") => "visCamera",
        (ENVIRONMENT_TAB, "Visible to Indirect Rays") => "visIndirect",
        (ENVIRONMENT_TAB, "Visible to Reflection Rays") => "visReflection",
        (ENVIRONMENT_TAB, "Visible to Refraction Rays") => "visRefraction",
        (ENVIRONMENT_TAB, "Light Path Expression Label") => "lpeLabel",
        (ENVIRONMENT_MATERIAL_TAB | ENVIRONMENT_LAYER_TAB, "Enable") => "enable",
        (ENVIRONMENT_MATERIAL_TAB | ENVIRONMENT_LAYER_TAB, "Invert") => "invert",
        (ENVIRONMENT_MATERIAL_TAB | ENVIRONMENT_LAYER_TAB, "Blend Mode") => "blend",
        (ENVIRONMENT_MATERIAL_TAB | ENVIRONMENT_LAYER_TAB, "Opacity") => "opacity",
        (ENVIRONMENT_MATERIAL_TAB, "Environment Type") => "type",
        (ENVIRONMENT_MATERIAL_TAB, "Zenith Color") => "zenColor",
        (ENVIRONMENT_MATERIAL_TAB, "Sky Color") => "skyColor",
        (ENVIRONMENT_MATERIAL_TAB, "Ground Color") => "gndColor",
        (ENVIRONMENT_MATERIAL_TAB, "Nadir Color") => "nadColor",
        (ENVIRONMENT_MATERIAL_TAB, "Sky Exponent") => "skyExp",
        (ENVIRONMENT_MATERIAL_TAB, "Ground Exponent") => "gndExp",
        (ENVIRONMENT_MATERIAL_TAB, "Solar Disc Size") => "discSize",
        (ENVIRONMENT_MATERIAL_TAB, "Haze Amount") => "haze",
        (ENVIRONMENT_MATERIAL_TAB, "Clamp Sky Brightness") => "clampSky",
        (ENVIRONMENT_MATERIAL_TAB, "Sky Gamma") => "skyGamma",
        (ENVIRONMENT_MATERIAL_TAB, "Ground Albedo") => "albedo",
        (ENVIRONMENT_MATERIAL_TAB, "Disc Color") => "discColor",
        (ENVIRONMENT_MATERIAL_TAB, "Disc In-Scatter") => "discInScatter",
        (ENVIRONMENT_MATERIAL_TAB, "Fog Type") => "fogType",
        (ENVIRONMENT_MATERIAL_TAB, "Fog Start") => "fogStart",
        (ENVIRONMENT_MATERIAL_TAB, "Fog End") => "fogEnd",
        (ENVIRONMENT_MATERIAL_TAB, "Fog Density") => "fogDensity",
        (ENVIRONMENT_MATERIAL_TAB, "Base Altitude") => "fogAltitude",
        (ENVIRONMENT_MATERIAL_TAB, "Altitude Falloff") => "fogFalloff",
        (ENVIRONMENT_LAYER_TAB, "Effect") => "effect",
        (ENVIRONMENT_LAYER_TAB, "Color") => "color",
        (ENVIRONMENT_LAYER_TAB, "Value") => "value",
        (ENVIRONMENT_LAYER_TAB, "Image") => "image",
        _ => return None,
    })
}

/// Which environment row the Shader Tree has selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EnvironmentRow {
    Item(u64),
    Material(u64),
    /// A texture layer under it (item id, layer id).
    Layer(u64, operandi_core::shader_tree::NodeId),
}

impl EnvironmentRow {
    pub(super) fn item(self) -> u64 {
        match self {
            EnvironmentRow::Item(id) | EnvironmentRow::Material(id) => id,
            EnvironmentRow::Layer(id, _) => id,
        }
    }
}

impl Operandi {
    pub(super) fn environment(&self, id: u64) -> Option<&Environment> {
        self.editor
            .scene
            .item(id)
            .and_then(|i| i.environment.as_deref())
    }

    fn env_edit(&mut self, item: u64, edit: EnvironmentEdit) {
        self.run(Command::EditEnvironment { item, edit });
    }

    /// A number row: typed or dragged, written when the drag or typing
    /// ends (one undo step). `scale` converts stored to shown (100 for %).
    #[allow(clippy::too_many_arguments)]
    fn env_number(
        &mut self,
        ui: &mut Ui,
        item: u64,
        key: &'static str,
        label: &str,
        value: f64,
        default: f64,
        range: std::ops::RangeInclusive<f64>,
        suffix: &str,
        scale: f64,
        make: impl Fn(f64) -> EnvironmentEdit,
    ) -> Response {
        let mut shown = match self.locator_draft {
            Some((i, k, v)) if i == item && k == key => v,
            _ => value * scale,
        };
        crate::form::rows::status((value - default).abs() > 1e-12);
        let response = ui
            .horizontal(|ui| {
                crate::option_row::label(ui, label);
                crate::option_row::fill(ui, |ui| {
                    ui.add(
                        crate::numeric_tab::DragValue::new(&mut shown)
                            .range(range)
                            .speed(0.01 * scale.max(1.0))
                            .max_decimals(3)
                            .suffix(suffix),
                    )
                })
            })
            .inner;
        if response.changed() {
            self.locator_draft = Some((item, key, shown));
        }
        let finished = response.drag_stopped()
            || response.lost_focus()
            || (response.changed() && !response.dragged() && !response.has_focus());
        if finished
            && let Some((i, k, v)) = self.locator_draft
            && i == item
            && k == key
        {
            self.locator_draft = None;
            if (v / scale - value).abs() > 1e-12 {
                self.env_edit(item, make(v / scale));
            }
        }
        response
    }

    fn env_check(
        &mut self,
        ui: &mut Ui,
        item: u64,
        label: &str,
        value: bool,
        default: bool,
        make: impl Fn(bool) -> EnvironmentEdit,
    ) -> Response {
        let mut on = value;
        crate::form::rows::status(value != default);
        let r = crate::option_row::check(ui, label, &mut on);
        if r.changed() {
            self.env_edit(item, make(on));
        }
        r
    }

    fn env_color(
        &mut self,
        ui: &mut Ui,
        item: u64,
        label: &str,
        value: [f64; 3],
        default: [f64; 3],
        make: impl Fn([f64; 3]) -> EnvironmentEdit,
    ) {
        crate::form::rows::status(value != default);
        ui.horizontal(|ui| {
            crate::option_row::label(ui, label);
            crate::option_row::fill(ui, |ui| {
                let mut rgb = value;
                let field =
                    crate::colour_field::show(ui, &mut rgb, crate::colour_field::Options::ui());
                if field.changed() && rgb != value {
                    self.env_edit(item, make(rgb));
                }
            })
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn env_choice<T: Copy + PartialEq>(
        &mut self,
        ui: &mut Ui,
        item: u64,
        label: &str,
        value: T,
        default: T,
        options: &[(T, &str)],
        make: impl Fn(T) -> EnvironmentEdit,
    ) -> Response {
        let mut v = value;
        crate::form::rows::status(value != default);
        let r =
            crate::option_row::choice(ui, &format!("env-{label}-{item}"), label, &mut v, options);
        if r.inner && v != value {
            self.env_edit(item, make(v));
        }
        r.response
    }

    /// ENV §7: Intensity, the four Visible to … switches, then the LPE
    /// label.
    pub(super) fn environment_item_rows(&mut self, ui: &mut Ui, id: u64) {
        let Some(env) = self.environment(id).cloned() else {
            ui.weak("Select an environment");
            return;
        };
        let idle = self.editor.session.live.is_none();
        crate::form::rows::section(ui, ENVIRONMENT_TAB, false, |ui| {
            ui.gated_ui(
                crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.;
                    self.env_number(
                        ui,
                        id,
                        "env-intensity",
                        "Intensity",
                        env.intensity,
                        1.0,
                        0.0..=1e6,
                        "",
                        1.0,
                        EnvironmentEdit::Intensity,
                    )
                    .on_hover_text("Multiplies the environment's colour");
                },
            );
        });
        crate::form::rows::section(ui, ENVIRONMENT_IMAGE_SECTION, false, |ui| {
            ui.gated_ui(
                crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.;
                    self.environment_image_rows(ui, id, None);
                },
            );
        });
        crate::form::rows::section(ui, ENVIRONMENT_VISIBILITY_SECTION, false, |ui| {
            ui.gated_ui(
                crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.;
                    for (kind, label, tip) in [
                        (RayKind::Camera, "Visible to Camera", "Seen behind objects"),
                        (
                            RayKind::Indirect,
                            "Visible to Indirect Rays",
                            "Lights the scene (GI)",
                        ),
                        (
                            RayKind::Reflection,
                            "Visible to Reflection Rays",
                            "Seen in reflections",
                        ),
                        (
                            RayKind::Refraction,
                            "Visible to Refraction Rays",
                            "Seen through glass",
                        ),
                    ] {
                        self.env_check(ui, id, label, env.visible_to(kind), true, move |on| {
                            EnvironmentEdit::Visible(kind, on)
                        })
                        .on_hover_text(format!(
                            "{tip}. With several environments each ray sees the newest one \
                         visible to it"
                        ));
                    }
                    let key = Id::new(("env-lpe", id));
                    let mut text: String =
                        ui.data_mut(|d| d.get_temp(key).unwrap_or_else(|| env.lpe_label.clone()));
                    crate::form::rows::status(!env.lpe_label.is_empty());
                    let r = crate::option_row::row(ui, "Light Path Expression Label", |ui| {
                        ui.add(TextEdit::singleline(&mut text).desired_width(ui.available_width()))
                    })
                    .inner
                    .on_hover_text("Saved with the scene; renders don't use it yet");
                    self.text_editors.insert(r.id);
                    crate::numeric_tab::register(ui, &r, Some(&text));
                    if r.changed() {
                        ui.data_mut(|d| d.insert_temp(key, text.clone()));
                    }
                    if r.lost_focus() {
                        ui.data_mut(|d| d.remove::<String>(key));
                        if crate::numeric_tab::accepted(&r) && text.trim() != env.lpe_label {
                            self.env_edit(id, EnvironmentEdit::LpeLabel(text.trim().to_owned()));
                        }
                    }
                },
            );
        });
    }

    /// ENV §7: the layer block, Environment Type, the colours and
    /// exponents, then the Physically-based Daylight and Environment Fog
    /// groups (collapsed).
    pub(super) fn environment_material_rows(&mut self, ui: &mut Ui, id: u64) {
        let Some(env) = self.environment(id).cloned() else {
            ui.weak("Select an environment");
            return;
        };
        let m = (*env.material.settings).clone();
        let d = EnvironmentMaterial::default();
        let idle = self.editor.session.live.is_none();
        ui.strong(ENVIRONMENT_MATERIAL_TAB);
        self.env_material_name_row(ui, id, &env.material.name);
        crate::form::rows::section(ui, "Layer", false, |ui| {
            ui.gated_ui(
                crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.;
                    let layer = &env.material.layer;
                    self.env_check(
                        ui,
                        id,
                        "Enable",
                        layer.enable,
                        true,
                        EnvironmentEdit::Enable,
                    );
                    self.env_check(
                        ui,
                        id,
                        "Invert",
                        layer.invert,
                        false,
                        EnvironmentEdit::Invert,
                    );
                    let blends: Vec<(BlendMode, &str)> =
                        BlendMode::ALL.iter().map(|&b| (b, b.label())).collect();
                    self.env_choice(
                        ui,
                        id,
                        "Blend Mode",
                        layer.blend,
                        BlendMode::Normal,
                        &blends,
                        EnvironmentEdit::Blend,
                    );
                    self.env_number(
                        ui,
                        id,
                        "env-opacity",
                        "Opacity",
                        layer.opacity,
                        1.0,
                        0.0..=100.0,
                        " %",
                        100.0,
                        EnvironmentEdit::Opacity,
                    );
                    ui.weak("Effect: Environment Color").on_hover_text(
                        "An Environment Material always sets the environment colour",
                    );
                },
            );
        });
        crate::form::rows::section(ui, "Environment Material", false, |ui| {
            ui.gated_ui(
                crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.;
                    let kinds: Vec<(EnvironmentType, &str)> = EnvironmentType::ALL
                        .iter()
                        .map(|&k| (k, k.label()))
                        .collect();
                    self.env_choice(
                        ui,
                        id,
                        "Environment Type",
                        m.kind,
                        d.kind,
                        &kinds,
                        EnvironmentEdit::Kind,
                    );
                    use EnvironmentType as T;
                    let uses = |c: GradientColor| match m.kind {
                        T::Constant => c == GradientColor::Zenith,
                        T::TwoColor | T::Overcast => {
                            matches!(c, GradientColor::Zenith | GradientColor::Nadir)
                        }
                        T::FourColor => true,
                        T::Physical => false,
                    };
                    for (c, label, value, default) in [
                        (GradientColor::Zenith, "Zenith Color", m.zenith, d.zenith),
                        (GradientColor::Sky, "Sky Color", m.sky, d.sky),
                        (GradientColor::Ground, "Ground Color", m.ground, d.ground),
                        (GradientColor::Nadir, "Nadir Color", m.nadir, d.nadir),
                    ] {
                        ui.gated_ui(
                            crate::unavailable::unless(
                                uses(c),
                                "Not used by this Environment Type",
                            ),
                            |ui| {
                                self.env_color(ui, id, label, value, default, move |rgb| {
                                    EnvironmentEdit::Color(c, rgb)
                                });
                            },
                        );
                    }
                    ui.gated_ui(
                        crate::unavailable::unless(
                            m.kind == T::FourColor,
                            "Set Environment Type to 4 Color Gradient first",
                        ),
                        |ui| {
                            self.env_number(
                                ui,
                                id,
                                "env-sky-exp",
                                "Sky Exponent",
                                m.sky_exponent,
                                d.sky_exponent,
                                0.0..=1e3,
                                "",
                                1.0,
                                EnvironmentEdit::SkyExponent,
                            )
                            .on_hover_text("Shapes the blend from zenith to sky");
                            self.env_number(
                                ui,
                                id,
                                "env-gnd-exp",
                                "Ground Exponent",
                                m.ground_exponent,
                                d.ground_exponent,
                                0.0..=1e3,
                                "",
                                1.0,
                                EnvironmentEdit::GroundExponent,
                            )
                            .on_hover_text("Shapes the blend from nadir to ground");
                        },
                    );
                },
            );
        });
        crate::form::rows::section(ui, "Physically-based Daylight", true, |ui| {
            ui.gated_ui(
                crate::unavailable::first([
                    crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                    crate::unavailable::unless(
                        m.kind == EnvironmentType::Physical,
                        "Set Environment Type to Physically-based Daylight first",
                    ),
                ]),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.;
                    self.env_daylight_rows(ui, id, &m, &d);
                },
            );
        });
        crate::form::rows::section(ui, "Environment Fog", true, |ui| {
            ui.gated_ui(
                crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.;
                    self.env_fog_rows(ui, id, &m, &d);
                },
            );
        });
    }

    fn env_material_name_row(&mut self, ui: &mut Ui, id: u64, name: &str) {
        let key = Id::new(("env-material-name", id));
        let mut text: String = ui.data_mut(|d| d.get_temp(key).unwrap_or_else(|| name.to_owned()));
        let r = crate::option_row::row(ui, "Name", |ui| {
            ui.add(TextEdit::singleline(&mut text).desired_width(ui.available_width()))
        })
        .inner
        .on_hover_text("The layer's name; Enter renames it");
        self.text_editors.insert(r.id);
        crate::numeric_tab::register(ui, &r, Some(&text));
        if r.changed() {
            ui.data_mut(|d| d.insert_temp(key, text.clone()));
        }
        if r.lost_focus() {
            ui.data_mut(|d| d.remove::<String>(key));
            // Runs item.name layer:<material> (UIC-164/165).
            if crate::numeric_tab::accepted(&r)
                && text.trim() != name
                && !text.trim().is_empty()
                && let Some(material) = self
                    .editor
                    .scene
                    .item(id)
                    .and_then(|i| i.environment.as_ref())
                    .map(|env| env.material.id)
            {
                self.run_command_line(&format!("item.name {{{}}} layer:{material}", text.trim()));
            }
        }
    }

    fn env_daylight_rows(
        &mut self,
        ui: &mut Ui,
        id: u64,
        m: &EnvironmentMaterial,
        d: &EnvironmentMaterial,
    ) {
        let (l, ld) = (&m.daylight, &d.daylight);
        let mut suns: Vec<(Option<u64>, String)> = vec![(None, "(none)".to_owned())];
        suns.extend(self.editor.scene.items.iter().filter_map(|i| {
            i.light
                .as_ref()
                .filter(|l| l.kind == operandi_core::camera_light::LightKind::Directional)
                .map(|_| (Some(i.id), i.name.clone()))
        }));
        let options: Vec<(Option<u64>, &str)> =
            suns.iter().map(|(v, n)| (*v, n.as_str())).collect();
        self.env_choice(
            ui,
            id,
            "Sun Light",
            l.sun_light,
            None,
            &options,
            EnvironmentEdit::SunLight,
        )
        .on_hover_text(
            "The directional light whose direction places the sun. Without one the \
             environment draws its 4 Color Gradient",
        );
        self.env_number(
            ui,
            id,
            "env-disc",
            "Solar Disc Size",
            l.disc_size,
            ld.disc_size,
            0.0..=1e4,
            " %",
            100.0,
            EnvironmentEdit::DiscSize,
        );
        self.env_number(
            ui,
            id,
            "env-haze",
            "Haze Amount",
            l.haze,
            ld.haze,
            0.0..=1e3,
            "",
            1.0,
            EnvironmentEdit::Haze,
        );
        self.env_check(
            ui,
            id,
            "Clamp Sky Brightness",
            l.clamp_sky_brightness,
            ld.clamp_sky_brightness,
            EnvironmentEdit::ClampSkyBrightness,
        );
        self.env_number(
            ui,
            id,
            "env-gamma",
            "Sky Gamma",
            l.sky_gamma,
            ld.sky_gamma,
            0.01..=100.0,
            "",
            1.0,
            EnvironmentEdit::SkyGamma,
        );
        self.env_color(
            ui,
            id,
            "Ground Albedo",
            l.ground_albedo,
            ld.ground_albedo,
            EnvironmentEdit::GroundAlbedo,
        );
        self.env_color(
            ui,
            id,
            "Disc Color",
            l.disc_color,
            ld.disc_color,
            EnvironmentEdit::DiscColor,
        );
        self.env_number(
            ui,
            id,
            "env-inscatter",
            "Disc In-Scatter",
            l.disc_in_scatter,
            ld.disc_in_scatter,
            0.0..=100.0,
            " %",
            100.0,
            EnvironmentEdit::DiscInScatter,
        );
    }

    fn env_fog_rows(
        &mut self,
        ui: &mut Ui,
        id: u64,
        m: &EnvironmentMaterial,
        d: &EnvironmentMaterial,
    ) {
        let (f, fd) = (&m.fog, &d.fog);
        let kinds: Vec<(EnvironmentFogType, &str)> = EnvironmentFogType::ALL
            .iter()
            .map(|&k| (k, k.label()))
            .collect();
        self.env_choice(
            ui,
            id,
            "Fog Type",
            f.fog_type,
            fd.fog_type,
            &kinds,
            EnvironmentEdit::FogType,
        )
        .on_hover_text("Saved with the scene; renders don't draw environment fog yet");
        use EnvironmentFogType as F;
        ui.gated_ui(
            crate::unavailable::unless(f.fog_type == F::Linear, "Set Fog Type to Linear first"),
            |ui| {
                self.env_number(
                    ui,
                    id,
                    "env-fog-start",
                    "Fog Start",
                    f.start,
                    fd.start,
                    -1e6..=1e6,
                    " m",
                    1.0,
                    EnvironmentEdit::FogStart,
                );
                self.env_number(
                    ui,
                    id,
                    "env-fog-end",
                    "Fog End",
                    f.end,
                    fd.end,
                    -1e6..=1e6,
                    " m",
                    1.0,
                    EnvironmentEdit::FogEnd,
                );
            },
        );
        ui.gated_ui(
            crate::unavailable::unless(
                !matches!(f.fog_type, F::None | F::Linear),
                "Set Fog Type to Exponential or Layered first",
            ),
            |ui| {
                self.env_number(
                    ui,
                    id,
                    "env-fog-density",
                    "Fog Density",
                    f.density,
                    fd.density,
                    0.0..=1e6,
                    "",
                    1.0,
                    EnvironmentEdit::FogDensity,
                );
            },
        );
        ui.gated_ui(
            crate::unavailable::unless(f.fog_type == F::Layered, "Set Fog Type to Layered first"),
            |ui| {
                self.env_number(
                    ui,
                    id,
                    "env-fog-alt",
                    "Base Altitude",
                    f.base_altitude,
                    fd.base_altitude,
                    -1e6..=1e6,
                    " m",
                    1.0,
                    EnvironmentEdit::FogBaseAltitude,
                );
                self.env_number(
                    ui,
                    id,
                    "env-fog-falloff",
                    "Altitude Falloff",
                    f.altitude_falloff,
                    fd.altitude_falloff,
                    0.0..=1e6,
                    "",
                    1.0,
                    EnvironmentEdit::FogAltitudeFalloff,
                );
            },
        );
    }
}

impl Operandi {
    /// The Environments section and its rows (ENV §6, ENVIRONMENT-PLACEMENT
    /// §3.1/§3.2): a plain header (no icon, eye or Properties; collapsed in
    /// a new scene), then each environment, newest (top-most, the one rays
    /// see first) at the top, with its texture layers above its
    /// Environment Material.
    pub(super) fn environment_section_rows(
        &self,
        rows: &mut Vec<tree_list::TreeRow<crate::shading_panel::ShaderKey>>,
        section: impl Fn(
            &'static str,
            Option<bool>,
        ) -> tree_list::TreeRow<crate::shading_panel::ShaderKey>,
    ) {
        use crate::shading_panel::ShaderKey;
        use tree_list::{Cell, RowIcon, Selected, TreeRow};
        let s = &self.shading;
        let scene = &self.editor.scene;
        let envs: Vec<&operandi_core::Item> = scene.environments().collect();
        let open = s.open_sections.contains("Environments");
        let mut heading = section("Environments", (!envs.is_empty()).then_some(open));
        heading.tip = Some(
            "Environments: what rays see when they miss every object. Each ray sees the \
             newest (top-most) environment visible to it"
                .into(),
        );
        rows.push(heading);
        if !open {
            return;
        }
        let columns = crate::shading_panel::shading_spec().columns.len();
        for item in envs {
            let Some(env) = item.environment.as_ref() else {
                continue;
            };
            let expanded = !s.environments_closed_contains(item.id);
            let mut row = TreeRow::new(ShaderKey::Environment(item.id), 1, &item.name, columns);
            row.disclosure = Some(expanded);
            // §3.6: Operandi's Environment icon (batch 13).
            row.icon = RowIcon::Icon(crate::icons::Icon::Environment);
            row.renamable = true;
            row.fixed = true;
            row.style.dimmed = !item.visible;
            row.tint = item.common.editor_color.map(crate::theme::editor_color);
            // §3.3: several environments can be selected (Shift, Cmd/Ctrl).
            row.selected = match s.environment {
                Some(EnvironmentRow::Item(primary)) if primary == item.id => Selected::Primary,
                Some(EnvironmentRow::Item(_)) if scene.selection.also.contains(&item.id) => {
                    Selected::Yes
                }
                _ => Selected::No,
            };
            // §3.2: the item's eye; a hidden environment isn't seen by any
            // ray (`Scene::environment_for`).
            row.cells[0] = Cell::Toggle {
                dimmed: false,
                on: item.visible,
                tip: if item.visible {
                    "Hide this environment"
                } else {
                    "Show this environment"
                }
                .into(),
            };
            row.tip = Some("Environment item: Intensity and which rays see it".into());
            rows.push(row);
            if expanded {
                // ENV §5: texture layers above the material, top first.
                for node in &env.layers {
                    let key = ShaderKey::EnvironmentLayer(item.id, node.id);
                    let mut row = TreeRow::new(key, 2, &node.name, columns);
                    row.icon = RowIcon::Icon(crate::icons::Icon::TreeTexture);
                    row.renamable = true;
                    row.fixed = true;
                    row.style.dimmed = !node.layer.enable;
                    row.selected = if s.environment == Some(EnvironmentRow::Layer(item.id, node.id))
                    {
                        Selected::Primary
                    } else {
                        Selected::No
                    };
                    row.cells[0] = Cell::Toggle {
                        dimmed: false,
                        on: node.layer.enable,
                        tip: if node.layer.enable {
                            "Disable layer"
                        } else {
                            "Enable layer"
                        }
                        .into(),
                    };
                    row.tip = Some(format!("{} · environment layer", node.type_label()));
                    rows.push(row);
                }
                let layer = &env.material.layer;
                let mut row = TreeRow::new(
                    ShaderKey::EnvironmentMaterial(item.id),
                    2,
                    &env.material.name,
                    columns,
                );
                row.icon = RowIcon::Icon(crate::icons::Icon::TreeMaterial);
                row.renamable = true;
                row.fixed = true;
                row.style.dimmed = !layer.enable;
                row.selected = if s.environment == Some(EnvironmentRow::Material(item.id)) {
                    Selected::Primary
                } else {
                    Selected::No
                };
                row.cells[0] = Cell::Toggle {
                    dimmed: false,
                    on: layer.enable,
                    tip: if layer.enable {
                        "Disable layer"
                    } else {
                        "Enable layer"
                    }
                    .into(),
                };
                row.tip = Some(format!(
                    "{} · {}",
                    env.material.settings.kind.label(),
                    "Effect Environment Color"
                ));
                rows.push(row);
            }
        }
    }

    /// Click on an Environment Material or environment layer row: it alone
    /// is selected (§3.3). An Environment row goes through
    /// [`Self::click_environment_row`].
    pub(super) fn select_environment_row(&mut self, row: EnvironmentRow) {
        if let EnvironmentRow::Item(id) = row {
            self.click_environment_row(id, tree_list::ClickHow::Only);
            return;
        }
        self.shading.selected.clear();
        self.shading.environment = Some(row);
    }

    /// Click on an Environment row (ENVIRONMENT-PLACEMENT §3.3): the
    /// Shader Tree's click, Shift range and Cmd/Ctrl toggle over the
    /// environment rows. The environments become the item selection (so
    /// item.* commands and Delete reach them); the Item List never shows
    /// them (§1).
    pub(super) fn click_environment_row(&mut self, id: u64, how: tree_list::ClickHow) {
        let order: Vec<u64> = self.editor.scene.environments().map(|i| i.id).collect();
        let mut selected: Vec<u64> = match self.shading.environment {
            Some(EnvironmentRow::Item(_)) => self
                .editor
                .scene
                .selected_items()
                .into_iter()
                .filter(|i| order.contains(i))
                .collect(),
            _ => Vec::new(),
        };
        let mut anchor = self.shading.environment_anchor;
        tree_list::click_select(&mut selected, &mut anchor, &order, id, how);
        self.shading.environment_anchor = anchor;
        self.shading.selected.clear();
        match selected.first().copied() {
            Some(primary) => {
                self.shading.environment = Some(EnvironmentRow::Item(primary));
                if self.editor.scene.selected_items() != selected {
                    self.run(Command::SelectItems(selected));
                }
            }
            None => {
                self.shading.environment = None;
                self.run(Command::SelectItem(None));
            }
        }
    }

    /// Item > Create Environment and Add Layer > Environment (ENV §6, §8):
    /// a new environment, selected and shown.
    pub(super) fn create_environment(&mut self) {
        if self.run_ok(Command::AddItem(
            operandi_core::item_list::AddItemKind::Environment,
        )) {
            self.reveal_new_environment();
        }
    }

    /// §3.5: a new environment is selected and shown (section open), from
    /// Item > Create Environment (`item.create environment`) or Add Item.
    pub(super) fn reveal_new_environment(&mut self) {
        self.shading.open_sections.insert("Environments");
        if let Some(id) = self.editor.scene.selection.item {
            self.shading.selected.clear();
            self.shading.environment = Some(EnvironmentRow::Item(id));
            self.shading.environment_anchor = Some(id);
            self.shading.environments_open(id);
        }
    }

    /// Delete an environment with its layers (ENV §6): one undo step.
    pub(super) fn delete_environment(&mut self, id: u64) {
        if self.run_ok(Command::SelectItem(Some(id))) {
            self.run(Command::DeleteItem);
            self.shading.environment = None;
        }
    }

    /// Right-click menus. The Environments header (§3.1, a header, not an
    /// item): only Create Environment. An Environment row (§3.4): Rename,
    /// Editor Color, Properties | Duplicate, Delete | Presets. Every entry
    /// runs an existing command (UIC-160..165).
    pub(super) fn environment_menu(
        &mut self,
        ui: &mut Ui,
        id: Option<u64>,
        rename: &mut dyn FnMut(),
    ) {
        self.menu_owned_input = true;
        let Some(id) = id else {
            // item.create environment, as Item > Create Environment.
            if ui.button("Create Environment").clicked() {
                ui.close();
                self.create_environment();
            }
            return;
        };
        let Some(item) = self.editor.scene.item(id).cloned() else {
            return;
        };
        let idle = self.editor.session.live.is_none();
        let busy = crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE);
        if ui.gated(busy, egui::Button::new("Rename…")).clicked() {
            rename();
            ui.close();
        }
        // Note 10: every selected environment when the clicked one is
        // selected (item.editorColor through EditItemCommon).
        if idle && let Some(color) = crate::editor_color_ui::submenu(ui, item.common.editor_color) {
            let items = self.menu_targets(id);
            self.set_item_editor_color(items, color);
        }
        if ui.button("Properties").clicked() {
            self.click_environment_row(id, tree_list::ClickHow::Only);
            self.show_properties_panel();
            ui.close();
        }
        // EP §Save.
        if ui
            .gated(busy, egui::Button::new("Save Environment Preset…"))
            .clicked()
        {
            ui.close();
            self.execute(&format!("preset.saveAs item:{id}"));
        }
        ui.separator();
        if ui.gated(busy, egui::Button::new("Duplicate")).clicked() {
            ui.close();
            if self.run_ok(Command::SelectItem(Some(id)))
                && self.run_ok(Command::DuplicateItem)
                && let Some(copy) = self.editor.scene.selection.item
            {
                self.shading.environment = Some(EnvironmentRow::Item(copy));
            }
        }
        if ui.gated(busy, egui::Button::new("Delete")).clicked() {
            ui.close();
            self.delete_environment(id);
        }
        ui.separator();
        // §3.4 disabled-reason rule: presets of one environment aren't
        // wired; the Environment preset saves every light and environment.
        ui.gated(
            Some(
                "Presets of one environment aren't available yet. File > Save Environment \
                 Preset… saves every light and environment",
            ),
            egui::Button::new("Presets"),
        );
    }
}

impl Operandi {
    fn environment_layer(
        &self,
        item: u64,
        layer: operandi_core::shader_tree::NodeId,
    ) -> Option<operandi_core::shader_tree::Node> {
        self.environment(item)?
            .layers
            .iter()
            .find(|n| n.id == layer)
            .cloned()
    }

    fn replace_environment_layer(
        &mut self,
        item: u64,
        node: operandi_core::shader_tree::Node,
        gesture: Option<u64>,
    ) {
        let previous = std::mem::replace(&mut self.editor.history.undo_fold, gesture);
        self.run(Command::EditEnvironmentLayers {
            item,
            edit: operandi_core::environment::EnvironmentLayerEdit::Replace(Box::new(node)),
        });
        self.editor.history.undo_fold = previous;
    }

    /// Add Layer with an environment row selected (ENV §8): the layer goes
    /// into that environment, directly above the selected layer (or at the
    /// top), and is selected.
    pub(super) fn add_environment_layer(&mut self, layer: operandi_core::shader_tree::NewLayer) {
        let Some(row) = self.shading.environment else {
            return;
        };
        let item = row.item();
        if !operandi_core::environment::environment_can_hold(layer) {
            self.status = format!("{} can't go under an environment yet", layer.label());
            return;
        }
        use operandi_core::environment::LayerPlace;
        let at = match row {
            EnvironmentRow::Layer(_, id) => LayerPlace::Above(id),
            EnvironmentRow::Material(_) => LayerPlace::AboveMaterial,
            EnvironmentRow::Item(_) => LayerPlace::Top,
        };
        let before: Vec<_> = self
            .environment(item)
            .map(|e| e.layers.iter().map(|n| n.id).collect())
            .unwrap_or_default();
        let edit = operandi_core::environment::EnvironmentLayerEdit::Add { layer, at };
        if self.run_ok(Command::EditEnvironmentLayers { item, edit }) {
            let new = self.environment(item).and_then(|e| {
                e.layers
                    .iter()
                    .find(|n| !before.contains(&n.id))
                    .map(|n| n.id)
            });
            if let Some(id) = new {
                self.shading.environment = Some(EnvironmentRow::Layer(item, id));
            }
            self.shading.environments_open(item);
        }
    }

    pub(super) fn rename_environment_layer(
        &mut self,
        item: u64,
        layer: operandi_core::shader_tree::NodeId,
        name: String,
    ) {
        // Runs item.name layer:<id> (UIC-164/165).
        if self.environment_layer(item, layer).is_some() && !name.trim().is_empty() {
            self.run_command_line(&format!("item.name {{{}}} layer:{layer}", name.trim()));
        }
    }

    pub(super) fn toggle_environment_layer(
        &mut self,
        item: u64,
        layer: operandi_core::shader_tree::NodeId,
    ) {
        if let Some(mut node) = self.environment_layer(item, layer) {
            node.layer.enable = !node.layer.enable;
            self.replace_environment_layer(item, node, None);
        }
    }

    /// Properties of a texture layer under an environment (ENV §5): the
    /// layer block with the environment effects, the layer's own settings,
    /// and its Texture Locator (whose rotation turns it).
    pub(super) fn environment_layer_rows(
        &mut self,
        ui: &mut Ui,
        item: u64,
        layer: operandi_core::shader_tree::NodeId,
    ) {
        use operandi_core::shader_tree::{NodeKind, TextureSource};
        let Some(node) = self.environment_layer(item, layer) else {
            ui.weak("Select an environment layer");
            return;
        };
        ui.strong(node.type_label());
        // A texture layer's Properties have the shader tree's two sub-tabs
        // (env-texture-locator).
        if let NodeKind::Texture(t) = &node.kind {
            use crate::shading_panel::LayerTab;
            ui.horizontal(|ui| {
                let tab = &mut self.shading.layer_tab;
                ui.selectable_value(tab, LayerTab::Layer, "Layer");
                ui.selectable_value(tab, LayerTab::TextureLocator, "Texture Locator");
            });
            if self.shading.layer_tab == LayerTab::TextureLocator {
                let locator = t.locator;
                let idle = self.editor.session.live.is_none();
                ui.gated_ui(
                    crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                    |ui| {
                        self.environment_locator_rows(ui, locator);
                    },
                );
                return;
            }
        }
        let mut edited = node.clone();
        let mut continuing = false;
        let mut load_image_here = false;
        let idle = self.editor.session.live.is_none();
        ui.gated_ui(crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE), |ui| {
            ui.spacing_mut().item_spacing.y = 2.;
            let key = Id::new(("env-layer-name", item, layer));
            let mut text: String =
                ui.data_mut(|d| d.get_temp(key).unwrap_or_else(|| node.name.clone()));
            let r = crate::option_row::row(ui, "Name", |ui| {
                ui.add(TextEdit::singleline(&mut text).desired_width(ui.available_width()))
            })
            .inner;
            self.text_editors.insert(r.id);
            crate::numeric_tab::register(ui, &r, Some(&text));
            if r.changed() {
                ui.data_mut(|d| d.insert_temp(key, text.clone()));
            }
            if r.lost_focus() {
                ui.data_mut(|d| d.remove::<String>(key));
                if crate::numeric_tab::accepted(&r) && !text.trim().is_empty() {
                    edited.name = text.trim().to_owned();
                }
            }
            crate::option_row::check(ui, "Enable", &mut edited.layer.enable);
            crate::option_row::check(ui, "Invert", &mut edited.layer.invert);
            crate::option_row::field_row(ui, "Blend Mode", |ui| {
                crate::shading_panel::enum_combo(
                    ui,
                    "env-layer-blend",
                    BlendMode::ALL,
                    &mut edited.layer.blend,
                    |m| m.label(),
                );
            })
            .response
            .on_hover_text(
                "Add, Multiply, Screen, Difference, Darken, Lighten and Overlay mix as \
                 named; the others blend as Normal for now",
            );
            crate::option_row::field_row(ui, "Opacity", |ui| {
                continuing |= crate::shading_panel::percent(ui, &mut edited.layer.opacity, 0.);
            });
            if let NodeKind::Texture(t) = &mut edited.kind {
                crate::option_row::field_row(ui, "Effect", |ui| {
                    crate::shading_panel::enum_combo(
                        ui,
                        "env-layer-effect",
                        operandi_core::environment::ENVIRONMENT_EFFECTS,
                        &mut t.effect,
                        |e| e.label(),
                    );
                });
                if let TextureSource::ImageMap { image, .. } = &mut t.source {
                    let images: Vec<(u64, String)> = self
                        .editor
                        .scene
                        .images
                        .iter()
                        .map(|i| (i.id, i.name.clone()))
                        .collect();
                    let shown = image.map_or("(none)".to_owned(), |id| {
                        images
                            .iter()
                            .find(|(i, _)| *i == id)
                            .map_or("(missing)".to_owned(), |(_, n)| n.clone())
                    });
                    crate::option_row::field_row(ui, "Image", |ui| {
                        ComboBox::from_id_salt("env-layer-image")
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(image, None, "(none)");
                                for (id, name) in &images {
                                    ui.selectable_value(image, Some(*id), name);
                                }
                            });
                    })
                    .response
                    .on_hover_text(
                        "The latitude-longitude picture this layer shows. HDR and EXR \
                         keep values above 1",
                    );
                    load_image_here = true;
                }
            }
            if load_image_here {
                self.environment_image_rows(ui, item, Some(layer));
            }
            // [chosen] An Image Map's adjustments (brightness, gamma…) aren't
            // applied to the sky, so they aren't shown here.
            let image_map = matches!(
                &edited.kind,
                NodeKind::Texture(t) if matches!(t.source, TextureSource::ImageMap { .. })
            );
            for (name, channel) in edited.channels().into_iter().filter(|_| !image_map) {
                crate::option_row::field_row(ui, name, |ui| {
                    continuing |= crate::shading_panel::channel_widget(ui, channel);
                });
            }
            if let NodeKind::Texture(t) = &node.kind {
                if !matches!(t.source, TextureSource::Constant { .. } | TextureSource::ImageMap { .. }) {
                    ui.weak("Saved with the scene; renders don't draw this layer under an environment yet");
                }
                if let Some(locator) = t.locator {
                    let name = self
                        .editor
                        .scene.item(locator)
                        .map_or("(missing)".to_owned(), |i| i.name.clone());
                    crate::option_row::row(ui, "Texture Locator", |ui| {
                        if ui
                            .button(name)
                            .on_hover_text("Select the locator: its Rotation turns this layer on the sky")
                            .clicked()
                        {
                            self.run(Command::SelectItem(Some(locator)));
                        }
                    });
                }
            }
        });
        if edited != node {
            // One drag is one undo step (ENV §11).
            let key = Id::new(("env-layer-gesture", item, layer));
            let gesture: u64 = if continuing {
                ui.data(|d| d.get_temp(key)).unwrap_or(1)
            } else {
                let next: u64 = ui.data(|d| d.get_temp(key)).unwrap_or(0) + 1;
                ui.data_mut(|d| d.insert_temp(key, next));
                next
            };
            let fold = Some(gesture ^ (layer << 20) ^ 0xE5_0000_0000);
            self.replace_environment_layer(item, edited, fold);
        }
    }
}

impl Operandi {
    /// The Texture Locator sub-tab of an environment texture layer
    /// (env-texture-locator): Transform and Projection, then Mapping and
    /// Falloff. Only Rotation, Rotation Order and Projection Axis move a
    /// picture at infinity; the rest is saved with the scene and says so
    /// in its tooltip. Every edit is one undo step, one drag one step
    /// (`ShaderTreeEdit::SetLocator`).
    fn environment_locator_rows(&mut self, ui: &mut Ui, locator: Option<u64>) {
        use crate::shading_panel::{enum_combo, rotation_order_row};
        use operandi_core::shader_tree::{Projection, ShaderTreeEdit};
        use operandi_core::texture_locator::{FalloffType, MIN_SIZE, Tiling};
        let Some(item) = locator.and_then(|id| {
            self.editor
                .scene
                .items
                .iter()
                .find(|i| i.id == id && i.texture_locator().is_some())
        }) else {
            ui.weak("This layer has no Texture Locator");
            return;
        };
        let locator = item.id;
        let settings = item.texture_locator().expect("checked").clone();
        let transform = item.transform.clone();
        let mut s = settings.clone();
        let mut tr = transform.clone();
        // Set while a drag continues, so one drag is one undo step.
        let dragging = std::cell::Cell::new(false);
        const AT_INFINITY: &str =
            "Saved with the scene; it doesn't move the picture, which is at infinity";
        const STORED: &str = "Saved with the scene; it doesn't change the environment yet";
        let drag = |ui: &mut Ui,
                    v: &mut f64,
                    range: std::ops::RangeInclusive<f64>,
                    speed: f64,
                    suffix: &str| {
            let field = crate::numeric_tab::DragValue::new(v)
                .range(range)
                .speed(speed)
                .max_decimals(4);
            let field = if suffix == " m" {
                field.distance()
            } else {
                field.suffix(suffix.to_owned())
            };
            let r = ui.add(field);
            if r.dragged() && !r.drag_started() {
                dragging.set(true);
            }
            r
        };
        let vector = |ui: &mut Ui,
                      label: &str,
                      v: &mut glam::DVec3,
                      range: std::ops::RangeInclusive<f64>,
                      scale: f64,
                      speed: f64,
                      suffix: &str,
                      tip: &str| {
            // UIC-114: one row per component (issue #6: three fields on one
            // row ran past the panel's edge).
            crate::option_row::vector(ui, label, &["X", "Y", "Z"], tip, |ui, k| {
                let mut x = v[k] * scale;
                drag(ui, &mut x, range.clone(), speed, suffix);
                if (x / scale - v[k]).abs() > 1e-12 {
                    v[k] = x / scale;
                }
            });
        };
        ui.weak(item.name.clone());
        crate::form::rows::section(ui, "Transform", false, |ui| {
            vector(
                ui,
                "Position",
                &mut tr.translation,
                -1e9..=1e9,
                1.,
                0.01,
                " m",
                AT_INFINITY,
            );
            vector(
                ui,
                "Rotation",
                &mut tr.rotation,
                -1e6..=1e6,
                180. / std::f64::consts::PI,
                0.5,
                "°",
                "Turns the picture on the sky; Y turns it about the vertical, the same way in \
                 the viewport and in renders",
            );
            rotation_order_row(ui, "env-locator-rotation-order", &mut tr);
            vector(
                ui,
                "Size",
                &mut tr.scale,
                MIN_SIZE..=1e8,
                1.,
                0.01,
                " m",
                AT_INFINITY,
            );
            ui.gated_ui(
                Some("An environment's picture is at infinity: there are no surfaces to fit"),
                |ui| crate::option_row::button(ui, "Auto Size"),
            );
            crate::option_row::check(ui, "World Transforms", &mut s.world_transforms)
                .on_hover_text(STORED);
            crate::option_row::check(ui, "World Coordinates", &mut s.world_coordinates)
                .on_hover_text(AT_INFINITY);
        });
        crate::form::rows::section(ui, "Projection", false, |ui| {
            crate::option_row::field_row(ui, "Projection Type", |ui| {
                enum_combo(
                    ui,
                    "env-locator-projection",
                    Projection::ALL,
                    &mut s.projection,
                    |p| p.label(),
                );
            })
            .response
            .on_hover_text(
                "Spherical reads the picture as latitude-longitude; Light Probe as a \
                 mirror-ball photo. Under an environment the other types draw as \
                 Spherical for now; the choice is saved",
            );
            crate::option_row::field_row(ui, "Projection Axis", |ui| {
                ComboBox::from_id_salt("env-locator-axis")
                    .selected_text(["X", "Y", "Z"][usize::from(s.axis.min(2))])
                    .show_ui(ui, |ui| {
                        for (k, name) in ["X", "Y", "Z"].into_iter().enumerate() {
                            ui.selectable_value(&mut s.axis, k as u8, name);
                        }
                    });
            })
            .response
            .on_hover_text("The picture's pole: its top row is seen along this axis (Y: overhead)");
        });
        crate::form::rows::section(ui, "Mapping", false, |ui| {
            for (k, label) in ["Horizontal Repeat", "Vertical Repeat"]
                .into_iter()
                .enumerate()
            {
                crate::option_row::field_row(ui, label, |ui| {
                    enum_combo(
                        ui,
                        &format!("env-locator-tiling-{k}"),
                        Tiling::ALL,
                        &mut s.tiling[k],
                        |t| t.label(),
                    );
                })
                .response
                .on_hover_text(STORED);
            }
            for (k, label) in ["Repeat U", "Repeat V"].into_iter().enumerate() {
                crate::option_row::field_row(ui, label, |ui| {
                    drag(ui, &mut s.repeat[k], -1e6..=1e6, 0.01, "");
                })
                .response
                .on_hover_text(STORED);
            }
            for (k, label) in ["U Tile Offset", "V Tile Offset"].into_iter().enumerate() {
                crate::option_row::field_row(ui, label, |ui| {
                    drag(ui, &mut s.tile_offset[k], -1e6..=1e6, 0.01, "");
                })
                .response
                .on_hover_text(STORED);
            }
            crate::option_row::field_row(ui, "UV Rotation", |ui| {
                drag(ui, &mut s.uv_rotation, -1e6..=1e6, 0.5, "°");
            })
            .response
            .on_hover_text(STORED);
            crate::option_row::field_row(ui, "Texture Offset Strength", |ui| {
                drag(ui, &mut s.random_strength, 0.0..=10.0, 0.01, "");
            })
            .response
            .on_hover_text(STORED);
        });
        crate::form::rows::section(ui, "Falloff", false, |ui| {
            crate::option_row::field_row(ui, "Falloff Type", |ui| {
                enum_combo(
                    ui,
                    "env-locator-falloff",
                    FalloffType::ALL,
                    &mut s.falloff_type,
                    |f| f.label(),
                );
            })
            .response
            .on_hover_text(AT_INFINITY);
            crate::option_row::vector(ui, "Falloff", &["X", "Y", "Z"], AT_INFINITY, |ui, k| {
                drag(ui, &mut s.falloff[k], 0.0..=1e9, 0.01, "");
            });
        });
        if s != settings || tr != transform {
            self.run(Command::ShaderTree(ShaderTreeEdit::SetLocator {
                locator,
                settings: Box::new(s),
                transform: tr,
                continuing: dragging.get(),
            }));
        }
    }

    /// The Environment Image rows (HDRI stage 1). On the Environment tab
    /// (`layer` None): Image (the picture's name), Load Image…, Clear
    /// Image, Rotation, for the environment's image layer (its top Image
    /// Map layer). On an image layer's own tab (`layer` Some): Load Image…
    /// and Rotation for that layer. Rows grey, not hide, without an image
    /// (UIC-117); labels right-aligned Title Case with a tooltip each
    /// (UIC-119); buttons fill the field column on rows of their own
    /// (UIC-116); Rotation is a degree field with mini-slider arrows
    /// (UIC-113, UIC-53).
    pub(super) fn environment_image_rows(
        &mut self,
        ui: &mut Ui,
        env: u64,
        layer: Option<operandi_core::shader_tree::NodeId>,
    ) {
        use operandi_core::shader_tree::NodeKind;
        let scene = &self.editor.scene;
        let image_layer = match layer {
            Some(id) => self
                .environment(env)
                .and_then(|e| e.layers.iter().find(|n| n.id == id)),
            None => scene.environment_image_layer(env),
        }
        .cloned();
        let picture = scene.environment_image(env).map(|i| i.name.clone());
        let locator = image_layer.as_ref().and_then(|n| match &n.kind {
            NodeKind::Texture(t) => t.locator,
            _ => None,
        });
        let heading = locator
            .and_then(|l| scene.item(l))
            .map(|i| i.transform.rotation.y);
        if layer.is_none() {
            crate::form::rows::status(picture.is_some());
            crate::option_row::value(ui, "Image", picture.as_deref().unwrap_or("(none)"))
                .on_hover_text(
                    "The latitude-longitude picture this environment shows (its top Image \
                     Map layer)",
                );
            if let Some(error) = scene
                .environment_image(env)
                .and_then(|i| self.sky_cache.error(&i.path))
            {
                crate::form::rows::note(ui, error);
            }
        }
        if crate::option_row::button(ui, "Load Image…")
            .on_hover_text(
                "Choose an HDR, EXR, PNG, JPEG or TIFF picture for the sky: it replaces the \
                 image layer's picture (keeping its rotation and opacity), or adds an image \
                 layer. One undo step",
            )
            .clicked()
        {
            // UIC-164: the button runs its command.
            self.run_command_line(&format!("operandi.environment.loadImage item:{env}"));
        }
        if layer.is_none() {
            let gate = crate::unavailable::unless(image_layer.is_some(), NO_ENVIRONMENT_IMAGE);
            let clear = ui
                .gated_ui(gate, |ui| crate::option_row::button(ui, "Clear Image"))
                .inner
                .on_hover_text("Remove the image layer (and its Texture Locator). One undo step");
            if clear.clicked()
                && let Some(node) = &image_layer
            {
                self.run_command_line(&format!("texture.delete item:{}", node.id));
            }
        }
        let gate = crate::unavailable::unless(locator.is_some(), NO_ENVIRONMENT_IMAGE);
        ui.gated_ui(gate, |ui| {
            self.environment_rotation_row(ui, env, locator, heading.unwrap_or(0.0));
        });
    }

    /// Rotation: the image layer's Texture Locator heading (rotation about
    /// +Y) in degrees; as a script, `transform.channel rot.Y <angle>
    /// item:<locator>`. It previews while dragging; one drag or one typed
    /// value is one undo step (Command::ItemTransform folded by gesture).
    fn environment_rotation_row(
        &mut self,
        ui: &mut Ui,
        env: u64,
        locator: Option<u64>,
        heading: f64,
    ) {
        let mut shown = heading.to_degrees();
        crate::form::rows::status(heading.abs() > 1e-12);
        let response = ui
            .horizontal(|ui| {
                crate::option_row::label(ui, "Rotation");
                crate::option_row::fill(ui, |ui| {
                    ui.add(
                        crate::numeric_tab::DragValue::new(&mut shown)
                            .range(-360.0..=360.0)
                            .speed(0.5)
                            .fixed_decimals(1)
                            .suffix(" °"),
                    )
                })
            })
            .inner
            .on_hover_text(
                "Turns the picture about the vertical axis (its Texture Locator's heading). \
                 Positive turns it the same way in the viewport and in renders",
            );
        let Some(locator) = locator else {
            return;
        };
        let key = Id::new(("env-rotation-gesture", env));
        if response.drag_started() || (response.changed() && !response.dragged()) {
            let next: u64 = ui.data(|d| d.get_temp(key)).unwrap_or(0) + 1;
            ui.data_mut(|d| d.insert_temp(key, next));
        }
        if response.changed() && (shown.to_radians() - heading).abs() > 1e-12 {
            let gesture: u64 = ui.data(|d| d.get_temp(key)).unwrap_or(1);
            let Some(mut transform) = self.editor.scene.item(locator).map(|i| i.transform.clone())
            else {
                return;
            };
            transform.rotation.y = shown.to_radians();
            let fold = Some(gesture ^ (locator << 24) ^ 0xE6_0000_0000);
            let previous = std::mem::replace(&mut self.editor.history.undo_fold, fold);
            self.run(Command::ItemTransform {
                item: locator,
                transform,
            });
            self.editor.history.undo_fold = previous;
        }
    }

    /// `operandi.environment.loadImage [filename:PATH] [item:ENV]` typed or
    /// sent as a command (registry thread, 2026-11-12). `args` are the
    /// line's words, the first lower-cased by the caller's check.
    pub(super) fn environment_canonical(&mut self, args: &[String]) -> bool {
        const NAME: &str = "operandi.environment.loadimage";
        if !args.first().is_some_and(|a| a.eq_ignore_ascii_case(NAME)) {
            return false;
        }
        let bound = match operandi_core::command_args::bind(
            "operandi.environment.loadImage",
            &args[1..],
            &["filename", "item"],
        ) {
            Ok(b) => b,
            Err(e) => {
                self.status = e;
                return true;
            }
        };
        let item = match bound[1] {
            None => None,
            Some(v) => match v.trim().parse::<u64>() {
                Ok(id) if self.environment(id).is_some() => Some(id),
                _ => {
                    self.status = format!(
                        "operandi.environment.loadImage: no environment {v} (item: takes an environment's id)"
                    );
                    return true;
                }
            },
        };
        match bound[0] {
            Some(path) => {
                self.environment_image_chosen(item, std::path::Path::new(path));
            }
            None => {
                let env = match item {
                    Some(id) => id,
                    None => {
                        // The top environment, made now when there is none
                        // (its own undo step only if a picture is chosen).
                        if self.editor.scene.environments().next().is_none() {
                            self.create_environment();
                        }
                        match self.editor.scene.environments().next() {
                            Some(i) => i.id,
                            None => return true,
                        }
                    }
                };
                self.request_environment_image(env);
            }
        }
        true
    }

    /// Load Image… for environment `env`: the native panel, or the status
    /// line when there is none.
    pub(super) fn request_environment_image(&mut self, env: u64) {
        let folder = self
            .editor
            .scene
            .environment_image(env)
            .and_then(|i| {
                std::path::Path::new(&i.path)
                    .parent()
                    .map(|p| p.to_path_buf())
            })
            .or_else(crate::backdrop_ui::dirs_home)
            .unwrap_or_default();
        if !self.choose(
            crate::file_chooser::Choose::EnvironmentImage { folder },
            crate::file_chooser::Purpose::EnvironmentImage(env),
        ) {
            self.status = "No file panel is available to choose a picture".into();
        }
    }

    /// The picture at `path` becomes environment `env`'s image (the top
    /// environment's for `None`): Command::SetEnvironmentImage, one undo
    /// step. Returns whether it did.
    pub(super) fn environment_image_chosen(
        &mut self,
        env: Option<u64>,
        path: &std::path::Path,
    ) -> bool {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let Ok((width, height)) = image::image_dimensions(path) else {
            self.status = format!("Can't read {name} as a picture");
            return false;
        };
        let ok = self.run_ok(Command::SetEnvironmentImage {
            path: path.to_string_lossy().into_owned(),
            width,
            height,
            item: env,
        });
        if ok {
            self.status = format!("Environment image: {name}");
        }
        ok
    }
}

impl Operandi {
    /// The environment picture this view draws behind everything (HDRI
    /// stage 2), once decoded. While it decodes the gradient shows and the
    /// view asks for another frame.
    pub(super) fn sky_draw(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<crate::environment_gpu::SkyDraw> {
        let sky = self
            .editor
            .scene
            .environment_sky(operandi_core::environment::RayKind::Camera)?;
        let picture = self
            .sky_cache
            .get(&crate::environment_gpu::sky_key(&sky.path, sky.light_probe));
        if self.sky_cache.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        let picture = picture?;
        let m = sky.to_image;
        let rows = [m.row(0), m.row(1), m.row(2)];
        let mut to_image = [0f32; 9];
        for (r, row) in rows.iter().enumerate() {
            to_image[r * 3] = row.x as f32;
            to_image[r * 3 + 1] = row.y as f32;
            to_image[r * 3 + 2] = row.z as f32;
        }
        Some(crate::environment_gpu::SkyDraw {
            picture,
            to_image,
            intensity: sky.intensity as f32,
            opacity: sky.opacity as f32,
            invert: sky.invert,
        })
    }
}

impl Operandi {
    /// HDRI stage 3: the light an environment picture gives the Advanced
    /// style: the picture of the environment indirect rays see (the one
    /// that lights a render), its harmonics and blurred copy, with
    /// Rotation and Intensity. `None` (the flat ambient stays) for a
    /// gradient-only environment or while the picture decodes.
    pub(super) fn env_light(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<(
        crate::gpu::EnvLook,
        std::sync::Arc<crate::environment_gpu::SkyPicture>,
    )> {
        let Some(sky) = self
            .editor
            .scene
            .environment_sky(operandi_core::environment::RayKind::Indirect)
        else {
            return self.gradient_env_light();
        };
        let picture = self
            .sky_cache
            .get(&crate::environment_gpu::sky_key(&sky.path, sky.light_probe));
        if self.sky_cache.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        let picture = picture?;
        let m = sky.to_image;
        let mut to_image = [0f32; 9];
        for r in 0..3 {
            let row = m.row(r);
            to_image[r * 3] = row.x as f32;
            to_image[r * 3 + 1] = row.y as f32;
            to_image[r * 3 + 2] = row.z as f32;
        }
        // [chosen] The layer's Opacity scales the light (over no light,
        // not over the gradient: the gradient's own light isn't added).
        let strength = (sky.intensity * sky.opacity) as f32;
        Some((
            crate::gpu::EnvLook {
                on: true,
                sh: picture.sh,
                to_image,
                intensity: strength,
            },
            picture,
        ))
    }

    /// VM-Q05 (shader-tree-complete): an environment without a picture
    /// (gradients, constant colours, Physically-based Daylight's sky
    /// colours) lights and reflects in the Advanced style through a small
    /// picture made from its own colours, as an image would. `None` when
    /// no environment lights indirect rays (the flat ambient stays).
    fn gradient_env_light(
        &mut self,
    ) -> Option<(
        crate::gpu::EnvLook,
        std::sync::Arc<crate::environment_gpu::SkyPicture>,
    )> {
        use operandi_core::environment::RayKind;
        let env = self
            .editor
            .scene
            .environment_for(RayKind::Indirect)?
            .environment
            .as_deref()?
            .clone();
        // The settings' text is the key: same settings, same picture.
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            format!("{env:?}").hash(&mut h);
            h.finish().to_string()
        };
        let picture = self.sky_cache.generated(&key, || {
            crate::environment_gpu::from_radiance(|d| env.radiance(d))
        });
        Some((
            crate::gpu::EnvLook {
                on: true,
                sh: picture.sh,
                to_image: [1., 0., 0., 0., 1., 0., 0., 0., 1.],
                intensity: 1.,
            },
            picture,
        ))
    }
}

/// Why the image rows are greyed (UIC-117).
const NO_ENVIRONMENT_IMAGE: &str = "This environment has no image; use Load Image… first";

/// Linear RGB (any brightness) as a screen colour.
fn screen_colour(c: [f64; 3]) -> egui::Color32 {
    egui::Rgba::from_rgb(
        c[0].clamp(0.0, 1.0) as f32,
        c[1].clamp(0.0, 1.0) as f32,
        c[2].clamp(0.0, 1.0) as f32,
    )
    .into()
}

impl Operandi {
    /// ENV §9: the environment the camera sees as the view's background.
    /// Colours come from the CPU colour (`stack_color`) at the vertices of
    /// a fine screen mesh; the 4 Color Gradient's hard horizon is kept
    /// exact by splitting the cells it crosses (its screen line is
    /// straight: the ray direction is linear in the pixel). Physically-
    /// based Daylight draws its gradient and image layers aren't drawn
    /// [chosen]. No environment for the camera: black.
    pub(super) fn paint_environment_background(&self, painter: &egui::Painter, r: Rect) {
        match self.environment_background_mesh(r) {
            Some(mesh) => {
                painter.add(mesh);
            }
            None => {
                painter.rect_filled(r, 0., egui::Color32::BLACK);
            }
        }
    }

    /// The background mesh for view rectangle `r`; `None` when no
    /// environment is visible to the camera.
    pub(super) fn environment_background_mesh(&self, r: Rect) -> Option<egui::Mesh> {
        use operandi_core::environment::RayKind;
        let scene = &self.editor.scene;
        let item = scene.environment_for(RayKind::Camera)?;
        let env = item.environment.as_deref()?;
        // ENVIRONMENT-PLACEMENT §2: the item has no transform, so world
        // directions are the environment's own and up is +Y.
        let up = DVec3::Y;
        let ray = |p: Pos2| self.camera.ray(p, r).1;
        let colour = |d: DVec3| screen_colour(env.radiance(d));
        // Just above and just below the horizon, for split edges.
        let side = |d: DVec3, above: bool| {
            let along = d.dot(up);
            let flat = (d - up * along).try_normalize().unwrap_or(d);
            let nudge = if above { 1e-6 } else { -1e-6 };
            colour(flat + up * nudge)
        };
        const COLS: usize = 24;
        const ROWS: usize = 32;
        let at = |i: usize, j: usize| {
            pos2(
                egui::lerp(r.left()..=r.right(), i as f32 / COLS as f32),
                egui::lerp(r.top()..=r.bottom(), j as f32 / ROWS as f32),
            )
        };
        // Each grid point's direction and colour, once.
        let mut grid_dir = vec![DVec3::ZERO; (COLS + 1) * (ROWS + 1)];
        let mut grid_colour = vec![egui::Color32::BLACK; grid_dir.len()];
        for j in 0..=ROWS {
            for i in 0..=COLS {
                let d = ray(at(i, j));
                grid_dir[j * (COLS + 1) + i] = d;
                grid_colour[j * (COLS + 1) + i] = colour(d);
            }
        }
        let mut mesh = egui::Mesh::default();
        let push = |mesh: &mut egui::Mesh, poly: &[(Pos2, egui::Color32)]| {
            let base = mesh.vertices.len() as u32;
            for &(p, c) in poly {
                mesh.colored_vertex(p, c);
            }
            for k in 1..poly.len().saturating_sub(1) as u32 {
                mesh.add_triangle(base, base + k, base + k + 1);
            }
        };
        for j in 0..ROWS {
            for i in 0..COLS {
                let index = |i: usize, j: usize| j * (COLS + 1) + i;
                for corners in [
                    [(i, j), (i + 1, j), (i + 1, j + 1)],
                    [(i, j), (i + 1, j + 1), (i, j + 1)],
                ] {
                    let tri = corners.map(|(a, b)| at(a, b));
                    let ids = corners.map(|(a, b)| index(a, b));
                    let dirs = ids.map(|k| grid_dir[k]);
                    let colours = ids.map(|k| grid_colour[k]);
                    let s = dirs.map(|d| d.dot(up));
                    if s.iter().all(|&v| v >= 0.0) || s.iter().all(|&v| v < 0.0) {
                        let poly: Vec<_> = (0..3).map(|k| (tri[k], colours[k])).collect();
                        push(&mut mesh, &poly);
                        continue;
                    }
                    // Clip the triangle at s = 0 into an above and a below part.
                    for above in [true, false] {
                        let mut poly = Vec::new();
                        for k in 0..3 {
                            let (a, b) = (k, (k + 1) % 3);
                            let inside = |v: f64| if above { v >= 0.0 } else { v < 0.0 };
                            if inside(s[a]) {
                                poly.push((tri[a], colours[a]));
                            }
                            if inside(s[a]) != inside(s[b]) {
                                let t = (s[a] / (s[a] - s[b])) as f32;
                                let p = tri[a] + (tri[b] - tri[a]) * t;
                                poly.push((p, side(ray(p), above)));
                            }
                        }
                        if poly.len() >= 3 {
                            push(&mut mesh, &poly);
                        }
                    }
                }
            }
        }
        Some(mesh)
    }
}
