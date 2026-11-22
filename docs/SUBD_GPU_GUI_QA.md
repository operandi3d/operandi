# Big subdivision surfaces: GUI check

Branch `claude/project-thread-pj3a5s`. Design: `SUBD_GPU_DESIGN.md`.

1. New scene, Sphere primitive at its most segments (about 250k
   polygons), then Subdivide (Faceted) twice: about 4M polygons. Tab. The
   smooth surface appears (not the cage); the GL counter shows about 128M
   triangles. Orbit: smooth, about 25–30 fps on the M1 Max.
2. Same sphere: Polygon mode, click a polygon. It highlights in the
   selection fill and its outline follows the surface.
3. Item mode: the whole sphere takes the selection colour and its wire
   the highlight colour.
4. Zoom the camera inside the sphere: the wire still shows all round
   (nothing goes missing).
5. A plane of 1000 × 1000 polygons, Tab, look at it from below: the wire
   shows as it does on smaller meshes.
6. A small Tab cube and a 50k-polygon Tab mesh: look and drag exactly as
   before (they keep the stored path).
7. With little memory free (or a huge mesh, e.g. 20M polygons), Tab:
   a message says how much memory it needs and nothing changes.

Pass 2 (drags, textures, wire fade):

8. On the 4M-polygon Tab sphere, Polygon mode, select a few polygons and
   drag them with Move: the surface follows smoothly (about 20+ fps), and
   undo puts it back.
9. Same sphere zoomed out: with Preferences > Display > Fade Wireframe on
   Small Polygons on, the wire fades to clean shading but the outline
   stays; turn it off and the dense wire comes back.
10. Open fixtures/viewport-image-textures/image-layers.opscene, Tab the
    "UV Image" item with Subdivision Level high enough (or Subdivide it
    first so it passes 500k pieces): the image stays on the surface, as
    after Freeze.
