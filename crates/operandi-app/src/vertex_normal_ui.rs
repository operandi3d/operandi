//! Vertex Normal Tools in the app (G9 VN-0..VN-10): the palette (its own
//! floating window, UIC-183/205, opened from the Model toolbar button and
//! Geometry ▸ Vertex Normal Tools...), its three tabs, the VN-6 Options
//! saved as a preference file, and the commands its buttons run.
//!
//! Every button runs its registry command (UIC-164), so a typed line, the
//! MCP bridge and the palette do the same thing. Rows follow the
//! option-row layout (UIC-87, UIC-100): labels on the left, checkboxes with
//! their name beside the box. The Cage tab (VN-11) is not built yet; it
//! says so in place (UIC-117).
use super::*;
use crate::command_handlers::{Commands, on_words};
use crate::prefs_file::PrefsFile;
use crate::unavailable::Gate;
use operandi_core::command_runtime::Call;
use operandi_core::vertex_normals::{
    EdgeKind, HardenMode, NormalCommand, NormalOptions, NormalRequest, SelectMode, ToggleMaps,
};
use serde::{Deserialize, Serialize};

/// The palette's hash (palettes/vertex-normal-tools.json).
pub(super) const PALETTE: &str = "vertexNormalTools";

/// The palette's tabs (VN-0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Tab {
    #[default]
    Smoothing,
    Tools,
    Cage,
}

/// The VN-6 Options as saved (vertex-normal-preferences.json).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct VertexNormalPreferences {
    pub options: NormalOptions,
}
impl PrefsFile for VertexNormalPreferences {
    const INVALID: &'static str = "Invalid Vertex Normal Tools preference";
    fn valid(&self) -> bool {
        self.options.validate().is_ok()
    }
}

/// The palette's state: its tab, the tool rows' choices and the options.
#[derive(Clone, Debug)]
pub(super) struct VnState {
    pub tab: Tab,
    pub prefs: VertexNormalPreferences,
    pub loaded: bool,
    pub harden_mode: HardenMode,
    pub soften_others: bool,
    pub connected: bool,
    pub soften_mode: HardenMode,
    pub select_soft: bool,
    pub select_mode: SelectMode,
    pub toggle: ToggleMaps,
    pub world: bool,
    pub remove_groups: bool,
    pub offset: i32,
    pub morph: String,
}

impl Default for VnState {
    fn default() -> Self {
        Self {
            tab: Tab::default(),
            prefs: Default::default(),
            loaded: false,
            harden_mode: HardenMode::default(),
            soften_others: false,
            connected: false,
            soften_mode: HardenMode::Edges,
            select_soft: false,
            select_mode: SelectMode::Set,
            toggle: ToggleMaps::Auto,
            // VN-8: World Space on by default (provisional).
            world: true,
            remove_groups: false,
            // VN-10: Offset 1 (provisional).
            offset: 1,
            morph: String::new(),
        }
    }
}

impl Operandi {
    /// The VN-6 options, loaded from the preference file the first time.
    pub(super) fn normal_options(&mut self) -> NormalOptions {
        if !self.vertex_normal_tools.loaded {
            self.vertex_normal_tools.loaded = true;
            self.vertex_normal_tools.prefs = VertexNormalPreferences::load(
                &self.environment.paths().vertex_normal_preferences(),
            );
        }
        self.vertex_normal_tools.prefs.options.clone()
    }

    fn set_normal_options(&mut self, options: NormalOptions) {
        let _ = self.normal_options();
        if options == self.vertex_normal_tools.prefs.options || options.validate().is_err() {
            return;
        }
        self.vertex_normal_tools.prefs.options = options;
        if let Err(e) = self
            .vertex_normal_tools
            .prefs
            .save(&self.environment.paths().vertex_normal_preferences())
        {
            self.status = format!("Vertex Normal options changed; could not save them: {e}");
        }
        // VN-2: Recalculate re-runs whenever an option changes, on meshes
        // that have a normal map.
        let has_map = self
            .editor
            .scene
            .active()
            .is_some_and(|i| operandi_core::vertex_normals::shown_map_name(&i.mesh).is_some());
        if has_map {
            self.execute("Recalculate Vertex Normals");
        }
    }

    /// Runs one request with the current options as one undo step.
    pub(super) fn run_normal_request(&mut self, request: NormalRequest) {
        let options = self.normal_options();
        let label = request.label();
        if self.run_ok(Command::VertexNormals(Box::new(NormalCommand {
            request,
            options,
        }))) {
            self.status = label.into();
        }
    }

    /// The palette's body (PanelKind::VertexNormalTools).
    pub(super) fn vertex_normal_panel(&mut self, ui: &mut Ui) {
        let mut options = self.normal_options();
        let tab = &mut self.vertex_normal_tools.tab;
        ui.horizontal(|ui| {
            for (t, label) in [
                (Tab::Smoothing, "Smoothing"),
                (Tab::Tools, "Tools"),
                (Tab::Cage, "Cage"),
            ] {
                if ui.selectable_label(*tab == t, label).clicked() {
                    *tab = t;
                }
            }
        });
        ui.separator();
        let tab = *tab;
        egui::ScrollArea::vertical().show(ui, |ui| match tab {
            Tab::Smoothing => self.smoothing_tab(ui, &mut options),
            Tab::Tools => self.tools_tab(ui, &options),
            Tab::Cage => {
                ui.weak(
                    "Cage (morph cage view and Morph Push, VN-11) isn't built yet: it comes after \
                     the other Vertex Normal Tools.",
                );
            }
        });
        self.set_normal_options(options);
    }

    fn smoothing_tab(&mut self, ui: &mut Ui, options: &mut NormalOptions) {
        let s = &mut self.vertex_normal_tools;
        let mut line: Option<String> = None;
        crate::form::rows::section(ui, "Recalculate", true, |ui| {
            if ui
                .add(crate::icons::button(
                    ui,
                    crate::icons::Icon::RecalculateNormals,
                    "Recalculate Vertex Normals",
                ))
                .on_hover_text(
                    "Rebuild the selected meshes' normal map: smooth except across hard edges",
                )
                .clicked()
            {
                line = Some("Recalculate Vertex Normals".into());
            }
        });
        crate::form::rows::section(ui, "Harden", true, |ui| {
            let modes: Vec<(HardenMode, &str)> =
                HardenMode::ALL.iter().map(|m| (*m, m.label())).collect();
            crate::option_row::choice(ui, "vn-harden-mode", "Mode", &mut s.harden_mode, &modes);
            crate::option_row::check(ui, "Soften Other Edges", &mut s.soften_others)
                .on_hover_text("Every other edge becomes soft");
            crate::option_row::check(ui, "Connected Verts", &mut s.connected)
                .on_hover_text("Vertices mode: only edges with both ends selected");
            if ui
                .add(crate::icons::button(
                    ui,
                    crate::icons::Icon::HardenNormals,
                    "Harden",
                ))
                .clicked()
            {
                line = Some(format!(
                    "Harden Vertex Normals mode:{} softenOthers:{} connected:{}",
                    s.harden_mode.word(),
                    s.soften_others,
                    s.connected
                ));
            }
        });
        crate::form::rows::section(ui, "Soften", true, |ui| {
            let modes = [
                (HardenMode::Vertices, "Vertices"),
                (HardenMode::Edges, "Edges"),
                (HardenMode::Polygons, "Polygons"),
            ];
            crate::option_row::choice(ui, "vn-soften-mode", "Mode", &mut s.soften_mode, &modes);
            crate::option_row::check(ui, "Connected Verts", &mut s.connected);
            ui.horizontal(|ui| {
                if ui
                    .add(crate::icons::button(
                        ui,
                        crate::icons::Icon::SoftenNormals,
                        "Soften Selected",
                    ))
                    .clicked()
                {
                    line = Some(format!(
                        "Soften Vertex Normals mode:{} connected:{}",
                        s.soften_mode.word(),
                        s.connected
                    ));
                }
                if ui
                    .add(crate::icons::button(
                        ui,
                        crate::icons::Icon::SoftenNormals,
                        "Soften",
                    ))
                    .on_hover_text("Soften every edge")
                    .clicked()
                {
                    line = Some("Soften Vertex Normals mode:all".into());
                }
            });
        });
        crate::form::rows::section(ui, "Select", true, |ui| {
            crate::option_row::choice(
                ui,
                "vn-select-kind",
                "Select",
                &mut s.select_soft,
                &[(false, "Hard Edges"), (true, "Soft Edges")],
            );
            crate::option_row::choice(
                ui,
                "vn-select-mode",
                "Mode",
                &mut s.select_mode,
                &[
                    (SelectMode::Set, "Set"),
                    (SelectMode::Add, "Add to Selection"),
                    (SelectMode::Remove, "Remove from Selection"),
                ],
            );
            if ui
                .add(crate::icons::button(
                    ui,
                    crate::icons::Icon::SelectHardEdges,
                    "Select",
                ))
                .clicked()
            {
                let mode = match s.select_mode {
                    SelectMode::Set => "set",
                    SelectMode::Add => "add",
                    SelectMode::Remove => "remove",
                };
                line = Some(if s.select_soft {
                    format!("Select Soft Edges mode:{mode}")
                } else {
                    format!("Select Hard Edges mode:{mode}")
                });
            }
        });
        crate::form::rows::section(ui, "Options", false, |ui| {
            crate::option_row::check(
                ui,
                "Use Custom Normal Map Name",
                &mut options.use_custom_name,
            );
            ui.horizontal(|ui| {
                crate::option_row::label(ui, "Normal Map Name");
                crate::option_row::fill(ui, |ui| {
                    ui.text_edit_singleline(&mut options.map_name);
                });
            });
            ui.horizontal(|ui| {
                crate::option_row::label(ui, "Angle");
                crate::option_row::fill(ui, |ui| {
                    ui.add(
                        crate::numeric_tab::DragValue::new(&mut options.angle)
                            .range(0.0..=180.)
                            .speed(0.5)
                            .suffix("°"),
                    )
                    .on_hover_text("Harden By Angle hardens edges sharper than this");
                });
            });
            let mut area = options.area_weighting;
            if crate::option_row::check(ui, "Area Weighting", &mut area).changed() {
                options.set_area_weighting(area);
            }
            crate::option_row::check(ui, "Sum Co-Planar Areas", &mut options.sum_coplanar_areas);
            crate::option_row::check(ui, "Angle Weighting", &mut options.angle_weighting);
            let mut tolerance = options.coplanar_tolerance.to_degrees();
            ui.horizontal(|ui| {
                crate::option_row::label(ui, "Coplanar Tolerance");
                crate::option_row::fill(ui, |ui| {
                    ui.add(
                        crate::numeric_tab::DragValue::new(&mut tolerance)
                            .range(0.0..=5.)
                            .speed(0.01)
                            .suffix("°"),
                    );
                });
            });
            options.coplanar_tolerance = tolerance.to_radians();
            let mut full = options.full_weighting;
            if crate::option_row::check(ui, "Full Weighting", &mut full).changed() {
                options.set_full_weighting(full);
            }
            crate::option_row::check(ui, "Harden Edges", &mut options.harden_edges)
                .on_hover_text("Recalculate keeps the HardEdges set hard");
            crate::option_row::check(
                ui,
                "Harden Smoothing Groups",
                &mut options.harden_smoothing_groups,
            )
            .on_hover_text("Recalculate also splits at smoothing-group borders");
            crate::option_row::check(
                ui,
                "High Quality Normals",
                &mut options.high_quality_normals,
            )
            .on_hover_text("Kept for scripts; no effect yet (VN-Q1)");
        });
        if let Some(line) = line {
            self.execute(&line);
        }
    }

    fn tools_tab(&mut self, ui: &mut Ui, options: &NormalOptions) {
        let morphs: Vec<String> = self
            .editor
            .scene
            .active()
            .map(|i| i.mesh.attributes.relative_morphs.keys().cloned().collect())
            .unwrap_or_default();
        let s = &mut self.vertex_normal_tools;
        if s.morph.is_empty() || !morphs.contains(&s.morph) {
            s.morph = morphs.first().cloned().unwrap_or_default();
        }
        let mut line: Option<String> = None;
        crate::form::rows::section(ui, "Toggle Vertex Normal Maps", true, |ui| {
            crate::option_row::choice(
                ui,
                "vn-toggle",
                "Mode",
                &mut s.toggle,
                &[
                    (ToggleMaps::Auto, "Auto"),
                    (ToggleMaps::On, "On"),
                    (ToggleMaps::Off, "Off"),
                ],
            );
            if ui.button("Toggle Vertex Normal Maps").clicked() {
                let mode = match s.toggle {
                    ToggleMaps::Auto => "auto",
                    ToggleMaps::On => "on",
                    ToggleMaps::Off => "off",
                };
                line = Some(format!("Toggle Vertex Normal Maps mode:{mode}"));
            }
        });
        crate::form::rows::section(ui, "Transfer Normals", true, |ui| {
            crate::option_row::check(ui, "World Space", &mut s.world)
                .on_hover_text("Take the items' transforms into account (provisional)");
            if ui
                .add(crate::icons::button(
                    ui,
                    crate::icons::Icon::TransferNormals,
                    "Transfer Normals",
                ))
                .on_hover_text("From the background mesh to the active mesh, nearest surface point")
                .clicked()
            {
                line = Some(format!("Transfer Normals world:{}", s.world));
            }
        });
        crate::form::rows::section(ui, "Convert to Hard Edges", true, |ui| {
            crate::option_row::check(ui, "Remove Smoothing Groups", &mut s.remove_groups);
            if ui.button("Convert to Hard Edges").clicked() {
                line = Some(format!(
                    "Convert to Hard Edges removeGroups:{}",
                    s.remove_groups
                ));
            }
        });
        crate::form::rows::section(ui, "Other Tools", true, |ui| {
            let names: Vec<(String, &str)> =
                morphs.iter().map(|m| (m.clone(), m.as_str())).collect();
            if !names.is_empty() {
                crate::option_row::choice(ui, "vn-morph", "Morph", &mut s.morph, &names);
            }
            if ui
                .gated(
                    crate::unavailable::unless(
                        !names.is_empty(),
                        "The active mesh has no morph map",
                    ),
                    egui::Button::new("Morph to Normals"),
                )
                .clicked()
            {
                line = Some(format!("Morph to Normals morph:{}", s.morph));
            }
            if ui
                .button("Rename Normal Maps")
                .on_hover_text(format!(
                    "Rename the normal maps to \"{}\"",
                    options.map_name
                ))
                .clicked()
            {
                line = Some(format!("Rename Normal Maps name:{}", options.map_name));
            }
            ui.horizontal(|ui| {
                crate::option_row::label(ui, "Offset");
                crate::option_row::fill(ui, |ui| {
                    ui.add(crate::numeric_tab::DragValue::new(&mut s.offset).speed(0.1));
                });
            });
            if ui
                .add(crate::icons::button(
                    ui,
                    crate::icons::Icon::SpinVertexIndices,
                    "Spin Polygon Vertex Indices",
                ))
                .clicked()
            {
                line = Some(format!("Spin Polygon Vertex Indices offset:{}", s.offset));
            }
        });
        if let Some(line) = line {
            self.execute(&line);
        }
    }
}

// ---------------------------------------------------------- commands

/// `key:value` words of a command line.
fn value<'a>(rest: &'a str, key: &str) -> Option<&'a str> {
    rest.split_whitespace().find_map(|w| {
        let (k, v) = w.split_once(':')?;
        k.eq_ignore_ascii_case(key).then_some(v)
    })
}
fn flag(rest: &str, key: &str, default: bool) -> bool {
    value(rest, key).map_or(default, |v| {
        matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "on" | "yes")
    })
}
/// A `key:value` whose value may hold spaces (the rest of the line).
fn tail<'a>(rest: &'a str, key: &str) -> Option<&'a str> {
    let lower = rest.to_ascii_lowercase();
    let at = lower.find(&format!("{}:", key.to_ascii_lowercase()))?;
    Some(rest[at + key.len() + 1..].trim())
}

fn select_mode(rest: &str) -> SelectMode {
    match value(rest, "mode").map(str::to_ascii_lowercase).as_deref() {
        Some("add") => SelectMode::Add,
        Some("remove") => SelectMode::Remove,
        _ => SelectMode::Set,
    }
}

pub(super) fn register(r: &mut Commands) {
    on_words(r, &["vertex normal tools..."], |a, _| {
        a.toggle_palette(PALETTE)
    });
    on_words(r, &["recalculate vertex normals"], |a, _| {
        a.run_normal_request(NormalRequest::Recalculate)
    });
    on_words(r, &["harden vertex normals"], |a, c: &Call<'_>| {
        let mode = value(c.rest, "mode")
            .and_then(HardenMode::from_word)
            .unwrap_or_default();
        a.run_normal_request(NormalRequest::Harden {
            mode,
            soften_others: flag(c.rest, "softenOthers", false),
            connected: flag(c.rest, "connected", false),
        })
    });
    on_words(r, &["soften vertex normals"], |a, c: &Call<'_>| {
        let mode = match value(c.rest, "mode") {
            None => None,
            Some(w) if w.eq_ignore_ascii_case("all") => None,
            Some(w) => HardenMode::from_word(w),
        };
        a.run_normal_request(NormalRequest::Soften {
            mode,
            connected: flag(c.rest, "connected", false),
        })
    });
    on_words(r, &["select hard edges"], |a, c: &Call<'_>| {
        a.run_normal_request(NormalRequest::Select {
            kind: EdgeKind::Hard,
            mode: select_mode(c.rest),
        })
    });
    on_words(r, &["select soft edges"], |a, c: &Call<'_>| {
        a.run_normal_request(NormalRequest::Select {
            kind: EdgeKind::Soft,
            mode: select_mode(c.rest),
        })
    });
    on_words(r, &["toggle vertex normal maps"], |a, c: &Call<'_>| {
        let mode = match value(c.rest, "mode")
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("on") => ToggleMaps::On,
            Some("off") => ToggleMaps::Off,
            _ => ToggleMaps::Auto,
        };
        a.run_normal_request(NormalRequest::ToggleMaps(mode))
    });
    on_words(r, &["transfer normals"], |a, c: &Call<'_>| {
        a.run_normal_request(NormalRequest::Transfer {
            world: flag(c.rest, "world", true),
        })
    });
    on_words(r, &["convert to hard edges"], |a, c: &Call<'_>| {
        a.run_normal_request(NormalRequest::ConvertToHard {
            remove_groups: flag(c.rest, "removeGroups", false),
        })
    });
    on_words(r, &["rename normal maps"], |a, c: &Call<'_>| {
        let mut options = a.normal_options();
        if let Some(name) = tail(c.rest, "name").filter(|n| !n.is_empty()) {
            options.map_name = name.to_owned();
        }
        if a.run_ok(Command::VertexNormals(Box::new(NormalCommand {
            request: NormalRequest::Rename,
            options,
        }))) {
            a.status = "Rename Normal Maps".into();
        }
    });
    on_words(r, &["spin polygon vertex indices"], |a, c: &Call<'_>| {
        let offset = value(c.rest, "offset")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        a.run_normal_request(NormalRequest::Spin { offset })
    });
    on_words(r, &["morph to normals"], |a, c: &Call<'_>| {
        let morph = tail(c.rest, "morph").unwrap_or("").to_owned();
        a.run_normal_request(NormalRequest::MorphToNormals { morph })
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_app() -> Operandi {
        let mut a = crate::tests::box_app();
        a.run(Command::Mode(operandi_core::Mode::Edge));
        a
    }

    /// VN-0: the menu item opens the palette; the buttons' lines run.
    #[test]
    fn the_palette_opens_and_its_commands_run() {
        let mut a = cube_app();
        a.execute("Vertex Normal Tools...");
        assert!(a.palette_open(PALETTE), "{}", a.status);
        a.execute("Harden Vertex Normals mode:angle");
        let m = a.editor.scene.active().unwrap().mesh.to_owned_value();
        assert_eq!(
            operandi_core::vertex_normals::hard_edges(&m).len(),
            12,
            "{}",
            a.status
        );
        a.execute("Select Hard Edges");
        assert_eq!(a.editor.scene.selection.edges.len(), 12);
        a.execute("Soften Vertex Normals mode:all");
        let m = a.editor.scene.active().unwrap().mesh.to_owned_value();
        assert!(operandi_core::vertex_normals::hard_edges(&m).is_empty());
        a.execute("Toggle Vertex Normal Maps mode:off");
        let m = a.editor.scene.active().unwrap().mesh.to_owned_value();
        assert!(m.attributes.corner_normals.is_empty());
        a.execute("Spin Polygon Vertex Indices offset:1");
        a.execute("Rename Normal Maps name:Custom");
        let undo = a.editor.undo.len();
        assert!(undo >= 4, "each a step ({undo})");
    }

    /// VN-6: options save to their preference file and load back.
    #[test]
    fn options_save_and_load() {
        let mut a = cube_app();
        let mut o = a.normal_options();
        o.angle = 25.;
        o.set_full_weighting(true);
        a.set_normal_options(o.clone());
        let back =
            VertexNormalPreferences::load(&a.environment.paths().vertex_normal_preferences());
        assert_eq!(back.options, o);
        assert!(!back.options.area_weighting, "linked to Full Weighting");
    }
}
