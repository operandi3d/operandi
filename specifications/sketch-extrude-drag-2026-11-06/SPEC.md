# Sketch Extrude by dragging — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-06. REA-assisted: no.

## Rules

1. **Sections per path step (E1):** dragging a curved path of 10 points
   from the handle on the selected cube top made 9 new sections along the
   path (8/6 → 44/42: 36 new points, a ring of 4 per section, with side
   walls between sections and the cap moved to the path's end).
2. **Align to Path on (default):** the sections turn to follow the path;
   the end cap is tilted to the path's direction.
3. **Align to Path off (E2):** blocked: nothing happened in three tries,
   and Modo crashed twice around this tool. Open (Q-SE-04).

## Open questions and decisions

- **Q-SE-04 (hands-on)** Align to Path off: whether sections keep the
  original rotation (guide intent). Blocked by crashes.
- **Q-SE-05** How sections are spaced (screen-pixel Precision, 30 px by
  default) and whether the cap is exactly at the path end.
