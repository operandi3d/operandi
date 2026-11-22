# Item transforms

**Status: v3 draft (partial); earlier handed-off IDs retained.** Ten measured GUI oracles and one
inconclusive rotation-order attempt. Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex), results read back from Modo (six decimals). REA-assisted: no.

Covered: live typed Move/Rotate/Scale, Move/Rotate/Scale and combined
translation handle samples, negative-scale option and percentage entry,
Gimbal-on Euler channels, a parented child's local Move output.

Not covered: rotation-order UI change, clone/haul, other action centers,
all remaining axes/handles, world-matrix readback and undo. Earlier script-only
no-op records remain raw evidence; the live GUI route supersedes the blocked
status. Negative Scale was restored on for the Scale tool after IT-05.


## Physical rotation-order addition

One physical XYZ→ZXY change observed on 2026-11-03 UTC (2026-11-02 local),
Jason at the dropdown, Codex reading native UI plus six-decimal channels
and evaluated matrices. BEHAVIOR-PHYSICAL-ORDER.md adds one oracle (11
current); all ten earlier oracle objects are unchanged. Euler (30,45,60)
stays numerically unchanged while world orientation changes. The older
Not covered list describes the earlier pass; clone/haul, other order pairs,
parent contexts and broader action-center coverage remain open.


## Physical item-drag addition

BEHAVIOR-PHYSICAL-DUPLICATE.md adds three observed physical drag oracles
(14 current), preserving all 11 prior objects. Ordinary left and middle
drags moved the sole item. Control+Shift+left drag with Duplicate Mode
Instance created a linked Mesh Instance and moved it 0.084 m, preserving the
source. The earlier clone/haul gap is answered for this configuration;
other modes, contexts, axes and sensitivity remain open. Native Input
Editor bindings and item/source/matrix readback are raw evidence.
