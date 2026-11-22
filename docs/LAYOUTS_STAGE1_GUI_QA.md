# Layouts stage 1 — GUI hand test

Branch `layouts-stage-1`. Spec: `specifications/layouts-2026-11-16`. Use the
QA build with its own data folder (`OPERANDI_DEV_DATA`), never your real one.
Window about 1440 × 900.

## Before you start

- [ ] Copy a QA data folder that has an old arrangement in it (a split view,
      a wider right panel, Groups in front in the upper tabs). Open it once
      with the build before this branch, note how it looks, quit.

## Migration: Model looks exactly as before

Automated: `migrated_model_paints_exactly_what_the_old_loaders_paint`
(layouts_tests.rs) draws the whole window from old files through the old
loaders and through the migration and finds every painted shape the same.
A side-by-side screenshot of this build and `Operandi uv-stage-1
55fe9ec5.app` on copies of one data folder is still to do by hand (step 1).

1. [ ] Open the same data folder with this build. The window opens on
       **Model**: same split views, same panel widths, same tab in front in
       each tab group, same Items filter, same per-view display settings.
2. [ ] Quit and reopen: still identical.
3. [ ] The data folder now has `layouts.json`, and `Layouts/Model.json`
       (only because this arrangement differs from the shipped Model).

## Switching

4. [ ] Click **UV** on the layout bar: UV view on the left, Perspective
       view on the right, 50/50; toolbox at its UV tab; UV Maps in front in
       the upper tab group.
5. [ ] In UV: orbit the 3D view, click the Edge toolbox tab, drag the right
       panel wider. Click **Model**: Model is exactly as you left it (views,
       cameras, toolbox tab, widths, tabs).
6. [ ] Click **UV** again: the orbit, Edge tab and wider panel are back
       (the UV tab isn't forced again).
7. [ ] **Layout > Layouts** lists Model and UV, the current one ticked;
       choosing one switches the same way as the bar.
8. [ ] Type `layout.restore UV` and `layout.restore Model` in the command
       line: the same switches. `layout.restore Nowhere` only reports it.
9. [ ] With a tool open (Move), clicking UV says "Drop the tool before
       switching layouts" and nothing changes.
10. [ ] Switching never changes the scene, the selection or Undo's label.

## Remembering

11. [ ] In UV, split a view and quit. Reopen: the app opens on UV, split.
12. [ ] Double-click the **UV** tab, answer Reset: the shipped UV comes
        back and `Layouts/UV.json` is gone.
13. [ ] Fresh data folder (empty): opens on Model with today's default
        look; no `Layouts/` files until you change something.
14. [ ] Give two 3D views different pans, orbit angles, zooms and view
        types. Pan and zoom the UV view. Quit and reopen, then switch
        between layouts and check that each view comes back as left.

## UV Editor (unchanged)

15. [ ] Texture > Open UV Editor still opens the 800 × 600 window with the
        tool strip; double-clicking a map in UV Maps still opens it.
