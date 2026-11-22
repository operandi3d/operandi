# Proposal: bible section "UI part names"

A proposed new section for [docs/UI_BIBLE.md](../UI_BIBLE.md). Not yet in
the bible; the maintainer adds it. The names are the ones agreed for the UI
map ([ui-map.md](ui-map.md)) on 2026-11-20; the map's numbers are given in
brackets. Tool handles and Tool hint don't show on the map's empty
window.

**Numbering.** UIC-240..259 is taken by §16 Future doors (UIC-240..244 in
use, the rest reserved). This section is §17 and uses the next free block,
**UIC-260..289**; UIC-260..284 are used below, 285..289 are left for later
names.

**Rule index line to add:** `· 17 UI part names UIC-260..289`

---

## 17. UI part names

Docs, the manual, tooltips, menus, code comments, PRs and reviews call each
part of the window by the name below. When another rule uses older wording
for the same part, the name here wins; the older words are listed so they
can be replaced as those rules are next touched.

- **UIC-260** The app mark at the top-left of the window is the **Logo**. [1]
- **UIC-261** The row of layout tabs (Model, UV, Paint…) is the **Layout
  bar** (UIC-11). [2]
- **UIC-262** The row of drop-down menus is the **Menu bar** (UIC-10). [3]
- **UIC-263** The Save, Undo and Redo buttons together are **Save, Undo and
  Redo**. [4]
- **UIC-264** The state buttons for points, edges, polygons and the other
  modes (UIC-24) are the **Selection modes**. They sit at the left of the
  mode bar. [5]
- **UIC-265** The buttons that change how every tool behaves (action
  centre, falloff, symmetry, mesh constraint, snapping, work plane, tool
  drop; the set UIC-63 lists) are the **Modelling aids**. [6]
- **UIC-266** The Render and Preview buttons together are **Render and
  Preview**. [7]
- **UIC-267** The tabs naming the lists that share a panel slot, with their
  **+**, are the **Panel-group tabs** (§10, UIC-155). [8]
- **UIC-268** Move, Rotate, Scale and Transform, shown above the toolbox in
  every tool tab, are the **Transform tools**. Replaces "the rows shared by
  every tab". [9]
- **UIC-269** The panel of tool buttons is the **Toolbox** (UIC-197). [10]
- **UIC-270** The strip of tabs on the toolbox (Create, Select, Deform…) is
  the **Tool Tabs**. They are not the toolbox set buttons of UIC-197
  (a separate row above the toolbox, still to be built); UIC-197 keeps
  its wording. [11]
- **UIC-271** The always-visible dot at the top-left of a view is the
  **View dot** (UIC-71, UIC-193). Replaces "corner dot" and "top-left
  dot". [12]
- **UIC-272** The bar along the top of a view (camera, look, what it shows)
  is the **View header**. Replaces "viewport header" (UIC-25) and "3D view
  header". [13]
- **UIC-273** The pan, turn, zoom, maximise and settings buttons at the
  right of the view header are the **Navigation buttons**. Replaces "the
  header's corner buttons". [14]
- **UIC-274** The Items panel's tree is the **Items list** (§10). [15]
- **UIC-275** A view showing the scene in 3D is a **3D view**. Replaces
  "viewport" wherever it means a 3D view (UIC-74, UIC-75, UIC-243,
  UIC-244); where it means any kind of view (UIC-60, UIC-70, §7's title) it
  becomes "view". Named panels and commands keep their names
  (Viewport Properties, Viewport Group, Tabbed Viewport). [16]
- **UIC-276** The active tool's arrows, rings and squares in a view are the
  **Tool handles**. [not on the map]
- **UIC-277** The active tool's settings panel is **Tool Properties** (§9).
  [17]
- **UIC-278** The selected item's settings panel, with page tabs down its
  side, is **Properties** (§9). [18]
- **UIC-279** The floor grid new shapes sit on is the **Work plane grid**.
  Replaces "Grid and Work Plane" when it means the grid itself. [19]
- **UIC-280** The X/Y/Z marker in a view's corner is the **Axis marker**.
  Replaces "axis gizmo". [20]
- **UIC-281** The line telling you how to finish or leave the active tool is
  the **Tool hint**. Replaces "active-tool exit hint". [not on the map]
- **UIC-282** The lines showing what is selected, the GL count and the grid
  size are the **Readout**. Replaces "readout lines". [21]
- **UIC-283** The line for the app's short messages ("Ready", what just
  happened) is the **Message line**. [22]
- **UIC-284** The one-line field for typing commands is the **Command
  entry** (UIC-228, UIC-234), labelled "Command" on screen. [23]
