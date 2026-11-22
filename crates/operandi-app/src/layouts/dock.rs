//! Dock tree persistence and the compatibility projection for stock columns.
use super::TOOLBOX;
use operandi_layout::{Dir, EMPTY, Frame, Part, Slot};

const TOOL_PROPERTIES: &str = "Tool Properties";

/// Version 1 layouts stored the view area's contents directly in the middle
/// column. Mark that subtree as a dock leaf without changing its contents.
pub(in crate::layouts) fn migrate(tree: &mut Frame) {
    if views_mut(tree).is_some() {
        return;
    }
    if let Ok(columns) = Columns::read(tree) {
        let views = std::mem::replace(&mut columns.view.frame, Frame::Panel(Slot::new(EMPTY)));
        columns.view.frame = Frame::ViewArea {
            frame: Box::new(views),
        };
    }
}

/// A view-area edit addresses the dock leaf, independently of its position.
pub(crate) fn views_mut(tree: &mut Frame) -> Option<&mut Frame> {
    match tree {
        Frame::ViewArea { frame } => Some(frame),
        Frame::Split { children, .. } => children
            .iter_mut()
            .find_map(|part| views_mut(&mut part.frame)),
        _ => None,
    }
}

/// Borrow the parts that the current renderer knows how to display. The
/// three-column constraint belongs here. A dock tree can represent other
/// arrangements, and the view inside its leaf still uses its own layout.
pub(in crate::layouts) struct Columns<'a> {
    pub left_width: &'a mut f32,
    pub right_width: &'a mut f32,
    pub view: &'a mut Part,
    pub toolbox: &'a mut Part,
    pub tool_props: &'a mut Part,
    pub upper: &'a mut Part,
    pub lower: &'a mut Part,
}

fn pair(frame: &mut Frame) -> Option<(&mut Part, &mut Part)> {
    let Frame::Split {
        dir: Dir::Column,
        children,
        ..
    } = frame
    else {
        return None;
    };
    let [first, second] = children.as_mut_slice() else {
        return None;
    };
    Some((first, second))
}

fn is_group(frame: &Frame) -> bool {
    match frame {
        Frame::Tabs { .. } => true,
        Frame::Panel(slot) => !operandi_layout::is_view_kind(&slot.kind),
        _ => false,
    }
}

impl<'a> Columns<'a> {
    pub fn read(tree: &'a mut Frame) -> Result<Self, String> {
        let wrong = || {
            "This build lays out only the three columns (toolbox | views | two tab groups, UIC-196)"
                .to_string()
        };
        let Frame::Split {
            dir: Dir::Row,
            children,
            ..
        } = tree
        else {
            return Err(wrong());
        };
        let [left, view, right] = children.as_mut_slice() else {
            return Err(wrong());
        };
        let (toolbox, tool_props) = pair(&mut left.frame).ok_or_else(wrong)?;
        if !matches!(&toolbox.frame, Frame::Panel(slot) if slot.kind == TOOLBOX)
            || !matches!(&tool_props.frame, Frame::Panel(slot) if slot.kind == TOOL_PROPERTIES)
        {
            return Err(wrong());
        }
        let (upper, lower) = pair(&mut right.frame).ok_or_else(wrong)?;
        if !is_group(&upper.frame) || !is_group(&lower.frame) {
            return Err(wrong());
        }
        Ok(Self {
            left_width: &mut left.fraction,
            right_width: &mut right.fraction,
            view,
            toolbox,
            tool_props,
            upper,
            lower,
        })
    }
}

use operandi_layout::{DockPanel, Layout, Pane};

// A legacy main window can share a saved layout with a modern floating pane.
pub(super) fn promote_columns(columns: Columns<'_>) {
    if let Frame::ViewArea { frame } = &mut columns.view.frame {
        columns.view.frame = std::mem::replace(frame.as_mut(), Frame::Panel(Slot::new(EMPTY)));
    }
    for (pane, part) in [
        (DockPanel::Toolbox, columns.toolbox),
        (DockPanel::ToolProperties, columns.tool_props),
        (DockPanel::Items, columns.upper),
        (DockPanel::Properties, columns.lower),
    ] {
        let frame = std::mem::replace(&mut part.frame, Frame::Panel(Slot::new(EMPTY)));
        part.frame = Frame::DockPanel {
            pane,
            frame: Box::new(frame),
        };
    }
}

pub(super) fn is_mixed(frame: &Frame) -> bool {
    match frame {
        Frame::DockPanel { .. } | Frame::DockTab(_) => true,
        Frame::Split { children, .. } => children.iter().any(|p| is_mixed(&p.frame)),
        Frame::Grid { children, .. } | Frame::DockTabs { children, .. } => {
            children.iter().any(is_mixed)
        }
        _ => false,
    }
}

pub(super) fn panel_frame(frame: &Frame, target: DockPanel) -> Option<&Frame> {
    match frame {
        Frame::DockPanel { pane, frame } if *pane == target => Some(frame),
        Frame::Split { children, .. } => {
            children.iter().find_map(|p| panel_frame(&p.frame, target))
        }
        Frame::Grid { children, .. } | Frame::DockTabs { children, .. } => {
            children.iter().find_map(|f| panel_frame(f, target))
        }
        _ => None,
    }
}

/// Materialize the shipped columns around the existing view tiles. Keep the
/// view keys, windows and tile settings when the first cross-panel move
/// replaces the three-column presentation (UIC-191, UIC-196).
pub(in crate::layouts) fn attach_columns(
    tree: &mut Layout,
    side: (f32, f32),
    tool_split: f32,
    scene_fraction: f32,
) -> Result<(), String> {
    use egui_tiles::{Linear, LinearDir};
    let centre = tree.main.root.ok_or("No view area")?;
    let tiles = &mut tree.main.tiles;
    let toolbox = tiles.insert_pane(Pane::Toolbox);
    let tool_props = tiles.insert_pane(Pane::ToolProperties);
    let items = tiles.insert_pane(Pane::Items);
    let properties = tiles.insert_pane(Pane::Properties);
    let mut split = |dir, parts: &[(egui_tiles::TileId, f32)]| {
        let mut linear = Linear::new(dir, parts.iter().map(|(id, _)| *id).collect());
        for &(id, fraction) in parts {
            linear.shares.set_share(id, fraction.max(0.001));
        }
        tiles.insert_container(linear)
    };
    let left = split(
        LinearDir::Vertical,
        &[(toolbox, 1. - tool_split), (tool_props, tool_split)],
    );
    let right = split(
        LinearDir::Vertical,
        &[(items, scene_fraction), (properties, 1. - scene_fraction)],
    );
    tree.main.root = Some(split(
        LinearDir::Horizontal,
        &[
            (left, side.0),
            (centre, 1. - side.0 - side.1),
            (right, side.1),
        ],
    ));
    Ok(())
}

/// Project the views for a saved viewport group (UIC-192). Removing a
/// dock leaf leaves the surrounding view splits with their relative sizes.
pub(crate) fn view_frame(frame: &Frame) -> Option<Frame> {
    fn legacy(frame: &Frame) -> Option<&Frame> {
        match frame {
            Frame::ViewArea { frame } => Some(frame),
            Frame::Split { children, .. } => children.iter().find_map(|p| legacy(&p.frame)),
            _ => None,
        }
    }
    legacy(frame).cloned().or_else(|| project_views(frame))
}

fn project_views(frame: &Frame) -> Option<Frame> {
    match frame {
        Frame::ViewArea { frame } => Some((**frame).clone()),
        Frame::DockPanel { .. } | Frame::DockTab(_) => None,
        Frame::DockTabs { children, active } => {
            let active = children
                .iter()
                .take(*active)
                .filter(|f| project_views(f).is_some())
                .count();
            let mut children: Vec<_> = children.iter().filter_map(project_views).collect();
            match children.len() {
                0 => None,
                1 => children.pop(),
                len => Some(Frame::DockTabs {
                    children,
                    active: active.min(len - 1),
                }),
            }
        }
        Frame::Split {
            dir,
            locked,
            children,
        } => {
            let mut children: Vec<_> = children
                .iter()
                .filter_map(|p| {
                    Some(Part {
                        fraction: p.fraction,
                        frame: project_views(&p.frame)?,
                    })
                })
                .collect();
            match children.len() {
                0 => None,
                1 => children.pop().map(|p| p.frame),
                _ => Some(Frame::Split {
                    dir: *dir,
                    locked: *locked,
                    children,
                }),
            }
        }
        Frame::Grid {
            columns,
            column_fractions,
            row_fractions,
            children,
        } => {
            let mut views: Vec<_> = children.iter().filter_map(project_views).collect();
            match views.len() {
                0 => None,
                1 => views.pop(),
                _ => Some(Frame::Grid {
                    columns: *columns,
                    column_fractions: column_fractions.clone(),
                    row_fractions: row_fractions.clone(),
                    children: views,
                }),
            }
        }
        _ => Some(frame.clone()),
    }
}

/// Clear Layout replaces the first view slot and closes the other views,
/// leaving every side leaf where it is. The first view corresponds to the
/// old view-area slot when the layout still has the compatibility wrapper.
pub(crate) fn replace_views(frame: &mut Frame, replacement: &Frame) -> bool {
    if let Some(centre) = views_mut(frame) {
        let changed = *centre != *replacement;
        *centre = replacement.clone();
        return changed;
    }
    if !is_mixed(frame) || view_frame(frame).as_ref() == Some(replacement) {
        return false;
    }
    fn replace(frame: &mut Frame, next: &mut Option<Frame>) -> bool {
        match frame {
            Frame::DockPanel { .. } | Frame::DockTab(_) => true,
            Frame::Split { children, .. } => {
                children.retain_mut(|p| replace(&mut p.frame, next));
                if children.len() == 1 {
                    *frame = children.remove(0).frame;
                }
                !matches!(frame, Frame::Split { children, .. } if children.is_empty())
            }
            Frame::Grid { children, .. } => {
                children.retain_mut(|f| replace(f, next));
                if children.len() == 1 {
                    *frame = children.remove(0);
                }
                !matches!(frame, Frame::Grid { children, .. } if children.is_empty())
            }
            Frame::DockTabs { children, active } => {
                let old_active = *active;
                let mut index = 0;
                let mut kept_before_active = 0;
                children.retain_mut(|f| {
                    let keep = replace(f, next);
                    kept_before_active += usize::from(keep && index < old_active);
                    index += 1;
                    keep
                });
                *active = kept_before_active.min(children.len().saturating_sub(1));
                if children.len() == 1 {
                    *frame = children.remove(0);
                }
                !matches!(frame, Frame::DockTabs { children, .. } if children.is_empty())
            }
            _ => {
                if let Some(new) = next.take() {
                    *frame = new;
                    true
                } else {
                    false
                }
            }
        }
    }
    replace(frame, &mut Some(replacement.clone()))
}

// UIC-191 / UIC-157: visit the main tree first, then windows in saved order.
// Views are independent instances; only registered panels are singletons.
#[derive(Default)]
pub(super) struct PlacedPanels {
    leaves: Vec<DockPanel>,
    tabs: Vec<crate::panel_registry::PanelKind>,
}

impl PlacedPanels {
    fn keep_tab(&mut self, slot: &Slot) -> bool {
        let Some(kind) = super::kind_named(&slot.kind) else {
            return true;
        };
        if self.tabs.contains(&kind) {
            return false;
        }
        self.tabs.push(kind);
        true
    }

    pub(super) fn repair_group(&mut self, frame: &mut Frame) -> bool {
        match frame {
            Frame::Tabs { tabs, active, .. } => {
                retain_tabs(tabs, active, |slot| self.keep_tab(slot));
                !tabs.is_empty()
            }
            Frame::Panel(slot) => self.keep_tab(slot),
            _ => true,
        }
    }

    pub(super) fn repair_tree(&mut self, frame: &mut Frame) -> bool {
        match frame {
            Frame::DockPanel { pane, frame: group } => {
                let sidebar = matches!(pane, DockPanel::Items | DockPanel::Properties);
                if sidebar && !self.repair_group(group) {
                    return false;
                }
                if !self.leaves.contains(pane) {
                    self.leaves.push(*pane);
                    return true;
                }
                if !sidebar {
                    return false;
                }
                // The row identity is duplicated, but its remaining unique
                // tabs still have a place in the saved arrangement.
                *frame = match group.as_mut() {
                    Frame::Tabs { tabs, active, .. } => Frame::DockTabs {
                        children: tabs.drain(..).map(Frame::DockTab).collect(),
                        active: *active,
                    },
                    Frame::Panel(slot) => Frame::DockTab(slot.clone()),
                    _ => return false,
                };
                true
            }
            Frame::DockTab(slot) => self.keep_tab(slot),
            Frame::Split { children, .. } if !children.is_empty() => {
                children.retain_mut(|part| self.repair_tree(&mut part.frame));
                !children.is_empty()
            }
            Frame::Grid { children, .. } if !children.is_empty() => {
                children.retain_mut(|child| self.repair_tree(child));
                !children.is_empty()
            }
            Frame::DockTabs { children, active } if !children.is_empty() => {
                retain_tabs(children, active, |child| self.repair_tree(child));
                !children.is_empty()
            }
            _ => true,
        }
    }
}

fn retain_tabs<T>(tabs: &mut Vec<T>, active: &mut usize, mut keep: impl FnMut(&mut T) -> bool) {
    let mut index = 0;
    let mut kept_before_active = 0;
    tabs.retain_mut(|tab| {
        let keep = keep(tab);
        kept_before_active += usize::from(keep && index < *active);
        index += 1;
        keep
    });
    *active = kept_before_active.min(tabs.len().saturating_sub(1));
}
