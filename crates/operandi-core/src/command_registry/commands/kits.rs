//! Kits: list, install, turn on and off (one section of `COMMANDS`, in
//! table order). Operandi-only: the map has no commands for kits.
//! docs/KITS.md says what a kit is.
use super::*;

pub(super) const COMMANDS: &[Entry] = &[
    script("operandi.kit.list", Files, "Lists the kits in the Kits folder: id, version, on or off, and what each adds or why it can't load. Not undoable.")
        .operandi()
        .ui(),
    script("operandi.kit.install", Files, "Installs the kit in a .zip or kit folder (path:) into the Kits folder, left off; replace:true installs over the same kit. Not undoable.")
        .operandi()
        .args(&[arg("path", ArgType::Path, N), opt("replace", BOOL, N, Some("false"))])
        .example("operandi.kit.install path:/nonexistent/off_import-1.0.0.zip")
        .side_effect(),
    script("operandi.kit.enable", Files, "Turns a kit on (id:). A kit can run programs on this computer: only turn on kits you trust. Its file types join File > Import. Not undoable.")
        .operandi()
        .args(&[arg("id", TEXT, N)])
        .example("operandi.kit.enable id:off_import")
        .side_effect(),
    script("operandi.kit.disable", Files, "Turns a kit off (id:): its file types leave File > Import; scenes that used it still open. Not undoable.")
        .operandi()
        .args(&[arg("id", TEXT, N)])
        .example("operandi.kit.disable id:off_import")
        .side_effect(),
    script("operandi.kit.reload", Files, "Reads the Kits folder again, after adding or changing a kit by hand. Not undoable.")
        .operandi()
        .side_effect(),
    script("operandi.kit.openFolder", Files, "Shows the Kits folder in the file browser (making it if needed). Not undoable.")
        .operandi()
        .ui(),
];
