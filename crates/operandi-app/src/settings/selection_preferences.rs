//! Selection preferences (spec preferences-2026-11-04/SELECTION.md): the
//! starting selection mode, rollovers, point size, click reach and lasso
//! rules. Saved per user in `selection-preferences.json`. Values the spec
//! has not measured yet are marked *provisional* where they are defined.
use crate::prefs_file::PrefsFile;
use eframe::egui;
use operandi_core::Mode;
use serde::{Deserialize, Serialize};

/// Pre-highlight under the pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rollovers {
    None,
    /// Elements of the current mode (Operandi's behaviour so far, so the
    /// provisional default).
    #[default]
    Manual,
    /// The nearest element of any kind (point, edge or polygon) in the
    /// component modes; pairs with Mouse Regions Trigger Selections.
    Closest,
}
impl Rollovers {
    pub const ALL: [Self; 3] = [Self::None, Self::Manual, Self::Closest];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Manual => "Manual",
            Self::Closest => "Closest",
        }
    }
}

/// Which polygons a lasso can pick (the reference's Polygon Selection).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolygonSelection {
    /// Front faces only, except in wireframe views (provisional default,
    /// today's behaviour; the middle-button lasso still reverses it).
    #[default]
    FrontfaceAndWireframe,
    FrontfaceAndBackface,
    FrontfaceOnly,
    AlwaysRaycast,
    NoRaycast,
}
impl PolygonSelection {
    pub const ALL: [Self; 5] = [
        Self::FrontfaceAndWireframe,
        Self::FrontfaceAndBackface,
        Self::FrontfaceOnly,
        Self::AlwaysRaycast,
        Self::NoRaycast,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::FrontfaceAndWireframe => "Frontface & Wireframe",
            Self::FrontfaceAndBackface => "Frontface & Backface",
            Self::FrontfaceOnly => "Frontface only",
            Self::AlwaysRaycast => "Always Raycast",
            Self::NoRaycast => "No Raycast",
        }
    }
    /// Whether a lasso skips elements hidden behind a surface. `wireframe`
    /// is the view's style (already false when the lasso ignores shading);
    /// `middle` is the through-lasso button, which only reverses the
    /// default mode. Provisional (SEL-P-Q4): Frontface only and Always
    /// Raycast behave alike here, as do Frontface & Backface and No Raycast.
    pub fn lasso_culls(self, wireframe: bool, middle: bool) -> bool {
        match self {
            Self::FrontfaceAndWireframe => middle == wireframe,
            Self::FrontfaceOnly | Self::AlwaysRaycast => true,
            Self::FrontfaceAndBackface | Self::NoRaycast => false,
        }
    }
}

pub const POINT_SIZE_RANGE: std::ops::RangeInclusive<u32> = 1..=20;
pub const SELECTED_SCALE_RANGE: std::ops::RangeInclusive<f32> = 1.0..=5.0;
pub const HIT_SIZE_RANGE: std::ops::RangeInclusive<u32> = 1..=100;
pub const POINT_PRIORITY_RANGE: std::ops::RangeInclusive<u32> = 0..=50;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SelectionPreferences {
    version: u8,
    /// Provisional: Polygon, the mode a new scene already starts in.
    pub initial_mode: Mode,
    pub rollovers: Rollovers,
    /// Measured: 3. Provisional (SEL-P-Q4): 3 draws today's 2.5-point dot
    /// radius; the radius scales with the size.
    pub point_size: u32,
    /// Provisional: 1.6, today's selected/plain ratio.
    pub selected_point_scale: f32,
    /// Provisional: 12 points, today's reach.
    pub hit_size: u32,
    pub polygon_selection: PolygonSelection,
    /// Provisional: off.
    pub lasso_ignore_shading: bool,
    /// Starting value of the lasso's Partial toggle. Provisional: off.
    pub lasso_partial: bool,
    /// Point Selection Priority, in points: where a pick weighs points and
    /// edges together (Tweak, Closest rollovers), a point wins over an edge
    /// that is up to this much nearer the pointer. Provisional (SEL-P-Q2):
    /// 12 px, the default hit size, so a point in reach always wins as
    /// before.
    pub point_priority: u32,
    /// Mouse Regions Trigger Selections: right-click an element to switch to
    /// its component mode (and pick it). Provisional default off.
    pub mouse_regions: bool,
    /// Transparency: Selection, as percent opacity of the selection where a
    /// surface hides it (x-ray). Provisional: 0, hidden as before.
    pub selection_xray: u8,
    /// Pick Other Meshes in Component Modes: a Vertex, Edge or Polygon
    /// click on another mesh makes it the active mesh. Off by default
    /// (Jason, 2026-11-08): picking sees only the active mesh, and other
    /// meshes neither pick nor hide it. Files from before this option load
    /// it as off.
    pub pick_other_meshes: bool,
    /// Lazy Selection (SL-70, `select.lazyState`): a click picks the
    /// nearest point or edge within three times the usual reach (SL-Q5,
    /// provisional). Off by default.
    pub lazy_selection: bool,
    /// Shortest Path Mode (SL-03): Shift+click adds the element and the
    /// path to it from the last selected one. Off by default.
    pub shortest_path_mode: bool,
}
impl Default for SelectionPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            initial_mode: Mode::Polygon,
            rollovers: Rollovers::Manual,
            point_size: 3,
            selected_point_scale: 1.6,
            hit_size: 12,
            polygon_selection: PolygonSelection::default(),
            lasso_ignore_shading: false,
            lasso_partial: false,
            point_priority: 12,
            mouse_regions: false,
            selection_xray: 0,
            pick_other_meshes: false,
            lazy_selection: false,
            shortest_path_mode: false,
        }
    }
}
impl SelectionPreferences {
    /// The x-ray opacity as a 0–1 factor.
    pub fn xray_opacity(&self) -> f32 {
        f32::from(self.selection_xray.min(100)) / 100.
    }
    /// Drawn radius of a plain point, in points.
    pub fn point_radius(&self) -> f32 {
        self.point_size as f32 * 2.5 / 3.
    }
    /// Drawn radius of a selected point.
    pub fn selected_point_radius(&self) -> f32 {
        self.point_radius() * self.selected_point_scale
    }
    /// How far from a point or edge a click still picks it: the hit size,
    /// or the drawn point if that is bigger.
    pub fn pick_radius(&self) -> f32 {
        let reach = (self.hit_size as f32).max(self.selected_point_radius());
        if self.lazy_selection {
            reach * 3.
        } else {
            reach
        }
    }
}
impl crate::prefs_file::PrefsFile for SelectionPreferences {
    const INVALID: &'static str = "Invalid selection preference";
    fn valid(&self) -> bool {
        self.version == 1
            && POINT_SIZE_RANGE.contains(&self.point_size)
            && SELECTED_SCALE_RANGE.contains(&self.selected_point_scale)
            && HIT_SIZE_RANGE.contains(&self.hit_size)
            && POINT_PRIORITY_RANGE.contains(&self.point_priority)
            && self.selection_xray <= 100
    }
}

fn mode_label(mode: Mode) -> &'static str {
    match mode {
        Mode::Vertex => "Vertices",
        Mode::Edge => "Edges",
        Mode::Polygon => "Polygons",
        Mode::Item => "Items",
    }
}

impl super::Operandi {
    /// Preferences > Input > Selection (with the selection rows from
    /// Defaults > Application and Display > OpenGL).
    pub(super) fn selection_page(&mut self, ui: &mut egui::Ui) {
        let before = self.selection_preferences;
        let partial_before = self.lasso_partial;
        let s = &mut self.selection_preferences;
        let d = SelectionPreferences::default();
        let partial = &mut self.lasso_partial;
        use crate::form::{Unit, rows};
        rows::section(ui, "Selection", false, |ui| {
            let modes: Vec<_> = [Mode::Vertex, Mode::Edge, Mode::Polygon, Mode::Item]
                .into_iter()
                .map(|m| (m, mode_label(m)))
                .collect();
            rows::choice_d(
                ui,
                "Initial Selection Type",
                &mut s.initial_mode,
                &d.initial_mode,
                &modes,
                "The selection mode a new session starts in",
            );
            let rollovers: Vec<_> = Rollovers::ALL.iter().map(|&r| (r, r.label())).collect();
            rows::choice_d(
                ui,
                "Selection Rollovers",
                &mut s.rollovers,
                &d.rollovers,
                &rollovers,
                "What the pointer pre-highlights before a click",
            );
            rows::check_d(
                ui,
                "Mouse Regions Trigger Selections",
                &mut s.mouse_regions,
                d.mouse_regions,
                "Right-click a point, edge or polygon to switch to its mode",
            );
            rows::check_d(
                ui,
                "Pick Other Meshes in Component Modes",
                &mut s.pick_other_meshes,
                d.pick_other_meshes,
                "In Vertex, Edge and Polygon modes, a click on another mesh makes it \
                 active. Off: clicks pick only the active mesh, through other meshes",
            );
            rows::number_d(
                ui,
                "OpenGL Point Size",
                &mut s.point_size,
                d.point_size,
                Unit::Suffix(" px"),
                POINT_SIZE_RANGE,
                "How large points draw in the views",
            );
            rows::number_d(
                ui,
                "Selected Point Size",
                &mut s.selected_point_scale,
                d.selected_point_scale,
                Unit::Suffix("×"),
                SELECTED_SCALE_RANGE,
                "How much larger selected points draw",
            );
            rows::number_d(
                ui,
                "Selection Hit Size",
                &mut s.hit_size,
                d.hit_size,
                Unit::Suffix(" px"),
                HIT_SIZE_RANGE,
                "How near the pointer must be to pick a point or edge",
            );
            let polygon: Vec<_> = PolygonSelection::ALL
                .iter()
                .map(|&m| (m, m.label()))
                .collect();
            rows::choice_d(
                ui,
                "Polygon Selection",
                &mut s.polygon_selection,
                &d.polygon_selection,
                &polygon,
                "Which polygons the lasso takes",
            );
            rows::number_d(
                ui,
                "Point Selection Priority",
                &mut s.point_priority,
                d.point_priority,
                Unit::Suffix(" px"),
                POINT_PRIORITY_RANGE,
                "Where points and edges are picked together, a point wins over an \
                 edge up to this much nearer",
            );
            rows::number_d(
                ui,
                "Transparency: Selection",
                &mut s.selection_xray,
                d.selection_xray,
                Unit::Percent,
                0..=100,
                "How much of the selection shows where a surface hides it",
            );
            rows::check_d(
                ui,
                "Lasso: Ignore Shading Style",
                &mut s.lasso_ignore_shading,
                d.lasso_ignore_shading,
                "The lasso picks the same elements whatever the view's shading",
            );
            rows::check_d(
                ui,
                "Lasso: Allow Partial Element Selection",
                partial,
                d.lasso_partial,
                "The lasso takes edges and polygons it only partly covers",
            );
        });
        rows::note(
            ui,
            "Polygon Selection applies to the lasso. Not built yet: Double-Sided \
             Selection (materials have no Double Sided switch).",
        );
        self.selection_preferences.lasso_partial = self.lasso_partial;
        if (self.selection_preferences != before || self.lasso_partial != partial_before)
            && let Err(error) = self
                .selection_preferences
                .save(&self.environment.paths().selection_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
    /// Drawn radii of a point and a selected point in this view: those of
    /// Preferences' Selection page, or the view's own Point Size (Active
    /// Meshes, Independent Point Size).
    pub(super) fn point_radii(&self) -> [f32; 2] {
        let d = &self.display.settings.drawing;
        let p = &self.selection_preferences;
        if !d.independent_point_size {
            return [p.point_radius(), p.selected_point_radius()];
        }
        let r = d.point_size.clamp(1., 20.) * 2.5 / 3.;
        [r, r * p.selected_point_scale]
    }
    pub(super) fn rollovers_on(&self) -> bool {
        self.selection_preferences.rollovers != Rollovers::None
            && self.display.settings.drawing.shows_rollovers()
            && !self.brush_hides_selection()
            // With a sculpt brush a press sculpts; nothing pre-highlights
            // (and a stroke must not re-pick the changing mesh every frame).
            && self.sculpt.brush.is_none()
    }
    /// Show Selections as the views draw it now: the selected tint,
    /// outline, edges and points. Off while a brush hides it.
    pub(super) fn shows_selections(&self) -> bool {
        self.display.settings.drawing.selections && !self.brush_hides_selection()
    }
    /// Painting builds: whether element highlighting is hidden now (the
    /// rule is [`brush_hides_selection`]). The wire and the brush ring
    /// always draw.
    #[cfg(feature = "painting")]
    pub(super) fn brush_hides_selection(&self) -> bool {
        brush_hides_selection(BrushInUse {
            paint_tool: self.painting.brush.tool.is_some(),
            limited_by_selection: self.sculpt.brush.is_some(),
            other_task: self.editor.session.has_active_task(),
        })
    }
    #[cfg(not(feature = "painting"))]
    pub(super) fn brush_hides_selection(&self) -> bool {
        false
    }
    /// Painting builds: Texture Locators don't draw while a paint tool or
    /// sculpt brush is the tool in use, unless Show Texture Locators while
    /// Painting is on.
    pub(super) fn brush_hides_locators(&self) -> bool {
        #[cfg(feature = "painting")]
        return (self.painting.brush.tool.is_some() || self.sculpt.brush.is_some())
            && !self.editor.session.has_active_task()
            && !self.painting.prefs.show_locators;
        #[cfg(not(feature = "painting"))]
        false
    }
    /// What the pointer at `p` pre-highlights, if anything.
    #[cfg(test)]
    pub(super) fn rollover_at(
        &self,
        p: eframe::egui::Pos2,
        r: eframe::egui::Rect,
    ) -> Option<(u64, operandi_core::Component)> {
        self.rollovers_on()
            .then(|| self.pick_component(p, r))
            .flatten()
    }
    /// The reach of a click on a point or edge.
    pub(super) fn pick_radius(&self) -> f32 {
        self.selection_preferences.pick_radius()
    }
    /// Apply what only takes effect at launch.
    pub(super) fn apply_launch_selection(&mut self) {
        let mode = self.selection_preferences.initial_mode;
        self.lasso_partial = self.selection_preferences.lasso_partial;
        if self.path.is_none() && self.editor.scene.selection.mode != mode {
            self.editor.scene.selection.mode = mode;
            self.saved.selection.mode = mode;
        }
    }
}

/// The brush tools on now, for [`brush_hides_selection`].
#[cfg_attr(not(feature = "painting"), allow(dead_code))]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BrushInUse {
    /// A paint tool (Paintbrush, Airbrush, …) is on.
    pub paint_tool: bool,
    /// A brush whose reach the component selection limits is on: every
    /// sculpt brush, the mask brush too (SCS-01..09).
    pub limited_by_selection: bool,
    /// Another tool is running on top of the brush.
    pub other_task: bool,
}

/// Whether element highlighting (the selected tint, outline, edges and
/// points) is hidden. A paint tool hides it so it doesn't cover what is
/// being painted (Jason, 2026-11-18). A brush the selection limits keeps it
/// drawn the whole time, so the area the brush can reach stays visible
/// (Jason, 2026-11-23); the pre-highlight stays off for it either way
/// (`rollovers_on`). Any other tool running brings the highlight back.
/// Display only: the selection itself never changes (UIC-243).
#[cfg_attr(not(feature = "painting"), allow(dead_code))]
pub(crate) fn brush_hides_selection(brush: BrushInUse) -> bool {
    brush.paint_tool && !brush.limited_by_selection && !brush.other_task
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn only_a_brush_the_selection_does_not_limit_hides_it() {
        let hides = |paint_tool, limited_by_selection, other_task| {
            brush_hides_selection(BrushInUse {
                paint_tool,
                limited_by_selection,
                other_task,
            })
        };
        assert!(!hides(false, false, false), "no brush: drawn");
        assert!(hides(true, false, false), "paint tool: hidden");
        assert!(!hides(false, true, false), "sculpt brush: drawn");
        assert!(!hides(true, true, false), "sculpt brush wins: drawn");
        assert!(!hides(true, false, true), "another tool on top: drawn");
    }

    #[test]
    fn defaults_keep_todays_look_and_reach() {
        let d = SelectionPreferences::default();
        assert_eq!(d.point_size, 3);
        assert_eq!(d.point_radius(), 2.5);
        assert!((d.selected_point_radius() - 4.).abs() < 1e-6);
        assert_eq!(d.pick_radius(), 12.);
        assert_eq!(d.initial_mode, Mode::Polygon);
        // A big point is clickable across its whole drawn size.
        let big = SelectionPreferences {
            point_size: 20,
            hit_size: 2,
            ..d
        };
        assert!(big.pick_radius() > 20.);
    }

    #[test]
    fn polygon_selection_modes_decide_lasso_culling() {
        use PolygonSelection::*;
        // Default: solid views cull, wireframe doesn't, middle reverses.
        assert!(FrontfaceAndWireframe.lasso_culls(false, false));
        assert!(!FrontfaceAndWireframe.lasso_culls(true, false));
        assert!(!FrontfaceAndWireframe.lasso_culls(false, true));
        for (wire, middle) in [(false, false), (true, false), (true, true)] {
            assert!(FrontfaceOnly.lasso_culls(wire, middle));
            assert!(AlwaysRaycast.lasso_culls(wire, middle));
            assert!(!FrontfaceAndBackface.lasso_culls(wire, middle));
            assert!(!NoRaycast.lasso_culls(wire, middle));
        }
    }

    #[test]
    fn preferences_roundtrip_and_reject_bad_values() {
        let dir = std::env::temp_dir().join(format!(
            "operandi-selection-prefs-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("selection-preferences.json");
        assert_eq!(
            SelectionPreferences::load(&path),
            SelectionPreferences::default()
        );
        let p = SelectionPreferences {
            initial_mode: Mode::Edge,
            rollovers: Rollovers::None,
            point_size: 6,
            polygon_selection: PolygonSelection::NoRaycast,
            lasso_partial: true,
            ..Default::default()
        };
        p.save(&path).unwrap();
        assert_eq!(SelectionPreferences::load(&path), p);
        assert!(
            SelectionPreferences { point_size: 0, ..p }
                .save(&path)
                .is_err()
        );
        assert_eq!(SelectionPreferences::load(&path), p);
        fs::remove_dir_all(dir).unwrap();
    }

    // App routes.
    use crate::tests::draw;
    use crate::{Command, Operandi};
    use eframe::egui::{Context, Event, vec2};

    #[test]
    fn initial_selection_type_and_partial_apply_at_launch_without_dirtying() {
        let mut a = Operandi::empty();
        a.selection_preferences.initial_mode = Mode::Vertex;
        a.selection_preferences.lasso_partial = true;
        a.apply_launch_selection();
        assert_eq!(a.editor.scene.selection.mode, Mode::Vertex);
        assert!(a.lasso_partial);
        assert!(!a.dirty());
    }

    #[test]
    fn rollovers_none_draws_no_pre_highlight() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
                .unwrap()
                .into();
        a.run(Command::Mode(Mode::Vertex));
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        // Hover a cube corner.
        let corner = a
            .camera
            .project(glam::DVec3::splat(0.5), a.viewport_rect)
            .unwrap()
            .0;
        draw(
            &mut a,
            &ctx,
            vec2(1440., 900.),
            vec![Event::PointerMoved(corner)],
        );
        let r = a.viewport_rect;
        assert!(a.rollover_at(corner, r).is_some());
        a.selection_preferences.rollovers = Rollovers::None;
        assert!(a.rollover_at(corner, r).is_none());
    }

    #[test]
    fn hit_size_sets_how_far_a_click_reaches() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
                .unwrap()
                .into();
        a.run(Command::Mode(Mode::Vertex));
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let r = a.viewport_rect;
        let corner = a.camera.project(glam::DVec3::splat(0.5), r).unwrap().0;
        let near = corner + vec2(8., 0.);
        assert!(a.pick_component(near, r).is_some(), "8 px is within 12");
        a.selection_preferences.hit_size = 4;
        assert!(a.pick_component(near, r).is_none(), "8 px is outside 4");
    }

    fn cube_in(mode: Mode) -> (Operandi, Context) {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
                .unwrap()
                .into();
        a.run(Command::Mode(mode));
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        (a, ctx)
    }
    fn screen(a: &Operandi, p: glam::DVec3) -> eframe::egui::Pos2 {
        a.camera.project(p, a.viewport_rect).unwrap().0
    }

    #[test]
    fn closest_rollovers_pick_any_element_kind_in_component_modes() {
        use operandi_core::Component;
        let (mut a, _) = cube_in(Mode::Vertex);
        let r = a.viewport_rect;
        let corner = glam::DVec3::splat(0.5);
        let edge_mid = glam::DVec3::new(0.5, 0.5, 0.);
        let face_mid = glam::DVec3::new(0., 0.5, 0.);
        // Manual: Vertex mode only ever offers points.
        assert!(a.rollover_at(screen(&a, edge_mid), r).is_none());
        a.selection_preferences.rollovers = Rollovers::Closest;
        assert!(a.closest_rollovers());
        let hit = |a: &Operandi, p| a.pick_closest(screen(a, p), r).map(|h| h.1);
        assert!(matches!(hit(&a, corner), Some(Component::Vertex(_))));
        assert!(matches!(hit(&a, edge_mid), Some(Component::Edge(..))));
        assert!(matches!(hit(&a, face_mid), Some(Component::Polygon(_))));
        // Item mode keeps item rollovers.
        a.editor.scene.selection.mode = Mode::Item;
        assert!(!a.closest_rollovers());
    }

    #[test]
    fn point_priority_decides_between_a_point_and_a_nearer_edge() {
        use operandi_core::{Component, Tool};
        let (mut a, _) = cube_in(Mode::Vertex);
        a.selection_preferences.rollovers = Rollovers::Closest;
        let r = a.viewport_rect;
        // 9 px from a corner along its edge: the edge is under the pointer.
        let corner = screen(&a, glam::DVec3::splat(0.5));
        let along = screen(&a, glam::DVec3::new(0.5, 0.5, 0.));
        let p = corner + (along - corner).normalized() * 9.;
        assert!(matches!(
            a.pick_closest(p, r),
            Some((_, Component::Vertex(_)))
        ));
        a.selection_preferences.point_priority = 0;
        assert!(matches!(
            a.pick_closest(p, r),
            Some((_, Component::Edge(..)))
        ));
        // Tweak weighs points and edges the same way.
        a.run(Command::Activate(Tool::Tweak));
        let edge = a.tweak_pick(p, r).unwrap();
        assert!(matches!(
            edge.grab.element,
            operandi_core::TweakElement::Edge(..)
        ));
        a.selection_preferences.point_priority = 12;
        let point = a.tweak_pick(p, r).unwrap();
        assert!(matches!(
            point.grab.element,
            operandi_core::TweakElement::Vertex(_)
        ));
    }

    fn right_click(a: &mut Operandi, ctx: &Context, p: eframe::egui::Pos2) {
        let press = |pressed| Event::PointerButton {
            pos: p,
            button: eframe::egui::PointerButton::Secondary,
            pressed,
            modifiers: eframe::egui::Modifiers::NONE,
        };
        for events in [
            vec![Event::PointerMoved(p)],
            vec![press(true)],
            vec![press(false)],
        ] {
            draw(a, ctx, vec2(1440., 900.), events);
        }
    }

    #[test]
    fn mouse_regions_right_click_switches_to_the_elements_mode() {
        let (mut a, ctx) = cube_in(Mode::Polygon);
        let edge_mid = screen(&a, glam::DVec3::new(0.5, 0.5, 0.));
        // Off: a right-click is the context menu's, the mode stays.
        right_click(&mut a, &ctx, edge_mid);
        assert_eq!(a.editor.scene.selection.mode, Mode::Polygon);
        let (mut a, ctx) = cube_in(Mode::Polygon);
        a.selection_preferences.mouse_regions = true;
        right_click(&mut a, &ctx, edge_mid);
        assert_eq!(a.editor.scene.selection.mode, Mode::Edge);
        assert_eq!(a.editor.scene.selection.edges.len(), 1);
        assert!(!a.selection_context_requested);
        // A corner switches to Vertex mode; undo goes back step by step.
        let corner = screen(&a, glam::DVec3::splat(0.5));
        right_click(&mut a, &ctx, corner);
        assert_eq!(a.editor.scene.selection.mode, Mode::Vertex);
        assert_eq!(a.editor.scene.selection.vertices.len(), 1);
        // From Item mode too.
        a.editor.scene.selection.mode = Mode::Item;
        let face_mid = screen(&a, glam::DVec3::new(0., 0.5, 0.));
        right_click(&mut a, &ctx, face_mid);
        assert_eq!(a.editor.scene.selection.mode, Mode::Polygon);
        assert_eq!(a.editor.scene.selection.faces.len(), 1);
    }

    #[test]
    fn selection_xray_and_new_fields_persist() {
        let d = SelectionPreferences::default();
        assert_eq!(d.xray_opacity(), 0.);
        assert_eq!(d.point_priority, 12);
        assert!(!d.mouse_regions);
        let dir = std::env::temp_dir().join(format!("operandi-sel-xray-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("selection-preferences.json");
        // A file from before these options loads with their defaults.
        fs::write(&path, br#"{"version":1,"point_size":5}"#).unwrap();
        let old = SelectionPreferences::load(&path);
        assert_eq!(old.point_size, 5);
        assert_eq!(old.point_priority, 12);
        let p = SelectionPreferences {
            rollovers: Rollovers::Closest,
            mouse_regions: true,
            point_priority: 3,
            selection_xray: 40,
            ..d
        };
        p.save(&path).unwrap();
        assert_eq!(SelectionPreferences::load(&path), p);
        assert!((p.xray_opacity() - 0.4).abs() < 1e-6);
        assert!(
            SelectionPreferences {
                selection_xray: 101,
                ..p
            }
            .save(&path)
            .is_err()
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hidden_handles_fade_by_the_handle_transparency() {
        use operandi_core::Tool;
        let (mut a, _) = cube_in(Mode::Polygon);
        a.run(Command::Activate(Tool::Move));
        let r = a.viewport_rect;
        // The whole cube moves about its centre, inside the cube: every
        // handle's middle is behind the near faces.
        let (_, origin, end, _) = a.handles(r).into_iter().find(|h| h.0 == 0).unwrap();
        assert_eq!(a.handle_fade(origin, end, r), 1., "default 100%: as before");
        a.handle_preferences.handle_xray = 30;
        assert!((a.handle_fade(origin, end, r) - 0.3).abs() < 1e-6);
    }
}
