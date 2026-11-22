//! The USD import options dialog (docs/BEHAVIOR-USD-IMPORT.md). Opening
//! or importing a .usd, .usda, .usdc or .usdz file (File > Open, File >
//! Import, a drop on the window) asks these first; OK imports with them
//! as an unsaved scene and shows the import report, Cancel leaves the
//! document alone. Scripts and the MCP bridge (`scene.importReference`,
//! `scene_import`) use the defaults and never open it.
//!
//! Built with the shared form dialog (UIC-80..99): title is the command
//! without "…" (UIC-81), short Title Case labels (UIC-86), every row has a
//! tooltip (UIC-82, UIC-119), values come back as last OK'd (UIC-95), and
//! rows that can't work yet stay in place greyed with their reason
//! (UIC-92, UIC-117) rather than disappearing.
use super::*;
use operandi_core::interchange::{self, UsdImportOptions, UsdMaterials};

pub(super) struct UsdImportDialog {
    path: PathBuf,
    form: form::Dialog,
}

/// The dialog's title (UIC-81).
pub(super) const TITLE: &str = "Import USD";

const UP_AXES: [&str; 3] = ["From File", "Y", "Z"];
/// Unit labels and metres per unit (`None`: the file's metersPerUnit).
const UNITS: [(&str, Option<f64>); 6] = [
    ("From File", None),
    ("Meters", Some(1.0)),
    ("Centimeters", Some(0.01)),
    ("Millimeters", Some(0.001)),
    ("Inches", Some(0.0254)),
    ("Feet", Some(0.3048)),
];
const MATERIALS: [&str; 3] = ["MaterialX", "Preview Surface", "None"];
const INSTANCES: [&str; 2] = ["Instances", "Copies"];

/// The command the dialog's OK runs (a placeholder name, docs/BEHAVIOR-USD-IMPORT.md).
pub(super) const COMMAND: &str = "operandi.usd.import";
const HELP: &str = "Help: operandi.usd.import [filename:<file.usd*>] [upAxis:file|y|z] [units:file|m|cm|mm|in|ft] [materials:materialx|preview|none] [cameras:true|false] [lights:true|false] [graphs:true|false] [graphsFile:<file.json>] [instances:instances|copies] opens a USD file as a new unsaved scene with those import options (defaults: the file's up axis and units, MaterialX first) and shows the import report. Without a filename it opens the file panel.";
/// Argument values, in the dialog's choice order.
const UP_ARGS: [&str; 3] = ["file", "y", "z"];
const UNIT_ARGS: [&str; 6] = ["file", "m", "cm", "mm", "in", "ft"];
const MATERIAL_ARGS: [&str; 3] = ["materialx", "preview", "none"];
const INSTANCE_ARGS: [&str; 2] = ["instances", "copies"];

/// The command line for importing `path` with the dialog's `values`.
pub(super) fn command_line(path: &std::path::Path, values: &form::Values) -> String {
    let at = |list: &[&'static str], i: usize| list.get(i).copied().unwrap_or(list[0]);
    format!(
        "{COMMAND} filename:{} upAxis:{} units:{} materials:{} cameras:{} lights:{} graphs:{} instances:{}{}",
        operandi_core::command_record::quote(&path.to_string_lossy()),
        at(&UP_ARGS, values.choice("upAxis")),
        at(&UNIT_ARGS, values.choice("units")),
        at(&MATERIAL_ARGS, values.choice("materials")),
        values.bool("cameras"),
        values.bool("lights"),
        values.bool("graphs"),
        at(&INSTANCE_ARGS, values.choice("instances")),
        match values.text("graphsFile").trim() {
            "" => String::new(),
            file => format!(" graphsFile:{}", operandi_core::command_record::quote(file)),
        },
    )
}

pub(super) fn rows() -> Vec<form::Row> {
    use form::{Row, When};
    let units: Vec<&str> = UNITS.iter().map(|(label, _)| *label).collect();
    vec![
        Row::choice("upAxis", "Up Axis", &UP_AXES, 0).tip(
            "Which way is up in the file: as the file says, or Y or Z instead. \
             The scene stays Y up, so a Z-up file is turned",
        ),
        Row::choice("units", "Units", &units, 0).tip(
            "The length one file unit stands for: the file's metersPerUnit, \
             or the unit you pick",
        ),
        Row::choice("materials", "Import Materials", &MATERIALS, 0).tip(
            "Which shading network each material comes from. MaterialX: its \
             MaterialX network, else its Preview Surface. Preview Surface: \
             the other way round. None: polygons keep their material tags, \
             no Material Groups",
        ),
        Row::check("cameras", "Import Cameras", true)
            .tip("Brings the file's cameras in as camera items, lens and film as written"),
        Row::check("lights", "Import Lights", true).tip(
            "Brings the file's lights in as lights with the power they had, and a dome \
             light's picture as the environment",
        ),
        Row::check("graphs", "Use Graphs Sidecar", true).tip(
            "Blender node graphs written by tools/blender-export replace the \
             materials of the same name with Node Material layers: the Graphs \
             File, else <file name>_graphs.json beside the USD file when there is one",
        ),
        Row::text("graphsFile", "Graphs File", "")
            .when(When::Checked("graphs"))
            .tip(
                "A node graph sidecar to read instead of <file name>_graphs.json \
                 beside the file; empty: that one",
            ),
        Row::segments("instances", "Instances", &INSTANCES, 0).tip(
            "Instances: the file's instances come in as Mesh Instances of the first item \
             showing their mesh (edit one, all change). Copies: as separate items that \
             share the mesh until one is edited",
        ),
    ]
}

/// The import options the dialog's values give. Pictures packed in a
/// .usdz go to `textures` (the project's folder for them) when set.
pub(super) fn options(values: &form::Values, textures: Option<PathBuf>) -> UsdImportOptions {
    UsdImportOptions {
        up_axis: match values.choice("upAxis") {
            1 => Some('Y'),
            2 => Some('Z'),
            _ => None,
        },
        meters_per_unit: UNITS.get(values.choice("units")).and_then(|(_, m)| *m),
        materials: match values.choice("materials") {
            1 => UsdMaterials::PreviewSurface,
            2 => UsdMaterials::None,
            _ => UsdMaterials::MaterialX,
        },
        cameras: values.bool("cameras"),
        lights: values.bool("lights"),
        graphs_sidecar: values.bool("graphs"),
        graphs_file: Some(values.text("graphsFile").trim())
            .filter(|t| !t.is_empty())
            .map(PathBuf::from),
        instances: values.choice("instances") == 0,
        texture_folder: textures,
    }
}

impl Operandi {
    /// Ask the import options for USD file `path` (its import follows OK).
    /// Callers have already passed unsaved-work protection (Open/Import).
    pub(super) fn ask_usd_import(&mut self, path: PathBuf) {
        if self.has_modal() {
            self.status = "Close the open dialog before importing USD".into();
            return;
        }
        self.usd_import = Some(UsdImportDialog {
            path,
            form: self.form_dialog(TITLE, rows()),
        });
    }

    /// Where a .usdz's pictures are unpacked: the project's source images
    /// folder, in a folder named after the file; `None` (beside the file)
    /// without a project.
    pub(super) fn usd_texture_folder(&self, path: &std::path::Path) -> Option<PathBuf> {
        let stem = path.file_stem()?.to_string_lossy().into_owned();
        self.project_folder(crate::project_files::SOURCE_IMAGES)
            .map(|f| f.join(stem))
    }

    /// `operandi.usd.import` (placeholder name): open a USD file with the
    /// import options given; without a filename, the file panel (whose
    /// USD files ask the dialog).
    pub(super) fn usd_canonical(&mut self, args: &[String]) -> bool {
        let Some(first) = args.first().map(|a| a.to_ascii_lowercase()) else {
            return false;
        };
        if first == "help" && args.len() == 2 && args[1].eq_ignore_ascii_case(COMMAND) {
            self.status = HELP.into();
            return true;
        }
        if first != COMMAND {
            return false;
        }
        let bound = match operandi_core::command_args::bind(
            COMMAND,
            &args[1..],
            &[
                "filename",
                "upAxis",
                "units",
                "materials",
                "cameras",
                "lights",
                "graphs",
                "graphsFile",
                "instances",
            ],
        ) {
            Ok(b) => b,
            Err(e) => {
                self.status = e;
                return true;
            }
        };
        let Some(path) = bound[0].map(PathBuf::from) else {
            self.request_file(FileAction::Open);
            return true;
        };
        let pick = |i: usize, names: &[&str]| -> std::result::Result<usize, String> {
            match bound[i] {
                None => Ok(0),
                Some(v) => names
                    .iter()
                    .position(|n| n.eq_ignore_ascii_case(v))
                    .ok_or_else(|| format!("{COMMAND}: {v} isn't one of {}", names.join(", "))),
            }
        };
        let picks = (|| {
            Ok::<_, String>([
                pick(1, &UP_ARGS)?,
                pick(2, &UNIT_ARGS)?,
                pick(3, &MATERIAL_ARGS)?,
                pick(8, &INSTANCE_ARGS)?,
            ])
        })();
        let [up, units, materials, instances] = match picks {
            Ok(p) => p,
            Err(e) => {
                self.status = e;
                return true;
            }
        };
        let flag = |i: usize, key: &str| match bound[i] {
            Some(v) => operandi_core::command_args::boolean(key, v),
            None => Ok(true),
        };
        let (cameras, lights, graphs) =
            match (flag(4, "cameras"), flag(5, "lights"), flag(6, "graphs")) {
                (Ok(c), Ok(l), Ok(g)) => (c, l, g),
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                    self.status = e;
                    return true;
                }
            };
        if !interchange::is_usd(&path) {
            self.status = format!("{COMMAND} reads .usd, .usda, .usdc and .usdz files");
            return true;
        }
        if !self.confirm_fields() {
            return true;
        }
        let mut values = form::Dialog::with_defaults(TITLE, rows()).values;
        values.set("upAxis", form::Value::Choice(up));
        values.set("units", form::Value::Choice(units));
        values.set("materials", form::Value::Choice(materials));
        values.set("cameras", form::Value::Bool(cameras));
        values.set("lights", form::Value::Bool(lights));
        values.set("graphs", form::Value::Bool(graphs));
        values.set("instances", form::Value::Choice(instances));
        values.set(
            "graphsFile",
            form::Value::Text(bound[7].unwrap_or_default().to_string()),
        );
        let textures = self.usd_texture_folder(&path);
        self.import_interchange_with(path, &options(&values, textures));
        true
    }

    /// One frame of the open USD import dialog.
    pub(super) fn usd_import_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.usd_import.take() else {
            return;
        };
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => self.status = "Import canceled; nothing changed".into(),
            form::Outcome::Ok => {
                // UIC-96: OK runs the import once, as its command line
                // (UIC-220).
                self.remember_form(&dialog.form);
                self.execute(&command_line(&dialog.path, &dialog.form.values));
                ctx.request_repaint();
            }
            _ => self.usd_import = Some(dialog),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Defaults: everything on, units and up axis from the file, MaterialX
    /// first, instances as instances; the Graphs File only with the
    /// sidecar on.
    #[test]
    fn defaults_read_the_file_and_grey_what_isnt_built() {
        let rows = rows();
        let values = form::Dialog::with_defaults(TITLE, rows.clone()).values;
        let o = options(&values, None);
        assert_eq!(o, UsdImportOptions::default());
        let file = rows.iter().find(|r| r.key == "graphsFile").unwrap();
        assert_eq!(file.when, form::When::Checked("graphs"));
        assert!(
            rows.iter()
                .all(|r| !r.tooltip.is_empty() && !r.label.ends_with(':'))
        );
    }

    #[test]
    fn choices_become_options() {
        let mut values = form::Dialog::with_defaults(TITLE, rows()).values;
        values.set("upAxis", form::Value::Choice(2));
        values.set("units", form::Value::Choice(2));
        values.set("materials", form::Value::Choice(1));
        let o = options(&values, Some(PathBuf::from("/p/sourceimages/x")));
        assert_eq!(o.up_axis, Some('Z'));
        assert_eq!(o.meters_per_unit, Some(0.01));
        assert_eq!(o.materials, UsdMaterials::PreviewSurface);
        assert_eq!(o.texture_folder, Some(PathBuf::from("/p/sourceimages/x")));
    }

    /// Opening a .usda asks first; OK imports it as an unsaved scene with
    /// the report, Cancel changes nothing.
    #[test]
    fn opening_a_usd_file_asks_then_imports() {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        let before = a.editor.scene.clone();
        let dir = std::env::temp_dir().join(format!("operandi-usd-dialog-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tri.usda");
        std::fs::write(
            &path,
            "#usda 1.0\ndef Mesh \"Tri\"\n{\n int[] faceVertexCounts = [3]\n \
             int[] faceVertexIndices = [0, 1, 2]\n point3f[] points = [(0, 0, 0), (1, 0, 0), (0, 1, 0)]\n}\n",
        )
        .unwrap();
        a.open(path.clone());
        assert!(a.usd_import.is_some(), "the dialog opens first");
        assert_eq!(a.editor.scene, before, "nothing changes before OK");
        // OK runs the dialog's command line (UIC-220).
        let dialog = a.usd_import.take().unwrap();
        let line = command_line(&dialog.path, &dialog.form.values);
        assert!(
            line.starts_with("operandi.usd.import filename:")
                && line.ends_with(
                    "upAxis:file units:file materials:materialx cameras:true lights:true graphs:true instances:instances"
                ),
            "{line}"
        );
        a.execute(&line);
        // Without the USD library (plain test builds) the import is
        // refused with a message and the document stays; with it, the
        // triangle arrives as an unsaved scene.
        if operandi_usd::available() {
            assert!(a.status.starts_with("Imported tri.usda"), "{}", a.status);
            assert!(a.editor.scene.items.iter().any(|i| i.name == "Tri"));
        } else {
            assert_eq!(a.editor.scene, before);
            assert!(a.import_message.is_some());
        }
    }

    #[test]
    fn the_command_checks_its_arguments() {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        a.execute("operandi.usd.import filename:/x/a.obj");
        assert_eq!(
            a.status,
            "operandi.usd.import reads .usd, .usda, .usdc and .usdz files"
        );
        a.execute("operandi.usd.import filename:/x/a.usda units:parsec");
        assert!(a.status.contains("parsec isn't one of"), "{}", a.status);
        a.execute("help operandi.usd.import");
        assert!(
            a.status.starts_with("Help: operandi.usd.import"),
            "{}",
            a.status
        );
    }
}
