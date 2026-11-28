//! The app's handlers for registry commands (architecture review
//! 2026-11-14, pick E). `execute` looks a line up in the run-time
//! registry (`operandi_core::command_runtime`) and calls the handler here;
//! lines no handler takes yet still go through the older parsers after
//! it, which move here a few at a time.
//!
//! Handlers attach to registry entries by name, so a handler for a
//! command the registry doesn't list can't be written, and two handlers
//! for one command are refused when the registry is built.
use super::*;
use operandi_core::command_registry::{self as table, Area, ArgType, Unit};
use operandi_core::command_runtime::{Call, CommandState, HostHandler, Registry};
use std::rc::Rc;

pub(crate) type Commands = Registry<Operandi>;

thread_local! {
    /// Built once per thread; each app holds a cheap reference to it.
    static BUILT: Rc<Commands> = Rc::new(build());
}

/// The registry with the app's handlers and its own commands.
pub(crate) fn registry() -> Rc<Commands> {
    BUILT.with(Rc::clone)
}

/// Attaches `run` to the command each of `names` names, for its bare line
/// only (what the old typed `match` took; a phrase's arguments, such as
/// `bridge segments 3`, stay with the parser that reads them).
fn on(r: &mut Commands, names: &[&str], run: HostHandler<Operandi>) {
    attach(r, names, run, true);
}

/// [`on`] for a handler that reads the line's words itself (`c.rest`).
pub(super) fn on_words(r: &mut Commands, names: &[&str], run: HostHandler<Operandi>) {
    attach(r, names, run, false);
}

fn attach(r: &mut Commands, names: &[&str], run: HostHandler<Operandi>, bare: bool) {
    for name in names {
        let Some((command, rest)) = r.find_line(name) else {
            panic!("{name} isn't in command_registry.rs");
        };
        assert!(
            rest.is_empty(),
            "{name}: give the command's name, not a line"
        );
        let entry = command.entry.name;
        let attached = if bare {
            r.handle_bare(entry, run)
        } else {
            r.handle(entry, run)
        };
        attached.unwrap_or_else(|e| panic!("{e}"));
    }
}

/// The argument `name` of a line's words: `name:value` anywhere, else the
/// `index`th word without a name. Quotes are dropped.
fn named_word(rest: &str, name: &str, index: usize) -> Option<String> {
    let words: Vec<&str> = rest.split_whitespace().collect();
    let unquote = |w: &str| w.trim_matches('"').to_owned();
    let prefix = format!("{name}:");
    if let Some(w) = words
        .iter()
        .find(|w| w.len() >= prefix.len() && w[..prefix.len()].eq_ignore_ascii_case(&prefix))
    {
        return Some(unquote(&w[prefix.len()..]));
    }
    words
        .iter()
        .filter(|w| !w.contains(':'))
        .nth(index)
        .map(|w| unquote(w))
}

/// The Select tab's selection lines (its form, `toolbox.select.json`).
const SELECT_LINES: &[&str] = &[
    "Select All",
    "Invert",
    "Connected",
    "Invert Connected",
    "Grow",
    "Shrink",
    "Close Loop",
    "Loop",
    "Next Loop",
    "Previous Loop",
    "Ring",
    "Select More",
    "Select Less",
    "Convert to Vertices",
    "Boundary",
    "Select Between",
];

/// The menus' other lines that wait like the selection commands (menus as
/// forms: Select's operations, Edit's Lock and View's Hide).
const MENU_SELECTION_LINES: &[&str] = &[
    "Convert to Edges",
    "Convert to Polygons",
    "Lock Selected",
    "Lock Unselected",
    "Lock Invert",
    "Unlock",
    "Hide Selected",
    "Hide Unselected",
    "Hide Invert",
    "Unhide",
];

/// Why selection commands can't run now: a tool or a primitive, spike or
/// bridge session is open (the menus' old rule, `unavailable::TOOL_ACTIVE`).
fn selection_why(a: &Operandi) -> Option<&'static str> {
    crate::unavailable::unless(
        a.editor.live.is_none()
            && a.editor.ellipsoid.is_none()
            && a.editor.shape.is_none()
            && a.editor.spike.is_none()
            && a.editor.bridge.is_none(),
        crate::unavailable::TOOL_ACTIVE,
    )
}

fn selection_state(a: &Operandi, _: &Call<'_>) -> CommandState {
    CommandState::unless(selection_why(a))
}

/// `operandi.notBuilt`'s name, unquoted.
/// The whole rest is the name, so it may hold spaces.
fn not_built_name(rest: &str) -> String {
    let rest = rest.trim();
    let rest = match rest.get(..5) {
        Some(head) if head.eq_ignore_ascii_case("name:") => &rest[5..],
        _ => rest,
    };
    rest.trim().trim_matches('"').to_owned()
}

/// The tool preset rows on the Deform tab (by id), which light from
/// the open preset.
const DEFORM_PRESETS: &[&str] = &[
    "xfrm.magnet",
    "xfrm.shear",
    "Flex",
    "xfrm.twist",
    "Swirl",
    "xfrm.taper",
    "xfrm.pole",
    "Flare",
    "ElementPush",
    "xfrm.softDrag",
    "xfrm.vortex",
    "SoftSelectionMove",
    "SoftSelectionRotate",
    "SoftSelectionScale",
    "SoftSelectionTransform",
];

type StateHandler = operandi_core::command_runtime::StateHandler<Operandi>;

/// The UV tab's stage 3 commands (uv_ops.rs) and their dialog opener.
const UV_OP_COMMANDS: &[&str] = &[
    "operandi.uv.dialog",
    "operandi.uv.quickAlign",
    "operandi.uv.islandAlign",
    "operandi.uv.orient",
    "operandi.uv.toOrigin",
    "operandi.uv.texel",
    "operandi.uv.realWorldSize",
    "operandi.uv.split",
    "operandi.uv.sew",
    "operandi.uv.moveSew",
    "operandi.uv.selectBorder",
    "operandi.uv.selectErrors",
    "operandi.uv.selectUnmapped",
    "operandi.uv.copy",
    "operandi.uv.cut",
    "operandi.uv.paste",
    "operandi.uv.delete",
    "operandi.uv.setValue",
    "operandi.uv.rotatePolygons",
    "operandi.uv.mergeMaps",
    "operandi.uv.seam",
    "operandi.uv.symmetryCenter",
    "operandi.uv.symmetrize",
    "operandi.uv.selectTile",
    "operandi.uv.nudgeTile",
    "operandi.uv.toMesh",
    "operandi.uv.transfer",
];

/// A Selection Set command's answer: greyed with the dialog's refusal.
fn set_state(a: &Operandi, action: crate::selection_set_dialog::SetAction) -> CommandState {
    CommandState {
        unavailable: a.selection_set_unavailable(action),
        on: false,
        title: None,
    }
}

/// The commands' answers about their own state (UIC-223): what a button,
/// menu or form running them shows as lit or greyed.
fn answers(r: &mut Commands) {
    // A tool's command is on while that tool is the live one.
    // Radial and Linear Align have their own answer below (lit and greyed).
    for tool in Tool::ALL
        .into_iter()
        .filter(|t| !matches!(t, Tool::RadialAlign | Tool::LinearAlign))
    {
        for name in tool_info::info(tool).names {
            r.answer(name, |a, c| {
                CommandState::lit(
                    tool_info::by_name(&c.lowered())
                        .is_some_and(|t| a.editor.live.as_ref().is_some_and(|l| l.tool == t)),
                )
            })
            .unwrap_or_else(|e| panic!("{e}"));
        }
    }
    // The Select tab's selection commands wait for the active tool to
    // drop, like the old tab's rows (layouts stage 3 batch 2).
    // The Select menu's operations, the Edit menu's Lock and the View
    // menu's Hide wait for every open session too (menus as forms: the
    // menus' rule, now the commands' own).
    for line in SELECT_LINES.iter().chain(MENU_SELECTION_LINES) {
        let name = r
            .find_line(line)
            .map(|(c, _)| c.entry.name)
            .unwrap_or_else(|| panic!("{line} isn't a command"));
        r.answer(name, selection_state)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    // Symmetrical also needs Symmetry on, like the Symmetry menu's entry.
    let symmetrical = r
        .find_line("Select Symmetrical")
        .map(|(c, _)| c.entry.name)
        .expect("Select Symmetrical is a command");
    r.answer(symmetrical, |a, _| {
        CommandState::unless(crate::unavailable::first([
            crate::unavailable::unless(a.editor.live.is_none(), crate::unavailable::TOOL_ACTIVE),
            crate::unavailable::unless(
                a.editor.symmetry.axis.is_some(),
                "Turn Symmetry on (X, Y or Z) first",
            ),
        ]))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    // The Selection Sets rows (bare lines open their dialogs, CMD-12)
    // grey for the same reasons the dialogs refuse to open.
    use crate::selection_set_dialog::SetAction;
    let sets: [(&str, StateHandler); 4] = [
        ("select.editSet", |a, _| set_state(a, SetAction::Assign)),
        ("select.convertGroup", |a, _| {
            set_state(a, SetAction::CreateGroup)
        }),
        ("select.useSet", |a, _| set_state(a, SetAction::Use)),
        ("select.deleteSet", |a, _| set_state(a, SetAction::Delete)),
    ];
    for (name, state) in sets {
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
    // The Deform tab's tool presets light while they are the open preset
    // (layouts stage 3 batch 3; the old rows read the preset themselves).
    for id in DEFORM_PRESETS {
        let row = operandi_core::tool_preset::row(id)
            .unwrap_or_else(|| panic!("tool preset {id} is in the table"));
        r.answer(&row.label, |a, c| {
            CommandState::lit(
                operandi_core::tool_preset::row(c.entry.name)
                    .is_some_and(|row| a.editor.preset.as_ref().is_some_and(|p| p.id == row.id)),
            )
        })
        .unwrap_or_else(|e| panic!("{e}"));
    }
    // Radial and Linear Align grey until there is something to align
    // (DEFORM §1), and light while they are the live tool.
    for name in ["Radial Align", "Linear Align"] {
        r.answer(name, |a, c| {
            let tool = if c.entry.name == "Radial Align" {
                Tool::RadialAlign
            } else {
                Tool::LinearAlign
            };
            let live = a.editor.live.as_ref().is_some_and(|l| l.tool == tool);
            let s = &a.editor.scene.selection;
            let usable = match s.mode {
                Mode::Vertex => s.vertices.len() >= 2,
                Mode::Edge => !s.edges.is_empty(),
                Mode::Polygon => !s.faces.is_empty(),
                Mode::Item => false,
            };
            CommandState {
                unavailable: (!live && !usable).then(|| {
                    "Select an edge loop or run, two or more points, or polygons first".to_owned()
                }),
                on: live,
                title: None,
            }
        })
        .unwrap_or_else(|e| panic!("{e}"));
    }
    // The view header's toggles (HT-3) light from these (UIC-223): the
    // view's own settings, the same fields as Viewport Properties.
    let toggles: [(&str, StateHandler); 6] = [
        ("view3d.showweights", |a, _| {
            CommandState::lit(a.display.settings.drawing.weights)
        }),
        ("view.topologyMode", |a, _| {
            CommandState::lit(a.display.settings.drawing.topology)
        }),
        ("view3d.showDistortion", |a, _| {
            CommandState::lit(a.display.settings.drawing.distortion)
        }),
        ("view3d.showLocators", |a, _| {
            CommandState::lit(a.display.settings.drawing.locators)
        }),
        ("view3d.inactiveInvisible", |a, _| {
            CommandState::lit(a.display.settings.drawing.inactive_invisible)
        }),
        ("view3d.sameAsActive", |a, _| {
            CommandState::lit(a.display.settings.drawing.inactive_same_as_active)
        }),
    ];
    for (name, state) in toggles {
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
    // Lazy Selection and Shortest Path Mode light while on (SL-03, SL-70).
    r.answer("select.lazyState", |a, _| {
        CommandState::lit(a.selection_preferences.lazy_selection)
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("Shortest Path Mode", |a, _| {
        CommandState::lit(a.selection_preferences.shortest_path_mode)
    })
    .unwrap_or_else(|e| panic!("{e}"));
    // Quick Align and Ground Align light while their mode is on.
    for name in ["Quick Align", "Ground Align"] {
        r.answer(name, |a, c| {
            CommandState::lit(a.utility.is_some_and(|u| u.label() == c.entry.name))
        })
        .unwrap_or_else(|e| panic!("{e}"));
    }
    // The Vertex and Edge tabs' modes light while they are on (layouts
    // stage 3 batch 4; the old rows read the state themselves).
    let lit: [(&str, StateHandler); 5] = [
        ("Weld", |a, _| CommandState::lit(a.vertex_weld)),
        ("Bridge", |a, _| {
            CommandState::lit(a.editor.bridge.is_some())
        }),
        ("Add Loop", |a, _| {
            CommandState::lit(a.edge_click == Some(edge_panel::EdgeClick::AddLoop))
        }),
        ("Add Point", |a, _| {
            CommandState::lit(a.edge_click == Some(edge_panel::EdgeClick::AddPoint))
        }),
        ("Show Vertices", |a, _| {
            CommandState::lit(a.display.settings.drawing.show_verts)
        }),
    ];
    for (name, state) in lit {
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
    // The Rig tab's Compensation lights while it is on (Jason, 2026-11-23).
    r.answer("operandi.rig.compensation", |a, _| {
        CommandState::lit(a.editor.rig_compensation)
    })
    .unwrap_or_else(|e| panic!("{e}"));
    // The Polygon tab's sessions and dialogs light while they are open
    // (layouts stage 3 batch 5; the old rows read the state themselves).
    let lit: [(&str, StateHandler); 9] = [
        ("Polygon Pen", |a, _| {
            CommandState::lit(a.editor.pen_draft.is_some())
        }),
        ("Spike", |a, _| CommandState::lit(a.editor.spike.is_some())),
        ("Polygon Reduction", |a, _| {
            CommandState::lit(a.editor.reduction.is_some())
        }),
        ("Set Position", |a, _| {
            CommandState::lit(a.set_position.is_some())
        }),
        ("Scale Offset", |a, _| {
            CommandState::lit(a.scale_offset.is_some())
        }),
        ("Flip It", |a, _| {
            CommandState::lit(a.flip_it.as_ref().is_some_and(|d| d.preset == "FlipIt"))
        }),
        ("Flip It Origin", |a, _| {
            CommandState::lit(
                a.flip_it
                    .as_ref()
                    .is_some_and(|d| d.preset == "FlipItOrigin"),
            )
        }),
        ("Set Part", |a, _| CommandState::lit(a.set_part.is_some())),
        ("Set Material", |a, _| {
            CommandState::lit(a.set_material.is_some())
        }),
    ];
    for (name, state) in lit {
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
    // Drop Weld only while Weld is on: the Vertex tab shows its Drop Weld
    // button then (LAY-85).
    r.answer("Drop Weld", |a, _| {
        CommandState::unless(crate::unavailable::unless(a.vertex_weld, "Weld is off"))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    // New Vertex and Reorder Index only make sense on points (their
    // dialogs), so they grey outside Vertex mode, as their rows did.
    for name in ["New Vertex", "Reorder Index"] {
        r.answer(name, |a, _| {
            CommandState::unless(crate::unavailable::unless(
                a.editor.scene.selection.mode == Mode::Vertex,
                "Switch to Vertices mode first",
            ))
        })
        .unwrap_or_else(|e| panic!("{e}"));
    }
    edit_answers(r);
    sculpt_answers(r);
    crate::multires_ui::multires_answers(r);
    create_answers(r);
    // A placeholder is never runnable: it says what isn't built.
    r.answer("operandi.notBuilt", |_, c| CommandState {
        unavailable: Some(format!("{} · not in Operandi yet", not_built_name(c.rest))),
        on: false,
        title: None,
    })
    .unwrap_or_else(|e| panic!("{e}"));
    // The UV island buttons wait for the active tool to drop.
    r.answer("operandi.uv.preset", |a, _| {
        CommandState::unless(crate::unavailable::unless(
            a.editor.live.is_none(),
            crate::unavailable::TOOL_ACTIVE,
        ))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    // So do the UV tab's stage 3 buttons and their dialogs.
    for name in UV_OP_COMMANDS {
        r.answer(name, |a, _| {
            CommandState::unless(crate::unavailable::unless(
                a.editor.live.is_none(),
                crate::unavailable::TOOL_ACTIVE,
            ))
        })
        .unwrap_or_else(|e| panic!("{e}"));
    }
}

/// The Create tab's sessions, draw tools and utilities light while they
/// are on (layouts stage 3 batch 7; the old cells read the state
/// themselves).
fn create_answers(r: &mut Commands) {
    fn shape(a: &Operandi, kind: ShapeKind) -> CommandState {
        CommandState::lit(a.editor.shape.as_ref().is_some_and(|s| s.kind() == kind))
    }
    fn draw(a: &Operandi, kind: draw_tools::DrawKind) -> CommandState {
        CommandState::lit(a.draw.as_ref().is_some_and(|d| d.kind == kind))
    }
    let lit: [(&str, StateHandler); 16] = [
        ("Ellipsoid", |a, _| {
            CommandState::lit(a.editor.ellipsoid.is_some())
        }),
        ("Cone", |a, _| CommandState::lit(a.editor.cone.is_some())),
        ("Torus", |a, _| shape(a, ShapeKind::Torus)),
        ("Capsule", |a, _| shape(a, ShapeKind::Capsule)),
        ("Arc", |a, _| shape(a, ShapeKind::Arc)),
        ("Tube", |a, _| shape(a, ShapeKind::Tube)),
        ("Text", |a, _| shape(a, ShapeKind::Text)),
        ("Teapot", |a, _| shape(a, ShapeKind::Teapot)),
        ("Solid Sketch", |a, _| {
            CommandState::lit(a.editor.solid_sketch.is_some())
        }),
        ("Curve", |a, _| draw(a, draw_tools::DrawKind::Curve)),
        ("Bezier Curve", |a, _| draw(a, draw_tools::DrawKind::Bezier)),
        ("B-Spline", |a, _| draw(a, draw_tools::DrawKind::BSpline)),
        ("Sketch", |a, _| draw(a, draw_tools::DrawKind::Sketch)),
        ("Draw Patch", |a, _| draw(a, draw_tools::DrawKind::Patch)),
        ("Snaps and Precision", |a, _| {
            CommandState::lit(a.snaps_open)
        }),
        ("Paste Tool", |a, c| {
            CommandState::lit(a.utility.is_some_and(|u| u.label() == c.entry.name))
        }),
    ];
    for (name, state) in lit {
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
    for name in ["Ruler", "Protractor", "Dimensions"] {
        r.answer(name, |a, c| {
            CommandState::lit(a.utility.is_some_and(|u| u.label() == c.entry.name))
        })
        .unwrap_or_else(|e| panic!("{e}"));
    }
    // #46: Mesh Paint lights while any of its variants is on (it is the
    // button the Alt variant sits on); each variant lights by its own name.
    r.answer("Mesh Paint", |a, _| {
        CommandState::lit(matches!(
            a.utility,
            Some(create_utility::Utility::MeshPaint(_))
        ))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    for name in ["Mesh Paint Instance", "Mesh Paint Instancer"] {
        r.answer(name, |a, c| {
            CommandState::lit(a.utility.is_some_and(|u| u.label() == c.entry.name))
        })
        .unwrap_or_else(|e| panic!("{e}"));
    }
}

/// The slide tool of the selection mode (the Edit tab's Slide).
fn mode_slide(a: &Operandi) -> Option<Tool> {
    match a.editor.scene.selection.mode {
        Mode::Vertex => Some(Tool::VertexSlide),
        Mode::Edge => Some(Tool::EdgeSlide),
        _ => None,
    }
}

/// `operandi.mode.is`'s modes are on now.
fn mode_is(a: &Operandi, modes: &str) -> bool {
    let mode = a.editor.scene.selection.mode;
    modes.split_whitespace().any(|w| {
        let named = match w.to_ascii_lowercase().as_str() {
            "item" => Some(Mode::Item),
            "vertex" => Some(Mode::Vertex),
            "edge" => Some(Mode::Edge),
            "polygon" => Some(Mode::Polygon),
            _ => None,
        };
        named == Some(mode)
    })
}

/// Idle: no tool live (the Edit tab's commands wait for it to drop).
fn idle(a: &Operandi) -> Option<&'static str> {
    crate::unavailable::unless(a.editor.live.is_none(), crate::unavailable::TOOL_ACTIVE)
}

/// The Edit tab's answers (layouts stage 3 batch 6; the old rows read the
/// state themselves).
fn edit_answers(r: &mut Commands) {
    r.answer("Edit Slide", |a, _| {
        let tool = mode_slide(a);
        CommandState {
            unavailable: crate::unavailable::unless(
                tool.is_some(),
                "Switch to Vertices or Edges mode (Polygon Slide isn't built yet)",
            )
            .map(str::to_owned),
            on: tool.is_some_and(|t| a.editor.live.as_ref().is_some_and(|l| l.tool == t)),
            title: None,
        }
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("operandi.mode.is", |a, c| {
        CommandState::unless(crate::unavailable::unless(
            mode_is(a, c.rest),
            "Not in that selection mode",
        ))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    // Subdivide and Center wait for the active tool, as their buttons did.
    for name in [
        "SDS Subdivide",
        "SDS Subdivide 2X",
        "Faceted Subdivide",
        "Smooth Subdivide",
        "Center X",
        "Center Y",
        "Center Z",
        "Center All",
    ] {
        r.answer(name, |a, _| CommandState::unless(idle(a)))
            .unwrap_or_else(|e| panic!("{e}"));
    }
    // Edit > Center Selected (center-selected-2026-11-11 §5): refused in
    // Item mode, else waits like the selection commands.
    r.answer("vert.center", |a, _| {
        if a.editor.scene.selection.mode == Mode::Item {
            CommandState::unless(Some(
                "Center works on vertices, edges or polygons; switch to a component mode",
            ))
        } else {
            CommandState::unless(selection_why(a))
        }
    })
    .unwrap_or_else(|e| panic!("{e}"));
    let open: [(&str, StateHandler); 7] = [
        ("Subdivide Options", |a, _| CommandState {
            unavailable: idle(a).map(str::to_owned),
            on: a.subdivide_ui.options.is_some(),
            title: None,
        }),
        ("UnSubdivide", |a, _| CommandState {
            unavailable: idle(a).map(str::to_owned),
            on: a.subdivide_ui.unsubdivide.is_some(),
            title: None,
        }),
        ("poly.freeze", |a, _| CommandState {
            unavailable: idle(a).map(str::to_owned),
            on: a.subdivide_ui.freeze.is_some(),
            title: None,
        }),
        ("Boolean", |a, _| CommandState {
            unavailable: idle(a).map(str::to_owned),
            on: a.boolean.is_some(),
            title: None,
        }),
        ("Solid Drill", |a, _| CommandState {
            unavailable: idle(a).map(str::to_owned),
            on: a
                .drill
                .as_ref()
                .is_some_and(|d| d.kind() == operandi_core::DrillKind::Solid),
            title: None,
        }),
        ("Axis Drill", |a, _| CommandState {
            unavailable: idle(a).map(str::to_owned),
            on: a
                .drill
                .as_ref()
                .is_some_and(|d| d.kind() == operandi_core::DrillKind::Axis),
            title: None,
        }),
        ("Automatic Retopology", |a, _| {
            remesh_state(a, crate::remesh_dialog::RemeshKind::Retopo)
        }),
    ];
    for (name, state) in open {
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
    r.answer("Remesh", |a, _| {
        remesh_state(a, crate::remesh_dialog::RemeshKind::Remesh)
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

/// The Sculpt tab's answers (layouts stage 3 batch 6): brushes light while
/// active and grey when unavailable, tips and locks light from settings.
fn sculpt_answers(r: &mut Commands) {
    use operandi_core::SculptBrush;
    for brush in SculptBrush::every() {
        r.answer(crate::sculpt_ui::action(brush), |a, c| {
            let brush = SculptBrush::every()
                .find(|b| crate::sculpt_ui::action(*b) == c.entry.name)
                .expect("a brush's action");
            CommandState {
                unavailable: brush
                    .unavailable_on(&a.editor.scene, &a.sculpt_targets())
                    .map(str::to_owned),
                on: a.sculpt.brush == Some(brush),
                title: None,
            }
        })
        .unwrap_or_else(|e| panic!("{e}"));
    }
    fn tip(a: &Operandi, kind: operandi_core::sculpt::TipKind) -> CommandState {
        let current = a.sculpt.active().map(|(_, s)| s.tip.kind);
        CommandState {
            unavailable: crate::unavailable::unless(current.is_some(), "Pick a brush first")
                .map(str::to_owned),
            on: current == Some(kind),
            title: None,
        }
    }
    let lit: [(&str, StateHandler); 6] = [
        ("brush.smooth", |a, _| {
            tip(a, operandi_core::sculpt::TipKind::Smooth)
        }),
        ("brush.hard", |a, _| {
            tip(a, operandi_core::sculpt::TipKind::Hard)
        }),
        ("sculpt.borderLock", |a, _| {
            CommandState::lit(a.sculpt.settings.lock_borders)
        }),
        ("sculpt.cornerLock", |a, _| {
            CommandState::lit(a.sculpt.settings.lock_corners)
        }),
        ("sculpt.liveWeightEdit", |a, _| {
            CommandState::lit(a.sculpt.settings.live_deformers)
        }),
        ("sculpt.remesh", |a, _| {
            remesh_state(a, crate::remesh_dialog::RemeshKind::Remesh)
        }),
    ];
    for (name, state) in lit {
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
}

/// Remesh / Automatic Retopology: lit while its dialog is open, greyed
/// while a tool is live or a remesh is running.
fn remesh_state(a: &Operandi, kind: crate::remesh_dialog::RemeshKind) -> CommandState {
    CommandState {
        unavailable: crate::unavailable::unless(
            a.editor.live.is_none() && a.remesh_job.is_none(),
            crate::unavailable::TOOL_ACTIVE,
        )
        .map(str::to_owned),
        on: a.remesh.as_ref().is_some_and(|d| d.kind == kind),
        title: None,
    }
}

/// Window-only commands that only the app has (benchmarks): they need the
/// window, so they live here rather than in core's table.
const ORBIT_ARGS: &[table::Arg] = &[table::opt("frames", ArgType::Int, Unit::None, Some("360"))];
const DRAG_ARGS: &[table::Arg] = &[
    table::opt("frames", ArgType::Int, Unit::None, Some("240")),
    table::opt("anywhere", ArgType::Choice(&["anywhere"]), Unit::None, None),
];
const BENCH_ORBIT: table::Entry = table::script(
    "operandi.bench.orbit",
    Area::Debug,
    "Viewport speed benchmark: orbits for some frames.",
)
.operandi()
.args(ORBIT_ARGS);
const BENCH_DRAG: table::Entry =
    table::script("operandi.bench.drag", Area::Debug, "Item drag benchmark.")
        .operandi()
        .args(DRAG_ARGS);

fn bench_orbit(a: &mut Operandi, c: &Call<'_>) {
    // Viewport speed benchmark: "operandi.bench.orbit [frames]".
    match c.rest {
        "" => a.start_orbit_bench(360, None, false),
        n => match n.parse() {
            Ok(frames) => a.start_orbit_bench(frames, None, false),
            Err(_) => a.status = "Usage: operandi.bench.orbit [frames]".into(),
        },
    }
}

fn bench_drag(a: &mut Operandi, c: &Call<'_>) {
    // Item drag benchmark: "operandi.bench.drag [frames] [anywhere]".
    let mut frames = 240;
    let mut anywhere = false;
    for word in c.rest.split_whitespace() {
        match (word, word.parse()) {
            (_, Ok(n)) => frames = n,
            ("anywhere", _) => anywhere = true,
            _ => {
                a.status = "Usage: operandi.bench.drag [frames] [anywhere]".into();
                return;
            }
        }
    }
    a.start_drag_bench(frames, anywhere, None, false);
}

impl Operandi {
    /// Runs `text` through its registry handler; false when no handler
    /// takes it (the older parsers in `execute` do).
    pub(super) fn run_registered(&mut self, text: &str) -> bool {
        let line = text.trim();
        let Some((run, entry, rest)) = self.commands.host_call(line) else {
            return false;
        };
        run(
            self,
            &Call {
                entry: &entry,
                line,
                rest,
            },
        );
        true
    }
}

fn build() -> Commands {
    let mut r = Commands::built_in();
    for (entry, run) in [
        (BENCH_ORBIT, bench_orbit as HostHandler<Operandi>),
        (BENCH_DRAG, bench_drag),
    ] {
        r.add_host_command(entry, run)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    phrases(&mut r);
    tables(&mut r);
    workbench(&mut r);
    form_editor(&mut r);
    menu_commands(&mut r);
    menu_answers(&mut r);
    texture_tools(&mut r);
    answers(&mut r);
    #[cfg(feature = "painting")]
    crate::painting::register(&mut r);
    r
}

/// Texture > Atlas Unwrap..., the bakes and Pad Texture to UVs...: the
/// bare line opens the dialog (bake_dialogs.rs); a line with arguments
/// runs on the editor.
fn texture_tools(r: &mut Commands) {
    use operandi_core::bake::BakeMap;
    on(r, &["operandi.uv.atlas"], |a, _| a.open_atlas_unwrap());
    on(r, &["operandi.bake.curvature"], |a, _| {
        a.open_bake(BakeMap::Curvature)
    });
    on(r, &["operandi.bake.thickness"], |a, _| {
        a.open_bake(BakeMap::Thickness)
    });
    on(r, &["operandi.bake.padImage"], |a, _| a.open_pad_texture());
}

/// Layouts stage 4's workbench commands (workbench.rs, LAY-82, UIC-212)
/// and their answers (UIC-223).
const WORKBENCH_COMMANDS: &[&str] = &[
    "operandi.workbench.edit",
    "operandi.workbench.pick",
    "operandi.workbench.add",
    "operandi.workbench.addProperty",
    "operandi.workbench.addDivider",
    "operandi.workbench.addChain",
    "operandi.workbench.move",
    "operandi.workbench.set",
    "operandi.workbench.chooseIcon",
    "operandi.workbench.duplicate",
    "operandi.workbench.delete",
    "operandi.workbench.undo",
    "operandi.workbench.topBar",
];

/// Layouts stage 7's Form Editor commands (form_editor.rs, LAY-83).
const FORM_COMMANDS: &[&str] = &[
    "operandi.form.newGroup",
    "operandi.form.new",
    "operandi.form.add",
    "operandi.form.addForm",
    "operandi.form.addDivider",
    "operandi.form.move",
    "operandi.form.set",
    "operandi.form.delete",
    "operandi.form.select",
    "operandi.form.find",
    "operandi.form.preview",
    "operandi.form.export",
];

/// Layouts stage 7: the answers the menus drawn from forms read (UIC-223;
/// menu.help, menu.render, menu.texture, menu.system), the same greying
/// and ticks the hand-written menus had.
fn menu_answers(r: &mut Commands) {
    file_answers(r);
    select_answers(r);
    item_answers(r);
    view_answers(r);
    layout_answers(r);
    r.answer("app.undo", |a, _| {
        history_state(a, HistoryAction::Undo, "Undo")
    })
    .unwrap_or_else(|e| panic!("{e}"));
    // MN-05.2: greyed, with the reason, when no tool is active (UIC-03's
    // unavailable rule via `unavailable::unless`).
    r.answer("Reset Tool Attributes", |a, _| {
        CommandState::unless(crate::unavailable::unless(
            a.editor.has_active_task(),
            "No tool is active, so there is nothing to reset",
        ))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("app.redo", |a, _| {
        history_state(a, HistoryAction::Redo, "Redo")
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("render", |_, _| {
        CommandState::unless(crate::unavailable::unless(
            operandi_cycles::available(),
            NO_CYCLES,
        ))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("Preview", |a, _| CommandState {
        unavailable: crate::unavailable::unless(operandi_cycles::available(), NO_CYCLES)
            .map(str::to_owned),
        on: a.preview.open,
        title: None,
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("Open UV Editor", |a, _| {
        CommandState::lit(a.palette_open(crate::palettes::UV_EDITOR))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    // A palette's entry is ticked while it is open (LAY-55).
    r.answer("layout.window", |a, c| {
        let hash = command::tokens(c.rest)
            .ok()
            .and_then(|w| {
                operandi_core::command_args::bind("layout.window", &w, &["hash", "open"])
                    .ok()
                    .and_then(|b| b[0].map(str::to_owned))
            })
            .unwrap_or_default();
        CommandState::lit(a.palette_open(&hash))
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

/// An `on:` argument (`true`, `false`; missing toggles).
fn on_word(rest: &str) -> std::result::Result<Option<bool>, String> {
    crate::workbench::on_arg(named_word(rest, "on", 0).as_deref())
}

/// An `index:` argument, from 0.
fn index_word(rest: &str) -> Option<usize> {
    named_word(rest, "index", 0).and_then(|v| v.trim().parse().ok())
}

/// A project preset by its script name (`operandi.project.loadPreset`).
fn project_preset(rest: &str) -> Option<project_presets::Preset> {
    let name = named_word(rest, "preset", 0)?.to_ascii_lowercase();
    project_presets::Preset::ALL
        .into_iter()
        .find(|p| p.script_name().eq_ignore_ascii_case(&name))
}

/// A polygon tag kind by its script name.
fn tag_kind(rest: &str) -> Option<operandi_core::PolygonTagKind> {
    let name = named_word(rest, "type", 0)?.to_ascii_lowercase();
    operandi_core::PolygonTagKind::ALL
        .into_iter()
        .find(|k| k.label().replace(' ', "").eq_ignore_ascii_case(&name))
}

/// A lasso style by its script name.
fn lasso_style(rest: &str) -> Option<lasso::Style> {
    let name = named_word(rest, "style", 0)?.to_ascii_lowercase();
    lasso::Style::ALL
        .into_iter()
        .find(|s| s.label().eq_ignore_ascii_case(&name))
}

/// Sets a flag from an `on:` argument, or toggles it, quietly as the menu
/// entries did; a bad `on:` says so.
fn set_flag(a: &mut Operandi, rest: &str, flag: fn(&mut Operandi) -> &mut bool) {
    match on_word(rest) {
        Ok(on) => {
            let now = on.unwrap_or(!*flag(a));
            *flag(a) = now;
        }
        Err(e) => a.status = e,
    }
}

/// Menus as forms: the commands of menu entries that had none (File's
/// scene and project entries, Input Map, the polygon tag type and lasso
/// style, View's view flags, Window's entries). Each entry ran this code
/// straight from the menu before.
fn menu_commands(r: &mut Commands) {
    on_words(r, &["operandi.scene.current"], |a, c| {
        match index_word(c.rest).filter(|i| *i < a.scenes.tabs.len()) {
            Some(i) => {
                a.switch_scene(i);
            }
            None => {
                a.status =
                    "operandi.scene.current: index: takes an open scene's place, from 0".into()
            }
        }
    });
    on_words(
        r,
        &["operandi.project.new", "operandi.project.set"],
        |a, c| {
            let new = c.entry.name == "operandi.project.new";
            // CMD-12: without path: the dialog opens.
            let Some(path) = named_word(c.rest, "path", 0) else {
                a.open_project_dialog(if new {
                    project_ui::ProjectDialogKind::New
                } else {
                    project_ui::ProjectDialogKind::Set
                });
                return;
            };
            let root = PathBuf::from(&path);
            let result = if new {
                let name = named_word(c.rest, "name", 1).unwrap_or_else(|| {
                    root.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                });
                operandi_core::project::Project::create(&root, &name)
            } else {
                operandi_core::project::Project::open(&root)
            };
            match result {
                Ok(project) => a.set_project(project),
                Err(e) => a.status = e.to_string(),
            }
        },
    );
    on_words(
        r,
        &["operandi.project.openRecent"],
        |a, c| match index_word(c.rest).filter(|i| *i < a.recent_projects.len()) {
            Some(i) => a.open_recent_project(i),
            None if a.recent_projects.is_empty() => a.status = "No recent projects yet".into(),
            None => {
                a.status =
                    "operandi.project.openRecent: index: takes a Recent Projects place, from 0"
                        .into()
            }
        },
    );
    on_words(
        r,
        &["operandi.project.loadPreset", "operandi.project.savePreset"],
        |a, c| {
            let Some(preset) = project_preset(c.rest) else {
                a.status = format!(
                    "{}: preset: is layout, viewportDisplay or inputMap",
                    c.entry.name
                );
                return;
            };
            let Some(ctx) = a.frame_ctx.clone() else {
                a.status = "No window to apply the preset to".into();
                return;
            };
            if c.entry.name == "operandi.project.loadPreset" {
                a.load_project_preset(&ctx, preset);
            } else {
                a.save_project_preset(&ctx, preset);
            }
        },
    );
    on(r, &["operandi.inputMap"], |a, _| {
        a.open_input_map();
    });
    on_words(
        r,
        &["operandi.select.polygonTagType"],
        |a, c| match tag_kind(c.rest) {
            Some(kind) => a.run(Command::SetPolygonTagKind(kind)),
            None => {
                a.status = "operandi.select.polygonTagType: type: is material, part, \
                        smoothingGroup or selectionSet"
                    .into()
            }
        },
    );
    on_words(r, &["operandi.lasso.style"], |a, c| {
        match lasso_style(c.rest) {
            Some(style) => a.lasso_style = style,
            None => {
                a.status =
                    "operandi.lasso.style: style: is freeform, rectangle, circle or ellipse".into()
            }
        }
    });
    on_words(r, &["operandi.lasso.partial"], |a, c| {
        set_flag(a, c.rest, |a| &mut a.lasso_partial)
    });
    on_words(r, &["operandi.view.showLocked"], |a, c| {
        set_flag(a, c.rest, |a| &mut a.show_locked)
    });
    on_words(r, &["operandi.view.showFalloffs"], |a, c| {
        set_flag(a, c.rest, |a| &mut a.show_falloffs)
    });
    on(r, &["operandi.viewport.close"], |a, _| {
        a.close_view(a.active_view)
    });
    on(r, &["viewport.split"], |a, _| a.split_view());
    on(r, &["viewport.detach"], |a, _| {
        a.tear_off_view(a.active_view)
    });
    on(r, &["operandi.help.navigation"], |a, _| {
        a.status = format!("Navigate: {}", a.input_preferences.navigation.help())
    });
    on(r, &["operandi.window.diagnostics"], |a, _| {
        a.show_diagnostics = true
    });
    on_words(r, &["operandi.panel.show"], |a, c| {
        let result = command::tokens(c.rest)
            .map_err(str::to_owned)
            .and_then(|words| {
                let args =
                    operandi_core::command_args::bind("operandi.panel.show", &words, &["kind"])?;
                let name = args[0].unwrap_or_default();
                let kind =
                    layouts::kind_named(name).ok_or_else(|| format!("Unknown panel: {name}"))?;
                a.reveal_registry_panel(kind)
            });
        if let Err(error) = result {
            a.status = error;
        }
    });
    on_words(r, &["operandi.window.frameTime"], |a, c| {
        set_flag(a, c.rest, |a| &mut a.show_frame_time)
    });
    on_words(r, &["operandi.mcp.bridge"], |a, c| match on_word(c.rest) {
        Ok(on) => {
            let on = on.unwrap_or(!a.mcp.is_on());
            match a.frame_ctx.clone() {
                Some(ctx) => a.set_mcp(&ctx, on),
                None => a.status = "No window to run the MCP bridge in".into(),
            }
        }
        Err(e) => a.status = e,
    });
    on_words(r, &["operandi.window.size"], |a, c| {
        let size = match named_word(c.rest, "size", 0)
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("compact") => vec2(1100., 720.),
            Some("standard") => vec2(1440., 900.),
            _ => {
                a.status = "operandi.window.size: size: is compact or standard".into();
                return;
            }
        };
        if let Some(ctx) = &a.frame_ctx {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
        }
    });
}

/// Undo and Redo's answer (menus as forms): greyed with the history's own
/// reason, titled with the step they take ("Undo Cube").
fn history_state(a: &Operandi, action: HistoryAction, word: &str) -> CommandState {
    let query = a.editor.history_eligibility(action);
    let unavailable = match &query.availability {
        HistoryAvailability::Refused(reason) => Some(reason.to_string()),
        HistoryAvailability::EmptyHistory => Some("Nothing to undo or redo".to_owned()),
        HistoryAvailability::Available => None,
    };
    CommandState {
        unavailable,
        on: false,
        title: Some(
            query
                .label
                .map(|label| format!("{word} {label}"))
                .unwrap_or_else(|| word.to_owned()),
        ),
    }
}

/// File's answers (menus as forms): the open scene and project ticked,
/// project presets greyed as the old menu did.
fn file_answers(r: &mut Commands) {
    r.answer("operandi.scene.current", |a, c| {
        CommandState::lit(index_word(c.rest) == Some(a.scenes.active))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("operandi.project.openRecent", |a, c| {
        if a.recent_projects.is_empty() {
            return CommandState::unless(Some("No recent projects yet"));
        }
        let root = index_word(c.rest).and_then(|i| a.recent_projects.get(i));
        CommandState::lit(
            root.is_some_and(|root| a.project.as_ref().is_some_and(|p| p.root() == root)),
        )
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("operandi.project.loadPreset", |a, c| {
        CommandState::unless(crate::unavailable::unless(
            project_preset(c.rest).is_some_and(|p| a.project_has_preset(p)),
            "This project has no saved copy of this preset",
        ))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("operandi.project.savePreset", |a, _| {
        CommandState::unless(crate::unavailable::unless(
            a.project.is_some(),
            "Set a project first",
        ))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("Snapping Toggle", |a, _| {
        CommandState::lit(a.snapping.enabled)
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

/// Item > Transform's answers (spec freeze-transforms section 10 Q10):
/// Reset needs a selected item, Freeze a selected mesh item, neither a
/// live tool nor only locked items.
fn item_answers(r: &mut Commands) {
    fn reason(a: &Operandi, freeze: bool) -> Option<String> {
        if a.editor.live.is_some() {
            return Some(crate::unavailable::TOOL_ACTIVE.to_owned());
        }
        let items: Vec<&operandi_core::Item> = a
            .editor
            .scene
            .selected_items()
            .iter()
            .filter_map(|id| a.editor.scene.item(*id))
            .filter(|i| !freeze || i.is_mesh())
            .collect();
        if items.is_empty() {
            Some(if freeze {
                "Freeze bakes a mesh item's transform; select a mesh item".to_owned()
            } else {
                "Select an item".to_owned()
            })
        } else if items.iter().all(|i| i.is_locked()) {
            Some(crate::unavailable::LOCKED.to_owned())
        } else {
            None
        }
    }
    r.answer("transform.reset", |a, _| CommandState {
        unavailable: reason(a, false),
        on: false,
        title: None,
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("transform.freeze", |a, _| CommandState {
        unavailable: reason(a, true),
        on: false,
        title: None,
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

/// Attaches `state` to the command `line` names (a spelled phrase such as
/// "Split View" names its dotted command).
fn answer_line(r: &mut Commands, line: &str, state: StateHandler) {
    let name = r
        .find_line(line)
        .map(|(c, _)| c.entry.name)
        .unwrap_or_else(|| panic!("{line} isn't a command"));
    r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
}

/// A `tool.handleStyle` argument as the style it names; `None` for
/// `default` (an action) and anything else.
fn handle_style_named(rest: &str) -> Option<crate::handle_preferences::HandleDetail> {
    use crate::handle_preferences::HandleDetail;
    let word = rest.trim();
    match word.split_once(':').map_or(word, |(_, v)| v) {
        "invisible" => Some(HandleDetail::Invisible),
        "basic" => Some(HandleDetail::Basic),
        "advanced" => Some(HandleDetail::Advanced),
        _ => None,
    }
}

/// View's answers (menus as forms): the active view's state, as the
/// old View menu read it.
fn view_answers(r: &mut Commands) {
    let lit: [(&str, StateHandler); 6] = [
        ("operandi.view.showLocked", |a, _| {
            CommandState::lit(a.show_locked)
        }),
        ("operandi.view.showFalloffs", |a, _| {
            CommandState::lit(a.show_falloffs)
        }),
        ("UV View", |a, _| {
            CommandState::lit(a.is_uv_view(a.active_view))
        }),
        ("Turn", |a, _| {
            CommandState::lit(matches!(
                a.view_motion,
                Some(crate::view_nav::Motion::Turn { .. })
            ))
        }),
        ("operandi.window.frameTime", |a, _| {
            CommandState::lit(a.show_frame_time)
        }),
        ("operandi.mcp.bridge", |a, _| CommandState {
            unavailable: crate::unavailable::unless(
                crate::mcp::McpState::available(),
                "The MCP bridge needs macOS or Linux",
            )
            .map(str::to_owned),
            on: a.mcp.is_on(),
            title: None,
        }),
    ];
    for (name, state) in lit {
        answer_line(r, name, state);
    }
    // MN-01.1: View > Tool Handles > Draw Style ticks the current style
    // (UIC-10 sub-menus, UIC-24 state items: one of the group is on).
    // Default is an action (put back the factory Basic), not a state, so
    // it is never ticked.
    r.answer("tool.handleStyle", |a, c| {
        CommandState::lit(handle_style_named(c.rest) == Some(a.handle_preferences.selected))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    answer_line(r, "viewport.split", |a, _| {
        CommandState::unless(crate::unavailable::unless(
            a.view_splits(a.active_view, operandi_layout::Side::Right),
            "The view is too small to split that way",
        ))
    });
    // View > Maximise: its title says what it will do (run-time title).
    r.answer("viewport.maximize", |a, _| {
        let maximized = a.views.is_maximized(a.active_view);
        let title = if !a.views.in_group(a.active_view) {
            "Four Views"
        } else if maximized {
            "Restore Viewport Layout"
        } else {
            "Maximise Viewport"
        };
        CommandState::lit(maximized).titled(title)
    })
    .unwrap_or_else(|e| panic!("{e}"));
    answer_line(r, "viewport.detach", |a, _| {
        let pane = a.view_pane_of(a.active_view);
        CommandState::unless(crate::unavailable::unless(
            a.views.viewports().len() > 1
                && !a.views.last_in_main(pane)
                && !a.views.alone_in_window(pane),
            "Split the view first: a window's only view stays",
        ))
    });
    r.answer("operandi.viewport.close", |a, _| {
        CommandState::unless(crate::unavailable::unless(
            a.can_close_view(a.active_view),
            "A window keeps its last view",
        ))
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

/// Layout's answers (menus as forms, layouts stage 2): Rename and Delete
/// only for layouts you made, Reset only for shipped ones; the locks and
/// Toggle Palettes ticked.
fn layout_answers(r: &mut Commands) {
    fn layout<'a>(a: &'a Operandi, rest: &str) -> Option<&'a crate::layouts::Entry> {
        let name =
            named_word(rest, "layout", usize::MAX).unwrap_or_else(|| a.layouts.current.clone());
        a.layouts.find(&name)
    }
    r.answer("operandi.layout.rename", |a, c| {
        CommandState::unless(crate::unavailable::unless(
            layout(a, c.rest).is_some_and(|e| e.shipped.is_none()),
            "Shipped layouts keep their names",
        ))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("operandi.layout.delete", |a, c| {
        CommandState::unless(crate::unavailable::unless(
            layout(a, c.rest).is_some_and(|e| e.shipped.is_none()),
            "Shipped layouts can be reset, not deleted",
        ))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("operandi.layout.revert", |a, c| {
        CommandState::unless(crate::unavailable::unless(
            layout(a, c.rest).is_some_and(|e| e.shipped.is_some()),
            "A layout you made has no shipped default",
        ))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    let lit: [(&str, StateHandler); 5] = [
        ("layout.lockThumbs", |a, _| {
            CommandState::lit(a.dots.locks.thumbs)
        }),
        ("layout.lockDividers", |a, _| {
            CommandState::lit(a.dots.locks.dividers)
        }),
        ("operandi.layout.togglePalettes", |a, _| {
            CommandState::lit(a.palettes.hidden)
        }),
        ("operandi.layout.buildMode", |a, _| {
            CommandState::lit(a.dots.build_mode)
        }),
        // Ticked while the Node View is open (node_view_open.rs).
        ("operandi.window.nodeView", |a, _| {
            CommandState::lit(a.registry_panel_open(crate::panel_registry::PanelKind::NodeView))
        }),
    ];
    for (name, state) in lit {
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
}

/// Select's answers (menus as forms): the mode ticked (not while Materials
/// mode is on), the tag type and lasso style ticked.
fn select_answers(r: &mut Commands) {
    fn mode(a: &Operandi, mode: Mode) -> CommandState {
        CommandState::lit(a.editor.scene.selection.mode == mode && !a.tag_mode_active())
    }
    let modes: [(&str, StateHandler); 4] = [
        ("Vertices", |a, _| mode(a, Mode::Vertex)),
        ("Edges", |a, _| mode(a, Mode::Edge)),
        ("Polygons", |a, _| mode(a, Mode::Polygon)),
        ("Items", |a, _| mode(a, Mode::Item)),
    ];
    for (name, state) in modes {
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
    r.answer(crate::tag_mode_ui::ACTION, |a, _| {
        CommandState::lit(a.tag_mode_active())
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("operandi.select.polygonTagType", |a, c| {
        match tag_kind(c.rest) {
            Some(kind) => CommandState {
                unavailable: kind.unavailable_reason().map(str::to_owned),
                on: a.editor.polygon_tag_kind == kind,
                title: None,
            },
            None => CommandState::unless(Some("No such polygon tag type")),
        }
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("operandi.lasso.style", |a, c| {
        CommandState::lit(lasso_style(c.rest) == Some(a.lasso_style))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    r.answer("operandi.lasso.partial", |a, _| {
        CommandState::lit(a.lasso_partial)
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

/// Why Render and Preview are greyed in a build without Cycles.
const NO_CYCLES: &str = "This build has no Cycles renderer";

fn form_editor(r: &mut Commands) {
    on_words(r, FORM_COMMANDS, |a, c| {
        a.form_command(&c.entry.name.to_ascii_lowercase(), c.rest)
    });
    for name in ["operandi.form.find", "operandi.form.preview"] {
        r.answer(name, |a, c| a.form_state(c.entry.name, c.rest))
            .unwrap_or_else(|e| panic!("{e}"));
    }
}

fn workbench(r: &mut Commands) {
    on_words(r, WORKBENCH_COMMANDS, |a, c| {
        a.workbench_command(c.entry.name, c.rest)
    });
    for name in WORKBENCH_COMMANDS {
        r.answer(name, |a, c| a.workbench_state(c.entry.name, c.rest))
            .unwrap_or_else(|e| panic!("{e}"));
    }
}

/// The typed phrases that used to be `execute`'s big `match` (one line,
/// one command, no arguments unless the handler reads `c.rest`).
fn phrases(r: &mut Commands) {
    on(r, &["pen interactive", "polygon pen"], |a, _| {
        a.start_interactive_pen()
    });
    on(r, &["solid sketch"], |a, _| a.start_solid_sketch(true));
    on(r, &["bridge"], |a, _| a.run(Command::ArmBridge));
    on(r, &["polygon reduction"], |a, _| {
        a.run(Command::ArmReduction)
    });
    on(r, &["bridge reset"], |a, _| a.run(Command::ResetBridge));
    on(r, &["tool properties"], |a, _| {
        a.tool_properties_open = true;
        a.surface_mini_properties = a
            .editor
            .live
            .as_ref()
            .is_some_and(|l| l.tool.is_smooth_shift());
    });
    on(r, &["weld"], |a, _| a.toggle_weld());
    on(r, &["drop weld"], |a, _| {
        // Drop ends Weld (`run`), as the tab's Drop Weld button does.
        if a.vertex_weld {
            a.run(Command::Drop);
        } else {
            a.status = "Weld is off".into();
        }
    });
    on(r, &["add loop"], |a, _| {
        a.toggle_edge_click(edge_panel::EdgeClick::AddLoop)
    });
    on(r, &["add point"], |a, _| {
        a.toggle_edge_click(edge_panel::EdgeClick::AddPoint)
    });
    on(r, &["spike"], |a, _| a.run(Command::ArmSpike));
    on(r, &["spike reset"], |a, _| a.run(Command::ResetSpike));
    // MN-05.3: with no tool active the key does nothing (the menu entry is
    // greyed then; `tool.reset` still says why).
    on(r, &["reset tool attributes"], |a, _| {
        if a.editor.has_active_task() {
            a.run(Command::ResetToolAttributes);
        }
    });
    on(r, &["ellipsoid"], |a, _| a.run(Command::ArmEllipsoid));
    on(r, &["ellipsoid reset"], |a, _| {
        a.run(Command::ResetEllipsoid)
    });
    on(r, &["ellipsoid unit"], |a, _| {
        a.run(Command::InsertEllipsoidUnit(UnitDestination::ExistingMesh))
    });
    on(r, &["ellipsoid new unit"], |a, _| {
        a.run(Command::InsertEllipsoidUnit(UnitDestination::NewMesh))
    });
    // Plane Item (the Text button's Shift-click) and Teapot Item
    // (missing-tools-g2 MT5-01, MT6-05): new mesh items at the origin.
    on(r, &["plane item", "teapot item"], |a, c| {
        let shape = if c.lowered() == "plane item" {
            UnitShape::Plane
        } else {
            UnitShape::Teapot
        };
        a.run(Command::InsertShapeUnit(shape, UnitDestination::NewMesh))
    });
    // The Create tab's Shift (New Unit) and Ctrl (Unit) variants (layouts
    // stage 3 batch 7): the commands the old variant route ran.
    on(
        r,
        &[
            "cube unit",
            "cube new unit",
            "sphere unit",
            "sphere new unit",
            "cylinder unit",
            "cylinder new unit",
            "cone unit",
            "cone new unit",
            "torus unit",
            "torus new unit",
            "capsule unit",
            "capsule new unit",
        ],
        |a, c| {
            let name = c.lowered();
            let destination = if name.ends_with(" new unit") {
                UnitDestination::NewMesh
            } else {
                UnitDestination::ExistingMesh
            };
            let command = match name.split(' ').next().unwrap_or_default() {
                "cube" => Command::InsertUnit {
                    tool: Tool::Box,
                    destination,
                },
                "sphere" => Command::InsertUnit {
                    tool: Tool::Sphere,
                    destination,
                },
                "cylinder" => Command::InsertUnit {
                    tool: Tool::Cylinder,
                    destination,
                },
                "cone" => Command::InsertShapeUnit(UnitShape::Cone, destination),
                "torus" => Command::InsertShapeUnit(UnitShape::Torus, destination),
                _ => Command::InsertShapeUnit(UnitShape::Capsule, destination),
            };
            a.run(command);
        },
    );
    on(r, &["cone"], |a, _| a.run(Command::ArmCone));
    on(r, &["cut selection"], |a, _| a.run(Command::CutSelection));
    on(r, &["copy selection"], |a, _| a.copy_selection());
    on(r, &["paste selection"], |a, _| a.paste_selection());
    on(r, &["paste tool"], |a, _| a.toggle_paste_tool());
    on(r, &["item merge", "merge meshes"], |a, _| {
        // The Create tab's Merge said this before running (batch 7).
        if a.editor.scene.selected_items().len() < 2 {
            a.status =
                "Merge: select two or more meshes (Ctrl/Shift-click in the Scenes list)".into();
        } else {
            a.run(Command::MergeItems);
        }
    });
    on(r, &["item unmerge", "unmerge mesh"], |a, _| {
        a.run(Command::UnmergeItem)
    });
    on(r, &["curve"], |a, _| {
        a.activate_draw(draw_tools::DrawKind::Curve)
    });
    on(r, &["b-spline", "bspline"], |a, _| {
        a.activate_draw(draw_tools::DrawKind::BSpline)
    });
    on(r, &["bezier curve", "bezier"], |a, _| {
        a.activate_draw(draw_tools::DrawKind::Bezier)
    });
    on(r, &["sketch"], |a, _| {
        a.activate_draw(draw_tools::DrawKind::Sketch)
    });
    on(r, &["draw patch"], |a, _| {
        a.activate_draw(draw_tools::DrawKind::Patch)
    });
    on(
        r,
        &[
            "quick align",
            "ground align",
            "ruler",
            "protractor",
            "dimensions",
        ],
        |a, c| {
            let lowered = c.lowered();
            a.utility_by_name(&lowered);
        },
    );
    on(
        r,
        &[
            "torus reset",
            "capsule reset",
            "arc reset",
            "tube reset",
            "text reset",
            "teapot reset",
        ],
        |a, _| a.run(Command::ResetShape),
    );
    on(
        r,
        &[
            "apply",
            "bridge apply",
            "cone apply",
            "ellipsoid apply",
            "spike apply",
            "torus apply",
            "capsule apply",
            "arc apply",
            "tube apply",
            "smooth shift apply",
            "thicken apply",
            "apply last cube",
        ],
        |a, _| a.run(Command::Apply),
    );
    on(r, &["smooth shift reset", "thicken reset"], |a, _| {
        a.run(Command::ResetSmoothShift)
    });
    on(r, &["smooth shift clone", "thicken clone"], |a, _| {
        a.run(Command::CloneSmoothShift)
    });
    on(
        r,
        &["smooth shift drop", "thicken drop", "drop", "drop cube"],
        |a, _| a.run(Command::Drop),
    );
    on(
        r,
        &["smooth shift cancel", "thicken cancel", "cancel"],
        |a, _| a.run(Command::Cancel),
    );
    on(r, &["continue", "repeat"], |a, _| a.run(Command::Continue));
    on(r, &["undo"], |a, _| a.run(Command::Undo));
    on(r, &["redo"], |a, _| a.run(Command::Redo));
    on(r, &["preferences"], |a, _| a.open_preferences());
    on(r, &["reset preferences"], |a, _| {
        a.preferences_ui.confirm_reset = true;
    });
    on(r, &["top view"], |a, _| {
        a.camera.view = if a.camera.view == View::Top {
            View::Bottom
        } else {
            View::Top
        };
    });
    on(r, &["front view"], |a, _| {
        a.camera.view = if a.camera.view == View::Front {
            View::Back
        } else {
            View::Front
        };
    });
    on(r, &["right view"], |a, _| {
        a.camera.view = if a.camera.view == View::Right {
            View::Left
        } else {
            View::Right
        };
    });
    on(r, &["perspective view"], |a, _| {
        a.camera.view = View::Perspective;
    });
    on(r, &["shaded view"], |a, _| {
        a.display.set_style(view_display::Style::Shaded, false)
    });
    on(r, &["wireframe view"], |a, _| {
        a.display.set_style(view_display::Style::Wireframe, false)
    });
    on(r, &["wireframe overlay none"], |a, _| {
        a.display.settings.active.overlay = view_display::Overlay::None;
    });
    on(r, &["wireframe overlay uniform"], |a, _| {
        a.display.settings.active.overlay = view_display::Overlay::Uniform;
    });
    on(r, &["wireframe overlay colored"], |a, _| {
        a.display.settings.active.overlay = view_display::Overlay::Colored;
    });
    on(r, &["toggle x-ray"], |a, _| a.toggle_xray());
    on(r, &["vertices"], |a, _| a.run(Command::Mode(Mode::Vertex)));
    on(r, &["edges"], |a, _| a.run(Command::Mode(Mode::Edge)));
    on(r, &["polygons"], |a, _| a.run(Command::Mode(Mode::Polygon)));
    on(r, &["items"], |a, _| a.run(Command::Mode(Mode::Item)));
    on(r, &["materials"], |a, _| a.toggle_tag_mode(false));
    on(r, &["select next mode"], |a, _| a.select_next_mode());
    on(r, &["more", "select more"], |a, _| {
        a.run(Command::Selection(SelectionOperation::More))
    });
    on(r, &["less", "select less"], |a, _| {
        a.run(Command::Selection(SelectionOperation::Less))
    });
    on(r, &["between", "select between"], |a, _| {
        a.run(Command::Selection(SelectionOperation::Between))
    });
    on(r, &["select symmetrical"], |a, _| {
        a.run(Command::SelectSymmetrical)
    });
    on(r, &["symmetrize positive"], |a, _| {
        a.run(Command::Symmetrize(SymmetrizeSide::Positive))
    });
    on(r, &["symmetrize negative"], |a, _| {
        a.run(Command::Symmetrize(SymmetrizeSide::Negative))
    });
    on(r, &["symmetrize polygons positive"], |a, _| {
        a.run(Command::SymmetrizePolygons {
            side: SymmetrizeSide::Positive,
            merge: true,
        })
    });
    on(r, &["symmetrize polygons negative"], |a, _| {
        a.run(Command::SymmetrizePolygons {
            side: SymmetrizeSide::Negative,
            merge: true,
        })
    });
    on(r, &["toggle symmetry"], |a, _| {
        a.run(Command::ToggleSymmetry)
    });
    on(r, &["selection fill"], |a, _| {
        a.status = "Selection Fill: Shift+Cmd click a polygon inside a selected perimeter".into();
    });
    on(r, &["next loop"], |a, _| {
        a.run(Command::Selection(SelectionOperation::NextLoop))
    });
    on(r, &["previous loop"], |a, _| {
        a.run(Command::Selection(SelectionOperation::PreviousLoop))
    });
    on(r, &["select through"], |a, _| {
        a.status =
            "Select Through is unavailable. Middle-drag the lasso to include hidden components."
                .into();
    });
    on(r, &["close loop"], |a, _| {
        a.run(Command::Selection(SelectionOperation::CloseLoop))
    });
    on(r, &["grow"], |a, _| {
        a.run(Command::Selection(SelectionOperation::Grow))
    });
    on(r, &["shrink"], |a, _| {
        a.run(Command::Selection(SelectionOperation::Shrink))
    });
    on(r, &["loop"], |a, _| {
        a.run(Command::Selection(SelectionOperation::Loop))
    });
    on(r, &["ring"], |a, _| {
        a.run(Command::Selection(SelectionOperation::Ring))
    });
    on(r, &["select all"], |a, _| {
        a.run(Command::Selection(SelectionOperation::All))
    });
    on(r, &["drop selection"], |a, _| {
        a.run(Command::ClearSelection)
    });
    on(r, &["invert"], |a, _| {
        a.run(Command::Selection(SelectionOperation::Invert))
    });
    on(r, &["connected"], |a, _| {
        a.run(Command::Selection(SelectionOperation::Connected))
    });
    on(r, &["invert connected"], |a, _| {
        a.run(Command::Selection(SelectionOperation::InvertConnected))
    });
    // select.convert's phrases (UIC-165), one per target mode.
    on(
        r,
        &[
            "convert to vertices",
            "convert to edges",
            "convert to polygons",
        ],
        |a, c| {
            let mode = match c.lowered().as_str() {
                "convert to vertices" => Mode::Vertex,
                "convert to edges" => Mode::Edge,
                _ => Mode::Polygon,
            };
            a.run(Command::Selection(SelectionOperation::Convert(mode)));
        },
    );
    on(r, &["boundary"], |a, _| {
        a.run(Command::Selection(SelectionOperation::Boundary))
    });
    on(
        r,
        &["delete item", "delete item forward", "delete mesh"],
        |a, c| {
            // With the Shading tab up, Delete removes the selected layers.
            let layers = !c.lowered().eq("delete mesh") && a.shading_tab();
            if !(layers && a.shading_delete_selected()) {
                a.request_delete();
            }
        },
    );
    on(r, &["delete"], |a, _| a.run(Command::DeleteComponents));
    on(r, &["flip"], |a, _| a.run(Command::FlipPolygons));
    // Tool presets 6b: with an axis the row runs at once, without one its
    // dialog opens. The registry checks the axis is X, Y or Z.
    fn flip_it(a: &mut Operandi, id: &str, rest: &str) {
        match rest.trim().to_ascii_lowercase().as_str() {
            "" => a.open_flip_it(id),
            axis => {
                let axis = ["x", "y", "z"].iter().position(|x| *x == axis).unwrap_or(0);
                a.run_flip_it(id, axis);
            }
        }
    }
    on_words(r, &["Flip It"], |a, c| flip_it(a, "FlipIt", c.rest));
    on_words(r, &["Flip It Origin"], |a, c| {
        flip_it(a, "FlipItOrigin", c.rest)
    });
    on(r, &["toggle subdivision"], |a, _| {
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::Subdivision,
        ))
    });
    on(r, &["toggle catmull-clark"], |a, _| {
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::CatmullClark,
        ))
    });
    on(r, &["freeze"], |a, _| {
        a.run(Command::Freeze(a.subdivide_ui.freeze_options.core()))
    });
    on(r, &["sds subdivide"], |a, _| {
        a.subdivide_with(operandi_core::SubdivideMethod::CatmullClark, 1)
    });
    on(r, &["sds subdivide 2x"], |a, _| {
        a.subdivide_with(operandi_core::SubdivideMethod::CatmullClark, 2)
    });
    on(r, &["edit slide"], |a, _| a.slide_from_tab());
    on_words(r, &["operandi.mode.is"], |a, c| {
        a.status = if mode_is(a, c.rest) {
            format!("The selection mode is one of: {}", c.rest.trim())
        } else {
            format!("The selection mode isn't any of: {}", c.rest.trim())
        };
    });
    on(r, &["faceted subdivide"], |a, _| {
        a.subdivide_with(operandi_core::SubdivideMethod::Faceted, 1)
    });
    on(r, &["smooth subdivide"], |a, _| {
        a.subdivide_with(operandi_core::SubdivideMethod::Smooth, 1)
    });
    on(r, &["subdivide options"], |a, _| a.open_subdivide_options());
    // DX-38: one instance of each selected mesh item.
    on(r, &["instance"], |a, _| a.run(Command::InstanceItems));
    on(r, &["merge instance"], |a, _| a.run(Command::MergeInstance));
    on(r, &["freeze instancer"], |a, _| {
        a.run(Command::FreezeReplicator)
    });
    on(r, &["instancers"], |a, _| {
        use crate::view_display::ReplicatorDisplay as R;
        let d = &mut a.display.settings.drawing;
        d.replicators = match d.replicators {
            R::None => R::Selected,
            R::Selected => R::All,
            R::All => R::None,
        };
        a.status = format!("Instancers: {}", d.replicators.label());
    });
    crate::vertex_normal_ui::register(r);
    crate::anim_ui::register(r);
    crate::channel_links_ui::register(r);
    crate::schematic_ui::register(r);
    crate::pose_tool_ui::register(r);
    crate::skeleton_tool_ui::register(r);
    crate::bind_dialog_ui::register(r);
    crate::python_ui::register(r);
    crate::pie_forms::register(r);
    crate::form_icons::register(r);
    on_words(r, &["view.topologyMode"], |a, c| {
        a.set_topology_mode(c.rest)
    });
    on(r, &["unsubdivide"], |a, _| a.open_unsubdivide_options());
    on(r, &["center x"], |a, _| a.center([true, false, false]));
    on(r, &["center y"], |a, _| a.center([false, true, false]));
    on(r, &["center z"], |a, _| a.center([false, false, true]));
    on(r, &["center all"], |a, _| a.center([true, true, true]));
    on(r, &["duplicate hierarchy"], |a, _| {
        a.run(Command::DuplicateHierarchy)
    });
    on(r, &["duplicate item"], |a, _| a.run(Command::DuplicateItem));
    on(r, &["new mesh"], |a, _| a.run(Command::NewMesh));
    on(r, &["set part"], |a, _| a.open_set_part());
    on(r, &["assign selection set"], |a, _| {
        a.open_selection_set(selection_set_dialog::SetAction::Assign)
    });
    on(r, &["use selection set"], |a, _| {
        a.open_selection_set(selection_set_dialog::SetAction::Use)
    });
    on(r, &["delete selection set"], |a, _| {
        a.open_selection_set(selection_set_dialog::SetAction::Delete)
    });
    on(r, &["create selection set group"], |a, _| {
        a.open_selection_set(selection_set_dialog::SetAction::CreateGroup)
    });
    on(r, &["set position"], |a, _| a.open_set_position());
    on(r, &["absolute scaling"], |a, _| a.toggle_absolute_scaling());
    on(r, &["snaps and precision"], |a, _| a.toggle_snaps_panel());
    on(r, &["preset browser"], |a, _| a.toggle_preset_browser());
    on(r, &["preset browser popover"], |a, _| {
        a.preset_browser.popover_toggle = true;
    });
    on(r, &["advanced handles"], |a, _| {
        a.set_handle_style(Some(handle_preferences::HandleDetail::Advanced))
    });
    on(r, &["set material"], |a, _| a.open_set_material());
    on(r, &["unify"], |a, _| a.open_unify());
    on(r, &["scale offset"], |a, _| a.open_scale_offset());
    on(r, &["flatten ngons"], |a, _| a.flatten_ngons());
    on(r, &["split"], |a, _| a.polygon_split());
    on(r, &["quadruple"], |a, _| a.polygon_quadruple());
    on(r, &["detriangulate"], |a, _| a.open_detriangulate());
    on(r, &["relax mesh"], |a, _| a.open_relax_mesh());
    on(r, &["triple fan"], |a, _| a.run(Command::TripleFan));
    on(r, &["tack tool"], |a, _| a.utility_by_name("tack tool"));
    // TC-53: the selected curves (polygons) join the named set; other
    // sets they're in stay.
    on(r, &["retopo edge lock"], |a, _| {
        a.retopo_set("RetopoEdgeLock")
    });
    on(r, &["retopo edge flow"], |a, _| {
        a.retopo_set("RetopoEdgeFlow")
    });
    on(
        r,
        &["mesh paint", "mesh paint instance", "mesh paint instancer"],
        |a, c| a.utility_by_name(&c.lowered()),
    );
    on(r, &["guides"], |a, _| a.utility_by_name("guides"));
    on(r, &["retopo sketch"], |a, _| {
        a.utility_by_name("retopo sketch")
    });
    on(r, &["collapse selection"], |a, _| {
        a.run(Command::CollapseSelection)
    });
    // Select tab tools (select_tools_ui.rs, SL-xx).
    {
        use select_tools_ui::SelectDialogKind as K;
        on_words(r, &["select.edgeSharp"], |a, c| {
            a.select_tool_line(K::Sharp, c.rest)
        });
        on_words(r, &["select.edgeCoplanar"], |a, c| {
            a.select_tool_line(K::CoplanarEdges, c.rest)
        });
        on_words(r, &["select.vertexColinear"], |a, c| {
            a.select_tool_line(K::Colinear, c.rest)
        });
        on_words(r, &["select.edgeDiagonal"], |a, c| {
            a.select_tool_line(K::Diagonal, c.rest)
        });
        on(r, &["coplanar polygons tool"], |a, _| {
            a.select_tool_line(K::CoplanarPolygons, "")
        });
        on(r, &["conditional loop tool"], |a, _| {
            a.select_tool_line(K::ConditionalLoop, "")
        });
    }
    on(r, &["select.shortestPath"], |a, _| a.shortest_path());
    on(r, &["select.conditionalLoop"], |a, _| a.conditional_loop());
    on(r, &["select.lazyState"], |a, _| {
        a.toggle_selection_preference(true)
    });
    on(r, &["shortest path mode"], |a, _| {
        a.toggle_selection_preference(false)
    });
    on_words(r, &["sculpt.relaxMesh"], |a, c| a.relax_mesh_line(c.rest));
    on(r, &["remesh"], |a, _| {
        a.open_remesh(remesh_dialog::RemeshKind::Remesh, None)
    });
    on(r, &["automatic retopology"], |a, _| {
        a.open_remesh(remesh_dialog::RemeshKind::Retopo, None)
    });
    on(r, &["uv unwrap"], |a, _| a.open_uv_unwrap());
    on(r, &["pack uvs"], |a, _| a.open_pack_uvs());
    on(r, &["boolean"], |a, _| a.open_boolean());
    on(r, &["solid drill"], |a, _| {
        a.open_drill(operandi_core::DrillKind::Solid)
    });
    on(r, &["axis drill"], |a, _| {
        a.open_drill(operandi_core::DrillKind::Axis)
    });
    on(r, &["spin quad"], |a, _| a.polygon_spin());
    on(r, &["polygon align"], |a, _| a.polygon_align());
    on(r, &["triple"], |a, _| a.polygon_triple());
    on(r, &["remove"], |a, _| a.remove_polygons());
    on(r, &["polygon merge"], |a, _| a.polygon_merge());
    on(r, &["polygon collapse"], |a, _| a.polygon_collapse());
    on(r, &["make"], |a, _| a.polygon_make());
    on(r, &["frame all"], |a, _| a.frame(false));
    on(r, &["parent to group locator"], |a, _| {
        a.run(Command::ParentToGroupLocator)
    });
    on(r, &["snapping toggle"], |a, _| a.snapping_key());
    on(r, &["constrain to background"], |a, _| {
        a.toggle_background_constraint()
    });
    on(r, &["snapping options"], |a, _| a.open_snapping_options());
    on(r, &["mesh constraints"], |a, _| {
        a.open_mesh_constraint_popover()
    });
    on(r, &["element snap"], |a, _| {
        a.activate_snap_preset(SnapPreset::DragSnap)
    });
    on(r, &["render"], |a, _| a.render_key());
    on(r, &["open render window"], |a, _| a.open_render_window());
    on(r, &["preview"], |a, _| a.open_preview());
    on(r, &["frame selection"], |a, _| a.frame(true));
    on(r, &["zoom in"], |a, _| a.zoom_step(true));
    on(r, &["zoom out"], |a, _| a.zoom_step(false));
    on(r, &["uv view"], |a, _| a.toggle_uv_view());
    on(r, &["open uv editor"], |a, _| a.toggle_uv_editor());
    on(r, &["re-centre", "recentre"], |a, _| a.recentre());
    on(r, &["turn"], |a, _| a.toggle_turn());
    on(r, &["turn upright"], |a, _| a.turn_upright());
    on(r, &["align selected"], |a, _| a.align_selected(false));
    on(r, &["fit and align selected"], |a, _| {
        a.align_selected(true)
    });
    on(
        r,
        &["torus", "capsule", "arc", "tube", "text", "teapot"],
        |a, c| {
            a.arm_shape(match c.lowered().as_str() {
                "teapot" => ShapeKind::Teapot,
                "torus" => ShapeKind::Torus,
                "capsule" => ShapeKind::Capsule,
                "arc" => ShapeKind::Arc,
                "text" => ShapeKind::Text,
                _ => ShapeKind::Tube,
            })
        },
    );
    on(r, &["viewport properties"], |a, _| {
        // VP-10..19: from a key, near the pointer; otherwise (a script,
        // the menu) under the view's gear.
        let pointer = a.corner.target_pointer.take();
        let view = a.corner.target.take().unwrap_or(a.active_view);
        a.toggle_view_properties(view, pointer);
    });
    on(
        r,
        &["maximise viewport", "maximise viewport (alternate)"],
        |a, _| {
            // MN-08: pressed over a docked panel, it maximises that panel.
            if let Some(pane) = a.panel_target.take() {
                a.corner.target = None;
                a.toggle_panel_maximized(pane);
                return;
            }
            let view = a.corner.target.take().unwrap_or(a.active_view);
            a.toggle_maximized(view);
        },
    );
    // The registry checks the axis is one of its choices.
    on_words(r, &["Symmetry"], |a, c| {
        let axis = match c.rest.to_ascii_lowercase().as_str() {
            "x" => 0,
            "y" => 1,
            "z" => 2,
            _ => return a.run(Command::Symmetry(Symmetry::OFF)),
        };
        a.run(Command::Symmetry(Symmetry {
            axis: Some(axis),
            ..a.editor.symmetry
        }));
    });
    on_words(r, &["Falloff"], |a, c| {
        let falloff = match c.rest.to_ascii_lowercase().as_str() {
            "linear" => Some(a.editor.linear_falloff_preset.into()),
            "radial" => Some(a.editor.radial_falloff_preset.into()),
            "cylinder" => Some(operandi_core::CylinderFalloff::default().into()),
            // Falloffs batches 2-6, for the Falloffs pie and scripts.
            "airbrush" => Some(Falloff::Airbrush(Default::default())),
            "screen" => Some(Falloff::Screen(Default::default())),
            "element" => Some(Falloff::Element(Default::default())),
            "vertex map" | "vertexmap" => Some(Falloff::VertexMap(Default::default())),
            "noise" => Some(Falloff::Noise(Default::default())),
            "curvature" => Some(Falloff::Curvature(Default::default())),
            "selection" => Some(Falloff::Selection(Default::default())),
            "soft selection" | "softselection" => Some(Falloff::SoftSelection(Default::default())),
            "lasso" => Some(Falloff::Lasso(Default::default())),
            "path" => Some(Falloff::Path(Default::default())),
            "image" => Some(Falloff::Image(Default::default())),
            "coplanar" => Some(Falloff::Coplanar(Default::default())),
            _ => None,
        };
        a.run(Command::SpatialFalloff(falloff));
        // As the Falloff menu: drawn kinds wait for a press, Path for clicks.
        a.falloff_draw_armed = falloff.is_some_and(|f| {
            f.has_handles() || matches!(f, Falloff::Screen(_) | Falloff::Lasso(_))
        });
        a.falloff_path_editing = matches!(falloff, Some(Falloff::Path(_)));
    });
    on(
        r,
        &[
            "drag snap",
            "drag snap rigid",
            "planar rotate snap",
            "planar rotate snap axis",
            "scale absolute",
            "size absolute",
        ],
        |a, c| {
            if let Some(preset) = SnapPreset::named(&c.lowered()) {
                a.activate_snap_preset(preset);
            }
        },
    );
}

/// Phrases that come from the app's own tables (tool names, deform
/// presets, Vertex and Edge panel rows, Transparency buttons, element
/// actions): one handler each, which looks the line up in its table.
fn tables(r: &mut Commands) {
    // Command History (command_history.rs, HIS-11/22/50, CMD-40).
    on_words(r, &["cmds.batch"], |a, c| a.command_batch(c.rest));
    on_words(r, &["cmds.undoTo"], |a, c| a.command_undo_to(c.rest));
    on_words(
        r,
        &["history.clear", "history.max", "history.resultsMax"],
        |a, c| a.command_history_log(c.entry.name, c.rest),
    );
    // Layouts stage 3 (toolbox_form.rs): form Revert and the UV island
    // buttons as lines.
    on_words(r, &["operandi.form.revert"], |a, c| {
        let id = named_word(c.rest, "form", 0).unwrap_or_default();
        if id.is_empty() {
            a.status = "Usage: operandi.form.revert <form id>".into();
        } else {
            a.revert_form(&id);
        }
    });
    on_words(r, &["operandi.notBuilt"], |a, c| {
        a.status = format!("{} · not in Operandi yet", not_built_name(c.rest));
    });
    // Cut Map's Unwrap Tool and Unwrap & Relax (UVS-1 §1.6): the tool with
    // the seam map in use as its seams.
    on(r, &["export uvs to svg..."], |a, _| a.ask_uv_layout(false));
    on(r, &["export uvs to eps..."], |a, _| a.ask_uv_layout(true));
    on_words(r, &["operandi.uv.brush"], |a, c| {
        let mode = named_word(c.rest, "mode", 0).unwrap_or_default();
        a.uv_brush_command(&mode);
    });
    on_words(r, &["operandi.uv.unwrapSeams"], |a, c| {
        let pelt = named_word(c.rest, "tool", 0).is_some_and(|w| w.eq_ignore_ascii_case("pelt"));
        let tool = if pelt { Tool::UvPelt } else { Tool::UvUnwrap };
        a.activate(tool);
        let name = a.editor.uv_seam_name();
        if let Some(mut p) = a
            .editor
            .live
            .as_ref()
            .filter(|l| l.tool == tool)
            .map(|l| l.params.clone())
        {
            let seams = operandi_core::uv_tools::SeamSource::SeamMap(name);
            if pelt {
                p.uv_pelt.unwrap.seams = seams;
            } else {
                p.uv_unwrap.seams = seams;
            }
            a.run(Command::Preview(p));
        }
    });
    on_words(r, &["operandi.uv.dialog"], |a, c| {
        let kind = named_word(c.rest, "kind", 0).unwrap_or_default();
        a.open_uv_op_dialog(&kind);
    });
    on_words(r, &["operandi.uv.preset"], |a, c| {
        let id = named_word(c.rest, "id", 0).unwrap_or_default();
        let click = named_word(c.rest, "click", 1).unwrap_or_else(|| "plain".into());
        match uv_edit_ui::IslandClick::named(&click) {
            Some(click) if !id.is_empty() => a.island_run(&id, click),
            _ => a.status = "Usage: operandi.uv.preset <id> [plain|shift|alt|options]".into(),
        }
    });
    // Layouts (layouts.rs, LAY-30 / UIC-198).
    on_words(r, &["layout.restore"], |a, c| a.restore_layout_line(c.rest));
    // Jason, 2026-11-19: a layout's entry (pie, Layout and Window menus) is
    // lit only while the screen matches it, so "in the UV layout" differs
    // from "using UV tools in Model".
    r.answer("layout.restore", |a, c| {
        CommandState::lit(a.layout_matches_screen(c.rest))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    // Layouts stage 2 (layouts_edit.rs): one handler reads the command.
    on_words(
        r,
        &[
            "operandi.layout.create",
            "operandi.layout.save",
            "operandi.layout.saveAs",
            "operandi.layout.rename",
            "operandi.layout.delete",
            "operandi.layout.revert",
            "operandi.layout.setKey",
            "operandi.layout.clearKeys",
            "operandi.layout.clear",
            "operandi.layout.star",
            "operandi.layout.starredOnly",
            "operandi.bar.collapse",
            "operandi.layout.move",
            "operandi.layout.switcher",
            "layout.lockThumbs",
            "layout.lockDividers",
            "operandi.layout.buildMode",
            "operandi.viewport.saveAs",
            "operandi.viewport.save",
            "operandi.viewport.use",
            "operandi.viewport.revert",
            "operandi.viewport.delete",
            "operandi.panel.kind",
        ],
        |a, c| a.layout_command(&c.entry.name.to_ascii_lowercase(), c.rest),
    );
    // UIC-18: the switcher while Ctrl+Shift are held (its key binding).
    on(r, &["Layout Switcher"], |a, _| a.open_switcher(true));
    // Tool tab cycling (tool_tab_cycle.rs).
    on(r, &["Next Tool Tab"], |a, _| {
        let ctx = a.frame_ctx.clone();
        if !a.step_tool_tab(ctx.as_ref(), 1) {
            a.status = "No other tool tab to open".into();
        }
    });
    on(r, &["Previous Tool Tab"], |a, _| {
        let ctx = a.frame_ctx.clone();
        if !a.step_tool_tab(ctx.as_ref(), -1) {
            a.status = "No other tool tab to open".into();
        }
    });
    // Palettes (layouts stage 6, palettes.rs).
    on(
        r,
        &["Toggle Palettes", "operandi.layout.togglePalettes"],
        |a, _| a.toggle_palettes(),
    );
    // Window ▸ Node View and its key (node_view_open.rs).
    on(r, &["Node View", "operandi.window.nodeView"], |a, _| {
        a.toggle_node_view()
    });
    on(r, &["UI Build Mode"], |a, _| {
        a.set_build_mode(!a.dots.build_mode)
    });
    // The Help menu (help_menu.rs).
    on_words(r, &["operandi.help.reportBug"], |a, c| {
        a.report_bug_command(c.rest)
    });
    on(r, &["operandi.help.documentation"], |a, _| {
        a.open_documentation()
    });
    on(r, &["operandi.help.whatsThis"], |a, _| a.toggle_help_mode());
    on(r, &["operandi.update.check"], |a, _| a.check_for_updates());
    // Kits (kit_commands.rs).
    on_words(
        r,
        &[
            "operandi.kit.disable",
            "operandi.kit.enable",
            "operandi.kit.install",
            "operandi.kit.list",
            "operandi.kit.openFolder",
            "operandi.kit.reload",
        ],
        |a, c| a.kit_command(c.entry.name, c.rest),
    );
    // Online libraries (online_library_ui.rs).
    on_words(
        r,
        &[
            "operandi.online.clearCache",
            "operandi.online.download",
            "operandi.online.refresh",
        ],
        |a, c| a.online_command(c.entry.name, c.rest),
    );
    fn tool_set(rest: &str) -> Option<toolbox::ToolSet> {
        let tokens = command::tokens(rest).ok()?;
        let args =
            operandi_core::command_args::bind("operandi.toolbox.set", &tokens, &["set"]).ok()?;
        toolbox::ToolSet::named(args[0]?)
    }
    on_words(r, &["operandi.toolbox.set"], |a, c| {
        if let Some(set) = tool_set(c.rest) {
            a.select_tool_set(set);
        } else {
            a.status = "Unknown or unavailable tool set".into();
        }
    });
    r.answer("operandi.toolbox.set", |a, c| match tool_set(c.rest) {
        Some(set) => CommandState::lit(set == a.layouts.tool_set),
        None => CommandState::unless(Some("Unknown or unavailable tool set")),
    })
    .unwrap_or_else(|e| panic!("{e}"));
    on_words(r, &["operandi.toolbox.tab"], |a, c| {
        a.toolbox_tab_line(c.rest)
    });
    on_words(r, &["curves.setColor"], |a, c| {
        a.curves_set_color_line(c.rest)
    });
    on_words(r, &["curve.convert"], |a, c| a.convert_curves_line(c.rest));
    on_words(r, &["ppi.curveOffset"], |a, c| a.curve_offset_line(c.rest));
    on_words(r, &["bezier.tangent"], |a, c| a.bezier_tangent_line(c.rest));
    on_words(r, &["operandi.topology.tool"], |a, c| {
        a.topology_tool_line(c.rest)
    });
    on_words(r, &["curve.boolean"], |a, c| a.curve_boolean_line(c.rest));
    r.answer("curve.convert", |a, c| {
        CommandState::unless(curve_hands_on::convert_state(a, c.rest).as_deref())
    })
    .unwrap_or_else(|e| panic!("{e}"));
    on(r, &["operandi.dicer"], |a, _| {
        a.open_cut_dialog(cut_dialogs::CutKind::Dicer)
    });
    for tool in Tool::ALL {
        on(r, tool_info::info(tool).names, |a, c| {
            if let Some(tool) = tool_info::by_name(&c.lowered()) {
                a.activate(tool);
            }
        });
    }
    // Tool preset rows with layers (tool_preset.rs): their older phrases.
    // Their labels and ids reach them through the registry and `tool.set`.
    for row in operandi_core::tool_preset::factory().iter().filter(|r| {
        (!r.layers.is_empty() || r.actor.single_ring || r.actor.topology_pen) && r.tool().is_some()
    }) {
        let names: Vec<&'static str> = row.names.iter().map(String::as_str).collect();
        on(r, &names, |a, c| a.activate_preset(&c.lowered()));
    }
    // The path tool buttons (CX-01) and the Duplicate copier buttons
    // (DX-20/30): their labels start their rows.
    for row in operandi_core::tool_preset::factory()
        .iter()
        .filter(|r| r.actor.path.is_some() || r.actor.copier.is_some())
    {
        on(r, &[row.label.as_str()], |a, c| {
            a.activate_preset(c.entry.name)
        });
    }
    for row in vertex_panel::VertexRow::ALL
        .iter()
        .filter(|r| r.owns_action())
    {
        on(r, &[row.action()], |a, c| {
            let line = c.lowered();
            if let Some(&row) = vertex_panel::VertexRow::ALL
                .iter()
                .find(|r| r.owns_action() && r.action().eq_ignore_ascii_case(&line))
            {
                a.run_vertex_row(row);
            }
        });
    }
    for command in EdgeCommand::ALL {
        on_words(r, &[edge_panel::edge_action(command)], edge_row);
    }
    for name in viewport_buttons::TRANSPARENCY_ACTIONS {
        on(r, &[name], |a, c| {
            let Some((mode, inactive)) = viewport_buttons::transparency_for_action(&c.lowered())
            else {
                return;
            };
            match viewport_buttons::transparency_unavailable(&a.display.settings, inactive) {
                Some(why) => a.status = why.into(),
                None => a.set_transparency(mode, inactive),
            }
        });
    }
    for &name in input::NAV_LOCK_HIDE_ACTIONS
        .iter()
        .filter(|n| input::element_action_lowered(&n.to_lowercase()).is_some())
    {
        on(r, &[name], |a, c| {
            if let Some((kind, op)) = input::element_action_lowered(&c.lowered()) {
                a.run(Command::EditElements { kind, op });
            }
        });
    }
}

/// An Edge panel row, by its action phrase; `edge subdivide <count>
/// [percent]` (CMD-B3-01) takes arguments.
fn edge_row(a: &mut Operandi, c: &Call<'_>) {
    let line = c.lowered();
    if let Some(rest) = line.strip_prefix("edge subdivide ") {
        let mut parts = rest.split_whitespace();
        match (
            parts.next().and_then(|c| c.parse::<u32>().ok()),
            parts.next().map(|p| p.parse::<f64>()),
        ) {
            (Some(count), None) => a.run(Command::SubdivideEdges {
                count,
                percent: None,
            }),
            (Some(count), Some(Ok(p))) => a.run(Command::SubdivideEdges {
                count,
                percent: Some(p),
            }),
            _ => a.status = "Use: edge subdivide <count> [percent]".into(),
        }
        return;
    }
    if let Some(&command) = EdgeCommand::ALL
        .iter()
        .find(|&&c| edge_panel::edge_action(c).eq_ignore_ascii_case(&line))
    {
        a.run_edge_command(command);
    }
}
