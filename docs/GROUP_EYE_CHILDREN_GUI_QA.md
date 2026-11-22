# Items list eye hides children: GUI check

GV-01..06 (2026-11-10). Branch `group-eye-children`. Use the QA app, not
your installed Operandi. Auto Hide Child Items (Preferences > Defaults >
Mesh Items) starts on.

1. Add a Cube and a Sphere. Drag the Sphere's row onto the Cube's row so
   it becomes the Cube's child. Click the Cube's eye: both leave the
   viewport. The Cube's eye closes; the Sphere's eye stays open but faded.
   Cmd+Z brings both back in one step.
2. Click the Cube's eye again (after hiding it): both come back. Now hide
   the Sphere on its own, then hide and show the Cube: the Sphere stays
   hidden.
3. Ctrl+click (and Cmd+click) the Cube's eye: same as a plain click, both
   hide.
4. Turn Auto Hide Child Items off. Click the Cube's eye: only the Cube
   hides, the Sphere stays. Turn the preference back on.
5. Select both meshes and choose **Parent to Group Locator**. Click the
   locator's eye: everything under it hides; click again, it comes back.
   Save, hide the locator, save again, reopen: the meshes are still hidden
   with their eyes faded.
6. Add a third mesh outside the group. Alt+click the Group Locator's eye:
   the group and its meshes show, the third mesh hides. Alt+click again:
   it comes back.
