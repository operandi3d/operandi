//! 3D painting, stage 1 in the app (PNT-01..39): New Image, Add Color /
//! Mask / Displacement Texture, the paint target, the Images window, the
//! image commands and saving images. Built only with `--features
//! painting`; release and tester builds have none of it.
//!
//! Every action is a command line (UIC-160): the menus, the dialogs' OK,
//! the Images window and its right-click menu all run `operandi.image.*`
//! or `operandi.paint.*` lines through `execute`, so a typed line does
//! exactly what a click does. Greying comes from each command's answer
//! (UIC-223). Dialogs are the shared form dialog (UIC-80..99) with values
//! remembered per dialog (UIC-95); each OK is one command and at most one
//! undo step (UIC-96).
//!
//! The command names are Operandi's own (`operandi` flag): the reference
//! name map has none for painting yet (PNT-Q01).
use super::*;
use crate::command_handlers::Commands;
use crate::unavailable::Gate;
use operandi_core::command_registry::{self as table, Area, ArgType, Unit};
use operandi_core::command_runtime::{Call, CommandState, HostHandler};
use operandi_core::paint::{self, NewImage, PaintEdit, TextureKind, pixels};
use std::path::{Path, PathBuf};

/// New Image's sizes are any whole number 64..8192 (PNT-02).
const FORMATS: [&str; 3] = ["Gray", "RGB", "RGBA"];
/// Bake UV Border Size's starting value (provisional, PNT-Q05).
pub(crate) const DEFAULT_UV_BORDER: u32 = 8;
pub(crate) const NOT_YET: &str = "Not yet";

/// What the New Image panel was opened for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NewFor {
    Image,
    Texture(TextureKind),
}

/// One scene's image bookkeeping that isn't in the scene itself. Image ids
/// are per scene, so two tabs can both have image 1: this has to travel
/// with its tab.
#[derive(Default)]
pub(crate) struct DocImages {
    /// Images still to save for Save All, one save panel at a time.
    pub(crate) save_queue: Vec<u64>,
    /// Each image file's modification time when last read or written.
    pub(crate) file_times: BTreeMap<u64, std::time::SystemTime>,
}

/// Painting's state in the app: dialogs, the Images window, preferences.
#[derive(Default)]
pub(crate) struct Painting {
    /// The open New Image panel.
    new_image: Option<(form::Dialog, NewFor)>,
    /// The open Set Resolution dialog and its image.
    set_resolution: Option<(form::Dialog, u64)>,
    /// The image the Images window has selected.
    pub(crate) selected_image: Option<u64>,
    /// The Images window is open.
    pub(crate) images_open: bool,
    /// "Painting works best…" shown already this session (PNT-05).
    warned_shape: bool,
    /// "No texture selected. Add a new texture?" is open (PNT-13).
    pub(crate) ask_add: bool,
    /// Quitting with changed images: the list question is open (PNT-34).
    pub(crate) quit_prompt: bool,
    /// The user answered the changed-images question for this quit.
    pub(crate) quit_decided: bool,
    /// The open scene's own image state; each parked tab keeps its own
    /// (`SceneState::images`, swapped by `exchange`; Astra P9-004/P9-005).
    pub(crate) doc: DocImages,
    /// An image whose file changed on disk while it has unsaved paint.
    pub(crate) reload_ask: Option<u64>,
    /// Frames since the last check for changed files.
    watch_tick: u32,
    pub(crate) prefs: PaintingPrefs,
    /// Stage 2's paint tools (painting_brush.rs).
    pub(crate) brush: crate::painting_brush::BrushState,
    /// "Paint into which layer?": the Image Map layer and the line to run
    /// again once a layer is picked (PNT-304).
    pub(crate) which_layer: Option<(u64, String)>,
    /// The layout whose open command turned on the Sculpt/Paint toolbox
    /// set (PNT-210, 211); the set shows while it is the current layout.
    pub(crate) set_layout: Option<String>,
    /// The open Geometry to Brush dialog (PNT-261).
    pub(crate) geometry_brush: Option<form::Dialog>,
    /// The Image Viewer's image and PSD layer (painting_viewer.rs), and
    /// the last layer picture it made.
    pub(crate) viewer: Option<crate::painting_viewer::ViewerState>,
    pub(crate) viewer_layer: Option<((u64, usize, usize), std::sync::Arc<pixels::PaintImage>)>,
    /// The brush preset strip's presets, and when they were read.
    pub(crate) strip: Option<(std::time::Instant, Vec<crate::painting_layout::StripItem>)>,
    /// Menu snapshot tests (default-build menus) leave painting's out.
    #[cfg(test)]
    pub(crate) hide_menus: bool,
}

/// Preferences > Images & Painting (PNT-19, PNT-26, PNT-33). Not saved
/// between sessions yet (provisional, PNT-Q06).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PaintingPrefs {
    /// Auto Create Textures (PNT-19): painting with no texture offers to
    /// add one. On by default.
    pub(crate) auto_create: bool,
    /// Bake UV Border Size, pixels (Expand UV Seams).
    pub(crate) uv_border: u32,
    /// Scene save's "Also save changed images" (PNT-33), on by default.
    pub(crate) also_save_images: bool,
    /// PNT-214's page (painting_prefs.rs).
    pub(crate) antialiasing: bool,
    pub(crate) new_resolution: u32,
    pub(crate) images_as_icons: bool,
    pub(crate) default_fg: crate::painting_prefs::DefaultFg,
    pub(crate) max_brush: f64,
    pub(crate) default_brush: crate::painting_prefs::DefaultBrush,
    /// Texture Locators still draw while a paint tool is in use (Jason,
    /// 2026-11-18: off by default, they get in the way of the paint).
    pub(crate) show_locators: bool,
}

impl Default for PaintingPrefs {
    fn default() -> Self {
        PaintingPrefs {
            auto_create: true,
            uv_border: DEFAULT_UV_BORDER,
            also_save_images: true,
            antialiasing: true,
            new_resolution: pixels::DEFAULT_SIZE,
            images_as_icons: false,
            default_fg: crate::painting_prefs::DefaultFg::White,
            max_brush: crate::painting_brush::MAX_BRUSH_SIZE,
            default_brush: crate::painting_prefs::DefaultBrush::Tip(
                operandi_core::paint_tips::TipChoice::Smooth,
            ),
            show_locators: false,
        }
    }
}

/// An Images list row: id, name, unsaved, size and format (PNT-216).
type ImageRow = (u64, String, bool, (u32, u32), String);

// ------------------------------------------------------------- commands

const IMAGE_ARG: table::Arg = table::opt("image", ArgType::Text, Unit::None, None);
const NEW_ARGS: &[table::Arg] = &[
    table::opt("resolution", ArgType::Int, Unit::None, Some("2048")),
    table::opt(
        "format",
        ArgType::Choice(&["gray", "rgb", "rgba"]),
        Unit::None,
        Some("rgba"),
    ),
    table::opt("float", ArgType::Bool, Unit::None, Some("false")),
    table::opt("color", ArgType::Text, Unit::None, None),
    table::opt("name", ArgType::Text, Unit::None, None),
];
const TEXTURE_ARGS: &[table::Arg] = &[
    table::arg(
        "kind",
        ArgType::Choice(&["color", "mask", "displacement"]),
        Unit::None,
    ),
    table::opt("resolution", ArgType::Int, Unit::None, Some("2048")),
    table::opt(
        "format",
        ArgType::Choice(&["gray", "rgb", "rgba"]),
        Unit::None,
        Some("rgba"),
    ),
    table::opt("float", ArgType::Bool, Unit::None, Some("false")),
    table::opt("color", ArgType::Text, Unit::None, None),
    table::opt("name", ArgType::Text, Unit::None, None),
    table::opt("layer", ArgType::Int, Unit::None, None),
];
const IMAGE_ARGS: &[table::Arg] = &[IMAGE_ARG];
const SIZE_ARGS: &[table::Arg] = &[
    IMAGE_ARG,
    table::opt("size", ArgType::Int, Unit::None, None),
];
const BORDER_ARGS: &[table::Arg] = &[
    IMAGE_ARG,
    table::opt("border", ArgType::Int, Unit::None, None),
];
const SAVE_AS_ARGS: &[table::Arg] = &[
    IMAGE_ARG,
    table::opt("filename", ArgType::Path, Unit::None, None),
];
const SELECT_ARGS: &[table::Arg] = &[IMAGE_ARG];
const LAYER_ARGS: &[table::Arg] = &[table::opt("layer", ArgType::Int, Unit::None, None)];
const PSD_LAYER_ARGS: &[table::Arg] = &[
    table::arg("layer", ArgType::Int, Unit::None),
    table::opt("name", ArgType::Text, Unit::None, None),
];
const SWITCH_ARGS: &[table::Arg] = &[table::opt("on", ArgType::Bool, Unit::None, None)];
const BORDER_PREF_ARGS: &[table::Arg] = &[table::opt("border", ArgType::Int, Unit::None, None)];

const fn cmd(name: &'static str, help: &'static str) -> table::Entry {
    table::script(name, Area::Shading, help).operandi()
}

/// Every painting command: entry, runner, answer.
/// A command's answer about its state (UIC-223).
type State = fn(&Operandi, &Call<'_>) -> CommandState;

fn commands() -> Vec<(table::Entry, HostHandler<Operandi>, State)> {
    vec![
        (
            cmd("operandi.image.new", "New Image: a blank image to paint (PNT-01..07). Bare opens the panel. Not undoable: images stay when steps are undone.")
                .args(NEW_ARGS)
                .side_effect()
                .example("operandi.image.new resolution:1024 format:rgb float:true"),
            run_new_image,
            always,
        ),
        (
            cmd("operandi.image.newSequence", "New Image Sequence (not built yet).").side_effect(),
            |a, _| a.status = format!("New Image Sequence: {NOT_YET}"),
            |_, _| CommandState::unless(Some(NOT_YET)),
        ),
        (
            cmd("operandi.paint.addTexture", "Add Color / Mask / Displacement Texture: a new image and an Image Map layer for it (PNT-08..12). Bare kind opens the New Image panel. One undo step; the image stays when undone.")
                .args(TEXTURE_ARGS)
                .undo()
                .example("operandi.paint.addTexture kind:color resolution:2048"),
            run_add_texture,
            add_texture_state,
        ),
        (
            cmd("operandi.paint.addFurTexture", "Add Fur … Texture (not built yet).").undo(),
            |a, _| a.status = format!("Fur textures: {NOT_YET}"),
            |_, _| CommandState::unless(Some(NOT_YET)),
        ),
        (
            cmd("operandi.image.invert", "Invert Image: colour becomes 1 − colour, alpha kept (PNT-23). One undo step.")
                .args(IMAGE_ARGS)
                .undo(),
            |a, c| a.image_edit(c, |image, via| PaintEdit::Invert { image, via }),
            image_state,
        ),
        (
            cmd("operandi.image.clear", "Clear Image: every pixel colour 0, alpha 0 (PNT-24). One undo step.")
                .args(IMAGE_ARGS)
                .undo(),
            |a, c| a.image_edit(c, |image, via| PaintEdit::Clear { image, via }),
            image_state,
        ),
        (
            cmd("operandi.image.setResolution", "Set Resolution: resize the image, filtered bilinearly (64..8192, PNT-25). Bare opens the dialog. One undo step.")
                .args(SIZE_ARGS)
                .undo()
                .example("operandi.image.setResolution size:1024"),
            run_set_resolution,
            image_state,
        ),
        (
            cmd("operandi.image.expandSeams", "Expand UV Seams: grow colour out of the active mesh's UV islands by Bake UV Border Size pixels (PNT-26). One undo step.")
                .args(BORDER_ARGS)
                .undo(),
            run_expand_seams,
            image_state,
        ),
        (
            cmd("operandi.image.flatten", "Flatten Layers: the selected Image Map layers (same effect, same size) into one new image layer (PNT-27). One undo step.")
                .undo(),
            run_flatten,
            flatten_state,
        ),
        (
            cmd("operandi.image.toVectorDisplacement", "Convert to Vector Displacement (not built yet).").undo(),
            |a, _| a.status = format!("Convert to Vector Displacement: {NOT_YET}"),
            |_, _| CommandState::unless(Some(NOT_YET)),
        ),
        (
            cmd("operandi.paint.psdLayer", "PSD Layer: which layer of a Photoshop image Image Map layer `layer` shows; no name is the Composite (PNT-302). One undo step.")
                .args(PSD_LAYER_ARGS)
                .undo()
                .example("operandi.paint.psdLayer layer:12 name:\"Layer 1\""),
            run_psd_layer,
            always,
        ),
        (
            cmd("operandi.paint.newPsdLayer", "New Layer: adds \"Operandi Paint 1\" (2, 3…) to the Photoshop image Image Map layer `layer` shows and shows it (PNT-304). One undo step.")
                .args(LAYER_ARGS)
                .undo(),
            run_new_psd_layer,
            always,
        ),
        (
            cmd("operandi.image.save", "Save Image: write the image to its file; an image without one asks where (PNT-31).")
                .args(IMAGE_ARGS)
                .side_effect(),
            run_save,
            image_state,
        ),
        (
            cmd("operandi.image.saveAs", "Save Image As: write the image to a new file and use it from then on (PNT-31).")
                .args(SAVE_AS_ARGS)
                .side_effect(),
            run_save_as,
            image_state,
        ),
        (
            cmd("operandi.image.saveAll", "Save All Images: every changed image (PNT-32).").side_effect(),
            |a, _| a.save_all_images(),
            save_all_state,
        ),
        (
            cmd("operandi.image.reload", "Reload Image: read the image's file again, losing unsaved paint (PNT-307).")
                .args(IMAGE_ARGS)
                .side_effect(),
            run_reload,
            image_state,
        ),
        (
            cmd("operandi.image.select", "Select an image in the Images window.")
                .args(SELECT_ARGS),
            run_select_image,
            always,
        ),
        (
            cmd("operandi.paint.target", "The paint target: select Image Map layer `layer` in the Shader Tree (PNT-13). Bare checks the current one and says what's missing.")
                .args(LAYER_ARGS),
            run_target,
            always,
        ),
        (
            cmd("operandi.paint.images", "Open or close the Images window (PNT-30)."),
            |a, _| a.painting.images_open = !a.painting.images_open,
            |a, _| CommandState::lit(a.painting.images_open),
        ),
        (
            cmd("operandi.paint.alsoSaveImages", "Scene save also saves changed images (PNT-33); on by default.")
                .args(SWITCH_ARGS)
                .query(),
            |a, c| {
                let p = &mut a.painting.prefs;
                p.also_save_images = switch(c.rest, p.also_save_images);
            },
            |a, _| CommandState::lit(a.painting.prefs.also_save_images),
        ),
        (
            cmd("operandi.paint.showLocators", "Texture > Paint Tools > Show Texture Locators while Painting: off by default, so they don't hide the paint.")
                .args(SWITCH_ARGS)
                .query(),
            |a, c| {
                let p = &mut a.painting.prefs;
                p.show_locators = switch(c.rest, p.show_locators);
            },
            |a, _| CommandState::lit(a.painting.prefs.show_locators),
        ),
        (
            cmd("operandi.paint.autoCreate", "Preferences > Images & Painting > Auto Create Textures (PNT-19); on by default.")
                .args(SWITCH_ARGS)
                .query(),
            |a, c| {
                let p = &mut a.painting.prefs;
                p.auto_create = switch(c.rest, p.auto_create);
            },
            |a, _| CommandState::lit(a.painting.prefs.auto_create),
        ),
        (
            cmd("operandi.paint.uvBorder", "Preferences > Images & Painting > Bake UV Border Size, in pixels (Expand UV Seams).")
                .args(BORDER_PREF_ARGS)
                .query(),
            |a, c| match word(c.rest, "border", 0).map(|w| w.parse::<u32>()) {
                Some(Ok(n)) if n <= 256 => a.painting.prefs.uv_border = n,
                Some(_) => a.status = "Bake UV Border Size is 0 to 256 pixels".into(),
                None => {
                    a.status = format!("Bake UV Border Size: {} px", a.painting.prefs.uv_border)
                }
            },
            always,
        ),
    ]
}

/// The registry gets painting's commands (painting builds only).
pub(crate) fn register(r: &mut Commands) {
    crate::painting_brush::register(r);
    crate::painting_inks::register(r);
    crate::painting_layout::register(r);
    crate::painting_viewer::register(r);
    crate::painting_sculpt::register(r);
    for (entry, run, state) in commands() {
        let name = entry.name;
        r.add_host_command(entry, run)
            .unwrap_or_else(|e| panic!("{e}"));
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
}

/// The names of painting's commands (tests, the default-build check).
#[cfg(test)]
pub(crate) fn command_names() -> Vec<&'static str> {
    commands().into_iter().map(|(e, ..)| e.name).collect()
}

fn always(_: &Operandi, _: &Call<'_>) -> CommandState {
    CommandState::default()
}

fn image_state(a: &Operandi, _: &Call<'_>) -> CommandState {
    CommandState::unless(
        a.current_image()
            .is_none()
            .then_some(paint::NO_IMAGE_SELECTED),
    )
}

fn save_all_state(a: &Operandi, _: &Call<'_>) -> CommandState {
    CommandState::unless(
        a.editor
            .changed_images()
            .is_empty()
            .then_some("No image has unsaved changes"),
    )
}

fn add_texture_state(a: &Operandi, c: &Call<'_>) -> CommandState {
    let mask = word(c.rest, "kind", 0).is_some_and(|k| k.eq_ignore_ascii_case("mask"));
    CommandState::unless(
        (mask && a.shading.selected.is_empty()).then_some("Select the layer to mask first"),
    )
}

fn flatten_state(a: &Operandi, _: &Call<'_>) -> CommandState {
    let images = a
        .shading
        .selected
        .iter()
        .filter(|&&id| paint::layer_image(&a.editor.scene, id).is_some())
        .count();
    CommandState::unless((images < 2).then_some(paint::FLATTEN_REFUSAL))
}

/// The value of argument `name` in `rest`: `name:value` anywhere, else
/// the `index`th word without a name.
pub(crate) fn word(rest: &str, name: &str, index: usize) -> Option<String> {
    let words = operandi_core::command_line::tokens(rest).ok()?;
    if let Some(v) = words
        .iter()
        .find_map(|w| match operandi_core::command_line::named(w) {
            (Some(n), v) if n.eq_ignore_ascii_case(name) => Some(v.to_owned()),
            _ => None,
        })
    {
        return Some(v);
    }
    words
        .iter()
        .filter(|w| operandi_core::command_line::named(w).0.is_none())
        .nth(index)
        .cloned()
}

/// An on/off argument: `on:true`, `true`, `on`, `off`; none toggles.
fn switch(rest: &str, now: bool) -> bool {
    match word(rest, "on", 0)
        .as_deref()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("true" | "on" | "1" | "yes") => true,
        Some("false" | "off" | "0" | "no") => false,
        _ => !now,
    }
}

/// New Image's settings from a line's words (unnamed: the defaults).
fn new_image_from(a: &Operandi, rest: &str) -> std::result::Result<NewImage, String> {
    let mut s = NewImage {
        resolution: a.painting.prefs.new_resolution,
        antialiasing: a.painting.prefs.antialiasing,
        ..NewImage::default()
    };
    if let Some(r) = word(rest, "resolution", usize::MAX) {
        s.resolution = r
            .trim()
            .parse()
            .map_err(|_| "resolution is a whole number of pixels".to_owned())?;
    }
    if let Some(f) = word(rest, "format", usize::MAX) {
        s.format = pixels::Format::named(&f).ok_or("format is gray, rgb or rgba")?;
    }
    if let Some(f) = word(rest, "float", usize::MAX) {
        s.float = operandi_core::command_args::boolean("float", &f)?;
    }
    if let Some(c) = word(rest, "color", usize::MAX) {
        let parts: Vec<f32> = c
            .split(',')
            .map(|p| p.trim().parse::<f32>())
            .collect::<std::result::Result<_, _>>()
            .map_err(|_| "color is r,g,b (0 to 1 each)".to_owned())?;
        let [r, g, b] = parts[..] else {
            return Err("color is r,g,b (0 to 1 each)".into());
        };
        s.color = Some([r, g, b]);
    }
    s.name = word(rest, "name", usize::MAX);
    pixels::check_side(s.resolution)?;
    Ok(s)
}

/// Whether a line names any New Image setting (else it opens the panel).
fn has_image_words(rest: &str) -> bool {
    ["resolution", "format", "float", "color", "name"]
        .iter()
        .any(|k| word(rest, k, usize::MAX).is_some())
}

fn run_new_image(a: &mut Operandi, c: &Call<'_>) {
    if c.rest.is_empty() {
        a.open_new_image(NewFor::Image);
        return;
    }
    match new_image_from(a, c.rest) {
        Ok(s) => {
            let shape = s.resolution.is_power_of_two();
            if a.run_ok(Command::Paint(PaintEdit::NewImage(s))) {
                a.painting.selected_image = a.editor.paint.last_image;
                a.status = "New image (not saved yet)".into();
                a.shape_note(shape);
            }
        }
        Err(e) => a.status = e,
    }
}

fn run_add_texture(a: &mut Operandi, c: &Call<'_>) {
    let Some(kind) = word(c.rest, "kind", 0).and_then(|k| TextureKind::named(&k)) else {
        a.status = "Usage: operandi.paint.addTexture kind:<color|mask|displacement>".into();
        return;
    };
    if !has_image_words(c.rest) {
        a.open_new_image(NewFor::Texture(kind));
        return;
    }
    let settings = match new_image_from(a, c.rest) {
        Ok(s) => s,
        Err(e) => {
            a.status = e;
            return;
        }
    };
    let selected = match word(c.rest, "layer", usize::MAX) {
        Some(l) => match l.parse() {
            Ok(id) => Some(id),
            Err(_) => {
                a.status = "layer is a shader tree layer id".into();
                return;
            }
        },
        // Only a layer this scene has (a closed scene's selection can
        // linger).
        None => a
            .shading
            .selected
            .last()
            .copied()
            .filter(|&id| a.editor.scene.shader_tree.get(id).is_some()),
    };
    let shape = settings.resolution.is_power_of_two();
    if a.run_ok(Command::Paint(PaintEdit::AddTexture {
        kind,
        image: settings,
        selected,
        item: None,
    })) {
        // The new layer is the paint target (PNT-10).
        if let Some(layer) = a.editor.paint.last_layer {
            a.shading.selected = vec![layer];
        }
        a.painting.selected_image = a.editor.paint.last_image;
        a.status = format!("{} (image not saved yet)", kind.label());
        a.shape_note(shape);
    }
}

fn run_set_resolution(a: &mut Operandi, c: &Call<'_>) {
    let Some(image) = a.image_from(c.rest) else {
        return;
    };
    match word(c.rest, "size", usize::MAX) {
        None => a.open_set_resolution(image),
        Some(s) => match s.parse::<u32>() {
            Ok(size) => {
                if a.run_ok(Command::Paint(PaintEdit::SetResolution {
                    image,
                    width: size,
                    height: size,
                })) {
                    a.status = format!("Image is now {size} × {size}");
                    a.shape_note(size.is_power_of_two());
                }
            }
            Err(_) => a.status = "size is a whole number of pixels".into(),
        },
    }
}

fn run_expand_seams(a: &mut Operandi, c: &Call<'_>) {
    let Some(image) = a.image_from(c.rest) else {
        return;
    };
    let border = match word(c.rest, "border", usize::MAX).map(|b| b.parse::<u32>()) {
        None => a.painting.prefs.uv_border,
        Some(Ok(b)) => b,
        Some(Err(_)) => {
            a.status = "border is a whole number of pixels".into();
            return;
        }
    };
    // The UV map of the layer showing the image, else the mesh's own.
    let uv_map = a.layer_uv_map(image);
    let via = a.paint_target();
    if a.run_ok(Command::Paint(PaintEdit::ExpandSeams {
        image,
        via,
        item: None,
        uv_map,
        border,
    })) {
        a.status = format!("Expanded UV seams by {border} px");
    } else {
        a.ask_which_layer(via, c.line);
    }
}

fn run_flatten(a: &mut Operandi, _: &Call<'_>) {
    let layers = a.shading.selected.clone();
    if a.run_ok(Command::Paint(PaintEdit::Flatten { layers })) {
        if let Some(layer) = a.editor.paint.last_layer {
            a.shading.selected = vec![layer];
        }
        a.painting.selected_image = a.editor.paint.last_image;
        a.status = "Flattened into a new image layer (not saved yet)".into();
    }
}

/// The line's `layer:` (else the paint target), as an Image Map layer.
fn image_map_layer(a: &mut Operandi, rest: &str) -> Option<u64> {
    let layer = match word(rest, "layer", usize::MAX) {
        Some(l) => l.parse().ok(),
        None => a.paint_target(),
    };
    let layer = layer.filter(|&l| paint::layer_image(&a.editor.scene, l).is_some());
    if layer.is_none() {
        a.status = "Name an Image Map layer with an image: layer:<id>".into();
    }
    layer
}

fn run_psd_layer(a: &mut Operandi, c: &Call<'_>) {
    let Some(layer) = image_map_layer(a, c.rest) else {
        return;
    };
    let name = word(c.rest, "name", usize::MAX).filter(|n| !n.is_empty() && n != "Composite");
    let mut scene = a.editor.scene.clone();
    if let Err(e) = paint::set_psd_layer(&mut scene, layer, name) {
        a.status = e;
        return;
    }
    let node = scene.shader_tree.get(layer).expect("checked").clone();
    a.run(Command::ShaderTree(
        operandi_core::shader_tree::ShaderTreeEdit::Replace {
            node,
            continuing: false,
        },
    ));
}

fn run_new_psd_layer(a: &mut Operandi, c: &Call<'_>) {
    if let Some(via) = image_map_layer(a, c.rest) {
        a.run(Command::Paint(PaintEdit::NewPsdLayer { via }));
    }
}

fn run_save(a: &mut Operandi, c: &Call<'_>) {
    if let Some(image) = a.image_from(c.rest) {
        a.save_image_now(image);
    }
}

fn run_save_as(a: &mut Operandi, c: &Call<'_>) {
    let Some(image) = a.image_from(c.rest) else {
        return;
    };
    match word(c.rest, "filename", usize::MAX) {
        Some(path) => {
            a.save_image_to(image, Path::new(&path));
        }
        None => a.ask_image_file(image),
    }
}

fn run_reload(a: &mut Operandi, c: &Call<'_>) {
    let Some(image) = a.image_from(c.rest) else {
        return;
    };
    match a.editor.reload_image(image) {
        Ok(()) => {
            a.painting.doc.file_times.remove(&image);
            a.note_file_time(image);
            a.status = "Image reloaded from disk".into();
        }
        Err(e) => a.status = e,
    }
}

fn run_select_image(a: &mut Operandi, c: &Call<'_>) {
    let Some(name) = word(c.rest, "image", 0) else {
        a.status = "Usage: operandi.image.select image:<name or id>".into();
        return;
    };
    match a.find_image(&name) {
        Some(id) => a.painting.selected_image = Some(id),
        None => a.status = format!("No image called {}", c.rest.trim()),
    }
}

fn run_target(a: &mut Operandi, c: &Call<'_>) {
    if let Some(l) = word(c.rest, "layer", 0) {
        let Ok(id) = l.parse::<u64>() else {
            a.status = "layer is a shader tree layer id".into();
            return;
        };
        if a.editor.scene.shader_tree.get(id).is_none() {
            a.status = format!("No shader tree layer {id}");
            return;
        }
        a.shading.selected = vec![id];
        a.shading.environment = None;
    }
    a.check_paint_target();
}

// ------------------------------------------------------------- app side

impl Operandi {
    /// The paint target: the Shader Tree's selected Image Map layer
    /// (PNT-13), the last one clicked.
    pub(crate) fn paint_target(&self) -> Option<u64> {
        let scene = &self.editor.scene;
        self.shading
            .selected
            .iter()
            .rev()
            .copied()
            .find(|&id| paint::layer_image(scene, id).is_some())
            // None selected: the top Image Map layer with an image, so a
            // texture applied any way can be painted (provisional,
            // PNT-Q40).
            .or_else(|| {
                scene
                    .shader_tree
                    .rows(&|_| false)
                    .into_iter()
                    .map(|(n, _)| n.id)
                    .find(|&id| paint::layer_image(scene, id).is_some())
            })
    }

    /// The image commands' image: the Images window's selection, else the
    /// paint target's.
    pub(crate) fn current_image(&self) -> Option<u64> {
        self.painting
            .selected_image
            .filter(|id| self.editor.scene.images.iter().any(|i| i.id == *id))
            .or_else(|| {
                self.paint_target()
                    .and_then(|l| paint::layer_image(&self.editor.scene, l))
            })
    }

    /// An image command on the line's image (or the current one).
    fn image_edit(&mut self, c: &Call<'_>, edit: impl FnOnce(u64, Option<u64>) -> PaintEdit) {
        if let Some(image) = self.image_from(c.rest) {
            let via = self.paint_target();
            if !self.run_ok(Command::Paint(edit(image, via))) {
                self.ask_which_layer(via, c.line);
            }
        }
    }

    /// A Photoshop image on its Composite: "Paint into which layer?"
    /// (PNT-304), then `line` runs again on the chosen layer.
    fn ask_which_layer(&mut self, via: Option<u64>, line: &str) {
        if self.status == paint::PSD_WHICH_LAYER
            && let Some(via) = via
        {
            self.painting.which_layer = Some((via, line.to_owned()));
        }
    }

    /// An image by id or name.
    fn find_image(&self, word: &str) -> Option<u64> {
        let word = word.trim().trim_matches('"');
        let images = &self.editor.scene.images;
        word.parse::<u64>()
            .ok()
            .filter(|id| images.iter().any(|i| i.id == *id))
            .or_else(|| images.iter().find(|i| i.name == word).map(|i| i.id))
    }

    /// The line's `image:` or the current image; says why when neither.
    pub(crate) fn image_from(&mut self, rest: &str) -> Option<u64> {
        match word(rest, "image", usize::MAX) {
            Some(w) => self.find_image(&w).or_else(|| {
                self.status = format!("No image called {w}");
                None
            }),
            None => self.current_image().or_else(|| {
                self.status = paint::NO_IMAGE_SELECTED.into();
                None
            }),
        }
    }

    /// The UV map of the first Image Map layer showing `image`.
    fn layer_uv_map(&self, image: u64) -> Option<String> {
        let scene = &self.editor.scene;
        scene.shader_tree.nodes.iter().find_map(|n| match &n.kind {
            operandi_core::shader_tree::NodeKind::Texture(t)
                if paint::layer_image(scene, n.id) == Some(image) =>
            {
                t.locator
                    .and_then(|l| scene.texture_locator(l))
                    .and_then(|l| l.uv_map.clone())
            }
            _ => None,
        })
    }

    /// PNT-05: once a session, after a size painting handles less well.
    fn shape_note(&mut self, square_pow2: bool) {
        if !square_pow2 && !self.painting.warned_shape {
            self.painting.warned_shape = true;
            self.status = pixels::SQUARE_POW2_NOTE.into();
        }
    }

    /// PNT-13..18: says what stops painting on the target, or offers to
    /// add a texture when there is none (Auto Create Textures).
    pub(crate) fn check_paint_target(&mut self) {
        let layer = self
            .paint_target()
            .or(self.shading.selected.last().copied());
        match paint::paint_target_problem(&self.editor.scene, layer) {
            None => {
                let image = layer.and_then(|l| paint::layer_image(&self.editor.scene, l));
                let big = image
                    .and_then(|id| self.editor.scene.images.iter().find(|i| i.id == id))
                    .is_some_and(|i| i.width.max(i.height) > crate::gpu_scene::MAX_IMAGE_SIDE);
                self.status = if big {
                    // PNT-18: paintable, but the viewport shows it smaller.
                    format!(
                        "The image is larger than the viewport shows ({} px); painting works on the full image",
                        crate::gpu_scene::MAX_IMAGE_SIDE
                    )
                } else {
                    "Ready to paint".into()
                };
            }
            Some(paint::NO_TEXTURE) if self.painting.prefs.auto_create => {
                self.painting.ask_add = true;
            }
            Some(problem) => self.status = problem.into(),
        }
    }

    pub(crate) fn open_new_image(&mut self, purpose: NewFor) {
        if self.has_modal() {
            self.status = "Close the open dialog before New Image".into();
            return;
        }
        use form::{Row, Unit as U, When};
        let rows = vec![
            Row::text("name", "Name", "Image").tip("The image's name in the Images list"),
            Row::number(
                "resolution",
                "Resolution",
                U::Suffix(" px"),
                f64::from(self.painting.prefs.new_resolution),
            )
            .range(pixels::MIN_SIZE as f64, pixels::MAX_SIZE as f64)
            .tip("Width and height, in pixels"),
            Row::choice("format", "Format", &FORMATS, 2).tip("Gray, RGB, or RGB with alpha"),
            Row::check("float", "Floating Point", false)
                .tip("32-bit floating point pixels, saved as OpenEXR"),
            Row::check("set_color", "Set Color", false)
                .tip("Fill with Color; off leaves every pixel colour 0 and alpha 0"),
            Row::colour("color", "Color", [1., 1., 1.])
                .when(When::Checked("set_color"))
                .tip("Turn on Set Color to fill the image with a colour"),
        ]
        .into_iter()
        .map(form::Row::fresh)
        .collect();
        let title = match purpose {
            NewFor::Image => "New Image",
            NewFor::Texture(k) => k.label(),
        };
        self.painting.new_image = Some((self.form_dialog(title, rows), purpose));
    }

    fn open_set_resolution(&mut self, image: u64) {
        use form::{Row, Unit as U};
        let now = self
            .editor
            .scene
            .images
            .iter()
            .find(|i| i.id == image)
            .map_or(pixels::DEFAULT_SIZE, |i| i.width);
        let rows = vec![
            Row::number("size", "Resolution", U::Suffix(" px"), now as f64)
                .range(pixels::MIN_SIZE as f64, pixels::MAX_SIZE as f64)
                .tip("The new width and height, in pixels"),
        ];
        self.painting.set_resolution = Some((
            form::Dialog::new(
                "Set Resolution",
                rows.clone(),
                form::Values::defaults(&rows),
            ),
            image,
        ));
    }

    #[cfg(test)]
    pub(crate) fn close_new_image_for_test(&mut self) {
        self.painting.new_image = None;
    }
}

impl Operandi {
    /// Painting's dialogs and windows, once a frame.
    pub(crate) fn painting_frame(&mut self, ctx: &Context) {
        // `alt_held` reads the frame's modifiers through it (the app sets
        // it in `update` too; test frames skip that).
        if self.frame_ctx.is_none() {
            self.frame_ctx = Some(ctx.clone());
        }
        if let Some((mut dialog, purpose)) = self.painting.new_image.take() {
            match self.show_form(ctx, &mut dialog) {
                form::Outcome::Ok => {
                    let v = dialog.values.clone();
                    let mut line = match purpose {
                        NewFor::Image => "operandi.image.new".to_owned(),
                        NewFor::Texture(k) => {
                            format!("operandi.paint.addTexture kind:{}", k.word())
                        }
                    };
                    let name = v.text("name").trim().to_owned();
                    line += &format!(
                        " resolution:{} format:{} float:{}",
                        v.number("resolution").round(),
                        FORMATS[v.choice("format").min(2)].to_ascii_lowercase(),
                        v.bool("float")
                    );
                    if v.bool("set_color") {
                        let c = v.colour("color");
                        line += &format!(" color:{},{},{}", c[0], c[1], c[2]);
                    }
                    if !name.is_empty() {
                        line += &format!(" name:\"{name}\"");
                    }
                    self.execute(&line);
                }
                form::Outcome::Cancel => self.status = "New Image canceled; nothing changed".into(),
                _ => self.painting.new_image = Some((dialog, purpose)),
            }
        }
        if let Some((mut dialog, image)) = self.painting.set_resolution.take() {
            match self.show_form(ctx, &mut dialog) {
                form::Outcome::Ok => {
                    let size = dialog.values.number("size").round();
                    self.execute(&format!(
                        "operandi.image.setResolution image:{image} size:{size}"
                    ));
                }
                form::Outcome::Cancel => {
                    self.status = "Set Resolution canceled; nothing changed".into()
                }
                _ => self.painting.set_resolution = Some((dialog, image)),
            }
        }
        if let Some(mut dialog) = self.painting.geometry_brush.take() {
            match self.show_form(ctx, &mut dialog) {
                form::Outcome::Ok => {
                    let v = dialog.values.clone();
                    self.execute(&format!(
                        "operandi.paint.geometryToBrush mode:{} resolution:{} axis:{} iterations:{}",
                        ["height", "vector"][v.choice("mode").min(1)],
                        v.number("resolution").round(),
                        ["auto", "x", "y", "z"][v.choice("axis").min(3)],
                        v.number("iterations").round()
                    ));
                }
                form::Outcome::Cancel => {
                    self.status = "Geometry to Brush canceled; nothing changed".into()
                }
                _ => self.painting.geometry_brush = Some(dialog),
            }
        }
        if self.painting.ask_add {
            match self.show_confirm(ctx, "Paint", paint::NO_TEXTURE, &[]) {
                form::Outcome::Ok => {
                    self.painting.ask_add = false;
                    self.execute("operandi.paint.addTexture kind:color");
                }
                form::Outcome::Open => {}
                _ => self.painting.ask_add = false,
            }
        }
        if let Some(image) = self.painting.reload_ask {
            let name = self.image_name(image);
            let message =
                format!("{name} changed on disk. Reload from disk and lose unsaved paint?");
            match self.show_confirm(ctx, "Image Changed", &message, &[]) {
                form::Outcome::Ok => {
                    self.painting.reload_ask = None;
                    self.execute(&format!("operandi.image.reload image:{image}"));
                }
                form::Outcome::Open => {}
                _ => {
                    // Keep the paint; ask again only after the next change.
                    self.painting.reload_ask = None;
                    self.note_file_time(image);
                }
            }
        }
        if self.painting.quit_prompt {
            self.quit_images_prompt(ctx);
        }
        self.which_layer_question(ctx);
        if self.category == crate::toolbox::Category::Paint {
            self.painting.brush.tab_shown = true;
        }
        self.warm_paint_surface();
        self.images_window(ctx);
        self.preset_strip(ctx);
        // A mask brush's Vertex Map Tool picked on its own isn't a mask brush.
        if self.sculpt.brush != self.painting.brush.mask_brush {
            self.painting.brush.mask_brush = None;
        }
        self.paint_viewer_window(ctx);
        self.watch_image_files();
    }

    /// "Paint into which layer?" (PNT-304): the Photoshop image's pixel
    /// layers (text, adjustment, smart object and group rows greyed) and
    /// New Layer.
    fn which_layer_question(&mut self, ctx: &Context) {
        let Some((via, line)) = self.painting.which_layer.clone() else {
            return;
        };
        let rows = paint::layer_image(&self.editor.scene, via)
            .and_then(|image| self.editor.psd_rows(image).ok())
            .unwrap_or_default();
        self.modal_owned_input = true;
        self.editor_owned_input = true;
        let mut picked: Option<Option<String>> = None;
        let mut cancel = false;
        crate::dialog_place::modal(ctx, egui::Id::new("paint-which-layer")).show(ctx, |ui| {
            ui.heading(paint::PSD_WHICH_LAYER);
            ui.separator();
            for row in rows.iter().skip(1) {
                let label = format!("{}{}", "    ".repeat(row.indent), row.name);
                let why = (!row.paintable()).then_some(match row.kind {
                    pixels::psd::LayerKind::Group => "A group has no pixels of its own",
                    pixels::psd::LayerKind::Text => "Text layers can't be painted",
                    pixels::psd::LayerKind::Adjustment => "Adjustment layers can't be painted",
                    pixels::psd::LayerKind::Smart => "Smart objects can't be painted",
                    _ => "This layer has no pixels to paint",
                });
                if ui.gated(why, egui::Button::new(label)).clicked() {
                    picked = Some(Some(row.name.clone()));
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("New Layer").clicked() {
                    picked = Some(None);
                }
                if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    cancel = true;
                }
            });
        });
        if cancel {
            self.painting.which_layer = None;
            self.status = "Nothing painted".into();
            return;
        }
        if let Some(choice) = picked {
            self.painting.which_layer = None;
            match choice {
                Some(name) => self.execute(&format!(
                    "operandi.paint.psdLayer layer:{via} name:\"{name}\""
                )),
                None => self.execute(&format!("operandi.paint.newPsdLayer layer:{via}")),
            }
            // A paint stroke's press asked: the next press paints.
            if !line.is_empty() {
                self.execute(&line);
            }
        }
    }

    /// The PSD Layer row of an Image Map with a Photoshop image (PNT-302):
    /// Composite, then every layer, groups as headers.
    pub(crate) fn psd_layer_row(&mut self, ui: &mut Ui, layer: u64) {
        let Some(image) = paint::layer_image(&self.editor.scene, layer) else {
            return;
        };
        let is_psd = self
            .editor
            .scene
            .images
            .iter()
            .find(|i| i.id == image)
            .and_then(|i| pixels::FileType::of(Path::new(&i.path)))
            .is_some_and(pixels::FileType::is_psd);
        if !is_psd {
            return;
        }
        let rows = match self.editor.psd_rows(image) {
            Ok(r) => r,
            Err(e) => {
                ui.weak(e);
                return;
            }
        };
        let now = paint::psd_layer_of(&self.editor.scene, layer);
        let mut line = None;
        crate::option_row::field_row(ui, "PSD Layer", |ui| {
            egui::ComboBox::from_id_salt("psd-layer")
                .selected_text(now.clone().unwrap_or_else(|| "Composite".into()))
                .show_ui(ui, |ui| {
                    for row in &rows {
                        let label = format!("{}{}", "    ".repeat(row.indent), row.name);
                        let composite = row.pick == pixels::psd::Pick::Composite;
                        if row.kind == pixels::psd::LayerKind::Group {
                            // Group rows are headers.
                            ui.weak(label);
                            continue;
                        }
                        let on = if composite {
                            now.is_none()
                        } else {
                            now.as_deref() == Some(&row.name)
                        };
                        if ui.selectable_label(on, label).clicked() {
                            line = Some(if composite {
                                format!("operandi.paint.psdLayer layer:{layer}")
                            } else {
                                format!(
                                    "operandi.paint.psdLayer layer:{layer} name:\"{}\"",
                                    row.name
                                )
                            });
                        }
                    }
                });
        });
        if let Some(line) = line {
            self.execute(&line);
        }
    }

    /// A painting dialog or question is open (it owns the keyboard).
    pub(crate) fn painting_dialog_open(&self) -> bool {
        let p = &self.painting;
        p.new_image.is_some()
            || p.set_resolution.is_some()
            || p.ask_add
            || p.quit_prompt
            || p.reload_ask.is_some()
            || p.which_layer.is_some()
    }

    pub(crate) fn image_name(&self, id: u64) -> String {
        self.editor
            .scene
            .images
            .iter()
            .find(|i| i.id == id)
            .map_or_else(|| "Image".into(), |i| i.name.clone())
    }

    /// Where new images are saved first: the project's images folder, the
    /// scene's folder, else home (as bakes are).
    fn image_folder(&self) -> PathBuf {
        self.texture_folder()
    }

    /// Save Image: to its file, or ask for one.
    pub(crate) fn save_image_now(&mut self, image: u64) -> bool {
        let path = self
            .editor
            .scene
            .images
            .iter()
            .find(|i| i.id == image)
            .map(|i| i.path.clone())
            .unwrap_or_default();
        if path.is_empty() {
            self.ask_image_file(image);
            false
        } else {
            self.save_image_to(image, Path::new(&path))
        }
    }

    /// The save panel for image `image` (first save, Save As).
    fn ask_image_file(&mut self, image: u64) {
        let depth = self
            .editor
            .scene
            .paint
            .buffers
            .get(&image)
            .map_or(pixels::Depth::U8, |b| b.depth);
        let ext = pixels::FileType::default_for(depth).extension();
        let name = format!("{}.{ext}", self.image_name(image));
        let request = file_chooser::Choose::SavePaintImage {
            folder: self.image_folder(),
            name,
        };
        if !self.choose(request, file_chooser::Purpose::PaintImage(image)) {
            self.status = format!(
                "Save Image: no file panel here; type operandi.image.saveAs image:{image} filename:<file.{ext}>"
            );
        }
    }

    /// Writes image `image` to `path`; the scene keeps the new file name.
    pub(crate) fn save_image_to(&mut self, image: u64, path: &Path) -> bool {
        let path = if pixels::FileType::of(path).is_none() {
            let depth = self
                .editor
                .scene
                .paint
                .buffers
                .get(&image)
                .map_or(pixels::Depth::U8, |b| b.depth);
            path.with_extension(pixels::FileType::default_for(depth).extension())
        } else {
            path.to_path_buf()
        };
        match self.editor.save_image(image, &path) {
            Ok(()) => {
                // Saving an image isn't a scene change of its own: the
                // saved-scene record takes the new file name and pixels.
                if let Some(i) = self.editor.scene.images.iter().find(|i| i.id == image) {
                    let i = i.clone();
                    match self.saved.images.iter_mut().find(|s| s.id == image) {
                        Some(s) => *s = i,
                        None => self.saved.images.push(i),
                    }
                }
                if let Some(b) = self.editor.scene.paint.buffers.get(&image) {
                    self.saved.paint.buffers.insert(image, b.clone());
                }
                self.note_file_time(image);
                self.status = format!("Saved {}", path.display());
                true
            }
            Err(e) => {
                self.report_file_error("Save Image", format!("The image was not saved.\n\n{e}"));
                self.status = e;
                false
            }
        }
    }

    /// Save All Images: those with files at once, then a save panel for
    /// each new one in turn.
    pub(crate) fn save_all_images(&mut self) {
        let changed = self.editor.changed_images();
        if changed.is_empty() {
            self.status = "No image has unsaved changes".into();
            return;
        }
        let mut saved = 0;
        let mut queue = Vec::new();
        for id in changed {
            let has_file = self
                .editor
                .scene
                .images
                .iter()
                .any(|i| i.id == id && !i.path.is_empty());
            if has_file {
                saved += usize::from(self.save_image_now(id));
            } else {
                queue.push(id);
            }
        }
        self.status = format!("Saved {saved} image(s)");
        self.painting.doc.save_queue = queue;
        self.next_queued_save();
    }

    fn next_queued_save(&mut self) {
        if let Some(id) = self.painting.doc.save_queue.first().copied() {
            self.painting.doc.save_queue.remove(0);
            self.ask_image_file(id);
        }
    }

    /// The save panel's answer for image `image`.
    pub(crate) fn paint_image_chosen(&mut self, image: u64, path: PathBuf) {
        self.save_image_to(image, &path);
        self.next_queued_save();
    }

    /// After a scene save (PNT-33): also save changed images, when on.
    pub(crate) fn after_scene_save(&mut self) {
        if self.painting.prefs.also_save_images && !self.editor.changed_images().is_empty() {
            self.save_all_images();
        }
    }

    /// Quitting: changed images are listed first (PNT-34). Returns true
    /// while the question stands (the quit waits).
    pub(crate) fn quit_waits_for_images(&mut self) -> bool {
        if self.painting.quit_decided || self.editor.changed_images().is_empty() {
            return false;
        }
        self.painting.quit_prompt = true;
        true
    }

    fn quit_images_prompt(&mut self, ctx: &Context) {
        let names: Vec<String> = self
            .editor
            .changed_images()
            .into_iter()
            .map(|id| format!("  {}", self.image_name(id)))
            .collect();
        let message = format!(
            "These images have unsaved changes:\n\n{}\n\nSave them before quitting?",
            names.join("\n")
        );
        match self.show_confirm(ctx, "Unsaved Images", &message, &["Don't Save"]) {
            form::Outcome::Ok => {
                self.painting.quit_prompt = false;
                self.save_all_images();
                // New images wait for their save panels; quit only when
                // every image is saved.
                if self.editor.changed_images().is_empty() {
                    self.painting.quit_decided = true;
                    self.request_file(FileAction::Close);
                }
            }
            form::Outcome::Extra(_) => {
                self.painting.quit_prompt = false;
                self.painting.quit_decided = true;
                self.request_file(FileAction::Close);
            }
            form::Outcome::Cancel | form::Outcome::No => self.painting.quit_prompt = false,
            form::Outcome::Open => {}
        }
    }

    /// Auto-save: changed images are backed up beside the scene's backup
    /// (PNT-35) as `<backup stem> <image>.<ext>`.
    pub(crate) fn autosave_images(&mut self, backup: &Path) {
        let ids = self.editor.changed_images();
        let stem = backup
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "autosave".into());
        for id in ids {
            let Some(px) = self.editor.scene.paint.buffers.get(&id).cloned() else {
                continue;
            };
            let ext = pixels::FileType::default_for(px.depth).extension();
            let name = self.image_name(id).replace(['/', '\\', ':'], "_");
            let path = backup.with_file_name(format!("{stem} {name}.{ext}"));
            if let Err(e) = pixels::io::save(&px, &path) {
                self.status = format!("Auto-save couldn't back up {name}: {e}");
            }
        }
    }

    /// The file's modification time is the one last read or written.
    fn note_file_time(&mut self, image: u64) {
        if let Some(t) = self.image_file_time(image) {
            self.painting.doc.file_times.insert(image, t);
        }
    }

    fn image_file_time(&self, image: u64) -> Option<std::time::SystemTime> {
        let path = &self
            .editor
            .scene
            .images
            .iter()
            .find(|i| i.id == image)?
            .path;
        if path.is_empty() {
            return None;
        }
        std::fs::metadata(path).and_then(|m| m.modified()).ok()
    }

    /// PNT-307: a painted image's file changed outside Operandi: reload it
    /// when it has no unsaved paint, else ask. Checked about once a second.
    fn watch_image_files(&mut self) {
        self.painting.watch_tick += 1;
        if self.painting.watch_tick < 60 || self.painting.reload_ask.is_some() {
            return;
        }
        self.painting.watch_tick = 0;
        let ids: Vec<u64> = self.editor.scene.paint.buffers.keys().copied().collect();
        for id in ids {
            let Some(now) = self.image_file_time(id) else {
                continue;
            };
            match self.painting.doc.file_times.get(&id) {
                None => {
                    self.painting.doc.file_times.insert(id, now);
                }
                Some(&then) if then != now => {
                    if self.editor.image_changed(id) {
                        self.painting.reload_ask = Some(id);
                        return;
                    }
                    let name = self.image_name(id);
                    self.execute(&format!("operandi.image.reload image:{id}"));
                    self.status = format!("{name} changed on disk and was reloaded");
                }
                Some(_) => {}
            }
        }
    }

    /// The Images window (PNT-30): every scene image, "*" after a changed
    /// one; click selects, right-click saves and edits. The Texture picker
    /// at its top picks the paint target (PNT-13).
    fn images_window(&mut self, ctx: &Context) {
        if !self.painting.images_open {
            return;
        }
        let mut open = true;
        let mut lines: Vec<String> = Vec::new();
        // PNT-216: name, unsaved mark, resolution and format.
        let images: Vec<ImageRow> = self
            .editor
            .scene
            .images
            .iter()
            .map(|i| {
                (
                    i.id,
                    i.name.clone(),
                    self.editor.image_changed(i.id),
                    (i.width, i.height),
                    self.editor
                        .scene
                        .paint
                        .buffers
                        .get(&i.id)
                        .map_or_else(String::new, |px| crate::painting_viewer::format_text(px)),
                )
            })
            .collect();
        let icons = self.painting.prefs.images_as_icons;
        let buffers = self.editor.scene.paint.buffers.clone();
        let targets: Vec<(u64, String)> = self
            .editor
            .scene
            .shader_tree
            .nodes
            .iter()
            .filter(|n| paint::layer_image(&self.editor.scene, n.id).is_some())
            .map(|n| (n.id, n.name.clone()))
            .collect();
        let target = self.paint_target();
        let selected = self.current_image();
        let psd_images: Vec<u64> = self
            .editor
            .scene
            .images
            .iter()
            .filter(|i| {
                pixels::FileType::of(Path::new(&i.path)).is_some_and(pixels::FileType::is_psd)
            })
            .map(|i| i.id)
            .collect();
        egui::Window::new("Images")
            .open(&mut open)
            .default_width(260.)
            .show(ctx, |ui| {
                crate::option_row::field_row(ui, "Texture", |ui| {
                    let shown = target
                        .and_then(|t| targets.iter().find(|(id, _)| *id == t))
                        .map_or("(none)".to_owned(), |(_, n)| n.clone());
                    egui::ComboBox::from_id_salt("paint-texture")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            for (id, name) in &targets {
                                if ui.selectable_label(target == Some(*id), name).clicked() {
                                    lines.push(format!("operandi.paint.target layer:{id}"));
                                }
                            }
                        });
                });
                ui.separator();
                if images.is_empty() {
                    ui.weak("No images yet: File > New Image…");
                }
                if icons && !images.is_empty() {
                    // Always Display Images as Icons (PNT-214).
                    ui.horizontal_wrapped(|ui| {
                        for (id, name, changed, _, _) in &images {
                            let r = ui
                                .vertical(|ui| {
                                    ui.set_width(72.);
                                    let (rect, r) = ui
                                        .allocate_exact_size(vec2(64., 64.), egui::Sense::click());
                                    match buffers.get(id) {
                                        Some(px) => {
                                            let t = crate::painting_viewer::texture_of(
                                                ui.ctx(),
                                                "paint-image-icon",
                                                px,
                                            );
                                            ui.painter().image(
                                                t.id(),
                                                rect,
                                                Rect::from_min_max(pos2(0., 0.), pos2(1., 1.)),
                                                egui::Color32::WHITE,
                                            );
                                        }
                                        None => {
                                            ui.painter().rect_filled(
                                                rect,
                                                2.,
                                                egui::Color32::from_gray(50),
                                            );
                                        }
                                    }
                                    if selected == Some(*id) {
                                        ui.painter().rect_stroke(
                                            rect,
                                            2.,
                                            egui::Stroke::new(
                                                2_f32,
                                                egui::Color32::from_rgb(255, 212, 112),
                                            ),
                                            egui::StrokeKind::Outside,
                                        );
                                    }
                                    ui.small(format!("{name}{}", if *changed { " *" } else { "" }));
                                    r
                                })
                                .inner;
                            if r.clicked() {
                                lines.push(format!("operandi.image.select image:{id}"));
                            }
                            if r.double_clicked() {
                                lines.push(format!("operandi.image.view image:{id}"));
                            }
                        }
                    });
                    return;
                }
                for (id, name, changed, (w, h), format) in &images {
                    let psd = psd_images.contains(id);
                    let label = format!(
                        "{name}{}   {w} × {h}   {format}",
                        if *changed { " *" } else { "" }
                    );
                    // A Photoshop image is marked with the layer-stack icon.
                    let row = if psd {
                        ui.add(
                            crate::icons::button(ui, crate::icons::Icon::PsdLayers, &label)
                                .selected(selected == Some(*id))
                                .frame(false),
                        )
                    } else {
                        ui.selectable_label(selected == Some(*id), label)
                    };
                    if row.clicked() {
                        lines.push(format!("operandi.image.select image:{id}"));
                    }
                    // PNT-216: a double-click opens it in the Image Viewer.
                    if row.double_clicked() {
                        lines.push(format!("operandi.image.view image:{id}"));
                    }
                    row.context_menu(|ui| {
                        for (title, line) in [
                            ("Save", "operandi.image.save"),
                            ("Save As…", "operandi.image.saveAs"),
                            ("Reload", "operandi.image.reload"),
                            ("Invert Image", "operandi.image.invert"),
                            ("Clear Image", "operandi.image.clear"),
                            ("Set Resolution…", "operandi.image.setResolution"),
                            ("Open in Image Viewer", "operandi.image.view"),
                        ] {
                            if ui.button(title).clicked() {
                                lines.push(format!("operandi.image.select image:{id}"));
                                lines.push(format!("{line} image:{id}"));
                                ui.close();
                            }
                        }
                    });
                }
            });
        self.painting.images_open = open;
        for line in lines {
            self.execute(&line);
        }
    }

    /// Painting's entries in the File and Texture menus (painting builds
    /// only; the shipped menu forms don't name them).
    pub(crate) fn painting_menus(
        &self,
        menus: &mut [crate::app_menu::Menu],
        item: crate::menu_forms::Maker<'_>,
    ) {
        use crate::app_menu::{Entry, line_action};
        #[cfg(test)]
        if self.painting.hide_menus {
            return;
        }
        let entry = |line: &str, title: &str| {
            let state = self.line_state(line);
            item(
                line_action(line),
                title.to_owned(),
                state.as_ref().and_then(|s| s.unavailable.clone()),
                state.is_some_and(|s| s.on),
                String::new(),
            )
        };
        if let Some(file) = menus.iter_mut().find(|m| m.id == "menu.file") {
            // After Save As… (and the Current submenu), before Import.
            let at = file
                .entries
                .iter()
                .position(|e| matches!(e, Entry::Separator))
                .unwrap_or(file.entries.len());
            let mut add = vec![
                Entry::Separator,
                entry("operandi.image.new", "New Image…"),
                entry("operandi.image.newSequence", "New Image Sequence…"),
                entry("operandi.image.save", "Save Image"),
                entry("operandi.image.saveAs", "Save Image As…"),
                entry("operandi.image.saveAll", "Save All Images"),
                entry(
                    "operandi.paint.alsoSaveImages",
                    "Scene Save Also Saves Changed Images",
                ),
            ];
            if at == file.entries.len() {
                add.remove(0);
            }
            for (k, e) in add.into_iter().enumerate() {
                file.entries.insert(at + k, e);
            }
        }
        if let Some(texture) = menus.iter_mut().find(|m| m.id == "menu.texture") {
            // PNT-86: Texture > Paint Tools.
            let tools = crate::app_menu::Menu {
                id: String::new(),
                title: "Paint Tools".into(),
                entries: vec![
                    entry("operandi.paint.tab", "Paint Tab"),
                    entry(
                        "operandi.paint.showLocators",
                        "Show Texture Locators while Painting",
                    ),
                    Entry::Separator,
                    entry("operandi.paint.airbrush", "Airbrush"),
                    entry("operandi.paint.paintbrush", "Paintbrush"),
                    entry("operandi.paint.eraser", "Eraser"),
                    entry("operandi.paint.clone", "Clone"),
                    entry("operandi.paint.line", "Line"),
                    entry("operandi.paint.fill", "Fill"),
                    entry("operandi.paint.linearGradient", "Linear Gradient"),
                    entry("operandi.paint.radialGradient", "Radial Gradient"),
                    entry("operandi.paint.smudge", "Smudge"),
                    entry("operandi.paint.blur", "Blur"),
                    entry("operandi.paint.sharpen", "Sharpen"),
                    entry("operandi.paint.lasso", "Lasso"),
                    Entry::Separator,
                    entry("operandi.paint.tip kind:smooth", "Smooth Tip"),
                    entry("operandi.paint.tip kind:hard", "Hard Tip"),
                    entry("operandi.paint.tip kind:procedural", "Procedural Tip"),
                    entry("operandi.paint.tip kind:sphere", "Sphere Tip"),
                    entry("operandi.paint.tip kind:image", "Texture Tip"),
                    entry("operandi.paint.tip kind:stamp", "Stamp Tip"),
                    entry("operandi.paint.tip kind:text", "Text Tip"),
                    Entry::Separator,
                    entry("operandi.paint.nozzle", "Tablet Nozzle"),
                    entry("operandi.paint.savePreset", "Save Brush Preset"),
                    Entry::Submenu(crate::app_menu::Menu {
                        id: String::new(),
                        title: "Brush Presets".into(),
                        entries: {
                            let names = crate::painting_brush::preset_names(self);
                            if names.is_empty() {
                                vec![item(
                                    line_action("operandi.paint.applyPreset"),
                                    "No Brush Presets".into(),
                                    Some("Save one with Save Brush Preset".into()),
                                    false,
                                    String::new(),
                                )]
                            } else {
                                names
                                    .iter()
                                    .map(|n| {
                                        entry(
                                            &format!("operandi.paint.applyPreset name:\"{n}\""),
                                            n,
                                        )
                                    })
                                    .collect()
                            }
                        },
                    }),
                ],
            };
            texture.entries.extend([
                Entry::Separator,
                Entry::Submenu(tools),
                entry("operandi.paint.addTexture kind:color", "Add Color Texture…"),
                entry("operandi.paint.addTexture kind:mask", "Add Mask Texture…"),
                entry(
                    "operandi.paint.addTexture kind:displacement",
                    "Add Displacement Texture…",
                ),
                entry("operandi.paint.addFurTexture", "Add Fur Textures…"),
                Entry::Separator,
                entry("operandi.paint.images", "Images"),
                // PNT-213: Texture > Image Processing.
                Entry::Submenu(crate::app_menu::Menu {
                    id: String::new(),
                    title: "Image Processing".into(),
                    entries: vec![
                        entry("operandi.image.invert", "Invert Image"),
                        entry("operandi.image.clear", "Clear Image"),
                        entry("operandi.image.expandSeams", "Expand UV Seams"),
                        entry(
                            "operandi.image.toVectorDisplacement",
                            "Convert to Vector Displacement",
                        ),
                        entry("operandi.image.setResolution", "Set Resolution…"),
                        entry("operandi.image.flatten", "Flatten Layers"),
                    ],
                }),
            ]);
        }
    }
}
