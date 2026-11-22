# Preferences: Tool Handles and Snapping (behaviour spec v1, 2026-11-04)

Behaviour-only. "Not yet measured" = factory value still to be read (v2);
use a provisional value and log it. Settings apply at once and persist per
user. All on Display > Tool Handles unless noted.

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Large Points (pixels) | whole number | not yet measured | Drawn size of large handle points (e.g. on curve tools). |
| Small Points (pixels) | whole number | not yet measured | Drawn size of small handle points. |
| Large Points Hit Size | whole number | not yet measured | Click radius of large handle points; normally bigger than the drawn size. Too big makes crowded handles hard to pick. |
| Small Points Hit Size | whole number | not yet measured | Same for small points. |
| Handle Line Width | whole number | not yet measured | Drawn width of handle lines (Move, Rotate, Scale widgets). |
| Handle Line Hit Width | whole number | not yet measured | Click width of handle lines; normally bigger than the drawn width. |
| Handle Scale: Standard | number | not yet measured | Overall on-screen size of tool handles; handles keep this screen size whatever the zoom. The - and = keys also shrink and grow handles interactively. |
| Handle Scale: Small (% of Standard) | percent | not yet measured | Size of secondary, smaller handles relative to Standard. |
| Handle Scale: Plane Handle Ratio | number | not yet measured | Size of the two-axis plane handles relative to Standard. |
| Selected Tool Handles | dropdown: Invisible, Basic, Advanced | not yet measured | Handle detail for the active tool. Invisible: none. Basic: the tool's constraint handles. Advanced: extra handles where a tool has them. |
| Unselected Tool Handles | dropdown: Invisible, Basic, Advanced | not yet measured | Same for secondary handles such as falloff position and size. |
| Handle transparency (Display > OpenGL) | percent | not yet measured | How visible handles are where a surface hides them. |
| Inner Range (pixels) | whole number | not yet measured | Distance from a snap target at which the pointer snaps to it. |
| Outer Range (pixels) | whole number | not yet measured | Distance at which snap targets start to pre-highlight. |
| Rotation Snap Angle | angle | not yet measured | Angle step for rotation while the constrain key (Ctrl) is held. |

Not for now: Auto-Haul Display (Off, Basic, Advanced, On) and its drop
shadow, which control on-screen slider helpers some tools draw.

Open: HND-P-Q1 factory values (an earlier snapping measurement suggests
inner 3 px and outer 15 px; to be confirmed).
