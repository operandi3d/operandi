# Radial falloff

**FAL-R-01** With Center C and Size (sx, sy, sz), a point p has
d = √(((px−cx)/sx)² + ((py−cy)/sy)² + ((pz−cz)/sz)²). Points with d ≥ 1 get
weight 0; otherwise w = f(d) with the same shape functions as Linear
(BEHAVIOR-LINEAR FAL-L-03 with t = d). So Size gives the ellipsoid's radii.

**FAL-R-02** Setup: Center 0, Size (1,1,1), points (x,0,0) for x = 0 … 1.5 in
steps of 0.1. Linear: 1, 0.9, … 0.1, 0, then 0. Ease-In at x = 0.5: 0.75;
Ease-Out: 0.25; Smooth: 0.5.

**FAL-R-03** Setup: Center 0, Size (4,1,2), Linear. (1,0,0) → 0.75;
(2,0,0) → 0.5; (0,0.5,0) → 0.5; (0,0,1) → 0.5; (2,0.5,0) → 0.292893;
(1,0.25,0.5) → 0.566987; (0,0,0) → 1; (-3,0,0) → 0.25.
