# Viewport handles

**Status: Spec drafted (partial).** Nine measured oracles; two inconclusive
off-handle trials and one failed setup. Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex), results read back from Modo (six decimals). REA-assisted: no.

Covered: Move X (two lengths, two zooms), Rotate blue ring (two chords),
Scale X (two lengths), Transform translation X (two lengths), on one selected
cube face. See BEHAVIOR-HANDLES.md and ORACLES.json.

Not covered: remaining axes/planes, complete handle inventories, primitive
and topology-tool handles, other buttons and modifier timing. The desktop API provides straight left-button drags, but no held-button segmented path, modifier changes during a drag, or intermediate held-drag readback. Straight drags replace the requested right-then-up path and are recorded individually. Modified clicks/keys were inconsistent; their results do not establish physical modifier semantics.
