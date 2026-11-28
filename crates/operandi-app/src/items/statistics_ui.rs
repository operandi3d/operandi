//! The Statistics panel (spec development/specs/statistics-panel/
//! BEHAVIOR-STATISTICS.md; counting in operandi-core statistics.rs, its
//! commands in statistics_edit.rs).
//!
//! A tree list on the shared component (UIC-130/131): two narrow state
//! columns "+" and "−" at the left (UIC-135), Name, then the value columns
//! "#" (count) and "Sel" (how many selected). + adds a row's elements to
//! the selection and switches to its mode (Q2), − removes them; each is one
//! `operandi.statistics.select` line, so the History, macros and the
//! bridge see the same step (UIC-160). Dragging down the + or − column
//! does every row it passes (UIC-138's sweep) as one step per row.
//!
//! Counting never runs on the UI thread (spec §9): every frame the panel
//! takes a cheap fingerprint of what it counts (mesh content stamps,
//! selection and hidden sets, open groups); when it changed, a snapshot of
//! shared meshes goes to a worker thread. At most one count runs at a
//! time; while a mouse button is held (a tool drag) a new one starts at
//! most every [`DRAG_INTERVAL`], and once more when the drag stops. Rows
//! of a group the last count didn't cover show "…".
//!
//! Registered under Info in the panel registry (UIC-156/157); Layout >
//! Windows/Palettes opens it as a floating window (palettes/statistics.json,
//! UIC-184). No default shortcut (Q1, Jason 2026-11-23).
use super::*;
use crate::tree_list::{
    self, Cell, ClickHow, Column, Heading, NameStyle, Selected, TreeEvent, TreeHooks, TreeRow,
    TreeSpec,
};
use crate::unavailable::Gate;
use operandi_core::statistics::{
    Border, Check, ItemType, Kind, MORE, MapKind, Need, PolyType, Report, Row, Tally,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Shortest gap between counts while a mouse button is held (spec §9: "at
/// most a few times a second while dragging a tool").
pub(crate) const DRAG_INTERVAL: Duration = Duration::from_millis(250);

/// Column indices.
const ADD: usize = 0;
const REMOVE: usize = 1;
const COUNT: usize = 3;
const SEL: usize = 4;

/// UIC-135: "+" and "−" state columns, Name, "#" and "Sel" value columns.
pub(crate) fn statistics_spec() -> TreeSpec {
    TreeSpec {
        id: "statistics-tree",
        columns: vec![
            Column::state(
                Heading::Glyph("+"),
                "Add the row's elements to the selection",
            ),
            Column::state(
                Heading::Glyph("−"),
                "Remove the row's elements from the selection",
            ),
            Column::name(),
            Column::value("#", 64., "How many"),
            Column::value("Sel", 64., "How many of them are selected"),
        ],
        own_search: false,
        last_fills: false,
    }
}

/// What a tree row stands for.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Entry {
    /// A heading with no numbers ("By polygon size"); its id opens it.
    Heading(&'static str),
    /// A heading under By vertex map for one map kind.
    MapHeading(MapKind),
    Row(Row),
    /// Drawn (GPU) Active / Background: read-only.
    Drawn {
        background: bool,
    },
}

impl Entry {
    /// The id its open state is saved under (rows with children).
    fn open_id(&self) -> Option<String> {
        match self {
            Entry::Heading(id) => Some((*id).to_owned()),
            Entry::MapHeading(k) => Some(format!("vertices.maps.{}", k.label())),
            Entry::Row(Row::All(k)) => Some(k.label().to_owned()),
            _ => None,
        }
    }
    fn key(&self) -> u64 {
        match self {
            Entry::Heading(id) => operandi_core::statistics::stable_hash(&format!("heading:{id}")),
            Entry::MapHeading(k) => {
                operandi_core::statistics::stable_hash(&format!("maps:{}", k.label()))
            }
            Entry::Row(r) => r.key(),
            Entry::Drawn { background } => {
                operandi_core::statistics::stable_hash(&format!("drawn:{background}"))
            }
        }
    }
}

/// Which groups are open, saved in statistics-panel.json (spec §9: "the
/// expanded state of each group is remembered"; provisional STAT-Q04: kept
/// per user, not per layout).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Saved {
    pub open: BTreeSet<String>,
}
impl Default for Saved {
    fn default() -> Self {
        Saved {
            open: ["Polygons", "polygons.vertices"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }
    }
}

/// What a count's result depends on, cheaply: equal fingerprints mean the
/// counts can't have changed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Fingerprint {
    meshes: Vec<(u64, u64, [u64; 3], [u64; 3])>,
    mode: u8,
    items: u64,
    need: (bool, bool, bool, bool),
}

/// A slice's length, address and a sample of its values: changes when
/// the list is replaced or edited (it's never hashed in full, so a 5M
/// polygon selection costs the same as a small one).
fn sample<T: Hash>(list: &[T]) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    list.len().hash(&mut h);
    (list.as_ptr() as usize).hash(&mut h);
    let step = (list.len() / 32).max(1);
    for x in list.iter().step_by(step).take(33) {
        x.hash(&mut h);
    }
    if let Some(last) = list.last() {
        last.hash(&mut h);
    }
    h.finish()
}
fn sample_set<T: Hash + Ord>(set: &BTreeSet<T>) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    set.len().hash(&mut h);
    set.first().hash(&mut h);
    set.last().hash(&mut h);
    h.finish()
}

/// The panel's own state (not part of the document).
#[derive(Default)]
pub(crate) struct StatsState {
    pub tree: tree_list::TreeState<u64>,
    pub saved: Saved,
    loaded: bool,
    /// The row picked in the list (UIC-30); the context menu acts on it.
    pub picked: Option<u64>,
    /// A copied selection set or vertex map (spec §7 Copy).
    pub clipboard: Option<Row>,
    pub report: Option<Report>,
    /// The fingerprint `report` was counted for.
    counted: Option<Fingerprint>,
    job: Option<(mpsc::Receiver<Report>, Fingerprint)>,
    last_start: Option<Instant>,
    /// Rows drawn last frame, by tree key.
    pub entries: HashMap<u64, Entry>,
    /// How many counts have run (tests).
    pub runs: u64,
}

fn tally_text(t: Option<Tally>) -> (String, String) {
    match t {
        Some(t) => (
            crate::defaults_preferences::thousands(t.count),
            crate::defaults_preferences::thousands(t.selected),
        ),
        None => ("…".into(), "…".into()),
    }
}

/// Draws the value cells: numbers right-aligned (they read as columns).
struct Hooks<'a> {
    app: &'a mut Operandi,
    values: HashMap<u64, (String, String, bool)>,
}

impl TreeHooks<u64> for Hooks<'_> {
    fn cell(&mut self, ui: &mut Ui, column: usize, key: u64, rect: Rect) {
        let Some((count, sel, dim)) = self.values.get(&key) else {
            return;
        };
        let text = if column == COUNT { count } else { sel };
        let color = if *dim {
            theme::weak_text()
        } else {
            theme::text()
        };
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        painter.text(
            pos2(rect.right() - 4., rect.center().y),
            Align2::RIGHT_CENTER,
            text,
            crate::typography::standard_font(),
            color,
        );
    }
    fn has_row_menu(&self, key: u64) -> bool {
        matches!(self.app.statistics.entries.get(&key), Some(Entry::Row(_)))
    }
    fn row_menu(&mut self, ui: &mut Ui, key: u64, rename: &mut dyn FnMut()) {
        self.app.menu_owned_input = true;
        let Some(Entry::Row(row)) = self.app.statistics.entries.get(&key).cloned() else {
            return;
        };
        self.app.statistics.picked = Some(key);
        let gate = None;
        // UIC-144: the row's own menu. Named things first (spec §7).
        if row.is_named() {
            if ui.gated(gate, Button::new("Rename…")).clicked() {
                rename();
                ui.close();
            }
            let delete_tip = "Remove it from every element; the elements stay";
            if ui
                .gated(gate, Button::new("Delete"))
                .on_hover_text(delete_tip)
                .clicked()
            {
                self.app
                    .execute(&format!("operandi.statistics.delete row:{{{row}}}"));
                ui.close();
            }
            if row.copies() {
                if ui
                    .button("Copy")
                    .on_hover_text("Remember this set's members or this map's values")
                    .clicked()
                {
                    self.app.statistics.clipboard = Some(row.clone());
                    ui.close();
                }
                let source = self
                    .app
                    .statistics
                    .clipboard
                    .clone()
                    .filter(|c| pastes_onto(c, &row));
                let why = if source.is_none() {
                    Some("Copy a set or map of the same kind first")
                } else {
                    gate
                };
                if ui
                    .gated(why, Button::new("Paste"))
                    .on_hover_text(
                        "Sets: add the copied members. Maps: overwrite with the copied values",
                    )
                    .clicked()
                    && let Some(from) = source
                {
                    self.app.execute(&format!(
                        "operandi.statistics.paste row:{{{row}}} from:{{{from}}}"
                    ));
                    ui.close();
                }
            }
            if matches!(row, Row::VertexMap(..))
                && ui
                    .gated(gate, Button::new("Clear"))
                    .on_hover_text("Set every value to zero; the map stays")
                    .clicked()
            {
                self.app
                    .execute(&format!("operandi.statistics.clear row:{{{row}}}"));
                ui.close();
            }
            ui.separator();
        }
        let select = |app: &mut Operandi, action: &str| {
            app.execute(&format!(
                "operandi.statistics.select row:{{{row}}} action:{action}"
            ));
        };
        if ui
            .button("Select Only")
            .on_hover_text("Replace the selection with this row's elements")
            .clicked()
        {
            select(self.app, "only");
            ui.close();
        }
        if !row.is_named() {
            if ui.button("Add").clicked() {
                select(self.app, "add");
                ui.close();
            }
            if ui.button("Remove").clicked() {
                select(self.app, "remove");
                ui.close();
            }
        }
        ui.separator();
        for (label, action) in [("Hide", "hide"), ("Unhide", "unhide")] {
            if ui.gated(gate, Button::new(label)).clicked() {
                self.app.execute(&format!(
                    "operandi.statistics.hide row:{{{row}}} action:{action}"
                ));
                ui.close();
            }
        }
    }
}

/// Whether a copied row can be pasted onto `row` (same kind of set, or a
/// map of the same kind, and not itself).
fn pastes_onto(from: &Row, row: &Row) -> bool {
    from != row
        && match (from, row) {
            (Row::VertexSet(_), Row::VertexSet(_))
            | (Row::EdgeSet(_), Row::EdgeSet(_))
            | (Row::PolygonSet(_), Row::PolygonSet(_)) => true,
            (Row::VertexMap(a, _), Row::VertexMap(b, _)) => a == b,
            _ => false,
        }
}

/// One tree row to show.
struct Line {
    entry: Entry,
    depth: usize,
    label: String,
    /// Has children (`Some(open)`), or `None`.
    open: Option<bool>,
    tip: Option<String>,
}

impl Operandi {
    fn stats_load(&mut self) {
        let s = &mut self.statistics;
        if s.loaded {
            return;
        }
        s.loaded = true;
        let path = self.environment.paths().statistics_panel();
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(saved) = serde_json::from_str::<Saved>(&text)
        {
            self.statistics.saved = saved;
        }
    }
    fn stats_save(&mut self) {
        let path = self.environment.paths().statistics_panel();
        if let Ok(text) = serde_json::to_string_pretty(&self.statistics.saved)
            && let Err(e) = std::fs::write(&path, text)
        {
            self.status = format!("Could not save the Statistics panel's open groups: {e}");
        }
    }
    fn stats_open(&self, id: &str) -> bool {
        self.statistics.saved.open.contains(id)
    }

    /// What the open groups need counted (spec §9).
    pub(crate) fn stats_need(&self) -> Need {
        Need {
            vertices: self.stats_open("Vertices"),
            edges: self.stats_open("Edges"),
            polygons: self.stats_open("Polygons"),
            checks: self.stats_open("Polygons") && self.stats_open("polygons.checks"),
        }
    }

    fn stats_fingerprint(&self, need: Need) -> Fingerprint {
        let scene = &self.editor.scene;
        let meshes = operandi_core::statistics::target_meshes(scene)
            .into_iter()
            .filter_map(|id| scene.item(id))
            .map(|i| {
                let s = &scene.selection;
                let h = &i.elements.hidden;
                (
                    i.id,
                    i.mesh.stamp(),
                    [
                        sample(s.vertices_of(i.id)),
                        sample(s.edges_of(i.id)),
                        sample(s.faces_of(i.id)),
                    ],
                    [
                        sample_set(&h.vertices),
                        sample_set(&h.edges),
                        sample_set(&h.faces),
                    ],
                )
            })
            .collect();
        let mut h = std::hash::DefaultHasher::new();
        for i in &scene.items {
            (i.id, ItemType::of(i)).hash(&mut h);
        }
        scene.selection.item.hash(&mut h);
        scene.selection.also.hash(&mut h);
        for g in &scene.groups {
            (g.id, &g.name, &g.members).hash(&mut h);
        }
        Fingerprint {
            meshes,
            mode: scene.selection.mode as u8,
            items: h.finish(),
            need: (need.vertices, need.edges, need.polygons, need.checks),
        }
    }

    /// Takes a finished count and starts the next one when the scene
    /// changed (spec §9). Cheap when nothing changed.
    pub(crate) fn stats_update(&mut self, ctx: &egui::Context) {
        if let Some((rx, print)) = &self.statistics.job {
            match rx.try_recv() {
                Ok(report) => {
                    self.statistics.counted = Some(print.clone());
                    self.statistics.report = Some(report);
                    self.statistics.job = None;
                }
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => self.statistics.job = None,
            }
        }
        let need = self.stats_need();
        let print = self.stats_fingerprint(need);
        if self.statistics.counted.as_ref() == Some(&print) {
            return;
        }
        let dragging = ctx.input(|i| i.pointer.any_down());
        if dragging
            && let Some(at) = self.statistics.last_start
            && at.elapsed() < DRAG_INTERVAL
        {
            ctx.request_repaint_after(DRAG_INTERVAL - at.elapsed());
            return;
        }
        let input = operandi_core::statistics::snapshot(&self.editor.scene, need);
        let (tx, rx) = mpsc::channel();
        let repaint = ctx.clone();
        let spawned = std::thread::Builder::new()
            .name("statistics".into())
            .spawn(move || {
                let report = operandi_core::statistics::compute(&input);
                let _ = tx.send(report);
                repaint.request_repaint();
            });
        if spawned.is_ok() {
            self.statistics.job = Some((rx, print));
            self.statistics.last_start = Some(Instant::now());
            self.statistics.runs += 1;
        }
    }

    /// Waits for the running count (tests and the bridge).
    #[cfg(test)]
    pub(crate) fn stats_settle(&mut self, ctx: &egui::Context) {
        for _ in 0..2 {
            self.statistics.last_start = None;
            self.stats_update(ctx);
            if let Some((rx, print)) = self.statistics.job.take()
                && let Ok(report) = rx.recv()
            {
                self.statistics.counted = Some(print);
                self.statistics.report = Some(report);
            }
        }
    }

    /// The rows, top to bottom, with what is open (`all_open`: as if
    /// every group were).
    fn stats_lines(&self, all_open: bool) -> Vec<Line> {
        let report = self.statistics.report.as_ref();
        let names = report.map(|r| r.names.clone()).unwrap_or_default();
        let mut out = Vec::new();
        let open = |id: &str| all_open || self.stats_open(id);
        let mut push =
            |entry: Entry, depth: usize, label: String, kids: bool, tip: Option<String>| {
                let o = entry.open_id().filter(|_| kids).map(|id| open(&id));
                let shown = o.unwrap_or(false);
                out.push(Line {
                    entry,
                    depth,
                    label,
                    open: o,
                    tip,
                });
                shown
            };
        let counts = |list: &mut dyn FnMut(Row, String), make: fn(u8) -> Row, from: u8| {
            for n in from..=MORE {
                list(make(n), Row::count_label(n));
            }
        };
        // Vertices.
        if push(
            Entry::Row(Row::All(Kind::Vertex)),
            0,
            "Vertices".into(),
            true,
            None,
        ) {
            if push(
                Entry::Heading("vertices.edges"),
                1,
                "By edge count".into(),
                true,
                None,
            ) {
                counts(
                    &mut |r, l| {
                        push(Entry::Row(r), 2, l, false, None);
                    },
                    Row::VertexEdges,
                    0,
                );
            }
            if push(
                Entry::Heading("vertices.polygons"),
                1,
                "By polygon count".into(),
                true,
                None,
            ) {
                counts(
                    &mut |r, l| {
                        push(Entry::Row(r), 2, l, false, None);
                    },
                    Row::VertexPolygons,
                    0,
                );
            }
            if push(
                Entry::Heading("vertices.border"),
                1,
                "Border".into(),
                true,
                None,
            ) {
                push(
                    Entry::Row(Row::VertexStraight),
                    2,
                    "In a straight line".into(),
                    false,
                    Some("Points whose two edges are in line: removable".into()),
                );
            }
            let kinds: Vec<MapKind> = MapKind::ALL
                .into_iter()
                .filter(|k| names.vertex_maps.iter().any(|(m, _)| m == k))
                .collect();
            if push(
                Entry::Heading("vertices.maps"),
                1,
                "By vertex map".into(),
                !kinds.is_empty(),
                None,
            ) {
                for k in kinds {
                    if push(Entry::MapHeading(k), 2, k.label().into(), true, None) {
                        for (_, n) in names.vertex_maps.iter().filter(|(m, _)| *m == k) {
                            push(
                                Entry::Row(Row::VertexMap(k, n.clone())),
                                3,
                                n.clone(),
                                false,
                                None,
                            );
                        }
                    }
                }
            }
            if push(
                Entry::Heading("vertices.sets"),
                1,
                "By selection set".into(),
                !names.vertex_sets.is_empty(),
                None,
            ) {
                for n in &names.vertex_sets {
                    push(
                        Entry::Row(Row::VertexSet(n.clone())),
                        2,
                        n.clone(),
                        false,
                        None,
                    );
                }
            }
        }
        // Edges.
        if push(
            Entry::Row(Row::All(Kind::Edge)),
            0,
            "Edges".into(),
            true,
            None,
        ) {
            if push(
                Entry::Heading("edges.polygons"),
                1,
                "By polygon count".into(),
                true,
                None,
            ) {
                counts(
                    &mut |r, l| {
                        let tip = match &r {
                            Row::EdgePolygons(1) => Some("Open border".to_owned()),
                            Row::EdgePolygons(n) if *n >= 3 => Some("Non-manifold".to_owned()),
                            _ => None,
                        };
                        push(Entry::Row(r), 2, l, false, tip);
                    },
                    Row::EdgePolygons,
                    1,
                );
            }
            if push(
                Entry::Heading("edges.border"),
                1,
                "Border".into(),
                true,
                None,
            ) {
                for b in Border::ALL {
                    let tip = match b {
                        Border::Part => "Between polygons of different parts",
                        Border::Material => "Between polygons of different materials",
                        Border::Geometry => "Open border: one polygon",
                        Border::Uv => "UV seam in the current UV map",
                        Border::Flat => "Between two polygons in one plane: removable",
                    };
                    push(
                        Entry::Row(Row::EdgeBorder(b)),
                        2,
                        b.label().into(),
                        false,
                        Some(tip.into()),
                    );
                }
            }
            if push(
                Entry::Heading("edges.maps"),
                1,
                "By vertex map".into(),
                names.edge_weights,
                None,
            ) {
                push(
                    Entry::Row(Row::EdgeWeight),
                    2,
                    "Subdivision edge weight".into(),
                    false,
                    None,
                );
            }
            if push(
                Entry::Heading("edges.sets"),
                1,
                "By selection set".into(),
                !names.edge_sets.is_empty(),
                None,
            ) {
                for n in &names.edge_sets {
                    push(
                        Entry::Row(Row::EdgeSet(n.clone())),
                        2,
                        n.clone(),
                        false,
                        None,
                    );
                }
            }
        }
        // Polygons.
        if push(
            Entry::Row(Row::All(Kind::Polygon)),
            0,
            "Polygons".into(),
            true,
            None,
        ) {
            if push(
                Entry::Heading("polygons.types"),
                1,
                "By type".into(),
                true,
                None,
            ) {
                for t in PolyType::ALL {
                    push(
                        Entry::Row(Row::PolygonType(t)),
                        2,
                        t.label().into(),
                        false,
                        None,
                    );
                }
            }
            if push(
                Entry::Heading("polygons.checks"),
                1,
                "Checks".into(),
                true,
                None,
            ) {
                for c in Check::ALL {
                    let tip = match c {
                        Check::SinglePoint => "Polygons of one point",
                        Check::NotFlat => "Not in one plane (the Flatten tools' rule)",
                        Check::Overlapping => "The same point positions as another polygon",
                        Check::FlatNeighbour => "In one plane with a neighbour: merge candidates",
                        Check::SharpAngle => "A corner under 1°: slivers",
                        Check::Concave => "A corner pointing inwards",
                    };
                    push(
                        Entry::Row(Row::PolygonCheck(c)),
                        2,
                        c.label().into(),
                        false,
                        Some(tip.into()),
                    );
                }
            }
            if push(
                Entry::Heading("polygons.vertices"),
                1,
                "By vertex count".into(),
                true,
                None,
            ) {
                for n in 1..=MORE {
                    let label = match n {
                        3 => "3 (triangles)".into(),
                        4 => "4 (quads)".into(),
                        MORE => "more than 4 (n-gons)".into(),
                        n => n.to_string(),
                    };
                    push(Entry::Row(Row::PolygonVertices(n)), 2, label, false, None);
                }
            }
            for (id, label, list, make) in [
                (
                    "polygons.parts",
                    "Part",
                    &names.parts,
                    Row::Part as fn(String) -> Row,
                ),
                (
                    "polygons.materials",
                    "Material",
                    &names.materials,
                    Row::Material,
                ),
                (
                    "polygons.sets",
                    "By selection set",
                    &names.polygon_sets,
                    Row::PolygonSet,
                ),
                (
                    "polygons.smoothing",
                    "Smoothing groups",
                    &names.smoothing,
                    Row::Smoothing,
                ),
            ] {
                if push(Entry::Heading(id), 1, label.into(), !list.is_empty(), None) {
                    for n in list {
                        push(Entry::Row(make(n.clone())), 2, n.clone(), false, None);
                    }
                }
            }
            if push(
                Entry::Heading("polygons.drawn"),
                1,
                "Drawn (GPU)".into(),
                true,
                Some("Triangles the views draw, after subdivision (the GL readout's count)".into()),
            ) {
                push(
                    Entry::Drawn { background: false },
                    2,
                    "Active".into(),
                    false,
                    None,
                );
                push(
                    Entry::Drawn { background: true },
                    2,
                    "Background".into(),
                    false,
                    None,
                );
            }
        }
        // Items.
        if push(
            Entry::Row(Row::All(Kind::Item)),
            0,
            "Items".into(),
            true,
            None,
        ) {
            if push(
                Entry::Heading("items.types"),
                1,
                "By type".into(),
                !names.item_types.is_empty(),
                None,
            ) {
                for t in &names.item_types {
                    push(
                        Entry::Row(Row::ItemType(*t)),
                        2,
                        t.label().into(),
                        false,
                        None,
                    );
                }
            }
            if push(
                Entry::Heading("items.groups"),
                1,
                "By group".into(),
                !names.item_groups.is_empty(),
                None,
            ) {
                for (id, n) in &names.item_groups {
                    push(Entry::Row(Row::ItemGroup(*id)), 2, n.clone(), false, None);
                }
            }
        }
        out
    }

    /// The panel (PanelKind::Statistics).
    pub(super) fn statistics_panel(&mut self, ui: &mut Ui, _area: Rect) {
        self.stats_load();
        let ctx = ui.ctx().clone();
        self.stats_update(&ctx);
        let mut state = std::mem::take(&mut self.statistics.tree);
        // UIC-132: no "add" control; the funnel searches row names.
        tree_list::tool_row(ui, &mut state, true, |_, _| {});
        let spec = statistics_spec();
        let lines = self.stats_lines(false);
        let report = self.statistics.report.as_ref();
        let meshes = report.is_some_and(|r| r.meshes > 0);
        let any_items = !self.editor.scene.items.is_empty();
        // Drawn (GPU): the GL readout's count, split by the counted meshes.
        let counted: BTreeSet<u64> = operandi_core::statistics::target_meshes(&self.editor.scene)
            .into_iter()
            .collect();
        let drawn_open = self.stats_open("Polygons") && self.stats_open("polygons.drawn");
        let (active_gl, background_gl) = if drawn_open {
            (
                self.drawn_triangles_where(|id| counted.contains(&id)),
                self.drawn_triangles_where(|id| !counted.contains(&id)),
            )
        } else {
            (0, 0)
        };
        let mut rows = Vec::with_capacity(lines.len());
        let mut values = HashMap::new();
        let mut entries = HashMap::new();
        for line in lines {
            let key = line.entry.key();
            let mut row = TreeRow::new(key, line.depth, line.label.clone(), spec.columns.len());
            row.disclosure = line.open;
            row.tip = line.tip.clone();
            let kind = match &line.entry {
                Entry::Row(r) => Some(r.kind()),
                Entry::Drawn { .. } => Some(Kind::Polygon),
                _ => None,
            };
            // UIC-141: a group that doesn't apply right now is dimmed
            // (spec §3, §10: no mesh counted; no items).
            let applies = match kind {
                Some(Kind::Item) => any_items,
                Some(_) => meshes,
                None => true,
            };
            row.style = NameStyle {
                dimmed: !applies,
                bold: line.depth == 0,
                ..Default::default()
            };
            row.selected = if self.statistics.picked == Some(key) {
                Selected::Primary
            } else {
                Selected::No
            };
            match &line.entry {
                Entry::Row(r) => {
                    let (verb, mode) = match r.kind() {
                        Kind::Item => ("items", "Items"),
                        Kind::Vertex => ("points", "Vertices"),
                        Kind::Edge => ("edges", "Edges"),
                        Kind::Polygon => ("polygons", "Polygons"),
                    };
                    row.cells[ADD] = Cell::Toggle {
                        on: false,
                        tip: format!("Add these {verb} to the selection (switches to {mode})"),
                        dimmed: !applies,
                    };
                    row.cells[REMOVE] = Cell::Toggle {
                        on: false,
                        tip: format!("Remove these {verb} from the selection"),
                        dimmed: !applies,
                    };
                    row.cells[COUNT] = Cell::Custom;
                    row.cells[SEL] = Cell::Custom;
                    row.renamable = r.is_named() && self.editor.session.live.is_none();
                    let tally = report.and_then(|rep| rep.get(r));
                    let (c, s) = tally_text(tally);
                    row.style.empty = tally.is_some_and(|t| t.count == 0) && applies;
                    values.insert(key, (c, s, !applies));
                }
                Entry::Drawn { background } => {
                    row.cells[COUNT] = Cell::Custom;
                    let n = if *background {
                        background_gl
                    } else {
                        active_gl
                    };
                    values.insert(
                        key,
                        (
                            crate::defaults_preferences::thousands(n),
                            String::new(),
                            false,
                        ),
                    );
                }
                _ => {}
            }
            entries.insert(key, line.entry);
            rows.push(row);
        }
        self.statistics.entries = entries;
        let mut editors = std::mem::take(&mut self.text_editors);
        let mut hooks = Hooks { app: self, values };
        let events = tree_list::show(ui, &spec, &mut state, &rows, &mut hooks, &mut editors);
        self.text_editors = editors;
        self.statistics.tree = state;
        for event in events {
            self.stats_event(event);
        }
    }

    pub(crate) fn stats_event(&mut self, event: TreeEvent<u64>) {
        let entry = |app: &Self, key: u64| app.statistics.entries.get(&key).cloned();
        match event {
            // UIC-30: a name click picks the row in the list; the 3D
            // selection stays (spec §6).
            TreeEvent::Click { key, how } => {
                self.statistics.picked = match how {
                    ClickHow::Toggle if self.statistics.picked == Some(key) => None,
                    _ => Some(key),
                };
            }
            TreeEvent::DoubleClick(_) | TreeEvent::Attributes(_) | TreeEvent::Drop { .. } => {}
            TreeEvent::Disclosure { key, subtree } => {
                if let Some(id) = entry(self, key).and_then(|e| e.open_id()) {
                    let open = !self.stats_open(&id);
                    let mut ids = vec![id];
                    // UIC-37: Shift opens or closes the whole subtree.
                    if subtree {
                        ids.extend(self.stats_subtree(key));
                    }
                    for id in ids {
                        if open {
                            self.statistics.saved.open.insert(id);
                        } else {
                            self.statistics.saved.open.remove(&id);
                        }
                    }
                    self.stats_save();
                }
            }
            TreeEvent::Rename { key, name } => {
                if let Some(Entry::Row(row)) = entry(self, key) {
                    let name = name.trim();
                    if !name.is_empty() {
                        self.execute(&format!(
                            "operandi.statistics.rename row:{{{row}}} name:{{{name}}}"
                        ));
                    }
                }
            }
            TreeEvent::Toggle { key, column, .. } => {
                if let Some(Entry::Row(row)) = entry(self, key) {
                    let action = if column == ADD { "add" } else { "remove" };
                    self.execute(&format!(
                        "operandi.statistics.select row:{{{row}}} action:{action}"
                    ));
                }
            }
            TreeEvent::Sweep { column, keys, .. } => {
                let action = if column == ADD { "add" } else { "remove" };
                for key in keys {
                    if let Some(Entry::Row(row)) = entry(self, key) {
                        self.execute(&format!(
                            "operandi.statistics.select row:{{{row}}} action:{action}"
                        ));
                    }
                }
            }
        }
    }

    /// The open ids of every group under `key`'s row (UIC-37).
    fn stats_subtree(&self, key: u64) -> Vec<String> {
        let lines = self.stats_lines(true);
        let Some(at) = lines.iter().position(|l| l.entry.key() == key) else {
            return Vec::new();
        };
        let depth = lines[at].depth;
        lines[at + 1..]
            .iter()
            .take_while(|l| l.depth > depth)
            .filter(|l| l.open.is_some())
            .filter_map(|l| l.entry.open_id())
            .collect()
    }
}
