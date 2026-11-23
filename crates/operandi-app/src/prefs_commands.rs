//! `pref.value <name> <value|?>`: one general setter for preferences
//! (extensibility stage 2; UIC-165), like `item.channel` for scene values.
//!
//! Each preference is one [`Pref`] in [`PREFS`]: its key, the Preferences
//! page and row label it belongs to, its value type (core's
//! `channels::ChannelType`, so values parse the same way), how to read and
//! set it in memory, and the hook that saves (and applies) its group. The
//! Preferences window runs `pref.value` for every row edit on a page that
//! has entries here, so the window and scripts can't disagree.
//!
//! Keys are Operandi's own for now (provisional, to be matched to Modo's
//! preference names later). Percents are 0–1 at the command boundary.
use super::*;
use crate::navigation_preferences::{InputDevice, ZoomDragAxis};
use crate::preferences::Page;
use crate::prefs_file::PrefsFile;
use operandi_core::channels::{ChannelType, ChannelValue, parse, text};

type SetResult = std::result::Result<(), String>;

/// A page's preference values before its rows draw.
pub(super) struct PrefSnapshot {
    entries: Vec<(usize, ChannelValue)>,
    groups: Vec<(&'static Group, serde_json::Value)>,
    colors: Option<crate::color_theme::Overrides>,
}

#[derive(Clone)]
pub(crate) struct Pref {
    pub key: &'static str,
    pub page: Page,
    /// The row's label on its page (the UIC-164 test reads it).
    pub label: &'static str,
    pub ty: ChannelType,
    pub get: fn(&Operandi) -> ChannelValue,
    /// Sets the value in memory only.
    pub set: fn(&mut Operandi, ChannelValue) -> SetResult,
    /// Saves (and applies) the preference's group after a change.
    pub after: fn(&mut Operandi),
}

fn as_bool(v: ChannelValue) -> std::result::Result<bool, String> {
    match v {
        ChannelValue::Bool(b) => Ok(b),
        _ => Err("Expected true or false".into()),
    }
}
fn as_float(v: ChannelValue) -> std::result::Result<f64, String> {
    match v {
        ChannelValue::Float(f) => Ok(f),
        _ => Err("Expected a number".into()),
    }
}
fn as_index(v: ChannelValue) -> std::result::Result<usize, String> {
    match v {
        ChannelValue::Enum(i) => Ok(i),
        _ => Err("Expected a choice".into()),
    }
}

/// Saves the Input preferences (Remapping page), as the page does.
fn save_input(a: &mut Operandi) {
    crate::dialog_place::set_under_mouse(a.input_preferences.options.dialogs_under_mouse);
    if let Err(error) = a
        .input_preferences
        .save(&a.environment.paths().input_preferences())
    {
        a.status = format!("Preference changed; could not save it: {error}");
    }
}

const DEVICES: &[&str] = &["Mouse", "Tablet", "Hybrid"];
const ZOOM_AXES: &[&str] = &[
    "Left to Right",
    "Right to Left",
    "Bottom to Top",
    "Top to Bottom",
];

/// A Remapping checkbox.
macro_rules! remap_bool {
    ($key:literal, $label:literal, $field:ident) => {
        Pref {
            key: $key,
            page: Page::Remapping,
            label: $label,
            ty: ChannelType::Bool,
            get: |a| ChannelValue::Bool(a.input_preferences.options.$field),
            set: |a, v| {
                a.input_preferences.options.$field = as_bool(v)?;
                Ok(())
            },
            after: save_input,
        }
    };
}

/// Every preference `pref.value` knows. Pages listed here run their row
/// edits through it; the rest are UIC-164 gaps until they're added.
pub(crate) static PREFS: &[Pref] = &[
    Pref {
        key: "input.device",
        page: Page::Remapping,
        label: "Input Device",
        ty: ChannelType::Enum(DEVICES),
        get: |a| {
            let d = a.input_preferences.options.input_device;
            ChannelValue::Enum(InputDevice::ALL.iter().position(|&x| x == d).unwrap_or(0))
        },
        set: |a, v| {
            a.input_preferences.options.input_device = *InputDevice::ALL
                .get(as_index(v)?)
                .ok_or("Unknown input device")?;
            Ok(())
        },
        after: save_input,
    },
    Pref {
        key: "input.tooltipDelay",
        page: Page::Remapping,
        label: "Tooltip Delay",
        ty: ChannelType::Float,
        get: |a| ChannelValue::Float(f64::from(a.input_preferences.options.tooltip_delay)),
        set: |a, v| {
            let s = as_float(v)?;
            let range = crate::navigation_preferences::TOOLTIP_DELAY_RANGE;
            if !range.contains(&(s as f32)) {
                return Err(format!(
                    "Tooltip Delay is {} to {} s",
                    range.start(),
                    range.end()
                ));
            }
            a.input_preferences.options.tooltip_delay = s as f32;
            Ok(())
        },
        after: save_input,
    },
    Pref {
        // Percent 0–1 (the page shows %).
        key: "input.wheelAcceleration",
        page: Page::Remapping,
        label: "Mouse Wheel Acceleration",
        ty: ChannelType::Percent,
        get: |a| {
            ChannelValue::Float(f64::from(a.input_preferences.options.wheel_acceleration) / 100.)
        },
        set: |a, v| {
            let percent = (as_float(v)? * 100.).round();
            let range = crate::navigation_preferences::WHEEL_ACCELERATION_RANGE;
            if percent < 0. || !range.contains(&(percent as u32)) {
                return Err(format!(
                    "Mouse Wheel Acceleration is {}% to {}%",
                    range.start(),
                    range.end()
                ));
            }
            a.input_preferences.options.wheel_acceleration = percent as u32;
            Ok(())
        },
        after: save_input,
    },
    remap_bool!(
        "input.wheelZoomAtCursor",
        "Wheel Zoom at Mouse Cursor",
        wheel_zoom_at_cursor
    ),
    remap_bool!(
        "input.invertWheelZoom",
        "Invert Mouse Wheel Zoom",
        invert_wheel_zoom
    ),
    Pref {
        key: "input.zoomDragAxis",
        page: Page::Remapping,
        label: "Zoom Drag Axis",
        ty: ChannelType::Enum(ZOOM_AXES),
        get: |a| {
            let z = a.input_preferences.options.zoom_drag_axis;
            ChannelValue::Enum(ZoomDragAxis::ALL.iter().position(|&x| x == z).unwrap_or(0))
        },
        set: |a, v| {
            a.input_preferences.options.zoom_drag_axis = *ZoomDragAxis::ALL
                .get(as_index(v)?)
                .ok_or("Unknown zoom drag axis")?;
            Ok(())
        },
        after: save_input,
    },
    remap_bool!(
        "input.reversePanWidget",
        "Reverse Pan Widget Direction",
        reverse_pan_widget
    ),
    remap_bool!(
        "input.invertWheelScrolling",
        "Invert Mouse Wheel Scrolling",
        invert_wheel_scrolling
    ),
    remap_bool!(
        "input.keypadEnterAsTab",
        "Numeric Enter as Tab",
        keypad_enter_as_tab
    ),
    remap_bool!(
        "input.dialogsUnderMouse",
        "Open Dialogs under Mouse",
        dialogs_under_mouse
    ),
    remap_bool!("input.trackball", "Trackball Rotation", trackball),
    remap_bool!(
        "input.trackballOverride",
        "Override Viewport-Level Trackball Option",
        trackball_override
    ),
    Pref {
        key: "input.trackballSpeedMouse",
        page: Page::Remapping,
        label: "Trackball Speed Multiplier (Mouse)",
        ty: ChannelType::Float,
        get: |a| ChannelValue::Float(f64::from(a.input_preferences.options.trackball_speed_mouse)),
        set: |a, v| {
            a.input_preferences.options.trackball_speed_mouse = trackball_speed(as_float(v)?)?;
            Ok(())
        },
        after: save_input,
    },
    Pref {
        key: "input.trackballSpeedTablet",
        page: Page::Remapping,
        label: "Trackball Speed Multiplier (Tablet)",
        ty: ChannelType::Float,
        get: |a| {
            ChannelValue::Float(f64::from(
                a.input_preferences.options.trackball_speed_tablet,
            ))
        },
        set: |a, v| {
            a.input_preferences.options.trackball_speed_tablet = trackball_speed(as_float(v)?)?;
            Ok(())
        },
        after: save_input,
    },
];

fn trackball_speed(s: f64) -> std::result::Result<f32, String> {
    let range = crate::navigation_preferences::TRACKBALL_SPEED_RANGE;
    if !range.contains(&(s as f32)) {
        return Err(format!(
            "Trackball speed is {} to {}",
            range.start(),
            range.end()
        ));
    }
    Ok(s as f32)
}

// ------------------------------------------------- whole preference groups

/// A preference group (one saved file) whose every field `pref.value`
/// reads and sets by name: `<group>.<field>` (nested fields joined with
/// dots). Values are JSON-like text (`true`, `2.5`, `Top`, `[1,2,3]`) in
/// the group's own stored units (provisional: Remapping, above, already
/// takes Modo's 0–1 percents; the rest follow as they're matched).
pub(crate) struct Group {
    pub name: &'static str,
    pub pages: &'static [Page],
    pub get: fn(&Operandi) -> serde_json::Value,
    set: fn(&mut Operandi, serde_json::Value) -> SetResult,
    /// Saves (and applies) the group; gets its value before the change.
    save: fn(&mut Operandi, serde_json::Value),
}

macro_rules! group {
    ($name:literal, [$($page:ident),*], $($field:ident).+, $ty:ty, $save:expr) => {
        Group {
            name: $name,
            pages: &[$(Page::$page),*],
            get: |a| serde_json::to_value(&a.$($field).+).unwrap_or_default(),
            set: |a, v| {
                a.$($field).+ = serde_json::from_value::<$ty>(v).map_err(|e| e.to_string())?;
                Ok(())
            },
            save: $save,
        }
    };
}

/// Saves `field` to its file, as its page does.
macro_rules! save_to {
    ($field:ident, $path:ident) => {
        |a: &mut Operandi, _| {
            if let Err(error) = a.$field.save(&a.environment.paths().$path()) {
                a.status = format!("Preference changed; could not save it: {error}");
            }
        }
    };
}

pub(crate) static GROUPS: &[Group] = &[
    group!(
        "input",
        [Input],
        input_preferences,
        preferences::InputPreferences,
        save_input_group
    ),
    group!(
        "selection",
        [Selection],
        selection_preferences,
        selection_preferences::SelectionPreferences,
        save_to!(selection_preferences, selection_preferences)
    ),
    group!(
        "units",
        [DistanceUnits, Grid],
        units_preferences,
        units_preferences::UnitsPreferences,
        |a, old| {
            if let Ok(before) = serde_json::from_value(old) {
                a.save_units(before);
            }
        }
    ),
    group!(
        "display",
        [Display],
        viewport_preferences,
        viewport_preferences::ViewportPreferences,
        save_to!(viewport_preferences, viewport_display)
    ),
    group!(
        "handles",
        [ToolHandles],
        handle_preferences,
        handle_preferences::HandlePreferences,
        save_to!(handle_preferences, handle_preferences)
    ),
    group!(
        "defaults",
        [Overlays, ToolSettings, FileInfo],
        defaults_preferences,
        defaults_preferences::DefaultsPreferences,
        |a, old| {
            if let Ok(before) = serde_json::from_value(old) {
                a.save_defaults(before);
            }
        }
    ),
    group!(
        "application",
        [Application, AutoSave],
        undo_autosave,
        undo_autosave::UndoAutosavePreferences,
        |a, _| a.save_undo_autosave()
    ),
    // The Application page's Updates section (update_check.rs).
    group!(
        "updates",
        [Application],
        update_check.prefs,
        update_check::UpdatePreferences,
        |a, _| {
            if let Err(error) = a
                .update_check
                .prefs
                .save(&a.environment.paths().update_preferences())
            {
                a.status = format!("Preference changed; could not save it: {error}");
            }
        }
    ),
    group!(
        "meshItems",
        [MeshItems],
        mesh_items_preferences,
        mesh_items_preferences::MeshItemsPreferences,
        save_to!(mesh_items_preferences, mesh_items_preferences)
    ),
    group!(
        "ui",
        [Ui],
        ui_preferences,
        ui_preferences::UiPreferences,
        // The UI page takes effect at once (popup transparency, icon size,
        // Original Compatible, Slider Min/Max Labels), as its rows do.
        |a, old| {
            a.ui_preferences.apply();
            save_to!(ui_preferences, ui_preferences)(a, old)
        }
    ),
    group!(
        "rendering",
        [Rendering],
        render_preferences,
        render_preferences::RenderPreferences,
        save_to!(render_preferences, render_preferences)
    ),
    // The Rendering page's Render Window section.
    group!(
        "renderWindow",
        [Rendering],
        render.prefs,
        crate::image_viewer::ViewerPreferences,
        |a, old| {
            a.save_viewer_preferences();
            let before: Option<crate::image_viewer::ViewerPreferences> =
                serde_json::from_value(old).ok();
            if before.is_some_and(|b| b.max_render_slots != a.render.prefs.max_render_slots) {
                // A lower cap takes effect at once, as on the page.
                for gone in a.render.history.trim(a.render.prefs.max_render_slots) {
                    a.render.viewer.forget(gone.id);
                }
                let _ = a.render.history.save_index();
            }
        }
    ),
    // Checked on set: a button shortcut must be one Operandi can read.
    Group {
        name: "spaceMouse",
        pages: &[Page::SpaceMouse],
        get: |a| serde_json::to_value(&a.space_mouse.prefs).unwrap_or_default(),
        set: |a, v| {
            let p: crate::space_mouse::SpaceMousePreferences =
                serde_json::from_value(v).map_err(|e| e.to_string())?;
            if !p.valid() {
                return Err("Not a valid 3D mouse setting".into());
            }
            a.space_mouse.prefs = p;
            Ok(())
        },
        save: |a, _| {
            let path = a.environment.paths().space_mouse_preferences();
            if let Err(error) = a.space_mouse.prefs.save(&path) {
                a.status = format!("Preference changed; could not save it: {error}");
            }
        },
    },
    group!(
        "fileIo",
        [FileIo],
        file_io_preferences,
        file_io_preferences::FileIoPreferences,
        save_to!(file_io_preferences, file_io_preferences)
    ),
];

fn save_input_group(a: &mut Operandi, _: serde_json::Value) {
    save_input(a);
}

/// Every leaf of `value` as (dotted path, leaf); arrays are leaves.
fn leaves(prefix: &str, value: &serde_json::Value, out: &mut Vec<(String, serde_json::Value)>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                leaves(&path, v, out);
            }
        }
        _ => out.push((prefix.to_owned(), value.clone())),
    }
}

/// `value` with the leaf at `path` replaced.
fn with_leaf(
    mut value: serde_json::Value,
    path: &str,
    leaf: serde_json::Value,
) -> Option<serde_json::Value> {
    let mut at = &mut value;
    for part in path.split('.') {
        at = at.as_object_mut()?.get_mut(part)?;
    }
    if at.is_object() {
        return None;
    }
    *at = leaf;
    Some(value)
}

/// A leaf as command text (strings bare, braced when they have spaces).
fn leaf_text(leaf: &serde_json::Value) -> String {
    match leaf {
        // Empty (a cleared shortcut) braced too, so it stays an argument.
        serde_json::Value::String(s) if s.contains(' ') || s.is_empty() => format!("{{{s}}}"),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// How a group field is stored when the command takes map units
/// (Jason, 2026-11-07: percents 0–1, angles in radians at the command).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Stored {
    /// A percentage stored as the number shown (50 = 50 %).
    Percent100,
    /// An angle stored in degrees.
    Degrees,
}

/// Group fields whose stored units differ from the map's. A test reads
/// the pages for every Percent and Angle row and checks it's listed.
pub(crate) const UNITS: &[(&str, Stored)] = &[
    ("selection.selection_xray", Stored::Percent100),
    ("units.preferred_bias", Stored::Percent100),
    ("display.workplane_opacity", Stored::Percent100),
    ("display.flatness", Stored::Percent100),
    ("display.topology_opacity", Stored::Percent100),
    ("handles.handle_xray", Stored::Percent100),
    ("handles.rotation_snap", Stored::Degrees),
    ("ui.popup_transparency", Stored::Percent100),
    ("fileIo.jpeg_quality", Stored::Percent100),
];

fn stored(key: &str) -> Option<Stored> {
    UNITS
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, u)| *u)
}

/// A stored leaf as command text in map units.
fn map_text(key: &str, leaf: &serde_json::Value) -> String {
    match (stored(key), leaf.as_f64()) {
        (Some(Stored::Percent100), Some(v)) => format!("{}", v / 100.),
        (Some(Stored::Degrees), Some(v)) => format!("{}", v.to_radians()),
        _ => leaf_text(leaf),
    }
}

/// Command text in map units as the stored leaf (`old` tells integer
/// fields from fractional ones). `None` when `key` has no conversion.
fn stored_leaf(
    key: &str,
    text: &str,
    old: &serde_json::Value,
) -> Option<std::result::Result<serde_json::Value, String>> {
    let value = match stored(key)? {
        Stored::Percent100 => operandi_core::command_names::script_percent(text),
        Stored::Degrees => operandi_core::command_names::script_angle(text),
    };
    Some(value.map(|v| {
        if old.is_u64() || old.is_i64() {
            serde_json::Value::from(v.round() as i64)
        } else {
            serde_json::Number::from_f64(v)
                .map_or(serde_json::Value::Null, serde_json::Value::Number)
        }
    }))
}

/// Each ranged row's (group, field, min, max) in stored units, from the
/// same constants (or literals) its Preferences row uses.
pub(crate) fn ranges() -> Vec<(&'static str, &'static str, f64, f64)> {
    use crate::{
        handle_preferences as h, render_preferences as r, selection_preferences as s,
        undo_autosave as u, units_preferences as un, viewport_preferences as v,
    };
    macro_rules! r {
        ($group:literal, $field:literal, $range:expr) => {
            ($group, $field, *$range.start() as f64, *$range.end() as f64)
        };
    }
    let level = f64::from(operandi_core::subdivision::MAX_LEVEL);
    vec![
        r!("selection", "point_size", s::POINT_SIZE_RANGE),
        r!("selection", "selected_point_scale", s::SELECTED_SCALE_RANGE),
        r!("selection", "hit_size", s::HIT_SIZE_RANGE),
        r!("selection", "point_priority", s::POINT_PRIORITY_RANGE),
        r!("selection", "selection_xray", 0..=100),
        r!("units", "inch_cutoff", 0..=10_000),
        r!("units", "meters_per_game_unit", 0.0001..=10_000.),
        r!("units", "fixed_increment", 1e-6..=1e6),
        r!("units", "grid_size", 1e-6..=1e6),
        r!("units", "fit_factor", un::FIT_RANGE),
        r!("units", "preferred_bias", 0..=100),
        r!("units", "locked_offset", -1e6..=1e6),
        r!("display", "workplane_opacity", 0..=100),
        r!("display", "flatness", v::FLATNESS_RANGE),
        r!("display", "topology_opacity", 0..=100),
        r!(
            "display",
            "item_draw_size",
            operandi_core::scene::ITEM_DRAW_SIZE_RANGE
        ),
        r!("display", "backdrop_pixel_size", 0.000_001..=1_000.),
        r!("handles", "line_width", h::LINE_WIDTH_RANGE),
        r!("handles", "line_hit_width", h::HIT_RANGE),
        r!("handles", "large_points", h::POINT_RANGE),
        r!("handles", "small_points", h::POINT_RANGE),
        r!("handles", "small_point_hit", h::HIT_RANGE),
        r!("handles", "point_hit_size", h::HIT_RANGE),
        r!("handles", "scale", h::SCALE_RANGE),
        r!("handles", "small_scale", h::SMALL_SCALE_RANGE),
        r!("handles", "plane_ratio", h::PLANE_RATIO_RANGE),
        r!("handles", "handle_xray", 0..=100),
        r!("handles", "rotation_snap", h::SNAP_ANGLE_RANGE),
        r!("application", "max_undo_level", u::MAX_UNDO_RANGE),
        r!("application", "max_sub_commands", u::SUB_COMMAND_RANGE),
        r!("application", "interval_minutes", u::INTERVAL_RANGE),
        r!("application", "revisions", u::REVISION_RANGE),
        r!("ui", "popup_transparency", 0..=90),
        r!(
            "renderWindow",
            "max_render_slots",
            crate::image_viewer::MAX_SLOTS_RANGE
        ),
        r!("rendering", "threads", r::THREAD_RANGE),
        r!("fileIo", "png_compression", 1..=9),
        r!("fileIo", "jpeg_quality", 1..=100),
        ("meshItems", "subdivision_level", 0., level),
        ("meshItems", "catmull_clark_level", 0., level),
    ]
}

/// The range of `<group>.<path>` (its last field), if it has one.
fn range_of(group: &str, path: &str) -> Option<(f64, f64)> {
    let field = path.rsplit('.').next()?;
    ranges()
        .into_iter()
        .find(|(g, f, _, _)| g.eq_ignore_ascii_case(group) && f.eq_ignore_ascii_case(field))
        .map(|(_, _, min, max)| (min, max))
}

fn group_named(name: &str) -> Option<&'static Group> {
    GROUPS.iter().find(|g| g.name.eq_ignore_ascii_case(name))
}

/// The groups a page's rows belong to.
fn groups_of(page: Page) -> Vec<&'static Group> {
    GROUPS.iter().filter(|g| g.pages.contains(&page)).collect()
}

/// Whether `pref.value` covers a page's rows through its group (the
/// UIC-164 test). Rows of other state on these pages are listed in
/// `NOT_IN_GROUP` and stay gaps.
#[cfg(test)]
pub(crate) fn page_by_group(page: Page) -> Option<&'static str> {
    if page == Page::Colors {
        return Some("color");
    }
    groups_of(page).first().map(|g| g.name)
}

/// Rows on group pages that aren't the group's own values (buttons, and
/// settings kept elsewhere: snapping, the Preview's samples, the scene's
/// item draw size, the lasso's Partial switch).
#[cfg(test)]
pub(crate) const NOT_IN_GROUP: &[&str] = &[
    "",
    "Scheme",
    "Choose…",
    "Choose...",
    "Clear",
    "Browse…",
    "Default",
    "Input Map…",
    "Inner Range",
    "Outer Range",
    "Max Samples",
    "Item Drawing: Current Scene",
    "Lasso: Allow Partial Element Selection",
    "Device",
];

// ------------------------------------------------- colours

/// `color.<key>`: one Display > Colors colour (theme::COLORS), as
/// `#rrggbb` (or `#rrggbbaa`), or `default` to clear the override.
fn color_entry(key: &str) -> Option<&'static crate::theme::ColorEntry> {
    let key = key.strip_prefix("color.")?;
    crate::theme::COLORS
        .iter()
        .find(|e| e.key.eq_ignore_ascii_case(key))
}

fn save_colors(a: &mut Operandi) {
    let file = crate::color_theme::ColorFile::for_preferences(a.preferences_ui.scheme.as_ref());
    if let Err(error) = file.save(&a.environment.paths().color_theme()) {
        a.status = format!("Colours changed; could not save them: {error}");
    }
}

/// Every preference: [`PREFS`], plus painting's in painting builds.
pub(crate) fn all_prefs() -> &'static [Pref] {
    #[cfg(feature = "painting")]
    {
        static ALL: std::sync::LazyLock<Vec<Pref>> = std::sync::LazyLock::new(|| {
            PREFS
                .iter()
                .chain(crate::painting_prefs::PAINTING_PREFS)
                .cloned()
                .collect()
        });
        &ALL
    }
    #[cfg(not(feature = "painting"))]
    PREFS
}

fn find(key: &str) -> Option<&'static Pref> {
    all_prefs().iter().find(|p| p.key.eq_ignore_ascii_case(key))
}

/// The pref.value key for a Preferences row, for the UIC-164 test.
#[cfg(test)]
pub(crate) fn pref_for_label(page: Page, label: &str) -> Option<&'static str> {
    all_prefs()
        .iter()
        .find(|p| p.page == page && p.label == label)
        .map(|p| p.key)
}

impl Operandi {
    /// `pref.value <name> <value|?>`. False when `text` isn't one.
    pub(super) fn pref_value_command(&mut self, line: &str) -> bool {
        let Ok(args) = command::tokens(line) else {
            return false;
        };
        if !args
            .first()
            .is_some_and(|f| f.eq_ignore_ascii_case("pref.value"))
        {
            return false;
        }
        let bound =
            match operandi_core::command_args::bind("pref.value", &args[1..], &["name", "value"]) {
                Ok(b) => b,
                Err(e) => {
                    self.status = e;
                    return true;
                }
            };
        let (Some(name), Some(value)) = (bound[0], bound[1]) else {
            self.status = "Use pref.value <name> <value|?>; command.list prefs lists none yet, help pref.value lists the names".into();
            return true;
        };
        if let Some(entry) = color_entry(name) {
            if value == "?" {
                let c = crate::theme::override_of(entry.id).unwrap_or(entry.default);
                self.status = crate::color_theme::hex(c);
            } else if value.eq_ignore_ascii_case("default") {
                crate::theme::set_override(entry.id, None);
                save_colors(self);
                self.status = "Ready".into();
            } else {
                match crate::color_theme::parse_hex(value) {
                    Some(c) => {
                        crate::theme::set_override(entry.id, Some(c));
                        save_colors(self);
                        self.status = "Ready".into();
                    }
                    None => {
                        self.status = format!("{name} takes #rrggbb or default, not \"{value}\"")
                    }
                }
            }
            return true;
        }
        if find(name).is_none()
            && let Some((group_name, path)) = name.split_once('.')
            && let Some(group) = group_named(group_name)
        {
            let current = (group.get)(self);
            let mut all = Vec::new();
            leaves("", &current, &mut all);
            let Some((_, leaf)) = all.iter().find(|(p, _)| p.eq_ignore_ascii_case(path)) else {
                let names: Vec<String> = all
                    .iter()
                    .map(|(p, _)| format!("{group_name}.{p}"))
                    .collect();
                self.status = format!(
                    "No preference \"{name}\"; {group_name} has {}",
                    names.join(", ")
                );
                return true;
            };
            let path = all
                .iter()
                .find(|(p, _)| p.eq_ignore_ascii_case(path))
                .map(|(p, _)| p.clone())
                .unwrap_or_default();
            let key = format!("{}.{path}", group.name);
            if value == "?" {
                self.status = map_text(&key, leaf);
                return true;
            }
            // Map units in (percent 0–1, radians; 50% and 30deg too), else
            // JSON (numbers, booleans, lists), else the text itself.
            let tries: Vec<serde_json::Value> = match stored_leaf(&key, value, leaf) {
                Some(Ok(v)) => vec![v],
                Some(Err(e)) => {
                    self.status = e;
                    return true;
                }
                None => serde_json::from_str::<serde_json::Value>(value)
                    .ok()
                    // "20.0" for a whole-number field is 20.
                    .map(|v| match v.as_f64() {
                        Some(f) if (leaf.is_u64() || leaf.is_i64()) && f.fract() == 0. => {
                            serde_json::Value::from(f as i64)
                        }
                        _ => v,
                    })
                    .into_iter()
                    .chain([serde_json::Value::String(value.to_owned())])
                    .collect(),
            };
            // The row's range (in stored units), told back in map units.
            if let Some((min, max)) = range_of(group.name, &path)
                && let Some(v) = tries.first().and_then(serde_json::Value::as_f64)
                && !(min..=max).contains(&v)
            {
                let shown = |x: f64| map_text(&key, &serde_json::json!(x));
                self.status = format!("{name} is {} to {}, not {value}", shown(min), shown(max));
                return true;
            }
            let result = tries.into_iter().find_map(|leaf| {
                let next = with_leaf(current.clone(), &path, leaf)?;
                (group.set)(self, next).ok()
            });
            match result {
                Some(()) => {
                    (group.save)(self, current);
                    if !self.status.starts_with("Preference changed") {
                        self.status = "Ready".into();
                    }
                }
                None => {
                    self.status = format!(
                        "{name} doesn't take \"{value}\" (it is {})",
                        leaf_text(leaf)
                    )
                }
            }
            return true;
        }
        let Some(pref) = find(name) else {
            let names: Vec<String> = all_prefs()
                .iter()
                .map(|p| format!("{} ({})", p.key, p.label))
                .collect();
            self.status = format!(
                "No preference \"{name}\"; pref.value knows {}",
                names.join(", ")
            );
            return true;
        };
        if value == "?" {
            self.status = text(pref.ty, &(pref.get)(self));
            return true;
        }
        match parse(pref.ty, pref.key, value).and_then(|v| (pref.set)(self, v)) {
            Ok(()) => {
                (pref.after)(self);
                if !self.status.starts_with("Preference changed") {
                    self.status = "Ready".into();
                }
            }
            Err(e) => self.status = e,
        }
        true
    }

    /// The current values of `page`'s preferences, before its rows draw.
    pub(super) fn pref_snapshot(&self, page: Page) -> PrefSnapshot {
        let entries: Vec<(usize, ChannelValue)> = all_prefs()
            .iter()
            .enumerate()
            .filter(|(_, p)| p.page == page)
            .map(|(i, p)| (i, (p.get)(self)))
            .collect();
        // Pages with their own entries don't also go by group.
        let groups = if entries.is_empty() {
            groups_of(page)
                .into_iter()
                .map(|g| (g, (g.get)(self)))
                .collect()
        } else {
            Vec::new()
        };
        let colors = (page == Page::Colors).then(crate::color_theme::current);
        PrefSnapshot {
            entries,
            groups,
            colors,
        }
    }

    /// After `page`'s rows drew: each preference a row changed is put back
    /// and set again through `pref.value` (UIC-164), so the row edit and
    /// the command are the same path.
    pub(super) fn pref_rows_through_commands(&mut self, before: PrefSnapshot) {
        for (group, old) in before.groups {
            let new = (group.get)(self);
            if new != old {
                let (mut was, mut now) = (Vec::new(), Vec::new());
                leaves("", &old, &mut was);
                leaves("", &new, &mut now);
                let _ = (group.set)(self, old);
                for ((path, a), (_, b)) in was.iter().zip(&now) {
                    if a != b {
                        let key = format!("{}.{path}", group.name);
                        self.execute(&format!("pref.value {key} {}", map_text(&key, b)));
                    }
                }
            }
        }
        if let Some(old) = before.colors {
            let new = crate::color_theme::current();
            if new != old {
                crate::theme::set_overrides(&old);
                for entry in crate::theme::COLORS {
                    if old.get(&entry.id) != new.get(&entry.id) {
                        let value = new
                            .get(&entry.id)
                            .map_or_else(|| "default".into(), |c| crate::color_theme::hex(*c));
                        self.execute(&format!("pref.value color.{} {value}", entry.key));
                    }
                }
            }
        }
        for (i, old) in before.entries {
            let pref = &all_prefs()[i];
            let new = (pref.get)(self);
            if new == old {
                continue;
            }
            let _ = (pref.set)(self, old);
            let value = text(pref.ty, &new);
            let value = if value.contains(' ') {
                format!("{{{value}}}")
            } else {
                value
            };
            self.execute(&format!("pref.value {} {value}", pref.key));
        }
    }
}

#[cfg(test)]
pub(crate) fn leaves_of(value: &serde_json::Value, out: &mut Vec<(String, serde_json::Value)>) {
    leaves("", value, out)
}
#[cfg(test)]
pub(crate) fn with_leaf_of(
    value: serde_json::Value,
    path: &str,
    leaf: serde_json::Value,
) -> Option<serde_json::Value> {
    with_leaf(value, path, leaf)
}
#[cfg(test)]
pub(crate) fn map_text_of(key: &str, leaf: &serde_json::Value) -> String {
    map_text(key, leaf)
}
