//! Painting stage 5 in the app (PNT-170..200): the ink buttons and rows,
//! Image Ink's on-screen widget and keys, Projection Ink's picture and the
//! Curve Constraint. Painting builds only.
//!
//! - The toolbox has one button per ink (no flyouts yet): Image Ink,
//!   Parametric Ink, Random Ink, Projection Ink. Each runs
//!   `operandi.paint.ink kind:<ink>`, which adds it to the end of the
//!   stack, or takes it off when it is in the stack. The stack is shared
//!   by the paint tools that take inks (provisional, PNT-Q41).
//! - Every row runs `operandi.paint.inkSet kind:<ink> <name>:<value>`.
//! - Image Ink's widget shows after a click in the view; its handles drag
//!   (left button), and E/R/D/F turn and size it while it shows
//!   (`operandi.paint.inkWidget`).
use super::*;
use crate::painting::word;
use crate::unavailable::Gate;
use operandi_core::brush_engine::TipShape;
use operandi_core::command_registry::{self as table, Area, ArgType, Unit};
use operandi_core::command_runtime::{Call, CommandState, HostHandler};
use operandi_core::paint_inks::{
    CameraSource, CurveConstraint, ImageInk, Ink, InkKind, InkRun, Inks, NO_CAMERA_IMAGE, Parameter,
};
use operandi_core::paint_stroke::PaintTool;

/// E/R turn the widget by this much, D/F size it by this factor
/// (provisional, PNT-Q44).
pub(crate) const KEY_TURN_DEGREES: f64 = 5.;
pub(crate) const KEY_SCALE: f64 = 1.05;
/// The handles' pick radius in points.
const HANDLE: f32 = 10.;
/// The widget picture's opacity over the view ("like glass").
const GLASS: f32 = 0.45;
/// Sculpt brushes take no inks (PNT-243).
pub(crate) const SCULPT_INKS: &str = "Inks work with the paint tools only";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handle {
    Move,
    Scale,
    Rotate,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WidgetDrag {
    handle: Handle,
    from: Pos2,
    start: ImageInk,
    rect: Rect,
}

/// The inks' app state.
#[derive(Default)]
pub(crate) struct InkState {
    pub(crate) inks: Inks,
    /// Image Ink's widget is on screen (after a click in the view).
    pub(crate) widget_shown: bool,
    pub(crate) drag: Option<WidgetDrag>,
    /// Auto Scale: the view distance the widget's size was set at.
    auto_scale_from: Option<f64>,
}

/// The tools inks work with (PNT-170).
pub(crate) fn takes_inks(tool: PaintTool) -> bool {
    matches!(
        tool,
        PaintTool::Airbrush
            | PaintTool::Paintbrush
            | PaintTool::Eraser
            | PaintTool::Clone
            | PaintTool::Smudge
            | PaintTool::Blur
            | PaintTool::Sharpen
            | PaintTool::Line
    )
}

// ------------------------------------------------------------- commands

const INK_ARGS: &[table::Arg] = &[table::arg(
    "kind",
    ArgType::Choice(&["image", "parametric", "random", "projection"]),
    Unit::None,
)];
const INKSET_ARGS: &[table::Arg] = &[
    table::arg(
        "kind",
        ArgType::Choice(&["image", "parametric", "random", "projection"]),
        Unit::None,
    ),
    table::opt("image", ArgType::Text, Unit::None, None),
    table::opt("flipH", ArgType::Bool, Unit::None, None),
    table::opt("flipV", ArgType::Bool, Unit::None, None),
    table::opt("stamp", ArgType::Bool, Unit::None, None),
    table::opt("pixelBlending", ArgType::Bool, Unit::None, None),
    table::opt("repeat", ArgType::Bool, Unit::None, None),
    table::opt("autoScale", ArgType::Bool, Unit::None, None),
    table::opt("fitView", ArgType::Bool, Unit::None, None),
    table::opt("softBorder", ArgType::Float, Unit::Percent, None),
    table::opt("scale", ArgType::Float, Unit::Percent, None),
    table::opt("posX", ArgType::Float, Unit::None, None),
    table::opt("posY", ArgType::Float, Unit::None, None),
    table::opt("rotation", ArgType::Float, Unit::None, None),
    table::opt("useColorAsMask", ArgType::Bool, Unit::None, None),
    table::opt("uvMask", ArgType::Text, Unit::None, None),
    table::opt("maskBlur", ArgType::Float, Unit::None, None),
    table::opt("invertMask", ArgType::Bool, Unit::None, None),
    table::opt("parameter", ArgType::Text, Unit::None, None),
    table::opt("useAsMask", ArgType::Bool, Unit::None, None),
    table::opt("shape", ArgType::Text, Unit::None, None),
    table::opt("in", ArgType::Float, Unit::Percent, None),
    table::opt("out", ArgType::Float, Unit::Percent, None),
    table::opt("hue", ArgType::Float, Unit::Percent, None),
    table::opt("saturation", ArgType::Float, Unit::Percent, None),
    table::opt("brightness", ArgType::Float, Unit::Percent, None),
    table::opt("showImage", ArgType::Bool, Unit::None, None),
    table::opt("distance", ArgType::Float, Unit::Distance, None),
    table::opt("opacity", ArgType::Float, Unit::Percent, None),
    table::opt("exposure", ArgType::Float, Unit::None, None),
];
const WIDGET_ARGS: &[table::Arg] = &[table::arg(
    "action",
    ArgType::Choice(&["show", "hide", "turnRight", "turnLeft", "smaller", "larger"]),
    Unit::None,
)];

const fn cmd(name: &'static str, help: &'static str) -> table::Entry {
    table::script(name, Area::Tools, help).operandi()
}

type State = fn(&Operandi, &Call<'_>) -> CommandState;

/// Why Projection Ink can't paint: no camera, or no background image.
fn projection_problem(a: &Operandi) -> Option<&'static str> {
    let scene = &a.editor.scene;
    match a.ink_camera() {
        None => Some("Needs a camera"),
        Some(id) => {
            let has = scene
                .item(id)
                .and_then(|i| i.camera.as_ref())
                .is_some_and(|c| c.background.image.is_some());
            (!has).then_some(NO_CAMERA_IMAGE)
        }
    }
}

pub(crate) fn commands() -> Vec<(table::Entry, HostHandler<Operandi>, State)> {
    vec![
        (
            cmd("operandi.paint.ink", "Adds an ink to the paint tool's ink stack, or takes it off: image (a picture over the view), parametric (FG to BG by slope, altitude, angle or pressure), random (per-dab colour), projection (the camera's background image) (PNT-170..192).")
                .args(INK_ARGS)
                .example("operandi.paint.ink kind:random"),
            run_ink,
            |a, c| {
                let kind = word(c.rest, "kind", 0).and_then(|k| InkKind::named(&k));
                // On sculpt brushes Projection Ink is paint only (PNT-243).
                if let Some(why) = kind.and_then(|k| a.ink_unavailable_for_sculpt(k)) {
                    return CommandState::unless(Some(why));
                }
                if kind == Some(InkKind::Projection)
                    && !a.painting.brush.ink.inks.has(InkKind::Projection)
                    && let Some(why) = projection_problem(a)
                {
                    return CommandState::unless(Some(why));
                }
                CommandState::lit(kind.is_some_and(|k| a.painting.brush.ink.inks.has(k)))
            },
        ),
        (
            cmd("operandi.paint.inkSet", "An ink's rows (PNT-172, 181, 185, 191). kind (image|parametric|random|projection), then: image ink image, flipH, flipV, stamp, pixelBlending, repeat, autoScale, fitView, softBorder, scale, posX, posY (view pixels), rotation (degrees), useColorAsMask, uvMask, maskBlur, invertMask; parametric parameter, useAsMask, shape, in, out; random hue, saturation, brightness; projection showImage, distance, opacity, stamp, exposure. Percents 0..1.")
                .args(INKSET_ARGS)
                .example("operandi.paint.inkSet kind:random brightness:0.3"),
            run_ink_set,
            |_, _| CommandState::default(),
        ),
        (
            cmd("operandi.paint.inkWidget", "Image Ink's widget: show, hide, turnRight (E), turnLeft (R), smaller (D), larger (F) (PNT-171).")
                .args(WIDGET_ARGS)
                .example("operandi.paint.inkWidget action:larger"),
            run_widget,
            |a, _| {
                CommandState::unless(
                    (!a.painting.brush.ink.inks.has(InkKind::Image)).then_some("Turn on Image Ink first"),
                )
            },
        ),
        (
            cmd("operandi.paint.sculptInks", "Inks on a sculpt brush (PNT-243): not available; inks work with the paint tools."),
            |a, _| a.status = SCULPT_INKS.into(),
            |_, _| CommandState::unless(Some(SCULPT_INKS)),
        ),
    ]
}

pub(crate) fn register(r: &mut crate::command_handlers::Commands) {
    for (entry, run, state) in commands() {
        let name = entry.name;
        r.add_host_command(entry, run)
            .unwrap_or_else(|e| panic!("{e}"));
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
}

fn run_ink(a: &mut Operandi, c: &Call<'_>) {
    let Some(kind) = word(c.rest, "kind", 0).and_then(|k| InkKind::named(&k)) else {
        a.status = "Usage: operandi.paint.ink kind:<image|parametric|random|projection>".into();
        return;
    };
    if let Some(why) = a.ink_unavailable_for_sculpt(kind) {
        a.status = why.into();
        return;
    }
    let ink = &mut a.painting.brush.ink;
    if kind == InkKind::Projection
        && !ink.inks.has(kind)
        && let Some(why) = projection_problem(a)
    {
        a.status = why.into();
        return;
    }
    let ink = &mut a.painting.brush.ink;
    let on = ink.inks.toggle(kind);
    a.load_ink_images();
    let ink = &mut a.painting.brush.ink;
    if kind == InkKind::Image {
        // Pos X/Y go back to the view's centre each time (PNT-172).
        ink.widget_shown = false;
        ink.drag = None;
        ink.auto_scale_from = None;
    }
    a.painting.brush.tab_shown = true;
    a.status = match (on, kind) {
        (true, InkKind::Image) => "Image Ink on: click in the view to show the picture".into(),
        (true, k) => format!("{} on", k.label()),
        (false, k) => format!("{} off", k.label()),
    };
}

fn number(v: &str, what: &str) -> std::result::Result<f64, String> {
    v.trim()
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite())
        .ok_or_else(|| format!("{what} is a number"))
}

fn run_ink_set(a: &mut Operandi, c: &Call<'_>) {
    let images: Vec<(u64, String)> = a
        .editor
        .scene
        .images
        .iter()
        .map(|i| (i.id, i.name.clone()))
        .collect();
    let image_named = |v: &str| -> std::result::Result<Option<u64>, String> {
        if v.eq_ignore_ascii_case("none") {
            return Ok(None);
        }
        images
            .iter()
            .find(|(id, n)| n == v || id.to_string() == v)
            .map(|(id, _)| Some(*id))
            .ok_or_else(|| format!("No image called {v}"))
    };
    let Some(kind) = word(c.rest, "kind", 0).and_then(|k| InkKind::named(&k)) else {
        a.status = "Name the ink: operandi.paint.inkSet kind:image scale:0.5".into();
        return;
    };
    let mut inks = a.painting.brush.ink.inks.clone();
    let Some(ink) = inks.get_mut(kind) else {
        a.status = format!("Turn on {} first", kind.label());
        return;
    };
    let apply = |ink: &mut Ink| -> std::result::Result<(), String> {
        let words = operandi_core::command_line::tokens(c.rest).map_err(str::to_owned)?;
        for w in words {
            let (Some(name), value) = operandi_core::command_line::named(&w) else {
                continue; // the positional kind
            };
            if name == "kind" {
                continue;
            }
            let flag = || operandi_core::command_args::boolean(name, value);
            let num = || number(value, name);
            let pct = || Ok::<f64, String>(num()?.clamp(0., 1.));
            match ink {
                Ink::Image(i) => match name {
                    "image" => i.image = image_named(value)?,
                    "flipH" => i.flip_h = flag()?,
                    "flipV" => i.flip_v = flag()?,
                    "stamp" => i.stamp = flag()?,
                    "pixelBlending" => i.pixel_blending = flag()?,
                    "repeat" => i.repeat = flag()?,
                    "autoScale" => i.auto_scale = flag()?,
                    "fitView" => i.fit_view = flag()?,
                    "softBorder" => i.soft_border = pct()?,
                    "scale" => i.scale = num()?.max(0.001),
                    "posX" | "posY" => {
                        let mut p = i.pos.unwrap_or(glam::DVec2::ZERO);
                        if name == "posX" {
                            p.x = num()?;
                        } else {
                            p.y = num()?;
                        }
                        i.pos = Some(p);
                    }
                    "rotation" => i.rotation = num()?.to_radians(),
                    "useColorAsMask" => i.use_color_as_mask = flag()?,
                    "uvMask" => i.uv_mask = image_named(value)?,
                    "maskBlur" => i.mask_blur = num()?.clamp(0., 8.),
                    "invertMask" => i.invert_mask = flag()?,
                    other => return Err(format!("Image Ink has no {other}")),
                },
                Ink::Parametric(p) => match name {
                    "parameter" => {
                        p.parameter = Parameter::named(value)
                            .ok_or_else(|| format!("No parameter {value}"))?
                    }
                    "useAsMask" => p.use_as_mask = flag()?,
                    "shape" => {
                        p.shape = TipShape::ALL
                            .into_iter()
                            .find(|t| {
                                t.label()
                                    .replace(' ', "")
                                    .eq_ignore_ascii_case(&value.replace(' ', ""))
                            })
                            .ok_or_else(|| format!("No shape preset {value}"))?
                    }
                    "in" => p.inn = pct()?,
                    "out" => p.out = pct()?,
                    other => return Err(format!("Parametric Ink has no {other}")),
                },
                Ink::Random(r) => match name {
                    "hue" => r.hue = pct()?,
                    "saturation" => r.saturation = pct()?,
                    "brightness" => r.brightness = pct()?,
                    other => return Err(format!("Random Ink has no {other}")),
                },
                Ink::Projection(p) => match name {
                    "showImage" => p.show_image = flag()?,
                    "distance" => p.distance = num()?.max(0.001),
                    "opacity" => p.opacity = pct()?,
                    "stamp" => p.stamp = flag()?,
                    "exposure" => p.exposure = num()?.clamp(-10., 10.),
                    other => return Err(format!("Projection Ink has no {other}")),
                },
            }
        }
        Ok(())
    };
    match apply(ink) {
        Ok(()) => {
            a.painting.brush.ink.inks = inks;
            a.load_ink_images();
        }
        Err(e) => a.status = e,
    }
}

fn run_widget(a: &mut Operandi, c: &Call<'_>) {
    let action = word(c.rest, "action", 0).unwrap_or_default();
    let view = a.viewport_rect;
    let st = &mut a.painting.brush.ink;
    let Some(Ink::Image(i)) = st.inks.get_mut(InkKind::Image) else {
        a.status = "Turn on Image Ink first".into();
        return;
    };
    match action.as_str() {
        "show" => {
            st.widget_shown = true;
            if i.pos.is_none() {
                i.pos = Some(glam::DVec2::new(
                    f64::from(view.width()) / 2.,
                    f64::from(view.height()) / 2.,
                ));
            }
        }
        "hide" => st.widget_shown = false,
        "turnRight" => i.rotation += KEY_TURN_DEGREES.to_radians(),
        "turnLeft" => i.rotation -= KEY_TURN_DEGREES.to_radians(),
        "smaller" => i.scale = (i.scale / KEY_SCALE).max(0.001),
        "larger" => i.scale *= KEY_SCALE,
        other => a.status = format!("No widget action {other}"),
    }
}

// ------------------------------------------------------------- app side

fn d2(p: Pos2) -> glam::DVec2 {
    glam::DVec2::new(f64::from(p.x), f64::from(p.y))
}

fn p2(v: glam::DVec2) -> Pos2 {
    pos2(v.x as f32, v.y as f32)
}

impl Operandi {
    /// Reads the pictures the inks show so the view can draw them.
    pub(crate) fn load_ink_images(&mut self) {
        let mut ids: Vec<u64> = Vec::new();
        for k in &self.painting.brush.ink.inks.0 {
            match k {
                Ink::Image(i) => ids.extend(i.image.into_iter().chain(i.uv_mask)),
                Ink::Projection(_) => ids.extend(
                    self.ink_camera()
                        .and_then(|c| self.editor.scene.item(c))
                        .and_then(|c| c.camera.as_ref())
                        .and_then(|c| c.background.image),
                ),
                _ => {}
            }
        }
        for id in ids {
            if let Err(e) = self.editor.image_pixels(id) {
                self.status = e;
            }
        }
    }

    /// Projection Ink's camera: a selected camera, else the render camera
    /// (PNT-190; provisional, PNT-Q45).
    pub(crate) fn ink_camera(&self) -> Option<u64> {
        let scene = &self.editor.scene;
        let s = &scene.selection;
        s.item
            .into_iter()
            .chain(s.also.iter().copied())
            .find(|&id| scene.item(id).is_some_and(|i| i.camera.is_some()))
            .or_else(|| scene.render_camera())
    }

    /// Image Ink's settings as they paint now (Auto Scale applied).
    fn image_ink_now(&self, r: Rect) -> Option<ImageInk> {
        let st = &self.painting.brush.ink;
        let mut i = *st.inks.image()?;
        if i.pos.is_none() {
            i.pos = Some(glam::DVec2::new(f64::from(r.width()), f64::from(r.height())) / 2.);
        }
        if i.auto_scale
            && let Some(from) = st.auto_scale_from
        {
            i.scale *= from / self.camera.distance.max(1e-9);
        }
        Some(i)
    }

    /// The inks for a stroke of `tool` on mesh `item` in view `r`.
    pub(crate) fn ink_run(
        &mut self,
        tool: PaintTool,
        item: u64,
        r: Rect,
    ) -> std::result::Result<InkRun, String> {
        let st = &self.painting.brush.ink;
        if !takes_inks(tool) || st.inks.0.is_empty() {
            return Ok(InkRun::default());
        }
        let mut inks = st.inks.clone();
        let mut run = InkRun {
            view: glam::DVec2::new(f64::from(r.width()), f64::from(r.height())),
            origin: d2(r.min),
            seed: self.painting.brush.stroke_seed(),
            ..InkRun::default()
        };
        if let Some(now) = self.image_ink_now(r)
            && let Some(Ink::Image(i)) = inks.get_mut(InkKind::Image)
        {
            *i = now;
            // Fit View with no picture uses the camera's background
            // (PNT-172).
            let source = i.image.or_else(|| {
                i.fit_view
                    .then(|| self.ink_camera())
                    .flatten()
                    .and_then(|c| self.editor.scene.item(c))
                    .and_then(|c| c.camera.as_ref())
                    .and_then(|c| c.background.image)
            });
            let Some(id) = source else {
                return Err("Image Ink: pick an image in its Image row".into());
            };
            run.image = Some(self.editor.image_pixels(id)?);
            if let Some(m) = i.uv_mask {
                run.uv_mask = Some(self.editor.image_pixels(m)?);
            }
        }
        if inks.has(InkKind::Projection) {
            let cam = self.ink_camera().ok_or("Projection Ink needs a camera")?;
            run.camera = Some(self.camera_source(cam)?);
        }
        let scene = &self.editor.scene;
        let world = scene.world_matrix(item)?;
        if let Some(it) = scene.item(item) {
            run.bounds = operandi_core::paint_inks::world_bounds(&it.mesh, world);
        }
        run.inks = inks;
        Ok(run)
    }

    fn camera_source(&mut self, cam: u64) -> std::result::Result<CameraSource, String> {
        let id = self
            .editor
            .scene
            .item(cam)
            .and_then(|i| i.camera.as_ref())
            .and_then(|c| c.background.image)
            .ok_or(NO_CAMERA_IMAGE)?;
        let px = self.editor.image_pixels(id)?;
        CameraSource::of(&self.editor.scene, cam, |i| (i == id).then(|| px.clone()))
    }

    /// A press with Image Ink on: shows the widget, or starts a handle
    /// drag. `None` lets the press paint.
    pub(crate) fn ink_widget_press(&mut self, p: Pos2, r: Rect) -> Option<input::GestureKind> {
        let tool = self.painting.brush.tool?;
        if !takes_inks(tool) || !self.painting.brush.ink.inks.has(InkKind::Image) {
            return None;
        }
        if !self.painting.brush.ink.widget_shown {
            self.execute("operandi.paint.inkWidget action:show");
            self.painting.brush.ink.auto_scale_from = Some(self.camera.distance);
            self.status = "Image Ink: drag the centre to move, the square to size, the top circle to turn; E/R turn, D/F size".into();
            return Some(input::GestureKind::Blocked);
        }
        let handle = self.ink_handle_at(p, r)?;
        let start = *self.painting.brush.ink.inks.image()?;
        self.painting.brush.ink.drag = Some(WidgetDrag {
            handle,
            from: p,
            start,
            rect: r,
        });
        Some(input::GestureKind::Paint)
    }

    /// The widget's centre, right-square and top-circle points on screen.
    fn ink_handles(&self, r: Rect) -> Option<[(Handle, Pos2); 3]> {
        let i = self.image_ink_now(r)?;
        let size = i
            .image
            .and_then(|id| self.editor.scene.images.iter().find(|m| m.id == id))
            .map(|m| (m.width, m.height));
        let view = glam::DVec2::new(f64::from(r.width()), f64::from(r.height()));
        let (c, half) = operandi_core::paint_inks::image_ink_frame(&i, size, view);
        let turn = |v: glam::DVec2| {
            let (s, co) = i.rotation.sin_cos();
            glam::DVec2::new(v.x * co - v.y * s, v.x * s + v.y * co)
        };
        let o = d2(r.min) + c;
        Some([
            (Handle::Move, p2(o)),
            (Handle::Scale, p2(o + turn(glam::DVec2::new(half.x, 0.)))),
            (Handle::Rotate, p2(o + turn(glam::DVec2::new(0., -half.y)))),
        ])
    }

    fn ink_handle_at(&self, p: Pos2, r: Rect) -> Option<Handle> {
        self.ink_handles(r)?
            .into_iter()
            .filter(|(_, q)| q.distance(p) <= HANDLE)
            .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)))
            .map(|(h, _)| h)
    }

    /// A handle drag's move (live; the release records it as a command).
    pub(crate) fn ink_widget_drag(&mut self, p: Pos2) -> bool {
        let Some(d) = self.painting.brush.ink.drag else {
            return false;
        };
        let mut i = d.start;
        let centre = d.rect.min
            + i.pos
                .map_or(d.rect.size() / 2., |v| vec2(v.x as f32, v.y as f32));
        match d.handle {
            Handle::Move => {
                i.pos = Some(i.pos.unwrap_or_default() + d2(p) - d2(d.from));
            }
            Handle::Scale => {
                let was = d.from.distance(centre).max(1.);
                i.scale = (d.start.scale * f64::from(p.distance(centre) / was)).max(0.001);
            }
            Handle::Rotate => {
                // Left/right turns it (PNT-171): half a degree per point.
                i.rotation = d.start.rotation + f64::from(p.x - d.from.x) * 0.5f64.to_radians();
            }
        }
        if let Some(Ink::Image(cur)) = self.painting.brush.ink.inks.get_mut(InkKind::Image) {
            *cur = i;
        }
        true
    }

    /// The release of a handle drag: its result as one command line.
    pub(crate) fn ink_widget_release(&mut self) -> bool {
        let Some(d) = self.painting.brush.ink.drag.take() else {
            return false;
        };
        let Some(i) = self.painting.brush.ink.inks.image().copied() else {
            return true;
        };
        if let Some(Ink::Image(cur)) = self.painting.brush.ink.inks.get_mut(InkKind::Image) {
            *cur = d.start;
        }
        let pos = i.pos.unwrap_or_default();
        self.execute(&format!(
            "operandi.paint.inkSet kind:image posX:{} posY:{} scale:{} rotation:{}",
            pos.x,
            pos.y,
            i.scale,
            i.rotation.to_degrees()
        ));
        true
    }

    /// E/R/D/F while the widget shows (PNT-171): they win over the global
    /// keys (Rotate, Scale, Subdivide, Flip) only then.
    pub(crate) fn ink_widget_keys(&mut self, ctx: &Context) -> bool {
        let st = &self.painting.brush.ink;
        let active = st.widget_shown
            && st.inks.has(InkKind::Image)
            && self.painting.brush.tool.is_some_and(takes_inks);
        if !active {
            return false;
        }
        for (key, action) in [
            (Key::E, "turnRight"),
            (Key::R, "turnLeft"),
            (Key::D, "smaller"),
            (Key::F, "larger"),
        ] {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                self.execute(&format!("operandi.paint.inkWidget action:{action}"));
                return true;
            }
        }
        false
    }

    /// Image or Projection Ink's Stamp is on for `tool`.
    pub(crate) fn ink_stamps(&self, tool: PaintTool) -> bool {
        takes_inks(tool)
            && self.painting.brush.ink.inks.0.iter().any(|k| match k {
                Ink::Image(i) => i.stamp,
                Ink::Projection(p) => p.stamp,
                _ => false,
            })
    }

    /// Projection Ink's Stamp fills the selected polygons of `item` when
    /// some are selected (PNT-191).
    pub(crate) fn ink_stamp_faces(&self, item: u64) -> Option<Vec<u32>> {
        let projection = self
            .painting
            .brush
            .ink
            .inks
            .0
            .iter()
            .any(|k| matches!(k, Ink::Projection(p) if p.stamp));
        let s = &self.editor.scene.selection;
        (projection
            && s.mode == operandi_core::Mode::Polygon
            && s.item == Some(item)
            && !s.faces.is_empty())
        .then(|| s.faces.clone())
    }

    /// The widget and Projection Ink's picture over the view.
    pub(crate) fn draw_inks(&self, painter: &Painter, r: Rect) {
        let st = &self.painting.brush.ink;
        if !self.painting.brush.tool.is_some_and(takes_inks) {
            return;
        }
        for ink in &st.inks.0 {
            match ink {
                Ink::Projection(pj) if pj.show_image => {
                    self.draw_projection_picture(painter, r, pj)
                }
                Ink::Image(_) if st.widget_shown => self.draw_image_widget(painter, r),
                _ => {}
            }
        }
    }

    fn ink_texture(&self, ctx: &Context, id: u64) -> Option<egui::TextureHandle> {
        // Loaded when the ink picks it (`load_ink_images`).
        let px = self.editor.scene.paint.buffers.get(&id).cloned()?;
        let key = egui::Id::new((
            "paint-ink-texture",
            id,
            std::sync::Arc::as_ptr(&px) as usize,
        ));
        if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
            return Some(t);
        }
        let (w, h) = (px.width as usize, px.height as usize);
        let rgba: Vec<egui::Color32> = (0..w * h)
            .map(|i| {
                let c = px.get(i).map(|v| (v.clamp(0., 1.) * 255.).round() as u8);
                egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
            })
            .collect();
        let image = egui::ColorImage {
            size: [w, h],
            pixels: rgba,
            source_size: vec2(w as f32, h as f32),
        };
        let t = ctx.load_texture(
            format!("paint-ink-{id}"),
            image,
            egui::TextureOptions::LINEAR,
        );
        ctx.data_mut(|d| d.insert_temp(key, t.clone()));
        Some(t)
    }

    fn textured_quad(painter: &Painter, tex: egui::TextureId, corners: [Pos2; 4], alpha: f32) {
        let tint = egui::Color32::from_white_alpha((alpha.clamp(0., 1.) * 255.) as u8);
        let mut mesh = egui::Mesh::with_texture(tex);
        for (p, uv) in
            corners
                .into_iter()
                .zip([pos2(0., 0.), pos2(1., 0.), pos2(1., 1.), pos2(0., 1.)])
        {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: p,
                uv,
                color: tint,
            });
        }
        mesh.indices = vec![0, 1, 2, 0, 2, 3];
        painter.add(egui::Shape::mesh(mesh));
    }

    fn draw_image_widget(&self, painter: &Painter, r: Rect) {
        let Some(i) = self.image_ink_now(r) else {
            return;
        };
        let size = i
            .image
            .and_then(|id| self.editor.scene.images.iter().find(|m| m.id == id))
            .map(|m| (m.width, m.height));
        let view = glam::DVec2::new(f64::from(r.width()), f64::from(r.height()));
        let (c, half) = operandi_core::paint_inks::image_ink_frame(&i, size, view);
        let (s, co) = i.rotation.sin_cos();
        let at = |x: f64, y: f64| {
            let v = glam::DVec2::new(x * half.x, y * half.y);
            p2(d2(r.min) + c + glam::DVec2::new(v.x * co - v.y * s, v.x * s + v.y * co))
        };
        let mut corners = [at(-1., -1.), at(1., -1.), at(1., 1.), at(-1., 1.)];
        if i.flip_h {
            corners.swap(0, 1);
            corners.swap(2, 3);
        }
        if i.flip_v {
            corners.swap(0, 3);
            corners.swap(1, 2);
        }
        let painter = painter.with_clip_rect(r);
        if let Some(t) = i.image.and_then(|id| self.ink_texture(painter.ctx(), id)) {
            Self::textured_quad(&painter, t.id(), corners, GLASS);
        }
        let line = egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(255, 212, 112));
        let outline = [
            at(-1., -1.),
            at(1., -1.),
            at(1., 1.),
            at(-1., 1.),
            at(-1., -1.),
        ];
        painter.add(egui::Shape::line(outline.to_vec(), line));
        let Some(h) = self.ink_handles(r) else {
            return;
        };
        let fill = egui::Color32::from_black_alpha(160);
        painter.circle(h[0].1, 7., fill, line);
        painter.rect(
            Rect::from_center_size(h[1].1, vec2(12., 12.)),
            0.,
            fill,
            line,
            egui::StrokeKind::Middle,
        );
        painter.circle(h[2].1, 6., fill, line);
    }

    fn draw_projection_picture(
        &self,
        painter: &Painter,
        r: Rect,
        pj: &operandi_core::paint_inks::ProjectionInk,
    ) {
        let Some(cam) = self.ink_camera() else {
            return;
        };
        let scene = &self.editor.scene;
        let Some(settings) = scene.item(cam).and_then(|i| i.camera.as_ref()) else {
            return;
        };
        let Some(id) = settings.background.image else {
            return;
        };
        let Some(info) = scene.images.iter().find(|m| m.id == id) else {
            return;
        };
        let Ok(m) = scene.world_matrix(cam) else {
            return;
        };
        let Some(f) = operandi_core::camera_light::Frame::from_matrix(m) else {
            return;
        };
        let half = settings.half_tangents(f64::from(info.width) / f64::from(info.height.max(1)));
        let d = pj.distance;
        let view = self.paint_view(r);
        let corner = |x: f64, y: f64| {
            let p =
                f.position + f.forward * d + f.right * (x * half.0 * d) + f.up * (y * half.1 * d);
            view.project(p).map(|(s, _)| p2(s))
        };
        let [Some(a), Some(b), Some(c), Some(e)] = [
            corner(-1., 1.),
            corner(1., 1.),
            corner(1., -1.),
            corner(-1., -1.),
        ] else {
            return;
        };
        let painter = painter.with_clip_rect(r);
        if let Some(t) = self.ink_texture(painter.ctx(), id) {
            Self::textured_quad(&painter, t.id(), [a, b, c, e], pj.opacity as f32);
        }
    }

    /// The Curve Constraint's guide for a stroke, when it has one
    /// (PNT-200). A constraint with no background curve paints freely and
    /// says so.
    pub(crate) fn curve_guide(
        &mut self,
        tool: PaintTool,
    ) -> Option<(CurveConstraint, operandi_core::paint_inks::CurveGuide)> {
        let c = self.painting.brush.settings(tool).curve;
        if c == CurveConstraint::None {
            return None;
        }
        let guide = operandi_core::paint_inks::CurveGuide::background(&self.editor.scene);
        if guide.is_empty() {
            self.paint_notice(
                "Curve Constraint: no background curve (show a curve in an unselected mesh)",
            );
            return None;
        }
        Some((c, guide))
    }
}

// ------------------------------------------------------------- rows

impl Operandi {
    /// Each ink's rows in stack order (PNT-172, 181, 185, 191).
    pub(crate) fn ink_rows(&self, ui: &mut Ui, lines: &mut Vec<String>) {
        let images: Vec<(u64, String)> = self
            .editor
            .scene
            .images
            .iter()
            .map(|i| (i.id, i.name.clone()))
            .collect();
        for ink in &self.painting.brush.ink.inks.0 {
            ui.separator();
            ui.strong(ink.kind().label());
            let k = ink.kind().word();
            let set = move |rest: String| format!("operandi.paint.inkSet kind:{k} {rest}");
            let flag = |ui: &mut Ui, label: &str, on: bool, name: &str, lines: &mut Vec<String>| {
                crate::option_row::field_row(ui, label, |ui| {
                    let mut v = on;
                    if ui.checkbox(&mut v, "").changed() {
                        lines.push(set(format!("{name}:{v}")));
                    }
                });
            };
            let num = |ui: &mut Ui,
                       label: &str,
                       v: f64,
                       factor: f64,
                       range: std::ops::RangeInclusive<f64>,
                       suffix: &str,
                       name: &str,
                       lines: &mut Vec<String>| {
                crate::option_row::field_row(ui, label, |ui| {
                    let mut x = v * factor;
                    if ui
                        .add(
                            crate::numeric_tab::DragValue::new(&mut x)
                                .range(range)
                                .suffix(suffix),
                        )
                        .changed()
                    {
                        lines.push(set(format!("{name}:{}", x / factor)));
                    }
                });
            };
            let picker =
                |ui: &mut Ui, label: &str, id: Option<u64>, name: &str, lines: &mut Vec<String>| {
                    crate::option_row::field_row(ui, label, |ui| {
                        let shown = id
                            .and_then(|id| images.iter().find(|(i, _)| *i == id))
                            .map_or("(none)".to_owned(), |(_, n)| n.clone());
                        egui::ComboBox::from_id_salt(("paint-ink-image", name))
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(id.is_none(), "(none)").clicked() {
                                    lines.push(set(format!("{name}:none")));
                                }
                                for (i, n) in &images {
                                    if ui.selectable_label(id == Some(*i), n).clicked() {
                                        lines.push(set(format!("{name}:{i}")));
                                    }
                                }
                            });
                    });
                };
            match *ink {
                Ink::Image(i) => {
                    picker(ui, "Image", i.image, "image", lines);
                    flag(ui, "Flip Horizontal", i.flip_h, "flipH", lines);
                    flag(ui, "Flip Vertical", i.flip_v, "flipV", lines);
                    flag(ui, "Stamp", i.stamp, "stamp", lines);
                    flag(
                        ui,
                        "Pixel Blending",
                        i.pixel_blending,
                        "pixelBlending",
                        lines,
                    );
                    flag(ui, "Repeat", i.repeat, "repeat", lines);
                    flag(ui, "Auto Scale", i.auto_scale, "autoScale", lines);
                    flag(ui, "Fit View", i.fit_view, "fitView", lines);
                    num(
                        ui,
                        "Soft Border",
                        i.soft_border,
                        100.,
                        0. ..=100.,
                        " %",
                        "softBorder",
                        lines,
                    );
                    num(
                        ui,
                        "Scale",
                        i.scale,
                        100.,
                        0.1..=100_000.,
                        " %",
                        "scale",
                        lines,
                    );
                    let view = self.viewport_rect.size();
                    let pos = i
                        .pos
                        .unwrap_or(glam::DVec2::new(f64::from(view.x), f64::from(view.y)) / 2.);
                    num(
                        ui,
                        "Pos X",
                        pos.x,
                        1.,
                        -100_000. ..=100_000.,
                        " px",
                        "posX",
                        lines,
                    );
                    num(
                        ui,
                        "Pos Y",
                        pos.y,
                        1.,
                        -100_000. ..=100_000.,
                        " px",
                        "posY",
                        lines,
                    );
                    num(
                        ui,
                        "Rotation",
                        i.rotation.to_degrees(),
                        1.,
                        -36_000. ..=36_000.,
                        "°",
                        "rotation",
                        lines,
                    );
                    flag(
                        ui,
                        "Use Color as Mask",
                        i.use_color_as_mask,
                        "useColorAsMask",
                        lines,
                    );
                    picker(ui, "UV Mask", i.uv_mask, "uvMask", lines);
                    num(
                        ui,
                        "Mask Blur Radius",
                        i.mask_blur,
                        1.,
                        0. ..=8.,
                        "",
                        "maskBlur",
                        lines,
                    );
                    flag(ui, "Invert Mask", i.invert_mask, "invertMask", lines);
                }
                Ink::Parametric(p) => {
                    crate::option_row::field_row(ui, "Parameter", |ui| {
                        egui::ComboBox::from_id_salt("paint-ink-parameter")
                            .selected_text(p.parameter.label())
                            .show_ui(ui, |ui| {
                                for q in Parameter::ALL {
                                    let why = match q {
                                        Parameter::Displacement => {
                                            Some("Needs a displacement image on the surface")
                                        }
                                        Parameter::TangentPressure => {
                                            Some("The pen doesn't report tangent pressure")
                                        }
                                        _ => None,
                                    };
                                    let b = egui::Button::selectable(p.parameter == q, q.label());
                                    if ui.gated(why, b).clicked() {
                                        lines.push(set(format!(
                                            "parameter:{}",
                                            q.label().replace(' ', "")
                                        )));
                                    }
                                }
                            });
                    });
                    flag(ui, "Use as Mask", p.use_as_mask, "useAsMask", lines);
                    crate::option_row::field_row(ui, "Shape Preset", |ui| {
                        egui::ComboBox::from_id_salt("paint-ink-shape")
                            .selected_text(p.shape.label())
                            .show_ui(ui, |ui| {
                                for t in TipShape::ALL {
                                    if ui.selectable_label(p.shape == t, t.label()).clicked() {
                                        lines.push(set(format!(
                                            "shape:{}",
                                            t.label().replace(' ', "")
                                        )));
                                    }
                                }
                            });
                    });
                    num(ui, "In", p.inn, 100., 0. ..=100., " %", "in", lines);
                    num(ui, "Out", p.out, 100., 0. ..=100., " %", "out", lines);
                }
                Ink::Random(rk) => {
                    num(ui, "Hue", rk.hue, 100., 0. ..=100., " %", "hue", lines);
                    num(
                        ui,
                        "Saturation",
                        rk.saturation,
                        100.,
                        0. ..=100.,
                        " %",
                        "saturation",
                        lines,
                    );
                    num(
                        ui,
                        "Brightness",
                        rk.brightness,
                        100.,
                        0. ..=100.,
                        " %",
                        "brightness",
                        lines,
                    );
                }
                Ink::Projection(pj) => {
                    flag(ui, "Show Image", pj.show_image, "showImage", lines);
                    num(
                        ui,
                        "Image Distance",
                        pj.distance,
                        1.,
                        0.001..=100_000.,
                        " m",
                        "distance",
                        lines,
                    );
                    num(
                        ui,
                        "Image Opacity",
                        pj.opacity,
                        100.,
                        0. ..=100.,
                        " %",
                        "opacity",
                        lines,
                    );
                    flag(ui, "Stamp", pj.stamp, "stamp", lines);
                    num(
                        ui,
                        "Exposure",
                        pj.exposure,
                        1.,
                        -10. ..=10.,
                        "",
                        "exposure",
                        lines,
                    );
                }
            }
        }
    }
}
