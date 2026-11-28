//! The `operandi.kit.*` commands and loading kits at start-up (kits plan
//! K1; docs/KITS.md). The kit contract itself (manifest, Kits folder,
//! helper programs) lives in core: `kit_manifest`, `kit_folder`,
//! `kit_import`.
//!
//! There is no Kits window yet: kits are listed, installed and turned on
//! with these commands (typed, or from a script). Turning a kit on is the
//! user's explicit "I trust this kit" step, since a kit can run programs.
use super::*;
use operandi_core::kit_folder::{self, FoundKit, KitState};
use operandi_core::kit_import;

impl Operandi {
    /// Read the Kits folder and put the importers of the kits that are on
    /// in the import table. Says so on the status line when one that's on
    /// can't load.
    pub(super) fn load_kits(&mut self) {
        let problems = self.reload_kits();
        if !problems.is_empty() {
            self.status = format!("{} — operandi.kit.list lists the kits", problems.join("; "));
        }
    }

    /// Read the Kits folder again; returns the problems found.
    fn reload_kits(&mut self) -> Vec<String> {
        let paths = self.environment.paths();
        let found = kit_folder::discover(&paths.kits());
        let state = KitState::read(&paths.kit_state());
        let report = kit_import::load(&found, &state);
        kit_import::set_kit_importers(report.importers);
        report.problems
    }

    /// `operandi.kit.list|install|enable|disable|reload|openFolder`.
    pub(super) fn kit_command(&mut self, name: &str, rest: &str) {
        match self.kit_command_result(name, rest) {
            Ok(m) => self.status = m,
            Err(m) => {
                self.status = m.clone();
                self.last_failure = Some(m);
            }
        }
    }

    fn kit_command_result(&mut self, name: &str, rest: &str) -> Result<String> {
        let tokens = command::tokens(rest)?;
        let bind = |names: &[&str]| operandi_core::command_args::bind(name, &tokens, names);
        let kits_dir = self.environment.paths().kits();
        let state_path = self.environment.paths().kit_state();
        match name {
            "operandi.kit.list" => {
                let found = kit_folder::discover(&kits_dir);
                if found.is_empty() {
                    return Ok(format!(
                        "No kits installed (Kits folder: {})",
                        kits_dir.display()
                    ));
                }
                let state = KitState::read(&state_path);
                let lines: Vec<String> = found.iter().map(|k| describe(k, &state)).collect();
                Ok(format!("{} kits: {}", found.len(), lines.join("; ")))
            }
            "operandi.kit.install" => {
                let b = bind(&["path", "replace"])?;
                let path = b[0].ok_or(format!("{name} needs path:<kit .zip or folder>"))?;
                let replace = b[1].is_some_and(|v| v.eq_ignore_ascii_case("true") || v == "1");
                let id = kit_folder::install(std::path::Path::new(path), &kits_dir, replace)?;
                let state = KitState::read(&state_path);
                if state.enabled.contains(&id) {
                    // Replacing a kit that's on: pick up its new parts.
                    let problems = self.reload_kits();
                    return Ok(with_problems(format!("Kit {id} updated"), &problems));
                }
                Ok(format!(
                    "Installed kit {id}, off. Turn it on with operandi.kit.enable id:{id} if you trust it: kits can run programs."
                ))
            }
            "operandi.kit.enable" | "operandi.kit.disable" => {
                let b = bind(&["id"])?;
                let id = b[0].ok_or(format!("{name} needs id:<kit id>"))?.to_owned();
                let on = name == "operandi.kit.enable";
                let found = kit_folder::discover(&kits_dir);
                let kit = found.iter().find(|k| k.id() == id);
                let mut state = KitState::read(&state_path);
                if on {
                    let kit = kit.ok_or(format!("No kit {id} in the Kits folder"))?;
                    if let Err(e) = &kit.manifest {
                        return Err(format!("Kit {id} can't load: {e}"));
                    }
                    state.enabled.insert(id.clone());
                } else if !state.enabled.remove(&id) {
                    return Ok(format!("Kit {id} is already off"));
                }
                state.write(&state_path)?;
                let problems = self.reload_kits();
                let message = if on {
                    let types: Vec<String> = kit_import::kit_importers()
                        .iter()
                        .filter(|i| i.kit_id == id)
                        .flat_map(|i| i.extensions.iter().map(|e| format!(".{e}")))
                        .collect();
                    if types.is_empty() {
                        format!("Kit {id} is on")
                    } else {
                        format!("Kit {id} is on: File > Import reads {}", types.join(", "))
                    }
                } else {
                    format!("Kit {id} is off")
                };
                Ok(with_problems(message, &problems))
            }
            "operandi.kit.reload" => {
                let problems = self.reload_kits();
                let n = kit_import::kit_importers().len();
                Ok(with_problems(
                    format!("Kits read again: {n} file types from kits"),
                    &problems,
                ))
            }
            "operandi.kit.openFolder" => {
                std::fs::create_dir_all(&kits_dir).map_err(|e| e.to_string())?;
                crate::bug_report::reveal(&kits_dir).map_err(|e| e.to_string())?;
                Ok(format!("Kits folder: {}", kits_dir.display()))
            }
            _ => Err(format!("Unknown kit command {name}")),
        }
    }
}

/// One kit for `operandi.kit.list`.
fn describe(kit: &FoundKit, state: &KitState) -> String {
    let id = kit.id();
    match &kit.manifest {
        Err(e) => format!("{id} (can't load: {e})"),
        Ok(m) => {
            let on = if state.enabled.contains(&id) {
                "on"
            } else {
                "off"
            };
            let types: Vec<String> = m
                .importers
                .iter()
                .flat_map(|i| i.extensions.iter().map(|e| format!(".{e}")))
                .collect();
            let adds = if types.is_empty() {
                String::new()
            } else {
                format!(", imports {}", types.join(" "))
            };
            format!("{id} {} ({on}{adds})", m.kit.version)
        }
    }
}

fn with_problems(message: String, problems: &[String]) -> String {
    if problems.is_empty() {
        message
    } else {
        format!("{message}. {}", problems.join("; "))
    }
}
