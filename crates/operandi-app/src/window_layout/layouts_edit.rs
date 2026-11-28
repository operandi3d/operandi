//! Layouts, stage 2 (specifications/layouts-2026-11-16,
//! BEHAVIOR-LAYOUTS-STAGE2.md): user layouts, Revert, the full switcher
//! bar, named pieces and the content arrow.
//!
//! Every action is a registered command (UIC-220/221, LAY-30): the Layout
//! menu, the Window menu, the dialogs, the keys and the pie all run the same
//! command lines.
//!
//! - New / Save As / Rename / Delete / Revert / Clear (UIC-199/200,
//!   LAY-42..47). Shipped layouts are read-only files built into the app; a
//!   user layout, or a changed copy of a shipped one, is a file in the data
//!   folder's `Layouts` folder (UIC-194). Only user layouts can be renamed
//!   or deleted; only shipped ones can be reverted (provisional, LAY-Q12).
//! - The quick switcher (UIC-18, LAY-32): Ctrl+Shift+Tab, Layout > Recent
//!   Layouts…, and the Layouts pie on Ctrl+`. Real Ctrl on a Mac.
//! - Pieces (UIC-192, LAY-72): a view-area panel or the whole view area
//!   saved under a name and a category, reused anywhere; saving it again
//!   updates every layout that shows it.
//! - The content arrow (UIC-193, LAY-73): what a view-area slot shows.
//!   With the corner dot it builds a layout from one empty panel (LAY-74).
//!   Stage 2 builds arbitrary arrangements in the view area only; the side
//!   columns stay the three-column shape (provisional, LAY-Q11).
use super::*;
use crate::layouts::dock::{replace_views, view_frame};
use crate::layouts::{Entry, file_name};
use crate::panel_dot::LayoutLocks;
use crate::unavailable::Gate;
use operandi_layout::{EMPTY, Frame, Pane, Slot, UV_VIEW, VIEW_3D, WindowLayout};
use serde::{Deserialize, Serialize};

/// A piece file's version.
const PIECE_VERSION: u32 = 1;

/// A saved piece (LAY-72): one view-area panel, or a whole view area.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub category: String,
    /// A whole view area (Save Viewport Group As), not one panel.
    #[serde(default)]
    pub group: bool,
    pub frame: Frame,
    /// Built into the app (read-only, UIC-194). Stage 2 ships none.
    #[serde(skip)]
    pub shipped: bool,
}

/// The layout dialogs (UIC-200 and friends). Each one's OK runs a command
/// line ([`dialog_line`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Dialog {
    New {
        name: String,
        copy: bool,
    },
    SaveAs {
        name: String,
    },
    Rename {
        layout: String,
        name: String,
        opening: bool,
    },
    Delete {
        layout: String,
    },
    SavePiece {
        name: String,
        category: String,
        group: bool,
    },
}

/// The quick layout switcher (UIC-18).
#[derive(Clone, Debug, PartialEq)]
pub struct Switcher {
    /// Layouts in most-recently-used order.
    pub list: Vec<String>,
    /// The outlined one.
    pub at: usize,
    /// Opened by Ctrl+Shift+Tab: releasing the keys switches. Otherwise a
    /// click (or Return) picks.
    pub held: bool,
}

/// A command line from a command name and `(argument, value)` pairs; a
/// value with spaces or braces-worthy characters is braced.
pub fn line(command: &str, args: &[(&str, &str)]) -> String {
    let mut out = command.to_owned();
    for (name, value) in args {
        out.push(' ');
        out.push_str(name);
        out.push(':');
        if !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            out.push_str(value);
        } else {
            out.push('{');
            out.push_str(value);
            out.push('}');
        }
    }
    out
}

/// The command line a dialog's OK runs.
pub fn dialog_line(dialog: &Dialog) -> String {
    match dialog {
        Dialog::New { name, copy } => line(
            "operandi.layout.create",
            &[
                ("name", name),
                ("copy", if *copy { "true" } else { "false" }),
            ],
        ),
        Dialog::SaveAs { name } => line("operandi.layout.saveAs", &[("name", name)]),
        Dialog::Rename { layout, name, .. } => line(
            "operandi.layout.rename",
            &[("name", name), ("layout", layout)],
        ),
        Dialog::Delete { layout } => line("operandi.layout.delete", &[("layout", layout)]),
        Dialog::SavePiece {
            name,
            category,
            group,
        } => line(
            "operandi.viewport.saveAs",
            &[
                ("name", name),
                ("category", category),
                ("group", if *group { "true" } else { "false" }),
            ],
        ),
    }
}

/// A view-area slot kind the content arrow can put in the middle column.
fn view_area_kind(kind: &str) -> Option<&'static str> {
    [VIEW_3D, UV_VIEW, EMPTY]
        .into_iter()
        .find(|k| k.eq_ignore_ascii_case(kind.trim()))
}

fn parse_bool(command: &str, value: Option<&str>) -> std::result::Result<Option<bool>, String> {
    value
        .map(|v| operandi_core::command_args::boolean(command, v).map_err(|e| e.to_string()))
        .transpose()
}

impl crate::layouts::Layouts {
    /// Sort the list by the state file's order; layouts it doesn't name
    /// keep their file order after those it does.
    pub(super) fn apply_state_order(&mut self, order: &[String]) {
        if order.is_empty() {
            return;
        }
        let place = |name: &str| {
            order
                .iter()
                .position(|o| o.eq_ignore_ascii_case(name))
                .unwrap_or(usize::MAX)
        };
        self.list.sort_by_key(|e| place(e.name()));
    }

    /// `name` was just shown: it goes to the front of the MRU list.
    pub(super) fn note_shown(&mut self, name: &str) {
        self.mru.retain(|n| !n.eq_ignore_ascii_case(name));
        self.mru.insert(0, name.to_owned());
    }

    /// Every layout in most-recently-used order: the MRU list, then the
    /// rest in switcher order (UIC-18).
    pub fn mru_order(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .mru
            .iter()
            .filter_map(|n| self.find(n).map(|e| e.name().to_owned()))
            .collect();
        for name in self.names() {
            if !out.contains(&name) {
                out.push(name);
            }
        }
        out
    }

    /// The layouts the pie shows: all, or the starred ones (UIC-14). The
    /// current layout always shows.
    pub fn shown_names(&self) -> Vec<String> {
        self.names()
            .into_iter()
            .filter(|n| !self.starred_only || self.stars.contains(n) || *n == self.current)
            .collect()
    }

    fn rename_everywhere(&mut self, old: &str, new: &str) {
        let swap = |n: &mut String| {
            if n == old {
                *n = new.to_owned();
            }
        };
        self.mru.iter_mut().for_each(swap);
        if self.stars.remove(old) {
            self.stars.insert(new.to_owned());
        }
        if let Some(locks) = self.locks.remove(old) {
            self.locks.insert(new.to_owned(), locks);
        }
        if self.current == old {
            self.current = new.to_owned();
        }
    }

    fn forget(&mut self, name: &str) {
        self.list.retain(|e| e.name() != name);
        self.mru.retain(|n| n != name);
        self.stars.remove(name);
        self.locks.remove(name);
    }
}

impl Operandi {
    fn pieces_folder(&self) -> PathBuf {
        self.environment.paths().data_root().join("Pieces")
    }

    /// Read the user's pieces (LAY-72). A file that doesn't read is left
    /// out.
    pub(super) fn load_pieces(&mut self) {
        let mut pieces = Vec::new();
        if let Ok(entries) = std::fs::read_dir(self.pieces_folder()) {
            let mut paths: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect();
            paths.sort();
            for path in paths {
                if let Some(piece) = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|t| serde_json::from_str::<Piece>(&t).ok())
                    .filter(|p| p.version == PIECE_VERSION && !p.name.trim().is_empty())
                {
                    pieces.push(piece);
                }
            }
        }
        self.layouts.pieces = pieces;
    }

    fn write_piece(&self, piece: &Piece) -> std::result::Result<(), String> {
        let folder = self.pieces_folder();
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let path = folder.join(file_name(&piece.name));
        let text = serde_json::to_string_pretty(piece).map_err(|e| e.to_string())?;
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, text).map_err(|e| e.to_string())?;
        std::fs::rename(&temp, &path).map_err(|e| e.to_string())
    }

    /// Write a user layout's file now.
    pub(super) fn write_user_layout(&self, doc: &WindowLayout) -> std::result::Result<(), String> {
        let path = self.layout_file(&doc.name).ok_or("No layouts folder")?;
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        }
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, doc.to_json()).map_err(|e| e.to_string())?;
        std::fs::rename(&temp, &path).map_err(|e| e.to_string())
    }

    fn remove_user_layout_file(&self, name: &str) {
        if let Some(path) = self.layout_file(name)
            && path.is_file()
        {
            let _ = std::fs::remove_file(path);
        }
    }

    /// The current layout's Lock Thumb / Divider Dragging (LAY-48).
    pub(super) fn apply_layout_locks(&mut self) {
        let locks = self
            .layouts
            .locks
            .get(&self.layouts.current)
            .copied()
            .unwrap_or(self.layouts.default_locks);
        self.dots.locks = locks;
        if locks.thumbs {
            self.dots.drag = None;
        }
        if locks.dividers {
            self.layout_capture = None;
        }
    }

    /// Set the current layout's locks and remember them (panel_dot.rs's
    /// View > Layout entries moved to Layout > Lock …, LAY-48).
    pub(super) fn set_current_layout_locks(&mut self, locks: LayoutLocks) {
        let name = self.layouts.current.clone();
        self.layouts.locks.insert(name, locks);
        self.apply_layout_locks();
        self.write_layouts_state();
    }

    fn tool_busy(&self) -> bool {
        self.editor.live.is_some() || self.pointer_held
    }

    fn free_layout_name(
        &self,
        name: &str,
        except: Option<&str>,
    ) -> std::result::Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("A layout needs a name".into());
        }
        if let Some(e) = self.layouts.find(name)
            && except.is_none_or(|x| !e.name().eq_ignore_ascii_case(x))
        {
            return Err(format!("There is a layout called {} already", e.name()));
        }
        Ok(name.to_owned())
    }

    fn layout_or_current(&self, layout: Option<&str>) -> std::result::Result<String, String> {
        let name = layout.unwrap_or(&self.layouts.current);
        self.layouts
            .find(name)
            .map(|e| e.name().to_owned())
            .ok_or_else(|| {
                format!(
                    "No layout named {name}; there are {}",
                    self.layouts.names().join(", ")
                )
            })
    }

    // ------------------------------------------------------------ actions

    /// New layout (LAY-43): a copy of the current one, or a blank window
    /// whose view area is one empty panel. It goes at the right-hand end
    /// and becomes current.
    pub(super) fn create_layout(
        &mut self,
        name: &str,
        copy: bool,
    ) -> std::result::Result<(), String> {
        let name = self.free_layout_name(name, None)?;
        if self.tool_busy() {
            return Err("Drop the tool before making a layout".into());
        }
        self.note_torn_windows();
        let mut doc = self.capture_layout().ok_or("No layout is showing")?;
        doc.name = name.clone();
        doc.order = self
            .layouts
            .list
            .iter()
            .map(|e| e.doc().order)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        doc.starred = false;
        if !copy {
            // Provisional (LAY-Q11): the side columns stay; the view area
            // is one empty panel to build from (LAY-74).
            replace_views(&mut doc.frame, &Frame::Panel(Slot::new(EMPTY)));
            doc.windows.clear();
            doc.group_piece = None;
            // A blank layout starts on the global keys; a copy keeps the
            // copied layout's own (provisional, LAY-Q57).
            doc.keys.clear();
        }
        self.write_user_layout(&doc)?;
        self.layouts.list.push(Entry {
            shipped: None,
            user: Some(doc),
        });
        self.restore_layout(&name)?;
        self.status = format!("{name} layout made");
        Ok(())
    }

    /// Save Layout (UIC-199): the current layout's file, now.
    fn save_layout_now(&mut self) {
        self.layouts.requests.clear();
        self.write_current_layout_now();
        let name = self.layouts.current.clone();
        let changed = self.layouts.find(&name).is_some_and(|e| e.user.is_some());
        self.status = if changed {
            format!("{name} layout saved")
        } else {
            format!("{name} is as shipped; nothing to save")
        };
    }

    /// Rename (LAY-45): layouts you made only.
    pub(super) fn rename_layout(
        &mut self,
        layout: &str,
        name: &str,
    ) -> std::result::Result<(), String> {
        let old = self.layout_or_current(Some(layout))?;
        let entry = self.layouts.find(&old).cloned().ok_or("No such layout")?;
        if entry.shipped.is_some() {
            return Err(format!(
                "{old} is a shipped layout: it keeps its name (Save Layout As makes a copy you can name)"
            ));
        }
        let new = self.free_layout_name(name, Some(&old))?;
        if new == old {
            return Ok(());
        }
        // The current layout's file is written from the window first.
        if old == self.layouts.current {
            self.write_current_layout_now();
        }
        let mut doc = self
            .layouts
            .find(&old)
            .map(|e| e.doc().clone())
            .ok_or("No such layout")?;
        doc.name = new.clone();
        self.write_user_layout(&doc)?;
        if !new.eq_ignore_ascii_case(&old) {
            self.remove_user_layout_file(&old);
        }
        if let Some(entry) = self.layouts.find_mut(&old) {
            entry.user = Some(doc);
        }
        self.layouts.rename_everywhere(&old, &new);
        self.rename_stash(&old, &new);
        if new == self.layouts.current {
            self.layouts_saved_clear();
        }
        self.write_layouts_state();
        self.status = format!("{old} layout renamed {new}");
        Ok(())
    }

    /// Delete (LAY-46): layouts you made only; the dialog asks first.
    pub(super) fn delete_layout(
        &mut self,
        layout: Option<&str>,
    ) -> std::result::Result<(), String> {
        let name = self.layout_or_current(layout)?;
        let entry = self.layouts.find(&name).cloned().ok_or("No such layout")?;
        if entry.shipped.is_some() {
            return Err(format!(
                "{name} is a shipped layout: it can be reset but not deleted"
            ));
        }
        if name == self.layouts.current {
            let other = self
                .layouts
                .mru_order()
                .into_iter()
                .find(|n| *n != name)
                .ok_or("The last layout can't be deleted")?;
            self.restore_layout(&other)?;
        }
        self.remove_user_layout_file(&name);
        self.drop_stash(&name);
        self.layouts.forget(&name);
        self.write_layouts_state();
        self.status = format!("{name} layout deleted");
        Ok(())
    }

    /// Clear Layout (LAY-47): the current view area becomes one empty
    /// panel.
    fn clear_layout(&mut self) -> std::result::Result<(), String> {
        if self.tool_busy() {
            return Err("Drop the tool before clearing the layout".into());
        }
        let mut doc = self.capture_layout().ok_or("No layout is showing")?;
        replace_views(&mut doc.frame, &Frame::Panel(Slot::new(EMPTY)));
        doc.windows.clear();
        doc.group_piece = None;
        // The window keeps its toolbox tab: no open command.
        doc.open_command = None;
        self.apply_layout_doc(&doc)?;
        self.layouts_saved_clear();
        self.status = format!("{} layout cleared", self.layouts.current);
        Ok(())
    }

    fn move_layout(&mut self, layout: &str, index: i64) -> std::result::Result<(), String> {
        let name = self.layout_or_current(Some(layout))?;
        let from = self
            .layouts
            .list
            .iter()
            .position(|e| e.name() == name)
            .ok_or("No such layout")?;
        let entry = self.layouts.list.remove(from);
        let to = (index.max(0) as usize).min(self.layouts.list.len());
        self.layouts.list.insert(to, entry);
        self.write_layouts_state();
        Ok(())
    }

    /// Drop a switched-away layout's kept state (its file opens next time).
    fn drop_stash(&mut self, name: &str) {
        self.layouts.stash_remove(name);
    }

    fn rename_stash(&mut self, old: &str, new: &str) {
        self.layouts.stash_rename(old, new);
    }

    // ------------------------------------------------------------- pieces

    fn piece(&self, name: &str) -> Option<&Piece> {
        self.layouts
            .pieces
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name.trim()))
    }

    /// The keyed pane `panel` names, or the active one.
    fn panel_pane(&self, panel: Option<i64>) -> std::result::Result<Pane, String> {
        let key = match panel {
            Some(k) => u32::try_from(k).map_err(|_| "No such panel".to_string())?,
            None => self.active_view,
        };
        self.views
            .pane_with_key(key)
            .ok_or_else(|| format!("There's no view-area panel {key}"))
    }

    /// A slot for the pane with this key, as a layout file would hold it.
    fn slot_of(&self, pane: Pane) -> Slot {
        let default_display = self.new_view_display().settings;
        let mut slot = match pane {
            Pane::Viewport(key) => {
                let display = if key == self.active_view {
                    Some(&self.display)
                } else {
                    self.displays.get(&key)
                };
                let mut slot = Slot::new(VIEW_3D);
                if let Some(d) = display.filter(|d| d.settings != default_display)
                    && let Ok(value) = serde_json::to_value(d.settings)
                {
                    slot.settings = serde_json::json!({ "display": value });
                }
                slot
            }
            Pane::Uv(_) => Slot::new(UV_VIEW),
            Pane::Missing(key) => {
                Slot::new(self.views.missing.get(&key).map_or(EMPTY, String::as_str))
            }
            _ => Slot::new(EMPTY),
        };
        slot.piece = pane
            .key()
            .and_then(|k| self.layouts.piece_of.get(&k).cloned());
        slot
    }

    /// Save Viewport As / Save Viewport Group As (LAY-72).
    pub(super) fn save_piece_as(
        &mut self,
        name: &str,
        category: &str,
        group: bool,
    ) -> std::result::Result<(), String> {
        let name = name.trim().to_owned();
        if name.is_empty() {
            return Err("A saved viewport needs a name".into());
        }
        if let Some(p) = self.piece(&name)
            && p.shipped
        {
            return Err(format!(
                "{} is a shipped piece; give yours another name",
                p.name
            ));
        }
        let frame = if group {
            let doc = self.capture_layout().ok_or("No layout is showing")?;
            view_frame(&doc.frame).ok_or("No view area")?
        } else {
            let pane = self.panel_pane(None)?;
            let mut slot = self.slot_of(pane);
            slot.piece = None;
            slot.maximized = false;
            Frame::Panel(slot)
        };
        // A group piece's panels lose any piece of their own: the group is
        // the piece now.
        let frame = strip_pieces(frame);
        let piece = Piece {
            version: PIECE_VERSION,
            name: name.clone(),
            category: category.trim().to_owned(),
            group,
            frame,
            shipped: false,
        };
        self.write_piece(&piece)?;
        self.layouts
            .pieces
            .retain(|p| !p.name.eq_ignore_ascii_case(&name));
        self.layouts.pieces.push(piece.clone());
        // The panel (or the view area) now shows that piece.
        if group {
            self.layouts.piece_of.clear();
            self.layouts.group_piece = Some(name.clone());
        } else if let Ok(pane) = self.panel_pane(None)
            && let Some(key) = pane.key()
        {
            self.layouts.piece_of.insert(key, name.clone());
        }
        self.update_piece_users(&piece);
        self.status = format!("Saved {name}");
        Ok(())
    }

    /// Save Viewport (Group): the piece the active panel shows, or the view
    /// area's group piece, saved again from the window.
    fn save_piece_again(&mut self) -> std::result::Result<(), String> {
        let (name, group) = self
            .active_piece()
            .ok_or("The active panel shows no saved viewport: use Save Viewport As first")?;
        let category = self
            .piece(&name)
            .map(|p| p.category.clone())
            .unwrap_or_default();
        self.save_piece_as(&name, &category, group)
    }

    /// The piece the active panel shows (else the view area's group piece).
    fn active_piece(&self) -> Option<(String, bool)> {
        if let Ok(pane) = self.panel_pane(None)
            && let Some(name) = pane.key().and_then(|k| self.layouts.piece_of.get(&k))
        {
            return Some((name.clone(), false));
        }
        self.layouts.group_piece.clone().map(|g| (g, true))
    }

    /// Saving a piece updates every layout that shows it: switched-away
    /// layouts' files (they reopen from them) and this window.
    fn update_piece_users(&mut self, piece: &Piece) {
        let current = self.layouts.current.clone();
        let names = self.layouts.names();
        for name in names {
            if name == current {
                continue;
            }
            let Some(entry) = self.layouts.find(&name) else {
                continue;
            };
            let mut doc = entry.doc().clone();
            let shipped = entry.shipped.clone();
            if !apply_piece_to_doc(&mut doc, piece) {
                continue;
            }
            let same = shipped.as_ref().is_some_and(|s| s.same_as(&doc));
            if same {
                self.remove_user_layout_file(&name);
            } else {
                let _ = self.write_user_layout(&doc);
            }
            if let Some(entry) = self.layouts.find_mut(&name) {
                entry.user = (!same).then_some(doc);
            }
            self.drop_stash(&name);
        }
        // This window: panels showing it take its slot again.
        self.refresh_piece_here(piece);
    }

    /// Put `piece`'s saved version on every panel of this window that
    /// shows it.
    fn refresh_piece_here(&mut self, piece: &Piece) {
        if piece.group {
            if self.layouts.group_piece.as_deref() == Some(piece.name.as_str())
                && let Some(mut doc) = self.capture_layout()
                && apply_piece_to_doc(&mut doc, piece)
            {
                // The view area's panels are this window's: rebuild it,
                // without the layout's open command.
                doc.open_command = None;
                let _ = self.apply_layout_doc(&doc);
            }
            return;
        }
        let Frame::Panel(slot) = &piece.frame else {
            return;
        };
        let keys: Vec<u32> = self
            .layouts
            .piece_of
            .iter()
            .filter(|(_, n)| n.eq_ignore_ascii_case(&piece.name))
            .map(|(k, _)| *k)
            .collect();
        for key in keys {
            if let Some(pane) = self.views.pane_with_key(key) {
                let _ = self.show_slot_in(pane, slot, Some(piece.name.clone()));
            }
        }
    }

    /// `operandi.viewport.use`: the piece in a panel's place.
    fn use_piece(&mut self, name: &str, panel: Option<i64>) -> std::result::Result<(), String> {
        let piece = self
            .piece(name)
            .cloned()
            .ok_or_else(|| format!("No saved viewport named {name}"))?;
        let pane = self.panel_pane(panel)?;
        if piece.group {
            let whole = self.views.last_in_main(pane);
            if let Some(key) = pane.key() {
                self.layouts.piece_of.remove(&key);
            }
            let grafted = self.views.graft(pane, &piece.frame)?;
            self.adopt_new_views(&grafted.slots);
            self.layouts.piece_of.extend(grafted.pieces);
            if whole {
                self.layouts.group_piece = Some(piece.name.clone());
            }
            self.forget_unlisted_views();
        } else {
            let Frame::Panel(slot) = &piece.frame else {
                return Err("The saved viewport isn't one panel".into());
            };
            self.show_slot_in(pane, slot, Some(piece.name.clone()))?;
        }
        self.status = format!("{} placed", piece.name);
        Ok(())
    }

    /// Revert (LAY-72): the active panel's piece (or the view area's group
    /// piece) back to its saved version.
    fn revert_piece(&mut self) -> std::result::Result<(), String> {
        let (name, _) = self
            .active_piece()
            .ok_or("The active panel shows no saved viewport")?;
        let piece = self.piece(&name).cloned().ok_or("The piece is gone")?;
        self.refresh_piece_here(&piece);
        self.status = format!("{name} reverted");
        Ok(())
    }

    /// Delete Saved Viewport (LAY-72): user pieces only; panels keep what
    /// they show.
    fn delete_piece(&mut self, name: &str) -> std::result::Result<(), String> {
        let piece = self
            .piece(name)
            .cloned()
            .ok_or_else(|| format!("No saved viewport named {name}"))?;
        if piece.shipped {
            return Err(format!("{} is a shipped piece", piece.name));
        }
        let path = self.pieces_folder().join(file_name(&piece.name));
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        self.layouts.pieces.retain(|p| p.name != piece.name);
        self.layouts.piece_of.retain(|_, n| *n != piece.name);
        if self.layouts.group_piece.as_deref() == Some(piece.name.as_str()) {
            self.layouts.group_piece = None;
        }
        self.status = format!("{} deleted", piece.name);
        Ok(())
    }

    /// Fresh views just put in the layout get cameras and settings (as a
    /// layout file opening does).
    fn adopt_new_views(&mut self, slots: &BTreeMap<u32, Slot>) {
        let mut camera = self.camera.clone();
        camera.view = camera::View::Perspective;
        for (&key, slot) in slots {
            self.cameras.insert(key, camera.clone());
            if slot.kind == UV_VIEW {
                self.uv_views.insert(key, uv_view::UvView::default());
            } else if let Ok(settings) =
                serde_json::from_value::<view_display::Settings>(slot.settings["display"].clone())
            {
                self.displays.insert(
                    key,
                    view_display::ViewDisplay {
                        settings,
                        applied: None,
                    },
                );
            } else {
                self.displays.insert(key, self.new_view_display());
            }
        }
    }

    /// Make `pane`'s place show `slot` (a kind and its settings) under a
    /// new key. Returns the new pane.
    fn show_slot_in(
        &mut self,
        pane: Pane,
        slot: &Slot,
        piece: Option<String>,
    ) -> std::result::Result<Pane, String> {
        let key = self.views.new_viewport_key();
        let new = match slot.kind.as_str() {
            VIEW_3D => Pane::Viewport(key),
            UV_VIEW => Pane::Uv(key),
            _ => Pane::Missing(key),
        };
        let was_active = pane.key() == Some(self.active_view);
        self.views.replace(pane, new)?;
        if let Some(old) = pane.key() {
            self.views.missing.remove(&old);
            self.layouts.piece_of.remove(&old);
        }
        if let Pane::Missing(k) = new {
            self.views.missing.insert(k, slot.kind.clone());
        } else {
            self.adopt_new_views(&[(key, slot.clone())].into());
        }
        if let Some(piece) = piece {
            self.layouts.piece_of.insert(key, piece);
        }
        if new.view_key().is_some() && was_active {
            // The slot was the active one: it stays so.
            self.active_view = u32::MAX;
            self.activate_view(key);
        } else if was_active {
            self.active_view = key;
        }
        self.forget_unlisted_views();
        Ok(new)
    }

    /// `operandi.panel.kind` (the content arrow, LAY-73): the slot shows
    /// another kind, in the same place.
    pub(super) fn set_panel_kind(
        &mut self,
        kind: &str,
        panel: Option<i64>,
    ) -> std::result::Result<(), String> {
        let pane = self.panel_pane(panel)?;
        let Some(kind) = view_area_kind(kind) else {
            let side = panel_registry::REGISTRY
                .iter()
                .any(|t| t.name.eq_ignore_ascii_case(kind.trim()));
            return Err(if side {
                format!(
                    "{} opens in the side columns in this build (a view-area panel shows 3D View, UV View or Empty)",
                    kind.trim()
                )
            } else {
                format!("No panel kind named {}", kind.trim())
            });
        };
        if self.slot_of(pane).kind == kind {
            return Ok(());
        }
        self.show_slot_in(pane, &Slot::new(kind), None)?;
        self.status = format!("The panel shows {kind}");
        Ok(())
    }

    // ------------------------------------------------------ command lines

    /// The stage 2 layout commands (command_handlers.rs routes them here).
    pub(super) fn layout_command(&mut self, command: &str, rest: &str) {
        let args = match command::tokens(rest) {
            Ok(args) => args,
            Err(e) => {
                self.status = format!("{command}: {e}");
                return;
            }
        };
        let bind = |names: &[&str]| {
            operandi_core::command_args::bind(command, &args, names)
                .map(|b| {
                    b.into_iter()
                        .map(|v| v.map(str::to_owned))
                        .collect::<Vec<_>>()
                })
                .map_err(|e| e.to_string())
        };
        let int = |v: &Option<String>| -> std::result::Result<Option<i64>, String> {
            v.as_deref()
                .map(|t| {
                    t.trim()
                        .parse::<i64>()
                        .map_err(|_| format!("{command}: {t} isn't a number"))
                })
                .transpose()
        };
        let need = |v: &Option<String>, what: &str| -> std::result::Result<String, String> {
            v.clone()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| format!("{command} needs {what}"))
        };
        let result: std::result::Result<(), String> = (|| match command {
            "operandi.layout.create" => {
                let b = bind(&["name", "copy"])?;
                let copy = parse_bool(command, b[1].as_deref())?.unwrap_or(false);
                // CMD-12: without a name, Layout > New Layout…'s dialog.
                if b[0].is_none() {
                    self.layouts.dialog = Some(Dialog::New {
                        name: String::new(),
                        copy,
                    });
                    return Ok(());
                }
                self.create_layout(&need(&b[0], "a name")?, copy)
            }
            "operandi.layout.save" => {
                bind(&[])?;
                self.save_layout_now();
                Ok(())
            }
            "operandi.layout.saveas" => {
                let b = bind(&["name"])?;
                if b[0].is_none() {
                    self.layouts.dialog = Some(Dialog::SaveAs {
                        name: String::new(),
                    });
                    return Ok(());
                }
                self.create_layout(&need(&b[0], "a name")?, true)
            }
            "operandi.layout.rename" => {
                let b = bind(&["name", "layout"])?;
                let layout = self.layout_or_current(b[1].as_deref())?;
                if b[0].is_none() {
                    self.layouts.dialog = Some(Dialog::Rename {
                        layout: layout.clone(),
                        name: layout,
                        opening: true,
                    });
                    return Ok(());
                }
                self.rename_layout(&layout, &need(&b[0], "a new name")?)
            }
            "operandi.layout.delete" => {
                let b = bind(&["layout", "ask"])?;
                // ask:true is Layout > Delete Layout…: its dialog asks.
                if parse_bool(command, b[1].as_deref())?.unwrap_or(false) {
                    let layout = self.layout_or_current(b[0].as_deref())?;
                    self.layouts.dialog = Some(Dialog::Delete { layout });
                    return Ok(());
                }
                self.delete_layout(b[0].as_deref())
            }
            "operandi.layout.revert" | "operandi.layout.reset" => {
                let b = bind(&["layout", "keys", "ask"])?;
                let name = self.layout_or_current(b[0].as_deref())?;
                let keys = parse_bool(command, b[1].as_deref())?.unwrap_or(false);
                // ask:true is Layout > Reset Layout…: LAY-42 asks first.
                if parse_bool(command, b[2].as_deref())?.unwrap_or(false) {
                    self.layouts.confirm_reset = Some(name);
                    return Ok(());
                }
                self.revert_layout_keys(&name, keys)
            }
            // Layouts stage 5 (layout_keys.rs, LAY-20).
            "operandi.layout.setkey" => {
                let b = bind(&["action", "key", "layout"])?;
                let name = self.layout_or_current(b[2].as_deref())?;
                self.set_layout_key(&need(&b[0], "an action")?, b[1].as_deref(), &name)
            }
            "operandi.layout.clearkeys" => {
                let b = bind(&["layout"])?;
                let name = self.layout_or_current(b[0].as_deref())?;
                self.clear_layout_keys(&name)
            }
            "operandi.layout.clear" => {
                bind(&[])?;
                self.clear_layout()
            }
            "operandi.layout.star" => {
                let b = bind(&["layout", "on"])?;
                let name = self.layout_or_current(b[0].as_deref())?;
                let on = parse_bool(command, b[1].as_deref())?
                    .unwrap_or(!self.layouts.stars.contains(&name));
                if on {
                    self.layouts.stars.insert(name.clone());
                } else {
                    self.layouts.stars.remove(&name);
                }
                self.write_layouts_state();
                self.status = format!("{name} {}", if on { "starred" } else { "unstarred" });
                Ok(())
            }
            "operandi.layout.starredonly" => {
                let b = bind(&["on"])?;
                self.layouts.starred_only =
                    parse_bool(command, b[0].as_deref())?.unwrap_or(!self.layouts.starred_only);
                self.write_layouts_state();
                Ok(())
            }
            "operandi.bar.collapse" => {
                use crate::collapse_line::Bar;
                let b = bind(&["bar", "on", "width"])?;
                let bar = b[0].as_deref().unwrap_or("top");
                let bar = Bar::parse(bar)
                    .ok_or(format!("{command}: bar: is top, left or right, not {bar}"))?;
                let on = parse_bool(command, b[1].as_deref())?
                    .unwrap_or(!self.layouts.is_collapsed(bar));
                let width = match b[2].as_deref() {
                    Some(w) => Some(
                        w.parse::<f32>()
                            .ok()
                            .filter(|w| w.is_finite())
                            .ok_or(format!("{command}: width: is a number, not {w}"))?,
                    ),
                    None => None,
                };
                self.layouts.set_collapsed(bar, on);
                // The side bars read it when they next draw (it needs the
                // window's panel state).
                if !on
                    && bar != Bar::Top
                    && let Some(width) = width
                {
                    self.layouts.side_reopen = Some((bar, width));
                }
                self.write_layouts_state();
                Ok(())
            }
            "operandi.layout.move" => {
                let b = bind(&["layout", "index"])?;
                let index = int(&b[1])?.ok_or(format!("{command} needs an index"))?;
                self.move_layout(&need(&b[0], "a layout")?, index)
            }
            "operandi.layout.switcher" => {
                bind(&[])?;
                self.open_switcher(false);
                Ok(())
            }
            "layout.lockthumbs" | "layout.lockdividers" => {
                let b = bind(&["lock"])?;
                let mut locks = self.dots.locks;
                let thumbs = command == "layout.lockthumbs";
                let now = if thumbs { locks.thumbs } else { locks.dividers };
                let on = parse_bool(command, b[0].as_deref())?.unwrap_or(!now);
                if thumbs {
                    locks.thumbs = on;
                } else {
                    locks.dividers = on;
                }
                self.set_current_layout_locks(locks);
                Ok(())
            }
            "operandi.layout.buildmode" => {
                let b = bind(&["on"])?;
                let on = parse_bool(command, b[0].as_deref())?.unwrap_or(!self.dots.build_mode);
                self.set_build_mode(on);
                Ok(())
            }
            "operandi.viewport.saveas" => {
                let b = bind(&["name", "category", "group"])?;
                let group = parse_bool(command, b[2].as_deref())?.unwrap_or(false);
                self.save_piece_as(
                    &need(&b[0], "a name")?,
                    b[1].as_deref().unwrap_or("User"),
                    group,
                )
            }
            "operandi.viewport.save" => {
                bind(&[])?;
                self.save_piece_again()
            }
            "operandi.viewport.use" => {
                let b = bind(&["name", "panel"])?;
                let panel = int(&b[1])?;
                self.use_piece(&need(&b[0], "a saved viewport's name")?, panel)
            }
            "operandi.viewport.revert" => {
                bind(&[])?;
                self.revert_piece()
            }
            "operandi.viewport.delete" => {
                let b = bind(&["name"])?;
                self.delete_piece(&need(&b[0], "a saved viewport's name")?)
            }
            "operandi.panel.kind" => {
                let b = bind(&["kind", "panel"])?;
                let panel = int(&b[1])?;
                self.set_panel_kind(&need(&b[0], "a panel kind")?, panel)
            }
            _ => Err(format!("{command} isn't a layout command")),
        })();
        if let Err(e) = result {
            self.status = e;
        }
    }

    // ------------------------------------------------------ the switcher

    /// Open the quick switcher (UIC-18) with the next layout outlined.
    pub(super) fn open_switcher(&mut self, held: bool) {
        let list = self.layouts.mru_order();
        let at = usize::from(list.len() > 1);
        self.layouts.switcher = Some(Switcher { list, at, held });
    }

    /// While the switcher is open it owns the keys: Tab moves the outline
    /// on (Shift+Tab back when not held), releasing Ctrl+Shift switches
    /// (held), Return switches, Esc closes. True while it is open.
    pub(super) fn switcher_input(&mut self, ctx: &Context, held_tab: &[Event]) -> bool {
        let Some(mut sw) = self.layouts.switcher.take() else {
            return false;
        };
        let (tabs, back, enter, escape, mods) = ctx.input(|i| {
            let mut tabs = 0usize;
            let mut back = 0usize;
            let mut enter = false;
            let mut escape = false;
            for event in i.events.iter().chain(held_tab) {
                if let Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                {
                    match key {
                        Key::Tab if !sw.held && modifiers.shift && !modifiers.ctrl => back += 1,
                        Key::Tab | Key::ArrowDown => tabs += 1,
                        Key::ArrowUp => back += 1,
                        Key::Enter => enter = true,
                        Key::Escape => escape = true,
                        _ => {}
                    }
                }
            }
            (tabs, back, enter, escape, i.modifiers)
        });
        ctx.input_mut(|i| {
            i.events
                .retain(|e| !matches!(e, Event::Key { .. } | Event::Text(_)));
        });
        ctx.request_repaint();
        if escape || sw.list.is_empty() {
            return true;
        }
        let n = sw.list.len();
        sw.at = (sw.at + tabs % n) % n;
        sw.at = (sw.at + n - back % n) % n;
        let released = sw.held && !(mods.ctrl && mods.shift);
        if released || enter {
            let name = sw.list[sw.at].clone();
            self.execute(&Self::restore_line(&name));
            return true;
        }
        self.layouts.switcher = Some(sw);
        true
    }

    /// Draw the switcher popup: the MRU list, the next one outlined.
    pub(super) fn switcher_window(&mut self, ctx: &Context) {
        let Some(sw) = self.layouts.switcher.clone() else {
            return;
        };
        let mut picked = None;
        egui::Area::new(Id::new("layout-switcher"))
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_CENTER, vec2(0., 0.))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(180.);
                    ui.label(RichText::new("Layouts").strong());
                    for (i, name) in sw.list.iter().enumerate() {
                        let current = *name == self.layouts.current;
                        let label = if current {
                            format!("{name}  (current)")
                        } else {
                            name.clone()
                        };
                        let response = ui.add(
                            Button::selectable(i == sw.at, label)
                                .stroke(if i == sw.at {
                                    Stroke::new(1.5_f32, theme::orange())
                                } else {
                                    Stroke::NONE
                                })
                                .min_size(vec2(170., 0.)),
                        );
                        if response.clicked() {
                            picked = Some(name.clone());
                        }
                    }
                    if !sw.held {
                        ui.label(
                            RichText::new("Return switches · Esc closes")
                                .small()
                                .color(theme::disabled_text()),
                        );
                    }
                });
            });
        if let Some(name) = picked {
            self.layouts.switcher = None;
            self.execute(&Self::restore_line(&name));
        }
    }

    /// The Layouts pie (UIC-18): a slot per layout, in switcher order (the
    /// first eight).
    pub(super) fn layouts_pie(&self) -> crate::pie_menu::Pie {
        let names = self.layouts.shown_names();
        let lines: Vec<(String, String)> = names
            .iter()
            .take(8)
            .map(|n| (n.clone(), Self::restore_line(n)))
            .collect();
        let entries: Vec<Option<(&str, &str)>> = lines
            .iter()
            .map(|(l, c)| Some((l.as_str(), c.as_str())))
            .collect();
        crate::pie_menu::Pie::new(crate::pie_menu::LAYOUTS, "Layouts", &entries)
    }

    // ------------------------------------------------------- dialogs

    /// The New / Save As / Rename / Delete / Save Viewport dialog. OK runs
    /// the dialog's command line; Cancel and Esc close it.
    pub(super) fn layout_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.layouts.dialog.clone() else {
            return;
        };
        let title = match &dialog {
            Dialog::New { .. } => "New Layout",
            Dialog::SaveAs { .. } => "Save Layout As",
            Dialog::Rename { .. } => "Rename Layout",
            Dialog::Delete { .. } => "Delete Layout",
            Dialog::SavePiece { group: false, .. } => "Save Viewport As",
            Dialog::SavePiece { group: true, .. } => "Save Viewport Group As",
        };
        let mut answer = None;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0., 0.))
            .show(ctx, |ui| {
                let mut ok_allowed = true;
                match &mut dialog {
                    Dialog::New { name, copy } => {
                        ui.horizontal(|ui| {
                            ui.label("Name");
                            ui.text_edit_singleline(name).request_focus();
                        });
                        ui.checkbox(copy, "As Copy of Current Layout");
                        ok_allowed = !name.trim().is_empty();
                    }
                    Dialog::SaveAs { name } => {
                        ui.horizontal(|ui| {
                            ui.label("Name");
                            ui.text_edit_singleline(name).request_focus();
                        });
                        ok_allowed = !name.trim().is_empty();
                    }
                    Dialog::Rename { name, opening, .. } => {
                        ui.horizontal(|ui| {
                            ui.label("Name");
                            let edit = ui.text_edit_singleline(name);
                            crate::numeric_tab::register(ui, &edit, Some(name));
                            crate::numeric_tab::focus_and_select_on_open(&edit, name, opening);
                        });
                        ok_allowed = !name.trim().is_empty();
                    }
                    Dialog::Delete { layout } => {
                        ui.label(format!("Delete the {layout} layout? This can't be undone."));
                    }
                    Dialog::SavePiece { name, category, .. } => {
                        ui.horizontal(|ui| {
                            ui.label("Name");
                            ui.text_edit_singleline(name).request_focus();
                        });
                        ui.horizontal(|ui| {
                            ui.label("Category");
                            ui.text_edit_singleline(category);
                        });
                        ok_allowed = !name.trim().is_empty();
                    }
                }
                ui.horizontal(|ui| {
                    let ok = match dialog {
                        Dialog::Delete { .. } => "Delete",
                        Dialog::New { .. } => "Make",
                        _ => "Save",
                    };
                    let ok = ui.gated(
                        crate::unavailable::unless(ok_allowed, "Give it a name"),
                        Button::new(ok),
                    );
                    if ok.clicked() || (ok_allowed && ui.input(|i| i.key_pressed(Key::Enter))) {
                        answer = Some(true);
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                        answer = Some(false);
                    }
                });
            });
        match answer {
            Some(true) => {
                self.layouts.dialog = None;
                self.execute(&dialog_line(&dialog));
            }
            Some(false) => self.layouts.dialog = None,
            None => self.layouts.dialog = Some(dialog),
        }
    }

    /// The dialog's OK, as its button runs it (tests).
    #[cfg(test)]
    pub(super) fn answer_layout_dialog(&mut self) {
        if let Some(dialog) = self.layouts.dialog.take() {
            self.execute(&dialog_line(&dialog));
        }
    }

    // ------------------------------------------------- the content arrow

    /// The content arrow at the top right of a view-area panel's header
    /// (LAY-73): a click or right-click lists what the slot can show.
    pub(super) fn content_arrow(&mut self, ui: &mut Ui, key: u32, rect: Rect) {
        let response = ui
            .interact(rect, Id::new(("content-arrow", key)), Sense::click())
            .on_hover_text("What this panel shows");
        let color = if response.hovered() {
            theme::text()
        } else {
            theme::disabled_text()
        };
        let c = rect.center();
        ui.painter().add(egui::Shape::convex_polygon(
            vec![c + vec2(-3.5, -1.5), c + vec2(3.5, -1.5), c + vec2(0., 2.5)],
            color,
            Stroke::NONE,
        ));
        if response.clicked() || response.secondary_clicked() {
            self.menu_owned_input = true;
        }
        egui::Popup::menu(&response).show(|ui| self.content_menu(ui, key));
        response.context_menu(|ui| self.content_menu(ui, key));
    }

    /// The content arrow's menu. Every entry queues a command line, run
    /// once the views have drawn.
    fn content_menu(&mut self, ui: &mut Ui, key: u32) {
        self.menu_owned_input = true;
        let mut chosen: Option<String> = None;
        let panel = key.to_string();
        let current = self
            .views
            .pane_with_key(key)
            .map(|p| self.slot_of(p).kind)
            .unwrap_or_default();
        // The common kinds first (LAY-73).
        for kind in [VIEW_3D, UV_VIEW, EMPTY] {
            if ui.add(Button::selectable(current == kind, kind)).clicked() {
                chosen = Some(line(
                    "operandi.panel.kind",
                    &[("kind", kind), ("panel", &panel)],
                ));
            }
        }
        ui.menu_button("All Panels", |ui| {
            for category in panel_registry::Category::ALL {
                let kinds: Vec<&panel_registry::PanelType> = panel_registry::REGISTRY
                    .iter()
                    .filter(|t| t.category == category)
                    .collect();
                let view_kinds: &[&str] = if category == panel_registry::Category::Views3d {
                    &[VIEW_3D, UV_VIEW]
                } else {
                    &[]
                };
                if kinds.is_empty() && view_kinds.is_empty() {
                    continue;
                }
                ui.menu_button(category.label(), |ui| {
                    for kind in view_kinds {
                        if ui.button(*kind).clicked() {
                            chosen = Some(line(
                                "operandi.panel.kind",
                                &[("kind", kind), ("panel", &panel)],
                            ));
                        }
                    }
                    for t in kinds {
                        // Provisional (LAY-Q11): side-column kinds stay there.
                        ui.gated(
                            Some("Opens in the side columns in this build"),
                            Button::new(t.name),
                        );
                    }
                });
            }
        });
        ui.separator();
        let groups: Vec<String> = self
            .layouts
            .pieces
            .iter()
            .filter(|p| p.group)
            .map(|p| p.name.clone())
            .collect();
        let singles: Vec<(String, String)> = self
            .layouts
            .pieces
            .iter()
            .filter(|p| !p.group)
            .map(|p| (p.category.clone(), p.name.clone()))
            .collect();
        ui.gated_ui(
            crate::unavailable::unless(
                !groups.is_empty(),
                "Save a group with Save Viewport Group As first",
            ),
            |ui| {
                ui.menu_button("Viewport Group", |ui| {
                    for name in &groups {
                        if ui.button(name).clicked() {
                            chosen = Some(line(
                                "operandi.viewport.use",
                                &[("name", name), ("panel", &panel)],
                            ));
                        }
                    }
                });
            },
        );
        ui.gated_ui(
            crate::unavailable::unless(!singles.is_empty(), "Save one with Save Viewport As first"),
            |ui| {
                ui.menu_button("Saved Viewports", |ui| {
                    let mut last = None;
                    for (category, name) in &singles {
                        if last != Some(category) && !category.is_empty() {
                            ui.label(
                                RichText::new(category)
                                    .small()
                                    .color(theme::disabled_text()),
                            );
                        }
                        last = Some(category);
                        if ui.button(name).clicked() {
                            chosen = Some(line(
                                "operandi.viewport.use",
                                &[("name", name), ("panel", &panel)],
                            ));
                        }
                    }
                });
            },
        );
        ui.gated(
            Some("Tab groups in the view area come in a later stage"),
            Button::new("Tabbed Viewport"),
        );
        ui.separator();
        if ui.button("Save Viewport As…").clicked() {
            self.layouts.dialog = Some(Dialog::SavePiece {
                name: String::new(),
                category: "User".into(),
                group: false,
            });
            ui.close();
        }
        if ui.button("Save Viewport Group As…").clicked() {
            self.layouts.dialog = Some(Dialog::SavePiece {
                name: String::new(),
                category: "User".into(),
                group: true,
            });
            ui.close();
        }
        let showing = self
            .layouts
            .piece_of
            .get(&key)
            .cloned()
            .or_else(|| self.layouts.group_piece.clone());
        let why =
            crate::unavailable::unless(showing.is_some(), "This panel shows no saved viewport");
        if ui
            .gated(
                why,
                Button::new(match &showing {
                    Some(n) => format!("Save Viewport ({n})"),
                    None => "Save Viewport".into(),
                }),
            )
            .clicked()
        {
            chosen = Some("operandi.viewport.save".into());
        }
        if ui.gated(why, Button::new("Revert Viewport")).clicked() {
            chosen = Some("operandi.viewport.revert".into());
        }
        ui.gated_ui(
            crate::unavailable::unless(
                self.layouts.pieces.iter().any(|p| !p.shipped),
                "No saved viewports of yours",
            ),
            |ui| {
                ui.menu_button("Delete Saved Viewport", |ui| {
                    let names: Vec<String> = self
                        .layouts
                        .pieces
                        .iter()
                        .filter(|p| !p.shipped)
                        .map(|p| p.name.clone())
                        .collect();
                    for name in names {
                        if ui.button(&name).clicked() {
                            chosen = Some(line("operandi.viewport.delete", &[("name", &name)]));
                        }
                    }
                });
            },
        );
        ui.separator();
        // That panel's own settings.
        let is_3d = matches!(self.views.pane_with_key(key), Some(Pane::Viewport(_)));
        if ui
            .gated(
                crate::unavailable::unless(is_3d, "This panel has no settings of its own"),
                Button::new("Viewport Properties…"),
            )
            .clicked()
        {
            chosen = Some("viewport properties".into());
        }
        ui.menu_button("Layout", |ui| self.layout_menu_items(ui, &mut chosen));
        if let Some(c) = chosen {
            // Activating the slot first, so lines without panel: act on it.
            if is_3d || matches!(self.views.pane_with_key(key), Some(Pane::Uv(_))) {
                self.layouts
                    .requests
                    .push(format!("operandi.layout.focus {key}"));
            }
            self.layouts.requests.push(c);
            ui.close();
        }
    }

    /// The Layout menu's own entries, for the content arrow's Layout
    /// sub-menu.
    fn layout_menu_items(&mut self, ui: &mut Ui, chosen: &mut Option<String>) {
        if ui.button("New Layout…").clicked() {
            self.layouts.dialog = Some(Dialog::New {
                name: String::new(),
                copy: false,
            });
            ui.close();
        }
        if ui.button("Save Layout").clicked() {
            *chosen = Some("operandi.layout.save".into());
        }
        if ui.button("Save Layout As…").clicked() {
            self.layouts.dialog = Some(Dialog::SaveAs {
                name: String::new(),
            });
            ui.close();
        }
        if ui.button("Clear Layout").clicked() {
            *chosen = Some("operandi.layout.clear".into());
        }
        if ui.button("Recent Layouts…").clicked() {
            *chosen = Some("operandi.layout.switcher".into());
        }
    }

    /// Run what panel headers asked for while the views drew.
    pub(super) fn layout_requests(&mut self) {
        for request in std::mem::take(&mut self.layouts.requests) {
            if let Some(key) = request
                .strip_prefix("operandi.layout.focus ")
                .and_then(|k| k.parse::<u32>().ok())
            {
                if self.views.view_pane(key).is_some() {
                    self.activate_view(key);
                }
                continue;
            }
            self.execute(&request);
        }
    }

    /// The current layout's file written now, whatever was last written.
    pub(super) fn write_current_layout_now(&mut self) {
        self.note_torn_windows();
        self.layouts_saved_clear();
        self.persist_layout_file();
    }
}

/// A frame with every slot's piece name taken off.
fn strip_pieces(mut frame: Frame) -> Frame {
    fn walk(f: &mut Frame) {
        match f {
            Frame::Split { children, .. } => children.iter_mut().for_each(|p| walk(&mut p.frame)),
            Frame::Tabs { tabs, .. } => tabs.iter_mut().for_each(|s| s.piece = None),
            Frame::Grid { children, .. } | Frame::DockTabs { children, .. } => {
                children.iter_mut().for_each(walk)
            }
            Frame::ViewArea { frame } | Frame::DockPanel { frame, .. } => walk(frame),
            Frame::Panel(s) | Frame::DockTab(s) => s.piece = None,
        }
    }
    walk(&mut frame);
    frame
}

/// Put `piece`'s saved version into a layout file wherever it shows: a
/// group piece replaces the view area of a layout whose `group_piece` is
/// it; a single piece replaces each slot tagged with it. Whether anything
/// changed.
fn apply_piece_to_doc(doc: &mut WindowLayout, piece: &Piece) -> bool {
    if piece.group {
        if doc.group_piece.as_deref() != Some(piece.name.as_str()) {
            return false;
        }
        return replace_views(&mut doc.frame, &piece.frame);
    }
    let Frame::Panel(saved) = &piece.frame else {
        return false;
    };
    fn walk(f: &mut Frame, name: &str, saved: &Slot, changed: &mut bool) {
        let fix = |s: &mut Slot, changed: &mut bool| {
            if s.piece.as_deref() == Some(name) {
                let new = Slot {
                    piece: Some(name.to_owned()),
                    maximized: s.maximized,
                    ..saved.clone()
                };
                if *s != new {
                    *s = new;
                    *changed = true;
                }
            }
        };
        match f {
            Frame::Split { children, .. } => children
                .iter_mut()
                .for_each(|p| walk(&mut p.frame, name, saved, changed)),
            Frame::Tabs { tabs, .. } => tabs.iter_mut().for_each(|s| fix(s, changed)),
            Frame::Grid { children, .. } | Frame::DockTabs { children, .. } => children
                .iter_mut()
                .for_each(|c| walk(c, name, saved, changed)),
            Frame::ViewArea { frame } => walk(frame, name, saved, changed),
            Frame::DockPanel { .. } => {}
            Frame::Panel(s) | Frame::DockTab(s) => fix(s, changed),
        }
    }
    let mut changed = false;
    walk(&mut doc.frame, &piece.name, saved, &mut changed);
    for w in &mut doc.windows {
        walk(&mut w.frame, &piece.name, saved, &mut changed);
    }
    changed
}
