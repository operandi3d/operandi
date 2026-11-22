//! Topology Mode and the Topology tab's settings (missing-tools-g3
//! G3-TOPOLOGY-TAB v1, MT7-MT10).
//!
//! MT7 Topology Mode is a per-view toggle (`view.topologyMode`, saved with
//! the view's Drawing settings). While it is on, the view's active meshes
//! (the foreground) draw over the background meshes: their polygons
//! see-through and flat in Topo Face at the Foreground Opacity (the
//! Transparency Override's Overlay fill), their wire and points ignoring
//! depth (gpu.rs `topology` pipelines) in Topo Edge, Topo Back Edge (for
//! polygons facing away) and Topo Vertices. Background meshes draw as
//! usual. Display only: selection, snapping and tools are unchanged.
//!
//! MT9 Foreground Opacity and MT10 Selection Hit Size are preferences
//! (`pref.value display.topology_opacity`, `selection.hit_size`); MT8
//! Topology Offset was dropped (Jason, 2026-11-21); the Topology tab shows them as number fields
//! that run those lines (UIC-164), an escape-hatch widget each because
//! forms have no number control yet.
use super::*;
use crate::theme::ColorId;
use crate::view_display::{Overlay, Transparency, TransparencyMode};

/// The `pref.value` key each Topology tab field sets.
pub(crate) const OPACITY_KEY: &str = "display.topology_opacity";
pub(crate) const HIT_SIZE_KEY: &str = "selection.hit_size";

/// The active group's look in Topology Mode (MT7-02): the faces as an
/// Overlay see-through fill in Topo Face at `opacity` percent, flat; the
/// plain wire in Topo Edge at full strength, with Topo Back Edge carried
/// in the inactive-wire row for the topology edge shader.
pub(crate) fn topology_look(look: crate::gpu::Look, opacity: u8) -> crate::gpu::Look {
    let rgb = |id| {
        let [r, g, b, _] = crate::theme::color(id).to_array();
        [r, g, b]
    };
    let face = rgb(ColorId::TopoFace);
    crate::gpu::Look {
        overlay: Overlay::Colored,
        colored: crate::theme::color(ColorId::TopoEdge),
        opacity: 1.,
        plain_wire: Some(crate::theme::color(ColorId::TopoBackEdge)),
        transparency: Transparency {
            mode: TransparencyMode::Ghost,
            front_color: face,
            front_opacity: opacity.min(100),
            back_color: face,
            back_opacity: opacity.min(100),
            overlay: true,
            flat: true,
        },
        ..look
    }
}

impl Operandi {
    /// Whether the active view is in Topology Mode.
    pub(super) fn topology_mode(&self) -> bool {
        self.display.settings.drawing.topology
    }

    /// `view.topologyMode [on|off]`: bare toggles (MT7-05).
    pub(super) fn set_topology_mode(&mut self, rest: &str) {
        let on = match rest.trim().to_ascii_lowercase().as_str() {
            "" => !self.topology_mode(),
            "1" | "true" | "on" => true,
            "0" | "false" | "off" => false,
            other => {
                self.status = format!("view.topologyMode: on or off, not {other}");
                return;
            }
        };
        self.display.settings.drawing.topology = on;
        self.status = format!("Topology Mode {}", if on { "on" } else { "off" });
    }

    /// The Topology tab's Foreground Opacity field (MT9), a percent.
    pub(super) fn topology_opacity_field(&mut self, ui: &mut Ui) {
        let d = crate::viewport_preferences::ViewportPreferences::default();
        let mut v = self.viewport_preferences.topology_opacity;
        crate::form::rows::number_d(
            ui,
            "Foreground Opacity",
            &mut v,
            d.topology_opacity,
            crate::form::Unit::Percent,
            0..=100,
            "How solid the active meshes' faces draw in Topology Mode",
        );
        if v != self.viewport_preferences.topology_opacity {
            // Percents are 0–1 at the command boundary.
            self.run_command_line(&format!("pref.value {OPACITY_KEY} {}", f64::from(v) / 100.));
        }
    }

    /// The Topology tab's Selection Hit Size field (MT10): the Selection
    /// preference's second front door.
    pub(super) fn topology_hit_size_field(&mut self, ui: &mut Ui) {
        let d = crate::selection_preferences::SelectionPreferences::default();
        let mut v = self.selection_preferences.hit_size;
        crate::form::rows::number_d(
            ui,
            "Selection Hit Size",
            &mut v,
            d.hit_size,
            crate::form::Unit::Suffix(" px"),
            crate::selection_preferences::HIT_SIZE_RANGE,
            "How far from an element a click still picks it, in points",
        );
        if v != self.selection_preferences.hit_size {
            self.run_command_line(&format!("pref.value {HIT_SIZE_KEY} {v}"));
        }
    }
}
