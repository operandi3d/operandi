//! Menus as forms (follows layouts stage 7, LAY-Q66; UIC-10, UIC-210,
//! UIC-213): the whole menu bar is data the Form Editor can change.
//!
//! - The bar is the form `menu.bar`; each of its sub-forms is one menu,
//!   titled with the sub-form's label (`menu.file`, `menu.edit`, …). A
//!   user's copy of any of them wins over the shipped one (LAY-87).
//! - In a menu form, a `command` control is an entry running its line; its
//!   greying, tick and (with a `{title}` label) its title come from the
//!   line's own answer (UIC-223). A divider is a separator. A sub-form is
//!   a submenu, titled with the `form` control's label, else the sub-form's.
//! - A `list` control is a live list: entries filled while the app runs
//!   from a named source in [`LISTS`] (open scenes, recent projects,
//!   layouts, palettes). Each source gives every entry its label, the line
//!   it runs and its tick and greying. The Form Editor shows it as one
//!   item ("Recent Projects (live)") that moves or deletes as a whole.
//! - Titles with run-time text: `{title}` in a label is the entry's own
//!   line's answered title (Undo's "Undo Cube"), `{title:<line>}` another
//!   line's, the way tooltips take `{shortcut}`. A submenu's label may hold
//!   `{title:<line>}` too.
//! - Every entry runs a registry line through `execute`, as a menu line
//!   (UIC-224), so every menu click shows in the Command History (UIC-160).
use super::*;
use crate::app_menu::{Action, Entry, Menu, line_action};
use crate::form::command_form::{Control, End, Form, FormRef};
use operandi_core::command_runtime::CommandState;

/// The menu bar's form.
pub(crate) const MENU_BAR: &str = "menu.bar";

/// One entry of a live list: its label, the line it runs, and its state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ListEntry {
    pub label: String,
    pub line: String,
    /// Why it is greyed; `None` when it can run.
    pub why: Option<String>,
    /// Ticked.
    pub on: bool,
}

/// Gives a live list's entries now.
pub(crate) type Provider = fn(&Operandi) -> Vec<ListEntry>;

/// The live lists menu forms can hold: source id, its name in the Form
/// Editor, and what fills it. Adding a list is adding a row here.
pub(crate) const LISTS: &[(&str, &str, Provider)] = &[
    ("scenes", "Open Scenes", scenes),
    ("recentProjects", "Recent Projects", recent_projects),
    ("layouts", "Layouts", layouts),
    ("panels", "Panels", panels),
    ("palettes", "Windows/Palettes", palettes),
];

/// A live list's name for the Form Editor ("Recent Projects"); an unknown
/// source shows its id.
pub(crate) fn list_title(source: &str) -> String {
    LISTS
        .iter()
        .find(|(id, _, _)| *id == source)
        .map_or_else(|| source.to_owned(), |(_, title, _)| (*title).to_owned())
}

/// File > Current: every open scene, the current one ticked.
fn scenes(a: &Operandi) -> Vec<ListEntry> {
    a.scene_menu_entries()
        .into_iter()
        .map(|(i, label, on)| ListEntry {
            label,
            line: format!("operandi.scene.current index:{i}"),
            why: None,
            on,
        })
        .collect()
}

/// File > Recent Projects: the project folders, the open one ticked; one
/// greyed entry when there are none.
fn recent_projects(a: &Operandi) -> Vec<ListEntry> {
    if a.recent_projects.is_empty() {
        return vec![ListEntry {
            label: "No Recent Projects".into(),
            line: "operandi.project.openRecent".into(),
            why: Some("No recent projects yet".into()),
            on: false,
        }];
    }
    a.recent_projects
        .iter()
        .enumerate()
        .map(|(i, root)| ListEntry {
            label: root.display().to_string(),
            line: format!("operandi.project.openRecent index:{i}"),
            why: None,
            on: a.project.as_ref().is_some_and(|p| p.root() == root),
        })
        .collect()
}

/// Layout > Layouts: every layout in the switcher's order (LAY-31), the
/// current one ticked; each runs `layout.restore` (LAY-30).
fn layouts(a: &Operandi) -> Vec<ListEntry> {
    a.layouts
        .names()
        .into_iter()
        .map(|name| ListEntry {
            line: Operandi::restore_line(&name),
            on: name == a.layouts.current,
            label: name,
            why: None,
        })
        .collect()
}

/// UIC-10 / UIC-157 / UIC-220: Window > Panels is a registry-backed list of commands.
fn panels(a: &Operandi) -> Vec<ListEntry> {
    crate::panel_registry::REGISTRY
        .iter()
        .map(|panel| {
            // Node View opens on the selection's Material Group, with its
            // key (node_view_open.rs): the one Window ▸ Node View entry.
            let node_view = panel.kind == crate::panel_registry::PanelKind::NodeView;
            ListEntry {
                label: if node_view {
                    format!("{}{}", panel.name, a.shortcut("Node View"))
                } else {
                    panel.name.into()
                },
                line: if node_view {
                    "operandi.window.nodeView".into()
                } else {
                    format!("operandi.panel.show kind:{{{}}}", panel.name)
                },
                why: None,
                on: a.registry_panel_open(panel.kind),
            }
        })
        .collect()
}

/// Layout > Windows/Palettes: every palette, ticked while open (LAY-55);
/// each runs `layout.window hash:<id>`.
fn palettes(a: &Operandi) -> Vec<ListEntry> {
    a.palettes
        .list
        .iter()
        .map(|p| {
            let shortcut = match p.hash.as_str() {
                crate::palettes::PRESET_BROWSER => a.shortcut(crate::preset_browser::WINDOW_ACTION),
                crate::palettes::UV_EDITOR => a.shortcut("Open UV Editor"),
                _ => String::new(),
            };
            ListEntry {
                label: format!("{}{shortcut}", p.name),
                line: format!("layout.window hash:{}", p.hash),
                why: None,
                on: a.palette_open(&p.hash),
            }
        })
        .collect()
}

/// Makes a menu entry: action, title, why greyed, ticked, native key. The
/// menu's own maker greys everything while a dialog or drag is open.
pub(crate) type Maker<'a> = &'a dyn Fn(Action, String, Option<String>, bool, String) -> Entry;

/// What a walk over the bar's forms carries: the entry maker and the
/// forms that sit in other forms' slots.
struct Walk<'a> {
    item: Maker<'a>,
    slots: &'a [&'a Form],
}

/// Sub-forms nest at most this deep, so a form holding itself stops.
const MAX_DEPTH: usize = 8;

impl Operandi {
    /// The menu bar from `menu.bar` (menus as forms). A menu the bar names
    /// that has no form is left out.
    pub(super) fn menus_from_forms(&self, item: Maker<'_>) -> Vec<Menu> {
        let Some(bar) = self.forms.get(MENU_BAR) else {
            return Vec::new();
        };
        // The forms that put themselves in another's slot (LAY-84), found
        // once: the bar is built every frame.
        let slots: Vec<&Form> = self
            .forms
            .ids()
            .into_iter()
            .filter_map(|id| self.forms.get(id))
            .filter(|f| f.slot.is_some())
            .collect();
        let walk = Walk {
            item,
            slots: &slots,
        };
        let mut out = Vec::new();
        for c in self.menu_controls(&walk, bar, 0) {
            let Some(sub) = &c.form else {
                continue;
            };
            let form = match sub {
                FormRef::Id(id) => self.forms.get(id),
                FormRef::Inline(inner) => Some(inner.as_ref()),
            };
            if let Some(form) = form.filter(|f| self.form_shows(f)) {
                out.push(Menu {
                    id: form.id.clone(),
                    title: self.submenu_title(&c, form),
                    entries: self.menu_entries(&walk, form, 1),
                });
            }
        }
        out
    }

    /// A menu's entries from form `form` (layouts stage 7, UIC-10 /
    /// UIC-210): command entries, separators, submenus and live lists.
    fn menu_entries(&self, walk: &Walk<'_>, form: &Form, depth: usize) -> Vec<Entry> {
        let item = walk.item;
        let mut out = Vec::new();
        for c in self.menu_controls(walk, form, depth) {
            if let Some(line) = c.command.as_deref().map(str::trim) {
                let state = self.line_state(line);
                let why = match &state {
                    Some(s) => s.unavailable.clone(),
                    None => Some(format!("{line} isn't a command")),
                };
                let key = c
                    .shortcut
                    .as_deref()
                    .map(|s| self.shortcut(s))
                    .unwrap_or_default();
                let label = if c.label.is_empty() {
                    line.to_owned()
                } else {
                    self.fill_title(&c.label, line, state.as_ref())
                };
                out.push(item(
                    line_action(line),
                    format!("{label}{key}"),
                    why,
                    state.is_some_and(|s| s.on),
                    c.key_equivalent.clone().unwrap_or_default(),
                ));
            } else if c.divider.is_some() {
                out.push(Entry::Separator);
            } else if let Some(source) = &c.list {
                match LISTS.iter().find(|(id, _, _)| id == source) {
                    Some((_, _, provider)) => {
                        for e in provider(self) {
                            out.push(item(
                                line_action(&e.line),
                                e.label,
                                e.why,
                                e.on,
                                String::new(),
                            ));
                        }
                    }
                    None => out.push(item(
                        line_action(&format!("operandi.notBuilt {source}")),
                        format!("{source} (live)"),
                        Some(format!("No live list called {source}")),
                        false,
                        String::new(),
                    )),
                }
            } else if let Some(sub) = &c.form {
                if depth >= MAX_DEPTH {
                    continue;
                }
                let inner = match sub {
                    FormRef::Id(id) => match self.forms.get(id) {
                        Some(inner) => inner,
                        None => {
                            out.push(item(
                                line_action(&format!("operandi.form.select form:{id}")),
                                id.clone(),
                                Some(format!("No form called {id}")),
                                false,
                                String::new(),
                            ));
                            continue;
                        }
                    },
                    FormRef::Inline(inner) => inner.as_ref(),
                };
                if !self.form_shows(inner) {
                    continue;
                }
                out.push(Entry::Submenu(Menu {
                    id: inner.id.clone(),
                    title: self.submenu_title(&c, inner),
                    entries: self.menu_entries(walk, inner, depth + 1),
                }));
            }
        }
        out
    }

    /// A form's controls with its head and tail slots' controls placed
    /// around them (LAY-84), as `FormStore::slotted` orders them.
    fn menu_controls(&self, walk: &Walk<'_>, form: &Form, depth: usize) -> Vec<Control> {
        let slotted = |at: End| {
            let mut forms: Vec<&Form> = walk
                .slots
                .iter()
                .copied()
                .filter(|f| {
                    f.slot
                        .as_ref()
                        .is_some_and(|s| s.form == form.id && s.at == at)
                })
                .filter(|f| self.form_shows(f))
                .collect();
            forms.sort_by(|a, b| {
                let sort = |f: &Form| f.slot.as_ref().map_or(0, |s| s.sort);
                sort(a).cmp(&sort(b)).then_with(|| a.id.cmp(&b.id))
            });
            forms
        };
        let mut out = Vec::new();
        if depth < MAX_DEPTH {
            for head in slotted(End::Head) {
                out.extend(self.menu_controls(walk, head, depth + 1));
            }
        }
        out.extend(form.controls.iter().cloned());
        if depth < MAX_DEPTH {
            for tail in slotted(End::Tail) {
                out.extend(self.menu_controls(walk, tail, depth + 1));
            }
        }
        out
    }

    /// A form's show-only-when line is enabled, or it has none (LAY-85).
    fn form_shows(&self, form: &Form) -> bool {
        form.show_when
            .as_deref()
            .is_none_or(|line| self.line_enabled(line))
    }

    /// A submenu's title: the `form` control's label, else the form's,
    /// with its `{title:<line>}` placeholders filled.
    fn submenu_title(&self, c: &Control, form: &Form) -> String {
        let label = if c.label.is_empty() {
            &form.label
        } else {
            &c.label
        };
        let label = if label.is_empty() { &form.id } else { label };
        self.fill_title(label, "", None)
    }

    /// `text` with `{title}` (`line`'s answered title, from `state`) and
    /// `{title:<line>}` (another line's) filled. A line that answers no
    /// title shows the line itself.
    pub(super) fn fill_title(
        &self,
        text: &str,
        line: &str,
        state: Option<&CommandState>,
    ) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(at) = rest.find("{title") {
            out.push_str(&rest[..at]);
            let tail = &rest[at..];
            let Some(end) = tail.find('}') else {
                out.push_str(tail);
                return out;
            };
            let inside = &tail[1..end];
            match inside.strip_prefix("title") {
                Some("") => out.push_str(
                    &state
                        .and_then(|s| s.title.clone())
                        .unwrap_or_else(|| line.to_owned()),
                ),
                Some(other) if other.starts_with(':') => {
                    let other = other[1..].trim();
                    out.push_str(
                        &self
                            .line_state(other)
                            .and_then(|s| s.title)
                            .unwrap_or_else(|| other.to_owned()),
                    );
                }
                _ => out.push_str(&tail[..=end]),
            }
            rest = &tail[end + 1..];
        }
        out.push_str(rest);
        out
    }
}
