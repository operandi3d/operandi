# Project file paths: GUI check (branch project-file-paths)

For Jason, at the mouse, about 10 minutes. Use the QA app named in the
hand-back. Make a throwaway project; don't use a real one.

1. **New project.** File > New Project… in a new folder `PathsTest`.
   Finder: it has `materials/` and `scripts/` beside the older folders.
2. **Copy on add.** Add a Backdrop, Load Image… a PNG from your Desktop.
   Status line ends "Copied <name> into the project's references folder";
   Finder shows it in `PathsTest/references/`; the Desktop file is still
   there.
3. **Same file again.** Load the same PNG on a second Backdrop: no
   "Copied" note, no second file. Load a different PNG with the same name:
   it arrives as `<name> (2).png`.
4. **Image map and IES.** Add an Image Map layer and pick a picture from
   outside the project, and a Photometric light with an IES file: both
   land in `sourceimages/`.
4b. **Drop.** Drag a JPEG from the Finder onto the viewport: a Backdrop,
   and the picture is copied into `references/`.
4c. **Preset Browser.** Double-click a library material with texture
   maps, then an HDRI, then an environment preset: their pictures appear
   in `sourceimages/` (a second use reuses them, no "(2)" copies). The
   library files are untouched.
5. **Undo.** Undo the last image change: the scene goes back; the copied
   file stays in the folder (by design).
6. **Preference off.** Preferences > File I/O > Projects: untick Copy
   Added Files into the Project. Load another outside picture: no note, no
   copy, the scene uses the original. Tick it again.
7. **Save and move.** Save the scene into `scenes/`. Quit. In Finder,
   duplicate the whole `PathsTest` folder to another place and rename the
   original out of the way. Open the copy's scene: every picture and the
   IES file show; Properties paths point into the copy.
8. **Outside stays outside.** With the preference off, add a picture from
   the Desktop, save, reopen: it still shows (absolute path).
9. **Default folders.** Render, then Render window Save Image: the panel
   starts in `renders/`. Preview > Options > Save Image: `renders/`.
   File > Export > OBJ: `exports/`. Export Material as MaterialX and Save
   Preset for a material: `materials/`.
10. **No project.** Restart without a project (or use a scene outside
    it): Load Image copies nothing and panels start where they did before.
