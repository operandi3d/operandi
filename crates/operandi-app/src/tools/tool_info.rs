//! One entry per [`Tool`]: what the app shows for it (icon, tooltip) and how
//! the command line names it. Add a tool here and its toolbox button,
//! `tool.set` ID and typed names all follow. Its behavior (geometry, handles,
//! panel fields) lives with the tool, not here.
use super::*;

pub(super) struct ToolInfo {
    pub tool: Tool,
    pub icon: icons::Icon,
    /// Toolbox tooltip, before the shared shortcut/drop text.
    pub tooltip: &'static str,
    /// `tool.set` ID; must be a row of the command table in `command.rs`.
    pub command_id: &'static str,
    /// Names typed in the command line that start the tool (first is canonical).
    pub names: &'static [&'static str],
}

pub(super) const TOOLS: [ToolInfo; 63] = [
    ToolInfo {
        tool: Tool::Box,
        icon: icons::Icon::Cube,
        tooltip: "Draw a live Cube · Ctrl before drawing: start is center, cursor is outer extent (1 m drag = 2 m size)",
        command_id: "prim.cube",
        names: &["cube", "box"],
    },
    ToolInfo {
        tool: Tool::Sphere,
        icon: icons::Icon::Sphere,
        tooltip: "Face Globe · ordinary draw: flat draft; Ctrl before drawing: start is center, cursor sets equal radii (1 m drag = 2 m diameter)",
        command_id: "prim.sphere",
        names: &["sphere"],
    },
    ToolInfo {
        tool: Tool::Cylinder,
        icon: icons::Icon::Cylinder,
        tooltip: "Face Cylinder · Ctrl before drawing: start is center, cursor sets radius; diameter and height = twice drag",
        command_id: "prim.cylinder",
        names: &["cylinder"],
    },
    ToolInfo {
        tool: Tool::Bevel,
        icon: icons::Icon::Bevel,
        tooltip: "Offset and inset polygon regions; Group Polygons shares cap topology",
        command_id: "poly.bevel",
        names: &["bevel"],
    },
    ToolInfo {
        tool: Tool::TwistBevel,
        icon: icons::Icon::TwistBevel,
        tooltip: "Bevel polygons out into a tube of rings that can taper, twist and bend",
        command_id: "poly.twistBevel",
        names: &["twist bevel"],
    },
    ToolInfo {
        tool: Tool::Chamfer,
        icon: icons::Icon::Chamfer,
        tooltip: "Edit one quad chamfer strip with a rounded profile",
        command_id: "chamfer.edit",
        names: &["chamfer edit"],
    },
    ToolInfo {
        tool: Tool::PolyHaul,
        icon: icons::Icon::PolyHaul,
        tooltip: "Drag over selected polygons to push or pull them along their normal",
        command_id: "poly.haul",
        names: &["poly haul"],
    },
    ToolInfo {
        tool: Tool::Inset,
        icon: icons::Icon::Inset,
        tooltip: "Frame a smaller copy inside each selected polygon (all when none are); negative grows",
        command_id: "poly.inset",
        names: &["inset"],
    },
    ToolInfo {
        tool: Tool::Shift,
        icon: icons::Icon::Shift,
        tooltip: "Move each selected polygon along its own normal on new points (all when none are selected)",
        command_id: "poly.shift",
        names: &["shift"],
    },
    ToolInfo {
        tool: Tool::SketchExtrude,
        icon: icons::Icon::SketchExtrude,
        tooltip: "Draw a path to sweep the selected polygons along",
        command_id: "poly.sweep",
        names: &["sketch extrude"],
    },
    ToolInfo {
        tool: Tool::Extrude,
        icon: icons::Icon::Extrude,
        tooltip: "Offset a cap and connect four walls",
        command_id: "poly.extrude",
        names: &["extrude"],
    },
    ToolInfo {
        tool: Tool::SmoothShift,
        icon: icons::Icon::SmoothShift,
        tooltip: "Offset and scale a polygon region with corner smoothing and shell options",
        command_id: "poly.smshift",
        names: &["smooth shift", "poly.smshift"],
    },
    ToolInfo {
        tool: Tool::Thicken,
        icon: icons::Icon::Thicken,
        tooltip: "Offset and scale a polygon region with corner smoothing and shell options",
        command_id: "Thicken",
        names: &["thicken"],
    },
    ToolInfo {
        tool: Tool::Transform,
        icon: icons::Icon::Transform,
        tooltip: "Combined Move, Rotate and Scale · world origin and axes · one live task",
        command_id: "Transform",
        names: &["transform"],
    },
    ToolInfo {
        tool: Tool::Move,
        icon: icons::Icon::Move,
        tooltip: "Translate components or item",
        command_id: "TransformMove",
        names: &["move"],
    },
    ToolInfo {
        tool: Tool::Rotate,
        icon: icons::Icon::Rotate,
        tooltip: "Rotate about world origin, XYZ degrees",
        command_id: "TransformRotate",
        names: &["rotate"],
    },
    ToolInfo {
        tool: Tool::Scale,
        icon: icons::Icon::Scale,
        tooltip: "Scale about world origin, XYZ factors",
        command_id: "TransformScale",
        names: &["scale"],
    },
    ToolInfo {
        tool: Tool::EdgeExtend,
        icon: icons::Icon::EdgeExtend,
        tooltip: "Extend selected boundary edges into a new row of polygons · drag the X/Y/Z handles",
        command_id: "edge.extend",
        names: &["extend"],
    },
    ToolInfo {
        tool: Tool::EdgeExtrude,
        icon: icons::Icon::EdgeExtrude,
        tooltip: "Raise selected edges along their neighbours' normal; neighbours shrink by Width",
        command_id: "edge.extrude",
        names: &["edge extrude"],
    },
    ToolInfo {
        tool: Tool::EdgeSlide,
        icon: icons::Icon::EdgeSlide,
        tooltip: "Slide selected edges along their connected edges · drag the handle",
        command_id: "edge.slide",
        names: &["slide"],
    },
    // Placeholder: the icon set has no Edge Crease
    // artwork yet.
    ToolInfo {
        tool: Tool::EdgeCrease,
        icon: icons::Icon::EdgeCrease,
        tooltip: "Set the subdivision weight of the selected edges (0 removes it; positive pulls the surface toward the edge, negative pushes it away on Tab surfaces; the effect stops at ±100%) · drag the handle · weighted edges are drawn in Edge mode",
        command_id: "edge.crease",
        names: &["edge weight", "edge crease"],
    },
    // Edge Chamfer reuses the Edge Bevel art.
    ToolInfo {
        tool: Tool::EdgeBevel,
        icon: icons::Icon::EdgeBevel,
        tooltip: "Widen selected edges into a strip; neighbours shrink by Inset · Segments rounds it",
        command_id: "edge.bevel",
        names: &["edge bevel"],
    },
    // Edge Chamfer reuses the Edge Bevel art.
    ToolInfo {
        tool: Tool::EdgeChamfer,
        icon: icons::Icon::EdgeChamfer,
        tooltip: "Round selected edges with new edges offset in parallel · Segments round it",
        command_id: "edge.chamfer",
        names: &["edge chamfer"],
    },
    ToolInfo {
        tool: Tool::EdgeRelax,
        icon: icons::Icon::EdgeRelax,
        tooltip: "Change the radius of a bevel: select its edges and one on either side, drag Offset",
        command_id: "edge.relax",
        names: &["edge relax"],
    },
    ToolInfo {
        tool: Tool::VertexCreate,
        icon: icons::Icon::VertexCreate,
        tooltip: "Click on the work plane to add points · X/Y/Z edit the last point · Drop keeps them · tool.set prim.makeVertex on",
        command_id: "prim.makeVertex",
        names: &["create"],
    },
    ToolInfo {
        tool: Tool::VertexBevel,
        icon: icons::Icon::VertexBevel,
        tooltip: "Replace each selected point by one point per edge at Inset, joined by a new polygon · tool.set vert.bevel on",
        command_id: "vert.bevel",
        names: &["vertex bevel"],
    },
    ToolInfo {
        tool: Tool::VertexExtrude,
        icon: icons::Icon::VertexExtrude,
        tooltip: "Raise each selected point along its normal inside a ring of new edges at Width · tool.set vert.extrude on",
        command_id: "vert.extrude",
        names: &["vertex extrude"],
    },
    // Placeholders: Merge Tool, Tear and Slide have no admitted
    // artwork yet (artwork gaps logged in the M3 acceptance packet).
    ToolInfo {
        tool: Tool::VertexMerge,
        icon: icons::Icon::VertexMerge,
        tooltip: "Drag Distance: selected points within it merge · tool.set vert.merge on",
        command_id: "vert.merge",
        names: &["merge tool"],
    },
    // Placeholders: Merge Tool, Tear and Slide have no admitted
    // artwork yet (artwork gaps logged in the M3 acceptance packet).
    ToolInfo {
        tool: Tool::VertexTear,
        icon: icons::Icon::VertexTear,
        tooltip: "Select polygons and a point on them, then drag: the selected polygons' corner tears off with its own point · tool.set vert.tear on",
        command_id: "vert.tear",
        names: &["tear"],
    },
    // Placeholders: Merge Tool, Tear and Slide have no admitted
    // artwork yet (artwork gaps logged in the M3 acceptance packet).
    ToolInfo {
        tool: Tool::VertexSlide,
        icon: icons::Icon::VertexSlide,
        tooltip: "Slide selected points along their connected edges · tool.set vert.slide on",
        command_id: "vert.slide",
        names: &["vertex slide"],
    },
    // Tweak is the Deform tab's Element Move row.
    ToolInfo {
        tool: Tool::Tweak,
        icon: icons::Icon::ElementMove,
        tooltip: "Element Move: drag a vertex, edge or polygon straight off the mesh, no selection needed",
        command_id: "ElementMove",
        names: &["element move", "tweak"],
    },
    ToolInfo {
        tool: Tool::Push,
        icon: icons::Icon::Push,
        tooltip: "Drag up or down: every selected point moves along its normal by Distance (whole mesh with nothing selected)",
        command_id: "xfrm.push",
        names: &["push"],
    },
    ToolInfo {
        tool: Tool::Smooth,
        icon: icons::Icon::Smooth,
        tooltip: "Evens out bumps: pulls each selected point toward its neighbours; drag left or right to change Strength",
        command_id: "xfrm.smooth",
        names: &["smooth"],
    },
    ToolInfo {
        tool: Tool::Jitter,
        icon: icons::Icon::Jitter,
        tooltip: "Moves each selected point by a random offset within Range X/Y/Z (whole mesh with nothing selected); the same Seed gives the same result. Drag right for more Range",
        command_id: "xfrm.jitter",
        names: &["jitter"],
    },
    ToolInfo {
        tool: Tool::Quantize,
        icon: icons::Icon::Quantize,
        tooltip: "Snaps each selected point to the nearest multiple of Step X/Y/Z, measured from the world origin. Drag right for a bigger step",
        command_id: "xfrm.quantize",
        names: &["quantize"],
    },
    ToolInfo {
        tool: Tool::RadialAlign,
        icon: icons::Icon::RadialAlign,
        tooltip: "Puts the selected edge loop, points or polygon border on a circle, arc or regular polygon; drag left or right to change Weight",
        command_id: "xfrm.radialAlign",
        names: &["radial align", "radial"],
    },
    ToolInfo {
        tool: Tool::LinearAlign,
        icon: icons::Icon::LinearAlign,
        tooltip: "Lines up each selected run of edges or points between its two end points; drag left or right to change Weight",
        command_id: "xfrm.linearAlign",
        names: &["linear align", "linear"],
    },
    ToolInfo {
        tool: Tool::Bend,
        icon: icons::Icon::Bend,
        tooltip: "Curls the selection into an arc about the action centre: points further along the spine turn further, up to Angle at its end. Drag the ring to bend; drag the spine's end to set the bent region; click elsewhere to move the centre",
        command_id: "xfrm.bend",
        names: &["bend"],
    },
    ToolInfo {
        tool: Tool::SymmetryTool,
        icon: icons::Icon::Symmetry,
        tooltip: "Fixes a nearly symmetric mesh: drag right to raise Threshold; points whose mirror partner is within it snap to match. Points still without a partner show as yellow squares",
        command_id: "operandi.symmetry.tool",
        names: &["symmetry tool"],
    },
    // Edit tab Slice tools.
    ToolInfo {
        tool: Tool::LoopSlice,
        icon: icons::Icon::LoopSlice,
        tooltip: "Cut new edge loops across a ring of polygons · select an edge or two neighbouring polygons, or click an edge · drag the slider bar",
        command_id: "poly.loopSlice",
        names: &["loop slice"],
    },
    ToolInfo {
        tool: Tool::EdgeSlice,
        icon: icons::Icon::EdgeSlice,
        tooltip: "Click from edge to edge to cut a path across polygons · click the first point again to close",
        command_id: "edge.knife",
        names: &["edge slice"],
    },
    ToolInfo {
        tool: Tool::PolygonSlice,
        icon: icons::Icon::PolygonSlice,
        tooltip: "Drag a line to cut the selected polygons (all when none are selected) with a plane through it",
        command_id: "poly.knife",
        names: &["polygon slice"],
    },
    // Edit tab waves 2 and 3 (icon batch 11, path_slice_icons.py).
    ToolInfo {
        tool: Tool::CurveSlice,
        icon: icons::Icon::CurveSlice,
        tooltip: "Click points to draw a curve; it cuts the mesh where it projects along the Axis (Y to start)",
        command_id: "Curve Slice",
        names: &["curve slice"],
    },
    ToolInfo {
        tool: Tool::PenSlice,
        icon: icons::Icon::PenSlice,
        tooltip: "Click points to draw a straight-sided path; it cuts the mesh where it projects along the Axis (Y to start)",
        command_id: "PenKnife",
        names: &["pen slice", "penknife"],
    },
    ToolInfo {
        tool: Tool::PrimitiveSlice,
        icon: icons::Icon::PrimitiveSlice,
        tooltip: "Click a face and drag to place a rectangle or circle; it cuts into the face along its normal",
        command_id: "Primitive Slice",
        names: &["primitive slice"],
    },
    ToolInfo {
        tool: Tool::AxisSlice,
        icon: icons::Icon::AxisSlice,
        tooltip: "Straight cuts across the selected polygons (all when none are selected) along X, Y and Z, spread over their bounding box",
        command_id: "poly.julienne",
        names: &["polygon axis slice", "axis slice"],
    },
    // Topology tab Contour (missing-tools-g4), its rendered icon.
    ToolInfo {
        tool: Tool::Contour,
        icon: icons::Icon::Contour,
        tooltip: "Drag across a background mesh: the cut under the line becomes a curve loop in the active mesh",
        command_id: "Contour",
        names: &["contour"],
    },
    // Duplicate tab (specifications/duplicate-tools), batch 12 icons. Ids
    // from the name map's Duplicate rows.
    ToolInfo {
        tool: Tool::Mirror,
        icon: icons::Icon::Mirror,
        tooltip: "Mirror-image copy across a plane (X to start), welded at the seam; click in the view to make it",
        command_id: "*.mirror",
        names: &["mirror"],
    },
    ToolInfo {
        tool: Tool::Lathe,
        icon: icons::Icon::RadialSweep,
        tooltip: "Spins the selected lines, edges or polygon outlines round an axis into one surface (a lathe); click to set the axis centre",
        command_id: "Radial Sweep",
        names: &["radial sweep", "lathe"],
    },
    ToolInfo {
        tool: Tool::Clone,
        icon: icons::Icon::Clone,
        tooltip: "Copies in a row: drag in the view to set the step between them",
        command_id: "*.clone",
        names: &["clone"],
    },
    ToolInfo {
        tool: Tool::Array,
        icon: icons::Icon::Array,
        tooltip: "Copies in a 3D grid: click to set the spacing to the source's size, drag to change it",
        command_id: "*.array",
        names: &["array"],
    },
    ToolInfo {
        tool: Tool::RadialArray,
        icon: icons::Icon::RadialArray,
        tooltip: "Copies round a circle or spiral: click to set the centre",
        command_id: "*.Radial Array",
        names: &["radial array"],
    },
    // Provisional icons (Sketch Extrude, Clone) until the path tools get
    // their own from the generator.
    ToolInfo {
        tool: Tool::CurveExtrude,
        icon: icons::Icon::SketchExtrude,
        tooltip: "Extrude the selected polygons along a path you click out in the view",
        command_id: "curve.extrude",
        // The bare tool; the buttons are its preset rows (Curve / Bezier /
        // B-Spline Extrude).
        names: &["curve.extrude"],
    },
    ToolInfo {
        tool: Tool::CurveClone,
        icon: icons::Icon::Clone,
        tooltip: "Copies along a path you click out in the view",
        command_id: "curve.clone",
        // The bare tool; the buttons are its preset rows (Curve / Bezier /
        // B-Spline / Pen Clone).
        names: &["curve.clone"],
    },
    ToolInfo {
        tool: Tool::ScatterClone,
        icon: icons::Icon::Clone,
        tooltip: "Copies scattered at random in a box: click in the view to make them",
        command_id: "scatter.clone",
        names: &["scatter clone"],
    },
    // DX-40: its own rendered icon (icon studio).
    ToolInfo {
        tool: Tool::Symmetrize,
        icon: icons::Icon::Symmetrize,
        tooltip: "Delete one side of the mesh and mirror the other side over it",
        command_id: "operandi.symmetrize.tool",
        names: &["symmetrize"],
    },
    // UV tab live tools (icons batch 22, approved 2026-11-18; Peeler shares
    // UV Rectangle's grid).
    ToolInfo {
        tool: Tool::UvProject,
        icon: icons::Icon::UvProject,
        tooltip: "Writes the selected polygons' UVs from a projection: Planar, Cylindrical, Spherical, Barycentric or Cubic",
        command_id: "operandi.uv.project",
        names: &["uv project"],
    },
    ToolInfo {
        tool: Tool::UvViewProject,
        icon: icons::Icon::UvProjectView,
        tooltip: "Click in a 3D view: the selected polygons get UVs as they look from there",
        command_id: "operandi.uv.viewProject",
        names: &["project from view"],
    },
    ToolInfo {
        tool: Tool::UvUnwrap,
        icon: icons::Icon::UvUnwrap,
        tooltip: "Unwraps every visible polygon along the selected edges into the UV map; drag for Iterations",
        command_id: "uv.unwrap",
        names: &["unwrap tool"],
    },
    ToolInfo {
        tool: Tool::UvRelax,
        icon: icons::Icon::UvRelax,
        tooltip: "Relaxes the UV map's islands to take out stretch; drag for Iterations",
        command_id: "uv.relax",
        names: &["uv relax"],
    },
    ToolInfo {
        tool: Tool::UvPelt,
        icon: icons::Icon::UvPelt,
        tooltip: "Unwraps, then relaxes the result, in one tool",
        command_id: "uv.pelt",
        names: &["unwrap & relax", "unwrap and relax"],
    },
    ToolInfo {
        tool: Tool::UvPeeler,
        icon: icons::Icon::UvRectangle,
        tooltip: "Lays clean quad strips out as a regular grid, cut at the selected edges",
        command_id: "operandi.uv.peeler",
        names: &["peeler"],
    },
    ToolInfo {
        tool: Tool::UvBoxTransform,
        icon: icons::Icon::Transform,
        tooltip: "In a UV view: drag inside the box to move the selected UVs, its round handle to turn them, right-drag a side or corner to scale",
        command_id: "operandi.uv.boxTransform",
        names: &["box transform"],
    },
];

/// The entry for `tool`.
pub(crate) fn info(tool: Tool) -> &'static ToolInfo {
    TOOLS
        .iter()
        .find(|t| t.tool == tool)
        .expect("every Tool has a TOOLS entry (checked by tests)")
}

/// The tool started by a typed command-line name, if any.
pub(crate) fn by_name(name: &str) -> Option<Tool> {
    TOOLS
        .iter()
        .find(|t| t.names.contains(&name))
        .map(|t| t.tool)
}
