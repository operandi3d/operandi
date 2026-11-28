//! The Replicator tab in Properties (G8 RP-2): Point Source, Prototype and
//! the copies' random variation. Each change is one undo step
//! (`Command::SetReplicator`); a field drag ends as one step too. Rows use
//! the option-row layout of the other item tabs (UIC-87, UIC-100).
use super::*;
use crate::unavailable::Gate;

pub(super) const REPLICATOR_TAB: &str = "Instancer";

impl Operandi {
    /// G8 RP-3: `scene` (the shown scene) with Replicators' copies as wire
    /// boxes unless this view's Replicators option shades them; `None`
    /// when nothing changes.
    pub(super) fn boxed_replicas(
        &self,
        scene: &operandi_core::Scene,
    ) -> Option<operandi_core::Scene> {
        use crate::view_display::ReplicatorDisplay as R;
        let mode = self.display.settings.drawing.replicators;
        let selection = &self.editor.scene.selection;
        let document = &self.editor.scene;
        operandi_core::replicator::boxed(scene, |rep| match mode {
            R::All => true,
            R::None => false,
            R::Selected => {
                selection.has_item(rep)
                    || document
                        .item(rep)
                        .and_then(|i| i.parent)
                        .is_some_and(|g| selection.has_item(g))
            }
        })
    }

    pub(super) fn replicator_properties(&mut self, ui: &mut Ui, item: &operandi_core::Item) {
        let Some(current) = item.replicator.as_deref().cloned() else {
            return;
        };
        let mut next = current.clone();
        let scene = &self.editor.scene;
        let left_out = operandi_core::replicator::copies_left_out(scene, item.id);
        // Point Source: any mesh item. Prototype: a mesh item or a group.
        let meshes: Vec<(u64, String)> = scene
            .items
            .iter()
            .filter(|i| i.is_mesh())
            .map(|i| (i.id, i.name.clone()))
            .collect();
        let prototypes: Vec<(u64, String)> = scene
            .items
            .iter()
            .filter(|i| (i.is_mesh() || i.is_group_locator()) && i.id != item.id)
            .map(|i| (i.id, i.name.clone()))
            .collect();
        crate::form::rows::section(ui, REPLICATOR_TAB, true, |ui| {
            ui.gated_ui(None, |ui| {
                let source: Vec<(u64, &str)> =
                    meshes.iter().map(|(id, n)| (*id, n.as_str())).collect();
                crate::option_row::choice(
                    ui,
                    "replicator-point-source",
                    "Point Source",
                    &mut next.point_source,
                    &source,
                )
                .response
                .on_hover_text("The mesh whose points place the copies");
                if left_out > 0 {
                    // Never a silent cut (hidden-limits A4).
                    crate::form::rows::note(
                        ui,
                        &format!(
                            "Shows the first {} copies; {} points of the Point Source get none.",
                            operandi_core::polygon_subdivide::group_thousands(
                                operandi_core::replicator::MAX_COPIES as u64
                            ),
                            operandi_core::polygon_subdivide::group_thousands(left_out as u64)
                        ),
                    );
                }
                let protos: Vec<(u64, &str)> =
                    prototypes.iter().map(|(id, n)| (*id, n.as_str())).collect();
                crate::option_row::choice(
                    ui,
                    "replicator-prototype",
                    "Prototype",
                    &mut next.prototype,
                    &protos,
                )
                .response
                .on_hover_text(
                    "The item copied; with a group, each point picks one of its meshes at random",
                );
                crate::option_row::check(
                    ui,
                    "Use Source Transforms",
                    &mut next.use_source_transforms,
                )
                .on_hover_text("Each copy also takes the prototype's own rotation and scale");
                crate::option_row::check(ui, "Hierarchy", &mut next.hierarchy)
                    .on_hover_text("Copies include the prototype's children");
                let vector = |ui: &mut Ui, label: &str, v: &mut DVec3, suffix: &str, hint: &str| {
                    for (axis, value) in [("X", &mut v.x), ("Y", &mut v.y), ("Z", &mut v.z)] {
                        ui.horizontal(|ui| {
                            crate::option_row::label(ui, &format!("{label} {axis}"));
                            crate::option_row::fill(ui, |ui| {
                                ui.add(
                                    crate::numeric_tab::DragValue::new(value)
                                        .speed(0.1)
                                        .suffix(suffix),
                                )
                                .on_hover_text(hint);
                            });
                        });
                    }
                };
                vector(
                    ui,
                    "Random Position",
                    &mut next.random_position,
                    " m",
                    "Each copy moves by up to this much either way",
                );
                vector(
                    ui,
                    "Random Rotation",
                    &mut next.random_rotation,
                    "°",
                    "Each copy turns by up to this much either way",
                );
                vector(
                    ui,
                    "Random Scale",
                    &mut next.random_scale,
                    " %",
                    "Each copy's size varies by up to this much either way",
                );
                crate::option_row::check(ui, "Uniform Scale", &mut next.uniform_scale)
                    .on_hover_text("One random scale for all three axes");
                let number = |ui: &mut Ui,
                              label: &str,
                              v: &mut f64,
                              range: std::ops::RangeInclusive<f64>,
                              hint: &str| {
                    ui.horizontal(|ui| {
                        crate::option_row::label(ui, label);
                        crate::option_row::fill(ui, |ui| {
                            ui.add(
                                crate::numeric_tab::DragValue::new(v)
                                    .range(range)
                                    .speed(0.5)
                                    .suffix(" %"),
                            )
                            .on_hover_text(hint);
                        });
                    });
                };
                number(
                    ui,
                    "Base Scale",
                    &mut next.base_scale,
                    0.0..=10_000.,
                    "Every copy's size",
                );
                crate::option_row::check(ui, "Snap Rotation", &mut next.snap_rotation)
                    .on_hover_text("Random rotation snaps to whole 90° steps (provisional, RP-Q1)");
                number(
                    ui,
                    "Density",
                    &mut next.density,
                    0.0..=100.,
                    "Share of points that get a copy in renders and exports",
                );
                number(
                    ui,
                    "Display Density",
                    &mut next.display_density,
                    0.0..=100.,
                    "Share of the copies drawn in the viewport",
                );
                ui.horizontal(|ui| {
                    crate::option_row::label(ui, "Seed");
                    crate::option_row::fill(ui, |ui| {
                        ui.add(crate::numeric_tab::DragValue::new(&mut next.seed).speed(1.))
                            .on_hover_text("Drives every random choice, so the copies repeat");
                    });
                });
                if ui
                    .button("Freeze Instancer")
                    .on_hover_text("Turn the copies into instance items in the group")
                    .clicked()
                {
                    self.execute("Freeze Instancer");
                }
            });
        });
        if next != current && next.validate().is_ok() {
            self.run(Command::SetReplicator {
                item: item.id,
                settings: Box::new(next),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_display::ReplicatorDisplay;
    use operandi_core::evaluate::Purpose;

    /// Spec test 6: Replicators None draws boxes; All draws shaded copies.
    #[test]
    fn replicators_draw_as_boxes_until_the_view_shades_them() {
        let mut a = crate::tests::box_app();
        a.run(Command::Mode(operandi_core::Mode::Item));
        a.activate(operandi_core::Tool::Array);
        let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
        p.duplicate.copier = Some(operandi_core::duplicate_copier::Copier::Replica);
        p.duplicate.array.count = [3, 1, 1];
        p.duplicate.placed = true;
        a.run(Command::Preview(p));
        a.run(Command::Drop);
        let shown = a
            .editor
            .evaluated(Purpose::Display { base_edit: None })
            .into_owned();
        let copies = |s: &operandi_core::Scene| -> Vec<usize> {
            s.items
                .iter()
                .filter(|i| operandi_core::replicator::copy_replicator(i.id).is_some())
                .map(|i| i.mesh.faces.len())
                .collect()
        };
        let boxed = a.boxed_replicas(&shown).expect("None boxes the copies");
        assert_eq!(
            copies(&boxed),
            vec![12, 12],
            "two copies, twelve lines each"
        );
        assert!(
            boxed
                .items
                .iter()
                .filter(|i| copies(&boxed).len() == 2 && i.mesh.faces.len() == 12)
                .all(|i| (0..12).all(|f| i.mesh.is_polyline(f)))
        );
        a.display.settings.drawing.replicators = ReplicatorDisplay::All;
        assert!(a.boxed_replicas(&shown).is_none(), "All shades every copy");
        let shaded = copies(&shown);
        assert_eq!(shaded, vec![6, 6], "the box prototype's six polygons");
        // Selected: shaded only while the Replicator (or its group) is.
        a.display.settings.drawing.replicators = ReplicatorDisplay::Selected;
        assert!(a.boxed_replicas(&shown).is_some());
        let rep = a
            .editor
            .scene
            .items
            .iter()
            .find(|i| i.replicator.is_some())
            .unwrap()
            .id;
        a.editor.scene.select_item(Some(rep));
        assert!(a.boxed_replicas(&shown).is_none());
    }

    /// RP-6: every Replica button starts its placement tool with the
    /// Replica copier, sits in the Duplicate tab and shows the Replica
    /// section.
    #[test]
    fn replica_buttons_start_their_tools_with_the_replica_copier() {
        use operandi_core::duplicate_copier::Copier;
        for (button, tool) in [
            ("Instancer Clone", operandi_core::Tool::Clone),
            ("Instancer Array", operandi_core::Tool::Array),
            ("Instancer Radial Array", operandi_core::Tool::RadialArray),
            ("Instancer Mirror", operandi_core::Tool::Mirror),
            ("Instancer Scatter", operandi_core::Tool::ScatterClone),
            ("Curve Instancer", operandi_core::Tool::CurveClone),
            ("Bezier Instancer", operandi_core::Tool::CurveClone),
            ("B-Spline Instancer", operandi_core::Tool::CurveClone),
            ("Pen Instancer", operandi_core::Tool::CurveClone),
        ] {
            let mut a = crate::tests::box_app();
            a.run(Command::Mode(operandi_core::Mode::Item));
            a.execute(button);
            let live = a
                .editor
                .session
                .live
                .as_ref()
                .unwrap_or_else(|| panic!("{button}: {}", a.status));
            assert_eq!(live.tool, tool, "{button}");
            assert_eq!(
                live.params.duplicate.copier,
                Some(Copier::Replica),
                "{button}"
            );
            let form = a.forms.get("toolbox.duplicate").unwrap();
            let text = serde_json::to_string(&form.controls).unwrap();
            assert!(
                text.contains(&format!("\"command\":\"{button}\"")),
                "{button} in the tab"
            );
        }
    }

    /// The Instancer names replaced Replica / Replicator on 2026-11-23; the
    /// old phrases still start the same tools and commands.
    #[test]
    fn old_replica_phrases_still_work() {
        use operandi_core::duplicate_copier::Copier;
        for (old, tool) in [
            ("Replica Clone", operandi_core::Tool::Clone),
            ("Replica Array", operandi_core::Tool::Array),
            ("Replica Radial Array", operandi_core::Tool::RadialArray),
            ("Replica Mirror", operandi_core::Tool::Mirror),
            ("Replica Scatter", operandi_core::Tool::ScatterClone),
            ("Curve Replica", operandi_core::Tool::CurveClone),
            ("Bezier Replica", operandi_core::Tool::CurveClone),
            ("B-Spline Replica", operandi_core::Tool::CurveClone),
            ("Pen Replicate", operandi_core::Tool::CurveClone),
        ] {
            let mut a = crate::tests::box_app();
            a.run(Command::Mode(operandi_core::Mode::Item));
            a.execute(old);
            let live = a
                .editor
                .session
                .live
                .as_ref()
                .unwrap_or_else(|| panic!("{old}: {}", a.status));
            assert_eq!(live.tool, tool, "{old}");
            assert_eq!(live.params.duplicate.copier, Some(Copier::Replica), "{old}");
        }
        // Freeze Replicator with nothing to freeze says so, as Freeze
        // Instancer does; Replicators steps the view option.
        let mut a = crate::tests::box_app();
        a.execute("Freeze Replicator");
        assert!(a.status.contains("Instancer"), "{}", a.status);
        a.execute("Replicators");
        assert_eq!(
            a.display.settings.drawing.replicators,
            crate::view_display::ReplicatorDisplay::Selected
        );
        assert!(a.status.starts_with("Instancers"), "{}", a.status);
        // Mesh Paint Replica turns on Mesh Paint making Instancers.
        for phrase in ["Mesh Paint Replica", "Mesh Paint Instancer"] {
            let mut a = crate::tests::box_app();
            a.execute(phrase);
            assert_eq!(
                a.utility.map(|u| u.label()),
                Some("Mesh Paint Instancer"),
                "{phrase}: {}",
                a.status
            );
        }
    }
}
