# Poly Haul modes by dragging — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-06; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Which element a mode acts on

- **On the selection:** Offset, Extrude and Scale act on the selected
  polygon, wherever on it the drag starts.
- **On the element under the pointer:** Bevel, Split, Tear and Slide act on
  the edge or point under the pointer, not on the selection. Remove deletes
  the clicked polygon.

## Rules

1. **Offset (C1):** the top face moves along its normal by the dragged
   amount (Offset 0.318 → y 0.818); no new geometry (8/6).
2. **Extrude (C2):** the same move, plus four new side walls
   (8/6 → 12/10, Offset 0.318).
3. **Bevel (C3):** a drag starting on the face's centre handle did nothing.
   A drag starting near the top face's +X edge chamfered **that edge**:
   inset 0.075 puts the new points 0.075 from the old edge on both faces
   (x 0.425 and y 0.425); 8/6 → 10/7.
4. **Scale (C4):** the face scales uniformly in its own plane about its
   centre (factor 0.26 → corners at ±0.13); no new geometry.
5. **Split (C5):** a drag along an edge of the quad slides a cut position
   along that edge, and the quad is split by a new edge across it at that
   position (49.3 % → new points at y 0.007); 4/1 → 6/2. A drag straight
   across the quad only moved a marker to 100 % and changed nothing.
   Selection cleared.
6. **Add Loop (C6, the "loopSlice" mode):** two tries, no cut (8/3
   unchanged); a 79.5 % marker showed on the bottom edge. Open (Q-PH-05).
7. **Remove (C7):** a click removes the clicked polygon; its points stay
   because the side polygons use them (8/6 → 8/5). Selection cleared.
8. **Tear (C8)** (Vertex mode, Select Through off): dragging a corner moves
   a **new copy** of the point for **one** polygon only (the +X side face);
   the other faces keep the original corner (8 → 9 points). The copy moved
   by (0.178, 0.074, 0).
9. **Slide (C9):** the centre point of a 2×2 grid moved mostly along the
   +X edge, by (0.362, 0.06, 0). Not perfectly held to the edge
   (Q-PH-06).
10. Exact amounts depend on drag length; the recorded tool settings give
    the amount Modo used for each case.

## Open questions and decisions

- **Q-PH-05** Add Loop made no cut in two tries (C6). Needs another try
  (drag across an edge of the middle quad, or on a quad loop).
- **Q-PH-06** Slide: should the point stay exactly on the edge? C9 drifted
  0.06 off it. Sane rule for Operandi: constrain to the edge direction.
- **Q-PH-07** Which polygon Tear picks (C8 picked the +X side face).
- **Q-PH-08** Bevel from the face handle did nothing (C3); does Bevel mode
  ever act on the selected face?
