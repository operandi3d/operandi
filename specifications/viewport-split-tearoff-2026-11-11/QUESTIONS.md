# Open questions (viewport split and tear-off v1)

Each has the spec's current answer, so the build can go ahead. Q02 to Q04 need a person at the mouse in the reference app.

- **VST-Q01** ANSWERED (Jason 2026-11-11): down makes the new panel below,
  right makes it to the right, and so on. Spec: four directions (VST-02).
- **VST-Q02** (was PCD-Q3) Does a Ctrl-drag split at half or at the
  pointer? Spec: at half (VST-05).
- **VST-Q03** When a panel in a row of three is deleted, who gets the space:
  both neighbours in proportion, or one neighbour? Spec: proportional
  (VST-19).
- **VST-Q04** Minimum docked panel sizes and what happens on a split that
  would go below them. Spec: 120 x 90 for views, split refused (VST-23,
  VST-24).
- **VST-Q05** DECIDED (Jason 2026-11-11): torn-off views stay on top of the
  main window, like Operandi's palettes (VST-30).
- **VST-Q06** DECIDED (Jason 2026-11-11): splitting is not an undo step;
  no layout change is (VST-39). No probe needed.
- **VST-Q07** Live feedback during a Ctrl-drag (VST-04) is an Operandi
  choice; the reference app's feedback for this gesture wasn't watched.
