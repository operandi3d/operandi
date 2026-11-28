//! Mesh Cleanup options dialog and `mesh.cleanup` (mesh-cleanup-2026-11-05):
//! all twelve options run through one core command.
use super::*;

const LABELS: [&str; 12] = [
    "Remove Floating Vertices",
    "Remove One Point Polygons",
    "Remove Two Points Polygons",
    "Fix Duplicate Points in Polygon",
    "Remove Colinear Vertices",
    "Fix Face Normal Vectors",
    "Merge Vertices",
    "Merge Disco Values",
    "Unify Polygons",
    "Force Unify",
    "Remove Disco Weight Values",
    "Fix Gaps",
];
const KEYS: [&str; 12] = [
    "floatingvertex",
    "onepointpolygon",
    "twopointpolygon",
    "duppointpolygon",
    "colinear",
    "facenormal",
    "mergevertex",
    "mergedisco",
    "unifypolygon",
    "forceunify",
    "removediscoweight",
    "fixgaps",
];

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CleanupOptions {
    version: u8,
    flags: [bool; 12],
}
impl Default for CleanupOptions {
    /// Every option on (the spec's defaults).
    fn default() -> Self {
        Self {
            version: 1,
            flags: [true; 12],
        }
    }
}
impl CleanupOptions {
    pub(super) fn load(path: &std::path::Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Self>(&b).ok())
            .filter(|o| o.version == 1)
            .unwrap_or_default()
    }
    fn save(&self, path: &std::path::Path) -> std::result::Result<(), String> {
        let data = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        let temp = path.with_extension(format!("tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &data).map_err(|e| e.to_string())
    }
    fn explicit(args: &[String]) -> std::result::Result<Self, String> {
        if args.len() != 12 {
            return Err("mesh.cleanup requires all twelve explicit boolean options".into());
        }
        let mut flags = [false; 12];
        let mut seen = [false; 12];
        for arg in args {
            let (key, value) = arg
                .split_once(':')
                .ok_or("Each mesh.cleanup option requires name:true or name:false")?;
            let i = KEYS
                .iter()
                .position(|k| k.eq_ignore_ascii_case(key))
                .ok_or_else(|| format!("Unknown Mesh Cleanup option: {key}"))?;
            if seen[i] {
                return Err(format!("Duplicate Mesh Cleanup option: {key}"));
            }
            seen[i] = true;
            flags[i] = if value.eq_ignore_ascii_case("true") {
                true
            } else if value.eq_ignore_ascii_case("false") {
                false
            } else {
                return Err(format!("Invalid boolean for Mesh Cleanup option: {key}"));
            };
        }
        if seen.iter().any(|v| !v) {
            return Err("mesh.cleanup requires every option exactly once".into());
        }
        Ok(Self { version: 1, flags })
    }
}
pub(super) struct CleanupDialog {
    target: CleanupTarget,
    form: form::Dialog,
}
/// One checkbox row per option (UIC-87). Force Unify needs Unify
/// Polygons, so it greys out without it (UIC-92/93). The options are
/// remembered in mesh-cleanup.json (they already were before UIC-95), so
/// the shared dialog memory doesn't keep a second copy.
fn rows(flags: &[bool; 12]) -> Vec<form::Row> {
    LABELS
        .iter()
        .zip(KEYS)
        .zip(flags)
        .map(|((label, key), on)| {
            let row = form::Row::check(key, label, *on).fresh();
            if key == "forceunify" {
                row.when(form::When::Checked("unifypolygon"))
                    .tip("Needs Unify Polygons")
            } else {
                row
            }
        })
        .collect()
}
impl Operandi {
    pub(super) fn open_mesh_cleanup(&mut self) {
        if self.has_modal() {
            self.status = "Close the open dialog before Mesh Cleanup".into();
            return;
        }
        match self.editor.cleanup_target() {
            Ok(target) => {
                let rows = rows(&self.cleanup_options.flags);
                self.mesh_cleanup = Some(CleanupDialog {
                    target,
                    form: self.form_dialog("Mesh Cleanup", rows),
                })
            }
            Err(error) => self.status = error,
        }
    }
    fn execute_mesh_cleanup(
        &mut self,
        options: &CleanupOptions,
        target: Option<CleanupTarget>,
        _explicit: bool,
    ) {
        if options.flags.iter().all(|f| !f) {
            self.status = "No Mesh Cleanup operations selected".into();
        } else {
            self.run(Command::MeshCleanup {
                flags: options.flags,
                target,
            });
        }
    }
    pub(super) fn mesh_cleanup_command(&mut self, args: &[String]) {
        if self.has_modal() {
            self.status = "Close the open dialog before Mesh Cleanup".into();
            return;
        }
        // CMD-12: the bare line opens the dialog (Geometry > Mesh
        // Cleanup..., menus as forms); some options but not all is an error.
        if args.is_empty() {
            self.open_mesh_cleanup();
            return;
        }
        match CleanupOptions::explicit(args) {
            Ok(options) => self.execute_mesh_cleanup(&options, None, true),
            Err(error) => self.status = error,
        }
    }
    pub(super) fn mesh_cleanup_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.mesh_cleanup.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "Mesh Cleanup canceled".into();
            return;
        }
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => self.status = "Mesh Cleanup canceled".into(),
            form::Outcome::Ok => {
                // UIC-95/96: OK keeps the choices for next time and runs once.
                let flags = KEYS.map(|k| dialog.form.values.bool(k));
                if flags != self.cleanup_options.flags {
                    self.cleanup_options.flags = flags;
                    if let Err(error) = self.cleanup_options.save(
                        &self
                            .environment
                            .paths()
                            .data_root()
                            .join("mesh-cleanup.json"),
                    ) {
                        self.status = format!("Mesh Cleanup options could not be saved: {error}");
                    }
                }
                let options = self.cleanup_options.clone();
                self.execute_mesh_cleanup(&options, Some(dialog.target), false);
            }
            _ => self.mesh_cleanup = Some(dialog),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_options_require_every_unique_boolean_key() {
        let mut args: Vec<String> = KEYS
            .iter()
            .enumerate()
            .map(|(i, key)| format!("{key}:{}", i == 0))
            .collect();
        let accepted = CleanupOptions::explicit(&args).unwrap();
        assert!(accepted.flags[0] && accepted.flags[1..].iter().all(|f| !f));
        args[11] = args[0].clone();
        assert!(CleanupOptions::explicit(&args).is_err());
        args[11] = "fixgaps:maybe".into();
        assert!(CleanupOptions::explicit(&args).is_err());
        assert!(CleanupOptions::explicit(&args[..11]).is_err());
    }

    #[test]
    fn every_option_defaults_on() {
        assert!(CleanupOptions::default().flags.iter().all(|f| *f));
    }
}

#[cfg(test)]
mod callback_tests {
    use super::*;
    use crate::tests::{draw, key, pointer};
    use glam::{DVec2, DVec3};
    use operandi_core::{Face, Mesh, SmoothShiftParameters};

    const SIZE: Vec2 = vec2(1440., 900.);
    const CLEAN_COMMAND: &str = "mesh.cleanup floatingVertex:true onePointPolygon:false twoPointPolygon:false dupPointPolygon:false colinear:false faceNormal:false mergeVertex:false mergeDisco:false unifyPolygon:false forceUnify:false removeDiscoWeight:false fixGaps:false";

    // Unlike Operandi::empty(), this constructor neither creates a test root
    // nor loads/saves profile state. draw() invokes compose, not App::update.
    fn app() -> Operandi {
        Operandi::with_environment(crate::app_environment::AppEnvironment::from_inputs(
            None,
            None,
            Vec::new(),
            env!("CARGO_MANIFEST_DIR"),
        ))
    }
    fn context() -> Context {
        let ctx = Context::default();
        theme::install(&ctx);
        ctx
    }
    fn click(a: &mut Operandi, ctx: &Context, pos: Pos2) {
        draw(
            a,
            ctx,
            SIZE,
            vec![
                Event::PointerMoved(pos),
                pointer(pos, true, Modifiers::NONE),
                pointer(pos, false, Modifiers::NONE),
            ],
        );
    }
    fn invoke_new_mesh(a: &mut Operandi, route: &str) {
        let ctx = context();
        match route {
            "callable" => a.execute("new mesh"),
            "row" => {
                draw(a, &ctx, SIZE, vec![]);
                let pos = a.toolbox_rects["New Mesh"].center();
                click(a, &ctx, pos);
            }
            "menu" => {
                draw(a, &ctx, SIZE, vec![]);
                let pos = a.toolbox_rects["Item list Add Item arrow"].center();
                click(a, &ctx, pos);
                draw(a, &ctx, SIZE, vec![]);
                let pos = a.toolbox_rects["Scenes menu New Mesh"].center();
                click(a, &ctx, pos);
            }
            "n" => {
                draw(a, &ctx, SIZE, vec![]);
                let pos = a.scene_rect.center();
                draw(a, &ctx, SIZE, vec![Event::PointerMoved(pos), key(Key::N)]);
            }
            _ => unreachable!(),
        }
    }
    fn cleanup_fixture() -> Operandi {
        let mut a = app();
        a.editor.scene.new_mesh().unwrap();
        let mesh = &mut a.editor.scene.active_mut().unwrap().mesh;
        *mesh = Mesh {
            vertices: vec![
                DVec3::ZERO,
                DVec3::X,
                DVec3::new(1., 1., 0.),
                DVec3::Y,
                DVec3::new(9., 8., 7.),
            ],
            faces: vec![Face {
                vertices: vec![0, 1, 2, 3],
                uv: Some(vec![DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y]),
                material: "original".into(),
                smooth: false,
                part: Some("kept".into()),
                material_label: None,
            }]
            .into(),
            ..Default::default()
        }
        .into();
        a.editor.scene.selection.mode = Mode::Item;
        a.editor.synchronize_identities().unwrap();
        a
    }
    // CLEAN-F01/F02: original two-item data, mapped into public Scene types.
    // Expected scenes are built by the explicit fixture differences, never by Cleanup.
    fn admitted_cleanup_fixture(interleaved: bool) -> (Operandi, Scene) {
        use glam::DVec4;
        use std::collections::{BTreeMap, BTreeSet};
        let mut a = app();
        a.editor.scene.new_mesh().unwrap();
        let item = a.editor.scene.active_mut().unwrap();
        item.transform.translation = DVec3::new(1., 2., 3.);
        let mesh = &mut item.mesh;
        mesh.vertices = if interleaved {
            vec![
                DVec3::ZERO,
                DVec3::new(4., 3., 2.),
                DVec3::new(2., 0., 0.),
                DVec3::new(2., 1., 0.),
                DVec3::new(0., 1., 0.),
            ]
        } else {
            vec![
                DVec3::ZERO,
                DVec3::new(2., 0., 0.),
                DVec3::new(2., 1., 0.),
                DVec3::new(0., 1., 0.),
                DVec3::new(4., 3., 2.),
            ]
        };
        mesh.faces = vec![Face {
            vertices: if interleaved {
                vec![0, 2, 3, 4]
            } else {
                vec![0, 1, 2, 3]
            },
            uv: Some(vec![DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y]),
            material: "CleanupOriginalA".into(),
            smooth: false,
            part: Some("Kept".into()),
            material_label: None,
        }]
        .into();
        mesh.attributes.point_weights.insert(
            "weight".into(),
            if interleaved {
                BTreeMap::from([(0, 0.25), (1, 0.5), (3, 0.75)])
            } else {
                BTreeMap::from([(0, 0.25), (2, 0.75), (4, 0.5)])
            },
        );
        // Continuous point UVs require the named corner map, with absent
        // slots separate from the face's independently authored UVs.
        mesh.uv_maps.insert("pointUV".into(), vec![vec![None; 4]]);
        mesh.attributes.point_uvs.insert(
            "pointUV".into(),
            if interleaved {
                BTreeMap::from([(1, DVec2::new(7., 9.)), (4, DVec2::new(0.25, 0.875))])
            } else {
                BTreeMap::from([(4, DVec2::new(7., 9.))])
            },
        );
        if interleaved {
            mesh.attributes.point_colors.insert(
                "color".into(),
                BTreeMap::from([(4, DVec4::new(1., 0.5, 0., 1.))]),
            );
            mesh.attributes.relative_morphs.insert(
                "morph".into(),
                BTreeMap::from([(2, DVec3::new(0.125, 0., 0.))]),
            );
            mesh.attributes
                .vertex_sets
                .insert("Keep".into(), BTreeSet::from([0, 4]));
            mesh.attributes
                .vertex_sets
                .insert("Mixed".into(), BTreeSet::from([1, 3]));
        }
        a.editor.scene.new_mesh().unwrap();
        let other = a.editor.scene.active_mut().unwrap();
        other.transform.translation = DVec3::new(-2., 1., 4.);
        other.mesh.vertices = vec![
            DVec3::new(10., 0., 0.),
            DVec3::new(12., 0., 0.),
            DVec3::new(12., 1., 0.),
            DVec3::new(10., 1., 0.),
            DVec3::new(14., 3., 2.),
        ];
        other.mesh.faces = vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: Some(vec![DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y]),
            material: "CleanupOriginalB".into(),
            smooth: false,
            part: Some("Unrelated".into()),
            material_label: None,
        }]
        .into();
        other.mesh.attributes.point_weights.insert(
            "weight".into(),
            BTreeMap::from([(0, 0.25), (2, 0.75), (4, 0.5)]),
        );
        other
            .mesh
            .uv_maps
            .insert("pointUV".into(), vec![vec![None; 4]]);
        other
            .mesh
            .attributes
            .point_uvs
            .insert("pointUV".into(), BTreeMap::from([(4, DVec2::new(7., 9.))]));
        a.editor.scene.select_item(Some(1));
        a.editor.scene.selection.mode = Mode::Item;
        a.editor.scene.selection.vertices = if interleaved {
            vec![4, 1, 0]
        } else {
            vec![0, 4]
        };
        a.editor.scene.selection.edges = if interleaved {
            vec![(0, 2), (3, 4)]
        } else {
            vec![(0, 1), (2, 3)]
        };
        a.editor.scene.selection.faces = vec![0];
        a.editor.scene.validate().unwrap();
        a.editor.synchronize_identities().unwrap();
        let mut expected = a.editor.scene.clone();
        let mesh = &mut expected.active_mut().unwrap().mesh;
        mesh.vertices = vec![
            DVec3::ZERO,
            DVec3::new(2., 0., 0.),
            DVec3::new(2., 1., 0.),
            DVec3::new(0., 1., 0.),
        ];
        mesh.faces[0].vertices = vec![0, 1, 2, 3];
        mesh.attributes
            .point_weights
            .insert("weight".into(), BTreeMap::from([(0, 0.25), (2, 0.75)]));
        mesh.attributes.point_uvs.insert(
            "pointUV".into(),
            if interleaved {
                BTreeMap::from([(3, DVec2::new(0.25, 0.875))])
            } else {
                BTreeMap::new()
            },
        );
        if interleaved {
            mesh.attributes.point_colors.insert(
                "color".into(),
                BTreeMap::from([(3, DVec4::new(1., 0.5, 0., 1.))]),
            );
            mesh.attributes.relative_morphs.insert(
                "morph".into(),
                BTreeMap::from([(1, DVec3::new(0.125, 0., 0.))]),
            );
            mesh.attributes
                .vertex_sets
                .insert("Keep".into(), BTreeSet::from([0, 3]));
            mesh.attributes
                .vertex_sets
                .insert("Mixed".into(), BTreeSet::from([2]));
            expected.selection.vertices = vec![3, 0];
            expected.selection.edges = vec![(0, 1), (2, 3)];
        } else {
            expected.selection.vertices = vec![0];
        }
        (a, expected)
    }

    #[test]
    fn cleanup_fixtures_protected_gesture_refuses_both_routes_with_parked_redo() {
        for interleaved in [false, true] {
            for route in ["menu", "callable"] {
                let (mut a, _) = admitted_cleanup_fixture(interleaved);
                a.run(Command::NewMesh);
                a.run(Command::Undo);
                assert!(!a.editor.history.redo.is_empty());
                a.editor.selection_start = Some(Box::new(a.editor.scene.clone()));
                let before = a.editor.clone();
                let next = a.editor.identities.next_id().unwrap();
                if route == "menu" {
                    let ctx = context();
                    a.menu_action(&ctx, crate::app_menu::line_action("mesh.cleanup"));
                    assert!(a.mesh_cleanup.is_none());
                } else {
                    a.execute(CLEAN_COMMAND);
                }
                assert!(
                    a.editor == before,
                    "F02={interleaved} route={route}: {}",
                    a.status
                );
                assert_eq!(a.editor.identities.next_id().unwrap(), next);
            }
        }
    }

    #[test]
    fn cleanup_menu_dialog_return_and_explicit_callback_match_literal_scene() {
        for route in ["menu", "callable"] {
            let mut a = cleanup_fixture();
            let source = a.editor.scene.clone();
            let mut expected = source.clone();
            expected.active_mut().unwrap().mesh.vertices =
                vec![DVec3::ZERO, DVec3::X, DVec3::new(1., 1., 0.), DVec3::Y];
            let before_ids = a.editor.identities.current.clone();
            if route == "menu" {
                let ctx = context();
                a.menu_action(&ctx, crate::app_menu::line_action("mesh.cleanup"));
                assert!(a.mesh_cleanup.is_some());
                assert_eq!(a.editor.scene, source);
                draw(&mut a, &ctx, SIZE, vec![]);
                draw(&mut a, &ctx, SIZE, vec![key(Key::Enter)]);
                assert!(a.mesh_cleanup.is_none());
            } else {
                a.execute(CLEAN_COMMAND);
            }
            assert_eq!(a.editor.scene, expected, "{route}: {}", a.status);
            assert_eq!(a.editor.history.undo.len(), 1);
            assert_eq!(a.editor.history.undo[0].label, "Mesh Cleanup");
            let after_ids = a.editor.identities.current.clone();
            a.run(Command::Undo);
            assert_eq!(a.editor.scene, source);
            assert_eq!(a.editor.identities.current, before_ids);
            a.run(Command::Redo);
            assert_eq!(a.editor.scene, expected);
            assert_eq!(a.editor.identities.current, after_ids);
        }
    }

    #[test]
    fn cleanup_numeric_owners_refuse_both_routes_with_fixture_and_parked_redo() {
        for owner in ["cone", "bridge", "reduction"] {
            for route in ["menu", "callable"] {
                let (mut a, _) = admitted_cleanup_fixture(true);
                a.run(Command::NewMesh);
                a.run(Command::Undo);
                match owner {
                    "cone" => a.editor.dispatch(Command::ArmCone).unwrap(),
                    "bridge" => a.editor.dispatch(Command::ArmBridge).unwrap(),
                    "reduction" => a.editor.dispatch(Command::ArmReduction).unwrap(),
                    _ => unreachable!(),
                }
                assert!(
                    !a.editor.history.redo.is_empty(),
                    "{owner} setup must retain redo"
                );
                let before = a.editor.clone();
                let next = a.editor.identities.next_id().unwrap();
                a.status = "untouched sentinel".into();
                if route == "menu" {
                    let ctx = context();
                    a.menu_action(&ctx, crate::app_menu::line_action("mesh.cleanup"));
                    assert!(a.mesh_cleanup.is_none());
                } else {
                    // UIC-307: the command line runs it; the owner settles and
                    // arms again.
                    a.execute(CLEAN_COMMAND);
                    assert!(a.editor.session.has_active_task(), "{owner}: {}", a.status);
                    continue;
                }
                assert!(a.editor == before, "{owner} {route}: {}", a.status);
                let reason = match owner {
                    "cone" => "active tool",
                    "bridge" => "Bridge",
                    "reduction" => "Reduction",
                    _ => unreachable!(),
                };
                assert!(a.status.contains(reason), "{owner} {route}: {}", a.status);
                assert_eq!(a.editor.identities.next_id().unwrap(), next);
            }
        }
    }

    #[test]
    fn cleanup_pen_owner_refuses_with_parked_redo_without_dropping_draft() {
        let mut a = app();
        a.editor.scene.new_mesh().unwrap();
        a.editor.synchronize_identities().unwrap();
        a.run(Command::NewMesh);
        a.run(Command::Undo);
        a.editor.dispatch(Command::PenStart).unwrap();
        assert!(a.editor.session.pen_draft.is_some());
        assert!(!a.editor.history.redo.is_empty());
        let before = a.editor.clone();
        let next = a.editor.identities.next_id().unwrap();
        a.status = "untouched sentinel".into();
        a.execute(CLEAN_COMMAND);
        assert!(a.status.contains("Pen"), "{}", a.status);
        assert!(a.editor == before);
        assert_eq!(a.editor.identities.next_id().unwrap(), next);
    }

    #[test]
    fn cleanup_malformed_explicit_commands_preserve_full_editor_and_parked_redo() {
        let (mut a, _) = admitted_cleanup_fixture(false);
        a.run(Command::NewMesh);
        a.run(Command::Undo);
        assert!(!a.editor.history.redo.is_empty());
        let before = a.editor.clone();
        let next = a.editor.identities.next_id().unwrap();
        let cases = [
            ("mesh.cleanup floatingVertex:true".to_owned(), "twelve"),
            (
                CLEAN_COMMAND.replace("fixGaps:false", "fixGaps:maybe"),
                "Invalid boolean",
            ),
            (
                CLEAN_COMMAND.replace("fixGaps:false", "unknown:false"),
                "Unknown Mesh Cleanup option",
            ),
            (
                CLEAN_COMMAND.replace("fixGaps:false", "floatingVertex:false"),
                "Duplicate Mesh Cleanup option",
            ),
        ];
        for (command, reason) in cases {
            a.status = "untouched sentinel".into();
            a.execute(&command);
            assert!(a.editor == before, "{command}: {}", a.status);
            assert!(a.status.contains(reason), "{command}: {}", a.status);
            assert_eq!(a.editor.identities.next_id().unwrap(), next);
        }
    }

    // SS-O-001 admits graph isomorphism and rotated (not reversed) cycles.
    // Oracle "Default" maps to Operandi's empty material / absent part, the
    // same mapping used by the existing core Smooth Shift fixture adapter.
    fn assert_ss_o001(stage: &Scene) {
        let item = stage.active().unwrap();
        let mesh = &item.mesh;
        let positions = [
            DVec3::new(-1., -0.5, 0.),
            DVec3::new(1., -0.5, 0.),
            DVec3::new(1., 0.5, 0.),
            DVec3::new(-1., 0.5, 0.),
            DVec3::new(-1., -0.5, 0.2),
            DVec3::new(1., -0.5, 0.2),
            DVec3::new(1., 0.5, 0.2),
            DVec3::new(-1., 0.5, 0.2),
        ];
        assert_eq!(mesh.vertices.len(), positions.len());
        let actual_to_oracle: Vec<usize> = mesh
            .vertices
            .iter()
            .map(|point| {
                let matches: Vec<usize> = positions
                    .iter()
                    .enumerate()
                    .filter(|(_, expected)| (**expected - *point).length() < 2e-5)
                    .map(|(i, _)| i)
                    .collect();
                assert_eq!(matches.len(), 1);
                matches[0]
            })
            .collect();
        assert_eq!(
            actual_to_oracle
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            positions.len()
        );
        let expected_faces = [
            vec![4, 5, 6, 7],
            vec![5, 4, 0, 1],
            vec![6, 5, 1, 2],
            vec![7, 6, 2, 3],
            vec![4, 7, 3, 0],
        ];
        assert_eq!(mesh.faces.len(), expected_faces.len());
        assert_eq!(stage.selection.mode, Mode::Polygon);
        assert_eq!(stage.selection.faces.len(), 1);
        let mut seen = std::collections::BTreeSet::new();
        for (index, face) in mesh.faces.iter().enumerate() {
            let cycle: Vec<usize> = face
                .vertices
                .iter()
                .map(|&v| actual_to_oracle[v as usize])
                .collect();
            let found = expected_faces
                .iter()
                .position(|want| {
                    cycle.len() == want.len()
                        && (0..want.len()).any(|shift| {
                            (0..want.len()).all(|i| cycle[i] == want[(i + shift) % want.len()])
                        })
                })
                .expect("oriented SS-O-001 face");
            assert!(seen.insert(found));
            assert_eq!(stage.selection.faces.contains(&(index as u32)), found == 0);
            assert!(face.material.is_empty(), "Default material");
            assert_eq!(face.part, None, "Default part");
            assert_eq!(face.material_label, None, "no additional material tag");
            assert_eq!(
                mesh.attributes
                    .representations
                    .get(index)
                    .copied()
                    .unwrap_or(PolygonRepresentation::Face),
                PolygonRepresentation::Face
            );
            assert!(
                mesh.attributes
                    .smoothing_labels
                    .get(index)
                    .is_none_or(Option::is_none)
            );
        }
        assert!(mesh.attributes.creases.is_empty());
        // SS-O-001 supplies no UV/map or additional per-corner oracle. Those
        // fields are deliberately not inferred from the preview here.
    }

    #[test]
    fn new_mesh_routes_resolve_smooth_shift_stage_against_ss_o_001() {
        for route in ["row", "menu", "callable"] {
            let mut a = app();
            a.editor.scene.new_mesh().unwrap();
            let mesh = &mut a.editor.scene.active_mut().unwrap().mesh;
            *mesh = Mesh {
                vertices: vec![
                    DVec3::new(-1., -0.5, 0.),
                    DVec3::new(1., -0.5, 0.),
                    DVec3::new(1., 0.5, 0.),
                    DVec3::new(-1., 0.5, 0.),
                ],
                faces: vec![Face {
                    vertices: vec![0, 1, 2, 3],
                    uv: None,
                    material: "".into(),
                    smooth: false,
                    part: None,
                    material_label: None,
                }]
                .into(),
                ..Default::default()
            }
            .into();
            a.editor.scene.selection.mode = Mode::Polygon;
            a.editor.scene.selection.faces = vec![0];
            a.editor.synchronize_identities().unwrap();
            let source = a.editor.scene.clone();
            a.run(Command::Activate(Tool::SmoothShift));
            a.run(Command::ConfigureSmoothShift(SmoothShiftParameters {
                offset: 0.2,
                ..Default::default()
            }));
            let params = a.editor.session.live.as_ref().unwrap().params.clone();
            a.run(Command::Preview(params));
            let stage = a.editor.scene.clone();
            let live = a
                .editor
                .session
                .live
                .as_ref()
                .expect("confirmed live stage");
            assert!(live.applied);
            assert!(live.preview_error.is_none());
            assert_eq!(
                live.params.smooth_shift,
                SmoothShiftParameters {
                    offset: 0.2,
                    ..Default::default()
                }
            );
            assert_ss_o001(&stage);
            // Outside mesh/selection, the stage preserves the entire source Scene.
            let mut stage_without_result = stage.clone();
            stage_without_result.active_mut().unwrap().mesh = source.active().unwrap().mesh.clone();
            stage_without_result.selection = source.selection.clone();
            assert_eq!(stage_without_result, source);
            // Oracle-unspecified mesh fields are checked for retarget preservation
            // using the observed stage, not claimed as independent oracle values.
            let stage_ids = a.editor.identities.current.clone();
            invoke_new_mesh(&mut a, route);
            let result = &a.editor.scene;
            assert_eq!(result.items.len(), stage.items.len() + 1);
            let id = stage.next_id;
            assert_eq!(result.next_id, id + 1);
            assert_eq!(
                result.active().unwrap(),
                &operandi_core::scene::Item {
                    id,
                    // "Mesh", or "Mesh (n)" when taken (new-mesh NM06).
                    name: result.active().unwrap().name.clone(),
                    visible: true,
                    transform: Default::default(),
                    mesh: Mesh::default().into(),
                    parent: None,
                    source_id: None,
                    source_display_name: None,
                    subdivision: None,
                    backdrop: None,
                    camera: None,
                    light: None,
                    texture: None,
                    locator: None,
                    environment: None,
                    common: Default::default(),
                    elements: Default::default(),
                    hides_children: false,
                    mesh_stack: None,
                    kit: None,
                    instance_of: None,
                    replicator: None,
                    multires: None,
                }
            );
            assert_eq!(
                result.selection,
                operandi_core::Selection {
                    item: Some(id),
                    mode: Mode::Polygon,
                    ..Default::default()
                }
            );
            let mut retained = result.clone();
            retained.items.pop();
            retained.next_id = stage.next_id;
            retained.selection = stage.selection.clone();
            assert_eq!(retained, stage, "stage preservation: {route}");
            assert_eq!(a.editor.history.undo.last().unwrap().label, "New Mesh");
            a.run(Command::Undo);
            assert_eq!(a.editor.scene, stage, "{route}");
            assert_eq!(a.editor.identities.current, stage_ids, "{route}");
        }
    }
}
