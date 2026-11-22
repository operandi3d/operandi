# Bevel tools: activation and when the operation applies — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05. REA-assisted: no.

Complements `polygon-bevel-2026-11-02` (geometry and defaults); this spec
covers only switching the tool on, its remembered settings and when it
changes the mesh.

## Rules

1. **The Bevel button picks the tool from the selection mode:** Polygon
   mode → Polygon Bevel (BA01), Edge mode → Edge Bevel (BA02), Vertex mode →
   Vertex Bevel (BA03). In Item mode no bevel tool turns on (BA04).
2. **Switching on changes nothing.** The mesh is unchanged until the tool is
   applied (BA09–BA10, start states).
3. **Settings are remembered** across turning the tool off and on and
   across dropping it: Inset 0.2 / Shift 0.1 set earlier are still there
   when the tool comes back (BA05). Only Reset restores the defaults
   (Inset 0, Shift 0, Group Polygons on).
4. **Turning the tool off or dropping it without applying** leaves the mesh
   as it was, even with non-zero settings (BA09, BA10). (In the UI,
   dragging the handle or editing a value while the tool is active is what
   applies it; that interactive path wasn't scripted.)
5. **Applying with all-zero settings still builds geometry:** Inset 0,
   Shift 0 on a cube top adds four new points at the top corners' positions
   and four zero-area side polygons; the top now uses the new points
   (8 → 12 points, 6 → 10 polygons, BA06). Points don't move. Decided:
   Operandi does the same (Q-BA-01).
6. **Each apply is a new bevel.** Applying again bevels the current cap a
   second time with the current settings (BA07: Inset 0.1 twice → corners
   at ±0.4 then ±0.3, 16 points / 14 polygons). Changing Shift between
   applies doesn't edit the first bevel; the second bevel uses the new
   Shift (BA08: second ring at y 0.7). A remembered setting is used by the
   next apply after switching back on (BA12).
7. **Polygon mode with nothing selected:** applying changes nothing
   (BA11). (Other polygon tools act on every polygon in that case; Bevel
   doesn't.)
8. **Selection after apply:** the cap polygon stays selected, and the tool
   stays on until it's turned off or dropped.

## Open questions and decisions

- **Q-BA-01** (decided below) Zero-settings apply (rule 5) leaves overlapping
  points and zero-area polygons. Sane rule for Operandi: copy Modo, since
  it's the behaviour users see and later edits rely on the new ring. The
  alternative is to skip the apply when Inset and Shift are both 0.
- **Q-BA-02** Interactive path: does each handle drag start a new bevel or
  edit the current one; does a numeric edit after a drag re-apply? Needs
  hands-on.
- **Q-BA-03** Whether remembered settings survive a restart (preferences).

## Decided (2026-11-05)

- **Q-BA-01 → decided:** copy Modo. An apply with Inset 0 and Shift 0 still
  builds the zero-size ring (BA06).
