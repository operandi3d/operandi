# Falloffs batches 2–6: GUI check

Build: `Operandi project-thread-m0er89 <commit>.app` in `~/Developer/Operandi-Builds`.

1. Make a sphere or a subdivided plane, select a few polygons in the
   middle, press W (Move), then Falloff ▸ Soft Selection. Coloured dots
   show around the selection, fading out. Tool Properties shows a "Soft
   Selection Falloff" section: Radius, Use Connectivity, Shape, In, Out,
   Show Vertex, Show Edge, Mix Mode.
2. Drag the Move handle: the selection moves fully and the geometry around
   it follows less and less. Undo: one step back.
3. Right-drag in the view to the right: the dots spread (Radius grows);
   to the left, they shrink. Type a Radius in the panel: same.
4. Tick Show Edge: the affected edges get coloured too.
5. Falloff ▸ Coplanar on a cube with one face selected: Angle 0 colours
   only that face's side; on a bevelled box, raise Angle to 50°: the
   bevel strips join in, weaker the more they turn away.
6. Falloff ▸ Element, keep Move on. Hover over the model: the vertex,
   edge or polygon under the pointer highlights. Press on it and drag: only
   it moves. Set Range to 0.5 and drag another one: the area around it
   follows softly. Set Connected Elements to Rigid Connections: Range
   greys out and the whole piece moves.
7. Each Element drag is its own undo step; Esc over the view (tool idle)
   clears the falloff.
8. Falloff ▸ Add lists Invert, Linear, Radial, Element, Vertex Map,
   Selection, Soft Selection, Coplanar.

Batch 3:

9. Make a cylinder primitive standing up, select all, press W, Falloff ▸
   Cylinder. It fits the selection: rings and lines show a tube along Y.
   Drag the Move handle sideways: the middle moves fully, the wall less.
   Set Axis to X: the tube turns.
10. Each falloff centre (and Linear's Start and End) has three short
    coloured lines. Drag the red one: only X changes.
11. Falloff ▸ Radial, then press and drag in empty space: a new sphere is
    drawn from the press, sized by the drag. The next press acts normally.
    The Draw in View button arms it again.
12. Move the mesh item (Item mode) away from the origin, back to Vertex
    mode, Falloff ▸ Linear: the falloff stays on the mesh (own space).
    Tick Use World Transforms: it jumps to world positions.
13. Two copies of a mesh side by side, both selected in Polygon mode, a
    Radial falloff, World off: both deform alike; World on: one shared
    field across them.

Batch 4:

14. A subdivided plane or sphere, Vertex mode, nothing selected, press W.
    Falloff ▸ Screen, then press on the mesh and drag outwards: a circle
    follows; dots show inside it. Drag the Move handle: only the circled
    area moves, softly.
15. Orbit the camera: the weights stay on the same points.
16. Falloff ▸ Lasso, drag a rectangle over part of the mesh: points inside
    get full weight. Set Style to Ellipse and press Draw in View, draw
    again. Raise Soft Border to 30 px: the edge softens.
17. Falloff ▸ Airbrush. Type a Move Y of 0.2 in the panel first, then drag
    over the mesh: it rises where you scrub, more with each pass.
    Right-drag sizes the brush circle. Clear Paint starts again.

Batch 5:

18. A dense plane, all selected, W, Falloff ▸ Noise. Drag the Move handle
    up: the surface rises in soft blobs. Lower Scale: smaller blobs.
19. On a mesh with a morph map or a colour map (e.g. an imported one),
    Falloff ▸ Vertex Map and pick the map in the menu's Weight Map list
    (marked "(morph)" or "(colour)"): the dots follow the map.
20. Falloff ▸ Image, Browse… to any picture: the dots follow its
    brightness over the view. Tick Use Selected UV Map on a mesh with UVs:
    they follow the UVs instead. Change Scale or Rotation: the dots update.

Batch 6:

21. A dense plane, all selected, W, Falloff ▸ Path. Click three or four
    times across the plane: a curve appears through the clicks, dots along
    it. Drag the Move handle up: a ridge rises along the curve. Press Stop
    Adding Points, then drag one of the path points: the ridge follows.
    Change Size: wider or narrower ridge.
22. A sphere with a few dents pushed in, all selected, Falloff ▸
    Curvature: bumps show warmer dots than dents. Scale −1 swaps them.

