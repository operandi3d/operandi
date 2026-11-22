# Duplicate tools: GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build
`Operandi duplicate-tools <commit>.app` in
`/Volumes/Development/Operandi Builds`, on a throwaway scene. Each line:
do this → expect that. Spec: `docs/specs/duplicate-tools/`.

## Duplicate tab

1. A **Duplicate** tab sits between Deform and Edit. Top to bottom:
   Extend (Radial Sweep), Clone (Clone), Array (Array, Radial Array), Duplicate
   (Mirror). Rows are text only (no icons yet).
2. Each row starts its tool (lit) and shows Tool Properties in folding
   sections: a Generator (Mirror, Linear, Array, Helix), then Clone
   Effector or Sweep Effector; Mirror also has Mirror Generator Snap. A
   second click drops it. Starting a tool changes nothing in the scene.
3. Geometry > Duplicate lists the same five; typing "lathe" also starts Radial Sweep.

## Mirror (the beginner case)

1. Cube, Polygon mode, select the polygons on the −X side and Delete them
   (half a cube, seam open on X = 0). Select nothing. Mirror.
2. The plane shows as a red square through the origin with a centre dot
   and an edge cube. Click anywhere in the view away from them → the
   mirrored half appears, joined at the seam: a closed cube, no seam
   points doubled, nothing inside-out (Toggle Subdivision: smooth across
   the seam).
3. Drag the centre dot → the plane slides along X and the copy follows;
   with snapping on (X) it snaps back onto the origin when near.
4. Angle starts at 0°. Drag the edge cube round → Angle changes and the
   plane turns about its hinge (the Left line): on a flat square, 90° puts
   the copy beside it, flipped, sharing the edge.
5. Invert Polygons → the copy's normals turn inward; Replace Source → only
   the mirrored half remains.
6. Drop (Space), Cmd+Z once → the copy is gone and the seam is open again.
7. Item mode, select a mesh, Mirror, click → a new item "<name> (2)" next
   to it in the Items list, mirrored, with a positive Scale.

## Radial Sweep (was Lathe)

1. Front view. Draw a half wine-glass outline as an open line or curve,
   its ends on the Y axis. Radial Sweep, click on the Y axis →
   a closed glass round the axis, facing out, no seam, the line used up.
2. Drag the arc's end cube back to 180° → an open half shell, capped at
   both ends (Cap Start / Cap End on); untick the caps → open.
3. Offset 0.1, End 1080 on a small square face away from the axis → a
   spring of three turns.
4. Count 8 → 8 segments. Drag an axis end cube → the axis tilts.
5. Edge mode, select a few border edges of a flat grid, Radial Sweep → they
   sweep from the grid's edge.

## Clone, Array, Radial Array

1. Clone on a cube: press and drag in the view → the first copy follows
   the pointer; right-drag left/right → more or fewer copies (one per
   20 px). Ctrl before pressing keeps the drag on one axis.
2. Array on a 1 m cube: click → 2 × 1 × 2 cubes side by side; drag →
   spacing changes on the view's two axes; right-drag → counts; drag an
   axis cube → that axis's spacing.
3. Radial Array on a cube at X = 2: click at the origin → 8 cubes round Y,
   each turned. End 90 → the 8 pieces spread from 0° to 90°. Right-drag
   → Count.
4. Merge Vertices on, Distance 0, touching copies → they weld into one
   piece.

## Detriangulate

1. Polygon tab: a **Detriangulate...** row (and Geometry > Polygon >
   Detriangulate...). Triangulate a grid (Triple), then Detriangulate... →
   the dialog (Flatness 20 %, Max Angle 170°, Keep … checkboxes) → OK →
   quads again, selected. Cmd+Z → triangles.
2. Run it again on quads → status "No triangle pairs met the settings.",
   nothing changes.
