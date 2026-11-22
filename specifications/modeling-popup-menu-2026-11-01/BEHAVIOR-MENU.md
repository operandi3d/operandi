# Modeling popup menu — observation status v1

> Physical-session update: the current blocked status below is superseded for the selected Polygon-face context by POP-PHY-01…03 in BEHAVIOR-PHYSICAL.md. These earlier synthetic-input trials remain historical diagnostics.

Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex), results
read back from Modo and UI. REA-assisted: no.

No menu-content requirements can yet be stated. Ten attempts failed to expose
a popup. These failures are not evidence that the ordinary physical gestures
lack a menu: modified keyboard presses also behaved anomalously during this
session. The remaining questions therefore stay open.

## Attempted contexts

Identity unit cube, coordinates ±0.5 m, eight points and six quads.
Front orthographic at 624 px/m (±1 px); the 1 m front face is 624 px wide.
Clicks over that face use pixel (1600,600), approximately world
(-0.002,0.022,0.5) m; the empty-space click (1000,400) is approximately
(-0.963,0.343,0.5) m on the same plane. These world values derive from the
screen calibration, not a claim about menu range. Only one zoom was tested.

| Mode / selection / hover | Attempted gestures | Visible result |
|---|---|---|
| Polygon / one face / mesh | Command+left, Control+left, Command+right, Control+right, right | no menu exposed |
| Vertex / one point / mesh | Command+left | no menu exposed |
| Edge / one edge / mesh | Command+left, 500 ms hold | no menu exposed |
| Items / one mesh / mesh | Command+left | no menu exposed |
| Polygon / empty / mesh | Command+left | no menu exposed |
| Polygon / one face / empty space | Command+left | no menu exposed |

No menu item, separator, submenu, disabled-item state or chosen-item result
was observed. ORACLES.json is deliberately empty. No acceptance oracle should
encode these inconclusive injected-click results as default Modo behavior.
*(open: SEL-Q-M3, POP-Q01–Q03)*
