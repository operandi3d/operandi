# Snapping settings

**SNP-ST-01** Snapping is one on/off switch, independent of which targets
are chosen. In the observed session it was off.

**SNP-ST-02** Targets are chosen per **mode**: Global, Component and Item.
Each mode keeps its own on/off flag for each target type: Grid, Vertex, Edge,
Edge Center, Polygon, Polygon Center, Pivot, World Axis, Straight Line,
Right Angle, Intersection, Box. In the observed session only Global/Vertex
was on; every Component and Item flag was off. *(Whether these are factory
values is open: SNP-Q01.)*

**SNP-ST-03** Snapping does not alter a transform entered as numbers. Setup:
two quads in one mesh, quad 1 with points (0,0,0) (1,0,0) (1,1,0) (0,1,0),
quad 2 with points (3,0,0) (4,0,0) (4,1,0) (3,1,0). Snapping on with Vertex
targets. Point (1,1,0) moved numerically by (1.98, 0.01, 0) ends at
(2.98, 1.01, 0), not at the nearby point (3,1,0). With Grid targets also on,
point (1,0,0) moved by (0.33, 0.27, 0) ends at (1.33, 0.27, 0).
