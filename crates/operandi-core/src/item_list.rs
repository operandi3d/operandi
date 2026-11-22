//! Items list header (spec item-list-header, IL-00..IL-08): what Add Item
//! creates and where it goes, the Select popover's name patterns, the
//! Recent list, and the Filter popover's view of the list. The GUI lives
//! in `operandi-app` (item_list_ui.rs); everything here works without a
//! window.
//!
//! Adding and pattern actions are `editor::Command`s (one undo step each).
//! Filtering is view-only: `list_rows` reads the scene and never changes it
//! (IL-06).
use crate::camera_light::LightKind;
use crate::scene::{Item, Scene};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// An item type Add Item can make today (IL-02 entries Operandi supports).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddItemKind {
    Mesh,
    Camera,
    Backdrop,
    Light(LightKind),
    /// GL-02.
    GroupLocator,
    /// ENV §6 Create Environment. It goes to the end of the scene's items
    /// (not after the selected item), so it is the newest and tops the
    /// Shader Tree's Environments section (ENVIRONMENT-PLACEMENT §3.5).
    /// It never shows in the Item List (§1).
    Environment,
}

impl AddItemKind {
    /// Every kind, for scripts and tests.
    pub const ALL: [AddItemKind; 14] = [
        AddItemKind::Backdrop,
        AddItemKind::Camera,
        AddItemKind::Environment,
        AddItemKind::GroupLocator,
        AddItemKind::Mesh,
        AddItemKind::Light(LightKind::Area),
        AddItemKind::Light(LightKind::Cylinder),
        AddItemKind::Light(LightKind::Directional),
        AddItemKind::Light(LightKind::Dome),
        AddItemKind::Light(LightKind::Mesh),
        AddItemKind::Light(LightKind::Photometric),
        AddItemKind::Light(LightKind::Point),
        AddItemKind::Light(LightKind::Portal),
        AddItemKind::Light(LightKind::Spot),
    ];
    /// The menu entry, which is also the new item's name (IL-03 point 2).
    pub fn label(self) -> &'static str {
        match self {
            AddItemKind::Mesh => "Mesh",
            AddItemKind::Camera => crate::camera_light_defaults::CAMERA_NAME,
            AddItemKind::Backdrop => "Backdrop Item",
            AddItemKind::Light(kind) => kind.item_name(),
            AddItemKind::GroupLocator => crate::locator::GROUP_LOCATOR_NAME,
            AddItemKind::Environment => crate::environment::ENVIRONMENT_NAME,
        }
    }
    /// Script key (provisional, IL-CMD), e.g. "mesh", "light.spot".
    pub fn key(self) -> &'static str {
        match self {
            AddItemKind::Mesh => "mesh",
            AddItemKind::Camera => "camera",
            AddItemKind::Backdrop => "backdrop",
            AddItemKind::Light(LightKind::Directional) => "light.directional",
            AddItemKind::Light(LightKind::Point) => "light.point",
            AddItemKind::Light(LightKind::Spot) => "light.spot",
            AddItemKind::Light(LightKind::Area) => "light.area",
            AddItemKind::Light(LightKind::Portal) => "light.portal",
            AddItemKind::Light(LightKind::Cylinder) => "light.cylinder",
            AddItemKind::Light(LightKind::Mesh) => "light.mesh",
            AddItemKind::Light(LightKind::Photometric) => "light.photometric",
            AddItemKind::Light(LightKind::Dome) => "light.dome",
            // Provisional (GL-Q2).
            AddItemKind::GroupLocator => "locator.group",
            AddItemKind::Environment => "environment",
        }
    }
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.key() == key)
    }
}

/// Recent list length (IL-04: cap not observed; Operandi keeps 8).
pub const RECENT_CAP: usize = 8;

/// IL-04: put `kind` at the top of `recent`, dropping an older copy and
/// anything past the cap.
pub fn push_recent(recent: &mut Vec<AddItemKind>, kind: AddItemKind) {
    recent.retain(|k| *k != kind);
    recent.insert(0, kind);
    recent.truncate(RECENT_CAP);
}

/// The item types the Select and Filter popovers can tick. Only types
/// Operandi has are listed (spec "Filter / Select type checkboxes");
/// Groups and the rest join as their item kinds land.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemType {
    Backdrop,
    Camera,
    Light,
    /// Group Locators (GL-05).
    Locator,
    Mesh,
}

impl ItemType {
    /// Popover order (alphabetical, as in IL-05/IL-06), which is also the
    /// Sort by Type order.
    pub const ALL: [ItemType; 5] = [
        ItemType::Backdrop,
        ItemType::Camera,
        ItemType::Light,
        ItemType::Locator,
        ItemType::Mesh,
    ];
    /// The checkbox label.
    pub fn label(self) -> &'static str {
        match self {
            ItemType::Backdrop => "Backdrops",
            ItemType::Camera => "Cameras",
            ItemType::Light => "Lights",
            ItemType::Locator => "Locators",
            ItemType::Mesh => "Meshes",
        }
    }
    pub fn of(item: &Item) -> Self {
        if item.backdrop.is_some() {
            ItemType::Backdrop
        } else if item.camera.is_some() {
            ItemType::Camera
        } else if item.light.is_some() {
            ItemType::Light
        } else if item.locator.is_some() {
            ItemType::Locator
        } else {
            ItemType::Mesh
        }
    }
}

/// Which item types are ticked. Every type starts ticked (IL-05, IL-06).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TypeTicks {
    pub backdrops: bool,
    pub cameras: bool,
    pub lights: bool,
    /// GL-05; older saved filters lack it and get the default (ticked).
    pub locators: bool,
    pub meshes: bool,
}

impl Default for TypeTicks {
    fn default() -> Self {
        Self::all()
    }
}

impl TypeTicks {
    pub fn all() -> Self {
        Self {
            backdrops: true,
            cameras: true,
            lights: true,
            locators: true,
            meshes: true,
        }
    }
    pub fn none() -> Self {
        Self {
            backdrops: false,
            cameras: false,
            lights: false,
            locators: false,
            meshes: false,
        }
    }
    pub fn invert(self) -> Self {
        Self {
            backdrops: !self.backdrops,
            cameras: !self.cameras,
            lights: !self.lights,
            locators: !self.locators,
            meshes: !self.meshes,
        }
    }
    pub fn get(&self, t: ItemType) -> bool {
        *self.slot(t)
    }
    pub fn set(&mut self, t: ItemType, on: bool) {
        *self.slot_mut(t) = on;
    }
    fn slot(&self, t: ItemType) -> &bool {
        match t {
            ItemType::Backdrop => &self.backdrops,
            ItemType::Camera => &self.cameras,
            ItemType::Light => &self.lights,
            ItemType::Locator => &self.locators,
            ItemType::Mesh => &self.meshes,
        }
    }
    fn slot_mut(&mut self, t: ItemType) -> &mut bool {
        match t {
            ItemType::Backdrop => &mut self.backdrops,
            ItemType::Camera => &mut self.cameras,
            ItemType::Light => &mut self.lights,
            ItemType::Locator => &mut self.locators,
            ItemType::Mesh => &mut self.meshes,
        }
    }
}

/// IL-05 pattern match: `*` is any run of characters, `?` exactly one;
/// case-insensitive; the whole name must match.
pub fn pattern_matches(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().flat_map(char::to_lowercase).collect();
    let n: Vec<char> = name.chars().flat_map(char::to_lowercase).collect();
    // Greedy wildcard match with backtracking to the last `*`.
    let (mut pi, mut ni) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some((pi, ni));
            pi += 1;
        } else if let Some((sp, sn)) = star {
            pi = sp + 1;
            ni = sn + 1;
            star = Some((sp, sn + 1));
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

/// The Select popover's settings (IL-05). The app keeps one for the
/// session.
#[derive(Clone, Debug, PartialEq)]
pub struct PatternQuery {
    /// Default `*`.
    pub pattern: String,
    pub types: TypeTicks,
    /// Only items parented (at any depth) under the selected items.
    pub limit_to_children: bool,
    /// By Color: only items with `color` (none = items without one).
    pub by_color: bool,
    pub color: Option<crate::EditorColor>,
}

impl Default for PatternQuery {
    fn default() -> Self {
        Self {
            pattern: "*".into(),
            types: TypeTicks::all(),
            limit_to_children: false,
            by_color: false,
            color: None,
        }
    }
}

/// IL-05 Action buttons Operandi has. Group waits for item groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternAction {
    /// Select every match, replacing the selection.
    Select,
    /// Take the matches out of the selection.
    Unselect,
    /// Turn the matches' visibility off.
    Hide,
}

impl PatternAction {
    pub fn label(self) -> &'static str {
        match self {
            PatternAction::Select => "Select",
            PatternAction::Unselect => "Unselect",
            PatternAction::Hide => "Hide",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            PatternAction::Select => "select",
            PatternAction::Unselect => "unselect",
            PatternAction::Hide => "hide",
        }
    }
    pub fn from_key(key: &str) -> Option<Self> {
        [Self::Select, Self::Unselect, Self::Hide]
            .into_iter()
            .find(|a| a.key() == key)
    }
}

impl Item {
    /// Whether the Item List shows this item. An item type with no
    /// transform doesn't go there: Environment items live only in the
    /// Shader Tree's Environments section (ENVIRONMENT-PLACEMENT §1).
    /// Everything the list drives (rows, Select patterns, select-all,
    /// drag and drop) goes through this.
    pub fn in_item_list(&self) -> bool {
        self.environment.is_none()
    }
}

/// Item id → its children, in item-list order. Items the Item List
/// doesn't show are left out (`Item::in_item_list`).
fn children_map(scene: &Scene) -> HashMap<Option<u64>, Vec<&Item>> {
    let mut map: HashMap<Option<u64>, Vec<&Item>> = HashMap::new();
    for item in scene.items.iter().filter(|i| i.in_item_list()) {
        map.entry(item.parent).or_default().push(item);
    }
    map
}

impl Scene {
    /// Ids of the items `query` matches, in item-list order (IL-05).
    pub fn pattern_matches(&self, query: &PatternQuery) -> Vec<u64> {
        let limit: Option<BTreeSet<u64>> = query.limit_to_children.then(|| {
            // Every descendant of a selected item, not the selected
            // items themselves.
            let children = children_map(self);
            let mut out = BTreeSet::new();
            let mut stack: Vec<u64> = self.selected_items();
            while let Some(id) = stack.pop() {
                for child in children.get(&Some(id)).into_iter().flatten() {
                    if out.insert(child.id) {
                        stack.push(child.id);
                    }
                }
            }
            out
        });
        self.items
            .iter()
            .filter(|i| i.in_item_list())
            .filter(|i| limit.as_ref().is_none_or(|l| l.contains(&i.id)))
            .filter(|i| query.types.get(ItemType::of(i)))
            .filter(|i| pattern_matches(&query.pattern, &i.name))
            .filter(|i| !query.by_color || i.common.editor_color == query.color)
            .map(|i| i.id)
            .collect()
    }

    /// IL-03 point 4: move the last item (just made) to directly after the
    /// selected item, as its sibling. With nothing selected it stays at
    /// the end of the root list. The render camera does not change.
    pub(crate) fn place_new_item(&mut self, anchor: Option<u64>, render_camera: Option<u64>) {
        let Some(new) = self.items.pop() else {
            return;
        };
        let mut new = new;
        match anchor.and_then(|a| self.item_index(a)) {
            Some(at) => {
                new.parent = self.items[at].parent;
                self.items.insert(at + 1, new);
            }
            None => {
                new.parent = None;
                self.items.push(new);
            }
        }
        // A camera placed ahead of the first camera would otherwise become
        // the render camera (CL-21 picks the first when none is chosen).
        if render_camera.is_some() && self.render_camera() != render_camera {
            self.render_camera = render_camera;
        }
    }
}

/// The Filter popover and the header toggles (IL-06..IL-08). View only;
/// saved with the layout.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ListFilter {
    pub flatten: bool,
    pub sort_by_name: bool,
    pub sort_by_type: bool,
    pub types: TypeTicks,
    /// IL-08.
    pub only_selected: bool,
    /// IL-07: `Some` while the text filter row is on.
    pub text: Option<String>,
    /// IL-07 Flat: matches as a flat list without their parents.
    pub text_flat: bool,
    /// UIC-40 search options: `*`/`?` pattern matching instead of plain
    /// text, and Match Case.
    pub text_pattern: bool,
    pub text_case: bool,
    /// Note 10 By Color: only items with `color` (and the parents needed
    /// to reach them); none = items without one.
    pub by_color: bool,
    #[serde(with = "crate::editor_color::serde_name")]
    pub color: Option<crate::EditorColor>,
}

/// One row of the filtered Items list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListRow {
    pub id: u64,
    /// Indent level, 1 for a root item (0 is the scene row).
    pub depth: usize,
    /// Nesting dots in front of the name (Flatten, IL-06).
    pub dots: usize,
    /// Shown only to keep a matching child in place (IL-06, IL-08).
    pub greyed: bool,
    /// Has children in the unfiltered tree (for the disclosure triangle).
    pub has_children: bool,
}

impl ListFilter {
    /// Nothing filtered or reordered: the plain tree.
    pub fn is_plain(&self) -> bool {
        !self.flatten
            && !self.sort_by_name
            && !self.sort_by_type
            && self.types == TypeTicks::all()
            && !self.only_selected
            && !self.by_color
            && self.search().is_none()
    }
    /// The text being searched for, if any.
    pub fn search(&self) -> Option<String> {
        self.text
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(|t| {
                if self.text_case {
                    t.to_owned()
                } else {
                    t.to_lowercase()
                }
            })
    }
    /// Does `name` match the search text (already case-folded by
    /// `search`)?
    pub fn text_matches(&self, search: &str, name: &str) -> bool {
        let name = if self.text_case {
            name.to_owned()
        } else {
            name.to_lowercase()
        };
        if self.text_pattern {
            wildcard_match(search, &name)
        } else {
            name.contains(search)
        }
    }
    fn matches(&self, item: &Item, selected: &BTreeSet<u64>, search: Option<&str>) -> bool {
        self.types.get(ItemType::of(item))
            && (!self.only_selected || selected.contains(&item.id))
            && (!self.by_color || item.common.editor_color == self.color)
            && search.is_none_or(|s| self.text_matches(s, &item.name))
    }
    fn sort(&self, items: &mut [&Item]) {
        if self.sort_by_type {
            items.sort_by_key(|i| ItemType::of(i));
        }
        if self.sort_by_name {
            // Stable: within a type when Sort by Type is on too.
            items.sort_by_cached_key(|i| {
                (
                    self.sort_by_type.then(|| ItemType::of(i)),
                    i.name.to_lowercase(),
                )
            });
        }
    }
}

/// `*` any run, `?` one character; the whole name must match. Case
/// matters here: callers fold case first when it shouldn't.
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = text.chars().collect();
    let (mut pi, mut ni) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some((pi, ni));
            pi += 1;
        } else if let Some((sp, sn)) = star {
            pi = sp + 1;
            ni = sn + 1;
            star = Some((sp, sn + 1));
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

/// The Items list's rows under the scene row, after `filter` (IL-06..08).
/// `collapsed` holds items whose children are folded away. Never changes
/// the scene.
pub fn list_rows(scene: &Scene, filter: &ListFilter, collapsed: &BTreeSet<u64>) -> Vec<ListRow> {
    let children = children_map(scene);
    let selected: BTreeSet<u64> = scene.selected_items().into_iter().collect();
    let search = filter.search();
    let matches = |i: &Item| filter.matches(i, &selected, search.as_deref());
    let sorted = |parent: Option<u64>| {
        let mut list: Vec<&Item> = children.get(&parent).cloned().unwrap_or_default();
        filter.sort(&mut list);
        list
    };
    let has_children = |id: u64| children.get(&Some(id)).is_some_and(|c| !c.is_empty());

    let flat = filter.flatten || (search.is_some() && filter.text_flat);
    if flat {
        // Every item at the base level, tree order (sorted siblings),
        // then sorted as a whole when a sort is on.
        let mut walk: Vec<(&Item, usize)> = Vec::new();
        let mut stack: Vec<(&Item, usize)> =
            sorted(None).into_iter().rev().map(|i| (i, 0)).collect();
        while let Some((item, level)) = stack.pop() {
            walk.push((item, level));
            stack.extend(
                sorted(Some(item.id))
                    .into_iter()
                    .rev()
                    .map(|i| (i, level + 1)),
            );
        }
        let mut kept: Vec<(&Item, usize)> = walk.into_iter().filter(|(i, _)| matches(i)).collect();
        if filter.sort_by_name || filter.sort_by_type {
            let mut items: Vec<&Item> = kept.iter().map(|(i, _)| *i).collect();
            filter.sort(&mut items);
            let level: HashMap<u64, usize> = kept.iter().map(|(i, l)| (i.id, *l)).collect();
            kept = items.into_iter().map(|i| (i, level[&i.id])).collect();
        }
        return kept
            .into_iter()
            .map(|(item, level)| ListRow {
                id: item.id,
                depth: 1,
                dots: if filter.flatten { level } else { 0 },
                greyed: false,
                has_children: false,
            })
            .collect();
    }

    // Tree: an item is drawn if it matches or has a matching descendant
    // (then greyed). Post-order pass for "keeps a descendant".
    let mut keeps: HashMap<u64, bool> = HashMap::new();
    if !filter.is_plain() {
        let mut order: Vec<&Item> = Vec::new();
        let mut stack: Vec<&Item> = sorted(None);
        while let Some(item) = stack.pop() {
            order.push(item);
            stack.extend(children.get(&Some(item.id)).into_iter().flatten().copied());
        }
        for item in order.into_iter().rev() {
            let child_keeps = children
                .get(&Some(item.id))
                .into_iter()
                .flatten()
                .any(|c| keeps.get(&c.id).copied().unwrap_or(false));
            keeps.insert(item.id, matches(item) || child_keeps);
        }
    }
    let shown = |i: &Item| filter.is_plain() || keeps.get(&i.id).copied().unwrap_or(false);
    let mut rows = Vec::new();
    let mut stack: Vec<(&Item, usize)> = sorted(None)
        .into_iter()
        .rev()
        .filter(|i| shown(i))
        .map(|i| (i, 1))
        .collect();
    while let Some((item, depth)) = stack.pop() {
        rows.push(ListRow {
            id: item.id,
            depth,
            dots: 0,
            greyed: !matches(item),
            has_children: has_children(item.id),
        });
        if !collapsed.contains(&item.id) {
            stack.extend(
                sorted(Some(item.id))
                    .into_iter()
                    .rev()
                    .filter(|i| shown(i))
                    .map(|i| (i, depth + 1)),
            );
        }
    }
    rows
}
