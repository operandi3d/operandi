//! Importers added by kits, and the helper-program route that runs them
//! (kits plan K1, K8; KIT-1; docs/KITS.md "Helper protocol 1").
//!
//! A kit importer is a separate program. For `File > Import` of a file
//! type a kit adds, Operandi runs
//!
//! ```text
//! <helper> import <input file> <output file>
//! ```
//!
//! in the kit's folder, with `OPERANDI_KIT_PROTOCOL=1` and
//! `OPERANDI_KIT_DIR` set. The helper writes the scene to the output file
//! in the format its manifest names (glTF, PLY, FBX or USD), prints any
//! notes for the user as `note: ...` lines, and exits 0. Operandi then
//! reads that file with its own reader. A helper that crashes, exits
//! non-zero, writes nothing or runs past its time limit fails the import
//! with its last error lines; Operandi itself is untouched.
//!
//! The table of importers is replaced as a whole whenever kits are
//! loaded, turned on or off ([`load`], [`set_kit_importers`]).
use crate::Result;
use crate::interchange::Imported;
use crate::kit_folder::{FoundKit, KitState};
use crate::kit_manifest::this_platform;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// The helper protocol this build speaks (docs/KITS.md).
pub const HELPER_PROTOCOL: u32 = 1;

/// A file type a turned-on kit imports.
#[derive(Clone, Debug, PartialEq)]
pub struct KitImporter {
    pub kit_id: String,
    pub kit_name: String,
    pub kit_dir: PathBuf,
    /// The Import dialog's name for it.
    pub label: String,
    pub extensions: Vec<String>,
    /// The extension of the file the helper writes.
    pub output: String,
    /// The helper program, inside `kit_dir`.
    pub helper: PathBuf,
    pub timeout: Duration,
}

static IMPORTERS: RwLock<Vec<KitImporter>> = RwLock::new(Vec::new());

/// Replace the table of kit importers.
pub fn set_kit_importers(importers: Vec<KitImporter>) {
    // Interchange asks the table through these hooks (it doesn't use kits).
    crate::interchange::set_added_importers(crate::interchange::AddedImporters {
        extensions: added_extensions,
        import: import_added,
    });
    *IMPORTERS.write().unwrap_or_else(|e| e.into_inner()) = importers;
}

/// Every extension in the table, each with its importer's label.
fn added_extensions() -> Vec<(String, String)> {
    let mut list = Vec::new();
    for importer in kit_importers() {
        for ext in &importer.extensions {
            list.push((ext.clone(), importer.label.clone()));
        }
    }
    list
}

/// Import `path` with the kit importer for `ext`, if there is one.
fn import_added(ext: &str, path: &Path) -> Option<Result<Imported>> {
    kit_importer_for(ext).map(|importer| run(&importer, path))
}

/// The kit importers now in the table.
pub fn kit_importers() -> Vec<KitImporter> {
    IMPORTERS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The kit importer for a lower-case extension, if a kit that's on reads it.
pub fn kit_importer_for(ext: &str) -> Option<KitImporter> {
    IMPORTERS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find(|i| i.extensions.iter().any(|e| e == ext))
        .cloned()
}

/// What loading the Kits folder found.
#[derive(Debug, Default)]
pub struct LoadReport {
    pub importers: Vec<KitImporter>,
    /// One line per kit that's on but couldn't load, or part left out.
    pub problems: Vec<String>,
}

/// The importers of every kit in `kits` that `state` has on. Kits that are
/// off add nothing and run nothing. Doesn't touch the table: pass the
/// result to [`set_kit_importers`].
pub fn load(kits: &[FoundKit], state: &KitState) -> LoadReport {
    let platform = this_platform();
    let mut report = LoadReport::default();
    for found in kits {
        let id = found.id();
        if !state.enabled.contains(&id) {
            continue;
        }
        let manifest = match &found.manifest {
            Ok(m) => m,
            Err(e) => {
                report.problems.push(format!("Kit {id}: {e}"));
                continue;
            }
        };
        for decl in &manifest.importers {
            let Some(rel) = decl.helper_for(&platform) else {
                report.problems.push(format!(
                    "Kit {id}: \"{}\" has no helper for {platform}",
                    decl.label
                ));
                continue;
            };
            let helper = found.dir.join(rel);
            if !helper.is_file() {
                report
                    .problems
                    .push(format!("Kit {id}: helper {rel} is missing"));
                continue;
            }
            // Two kits can't both own a file type: the first (by folder
            // name) keeps it.
            let taken: Vec<&String> = decl
                .extensions
                .iter()
                .filter(|e| report.importers.iter().any(|i| i.extensions.contains(e)))
                .collect();
            if !taken.is_empty() {
                report.problems.push(format!(
                    "Kit {id}: .{} is already imported by another kit",
                    taken[0]
                ));
                continue;
            }
            report.importers.push(KitImporter {
                kit_id: id.clone(),
                kit_name: manifest.kit.name.clone(),
                kit_dir: found.dir.clone(),
                label: decl.label.clone(),
                extensions: decl.extensions.clone(),
                output: decl.output.clone(),
                helper,
                timeout: Duration::from_secs(decl.timeout_seconds),
            });
        }
    }
    report
}

/// Most error lines of a failed helper shown to the user.
const ERROR_LINES: usize = 6;

/// Import `input` with a kit's helper (helper protocol 1). Nothing is
/// published on error; the helper's scratch folder is always removed.
pub fn run(importer: &KitImporter, input: &Path) -> Result<Imported> {
    let scratch = scratch_dir()?;
    let result = run_in(importer, input, &scratch);
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

fn run_in(importer: &KitImporter, input: &Path, scratch: &Path) -> Result<Imported> {
    let who = &importer.kit_name;
    // Named after the input, so imported items get the file's name.
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Import".into());
    let output = scratch.join(format!("{stem}.{}", importer.output));
    let out_log = scratch.join("helper-stdout.txt");
    let err_log = scratch.join("helper-stderr.txt");
    let file = |p: &Path| std::fs::File::create(p).map_err(|e| e.to_string());
    let input = std::path::absolute(input).map_err(|e| e.to_string())?;
    let mut child = Command::new(&importer.helper)
        .arg("import")
        .arg(&input)
        .arg(&output)
        .current_dir(&importer.kit_dir)
        .env("OPERANDI_KIT_PROTOCOL", HELPER_PROTOCOL.to_string())
        .env("OPERANDI_KIT_DIR", &importer.kit_dir)
        .stdin(Stdio::null())
        .stdout(file(&out_log)?)
        .stderr(file(&err_log)?)
        .spawn()
        .map_err(|e| format!("{who}: can't start its helper program: {e}"))?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break status,
            None if started.elapsed() >= importer.timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{who}: the import took longer than {} s and was stopped",
                    importer.timeout.as_secs()
                ));
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    let stdout = std::fs::read_to_string(&out_log).unwrap_or_default();
    if !status.success() {
        let stderr = std::fs::read_to_string(&err_log).unwrap_or_default();
        let lines: Vec<&str> = stderr.lines().filter(|l| !l.trim().is_empty()).collect();
        let tail = lines[lines.len().saturating_sub(ERROR_LINES)..].join("\n");
        let code = status
            .code()
            .map_or_else(|| "it crashed".to_owned(), |c| format!("exit code {c}"));
        return Err(if tail.is_empty() {
            format!("{who}: the helper failed ({code})")
        } else {
            format!("{who}: the helper failed ({code}):\n{tail}")
        });
    }
    if !output.is_file() {
        return Err(format!("{who}: the helper finished but wrote no file"));
    }
    let mut imported = crate::interchange::import(&output)?;
    let mut notes: Vec<String> = stdout
        .lines()
        .filter_map(|l| l.strip_prefix("note:"))
        .map(|n| n.trim().to_owned())
        .filter(|n| !n.is_empty())
        .collect();
    notes.append(&mut imported.notes);
    imported.notes = notes;
    Ok(imported)
}

/// A new, empty folder for one helper run.
fn scratch_dir() -> Result<PathBuf> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "operandi-kit-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Can't make {}: {e}", dir.display()))?;
    Ok(dir)
}
