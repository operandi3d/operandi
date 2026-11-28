//! Layouts stage 5: per-layout key maps (LAY-20, LAY-71, UIC-201).
//!
//! Each layout carries its own key map context, the `keys` of its layout
//! file: Input Map action -> chord, where an empty chord unbinds the action
//! in that layout. Keys a layout doesn't set come from the global input map
//! (input-map.json). Shipped layouts have no keys, so until a layout sets
//! some nothing changes.
//!
//! `Operandi::bindings` is the map that really fires in the current
//! layout: the global map with the layout's keys on top ([`layered`]).
//! Every key hint (button and menu shortcuts, toolbox `{binding}`
//! placeholders, pie labels) and the shortcut dispatcher read it, so they
//! all follow a layout switch. `Operandi::global_bindings` is the global
//! map as saved.
//!
//! Lookup order (provisional, LAY-Q55): the selection mode, tool state,
//! pointer and Render-window conditions the dispatcher already checks stay
//! as they are. Within them a layout key wins over the global one: setting
//! a chord on action A in a layout hides, in that layout only, every global
//! action with the same chord that can apply at the same time as A
//! ([`hidden`]). Global actions sharing the chord in other contexts (Edge
//! Bevel's B beside Polygon Bevel, the Render window's keys) keep working.
use crate::input_map_editor::same_chord;
use crate::{Operandi, input};
use std::collections::BTreeMap;

pub type Keys = BTreeMap<String, String>;

/// The layout's keys the app can use: actions it knows, chords that read
/// (or empty, unbinding). Unknown entries stay in the file but do nothing.
fn usable<'a>(
    global: &'a Keys,
    layout: &'a Keys,
) -> impl Iterator<Item = (&'a String, &'a String)> + 'a {
    layout.iter().filter(|(action, chord)| {
        // A user pie's line may have no global key at all (UIC-311).
        (global.contains_key(*action) || crate::pie_menu::is_pie(action))
            && (chord.is_empty() || input::chord(chord).is_some())
    })
}

/// Global actions a layout's keys hide in that layout: action -> the layout
/// action whose key now fires instead.
pub fn hidden(global: &Keys, layout: &Keys) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if layout.is_empty() {
        return out;
    }
    for (action, chord) in global {
        if chord.is_empty() || layout.contains_key(action) {
            continue;
        }
        if let Some((winner, _)) = usable(global, layout).find(|(other, theirs)| {
            *other != action && same_chord(theirs, chord) && input::contexts_overlap(other, action)
        }) {
            out.insert(action.clone(), winner.clone());
        }
    }
    out
}

/// The keys that really fire in a layout: `global` with `layout` on top.
pub fn layered(global: &Keys, layout: &Keys) -> Keys {
    let mut out = global.clone();
    if layout.is_empty() {
        return out;
    }
    for (action, chord) in usable(global, layout) {
        out.insert(action.clone(), chord.clone());
    }
    for action in hidden(global, layout).into_keys() {
        out.insert(action, String::new());
    }
    out
}

/// What a row of the Input Map shows about the current layout's keys.
#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    /// The layout sets this action's key; `global` is the global one.
    Set { global: String },
    /// The layout's key for `by` hides this action's global key.
    Hidden { by: String },
}

/// The marker for `action`, if the layout's keys touch it.
pub fn mark(global: &Keys, layout: &Keys, action: &str) -> Option<Mark> {
    if usable(global, layout).any(|(a, _)| a == action) {
        return Some(Mark::Set {
            global: global.get(action).cloned().unwrap_or_default(),
        });
    }
    hidden(global, layout)
        .remove(action)
        .map(|by| Mark::Hidden { by })
}

impl Operandi {
    /// The current layout's own keys (empty for shipped layouts).
    pub(crate) fn current_layout_keys(&self) -> Keys {
        let mut keys = self
            .layouts
            .find(&self.layouts.current)
            .map(|e| e.doc().keys.clone())
            .unwrap_or_default();
        // A layout saved before pies were forms names their old actions.
        crate::pie_menu::migrate_keys(&mut keys);
        keys
    }

    /// Make `bindings` the global map with the current layout's keys on
    /// top. Called whenever either changes.
    pub(crate) fn refresh_bindings(&mut self) {
        let keys = self.current_layout_keys();
        self.bindings = layered(&self.global_bindings, &keys);
        self.layout_keys_applied = keys;
    }

    /// Each frame: follow the current layout's keys when they changed
    /// (a switch, a revert, a delete). Layouts with the same keys (two with
    /// none) leave `bindings` alone.
    pub(crate) fn sync_layout_keys(&mut self) {
        let same = self
            .layouts
            .find(&self.layouts.current)
            .map_or(self.layout_keys_applied.is_empty(), |e| {
                e.doc().keys == self.layout_keys_applied
            });
        if !same {
            self.refresh_bindings();
        }
    }

    /// Put `keys` on the layout called `name` and write its file: the
    /// user's copy, made now if the layout had none (LAY-40/41).
    pub(crate) fn set_layout_keys(&mut self, name: &str, keys: Keys) -> Result<(), String> {
        let entry = self
            .layouts
            .find(name)
            .ok_or_else(|| format!("No layout named {name}"))?;
        if entry.doc().keys == keys {
            return Ok(());
        }
        let name = entry.name().to_owned();
        let mut doc = entry.doc().clone();
        doc.keys = keys;
        let same = entry.shipped.as_ref().is_some_and(|s| s.same_as(&doc));
        let active = name == self.layouts.current;
        if !active {
            if same {
                let path = self.layout_file(&name).ok_or("No layouts folder")?;
                match std::fs::remove_file(path) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(format!("Could not remove layout {name}: {e}")),
                }
            } else {
                self.write_user_layout(&doc)
                    .map_err(|e| format!("Could not save layout {name}: {e}"))?;
            }
        }
        self.layouts.find_mut(&name).expect("found above").user = (!same).then_some(doc);
        if active {
            // The next write captures the window with these keys.
            self.layouts_saved_clear();
            self.persist_layout_file();
        }
        self.refresh_bindings();
        Ok(())
    }

    /// `operandi.layout.setKey action:<action> [key:<chord>|none] [layout:]`:
    /// set one key in a layout's context; `none` unbinds the action there,
    /// no key gives it back to the global map.
    pub(crate) fn set_layout_key(
        &mut self,
        action: &str,
        key: Option<&str>,
        layout: &str,
    ) -> Result<(), String> {
        // Any pie's line too (UIC-311), a user pie's included.
        let pie_line = crate::pie_menu::pie_id(action)
            .filter(|id| self.pie_named(id).is_some())
            .map(|id| crate::pie_menu::line(&id));
        let Some(action) = input::ACTIONS
            .iter()
            .find(|(a, _)| a.eq_ignore_ascii_case(action.trim()))
            .map(|(a, _)| (*a).to_owned())
            .or(pie_line)
        else {
            return Err(format!(
                "operandi.layout.setKey: no Input Map action {action}"
            ));
        };
        let action = action.as_str();
        let chord = match key.map(str::trim) {
            None => None,
            Some(k) if k.eq_ignore_ascii_case("none") || k.is_empty() => Some(String::new()),
            Some(k) if input::chord(k).is_some() => Some(k.to_owned()),
            Some(k) => return Err(format!("operandi.layout.setKey: {k} isn't a key")),
        };
        if let Some(chord) = &chord
            && input::reserved_file_chord(action, chord)
        {
            return Err(format!(
                "{} is reserved for the File menu",
                input::display_chord(chord)
            ));
        }
        let mut keys = self
            .layouts
            .find(layout)
            .map(|e| e.doc().keys.clone())
            .ok_or_else(|| format!("No layout named {layout}"))?;
        match &chord {
            Some(chord) => {
                keys.insert(action.to_owned(), chord.clone());
            }
            None => {
                keys.remove(action);
            }
        }
        let clashes = crate::input_map_editor::problems(&layered(&self.global_bindings, &keys));
        if let Some(others) = clashes.get(action).filter(|o| !o.is_empty()) {
            return Err(format!(
                "{} is already {}'s key in the {layout} layout",
                chord
                    .as_deref()
                    .map(input::display_chord)
                    .unwrap_or_default(),
                others.join(", ")
            ));
        }
        self.set_layout_keys(layout, keys)?;
        self.status = match chord {
            None => format!("{action} uses the global key in {layout}"),
            Some(c) if c.is_empty() => format!("{action} has no key in {layout}"),
            Some(c) => format!("{action}: {} in {layout}", input::display_chord(&c)),
        };
        Ok(())
    }

    /// `operandi.layout.clearKeys [layout:]`: the layout's keys go; it
    /// uses the global map again.
    pub(crate) fn clear_layout_keys(&mut self, layout: &str) -> Result<(), String> {
        self.set_layout_keys(layout, Keys::new())?;
        self.status = format!("{layout} uses the global keys");
        Ok(())
    }
}
