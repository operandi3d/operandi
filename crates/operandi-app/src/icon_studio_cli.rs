//! `operandi-app --icon-studio`: UI icons rendered from small 3D scenes
//! with Cycles (core `icon_studio`). No window opens.
//!
//! ```text
//! operandi-app --icon-studio template <studio.opscene>
//! operandi-app --icon-studio sample <slug> <icon.opscene>
//! operandi-app --icon-studio render <icon.opscene | folder>... --out <folder>
//!              [--studio <studio.opscene>] [--samples <n>] [--no-outline]
//! ```
//!
//! `render` renders every icon straight at each size in
//! `icon_studio::SIZES` and at twice that, plus 32 px at 4× (128 px, for
//! big toolbox icons on high-density displays and large Interface Scales,
//! which would otherwise be scaled up), with the camera's frame fitted
//! to the picture, so edges are anti-aliased by the renderer's own pixel
//! filter and samples rather than by shrinking a big picture. Each file
//! comes in two sets with the same file names, a dark outline for light
//! interface colours and a light outline for dark ones:
//! `<out>/<slug>/dark-outline/<slug>-<size>.png` (and `-<size>@2x.png`) and
//! the same under `light-outline/`. `<out>/contact-sheet.png` shows the
//! dark-outline set on the generator's light preview background and the
//! light-outline set on its dark one.
use eframe::egui;
use operandi_core::Scene;
use operandi_core::icon_studio::{self, SIZES};
use std::path::{Path, PathBuf};

const USAGE: &str = "usage: operandi-app --icon-studio template <studio.opscene>\n       \
operandi-app --icon-studio sample <slug> <icon.opscene>\n       \
operandi-app --icon-studio render <icon.opscene | folder>... --out <folder> \
[--studio <studio.opscene>] [--samples <n>] [--no-outline]";

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Job {
    Template {
        out: PathBuf,
    },
    Sample {
        sample: icon_studio::Sample,
        out: PathBuf,
    },
    Render(RenderJob),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderJob {
    /// Icon scenes, and folders whose `.opscene` files are icon scenes.
    pub inputs: Vec<PathBuf>,
    pub out: PathBuf,
    /// The studio scene; `None` uses the built-in one.
    pub studio: Option<PathBuf>,
    /// Overrides the studio's own render samples.
    pub samples: Option<u32>,
    /// Outline each silhouette (the generator's edge stroke), in a dark and
    /// a light set; off writes one set with no outline.
    pub outline: bool,
}

/// Parses the arguments after `--icon-studio`.
pub(crate) fn parse(args: &[String]) -> Result<Job, String> {
    let (verb, rest) = args
        .split_first()
        .ok_or("give template, sample or render")?;
    match verb.as_str() {
        "template" => match rest {
            [out] => Ok(Job::Template { out: out.into() }),
            _ => Err("template takes one output .opscene".into()),
        },
        "sample" => match rest {
            [slug, out] => {
                let sample = icon_studio::Sample::named(slug).ok_or_else(|| {
                    let names: Vec<_> = icon_studio::Sample::ALL.iter().map(|s| s.slug()).collect();
                    format!("no sample {slug:?} ({})", names.join(", "))
                })?;
                Ok(Job::Sample {
                    sample,
                    out: out.into(),
                })
            }
            _ => Err("sample takes a sample name and an output .opscene".into()),
        },
        "render" => {
            let mut job = RenderJob {
                inputs: Vec::new(),
                out: PathBuf::new(),
                studio: None,
                samples: None,
                outline: true,
            };
            let mut out = None;
            let mut it = rest.iter();
            while let Some(arg) = it.next() {
                let mut value = |name: &str| {
                    it.next()
                        .cloned()
                        .ok_or_else(|| format!("{name} needs a value"))
                };
                match arg.as_str() {
                    "--out" => out = Some(PathBuf::from(value("--out")?)),
                    "--studio" => job.studio = Some(value("--studio")?.into()),
                    "--samples" => {
                        job.samples = Some(
                            value("--samples")?
                                .parse()
                                .ok()
                                .filter(|&s: &u32| (1..=65536).contains(&s))
                                .ok_or("--samples takes a whole number from 1 to 65536")?,
                        );
                    }
                    "--no-outline" => job.outline = false,
                    other if other.starts_with("--") => {
                        return Err(format!("unknown option {other}"));
                    }
                    other => job.inputs.push(other.into()),
                }
            }
            job.out = out.ok_or("--out <folder> is required")?;
            if job.inputs.is_empty() {
                return Err("give at least one icon .opscene or folder".into());
            }
            Ok(Job::Render(job))
        }
        other => Err(format!("unknown verb {other:?}")),
    }
}

fn is_scene(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("opscene"))
}

/// The icon scenes named by `inputs`, folders listed (not recursively),
/// sorted by file name so the contact sheet's order is stable.
pub(crate) fn icon_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for input in inputs {
        if input.is_dir() {
            let entries = std::fs::read_dir(input)
                .map_err(|e| format!("Couldn't list {}: {e}", input.display()))?;
            let mut found: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file() && is_scene(p))
                .collect();
            found.sort();
            files.extend(found);
        } else if is_scene(input) {
            files.push(input.clone());
        } else {
            return Err(format!("{} is not an .opscene or folder", input.display()));
        }
    }
    if files.is_empty() {
        return Err("No icon scenes found".into());
    }
    Ok(files)
}

/// The icon's slug: its file stem.
fn slug_of(path: &Path) -> String {
    path.file_stem()
        .map_or_else(|| "icon".into(), |s| s.to_string_lossy().into())
}

/// The outline width in pixels for a picture `px` pixels across: the
/// generator's edge stroke (0.96 px on 32), never thinner than 0.7 px so
/// it still reads at 16 px.
pub(crate) fn outline_width(px: u32) -> f32 {
    (0.96 * px as f32 / 32.).max(0.7)
}

/// Lays an ink outline `width` pixels wide behind the subject, in place on
/// a square premultiplied linear RGBA picture: the ink's coverage at a
/// pixel is the strongest nearby coverage, faded by distance, so the
/// outline is anti-aliased like the render. The subject stays on top.
pub(crate) fn outline(rgba: &mut [f32], size: usize, width: f32, ink: [f32; 3]) {
    if rgba.len() != size * size * 4 || width <= 0. {
        return;
    }
    let alpha: Vec<f32> = rgba.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
    let reach = width.ceil() as isize + 1;
    for y in 0..size as isize {
        for x in 0..size as isize {
            let mut ink_a = 0f32;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= size as isize || ny >= size as isize {
                        continue;
                    }
                    let d = ((dx * dx + dy * dy) as f32).sqrt();
                    let fade = (width + 1. - d).clamp(0., 1.);
                    ink_a = ink_a.max(alpha[ny as usize * size + nx as usize] * fade);
                }
            }
            let i = (y as usize * size + x as usize) * 4;
            let a = rgba[i + 3];
            // Subject over ink (both premultiplied).
            let under = ink_a * (1. - a);
            for (c, ink_c) in ink.iter().enumerate() {
                rgba[i + c] += ink_c * under;
            }
            rgba[i + 3] = a + under;
        }
    }
}

/// Premultiplied linear RGBA to straight 8-bit sRGB with alpha.
pub(crate) fn finish(mut rgba: Vec<f32>) -> Vec<u8> {
    for p in rgba.as_chunks_mut::<4>().0 {
        let a = p[3];
        if a > 0. {
            for c in &mut p[..3] {
                *c = (*c / a).clamp(0., 1.);
            }
        }
    }
    operandi_cycles::srgb8(&rgba)
}

/// Which outline a picture has. Every icon comes in two sets: a dark
/// outline for light interface colours and a light outline for dark ones
/// (Jason, 2026-11-20), with the same file names in each set's folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outline {
    Dark,
    Light,
    /// `--no-outline`: one set, no outline.
    None,
}

impl Outline {
    /// The set's folder under the icon's own.
    pub(crate) fn folder(self) -> &'static str {
        match self {
            Outline::Dark => crate::rendered_icons::IconSet::DarkOutline.folder(),
            Outline::Light => crate::rendered_icons::IconSet::LightOutline.folder(),
            Outline::None => "no-outline",
        }
    }

    fn color(self) -> Option<[f32; 3]> {
        let hex = match self {
            Outline::Dark => icon_studio::INK,
            Outline::Light => icon_studio::LIGHT_INK,
            Outline::None => return None,
        };
        Some(icon_studio::linear_from_hex(hex).map(|c| c as f32))
    }

    /// Whether this set suits a light background (the contact sheet's top
    /// panel); the unoutlined set goes on both.
    fn for_light_background(self) -> bool {
        self != Outline::Light
    }

    fn for_dark_background(self) -> bool {
        self != Outline::Dark
    }
}

/// One finished icon picture.
pub(crate) struct Picture {
    pub px: u32,
    pub outline: Outline,
    pub rgba8: Vec<u8>,
}

/// Renders `scene` (a studio render scene) to a `px`-pixel square,
/// premultiplied linear RGBA with no outline yet.
fn render_picture(scene: &Scene, px: u32, job: &RenderJob) -> Result<Vec<f32>, String> {
    let view = operandi_cycles::scene_camera_view(scene, 1.).ok_or("The studio has no camera")?;
    let mut settings = operandi_cycles::Settings::studio(px, px, view)
        .with_scene_world(scene)
        .with_only_scene_lights(scene);
    settings.samples = job.samples.unwrap_or(scene.render.samples.max(1));
    // Denoising smears edges on pictures this small; the samples do the
    // work instead.
    settings.denoise = false;
    settings.adaptive_threshold = 0.;
    settings.transparent = true;
    let meshes = operandi_cycles::scene_meshes(scene).map_err(|e| e.to_string())?;
    let image = operandi_cycles::render(&meshes, &settings).map_err(|e| e.to_string())?;
    Ok(image.rgba)
}

/// `rgba` (one render) finished with `kind`'s outline.
fn outlined(rgba: &[f32], px: u32, kind: Outline) -> Picture {
    let mut rgba = rgba.to_vec();
    if let Some(color) = kind.color() {
        outline(&mut rgba, px as usize, outline_width(px), color);
    }
    Picture {
        px,
        outline: kind,
        rgba8: finish(rgba),
    }
}

/// One icon's pictures: each size at 1× and 2×, in each outline set.
pub(crate) struct IconSet {
    pub slug: String,
    pub pictures: Vec<(u32, u32, Picture)>,
}

fn save_png(path: &Path, p: &Picture) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    image::save_buffer(path, &p.rgba8, p.px, p.px, image::ExtendedColorType::Rgba8)
        .map_err(|e| format!("Couldn't write {}: {e}", path.display()))
}

/// Every (size, scale) rendered per icon: each of [`SIZES`] at 1× and 2×,
/// then [`LARGEST`].
pub(crate) fn renders() -> impl Iterator<Item = (u32, u32)> {
    SIZES
        .into_iter()
        .flat_map(|s| [(s, 1), (s, 2)])
        .chain([LARGEST])
}
/// The biggest picture: 32 px at 4×, 128 px across.
pub(crate) const LARGEST: (u32, u32) = (32, 4);

/// The file name for `slug` at `size` and `scale`.
pub(crate) fn file_name(slug: &str, size: u32, scale: u32) -> String {
    if scale == 1 {
        format!("{slug}-{size}.png")
    } else {
        format!("{slug}-{size}@{scale}x.png")
    }
}

fn render_icon(studio: &Scene, file: &Path, job: &RenderJob) -> Result<IconSet, String> {
    let icon = operandi_core::persistence::load(file).map_err(|e| e.to_string())?;
    for tag in icon_studio::unknown_tags(&icon) {
        eprintln!(
            "{}: polygon tag {tag:?} names no studio material; it renders as Clay",
            file.display()
        );
    }
    let scene = icon_studio::render_scene(studio, &icon).map_err(|e| e.to_string())?;
    let slug = slug_of(file);
    let mut pictures = Vec::new();
    let sets: &[Outline] = if job.outline {
        &[Outline::Dark, Outline::Light]
    } else {
        &[Outline::None]
    };
    for (size, scale) in renders() {
        {
            // One render, outlined once per set.
            let rgba = render_picture(&scene, size * scale, job)?;
            for &kind in sets {
                let picture = outlined(&rgba, size * scale, kind);
                save_png(
                    &job.out
                        .join(&slug)
                        .join(kind.folder())
                        .join(file_name(&slug, size, scale)),
                    &picture,
                )?;
                pictures.push((size, scale, picture));
            }
        }
    }
    Ok(IconSet { slug, pictures })
}

// ---- Contact sheet --------------------------------------------------------

/// The generator's preview backgrounds and text colours (`previews.py`),
/// so rendered and drawn icons can be compared on the same ground.
const LIGHT_BG: [u8; 3] = [0xF8, 0xFA, 0xFB];
const LIGHT_FG: [u8; 3] = [0x26, 0x34, 0x42];
const DARK_BG: [u8; 3] = [0x20, 0x26, 0x2E];
const DARK_FG: [u8; 3] = [0xDC, 0xE3, 0xE9];
const ROW: u32 = 76;
const LABEL_W: u32 = 150;
const HEADER: u32 = 28;
const TEXT_PX: f32 = 13.;
/// The 16 px picture is also shown 4× bigger, pixel for pixel, to judge
/// its pixels.
const ZOOM: u32 = 4;

/// Column widths of one panel after the label: 1× sizes, 2× sizes, then
/// the zoomed 16 px.
fn columns() -> Vec<(String, u32)> {
    let mut cols: Vec<(String, u32)> = SIZES.iter().map(|s| (format!("{s}"), s + 16)).collect();
    cols.extend(SIZES.iter().map(|s| (format!("{s}@2x"), 2 * s + 16)));
    cols.push((format!("16 ×{ZOOM}"), 16 * ZOOM + 16));
    cols
}

fn panel_width() -> u32 {
    LABEL_W + columns().iter().map(|c| c.1).sum::<u32>() + 8
}

/// The contact sheet's size for `rows` icons: a light panel above a dark
/// one.
pub(crate) fn sheet_size(rows: usize) -> (u32, u32) {
    let panel_h = HEADER + ROW * rows as u32 + 8;
    (panel_width(), panel_h * 2)
}

struct Canvas {
    w: u32,
    h: u32,
    px: Vec<u8>,
}

impl Canvas {
    fn fill(&mut self, x0: u32, y0: u32, w: u32, h: u32, c: [u8; 3]) {
        for y in y0..(y0 + h).min(self.h) {
            for x in x0..(x0 + w).min(self.w) {
                let i = ((y * self.w + x) * 4) as usize;
                self.px[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
    }

    /// Lays straight sRGB RGBA `src` (`sw`×`sh`) over the canvas at
    /// (`x0`, `y0`), each source pixel `zoom`×`zoom`. Blends in sRGB, as a
    /// UI toolkit draws an icon.
    fn blit(&mut self, src: &[u8], sw: u32, sh: u32, x0: u32, y0: u32, zoom: u32) {
        for y in 0..sh * zoom {
            for x in 0..sw * zoom {
                let (cx, cy) = (x0 + x, y0 + y);
                if cx >= self.w || cy >= self.h {
                    continue;
                }
                let s = (((y / zoom) * sw + x / zoom) * 4) as usize;
                let d = ((cy * self.w + cx) * 4) as usize;
                let a = u32::from(src[s + 3]);
                for c in 0..3 {
                    let v = u32::from(src[s + c]) * a + u32::from(self.px[d + c]) * (255 - a);
                    self.px[d + c] = (v / 255) as u8;
                }
            }
        }
    }

    fn text(&mut self, font: &ab_glyph::FontRef, s: &str, x0: f32, baseline: f32, c: [u8; 3]) {
        use ab_glyph::{Font, ScaleFont};
        let scaled = font.as_scaled(TEXT_PX);
        let mut x = x0;
        for ch in s.chars() {
            let id = font.glyph_id(ch);
            let glyph = id.with_scale_and_position(TEXT_PX, ab_glyph::point(x, baseline));
            x += scaled.h_advance(id);
            let Some(outlined) = font.outline_glyph(glyph) else {
                continue;
            };
            let b = outlined.px_bounds();
            outlined.draw(|gx, gy, cov| {
                let (px, py) = (
                    b.min.x as i64 + i64::from(gx),
                    b.min.y as i64 + i64::from(gy),
                );
                if px < 0 || py < 0 || px >= i64::from(self.w) || py >= i64::from(self.h) {
                    return;
                }
                let d = ((py as u32 * self.w + px as u32) * 4) as usize;
                let a = cov.clamp(0., 1.);
                for (k, &ck) in c.iter().enumerate() {
                    let v = f32::from(ck) * a + f32::from(self.px[d + k]) * (1. - a);
                    self.px[d + k] = v.round() as u8;
                }
            });
        }
    }
}

/// Egui's own UI font (Ubuntu Light), for the sheet's labels.
fn label_font_bytes() -> Option<Vec<u8>> {
    let fonts = egui::FontDefinitions::default();
    let data = fonts
        .font_data
        .get("Ubuntu-Light")
        .or_else(|| fonts.font_data.values().next())?;
    Some(data.font.to_vec())
}

/// Draws the contact sheet: one row per icon, every size at 1× and 2×
/// at their real pixel size, and the 16 px picture zoomed, once on the
/// light background (the dark-outline set) and once on the dark (the
/// light-outline set).
pub(crate) fn contact_sheet(icons: &[IconSet]) -> (u32, u32, Vec<u8>) {
    let (w, h) = sheet_size(icons.len());
    let mut canvas = Canvas {
        w,
        h,
        px: vec![0; (w * h * 4) as usize],
    };
    let font_bytes = label_font_bytes();
    let font = font_bytes
        .as_deref()
        .and_then(|b| ab_glyph::FontRef::try_from_slice(b).ok());
    let panel_h = h / 2;
    for (panel, (bg, fg)) in [(LIGHT_BG, LIGHT_FG), (DARK_BG, DARK_FG)]
        .into_iter()
        .enumerate()
    {
        let top = panel as u32 * panel_h;
        let suits = |o: Outline| {
            if panel == 0 {
                o.for_light_background()
            } else {
                o.for_dark_background()
            }
        };
        canvas.fill(0, top, w, panel_h, bg);
        let mut x = LABEL_W;
        for (name, width) in columns() {
            if let Some(font) = &font {
                canvas.text(font, &name, x as f32 + 4., (top + 19) as f32, fg);
            }
            x += width;
        }
        for (row, icon) in icons.iter().enumerate() {
            let row_top = top + HEADER + row as u32 * ROW;
            if let Some(font) = &font {
                canvas.text(font, &icon.slug, 8., (row_top + ROW / 2 + 4) as f32, fg);
            }
            let mut x = LABEL_W;
            let cols = columns();
            let mut col = 0;
            for scale in [1, 2] {
                for size in SIZES {
                    if let Some((_, _, p)) = icon
                        .pictures
                        .iter()
                        .find(|(s, k, p)| *s == size && *k == scale && suits(p.outline))
                    {
                        let y = row_top + (ROW - p.px) / 2;
                        canvas.blit(&p.rgba8, p.px, p.px, x + 8, y, 1);
                    }
                    x += cols[col].1;
                    col += 1;
                }
            }
            if let Some((_, _, p)) = icon
                .pictures
                .iter()
                .find(|(s, k, p)| *s == 16 && *k == 1 && suits(p.outline))
            {
                let y = row_top + (ROW - p.px * ZOOM) / 2;
                canvas.blit(&p.rgba8, p.px, p.px, x + 8, y, ZOOM);
            }
        }
    }
    (w, h, canvas.px)
}

fn studio_of(job: &RenderJob) -> Result<Scene, String> {
    match &job.studio {
        Some(path) => operandi_core::persistence::load(path)
            .map_err(|e| format!("Couldn't open the studio {}: {e}", path.display())),
        None => icon_studio::studio_scene().map_err(|e| e.to_string()),
    }
}

fn save_scene(path: &Path, scene: &Scene) -> Result<(), String> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    operandi_core::persistence::save(path, scene).map_err(|e| e.to_string())
}

fn run(job: &Job) -> Result<(), String> {
    match job {
        Job::Template { out } => {
            save_scene(
                out,
                &icon_studio::studio_scene().map_err(|e| e.to_string())?,
            )?;
            println!("{}", out.display());
            Ok(())
        }
        Job::Sample { sample, out } => {
            let scene = icon_studio::sample_scene(*sample).map_err(|e| e.to_string())?;
            save_scene(out, &scene)?;
            println!("{}", out.display());
            Ok(())
        }
        Job::Render(job) => {
            if !operandi_cycles::available() {
                return Err(
                    "This build has no Cycles renderer (build with --features cycles)".into(),
                );
            }
            let studio = studio_of(job)?;
            let files = icon_files(&job.inputs)?;
            let mut icons = Vec::new();
            let mut failed = 0;
            for file in &files {
                match render_icon(&studio, file, job) {
                    Ok(set) => {
                        println!("{}", job.out.join(&set.slug).display());
                        icons.push(set);
                    }
                    Err(e) => {
                        eprintln!("failed: {}: {e}", file.display());
                        failed += 1;
                    }
                }
            }
            if !icons.is_empty() {
                let (w, h, px) = contact_sheet(&icons);
                let sheet = job.out.join("contact-sheet.png");
                image::save_buffer(&sheet, &px, w, h, image::ExtendedColorType::Rgba8)
                    .map_err(|e| format!("Couldn't write {}: {e}", sheet.display()))?;
                println!("{}", sheet.display());
            }
            println!("Rendered {} icons; {failed} failed", icons.len());
            if failed > 0 {
                Err(format!("{failed} icons failed"))
            } else {
                Ok(())
            }
        }
    }
}

/// `operandi-app --icon-studio ...`. Returns the process exit code.
pub(crate) fn command(args: &[String]) -> i32 {
    let job = match parse(args) {
        Ok(job) => job,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return 2;
        }
    };
    match run(&job) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn verbs_parse_with_their_defaults() {
        assert_eq!(
            parse(&args("template s.opscene")).unwrap(),
            Job::Template {
                out: "s.opscene".into()
            }
        );
        assert_eq!(
            parse(&args("sample prim-cube c.opscene")).unwrap(),
            Job::Sample {
                sample: icon_studio::Sample::Cube,
                out: "c.opscene".into()
            }
        );
        let Job::Render(r) = parse(&args("render icons a.opscene --out o")).unwrap() else {
            panic!("not a render");
        };
        assert_eq!(r.inputs, vec![PathBuf::from("icons"), "a.opscene".into()]);
        assert_eq!(r.out, PathBuf::from("o"));
        assert_eq!((r.studio, r.samples, r.outline), (None, None, true));
        let Job::Render(r) = parse(&args(
            "render a.opscene --out o --studio s.opscene --samples 64 --no-outline",
        ))
        .unwrap() else {
            panic!("not a render");
        };
        assert_eq!(r.studio, Some("s.opscene".into()));
        assert_eq!(r.samples, Some(64));
        assert!(!r.outline);
    }

    #[test]
    fn bad_arguments_are_errors_not_panics() {
        for bad in [
            "",
            "paint x",
            "template",
            "template a b",
            "sample teapot t.opscene",
            "sample prim-cube",
            "render a.opscene",
            "render --out o",
            "render a.opscene --out",
            "render a.opscene --out o --samples 0",
            "render a.opscene --out o --wobble",
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn folders_list_their_scenes_sorted_and_other_files_are_refused() {
        let dir = std::env::temp_dir().join(format!("icon-studio-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["b.opscene", "a.opscene", "notes.txt"] {
            std::fs::write(dir.join(name), "").unwrap();
        }
        let files = icon_files(std::slice::from_ref(&dir)).unwrap();
        assert_eq!(files, vec![dir.join("a.opscene"), dir.join("b.opscene")]);
        assert!(icon_files(&[dir.join("notes.txt")]).is_err());
        let empty = dir.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(icon_files(&[empty]).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_names_mark_the_2x_pictures() {
        assert_eq!(file_name("prim-cube", 16, 1), "prim-cube-16.png");
        assert_eq!(file_name("prim-cube", 16, 2), "prim-cube-16@2x.png");
        assert_eq!(file_name("prim-cube", 32, 4), "prim-cube-32@4x.png");
        let px: Vec<u32> = renders().map(|(s, k)| s * k).collect();
        assert_eq!(px.iter().max(), Some(&128));
    }

    #[test]
    fn the_outline_rings_the_silhouette_and_keeps_the_subject() {
        // A 3×3 opaque white square in the middle of a 9×9 picture.
        let size = 9;
        let mut rgba = vec![0f32; size * size * 4];
        for y in 3..6 {
            for x in 3..6 {
                let i = (y * size + x) * 4;
                rgba[i..i + 4].copy_from_slice(&[1., 1., 1., 1.]);
            }
        }
        let ink = [0.1, 0.2, 0.3];
        outline(&mut rgba, size, 1., ink);
        let at = |x: usize, y: usize| &rgba[(y * size + x) * 4..(y * size + x) * 4 + 4];
        assert_eq!(at(4, 4), &[1., 1., 1., 1.], "the subject stays on top");
        // Right beside the square: full ink.
        assert_eq!(at(2, 4), &[0.1, 0.2, 0.3, 1.]);
        // Two pixels out: none.
        assert_eq!(at(1, 4)[3], 0.);
        assert_eq!(at(0, 0)[3], 0.);
        // The diagonal corner pixel is partly covered.
        let corner = at(2, 2)[3];
        assert!(corner > 0. && corner < 1., "{corner}");
    }

    #[test]
    fn outlines_scale_with_the_picture_but_stay_visible() {
        assert_eq!(outline_width(32), 0.96);
        assert_eq!(outline_width(64), 1.92);
        assert_eq!(outline_width(16), 0.7);
    }

    #[test]
    fn finish_unpremultiplies_and_keeps_alpha() {
        let out = finish(vec![0.5, 0., 0., 0.5, 0., 0., 0., 0.]);
        assert_eq!(&out[..4], &[255, 0, 0, 128]);
        assert_eq!(&out[4..], &[0, 0, 0, 0]);
    }

    #[test]
    fn each_panel_shows_the_outline_set_made_for_its_background() {
        let solid = |px: u32, outline: Outline, c: [u8; 4]| Picture {
            px,
            outline,
            rgba8: c.repeat((px * px) as usize),
        };
        let mut pictures = Vec::new();
        for size in SIZES {
            for scale in [1, 2] {
                let px = size * scale;
                pictures.push((size, scale, solid(px, Outline::Dark, [255, 0, 0, 255])));
                pictures.push((size, scale, solid(px, Outline::Light, [0, 0, 255, 255])));
            }
        }
        let (w, h, px) = contact_sheet(&[IconSet {
            slug: "x".into(),
            pictures,
        }]);
        let half = (w * (h / 2) * 4) as usize;
        let count = |part: &[u8], c: [u8; 3]| {
            part.as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[..3] == c)
                .count()
        };
        let (light, dark) = px.split_at(half);
        assert!(count(light, [255, 0, 0]) > 0 && count(light, [0, 0, 255]) == 0);
        assert!(count(dark, [0, 0, 255]) > 0 && count(dark, [255, 0, 0]) == 0);
    }

    #[test]
    fn outline_sets_have_their_own_folders_and_colours() {
        assert_eq!(Outline::Dark.folder(), "dark-outline");
        assert_eq!(Outline::Light.folder(), "light-outline");
        assert!(Outline::None.color().is_none());
        let dark = Outline::Dark.color().unwrap();
        let light = Outline::Light.color().unwrap();
        assert!(light.iter().sum::<f32>() > 4. * dark.iter().sum::<f32>());
    }

    #[test]
    fn the_contact_sheet_shows_every_picture_at_real_size_on_both_backgrounds() {
        let picture = |px: u32| Picture {
            px,
            outline: Outline::None,
            rgba8: [255, 0, 0, 255].repeat((px * px) as usize),
        };
        let mut pictures = Vec::new();
        for size in SIZES {
            for scale in [1, 2] {
                pictures.push((size, scale, picture(size * scale)));
            }
        }
        let icons = vec![IconSet {
            slug: "prim-cube".into(),
            pictures,
        }];
        let (w, h, px) = contact_sheet(&icons);
        assert_eq!((w, h), sheet_size(1));
        let red = px
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[..3] == [255, 0, 0])
            .count() as u32;
        let one_panel: u32 =
            SIZES.iter().map(|s| s * s + 4 * s * s).sum::<u32>() + 16 * 16 * ZOOM * ZOOM;
        assert_eq!(red, 2 * one_panel, "every picture, both panels, unscaled");
        // The two panels' backgrounds.
        assert_eq!(&px[..3], &LIGHT_BG);
        let last = px.len() - 4;
        assert_eq!(&px[last..last + 3], &DARK_BG);
    }
}
