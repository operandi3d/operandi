# Duplicate tools: shared behaviour (v1, 2026-11-08)

Behaviour-only spec. Applies to Mirror, Lathe, Clone, Array and Radial
Array. Each tool's own file adds to or overrides this one. Where both are
silent, the UI bible holds (section 9 for the tool's property rows,
UIC-53 for drag arrows on every number field). Tags as in README.md.

---

## 1. The big idea: a placer plus a copier

- DS-01 CONFIRMED (docs). Every tool in this batch is two parts working
  together:
  - a **placer** that works out *where* copies go: a mirror plane, a line,
    a 3D grid, or a circle/spiral round an axis;
  - a **copier** that decides *what* goes there and how it is joined:
    - **Copy** (Mirror, Clone, Array, Radial Array): puts a separate copy of
      the source at each spot;
    - **Sweep** (Lathe): joins the source's successive positions into one
      continuous surface.
- DS-02 [OP]. Build each placer and each copier once, as reusable parts.
  A toolbox button is one placer + one copier + starting values. Later
  variants (Instance, Replica, Transform, Curve Clone, Scatter Clone) are
  new pairs, not new tools.
- DS-03 CONFIRMED (docs). In the tool's property panel the two parts show
  as **two sections**, placer first, then copier. Section names [OP]:
  "Mirror Plane" / "Line" / "Grid" / "Circle" for placers, "Copies" or
  "Sweep" for copiers. (The reference uses its own internal-sounding
  names; ours just say what they are.)

## 2. What gets copied (the source)

- DS-10 CONFIRMED (docs). **Polygon mode:** the selected polygons of the
  active mesh(es). **Nothing selected:** every polygon of the active
  mesh(es) (Operandi's usual "no selection = everything" rule).
- DS-11 OPEN (DQ-2). **Edge or vertex mode:** provisional: the polygons
  that touch the selected edges or vertices are copied. Exception: Lathe
  sweeps edges and open curves directly (see that file).
- DS-12 CONFIRMED (docs). **Item mode:** the selected mesh items are copied
  as **whole new mesh items** (Radial Array docs: item selections make
  extra item layers with their transforms set). Each new item gets the
  source's name plus a number, the same naming Duplicate uses. [OP for
  naming.] The copies go at the same level of the Item List as their
  source, straight after it. [OP; the reference's Parent options are
  later.] Answers DUP-Q06: new items go straight after the source with
  the **same parent**; with Replace Source the source item itself moves
  into slot 0. A rotated item given a non-uniform size change keeps no
  shear: use the nearest plain scale and rotation [OP].
- DS-13 CONFIRMED (docs). Component-mode copies land **inside the same mesh
  item** as the source, as new geometry. With several active meshes
  (multi-active), each mesh copies its own selection into itself, using
  the one shared placer. [OP for multi-active.]
- DS-14 [OP]. Copies carry everything the source polygons carry: UVs,
  material tags, part tags, smoothing, subdivision flag and weights,
  vertex maps. Nothing is dropped.
- DS-15 later. Source = a named mesh, all background meshes, a random
  background mesh per copy, a preset shape. Build "the selection" only;
  leave the Source row out of the panel until the others exist (UIC-117).

## 3. Shared copier options

| Row | Kind | Start value | What it does |
|---|---|---|---|
| **Replace Source** | checkbox | off | DS-20 |
| **Flip Polygons** | checkbox | off (Mirror: see its file) | DS-21 |
| **Merge Vertices** | checkbox | off (Mirror: on) | DS-22 |
| **Distance** | number, length | 0 m CONFIRMED (probe) | DS-22 |

Labels [OP]: the reference says "Invert Polygons"; we use the plainer
**Flip Polygons** (tooltip: "turns the copies' normals the other way").
If Operandi's existing normals command uses another word, match that
instead. Bible: checkboxes per UIC-115, Distance
greyed (UIC-117) while Merge Vertices is off.

- DS-20 CONFIRMED (docs). **Replace Source.** On: the original geometry is
  removed and a copy takes its place in the first slot, so anything the
  placer does to each copy (jitter, scale, the start angle) also applies
  to the first one. Off: the original stays exactly as it was and copies
  are added.
- DS-21 CONFIRMED (docs). **Flip Polygons** turns every *new* polygon's
  facing the other way. It never touches the original. It is a manual
  fix for when copies come out inside-out (most often a Lathe drawn the
  "wrong" way round).
- DS-22 CONFIRMED (docs). **Merge Vertices.** On: after the copies are made,
  any new point that sits within **Distance** of another point (new or
  original) in the same mesh is welded to it, so the pieces join into one
  connected surface. Distance 0 welds only points that are exactly on top
  of each other (use a tiny tolerance, 1e-6 m, for "exactly" [OP]).
  Welding follows Operandi's existing Merge rules (polygon left with
  fewer than three corners is removed).
- DS-23 [OP]. Merge only ever welds a **new** point to something; two
  original points that already happen to be close are left alone. (This
  stops Mirror from quietly welding unrelated detail in the source.)
- DS-25 [OP] (answers DUP-Q01). **Internal walls.** After welding, if two
  polygons use exactly the same points (an original and a copy, or two
  copies), **both are removed**, in every tool. This is the general form
  of MR-43; Clone's CL-31 and Array's AR-32 follow it too.
- DS-26 [OP] (answers DUP-Q02). Points of the **same copy** never weld to
  each other, however close: only points from different pieces weld.
  Together with DS-23, each piece keeps its own detail.
- DS-24 [OP]. In Item mode the copies are separate items, so Merge
  Vertices does nothing and its rows are greyed.

## 4. Activating and driving a tool

- DS-30 CONFIRMED (docs). Clicking a toolbox button **arms** the tool and
  shows its property panel. Nothing changes in the scene yet.
- DS-31 CONFIRMED (docs). Two ways to make copies:
  1. **Click in a viewport.** The first press sets up the placer from
     where you clicked (each tool says how) and shows the copies at once.
     Dragging then adjusts it, and handles appear.
  2. **Type in the panel and press Apply** without clicking the view.
- DS-32 CONFIRMED (docs). **Live preview.** While the tool is active the
  copies update live as handles are dragged or values are typed.
- DS-33 [OP], matches Operandi's other tools. The result is **kept** when
  the tool is dropped, by whatever drops other tools today (Space,
  picking another tool, changing selection mode). The
  existing mid-tool undo design applies (mid-tool-undo-design.md): each
  press-to-release or field edit is one step while the tool is active,
  and after the drop one undo removes the whole operation.
- DS-34 [OP] (answers DUP-Q05). Clicking **in empty space away from the
  handles** after the first click starts a **new** placement from that
  spot for **Lathe, Radial Array and Clone** (the reference does this for
  Radial Array's centre). The previous result is kept as its own undo
  step. Exceptions: **Mirror** ignores later clicks off its handle (a
  second mirror would double the result), and **Array**'s left-drag
  anywhere edits spacing (AR-21), so it never restarts.
- DS-38 [OP] (answers DUP-Q07). Handle tooltips (UIC-05), shown after the
  usual hover delay:
  - Mirror: centre grip "Drag to move the mirror plane"; angle grip
    "Drag to hinge the mirror plane (Angle)"; second grip "Drag to tilt
    the mirror plane".
  - Circle (Lathe, Radial Array): start cube "Drag to set Start Angle";
    end cube "Drag to set End Angle"; axis end cubes "Drag to tilt the
    axis"; axis line "Drag to move the centre".
  - Array: axis grips "Drag to set X spacing" (Y, Z likewise).
  - Clone: tip grip "Drag to set the step between copies".
- DS-35 [OP]. Handle look follows Operandi's existing handles: axis colours
  (X red, Y green, Z blue), constant screen size, hover highlight. Plane
  handles reuse the corner-circle plane handle from the slice tools.
- DS-36 CONFIRMED (docs; Operandi already has it). Holding **Ctrl** before
  pressing constrains a drag to the first direction moved (Array docs;
  same rule as Operandi's handle modifiers spec).
- DS-37 CONFIRMED (docs). Snapping (X toggles it) applies to handle drags
  as it does for other tools.

## 5. Selection after

- DS-40 OPEN (DQ-3). Provisional: after the tool is dropped, the
  **original selection is kept** and the new geometry is **not** added to
  it. Exception: with Replace Source on, the first copy (which replaced
  the source) is selected in the source's place. In Item mode, the
  source item stays selected and stays the primary item.

## 6. Refusals

- DS-50 [OP]. With nothing to copy (an empty active mesh, or no item
  selected in Item mode) the tool still arms but a click in the view does
  nothing, and the panel's status line says "Nothing to copy: select
  polygons or an item." No dialog.
- DS-51 [OP]. A count that would make more than **1,000,000 new polygons**
  stops the preview and shows "Too many copies (N polygons)" until the
  values drop. Apply is greyed while that is shown.

## 7. Toolbox placement

- DS-60 CONFIRMED (earlier panel research). These buttons live on the
  **Duplicate** tab, between Deform and Edit, in these groups, top to
  bottom: **Extend**: Lathe · **Clone**: Clone · **Array**: Array,
  Radial Array · **Duplicate**: Mirror. Detriangulate is not a Duplicate
  tool: it goes in the **Polygon** tab and the Geometry > Polygon menu.
- DS-61 [OP]. The reference names the lathe button "Radial Sweep". Label
  ours **Lathe**, and put "Radial Sweep" in its tooltip and in the
  command search so people coming from the reference find it.
- DS-62 later. Flyouts for Instance / Replica / Transform variants and the
  Alt+click shortcut to the Instance variant.
- DS-63 [OP]. Each tool is also in the menu **Geometry > Duplicate**, with
  the same names.
