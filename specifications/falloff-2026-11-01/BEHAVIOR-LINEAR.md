# Linear falloff

Let S = Start, E = End, and for a point p let
t = ((p − S) · (E − S)) / |E − S|², clamped to 0…1.

**FAL-L-01** The weight depends only on t: points are projected onto the line
through S and E; distance from the line does not matter. Setup: S (0,0,0),
E (1,0,0). Points (0.25,0.5,0), (0.25,0,0.7) and (0.25,-3,2) all get 0.75;
(0.75,1,1) gets 0.25. With S (0,0,0), E (1,1,0), points (x,0,0) get
1 − x/2 (x from 0 to 2).

**FAL-L-02** At and before Start the weight is 1; at and beyond End it is 0.
(Points from x = -1 to 2 with S x = 0, E x = 1.)

**FAL-L-03** Shapes, as w(t) (matched to 21 samples at t = 0, 0.05 … 1 with
error below 10⁻¹⁵):

| Shape | w(t) |
|---|---|
| Linear | 1 − t |
| Ease-In | 1 − t² |
| Ease-Out | (1 − t)² |
| Smooth | 1 − (3t² − 2t³) |
| Custom (In, Out) | (1−t)³ + (1−t)²·t·(2 − Out) + (1−t)·t²·(1 + In) |

Custom with In 0, Out 0 equals Linear. The Custom row is a formula fitted to
five measured (In, Out) pairs: (0,0), (1,1), (0.5,0.5), (1,0), (0,1); it
matched each exactly, but it is **inferred** for other values. *(Which UI
label corresponds to which of the two stored values is also inferred:
FAL-Q02.)*

**FAL-L-04** Symmetric (with S x = 0, E x = 1, points x -1…2, Linear shape):
- None: as FAL-L-02.
- Start: the falloff is mirrored about End: beyond End the weight rises
  again, reaching 1 at twice the length (x = 2 → 1, x = 1.5 → 0.5).
  Before Start it stays 1.
- End: mirrored about Start: before Start the weight falls again
  (x = -0.5 → 0.5, x = -1 → 0). Beyond End it stays 0.

**FAL-L-05** Swapping Start and End reverses the weights (Ease-In with S at
x = 1 and E at x = 0 gives 0.0975 at x = 0.05, the mirror of the normal
curve).
