# Edit ▸ Center Selected menu — behavior spec v1

Provenance: the reference help page "Center Commands"; the reference app's
plain-text menu and command-help configuration (17.1v1) for menu layout,
labels and command names; Jason's screenshot of the Edit menu. Geometry
rules repeat the probe-backed Center group spec already handed off
(CEN-01..04). REA-assisted: no. No new probes (docs-only job, Jason
2026-11-11).

Tags: **[docs]**, **[cfg]**, **[obs]** = observed earlier for CEN-01..03,
**[chosen]** = Operandi's pick.

## 1. Summary

**Edit ▸ Center Selected** is a submenu of seven one-shot commands that
move the selected geometry so the center of its bounding box sits at the
world origin on the chosen axis or axes. No dialog, no options. [docs] It is
**the same command** as Operandi's Edit-tab Center group: every menu entry
runs Center Vertices with one axis argument. [cfg] So the menu needs no new
geometry code, only menu wiring.

## 2. Where it is [cfg][Jason's screenshot]

Edit menu, in its own group between dividers: … Reset Tool Attributes ─
**Center Selected ▸** ─ Center to Bounding Box ▸, Pivot to Bounding Box ▸ ─
Symmetry ▸ …

## 3. Entries (in menu order) [cfg]

| # | Label | Moves to 0 on | Invocation |
|---|---|---|---|
| 1 | **All** | X, Y and Z | `vert.center all` |
| 2 | **X** | X only | `vert.center x` |
| 3 | **Y** | Y only | `vert.center y` |
| 4 | **Z** | Z only | `vert.center z` |
| 5 | **XY** | X and Y | `vert.center xy` |
| 6 | **YZ** | Y and Z | `vert.center yz` |
| 7 | **ZX** | Z and X | `vert.center zx` |

- Command: `vert.center`, display name "Center Vertices", one argument
  `axis` with options x, y, z, all, xy, yz, zx (labels X, Y, Z, All, XY,
  YZ, ZX). [cfg]
- Axes not named are left untouched. [docs][obs CEN-01]
- No shortcuts are assigned in the menu. [cfg]

## 4. What it moves [docs][obs]

- One vertex: the vertex goes to 0 on the chosen axes. [docs][obs CEN-02]
- One edge: the edge's midpoint goes to 0; only its two points move.
  [docs][obs CEN-02]
- One polygon: the polygon's center goes to 0; its points move together.
  [docs][obs CEN-02]
- Several elements: the **center of their bounding box** goes to 0 (not the
  average of the points). Everything selected moves by the same offset, so
  shapes don't change. [docs][obs CEN-01]
- Nothing selected in a component mode: the whole mesh. [obs CEN-02]
- World space: the bounding box is measured in world coordinates, so on a
  moved item the geometry moves until its world-space center is at 0.
  [obs CEN-03] The help warns that on items carrying a transform the result
  can be surprising and suggests freezing the item transform first. [docs]
- One undo step per menu pick. [CEN-04]

## 5. Item mode [docs]

- The help says these commands are meant for component selections. To put
  an *item* at the origin you use the item's Reset (position) in its
  properties instead, which moves the item, not its geometry.
- Operandi (CEN-04): in Item mode the command does nothing and shows a
  status message. [chosen, provisional] Keep the same for the menu, and use
  the same message.
- [chosen, suggestion] Grey the seven entries out in Item mode rather than
  letting them be picked, to match the refusal. The reference's menu greying
  isn't documented, so this is our choice.

## 6. Differences from Operandi's Edit-tab Center group

| | Edit-tab Center group (CEN-01..04) | Edit ▸ Center Selected menu |
|---|---|---|
| Command | same (`vert.center`) | same |
| Entries | four buttons X, Y, Z, All; Alt+click X/Y/Z gives XY, YZ, ZX | seven plain menu items, no modifier needed |
| Order | X, Y, Z, All | **All first**, then X, Y, Z, XY, YZ, ZX |
| Geometry, world space, undo | as CEN-01..03 | identical |
| Item mode | refuses with a status message | identical (help: use item Reset instead) |

The only real work is the menu itself: a "Center Selected" submenu in the
Edit menu with the seven items, each calling the existing command.

## 7. Acceptance

Reuse CEN-01..03's cases through the menu: one per entry (All, X, Y, Z,
XY, YZ, ZX), each checking that only the named axes move to 0, plus one in
Item mode (no change, status message).

## 8. Neighbors (not in scope, for reference) [cfg]

The next group in the Edit menu acts on **items**, not geometry:
- **Center to Bounding Box ▸** Center, Top, Bottom, Left, Right, Front,
  Back: `center.bbox <center|top|bottom|left|right|front|back>`, which
  moves the item's center to that point of its bounding box.
- **Pivot to Bounding Box ▸** same seven entries: `pivot.bbox <…>`, which
  moves the item's pivot.
Research these on request. Their exact item/geometry behavior isn't
covered here.

## 9. Open questions

- Q1. Does the reference grey these entries out in Item mode, or let them
  run and do nothing? (Operandi: see §5.)
- Q2. Center or Pivot mode selections: the help doesn't say whether the
  command acts on them.
