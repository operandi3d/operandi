//! The tool cards (tool_card.rs) give the same answers as the hand-written
//! lists they replaced. Each `old_*` function below is a verbatim copy of a
//! list as it stood on main 02355fbf, before the card existed; keep them as
//! the record of what the card had to match. Tools added since (Twist
//! Bevel, 2026-11-15) are added to the lists with a note.
use glam::DVec3;
use operandi_core::tool_card::{ActionCentre, Family, Modes, Snap};
use operandi_core::*;

/// falloff.rs `supports_falloff`.
fn old_supports_falloff(tool: Tool) -> bool {
    use Tool::*;
    matches!(
        tool,
        Bevel
            | TwistBevel // added 2026-11-15
            | Move
            | Rotate
            | Scale
            | Transform
            | EdgeBevel
            | EdgeChamfer
            | VertexBevel
            | EdgeSlide
            | VertexExtrude
            | Push
            | Smooth
            | Jitter
            | Quantize
            | RadialAlign
            | UvRelax // added 2026-11-18
            | UvPelt
            | UvBoxTransform
            | LinearAlign
            | Bend
    )
}

/// symmetry_support.rs `tool_support`.
fn old_symmetry(tool: Tool) -> symmetry_support::SymmetrySupport {
    use symmetry_support::SymmetrySupport::*;
    match tool {
        Tool::Box | Tool::Sphere | Tool::Cylinder => Built,
        Tool::Move | Tool::Rotate | Tool::Scale | Tool::Transform | Tool::Tweak => Built,
        Tool::Mirror | Tool::SymmetryTool => Built,
        // Deform finish: mirrored after the primary side (class C).
        Tool::Jitter | Tool::Quantize | Tool::RadialAlign | Tool::LinearAlign => Built,
        // Added 2026-11-15: mirrors bend and twist itself.
        Tool::TwistBevel => Built,
        Tool::Bevel
        | Tool::Chamfer
        | Tool::SmoothShift
        | Tool::Thicken
        | Tool::Push
        | Tool::Smooth
        | Tool::Inset
        | Tool::Shift
        | Tool::EdgeCrease
        | Tool::EdgeBevel
        | Tool::EdgeChamfer
        | Tool::EdgeRelax
        | Tool::VertexBevel
        | Tool::VertexMerge => ViaSelection,
        Tool::Extrude | Tool::EdgeExtend | Tool::EdgeSlide | Tool::VertexSlide => Built,
        Tool::Bend | Tool::VertexTear | Tool::SketchExtrude | Tool::PolyHaul => Built,
        Tool::EdgeExtrude | Tool::VertexExtrude => ViaSelection,
        Tool::LoopSlice
        | Tool::EdgeSlice
        | Tool::PolygonSlice
        | Tool::CurveSlice
        | Tool::PenSlice
        | Tool::PrimitiveSlice
        | Tool::AxisSlice
        | Tool::Contour => Built,
        Tool::Lathe | Tool::Clone | Tool::Array | Tool::RadialArray => Built,
        Tool::CurveExtrude | Tool::CurveClone | Tool::ScatterClone | Tool::Symmetrize => Built,
        Tool::VertexCreate => Built,
        // UV tab (added 2026-11-18): they write UVs, not points.
        Tool::UvProject
        | Tool::UvViewProject
        | Tool::UvUnwrap
        | Tool::UvRelax
        | Tool::UvPelt
        | Tool::UvPeeler
        | Tool::UvBoxTransform => ViaSelection,
    }
}

/// multi_tools.rs `tool_runs_on_each_mesh`.
fn old_multi_mesh(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::Bevel
            | Tool::Extrude
            | Tool::SmoothShift
            | Tool::Thicken
            | Tool::Inset
            | Tool::Shift
            | Tool::Chamfer
            | Tool::Push
            | Tool::Smooth
            | Tool::Jitter
            | Tool::Quantize
            | Tool::RadialAlign
            | Tool::LinearAlign
            | Tool::Bend
            | Tool::EdgeExtend
            | Tool::EdgeExtrude
            | Tool::EdgeSlide
            | Tool::EdgeCrease
            | Tool::EdgeBevel
            | Tool::EdgeChamfer
            | Tool::EdgeRelax
            | Tool::VertexBevel
            | Tool::VertexExtrude
            | Tool::VertexMerge
            | Tool::VertexTear
            | Tool::VertexSlide
            | Tool::PolygonSlice
            | Tool::PenSlice
            | Tool::CurveSlice
            | Tool::PrimitiveSlice
            | Tool::AxisSlice
    )
}

/// editor.rs activation: the tools that capture an action frame.
fn old_action_frame(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::Transform
            | Tool::Move
            | Tool::Rotate
            | Tool::Scale
            | Tool::Bevel
            | Tool::TwistBevel // added 2026-11-15
            | Tool::Bend // v2, bend-handles-2026-11-15
            | Tool::Extrude
            | Tool::SmoothShift
            | Tool::Thicken
            | Tool::EdgeExtend
    )
}

/// editor.rs activation: the tools that need a neutral context.
fn old_neutral(tool: Tool) -> bool {
    matches!(tool, Tool::Chamfer | Tool::PolyHaul | Tool::SketchExtrude)
}

/// editor.rs `Tool::is_transform`.
fn old_is_transform(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::Move | Tool::Rotate | Tool::Scale | Tool::Transform | Tool::Tweak
    ) || tool == Tool::UvBoxTransform // added 2026-11-18: a step per drag
}

/// The Move/Rotate/Scale/Transform list repeated through editor.rs and the app.
fn old_item_transform(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::Transform | Tool::Move | Tool::Rotate | Tool::Scale
    )
}

/// editor.rs `handle_continue`.
fn old_continues(tool: Tool) -> bool {
    matches!(tool, Tool::Bevel | Tool::Extrude | Tool::EdgeExtend)
        || matches!(tool, Tool::SmoothShift | Tool::Thicken)
        // Added 2026-11-15.
        || tool == Tool::TwistBevel
        || matches!(
            tool,
            Tool::Mirror | Tool::Lathe | Tool::Clone | Tool::Array | Tool::RadialArray | Tool::ScatterClone
        )
}

/// editor.rs activation's family lists, and `is_primitive`, `is_slice`,
/// `is_duplicate`.
fn old_family(tool: Tool) -> Option<Family> {
    if matches!(tool, Tool::Box | Tool::Sphere | Tool::Cylinder) {
        return Some(Family::Primitive);
    }
    if matches!(
        tool,
        Tool::EdgeExtend
            | Tool::EdgeExtrude
            | Tool::EdgeSlide
            | Tool::EdgeCrease
            | Tool::EdgeBevel
            | Tool::EdgeChamfer
            | Tool::EdgeRelax
    ) {
        return Some(Family::Edge);
    }
    if matches!(
        tool,
        Tool::VertexBevel
            | Tool::VertexExtrude
            | Tool::VertexMerge
            | Tool::VertexTear
            | Tool::VertexSlide
    ) {
        return Some(Family::Vertex);
    }
    if matches!(
        tool,
        Tool::Push
            | Tool::Smooth
            | Tool::Bend
            | Tool::SymmetryTool
            | Tool::Jitter
            | Tool::Quantize
            | Tool::RadialAlign
            | Tool::LinearAlign
    ) {
        return Some(Family::Deform);
    }
    if matches!(
        tool,
        Tool::LoopSlice
            | Tool::EdgeSlice
            | Tool::PolygonSlice
            | Tool::CurveSlice
            | Tool::PenSlice
            | Tool::PrimitiveSlice
            | Tool::AxisSlice
            | Tool::Contour
    ) {
        return Some(Family::Slice);
    }
    if matches!(
        tool,
        Tool::Mirror
            | Tool::Lathe
            | Tool::Clone
            | Tool::Array
            | Tool::RadialArray
            | Tool::CurveExtrude
            | Tool::CurveClone
            | Tool::ScatterClone
            | Tool::Symmetrize
    ) {
        return Some(Family::Duplicate);
    }
    // Added 2026-11-18.
    if matches!(
        tool,
        Tool::UvProject
            | Tool::UvViewProject
            | Tool::UvUnwrap
            | Tool::UvRelax
            | Tool::UvPelt
            | Tool::UvPeeler
            | Tool::UvBoxTransform
    ) {
        return Some(Family::Uv);
    }
    None
}

/// snapping.rs `snap_move`: the tools whose move handle snaps.
fn old_handle_snaps(tool: Tool) -> bool {
    // Twist Bevel (added 2026-11-15): its Shift arrow.
    matches!(tool, Tool::Move | Tool::Transform | Tool::TwistBevel)
}

#[test]
fn cards_match_the_old_lists() {
    for card in tool_card::cards() {
        let t = card.tool;
        let name = t.label();
        assert_eq!(
            card.layers.falloff,
            old_supports_falloff(t),
            "{name} falloff"
        );
        assert_eq!(card.layers.symmetry, old_symmetry(t), "{name} symmetry");
        assert_eq!(card.multi_mesh, old_multi_mesh(t), "{name} multi-mesh");
        assert_eq!(
            card.layers.action_centre == ActionCentre::Frame,
            old_action_frame(t),
            "{name} action frame"
        );
        assert_eq!(
            card.layers.action_centre == ActionCentre::Neutral,
            old_neutral(t),
            "{name} neutral context"
        );
        assert_eq!(
            card.undo_per_drag,
            old_is_transform(t),
            "{name} per-drag undo"
        );
        assert_eq!(card.transforms_items, old_item_transform(t), "{name} items");
        assert_eq!(card.continues, old_continues(t), "{name} continues");
        assert_eq!(
            card.layers.snap == Snap::Handle,
            old_handle_snaps(t),
            "{name} handle snap"
        );
        if let Some(family) = old_family(t) {
            assert_eq!(card.family, family, "{name} family");
        } else {
            assert!(
                matches!(
                    card.family,
                    Family::Transform | Family::Polygon | Family::Create
                ),
                "{name} family"
            );
        }
        assert_eq!(card, t.card(), "{name}");
    }
}

#[test]
fn every_tool_has_one_card() {
    let tools: Vec<Tool> = tool_card::cards().map(|c| c.tool).collect();
    assert_eq!(tools, Tool::ALL.to_vec());
}

#[test]
fn the_falloff_refusal_names_the_falloff_tools() {
    assert_eq!(
        tool_card::falloff_refusal(),
        "This tool doesn't work with a falloff yet; set Falloff to None, or use \
         Bevel, Twist Bevel, Transform, Move, Rotate, Scale, Slide, Edge Bevel, Edge Chamfer, \
         Vertex Bevel, Vertex Extrude, Push, Smooth, Jitter, Quantize, Radial Align, \
         Linear Align, Bend, UV Relax, Unwrap & Relax or Box Transform"
    );
}

/// A 2 m cube in `mode` with one point, one edge and one polygon selected.
fn cube(mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 1)
        .unwrap()
        .into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(mode)).unwrap();
    let mesh = e.scene.active().unwrap().mesh.clone();
    let s = &mut e.scene.selection;
    // The first edge's two points: edge tools from Vertex mode take the
    // edge between them (selection inference, M06).
    let edge = mesh.edges()[0];
    s.vertices = vec![edge.0, edge.1];
    s.edges = vec![edge];
    s.faces = vec![0];
    e
}

/// A tool refuses to start in a mode its card leaves out.
#[test]
fn tools_refuse_modes_outside_their_card() {
    for card in tool_card::cards() {
        for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon, Mode::Item] {
            if card.modes.contains(mode) {
                continue;
            }
            let mut e = cube(mode);
            if card.tool == Tool::SymmetryTool {
                e.dispatch(Command::Symmetry(Symmetry {
                    axis: Some(0),
                    ..Default::default()
                }))
                .unwrap();
            }
            assert!(
                e.dispatch(Command::Activate(card.tool)).is_err(),
                "{} started in {mode:?}",
                card.tool.label()
            );
        }
    }
}

/// And starts in the modes its card names (with a cube to work on).
#[test]
fn component_tools_start_in_their_card_modes() {
    for card in tool_card::cards() {
        // Tools with further conditions (symmetry, drawn paths) are covered
        // by their own tests.
        if matches!(
            card.tool,
            Tool::SymmetryTool
                | Tool::SketchExtrude
                | Tool::Chamfer
                | Tool::EdgeRelax
                // Need a run of points to align (tests/deform_finish.rs).
                | Tool::RadialAlign
                | Tool::LinearAlign
        ) || card.modes == Modes::ALL
        {
            continue;
        }
        for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon] {
            if !card.modes.contains(mode) {
                continue;
            }
            let mut e = cube(mode);
            if let Err(err) = e.dispatch(Command::Activate(card.tool)) {
                panic!("{} refused {mode:?}: {err}", card.tool.label());
            }
        }
    }
}
