# Point and polygon counts per case

Every case starts from an empty mesh; unlisted options at their reset values. Full geometry and UVs: ORACLES.json.

| Case | Tool | Settings | Points | Polygons | Message |
|---|---|---|---|---|---|
| cone-5-quads | Cone | sides 5, capBottom quads | 11 | 12 |  |
| cone-8-quads | Cone | sides 8, capBottom quads | 18 | 20 |  |
| cone-8 | Cone | sides 8 | 9 | 9 |  |
| cone-sizeY0 | Cone | sides 8, sizeY 0 | 8 | 1 |  |
| cone-sizeYneg | Cone | sides 8, sizeY -0.75 | 9 | 9 |  |
| cube-bulge0.2-seg2 | Cube | bulge 0.2, segmentsX 2, segmentsY 2, segmentsZ 2 | 26 | 24 | Refused: Corner radius must be non-zero. |
| cube-default | Cube | defaults | 8 | 6 |  |
| cube-radius0.1-segR1 | Cube | radius 0.1, segmentsR 1 | 104 | 114 |  |
| cube-radius0.1-segR2-axis0 | Cube | radius 0.1, segmentsR 2, axis 0 | 168 | 182 |  |
| cube-radius0.1-segR2-axis1 | Cube | radius 0.1, segmentsR 2, axis 1 | 168 | 182 |  |
| cube-radius0.1-segR3-sharp0 | Cube | radius 0.1, segmentsR 3, sharp 0 | 104 | 114 |  |
| cube-radius0.1-segR3 | Cube | radius 0.1, segmentsR 3 | 248 | 266 |  |
| cube-seg-2-1-1 | Cube | segmentsX 2 | 12 | 10 |  |
| cube-size0 | Cube | sizeY 0 | 4 | 1 |  |
| cube-sizeneg | Cube | sizeX -1 | 8 | 6 |  |
| cube-uvcomp | Cube | sizeX 2, uvCompensation 1 | 8 | 6 |  |
| cube-uvgaps0.1 | Cube | uvGaps 0.1 | 8 | 6 |  |
| cyl-8-axis0 | Cylinder | sides 8, axis 0 | 16 | 10 |  |
| cyl-8-none | Cylinder | sides 8, capTop none, capBottom none | 16 | 8 |  |
| cyl-8-quads | Cylinder | sides 8, capTop quads, capBottom quads | 34 | 32 |  |
| cyl-8-radial | Cylinder | sides 8, capTop radial, capBottom radial | 18 | 24 |  |
| cyl-8-seg2-ew0.5 | Cylinder | sides 8, segments 2, edgeWeight 0.5 | 24 | 18 | Refused: This setting only applies for subdivision surfaces. |
| cyl-8-single | Cylinder | sides 8 | 16 | 10 |  |
| ellipsoid-8x4-uvcomp-sizeXneg | Ellipsoid | sides 8, segments 4, uvCompensation 1, sizeX -0.5 | 26 | 32 |  |
| ellipsoid-8x4-uvcomp-sizeY0 | Ellipsoid | sides 8, segments 4, uvCompensation 1, sizeY 0 | 26 | 32 |  |
| ellipsoid-8x4 | Ellipsoid | sides 8, segments 4 | 26 | 32 |  |
| sphere-8x4-globe-axis0 | Sphere | sides 8, segments 4, axis 0 | 26 | 32 |  |
| sphere-8x4-globe | Sphere | sides 8, segments 4 | 26 | 32 |  |
| sphere-qball-order0 | Sphere | method qball, order 0 | 8 | 6 |  |
| sphere-qball-order1 | Sphere | method qball, order 1 | 26 | 24 |  |
| sphere-tess-order0 | Sphere | method tess, order 0 | 12 | 20 |  |
| sphere-tess-order1 | Sphere | method tess, order 1 | 42 | 80 |  |
