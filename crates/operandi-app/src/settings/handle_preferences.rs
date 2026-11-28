//! Display > Tool Handles preferences (spec preferences-2026-11-04/
//! HANDLES-SNAPPING.md): handle size, line width and how far a click
//! reaches. Saved per user in `handle-preferences.json`. Every default is
//! provisional (HND-P-Q1) and keeps Operandi's earlier handles.
use crate::prefs_file::PrefsFile;
use eframe::egui;
use serde::{Deserialize, Serialize};

/// How much of the active tool's handles is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandleDetail {
    /// No handles: the tool is driven from its properties only.
    Invisible,
    /// Provisional: Operandi's tools have one level of handles, so Basic
    /// and Advanced show the same.
    #[default]
    Basic,
    Advanced,
}
impl HandleDetail {
    pub const ALL: [Self; 3] = [Self::Invisible, Self::Basic, Self::Advanced];
    pub fn label(self) -> &'static str {
        match self {
            Self::Invisible => "Invisible",
            Self::Basic => "Basic",
            Self::Advanced => "Advanced",
        }
    }
}

pub const LINE_WIDTH_RANGE: std::ops::RangeInclusive<f32> = 1.0..=10.0;
pub const HIT_RANGE: std::ops::RangeInclusive<f32> = 2.0..=60.0;
pub const SCALE_RANGE: std::ops::RangeInclusive<f32> = 0.25..=4.0;
pub const POINT_RANGE: std::ops::RangeInclusive<f32> = 2.0..=40.0;
pub const SMALL_SCALE_RANGE: std::ops::RangeInclusive<u32> = 10..=400;
pub const PLANE_RATIO_RANGE: std::ops::RangeInclusive<f32> = 0.25..=4.0;
pub const SNAP_ANGLE_RANGE: std::ops::RangeInclusive<f64> = 0.1..=180.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HandlePreferences {
    version: u8,
    /// Drawn width of handle lines, in points (today's 2.5): bolder than
    /// wireframe lines so handles read clearly over geometry.
    pub line_width: f32,
    /// Width of the band around a handle line that a click hits (today's 10).
    pub line_hit_width: f32,
    /// Size of the square around a handle point that a click hits (14).
    pub point_hit_size: f32,
    /// Handle Scale: Standard, as a multiple of today's on-screen size.
    pub scale: f32,
    pub selected: HandleDetail,
    /// Transparency: Handles, as percent opacity where a surface hides a
    /// handle. Provisional: 100, always fully drawn as before.
    pub handle_xray: u8,
    /// Large Points: drawn size of the tool's handle points (the squares
    /// on Scale, Box and side handles), px at Handle Scale 1. Today's 9.
    /// Their hit size is `point_hit_size`.
    pub large_points: f32,
    /// Small Points: drawn size of secondary handle points (falloff
    /// handles), px. Today's 12.
    pub small_points: f32,
    /// Small Points Hit Size: click reach of those, px across. Today's 16.
    pub small_point_hit: f32,
    /// Handle Scale: Small, percent of Standard, for secondary handles.
    /// Provisional: 100.
    pub small_scale: u32,
    /// Plane Handle Ratio: size of the two-axis (view-plane) handle against
    /// the axis handles' points. Provisional: 1.
    pub plane_ratio: f32,
    /// Unselected Tool Handles: secondary handles (falloff). Provisional:
    /// Basic, shown as before.
    pub unselected: HandleDetail,
    /// Rotation Snap Angle, degrees: Rotate handle steps while Ctrl is
    /// held. Provisional: 15.
    pub rotation_snap: f64,
}
impl Default for HandlePreferences {
    fn default() -> Self {
        Self {
            version: 1,
            line_width: 2.5,
            line_hit_width: 10.,
            point_hit_size: 14.,
            scale: 1.,
            selected: HandleDetail::Basic,
            handle_xray: 100,
            large_points: 9.,
            small_points: 12.,
            small_point_hit: 16.,
            small_scale: 100,
            plane_ratio: 1.,
            unselected: HandleDetail::Basic,
            rotation_snap: 15.,
        }
    }
}
impl HandlePreferences {
    /// Drawn size of a handle point, at Handle Scale.
    pub fn large_point_size(&self) -> f32 {
        self.large_points * self.scale
    }
    /// Drawn size of the view-plane handle.
    pub fn plane_point_size(&self) -> f32 {
        self.large_point_size() * self.plane_ratio
    }
    /// Drawn size of a secondary handle point, at Handle Scale: Small.
    pub fn small_point_size(&self) -> f32 {
        self.small_points * self.scale * self.small_scale as f32 / 100.
    }
    /// Whether the active tool's primary handles draw and take clicks.
    pub fn selected_shown(&self) -> bool {
        self.selected != HandleDetail::Invisible
    }
    /// Whether secondary handles draw and take clicks.
    pub fn unselected_shown(&self) -> bool {
        self.unselected != HandleDetail::Invisible
    }
    /// `degrees` rounded to whole steps of Rotation Snap Angle (HM-20/21,
    /// SN2-40): the one rounding every Ctrl-stepped or snapped angle uses.
    /// A step of zero or less leaves the angle as it is.
    pub fn round_to_snap_angle(&self, degrees: f64) -> f64 {
        let step = self.rotation_snap;
        if step > 0. && step.is_finite() {
            (degrees / step).round() * step
        } else {
            degrees
        }
    }
    pub fn xray_opacity(&self) -> f32 {
        f32::from(self.handle_xray.min(100)) / 100.
    }
}
impl crate::prefs_file::PrefsFile for HandlePreferences {
    const INVALID: &'static str = "Invalid tool handle preference";
    fn valid(&self) -> bool {
        self.version == 1
            && LINE_WIDTH_RANGE.contains(&self.line_width)
            && HIT_RANGE.contains(&self.line_hit_width)
            && HIT_RANGE.contains(&self.point_hit_size)
            && SCALE_RANGE.contains(&self.scale)
            && self.handle_xray <= 100
            && POINT_RANGE.contains(&self.large_points)
            && POINT_RANGE.contains(&self.small_points)
            && HIT_RANGE.contains(&self.small_point_hit)
            && SMALL_SCALE_RANGE.contains(&self.small_scale)
            && PLANE_RATIO_RANGE.contains(&self.plane_ratio)
            && SNAP_ANGLE_RANGE.contains(&self.rotation_snap)
    }
}

impl super::Operandi {
    /// Preferences > Display > Tool Handles.
    pub(super) fn tool_handles_page(&mut self, ui: &mut egui::Ui) {
        let before = self.handle_preferences;
        let h = &mut self.handle_preferences;
        let d = HandlePreferences::default();
        use crate::form::{Unit, rows};
        let details: Vec<_> = HandleDetail::ALL.iter().map(|&x| (x, x.label())).collect();
        rows::section(ui, "Tool Handles", false, |ui| {
            rows::number_d(
                ui,
                "Handle Line Width",
                &mut h.line_width,
                d.line_width,
                Unit::Suffix(" px"),
                LINE_WIDTH_RANGE,
                "How thick handle lines draw",
            );
            rows::number_d(
                ui,
                "Handle Line Hit Width",
                &mut h.line_hit_width,
                d.line_hit_width,
                Unit::Suffix(" px"),
                HIT_RANGE,
                "How near the pointer must be to grab a handle line",
            );
            rows::number_d(
                ui,
                "Large Points",
                &mut h.large_points,
                d.large_points,
                Unit::Suffix(" px"),
                POINT_RANGE,
                "How large the big handle points draw",
            );
            rows::number_d(
                ui,
                "Small Points",
                &mut h.small_points,
                d.small_points,
                Unit::Suffix(" px"),
                POINT_RANGE,
                "How large the small handle points draw",
            );
            rows::number_d(
                ui,
                "Small Points Hit Size",
                &mut h.small_point_hit,
                d.small_point_hit,
                Unit::Suffix(" px"),
                HIT_RANGE,
                "How near the pointer must be to grab a small point",
            );
            rows::number_d(
                ui,
                "Large Points Hit Size",
                &mut h.point_hit_size,
                d.point_hit_size,
                Unit::Suffix(" px"),
                HIT_RANGE,
                "How near the pointer must be to grab a large point",
            );
            rows::number_d(
                ui,
                "Handle Scale: Standard",
                &mut h.scale,
                d.scale,
                Unit::Suffix("×"),
                SCALE_RANGE,
                "How large tool handles draw",
            );
            rows::number_d(
                ui,
                "Handle Scale: Small",
                &mut h.small_scale,
                d.small_scale,
                Unit::Suffix("% of Standard"),
                SMALL_SCALE_RANGE,
                "How large small handles draw, against the standard size",
            );
            rows::number_d(
                ui,
                "Plane Handle Ratio",
                &mut h.plane_ratio,
                d.plane_ratio,
                Unit::Suffix("×"),
                PLANE_RATIO_RANGE,
                "How large the plane handles are against the axis handles",
            );
            rows::choice_d(
                ui,
                "Selected Tool Handles",
                &mut h.selected,
                &d.selected,
                &details,
                "How much of the active tool's handles draw",
            );
            rows::choice_d(
                ui,
                "Unselected Tool Handles",
                &mut h.unselected,
                &d.unselected,
                &details,
                "Secondary handles, such as falloff",
            );
            rows::number_d(
                ui,
                "Transparency: Handles",
                &mut h.handle_xray,
                d.handle_xray,
                Unit::Percent,
                0..=100,
                "How much of a handle shows where a surface hides it",
            );
            rows::number_d(
                ui,
                "Rotation Snap Angle",
                &mut h.rotation_snap,
                d.rotation_snap,
                Unit::Angle,
                SNAP_ANGLE_RANGE,
                "Rotate handle steps while Ctrl is held as the drag starts",
            );
        });
        let snapping = &mut self.snapping;
        let snap_before = snapping.clone();
        rows::section(ui, "Handle Snapping", false, |ui| snapping.range_rows(ui));
        if *snapping != snap_before {
            self.save_snapping();
        }
        rows::note(
            ui,
            "Not built: Auto-Haul Display and the - and = handle size keys. Basic and \
             Advanced draw alike (Operandi's tools have one level of handles).",
        );
        if self.handle_preferences != before
            && let Err(error) = self
                .handle_preferences
                .save(&self.environment.paths().handle_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::draw;
    use crate::{Command, Operandi};
    use eframe::egui::{Context, vec2};
    use operandi_core::Tool;
    use std::fs;

    fn moving() -> (Operandi, Context) {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
                .unwrap()
                .into();
        a.run(Command::Activate(Tool::Move));
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        (a, ctx)
    }

    #[test]
    fn handle_scale_lengthens_the_handles_on_screen() {
        let (mut a, _) = moving();
        // Seen from the front, X is side-on: arrows are drawn in
        // perspective, so only a side-on arrow is the full 72 px.
        a.camera.view = crate::camera::View::Front;
        let r = a.viewport_rect;
        let length = |a: &Operandi| {
            let (_, origin, end, _) = a.handles(r).into_iter().find(|h| h.0 == 0).unwrap();
            origin.distance(end)
        };
        let normal = length(&a);
        assert!((normal - 72.).abs() < 0.5);
        a.handle_preferences.scale = 2.;
        assert!((length(&a) - 144.).abs() < 1.);
    }

    #[test]
    fn invisible_handles_leave_nothing_to_grab() {
        let (mut a, _) = moving();
        let r = a.viewport_rect;
        assert!(!a.handles(r).is_empty());
        a.handle_preferences.selected = HandleDetail::Invisible;
        assert!(a.handles(r).is_empty());
    }

    #[test]
    fn hit_sizes_set_how_far_a_click_on_a_handle_reaches() {
        let (mut a, _) = moving();
        let r = a.viewport_rect;
        let (axis, origin, end, _) = a.handles(r).into_iter().find(|h| h.0 == 0).unwrap();
        let side = (end - origin).normalized().rot90();
        let beside = origin.lerp(end, 0.5) + side * 4.;
        assert_eq!(a.handle_hit(beside, r), Some(axis), "4 px is within 10/2");
        a.handle_preferences.line_hit_width = 4.;
        assert_ne!(a.handle_hit(beside, r), Some(axis), "4 px is outside 4/2");
    }

    #[test]
    fn preferences_roundtrip_and_reject_bad_values() {
        let dir = std::env::temp_dir().join(format!("operandi-handles-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("handle-preferences.json");
        assert_eq!(HandlePreferences::load(&path), HandlePreferences::default());
        let p = HandlePreferences {
            scale: 1.5,
            selected: HandleDetail::Invisible,
            ..Default::default()
        };
        p.save(&path).unwrap();
        assert_eq!(HandlePreferences::load(&path), p);
        assert!(HandlePreferences { scale: 9., ..p }.save(&path).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
