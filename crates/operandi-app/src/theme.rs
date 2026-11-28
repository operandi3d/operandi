pub(crate) mod presets;

use eframe::egui::*;
// Classic Grey values (colors-2026-11-16 BEHAVIOR-COLORS-DEFAULTS v2,
// section 2). The registry substitutes Steel's palette roles for factory
// defaults. Draw with the live colour functions below.
pub const ORANGE: Color32 = Color32::from_rgb(0xF4, 0x9C, 0x1C);
pub const CYAN: Color32 = Color32::from_rgb(154, 217, 236);
pub const ERROR: Color32 = Color32::from_rgb(255, 164, 164);
pub const TEXT: Color32 = Color32::from_gray(0xDE);
pub const BORDER: Color32 = Color32::from_gray(0x30);
pub const CONTROL: Color32 = Color32::from_gray(0x4C);
pub const HOVER: Color32 = Color32::from_gray(0x55);
pub const PANEL: Color32 = Color32::from_gray(0x3E);
/// Dialogs, menus and popups: one look with the panels.
pub const WINDOW: Color32 = Color32::from_gray(0x3E);
/// Around the picture in the Render window (IV-06).
pub const IMAGE_VIEWER_BACKGROUND: Color32 = Color32::from_gray(30);
pub const FIELD: Color32 = Color32::from_gray(0x4C);
/// A button being pressed: lighter than hover, as in the reference style.
pub const PRESSED: Color32 = Color32::from_gray(0x5F);
/// Secondary panel text (hints, units): between text and disabled.
pub const WEAK_TEXT: Color32 = Color32::from_gray(0xB4);
pub const HOVER_OUTLINE: Color32 = Color32::from_gray(0x70);
pub const DISABLED_TEXT: Color32 = Color32::from_gray(0x88);
/// Text drawn on an orange (selection) fill.
pub const ON_ORANGE: Color32 = Color32::BLACK;
/// The strip behind a panel group's tabs, and inactive tabs on it.
pub const TAB_STRIP: Color32 = Color32::from_gray(0x2A);
/// Inactive tab names.
pub const TAB_TEXT: Color32 = Color32::from_gray(0xC8);
/// The inactive panel corner dot (panel_dot.rs).
pub const PANEL_DOT: Color32 = Color32::from_rgb(86, 88, 93);
// Live colours: the user's override from Preferences > Display > Colors, else
// the built-in default above. Draw with these, never with the constants.
pub fn orange() -> Color32 {
    color(ColorId::Accent)
}
pub fn cyan() -> Color32 {
    color(ColorId::FocusRing)
}
pub fn error() -> Color32 {
    color(ColorId::ErrorText)
}
pub fn text() -> Color32 {
    color(ColorId::Text)
}
pub fn border() -> Color32 {
    color(ColorId::Border)
}
pub fn control() -> Color32 {
    color(ColorId::Control)
}
pub fn hover() -> Color32 {
    color(ColorId::Hover)
}
pub fn panel() -> Color32 {
    color(ColorId::Panel)
}
pub fn window() -> Color32 {
    color(ColorId::Window)
}
pub fn field() -> Color32 {
    color(ColorId::Field)
}
pub fn pressed() -> Color32 {
    color(ColorId::Pressed)
}
pub fn weak_text() -> Color32 {
    color(ColorId::WeakText)
}
/// The channel status light at the default (a ring) and changed (a dot).
pub fn channel_default() -> Color32 {
    color(ColorId::ChannelDefault)
}
pub fn channel_changed() -> Color32 {
    color(ColorId::ChannelChanged)
}
pub fn hover_outline() -> Color32 {
    color(ColorId::HoverOutline)
}
pub fn disabled_text() -> Color32 {
    color(ColorId::DisabledText)
}
pub fn on_orange() -> Color32 {
    color(ColorId::OnAccent)
}
/// The active panel's corner dot (panel_dot.rs).
pub fn panel_dot_active() -> Color32 {
    color(ColorId::PanelDotActive)
}
/// Every other panel's corner dot.
pub fn panel_dot() -> Color32 {
    color(ColorId::PanelDot)
}
pub fn install(ctx: &Context) {
    // One application update per input frame: an egui layout retry must never
    // replay a document command or a captured pointer sequence.
    ctx.options_mut(|o| o.max_passes = std::num::NonZeroUsize::MIN);
    crate::typography::install(ctx);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = vec2(4., 4.);
    style.spacing.button_padding = vec2(6., 2.);
    style.spacing.interact_size = vec2(20., 22.);
    style.spacing.indent = 14.;
    // #68: dropdowns open to their full length (egui's 200 px default
    // scrolled a 10-entry list like the viewport's view menu); only genuinely
    // long lists (fonts, images) still scroll.
    style.spacing.combo_height = 480.;
    apply_visuals(&mut style.visuals);
    // Keep numeric/command monospace, small overlays and headings at their
    // established sizes. FONT-001 calibrates standard proportional chrome only.
    for (role, font) in &mut style.text_styles {
        font.size = match role {
            TextStyle::Body | TextStyle::Button => crate::typography::STANDARD_SIZE,
            _ => 11.,
        };
    }
    ctx.set_style(style);
    INSTALLED.with(|g| g.set(generation()));
}
/// Re-applies the window colours after a Preferences change. Cheap when
/// nothing changed, so the app calls it every frame.
pub fn refresh(ctx: &Context) {
    let applied = POPUP_APPLIED.with(std::cell::Cell::get);
    let wanted = POPUP_TRANSPARENCY.with(std::cell::Cell::get);
    // Our colour scheme, whatever the OS appearance: egui otherwise follows
    // the system's light or dark setting and draws an unthemed light style
    // in Light appearance (contributor report, 2026-11-19).
    if ctx.options(|o| o.theme_preference) != ThemePreference::Dark {
        ctx.set_theme(ThemePreference::Dark);
        INSTALLED.with(|g| g.set(u64::MAX));
    }
    if INSTALLED.with(|g| g.get()) != generation() || applied != wanted {
        ctx.all_styles_mut(|style| apply_visuals(&mut style.visuals));
        INSTALLED.with(|g| g.set(generation()));
    }
}
thread_local! {
    /// Popup Transparency (Preferences > Defaults > UI), percent.
    static POPUP_TRANSPARENCY: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
    static POPUP_APPLIED: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}
pub fn set_popup_transparency(percent: u8) {
    POPUP_TRANSPARENCY.with(|c| c.set(percent.min(90)));
}
/// The fill of popups, menus and popovers: the Window colour, as see-through
/// as Popup Transparency says. Dialogs stay opaque (`window()`).
pub fn popup_fill() -> Color32 {
    let t = POPUP_TRANSPARENCY.with(std::cell::Cell::get).min(90);
    window().gamma_multiply(1. - f32::from(t) / 100.)
}
fn apply_visuals(visuals: &mut Visuals) {
    *visuals = Visuals::dark();
    visuals.weak_text_color = Some(weak_text());
    visuals.override_text_color = Some(text());
    visuals.panel_fill = panel();
    // Menus and popups fill with window_fill; dialogs set their own.
    visuals.window_fill = popup_fill();
    POPUP_APPLIED.with(|c| c.set(POPUP_TRANSPARENCY.with(std::cell::Cell::get)));
    visuals.extreme_bg_color = field();
    visuals.selection.bg_fill = orange();
    visuals.selection.stroke = Stroke::new(1.0f32, on_orange());
    for widget in [&mut visuals.widgets.inactive, &mut visuals.widgets.open] {
        widget.bg_fill = control();
        widget.weak_bg_fill = control();
        widget.bg_stroke = Stroke::new(1.0f32, border());
        widget.fg_stroke = Stroke::new(1.0f32, text());
    }
    visuals.widgets.hovered.bg_fill = hover();
    visuals.widgets.hovered.weak_bg_fill = hover();
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0f32, text());
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0f32, hover_outline());
    visuals.widgets.active.bg_fill = pressed();
    visuals.widgets.active.fg_stroke = Stroke::new(1.0f32, text());
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0f32, text());
}
/// A tree list's eye (N16): open, or closed when hidden (N19), in the list
/// glyph colour; faded for an item hidden by a parent.
pub fn list_eye(ui: &Ui, r: Rect, visible: bool, faded: bool) {
    let tint = workspace::glyphs();
    let (tint, white) = if faded {
        (tint.gamma_multiply(0.4), Color32::from_white_alpha(100))
    } else {
        (tint, Color32::WHITE)
    };
    if visible {
        // Two tones (Jason, 2026-11-16): the white of the eye under a bold
        // dark band and pupil, so it reads on any grey.
        crate::icons::paint_in(ui, r, crate::icons::Icon::EyeMonoLight, white);
        crate::icons::paint_in(ui, r, crate::icons::Icon::EyeMono, tint);
    } else {
        // N19: hidden is a closed eye, not a slash.
        crate::icons::paint_in(ui, r, crate::icons::Icon::EyeClosedMono, tint);
    }
}

/// Shared toolbox chrome appearance targets, explicitly sRGB. Other panels keep
/// their prior styling; this scope does not change scene-panel behavior/colors.
pub mod chrome {
    use super::*;
    /// An "on" button or ticked box (accent fill), black text on it.
    pub const ACTIVE: Color32 = Color32::from_rgb(0xF8, 0x9A, 0x2B);
    pub const ACTIVE_TEXT: Color32 = Color32::BLACK;
    // The classic look (colors-2026-11-16 section 2a).
    pub const BACKGROUND: Color32 = Color32::from_gray(0x3E);
    /// Collapsible section header bars.
    pub const HEADER: Color32 = Color32::from_gray(0x34);
    pub const CONTROL: Color32 = Color32::from_gray(0x4C);
    pub const BORDER: Color32 = Color32::from_gray(0x30);
    pub const TEXT: Color32 = Color32::from_gray(0xDE);
    pub const HOVER: Color32 = Color32::from_gray(0x55);
    pub const PRESSED: Color32 = Color32::from_gray(0x5F);
    pub const DISABLED_TEXT: Color32 = Color32::from_gray(0x88);
    pub fn active() -> Color32 {
        color(ColorId::ToolboxActive)
    }
    pub fn active_text() -> Color32 {
        color(ColorId::ToolboxActiveText)
    }
    pub fn background() -> Color32 {
        color(ColorId::ToolboxBackground)
    }
    pub fn header() -> Color32 {
        color(ColorId::ToolboxHeader)
    }
    pub fn control() -> Color32 {
        color(ColorId::ToolboxControl)
    }
    pub fn border() -> Color32 {
        color(ColorId::ToolboxBorder)
    }
    pub fn text() -> Color32 {
        color(ColorId::ToolboxText)
    }
    pub fn hover() -> Color32 {
        color(ColorId::ToolboxHover)
    }
    pub fn pressed() -> Color32 {
        color(ColorId::ToolboxPressed)
    }
    pub fn disabled_text() -> Color32 {
        color(ColorId::ToolboxDisabledText)
    }
    pub fn apply(ui: &mut Ui) {
        let visuals = ui.visuals_mut();
        visuals.override_text_color = Some(text());
        visuals.weak_text_color = Some(text());
        visuals.panel_fill = background();
        for widget in [&mut visuals.widgets.inactive, &mut visuals.widgets.open] {
            widget.bg_fill = control();
            widget.weak_bg_fill = control();
            widget.bg_stroke = Stroke::new(1.0f32, border());
            widget.fg_stroke = Stroke::new(1.0f32, text());
        }
        visuals.widgets.hovered.bg_fill = hover();
        visuals.widgets.hovered.weak_bg_fill = hover();
        visuals.widgets.active.bg_fill = pressed();
        visuals.selection.bg_fill = active();
        visuals.selection.stroke = Stroke::new(1.0f32, active_text());
    }
    pub fn button_fill(response: &Response, active: bool) -> Color32 {
        if response.is_pointer_button_down_on() {
            pressed()
        } else if active {
            self::active()
        } else if response.hovered() {
            hover()
        } else {
            control()
        }
    }
    /// egui marks focused Space/Enter as clicks without consuming the key.
    /// Keep that activation from reaching the global Space drop/cycle action.
    pub fn own_activation(ui: &Ui, response: &Response) {
        if response.has_focus() {
            ui.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Space);
                i.consume_key(Modifiers::NONE, Key::Enter);
            });
        }
    }
    pub fn focus(ui: &Ui, rect: Rect, response: &Response) {
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect,
                2.,
                Stroke::new(1.0f32, super::cyan()),
                StrokeKind::Inside,
            );
        }
    }
    pub fn disclosure(ui: &mut Ui, id: &str, label: &str, open: bool) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 20.), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
        own_activation(ui, &response);
        ui.painter().rect_filled(rect, 3., header());
        crate::icons::disclosure(
            ui,
            Rect::from_center_size(rect.left_center() + vec2(7., 0.), vec2(12., 12.)),
            if open { 1. } else { 0. },
        );
        ui.painter().text(
            rect.left_center() + vec2(16., 0.),
            Align2::LEFT_CENTER,
            label,
            crate::typography::standard_font(),
            text(),
        );
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect,
                0.,
                Stroke::new(1.0f32, super::cyan()),
                StrokeKind::Inside,
            );
        }
        response.on_hover_text(format!(
            "{} {label} ({id})",
            if open { "Collapse" } else { "Expand" }
        ))
    }
}

/// Investigator-transcribed, two-decimal configuration values. They do not
/// establish reference encoding/lighting/opacity. Initial Operandi adapter uses
/// display-sRGB swatches (rounded to8-bit) with existing shading/compositing/depth.
/// This explicit provisional interpretation is independent from chrome tokens.
pub mod viewport {
    use super::*;
    pub const fn swatch(v: [f32; 3]) -> Color32 {
        Color32::from_rgb(
            (v[0] * 255. + 0.5) as u8,
            (v[1] * 255. + 0.5) as u8,
            (v[2] * 255. + 0.5) as u8,
        )
    }
    // Stock view schemes (colors-2026-11-16 section 3; COLORS.md): screen
    // colours, no gamma step. Perspective first, then Orthographic and UV.
    pub const BACKGROUND: Color32 = Color32::from_gray(0x38);
    pub const ORTHO_BACKGROUND: Color32 = Color32::from_gray(0x39);
    pub const ORTHO_GRID: Color32 = Color32::BLACK;
    pub const ORTHO_WORK_PLANE: Color32 = Color32::from_rgb(0x8F, 0x99, 0xA6);
    pub const UV_BACKGROUND: Color32 = Color32::from_gray(0x43);
    pub const UV_GRID: Color32 = Color32::BLACK;
    pub const UV_WIREFRAME: Color32 = Color32::from_rgb(0xB0, 0xB2, 0xB2);
    pub const UV_AXES: [Color32; 2] = [
        Color32::from_rgb(0xB2, 0x99, 0x1A),
        Color32::from_rgb(0x1A, 0x99, 0xB2),
    ];
    /// The thin edge round each floating view-header button; Operandi's
    /// own value. The buttons fill with the view's background colour at
    /// `HEADER_BUTTON_ALPHA` (nearly opaque: readable over geometry).
    pub const HEADER_BORDER: Color32 = Color32::from_gray(0x30);
    pub const HEADER_BUTTON: Color32 = Color32::from_gray(0x46);
    pub const HEADER_TEXT: Color32 = Color32::from_gray(0xBC);
    /// A lit header button: the orange accent (UIC-01; Jason 2026-11-16
    /// chose this over the reference's blue).
    pub const HEADER_ON: Color32 = ORANGE;
    pub const HEADER_BUTTON_ALPHA: u8 = 235;
    /// The top of a gradient background (Viewport Properties); Operandi's
    /// own value, a touch lighter than the background.
    pub const BACKGROUND_TOP: Color32 = Color32::from_rgb(86, 88, 93);
    pub const GRID: Color32 = Color32::from_gray(0x85);
    pub const WORK_PLANE: Color32 = Color32::from_rgb(0x5C, 0x8A, 0xED);
    pub const GRID_AXES: [Color32; 3] = [
        swatch([1., 0.58, 0.49]),
        swatch([0.58, 1., 0.67]),
        swatch([0.58, 0.67, 1.]),
    ];
    /// The two lines through world zero in the Top/Front/Side views: a
    /// near-black, clearly darker than the grey grid (Operandi's value).
    pub const ZERO_AXES: Color32 = Color32::from_rgb(22, 22, 24);
    pub const WIREFRAME: Color32 = Color32::from_gray(0xB8);
    /// The plain wire of inactive meshes (visible, not the selected item):
    /// pure black so the active mesh's wire stands out (Operandi's own
    /// value, Jason 2026-11-08).
    pub const INACTIVE_WIREFRAME: Color32 = Color32::from_rgb(0, 0, 0);
    /// The active meshes' plain wire over a filled display style (every
    /// style but Wireframe and Vertex Map): black, 1 px (VS-1).
    pub const SHADED_WIREFRAME: Color32 = Color32::from_rgb(0, 0, 0);
    pub const SOLID: Color32 = swatch([0.80, 0.80, 0.80]);
    pub const SELECTION: Color32 = swatch([1., 0.83, 0.44]);
    /// The mirror-side half of a Symmetry selection, a dimmer selection tone.
    pub const MIRROR_SELECTION: Color32 = swatch([0.80, 0.58, 0.34]);
    /// LCK-23/24: locked vertices, edges and polygons, a cool grey-blue
    /// apart from selection and wire (provisional value).
    pub const LOCKED: Color32 = swatch([0.45, 0.55, 0.72]);
    /// Tool handle parts not under the pointer, such as a curve tool's
    /// points other than the current one (colours spec row 21).
    pub const HANDLE: Color32 = swatch([0.40, 1.00, 1.00]);
    pub const HANDLES: [Color32; 3] = [
        swatch([0.96, 0.49, 0.49]),
        swatch([0.49, 0.91, 0.49]),
        swatch([0.49, 0.67, 1.]),
    ];
    pub const ROLLOVER: Color32 = swatch([0.77, 0.86, 0.90]);
    /// Falloff weight colours, weak to strong. Provisional (Operandi's own
    /// values): a cool blue rising to the selection orange.
    pub const FALLOFF_COOL: Color32 = Color32::from_rgb(70, 120, 235);
    pub const FALLOFF_WARM: Color32 = SELECTION;
    // Selected and pre-highlighted edges have no dark backing either (Jason
    // 2026-11-22: it read as a wireframe outline around the selection); the
    // colour now only backs tool-handle lines.
    pub const EDGE_BACKING: Color32 = Color32::from_rgb(41, 44, 49);
    pub const LABEL: Color32 = Color32::from_gray(210);
    /// Viewport display styles (view_display.rs); Operandi's own choices.
    /// Vertex Map style on a mesh without a vertex map.
    pub const VERTEX_MAP_NONE: Color32 = swatch([0.62, 0.62, 0.62]);
    /// The Colored (dark lines) overlay, a new view's wire: black (VS-6).
    pub const COLORED_WIRE: Color32 = Color32::from_rgb(0, 0, 0);
    /// UV seam edges.
    pub const UV_SEAM: Color32 = swatch([0.95, 0.45, 0.85]);
    /// The corners of an inactive mesh's bounding box, drawn when its
    /// polygons are too small on screen for their wire (Fade Wireframe on
    /// Small Polygons): blue-grey (Jason, 2026-11-13).
    pub const DENSE_MESH_BOX: Color32 = swatch([0.52, 0.6, 0.7]);
    /// Weighted (creased) edges in Edge mode, drawn brighter and wider as
    /// the weight rises (EW-08; Operandi's own colour).
    pub const EDGE_WEIGHT: Color32 = swatch([0.95, 0.30, 0.25]);
    /// The snapping target near the handle (SN-08; Operandi's own colour).
    pub const SNAP_TARGET: Color32 = swatch([1.0, 0.85, 0.2]);
    /// SN2-43: the pre-highlight marker before it snaps (provisional
    /// colour).
    pub const SNAP_UNSNAPPED: Color32 = swatch([0.55, 0.8, 1.0]);
    /// Slice tools (Operandi's own colours): Loop Slice's slider bar and
    /// Polygon Slice's line, the cut points and handles, and Edge Slice's
    /// first point when the pointer is on it (the loop would close).
    pub const SLICE_BAR: Color32 = swatch([0.40, 0.80, 0.95]);
    pub const SLICE_POINT: Color32 = swatch([1.0, 0.62, 0.22]);
    pub const SLICE_CLOSE: Color32 = swatch([0.45, 0.95, 0.45]);
    /// Outline and grid of a Backdrop that has no picture to show yet.
    pub const BACKDROP_PLACEHOLDER: Color32 = swatch([0.96, 0.56, 0.12]);
    /// Camera view: the colour of the darker bands outside the render
    /// frame (drawn at `CAMERA_FRAME_SHADE_ALPHA`, CL-19).
    pub const CAMERA_FRAME_SHADE: Color32 = Color32::BLACK;
    /// How opaque those bands are (110 of 255, about 43 %). Colour
    /// preferences store opaque colours, so the opacity stays fixed here.
    pub const CAMERA_FRAME_SHADE_ALPHA: u8 = 110;
    /// Camera/light handles (spec v4): circles light purple, grab crosses
    /// cyan.
    pub const LIGHT_HANDLE_CIRCLE: Color32 = swatch([0.78, 0.66, 1.]);
    pub const LIGHT_HANDLE_GRAB: Color32 = swatch([0.3, 0.9, 1.]);
    /// Camera and light glyphs: light grey when unselected (CL-13).
    pub const LOCATOR: Color32 = swatch([0.78, 0.78, 0.78]);
    /// Tweak's range sphere (TWK-14; the reference draws it yellow).
    pub const TWEAK_RANGE: Color32 = swatch([0.95, 0.82, 0.18]);
    /// Gooch tones (display_shading.rs): cool facing across the light,
    /// warm facing along it either way (VS-3).
    pub const GOOCH_COOL: Color32 = Color32::from_rgb(0x1F, 0x1F, 0x78);
    pub const GOOCH_WARM: Color32 = Color32::from_rgb(0xCC, 0xCC, 0x5C);
    pub fn gooch_cool() -> Color32 {
        color(ColorId::GoochCool)
    }
    pub fn gooch_warm() -> Color32 {
        color(ColorId::GoochWarm)
    }
    pub fn tweak_range() -> Color32 {
        TWEAK_RANGE
    }
    /// Which stock scheme the view being drawn uses (colors-2026-11-16
    /// section 3): Perspective (and camera views), Orthographic (Top,
    /// Front, Side ...) or UV. Each overrides a few base colours.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub enum ViewScheme {
        #[default]
        Perspective,
        Orthographic,
        Uv,
    }
    thread_local! {
        static SCHEME: std::cell::Cell<ViewScheme> =
            const { std::cell::Cell::new(ViewScheme::Perspective) };
    }
    /// Draw the following with `scheme`'s colours until the returned guard
    /// drops (then the previous scheme comes back).
    pub fn use_scheme(scheme: ViewScheme) -> SchemeGuard {
        SchemeGuard(SCHEME.with(|s| s.replace(scheme)))
    }
    pub struct SchemeGuard(ViewScheme);
    impl Drop for SchemeGuard {
        fn drop(&mut self) {
            SCHEME.with(|s| s.set(self.0));
        }
    }
    pub(super) fn scheme() -> ViewScheme {
        SCHEME.with(std::cell::Cell::get)
    }
    pub(super) fn set_scheme(scheme: ViewScheme) {
        SCHEME.with(|s| s.set(scheme));
    }
    pub fn background() -> Color32 {
        color(match scheme() {
            ViewScheme::Perspective => ColorId::ViewportBackground,
            ViewScheme::Orthographic => ColorId::OrthoBackground,
            ViewScheme::Uv => ColorId::UvBackground,
        })
    }
    /// The UV view's U and V axis lines.
    pub fn uv_axes() -> [Color32; 2] {
        [color(ColorId::UvAxisU), color(ColorId::UvAxisV)]
    }
    pub fn background_top() -> Color32 {
        color(ColorId::ViewportGradientTop)
    }
    /// The floating view header's button fill: the background colour,
    /// nearly opaque.
    /// The view header's buttons (colors-2026-11-16 section 2c), nearly
    /// opaque so they read over geometry.
    pub fn header_button() -> Color32 {
        let c = color(ColorId::ViewportHeaderButton);
        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), HEADER_BUTTON_ALPHA)
    }
    pub fn header_text() -> Color32 {
        color(ColorId::ViewportHeaderText)
    }
    /// An "on" header icon button (accent orange by default).
    pub fn header_on() -> Color32 {
        color(ColorId::ViewportHeaderOn)
    }
    /// The floating view header's button edge.
    pub fn header_border() -> Color32 {
        color(ColorId::ViewportHeaderBorder)
    }
    pub fn grid() -> Color32 {
        color(match scheme() {
            ViewScheme::Perspective => ColorId::Grid,
            ViewScheme::Orthographic => ColorId::OrthoGrid,
            ViewScheme::Uv => ColorId::UvGrid,
        })
    }
    pub fn work_plane() -> Color32 {
        color(match scheme() {
            ViewScheme::Orthographic => ColorId::OrthoWorkPlane,
            _ => ColorId::WorkPlane,
        })
    }
    pub fn grid_axes() -> [Color32; 3] {
        [ColorId::GridAxisX, ColorId::GridAxisY, ColorId::GridAxisZ].map(color)
    }
    pub fn zero_axes() -> Color32 {
        color(ColorId::ZeroAxes)
    }
    pub fn wireframe() -> Color32 {
        color(match scheme() {
            ViewScheme::Perspective => ColorId::Wireframe,
            ViewScheme::Orthographic => ColorId::OrthoWireframe,
            ViewScheme::Uv => ColorId::UvWireframe,
        })
    }
    pub fn inactive_wireframe() -> Color32 {
        color(ColorId::InactiveWireframe)
    }
    pub fn shaded_wireframe() -> Color32 {
        color(ColorId::ShadedWireframe)
    }
    pub fn solid() -> Color32 {
        color(ColorId::Solid)
    }
    pub fn selection() -> Color32 {
        color(ColorId::Selection)
    }
    pub fn mirror_selection() -> Color32 {
        color(ColorId::MirrorSelection)
    }
    /// LCK-24 "Locked mesh elements".
    pub fn locked() -> Color32 {
        color(ColorId::Locked)
    }
    pub fn handles() -> [Color32; 3] {
        [ColorId::HandleX, ColorId::HandleY, ColorId::HandleZ].map(color)
    }
    pub fn rollover() -> Color32 {
        color(ColorId::Rollover)
    }
    /// A whole item's rollover fill in Item mode (the Face rollover).
    pub fn rollover_face() -> Color32 {
        color(ColorId::RolloverFace)
    }
    /// The rollover of polygon `face`, by its kind: Face, Subdivision or
    /// Catmull-Clark (points and edges use the Generic rollover).
    pub fn polygon_rollover(mesh: &operandi_core::Mesh, face: usize) -> Color32 {
        use operandi_core::PolygonRepresentation as Kind;
        color(
            match mesh
                .attributes
                .representations
                .get(face)
                .copied()
                .unwrap_or_default()
            {
                Kind::Subdivs => ColorId::RolloverSubdivision,
                Kind::CatmullClark => ColorId::RolloverCatmullClark,
                _ => ColorId::RolloverFace,
            },
        )
    }
    /// The falloff weight shown at `weight` (0–1): from the Cool colour at
    /// the edge of the falloff to the Warm one at full strength.
    pub fn falloff_weight(weight: f32) -> Color32 {
        let (cool, warm) = (color(ColorId::FalloffCool), color(ColorId::FalloffWarm));
        let w = weight.clamp(0., 1.);
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * w).round() as u8;
        Color32::from_rgb(
            mix(cool.r(), warm.r()),
            mix(cool.g(), warm.g()),
            mix(cool.b(), warm.b()),
        )
    }
    /// Fill of selected polygons (Selection is their points and edges).
    pub fn selection_fill() -> Color32 {
        color(ColorId::SelectionFill)
    }
    /// The lasso / drag-select outline.
    pub fn lasso() -> Color32 {
        color(ColorId::Lasso)
    }
    /// A tool handle part that is not under the pointer.
    pub fn handle() -> Color32 {
        color(ColorId::Handle)
    }
    /// A tool handle under the pointer or being dragged.
    pub fn handle_active() -> Color32 {
        color(ColorId::HandleActive)
    }
    pub fn edge_backing() -> Color32 {
        color(ColorId::EdgeBacking)
    }
    pub fn label() -> Color32 {
        color(ColorId::ViewportLabel)
    }
    pub fn vertex_map_none() -> Color32 {
        color(ColorId::VertexMapNone)
    }
    pub fn colored_wire() -> Color32 {
        color(ColorId::ColoredWire)
    }
    pub fn uv_seam() -> Color32 {
        color(ColorId::UvSeam)
    }
    pub fn dense_mesh_box() -> Color32 {
        color(ColorId::DenseMeshBox)
    }
    pub fn edge_weight() -> Color32 {
        color(ColorId::EdgeWeight)
    }
    pub fn snap_target() -> Color32 {
        color(ColorId::SnapTarget)
    }
    pub fn snap_unsnapped() -> Color32 {
        color(ColorId::SnapUnsnapped)
    }
    pub fn slice_bar() -> Color32 {
        color(ColorId::SliceBar)
    }
    pub fn slice_point() -> Color32 {
        color(ColorId::SlicePoint)
    }
    pub fn slice_close() -> Color32 {
        color(ColorId::SliceClose)
    }
    pub fn backdrop_placeholder() -> Color32 {
        color(ColorId::BackdropPlaceholder)
    }
    /// The Camera view's darker bands outside the render frame, with their
    /// fixed opacity applied.
    pub fn camera_frame_shade() -> Color32 {
        let c = color(ColorId::CameraFrameShade);
        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), CAMERA_FRAME_SHADE_ALPHA)
    }
    pub fn light_handle_circle() -> Color32 {
        color(ColorId::LightHandleCircle)
    }
    pub fn light_handle_grab() -> Color32 {
        color(ColorId::LightHandleGrab)
    }
    /// Unselected camera and light shapes.
    pub fn locator() -> Color32 {
        color(ColorId::Locator)
    }
}

/// Scenes-list colours. Operandi dark theme, our own values chosen by eye
/// (2026-11-03). Header/selection role calibration remains separate (PAR-Q04).
pub mod workspace {
    use super::*;
    // Lists and trees (colors-2026-11-16 section 2b): mid-grey rows with
    // near-white names (6:1), a gentle stripe.
    pub const LIST: Color32 = Color32::from_gray(0x4C);
    /// Every other row, a touch lighter than LIST. Selection fills win.
    pub const LIST_STRIPE: Color32 = Color32::from_gray(0x50);
    pub const FIELD_TEXT: Color32 = Color32::from_gray(0xDE);
    /// The column header row matches the list (Jason, 2026-11-16: a darker
    /// header and state columns read as a near-black gutter); the column
    /// lines still mark it off.
    pub const HEADER: Color32 = LIST;
    pub const HEADER_TEXT: Color32 = Color32::from_gray(0xC8);
    /// The active tab joins the panel below it (same as the panel).
    pub const TAB: Color32 = PANEL;
    /// Selected rows: a darker band, names in accent (UIC-33). One colour
    /// everywhere (deviation D4).
    pub const SELECTED: Color32 = Color32::from_gray(0x3B);
    /// The primary selection a shade darker still (UIC-34).
    pub const PRIMARY: Color32 = Color32::from_gray(0x35);
    /// Hidden or empty items, secondary text (deliberately faint, 2.8:1).
    pub const INACTIVE: Color32 = Color32::from_gray(0x93);
    /// Vertical lines between columns: near-black (Jason, 2026-11-16).
    pub const SEPARATOR: Color32 = Color32::from_gray(0x1E);
    /// Disclosure triangles and eyes (Jason, 2026-11-16: black; the spec's
    /// arrow colour #1C1C1C).
    pub const GLYPHS: Color32 = Color32::from_gray(0x1C);
    /// Dotted tree connector lines (UIC-140).
    pub const TREE_LINES: Color32 = Color32::from_gray(0x6C);
    /// Tree-list drag and drop (UIC-143): the insert line between rows and
    /// the outline on a target row. Defaults to the accent orange it was
    /// drawn in before it had its own token.
    pub const DROP_INDICATOR: Color32 = ORANGE;
    /// A Group Locator's Items-row icon: a little blue folder (Jason,
    /// 2026-11-08). Bright enough to read on the selected rows' blue too.
    pub const GROUP_LOCATOR_ICON: Color32 = Color32::from_rgb(92, 158, 240);
    pub fn group_locator_icon() -> Color32 {
        color(ColorId::ItemsGroupLocatorIcon)
    }
    pub fn drop_indicator() -> Color32 {
        color(ColorId::ItemsDropIndicator)
    }
    pub fn list() -> Color32 {
        color(ColorId::ItemsList)
    }
    pub fn header_text() -> Color32 {
        color(ColorId::ItemsHeaderText)
    }
    pub fn selected_text() -> Color32 {
        color(ColorId::ItemsSelectedText)
    }
    /// The editable-mesh icon of the active mesh (N18).
    pub fn active_mesh() -> Color32 {
        color(ColorId::ItemsActiveMesh)
    }
    /// Disclosure triangles and eyes in tree lists (N16: black).
    pub fn glyphs() -> Color32 {
        color(ColorId::ItemsGlyphs)
    }
    pub fn tree_lines() -> Color32 {
        color(ColorId::ItemsTreeLines)
    }
    pub fn tab_strip() -> Color32 {
        color(ColorId::TabStrip)
    }
    pub fn tab_text() -> Color32 {
        color(ColorId::TabText)
    }
    pub fn list_stripe() -> Color32 {
        color(ColorId::ItemsListStripe)
    }
    pub fn field_text() -> Color32 {
        color(ColorId::ItemsText)
    }
    pub fn header() -> Color32 {
        color(ColorId::ItemsHeader)
    }
    pub fn tab() -> Color32 {
        color(ColorId::ItemsTab)
    }
    pub fn selected() -> Color32 {
        color(ColorId::ItemsSelected)
    }
    pub fn primary() -> Color32 {
        color(ColorId::ItemsPrimary)
    }
    pub fn inactive() -> Color32 {
        color(ColorId::ItemsInactive)
    }
    pub fn separator() -> Color32 {
        color(ColorId::ItemsSeparator)
    }
}

/// One user-editable colour: where Preferences shows it and its default.
pub struct ColorEntry {
    pub id: ColorId,
    /// Stable name in the preferences and scheme files. Never rename one.
    pub key: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    pub default: Color32,
    pub classic_default: Color32,
}

macro_rules! colors {
    ($($group:literal { $($id:ident $key:literal $label:literal = $default:expr;)* })*) => {
        /// Every user-editable colour, in Preferences order.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum ColorId { $($($id,)*)* }
        pub const COLORS: &[ColorEntry] = &[$($(ColorEntry {
            id: ColorId::$id,
            key: $key,
            group: $group,
            label: $label,
            default: match presets::steel_color(ColorId::$id) {
                Some(color) => color,
                None => $default,
            },
            classic_default: $default,
        },)*)*];
    };
}
// Order and section names follow the Display > Colors spec
// (specifications/preferences-colors-2026-11-04); our dark theme's values
// stay the defaults. Colours with no Operandi drawing yet are left out.
colors! {
    "Interface" {
        Panel "interface.panel" "Panel" = PANEL;
        Window "interface.window" "Window" = WINDOW;
        ImageViewerBackground "interface.image_viewer_background" "Render Window Background" = IMAGE_VIEWER_BACKGROUND;
        Field "interface.field" "Field" = FIELD;
        Control "interface.control" "Button" = CONTROL;
        Hover "interface.hover" "Button Hover" = HOVER;
        Pressed "interface.pressed" "Button Pressed" = PRESSED;
        Border "interface.border" "Border" = BORDER;
        HoverOutline "interface.hover_outline" "Hover Outline" = HOVER_OUTLINE;
        Accent "interface.accent" "Accent" = ORANGE;
        FocusRing "interface.focus_ring" "Keyboard Focus" = CYAN;
        PanelDotActive "interface.panel_dot_active" "Active Panel Dot" = ORANGE;
        PanelDot "interface.panel_dot" "Panel Dot" = PANEL_DOT;
        // Channel status light (UIC-112, CH-Q1): ring at the default,
        // filled accent dot once changed.
        ChannelDefault "interface.channel_default" "Channel Light (Default)" = WEAK_TEXT;
        ChannelChanged "interface.channel_changed" "Channel Light (Changed)" = ORANGE;
    }
    "Toolbox" {
        ToolboxBackground "toolbox.background" "Background" = chrome::BACKGROUND;
        ToolboxHeader "toolbox.header" "Section Header" = chrome::HEADER;
        ToolboxControl "toolbox.control" "Button" = chrome::CONTROL;
        ToolboxHover "toolbox.hover" "Button Hover" = chrome::HOVER;
        ToolboxPressed "toolbox.pressed" "Button Pressed" = chrome::PRESSED;
        ToolboxActive "toolbox.active" "Active Button" = chrome::ACTIVE;
        ToolboxBorder "toolbox.border" "Border" = chrome::BORDER;
    }
    "Items List" {
        ItemsList "items.list" "Background" = workspace::LIST;
        ItemsListStripe "items.list_stripe" "Alternate Row" = workspace::LIST_STRIPE;
        ItemsHeader "items.header" "Header" = workspace::HEADER;
        ItemsTab "items.tab" "Active Tab" = workspace::TAB;
        TabStrip "items.tab_strip" "Tab Strip" = TAB_STRIP;
        ItemsSelected "items.selected" "Selected Row" = workspace::SELECTED;
        ItemsPrimary "items.primary" "Primary Selection" = workspace::PRIMARY;
        ItemsSeparator "items.separator" "Column Lines" = workspace::SEPARATOR;
        ItemsTreeLines "items.tree_lines" "Tree Lines" = workspace::TREE_LINES;
        ItemsGlyphs "items.glyphs" "Arrows and Eyes" = workspace::GLYPHS;
        ItemsActiveMesh "items.active_mesh" "Active Mesh Icon" = ORANGE;
        ItemsDropIndicator "items.drop_indicator" "Drop Indicator" = workspace::DROP_INDICATOR;
        ItemsGroupLocatorIcon "items.group_locator_icon" "Group Locator Icon" = workspace::GROUP_LOCATOR_ICON;
    }
    "Interface Text" {
        Text "text.normal" "Text" = TEXT;
        WeakText "text.secondary" "Secondary Text" = WEAK_TEXT;
        DisabledText "text.disabled" "Disabled Text" = DISABLED_TEXT;
        ErrorText "text.error" "Error Text" = ERROR;
        OnAccent "text.on_accent" "Text on Accent" = ON_ORANGE;
        ToolboxText "text.toolbox" "Toolbox Text" = chrome::TEXT;
        ToolboxActiveText "text.toolbox_active" "Toolbox Active Text" = chrome::ACTIVE_TEXT;
        ToolboxDisabledText "text.toolbox_disabled" "Toolbox Disabled Text" = chrome::DISABLED_TEXT;
        ItemsText "text.items" "Items Text" = workspace::FIELD_TEXT;
        ItemsHeaderText "text.items_header" "Items Header Text" = workspace::HEADER_TEXT;
        ItemsSelectedText "text.items_selected" "Items Selected Text" = ORANGE;
        TabText "text.tab" "Tab Text" = TAB_TEXT;
        ItemsInactive "text.items_hidden" "Items Hidden Text" = workspace::INACTIVE;
    }
    "Background and Grids" {
        ViewportBackground "viewport.background" "Background" = viewport::BACKGROUND;
        ViewportGradientTop "viewport.gradient_top" "Gradient Top" = viewport::BACKGROUND_TOP;
        ViewportHeaderBorder "viewport.header_border" "Header Button Border" = viewport::HEADER_BORDER;
        ViewportHeaderButton "viewport.header_button" "Header Button" = viewport::HEADER_BUTTON;
        ViewportHeaderText "viewport.header_text" "Header Button Text" = viewport::HEADER_TEXT;
        ViewportHeaderOn "viewport.header_on" "Header Button On" = viewport::HEADER_ON;
        Grid "viewport.grid" "Grid" = viewport::GRID;
        WorkPlane "viewport.work_plane" "Work Plane" = viewport::WORK_PLANE;
        GridAxisX "viewport.grid_axis_x" "X Axis" = viewport::GRID_AXES[0];
        GridAxisY "viewport.grid_axis_y" "Y Axis" = viewport::GRID_AXES[1];
        GridAxisZ "viewport.grid_axis_z" "Z Axis" = viewport::GRID_AXES[2];
        ZeroAxes "viewport.zero_axes" "Zero Axes (Top/Front/Side)" = viewport::ZERO_AXES;
    }
    // The stock Orthographic and UV schemes' own colours (section 3); the
    // rows above are the Perspective scheme's.
    "Orthographic Views" {
        OrthoBackground "viewport.ortho.background" "Background" = viewport::ORTHO_BACKGROUND;
        OrthoGrid "viewport.ortho.grid" "Grid" = viewport::ORTHO_GRID;
        OrthoWorkPlane "viewport.ortho.work_plane" "Work Plane" = viewport::ORTHO_WORK_PLANE;
        OrthoWireframe "viewport.ortho.wireframe" "Wireframe" = viewport::WIREFRAME;
    }
    "UV View" {
        UvBackground "viewport.uv.background" "Background" = viewport::UV_BACKGROUND;
        UvGrid "viewport.uv.grid" "Grid" = viewport::UV_GRID;
        UvWireframe "viewport.uv.wireframe" "Wireframe" = viewport::UV_WIREFRAME;
        UvAxisU "viewport.uv.axis_u" "U Axis" = viewport::UV_AXES[0];
        UvAxisV "viewport.uv.axis_v" "V Axis" = viewport::UV_AXES[1];
    }
    "Geometry" {
        Wireframe "viewport.wireframe" "Wireframe" = viewport::WIREFRAME;
        InactiveWireframe "viewport.inactive_wireframe" "Inactive Wireframe" = viewport::INACTIVE_WIREFRAME;
        ShadedWireframe "viewport.shaded_wireframe" "Shaded Wireframe" = viewport::SHADED_WIREFRAME;
        Solid "viewport.solid" "Solid" = viewport::SOLID;
        EdgeBacking "viewport.edge_backing" "Edge Backing" = viewport::EDGE_BACKING;
        ColoredWire "viewport.colored_wire" "Dark Lines Overlay" = viewport::COLORED_WIRE;
        VertexMapNone "viewport.vertex_map_none" "No Vertex Map" = viewport::VERTEX_MAP_NONE;
        Locator "viewport.locator" "Cameras and Lights" = viewport::LOCATOR;
        DenseMeshBox "viewport.dense_mesh_box" "Dense Mesh Box" = viewport::DENSE_MESH_BOX;
        // Topology Mode (missing-tools-g3 MT7-02): the spec's RGB defaults.
        TopoFace "viewport.topo_face" "Topo Face" = Color32::from_rgb(51, 51, 51);
        TopoEdge "viewport.topo_edge" "Topo Edge" = Color32::from_rgb(28, 64, 105);
        TopoBackEdge "viewport.topo_back_edge" "Topo Back Edge" = Color32::from_rgb(28, 64, 105);
        TopoVertices "viewport.topo_vertices" "Topo Vertices" = Color32::from_rgb(97, 158, 235);
    }
    "Display Styles" {
        GoochCool "viewport.gooch_cool" "Gooch Cool" = viewport::GOOCH_COOL;
        GoochWarm "viewport.gooch_warm" "Gooch Warm" = viewport::GOOCH_WARM;
    }
    "Selection" {
        Selection "viewport.selection" "Selection" = viewport::SELECTION;
        SelectionFill "viewport.selection_fill" "Selection Fill" = viewport::SELECTION;
        MirrorSelection "viewport.mirror_selection" "Symmetry Selection" = viewport::MIRROR_SELECTION;
        Locked "viewport.locked" "Locked mesh elements" = viewport::LOCKED;
        Lasso "viewport.lasso" "Lasso" = viewport::ROLLOVER;
    }
    "Handles" {
        Handle "viewport.handle" "Handle" = viewport::HANDLE;
        HandleActive "viewport.handle_active" "Active" = viewport::SELECTION;
        HandleX "viewport.handle_x" "Axis X" = viewport::HANDLES[0];
        HandleY "viewport.handle_y" "Y" = viewport::HANDLES[1];
        HandleZ "viewport.handle_z" "Z" = viewport::HANDLES[2];
        LightHandleCircle "viewport.light_handle_circle" "Light Handle Circles" = viewport::LIGHT_HANDLE_CIRCLE;
        LightHandleGrab "viewport.light_handle_grab" "Light Handle Grabs" = viewport::LIGHT_HANDLE_GRAB;
    }
    "Text" {
        ViewportLabel "text.viewport" "Info" = viewport::LABEL;
    }
    "Selection Rollovers" {
        Rollover "viewport.rollover" "Generic" = viewport::ROLLOVER;
        RolloverFace "viewport.rollover_face" "Face" = viewport::ROLLOVER;
        RolloverSubdivision "viewport.rollover_subdivision" "Subdivision" = viewport::ROLLOVER;
        RolloverCatmullClark "viewport.rollover_catmull_clark" "Catmull-Clark" = viewport::ROLLOVER;
    }
    "Falloff Range" {
        FalloffCool "viewport.falloff_cool" "Cool" = viewport::FALLOFF_COOL;
        FalloffWarm "viewport.falloff_warm" "Warm" = viewport::FALLOFF_WARM;
    }
    "Highlights" {
        UvSeam "viewport.uv_seam" "UV Seam" = viewport::UV_SEAM;
        EdgeWeight "viewport.edge_weight" "Edge Weight" = viewport::EDGE_WEIGHT;
        SnapTarget "viewport.snap_target" "Snapped" = viewport::SNAP_TARGET;
        SnapUnsnapped "viewport.snap_unsnapped" "Unsnapped" = viewport::SNAP_UNSNAPPED;
        SliceBar "viewport.slice_bar" "Slice Bar and Line" = viewport::SLICE_BAR;
        SlicePoint "viewport.slice_point" "Slice Points" = viewport::SLICE_POINT;
        SliceClose "viewport.slice_close" "Slice Closing Point" = viewport::SLICE_CLOSE;
        BackdropPlaceholder "viewport.backdrop_placeholder" "Backdrop Placeholder" = viewport::BACKDROP_PLACEHOLDER;
        CameraFrameShade "viewport.camera_frame_shade" "Outside Render Frame" = viewport::CAMERA_FRAME_SHADE;
    }
    // Note 10 IC-B2: item editor colours (row bar and tint). Mid tones
    // that read on dark and light themes.
    "Item Editor Colors" {
        TagRed "items.tag_red" "Red" = Color32::from_rgb(214, 64, 64);
        TagMagenta "items.tag_magenta" "Magenta" = Color32::from_rgb(204, 60, 180);
        TagPink "items.tag_pink" "Pink" = Color32::from_rgb(236, 122, 164);
        TagBrown "items.tag_brown" "Brown" = Color32::from_rgb(150, 100, 58);
        TagOrange "items.tag_orange" "Orange" = Color32::from_rgb(232, 130, 40);
        TagYellow "items.tag_yellow" "Yellow" = Color32::from_rgb(222, 196, 50);
        TagGreen "items.tag_green" "Green" = Color32::from_rgb(58, 160, 70);
        TagLightGreen "items.tag_lightgreen" "Light Green" = Color32::from_rgb(130, 206, 110);
        TagBlue "items.tag_blue" "Blue" = Color32::from_rgb(58, 108, 220);
        TagLightBlue "items.tag_lightblue" "Light Blue" = Color32::from_rgb(110, 170, 236);
        TagCyan "items.tag_cyan" "Cyan" = Color32::from_rgb(48, 190, 200);
        TagUltramarine "items.tag_ultramarine" "Ultramarine" = Color32::from_rgb(72, 60, 204);
        TagPurple "items.tag_purple" "Purple" = Color32::from_rgb(132, 70, 192);
        TagLightPurple "items.tag_lightpurple" "Light Purple" = Color32::from_rgb(176, 140, 226);
        TagWhite "items.tag_white" "White" = Color32::from_rgb(236, 236, 236);
        TagGrey "items.tag_grey" "Grey" = Color32::from_rgb(150, 150, 150);
        TagDarkGrey "items.tag_darkgrey" "Dark Grey" = Color32::from_rgb(86, 86, 86);
    }
}
const COUNT: usize = COLORS.len();

impl ColorId {
    pub fn entry(self) -> &'static ColorEntry {
        &COLORS[self as usize]
    }
    pub fn from_key(key: &str) -> Option<Self> {
        COLORS.iter().find(|e| e.key == key).map(|e| e.id)
    }
    /// Swatches that now follow Accent, with their former fixed defaults
    /// for migrating legacy files. Keep these RGB values fixed.
    pub fn legacy_accent_default(self) -> Option<Color32> {
        match self {
            Self::Selection | Self::SelectionFill => Some(Color32::from_rgb(0xFF, 0xC2, 0x4A)),
            Self::PanelDotActive => Some(Color32::from_rgb(0xFF, 0xB8, 0x28)),
            Self::ChannelChanged | Self::ItemsActiveMesh | Self::ItemsSelectedText => {
                Some(Color32::from_rgb(0xF4, 0x9C, 0x1C))
            }
            _ => None,
        }
    }
    pub fn follows_accent_by_default(self) -> bool {
        self.legacy_accent_default().is_some()
    }
    /// The live default, ignoring this swatch's own override.
    pub fn default_color(self) -> Color32 {
        // The polygon fill follows the (resolved) Selection colour, so a
        // pinned or scheme Selection colour fills faces in that colour too
        // (Jason, 2026-11-19: the fill looked missing when Selection was
        // amber but the fill had gone Accent blue).
        if self == Self::SelectionFill {
            return color(Self::Selection);
        }
        if self.follows_accent_by_default() {
            orange()
        } else {
            self.entry().default
        }
    }
}

// Overrides live per thread: the app draws on one thread, and tests that
// change colours can't leak into tests running beside them.
thread_local! {
    static OVERRIDES: [std::cell::Cell<Option<Color32>>; COUNT] =
        const { [const { std::cell::Cell::new(None) }; COUNT] };
    static GENERATION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static INSTALLED: std::cell::Cell<u64> = const { std::cell::Cell::new(u64::MAX) };
}

/// An item editor colour as the theme draws it (note 10).
pub fn editor_color(c: operandi_core::EditorColor) -> Color32 {
    use operandi_core::EditorColor;
    color(match c {
        EditorColor::Red => ColorId::TagRed,
        EditorColor::Magenta => ColorId::TagMagenta,
        EditorColor::Pink => ColorId::TagPink,
        EditorColor::Brown => ColorId::TagBrown,
        EditorColor::Orange => ColorId::TagOrange,
        EditorColor::Yellow => ColorId::TagYellow,
        EditorColor::Green => ColorId::TagGreen,
        EditorColor::LightGreen => ColorId::TagLightGreen,
        EditorColor::Blue => ColorId::TagBlue,
        EditorColor::LightBlue => ColorId::TagLightBlue,
        EditorColor::Cyan => ColorId::TagCyan,
        EditorColor::Ultramarine => ColorId::TagUltramarine,
        EditorColor::Purple => ColorId::TagPurple,
        EditorColor::LightPurple => ColorId::TagLightPurple,
        EditorColor::White => ColorId::TagWhite,
        EditorColor::Grey => ColorId::TagGrey,
        EditorColor::DarkGrey => ColorId::TagDarkGrey,
    })
}

/// This thread's drawing colours: every override and the view scheme.
/// Colours live per thread, so work spread over worker threads takes the
/// drawing thread's with it (`Colours::apply`).
#[derive(Clone)]
pub struct Colours {
    overrides: [Option<Color32>; COUNT],
    scheme: viewport::ViewScheme,
}
pub fn colours() -> Colours {
    Colours {
        overrides: OVERRIDES.with(|o| std::array::from_fn(|i| o[i].get())),
        scheme: viewport::scheme(),
    }
}
impl Colours {
    /// Draw with these colours on this thread. Leaves the colour
    /// generation alone: these are another thread's colours, not a change.
    pub fn apply(&self) {
        OVERRIDES.with(|o| {
            for (cell, value) in o.iter().zip(self.overrides) {
                cell.set(value);
            }
        });
        viewport::set_scheme(self.scheme);
    }
}

/// The colour to draw `id` with now.
pub fn color(id: ColorId) -> Color32 {
    override_of(id).unwrap_or_else(|| id.default_color())
}
/// The user's override for `id`, if any.
pub fn override_of(id: ColorId) -> Option<Color32> {
    OVERRIDES.with(|o| o[id as usize].get())
}
/// Sets or clears (None) one override. Fixed defaults need no override.
/// Accent followers keep explicit colours even when they match Accent.
pub fn set_override(id: ColorId, value: Option<Color32>) {
    let value = value.filter(|c| id.follows_accent_by_default() || *c != id.entry().default);
    OVERRIDES.with(|o| {
        if o[id as usize].replace(value) != value {
            GENERATION.with(|g| g.set(g.get() + 1));
        }
    });
}
/// Replaces every override at once (load a scheme, reset all).
pub fn set_overrides(all: &std::collections::BTreeMap<ColorId, Color32>) {
    for entry in COLORS {
        set_override(entry.id, all.get(&entry.id).copied());
    }
}
/// Changes whenever any colour changes; caches of drawn colours key on it.
pub fn generation() -> u64 {
    GENERATION.with(|g| g.get())
}

#[cfg(test)]
mod tests {
    /// #68: a ten-entry dropdown (the viewport's view menu) opens without a
    /// scrollbar.
    #[test]
    fn theme_lets_a_ten_entry_dropdown_open_without_scrolling() {
        let ctx = eframe::egui::Context::default();
        super::install(&ctx);
        let spacing = ctx.style().spacing.clone();
        let row = spacing.interact_size.y + spacing.item_spacing.y;
        let needed = 10. * row + spacing.menu_margin.sum().y;
        assert!(
            spacing.combo_height >= needed,
            "combo_height {} < {needed}",
            spacing.combo_height
        );
    }
}
