# Selection tools: GUI check

QA app: `~/Developer/Operandi-Builds/Operandi deform-finish <commit>.app`. Start with a Cube, Segments 6 x 6 x 6, Select tab.

1. **Shortest Path**: Vertex mode, click a corner, Shift+click the far corner of the same face, click Shortest Path: the points between are added. Same in Edge and Polygon mode. One Cmd+Z takes the path back.
2. **Shortest Path Mode**: run `shortest path mode`, then click one point and Shift+click another: the path comes with it.
3. **Sharp...**: Edge mode, Sharp...: dialog Minimum 89.55°, Maximum 180°; OK selects the cube's 12 outer edges (all 6 x 4 segments along them).
4. **Coplanar Edges...**: on a triangulated plane (Polygon ▸ Triple), OK selects the inner edges.
5. **Colinear Vertices...**: on a polygon with an extra point mid-side, OK selects that point.
6. **Diagonal...**: on a triangulated plane, OK selects one diagonal per quad (the edges Detriangulate removes).
7. **Coplanar Polygons Tool**: select one polygon on a face, open it: the whole flat face selects; change Angle, then Cancel: back to the one polygon.
8. **Conditional Loop**: select one edge in the middle of a face, click it: the loop round the cube selects. Conditional Loop Tool with Max Loop 4: only 4 edges.
9. **Lazy Selection**: click it (it lights; check the button itself toggles, a headless test only reached it from the command line), then click a little way off a point: the point still picks.
