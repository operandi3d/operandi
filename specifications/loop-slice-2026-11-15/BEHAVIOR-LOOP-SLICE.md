# Loop Slice: drags and options (behaviour spec v0.1, 2026-11-15)

Behaviour only. Edit tab, Alt+C. Read with `BEHAVIOR-DRAG-LAYER.md` (same
folder): the mouse-button drags described there are shared by most tools,
and Loop Slice is just one row of that table.

Tags: **[docs]** = the reference app's public help. **[settings]** = the
reference app's plain-text settings and preset files (defaults, enable
rules, default drags). **[measured]** = already measured in an earlier
Operandi research spec (named). **[guess]** = our best reading, to confirm
(see `QUESTIONS.md`). REA-assisted: no.

---

## 0. The short version

- Operandi's slider bar near the top-left is **correct**: the reference
  app draws the same bar (a coloured strip near the top of the view) and it
  is the main way to place slices. Its default spot is 20 px in, 50 px
  down, 200 px long. [docs, settings]
- What is missing is the **drag-anywhere layer**: in the reference app you
  don't have to aim at the bar. Left-drag anywhere moves the current
  slice, right-drag adds or removes slices, middle-drag picks which slice
  is current (sideways) or digs the cut in (up/down). [docs, settings]
- Most greyed-out options shouldn't be greyed. Only five options have an
  "only when..." rule (section 4).

---

## 1. Starting the tool

- **LS-01** Select two neighbouring polygons, or the edge between them
  (or a ring of them). That picks the **ring**: the strip of polygons the
  cut runs across. [docs]
- **LS-02** The cut appears as soon as the tool is on, at the starting
  values (1 slice at 50%). [docs, settings; same as Add Loop's live cut]
- **LS-03** Operandi already lets you click an edge with nothing selected to
  pick a ring. Keep it; whether the reference app does the same, or whether
  clicking another edge while the tool is live moves the cut to that ring,
  is **LS-Q01**.

## 2. The slider bar (on-screen strip)

- **LS-10** A horizontal strip in the viewport's top-left. Its left end is
  0%, its right end 100% of the way across the ring. [docs]
- **LS-11** Each slice is a small **triangle** on the strip, numbered left
  to right (that's what **Current** counts). The current one is
  highlighted. [docs]
- **LS-12** Drag a triangle sideways: that slice's **Position** follows the
  pointer, 1:1 with the strip (pointer at 75% of the strip = 75%). [docs;
  1:1 is a guess, LS-Q02]
- **LS-13** The strip has a **dot at each end**. Drag a dot to make the
  strip longer or shorter (Length). Where the strip sits (Slider X/Y) is
  set in the panel; whether you can also drag the whole strip is LS-Q03.
  [docs]
- **LS-14** Clicking on the strip does what **Edit** says (section 3).

## 3. Drags in the viewport (off the strip)

What each mouse button does when you drag **anywhere in the viewport**
(on or off the mesh, not on the strip). These are the reference app's
defaults for this tool. [settings, docs]

| Drag | Sideways (left/right) | Up/down |
|---|---|---|
| **Left** | Moves the **current** slice (Position), when Edit = Move | nothing |
| **Right** | **Count**: right = more slices, left = fewer | nothing |
| **Middle** | **Current**: steps through the slices | **Inset**: pushes the cut in or out (see LS-41) |

- **LS-20** On or off the mesh makes no difference: it is a drag on the
  screen, not on the surface. [settings; the haul layer is screen-based,
  see DRAG-LAYER DL-02]
- **LS-21** Left-drag moves the **current** slice only. In Uniform and
  Symmetry modes the others follow the mode's rule (section 4, Mode).
  [docs + guess]
- **LS-22** How far a left-drag moves the slice, and which way is
  "positive": LS-Q02. Add Loop's numbers are a useful reference: there a
  separate drag changed Position by about 0.43% per pixel at one zoom and
  0.62% at another, so it follows the view scale [measured:
  add-loop-edge-slide AL-03]. Starting guess for Loop Slice: same feel as
  dragging along the strip (1 strip pixel = 1 screen pixel).
- **LS-23** Right-drag changes Count in whole steps. Pixels per step:
  LS-Q04 (use the drag layer's integer step, DL-05).
- **LS-24** Modifier keys: none are documented for this tool's drags.
  Ctrl/Shift drag behaviour is LS-Q05. Alt stays navigation (as everywhere).
- **LS-25** Edit = **Add**: a left-click on the strip adds a slice there
  and makes it current. Edit = **Remove**: a left-click on a triangle
  removes that slice. Whether Add/Remove also work by clicking in the
  viewport (not on the strip) is LS-Q06. [docs]
- **LS-26** The cut updates live during every drag and is kept when the
  tool drops (same as Add Loop and Bevel).

## 4. Options (panel order, with defaults)

Defaults are the reference app's starting values. [settings] "Greyed
when" = the only enable rules the reference app has for this tool
[settings]. Anything not listed there is **always on**.

| Option | Default | What it does | Greyed when |
|---|---|---|---|
| Edit | Move | What a left-click/drag does: Move, Add, Remove (LS-25) | never |
| Mode | **Free** | Free: each slice anywhere. Uniform: slices evenly spaced across the ring. Symmetry: slices mirrored about 50% | never |
| *(divider)* | | | |
| Current | first slice | Which slice is active; numbered left to right on the strip | never (first shown as 0 or 1: LS-Q07) |
| Count | 1 | How many slices | never |
| **Uniform** (button) | | One-shot: re-spaces Free slices evenly | never |
| *(divider)* | | | |
| Slice Selected | off | Cut only inside the selected polygons instead of running round the whole ring; follows corners | never |
| Keep Quads | off | Adds extra edges so the sliced area stays all quads | see LS-Q08 |
| Slice N-gon | off | Carry the cut through polygons with more than 4 sides | see LS-Q08 |
| Reverse Direction | off | Reads the profile from the other end | **no profile picked** |
| Reverse Inset | off | Flips the inset (in becomes out) | **no profile picked** |
| Keep Aspect | **on** | Sets Inset automatically from the profile's own shape | **no profile picked** |
| Preserve Curvature | off | New points follow a curve through the surface instead of a straight line | never |
| Tension | 100% | Strength of the curve. 0% = like off; negative pulls the curve in | **Preserve Curvature off** |
| Inset | 0 | How far the cut is pushed in/out (the profile's depth) | **a profile is picked and Keep Aspect is on** |
| Select New Polygons | **on** | Selects the new polygons and draws them in the guide colour | never |
| Draw Sliced Edges | **on** | Draws the cut edges in the guide colour while the tool is live | never |
| *(divider)* | | | |
| Split | off | Cut leaves two separate, overlapping edge loops (the mesh comes apart there) | never |
| Cap Sections | off | With Split: closes each open side with one cap polygon | **Split off** |
| Gap | 0 | With Split: a gap of this width around the cut | **Split off** |
| *(Profiles)* | none | A small preset list of 1D profiles; click one to use it as the cutter shape | never |
| Length | 200 px | Strip length | never |
| Slider X / Slider Y | 20 / 50 px | Strip position in the viewport | never |

**Operandi decision (Jason, 2026-11-16, LS-Q13):** Current, Count, the
Uniform button and Position also grey while a profile is picked: the
profile sets the loops (LS-74). Not a measured enable rule.

So, against today's Operandi: **Keep Quads, Split and Profile should never be
greyed**; Cap Sections and Gap light up with Split; Reverse Direction,
Reverse Inset and Keep Aspect light up once a profile is picked; Inset is
live with no profile (and with a profile when Keep Aspect is off).

The reference app also has two unattached "only when Slice Selected is
on/off" rules; which options they belong to is LS-Q08 (likely Slice N-gon
or Keep Quads).

### Options in more detail

- **LS-30 Mode.** Free (default): Count adds slices and each can be
  dragged anywhere. Uniform: slices sit at equal spacing (Count 3 → 25, 50,
  75%) and dragging one... LS-Q09 (does it scale the spread, or is it
  locked?). Symmetry: every slice has a partner mirrored about 50%;
  dragging one moves its partner the opposite way. [docs; detail guess]
- **LS-31 Uniform button** respaces whatever Free slices exist, evenly,
  then you're back in Free. [docs]
- **LS-32 Slice Selected**: cut only the selected polygons, turning at
  corners with the selection. [docs]
- **LS-33 Split / Cap Sections / Gap**: same meaning as in Polygon Slice
  and Add Loop. Add Loop has the same three options and the same rule
  (Caps and Gap need Split) [measured: add-loop-edge-slide AL-04, AL-07].
- **LS-34 Preserve Curvature / Tension**: same as Add Loop's Preserve
  Curvature. 100% default. [docs]

### Profile, Inset, Reverse, Keep Aspect (the "router" cut)

- **LS-40** A **1D profile** is a small curve (a cross-section shape).
  Picking one turns Loop Slice into a router: the profile decides how many
  loops are cut and where they sit across the ring, and how deep each one
  is pushed. [docs: "load that shape as a Loop Slice cutter"; profile
  points driving slice positions is from the procedural version's help;
  the depth part is a guess, LS-Q10]
- **LS-41 Inset** = the depth of that shape: how far the new loops are
  pushed along the surface normal (in = groove). Middle-drag up/down
  changes it (section 3). With no profile, Inset is still live; what it
  does then (all slices pushed by the same amount?) is LS-Q10.
- **LS-42 Keep Aspect** (on by default): Inset is set for you so the groove
  has the profile's own width-to-depth ratio, scaled to the ring's width.
  Turn it off to set Inset by hand. [docs + settings]
- **LS-43 Reverse Direction**: flips the profile end to end (read from the
  other side of the ring). **Reverse Inset**: flips in/out (groove becomes
  ridge). [docs]
- **LS-44** Operandi must ship its **own** profile shapes (never the
  reference app's content). The same profile list should serve every tool
  that takes a profile (see DRAG-LAYER section 3, "Profile layer").

## 5. How it sits in the tool pipe

- **LS-50 Falloff**: no effect. A one-slice cut was identical with and
  without a falloff [measured: falloff spec FAL-B3-05]. Show the falloff
  layer but it does nothing here, or hide it; builder's call (BIBLE).
- **LS-51 Symmetry**: with Symmetry on, the cut position is mirrored for
  rings on the other side, but only on rings that are selected [measured:
  symmetry SYM-B3-03]. This is separate from Mode = Symmetry (LS-30).
- **LS-52 Action centre**: present in the reference app's pipe but no
  visible role. **Snapping**: off by default for this tool. [settings]
- **LS-53 Background constraint**: the reference app has a Topology-layout
  preset of Loop Slice that runs with the background constraint on (the
  new points stick to a background mesh). Same settings otherwise.
  Operandi's background constraint (merged 11-15) can be reused. [settings]

## 6. What a builder can reuse

| Piece | Reuse from |
|---|---|
| Walking the ring, stopping at triangles/n-gons/boundaries | Add Loop (AL-08..AL-10: stop rules measured) |
| Same fraction on every crossed edge | Add Loop (AL-12) |
| Preserve Curvature + Tension | Add Loop |
| Split / Cap Sections / Gap | Add Loop and Polygon Slice (same rule set) |
| Keep Quads | Add Loop has a Keep Quads option of its own |
| Drag-anywhere hauling (L/M/R) | the shared drag layer, `BEHAVIOR-DRAG-LAYER.md` |
| Profile list and Reverse / Keep Aspect | a shared Profile layer (also Bevel, Edge Bevel/Chamfer, Edge Slide, Bridge, Sweep, Tube) |
| Symmetry, falloff, background constraint | the shared tool pipe as it is |
| Slider bar | Operandi's existing bar; add triangles per slice, Add/Remove clicks, end-dot resize |

Questions: `QUESTIONS.md`.

## 7. Measured (2026-11-16)

From the coordinator's brief to the builder (behaviour only). Fixture: a
flat 3x3 grid of 1 m squares, the two bottom-left squares selected, so the
ring is the bottom row. [measured]

- **LS-70** Count 1 at 50% cuts one loop: 16 -> 20 points, 9 -> 12 quads.
  Mode Symmetry, Count 2, Position 30% -> loops at 30% and 70%.
- **LS-71** Current counts from 0 and follows creation order; slices are
  not re-sorted by position. (This replaces "numbered left to right" in
  LS-11 and the options table.)
- **LS-72** Slice Selected with the two squares cuts just those two; the
  next square over takes an extra point and becomes 5-sided (16 -> 19
  points, 11 polygons). Keep Quads on gives the same result here: it only
  changes polygons inside the selection.
- **LS-73** Split doubles the new points (16 -> 24) with the same polygon
  count. Cap Sections adds nothing on an open flat strip (nothing closed to
  cap). Gap 0.1 gives two loops at 45% and 55%, centred on the cut.
- **LS-74** Profile cut: a T-slot profile made 8 loops from one Loop Slice
  (16 -> 48 points, 33 polygons), the slot pushed into the surface, scaled
  to fit the ring's width and centred. Position and Inset change nothing
  while Keep Aspect is on. The profile decides how many loops there are;
  Count stays 1.
- **LS-Q12** Which of Reverse Direction / Reverse Inset / Keep Aspect light
  up with a profile is open; until answered, follow the greying table in
  section 4.

