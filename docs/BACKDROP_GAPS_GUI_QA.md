# Backdrop gaps (BD-60..62, BD-12) — GUI check

Branch `backdrop-gaps`. One pass, Jason or Codex. Use a QA build with its own
data folder; any small PNG or JPEG will do (a wide one, e.g. 400 × 200 px,
makes the sides easy to tell apart).

## BD-61 Auto Fit uses the active meshes

1. Add two meshes far apart (e.g. a Cube at the origin, a second Cube moved
   to X 5) and a third at X −10. Add a Backdrop (Front), load a picture,
   turn Use Uniform Size on.
2. In the Items list select the Backdrop, then Ctrl/Cmd-click the first two
   meshes. Press Auto Fit. **Expect:** the picture spans the first two
   cubes' combined height, centred between them; the third cube is ignored.
3. Hide the second cube (eye), keep it selected, Auto Fit again.
   **Expect:** same result: a hidden selected mesh still counts.
4. Select only the Backdrop, after last having a single mesh selected.
   **Expect:** Auto Fit fits that last mesh (it stays active).
5. New scene, add only a Backdrop (Uniform Size on), Auto Fit.
   **Expect:** status "Auto Fit needs an active mesh with geometry"; nothing
   moves. Undo history has no new step.

## BD-60 Drop a picture into a view

1. Use a quad layout (or split views) with Top, Front, Right and
   Perspective. Drag a PNG from Finder onto the **Top** view.
   **Expect:** a new "Backdrop Item", alone selected, at the origin, showing
   the picture in Top only (and Perspective). Its Projection Type is Top.
2. Cmd+Z once. **Expect:** the Backdrop and the image entry are both gone
   (the Image list in Properties no longer offers it).
3. Drop onto Front, Right: Projection Front / Right. Drop onto
   Perspective, and onto a camera view: Projection **Front**.
4. Drop the picture onto the Items list or Properties panel.
   **Expect:** no Backdrop; the picture appears in a Backdrop's Image list
   (and `image.list` in the command line shows it).
5. Drop a `.opscene` or `.lxo` file onto a view. **Expect:** it opens as a
   scene, as before.
6. Command line: `backdrop.add name:Plan {image:/path/pic.png} project:top`
   makes a Top Backdrop named Plan; without `project` it faces Front.
   `clip.addStill {filename:/path/pic.png}` adds only the image entry.

## BD-62 Pen "Backdrop Item" UVs

1. Front view: drop a picture so a Front Backdrop shows it. Select a mesh,
   Polygon mode, Pen, tick Make UVs.
2. Click the four corners of the picture, then a fifth point well outside
   it, Drop. Open the UV view (or check the Texture map in the vertex map
   list). **Expect:** the corners map to 0,0 / 1,0 / 1,1 / 0,1 (bottom-left
   is 0,0); the outside point has a UV below 0 or above 1.
3. Turn the Backdrop's Flip on, repeat. **Expect:** U is mirrored (the
   picture's bottom-left corner now gets U 1).
4. Repeat drawing in Perspective, or in Front with Show Backdrop off.
   **Expect:** UVs are world X and Y as before.
5. Two Front Backdrops overlapping: UVs follow the first created (the one
   drawn on top).

## BD-12 Camera Backdrop

1. Add a Camera, make it the render camera. Add a Backdrop with a wide
   picture, set Projection Type to Camera.
2. **Expect:** it shows in no ordinary view (Front, Top, Perspective…).
3. Look through the camera (view menu > Camera). **Expect:** the picture
   fills the render frame exactly, stretched to the frame's shape. Moving
   or scaling the Backdrop item changes nothing there.
