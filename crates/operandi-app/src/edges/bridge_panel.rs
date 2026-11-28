//! BRI-R18 numeric S1 panel. Deferred families stay visible with scope reasons.
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, ToolFields, ToolForm};
use operandi_core::{BridgeMode, BridgeParameters};
#[derive(Default)]
pub(super) struct BridgeUi {
    eligibility: Option<(u64, BridgeParameters, Result<()>)>,
}
impl Operandi {
    pub(super) fn execute_bridge_parameter(&mut self, text: &str) -> bool {
        let lower = text.trim().to_ascii_lowercase();
        let Some(rest) = lower.strip_prefix("bridge ") else {
            return false;
        };
        let Some((name, _)) = rest.split_once(' ') else {
            return false;
        };
        if ![
            "segments", "twist", "tension", "mode", "remove", "flip", "reverse",
        ]
        .contains(&name)
        {
            return false;
        }
        let original = text.trim();
        let rest = original.split_once(' ').unwrap().1;
        let (_, value) = rest.split_once(' ').unwrap();
        self.bridge_property(
            if name == "reverse" { "orient" } else { name },
            value.trim(),
            true,
        );
        true
    }

    /// Shared Bridge property transport for legacy and canonical commands.
    pub(super) fn bridge_property(&mut self, name: &str, value: &str, legacy: bool) {
        let Some(session) = self.editor.session.bridge.as_ref() else {
            self.status = "Activate Bridge first".into();
            return;
        };
        let current = session.requested;
        if ![
            "segments", "twist", "mode", "remove", "flip", "orient", "tension",
        ]
        .contains(&name)
        {
            self.status = "Unknown Bridge attribute; use help tool.attr".into();
            return;
        }
        if value == "?" && !legacy {
            self.status = match name {
                "segments" => current.segments.to_string(),
                "twist" => current.twist.to_string(),
                "mode" => format!("{:?}", current.mode).to_ascii_lowercase(),
                "remove" => (current.remove as u8).to_string(),
                "flip" => (current.flip as u8).to_string(),
                "orient" => (current.reverse as u8).to_string(),
                _ => current.tension.to_string(),
            };
            return;
        }
        let parsed = (|| -> Result<BridgeParameters> {
            let mut p = current;
            match name {
                "segments" => {
                    p.segments = BridgeParameters::segments_field(numeric::parse_quantity(
                        value,
                        numeric::Quantity::Scalar,
                    )?)?
                }
                "twist" => {
                    p.twist = BridgeParameters::twist_field(numeric::parse_quantity(
                        value,
                        numeric::Quantity::Scalar,
                    )?)?
                }
                "tension" => {
                    p.tension = if legacy || value.contains('%') {
                        numeric::parse_quantity(value, numeric::Quantity::Percent)? / 100.
                    } else {
                        numeric::parse_quantity(value, numeric::Quantity::Scalar)?
                    }
                }
                "mode" => {
                    p.mode = match value.to_ascii_lowercase().as_str() {
                        "linear" => BridgeMode::Linear,
                        "curve" => BridgeMode::Curve,
                        "smooth" => BridgeMode::Smooth,
                        _ => return Err("Bridge Mode: Linear, Curve or Smooth".into()),
                    }
                }
                _ => {
                    let v = match value.to_ascii_lowercase().as_str() {
                        "1" | "true" | "on" => true,
                        "0" | "false" | "off" => false,
                        _ => return Err("Use on/off or true/false".into()),
                    };
                    match name {
                        "remove" => p.remove = v,
                        "flip" => p.flip = v,
                        _ => p.reverse = v,
                    }
                }
            }
            let mut staged = self.editor.clone();
            staged.dispatch(Command::ConfigureBridge(p))?;
            Ok(p)
        })();
        let p = match parsed {
            Ok(p) => p,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        if !self.confirm_fields() {
            return;
        }
        let mut accepted = self.editor.session.bridge.as_ref().unwrap().requested;
        match name {
            "segments" => accepted.segments = p.segments,
            "twist" => accepted.twist = p.twist,
            "mode" => accepted.mode = p.mode,
            "remove" => accepted.remove = p.remove,
            "flip" => accepted.flip = p.flip,
            "orient" => accepted.reverse = p.reverse,
            _ => accepted.tension = p.tension,
        }
        match self.editor.dispatch(Command::ConfigureBridge(accepted)) {
            Ok(()) => {
                match name {
                    "segments" => {
                        self.sync_numeric_value("bri_segments", accepted.segments as f64, "")
                    }
                    "twist" => self.sync_numeric_value("bri_twist", accepted.twist as f64, ""),
                    "tension" => {
                        self.sync_numeric_value("bri_tension", accepted.tension * 100., "%")
                    }
                    _ => {}
                }
                self.bridge_ui.eligibility = None;
                self.status = if self.editor.bridge_step_newest() {
                    "Bridge reshaped · Q keeps it".into()
                } else {
                    "Bridge configured · Apply uses the current ordered selection".into()
                };
            }
            Err(e) => self.status = e,
        }
    }
    /// The panel's Apply check, from the last computed result when the
    /// scene and parameters haven't changed since.
    fn bridge_eligibility_cached(&self) -> Result<()> {
        self.editor.bridge_eligibility()?;
        let p = self
            .editor
            .session
            .bridge
            .as_ref()
            .ok_or("Arm Bridge first")?
            .requested;
        if let Some((r, parameters, result)) = &self.bridge_ui.eligibility
            && *r == self.editor.revision
            && *parameters == p
        {
            return result.clone();
        }
        self.bridge_candidate(p)
    }
    /// Try Bridge on the current selection without changing anything: the
    /// selected edge runs in Edge mode, the selected polygons otherwise.
    fn bridge_candidate(&self, p: BridgeParameters) -> Result<()> {
        // What Apply would do, Symmetry's per-side pairs included (N28: the
        // panel's own check didn't know about them and greyed Apply).
        self.editor.bridged_scene(p).map(|_| ())
    }
    fn bridge_panel_eligibility(&mut self) -> Result<()> {
        self.editor.bridge_eligibility()?;
        let p = self
            .editor
            .session
            .bridge
            .as_ref()
            .ok_or("Arm Bridge first")?
            .requested;
        let rev = self.editor.revision;
        if let Some((r, parameters, result)) = &self.bridge_ui.eligibility
            && *r == rev
            && *parameters == p
        {
            return result.clone();
        }
        let result = self.bridge_candidate(p);
        self.bridge_ui.eligibility = Some((rev, p, result.clone()));
        result
    }
}

/// Why Flip Polygons and Reverse Direction grey out in Edge mode.
const EDGE_FACING: &str =
    "An edge bridge always faces like the polygons it joins, so this doesn't apply to edges";

/// Bridge's Tool Properties (BRI-R18).
pub(super) struct BridgeFields;

impl ToolFields for BridgeFields {
    type P = BridgeParameters;
    fn params(app: &Operandi) -> Option<BridgeParameters> {
        app.editor.session.bridge.as_ref().map(|s| s.requested)
    }
    fn form(app: &Operandi, _: &BridgeParameters) -> ToolForm<BridgeParameters> {
        let mut f = ToolForm::new();
        let later = |label: &str, value: &str, why: &str| tf::unbuilt(label, value, why);
        let edges = app.editor.scene.selection.mode == Mode::Edge;
        f.title("Bridge")
            .field(tf::number(
                "bri_segments",
                "Segments",
                Unit::Count,
                |p: &BridgeParameters| p.segments as f64,
                |p, v| {
                    p.segments = BridgeParameters::segments_field(v)?;
                    Ok(())
                },
            ))
            .field(
                tf::number(
                    "bri_twist",
                    "Twist",
                    Unit::Scalar,
                    |p: &BridgeParameters| p.twist as f64,
                    |p, v| {
                        p.twist = BridgeParameters::twist_field(v)?;
                        Ok(())
                    },
                )
                .suffix(""),
            )
            .field(tf::choice(
                "bridge-mode",
                "Mode",
                &[
                    (BridgeMode::Linear, "Linear"),
                    (BridgeMode::Curve, "Curve"),
                    (BridgeMode::Smooth, "Smooth"),
                ],
                |p: &BridgeParameters| p.mode,
                |p, m| p.mode = m,
            ))
            // UIC-92: in Edge mode these three grey out in place. An edge
            // bridge has no source polygons to remove, and it always faces
            // like the polygons it joins, so flipping or reversing it would
            // turn it inside out against them (BRI-Q08).
            .field(
                tf::check(
                    "bri_remove",
                    "Remove Polygons",
                    |p: &BridgeParameters| p.remove,
                    |p, on| p.remove = on,
                )
                .enabled_if(!edges, "An edge bridge has no source polygons to remove"),
            )
            .field(
                tf::check(
                    "bri_flip",
                    "Flip Polygons",
                    |p: &BridgeParameters| p.flip,
                    |p, on| p.flip = on,
                )
                .enabled_if(!edges, EDGE_FACING),
            )
            .field(
                tf::check(
                    "bri_reverse",
                    "Reverse Direction",
                    |p: &BridgeParameters| p.reverse,
                    |p, on| p.reverse = on,
                )
                .enabled_if(!edges, EDGE_FACING),
            )
            // Spec: BRI-Q03
            .push(later(
                "Auto Connection",
                "off",
                "Always off for now: automatic edge matching isn't available yet.",
            ))
            // Spec: BRI-Q01/03
            .push(later(
                "Continuous Bridges",
                "on",
                "On for a single two-section bridge; multi-section chains aren't available yet.",
            ))
            // Spec: BRI-Q03
            .push(later(
                "Use Guide Curve",
                "off",
                "Guide curves aren't available yet.",
            ))
            .push(later(
                "Close Bridges",
                "off",
                "Closed chains aren't available yet.",
            ))
            .field(tf::percent(
                "bri_tension",
                "Tension",
                "%",
                |p: &BridgeParameters| p.tension,
                |p, v| {
                    p.tension = v / 100.;
                    Ok(())
                },
            ))
            .separator();
        // Make UVs (BRI-Q04), Curve and Profiles (BRI-Q03/06) are left out
        // until built (UIC-117).
        // UIC-78: after Apply (or a click in the view) the bridge stays live
        // and the values reshape it. The cheap check: the newest step is
        // this session's Apply (the scene comparison runs on edits only).
        let live = app.editor.bridge_step_newest();
        let eligibility = app.bridge_eligibility_cached();
        f.wide(tf::Button::command("Apply", Command::Apply).enabled_if(
            eligibility.is_ok() && app.field_errors.is_empty(),
            match &eligibility {
                Err(e) => e.to_string(),
                Ok(()) => crate::unavailable::FIELD_ERRORS.to_owned(),
            },
        ));
        if live {
            f.note("Applied · values reshape this bridge; Q keeps it, Cancel removes it");
        } else if let Err(e) = eligibility {
            f.note(e);
        }
        // Spec: BRI-R19 (named tool states for Bridge unmeasured)
        f.unbuilt(
            "Tool States",
            "(none)",
            "Named Bridge tool states aren't available yet",
        )
        .buttons(vec![
            tf::Button::command("Drop", Command::Drop),
            tf::Button::command("Cancel", Command::Cancel),
            tf::Button::command("Reset Attributes", Command::ResetBridge),
        ])
        // Spec: BRI-Q05/06
        .note("Apply bridges two selected polygons or groups, or two selected border edges, edge runs or loops; both sides need the same number of edges. A click in the view applies too (UIC-78); dragging left/right sets Segments. Presets aren't available yet.");
        f
    }
    fn prepare(app: &mut Operandi) {
        let _ = app.bridge_panel_eligibility();
    }
    fn publish(app: &mut Operandi, p: BridgeParameters) -> Result<()> {
        app.editor.dispatch(Command::ConfigureBridge(p))?;
        app.bridge_ui.eligibility = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// N28: with Symmetry, the panel's Apply check accepts a pair of border
    /// edges picked with their mirrors (four edges), as Apply does.
    #[test]
    fn n28_panel_check_knows_symmetry() {
        let mut a = Operandi::empty();
        a.editor.dispatch(Command::NewMesh).unwrap();
        let m = &mut a.editor.scene.active_mut().unwrap().mesh;
        let mut quad = |x0: f64, x1: f64, y0: f64, y1: f64| {
            let base = m.vertices.len() as u32;
            m.vertices.extend([
                DVec3::new(x0, y0, 0.),
                DVec3::new(x1, y0, 0.),
                DVec3::new(x1, y1, 0.),
                DVec3::new(x0, y1, 0.),
            ]);
            m.faces.push(operandi_core::Face {
                vertices: vec![base, base + 1, base + 2, base + 3],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        };
        for (x0, x1) in [(1., 2.), (-2., -1.)] {
            quad(x0, x1, 0., 1.);
            quad(x0, x1, 1.2, 2.2);
        }
        a.editor.dispatch(Command::Mode(Mode::Edge)).unwrap();
        // Each side's facing edges: 2-3 / 4-5 and 10-11 / 12-13.
        a.editor.scene.selection.edges = vec![(2, 3), (10, 11), (4, 5), (12, 13)];
        a.editor.dispatch(Command::ArmBridge).unwrap();
        assert!(
            a.bridge_eligibility_cached().is_err(),
            "Symmetry off: four runs"
        );
        a.editor
            .dispatch(Command::Symmetry(operandi_core::Symmetry::x()))
            .unwrap();
        a.bridge_eligibility_cached().unwrap();
    }
    fn app() -> Operandi {
        let mut a = Operandi::empty();
        a.editor.dispatch(Command::NewMesh).unwrap();
        let m = &mut a.editor.scene.active_mut().unwrap().mesh;
        m.vertices = vec![
            DVec3::new(-2., -1., 0.),
            DVec3::new(3., -1., 0.),
            DVec3::new(2., 2., 0.),
            DVec3::new(-1., 1., 0.),
            DVec3::new(-1.5, -0.5, 4.),
            DVec3::new(2., -0.5, 4.),
            DVec3::new(1.5, 1.5, 4.),
            DVec3::new(-0.5, 1., 4.),
        ];
        for vertices in [vec![0, 1, 2, 3], vec![4, 7, 6, 5]] {
            m.faces.push(operandi_core::Face {
                vertices,
                uv: None,
                material: "Default".into(),
                material_label: None,
                part: None,
                smooth: false,
            });
        }
        a.editor.scene.selection.mode = Mode::Polygon;
        a.editor.scene.selection.faces = vec![0, 1];
        a.editor.history.undo.clear();
        a.execute("Bridge");
        a
    }
    #[test]
    fn s1_t10_t12_t13_numeric_command_menu_validation_and_history() {
        let mut a = app();
        let before = a.editor.scene.clone();
        a.fields.insert("bri_segments".into(), "1+2".into());
        a.fields.insert("bri_tension".into(), "25%*2".into());
        assert!(a.confirm_fields());
        assert_eq!(
            a.editor.session.bridge.as_ref().unwrap().requested.segments,
            3
        );
        assert_eq!(
            a.editor.session.bridge.as_ref().unwrap().requested.tension,
            0.5
        );
        assert_eq!(a.editor.scene, before);
        a.fields.insert("bri_twist".into(), "-2147483648".into());
        assert!(a.confirm_fields());
        assert_eq!(a.fields["bri_twist"], "-2147483647");
        a.fields.insert("bri_twist".into(), "bad".into());
        a.execute("bridge apply");
        assert_eq!(a.editor.scene, before);
        assert_eq!(a.fields["bri_twist"], "bad");
        assert!(a.editor.history.undo.is_empty());
        a.fields.insert("bri_twist".into(), "0".into());
        assert!(a.confirm_fields());
        a.execute("bridge mode smooth");
        a.execute("bridge tension 100%");
        let menus = a.menu_model(&Context::default());
        assert!(menus.iter().any(|m| {
            m.entries
                .iter()
                .any(|e| matches!(e,app_menu::Entry::Item(i) if i.action==app_menu::line_action("Bridge")))
        }));
        assert!(a.bridge_panel_eligibility().is_ok());
        a.execute("bridge apply");
        let after = a.editor.scene.clone();
        assert_eq!(after.active().unwrap().mesh.faces.len(), 12);
        assert!(a.bridge_panel_eligibility().is_err());
        a.execute("bridge apply");
        assert_eq!(a.editor.scene, after);
        assert_eq!(a.editor.history.undo.len(), 1);
        a.execute("undo");
        assert_eq!(a.editor.scene, before);
        a.execute("redo");
        assert_eq!(a.editor.scene, after);
        a.execute("bridge reset");
        assert_eq!(
            a.editor.session.bridge.as_ref().unwrap().requested,
            BridgeParameters::default()
        );
    }
    #[test]
    fn s1_t11_t14_display_and_picking_consume_current_cage_after_apply_history() {
        let mut a = app();
        a.execute("bridge segments 3");
        a.execute("bridge mode linear");
        a.camera.view = View::Right;
        a.camera.target = DVec3::new(0., 0., 2.);
        a.camera.distance = 8.;
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
        let point = a.camera.project(DVec3::new(2.3, 0.2, 2.), rect).unwrap().0;
        assert!(a.pick_component(point, rect).is_none());
        a.execute("bridge apply");
        assert!(a.pick_component(point, rect).is_some());
        a.execute("undo");
        assert!(a.pick_component(point, rect).is_none());
        a.execute("redo");
        assert!(a.pick_component(point, rect).is_some());
    }
    #[test]
    fn bc_canonical_alias_properties_queries_and_refusals() {
        let mut a = app();
        let mut b = app();
        let source = a.editor.scene.clone();
        for (canonical, legacy) in [
            ("tool.attr edge.bridge segments 3", "bridge segments 3"),
            ("tool.attr edge.bridge twist -1", "bridge twist -1"),
            ("tool.attr edge.bridge mode linear", "bridge mode linear"),
            ("tool.attr edge.bridge remove off", "bridge remove off"),
            ("tool.attr edge.bridge flip on", "bridge flip on"),
            ("tool.attr edge.bridge orient on", "bridge reverse on"),
            ("tool.attr edge.bridge tension 0.5", "bridge tension 50"),
        ] {
            a.execute(canonical);
            b.execute(legacy);
            assert_eq!(
                a.editor.session.bridge.as_ref().unwrap().requested,
                b.editor.session.bridge.as_ref().unwrap().requested,
                "{canonical}"
            );
        }
        assert_eq!(a.fields, b.fields);
        assert_eq!(a.editor.scene, source);
        a.fields.insert("bri_twist".into(), "invalid".into());
        let before = a.editor.clone();
        let fields = a.fields.clone();
        let confirmed = a.confirmed_fields.clone();
        for query in [
            "tool.set edge.bridge ?",
            "tool.attr edge.bridge tension ?",
            "help tool.attr",
            "tool.attr edge.bridge bogus 2",
            "tool.attr edge.bridge flip maybe",
            "tool.attr edge.bridge tension {bad",
            "tool.attr edge.bridge flip on extra",
            "tool.set prim.cube off",
        ] {
            a.execute(query);
            assert!(a.editor == before, "{query}");
            assert_eq!(a.fields, fields, "{query}");
            assert_eq!(a.confirmed_fields, confirmed, "{query}");
        }
        a.fields.insert("bri_twist".into(), "-1".into());
        assert!(a.confirm_fields());
        a.execute("tool.attr edge.bridge tension 50%");
        assert_eq!(
            a.editor.session.bridge.as_ref().unwrap().requested.tension,
            0.5
        );
        a.execute("tool.attr edge.bridge segments 0");
        assert_eq!(
            a.editor.session.bridge.as_ref().unwrap().requested.segments,
            0
        );
        a.execute("tool.attr edge.bridge orient ?");
        assert_eq!(a.status, "1");
        a.execute("tool.apply");
        let after = a.editor.scene.clone();
        assert_ne!(after, source);
        a.execute("undo");
        assert_eq!(a.editor.scene, source);
        a.execute("redo");
        assert_eq!(a.editor.scene, after);
        a.execute("tool.reset edge.bridge");
        assert_eq!(
            a.editor.session.bridge.as_ref().unwrap().requested,
            BridgeParameters::default()
        );
        a.execute("tool.set edge.bridge off");
        assert!(a.editor.session.bridge.is_none());
        a.execute("tool.set preset:edge.bridge mode:on");
        assert!(a.editor.session.bridge.is_some());
        a.execute("tool.set edge.bridge ?");
        assert_eq!(a.status, "1");
    }

    #[test]
    fn bc_activation_menu_cache_and_field_recovery() {
        let mut canonical = app();
        let source = canonical.editor.scene.clone();
        canonical.execute("tool.drop");
        let mut alias = app();
        alias.execute("tool.drop");
        canonical.execute("tool.set preset:edge.bridge mode:on");
        alias.execute("Bridge");
        assert!(canonical.editor.session.bridge.is_some());
        assert_eq!(canonical.editor.session.bridge, alias.editor.session.bridge);
        assert_eq!(canonical.editor.scene, source);
        let menu = canonical.menu_model(&Context::default());
        let bridge = menu
            .iter()
            .flat_map(|m| m.leaves())
            .filter(|i| i.action == app_menu::line_action("Bridge"))
            .collect::<Vec<_>>();
        assert_eq!(
            bridge.len(),
            3,
            "Geometry > Polygon, Tools and Geometry > Duplicate (DX-05)"
        );
        assert!(bridge.iter().all(|i| i.checked && i.enabled));
        assert!(canonical.bridge_panel_eligibility().is_ok());
        canonical.execute("tool.attr edge.bridge mode curve");
        canonical.execute("tool.attr edge.bridge mode smooth");
        assert_eq!(
            canonical
                .editor
                .session
                .bridge
                .as_ref()
                .unwrap()
                .requested
                .mode,
            BridgeMode::Smooth
        );
        canonical.execute("tool.attr edge.bridge tension -0.5");
        assert_eq!(
            canonical
                .editor
                .session
                .bridge
                .as_ref()
                .unwrap()
                .requested
                .tension,
            -0.5
        );
        canonical.fields.insert("bri_segments".into(), "bad".into());
        let before = canonical.editor.clone();
        canonical.execute("tool.attr edge.bridge tension infinity");
        assert!(canonical.editor == before);
        assert_eq!(canonical.fields["bri_segments"], "bad");
        canonical.execute("tool.apply");
        assert_eq!(canonical.editor.scene, source);
        canonical.fields.insert("bri_segments".into(), "3".into());
        canonical.execute("tool.apply");
        assert_ne!(canonical.editor.scene, source);
        assert!(canonical.bridge_panel_eligibility().is_err());
        canonical.execute("undo");
        assert!(canonical.bridge_panel_eligibility().is_ok());
        canonical.execute("tool.drop");
        canonical.execute("tool.attr edge.bridge tension 1");
        assert!(canonical.status.contains("Activate Bridge"));
    }
    #[test]
    fn bc_complete_property_queries_and_confirmed_fields() {
        let mut a = app();
        for (input, key, expected) in [
            ("tool.attr edge.bridge segments -1", "bri_segments", "0"),
            ("tool.attr edge.bridge segments 3.9", "bri_segments", "3"),
            ("tool.attr edge.bridge twist -1.9", "bri_twist", "-1"),
            ("tool.attr edge.bridge tension 1", "bri_tension", "100%"),
            ("tool.attr edge.bridge tension 0.5", "bri_tension", "50%"),
            ("tool.attr edge.bridge tension 50%", "bri_tension", "50%"),
            ("bridge tension 50", "bri_tension", "50%"),
        ] {
            a.execute(input);
            assert_eq!(a.fields[key], expected, "{input}");
            assert_eq!(a.confirmed_fields[key], expected, "{input}");
            let p = a.editor.session.bridge.as_ref().unwrap().requested;
            assert!(a.confirm_fields());
            assert_eq!(a.editor.session.bridge.as_ref().unwrap().requested, p);
        }
        for mode in ["linear", "curve", "smooth"] {
            a.execute(&format!("tool.attr edge.bridge mode {mode}"));
            a.execute("tool.attr edge.bridge mode ?");
            assert_eq!(a.status, mode);
        }
        for property in ["remove", "flip", "orient"] {
            for (input, expected) in [("on", "1"), ("off", "0")] {
                a.execute(&format!("tool.attr edge.bridge {property} {input}"));
                a.execute(&format!("tool.attr edge.bridge {property} ?"));
                assert_eq!(a.status, expected);
            }
        }
        a.execute("tool.attr edge.bridge segments 3.9");
        a.execute("tool.attr edge.bridge twist -1.9");
        a.execute("tool.attr edge.bridge tension 0.5");
        a.fields.insert("bri_segments".into(), "bad draft".into());
        a.name = "invalid pending name".into();
        let editor = a.editor.clone();
        let fields = a.fields.clone();
        let confirmed = a.confirmed_fields.clone();
        let name = a.name.clone();
        for (attr, expected) in [
            ("segments", "3"),
            ("twist", "-1"),
            ("mode", "smooth"),
            ("remove", "0"),
            ("flip", "0"),
            ("orient", "0"),
            ("tension", "0.5"),
        ] {
            a.execute(&format!("tool.attr edge.bridge {attr} ?"));
            assert_eq!(a.status, expected);
            assert!(a.editor == editor);
            assert_eq!(a.fields, fields);
            assert_eq!(a.confirmed_fields, confirmed);
            assert_eq!(a.name, name);
        }
    }

    #[test]
    fn bc_bridge_menu_history_and_failed_switch_preserve_state() {
        fn item(a: &Operandi, ctx: &Context, action: app_menu::Action) -> app_menu::Item {
            a.menu_model(ctx)
                .iter()
                .flat_map(|m| m.leaves())
                .find(|i| i.action == action)
                .unwrap()
                .clone()
        }
        let ctx = Context::default();
        let mut menu = app();
        let mut typed = app();
        let source = menu.editor.scene.clone();
        // Undo would close the armed Bridge (MU-MID-02).
        assert!(item(&menu, &ctx, app_menu::line_action("app.undo")).enabled);
        menu.execute("bridge segments 3");
        typed.execute("bridge segments 3");
        menu.execute("tool.apply");
        typed.execute("tool.apply");
        let output = menu.editor.scene.clone();
        assert_eq!(output, typed.editor.scene);
        let undo = item(&menu, &ctx, app_menu::line_action("app.undo"));
        assert!(undo.enabled);
        assert!(undo.title.contains("Bridge Apply"));
        menu.fields.insert("bri_twist".into(), "bad".into());
        typed.fields.insert("bri_twist".into(), "bad".into());
        menu.menu_action(&ctx, app_menu::line_action("app.undo"));
        typed.execute("undo");
        assert_eq!(menu.editor.scene, output);
        assert_eq!(menu.editor.scene, typed.editor.scene);
        menu.fields.insert("bri_twist".into(), "0".into());
        typed.fields.insert("bri_twist".into(), "0".into());
        menu.menu_action(&ctx, app_menu::line_action("app.undo"));
        typed.execute("undo");
        assert_eq!(menu.editor.scene, source);
        assert_eq!(menu.editor.scene, typed.editor.scene);
        let redo = item(&menu, &ctx, app_menu::line_action("app.redo"));
        assert!(redo.enabled);
        assert!(redo.title.contains("Bridge Apply"));
        menu.menu_action(&ctx, app_menu::line_action("app.redo"));
        typed.execute("redo");
        assert_eq!(menu.editor.scene, output);
        assert_eq!(menu.editor.scene, typed.editor.scene);
        menu.execute("undo");
        assert!(menu.bridge_panel_eligibility().is_ok());
        menu.fields.insert("bri_segments".into(), "invalid".into());
        assert!(!menu.confirm_fields());
        let prior = menu.editor.clone();
        let fields = menu.fields.clone();
        let confirmed = menu.confirmed_fields.clone();
        let errors = menu.field_errors.clone();
        let cache = menu.bridge_ui.eligibility.clone();
        menu.execute("tool.set prim.cube on");
        assert!(menu.editor == prior);
        assert_eq!(menu.fields, fields);
        assert_eq!(menu.confirmed_fields, confirmed);
        assert_eq!(menu.field_errors, errors);
        assert_eq!(menu.bridge_ui.eligibility, cache);

        let mut other = app();
        other.execute("tool.drop");
        other.execute("spike");
        assert!(other.editor.session.spike.is_some());
        other.fields.insert("spi_strength".into(), "invalid".into());
        assert!(!other.confirm_fields());
        let prior = other.editor.clone();
        let fields = other.fields.clone();
        let confirmed = other.confirmed_fields.clone();
        let errors = other.field_errors.clone();
        let cache = other.bridge_ui.eligibility.clone();
        other.execute("tool.set edge.bridge on");
        assert!(other.editor == prior);
        assert_eq!(other.fields, fields);
        assert_eq!(other.confirmed_fields, confirmed);
        assert_eq!(other.field_errors, errors);
        assert_eq!(other.bridge_ui.eligibility, cache);
    }

    /// Jason's case (2026-11-15): two open border edges in Edge mode with
    /// the Bottom view's work plane, a Selection action center and a
    /// falloff. Apply is live, Remove/Flip/Reverse grey out with reasons
    /// (UIC-92), and Apply joins the edges with one quad facing up.
    #[test]
    fn edge_bridge_applies_under_any_work_plane_and_greys_polygon_options() {
        let mut a = Operandi::empty();
        a.editor.dispatch(Command::NewMesh).unwrap();
        let m = &mut a.editor.scene.active_mut().unwrap().mesh;
        m.vertices = [0., 1., 2., 3.]
            .into_iter()
            .flat_map(|x| [DVec3::new(x, 0., 0.), DVec3::new(x, 1., 0.)])
            .collect();
        for vertices in [vec![0, 2, 3, 1], vec![4, 6, 7, 5]] {
            m.faces.push(operandi_core::Face {
                vertices,
                uv: None,
                material: "Default".into(),
                material_label: None,
                part: None,
                smooth: false,
            });
        }
        a.editor.synchronize_identities().unwrap();
        a.editor.scene.selection.mode = Mode::Edge;
        a.editor.scene.selection.edges = vec![(2, 3), (4, 5)];
        a.editor.scene.workplane = operandi_core::scene::Workplane {
            fixed: true,
            plane: 1,
            center: DVec3::ZERO,
        };
        a.editor.action_context.center = operandi_core::CenterMode::Selection;
        a.editor.history.undo.clear();
        a.execute("Bridge");
        assert!(a.bridge_panel_eligibility().is_ok());
        let p = a.editor.session.bridge.as_ref().unwrap().requested;
        let form = BridgeFields::form(&a, &p);
        let greyed: Vec<&str> = form
            .fields()
            .iter()
            .filter(|f| f.disabled_tip.is_some())
            .map(|f| f.row.label.as_str())
            .collect();
        for label in ["Remove Polygons", "Flip Polygons", "Reverse Direction"] {
            assert!(greyed.contains(&label), "{label} greyed in Edge mode");
        }
        for label in ["Segments", "Twist", "Mode", "Tension"] {
            assert!(!greyed.contains(&label), "{label} live in Edge mode");
        }
        a.execute("bridge apply");
        let mesh = &a.editor.scene.active().unwrap().mesh;
        assert_eq!(mesh.faces.len(), 3);
        assert!(mesh.normal(2).z > 0.9);
        assert_eq!(a.editor.history.undo.len(), 1);
        // Polygon mode: the options are live again.
        a.editor.scene.selection.mode = Mode::Polygon;
        let form = BridgeFields::form(&a, &p);
        assert!(form.fields().iter().all(|f| {
            f.disabled_tip.is_none()
                || !["Remove Polygons", "Flip Polygons", "Reverse Direction"]
                    .contains(&f.row.label.as_str())
        }));
    }
}
