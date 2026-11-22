//! The Topology tab (TOOLBOX-TOPOLOGY-CURVE TB-10/11): its form is
//! `assets/forms/toolbox.topology.json`. Its tool buttons are the same
//! tools as elsewhere with Constrain to Background switched on as part of
//! the button (TB-11); Pen and Topology Sketch also turn snapping on.
//! Topology Pen instead turns snapping off while it is up and gives both
//! back when it drops (TP-04, `topology_tab_pen`). `operandi.topology.tool <tool>` does that, then starts the
//! tool.
//!
//! Provisional (TB-Q1 in docs/SPECIFICATION_QUESTIONS.md): the constraint
//! and snapping stay on after the tool drops (the Settings section's
//! Constrain to Background button turns it off), and Grow Quads and
//! Subdivide don't project their results onto the background yet (TB-12).
use super::*;

/// The tools a Topology button may start, by the line the button runs,
/// and whether it turns snapping on.
const TOOLS: &[(&str, bool)] = &[
    ("Topology Pen", false),
    ("tool.set prim.pen on", true),
    ("Sketch", true),
    ("Bridge", false),
    ("Element Move", false),
    ("Slide", false),
    ("Smooth", false),
    ("Edge Slice", false),
    ("Loop Slice", false),
    ("Add Loop", false),
];

impl Operandi {
    /// `operandi.topology.tool <line>`: background constraint on (when
    /// there is a background mesh to constrain to), snapping on for the
    /// drawing tools, then the tool itself.
    pub(super) fn topology_tool_line(&mut self, rest: &str) {
        let line = rest.trim().trim_matches('"');
        let Some(&(tool, snap)) = TOOLS.iter().find(|(t, _)| t.eq_ignore_ascii_case(line)) else {
            self.status = format!(
                "operandi.topology.tool takes one of: {}",
                TOOLS.iter().map(|t| t.0).collect::<Vec<_>>().join(", ")
            );
            return;
        };
        if tool == "Topology Pen" {
            return self.topology_tab_pen();
        }
        if self.editor.background_constraint.is_none() {
            self.const_bg_set("on");
        }
        if snap && !self.snapping.enabled {
            self.toggle_snapping();
        }
        self.execute(tool);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_topology_button_turns_the_constraint_on_and_starts_its_tool() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.run(Command::Mode(Mode::Polygon));
        a.execute("operandi.topology.tool Sketch");
        assert!(a.editor.background_constraint.is_some(), "{}", a.status);
        assert!(a.snapping.enabled, "the drawing tools turn snapping on");
        assert!(!a.status.contains("Unknown"), "{}", a.status);
        a.execute("operandi.topology.tool Nope");
        assert!(a.status.contains("takes one of"));
    }
}
