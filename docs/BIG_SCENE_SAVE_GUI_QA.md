# Big scene save: GUI check

Branch `big-scene-save`. QA app from `scripts/bundle-dev.sh`.

1. File > Import… a big OBJ (e.g. Guitar.obj). Title reads "Untitled *".
2. File > Save As… `Guitar.opscene`. The title becomes "Guitar" (no "*"),
   the status line says "Scene saved".
3. Close the scene, File > Open… `Guitar.opscene`: same items and polygon
   count as after import.
4. Change something, File > Save (over the same file): saves without delay
   beyond the write itself.
5. Save As… into a folder you can't write to (e.g. `/System/Guitar.opscene`
   or a locked folder): a "Save" dialog explains why; OK closes it; the
   title still shows "*" and the old name.
6. Open a damaged `.opscene` (e.g. a text file renamed): an "Open" dialog
   explains why; the current scene is unchanged.
