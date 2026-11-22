# Tool activation: does a panel edit act before the first viewport click? (v1.2)

Status: v1, observed 2026-11-09 in the reference app (17.1), hands-on in
the GUI. Each case: fresh scene, 1 m cube at the origin, selection made,
tool turned on with its shortcut or toolbox button, then a value typed in
the tool panel (or the mini properties popup) and Return. No viewport click
or handle touch before the edit. Behaviour only, no internals. Public docs
agree where they say anything.

## The bug Jason hit

In Operandi: select a polygon, press B (Polygon Bevel), drag the Shift or
Inset slider in the tool panel. Nothing happens until you click in the
viewport. In the reference app the bevel shows up straight away.

## Results

| Tool | Handles on activation? | Panel edit before any viewport click | Kind |
|---|---|---|---|
| Polygon Bevel (B, Polygon mode) | Yes, blue + red at once | Inset 0.1 → top inset live. Then Shift 0.1 → **same** bevel raised (still 12 points / 10 polys, so one bevel adjusted, not a second one). | **Live** |
| Edge Bevel (B, Edge mode) | Yes, at once | Value 0.1 → bevel appears live. (With no edges selected, it bevelled **all** edges.) | **Live** |
| Vertex Bevel (B, Vertex mode) | Yes, one blue handle | Inset 0.2 → corner cut live (1 → 3 points). | **Live** |
| Polygon Extrude (Shift+X, Polygon mode) | **No** | Extent Y 0.3 → field changes, mesh unchanged. Pressing the panel's Apply button then extrudes. | **Waits** |
| Smooth Shift (Polygon tab) | **No** | Offset 0.3 → mesh unchanged. | **Waits** |
| Thicken (Polygon tab; same panel as Smooth Shift with Thicken ticked) | **No** | Offset 0.3 → mesh unchanged. | **Waits** |
| Edge Extrude (Shift+X, Edge mode) | Yes, at once | Extrude 0.3 → the handles moved to follow the value, but the test edge (a closed cube edge) gave no new geometry. Probably live; needs a boundary edge to prove. | Probably live |
| Edge Chamfer | (not run) | Docs: "click in the viewport to activate the tool." | Waits (docs) |
| Inset / Shift (Geometry > Polygon) | (not run) | Docs: "Set the Inset value ... then use the Apply button." | Apply button |
| Vertex Extrude | (not run) | — | Open |

**Pattern:** if handles appear the moment the tool turns on, panel edits
are live. If no handles appear, the tool waits for a viewport click (or
the panel's Apply button).

## Rules for Operandi

- **TA-01.** Polygon, Edge and Vertex Bevel are live from the moment they
  turn on. Handles appear at once, placed from the selection, and any panel
  edit (typed value or slider drag) updates the mesh right away. No
  viewport click required.
- **TA-02.** One bevel stays "open" while the tool is on. Panel edits and
  further handle drags (after releasing) all adjust that same bevel; they
  don't stack new ones (Jason, hands-on 11-09). **Shift+click on a face**
  commits the current bevel and starts a new one. (Corrects v1, which
  said a re-drag stacks a new bevel; that came from a scripted 11-05 run
  and was wrong for the real GUI.)
  Shift+drag also starts a new bevel (Jason, hands-on 11-09).
- **TA-02b.** Undo: each panel edit is its own undo step; Cmd+Z undoes the
  last edit and the tool stays on (earlier run). Undo per handle drag not
  yet recorded.
- **TA-03.** Turning a tool on changes nothing by itself.
- **TA-04.** Polygon Extrude, Smooth Shift and Thicken show no handles on
  activation, and panel edits only store the value. The first viewport
  click places the handles and acts. Don't change these to live.
- **TA-05.** Polygon Extrude's panel has an Apply button that runs the
  extrude with the panel values, no viewport click needed.
- **TA-06.** Nothing selected = the tool acts on everything. Edge Bevel
  with no edges selected bevelled every edge (GUI, 11-09). Polygon Bevel:
  follow PB-O-01 (bevels every polygon, leaves nothing selected; measured
  11-02). The 11-05 note "Polygon Bevel with nothing selected does
  nothing" came from a scripted run, as did the wrong re-drag claim, so it
  is set aside. Unconfirmed in the GUI; see Open.
- **TA-07 (open).** Undo after the tool is dropped: one step for the whole
  session, or one per stacked bevel? Not recorded. Provisional for
  building: one undo step per bevel (each Shift+click commit is a step),
  matching how each panel edit is its own step while the tool is on.

## Open (needs a GUI check)

- O-1: Polygon Bevel with nothing selected, typed Inset: all polygons or none?
- O-2: Bevel, two stacked bevels (Shift+click between), drop, Cmd+Z: how
  many bevels does one Undo remove?
