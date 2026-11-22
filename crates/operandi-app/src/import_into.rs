//! `scene.importReference` (PG-07, registry ruling 2026-11-12): a scene or
//! model file's items join the current scene, selected, one undo step. The
//! format is picked by the file's extension: Operandi scenes, Modo scenes
//! (the LXO reader) and the interchange formats. The kind filters
//! (`meshes`, `shaders`, `lights`, `environments`, `cameras`, all on by
//! default) leave out what's switched off. Opening a file in its own scene
//! tab is `scene.open`'s job.
use super::*;
use operandi_core::preset::{Preset, PresetHeader, PresetKind};
use std::path::Path;

/// What `scene.importReference` brings in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ImportKinds {
    pub meshes: bool,
    pub shaders: bool,
    pub lights: bool,
    pub environments: bool,
    pub cameras: bool,
}

impl Default for ImportKinds {
    fn default() -> Self {
        Self {
            meshes: true,
            shaders: true,
            lights: true,
            environments: true,
            cameras: true,
        }
    }
}

/// Leave out of `scene` the kinds `kinds` switches off. An item whose
/// parent goes moves up to that parent's parent.
pub(crate) fn filter_import(scene: &mut Scene, kinds: ImportKinds) {
    let keep = |i: &operandi_core::Item| {
        if i.is_texture_item() {
            return true;
        }
        if i.light.is_some() {
            kinds.lights
        } else if i.environment.is_some() {
            kinds.environments
        } else if i.camera.is_some() {
            kinds.cameras
        } else if i.is_mesh() {
            kinds.meshes
        } else {
            true
        }
    };
    let gone: std::collections::HashMap<u64, Option<u64>> = scene
        .items
        .iter()
        .filter(|i| !keep(i))
        .map(|i| (i.id, i.parent))
        .collect();
    scene.items.retain(|i| !gone.contains_key(&i.id));
    for item in &mut scene.items {
        while let Some(p) = item.parent
            && let Some(&up) = gone.get(&p)
        {
            item.parent = up;
        }
    }
    if !kinds.shaders {
        scene.shader_tree = Default::default();
        scene.materials.clear();
    }
}

/// Read a scene or model file into a scene of its own (nothing is
/// published), with notes on what didn't come across. The format is picked
/// by the extension (PG-07); thumbnails use it too (PG-05).
pub(crate) fn read_scene_file(path: &Path) -> std::result::Result<(Scene, Vec<String>), String> {
    {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if ext == "lxo" {
            let mut go = |_: &str| true;
            let mut control = operandi_core::lxo::Control::new(&mut go);
            let candidate =
                operandi_core::lxo::load(path, &mut control).map_err(|e| e.to_string())?;
            let notes = if candidate.report.rejected_groups.is_empty() {
                Vec::new()
            } else {
                vec![candidate.report.summary()]
            };
            return Ok((candidate.scene, notes));
        }
        if operandi_core::interchange::is_importable(path) {
            let imported = operandi_core::interchange::import(path)?;
            return Ok((imported.scene, imported.notes));
        }
        if ext == "opscene" {
            return persistence::load(path)
                .map(|s| (s, Vec::new()))
                .map_err(|e| e.to_string());
        }
        Err(format!(
            "{} isn't a scene or model file Operandi reads",
            crate::preset_browser::folder_name(path)
        ))
    }
}

impl Operandi {
    /// PG-07: add `path`'s items to the current scene, selected, as one
    /// undo step. Returns whether anything was added.
    /// `rest_at` (PG-06's Shift-drop) puts the bottom middle of what
    /// arrives on that world point.
    pub(super) fn import_into_current(
        &mut self,
        path: &Path,
        kinds: ImportKinds,
        rest_at: Option<glam::DVec3>,
    ) -> bool {
        let name = crate::preset_browser::folder_name(path);
        let (mut scene, notes) = match read_scene_file(path) {
            Ok(r) => r,
            Err(e) => {
                self.status = format!("Import {name}: {e}");
                return false;
            }
        };
        filter_import(&mut scene, kinds);
        if let Some(point) = rest_at {
            operandi_core::preset::placement::rest_on(&mut scene, point);
        }
        if scene.items.iter().all(|i| i.is_texture_item()) {
            self.status = format!("Import {name}: nothing to import");
            return false;
        }
        if !self.confirm_fields() {
            return false;
        }
        let ok = self.run_ok(Command::LoadPreset {
            preset: Box::new(Preset {
                header: PresetHeader {
                    kind: PresetKind::Assembly,
                    thumbnail: None,
                    source: None,
                },
                scene,
            }),
            group: None,
        });
        if ok {
            self.status = std::iter::once(format!("Imported {name} into this scene"))
                .chain(notes)
                .collect::<Vec<_>>()
                .join(" · ");
        }
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("operandi-import-into-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    /// PG-07: a scene's items join the current scene, selected, in one
    /// undo step; the kind filters leave things out.
    #[test]
    fn pg_07_a_scene_imports_into_the_current_one_in_one_step() {
        let mut a = crate::tests::box_app();
        let other = a.editor.scene.clone();
        let file = temp("other.opscene");
        persistence::save(&file, &other).unwrap();
        let before = a.editor.scene.clone();
        let count = before.items.len();
        let undo = a.editor.undo.len();
        a.execute(&format!(
            "scene.importReference filename:\"{}\"",
            file.display()
        ));
        assert_eq!(a.editor.undo.len(), undo + 1, "{}", a.status);
        assert_eq!(a.editor.scene.items.len(), count + other.items.len());
        assert!(
            a.status.starts_with("Imported other.opscene"),
            "{}",
            a.status
        );
        let selected = a.editor.scene.selection.item.unwrap();
        assert!(before.items.iter().all(|i| i.id != selected));
        a.run(Command::Undo);
        assert_eq!(a.editor.scene.items.len(), count);
        // Only meshes.
        a.execute(&format!(
            "scene.importReference filename:\"{}\" lights:false cameras:false environments:false",
            file.display()
        ));
        let meshes = other
            .items
            .iter()
            .filter(|i| i.is_mesh() && !i.is_texture_item())
            .count();
        let added: Vec<_> = a.editor.scene.items[count..].iter().collect();
        assert_eq!(added.len(), meshes);
        assert!(
            added
                .iter()
                .all(|i| i.light.is_none() && i.camera.is_none())
        );
        // A model file goes through the same command; a bad value refuses.
        let obj = temp("tri.obj");
        std::fs::write(&obj, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        let n = a.editor.scene.items.len();
        a.execute(&format!(
            "scene.importReference filename:\"{}\"",
            obj.display()
        ));
        assert_eq!(a.editor.scene.items.len(), n + 1, "{}", a.status);
        a.execute(&format!(
            "scene.importReference filename:\"{}\" meshes:maybe",
            obj.display()
        ));
        assert_eq!(a.editor.scene.items.len(), n + 1);
        std::fs::remove_dir_all(file.parent().unwrap()).ok();
    }
}
