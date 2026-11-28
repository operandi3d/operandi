//! The active-tool exit hint: while a tool is active, the viewport's
//! bottom-left line says how to finish or leave it ("Q / Escape: drop tool ·
//! Cancel: Tool Properties"). Keys come from the user's input map, so a
//! rebind shows at once; the viewport Escape owners mirror
//! `Operandi::keyboard` and drop_keys.rs. Space opens Mini Properties
//! (UIC-66), so it isn't a way out.
use super::*;

/// What Escape over the viewport does to the active tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Escape {
    Drops,
    Cancels,
}

/// The ways out of the active tool right now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Exits {
    /// Chords bound to Drop (and Drop Cube where it applies), as typed in
    /// the input map.
    pub drop_chords: Vec<String>,
    pub escape: Option<Escape>,
    /// Tool Properties has a Cancel button for it.
    pub cancel_button: bool,
}

/// The hint line for `exits`; `None` when there is no way to show.
pub(crate) fn hint(exits: &Exits) -> Option<String> {
    let mut drop: Vec<String> = Vec::new();
    for chord in &exits.drop_chords {
        let shown = input::display_chord(chord);
        if !drop.contains(&shown) {
            drop.push(shown);
        }
    }
    let escape = input::display_chord("Escape");
    if exits.escape == Some(Escape::Drops) && !drop.contains(&escape) {
        drop.push(escape.clone());
    }
    let mut parts = Vec::new();
    if !drop.is_empty() {
        parts.push(format!("{}: drop tool", drop.join(" / ")));
    }
    if exits.escape == Some(Escape::Cancels) {
        parts.push(format!("{escape}: cancel"));
    } else if exits.cancel_button {
        parts.push("Cancel: Tool Properties".into());
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

impl Operandi {
    /// Escape over the viewport drops the Extrude (i2) session's armed or
    /// selected handle (shared with `keyboard`).
    pub(super) fn i2_escape_owner(&self) -> bool {
        self.editor.i2_active()
            || (self.editor.i2.selected.is_some()
                && self
                    .editor
                    .session
                    .live
                    .as_ref()
                    .is_none_or(|l| !l.applied && l.tool == Tool::Extrude)
                && self.editor.session.cone.is_none()
                && self.editor.session.bridge.is_none()
                && self.editor.session.spike.is_none()
                && self.editor.session.ellipsoid.is_none()
                && self.editor.session.shape.is_none())
    }

    /// How the active tool can be left now, by the same rules as
    /// `keyboard` (the input map's Drop, the viewport Escape owners).
    pub(super) fn tool_exits(&self) -> Exits {
        let task = self.editor_tool_active();
        let armed = task || self.edge_click.is_some() || self.vertex_weld;
        let side_tool =
            self.utility.is_some() || self.sculpt.brush.is_some() || self.draw.is_some();
        let live_tool = self.editor.session.live.as_ref().map(|l| l.tool);
        let smooth_shift = live_tool.is_some_and(|t| t.is_smooth_shift());
        let bound = |action: &str| {
            self.bindings
                .get(action)
                .filter(|chord| !chord.is_empty())
                .cloned()
        };
        let mut drop_chords = Vec::new();
        if armed || side_tool {
            drop_chords.extend(bound("Drop"));
            // Drop Cube's chord (Escape by default) drops the Cube tool,
            // and Smooth Shift's only on the factory chord (SS-U03).
            let cube = bound("Drop Cube");
            let factory = input::ACTIONS
                .iter()
                .find(|(a, _)| *a == "Drop Cube")
                .map(|(_, c)| *c);
            if live_tool == Some(Tool::Box) || (smooth_shift && cube.as_deref() == factory) {
                drop_chords.extend(cube);
            }
        }
        let escape = if live_tool == Some(Tool::VertexCreate) {
            Some(Escape::Cancels)
        } else if self.i2_escape_owner()
            || self.vertex_weld
            || side_tool
            || self.escape_drops_tool()
        {
            Some(Escape::Drops)
        } else {
            None
        };
        Exits {
            drop_chords,
            escape,
            // Every editor task's Tool Properties ends in Drop Tool and
            // Cancel (live_footer and the session panels).
            cancel_button: task,
        }
    }

    /// The exit hint for the active tool, or `None` with no tool active.
    pub(super) fn tool_exit_hint(&self) -> Option<String> {
        hint(&self.tool_exits())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exits(chords: &[&str], escape: Option<Escape>, cancel: bool) -> Exits {
        Exits {
            drop_chords: chords.iter().map(|c| (*c).to_owned()).collect(),
            escape,
            cancel_button: cancel,
        }
    }

    #[test]
    fn the_hint_lists_the_drop_keys_then_the_way_to_cancel() {
        assert_eq!(
            hint(&exits(&["Q"], Some(Escape::Drops), true)).as_deref(),
            Some("Q / Escape: drop tool · Cancel: Tool Properties")
        );
        assert_eq!(
            hint(&exits(&["Q"], Some(Escape::Cancels), true)).as_deref(),
            Some("Q: drop tool · Escape: cancel")
        );
        assert_eq!(hint(&exits(&[], None, false)), None);
    }

    /// A one-cube scene with the cube selected.
    fn cube_app() -> Operandi {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        let item = a.editor.scene.active_mut().unwrap();
        item.mesh = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
            .unwrap()
            .into();
        let id = item.id;
        a.editor.synchronize_identities().unwrap();
        a.run(Command::SelectItem(Some(id)));
        a
    }

    #[test]
    fn no_tool_no_hint() {
        let a = cube_app();
        assert_eq!(a.tool_exit_hint(), None);
    }

    #[test]
    fn the_hint_follows_a_rebound_drop_key() {
        let mut a = cube_app();
        a.run(Command::Activate(Tool::Move));
        assert!(a.editor.session.live.is_some());
        let shown = a.tool_exit_hint().unwrap();
        assert!(shown.starts_with("Q / Escape: drop tool"), "{shown}");
        assert!(shown.ends_with("Cancel: Tool Properties"), "{shown}");
        a.bindings.insert("Drop".into(), "Shift+D".into());
        let shown = a.tool_exit_hint().unwrap();
        assert!(
            shown.starts_with(&format!(
                "{} / Escape: drop tool",
                input::display_chord("Shift+D")
            )),
            "{shown}"
        );
        a.bindings.insert("Drop".into(), String::new());
        assert!(a.tool_exit_hint().unwrap().starts_with("Escape: drop tool"));
    }

    #[test]
    fn the_cube_tool_also_drops_with_its_escape_chord() {
        let mut a = cube_app();
        a.run(Command::Activate(Tool::Box));
        let shown = a.tool_exit_hint().unwrap();
        assert!(shown.starts_with("Q / Escape: drop tool"), "{shown}");
    }
}
