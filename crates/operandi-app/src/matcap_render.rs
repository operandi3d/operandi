//! `operandi-app --render-matcap`: Operandi's own matcaps ("Our own
//! renders" in the preset-browser matcap spec, 2026-11-12). No window
//! opens.
//!
//! A plain smooth sphere (core `preset::matcap`) fills a square picture,
//! seen by an orthographic camera straight on and lit only by one studio
//! HDRI, rendered with Cycles. Outside the circle is black (or transparent
//! with `--transparent`). Toon looks are a clay render whose lightness is
//! quantised into flat bands afterwards: the renderer has no stepped ramp.
//!
//! ```text
//! operandi-app --render-matcap <preset.oppreset | look name> <out.png>
//!              --hdri <file> [--size 512] [--samples 128] [--rotate <deg>]
//!              [--exposure <stops>] [--transparent]
//! operandi-app --render-matcap --batch <folder> --hdri <file> [same options]
//! ```
//!
//! The batch form renders the fixed set (`preset::matcap::LOOKS`) to
//! `<folder>/<Name>.png` and writes `<folder>/CREDITS.txt`.
use operandi_core::preset::matcap::{self, CodeLook, Look, LookPost};
use std::path::{Path, PathBuf};

const USAGE: &str = "usage: operandi-app --render-matcap <preset.oppreset | look name> <out.png> \
--hdri <file> [--size 512] [--samples 128] [--rotate <deg>] [--exposure <stops>] [--transparent]\n       \
operandi-app --render-matcap --batch <folder> --hdri <file> [same options]";

/// What to render the sphere with.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Subject {
    /// A material preset file.
    Preset(PathBuf),
    /// One of the fixed looks.
    Look(&'static Look),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Job {
    One { subject: Subject, out: PathBuf },
    Batch { folder: PathBuf },
}

/// Parsed `--render-matcap` arguments.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Options {
    pub job: Job,
    pub hdri: PathBuf,
    /// Picture width and height in pixels.
    pub size: u32,
    pub samples: u32,
    /// Turns the HDRI about the vertical axis, degrees.
    pub rotate: f64,
    /// Leave alpha 0 outside the circle instead of black.
    pub transparent: bool,
    /// Brightens (+) or darkens (−) the HDRI, in stops.
    pub exposure: f64,
}

pub(crate) const DEFAULT_SIZE: u32 = 512;
pub(crate) const DEFAULT_SAMPLES: u32 = 128;
pub(crate) const DEFAULT_EXPOSURE: f64 = 0.;

/// Parses the arguments after `--render-matcap`.
pub(crate) fn parse(args: &[String]) -> Result<Options, String> {
    let mut positional = Vec::new();
    let mut hdri = None;
    let mut size = DEFAULT_SIZE;
    let mut samples = DEFAULT_SAMPLES;
    let mut rotate = 0.;
    let mut transparent = false;
    let mut exposure = DEFAULT_EXPOSURE;
    let mut batch = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match arg.as_str() {
            "--hdri" => hdri = Some(PathBuf::from(value("--hdri")?)),
            "--size" => {
                size = value("--size")?
                    .parse()
                    .ok()
                    .filter(|s| (16..=8192).contains(s))
                    .ok_or("--size takes a whole number of pixels from 16 to 8192")?;
            }
            "--samples" => {
                samples = value("--samples")?
                    .parse()
                    .ok()
                    .filter(|&s: &u32| s > 0)
                    .ok_or("--samples takes a whole number above 0")?;
            }
            "--rotate" => {
                rotate = value("--rotate")?
                    .parse()
                    .ok()
                    .filter(|r: &f64| r.is_finite())
                    .ok_or("--rotate takes degrees")?;
            }
            "--exposure" => {
                exposure = value("--exposure")?
                    .parse()
                    .ok()
                    .filter(|e: &f64| e.is_finite() && e.abs() <= 16.)
                    .ok_or("--exposure takes stops from -16 to 16")?;
            }
            "--transparent" => transparent = true,
            "--batch" => batch = Some(PathBuf::from(value("--batch")?)),
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => positional.push(other.to_string()),
        }
    }
    let hdri = hdri.ok_or("--hdri <file> is required: the studio HDRI that lights the sphere")?;
    let job = match (batch, positional.as_slice()) {
        (Some(folder), []) => Job::Batch { folder },
        (Some(_), _) => return Err("--batch takes no other subject or output".into()),
        (None, [subject, out]) => Job::One {
            subject: subject_of(subject)?,
            out: PathBuf::from(out),
        },
        (None, _) => return Err("give a preset or look name and an output .png".into()),
    };
    Ok(Options {
        job,
        hdri,
        size,
        samples,
        rotate,
        transparent,
        exposure,
    })
}

/// A `.oppreset` path is a preset; anything else must be a look name.
fn subject_of(arg: &str) -> Result<Subject, String> {
    let path = Path::new(arg);
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(operandi_core::preset::PRESET_EXTENSION))
    {
        return Ok(Subject::Preset(path.to_path_buf()));
    }
    matcap::look_named(arg).map(Subject::Look).ok_or_else(|| {
        let names: Vec<_> = matcap::LOOKS.iter().map(|l| l.name).collect();
        format!(
            "{arg:?} is neither a .oppreset file nor a look ({})",
            names.join(", ")
        )
    })
}

/// The Cycles view for the matcap framing: orthographic, straight on.
pub(crate) fn view() -> operandi_cycles::View {
    use glam::DVec3;
    let f = matcap::framing();
    let mut view = operandi_cycles::View::perspective(
        f.eye,
        DVec3::X,
        DVec3::Y,
        (f.target - f.eye).normalize(),
        0.5,
    );
    view.orthographic = Some(f.half_height);
    view
}

/// The world: the HDRI alone, seen by every ray, turned `rotate` degrees
/// about +Y.
fn hdri_world(hdri: &Path, rotate: f64, exposure: f64) -> operandi_cycles::EnvironmentLayer {
    let mut layer = operandi_cycles::EnvironmentLayer::flat([0.; 3], 2f64.powf(exposure) as f32);
    layer.layers = vec![operandi_cycles::EnvironmentTexture {
        source: operandi_cycles::EnvironmentTextureSource::Image(hdri.display().to_string()),
        opacity: 1.,
        mix: operandi_core::environment::environment_mix(
            operandi_core::shader_tree::BlendMode::default(),
        )
        .cycles_code(),
        invert: false,
        to_image: glam::DMat3::from_rotation_y(-rotate.to_radians()),
        light_probe: false,
    }];
    layer
}

/// Renders `scene` as a matcap: premultiplied linear RGBA, alpha the
/// sphere's coverage.
fn render_scene(scene: &operandi_core::Scene, o: &Options) -> Result<Vec<f32>, String> {
    let mut settings = operandi_cycles::Settings::studio(o.size, o.size, view());
    settings.samples = o.samples;
    settings.transparent = true;
    // Only the HDRI lights the sphere: no studio sun, no lights.
    settings.lights = Some(Vec::new());
    settings.sun_strength = 0.;
    settings.world = vec![hdri_world(&o.hdri, o.rotate, o.exposure)];
    let meshes = operandi_cycles::scene_meshes(scene).map_err(|e| e.to_string())?;
    let image = operandi_cycles::render(&meshes, &settings).map_err(|e| e.to_string())?;
    Ok(image.rgba)
}

/// The darkest toon band's sRGB lightness, as a fraction of the brightest.
const TOON_SHADOW: f32 = 0.45;

fn srgb_of(v: f32) -> f32 {
    let v = v.clamp(0., 1.);
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}

fn linear_of(s: f32) -> f32 {
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// The sphere's normal (toward the camera +Z) under pixel `(x, y)` of a
/// `size`-pixel matcap, `None` outside the disc.
fn normal_at(x: usize, y: usize, size: usize) -> Option<glam::DVec3> {
    let s = size.max(1) as f64;
    let nx = (x as f64 + 0.5) / s * 2. - 1.;
    let ny = 1. - (y as f64 + 0.5) / s * 2.;
    let zz = 1. - nx * nx - ny * ny;
    (zz >= 0.).then(|| glam::DVec3::new(nx, ny, zz.sqrt()))
}

/// Turns a clay render into a toon matcap with `bands` flat steps, in
/// place on a square `size`-pixel premultiplied linear RGBA picture.
///
/// The renderer has no stepped ramp, so this is post-processing. A studio
/// HDRI's diffuse light is soft and uneven, and banding it directly gives
/// blotchy edges, so the clay's sRGB lightness is first fitted to its
/// dominant light direction (lightness ≈ a + b·normal, least squares over
/// the sphere). That fitted field is split into bands of equal area, so
/// band edges are clean circles about the light.
/// Every pixel takes the clay's average hue at its band's lightness, the
/// top band the clay's brightest. Coverage (the rim's anti-aliasing) is
/// kept.
pub(crate) fn toon_bands(rgba: &mut [f32], size: usize, bands: u8) {
    let bands = bands.max(2);
    let pixels = rgba.as_chunks_mut::<4>().0;
    if pixels.len() != size * size {
        return;
    }
    let unpremult = |p: &[f32; 4]| {
        let a = p[3].max(1e-6);
        [p[0] / a, p[1] / a, p[2] / a]
    };
    let luma = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    // Normal equations for lightness = a + b·n.
    let mut ata = glam::DMat4::ZERO;
    let mut atb = glam::DVec4::ZERO;
    let mut mean = [0f64; 3];
    let mut count = 0usize;
    for (i, p) in pixels.iter().enumerate() {
        let Some(n) = normal_at(i % size, i / size, size).filter(|_| p[3] > 0.5) else {
            continue;
        };
        let c = unpremult(p);
        let row = glam::DVec4::new(1., n.x, n.y, n.z);
        ata += glam::DMat4::from_cols(row * row.x, row * row.y, row * row.z, row * row.w);
        atb += row * f64::from(srgb_of(luma(c)));
        for k in 0..3 {
            mean[k] += f64::from(c[k]);
        }
        count += 1;
    }
    if count < 4 || ata.determinant().abs() < 1e-12 {
        return;
    }
    let fit = ata.inverse() * atb;
    let fitted = |n: glam::DVec3| (fit.x + fit.y * n.x + fit.z * n.y + fit.w * n.z) as f32;
    let mut values: Vec<f32> = pixels
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            normal_at(i % size, i / size, size)
                .filter(|_| p[3] > 0.5)
                .map(fitted)
        })
        .collect();
    values.sort_by(f32::total_cmp);
    let (lo, hi) = (values[0], values[values.len() - 1]);
    // Band edges split the sphere's area evenly (quantiles of the fitted
    // field), so no band swallows the picture when the light is broad.
    let edges: Vec<f32> = (1..bands)
        .map(|k| values[values.len() * usize::from(k) / usize::from(bands)])
        .collect();
    let mean = mean.map(|m| (m / count as f64) as f32);
    let mean_luma = luma(mean);
    if mean_luma <= 0. || hi <= lo {
        return;
    }
    let top_s = hi.clamp(0., 1.);
    let n_bands = f32::from(bands);
    for (i, p) in pixels.iter_mut().enumerate() {
        if p[3] <= 0. {
            continue;
        }
        // Rim pixels just outside the ideal disc use the nearest normal.
        let n = normal_at(i % size, i / size, size).unwrap_or_else(|| {
            let s = size as f64;
            let v = glam::DVec2::new(
                ((i % size) as f64 + 0.5) / s * 2. - 1.,
                1. - ((i / size) as f64 + 0.5) / s * 2.,
            )
            .normalize_or_zero();
            v.extend(0.)
        });
        let f = fitted(n);
        let band = edges.iter().filter(|&&e| f >= e).count() as f32;
        let level_s = top_s * (TOON_SHADOW + (1. - TOON_SHADOW) * band / (n_bands - 1.));
        let scale = linear_of(level_s) / mean_luma;
        let a = p[3];
        for k in 0..3 {
            p[k] = mean[k] * scale * a;
        }
    }
}

/// Premultiplied linear RGBA to 8-bit sRGB: over black, or unpremultiplied
/// with alpha kept.
pub(crate) fn finish(mut rgba: Vec<f32>, transparent: bool) -> Vec<u8> {
    for p in rgba.as_chunks_mut::<4>().0 {
        if transparent {
            let a = p[3];
            if a > 0. {
                for c in &mut p[..3] {
                    *c /= a;
                }
            }
        } else {
            p[3] = 1.;
        }
    }
    operandi_cycles::srgb8(&rgba)
}

/// The scene for one subject, and where its material came from (for the
/// credits).
fn scene_for(subject: &Subject) -> Result<(operandi_core::Scene, String), String> {
    match subject {
        Subject::Preset(path) => {
            let preset =
                operandi_core::persistence::load_preset(path).map_err(|e| e.to_string())?;
            let scene = matcap::preset_scene(&preset).map_err(|e| e.to_string())?;
            Ok((scene, format!("material preset {}", path.display())))
        }
        Subject::Look(look) => Ok((
            matcap::code_scene(&look.material).map_err(|e| e.to_string())?,
            describe(&look.material, look.post),
        )),
    }
}

fn describe(c: &CodeLook, post: LookPost) -> String {
    let mut s = format!(
        "defined in code: base {:?}, metalness {}, roughness {}",
        c.base_color, c.metallic, c.roughness
    );
    if c.coat > 0. {
        s += &format!(", coat {} (roughness {})", c.coat, c.coat_roughness);
    }
    if c.subsurface > 0. {
        s += &format!(
            ", subsurface {} {:?} over {} m",
            c.subsurface, c.subsurface_color, c.subsurface_distance
        );
    }
    if let LookPost::Toon(n) = post {
        s += &format!(
            "; post-processed: the render's lightness fitted to its dominant light \
             direction and quantised into {n} flat bands (the renderer has no stepped ramp)"
        );
    }
    s
}

/// Renders one subject to `out`; returns the credit line's source.
fn render_one(subject: &Subject, out: &Path, o: &Options) -> Result<String, String> {
    let (scene, source) = scene_for(subject)?;
    let mut rgba = render_scene(&scene, o)?;
    if let Subject::Look(Look {
        post: LookPost::Toon(n),
        ..
    }) = subject
    {
        toon_bands(&mut rgba, o.size as usize, *n);
    }
    let bytes = finish(rgba, o.transparent);
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    image::save_buffer(out, &bytes, o.size, o.size, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(source)
}

/// The batch's CREDITS.txt.
pub(crate) fn credits(o: &Options, lines: &[(&str, String)]) -> String {
    let hdri = o.hdri.file_name().map_or_else(
        || o.hdri.display().to_string(),
        |n| n.to_string_lossy().into(),
    );
    let mut s = String::from(
        "Operandi matcaps (our own renders)\n\
         ==================================\n\n\
         Rendered by Operandi (operandi-app --render-matcap) with Cycles: a plain\n\
         smooth sphere, orthographic camera straight on, lit only by the HDRI below,\n\
         black outside the circle.\n\n\
         Licence of these pictures: CC0-1.0.\n\n",
    );
    s += &format!(
        "HDRI: {hdri} ({} px renders, {} samples, rotated {}°, exposure {:+} stops)\n  {}\n\n",
        o.size,
        o.samples,
        o.rotate,
        o.exposure,
        hdri_credit(&hdri)
    );
    s += "Per image:\n";
    for (name, source) in lines {
        s += &format!("  {name}.png: {source}\n");
    }
    s
}

/// Known HDRIs' source and licence; others are named only.
fn hdri_credit(file: &str) -> String {
    // "studio_small_09_1k.hdr" → "studio_small_09".
    let stem = file.split('.').next().unwrap_or(file);
    let slug = match stem.rsplit_once('_') {
        Some((head, res))
            if res.ends_with('k') && res[..res.len() - 1].chars().all(|c| c.is_ascii_digit()) =>
        {
            head
        }
        _ => stem,
    };
    if ["studio_small_08", "studio_small_09", "brown_photostudio_02"].contains(&slug) {
        format!("Poly Haven, https://polyhaven.com/a/{slug}, CC0-1.0")
    } else {
        "source and licence: see the HDRI's own credits".into()
    }
}

/// `operandi-app --render-matcap ...`. Returns the process exit code.
pub(crate) fn command(args: &[String]) -> i32 {
    let o = match parse(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return 2;
        }
    };
    if !operandi_cycles::available() {
        eprintln!("This build has no Cycles renderer (build with --features cycles)");
        return 1;
    }
    if !o.hdri.is_file() {
        eprintln!("No HDRI at {}", o.hdri.display());
        return 1;
    }
    match &o.job {
        Job::One { subject, out } => match render_one(subject, out, &o) {
            Ok(_) => {
                println!("{}", out.display());
                0
            }
            Err(e) => {
                eprintln!("failed: {e}");
                1
            }
        },
        Job::Batch { folder } => {
            let mut lines = Vec::new();
            let mut failed = 0;
            for look in matcap::LOOKS {
                let out = folder.join(format!("{}.png", look.name));
                match render_one(&Subject::Look(look), &out, &o) {
                    Ok(source) => {
                        println!("{}", out.display());
                        lines.push((look.name, source));
                    }
                    Err(e) => {
                        eprintln!("failed: {}: {e}", look.name);
                        failed += 1;
                    }
                }
            }
            if let Err(e) = std::fs::write(folder.join("CREDITS.txt"), credits(&o, &lines)) {
                eprintln!("Couldn't write CREDITS.txt: {e}");
                failed += 1;
            }
            println!("Rendered {} matcaps; {failed} failed", lines.len());
            i32::from(failed > 0)
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
    fn one_look_or_preset_with_defaults() {
        let o = parse(&args("Gold out/gold.png --hdri s.hdr")).unwrap();
        assert_eq!(
            o.job,
            Job::One {
                subject: Subject::Look(matcap::look_named("Gold").unwrap()),
                out: "out/gold.png".into()
            }
        );
        assert_eq!(
            (o.size, o.samples, o.rotate, o.transparent),
            (512, 128, 0., false)
        );
        assert_eq!(o.hdri, PathBuf::from("s.hdr"));
        let o = parse(&args(
            "--hdri s.hdr Lib/Gold.OPPRESET g.png --size 1024 --transparent",
        ))
        .unwrap();
        assert_eq!(
            o.job,
            Job::One {
                subject: Subject::Preset("Lib/Gold.OPPRESET".into()),
                out: "g.png".into()
            }
        );
        assert_eq!(o.size, 1024);
        assert!(o.transparent);
        // Look names ignore case and separators.
        let o = parse(&args("toon-2-step t.png --hdri s.hdr --rotate -90")).unwrap();
        assert!(
            matches!(o.job, Job::One { subject: Subject::Look(l), .. } if l.name == "Toon 2-step")
        );
        assert_eq!(o.rotate, -90.);
    }

    #[test]
    fn batch_form() {
        let o = parse(&args("--batch Matcaps --hdri s.hdr --samples 64")).unwrap();
        assert_eq!(
            o.job,
            Job::Batch {
                folder: "Matcaps".into()
            }
        );
        assert_eq!(o.samples, 64);
        assert!(parse(&args("--batch Matcaps Gold --hdri s.hdr")).is_err());
    }

    #[test]
    fn bad_arguments_are_errors_not_panics() {
        for bad in [
            "Gold out.png",                   // no HDRI
            "Gold --hdri s.hdr",              // no output
            "Plutonium out.png --hdri s.hdr", // unknown look
            "Gold out.png --hdri",            // missing value
            "Gold out.png --hdri s.hdr --size 0",
            "Gold out.png --hdri s.hdr --size big",
            "Gold out.png --hdri s.hdr --samples 0",
            "Gold out.png --hdri s.hdr --rotate nan",
            "Gold out.png --hdri s.hdr --wobble",
            "a b c --hdri s.hdr",
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad} should fail");
        }
    }

    #[test]
    fn the_view_is_orthographic_and_straight_on() {
        let v = view();
        assert_eq!(v.orthographic, Some(matcap::RADIUS));
        assert_eq!(v.forward, -glam::DVec3::Z);
        assert_eq!(v.up, glam::DVec3::Y);
        assert_eq!(v.right, glam::DVec3::X);
        assert_eq!(v.position.truncate(), glam::DVec2::ZERO);
    }

    #[test]
    fn toon_bands_are_clean_flat_steps_that_keep_coverage() {
        // A noisy clay sphere lit from the upper left.
        let size = 64;
        let light = glam::DVec3::new(-0.5, 0.6, 0.6).normalize();
        let mut rgba = vec![0f32; size * size * 4];
        for y in 0..size {
            for x in 0..size {
                let Some(n) = normal_at(x, y, size) else {
                    continue;
                };
                let noise = (((x * 7 + y * 13) % 5) as f32 - 2.) * 0.01;
                let v = (0.15 + 0.5 * n.dot(light).max(0.) as f32 + noise).max(0.);
                let i = (y * size + x) * 4;
                rgba[i..i + 4].copy_from_slice(&[v * 0.9, v * 0.8, v * 0.7, 1.]);
            }
        }
        // A half-covered rim pixel at the left edge's middle.
        let rim = (32 * size) * 4;
        rgba[rim..rim + 4].copy_from_slice(&[0.05, 0.05, 0.05, 0.5]);
        toon_bands(&mut rgba, size, 3);
        let px = rgba.as_chunks::<4>().0;
        let mut levels: Vec<i64> = px
            .iter()
            .filter(|p| p[3] == 1.)
            .map(|p| (p[0] * 1e5).round() as i64)
            .collect();
        levels.sort_unstable();
        levels.dedup();
        assert_eq!(levels.len(), 3, "three flat bands: {levels:?}");
        // Bands step up toward the light: brightest at the upper left,
        // darkest at the lower right, nothing noisy in between.
        let at = |x: usize, y: usize| px[y * size + x][0];
        assert!(at(20, 16) >= at(32, 32) && at(32, 32) >= at(52, 52));
        assert!(at(20, 16) > at(52, 52));
        assert_eq!(px[32 * size][3], 0.5, "rim coverage kept");
        assert_eq!(px[0], [0.; 4], "outside the circle stays empty");
        // The clay's hue is kept (red > green > blue).
        let p = px[32 * size + 32];
        assert!(p[0] > p[1] && p[1] > p[2]);
    }

    #[test]
    fn outside_the_circle_is_black_or_clear() {
        let rgba = vec![0.5, 0.25, 0.1, 0.5, 0., 0., 0., 0.];
        let black = finish(rgba.clone(), false);
        assert_eq!(&black[4..], &[0, 0, 0, 255]);
        assert_eq!(black[3], 255);
        let clear = finish(rgba, true);
        assert_eq!(&clear[4..], &[0, 0, 0, 0]);
        // Unpremultiplied: 0.5 / 0.5 = full red.
        assert_eq!(clear[0], 255);
        assert_eq!(clear[3], 128);
    }

    #[test]
    fn credits_name_the_hdri_and_every_image() {
        let o = parse(&args("--batch M --hdri /x/studio_small_09_1k.hdr")).unwrap();
        let text = credits(&o, &[("Gold", "defined in code".into())]);
        assert!(text.contains("https://polyhaven.com/a/studio_small_09, CC0-1.0"));
        assert!(text.contains("Gold.png: defined in code"));
        assert!(text.contains("Licence of these pictures: CC0-1.0."));
    }
}
