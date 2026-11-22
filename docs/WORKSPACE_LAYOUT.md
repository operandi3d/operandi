# Workspace layout (mini milestone, 2026-11-03)

Goal: tear-off, docking and rearranging of panels and viewports (Vision UI02/UI03;
architecture section 15, decision D11, feasibility step P06). This first step
answers the toolkit question and lays the model down; the app itself is unchanged.

## What's here

- `crates/operandi-layout`: the layout model. A layout is the main window's tree of
  panes (Toolbox, Viewport n, Items, Properties) in splits and tab groups, plus
  torn-off windows, each with its own tree. Panes are ids, not widgets, so moving
  one never touches the document, its history or a viewport's camera. Operations:
  tear off, dock back, add a viewport (each with its own camera key), save and load
  as versioned JSON (refused if the version, pane set or ids don't check out). Six
  tests cover these.
- `examples/spike.rs`: the P06 check, a small window built on that model.
  Splits, tabs and drag-and-drop come from `egui_tiles` 0.14.1 (MIT/Apache, the
  newest release for our egui 0.33); torn-off panes are egui native windows.

Run it:

```bash
cargo run -p operandi-layout --example spike
```

## P06 checks (for Jason)

| Check | Expected |
|---|---|
| L-01 Rearrange | Drag the Items tab onto the viewport's left edge: it becomes a split there. Drag it onto Properties' tab bar: they share one tab group. |
| L-02 Tear off | Right-click the Properties tab > Tear Off: it opens in its own OS window; the rest of the layout closes the gap. |
| L-03 Dock back | Dock Back in that window, or close it: Properties comes back as a tab. |
| L-04 Two views | + Viewport adds a second view. Drag in each: their angles change separately. |
| L-05 Document survives | Add 1 a few times in the Toolbox, then tear off, dock and rearrange panes: the value and Undo steps stay the same everywhere. |
| L-06 Saved layout | Arrange something, Save Layout, quit, run again: the same arrangement and torn-off windows come back. |
| L-07 Scales | Move a torn-off window to another display (if you have one): it redraws sharply at that display's scale. |

## Step 1 in the app: the view area (2026-11-03)

Jason checked L-01…L-06 ("looks good"), so P06 passed. The app's view area is
now a layout of viewports; the toolbox and the Items/Properties panels stay fixed
for now.

- Each view has its own camera and GPU render target. `camera` is always the
  active view's, so tools, shortcuts and Frame commands act on it. A press inside
  a view makes it active; other views draw but take no input.
- Split View (view toolbar) adds a view beside the others, starting from the
  active camera. With two or more views each has a tab: drag it to rearrange,
  right-click it to Tear Off (its own window) or Close View.
- The arrangement is saved to `view-layout.json` in the app's data folder and
  restored at launch; cameras start from the active one.
- With one view nothing changes: no tab bar, same size, same behaviour.

| Check | Expected |
|---|---|
| V-01 One view | The app looks and works as before. |
| V-02 Split | Split View: two views. Orbit one; the other doesn't move. Set one to Top, the other to Perspective. |
| V-03 Active view | Click in either view: tools, selection and Frame All act there; handles work in both. |
| V-04 Tear off | Right-click a view's tab > Tear Off: it opens in its own window and still edits the same scene (undo works across). Close that window: it comes back. |
| V-05 Relaunch | Quit with two views, reopen: two views again. |

## Step 2: floating side panels (2026-11-03)

The Toolbox, the Items list and Properties can each float in their own OS
window, outside the main one (Jason: "torn off into floating panels outside the
main app window"). Window > Float Toolbox / Items / Properties, or right-click
the Scenes or Properties tab > Float in Its Own Window. Closing a panel's window,
or unticking it in the Window menu, docks it back. When the side panel empties
the view grows into its space; with both right-hand panels floating, the command
line keeps a strip at the bottom. Floating panels and their window sizes are
saved in `panel-layout.json` and come back at launch.

Provisional: shortcuts are read from the main window, so they don't fire while a
floating panel's window has keyboard focus (click the main window first).

| Check | Expected |
|---|---|
| P-01 Float | Window > Float Toolbox: the toolbox opens in its own window; the view widens. Its tools still work. |
| P-02 Items / Properties | Right-click the Scenes tab > Float in Its Own Window: Items floats, Properties fills the right side. Same for Properties. |
| P-03 Dock | Close a floating panel's window: it's back in its side panel. |
| P-04 Relaunch | Quit with panels floating and reopen: they float again, at their sizes. |

## Known issues from Jason's QA (2026-11-03)

- P-01: Float Toolbox opened the toolbox as a macOS window tab of the main
  window (shrinking the main window to the toolbox's size) instead of a separate
  window. Tearing the tab off then floats it, but it's confusing. Likely macOS
  automatic window tabbing joining same-app windows; floating windows need tabbing
  turned off (and possibly a utility/panel window style).
- P-02: right-click Scenes > Float does the same (opens as a tab of the main
  window). Otherwise the floating Items panel works ("a good first step").
- P-03: closing floating windows works well.
- Request: each panel gets a small dot in its top-left corner, like the
  reference. Hovering the dot outlines the whole panel in orange, so it's clear
  the panel is about to be changed; the dot is where the panel is grabbed to
  move, float or dock it. Jason-specified, provisional until measured
  (LAYOUT-Q01: what the reference's dot does on click, drag and right-click).
  Built 2026-11-06 from PANEL-DOT v1: see PANEL_CORNER_DOT.md.

## Not decided or not done yet

- Docking a side panel to a different side, or into a tab beside a view
  (views dock beside each other by their corner dot since 2026-11-06).
- Handles show in every view but respond only in the active one; a click in
  another view first makes it active.
- Saved Layouts with names, a Layouts menu, reset to default, and which panels
  exist beyond the four here (Tool Properties, a UV view).
- Several open scenes, each a root row in Items (Jason, 2026-11-03).
