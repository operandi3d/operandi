//! Control icons and Pick in the Form Editor (UIC-314, UIC-212's Pick).
//!
//! - A control's icon is a built-in icon by name (`icon`) or a custom
//!   picture file (`iconFile`, PNG, JPEG or SVG), or none. A chosen file is
//!   copied into the data folder's `Layouts/Icons`, so the form keeps
//!   working when the original moves; a file that is gone draws a
//!   placeholder, never an error.
//! - Export Pie embeds the custom files (`PieFile::icons`) and Import Pie
//!   writes them back into the importer's icons folder; Save Form copies
//!   them into a folder beside the saved form.
//! - Pick (`operandi.form.pick`): the next click on a toolbox, toolbar or
//!   menu button doesn't run it but puts its line and icon (and its label,
//!   when the control has none) on the control. Esc or a right-click stops;
//!   the Form Editor's own lines still run. The pointer is the system's
//!   pointing hand while picking, as for Pick to Add.
use super::*;
use crate::form::command_form::{Control, Form, Style};
use crate::workbench::{bound, index_arg, word};
use std::path::Path;

/// The icons folder inside the data folder's Layouts folder.
const ICONS_FOLDER: &str = "Icons";
/// How an exported pie names an icon file it carries.
pub(crate) const EMBEDDED: &str = "embedded:";

impl Operandi {
    /// Where custom icons are kept (UIC-314).
    pub(crate) fn icons_folder(&self) -> PathBuf {
        self.environment
            .paths()
            .data_root()
            .join("Layouts")
            .join(ICONS_FOLDER)
    }

    /// Copy `bytes` into the icons folder as `name` (or `name-2`, ... when
    /// a different file has that name already); the copy's path.
    pub(crate) fn adopt_icon_bytes(
        &self,
        name: &str,
        bytes: &[u8],
    ) -> std::result::Result<PathBuf, String> {
        let folder = self.icons_folder();
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let path = Path::new(name);
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "icon".into());
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        for n in 1.. {
            let file = if n == 1 {
                format!("{stem}{ext}")
            } else {
                format!("{stem}-{n}{ext}")
            };
            let at = folder.join(file);
            match std::fs::read(&at) {
                Ok(existing) if existing == bytes => return Ok(at),
                Ok(_) => continue,
                Err(_) => {
                    std::fs::write(&at, bytes).map_err(|e| e.to_string())?;
                    return Ok(at);
                }
            }
        }
        unreachable!("some name is free")
    }

    /// A chosen icon file, copied into the icons folder (a file already
    /// there stays where it is).
    fn adopt_icon(&self, file: &Path) -> std::result::Result<PathBuf, String> {
        if file.starts_with(self.icons_folder()) && file.exists() {
            return Ok(file.to_path_buf());
        }
        let bytes =
            std::fs::read(file).map_err(|e| format!("Couldn't read {}: {e}", file.display()))?;
        if crate::workbench::custom_icon_picture(file).is_none() {
            return Err(format!(
                "{} isn't a PNG, JPEG or SVG picture",
                file.display()
            ));
        }
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "icon.png".into());
        self.adopt_icon_bytes(&name, &bytes)
    }

    /// Pick is on (the Form Editor's, not Pick to Add).
    pub(crate) fn form_picking(&self) -> bool {
        self.pie_edit.pick.is_some()
    }

    /// Start of a frame: Esc or a right-click stops Pick.
    pub(crate) fn form_pick_frame_begin(&mut self, ctx: &Context) {
        if self.pie_edit.pick.is_none() {
            return;
        }
        let right = ctx.input(|i| i.pointer.secondary_clicked());
        if right || ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.pie_edit.pick = None;
            self.status = "Pick is off".into();
        }
    }

    /// A button's line clicked while Pick is on: it fills the control
    /// instead of running. True when it was taken.
    pub(crate) fn form_pick_take(&mut self, line: &str) -> bool {
        let Some((form, index)) = self.pie_edit.pick.clone() else {
            return false;
        };
        if !self.workbench_clicked_now() {
            return false;
        }
        let line = line.trim();
        let lower = line.to_ascii_lowercase();
        // The Form Editor's own buttons still work while picking.
        if ["operandi.form.", "operandi.pie.", "operandi.workbench."]
            .iter()
            .any(|p| lower.starts_with(p))
        {
            return false;
        }
        self.workbench.click_now = false;
        self.pie_edit.pick = None;
        match self.fill_from_pick(&form, index, line) {
            Ok(status) => self.status = status,
            Err(e) => {
                self.last_failure = Some(e.clone());
                self.status = e;
            }
        }
        true
    }

    /// The label and icon a button running `line` wears: its control in
    /// any form, else its tool's.
    fn picked_face(&self, line: &str) -> (String, Option<String>, Option<String>) {
        let control = self
            .forms
            .ids()
            .into_iter()
            .filter_map(|id| self.forms.get(id))
            .flat_map(|f| f.controls.iter())
            .find(|c| c.command.as_deref() == Some(line))
            .cloned()
            .unwrap_or_default();
        let tool = crate::tool_info::by_name(&line.to_lowercase());
        let icon = control
            .icon
            .clone()
            .or_else(|| tool.map(|t| crate::tool_info::info(t).icon.slug().to_owned()))
            .filter(|s| !s.is_empty());
        let label = if !control.label.is_empty() {
            control.label.clone()
        } else {
            tool.map(|t| t.label().to_owned()).unwrap_or_default()
        };
        (label, icon, control.icon_file.clone())
    }

    /// Pick's edit: the control gets the line, the button's icon, and its
    /// label when it has none. One edit (Undo Edit takes it back).
    fn fill_from_pick(
        &mut self,
        form: &str,
        index: usize,
        line: &str,
    ) -> std::result::Result<String, String> {
        let (label, icon, icon_file) = self.picked_face(line);
        let pie = self.forms.get(form).is_some_and(|f| f.style == Style::Pie);
        self.form_edit(form, |f| {
            if pie {
                crate::pie_forms::padded(f);
            }
            let c = f
                .controls
                .get_mut(index)
                .ok_or_else(|| format!("{form} has no control {index}"))?;
            if c.command.is_none() {
                if !pie {
                    return Err(format!(
                        "{form} control {index} isn't a button; only buttons run a line"
                    ));
                }
                // An empty slot or a sub-pie becomes a command slot.
                *c = Control::default();
            }
            c.command = Some(line.to_owned());
            if c.label.is_empty() {
                c.label = label.clone();
            }
            if icon.is_some() || icon_file.is_some() {
                c.icon = icon.clone();
                c.icon_file = icon_file.clone();
            }
            Ok(format!("Picked {line} for {form} control {index}"))
        })
    }

    /// `operandi.form.pick`, `operandi.form.setIcon` and
    /// `operandi.form.chooseIcon`.
    pub(crate) fn form_icon_command(&mut self, name: &str, rest: &str) {
        match self.form_icon_run(&name.to_ascii_lowercase(), rest) {
            Ok(status) => self.status = status,
            Err(e) => {
                self.last_failure = Some(e.clone());
                self.status = e;
            }
        }
    }

    fn form_icon_run(&mut self, name: &str, rest: &str) -> std::result::Result<String, String> {
        match name {
            "operandi.form.pick" => {
                let a = bound(name, rest, &["index", "form", "on"])?;
                let index = index_arg(a[0].as_deref(), "index")?;
                let form = self.icon_form(a[1].as_deref(), index)?;
                let target = (form.clone(), index);
                let on = match a[2].as_deref() {
                    Some(v) => operandi_core::command_args::boolean("on", v)?,
                    None => self.pie_edit.pick.as_ref() != Some(&target),
                };
                // The click that turned it on isn't the pick.
                self.workbench.click_now = false;
                self.pie_edit.pick = on.then_some(target);
                Ok(if on {
                    format!(
                        "Pick: click a button to put its line on {form} control {index} (Esc stops)"
                    )
                } else {
                    "Pick is off".into()
                })
            }
            "operandi.form.seticon" => {
                let a = bound(name, rest, &["index", "name", "file", "form"])?;
                let index = index_arg(a[0].as_deref(), "index")?;
                let form = self.icon_form(a[3].as_deref(), index)?;
                let slug = a[1]
                    .clone()
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty());
                let file = a[2]
                    .clone()
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty());
                let (icon, icon_file, what) = match (slug, file) {
                    (Some(_), Some(_)) => return Err("Give name: or file:, not both".into()),
                    (Some(slug), None) => {
                        if icons::Icon::from_slug(&slug).is_none() {
                            return Err(format!("No built-in icon called {slug}"));
                        }
                        (Some(slug.clone()), None, slug)
                    }
                    (None, Some(file)) => {
                        let copy = self.adopt_icon(Path::new(&file))?;
                        let shown = copy.display().to_string();
                        (None, Some(shown.clone()), shown)
                    }
                    (None, None) => (None, None, "no icon".to_owned()),
                };
                self.form_edit(&form, |f| {
                    let c = f
                        .controls
                        .get_mut(index)
                        .ok_or_else(|| format!("{form} has no control {index}"))?;
                    c.icon = icon;
                    c.icon_file = icon_file;
                    Ok(format!("{form} control {index}: {what}"))
                })
            }
            "operandi.form.chooseicon" => {
                let a = bound(name, rest, &["index", "form"])?;
                let index = index_arg(a[0].as_deref(), "index")?;
                let form = self.icon_form(a[1].as_deref(), index)?;
                self.pie_edit.icon_target = Some((form, index));
                let folder = crate::backdrop_ui::dirs_home().unwrap_or_default();
                if !self.choose(
                    crate::file_chooser::Choose::IconFile { folder },
                    crate::file_chooser::Purpose::FormIcon,
                ) {
                    self.pie_edit.icon_target = None;
                    return Err(
                        "No file panel is available; use operandi.form.setIcon file:<path>".into(),
                    );
                }
                Ok(if self.pie_edit.icon_target.is_some() {
                    "Choose a PNG, JPEG or SVG for the icon".into()
                } else {
                    self.status.clone()
                })
            }
            other => Err(format!("{other} isn't an icon command")),
        }
    }

    /// The form an icon or Pick line acts on (`given`, else the Form
    /// Editor's selection), which must have control `index` (a pie always
    /// has its eight slots).
    fn icon_form(&self, given: Option<&str>, index: usize) -> std::result::Result<String, String> {
        let id = given
            .map(str::to_owned)
            .or_else(|| self.form_editor.selected.as_ref().map(|(f, _)| f.clone()))
            .ok_or("Give form: (or select a form in the Form Editor)")?;
        let form = self
            .forms
            .get(&id)
            .ok_or_else(|| format!("No form called {id}"))?;
        let slots = if form.style == Style::Pie {
            crate::pie_menu::SLOTS.max(form.controls.len())
        } else {
            form.controls.len()
        };
        if index >= slots {
            return Err(format!("{id} has no control {index}"));
        }
        Ok(id)
    }

    /// Choose File… answered: that picture becomes the control's icon.
    pub(crate) fn form_icon_chosen(&mut self, path: PathBuf) {
        if let Some((form, index)) = self.pie_edit.icon_target.take() {
            self.execute(&format!(
                "operandi.form.setIcon index:{index} file:{} form:{form}",
                word(&path.to_string_lossy())
            ));
        }
    }
}

/// Carry the forms' custom icon files in an export: each readable file
/// goes into the map by name, its control naming it `embedded:<name>`;
/// a missing file keeps its path (it shows a placeholder either way).
pub(crate) fn embed_icons(forms: &mut [Form]) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let mut by_path: BTreeMap<String, String> = BTreeMap::new();
    for c in forms.iter_mut().flat_map(|f| f.controls.iter_mut()) {
        let Some(path) = c.icon_file.clone() else {
            continue;
        };
        if let Some(name) = by_path.get(&path) {
            c.icon_file = Some(format!("{EMBEDDED}{name}"));
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let file = Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "icon.png".into());
        let mut name = file.clone();
        let mut n = 2;
        while out.contains_key(&name) {
            name = format!("{n}-{file}");
            n += 1;
        }
        out.insert(name.clone(), base64_encode(&bytes));
        by_path.insert(path, name.clone());
        c.icon_file = Some(format!("{EMBEDDED}{name}"));
    }
    out
}

/// The icon files carried by an import, written into this user's icons
/// folder; controls naming them point at the copies. A name the file
/// doesn't carry loses its icon file.
pub(crate) fn unpack_icons(
    a: &Operandi,
    forms: &mut [Form],
    icons: &BTreeMap<String, String>,
) -> std::result::Result<(), String> {
    let mut written: BTreeMap<String, String> = BTreeMap::new();
    for c in forms.iter_mut().flat_map(|f| f.controls.iter_mut()) {
        let Some(name) = c
            .icon_file
            .as_deref()
            .and_then(|p| p.strip_prefix(EMBEDDED))
            .map(str::to_owned)
        else {
            continue;
        };
        if let Some(path) = written.get(&name) {
            c.icon_file = Some(path.clone());
            continue;
        }
        c.icon_file = match icons.get(&name) {
            Some(data) => {
                let bytes = base64_decode(data).ok_or_else(|| format!("icon {name} is damaged"))?;
                let path = a.adopt_icon_bytes(&name, &bytes)?.display().to_string();
                written.insert(name, path.clone());
                Some(path)
            }
            None => None,
        };
    }
    Ok(())
}

/// Save Form: copy the form's custom icon files into `<stem> icons/`
/// beside the saved file, the saved form naming the copies there.
pub(crate) fn copy_icons_beside(form: &mut Form, saved: &Path) -> std::result::Result<(), String> {
    let stem = saved
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "form".into());
    let folder = saved
        .parent()
        .unwrap_or(Path::new(""))
        .join(format!("{stem} icons"));
    for c in &mut form.controls {
        let Some(path) = c.icon_file.clone() else {
            continue;
        };
        let Some(name) = Path::new(&path).file_name() else {
            continue;
        };
        if Path::new(&path).exists() {
            std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
            std::fs::copy(&path, folder.join(name)).map_err(|e| e.to_string())?;
            c.icon_file = Some(folder.join(name).display().to_string());
        }
    }
    Ok(())
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding (no line breaks).
pub(crate) fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(B64[((n >> shift) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The bytes of standard base64 text; `None` if it isn't.
pub(crate) fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0;
    for ch in text.bytes().filter(|c| !c.is_ascii_whitespace()) {
        if ch == b'=' {
            break;
        }
        let v = B64.iter().position(|&b| b == ch)? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// The icon and Pick commands' handlers (UIC-160).
pub(crate) fn register(r: &mut crate::command_handlers::Commands) {
    crate::command_handlers::on_words(
        r,
        &[
            "operandi.form.pick",
            "operandi.form.setIcon",
            "operandi.form.chooseIcon",
        ],
        |a, c| a.form_icon_command(c.entry.name, c.rest),
    );
    r.answer("operandi.form.pick", |a, c| {
        let lit = a.pie_edit.pick.as_ref().is_some_and(|(f, i)| {
            let words = bound(c.entry.name, c.rest, &["index", "form"]).unwrap_or_default();
            words
                .first()
                .cloned()
                .flatten()
                .and_then(|w| w.parse().ok())
                == Some(*i)
                && words.get(1).cloned().flatten().is_none_or(|w| w == *f)
        });
        operandi_core::command_runtime::CommandState::lit(lit)
    })
    .unwrap_or_else(|e| panic!("{e}"));
}
