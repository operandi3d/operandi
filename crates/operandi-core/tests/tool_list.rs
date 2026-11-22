//! `Tool::ALL` must list every tool exactly once. The match below has no
//! wildcard, so adding a `Tool` variant breaks the build until it is listed.
use operandi_core::Tool;

fn position(tool: Tool) -> usize {
    match tool {
        Tool::Box => 0,
        Tool::Sphere => 1,
        Tool::Cylinder => 2,
        Tool::Bevel => 3,
        Tool::TwistBevel => 4,
        Tool::Chamfer => 5,
        Tool::PolyHaul => 6,
        Tool::SketchExtrude => 7,
        Tool::Extrude => 8,
        Tool::SmoothShift => 9,
        Tool::Thicken => 10,
        Tool::Transform => 11,
        Tool::Move => 12,
        Tool::Rotate => 13,
        Tool::Scale => 14,
        Tool::EdgeExtend => 15,
        Tool::EdgeExtrude => 16,
        Tool::EdgeSlide => 17,
        Tool::EdgeCrease => 18,
        Tool::EdgeBevel => 19,
        Tool::EdgeChamfer => 20,
        Tool::EdgeRelax => 21,
        Tool::VertexCreate => 22,
        Tool::VertexBevel => 23,
        Tool::VertexExtrude => 24,
        Tool::VertexMerge => 25,
        Tool::VertexTear => 26,
        Tool::VertexSlide => 27,
        Tool::Tweak => 28,
        Tool::Push => 29,
        Tool::Smooth => 30,
        Tool::Jitter => 31,
        Tool::Quantize => 32,
        Tool::RadialAlign => 33,
        Tool::LinearAlign => 34,
        Tool::Bend => 35,
        Tool::LoopSlice => 36,
        Tool::EdgeSlice => 37,
        Tool::PolygonSlice => 38,
        Tool::Inset => 39,
        Tool::Shift => 40,
        Tool::CurveSlice => 41,
        Tool::PenSlice => 42,
        Tool::PrimitiveSlice => 43,
        Tool::AxisSlice => 44,
        Tool::Contour => 45,
        Tool::Mirror => 46,
        Tool::Lathe => 47,
        Tool::Clone => 48,
        Tool::Array => 49,
        Tool::RadialArray => 50,
        Tool::CurveExtrude => 51,
        Tool::CurveClone => 52,
        Tool::ScatterClone => 53,
        Tool::Symmetrize => 54,
        Tool::UvProject => 56,
        Tool::UvViewProject => 57,
        Tool::UvUnwrap => 58,
        Tool::UvRelax => 59,
        Tool::UvPelt => 60,
        Tool::UvPeeler => 61,
        Tool::UvBoxTransform => 62,
        Tool::SymmetryTool => 55,
    }
}

#[test]
fn all_lists_every_tool() {
    for (index, &tool) in Tool::ALL.iter().enumerate() {
        assert_eq!(position(tool), index, "{}", tool.label());
    }
}
