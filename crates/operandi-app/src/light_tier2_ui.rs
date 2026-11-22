//! Light properties Tier 2 in the app: Targeting rows (LP-04, shared with
//! the camera), the Physical Sun section (LP-SUN), the photometric light's
//! file rows (LP-PH) and the mesh light's Prototype rows (LP-MS), plus the
//! viewport shapes of the cylinder, mesh and photometric lights.
use super::camera_light_ui::{Mark, ring};
use super::*;
use crate::unavailable::Gate;
use operandi_core::camera_light::{LightEdit, LightKind, LightSettings};
use operandi_core::item_common::ItemCommonEdit;
use operandi_core::photometry::Photometry;
use operandi_core::physical_sun::{LOCATIONS, SunClamp, SunEdit};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    /// Parsed photometry files by path, for the viewport shape (LP-PH-05).
    /// A file that can't be read is remembered as `None`.
    static PHOTOMETRY: RefCell<HashMap<String, Option<Rc<Photometry>>>> =
        RefCell::new(HashMap::new());
}

/// The parsed file at `path`, read once.
pub(super) fn photometry(path: &str) -> Option<Rc<Photometry>> {
    if path.is_empty() {
        return None;
    }
    PHOTOMETRY.with(|cache| {
        cache
            .borrow_mut()
            .entry(path.to_string())
            .or_insert_with(|| {
                Photometry::load(std::path::Path::new(path))
                    .ok()
                    .map(Rc::new)
            })
            .clone()
    })
}

/// Forget a cached file (after Browse... loads it again).
pub(super) fn forget_photometry(path: &str) {
    PHOTOMETRY.with(|cache| cache.borrow_mut().remove(path));
}

/// Cylinder light (LP-CY): its tube along local X — an end circle at each
/// end and four lines along it.
pub(super) fn cylinder_marks(world: glam::DMat4, light: &LightSettings) -> Vec<Mark> {
    let (r, h) = (light.radius.max(1e-4), light.length / 2.0);
    let p = |x: f64, y: f64, z: f64| world.transform_point3(DVec3::new(x, y, z));
    // `ring` draws in local XY; turn it to the YZ plane at each end.
    let mut out = Vec::new();
    for x in [-h, h] {
        let m = world
            * glam::DMat4::from_translation(DVec3::new(x, 0.0, 0.0))
            * glam::DMat4::from_rotation_y(std::f64::consts::FRAC_PI_2);
        out.extend(ring(m, r, r, 0.0, 24, false));
    }
    for (y, z) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        out.push(Mark::Line([p(-h, y * r, z * r), p(h, y * r, z * r)]));
    }
    out
}

/// Photometric light (LP-PH-05): the file's light as a web, each direction
/// pushed out by its brightness (peak = Display Size metres), in the four
/// vertical planes 0°, 45°, 90° and 135° and the horizontal ring at 90°.
/// Without a file, a small star.
pub(super) fn photometric_marks(world: glam::DMat4, light: &LightSettings) -> Vec<Mark> {
    let size = light.stored.display_size.max(0.05);
    let Some(file) = photometry(&light.photometry_file).filter(|f| f.peak_candela() > 0.0) else {
        return star(world, 0.25 * size);
    };
    let peak = file.peak_candela();
    // Vertical 0° shines down the light's −Z.
    let dir = |v: f64, h: f64| {
        let (v, h) = (v.to_radians(), h.to_radians());
        DVec3::new(v.sin() * h.cos(), v.sin() * h.sin(), -v.cos())
    };
    let point =
        |v: f64, h: f64| world.transform_point3(dir(v, h) * (size * file.sample(v, h) / peak));
    let mut out = Vec::new();
    for h in [0.0, 45.0, 90.0, 135.0] {
        let pts: Vec<DVec3> = (0..=72)
            .map(|k| {
                // Down one side and back up the other.
                let t = k as f64 * 5.0;
                if t <= 180.0 {
                    point(t, h)
                } else {
                    point(360.0 - t, h + 180.0)
                }
            })
            .collect();
        out.extend(pts.windows(2).map(|w| Mark::Line([w[0], w[1]])));
    }
    let ring: Vec<DVec3> = (0..=72).map(|k| point(90.0, k as f64 * 5.0)).collect();
    out.extend(ring.windows(2).map(|w| Mark::Line([w[0], w[1]])));
    out
}

/// Mesh light (LP-MS): a small star at the light; the glow itself is the
/// prototype mesh.
pub(super) fn mesh_light_marks(world: glam::DMat4, light: &LightSettings) -> Vec<Mark> {
    star(world, 0.15 * light.stored.display_size.max(0.05))
}

fn star(world: glam::DMat4, h: f64) -> Vec<Mark> {
    let p = |v: DVec3| world.transform_point3(v);
    let d = h / 3f64.sqrt();
    let mut dirs = vec![DVec3::X * h, DVec3::Y * h, DVec3::Z * h];
    for (x, y) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        dirs.push(DVec3::new(x * d, y * d, d));
    }
    dirs.into_iter()
        .map(|v| Mark::Line([p(-v), p(v)]))
        .collect()
}

impl Operandi {
    /// LP-04 Targeting rows for `item` (lights of the kinds that have them,
    /// and cameras): Set Target (a list of items), Remove Target, Enable,
    /// Roll. Time Offset is parked (no animation); Set Focus Distance is
    /// not shown on lights. One undo step each.
    pub(super) fn targeting_rows(&mut self, ui: &mut Ui, item: &operandi_core::Item) {
        let id = item.id;
        let scene = &self.editor.scene;
        let target = item.common.target.clone();
        let name_of = |t: u64| {
            scene
                .item(t)
                .map_or("(missing)".to_string(), |i| i.name.clone())
        };
        let shown = target
            .as_ref()
            .map_or("(none)".to_string(), |t| name_of(t.item));
        let candidates: Vec<(u64, String)> = scene
            .target_candidates(id)
            .into_iter()
            .map(|t| (t, name_of(t)))
            .collect();
        let locked = item.is_locked();
        let run = |a: &mut Operandi, edit: ItemCommonEdit| {
            a.run_common_on(vec![id], edit);
        };
        ui.gated_ui(
            crate::unavailable::unless(!locked, crate::unavailable::LOCKED),
            |ui| {
                let picked = crate::option_row::dropdown(
                    ui,
                    &format!("set-target-{id}"),
                    "Set Target",
                    &shown,
                    |ui| {
                        let mut picked = None;
                        for (t, name) in &candidates {
                            let on = target.as_ref().is_some_and(|x| x.item == *t);
                            if ui.selectable_label(on, name).clicked() && !on {
                                picked = Some(*t);
                            }
                        }
                        picked
                    },
                )
                .inner;
                if let Some(t) = picked {
                    run(self, ItemCommonEdit::Target(Some(t)));
                }
                if let Some(t) = &target {
                    ui.horizontal(|ui| {
                        crate::option_row::label(ui, "");
                        if ui.button("Remove Target").clicked() {
                            run(self, ItemCommonEdit::Target(None));
                        }
                    });
                    let mut enable = t.enable;
                    if crate::option_row::check(ui, "Enable", &mut enable).changed() {
                        run(self, ItemCommonEdit::TargetEnable(enable));
                    }
                    let deg = 180. / std::f64::consts::PI;
                    self.locator_number(
                        ui,
                        id,
                        "target-roll",
                        "Roll",
                        t.roll,
                        -36000.0..=36000.0,
                        "°",
                        deg,
                        |v| Command::EditItemCommon {
                            items: vec![id],
                            edit: ItemCommonEdit::TargetRoll(v),
                        },
                    );
                }
            },
        );
    }

    /// LP-SUN-01..12 (no Gamma or Store Location, Tier 3) on a directional
    /// light. Rows below the checkbox grey while Physical Sun is off
    /// (LPQ-03) and keep their values.
    pub(super) fn physical_sun_rows(&mut self, ui: &mut Ui, id: u64, l: &LightSettings) {
        let sun = l.sun.clone();
        let edit = |e: SunEdit| Command::EditLight {
            item: id,
            edit: LightEdit::Sun(e),
        };
        let mut on = sun.enabled;
        self.probe_light_row(&edit(SunEdit::Enabled(on)));
        if crate::option_row::check(ui, "Physical Sun", &mut on)
            .on_hover_text(
                "Aim this light where the real sun is for the place, date and time below",
            )
            .changed()
        {
            self.run(edit(SunEdit::Enabled(on)));
        }
        ui.gated_ui(
            crate::unavailable::unless(sun.enabled, "Turn on Physical Sun first"),
            |ui| {
                self.locator_number(
                    ui,
                    id,
                    "sun-time",
                    "Time",
                    sun.time,
                    0.0..=23.99,
                    " h",
                    1.,
                    |v| edit(SunEdit::Time(v)),
                );
                let hours = sun.time.floor();
                ui.horizontal(|ui| {
                    crate::option_row::label(ui, "");
                    ui.weak(format!(
                        "{:02}:{:02}",
                        hours as u32,
                        ((sun.time - hours) * 60.0).round() as u32 % 60
                    ));
                });
                // Day: month and day of the month.
                const MONTHS: [&str; 12] = [
                    "January",
                    "February",
                    "March",
                    "April",
                    "May",
                    "June",
                    "July",
                    "August",
                    "September",
                    "October",
                    "November",
                    "December",
                ];
                let mut month = sun.month;
                let months: Vec<(u32, &str)> = (1..=12).zip(MONTHS).collect();
                if crate::option_row::choice(
                    ui,
                    &format!("sun-month-{id}"),
                    "Day",
                    &mut month,
                    &months,
                )
                .inner
                {
                    let days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month as usize - 1];
                    self.run(edit(SunEdit::Day(month, sun.day.min(days))));
                }
                self.locator_number(
                    ui,
                    id,
                    "sun-day",
                    "Day of Month",
                    f64::from(sun.day),
                    1.0..=31.0,
                    "",
                    1.,
                    |v| edit(SunEdit::Day(sun.month, v.round().clamp(1.0, 31.0) as u32)),
                );
                let mut dst = sun.daylight_saving;
                self.probe_light_row(&edit(SunEdit::DaylightSaving(dst)));
                if crate::option_row::check(ui, "Daylight Savings Time", &mut dst).changed() {
                    self.run(edit(SunEdit::DaylightSaving(dst)));
                }
                self.locator_number(
                    ui,
                    id,
                    "sun-haze",
                    "Haze",
                    sun.haze,
                    1.0..=10.0,
                    "",
                    1.,
                    |v| edit(SunEdit::Haze(v)),
                );
                let clamp = SunClamp::ALL.map(|c| (c, c.label()));
                let mut c = sun.clamp;
                crate::option_row::choice(
                    ui,
                    &format!("sun-clamp-{id}"),
                    "Clamp Intensity",
                    &mut c,
                    &clamp,
                )
                .response
                .on_hover_text(
                    "None: real (very bright) sunlight; Clamp: its rise and fall up to Radiant \
                 Exitance; Replace: Radiant Exitance",
                );
                if c != sun.clamp {
                    self.run(edit(SunEdit::Clamp(c)));
                }
                let deg = 180. / std::f64::consts::PI;
                self.locator_number(
                    ui,
                    id,
                    "sun-north",
                    "North Offset",
                    sun.north_offset,
                    -360.0..=360.0,
                    "°",
                    deg,
                    |v| edit(SunEdit::NorthOffset(v)),
                );
                let shown = sun.location().unwrap_or("(custom)");
                let picked = crate::option_row::dropdown(
                    ui,
                    &format!("sun-location-{id}"),
                    "Location",
                    shown,
                    |ui| {
                        let mut picked = None;
                        for (i, (name, ..)) in LOCATIONS.iter().enumerate() {
                            if ui
                                .selectable_label(Some(*name) == sun.location(), *name)
                                .clicked()
                            {
                                picked = Some(i);
                            }
                        }
                        picked
                    },
                )
                .inner;
                if let Some(i) = picked {
                    self.run(edit(SunEdit::Location(i)));
                }
                self.locator_number(
                    ui,
                    id,
                    "sun-lat",
                    "North Latitude",
                    sun.latitude,
                    -90.0..=90.0,
                    "°",
                    1.,
                    |v| edit(SunEdit::Latitude(v)),
                );
                self.locator_number(
                    ui,
                    id,
                    "sun-lon",
                    "East Longitude",
                    sun.longitude,
                    -180.0..=180.0,
                    "°",
                    1.,
                    |v| edit(SunEdit::Longitude(v)),
                );
                self.locator_number(
                    ui,
                    id,
                    "sun-tz",
                    "Time Zone",
                    sun.time_zone,
                    -14.0..=14.0,
                    " h",
                    1.,
                    |v| edit(SunEdit::TimeZone(v)),
                );
                if sun.enabled {
                    let p = sun.position();
                    ui.weak(format!(
                        "Sun: azimuth {:.1}°, elevation {:.1}°{}",
                        p.azimuth,
                        p.elevation,
                        if p.elevation < 0.0 {
                            " (below the horizon: no sunlight)"
                        } else {
                            ""
                        }
                    ));
                }
            },
        );
    }

    /// LP-MS-01/02: Prototype (mesh items, or none) and Use Prototype
    /// Transform.
    pub(super) fn mesh_light_rows(&mut self, ui: &mut Ui, id: u64, l: &LightSettings) {
        let meshes: Vec<(u64, String)> = self
            .editor
            .scene
            .items
            .iter()
            .filter(|i| i.is_mesh())
            .map(|i| (i.id, i.name.clone()))
            .collect();
        let shown = l.prototype.map_or("(none)".to_string(), |p| {
            meshes
                .iter()
                .find(|m| m.0 == p)
                .map_or("(missing)".to_string(), |m| m.1.clone())
        });
        let r = crate::option_row::dropdown(
            ui,
            &format!("mesh-light-prototype-{id}"),
            "Prototype",
            &shown,
            |ui| {
                let mut picked = None;
                if ui
                    .selectable_label(l.prototype.is_none(), "(none)")
                    .clicked()
                {
                    picked = Some(None);
                }
                for (m, name) in &meshes {
                    if ui.selectable_label(l.prototype == Some(*m), name).clicked() {
                        picked = Some(Some(*m));
                    }
                }
                picked
            },
        );
        r.response
            .on_hover_text("The mesh whose shape glows; it is hidden in renders");
        if let Some(prototype) = r.inner
            && prototype != l.prototype
        {
            self.run_light(Command::EditLight {
                item: id,
                edit: LightEdit::Prototype(prototype),
            });
        }
        let mut use_transform = l.use_prototype_transform;
        if crate::option_row::check(ui, "Use Prototype Transform", &mut use_transform).changed() {
            self.run_light(Command::EditLight {
                item: id,
                edit: LightEdit::UsePrototypeTransform(use_transform),
            });
        }
    }

    /// LP-PH-01: the Photometry File and Browse... (IES files).
    pub(super) fn photometry_rows(&mut self, ui: &mut Ui, id: u64, l: &LightSettings) {
        let shown = if l.photometry_file.is_empty() {
            "(none)".to_string()
        } else {
            std::path::Path::new(&l.photometry_file)
                .file_name()
                .map_or(l.photometry_file.clone(), |n| {
                    n.to_string_lossy().into_owned()
                })
        };
        ui.horizontal(|ui| {
            crate::option_row::label(ui, "Photometry File");
            ui.weak(shown).on_hover_text(&l.photometry_file);
        });
        ui.horizontal(|ui| {
            crate::option_row::label(ui, "");
            if ui
                .button("Browse...")
                .on_hover_text(
                    "Load an IES photometry file: sets Radiant Intensity, Width and Height",
                )
                .clicked()
            {
                let folder = std::path::Path::new(&l.photometry_file)
                    .parent()
                    .map(|p| p.to_path_buf())
                    .filter(|p| p.is_dir())
                    .or_else(crate::backdrop_ui::dirs_home)
                    .unwrap_or_default();
                if !self.choose(
                    crate::file_chooser::Choose::Photometry { folder },
                    crate::file_chooser::Purpose::Photometry(id),
                ) {
                    self.status = "No file panel is available to choose a photometry file".into();
                }
            }
        });
        if !l.photometry_file.is_empty() && photometry(&l.photometry_file).is_none() {
            ui.colored_label(theme::error(), "The file can't be read");
        }
    }

    /// A photometry file chosen for light `item`.
    pub(super) fn photometry_chosen(&mut self, item: u64, path: std::path::PathBuf) {
        let key = path.to_string_lossy().into_owned();
        forget_photometry(&key);
        if self.run_ok(Command::LoadPhotometry { item, path: key }) {
            self.status = "Photometry file loaded".into();
        }
    }
}

/// Which light kinds show the cone rows (Cone Angle, Soft Edge).
pub(super) fn has_cone(kind: LightKind) -> bool {
    matches!(kind, LightKind::Spot | LightKind::Photometric)
}
