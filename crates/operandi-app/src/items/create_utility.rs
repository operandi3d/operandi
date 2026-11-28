//! Create panel groups below Draw (Create panel spec v2): Edit (Cut, Copy,
//! Paste, Paste Tool), Merge (Unmerge, Merge), Align (Quick Align, Ground
//! Align) and Measure (Ruler, Protractor, Dimensions).
//!
//! The tools here that stay on (Paste Tool, Align, Measure) are "utility"
//! tools: the app holds their settings, each viewport action runs one core
//! command (one undo step), and Measure never changes the scene. Turning on
//! any modeling tool turns a utility tool off, and the other way round.
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, ToolFields, ToolForm};
use operandi_core::{
    GroundAlign, GroundMode, PastePlacement, QuickAlign, QuickAlignKind, SelectionAxisMode,
};

/// The Create utilities' Tool Properties (Paste Tool, Ruler, Protractor,
/// Dimensions, Ground Align, Quick Align).
pub(super) struct UtilityFields;

/// Three distance fields of one utility's vector.
fn utility_vec(
    f: &mut ToolForm<Utility>,
    key: &str,
    label: &str,
    unit: Unit,
    get: impl Fn(&Utility) -> Option<DVec3> + 'static,
    set: impl Fn(&mut Utility, usize, f64) + 'static,
) {
    let fields = tf::vector(
        key,
        label,
        unit,
        move |u: &Utility| get(u).unwrap_or_default(),
        move |u, a, v| {
            set(u, a, v);
            Ok(())
        },
    );
    if unit == Unit::Percent {
        f.fields_from(fields.into_iter().map(|x| x.suffix("%")).collect());
    } else {
        f.fields_from(fields);
    }
}

impl ToolFields for UtilityFields {
    type P = Utility;
    fn params(app: &Operandi) -> Option<Utility> {
        app.utility
    }
    fn form(app: &Operandi, u: &Utility) -> ToolForm<Utility> {
        use Utility as U;
        let mut f = ToolForm::new();
        f.title(u.label());
        match u {
            U::PasteTool(_) => {
                utility_vec(
                    &mut f,
                    "pst_center",
                    "Center",
                    Unit::Distance,
                    |u| matches!(u, U::PasteTool(_)).then(|| paste(u).center),
                    |u, a, v| {
                        if let U::PasteTool(p) = u {
                            p.center[a] = v;
                        }
                    },
                );
                utility_vec(
                    &mut f,
                    "pst_size",
                    "Size",
                    Unit::Percent,
                    |u| matches!(u, U::PasteTool(_)).then(|| paste(u).size * 100.),
                    |u, a, v| {
                        if let U::PasteTool(p) = u {
                            p.size[a] = v / 100.;
                        }
                    },
                );
                utility_vec(
                    &mut f,
                    "pst_bias",
                    "Bias",
                    Unit::Distance,
                    |u| matches!(u, U::PasteTool(_)).then(|| paste(u).bias),
                    |u, a, v| {
                        if let U::PasteTool(p) = u {
                            p.bias[a] = v;
                        }
                    },
                );
                f.wide(
                    tf::Button::run("Apply", |app: &mut Operandi| {
                        if let Some(U::PasteTool(p)) = app.utility {
                            app.run(Command::PasteAt(p));
                        }
                    })
                    .tip("Add one copy at Center"),
                );
            }
            U::Ruler(ruler) => {
                for (key, label, end) in [("rul_start", "Start", false), ("rul_end", "End", true)] {
                    utility_vec(
                        &mut f,
                        key,
                        label,
                        Unit::Distance,
                        move |u| match u {
                            U::Ruler(r) => Some(if end { r.end } else { r.start }),
                            _ => None,
                        },
                        move |u, a, v| {
                            if let U::Ruler(r) = u {
                                if end {
                                    r.end[a] = v;
                                } else {
                                    r.start[a] = v;
                                }
                            }
                        },
                    );
                }
                let d = ruler.end - ruler.start;
                f.field(tf::check(
                    "rul_snap",
                    "Angle Snap",
                    |u: &Utility| matches!(u, U::Ruler(r) if r.angle_snap),
                    |u, on| {
                        if let U::Ruler(r) = u {
                            r.angle_snap = on;
                        }
                    },
                ))
                .note(format!(
                    "Length {} · X {} · Y {} · Z {}",
                    workspace::number_text(d.length(), " m"),
                    workspace::number_text(d.x, " m"),
                    workspace::number_text(d.y, " m"),
                    workspace::number_text(d.z, " m")
                ));
            }
            U::Protractor(pr) => {
                for (key, label, which) in [
                    ("pro_center", "Center", 0),
                    ("pro_start", "Start", 1),
                    ("pro_end", "End", 2),
                ] {
                    utility_vec(
                        &mut f,
                        key,
                        label,
                        Unit::Distance,
                        move |u| match u {
                            U::Protractor(p) => Some([p.center, p.start, p.end][which]),
                            _ => None,
                        },
                        move |u, a, v| {
                            if let U::Protractor(p) = u {
                                [&mut p.center, &mut p.start, &mut p.end][which][a] = v;
                            }
                        },
                    );
                }
                f.field(tf::axis(
                    |u: &Utility| match u {
                        U::Protractor(p) => p.axis,
                        _ => 1,
                    },
                    |u, a| {
                        if let U::Protractor(p) = u {
                            p.axis = a;
                        }
                    },
                ))
                .note(format!("Angle {:.4}°", pr.angle()));
            }
            U::Dimensions { .. } => {
                f.field(tf::check(
                    "dim_orient",
                    "Orient to Selection",
                    |u: &Utility| matches!(u, U::Dimensions { orient: true }),
                    |u, on| {
                        if let U::Dimensions { orient } = u {
                            *orient = on;
                        }
                    },
                ));
                if let Some((size, _, _)) = app.dimension_box() {
                    f.note(format!(
                        "X {} · Y {} · Z {}",
                        workspace::number_text(size.x, " m"),
                        workspace::number_text(size.y, " m"),
                        workspace::number_text(size.z, " m")
                    ));
                }
            }
            U::GroundAlign(_) => {
                let ground = |u: &mut Utility, edit: &dyn Fn(&mut GroundAlign)| {
                    if let U::GroundAlign(g) = u {
                        edit(g);
                    }
                };
                // GAL-07: Connected is always on when the whole mesh moves.
                let whole = app.nothing_selected();
                f.field(tf::choice(
                    "gal-mode",
                    "Mode",
                    &[
                        (GroundMode::DropToGround, "Drop To Ground"),
                        (GroundMode::DropToOrigin, "Drop To Origin"),
                        (GroundMode::RotateToGround, "Rotate To Ground"),
                    ],
                    |u: &Utility| match u {
                        U::GroundAlign(g) => g.mode,
                        _ => GroundMode::DropToGround,
                    },
                    move |u, m| ground(u, &|g| g.mode = m),
                ))
                .field(
                    tf::check(
                        "gal_connected",
                        "Connected Geometry",
                        |u: &Utility| matches!(u, U::GroundAlign(g) if g.connected),
                        move |u, on| ground(u, &|g| g.connected = on),
                    )
                    .enabled_if(!whole, "With nothing selected the whole mesh moves, so Connected is always on"),
                )
                .field(tf::check(
                    "gal_by_selection",
                    "By Selection",
                    |u: &Utility| matches!(u, U::GroundAlign(g) if g.by_selection),
                    move |u, on| ground(u, &|g| g.by_selection = on),
                ))
                .note("Click in the viewport to apply. Ctrl-click: Drop To Origin. Shift-click: By Selection.");
            }
            U::QuickAlign(_) => {
                let quick = |u: &mut Utility, edit: &dyn Fn(&mut QuickAlignOptions)| {
                    if let U::QuickAlign(q) = u {
                        edit(q);
                    }
                };
                let read = |u: &Utility| match u {
                    U::QuickAlign(q) => *q,
                    _ => QuickAlignOptions::default(),
                };
                f.field(tf::choice(
                    "qal-mode",
                    "Mode",
                    &[(false, "Position"), (true, "Orientation")],
                    move |u: &Utility| read(u).orientation,
                    move |u, o| quick(u, &|q| q.orientation = o),
                ))
                .field(tf::choice(
                    "qal-axis-mode",
                    "Selection Axis",
                    &[
                        (SelectionAxisMode::SelectionIsland, "Selection Island"),
                        (SelectionAxisMode::MeshIsland, "Mesh Island"),
                        (SelectionAxisMode::TotalSelection, "Total Selection"),
                    ],
                    move |u: &Utility| read(u).islands,
                    move |u, m| quick(u, &|q| q.islands = m),
                ))
                .field(tf::check(
                    "qal_connected",
                    "Connected",
                    move |u: &Utility| read(u).connected,
                    move |u, on| quick(u, &|q| q.connected = on),
                ))
                .field(tf::check(
                    "qal_flatten",
                    "Flatten",
                    move |u: &Utility| read(u).flatten,
                    move |u, on| quick(u, &|q| q.flatten = on),
                ))
                .note("Press on a target element, move to an arrow and release. Shift: Connected. Ctrl: Flatten.");
            }
            U::Tack(_) => crate::topology_tools_ui::tack_form(&mut f),
            U::MeshPaint(s) => crate::mesh_paint_ui::mesh_paint_form(&mut f, s),
            U::Guides(e) => crate::guides_ui::guides_form(&mut f, e),
            U::RetopoSketch => {
                f.note("Left-drag across a background mesh (a shown mesh that isn't selected): the stroke becomes a Curve in the active mesh, a point every 3 cm, lying on the background.");
            }
        }
        f.separator()
            .wide(tf::Button::run("Drop", |app: &mut Operandi| {
                app.drop_utility()
            }));
        f
    }
    fn publish(app: &mut Operandi, u: Utility) -> Result<()> {
        if let Utility::Guides(edit) = u {
            app.publish_guide(edit);
            return Ok(());
        }
        if app.utility.is_some() {
            app.utility = Some(u);
            // #46: Mesh Paint keeps its settings for the next start.
            if let Utility::MeshPaint(s) = u {
                app.mesh_paint_memory = Some(s);
            }
        }
        Ok(())
    }
}

/// The Paste Tool's placement (or the default).
fn paste(u: &Utility) -> PastePlacement {
    match u {
        Utility::PasteTool(p) => *p,
        _ => PastePlacement::default(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Ruler {
    pub start: DVec3,
    pub end: DVec3,
    pub angle_snap: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Protractor {
    pub center: DVec3,
    pub start: DVec3,
    pub end: DVec3,
    /// 0 X, 1 Y, 2 Z: the plane's normal (MSR-01 default Y).
    pub axis: usize,
}
impl Default for Protractor {
    /// MSR-01 / PRO-03 factory pose.
    fn default() -> Self {
        Self {
            center: DVec3::ZERO,
            start: DVec3::X,
            end: DVec3::Z,
            axis: 1,
        }
    }
}
impl Protractor {
    fn normal(&self) -> DVec3 {
        [DVec3::X, DVec3::Y, DVec3::Z][self.axis.min(2)]
    }
    /// Degrees counter-clockwise from the first arm to the second, seen from
    /// the axis' positive side (PRO-03), 0..360.
    pub fn angle(&self) -> f64 {
        let n = self.normal();
        let (s, e) = (self.start - self.center, self.end - self.center);
        let s = s - n * s.dot(n);
        let e = e - n * e.dot(n);
        let a = n.dot(s.cross(e)).atan2(s.dot(e)).to_degrees();
        if a < 0. { a + 360. } else { a }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct QuickAlignOptions {
    pub orientation: bool,
    pub connected: bool,
    pub flatten: bool,
    pub islands: SelectionAxisMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Utility {
    PasteTool(PastePlacement),
    Ruler(Ruler),
    Protractor(Protractor),
    Dimensions {
        orient: bool,
    },
    GroundAlign(GroundAlign),
    QuickAlign(QuickAlignOptions),
    /// Tack Tool (topology_tools_ui.rs, TC-20..22).
    Tack(operandi_core::TackOptions),
    /// Retopo Sketch (TC-50..52): a stroke on the background makes a curve.
    RetopoSketch,
    /// Mesh Paint / Mesh Paint Instance (mesh_paint_ui.rs, missing-tools-g6).
    MeshPaint(operandi_core::mesh_paint::MeshPaintSettings),
    /// The Guides tool (guides_ui.rs, G9 GD-1).
    Guides(crate::guides_ui::GuideEdit),
}
impl Utility {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PasteTool(_) => "Paste Tool",
            Self::Ruler(_) => "Ruler",
            Self::Protractor(_) => "Protractor",
            Self::Dimensions { .. } => "Dimensions",
            Self::GroundAlign(_) => "Ground Align",
            Self::QuickAlign(_) => "Quick Align",
            Self::Tack(_) => "Tack Tool",
            Self::RetopoSketch => "Retopo Sketch",
            Self::MeshPaint(s) if s.replicas => "Mesh Paint Instancer",
            Self::MeshPaint(s) if s.instances => "Mesh Paint Instance",
            Self::MeshPaint(_) => "Mesh Paint",
            Self::Guides(_) => "Guides",
        }
    }
}

/// What a viewport press holds while a utility tool is on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UtilityDrag {
    /// Ruler end 0 or 1, or 2 for the whole ruler, from where it was.
    Ruler(usize, Ruler),
    /// Protractor centre (0), first arm (1) or second arm (2).
    Protractor(usize, Protractor),
    /// Quick Align: pressed on a target; release over an arrow applies.
    QuickAlign {
        target: DVec3,
        normal: DVec3,
        connected: bool,
        flatten: bool,
        /// Normal arrows (right-press or Mode Orientation), not axis arrows.
        orientation: bool,
    },
    /// Retopo Sketch: a stroke in progress (points in `retopo_stroke`).
    RetopoSketch,
    /// Mesh Paint: a click or stroke in progress (`mesh_paint`).
    MeshPaint,
    /// A Topology Pen drag (its state is `pen.drag`, topology_pen_ui.rs).
    TopologyPen,
    /// Drawing the current guide from this point (guides_ui.rs, GD-1).
    Guide(DVec3),
}

/// Screen distance that grabs a handle or arrow.
const GRAB: f32 = 10.;
/// Length of Quick Align's axis arrows on screen.
const ARROW: f32 = 60.;

/// The Ruler's angle snap: `dir` turned in the work plane with `normal` to
/// the nearest `step_degrees`, measured from the plane's first axis.
fn snap_angle(dir: DVec3, normal: DVec3, step_degrees: f64) -> DVec3 {
    let u = if normal.x.abs() > 0.9 {
        DVec3::Z
    } else {
        DVec3::X
    };
    let u = (u - normal * u.dot(normal)).normalize();
    let v = normal.cross(u);
    operandi_core::angle_snap::round_in_plane(dir, u, v, normal, step_degrees.to_radians())
}

impl Operandi {
    /// Turn a utility tool on, or off when it is already on.
    pub(super) fn activate_utility(&mut self, utility: Utility) {
        if self
            .utility
            .is_some_and(|u| std::mem::discriminant(&u) == std::mem::discriminant(&utility))
        {
            self.drop_utility();
            return;
        }
        if !self.confirm_fields() {
            return;
        }
        self.drop_draw();
        if self.editor.session.has_active_task() {
            self.run(Command::Drop);
            if self.editor.session.has_active_task() {
                return;
            }
        }
        if matches!(
            utility,
            Utility::GroundAlign(_) | Utility::QuickAlign(_) | Utility::Tack(_)
        ) && self.editor.scene.selection.mode == Mode::Item
        {
            // GAL-06.
            self.status = format!(
                "{}: selection mode must be vertex, edge or polygon",
                utility.label()
            );
            return;
        }
        self.utility = Some(utility);
        self.tool_properties_open = true;
        self.status = match utility {
            Utility::PasteTool(_) => {
                "Paste Tool · click in the viewport (or Apply) to add a copy there".into()
            }
            Utility::Ruler(_) => "Ruler · drag an end, or the middle to move it".into(),
            Utility::Protractor(_) => "Protractor · drag the centre or an arm end".into(),
            Utility::Dimensions { .. } => "Dimensions · the selection's size".into(),
            Utility::GroundAlign(_) => {
                "Ground Align · click: drop to ground · Ctrl-click: to origin · Shift-click: by selection".into()
            }
            Utility::QuickAlign(_) => {
                "Quick Align · press on a target, then release over an arrow · Shift: connected · Ctrl: flatten".into()
            }
            Utility::Tack(_) => {
                "Tack Tool · left-click a polygon (front side) to put the selection on it".into()
            }
            Utility::RetopoSketch => {
                "Retopo Sketch · left-drag across the background mesh to draw a guide curve on it".into()
            }
            Utility::MeshPaint(_) => format!(
                "{} · paint copies of a background mesh onto the selected mesh",
                utility.label()
            ),
            Utility::Guides(_) => {
                "Guides · drag to draw the current guide; its settings are in Tool Properties".into()
            }
        };
        if let Some(Utility::Ruler(_)) = self.utility {
            self.reset_ruler_pose();
        }
    }

    /// Turn a utility on or off by its command name (shortcuts and the
    /// command line).
    pub(super) fn utility_by_name(&mut self, name: &str) {
        let utility = match name {
            "quick align" => Utility::QuickAlign(Default::default()),
            "ground align" => Utility::GroundAlign(Default::default()),
            "ruler" => Utility::Ruler(Ruler {
                start: DVec3::ZERO,
                end: DVec3::X * 0.2,
                angle_snap: true,
            }),
            "protractor" => Utility::Protractor(Default::default()),
            "tack tool" => Utility::Tack(Default::default()),
            "retopo sketch" => Utility::RetopoSketch,
            // #46: each variant starts from Mesh Paint's last settings;
            // which one was started decides the two instance switches.
            "mesh paint" => self.mesh_paint_start(false, false),
            "guides" => self.guides_utility(),
            "mesh paint instance" => self.mesh_paint_start(true, false),
            // G8 RP-7: every background mesh can be picked, one per dab.
            // "Mesh Paint Replica": its name before 2026-11-23.
            "mesh paint instancer" | "mesh paint replica" => self.mesh_paint_start(false, true),
            _ => Utility::Dimensions { orient: false },
        };
        self.activate_utility(utility);
    }

    /// Mesh Paint's settings for a start: the last ones used (or the
    /// defaults) with the variant's instance switches.
    fn mesh_paint_start(&self, instances: bool, replicas: bool) -> Utility {
        Utility::MeshPaint(operandi_core::mesh_paint::MeshPaintSettings {
            instances,
            replicas,
            ..self.mesh_paint_memory.unwrap_or_default()
        })
    }

    pub(super) fn drop_utility(&mut self) {
        if let Some(u) = self.utility.take() {
            self.status = format!("{} off", u.label());
        }
    }

    /// RUL-03: a 200 mm ruler along +X near the middle of the view, on the
    /// construction plane.
    fn reset_ruler_pose(&mut self) {
        let r = self.viewport_rect;
        let start = self
            .camera
            .plane_point(r.center(), r, self.actual_plane(), self.workplane_center())
            .unwrap_or(DVec3::ZERO);
        if let Some(Utility::Ruler(ruler)) = &mut self.utility {
            ruler.start = start;
            ruler.end = start + DVec3::X * 0.2;
        }
    }

    pub(super) fn copy_selection(&mut self) {
        match self.editor.copy_selection() {
            Ok(()) => self.status = "Copied".into(),
            Err(e) => self.status = e,
        }
    }
    pub(super) fn paste_selection(&mut self) {
        if self.editor.clipboard.is_none() {
            // CLP-13: nothing to paste does nothing.
            return;
        }
        self.run(Command::PasteSelection);
    }
    pub(super) fn toggle_paste_tool(&mut self) {
        if self.editor.clipboard.is_none() {
            self.status = "Paste Tool: Cut or Copy something first".into();
            return;
        }
        self.activate_utility(Utility::PasteTool(PastePlacement::default()));
    }

    /// Called once a frame: a modeling tool that came on turns this off.
    pub(super) fn keep_utility_exclusive(&mut self) {
        if self.utility.is_some() && self.editor.session.has_active_task() {
            self.utility = None;
        }
    }

    // ------------------------------------------------------------- panel

    pub(super) fn nothing_selected(&self) -> bool {
        let s = &self.editor.scene.selection;
        match s.mode {
            Mode::Vertex => s.vertices.is_empty(),
            Mode::Edge => s.edges.is_empty(),
            Mode::Polygon => s.faces.is_empty(),
            Mode::Item => true,
        }
    }

    // ---------------------------------------------------------- viewport

    /// A press while a utility tool is on. `None` leaves the press to the
    /// normal viewport handling. Right-press only means something to the
    /// Align tools: Rotate To Ground, and Quick Align's normal arrows.
    pub(super) fn utility_press(
        &mut self,
        pos: Pos2,
        r: Rect,
        modifiers: Modifiers,
        secondary: bool,
    ) -> Option<input::GestureKind> {
        let utility = self.utility?;
        if secondary && !matches!(utility, Utility::GroundAlign(_) | Utility::QuickAlign(_)) {
            return None;
        }
        let near = |s: &Self, p: DVec3| {
            s.camera
                .project(p, r)
                .is_some_and(|(q, _)| q.distance(pos) <= GRAB)
        };
        match utility {
            Utility::PasteTool(p) => {
                let plane = self.actual_plane();
                let Some(at) = self.work_point(pos, r, plane, self.workplane_center()) else {
                    self.status = "View is parallel to the construction plane".into();
                    return Some(input::GestureKind::Blocked);
                };
                let placed = PastePlacement { center: at, ..p };
                if self.run_ok(Command::PasteAt(placed)) {
                    self.utility = Some(Utility::PasteTool(placed));
                }
                Some(input::GestureKind::Blocked)
            }
            Utility::Ruler(ruler) => {
                let hit = if near(self, ruler.end) {
                    Some(1)
                } else if near(self, ruler.start) {
                    Some(0)
                } else {
                    let (a, b) = (
                        self.camera.project(ruler.start, r),
                        self.camera.project(ruler.end, r),
                    );
                    match (a, b) {
                        (Some((a, _)), Some((b, _)))
                            if input::segment_distance(pos, a, b) <= GRAB =>
                        {
                            Some(2)
                        }
                        _ => None,
                    }
                };
                hit.map(|h| input::GestureKind::Utility(UtilityDrag::Ruler(h, ruler)))
                    .or(Some(input::GestureKind::Blocked))
            }
            Utility::Protractor(p) => {
                let hit = [p.start, p.end, p.center]
                    .iter()
                    .position(|&q| near(self, q))
                    .map(|i| [1, 2, 0][i]);
                hit.map(|h| input::GestureKind::Utility(UtilityDrag::Protractor(h, p)))
                    .or(Some(input::GestureKind::Blocked))
            }
            Utility::Dimensions { .. } => None,
            Utility::GroundAlign(g) => {
                let options = GroundAlign {
                    mode: if secondary {
                        GroundMode::RotateToGround
                    } else if modifiers.ctrl || modifiers.mac_cmd {
                        GroundMode::DropToOrigin
                    } else {
                        g.mode
                    },
                    by_selection: g.by_selection || modifiers.shift,
                    connected: g.connected || self.nothing_selected(),
                };
                self.run(Command::GroundAlign(options));
                Some(input::GestureKind::Blocked)
            }
            Utility::Tack(o) => {
                self.tack_press(o, pos, r, modifiers.shift);
                Some(input::GestureKind::Blocked)
            }
            Utility::RetopoSketch => Some(self.begin_retopo_sketch(pos, r)),
            Utility::MeshPaint(_) => Some(self.mesh_paint_press(pos, r, modifiers)),
            Utility::Guides(edit) => Some(self.guide_press(edit, pos, r)),
            Utility::QuickAlign(q) => {
                let Some((target, normal)) = self.quick_align_target(pos, r) else {
                    self.status = "Quick Align: press on a target element".into();
                    return Some(input::GestureKind::Blocked);
                };
                Some(input::GestureKind::Utility(UtilityDrag::QuickAlign {
                    target,
                    normal,
                    connected: q.connected || modifiers.shift,
                    flatten: q.flatten || modifiers.ctrl || modifiers.mac_cmd,
                    orientation: q.orientation || secondary,
                }))
            }
        }
    }

    /// The target element under the pointer: in Vertex mode the polygon's
    /// nearest point, in Edge mode its nearest edge's middle, otherwise the
    /// polygon's centre; and the polygon's normal (all world space).
    fn quick_align_target(&self, pos: Pos2, r: Rect) -> Option<(DVec3, DVec3)> {
        let surface = self.pick_surface();
        let (item, face, depth) = surface.pick(&self.camera, pos, r)?;
        let it = self.editor.scene.item(item)?;
        let mesh = &it.mesh;
        let f = mesh.faces.get(face as usize)?;
        let world = self.editor.scene.world_matrix(item).ok()?;
        let (o, d) = self.camera.ray(pos, r);
        let hit = o + d * depth;
        let wp = |v: u32| world.transform_point3(mesh.vertices[v as usize]);
        let target = match self.editor.scene.selection.mode {
            Mode::Vertex => f
                .vertices
                .iter()
                .map(|&v| wp(v))
                .min_by(|a, b| a.distance(hit).total_cmp(&b.distance(hit)))?,
            Mode::Edge => {
                let n = f.vertices.len();
                (0..n)
                    .map(|i| (wp(f.vertices[i]) + wp(f.vertices[(i + 1) % n])) / 2.)
                    .min_by(|a, b| a.distance(hit).total_cmp(&b.distance(hit)))?
            }
            _ => world.transform_point3(mesh.center(face as usize)),
        };
        let normal = if mesh.is_polyline(face as usize) {
            DVec3::Y
        } else {
            (glam::DMat3::from_mat4(world).inverse().transpose() * mesh.normal(face as usize))
                .normalize_or_zero()
        };
        Some((target, normal))
    }

    /// Screen arrows for a Quick Align press: (direction id, tip).
    /// Position: 0 X, 1 Y, 2 Z. Orientation: 3 along the normal, 4 against.
    fn quick_align_arrows(
        &self,
        target: DVec3,
        normal: DVec3,
        orientation: bool,
        r: Rect,
    ) -> Vec<(usize, Pos2, Pos2)> {
        let Some((base, _)) = self.camera.project(target, r) else {
            return Vec::new();
        };
        let dirs: Vec<(usize, DVec3)> = if orientation {
            vec![(3, normal), (4, -normal)]
        } else {
            vec![(0, DVec3::X), (1, DVec3::Y), (2, DVec3::Z)]
        };
        let scale = self.camera.distance.max(1e-6) * 0.05;
        dirs.into_iter()
            .filter_map(|(id, dir)| {
                let (tip, _) = self.camera.project(target + dir * scale, r)?;
                let v = tip - base;
                let v = if v.length() < 1e-3 {
                    vec2(0., -1.)
                } else {
                    v.normalized()
                };
                Some((id, base, base + v * ARROW))
            })
            .collect()
    }

    pub(super) fn utility_drag(&mut self, drag: &UtilityDrag, start: Pos2, pos: Pos2, r: Rect) {
        match *drag {
            UtilityDrag::RetopoSketch => self.retopo_sketch_drag(pos, r),
            UtilityDrag::MeshPaint => self.mesh_paint_drag(pos, r),
            UtilityDrag::Guide(start) => self.guide_drag(start, pos, r),
            UtilityDrag::TopologyPen => self.topology_pen_drag(pos, r),
            UtilityDrag::Ruler(handle, base) => {
                let plane = self.actual_plane();
                let anchor = match handle {
                    1 => base.end,
                    _ => base.start,
                };
                let (Some(a), Some(b)) = (
                    self.work_point(start, r, plane, anchor),
                    self.work_point(pos, r, plane, anchor),
                ) else {
                    return;
                };
                let mut ruler = base;
                match handle {
                    2 => {
                        ruler.start += b - a;
                        ruler.end += b - a;
                    }
                    end => {
                        let on_point = self.snap_to_point(pos, r);
                        let mut p = on_point.unwrap_or(anchor + (b - a));
                        let other = if end == 0 { base.end } else { base.start };
                        if ruler.angle_snap && on_point.is_none() {
                            let normal = [DVec3::Y, DVec3::X, DVec3::Z][plane.min(2) as usize];
                            p = other + snap_angle(p - other, normal, 15.);
                        }
                        if end == 0 {
                            ruler.start = p;
                        } else {
                            ruler.end = p;
                        }
                    }
                }
                self.utility = Some(Utility::Ruler(ruler));
                let d = ruler.end - ruler.start;
                self.status = format!(
                    "Ruler · {} · X {} · Y {} · Z {}",
                    workspace::number_text(d.length(), " m"),
                    workspace::number_text(d.x, " m"),
                    workspace::number_text(d.y, " m"),
                    workspace::number_text(d.z, " m")
                );
            }
            UtilityDrag::Protractor(handle, base) => {
                // Arms move in the protractor's own plane; the centre moves
                // on the construction plane.
                let n = base.normal();
                let plane_at = |q: Pos2, through: DVec3| {
                    let (o, d) = self.camera.ray(q, r);
                    let denom = d.dot(n);
                    (denom.abs() > 1e-9).then(|| o + d * ((through - o).dot(n) / denom))
                };
                let mut p = base;
                if handle == 0 {
                    let plane = self.actual_plane();
                    let (Some(a), Some(b)) = (
                        self.work_point(start, r, plane, base.center),
                        self.work_point(pos, r, plane, base.center),
                    ) else {
                        return;
                    };
                    let shift = b - a;
                    p.center += shift;
                    p.start += shift;
                    p.end += shift;
                } else if let Some(q) = plane_at(pos, base.center) {
                    if handle == 1 {
                        p.start = q;
                    } else {
                        p.end = q;
                    }
                }
                self.utility = Some(Utility::Protractor(p));
                self.status = format!("Protractor · {:.4}°", p.angle());
            }
            UtilityDrag::QuickAlign { .. } => {}
        }
    }

    pub(super) fn utility_release(&mut self, drag: &UtilityDrag, pos: Pos2, r: Rect) {
        match *drag {
            UtilityDrag::RetopoSketch => return self.retopo_sketch_release(),
            UtilityDrag::MeshPaint => return self.mesh_paint_release(r),
            UtilityDrag::Guide(_) => return self.guide_release(),
            UtilityDrag::TopologyPen => return self.topology_pen_release(pos, r),
            _ => {}
        }
        if let UtilityDrag::QuickAlign {
            target,
            normal,
            connected,
            flatten,
            orientation,
        } = *drag
        {
            let Some(Utility::QuickAlign(options)) = self.utility else {
                return;
            };
            let chosen = self
                .quick_align_arrows(target, normal, orientation, r)
                .into_iter()
                .filter(|(_, base, tip)| input::segment_distance(pos, *base, *tip) <= GRAB * 1.5)
                .filter(|(_, base, _)| base.distance(pos) > GRAB)
                .min_by(|a, b| {
                    input::segment_distance(pos, a.1, a.2)
                        .total_cmp(&input::segment_distance(pos, b.1, b.2))
                })
                .map(|(id, ..)| id);
            let Some(id) = chosen else {
                self.status = "Quick Align: release over an arrow to apply".into();
                return;
            };
            let kind = match id {
                3 => QuickAlignKind::Orientation {
                    normal,
                    opposite: false,
                },
                4 => QuickAlignKind::Orientation {
                    normal,
                    opposite: true,
                },
                axis => QuickAlignKind::Position(axis),
            };
            self.run(Command::QuickAlign(QuickAlign {
                target,
                kind,
                connected,
                flatten,
                islands: options.islands,
            }));
        }
    }

    /// The nearest visible mesh point within grabbing distance (RUL-02).
    /// Points hidden behind the surface don't count.
    fn snap_to_point(&self, pos: Pos2, r: Rect) -> Option<DVec3> {
        self.snap_scene_point(pos, crate::snap_service::Role::RulerEnd, r, &|_| false)
            .map(|h| h.hit.point)
    }

    /// Dimensions: the measured box as (size, centre, axes), world space.
    /// Nothing selected measures every point of the active mesh (DIM-03).
    pub(super) fn dimension_box(&self) -> Option<(DVec3, DVec3, [DVec3; 3])> {
        let Some(Utility::Dimensions { orient }) = self.utility else {
            return None;
        };
        let item = self.editor.scene.active().filter(|i| i.is_mesh())?;
        let world = self.editor.scene.world_matrix(item.id).ok()?;
        let s = &self.editor.scene.selection;
        let mesh = &item.mesh;
        let mut points = s.points(mesh);
        if points.is_empty() {
            points = (0..mesh.vertices.len() as u32).collect();
        }
        let pts: Vec<DVec3> = points
            .iter()
            .filter_map(|&v| mesh.vertices.get(v as usize))
            .map(|p| world.transform_point3(*p))
            .collect();
        if pts.is_empty() {
            return None;
        }
        let axes = if orient {
            principal_axes(&pts)
        } else {
            [DVec3::X, DVec3::Y, DVec3::Z]
        };
        let mut lo = DVec3::splat(f64::MAX);
        let mut hi = DVec3::splat(f64::MIN);
        for p in &pts {
            let q = DVec3::new(p.dot(axes[0]), p.dot(axes[1]), p.dot(axes[2]));
            lo = lo.min(q);
            hi = hi.max(q);
        }
        let mid = (lo + hi) / 2.;
        let centre = axes[0] * mid.x + axes[1] * mid.y + axes[2] * mid.z;
        Some((hi - lo, centre, axes))
    }

    // ----------------------------------------------------------- drawing

    /// `view_hover`: the pointer while it is over the view itself (none
    /// while it works a panel field), for overlays that follow it.
    pub(super) fn draw_utility(&self, painter: &egui::Painter, r: Rect, view_hover: Option<Pos2>) {
        let Some(utility) = self.utility else {
            return;
        };
        let accent = theme::chrome::active();
        let text = theme::viewport::label();
        let font = FontId::proportional(12.);
        let project = |p: DVec3| self.camera.project(p, r).map(|(q, _)| q);
        let line = |a: DVec3, b: DVec3, width: f32| {
            if let (Some(a), Some(b)) = (project(a), project(b)) {
                painter.line_segment([a, b], Stroke::new(width, accent));
            }
        };
        match utility {
            Utility::Ruler(ruler) => {
                line(ruler.start, ruler.end, 2.);
                let d = ruler.end - ruler.start;
                let length = label_in_units(d.length());
                if let Some(q) = project(ruler.end) {
                    painter.text(
                        q + vec2(10., -10.),
                        Align2::LEFT_BOTTOM,
                        length,
                        font.clone(),
                        text,
                    );
                }
                for p in [ruler.start, ruler.end] {
                    if let Some(q) = project(p) {
                        painter.circle_stroke(q, 5., Stroke::new(1.5f32, accent));
                    }
                }
                // Ticks: ten along the ruler, across it on screen.
                if let (Some(a), Some(b)) = (project(ruler.start), project(ruler.end)) {
                    let along = b - a;
                    if along.length() > 1. {
                        let across = vec2(-along.y, along.x).normalized() * 4.;
                        for k in 1..10 {
                            let p = a + along * (k as f32 / 10.);
                            let s = if k == 5 { 1.8 } else { 1. };
                            painter.line_segment(
                                [p - across * s, p + across * s],
                                Stroke::new(1f32, accent),
                            );
                        }
                    }
                }
            }
            Utility::Protractor(p) => {
                let n = p.normal();
                let radius = (p.start - p.center)
                    .length()
                    .max((p.end - p.center).length())
                    .max(1e-6);
                let u =
                    (p.start - p.center - n * (p.start - p.center).dot(n)).normalize_or(DVec3::X);
                let v = n.cross(u);
                // Dial, counter-clockwise seen from +axis, labels every 45°.
                let at = |deg: f64, scale: f64| {
                    let a = deg.to_radians();
                    p.center + (u * a.cos() - v * a.sin()) * radius * scale
                };
                let mut prev = at(0., 1.);
                for k in 1..=72 {
                    let next = at(k as f64 * 5., 1.);
                    if let (Some(a), Some(b)) = (project(prev), project(next)) {
                        painter.line_segment([a, b], Stroke::new(1f32, accent.gamma_multiply(0.6)));
                    }
                    prev = next;
                }
                for k in 0..8 {
                    let deg = k as f64 * 45.;
                    line(at(deg, 0.92), at(deg, 1.), 1.);
                    if let Some(q) = project(at(deg, 1.1)) {
                        painter.text(
                            q,
                            Align2::CENTER_CENTER,
                            format!("{deg:.0}"),
                            FontId::proportional(10.),
                            text,
                        );
                    }
                }
                line(p.center, p.start, 2.);
                line(p.center, p.end, 2.);
                for q in [p.center, p.start, p.end] {
                    if let Some(q) = project(q) {
                        painter.circle_stroke(q, 5., Stroke::new(1.5f32, accent));
                    }
                }
                if let Some(q) = project(p.center) {
                    painter.text(
                        q + vec2(10., -10.),
                        Align2::LEFT_BOTTOM,
                        format!("{:.2}°", p.angle()),
                        font,
                        text,
                    );
                }
            }
            Utility::Dimensions { .. } => {
                let Some((size, centre, axes)) = self.dimension_box() else {
                    return;
                };
                let half = [
                    axes[0] * size.x / 2.,
                    axes[1] * size.y / 2.,
                    axes[2] * size.z / 2.,
                ];
                let corner =
                    |sx: f64, sy: f64, sz: f64| centre + half[0] * sx + half[1] * sy + half[2] * sz;
                for (a, b) in [
                    ((-1., -1., -1.), (1., -1., -1.)),
                    ((-1., 1., -1.), (1., 1., -1.)),
                    ((-1., -1., 1.), (1., -1., 1.)),
                    ((-1., 1., 1.), (1., 1., 1.)),
                    ((-1., -1., -1.), (-1., 1., -1.)),
                    ((1., -1., -1.), (1., 1., -1.)),
                    ((-1., -1., 1.), (-1., 1., 1.)),
                    ((1., -1., 1.), (1., 1., 1.)),
                    ((-1., -1., -1.), (-1., -1., 1.)),
                    ((1., -1., -1.), (1., -1., 1.)),
                    ((-1., 1., -1.), (-1., 1., 1.)),
                    ((1., 1., -1.), (1., 1., 1.)),
                ] {
                    line(corner(a.0, a.1, a.2), corner(b.0, b.1, b.2), 1.);
                }
                // One size per axis, written along three of the box's edges.
                for (axis, mid, value) in [
                    ("X", corner(0., -1., 1.), size.x),
                    ("Y", corner(1., 0., 1.), size.y),
                    ("Z", corner(1., -1., 0.), size.z),
                ] {
                    if let Some(q) = project(mid) {
                        painter.text(
                            q,
                            Align2::CENTER_CENTER,
                            format!("{axis} {}", label_in_units(value)),
                            font.clone(),
                            text,
                        );
                    }
                }
            }
            Utility::QuickAlign(_) => {
                if let Some(g) = &self.navigation
                    && let input::GestureKind::Utility(UtilityDrag::QuickAlign {
                        target,
                        normal,
                        orientation,
                        ..
                    }) = g.kind
                {
                    let hover = g.last;
                    for (id, base, tip) in self.quick_align_arrows(target, normal, orientation, r) {
                        let colour = match id {
                            0 => Color32::from_rgb(220, 70, 70),
                            1 => Color32::from_rgb(90, 200, 90),
                            2 => Color32::from_rgb(80, 130, 230),
                            _ => accent,
                        };
                        let hot = input::segment_distance(hover, base, tip) <= GRAB * 1.5
                            && base.distance(hover) > GRAB;
                        let width = if hot { 4f32 } else { 2. };
                        painter.arrow(base, tip - base, Stroke::new(width, colour));
                    }
                }
            }
            Utility::RetopoSketch => self.draw_retopo_stroke(painter, r),
            // #46.4: the brush circle only over the view, not over a field
            // whose arrows are being dragged.
            Utility::MeshPaint(_) => self.draw_mesh_paint(painter, view_hover, r),
            Utility::PasteTool(_)
            | Utility::GroundAlign(_)
            | Utility::Tack(_)
            | Utility::Guides(_) => {}
        }
    }
}

/// Measurements show four decimals in the current units (RUL-05).
pub(super) fn label_in_units(metres: f64) -> String {
    let text = operandi_core::units::format_distance(metres);
    // format_distance already uses the user's units; keep it as is.
    text
}

/// Directions of most, middle and least spread of `points` (a fitted box's
/// axes, DIM-02), by Jacobi rotations on the covariance.
fn principal_axes(points: &[DVec3]) -> [DVec3; 3] {
    let n = points.len() as f64;
    let mean = points.iter().copied().sum::<DVec3>() / n;
    let mut c = [[0f64; 3]; 3];
    for p in points {
        let d = *p - mean;
        let d = [d.x, d.y, d.z];
        for i in 0..3 {
            for j in 0..3 {
                c[i][j] += d[i] * d[j] / n;
            }
        }
    }
    let mut v = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..32 {
        let (mut p, mut q, mut big) = (0, 1, 0.);
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            if c[i][j].abs() > big {
                big = c[i][j].abs();
                p = i;
                q = j;
            }
        }
        if big < 1e-15 {
            break;
        }
        let theta = (c[q][q] - c[p][p]) / (2. * c[p][q]);
        let t =
            theta.signum().max(0.).mul_add(2., -1.) / (theta.abs() + (theta * theta + 1.).sqrt());
        let t = if theta == 0. { 1. } else { t };
        let cos = 1. / (t * t + 1.).sqrt();
        let sin = t * cos;
        let mut next = c;
        for k in 0..3 {
            next[k][p] = cos * c[k][p] - sin * c[k][q];
            next[k][q] = sin * c[k][p] + cos * c[k][q];
        }
        let copy = next;
        for k in 0..3 {
            next[p][k] = cos * copy[p][k] - sin * copy[q][k];
            next[q][k] = sin * copy[p][k] + cos * copy[q][k];
        }
        c = next;
        for row in &mut v {
            let (a, b) = (row[p], row[q]);
            row[p] = cos * a - sin * b;
            row[q] = sin * a + cos * b;
        }
    }
    let mut axes: Vec<(f64, DVec3)> = (0..3)
        .map(|k| {
            (
                c[k][k],
                DVec3::new(v[0][k], v[1][k], v[2][k]).normalize_or(DVec3::X),
            )
        })
        .collect();
    axes.sort_by(|a, b| b.0.total_cmp(&a.0));
    let x = axes[0].1;
    let y = axes[1].1;
    let z = x.cross(y).normalize_or(DVec3::Z);
    [x, y, z]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protractor_default_reads_270() {
        assert!((Protractor::default().angle() - 270.).abs() < 1e-9);
    }

    #[test]
    fn angle_snap_rounds_to_15_degrees() {
        let d = snap_angle(DVec3::new(1., 0., 0.3), DVec3::Y, 15.);
        let a = d.z.atan2(d.x).to_degrees();
        assert!((a - 15.).abs() < 1e-9, "{a}");
        assert!((d.length() - DVec3::new(1., 0., 0.3).length()).abs() < 1e-9);
    }

    #[test]
    fn principal_axes_find_a_turned_box() {
        let turn = glam::DQuat::from_rotation_y(0.5);
        let mut pts = Vec::new();
        for x in [-1., 1.] {
            for y in [-0.25, 0.25] {
                for z in [-0.5, 0.5] {
                    pts.push(turn * DVec3::new(x, y, z));
                }
            }
        }
        let axes = principal_axes(&pts);
        assert!(axes[0].dot(turn * DVec3::X).abs() > 0.999);
        assert!(axes[2].dot(DVec3::Y).abs() > 0.999);
    }
}
