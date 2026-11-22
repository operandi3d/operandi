//! Selection clicks in one place (AR-23, F21; master plan phase 0 door
//! 6): what a viewport pick does to the selection.
//!
//! The viewport finds what is under the pointer (a point, edge, polygon or
//! item). Every selecting gesture then turns that hit into a selection
//! change here: a plain click, Shift and Ctrl clicks, paint selection,
//! lasso, Mouse Regions right-clicks, Selection Fill's single pick. So a
//! later selection style (object-first: the first click picks the object,
//! then its components) changes this one function, not every gesture.
use crate::{Command, Component, Scene};

/// How a click combines with the selection (Shift adds, Ctrl removes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClickHow {
    Replace,
    Add,
    Remove,
}

impl ClickHow {
    /// The usual keys: Ctrl removes, Shift adds, neither replaces.
    pub fn from_keys(shift: bool, ctrl: bool) -> Self {
        if ctrl {
            ClickHow::Remove
        } else if shift {
            ClickHow::Add
        } else {
            ClickHow::Replace
        }
    }
}

/// One click's evidence: what was under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Click {
    pub hit: Option<(u64, Component)>,
    pub how: ClickHow,
    /// Nothing was hit only because picking skips other meshes (Pick
    /// Other Meshes in Component Modes off) and one is under the pointer.
    pub over_skipped_mesh: bool,
}

/// What a click does.
#[derive(Clone)]
pub enum Outcome {
    /// Run this command (one selection step).
    Run(Box<Command>),
    /// Nothing changes.
    Nothing,
    /// Nothing changes, because only the active mesh picks here; the
    /// status line says so.
    OnlyActiveMeshPicks,
}

/// The selection change `click` makes in `scene`.
pub fn resolve(scene: &Scene, click: Click) -> Outcome {
    match click.hit {
        Some((item, hit)) => {
            // CP-A03: an item's Redirect is selected in its place.
            let item = if hit == Component::Item {
                scene.redirected(item)
            } else {
                item
            };
            Outcome::Run(Box::new(Command::Select {
                item,
                hit,
                replace: click.how == ClickHow::Replace,
                remove: click.how == ClickHow::Remove,
            }))
        }
        // Only a plain click in empty space clears; a click on another
        // mesh while picking is limited to the active one leaves the
        // selection alone.
        None if click.how != ClickHow::Replace => Outcome::Nothing,
        None if click.over_skipped_mesh => Outcome::OnlyActiveMeshPicks,
        None => Outcome::Run(Box::new(Command::ClearSelection)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn click(hit: Option<(u64, Component)>, how: ClickHow) -> Click {
        Click {
            hit,
            how,
            over_skipped_mesh: false,
        }
    }

    #[test]
    fn keys_map_to_replace_add_remove() {
        assert_eq!(ClickHow::from_keys(false, false), ClickHow::Replace);
        assert_eq!(ClickHow::from_keys(true, false), ClickHow::Add);
        assert_eq!(ClickHow::from_keys(false, true), ClickHow::Remove);
        assert_eq!(ClickHow::from_keys(true, true), ClickHow::Remove);
    }

    #[test]
    fn a_hit_selects_and_empty_space_clears_only_on_a_plain_click() {
        let scene = Scene::default();
        let hit = Some((7, Component::Polygon(3)));
        let run = |c: Click| match resolve(&scene, c) {
            Outcome::Run(command) => Some(*command),
            _ => None,
        };
        assert!(matches!(
            run(click(hit, ClickHow::Add)),
            Some(Command::Select {
                item: 7,
                hit: Component::Polygon(3),
                replace: false,
                remove: false,
            })
        ));
        assert!(matches!(
            run(click(hit, ClickHow::Remove)),
            Some(Command::Select {
                remove: true,
                replace: false,
                ..
            })
        ));
        assert!(matches!(
            run(click(None, ClickHow::Replace)),
            Some(Command::ClearSelection)
        ));
        assert!(matches!(
            resolve(&scene, click(None, ClickHow::Add)),
            Outcome::Nothing
        ));
        let mut skipped = click(None, ClickHow::Replace);
        skipped.over_skipped_mesh = true;
        assert!(matches!(
            resolve(&scene, skipped),
            Outcome::OnlyActiveMeshPicks
        ));
    }
}
