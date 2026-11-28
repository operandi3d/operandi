//! Translate coalesced native events at the platform boundary. A complete fast
//! drag can arrive within one egui frame; retain its press and release positions
//! instead of relying only on end-of-frame button state.
use eframe::egui::{Modifiers, Pos2, Rect};
#[derive(Clone)]
pub struct SketchStroke {
    pub camera: crate::Camera,
    pub rect: Rect,
    pub plane_center: glam::DVec3,
    pub plane_normal: glam::DVec3,
    pub last_sample: Pos2,
    pub precision: f64,
    pub samples: Vec<glam::DVec3>,
    pub current: glam::DVec3,
    pub started: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NavigationKind {
    Orbit,
    Pan,
    Zoom,
    /// NAV-02: bank about the viewing axis.
    Roll,
    /// NAV-03: look around: the eye stays put and the view turns about it
    /// (Jason, 2026-11-22; was a flick orbit).
    Look,
    /// NAV-05: pan up and down only.
    PanVertical,
    /// NAV-07: drag a rectangle to zoom to.
    BoxZoom,
}
#[derive(Clone)]
pub enum GestureKind {
    Navigate(NavigationKind),
    Handle(usize, operandi_core::Parameters, operandi_core::Tool),
    PolyHaul(Box<crate::poly_haul_drag::HaulDrag>),
    /// The shared drag layer (drag controls): a drag away from the handles
    /// changing one of the live tool's values (haul_view.rs).
    Haul(Box<crate::haul_view::HaulDrag>),
    SketchStroke(SketchStroke),
    Primitive(crate::primitive_input::Gesture),
    /// Dragging a camera or light handle (CL-14).
    ItemHandle(crate::camera_light_handles::Grab),
    /// Dragging falloff handle `handle` from the falloff as it was at the press.
    Falloff(usize, operandi_core::Falloff),
    /// Dragging a handle of the selected primitive MeshOp (meshop_handles.rs).
    MeshOpHandle(Box<crate::viewport::meshop_handles::OpGrab>),
    /// Drawing a numeric Cone (`cone`) or Ellipsoid on construction plane
    /// `plane` from `anchor` (Jason, 2026-11-02).
    SolidDraw {
        cone: bool,
        plane: u8,
        anchor: glam::DVec3,
    },
    /// Giving a flat Cone/Ellipsoid/Torus/Capsule base its height.
    SessionHeight {
        base: glam::DVec3,
        axis: usize,
    },
    /// Dragging a drawn Cone/Ellipsoid/Torus/Capsule/Text's handle (#37).
    SessionHandle {
        id: usize,
        base: crate::session_handles::SessionShape,
    },
    /// Add Loop / Add Point held after the click: sliding re-places the new
    /// loop along edge `a`–`b` (screen ends `ends`, with their depths).
    EdgeInsert {
        tool: crate::edge_panel::EdgeClick,
        a: u32,
        b: u32,
        ends: [(eframe::egui::Pos2, f64); 2],
    },
    Lasso(Vec<Pos2>, bool),
    Select,
    /// A Solid Sketch press: a new node being sized, a node moved or
    /// scaled, or one of its handles (solid_sketch_ui.rs).
    SolidSketch(Box<crate::solid_sketch_ui::SketchDrag>),
    /// Placing or moving Pen draft point `point` (press created or picked it).
    PenDrag {
        point: usize,
    },
    /// Pen walls: a right-drag setting Offset from `start_offset`.
    PenOffset {
        start_offset: f64,
        metres_per_pixel: f64,
    },
    /// Tweak: dragging the element in `pick` from the parameters at the press.
    Tweak {
        pick: crate::viewport::tweak_view::TweakPick,
        base: operandi_core::Parameters,
    },
    /// Tweak: a right-drag resizing Range (TWR-06..11). The sphere shows
    /// at `center` (world); `metres_per_pixel` is the view's scale there at
    /// the press.
    TweakRange {
        center: glam::DVec3,
        metres_per_pixel: f64,
    },
    /// A Bend drag: Angle, or the spine's end (bend_view.rs).
    Bend(crate::viewport::bend_view::BendDrag),
    /// A right-drag changing Steps of the Selection falloff at stack
    /// position `layer`, from `start`; a right-click adds one (FO-46).
    FalloffSteps {
        layer: usize,
        start: u32,
    },
    /// FO-43/52: drawing a Screen or Lasso falloff from `start` (view
    /// pixels), with the pointer's trail.
    ScreenFalloffDraw {
        start: glam::DVec2,
        path: Vec<glam::DVec2>,
    },
    /// FO-44: painting the Airbrush falloff; MN-03.4: adding, lowering
    /// (Shift) or erasing (Ctrl, Cmd on a Mac) its weight.
    Airbrush(crate::viewport::falloff_screen_view::AirbrushStroke),
    /// FO-44: a right-drag changing Airbrush Size from `start` pixels.
    AirbrushSize {
        layer: usize,
        start: f64,
    },
    /// FO-05: drawing a geometric falloff from the press at `origin`.
    FalloffDraw {
        origin: glam::DVec3,
    },
    /// A right-drag changing Radius of the Soft Selection falloff at stack
    /// position `layer`, from `start` (FO-47, FO-Q05).
    FalloffRadius {
        layer: usize,
        start: f64,
        metres_per_pixel: f64,
    },
    /// A drag anywhere for a numeric session (Jason, 2026-11-04;
    /// DRAG-Q01), from its values at the press.
    Numeric(NumericDrag),
    /// A Pose tool drag (IK-50..54).
    Pose(crate::pose_tool_ui::PoseDrag),
    /// A Skeleton tool press placing or moving a joint (SK-10, SK-12).
    Skeleton(crate::skeleton_tool_ui::SkelDrag),
    /// A Create panel utility (Ruler, Protractor, Quick Align) handle.
    Utility(crate::create_utility::UtilityDrag),
    /// A Draw curve tool's point, handle or new point.
    Draw(crate::draw_tools::DrawDrag),
    /// Dragging a point with Weld (its state is `Operandi::weld_drag`).
    WeldDrag,
    /// A Slice tool drag (slice_view.rs).
    Slice(crate::slice_view::SliceDrag),
    /// A sculpt stroke (its state is `Operandi::sculpt.stroke`).
    Sculpt,
    /// Sculpt: a right-drag resizing the brush from Size `start` (SC-22).
    SculptSize {
        start: f64,
    },
    /// Sculpt: a middle-drag changing Offset Amount from `start` (SC-23).
    SculptStrength {
        start: f64,
    },
    /// A paint stroke (painting_brush.rs, PNT-48).
    #[cfg(feature = "painting")]
    Paint,
    /// Paint: a right-drag sizing the tip from Size `start` (PNT-53).
    #[cfg(feature = "painting")]
    PaintSize {
        start: f64,
    },
    /// A Duplicate tool drag (duplicate_view.rs).
    Duplicate(crate::duplicate_view::DupDrag),
    Blocked,
}
/// What a drag anywhere changes in a numeric (panel-driven) session.
#[derive(Clone)]
pub enum NumericDrag {
    /// Spike: up lengthens the spikes (Strength).
    Spike(f64),
    /// Bridge: right adds Segments, left removes them.
    Bridge(operandi_core::BridgeParameters),
    /// Polygon Reduction: left lowers Number, right raises it, up to the
    /// source count.
    Reduction(operandi_core::ReductionParameters, u32),
    /// Torus, Capsule, Arc: drawn on construction plane `plane` from
    /// `anchor`, like the Cone and Ellipsoid.
    Shape { plane: u8, anchor: glam::DVec3 },
    /// Tube: each press adds a control point (or grabs the one under the
    /// pointer); the drag moves point `index` on construction plane `plane`.
    TubePoint { plane: u8, index: usize },
}
#[derive(Clone)]
pub struct Navigation {
    pub kind: GestureKind,
    pub start: Pos2,
    pub last: Pos2,
    pub modifiers: Modifiers,
    pub button: eframe::egui::PointerButton,
}
pub fn navigation(mods: Modifiers) -> Option<NavigationKind> {
    mods.alt.then_some(if mods.shift {
        NavigationKind::Pan
    } else if mods.ctrl || mods.mac_cmd {
        // NAV-06: on macOS both the Control and the Command key.
        NavigationKind::Zoom
    } else {
        NavigationKind::Orbit
    })
}
pub fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_sq().max(1e-8)).clamp(0., 1.);
    p.distance(a + d * t)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FileShortcut {
    Save,
    Close,
    /// Cmd+Shift+W.
    CloseAll,
}
/// Use each key event's modifiers: native down/up sequences may be coalesced
/// into one frame whose final modifier state is already empty.
pub fn file_shortcut(events: &[eframe::egui::Event]) -> Option<FileShortcut> {
    use eframe::egui::Event;
    events.iter().find_map(|event| match event {
        Event::Key {
            key,
            pressed: true,
            repeat: false,
            modifiers,
            ..
        } => fixed_file_action(*key, *modifiers, CMD_IS_CTRL),
        _ => None,
    })
}

fn fixed_file_action(
    key: eframe::egui::Key,
    mods: Modifiers,
    cmd_is_ctrl: bool,
) -> Option<FileShortcut> {
    use eframe::egui::Key;
    // Compare the physical modifiers. egui's command flag also aliases Ctrl off macOS.
    if mods.alt || mods.ctrl != cmd_is_ctrl || mods.mac_cmd == cmd_is_ctrl {
        return None;
    }
    match (key, mods.shift) {
        (Key::S, false) => Some(FileShortcut::Save),
        (Key::W, true) => Some(FileShortcut::CloseAll),
        (Key::W, false) => Some(FileShortcut::Close),
        _ => None,
    }
}

/// UIC-201: the main window's fixed File chords apply in every layout.
pub fn reserved_file_chord(action: &str, binding: &str) -> bool {
    !crate::image_viewer_ui::is_viewer_action(action)
        && chord(binding)
            .is_some_and(|(mods, key)| fixed_file_action(key, mods, CMD_IS_CTRL).is_some())
}

pub const ACTIONS: &[(&str, &str)] = &[
    ("Top View", "Numpad1"),
    ("Front View", "Numpad2"),
    ("Right View", "Numpad3"),
    ("Perspective View", "NumpadDecimal"),
    // BEHAVIOR-VIEWPORT-BUTTONS: Maximise/Restore the view under the
    // pointer; Alt+0 for keyboards without a keypad.
    (crate::viewport_buttons::MAXIMIZE, "Numpad0"),
    (crate::viewport_buttons::MAXIMIZE_ALT, "Alt+0"),
    (crate::viewport_properties::ACTION, "O"),
    ("Shaded View", "Numpad4"),
    ("Wireframe View", "Numpad7"),
    // VP-30..34: the active meshes' Wireframe Overlay in the view under the
    // pointer. Unbound for now: / and Shift+/ are Turn and Turn Upright
    // (VP-Q01).
    ("Wireframe Overlay None", ""),
    ("Wireframe Overlay Uniform", ""),
    ("Wireframe Overlay Colored", ""),
    // XR-2: the view's Transparency Mode, active then inactive meshes, and
    // the Normal/X-Ray toggle. Unbound (XR-Q1).
    (crate::viewport_buttons::TRANSPARENCY_ACTIONS[0], ""),
    (crate::viewport_buttons::TRANSPARENCY_ACTIONS[1], ""),
    (crate::viewport_buttons::TRANSPARENCY_ACTIONS[2], ""),
    (crate::viewport_buttons::TRANSPARENCY_ACTIONS[3], ""),
    (crate::viewport_buttons::TRANSPARENCY_ACTIONS[4], ""),
    (crate::viewport_buttons::TRANSPARENCY_ACTIONS[5], ""),
    (crate::viewport_buttons::TOGGLE_XRAY, ""),
    ("New Mesh", "N"),
    ("Delete Item", "Backspace"),
    ("Delete Item Forward", "Delete"),
    ("Vertices", "1"),
    ("Edges", "2"),
    ("Polygons", "3"),
    // Note 04: polygon tag selection mode (PT-Q2: 4 was free).
    (crate::tag_mode_ui::ACTION, "4"),
    ("Items", "5"),
    ("Drop Cube", "Escape"),
    ("Cube", ""),
    ("Sphere", ""),
    ("Cylinder", ""),
    ("Cone", ""),
    ("Ellipsoid", ""),
    ("Torus", ""),
    ("Capsule", ""),
    ("Arc", ""),
    ("Tube", ""),
    ("Text", ""),
    ("Bridge", ""),
    ("Spike", ""),
    // Sculpt tab brushes: no default keys (SQ-08, SCT-05).
    ("Sculpt Smooth", ""),
    ("Sculpt Push", ""),
    ("Sculpt Inflate", ""),
    ("Sculpt Move", ""),
    ("Sculpt Fold", ""),
    ("Sculpt Clay", ""),
    ("Sculpt Smudge", ""),
    ("Sculpt Carve", ""),
    ("Sculpt Scar", ""),
    ("Sculpt Flatten", ""),
    ("Sculpt Tangent Pinch", ""),
    ("Sculpt Center Pinch", ""),
    ("Sculpt Emboss", ""),
    ("Sculpt Attenuate", ""),
    ("Sculpt Spin", ""),
    ("Vertex Map Paint", ""),
    ("Vertex Map Smooth", ""),
    ("Vertex Map Scale Down", ""),
    ("Vertex Map Erase", ""),
    ("Vertex Map Set Value", ""),
    ("Polygon Pen", ""),
    // KEY-01/02: plain B is Polygon Bevel in Polygon mode (Shift+B stays
    // in the reference's key map but isn't needed here).
    ("Bevel", "B"),
    // Twist Bevel: no default chord (spec gives none); bind it in
    // Preferences > Input.
    ("Twist Bevel", ""),
    ("Chamfer Edit", ""),
    ("Poly Haul", ""),
    ("Inset", ""),
    ("Shift", ""),
    ("Sketch Extrude", "Alt+B"),
    ("Extrude", "Shift+X"),
    // M2 Edge tools. Shift+X is shared with polygon
    // Extrude by context: Edge Extrude in Edge mode, Extrude elsewhere.
    ("Extend", "Z"),
    ("Edge Extrude", "Shift+X"),
    ("Slide", "S"),
    ("Edge Weight", ""),
    // KEY-02: B opens Edge Bevel in Edge mode (measured).
    ("Edge Bevel", "B"),
    ("Edge Chamfer", ""),
    ("Edge Relax", ""),
    // M2 Edge command rows. Guide chords; the Edge-mode actions share
    // Remove/Collapse chords with the Polygon-mode actions by context.
    ("Grow Quads", "Shift+Q"),
    ("Fill Quads", ""),
    ("Edge Split", ""),
    ("Split Normals", ""),
    ("Edge Spin", "V"),
    ("Join", ""),
    ("Join Averaged", ""),
    ("Edge Collapse", "Ctrl+Backspace"),
    ("Edge Remove", "Shift+Backspace"),
    ("Edges To Curves", ""),
    ("Unbevel", ""),
    ("Edge Subdivide", "D"),
    // M3 Vertex tools and rows. Guide chords shared by context: B, Shift+X,
    // S, Ctrl+Backspace and Shift+Backspace act on points in Vertex mode.
    ("Create", ""),
    ("Vertex Bevel", "B"),
    ("Vertex Extrude", "Shift+X"),
    ("Merge Tool", ""),
    ("Weld", ""),
    ("Tear", ""),
    ("Vertex Slide", "S"),
    ("New Vertex", ""),
    ("Show Vertices", ""),
    ("Vertex Split", ""),
    ("Vertex Split Edge", ""),
    ("Vertex Set Position", ""),
    ("Vertex Align", ""),
    ("Vertex Merge", ""),
    ("Vertex Join", ""),
    ("Vertex Join Averaged", ""),
    ("Vertex Collapse", "Ctrl+Backspace"),
    ("Vertex Remove", "Shift+Backspace"),
    ("Vertex Remove Edge", ""),
    ("Reorder Index", ""),
    ("Sync Index Order", ""),
    ("Tool Properties", "K"),
    ("Smooth Shift", ""),
    ("Thicken", ""),
    // KEY-01: Y opens Transform.
    ("Transform", "Y"),
    ("Flex", "Shift+Y"),
    ("Flare", ""),
    ("Bend", "Ctrl+E"),
    ("Move", "W"),
    // TWK-01 / KEY-O-KEY-P06: plain T is Tweak (Element Move).
    ("Element Move", "T"),
    ("Rotate", "E"),
    // TC-30.
    ("Axis Rotate", "Alt+Shift+E"),
    ("Scale", "R"),
    ("Drop", "Q"),
    // CV-03 / UIC-66: Space opens Mini Properties (a fixed key, see
    // mini_properties.rs); its old mode cycle moves here.
    ("Select Next Mode", "Alt+Shift+Space"),
    // macOS standard chords, requested by Jason (undo is not in the
    // default-shortcuts spec). Text fields keep their own Cmd+Z.
    ("Undo", "Cmd+Z"),
    ("Cut Selection", "Cmd+X"),
    ("Copy Selection", "Cmd+C"),
    ("Paste Selection", "Cmd+V"),
    ("Paste Tool", ""),
    // Create panel Merge, Align and Measure groups (create panel v2).
    ("Item Merge", ""),
    ("Item Unmerge", ""),
    ("Quick Align", "Shift+R"),
    ("Ground Align", ""),
    ("Ruler", ""),
    ("Protractor", ""),
    ("Dimensions", ""),
    ("Absolute Scaling", ""),
    ("Snaps and Precision", ""),
    // Preset browser (PB-101, BQ-01): F6 toggles its floating window
    // under the pointer; Window > Preset Browser has no key of its own.
    (crate::preset_browser::WINDOW_ACTION, ""),
    (crate::preset_browser::POPOVER_ACTION, "F6"),
    ("Curve", ""),
    ("B-Spline", ""),
    ("Bezier Curve", ""),
    ("Sketch", ""),
    ("Draw Patch", ""),
    ("Redo", "Shift+Cmd+Z"),
    ("Preferences", "Cmd+,"),
    // MN-05.3 / SC-083: Ctrl+D (Cmd+D on a Mac) while a tool is active.
    ("Reset Tool Attributes", "Cmd+D"),
    ("Flip", "F"),
    // SDV-01/02 (subdivision-2026-11-03).
    ("Toggle Subdivision", "Tab"),
    ("Toggle Catmull-Clark", "Shift+Tab"),
    ("Freeze", ""),
    // Edit tab Subdivide group (SUB-03, corrected 2026-11-08): D is SDS
    // Subdivide outside Edge mode (Edge Subdivide keeps D in Edge mode).
    // Shift+D opens the Subdivide options in Vertex/Edge/Polygon mode and
    // is Instance in Item mode (not built yet: a status message).
    ("SDS Subdivide", "D"),
    ("Faceted Subdivide", ""),
    ("Smooth Subdivide", ""),
    ("Subdivide Options", "Shift+D"),
    ("Instance", "Shift+D"),
    ("UnSubdivide", ""),
    // Edit tab Center row (CEN).
    ("Center X", ""),
    ("Center Y", ""),
    ("Center Z", ""),
    ("Center All", ""),
    ("Frame All", "A"),
    // N11 (Jason, 2026-11-16, provisional): Ctrl+G groups the selected
    // items under a new Group Locator.
    ("Parent to Group Locator", "Ctrl+G"),
    // SN-01/SN-04 (snapping-2026-11-05).
    ("Snapping Toggle", "X"),
    ("Snapping Options", "F11"),
    // SN2-04b: the Element Snap tool (Drag Snap preset), snapping on.
    ("Element Snap", "Ctrl+Shift+T"),
    // SPEC-BACKGROUND-CONSTRAINT: no default key (unbound, like the
    // other tool-setup toggles).
    (crate::background_ui::ACTION, ""),
    (crate::background_ui::POPOVER_ACTION, ""),
    // RP-Q1: final render and the Preview window.
    ("Render", "F9"),
    ("Preview", "F8"),
    // IV-02: the Render window without rendering, from anywhere.
    (crate::image_viewer_ui::OPEN, "Alt+Shift+F9"),
    // Image Viewer keys (IV-10, IV-35, IV-36, IV-Q2): only while the Render
    // window has the focus, so they share keys with main-window actions.
    (crate::image_viewer_ui::STRIP, "T"),
    (crate::image_viewer_ui::PAUSE, "P"),
    (crate::image_viewer_ui::TAB_PROCESSING, "I"),
    (crate::image_viewer_ui::TAB_COMPARE, "C"),
    (crate::image_viewer_ui::TAB_STATISTICS, "S"),
    (crate::image_viewer_ui::FIT, "Cmd+A"),
    (crate::image_viewer_ui::SAVE, "Cmd+S"),
    ("Frame Selection", "Shift+A"),
    // NAV-10..18 (nav-lock-hide-2026-11-07). Re-centre is unbound: G stays
    // Edge Slice (Jason, 2026-11-07).
    ("Zoom In", "Period"),
    ("Zoom Out", "Comma"),
    (RECENTRE, ""),
    ("Turn", "Slash"),
    ("Turn Upright", "Shift+Slash"),
    ("Align Selected", "Cmd+A"),
    ("Fit and Align Selected", "Shift+Cmd+A"),
    // LCK-30..33 geometry lock; HID-10..13 hide. Hide Invert is the real
    // Control key on macOS (Cmd+H hides the app).
    ("Lock Selected", "J"),
    ("Lock Unselected", "Shift+J"),
    ("Lock Invert", "Cmd+J"),
    ("Unlock", "I"),
    ("Hide Selected", "H"),
    ("Hide Unselected", "Shift+H"),
    ("Hide Invert", "Ctrl+H"),
    ("Unhide", "U"),
    ("Close Loop", "Shift+]"),
    ("Grow", "Shift+ArrowUp"),
    ("Shrink", "Shift+ArrowDown"),
    ("Loop", "L"),
    ("Ring", "Alt+L"),
    ("Select All", "Alt+Shift+A"),
    // SC-080: Esc with no tool on drops the selection (drop_keys.rs).
    ("Drop Selection", "Escape"),
    ("Invert", "["),
    ("Connected", "]"),
    ("Invert Connected", ""),
    ("Convert to Vertices", "Alt+1"),
    ("Convert to Edges", "Alt+2"),
    ("Convert to Polygons", "Alt+3"),
    ("Boundary", ""),
    // PART-005: no shipped default; remappable through the Input Map.
    ("Set Part", ""),
    ("Set Position", ""),
    ("Set Material", "M"),
    ("Split", "Ctrl+L"),
    ("Unify", ""),
    ("Flip It", ""),
    ("Flip It Origin", ""),
    ("Quadruple", ""),
    ("Spin Quad", "Alt+V"),
    // ALN-B03: text-only Polygon row, no shipped default; remappable. Named
    // for its Polygon scope, like Polygon Merge.
    ("Polygon Align", ""),
    // TRI-001: remappable default.
    ("Triple", "Shift+T"),
    // REM-001: contextual Polygon-mode Remove; not item deletion. Remappable.
    ("Remove", "Shift+Backspace"),
    // MER-001: no shipped default; remappable through the Input Map. The
    // action is named for its Polygon scope, not a bare "Merge" that could be
    // confused with vertex/edge merging.
    ("Polygon Merge", ""),
    // RED-001: no shipped default activation binding; remappable.
    ("Polygon Reduction", ""),
    // COL-B02: Polygon-context Collapse, default literal Ctrl+Backspace (no
    // Control/Command substitution). Named for its Polygon scope; no Edge or
    // Vertex collapse workflow exists. Remappable; not item deletion.
    ("Polygon Collapse", "Ctrl+Backspace"),
    // SO-U03: no direct mapping in the checked shipped default maps, so this
    // action starts unassigned like Set Position/Unify. Remappable.
    ("Scale Offset", ""),
    ("Flatten Ngons", ""),
    // MK-U01: one-shot Make, default plain P; remappable. Edge context only.
    ("Make", "P"),
    ("Select More", "ArrowUp"),
    ("Select Less", "ArrowDown"),
    ("Select Between", "Shift+G"),
    // KBD-125/127 (SEL-057): Left/Right step to the previous/next loop.
    ("Previous Loop", "ArrowLeft"),
    ("Next Loop", "ArrowRight"),
    // SYM-SEL-01 / SYM-SZ-01: no shipped default binding; remappable.
    ("Select Symmetrical", ""),
    // Section 8 (Q2): one key toggles Symmetry. No default key until the
    // reference default is known (SYM3-Q18); remappable.
    ("Toggle Symmetry", ""),
    ("Symmetrize Positive", ""),
    ("Symmetrize Negative", ""),
    ("Symmetrize Polygons Positive", ""),
    ("Symmetrize Polygons Negative", ""),
    ("Symmetry Tool", ""),
    // Edit tab Slice tools (default shortcuts spec: G = Edge Slice; Edit
    // panel spec: Alt+C, Shift+C).
    ("Loop Slice", "Alt+C"),
    ("Edge Slice", "G"),
    ("Polygon Slice", "Shift+C"),
    // Waves 2 and 3: the spec gives no shortcuts (assign in the Input Map).
    ("Curve Slice", ""),
    ("Pen Slice", ""),
    ("Primitive Slice", ""),
    ("Polygon Axis Slice", ""),
    // Topology tab Contour (missing-tools-g4): no shortcut given.
    ("Contour", ""),
    // Duplicate tab: the spec gives no shortcuts (assign in the Input Map).
    ("Mirror", ""),
    ("Radial Sweep", ""),
    ("Clone", ""),
    ("Array", ""),
    ("Radial Array", ""),
    ("Detriangulate", ""),
    // Remesh: no shortcuts (assign in the Input Map).
    ("Remesh", ""),
    ("Automatic Retopology", ""),
    // BOOL-23: no shortcut in the spec (assign in the Input Map).
    ("Boolean", ""),
    // DRILL-08: no shortcuts in the spec.
    ("Solid Drill", ""),
    ("Axis Drill", ""),
    // Pie menus (pie_menu.rs): each is the line that opens it (UIC-311),
    // so any pie, a user's too, binds like an action; the Input Map shows
    // these as "Pie: View Direction" and so on. "Ctrl" is the real Control
    // key on macOS (Cmd+Space is Spotlight). The other three start
    // unassigned.
    (crate::pie_menu::VIEW_LINE, "Ctrl+Space"),
    (crate::pie_menu::SELECTION_LINE, "Alt+Q"),
    (crate::pie_menu::SHADING_LINE, "Ctrl+2"),
    (crate::pie_menu::TOGGLES_LINE, "Ctrl+1"),
    (crate::pie_menu::FALLOFFS_LINE, ""),
    (crate::pie_menu::ACTION_CENTERS_LINE, ""),
    (crate::pie_menu::MODELING_LINE, ""),
    // Layouts stage 2 (UIC-18, LAY-32): "Ctrl" is the real Control key on
    // macOS (Cmd+Shift+Tab and Cmd+` are the system's).
    (crate::pie_menu::LAYOUTS_LINE, "Ctrl+`"),
    // Ctrl+Shift+Tab pages the tool tabs now (tool-tab-cycling v1.1,
    // Jason 2026-11-25): the switcher keeps Layout > Recent Layouts….
    ("Layout Switcher", ""),
    // Tool tab cycling (tool_tab_cycle.rs): the real Control key on a Mac;
    // Cmd+Tab belongs to the system.
    (crate::tool_tab_cycle::NEXT, "Ctrl+Tab"),
    (crate::tool_tab_cycle::PREVIOUS, "Ctrl+Shift+Tab"),
    // Layouts stage 6 (LAY-53, UIC-206).
    ("Toggle Palettes", "`"),
    // Window ▸ Node View (node_view_open.rs): N for nodes; Cmd+Shift+N is
    // free in the global map, the shortcuts spec and every shipped
    // layout's keys (UIC-201). Ctrl+Shift+N off macOS.
    ("Node View", "Cmd+Shift+N"),
    // Help > What's This? (UIC-06, help_mode.rs): F1 is free in the
    // factory map and every shipped layout's keys.
    (crate::help_mode::ACTION, "F1"),
    // UI build mode (Jason, 2026-11-20): not in the shortcuts spec (v1.1),
    // where Ctrl+Shift+L is free; "Ctrl" is the real Control key on macOS.
    ("UI Build Mode", "Ctrl+Shift+L"),
    // Animation keys (anim_ui.rs, SC-241..249), Items mode only.
    ("Play/Stop", ""),
    ("Play Backwards", ""),
    ("Back One Frame", "Shift+ArrowLeft"),
    ("Forward One Frame", "Shift+ArrowRight"),
    ("Step Back", ""),
    ("Step Forward", ""),
    ("First Frame", "Shift+ArrowUp"),
    ("Last Frame", "Shift+ArrowDown"),
    ("Previous Key", "Ctrl+ArrowLeft"),
    ("Next Key", "Ctrl+ArrowRight"),
    ("First Key", "Ctrl+ArrowUp"),
    ("Last Key", "Ctrl+ArrowDown"),
    ("Key Selected Channels", "S"),
    ("Key Selected Items", "Shift+S"),
    ("Key Position", "Shift+W"),
    ("Key Rotation", "Shift+E"),
    ("Key Scale", ""),
    ("Key Transforms", ""),
];
pub fn default_bindings() -> std::collections::BTreeMap<String, String> {
    ACTIONS
        .iter()
        .map(|&(action, chord)| (action.into(), chord.into()))
        .collect()
}
/// Merge a persisted map over factory bindings. Before Set Material existed,
/// a user could assign plain M to another action. Keep that saved action's
/// effective shortcut by leaving only the newly introduced default unbound.
/// Explicit Set Material entries, including an empty binding, retain the
/// established conflict behavior and are never rewritten here.
/// Edge-panel actions added in Milestone 2 (see `load_bindings`).
const M2_EDGE_ACTIONS: &[&str] = &[
    "Extend",
    "Edge Extrude",
    "Slide",
    "Edge Chamfer",
    "Grow Quads",
    "Edge Spin",
    "Edge Collapse",
    "Edge Remove",
    "Edge Subdivide",
];
/// NAV-12: the view centre moves to the point under the pointer.
pub const RECENTRE: &str = "Re-centre";
/// Navigation, geometry lock and hide actions (nav-lock-hide-2026-11-07).
pub const NAV_LOCK_HIDE_ACTIONS: &[&str] = &[
    "Zoom In",
    "Zoom Out",
    "Turn",
    "Turn Upright",
    "Align Selected",
    "Fit and Align Selected",
    "Lock Selected",
    "Lock Unselected",
    "Lock Invert",
    "Unlock",
    "Hide Selected",
    "Hide Unselected",
    "Hide Invert",
    "Unhide",
];
/// The lock/hide request a lower-case action name stands for.
pub fn element_action_lowered(
    action: &str,
) -> Option<(operandi_core::ElementKind, operandi_core::ElementOp)> {
    use operandi_core::{ElementKind::*, ElementOp::*};
    Some(match action {
        "lock selected" => (Lock, Selected),
        "lock unselected" => (Lock, Unselected),
        "lock invert" => (Lock, Invert),
        "unlock" => (Lock, Clear),
        "hide selected" => (Hide, Selected),
        "hide unselected" => (Hide, Unselected),
        "hide invert" => (Hide, Invert),
        "unhide" => (Hide, Clear),
        _ => return None,
    })
}
/// Vertex-panel actions added in Milestone 3 with default chords.
const M3_VERTEX_ACTIONS: &[&str] = &[
    "Vertex Bevel",
    "Vertex Extrude",
    "Vertex Slide",
    "Vertex Collapse",
    "Vertex Remove",
];
pub fn load_bindings(path: &std::path::Path) -> std::collections::BTreeMap<String, String> {
    let mut stored: std::collections::BTreeMap<String, String> = std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    // Pies became forms (UIC-311): a map saved before then names the
    // shipped pies' old actions; they become the lines that open them.
    crate::pie_menu::migrate_keys(&mut stored);
    let mut bindings = default_bindings();
    // Ctrl+Shift+Tab moved from the Layout Switcher to Previous Tool Tab
    // (2026-11-25): a map saved with the switcher's old factory key lets
    // it go, so the new action gets it.
    if !stored.contains_key(crate::tool_tab_cycle::PREVIOUS)
        && stored.get("Layout Switcher").map(String::as_str) == Some("Ctrl+Shift+Tab")
    {
        stored.insert("Layout Switcher".into(), String::new());
    }
    // Tweak was renamed Element Move (Jason, 2026-11-06): a saved map keeps
    // its binding under the new name.
    if let Some(old) = stored.remove("Tweak") {
        stored.entry("Element Move".into()).or_insert(old);
    }
    // Undo, Redo and Preferences gained chords together. A map saved before
    // then has no Preferences entry, and its empty Undo/Redo were only the
    // old factory default: give them the new chords unless the saved map
    // already uses a chord for another action.
    if !stored.contains_key("Preferences") {
        for action in ["Undo", "Redo", "Preferences"] {
            let Some((mods, key)) = bindings.get(action).and_then(|b| chord(b)) else {
                continue;
            };
            if stored
                .iter()
                .any(|(other, binding)| other != action && matches_chord(binding, key, mods))
            {
                bindings.insert(action.into(), String::new());
            } else if stored.get(action).is_some_and(String::is_empty) {
                stored.remove(action);
            }
        }
    }
    if !stored.contains_key("Set Material")
        && stored
            .values()
            .any(|binding| matches_chord(binding, eframe::egui::Key::M, Modifiers::NONE))
    {
        bindings.insert("Set Material".into(), String::new());
    }
    if !stored.contains_key("Split")
        && stored
            .values()
            .any(|binding| matches_chord(binding, eframe::egui::Key::L, Modifiers::CTRL))
    {
        bindings.insert("Split".into(), String::new());
    }
    if !stored.contains_key("Spin Quad")
        && stored
            .values()
            .any(|binding| matches_chord(binding, eframe::egui::Key::V, Modifiers::ALT))
    {
        bindings.insert("Spin Quad".into(), String::new());
    }
    // Tweak gained plain T later: a saved map that already uses T for
    // another action keeps it, and Tweak starts unassigned there.
    if !stored.contains_key("Element Move")
        && stored
            .values()
            .any(|binding| matches_chord(binding, eframe::egui::Key::T, Modifiers::NONE))
    {
        bindings.insert("Element Move".into(), String::new());
    }
    if !stored.contains_key("Make")
        && stored
            .values()
            .any(|binding| matches_chord(binding, eframe::egui::Key::P, Modifiers::NONE))
    {
        bindings.insert("Make".into(), String::new());
    }
    if !stored.contains_key("Polygon Collapse")
        && stored.iter().any(|(action, binding)| {
            contexts_overlap(action, "Polygon Collapse")
                && matches_chord(binding, eframe::egui::Key::Backspace, Modifiers::CTRL)
        })
    {
        bindings.insert("Polygon Collapse".into(), String::new());
    }
    // Actions added after the first saved maps: a map that already gives
    // their chord to another action keeps it, and they start unassigned.
    for action in [
        crate::viewport_buttons::MAXIMIZE,
        crate::viewport_buttons::MAXIMIZE_ALT,
        crate::viewport_properties::ACTION,
        "Loop Slice",
        "Edge Slice",
        "Polygon Slice",
        "Curve Slice",
        "Pen Slice",
        "Primitive Slice",
        "Polygon Axis Slice",
        "Contour",
        "Mirror",
        "Radial Sweep",
        "Clone",
        "Array",
        "Radial Array",
        "Detriangulate",
        "Remesh",
        "Automatic Retopology",
        "Boolean",
        "Solid Drill",
        "Axis Drill",
        "Reset Tool Attributes",
        crate::pie_menu::VIEW_LINE,
        crate::pie_menu::SELECTION_LINE,
        crate::pie_menu::SHADING_LINE,
        crate::pie_menu::TOGGLES_LINE,
        crate::tool_tab_cycle::NEXT,
        crate::tool_tab_cycle::PREVIOUS,
    ]
    .into_iter()
    .chain(NAV_LOCK_HIDE_ACTIONS.iter().copied())
    {
        let Some((mods, key)) = bindings.get(action).and_then(|b| chord(b)) else {
            continue;
        };
        let keypad = bindings[action].contains("Numpad");
        if !stored.contains_key(action)
            && stored.values().any(|binding| {
                matches_chord(binding, key, mods) && binding.contains("Numpad") == keypad
            })
        {
            bindings.insert(action.into(), String::new());
        }
    }
    // M2 Edge actions: a map saved before they existed keeps any action it
    // already bound to the same chord in an overlapping context.
    for (action, default) in ACTIONS {
        let added = M2_EDGE_ACTIONS.contains(action) || M3_VERTEX_ACTIONS.contains(action);
        if !added || default.is_empty() || stored.contains_key(*action) {
            continue;
        }
        let Some((mods, key)) = chord(default) else {
            continue;
        };
        if stored.iter().any(|(other, binding)| {
            contexts_overlap(other, action) && matches_chord(binding, key, mods)
        }) {
            bindings.insert((*action).into(), String::new());
        }
    }
    // KEY-01/02 (2026-11-03): a map saved with the earlier factory chords
    // for these actions takes the measured ones, unless another action in
    // an overlapping context already uses the new chord.
    for (action, old_default) in [
        ("Bevel", "Shift+B"),
        ("Edge Chamfer", "B"),
        ("Edge Bevel", ""),
        ("Transform", ""),
    ] {
        if stored.get(action).map(String::as_str) != Some(old_default) {
            continue;
        }
        let Some(new) = bindings.get(action).cloned() else {
            continue;
        };
        let taken = chord(&new).is_some_and(|(mods, key)| {
            stored.iter().any(|(other, binding)| {
                other != action
                    && !(other == "Edge Chamfer" && action == "Edge Bevel")
                    && contexts_overlap(other, action)
                    && matches_chord(binding, key, mods)
            })
        });
        if !taken {
            stored.remove(action);
        }
    }
    if stored.get("Edge Chamfer").map(String::as_str) == Some("B")
        && !stored.contains_key("Edge Bevel")
    {
        stored.remove("Edge Chamfer");
    }
    bindings.extend(stored);
    bindings
}
/// "Cmd" in a binding is the platform's command key: Command on macOS,
/// Control on Linux and Windows, so one saved map works on all three.
/// "Ctrl" is always the real Control key.
const CMD_IS_CTRL: bool = !cfg!(target_os = "macos");
/// A binding as it reads on this platform: "Cmd" shows as "Ctrl" on Linux
/// and Windows. The saved text keeps "Cmd".
pub fn display_chord(text: &str) -> String {
    display_chord_on(text, CMD_IS_CTRL)
}
fn display_chord_on(text: &str, cmd_is_ctrl: bool) -> String {
    if !cmd_is_ctrl || chord_on(text, true).is_none() {
        return text.to_owned();
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in text.split('+') {
        let part = match part.to_ascii_lowercase().as_str() {
            "cmd" | "command" | "ctrl" | "control" => "Ctrl",
            _ => part,
        };
        if part != "Ctrl" || !parts.contains(&"Ctrl") {
            parts.push(part);
        }
    }
    parts.join("+")
}
/// Literal modifiers apart from "Cmd" (see [`CMD_IS_CTRL`]). Numeric keypad
/// bindings require separate native location evidence at invocation, never a
/// digit alias.
pub fn chord(text: &str) -> Option<(Modifiers, eframe::egui::Key)> {
    chord_on(text, CMD_IS_CTRL)
}
fn chord_on(text: &str, cmd_is_ctrl: bool) -> Option<(Modifiers, eframe::egui::Key)> {
    let mut mods = Modifiers::NONE;
    let mut parts = text.split('+').peekable();
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            let key = if let Some(suffix) = part.strip_prefix("Numpad") {
                if suffix == "Decimal" {
                    eframe::egui::Key::Period
                } else if suffix.len() == 1 && suffix.as_bytes()[0].is_ascii_digit() {
                    eframe::egui::Key::from_name(suffix)?
                } else {
                    return None;
                }
            } else {
                eframe::egui::Key::from_name(part)?
            };
            // Plain Space is reserved for navigation and the mode cycle;
            // with a modifier (Ctrl+Space opens the view pie) it binds.
            if key == eframe::egui::Key::Space && mods.is_none() {
                return None;
            }
            return Some((mods, key));
        }
        match part.to_ascii_lowercase().as_str() {
            "shift" => mods.shift = true,
            "alt" | "option" => mods.alt = true,
            "ctrl" | "control" => mods.ctrl = true,
            "cmd" | "command" if cmd_is_ctrl => {
                mods.ctrl = true;
                mods.command = true;
            }
            "cmd" | "command" => {
                mods.mac_cmd = true;
                mods.command = true;
            }
            _ => return None,
        }
    }
    None
}
/// The key a shifted symbol is typed on (US layout). With Shift held the
/// window reports the symbol it types, so Shift+] arrives as "}" and
/// Shift+/ as "?"; bindings and the input map name the key under it
/// (bug: Close Loop's Shift+] never fired, 2026-11-21). Keys without a
/// shifted twin come back unchanged.
pub fn unshifted(key: eframe::egui::Key) -> eframe::egui::Key {
    use eframe::egui::Key;
    match key {
        Key::Questionmark => Key::Slash,
        Key::OpenCurlyBracket => Key::OpenBracket,
        Key::CloseCurlyBracket => Key::CloseBracket,
        Key::Colon => Key::Semicolon,
        Key::Pipe => Key::Backslash,
        Key::Plus => Key::Equals,
        Key::Exclamationmark => Key::Num1,
        other => other,
    }
}
pub fn matches_chord(text: &str, key: eframe::egui::Key, mods: Modifiers) -> bool {
    use eframe::egui::Key;
    chord(text).is_some_and(|(expected, k)| {
        // Shift+/ arrives as "?" on most layouts: a Slash binding takes it
        // too (NAV-14). The same for every shifted symbol: Shift+] arrives
        // as "}" and takes a "Shift+]" binding.
        (k == key
            || (k == Key::Slash && key == Key::Questionmark)
            || (mods.shift && k == unshifted(key)))
            && expected.alt == mods.alt
            && expected.shift == mods.shift
            && expected.ctrl == mods.ctrl
            && expected.mac_cmd == mods.mac_cmd
    })
}
/// The macOS app menu's Preferences… key equivalent (#67), from the input
/// map's Preferences binding: "," (shown as ⌘, and handled by the menu)
/// while it is Cmd+, as by default; none once it's remapped or cleared, so
/// the menu never shadows the new chord or takes Cmd+, from another action.
pub fn preferences_key_equivalent(binding: Option<&str>) -> &'static str {
    let cmd_comma = binding.is_some_and(|b| {
        matches_chord(
            b,
            eframe::egui::Key::Comma,
            eframe::egui::Modifiers::MAC_CMD,
        )
    });
    if cmd_comma { "," } else { "" }
}
/// Actions that keep a live Move, Rotate, Scale or Transform tool and
/// re-target it instead of needing Drop first (UIC-306).
pub fn keeps_transform(action: &str) -> bool {
    matches!(action, "Grow" | "Shrink")
}

pub fn available(action: &str, mode: operandi_core::Mode, live: bool) -> bool {
    match action {
        "Grow"
        | "Shrink"
        | "Loop"
        | "Ring"
        | "Select All"
        | "Invert"
        | "Connected"
        | "Invert Connected"
        | "Convert to Vertices"
        | "Convert to Edges"
        | "Convert to Polygons"
        | "Boundary"
        | "Select More"
        | "Select Less"
        | "Select Between"
        | "Next Loop"
        | "Previous Loop"
        | "Select Symmetrical" => mode != operandi_core::Mode::Item && !live,
        "Symmetrize Positive"
        | "Symmetrize Negative"
        | "Symmetrize Polygons Positive"
        | "Symmetrize Polygons Negative" => !live,
        "Flip" => mode == operandi_core::Mode::Polygon && !live,
        // Like Flip: Polygon mode, nothing live (Item mode runs from the
        // command line on the whole mesh).
        "Flip It" | "Flip It Origin" => mode == operandi_core::Mode::Polygon && !live,
        "Split" => mode == operandi_core::Mode::Vertex,
        "Unify" => mode == operandi_core::Mode::Polygon,
        // Opens the inset dialog; an active tool is refused by the shared
        // read-only target check with feedback, never dropped.
        "Scale Offset" | "Flatten Ngons" => mode == operandi_core::Mode::Polygon,
        "Quadruple" => mode == operandi_core::Mode::Polygon,
        "Spin Quad" => mode == operandi_core::Mode::Polygon,
        "Polygon Align" => mode == operandi_core::Mode::Polygon,
        // Live tools still reach the command, which refuses with feedback.
        "Polygon Merge" => mode == operandi_core::Mode::Polygon,
        // DT-11: any mode (the whole mesh outside Polygon mode).
        "Detriangulate" => !live,
        "Boolean" | "Solid Drill" | "Axis Drill" => !live,
        "Remesh" | "Automatic Retopology" => !live,
        // UIC-307: N works with a tool on; the editor settles the tool and
        // starts it again on the new mesh (tool_yield.rs).
        "New Mesh" => true,
        // Polygon context only; an active tool is refused by the shared
        // operation with feedback rather than silently ignored here.
        "Remove" => mode == operandi_core::Mode::Polygon,
        // Same contextual rule as Remove: Polygon mode only; an active tool
        // is refused by the shared operation with feedback.
        "Polygon Collapse" => mode == operandi_core::Mode::Polygon,
        // MK-U01: Edge context only; an active tool reaches the shared
        // operation, which refuses with feedback instead of dropping it.
        "Make" => matches!(
            mode,
            operandi_core::Mode::Edge | operandi_core::Mode::Vertex
        ),
        // M3 Vertex panel: Vertex mode; the shared operations refuse other
        // states (an active tool, no selection) with feedback.
        "Create"
        | "Vertex Bevel"
        | "Vertex Extrude"
        | "Merge Tool"
        | "Weld"
        | "Tear"
        | "Vertex Slide"
        | "Vertex Split"
        | "Vertex Split Edge"
        | "Vertex Set Position"
        | "Vertex Align"
        | "Vertex Merge"
        | "Vertex Join"
        | "Vertex Join Averaged"
        | "Vertex Collapse"
        | "Vertex Remove"
        | "Vertex Remove Edge"
        | "Reorder Index" => mode == operandi_core::Mode::Vertex,
        "Sync Index Order" => mode == operandi_core::Mode::Polygon,
        // KEY-02: B is Polygon Bevel only in Polygon mode.
        "Bevel" => mode == operandi_core::Mode::Polygon,
        "Inset" | "Shift" | "Twist Bevel" => mode == operandi_core::Mode::Polygon,
        "Edge Extrude" | "Extend" | "Slide" | "Edge Weight" | "Edge Bevel" | "Edge Chamfer"
        | "Edge Relax" => mode == operandi_core::Mode::Edge,
        "Grow Quads" | "Fill Quads" | "Edge Split" | "Split Normals" | "Edge Spin" | "Join"
        | "Join Averaged" | "Edge Collapse" | "Edge Remove" | "Edges To Curves" | "Unbevel"
        | "Edge Subdivide" => mode == operandi_core::Mode::Edge,
        // SUB-03: D runs SDS Subdivide in every mode but Edge.
        "SDS Subdivide" => mode != operandi_core::Mode::Edge,
        // SUB-03: Shift+D is the options in component modes, Instance in
        // Item mode.
        "Subdivide Options" => mode != operandi_core::Mode::Item,
        "Instance" => mode == operandi_core::Mode::Item,
        "Extrude" => !matches!(
            mode,
            operandi_core::Mode::Edge | operandi_core::Mode::Vertex
        ),
        "Drop" | "Drop Cube" => live,
        // TK-65 / UIC-201: the animation keys are Items-mode keys.
        a if crate::anim_keys::is_animation_action(a) => crate::anim_keys::available(a, mode, live),
        "Drop Selection" => !live,
        _ => true,
    }
}

/// What Delete / Backspace run over a viewport in `mode`: one app-wide
/// rule, whichever tool or panel is showing (Jason, 2026-11-21). In
/// Vertex, Edge and Polygon mode both keys run `delete`: the selected
/// elements go with every point left unused (a deleted point takes its
/// polygons). Removing polygons or edges but keeping their points is
/// Shift+Backspace (Remove, Edge Remove, Vertex Remove). In Item mode they
/// delete the selected items, asking first before the scene's only mesh
/// item goes (`Operandi::request_delete`); "Delete Mesh" is Delete Item
/// without the Shader Tree's layer deletion, which belongs to the list.
/// Remapping "Delete Item" / "Delete Item Forward" moves this too.
pub fn viewport_delete_action(mode: operandi_core::Mode) -> &'static str {
    match mode {
        operandi_core::Mode::Item => "Delete Mesh",
        _ => "delete",
    }
}

/// Why `action` is not [`available`] now, worded as what to do ("Switch
/// to Polygons mode first"); `None` when it is available. Derived from
/// `available` itself, so the two can't disagree.
pub fn unavailable_reason(action: &str, mode: operandi_core::Mode, live: bool) -> Option<String> {
    use operandi_core::Mode;
    if available(action, mode, live) {
        return None;
    }
    let name = |m: Mode| match m {
        Mode::Vertex => "Vertices",
        Mode::Edge => "Edges",
        Mode::Polygon => "Polygons",
        Mode::Item => "Items",
    };
    let modes = |live: bool| -> Vec<&str> {
        [Mode::Vertex, Mode::Edge, Mode::Polygon, Mode::Item]
            .into_iter()
            .filter(|&m| available(action, m, live))
            .map(name)
            .collect()
    };
    let to_mode = |modes: Vec<&str>| match modes.as_slice() {
        [one] => format!("Switch to {one} mode"),
        [rest @ .., last] => format!("Switch to {} or {last} mode", rest.join(", ")),
        [] => String::new(),
    };
    Some(if live && available(action, mode, false) {
        crate::unavailable::TOOL_ACTIVE.to_owned()
    } else if !live && available(action, mode, true) {
        "Only while a tool is active".to_owned()
    } else if !modes(live).is_empty() {
        format!("{} first", to_mode(modes(live)))
    } else if live && !modes(false).is_empty() {
        format!(
            "Drop or Cancel the active tool, then {} first",
            to_mode(modes(false)).replacen("Switch", "switch", 1)
        )
    } else {
        "Not available now".to_owned()
    })
}

/// Whether two actions can both be available in some selection mode and tool
/// state. Actions that never overlap (for example Edge Extrude and polygon
/// Extrude) may share a chord, as context shortcuts do.
pub fn contexts_overlap(a: &str, b: &str) -> bool {
    use operandi_core::Mode;
    // Render window keys never apply in the main window, and back.
    let viewer = crate::image_viewer_ui::is_viewer_action;
    if viewer(a) != viewer(b) {
        return false;
    }
    [Mode::Vertex, Mode::Edge, Mode::Polygon, Mode::Item]
        .into_iter()
        .any(|mode| {
            [false, true]
                .into_iter()
                .any(|live| available(a, mode, live) && available(b, mode, live))
        })
}

/// Input-map validity: every chord parses, and actions sharing a chord never
/// overlap in context.
pub fn bindings_valid(bindings: &std::collections::BTreeMap<String, String>) -> bool {
    let mut by_chord: std::collections::BTreeMap<String, Vec<&str>> = Default::default();
    for (action, binding) in bindings.iter().filter(|(_, b)| !b.is_empty()) {
        if reserved_file_chord(action, binding) {
            return false;
        }
        let Some(parsed) = chord(binding) else {
            return false;
        };
        by_chord
            .entry(format!("{parsed:?}:{}", binding.contains("Numpad")))
            .or_default()
            .push(action);
    }
    by_chord.values().all(|actions| {
        actions
            .iter()
            .enumerate()
            .all(|(i, a)| actions[i + 1..].iter().all(|b| !contexts_overlap(a, b)))
    })
}

impl GestureKind {
    pub fn draw(params: operandi_core::Parameters, plane: u8, center: glam::DVec3) -> Self {
        Self::Primitive(crate::primitive_input::Gesture::Draw(params, plane, center))
    }
    pub fn side(axis: usize, maximum: bool, params: operandi_core::Parameters, jump: bool) -> Self {
        Self::Primitive(crate::primitive_input::Gesture::Side(
            axis, maximum, params, jump,
        ))
    }
    pub fn segments(params: operandi_core::Parameters, plane: u8) -> Self {
        Self::Primitive(crate::primitive_input::Gesture::Segments(params, plane))
    }
}
#[cfg(test)]
mod platform_tests {
    use super::*;
    use eframe::egui::Key;
    /// What egui reports for the Control key on Linux and Windows.
    const LINUX_CTRL: Modifiers = Modifiers {
        alt: false,
        ctrl: true,
        shift: false,
        mac_cmd: false,
        command: true,
    };
    fn matches_on(text: &str, key: Key, mods: Modifiers, cmd_is_ctrl: bool) -> bool {
        chord_on(text, cmd_is_ctrl).is_some_and(|(expected, k)| {
            k == key
                && expected.alt == mods.alt
                && expected.shift == mods.shift
                && expected.ctrl == mods.ctrl
                && expected.mac_cmd == mods.mac_cmd
        })
    }
    #[test]
    fn cmd_bindings_are_ctrl_on_linux_and_windows() {
        assert!(matches_on("Cmd+Z", Key::Z, LINUX_CTRL, true));
        assert!(matches_on(
            "Shift+Cmd+Z",
            Key::Z,
            LINUX_CTRL.plus(Modifiers::SHIFT),
            true
        ));
        assert!(matches_on("Cmd+,", Key::Comma, LINUX_CTRL, true));
        assert!(!matches_on("Cmd+Z", Key::Z, Modifiers::NONE, true));
        // macOS keeps Command and Control apart.
        assert!(!matches_on("Cmd+Z", Key::Z, Modifiers::CTRL, false));
        assert!(matches_on(
            "Cmd+Z",
            Key::Z,
            Modifiers::MAC_CMD.plus(Modifiers::COMMAND),
            false
        ));
        // Ctrl stays the real Control key everywhere.
        assert!(matches_on("Ctrl+L", Key::L, LINUX_CTRL, true));
        assert!(matches_on("Ctrl+L", Key::L, Modifiers::CTRL, false));
    }
    #[test]
    fn file_chords_are_exact_on_both_platform_mappings() {
        for (cmd, cmd_is_ctrl) in [
            (LINUX_CTRL, true),
            (Modifiers::MAC_CMD | Modifiers::COMMAND, false),
        ] {
            assert_eq!(
                fixed_file_action(Key::S, cmd, cmd_is_ctrl),
                Some(FileShortcut::Save)
            );
            assert_eq!(
                fixed_file_action(Key::W, cmd, cmd_is_ctrl),
                Some(FileShortcut::Close)
            );
            assert_eq!(
                fixed_file_action(Key::W, cmd | Modifiers::SHIFT, cmd_is_ctrl),
                Some(FileShortcut::CloseAll)
            );
            for extra in [Modifiers::ALT, Modifiers::SHIFT] {
                assert_eq!(fixed_file_action(Key::S, cmd | extra, cmd_is_ctrl), None);
            }
            assert_eq!(
                fixed_file_action(Key::W, cmd | Modifiers::ALT, cmd_is_ctrl),
                None
            );
        }
        assert_eq!(
            fixed_file_action(Key::S, Modifiers::MAC_CMD | Modifiers::CTRL, false),
            None
        );
    }

    #[test]
    fn cmd_reads_as_ctrl_on_linux_and_windows() {
        assert_eq!(display_chord_on("Cmd+Z", true), "Ctrl+Z");
        assert_eq!(display_chord_on("Shift+Cmd+Z", true), "Shift+Ctrl+Z");
        assert_eq!(display_chord_on("Ctrl+Cmd+Z", true), "Ctrl+Z");
        assert_eq!(display_chord_on("Cmd+Z", false), "Cmd+Z");
        assert_eq!(display_chord_on("Alt+1", true), "Alt+1");
        assert_eq!(display_chord_on("", true), "");
    }
    #[test]
    fn factory_cmd_chords_do_not_collide_with_ctrl_chords_off_mac() {
        let defaults = default_bindings();
        let parsed: Vec<(&String, (Modifiers, Key))> = defaults
            .iter()
            .filter_map(|(action, b)| chord_on(b, true).map(|c| (action, c)))
            .filter(|(_, (m, _))| m.ctrl)
            .collect();
        for (i, (a, x)) in parsed.iter().enumerate() {
            for (b, y) in &parsed[i + 1..] {
                assert!(
                    format!("{x:?}") != format!("{y:?}") || !contexts_overlap(a, b),
                    "{a} and {b} share a chord off macOS"
                );
            }
        }
    }
}

#[cfg(test)]
mod unavailable_reason_tests {
    use super::*;
    use operandi_core::Mode;

    #[test]
    fn every_unavailable_action_says_why_and_no_available_one_does() {
        for (action, _) in ACTIONS {
            for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon, Mode::Item] {
                for live in [false, true] {
                    let why = unavailable_reason(action, mode, live);
                    assert_eq!(why.is_none(), available(action, mode, live), "{action}");
                    assert!(why.is_none_or(|w| !w.is_empty()), "{action}");
                }
            }
        }
    }

    #[test]
    fn reasons_say_what_to_do() {
        let why = |a, m, l| unavailable_reason(a, m, l).unwrap();
        assert_eq!(
            why("Bevel", Mode::Edge, false),
            "Switch to Polygons mode first"
        );
        assert_eq!(
            why("Grow", Mode::Polygon, true),
            crate::unavailable::TOOL_ACTIVE
        );
        assert_eq!(
            why("Drop", Mode::Polygon, false),
            "Only while a tool is active"
        );
        assert_eq!(
            why("Make", Mode::Polygon, false),
            "Switch to Vertices or Edges mode first"
        );
        assert_eq!(
            why("Flip", Mode::Edge, true),
            "Drop or Cancel the active tool, then switch to Polygons mode first"
        );
    }
}
