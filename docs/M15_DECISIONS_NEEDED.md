# Consolidated M1.5 reference decisions needed

2026-10-21. One consolidated list for the separate documentation/investigation
process. Supply reviewed observable behavior and original acceptance oracles,
not investigation records, internal algorithms or implementation patches.
The Rust implementation session must not open reference applications or archives.

The user explicitly deferred approval of unresolved BOX-Q05/Q07/Q08 gesture
details to a focused runtime supplement. Restrictions below are current support
limits, not permanent product requirements or reference claims.

| IDs | Initial state and actions | Missing observable decision / affected workflow |
|---|---|---|
| BOX-Q05, BOX-L02/L03, BOX-T09 | Plain live Cube with asymmetric position/size/segments and prior scene geometry; Shift haul, then middle clone haul; repeat with stationary clicks, different construction planes and Undo/Redo | Geometry retained per application, next origin/placement, reset versus retained parameters, click/haul transition and history checkpoints. Ordinary hauling already continues one primitive; no blanket repeat prohibition is asserted. Modified repeat/clone currently reports unavailable rather than inventing placement. |
| BOX-Q07, BOX-A03, BOX-T10 | Initial drag in XY/XZ/YZ, asymmetric signed pointer deltas; Ctrl before press, pressed during drag, released during drag | Which dimensions become equal, anchor, constraint metric and transition behavior. Current Ctrl-initial drawing reports unresolved; Ctrl on an existing side handle moves the box under the established scope. |
| BOX-Q08/Q13, BOX-A04..06, BOX-T11 | Ctrl/Shift Cube-button clicks with no mesh, one/multiple/hidden mesh selections, transformed or parented mesh; tool inactive and already active | Exact target/new-item rule, insertion location, retained topology/attributes, resulting active/inactive lifecycle and history. PM-UP01..06 now approve and implement canonical one-metre units, local/new-mesh origins, immediate finalized completion, one undo and supported segmentation retention. Existing eligibility/live-task limits remain support limits; reference targeting/lifecycle and combined modifiers stay open. See PRIMITIVE_MODIFIERS.md. |
| BOX-Q04, BOX-H04/H05, BOX-T08 | Flat then positive-depth Cube; side/corner/axis hits in all Handle Modes, overlapping/occluded targets, Ctrl transitions | Corner layout, eligible target precedence, corner dimensional constraints, depth side when min/max project to one point. Show Corner remains visibly unavailable. Six independent sides and translation arrows work in current bounded scope. |
| BOX-Q09/Q10/Q14, BOX-T12/T20/T22 | Reset/retained properties; rotated/offset plane, transformed target, symmetry, snapping, falloff/action center | Defaults versus sample state; exact Reset Attributes scope; coordinate/target/context laws. Current major-axis/origin-locked and identity-target support is an implementation limitation; unsupported contexts are not silently accepted. Exact rounding/Bulge/atlas stay deferred by user scope. |
| SEL-U01/U02, SEL-035/036/064/090/091 | Native macOS Control/Command separately on picks, lasso, Boundary/fill and double clicks; tool inactive/active, field/menu focus | Per-gesture qualifiers, middle-click target precedence, fill versus extended-loop ownership, idle Escape and Shift+Space restoration. Existing logical Control subtraction and prior history policies are product behavior, not newly verified reference defaults. No universal Ctrl→Cmd mapping. |
| SEL-U05/U06, SEL-056/057/061/064, SEL-T044/T047/T048/T051 | F-S2 seeds q1,q3; More then Less and contiguous variants; F-S1 ordered loop seeds and Left/Right; offset q(0,0),q(2,2) Between; connected enclosing perimeter and fill | Exact sequence membership/removal, neighboring-loop direction relative to seed/view, inclusive rectangular Between membership, fill seed/qualifier/region and ambiguous ties. These commands remain unassigned/unavailable; none is aliased to Grow or a one-row shortest path. |
| SEL-U03/U04/U07, SEL-043/044/080 | Overlapping front-facing geometry; Through toggle with all pick/lasso styles; evaluated SubD/cage; hidden/locked/background and multi-active items | Through precedence and actual depth behavior, evaluated-to-editable targeting, hidden/locked/multi-item universe. Current lasso uses facing only (explicitly no depth occlusion) on one active visible mesh. Item lasso/multiple-item ordered ranges and component hide/lock need coherent supported state and corresponding eligibility decisions. |

## Already resolved or provisionally admitted in this implementation chat

Do not re-request these approvals:

- BOX-Q01: single quad at one zero dimension, no geometry at two zero dimensions,
  eight-vertex/six-quad positive box; original product topology, not reference proof.
- BOX-Q02: 1-logical-pixel off-handle threshold, nearest projected side center
  chosen at press, deterministic X/Y/Z and min/max tie order, lock side during
  hauling, one changed boundary. Unresolvable projected axes report feedback.
- BOX-Q03/Q15: reject crossed/negative bounds, retain existing Drop/Cancel/save/
  history policy, disable active Apply, apply retained Cube once while inactive.
- SEL-034: one history entry per completed selection gesture, first encounters
  ordered, focus loss restores starting selection.
- BOX-Q06: configurable provisional 12-logical-pixel detents and replaceable
  plane-axis mapping. No permanent 64-segment cap. Current preview limit is a
  technical 250,000-face allocation budget, with atomic refusal.
- SEL-U03: provisional shape sizing, even-odd freeform fill, inclusive boundary,
  full/partial intersection laws. Preserve the four-case facing matrix; facing
  is explicitly independent of depth occlusion. Current curved outlines use 128
  segments and are an original display approximation.

Close Loop's unequal two-region case is implemented. Equal regions, nonseparating
loops and unsupported topology report explicit feedback; no arbitrary side is
chosen. Regular-quad loops/rings have named seed/topology limits in the progress
ledger, not a universal all-topology claim.
