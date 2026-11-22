# Action centers with the Scale tool — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-06. REA-assisted: no.

## Scale centre per mode

| Mode | Centre | Case |
|---|---|---|
| Selection | the selection's centre (top face centre) | F1 |
| Selection Border | the border's centre; for one face the same as Selection | F2 |
| Selection Center Auto Axis | same as Selection here | F3 |
| Element | the clicked element (a top corner, (0.5, 0.5, 0.5) local), with the element's own axes | F4 |
| Origin | the world origin | F5 |
| Screen | a point fixed by the click in the viewport (here world (1.642, 0.407, 0.645)) | F6 |
| Local | each connected piece about its own centre | F7 |

## Results (local points; item at (2, 1, 0))

1. F1–F3: top corners to (±0.25, 0.5, ±0.25).
2. F4: top corners between the clicked corner and their old place:
   x and z in 0…0.5, y 0.5.
3. F5: about world (0, 0, 0): world (1 ± 0.25, 0.75, ±0.25), local
   (−1 ± 0.25, −0.25, ±0.25).
4. F6: about the clicked point: the top moved to y −0.047 (local).
5. F7: two separate quads, each scaled about its own centre (x 0.25…0.75
   and 2.25…2.75).
6. The Automatic action center was restored afterwards.

## Open questions and decisions

- **Q-AC-10** Screen: how the click maps to the 3D centre (depth): one case.
- **Q-AC-11** Auto Axis and Border differ from Selection only on rotated or
  multi-face selections (not tested).
- **Q-AC-12** Parent and Pivot modes not tested.
