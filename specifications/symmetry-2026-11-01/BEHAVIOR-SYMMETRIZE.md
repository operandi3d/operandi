# Symmetrize

**SYM-SZ-01** Symmetrize (side Positive or Negative) moves points on one side
so they mirror their partners on the other side. Positive copies from the
positive side to the negative side; Negative the reverse. It runs on the
whole mesh with nothing selected.

**SYM-SZ-02** Setup: F1 with point 9 moved to (2.003, 1.02, 0.01) and point 5
to (-1.99, 1, 0), Symmetry X. Positive → point 5 becomes (-2.003, 1.02, 0.01),
point 9 unchanged. Negative → point 9 becomes (1.99, 1, 0), point 5 unchanged.
Point 7 on the plane stays at (0,1,0). (SYM-O-11.)

**SYM-SZ-03** Its partner matching is looser than Select Symmetrical's: the
pairs above, off by up to 0.02 m, were still matched. *(Its exact matching
rule is open: SYM-Q09.)*
