# Defaults (values after Reset)

**PRIM-D-CUBE** Cube: cenX 0.0; cenY 0.0; cenZ 0.0; sizeX 1.0; sizeY 1.0; sizeZ 1.0; segmentsX 1; segmentsY 1; segmentsZ 1; radius 0.0; segmentsR 3; sharp 1; axis 1; uvs 1; minX -0.5; minY -0.5; minZ -0.5; maxX 0.5; maxY 0.5; maxZ 0.5; patch 0; flip 0; handleMode resize; showCorner 0; uvCompensation 0; uvGaps 0.0; bulge 0.0.

**PRIM-D-SPHERE** Sphere: cenX 0.0; cenY 0.0; cenZ 0.0; sizeX 0.5; sizeY 0.5; sizeZ 0.5; sides 24; segments 12; axis 1; uvs 1; flip 0; minX -0.5; minY -0.5; minZ -0.5; maxX 0.5; maxY 0.5; maxZ 0.5; handleMode center; showCorner 0; uvCompensation 0; uvGaps 0.0; method globe; order 0; polType face.

**PRIM-D-CYLINDER** Cylinder: cenX 0.0; cenY 0.0; cenZ 0.0; sizeX 0.5; sizeY 0.75; sizeZ 0.5; sides 24; segments 1; axis 1; uvs 1; flip 0; minX -0.5; minY -0.75; minZ -0.5; maxX 0.5; maxY 0.75; maxZ 0.5; handleMode resize; showCorner 0; uvCompensation 0; uvGaps 0.0; polType face; capTop single; capBottom single; edgeWeight 1.0; bulge 0.0.

**PRIM-D-CONE** Cone: cenX 0.0; cenY 0.0; cenZ 0.0; sizeX 0.5; sizeY 0.75; sizeZ 0.5; sides 24; segments 1; axis 1; uvs 1; flip 0; minX -0.5; minY -0.75; minZ -0.5; maxX 0.5; maxY 0.75; maxZ 0.5; handleMode resize; showCorner 0; uvCompensation 0; uvGaps 0.0; polType face; capTop single; capBottom single; edgeWeight 1.0; bulge 0.0.

**PRIM-D-ELLIPSOID** Ellipsoid: cenX 0.0; cenY 0.0; cenZ 0.0; sizeX 0.5; sizeY 0.5; sizeZ 0.5; sides 24; segments 12; axis 1; uvs 1; flip 0; minX -0.5; minY -0.5; minZ -0.5; maxX 0.5; maxY 0.5; maxZ 0.5; handleMode center; showCorner 0; uvCompensation 0; uvGaps 0.0; hole 2.0; bulgeTop 0.0; bulgeSide 0.0; polType face; ringRadius 1.0; xSection 0.5; height 0.5.

Accepted values: Sphere method globe, qball, tess; polygon type face, subdiv, psubdiv; Cylinder and Cone caps none, single, quads, radial; Cube axis 0 (X), 1 (Y), 2 (Z).

(Option names are written as short camelCase keys of the tool's options; the same keys are used in ORACLES.json.)
