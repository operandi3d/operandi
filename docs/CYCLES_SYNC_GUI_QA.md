# Cycles sync — GUI check

QA build with Cycles (`OPERANDI_CYCLES=1 scripts/bundle-dev.sh`). Use a scene
with a few meshes, a textured material and a light.

1. Open the Preview (GPU on). Wait for it to finish once.
2. Orbit the viewport camera a little. The Preview should restart noticeably
   faster than the first time (no long "preparing" pause).
3. Move one item. The Preview restarts quickly and shows it moved.
4. Change a material colour. Only the colour changes; the rest looks the same.
5. Smooth shading: a cube at 40° Smoothing shows hard edges, a sphere looks
   smooth, and both match the viewport's shading. Set Smoothing to 50 %:
   the render should soften like the viewport does.
6. Clone an item several times (Duplicate tools). The Preview still renders
   them all correctly.
7. Overwrite a texture image file on disk, then nudge the camera. The
   Preview shows the new image.
8. Close and reopen the Preview: it renders normally again.
9. Press F9 for a final render: it matches the Preview's look.
