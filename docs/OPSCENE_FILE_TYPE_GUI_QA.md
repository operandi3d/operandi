# .opscene file type: GUI check (macOS)

Use the QA build `Operandi opscene-file-type <commit>.app` in
`~/Developer/Operandi-Builds`. Use copies of scenes, not originals.

1. Open the QA app once and quit, so Finder registers it. If other
   Operandi QA apps are installed, select a `.opscene` file, Get Info >
   Open with > pick this build > Change All….
2. Finder shows `.opscene` files with the dark page and orange O icon, and
   Get Info's Kind says "Operandi Scene".
3. App not running: double-click a scene. The app starts and the scene
   opens in the first tab (no extra Untitled tab).
4. App running with an edited scene: double-click another scene. It opens
   in a new tab; the edited one keeps its changes.
5. Drag two scenes onto the app's Dock icon. Both open, each in its own tab.
6. Double-click a scene that's already open: Operandi switches to its tab.
7. With File > Open's dialog showing, double-click a scene in Finder. It
   opens after you cancel the dialog.
8. Drop a scene onto the window: still opens as before.
