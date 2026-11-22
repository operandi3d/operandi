//! The viewport reads each tool's handle layout from its card
//! (operandi-core tool_card.rs). Each `old_*` function below is a verbatim
//! copy of a list from viewport.rs, snapping.rs or plane_handles.rs as it
//! stood on main 02355fbf; the card must give the same answers.
use operandi_core::Tool;
use operandi_core::tool_card::{HandleAnchor, HandleAxes, HandleKind, Snap};

fn old_combined_handles(tool: Tool) -> bool {
    matches!(tool, Tool::Transform | Tool::EdgeExtend)
}

fn old_off_handle_drag(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::Bevel
            // Added with Twist Bevel (twist-bevel-2026-11-15).
            | Tool::TwistBevel
            | Tool::Chamfer
            | Tool::SmoothShift
            | Tool::Thicken
            | Tool::EdgeExtend
            | Tool::EdgeExtrude
            | Tool::EdgeSlide
            | Tool::EdgeCrease
            | Tool::EdgeBevel
            | Tool::EdgeChamfer
            | Tool::EdgeRelax
            | Tool::VertexExtrude
            | Tool::VertexMerge
            | Tool::VertexTear
            | Tool::VertexSlide
            | Tool::Push
            | Tool::Smooth
            | Tool::Jitter
            | Tool::Quantize
            | Tool::RadialAlign
            | Tool::LinearAlign
            | Tool::SymmetryTool
            | Tool::Inset
            | Tool::Shift
            // Added with the UV tab's solvers (2026-11-18): Iterations.
            | Tool::UvUnwrap
            | Tool::UvRelax
            | Tool::UvPelt
            | Tool::UvBoxTransform
    )
}

/// `handles()`: the tools that show none (the early return, and Create's).
fn old_no_handles(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::PolyHaul
            | Tool::SketchExtrude
            | Tool::Tweak
            | Tool::Push
            | Tool::Smooth
            | Tool::Jitter
            | Tool::Quantize
            | Tool::RadialAlign
            | Tool::LinearAlign
            | Tool::Bend
            | Tool::SymmetryTool
            // No handle drawn (ACS-Q-HANDLE1).
            | Tool::EdgeRelax
            | Tool::VertexBevel
            // Contour draws its own line (contour_ui.rs).
            | Tool::Contour
    ) || tool == Tool::VertexCreate
        // The path tools draw their own path points (path_tool_view.rs).
        || matches!(tool, Tool::CurveExtrude | Tool::CurveClone | Tool::ScatterClone)
        // Symmetrize: fields only (DX-41), no handles.
        || tool == Tool::Symmetrize
        // The UV tab's tools (2026-11-18): fields, clicks and drags only.
        || matches!(
            tool,
            Tool::UvProject
                | Tool::UvViewProject
                | Tool::UvUnwrap
                | Tool::UvRelax
                | Tool::UvPelt
                | Tool::UvPeeler
                | Tool::UvBoxTransform
        )
}

/// `handles()`'s general layout: how many numbered axis handles.
fn old_axis_count(tool: Tool) -> usize {
    if matches!(
        tool,
        Tool::Chamfer
            | Tool::EdgeSlide
            | Tool::EdgeCrease
            | Tool::EdgeBevel
            | Tool::EdgeChamfer
            | Tool::EdgeRelax
            | Tool::VertexBevel
            | Tool::VertexMerge
            | Tool::VertexSlide
            | Tool::Inset
            | Tool::Shift
    ) {
        1
    } else if matches!(tool, Tool::Bevel | Tool::EdgeExtrude | Tool::VertexExtrude)
        || tool.is_smooth_shift()
    {
        2
    } else {
        3
    }
}

/// The handle painter: tools whose handle 0 is blue.
fn old_blue_first(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::Bevel | Tool::Chamfer | Tool::EdgeExtrude | Tool::VertexExtrude
    ) || tool.is_smooth_shift()
}

/// The handle painter: tools whose handle 1 is a line with a square end.
fn old_second_is_line(tool: Tool) -> bool {
    matches!(tool, Tool::Bevel | Tool::EdgeExtrude | Tool::VertexExtrude) || tool.is_smooth_shift()
}

/// `handles()`: where each tool's handles sit (`None` for the general
/// action-centre fallback).
fn old_anchor(tool: Tool) -> Option<HandleAnchor> {
    Some(match tool {
        Tool::EdgeExtrude => HandleAnchor::Edge,
        Tool::EdgeSlide => HandleAnchor::EdgeRail,
        Tool::EdgeBevel | Tool::EdgeChamfer => HandleAnchor::Edge,
        Tool::EdgeCrease => HandleAnchor::Edge,
        Tool::VertexExtrude | Tool::VertexMerge | Tool::VertexTear | Tool::VertexSlide => {
            HandleAnchor::Vertex
        }
        Tool::Chamfer => HandleAnchor::ChamferStrip,
        Tool::Box => HandleAnchor::Position,
        Tool::Bevel | Tool::TwistBevel | Tool::Extrude | Tool::SmoothShift | Tool::Thicken => {
            HandleAnchor::PolygonStage
        }
        Tool::Inset | Tool::Shift => HandleAnchor::FirstPolygon,
        _ => return None,
    })
}

/// `handles()`: anchors found in item space.
fn old_local_origin(tool: Tool) -> bool {
    matches!(tool, Tool::Chamfer | Tool::Inset | Tool::Shift)
}

/// `handle_axis()`'s frame-based tools.
fn old_axes(tool: Tool) -> HandleAxes {
    if matches!(
        tool,
        Tool::VertexBevel | Tool::VertexExtrude | Tool::VertexMerge | Tool::VertexSlide
    ) {
        return if tool == Tool::VertexExtrude {
            HandleAxes::VertexNormal
        } else {
            HandleAxes::VertexRail
        };
    }
    if tool == Tool::EdgeSlide {
        return HandleAxes::EdgeRail;
    }
    if matches!(tool, Tool::EdgeBevel | Tool::EdgeChamfer | Tool::EdgeRelax) {
        return HandleAxes::EdgeSide;
    }
    if matches!(tool, Tool::EdgeExtrude | Tool::EdgeCrease) {
        return HandleAxes::EdgeNormal;
    }
    HandleAxes::Tool
}

fn old_plane_handles(tool: Tool) -> bool {
    matches!(tool, Tool::Move | Tool::Scale | Tool::Transform)
}

fn old_snap_move(tool: Tool) -> bool {
    // Twist Bevel's Shift arrow snaps through its own drag (spec section 5).
    matches!(tool, Tool::Move | Tool::Transform | Tool::TwistBevel)
}

#[test]
fn handle_cards_match_the_old_viewport_lists() {
    for card in operandi_core::tool_card::cards() {
        let t = card.tool;
        let h = card.handles;
        let name = t.label();
        assert_eq!(
            h.kind == HandleKind::Combined,
            old_combined_handles(t),
            "{name}"
        );
        assert_eq!(
            h.drag_anywhere,
            old_off_handle_drag(t),
            "{name} drag anywhere"
        );
        assert_eq!(h.kind == HandleKind::None, old_no_handles(t), "{name} none");
        assert_eq!(
            h.kind == HandleKind::RadialBounds,
            matches!(t, Tool::Sphere | Tool::Cylinder),
            "{name} radial"
        );
        assert_eq!(
            h.kind == HandleKind::Rings,
            t == Tool::Rotate,
            "{name} rings"
        );
        assert_eq!(
            h.kind == HandleKind::Bounds,
            t == Tool::Box,
            "{name} bounds"
        );
        if h.kind != HandleKind::None {
            assert_eq!(h.kind.axis_count(), old_axis_count(t), "{name} axes");
        }
        assert_eq!(
            matches!(h.kind, HandleKind::Pair | HandleKind::Normal),
            old_blue_first(t),
            "{name} blue"
        );
        assert_eq!(
            h.kind == HandleKind::Pair,
            old_second_is_line(t),
            "{name} pair"
        );
        match old_anchor(t) {
            Some(anchor) => assert_eq!(h.anchor, anchor, "{name} anchor"),
            // Sphere and Cylinder place their bounds from their own branch.
            None => assert!(
                matches!(h.anchor, HandleAnchor::ActionCentre | HandleAnchor::None)
                    || (h.kind == HandleKind::RadialBounds && h.anchor == HandleAnchor::Position),
                "{name} anchor"
            ),
        }
        assert_eq!(
            h.anchor == HandleAnchor::None,
            old_no_handles(t),
            "{name} anchor"
        );
        assert_eq!(h.anchor.item_local(), old_local_origin(t), "{name} local");
        assert_eq!(h.axes, old_axes(t), "{name} axis directions");
        assert_eq!(
            h.plane_circles,
            old_plane_handles(t),
            "{name} plane circles"
        );
        assert_eq!(
            card.layers.snap == Snap::Handle,
            old_snap_move(t),
            "{name} snap"
        );
    }
}
