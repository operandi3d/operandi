//! Texture > Atlas Unwrap…, Bake Curvature…, Bake Thickness… and Pad
//! Texture to UVs… (from TextureCraft, texturecraft-mining-2026-11-17).
//! Each is the shared dialog (UIC-80..99) over a command: OK runs
//! `operandi.uv.atlas` (one undo step, UIC-96), or asks where to save
//! (UIC-116's panel) and then runs `operandi.bake.*` with the file, so a
//! typed line does exactly what the dialog does (UIC-160/162). Values are
//! remembered per dialog (UIC-95).
use super::*;
use operandi_core::bake::{BakeMap, BakeOptions};
use operandi_core::uv_atlas::AtlasOptions;

pub(super) enum BakeDialog {
    Atlas(form::Dialog),
    Bake(form::Dialog, BakeMap),
    Pad(form::Dialog),
}

const SIZES: [&str; 6] = ["256", "512", "1024", "2048", "4096", "8192"];

fn size_index(size: u32) -> usize {
    SIZES
        .iter()
        .position(|s| s.parse::<u32>().ok() == Some(size))
        .unwrap_or(2)
}

/// The active mesh's UV map names, for a UV Map field's suggestions.
fn uv_maps(a: &Operandi) -> Vec<String> {
    a.editor
        .scene
        .active()
        .map(|i| i.mesh.uv_maps.keys().cloned().collect())
        .unwrap_or_default()
}

fn atlas_rows(o: &AtlasOptions, maps: Vec<String>) -> Vec<form::Row> {
    use form::{Row, Unit};
    vec![
        Row::text("uv_map", "UV Map", o.uv_map.as_deref().unwrap_or("Texture"))
            .suggestions(maps)
            .tip("The UV map written; made when the mesh has none of that name"),
        Row::number("angle", "Max Angle", Unit::Angle, o.angle.to_degrees())
            .range(1., 90.)
            .tip("How far a chart may bend: polygons turned further start a new chart"),
        Row::number("gaps", "Gaps", Unit::Suffix(" px"), o.gaps)
            .range(0., 64.)
            .tip("Space around each chart, in pixels of a texture of Texture Size"),
        Row::number(
            "resolution",
            "Texture Size",
            Unit::Suffix(" px"),
            o.resolution as f64,
        )
        .range(16., 16384.)
        .tip("The texture size Gaps is measured on"),
    ]
    .into_iter()
    .map(form::Row::fresh)
    .collect()
}

fn bake_rows(o: &BakeOptions, maps: Vec<String>) -> Vec<form::Row> {
    use form::{Row, Unit};
    let mut rows = vec![
        Row::choice("size", "Size", &SIZES, size_index(o.size))
            .tip("Width and height of the picture, in pixels"),
        Row::text("uv_map", "UV Map", o.uv_map.as_deref().unwrap_or("Texture"))
            .suggestions(maps)
            .tip("The UV layout baked into"),
        Row::number("padding", "Padding", Unit::Suffix(" px"), o.padding as f64)
            .range(0., 256.)
            .tip("Pixels of colour grown out from each UV island, so seams don't show"),
        Row::check("fill", "Fill Background", o.fill)
            .tip("Fill the rest of the picture with a smooth blend of the islands"),
        Row::number("rays", "Rays", Unit::Count, o.rays as f64)
            .range(1., 1024.)
            .tip("Rays per pixel: more is smoother and slower"),
    ];
    rows.push(match o.map {
        BakeMap::Curvature => {
            Row::number("radius", "Radius", Unit::Percent, o.radius as f64 * 100.)
                .range(0.001, 100.)
                .tip("How far around each point the surface is read, as a share of the mesh's size")
        }
        BakeMap::Thickness => Row::number(
            "distance",
            "Max Distance",
            Unit::Percent,
            o.distance as f64 * 100.,
        )
        .range(0.01, 1000.)
        .tip("The longest distance through the mesh measured, as a share of its size"),
    });
    rows.into_iter().map(form::Row::fresh).collect()
}

fn pad_rows(maps: Vec<String>) -> Vec<form::Row> {
    use form::{Row, Unit};
    vec![
        Row::text("uv_map", "UV Map", "Texture")
            .suggestions(maps)
            .tip("The UV layout the texture is painted on"),
        Row::number("padding", "Padding", Unit::Suffix(" px"), 16.)
            .range(1., 256.)
            .tip("Pixels of colour grown out from each UV island"),
    ]
    .into_iter()
    .map(form::Row::fresh)
    .collect()
}

/// A command-line word, quoted when it has spaces.
fn word(s: &str) -> String {
    if s.contains(char::is_whitespace) {
        format!("\"{s}\"")
    } else {
        s.to_owned()
    }
}

impl Operandi {
    fn bake_dialog_free(&mut self, title: &str) -> bool {
        if self.bake_dialog.is_some() {
            return false;
        }
        if self.has_modal() {
            self.status = format!("Close the open dialog before {title}");
            return false;
        }
        if self.editor.scene.active().is_none_or(|i| !i.is_mesh()) {
            self.status = format!("{title}: select a mesh item first");
            return false;
        }
        true
    }

    /// `operandi.uv.atlas` with no arguments: the dialog.
    pub(super) fn open_atlas_unwrap(&mut self) {
        if !self.bake_dialog_free("Atlas Unwrap") {
            return;
        }
        let rows = atlas_rows(&self.editor.uv_atlas_preset, uv_maps(self));
        self.bake_dialog = Some(BakeDialog::Atlas(self.form_dialog("Atlas Unwrap", rows)));
    }

    /// `operandi.bake.curvature` / `.thickness` with no arguments.
    pub(super) fn open_bake(&mut self, map: BakeMap) {
        let title = format!("Bake {}", map.label());
        if !self.bake_dialog_free(&title) {
            return;
        }
        let rows = bake_rows(&BakeOptions::new(map), uv_maps(self));
        self.bake_dialog = Some(BakeDialog::Bake(self.form_dialog(&title, rows), map));
    }

    /// `operandi.bake.padImage` with no arguments.
    pub(super) fn open_pad_texture(&mut self) {
        if !self.bake_dialog_free("Pad Texture to UVs") {
            return;
        }
        let rows = pad_rows(uv_maps(self));
        self.bake_dialog = Some(BakeDialog::Pad(
            self.form_dialog("Pad Texture to UVs", rows),
        ));
    }

    /// Where bakes are saved and textures looked for: the project's
    /// images folder, else the scene's folder, else home.
    pub(super) fn texture_folder(&self) -> std::path::PathBuf {
        if let Some(f) = self.project_folder(crate::project_files::SOURCE_IMAGES) {
            return f;
        }
        self.path
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .or_else(|| std::env::var_os("HOME").map(Into::into))
            .unwrap_or_default()
    }

    pub(super) fn bake_dialogs(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.bake_dialog.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "Dialog closed; nothing changed".into();
            return;
        }
        let (form, title) = match &mut dialog {
            BakeDialog::Atlas(f) => (f, "Atlas Unwrap".to_owned()),
            BakeDialog::Bake(f, map) => (f, format!("Bake {}", map.label())),
            BakeDialog::Pad(f) => (f, "Pad Texture to UVs".to_owned()),
        };
        match self.show_form(ctx, form) {
            form::Outcome::Cancel => self.status = format!("{title} canceled; nothing changed"),
            form::Outcome::Ok => {
                let v = form.values.clone();
                let map_word = |v: &form::Values| {
                    let m = v.text("uv_map").trim().to_owned();
                    if m.is_empty() {
                        String::new()
                    } else {
                        format!(" uvMap:{}", word(&m))
                    }
                };
                match &dialog {
                    BakeDialog::Atlas(_) => {
                        let name = v.text("uv_map").trim().to_owned();
                        let line = format!(
                            "operandi.uv.atlas{} angle:{} gaps:{} resolution:{}",
                            if name.is_empty() {
                                String::new()
                            } else {
                                format!(" name:{}", word(&name))
                            },
                            v.number("angle").to_radians(),
                            v.number("gaps"),
                            v.number("resolution").round()
                        );
                        self.run_command_line(&line);
                    }
                    BakeDialog::Bake(_, map) => {
                        let size = SIZES[v.choice("size").min(SIZES.len() - 1)];
                        let extra = match map {
                            BakeMap::Curvature => format!(" radius:{}", v.number("radius") / 100.),
                            BakeMap::Thickness => {
                                format!(" distance:{}", v.number("distance") / 100.)
                            }
                        };
                        let line = format!(
                            "{} size:{size} padding:{} fill:{} rays:{}{}{extra}",
                            map.command(),
                            v.number("padding").round(),
                            v.bool("fill"),
                            v.number("rays").round(),
                            map_word(&v),
                        );
                        let item = self
                            .editor
                            .scene
                            .active()
                            .map(|i| i.name.clone())
                            .unwrap_or_else(|| "Mesh".into());
                        let name = format!("{item}_{}.png", map.label().to_ascii_lowercase());
                        self.bake_pending = Some(line);
                        let request = file_chooser::Choose::SaveImage {
                            folder: self.texture_folder(),
                            name,
                            title: title.clone(),
                        };
                        if !self.choose(request, file_chooser::Purpose::BakeTo) {
                            self.bake_pending = None;
                            self.status = format!(
                                "{title}: no file panel here; type {} filename:<file.png>",
                                map.command()
                            );
                        }
                    }
                    BakeDialog::Pad(_) => {
                        self.bake_pending = Some(format!(
                            "operandi.bake.padImage padding:{}{}",
                            v.number("padding").round(),
                            map_word(&v)
                        ));
                        let request = file_chooser::Choose::LoadTexture {
                            folder: self.texture_folder(),
                        };
                        if !self.choose(request, file_chooser::Purpose::PadTexture) {
                            self.bake_pending = None;
                            self.status = "Pad Texture to UVs: no file panel here; type operandi.bake.padImage filename:<file.png>".into();
                        }
                    }
                }
            }
            _ => self.bake_dialog = Some(dialog),
        }
    }

    /// The save panel's answer for a bake: run its line with the file.
    pub(super) fn bake_to(&mut self, path: &std::path::Path) {
        let Some(line) = self.bake_pending.take() else {
            return;
        };
        let path = if path.extension().is_none() {
            path.with_extension("png")
        } else {
            path.to_path_buf()
        };
        self.execute(&format!(
            "{line} filename:{}",
            word(&path.display().to_string())
        ));
    }

    /// The open panel's answer for Pad Texture to UVs: pads a copy beside
    /// the picture (`<name>_padded.png`), never the original.
    pub(super) fn pad_texture(&mut self, path: &std::path::Path) {
        let Some(line) = self.bake_pending.take() else {
            return;
        };
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "texture".into());
        let output = path.with_file_name(format!("{stem}_padded.png"));
        self.execute(&format!(
            "{line} filename:{} output:{}",
            word(&path.display().to_string()),
            word(&output.display().to_string())
        ));
    }
}

#[cfg(test)]
#[path = "bake_dialogs_tests.rs"]
mod tests;
