//! The Display and Assembly tabs every item shares (BEHAVIOR-ITEM-COMMON-TABS,
//! Tier 1): Visible (CP-D01), Label with Show Label and Label Offset
//! (CP-D06), Select (CP-A01) and Lock (CP-A02). The same settings on every
//! item kind (mesh, camera, light, Backdrop), stored on `Item::common`.
//!
//! Provisional readings (docs/SPECIFICATION_QUESTIONS.md, LPROP-Q*):
//! - Select and Lock "Default" behave as "Yes" and "No": selectable and
//!   unlocked. Nothing is inherited from parents (LPROP-Q02).
//! - Visible "Yes" overrides the viewport's Show Cameras / Show Lights
//!   toggle, but the Items-list eye still hides the item (LPROP-Q03).
//! - Select "No" only stops Item-mode picks in 3D views; component modes on
//!   the active mesh still pick (LPROP-Q04).
use crate::Result;
use crate::scene::{Item, Scene};
use serde::{Deserialize, Serialize};

/// Visible (CP-D01).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayVisible {
    /// Follow the view's own show toggle for this kind of item.
    #[default]
    Default,
    /// Always drawn.
    Yes,
    /// Never drawn.
    No,
    /// This item and every item under it never drawn.
    NoHideChildren,
}

impl DisplayVisible {
    pub const ALL: [DisplayVisible; 4] = [
        DisplayVisible::Default,
        DisplayVisible::Yes,
        DisplayVisible::No,
        DisplayVisible::NoHideChildren,
    ];
    pub fn label(self) -> &'static str {
        match self {
            DisplayVisible::Default => "Default",
            DisplayVisible::Yes => "Yes",
            DisplayVisible::No => "No",
            DisplayVisible::NoHideChildren => "No (Hide Children)",
        }
    }
}

/// A Default / Yes / No choice (Select, Lock).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Choice {
    #[default]
    Default,
    Yes,
    No,
}

impl Choice {
    pub const ALL: [Choice; 3] = [Choice::Default, Choice::Yes, Choice::No];
    pub fn label(self) -> &'static str {
        match self {
            Choice::Default => "Default",
            Choice::Yes => "Yes",
            Choice::No => "No",
        }
    }
}

/// Display tab, Tier 1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ItemDisplay {
    pub visible: DisplayVisible,
    /// Text drawn next to the item in 3D views; empty draws nothing.
    pub label: String,
    /// Shown once a Label is typed; on by default.
    pub show_label: bool,
    /// Screen offset of the label from the item's centre, points (X right,
    /// Y up).
    pub label_offset: [f64; 2],
}

impl Default for ItemDisplay {
    fn default() -> Self {
        Self {
            visible: DisplayVisible::Default,
            label: String::new(),
            show_label: true,
            label_offset: [0.0, 0.0],
        }
    }
}

/// Assembly tab, Tier 1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ItemAssembly {
    pub select: Choice,
    pub lock: Choice,
}

/// The shared tabs' settings of one item.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ItemCommon {
    pub display: ItemDisplay,
    pub assembly: ItemAssembly,
    /// Targeting (LP-04, item_target.rs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<crate::item_target::ItemTarget>,
    /// Tier 3 rows, User Channels and Tags (item_extras.rs).
    #[serde(skip_serializing_if = "is_default_extras")]
    pub extras: crate::item_extras::ItemExtras,
    /// Editor Color (note 10): a named colour tag for the lists; no effect
    /// on rendering. Native version 31.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::editor_color::serde_name"
    )]
    pub editor_color: Option<crate::EditorColor>,
    /// Set on the item group an environment preset's lights, meshes and
    /// cameras arrive in (EP v0.2), naming the preset: Replace removes
    /// such groups (even renamed) and keeps hand-made items outside them.
    /// Shown only as a tooltip line. Native version 42.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_preset: Option<String>,
}

fn is_default_extras(x: &crate::item_extras::ItemExtras) -> bool {
    *x == Default::default()
}

impl ItemCommon {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Longest Label, bytes.
pub const MAX_LABEL_BYTES: usize = 1024;
/// Largest Label Offset, points either way.
pub const MAX_LABEL_OFFSET: f64 = 10_000.0;

/// One control on the Display or Assembly tab; each is its own undo step
/// named after the property (IC-1).
#[derive(Clone, Debug, PartialEq)]
pub enum ItemCommonEdit {
    Visible(DisplayVisible),
    Label(String),
    ShowLabel(bool),
    LabelOffsetX(f64),
    LabelOffsetY(f64),
    Select(Choice),
    Lock(Choice),
    /// Set Target (an item) or Remove Target (none), LP-04.
    Target(Option<u64>),
    /// Targeting Enable and Roll (radians).
    TargetEnable(bool),
    TargetRoll(f64),
    /// The Tier 3 rows, User Channels and Tags.
    Extra(crate::item_extras::ExtraEdit),
    /// Editor Color (note 10); `None` clears it.
    EditorColor(Option<crate::EditorColor>),
}

impl ItemCommonEdit {
    /// The undo step's name (IC-1).
    pub fn label(&self) -> &'static str {
        match self {
            ItemCommonEdit::Visible(_) => "Visible",
            ItemCommonEdit::Label(_) => "Label",
            ItemCommonEdit::ShowLabel(_) => "Show Label",
            ItemCommonEdit::LabelOffsetX(_) => "Label Offset X",
            ItemCommonEdit::LabelOffsetY(_) => "Label Offset Y",
            ItemCommonEdit::Select(_) => "Select",
            ItemCommonEdit::Lock(_) => "Lock",
            ItemCommonEdit::Target(Some(_)) => "Set Target",
            ItemCommonEdit::Target(None) => "Remove Target",
            ItemCommonEdit::TargetEnable(_) => "Target Enable",
            ItemCommonEdit::TargetRoll(_) => "Target Roll",
            ItemCommonEdit::Extra(e) => e.label(),
            ItemCommonEdit::EditorColor(_) => "Editor Color",
        }
    }

    /// Whether Lock "Yes" refuses this edit. CP-A02 locks the display rows
    /// too (Visible, Pivot, Center, Size); Label and the Assembly rows stay
    /// editable so a locked item can be unlocked.
    fn locked_by(&self) -> bool {
        // Targeting turns the item, which Lock refuses like a transform.
        match self {
            ItemCommonEdit::Extra(e) => e.locked_by(),
            _ => matches!(
                self,
                ItemCommonEdit::Visible(_)
                    | ItemCommonEdit::Target(_)
                    | ItemCommonEdit::TargetEnable(_)
                    | ItemCommonEdit::TargetRoll(_)
            ),
        }
    }
}

impl ItemCommon {
    /// Apply one edit; invalid values leave everything as it was.
    pub fn apply(&mut self, edit: &ItemCommonEdit) -> Result<()> {
        if edit.locked_by() && self.assembly.lock == Choice::Yes {
            return Err("The item is locked (Assembly > Lock)".into());
        }
        let offset_ok = |v: f64| v.is_finite() && v.abs() <= MAX_LABEL_OFFSET;
        match edit {
            ItemCommonEdit::Visible(v) => self.display.visible = *v,
            ItemCommonEdit::EditorColor(c) => self.editor_color = *c,
            ItemCommonEdit::Label(text) => {
                if text.len() > MAX_LABEL_BYTES {
                    return Err("Label is too long".into());
                }
                self.display.label = text.clone();
            }
            ItemCommonEdit::ShowLabel(on) => self.display.show_label = *on,
            ItemCommonEdit::LabelOffsetX(v) | ItemCommonEdit::LabelOffsetY(v) if !offset_ok(*v) => {
                return Err("Label Offset out of range".into());
            }
            ItemCommonEdit::LabelOffsetX(v) => self.display.label_offset[0] = *v,
            ItemCommonEdit::LabelOffsetY(v) => self.display.label_offset[1] = *v,
            ItemCommonEdit::Select(c) => self.assembly.select = *c,
            ItemCommonEdit::Lock(c) => self.assembly.lock = *c,
            ItemCommonEdit::Target(None) => self.target = None,
            ItemCommonEdit::Target(Some(id)) => {
                let previous = self.target.take().unwrap_or_default();
                self.target = Some(crate::item_target::ItemTarget {
                    item: *id,
                    ..previous
                });
            }
            ItemCommonEdit::TargetEnable(on) => {
                self.target.as_mut().ok_or("Set a target first")?.enable = *on
            }
            ItemCommonEdit::TargetRoll(v) if !v.is_finite() => {
                return Err("Roll must be a number".into());
            }
            ItemCommonEdit::TargetRoll(v) => {
                self.target.as_mut().ok_or("Set a target first")?.roll = *v
            }
            ItemCommonEdit::Extra(e) => self.apply_extra(e)?,
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        let d = &self.display;
        if self.target.as_ref().is_some_and(|t| !t.roll.is_finite())
            || d.label.len() > MAX_LABEL_BYTES
            || !d
                .label_offset
                .iter()
                .all(|v| v.is_finite() && v.abs() <= MAX_LABEL_OFFSET)
        {
            return Err("Invalid item display settings".into());
        }
        self.extras.validate()
    }
}

impl Item {
    /// CP-A02: Lock "Yes" refuses item transforms.
    pub fn is_locked(&self) -> bool {
        self.common.assembly.lock == Choice::Yes
    }
    /// CP-A01: Select "No" can't be picked in 3D views.
    pub fn is_viewport_selectable(&self) -> bool {
        self.common.assembly.select != Choice::No
    }
    /// The Label to draw, if any (CP-D06).
    pub fn shown_label(&self) -> Option<&str> {
        let d = &self.common.display;
        (d.show_label && !d.label.trim().is_empty()).then_some(d.label.as_str())
    }
}

impl Scene {
    /// Whether item `id` is drawn in 3D views by its own Display settings
    /// (CP-D01), given whether the view shows its kind (`kind_shown`: Show
    /// Cameras / Show Lights for those, always true for meshes). The
    /// Items-list eye (`Item::visible`) hides it too.
    pub fn drawn_in_viewport(&self, id: u64, kind_shown: bool) -> bool {
        let Some(item) = self.item(id) else {
            return false;
        };
        if !self.shows(item) || self.hidden_by_ancestor(item) {
            return false;
        }
        match item.common.display.visible {
            DisplayVisible::Default => kind_shown,
            DisplayVisible::Yes => true,
            DisplayVisible::No | DisplayVisible::NoHideChildren => false,
        }
    }

    /// Whether `item`'s eye shows it: its own eye is on and no ancestor's
    /// eye is "all off" (GV-01/02). A child under a hidden parent keeps its
    /// own `visible` flag; it is hidden through the parent.
    pub fn shows(&self, item: &Item) -> bool {
        item.visible && !self.hidden_by_parent_eye(item)
    }

    /// A copy whose items' own `visible` flags carry the hiding they get
    /// from a parent's eye (GV-01), for readers that only look at the
    /// flag: render and file export. Meshes are shared, so it's cheap.
    pub fn resolved_eyes(&self) -> Scene {
        let mut out = self.clone();
        if !self.items.iter().any(|i| !i.visible && i.hides_children) {
            return out;
        }
        for (o, i) in out.items.iter_mut().zip(&self.items) {
            o.visible = self.shows(i);
            o.hides_children = false;
        }
        out
    }

    /// Whether an ancestor's eye is off with `hides_children` (GV-01).
    pub fn hidden_by_parent_eye(&self, item: &Item) -> bool {
        let mut parent = item.parent;
        // Acyclic (Scene::validate); the bound only guards a corrupt scene.
        for _ in 0..self.items.len() {
            let Some(p) = parent.and_then(|p| self.item(p)) else {
                return false;
            };
            if !p.visible && p.hides_children {
                return true;
            }
            parent = p.parent;
        }
        false
    }

    /// Whether an ancestor's Visible is "No (Hide Children)".
    fn hidden_by_ancestor(&self, item: &Item) -> bool {
        let mut parent = item.parent;
        // The parent chain is acyclic (Scene::validate); the bound only
        // guards a corrupt scene.
        for _ in 0..self.items.len() {
            let Some(p) = parent.and_then(|p| self.item(p)) else {
                return false;
            };
            if p.common.display.visible == DisplayVisible::NoHideChildren {
                return true;
            }
            parent = p.parent;
        }
        false
    }

    /// Item ids hidden in 3D views whatever the view's toggles and their
    /// own eye: their Display settings (Visible No, or under No (Hide
    /// Children)), or an ancestor's eye that hides its children (GV-01).
    /// Readers check the item's own `visible` alongside this. Meshes use
    /// it to leave the viewport; it's empty in the common case.
    /// `display_hidden().contains(&id)` for one item, without visiting the
    /// others (a parent eye that hides children is itself unseen, so the
    /// whole-scene check `display_hidden` starts with is implied).
    pub fn is_display_hidden(&self, id: u64) -> bool {
        self.item(id).is_some_and(|i| {
            matches!(
                i.common.display.visible,
                DisplayVisible::No | DisplayVisible::NoHideChildren
            ) || self.hidden_by_ancestor(i)
                || self.hidden_by_parent_eye(i)
        })
    }
    pub fn display_hidden(&self) -> std::collections::BTreeSet<u64> {
        let eyes = self.items.iter().any(|i| !i.visible && i.hides_children);
        if !eyes && self.items.iter().all(|i| i.common.is_default()) {
            return Default::default();
        }
        self.items
            .iter()
            .filter(|i| {
                matches!(
                    i.common.display.visible,
                    DisplayVisible::No | DisplayVisible::NoHideChildren
                ) || self.hidden_by_ancestor(i)
                    || (eyes && self.hidden_by_parent_eye(i))
            })
            .map(|i| i.id)
            .collect()
    }

    /// Apply `edit` to every item in `ids` (IC-2). All or nothing: when any
    /// item refuses, none changes.
    pub fn apply_item_common(&mut self, ids: &[u64], edit: &ItemCommonEdit) -> Result<()> {
        if ids.is_empty() {
            return Err("Select an item".into());
        }
        if let ItemCommonEdit::Target(Some(target)) = edit {
            self.check_target(ids, *target)?;
        }
        if let ItemCommonEdit::Extra(e) = edit
            && let Some(to) = e.references()
        {
            if !self.item(to).is_some() {
                return Err("Choose an item".into());
            }
            if ids.contains(&to) {
                return Err("An item can't point at itself".into());
            }
        }
        let mut next = Vec::with_capacity(ids.len());
        for id in ids {
            let index = self.item_index(*id).ok_or("Missing item")?;
            let mut common = self.items[index].common.clone();
            common.apply(edit)?;
            next.push((index, common));
        }
        for (index, common) in next {
            self.items[index].common = common;
        }
        Ok(())
    }

    /// CP-A02: an error naming the first locked item among `ids`.
    pub fn refuse_locked(&self, ids: &[u64]) -> Result<()> {
        match self
            .items
            .iter()
            .find(|i| ids.contains(&i.id) && i.is_locked())
        {
            Some(item) => Err(format!("{} is locked (Assembly > Lock)", item.name)),
            None => Ok(()),
        }
    }
}
