//! scene.* File menu commands (UIC-164/165): New, Save, Save As, Export,
//! Close, Close All and Import run these lines, so a script does what the
//! menu does. scene.open with a path stays in `execute` (its older phrase "open <path>").
use super::*;
use operandi_core::command_args;
use operandi_core::interchange::ExportFormat;

/// scene.saveAs `format:` values, in `command_registry::EXPORT_FORMATS`
/// order (core's table, shared with the headless runner).
pub(super) const EXPORT_FORMATS: [ExportFormat; 17] =
    operandi_core::interchange::SCRIPT_EXPORT_FORMATS;

/// The export format a file name's extension names (.stl and .ply mean
/// their binary forms).
fn format_of_path(path: &str) -> Option<ExportFormat> {
    operandi_core::interchange::export_format_of_path(path)
}

/// The scene.saveAs line the Export menu entry runs.
#[cfg(test)] // File > Export's lines are in its form (menu.file.export)
pub(super) fn export_line(format: ExportFormat) -> String {
    let index = EXPORT_FORMATS
        .iter()
        .position(|f| *f == format)
        .unwrap_or(0);
    let operandi_core::command_registry::ArgType::Choice(names) =
        operandi_core::command_registry::EXPORT_FORMATS
    else {
        unreachable!("EXPORT_FORMATS is a choice")
    };
    format!("scene.saveAs format:{} export:true", names[index])
}

impl Operandi {
    pub(super) fn file_canonical(&mut self, text: &str) -> bool {
        let Ok(args) = command::tokens(text) else {
            return false;
        };
        let Some(first) = args.first().map(|a| a.to_ascii_lowercase()) else {
            return false;
        };
        if (first == "scene.open" || first == "open") && args.len() == 1 {
            self.request_file(FileAction::Open);
            return true;
        }
        let name = match first.as_str() {
            "scene.new" => "scene.new",
            "scene.save" => "scene.save",
            "scene.saveas" => "scene.saveAs",
            "scene.close" => "scene.close",
            "scene.closeall" => "scene.closeAll",
            "scene.importreference" => "scene.importReference",
            _ => return false,
        };
        if let Err(e) = self.file_command(name, &args[1..]) {
            self.status = e;
        }
        true
    }

    fn file_command(&mut self, name: &str, args: &[String]) -> std::result::Result<(), String> {
        match name {
            "scene.new" => {
                command_args::bind(name, args, &[])?;
                self.request_file(FileAction::New);
            }
            "scene.save" => {
                command_args::bind(name, args, &[])?;
                self.save_current();
            }
            "scene.close" => {
                command_args::bind(name, args, &[])?;
                self.request_file(FileAction::CloseScene);
            }
            "scene.closeAll" => {
                command_args::bind(name, args, &[])?;
                self.request_close_all();
            }
            // PG-07 (registry ruling 2026-11-12): into the current scene.
            "scene.importReference" => {
                let bound = command_args::bind(
                    name,
                    args,
                    &[
                        "filename",
                        "meshes",
                        "shaders",
                        "lights",
                        "environments",
                        "cameras",
                    ],
                )?;
                let flag = |i: usize, key: &str| match bound[i] {
                    Some(v) => command_args::boolean(key, v),
                    None => Ok(true),
                };
                let kinds = crate::import_into::ImportKinds {
                    meshes: flag(1, "meshes")?,
                    shaders: flag(2, "shaders")?,
                    lights: flag(3, "lights")?,
                    environments: flag(4, "environments")?,
                    cameras: flag(5, "cameras")?,
                };
                match bound[0] {
                    Some(path) => {
                        self.import_into_current(&PathBuf::from(path), kinds, None);
                    }
                    None => self.request_file(FileAction::Import),
                }
            }
            "scene.saveAs" => {
                let bound = command_args::bind(name, args, &["filename", "format", "export"])?;
                let export = match bound[2] {
                    Some(v) => command_args::boolean("export", v)?,
                    None => false,
                };
                let format = match bound[1] {
                    Some(v) => {
                        let operandi_core::command_registry::ArgType::Choice(names) =
                            operandi_core::command_registry::EXPORT_FORMATS
                        else {
                            unreachable!("EXPORT_FORMATS is a choice")
                        };
                        Some(EXPORT_FORMATS[command_args::choice("format", v, names)?])
                    }
                    None => None,
                };
                // A foreign format (named, or said by the file name's
                // extension) exports: the scene keeps its own name and
                // dirty state. Both given and disagreeing is an error.
                let from_path = bound[0].and_then(format_of_path);
                let format = match (format, from_path) {
                    (Some(named), Some(said)) if named.extension() != said.extension() => {
                        return Err(format!(
                            "scene.saveAs: format:{} doesn't match the file name's .{}",
                            bound[1].unwrap_or_default(),
                            said.extension()
                        ));
                    }
                    (Some(named), _) => Some(named),
                    (None, said) => said,
                };
                match (export, format, bound[0]) {
                    (true, None, _) => {
                        return Err("scene.saveAs: export:true needs a format:".into());
                    }
                    (_, Some(format), Some(path)) => {
                        self.export_to(PathBuf::from(path), format);
                    }
                    (_, Some(format), None) => self.request_file(FileAction::Export(format)),
                    (false, None, Some(path)) => {
                        self.save(PathBuf::from(path));
                    }
                    (false, None, None) => self.request_file(FileAction::SaveAs),
                }
            }
            _ => unreachable!("file_canonical names only scene.* commands"),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("operandi-file-commands-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn scene_new_and_close_all_do_what_the_menu_did() {
        let mut a = Operandi::empty();
        a.execute("scene.new");
        assert_eq!(a.scenes.tabs.len(), 2);
        assert_eq!(a.scenes.active, 1);
        a.execute("scene.closeAll");
        assert!(a.scenes.confirm_close_all);
    }

    #[test]
    fn scene_save_as_with_a_filename_saves_and_scene_save_reuses_it() {
        let mut a = Operandi::empty();
        let path = temp("saved.opscene");
        let _ = std::fs::remove_file(&path);
        a.execute(&format!("scene.saveAs filename:{{{}}}", path.display()));
        assert!(path.exists(), "{}", a.status);
        assert_eq!(a.path.as_deref(), Some(path.as_path()));
        std::fs::remove_file(&path).unwrap();
        a.execute("scene.save");
        assert!(path.exists(), "scene.save writes the scene's own file");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn scene_save_as_export_writes_the_interchange_file() {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        a.editor.scene.items[0].mesh =
            operandi_core::Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 1)
                .unwrap()
                .into();
        let path = temp("cube.obj");
        let _ = std::fs::remove_file(&path);
        a.execute(&format!(
            "scene.saveAs filename:{{{}}} format:obj export:true",
            path.display()
        ));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{}", a.status));
        assert!(text.lines().any(|l| l.starts_with("v ")));
        assert_eq!(a.path, None, "exporting doesn't rename the scene");
        std::fs::remove_file(&path).unwrap();
    }

    /// Registry (2026-11-11): the extension names the format, no
    /// export:true needed, and the scene keeps its own name.
    #[cfg(feature = "usd")]
    #[test]
    fn scene_save_as_a_usd_file_name_exports_usd() {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        a.editor.scene.items[0].mesh =
            operandi_core::Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 1)
                .unwrap()
                .into();
        let path = temp("cube.usda");
        let _ = std::fs::remove_file(&path);
        a.execute(&format!("scene.saveAs filename:{{{}}}", path.display()));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{}", a.status));
        assert!(text.starts_with("#usda 1.0"), "{text}");
        assert!(text.contains("def Mesh"), "{text}");
        assert_eq!(a.path, None, "exporting doesn't rename the scene");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn scene_save_as_refuses_bad_format_lines() {
        let mut a = Operandi::empty();
        a.execute("scene.saveAs export:true");
        assert!(a.status.contains("needs a format"), "{}", a.status);
        a.execute("scene.saveAs filename:{/tmp/x.ply} format:abc");
        assert!(a.status.contains("doesn't match"), "{}", a.status);
        a.execute("scene.saveAs format:dwg");
        assert!(a.status.contains("valid choices are obj"), "{}", a.status);
        assert!(a.status.contains("abc"), "{}", a.status);
        a.execute("scene.new extra");
        assert!(a.file_dialog.is_none());
        assert_eq!(a.scenes.tabs.len(), 1, "a bad line runs nothing");
    }

    #[test]
    fn scene_save_as_a_foreign_file_name_exports_without_renaming() {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        a.editor.scene.items[0].mesh =
            operandi_core::Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 1)
                .unwrap()
                .into();
        for (name, format) in [
            ("cube.ply", ""),
            ("cube-ascii.ply", " format:plyAscii"),
            ("cube.abc", ""),
            ("cube2.abc", " format:abc"),
        ] {
            let path = temp(name);
            let _ = std::fs::remove_file(&path);
            a.execute(&format!(
                "scene.saveAs filename:{{{}}}{format}",
                path.display()
            ));
            assert!(path.exists(), "{name}: {}", a.status);
            let back = operandi_core::interchange::import(&path).unwrap();
            assert_eq!(back.scene.items[0].mesh.faces.len(), 6, "{name}");
            std::fs::remove_file(&path).unwrap();
        }
        assert_eq!(a.path, None, "exporting doesn't rename the scene");
    }

    #[test]
    fn every_export_format_has_a_line() {
        for format in ExportFormat::ALL {
            let line = export_line(format);
            let (entry, _) = operandi_core::command_registry::find_line(&line).unwrap();
            assert_eq!(entry.name, "scene.saveAs");
            assert!(EXPORT_FORMATS.contains(&format));
        }
    }
}
