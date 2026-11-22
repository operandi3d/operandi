//! One card per tool (architecture review 2026-11-14, pick A): everything
//! the rest of the program asks about a tool, in one place.
//!
//! Every tool is a stack of shared layers, the "tool pipe" (Jason,
//! 2026-11-14): action centre, falloff, symmetry, snapping and the mesh
//! constraint (MC-08), then the tool's own geometry. A card says which layers the tool uses, and how its
//! handles look and where they sit, instead of each layer keeping its own
//! list of tools. The match in [`card`] has no wildcard arm, so a new tool
//! doesn't compile until someone fills in its card.
//!
//! Cards are plain data (`Copy`, built at compile time), so a later tool
//! registry can hold cards for tools that plugins add. The geometry itself
//! (one arm per tool in `Editor::preview_one`) is not on the card yet.
use crate::symmetry_support::SymmetrySupport;
use crate::{Mode, Tool};

/// What kind of tool it is: the toolbox group, and the activation rules
/// its group shares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// Cube, Sphere, Cylinder: drawn on the work plane.
    Primitive,
    /// Move, Rotate, Scale, Transform and Element Move.
    Transform,
    /// Polygon-mode tools (Bevel, Extrude, Inset...).
    Polygon,
    /// Edge-mode tools: need selected edges in Edge mode.
    Edge,
    /// Vertex-mode tools: need selected points in Vertex mode.
    Vertex,
    /// Create: places new points by clicking.
    Create,
    /// Deform tab (Push, Smooth, Bend, Symmetry Tool): component modes only.
    Deform,
    /// Edit tab slice tools.
    Slice,
    /// Duplicate tab tools.
    Duplicate,
    /// UV tab tools (Project, Unwrap, Relax): any selection mode; they
    /// write the current UV map and never move points.
    Uv,
}

/// The selection modes a tool starts in (others refuse it at activation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modes(u8);
impl Modes {
    pub const VERTEX: Modes = Modes(1);
    pub const EDGE: Modes = Modes(2);
    pub const POLYGON: Modes = Modes(4);
    pub const ITEM: Modes = Modes(8);
    pub const COMPONENTS: Modes = Modes(1 | 2 | 4);
    pub const ALL: Modes = Modes(1 | 2 | 4 | 8);
    pub const fn or(self, other: Modes) -> Modes {
        Modes(self.0 | other.0)
    }
    pub const fn contains(self, mode: Mode) -> bool {
        let bit = match mode {
            Mode::Vertex => 1,
            Mode::Edge => 2,
            Mode::Polygon => 4,
            Mode::Item => 8,
        };
        self.0 & bit != 0
    }
    /// The modes in words, for messages: "Polygon mode", "Polygon or
    /// Vertex mode".
    pub fn describe(self) -> String {
        let names: Vec<&str> = [
            (Mode::Polygon, "Polygon"),
            (Mode::Edge, "Edge"),
            (Mode::Vertex, "Vertex"),
            (Mode::Item, "Item"),
        ]
        .into_iter()
        .filter(|&(m, _)| self.contains(m))
        .map(|(_, n)| n)
        .collect();
        format!("{} mode", or_list(&names))
    }
}

/// "a", "a or b", "a, b or c".
fn or_list(names: &[&str]) -> String {
    match names.split_last() {
        Some((last, [])) => (*last).to_owned(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// Pipe layer 0, selection inference (SPEC selection-inference v1.1,
/// SI-01): which element type the tool acts on, so a point or edge tool
/// can start from another selection mode and work out its elements from
/// the current selection (`selection_inference.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Takes {
    /// The tool reads the selection its own way (transforms, deforms,
    /// slices, primitives): no inference.
    Native,
    /// Points: from an Edge or Polygon selection too.
    Points,
    /// Edges: from a Vertex (edges between selected points) or Polygon
    /// (their outline) selection too.
    Edges,
    /// Polygons: never inferred; Vertex and Edge mode refuse (SI-03).
    Polygons,
}

/// Pipe layer 1: how the tool uses the action centre and axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionCentre {
    /// The tool captures an action frame at activation (`Live::action_frame`)
    /// and works about it.
    Frame,
    /// The tool refuses any action context but the default (and a falloff).
    Neutral,
    /// The tool works from its elements' own frames and ignores the setting.
    Unused,
}

/// Pipe layer 4: what of the tool snaps (Snapping Options targets).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Snap {
    /// Nothing the tool places or drags snaps.
    None,
    /// Its move handle (SN-08), and its centre when a press relocates it.
    Handle,
    /// Only its centre, when a press away from the model relocates it (SN-12).
    Centre,
    /// The element it grabs (Tweak).
    Grab,
    /// The points it places on the work plane (Duplicate tools, DS-37).
    Placement,
}

/// Pipe layer 5: how the mesh constraint (Constrain to Background) takes
/// part. It runs after snapping: a snapped point is then pushed onto the
/// background (MC-08). Its values are the editor's for now; a tool preset
/// may carry its own later (MC-10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Constraint {
    /// The points the tool moves land on the background, when the tool
    /// neither adds nor removes points (BG-Q8).
    MovedPoints,
    /// The tool makes new geometry; the constraint leaves it alone.
    Ignores,
}

/// The shared layers a tool runs through, in pipe order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layers {
    pub action_centre: ActionCentre,
    /// Pipe layer 2: a falloff scales the tool's effect (FAL-M-02,
    /// FAL-T-01..03). Without it the tool refuses an active falloff.
    pub falloff: bool,
    /// Pipe layer 3: how the tool takes part in Symmetry.
    pub symmetry: SymmetrySupport,
    pub snap: Snap,
    pub constraint: Constraint,
}

/// The tool's on-screen handle layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleKind {
    /// No handles: the tool works by grabbing, painting, drawing or
    /// clicking.
    None,
    /// The Cube's bound squares, and three axis arrows once drawn.
    Bounds,
    /// Sphere and Cylinder: one square per radius bound.
    RadialBounds,
    /// Three axis arrows.
    Axes,
    /// Three rotation rings.
    Rings,
    /// Transform's layout: arrows 0..3, rings 3..6, scale 6..9.
    Combined,
    /// Two handles: along the normal (blue, 0) and sideways (red, 1).
    Pair,
    /// One blue handle along the normal.
    Normal,
    /// One handle along the tool's own direction.
    Single,
    /// Twist Bevel's spine handles (spec section 4): Shift arrow, Inset
    /// and twist ring at the cap, bend arrow and Bend Direction ring at
    /// the base.
    Spine,
}
impl HandleKind {
    /// How many numbered axis handles the general layout draws.
    pub const fn axis_count(self) -> usize {
        match self {
            HandleKind::Normal | HandleKind::Single => 1,
            HandleKind::Pair => 2,
            _ => 3,
        }
    }
}

/// Where the handles sit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleAnchor {
    /// The tool has no handles.
    None,
    /// The action centre (moved by the tool's offset for Move-like tools).
    ActionCentre,
    /// The tool's drawn position (primitives).
    Position,
    /// The polygon stage's frame (Bevel, Extrude, Smooth Shift).
    PolygonStage,
    /// The first selected polygon's centre.
    FirstPolygon,
    /// The first Chamfer strip polygon's centre.
    ChamferStrip,
    /// The handle edge's midpoint: the selected edge nearest the
    /// selection's centre ([`crate::edge_handle_frame`], ACS-Q-HANDLE1).
    Edge,
    /// The handle edge's midpoint, on its slide rail
    /// ([`crate::edge_slide_handle_frame`]).
    EdgeRail,
    /// The handle point: the selected point nearest the selection's
    /// centre ([`crate::vertex_handle_frame`]).
    Vertex,
}
impl HandleAnchor {
    /// The anchor is found in the item's own space and drawn mapped to world.
    pub const fn item_local(self) -> bool {
        matches!(
            self,
            HandleAnchor::FirstPolygon | HandleAnchor::ChamferStrip
        )
    }
}

/// Which way the numbered handles point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleAxes {
    /// The tool's own rule, or the action axes.
    Tool,
    /// The first point's rail (to its first neighbour).
    VertexRail,
    /// Handle 0 along the first point's normal, the others along its rail.
    VertexNormal,
    /// The first edge's slide rail.
    EdgeRail,
    /// Into the first edge's first polygon (Width).
    EdgeSide,
    /// Handle 0 along the first edge's normal, the others into its polygon.
    EdgeNormal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handles {
    pub kind: HandleKind,
    pub anchor: HandleAnchor,
    pub axes: HandleAxes,
    /// Move's and Scale's plane circles (PH-30/31).
    pub plane_circles: bool,
    /// A drag away from the handles drives the tool (up/down its first
    /// handle, left/right its second); without it the tool has a gesture of
    /// its own or places points by clicking.
    pub drag_anywhere: bool,
}

/// Everything shared code asks about a tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolCard {
    pub tool: Tool,
    pub family: Family,
    /// The modes it starts in, the inferred ones included (SI §7).
    pub modes: Modes,
    /// What it acts on; the shared inference step reads this.
    pub takes: Takes,
    pub layers: Layers,
    pub handles: Handles,
    /// With several active meshes the tool runs on each (multi_tools.rs).
    pub multi_mesh: bool,
    /// Moves whole items in Item mode (and backdrops, cameras and lights).
    pub transforms_items: bool,
    /// Each drag is its own undo step and Undo/Redo keep the tool active.
    pub undo_per_drag: bool,
    /// Shift-click (Continue) starts a new stage on the result.
    pub continues: bool,
}

// A small const builder, so each card below reads as one row.
impl ToolCard {
    const fn new(tool: Tool, family: Family, modes: Modes) -> Self {
        Self {
            tool,
            family,
            modes,
            takes: Takes::Native,
            layers: Layers {
                action_centre: ActionCentre::Unused,
                falloff: false,
                symmetry: SymmetrySupport::Ignores,
                snap: Snap::None,
                constraint: Constraint::MovedPoints,
            },
            handles: Handles {
                kind: HandleKind::None,
                anchor: HandleAnchor::None,
                axes: HandleAxes::Tool,
                plane_circles: false,
                drag_anywhere: false,
            },
            multi_mesh: false,
            transforms_items: false,
            undo_per_drag: false,
            continues: false,
        }
    }
    const fn layers(
        mut self,
        action_centre: ActionCentre,
        falloff: bool,
        symmetry: SymmetrySupport,
        snap: Snap,
    ) -> Self {
        self.layers = Layers {
            action_centre,
            falloff,
            symmetry,
            snap,
            // Primitives and Duplicate tools make new geometry.
            constraint: match self.family {
                Family::Primitive | Family::Duplicate => Constraint::Ignores,
                _ => Constraint::MovedPoints,
            },
        };
        self
    }
    const fn handles(mut self, kind: HandleKind, anchor: HandleAnchor, axes: HandleAxes) -> Self {
        self.handles.kind = kind;
        self.handles.anchor = anchor;
        self.handles.axes = axes;
        self
    }
    const fn plane_circles(mut self) -> Self {
        self.handles.plane_circles = true;
        self
    }
    const fn drag_anywhere(mut self) -> Self {
        self.handles.drag_anywhere = true;
        self
    }
    const fn multi_mesh(mut self) -> Self {
        self.multi_mesh = true;
        self
    }
    const fn transforms_items(mut self) -> Self {
        self.transforms_items = true;
        self
    }
    const fn undo_per_drag(mut self) -> Self {
        self.undo_per_drag = true;
        self
    }
    const fn continues(mut self) -> Self {
        self.continues = true;
        self
    }
    /// The tool acts on `takes`: it also starts from the modes that
    /// selection inference converts from (SI-01).
    const fn takes(mut self, takes: Takes) -> Self {
        self.takes = takes;
        self.modes = match takes {
            Takes::Points | Takes::Edges => self.modes.or(Modes::COMPONENTS),
            Takes::Native | Takes::Polygons => self.modes,
        };
        self
    }
}

/// The card of `tool`.
pub fn card(tool: Tool) -> ToolCard {
    use ActionCentre::{Frame, Neutral, Unused};
    use Family as F;
    use HandleAnchor as At;
    use HandleAxes as Ax;
    use HandleKind as K;
    use SymmetrySupport::{Built, ViaSelection};
    const FALLOFF: bool = true;
    const NO_FALLOFF: bool = false;
    let c = ToolCard::new;
    // A tool that doesn't support Symmetry yet says `SymmetrySupport::Ignores`
    // (SYM3-40); every current tool does.
    match tool {
        // ---- Primitives: Duplicate policy for Symmetry (class E).
        // Placed corners and handles snap (SN2-41).
        Tool::Box => c(tool, F::Primitive, Modes::ALL)
            .layers(Unused, NO_FALLOFF, Built, Snap::Placement)
            .handles(K::Bounds, At::Position, Ax::Tool),
        Tool::Sphere | Tool::Cylinder => c(tool, F::Primitive, Modes::ALL)
            .layers(Unused, NO_FALLOFF, Built, Snap::Placement)
            .handles(K::RadialBounds, At::Position, Ax::Tool),

        // ---- Transforms: mirrored edit, handle on the picked side (class A).
        Tool::Move => c(tool, F::Transform, Modes::ALL)
            .layers(Frame, FALLOFF, Built, Snap::Handle)
            .handles(K::Axes, At::ActionCentre, Ax::Tool)
            .plane_circles()
            .transforms_items()
            .undo_per_drag(),
        Tool::Rotate => c(tool, F::Transform, Modes::ALL)
            .layers(Frame, FALLOFF, Built, Snap::Centre)
            .handles(K::Rings, At::ActionCentre, Ax::Tool)
            .transforms_items()
            .undo_per_drag(),
        Tool::Scale => c(tool, F::Transform, Modes::ALL)
            .layers(Frame, FALLOFF, Built, Snap::Centre)
            .handles(K::Axes, At::ActionCentre, Ax::Tool)
            .plane_circles()
            .transforms_items()
            .undo_per_drag(),
        Tool::Transform => c(tool, F::Transform, Modes::ALL)
            .layers(Frame, FALLOFF, Built, Snap::Handle)
            .handles(K::Combined, At::ActionCentre, Ax::Tool)
            .plane_circles()
            .transforms_items()
            .undo_per_drag(),
        // Element Move grabs elements directly; its click-to-show handle is TWK-3.
        Tool::Tweak => c(tool, F::Transform, Modes::ALL)
            .layers(Unused, NO_FALLOFF, Built, Snap::Grab)
            .undo_per_drag(),

        // ---- Polygon tools.
        Tool::Bevel => c(tool, F::Polygon, Modes::POLYGON)
            .takes(Takes::Polygons)
            .layers(Frame, FALLOFF, ViaSelection, Snap::None)
            .handles(K::Pair, At::PolygonStage, Ax::Tool)
            .drag_anywhere()
            .multi_mesh()
            .continues(),
        // Twist Bevel: each tube uses its own frame; the action axis only
        // for Rotation Axis = Action Axis. Its symmetry mirrors bend and
        // twist itself (spec section 5); only the Shift arrow snaps.
        Tool::TwistBevel => c(tool, F::Polygon, Modes::POLYGON)
            .takes(Takes::Polygons)
            .layers(Frame, FALLOFF, Built, Snap::Handle)
            .handles(K::Spine, At::PolygonStage, Ax::Tool)
            .drag_anywhere()
            .continues(),
        Tool::Chamfer => c(tool, F::Polygon, Modes::POLYGON)
            .takes(Takes::Polygons)
            .layers(Neutral, NO_FALLOFF, ViaSelection, Snap::None)
            .handles(K::Normal, At::ChamferStrip, Ax::Tool)
            .drag_anywhere()
            .multi_mesh(),
        // Poly Haul hauls the patch under the pointer; Sketch Extrude
        // follows a drawn path. Class C, phase 2 for Symmetry.
        Tool::PolyHaul => c(tool, F::Polygon, Modes::POLYGON.or(Modes::VERTEX)).layers(
            Neutral,
            NO_FALLOFF,
            Built,
            Snap::None,
        ),
        Tool::SketchExtrude => c(tool, F::Polygon, Modes::POLYGON)
            .takes(Takes::Polygons)
            .layers(Neutral, NO_FALLOFF, Built, Snap::None),
        // Extrude's falloff rule is still open (FAL-Q04). Class C wired.
        Tool::Extrude => c(tool, F::Polygon, Modes::POLYGON)
            .takes(Takes::Polygons)
            .layers(Frame, NO_FALLOFF, Built, Snap::None)
            .handles(K::Axes, At::PolygonStage, Ax::Tool)
            .multi_mesh()
            .continues(),
        // Smooth Shift and Thicken under a falloff weren't observed (FAL-Q06).
        Tool::SmoothShift | Tool::Thicken => c(tool, F::Polygon, Modes::POLYGON)
            .takes(Takes::Polygons)
            .layers(Frame, NO_FALLOFF, ViaSelection, Snap::None)
            .handles(K::Pair, At::PolygonStage, Ax::Tool)
            .drag_anywhere()
            .multi_mesh()
            .continues(),
        Tool::Inset | Tool::Shift => c(tool, F::Polygon, Modes::POLYGON)
            .takes(Takes::Polygons)
            .layers(Unused, NO_FALLOFF, ViaSelection, Snap::None)
            .handles(K::Single, At::FirstPolygon, Ax::Tool)
            .drag_anywhere()
            .multi_mesh(),

        // ---- Edge tools.
        // Extend's Rotate and Scale aren't mirrored yet (provisional).
        Tool::EdgeExtend => c(tool, F::Edge, Modes::EDGE)
            .takes(Takes::Edges)
            .layers(Frame, NO_FALLOFF, Built, Snap::None)
            .handles(K::Combined, At::ActionCentre, Ax::Tool)
            .drag_anywhere()
            .multi_mesh()
            .continues(),
        // Pushes along each edge's own normal: symmetric by selection.
        Tool::EdgeExtrude => c(tool, F::Edge, Modes::EDGE)
            .takes(Takes::Edges)
            .layers(Unused, NO_FALLOFF, ViaSelection, Snap::None)
            .handles(K::Pair, At::Edge, Ax::EdgeNormal)
            .drag_anywhere()
            .multi_mesh(),
        Tool::EdgeSlide => c(tool, F::Edge, Modes::EDGE)
            .takes(Takes::Edges)
            .layers(Unused, FALLOFF, Built, Snap::None)
            .handles(K::Single, At::EdgeRail, Ax::EdgeRail)
            .drag_anywhere()
            .multi_mesh(),
        Tool::EdgeCrease => c(tool, F::Edge, Modes::EDGE)
            .takes(Takes::Edges)
            .layers(Unused, NO_FALLOFF, ViaSelection, Snap::None)
            .handles(K::Single, At::Edge, Ax::EdgeNormal)
            .drag_anywhere()
            .multi_mesh(),
        Tool::EdgeBevel | Tool::EdgeChamfer => c(tool, F::Edge, Modes::EDGE)
            .takes(Takes::Edges)
            .layers(Unused, FALLOFF, ViaSelection, Snap::None)
            .handles(K::Single, At::Edge, Ax::EdgeSide)
            .drag_anywhere()
            .multi_mesh(),
        // No handle: drag anywhere (ACS-Q-HANDLE1).
        Tool::EdgeRelax => c(tool, F::Edge, Modes::EDGE)
            .takes(Takes::Edges)
            .layers(Unused, NO_FALLOFF, ViaSelection, Snap::None)
            // Its drag still reads the edge-side direction.
            .handles(K::None, At::None, Ax::EdgeSide)
            .drag_anywhere()
            .multi_mesh(),

        // ---- Vertex tools.
        // Placed points get mirror points, welded on the seam (class F).
        // Placed points snap (VX-CR02).
        Tool::VertexCreate => {
            c(tool, F::Create, Modes::VERTEX).layers(Unused, NO_FALLOFF, Built, Snap::Placement)
        }
        // Vertex Bevel's plain drag is its own gesture; no handle is drawn
        // (ACS-Q-HANDLE1).
        Tool::VertexBevel => c(tool, F::Vertex, Modes::VERTEX)
            .takes(Takes::Points)
            .layers(Unused, FALLOFF, ViaSelection, Snap::None)
            .handles(K::None, At::None, Ax::VertexRail)
            .multi_mesh(),
        Tool::VertexExtrude => c(tool, F::Vertex, Modes::VERTEX)
            .takes(Takes::Points)
            .layers(Unused, FALLOFF, ViaSelection, Snap::None)
            .handles(K::Pair, At::Vertex, Ax::VertexNormal)
            .drag_anywhere()
            .multi_mesh(),
        Tool::VertexMerge => c(tool, F::Vertex, Modes::VERTEX)
            .takes(Takes::Points)
            .layers(Unused, NO_FALLOFF, ViaSelection, Snap::None)
            .handles(K::Single, At::Vertex, Ax::VertexRail)
            .drag_anywhere()
            .multi_mesh(),
        // Tear's drag is mirrored for the other side (class C, phase 2).
        Tool::VertexTear => c(tool, F::Vertex, Modes::VERTEX)
            .layers(Unused, NO_FALLOFF, Built, Snap::None)
            .handles(K::Axes, At::Vertex, Ax::Tool)
            .drag_anywhere()
            .multi_mesh(),
        Tool::VertexSlide => c(tool, F::Vertex, Modes::VERTEX)
            .takes(Takes::Points)
            .layers(Unused, NO_FALLOFF, Built, Snap::None)
            .handles(K::Single, At::Vertex, Ax::VertexRail)
            .drag_anywhere()
            .multi_mesh(),

        // ---- Deform tab: painted or dragged anywhere, no handles.
        Tool::Push | Tool::Smooth => c(tool, F::Deform, Modes::COMPONENTS)
            .layers(Unused, FALLOFF, ViaSelection, Snap::None)
            .drag_anywhere()
            .multi_mesh(),
        // Deform finish (deform_finish.rs): the primary side's result is
        // mirrored for the other (class C), so random offsets and aligned
        // runs match across the plane.
        Tool::Jitter | Tool::Quantize | Tool::RadialAlign => c(tool, F::Deform, Modes::COMPONENTS)
            .layers(Unused, FALLOFF, Built, Snap::None)
            .drag_anywhere()
            .multi_mesh(),
        // A polygon border is a closed loop, which has no ends to line up
        // between (DFF-Q06), so Linear Align takes points and edges.
        Tool::LinearAlign => c(tool, F::Deform, Modes::VERTEX.or(Modes::EDGE))
            .layers(Unused, FALLOFF, Built, Snap::None)
            .drag_anywhere()
            .multi_mesh(),
        // ---- UV tab (E2/E3/E4): they write UVs, never points, so 3D
        // Symmetry has nothing to mirror; a symmetric selection projects
        // both sides (class B). Unwrap and Relax read the falloff as
        // weights on the relax amount (UVT-21).
        Tool::UvProject | Tool::UvViewProject | Tool::UvPeeler => {
            c(tool, F::Uv, Modes::ALL).layers(Unused, NO_FALLOFF, ViaSelection, Snap::None)
        }
        Tool::UvUnwrap => c(tool, F::Uv, Modes::ALL)
            .layers(Unused, NO_FALLOFF, ViaSelection, Snap::None)
            .drag_anywhere(),
        // UVT-43: UV-only; the UV Symmetry layer mirrors it (uv_pipe.rs).
        // Each drag ends a step of the session, as Move's do, so the next
        // drag's box is the moved UVs'.
        Tool::UvBoxTransform => c(tool, F::Uv, Modes::ALL)
            .layers(Unused, FALLOFF, ViaSelection, Snap::None)
            .drag_anywhere()
            .undo_per_drag(),
        Tool::UvRelax | Tool::UvPelt => c(tool, F::Uv, Modes::ALL)
            .layers(Unused, FALLOFF, ViaSelection, Snap::None)
            .drag_anywhere(),
        // Bend's spine is mirrored for the other side (class C, phase 2).
        // v2 (BN-02/30/31): it bends about the action centre, which a press
        // off its handles moves through the snapping layer.
        Tool::Bend => c(tool, F::Deform, Modes::COMPONENTS)
            .layers(Frame, FALLOFF, Built, Snap::Centre)
            .multi_mesh(),
        // The Symmetry setting's own repair tool (class G).
        Tool::SymmetryTool => c(tool, F::Deform, Modes::COMPONENTS)
            .layers(Unused, NO_FALLOFF, Built, Snap::None)
            .drag_anywhere(),

        // ---- Slice tools: the mirror ring, path or cut is cut too (class D).
        // They draw and drag their own handles (slice_view.rs), so their
        // cards have none: only transform tools get the Move gizmo, and a
        // highlighted handle must do what its tip says (UIC-05).
        Tool::LoopSlice | Tool::EdgeSlice => {
            c(tool, F::Slice, Modes::ALL).layers(Unused, NO_FALLOFF, Built, Snap::None)
        }
        Tool::PolygonSlice | Tool::CurveSlice | Tool::PrimitiveSlice | Tool::AxisSlice => {
            c(tool, F::Slice, Modes::ALL)
                .layers(Unused, NO_FALLOFF, Built, Snap::None)
                .multi_mesh()
        }
        // Contour (missing-tools-g4): adds new curve polygons to the active
        // mesh, cut from the background; no handles yet.
        Tool::Contour => {
            c(tool, F::Slice, Modes::ALL).layers(Unused, NO_FALLOFF, Built, Snap::None)
        }
        // Its points snap like the Pen's (SN2-34..37).
        Tool::PenSlice => c(tool, F::Slice, Modes::ALL)
            .layers(Unused, NO_FALLOFF, Built, Snap::Placement)
            .multi_mesh(),

        // ---- Duplicate tools: the other side gets the mirrored placement
        // (class C); Mirror is its own tool and ignores the setting (class G).
        // Like the Slice tools they draw their own handles
        // (duplicate_view.rs), so the cards have none (UIC-05).
        Tool::Mirror | Tool::Array => c(tool, F::Duplicate, Modes::ALL)
            .layers(Unused, NO_FALLOFF, Built, Snap::None)
            .continues(),
        Tool::Lathe | Tool::Clone | Tool::RadialArray => c(tool, F::Duplicate, Modes::ALL)
            .layers(Unused, NO_FALLOFF, Built, Snap::Placement)
            .continues(),
        // Scatter Clone: copies in a box, no handles yet (GO-01..07).
        Tool::ScatterClone => c(tool, F::Duplicate, Modes::ALL)
            .layers(Unused, NO_FALLOFF, Built, Snap::None)
            .continues(),
        // Symmetrize (DX-40..45): its own plane, whole mesh; ignores the
        // Symmetry setting (class G) and has no handles.
        Tool::Symmetrize => {
            c(tool, F::Duplicate, Modes::ALL).layers(Unused, NO_FALLOFF, Built, Snap::None)
        }
        // ---- Path tools (CX-xx): the other side follows the mirrored path
        // (class C); path points are placed through the snapping layer.
        Tool::CurveExtrude => c(tool, F::Duplicate, Modes::POLYGON)
            .takes(Takes::Polygons)
            .layers(Unused, NO_FALLOFF, Built, Snap::Placement),
        Tool::CurveClone => c(tool, F::Duplicate, Modes::COMPONENTS).layers(
            Unused,
            NO_FALLOFF,
            Built,
            Snap::Placement,
        ),
    }
}

impl Tool {
    /// This tool's card.
    pub fn card(self) -> ToolCard {
        card(self)
    }
}

/// Every tool's card, in toolbox order.
pub fn cards() -> impl Iterator<Item = ToolCard> {
    Tool::ALL.into_iter().map(card)
}

/// What a tool without the falloff layer says when a falloff is active,
/// naming the tools that have it.
pub fn falloff_refusal() -> String {
    let names: Vec<&str> = cards()
        .filter(|c| c.layers.falloff)
        .map(|c| c.tool.label())
        .collect();
    format!(
        "This tool doesn't work with a falloff yet; set Falloff to None, or use {}",
        or_list(&names)
    )
}
