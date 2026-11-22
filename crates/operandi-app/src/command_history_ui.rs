//! The Command History panel (UIC-228, HIS-01..61): Undos, History,
//! Commands and Results tabs over the one-line command entry.
//!
//! In the Model layout it sits at the bottom of the right column,
//! collapsed to the command entry; dragging the line above the entry up
//! opens the tabs (HIS-01). It is also a panel kind (`PanelKind::
//! CommandHistory`) any panel group can show. Every list is the shared
//! tree list (UIC-130); each tab's F toggle is the list's search row
//! (UIC-40). The Scripts tab, Map Command to Key, Macro from History and
//! Fire Again wait for their stages and aren't shown.
use super::*;
use crate::command_history::Tab;
use crate::tree_list::{
    self, Cell, Column, DropAt, RowIcon, Selected, TreeEvent, TreeHooks, TreeRow, TreeSpec,
    TreeState,
};
use operandi_core::command_registry::{self, Area, COMMANDS};
use operandi_core::{CommandClass, CommandRecord, Source};
use std::collections::{BTreeSet, HashMap};

/// Height the panel opens to when its divider is double-clicked.
pub const OPEN_HEIGHT: f32 = 220.;
/// Below this the panel snaps shut when the divider is let go.
const SNAP_SHUT: f32 = 60.;
/// The command entry's row, as the right column has always reserved it.
pub const ENTRY_HEIGHT: f32 = 30.;
/// The divider's grab height above the entry (inside `ENTRY_HEIGHT` when
/// collapsed, so the column's layout is unchanged).
const DIVIDER: f32 = 4.;
/// The tab strip.
const TABS: f32 = 20.;

/// Rows of the Undos tab (HIS-10..14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UndoKey {
    /// `editor.undo[i]`.
    Step(usize),
    /// The bold "Drag to undo/redo" row (HIS-11).
    Marker,
    /// The `k`-th step Redo would bring back (0 = the next one).
    Redo(usize),
    /// Line `line` under step row `of` (its records and sub-commands).
    Line { of: (bool, usize), line: usize },
}

/// Rows of the Commands tab (HIS-30).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandKey {
    Area(usize),
    Entry(usize),
}

/// The panel's list states, one per tab.
#[derive(Debug, Default)]
pub struct HistoryTrees {
    pub undos: TreeState<UndoKey>,
    pub history: TreeState<usize>,
    pub commands: TreeState<CommandKey>,
    pub results: TreeState<usize>,
    /// Open Undos rows (by step) and Commands areas.
    pub open_steps: BTreeSet<(bool, usize)>,
    pub open_areas: BTreeSet<usize>,
    pub selected_undo: Option<UndoKey>,
    pub selected_command: Option<CommandKey>,
}

/// Value cells drawn as plain text (the tree list's custom cells).
struct TextCells<K> {
    text: HashMap<(K, usize), String>,
}

impl<K: Copy + Eq + std::hash::Hash> TreeHooks<K> for TextCells<K> {
    fn cell(&mut self, ui: &mut Ui, column: usize, key: K, rect: Rect) {
        if let Some(t) = self.text.get(&(key, column)) {
            // Cut-off text shows in full on hover (the list's last column
            // takes the spare width, TreeSpec::last_fills).
            tree_list::value_text(
                ui,
                ui.id().with(("cell", key, column)),
                rect,
                t,
                FontId::proportional(11.),
                theme::weak_text(),
            );
        }
    }
}

/// The class letter before a row (a tag until the icon generator has
/// recipes for the classes; HIS-10, CH-Q05).
fn class_letter(class: CommandClass, undone: bool) -> RowIcon {
    RowIcon::Letter(match class {
        _ if undone => "R",
        CommandClass::Undoable => "U",
        CommandClass::Ui => "I",
        CommandClass::SideEffect => "S",
        CommandClass::Quiet => "Q",
    })
}

fn class_tip(class: CommandClass, undone: bool) -> &'static str {
    match class {
        _ if undone => "Redoable: undone, kept until a new edit throws it away",
        CommandClass::Undoable => "Undoable",
        CommandClass::Ui => "UI: changes only the interface",
        CommandClass::SideEffect => "Side effect: acts outside the scene, can't be undone",
        CommandClass::Quiet => "Quiet: plumbing and queries",
    }
}

/// The help line of the command a recorded line names.
fn help_of(line: &str) -> &'static str {
    command_registry::find_line(line).map_or("", |(e, _)| e.help)
}

fn source_label(source: Source) -> &'static str {
    source.label()
}

impl Operandi {
    /// The right column's bottom: the divider, the open panel (when
    /// dragged open) and the command entry (HIS-01, HIS-02).
    pub(super) fn command_history_dock(&mut self, ui: &mut Ui) {
        let width = ui.available_width();
        let (divider, response) =
            ui.allocate_exact_size(vec2(width, DIVIDER), Sense::click_and_drag());
        if self.history.open_height > 0. {
            ui.painter().hline(
                divider.x_range(),
                divider.center().y,
                Stroke::new(1.0f32, theme::border()),
            );
        } else if response.hovered() || response.dragged() {
            ui.painter().hline(
                divider.x_range(),
                divider.center().y,
                Stroke::new(1.0f32, theme::hover_outline()),
            );
        }
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
        }
        let response = response.on_hover_text(
            "Command History: drag up to open its tabs (Undos, History, Commands, Results); \
             double-click opens or closes it",
        );
        let max = (ui.ctx().content_rect().height() * 0.7).max(OPEN_HEIGHT);
        if response.dragged() {
            self.history.open_height =
                (self.history.open_height - response.drag_delta().y).clamp(0., max);
        }
        if response.drag_stopped() && self.history.open_height < SNAP_SHUT {
            self.history.open_height = 0.;
        }
        if response.double_clicked() {
            self.history.open_height = if self.history.open_height > 0. {
                0.
            } else {
                OPEN_HEIGHT
            };
        }
        if self.history.open_height > 0. {
            let (rect, _) =
                ui.allocate_exact_size(vec2(width, self.history.open_height), Sense::hover());
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            child.set_clip_rect(rect);
            self.command_history_tabs(&mut child);
        }
        self.command_entry(ui);
    }

    /// The space the dock takes under the column's panels.
    pub(super) fn command_history_reserved(&self) -> f32 {
        ENTRY_HEIGHT + self.history.open_height
    }

    /// The panel as a panel group's tab: tabs above, entry at the bottom.
    pub(super) fn command_history_panel(&mut self, ui: &mut Ui, rect: Rect) {
        let lists = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.max.y - ENTRY_HEIGHT));
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(lists)
                .layout(Layout::top_down(Align::Min)),
        );
        child.set_clip_rect(lists);
        self.command_history_tabs(&mut child);
        let entry = Rect::from_min_max(pos2(rect.min.x, lists.max.y), rect.max);
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(entry)
                .layout(Layout::top_down(Align::Min)),
        );
        self.command_entry(&mut child);
    }

    /// The tab strip and the front tab's list.
    fn command_history_tabs(&mut self, ui: &mut Ui) {
        let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), TABS), Sense::hover());
        let mut row = ui.new_child(
            UiBuilder::new()
                .max_rect(strip)
                .layout(Layout::left_to_right(Align::Center)),
        );
        row.spacing_mut().item_spacing.x = 2.;
        for tab in Tab::ALL {
            let front = self.history.tab == tab;
            if row
                .add(Button::selectable(
                    front,
                    RichText::new(tab.label()).size(11.),
                ))
                .clicked()
            {
                self.history.tab = tab;
            }
        }
        let mut trees = std::mem::take(&mut self.history_trees);
        match self.history.tab {
            Tab::Undos => self.undos_tab(ui, &mut trees),
            Tab::History => self.history_tab(ui, &mut trees),
            Tab::Commands => self.commands_tab(ui, &mut trees),
            Tab::Results => self.results_tab(ui, &mut trees),
        }
        self.history_trees = trees;
    }

    // ------------------------------------------------------------- Undos

    /// The Undos tab: one row per undo step, oldest first, the drag row,
    /// then the undone steps greyed (HIS-10..14).
    fn undos_tab(&mut self, ui: &mut Ui, trees: &mut HistoryTrees) {
        let spec = TreeSpec {
            id: "command-history-undos",
            columns: vec![
                Column::name(),
                Column::value(
                    "Fired name and arguments",
                    170.,
                    "The line that made the step",
                ),
                Column::value("Description", 160., "What the command does"),
            ],
            own_search: false,
            last_fills: true,
        };
        tree_list::tool_row(ui, &mut trees.undos, true, |_, _| {});
        let mut rows = Vec::new();
        let mut cells = TextCells {
            text: HashMap::new(),
        };
        let steps: Vec<(bool, usize, &operandi_core::Checkpoint)> = self
            .editor
            .undo
            .iter()
            .enumerate()
            .map(|(i, c)| (false, i, &**c))
            .chain(
                self.editor
                    .redo
                    .iter()
                    .rev()
                    .enumerate()
                    .map(|(k, c)| (true, k, &**c)),
            )
            .collect();
        let mut marker_done = false;
        for (undone, index, step) in steps {
            if undone && !marker_done {
                rows.push(marker_row(&spec));
                marker_done = true;
            }
            let key = if undone {
                UndoKey::Redo(index)
            } else {
                UndoKey::Step(index)
            };
            let mut row = TreeRow::new(key, 0, step.label.clone(), spec.columns.len());
            let class = step
                .records
                .0
                .first()
                .map_or(CommandClass::Undoable, |r| r.class);
            row.icon = class_letter(class, undone);
            row.style.dimmed = undone;
            row.tip = Some(class_tip(class, undone).into());
            row.selected = selected(trees.selected_undo == Some(key));
            row.cells[1] = Cell::Custom;
            row.cells[2] = Cell::Custom;
            let fired = step.records.fired();
            let first = step.records.0.first().map(|r| r.line.clone());
            cells.text.insert(
                (key, 1),
                if fired.is_empty() {
                    "(no command line yet)".into()
                } else {
                    fired
                },
            );
            cells
                .text
                .insert((key, 2), first.as_deref().map_or("", help_of).into());
            let lines = step_lines(&step.records.0);
            let open = trees.open_steps.contains(&(undone, index));
            if lines.len() > 1 {
                row.disclosure = Some(open);
            }
            rows.push(row);
            if open && lines.len() > 1 {
                for (n, (depth, line)) in lines.iter().enumerate() {
                    let key = UndoKey::Line {
                        of: (undone, index),
                        line: n,
                    };
                    let mut child = TreeRow::new(key, 1 + depth, line.clone(), spec.columns.len());
                    child.style.dimmed = undone;
                    child.fixed = true;
                    child.selected = selected(trees.selected_undo == Some(key));
                    rows.push(child);
                }
            }
        }
        if !marker_done {
            rows.push(marker_row(&spec));
        }
        let mut editors = std::mem::take(&mut self.text_editors);
        let events = tree_list::show(ui, &spec, &mut trees.undos, &rows, &mut cells, &mut editors);
        self.text_editors = editors;
        let done = self.editor.undo.len();
        for event in events {
            match event {
                TreeEvent::Click { key, .. } => {
                    if key != UndoKey::Marker {
                        trees.selected_undo = Some(key);
                    }
                    if let Some(line) = self.undo_row_line(key) {
                        self.command = line;
                    }
                }
                TreeEvent::DoubleClick(key) => {
                    if let Some(line) = self.undo_row_line(key) {
                        self.execute_from(Source::Typed, &line);
                    }
                }
                TreeEvent::Disclosure { key, .. } => {
                    let of = match key {
                        UndoKey::Step(i) => (false, i),
                        UndoKey::Redo(k) => (true, k),
                        _ => continue,
                    };
                    if !trees.open_steps.remove(&of) {
                        trees.open_steps.insert(of);
                    }
                }
                TreeEvent::Drop { keys, at, .. } if keys == [UndoKey::Marker] => {
                    let target = drop_target(at, done, self.editor.redo.len());
                    if let Some(target) = target.filter(|&t| t != done) {
                        self.execute_from(Source::Button, &format!("cmds.undoTo {target}"));
                        trees.open_steps.clear();
                    }
                }
                _ => {}
            }
        }
    }

    /// The line a click on an Undos row puts in the entry (HIS-13).
    fn undo_row_line(&self, key: UndoKey) -> Option<String> {
        let step = |undone: bool, i: usize| {
            if undone {
                let n = self.editor.redo.len();
                n.checked_sub(i + 1).and_then(|j| self.editor.redo.get(j))
            } else {
                self.editor.undo.get(i)
            }
        };
        match key {
            UndoKey::Step(i) => step(false, i)?.records.0.first().map(|r| r.line.clone()),
            UndoKey::Redo(k) => step(true, k)?.records.0.first().map(|r| r.line.clone()),
            UndoKey::Line { of, line } => {
                let lines = step_lines(&step(of.0, of.1)?.records.0);
                lines.get(line).map(|(_, l)| l.clone())
            }
            UndoKey::Marker => None,
        }
    }

    // ----------------------------------------------------------- History

    /// Every line run this session, undone or not (HIS-20..22).
    fn history_tab(&mut self, ui: &mut Ui, trees: &mut HistoryTrees) {
        let spec = TreeSpec {
            id: "command-history-log",
            columns: vec![
                Column::name(),
                Column::value("Source", 60., "Where the line came from"),
                Column::value("Result", 140., "What it reported, or why it failed"),
            ],
            own_search: false,
            last_fills: true,
        };
        tree_list::tool_row(ui, &mut trees.history, true, |_, _| {});
        let (rows, mut cells) =
            record_rows(&spec, &self.history.log.records, self.history.selected);
        let mut editors = std::mem::take(&mut self.text_editors);
        let events = tree_list::show(
            ui,
            &spec,
            &mut trees.history,
            &rows,
            &mut cells,
            &mut editors,
        );
        self.text_editors = editors;
        self.record_events(events, true);
    }

    /// Lines run from the entry or this panel and how they ended (HIS-50).
    fn results_tab(&mut self, ui: &mut Ui, trees: &mut HistoryTrees) {
        let spec = TreeSpec {
            id: "command-history-results",
            columns: vec![
                Column::name(),
                Column::value("Source", 60., "Where the line came from"),
                Column::value("Result", 160., "What it reported, or why it failed"),
            ],
            own_search: false,
            last_fills: true,
        };
        tree_list::tool_row(ui, &mut trees.results, true, |_, _| {});
        let (rows, mut cells) =
            record_rows(&spec, &self.history.results.records, self.history.selected);
        let mut editors = std::mem::take(&mut self.text_editors);
        let events = tree_list::show(
            ui,
            &spec,
            &mut trees.results,
            &rows,
            &mut cells,
            &mut editors,
        );
        self.text_editors = editors;
        self.record_events(events, false);
    }

    fn record_events(&mut self, events: Vec<TreeEvent<usize>>, history: bool) {
        for event in events {
            let line = |a: &Operandi, i: usize| {
                let log = if history {
                    &a.history.log
                } else {
                    &a.history.results
                };
                log.records.get(i).map(|r| r.line.clone())
            };
            match event {
                TreeEvent::Click { key, .. } => {
                    // Pick to Add takes the line (LAY-82).
                    if let Some(l) = line(self, key)
                        && self.workbench_pick_history(&l)
                    {
                        continue;
                    }
                    self.history.selected = Some(key);
                    if let Some(l) = line(self, key) {
                        self.command = l;
                    }
                }
                TreeEvent::DoubleClick(key) => {
                    if let Some(l) = line(self, key) {
                        self.execute_from(Source::Typed, &l);
                    }
                }
                _ => {}
            }
        }
    }

    // ---------------------------------------------------------- Commands

    /// Every command by category, with its help (HIS-30). Double-click
    /// runs it bare, which opens its dialog when it has one.
    fn commands_tab(&mut self, ui: &mut Ui, trees: &mut HistoryTrees) {
        let spec = TreeSpec {
            id: "command-history-commands",
            columns: vec![
                Column::name(),
                Column::value("Help", 220., "What the command does"),
            ],
            own_search: false,
            last_fills: true,
        };
        tree_list::tool_row(ui, &mut trees.commands, true, |_, _| {});
        let searching = trees.commands.search.active();
        let mut rows = Vec::new();
        let mut cells = TextCells {
            text: HashMap::new(),
        };
        for (a, area) in Area::ALL.iter().enumerate() {
            let key = CommandKey::Area(a);
            let open = searching || trees.open_areas.contains(&a);
            let mut row = TreeRow::new(key, 0, capitalised(area.label()), spec.columns.len());
            row.disclosure = Some(open);
            row.style.bold = true;
            row.fixed = true;
            rows.push(row);
            if !open {
                continue;
            }
            for (i, entry) in COMMANDS.iter().enumerate().filter(|(_, e)| e.area == *area) {
                let key = CommandKey::Entry(i);
                let mut row = TreeRow::new(key, 1, entry.name, spec.columns.len());
                let class = entry.class().unwrap_or(CommandClass::Undoable);
                row.icon = class_letter(class, false);
                row.tip = Some(entry.usage());
                row.selected = selected(trees.selected_command == Some(key));
                row.cells[1] = Cell::Custom;
                cells.text.insert((key, 1), entry.help.into());
                rows.push(row);
            }
        }
        let mut editors = std::mem::take(&mut self.text_editors);
        let events = tree_list::show(
            ui,
            &spec,
            &mut trees.commands,
            &rows,
            &mut cells,
            &mut editors,
        );
        self.text_editors = editors;
        for event in events {
            match event {
                TreeEvent::Click {
                    key: CommandKey::Entry(i),
                    ..
                } => {
                    trees.selected_command = Some(CommandKey::Entry(i));
                    self.command = COMMANDS[i].name.into();
                }
                TreeEvent::DoubleClick(CommandKey::Entry(i)) => {
                    self.execute_from(Source::Typed, COMMANDS[i].name);
                }
                TreeEvent::Disclosure {
                    key: CommandKey::Area(a),
                    ..
                }
                | TreeEvent::Click {
                    key: CommandKey::Area(a),
                    ..
                } if !trees.open_areas.remove(&a) => {
                    trees.open_areas.insert(a);
                }
                _ => {}
            }
        }
    }
}

fn selected(yes: bool) -> Selected {
    if yes { Selected::Primary } else { Selected::No }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

fn marker_row(spec: &TreeSpec) -> TreeRow<UndoKey> {
    let mut row = TreeRow::new(UndoKey::Marker, 0, "Drag to undo/redo", spec.columns.len());
    row.style.bold = true;
    row.draggable = true;
    row.tip = Some("Drag up to undo several steps, down to redo them".into());
    row
}

/// A step's lines as child rows: each record (when there are several),
/// each with its sub-commands one level in (HIS-12).
fn step_lines(records: &[CommandRecord]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let several = records.len() > 1;
    for r in records {
        let depth = usize::from(several);
        if several {
            out.push((0, r.line.clone()));
        }
        for s in &r.sub {
            out.push((depth, s.clone()));
        }
        if r.sub_dropped > 0 {
            out.push((depth, format!("… {} more not kept", r.sub_dropped)));
        }
    }
    out
}

/// Where the drag row dropped as the undo length it leaves (HIS-11):
/// rows above it stay done.
fn drop_target(at: DropAt<UndoKey>, done: usize, undone: usize) -> Option<usize> {
    let before = |k: UndoKey| match k {
        UndoKey::Step(i) => Some(i),
        UndoKey::Redo(k) => Some(done + k),
        UndoKey::Marker => Some(done),
        UndoKey::Line { .. } => None,
    };
    let t = match at {
        DropAt::Before(k) => before(k)?,
        DropAt::After(k) | DropAt::On(k) => match k {
            UndoKey::Line { of: (u, i), .. } => {
                if u {
                    done + i + 1
                } else {
                    i + 1
                }
            }
            k => before(k)? + usize::from(k != UndoKey::Marker),
        },
        DropAt::End => done + undone,
    };
    Some(t.min(done + undone))
}

/// History or Results rows: the line, its source and result.
fn record_rows(
    spec: &TreeSpec,
    records: &std::collections::VecDeque<CommandRecord>,
    picked: Option<usize>,
) -> (Vec<TreeRow<usize>>, TextCells<usize>) {
    let mut rows = Vec::with_capacity(records.len());
    let mut cells = TextCells {
        text: HashMap::new(),
    };
    for (i, r) in records.iter().enumerate() {
        let mut row = TreeRow::new(i, 0, r.line.clone(), spec.columns.len());
        row.icon = class_letter(r.class, false);
        row.selected = selected(picked == Some(i));
        row.tip = Some(format!(
            "{} · {} · {:.1} s{}",
            class_tip(r.class, false),
            source_label(r.source),
            r.time,
            if r.sub.is_empty() {
                String::new()
            } else {
                format!(" · {} sub-commands", r.sub.len() as u32 + r.sub_dropped)
            }
        ));
        if !r.ok {
            row.tint = Some(theme::error());
        }
        row.cells[1] = Cell::Custom;
        row.cells[2] = Cell::Custom;
        cells.text.insert((i, 1), source_label(r.source).into());
        cells.text.insert(
            (i, 2),
            if r.ok {
                r.message.clone()
            } else {
                format!("Failed: {}", r.message)
            },
        );
        rows.push(row);
    }
    (rows, cells)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_drag_row_maps_drops_to_undo_lengths() {
        // Three done, two undone: rows are Step 0..2, Marker, Redo 0..1.
        assert_eq!(drop_target(DropAt::Before(UndoKey::Step(1)), 3, 2), Some(1));
        assert_eq!(drop_target(DropAt::After(UndoKey::Step(0)), 3, 2), Some(1));
        assert_eq!(drop_target(DropAt::Before(UndoKey::Step(0)), 3, 2), Some(0));
        assert_eq!(drop_target(DropAt::After(UndoKey::Redo(0)), 3, 2), Some(4));
        assert_eq!(drop_target(DropAt::End, 3, 2), Some(5));
    }
}
