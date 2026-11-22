# Set Part (observed) — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

This spec checks the guide-only `set-part-2026-11-04` spec (SP-xx) against
Modo and settles four of its open questions.

## Rules

1. **Selected polygons get the Part** (PT01). An earlier Part is replaced
   (PT03: polygon 1 went from A to C), and reusing an existing name just
   adds polygons to it (PT04). Confirms SP-01, SP-05.
2. **Nothing selected in Polygon mode → every polygon** of the mesh (PT02).
   Item mode → every polygon of the selected mesh (PT10). Settles SP-Q01:
   yes.
3. **Vertex or Edge mode with something selected: nothing is tagged**, and
   there's no error message (PT07, PT09). With nothing selected in those
   modes, every polygon is tagged (PT08). Settles SP-Q02: no polygons, no
   refusal message. Set Material behaves the same way.
4. **Names are kept exactly** (PT05 "Left Wing"). **A blank name is
   accepted** and stored as an empty Part (PT06). "Default" is stored like
   any name (PT11). Polygons that never had a Part read back as "Default".
   Settles SP-Q04 for Modo. **Decided for Operandi (Q-PT-01):** refuse a
   blank name.
5. **Nothing else changes:** material tags stay, points don't move, no
   shader-tree items are created (all cases; confirms SP-02, SP-04).
6. **Selection after:** unchanged, mode unchanged.

## Open questions and decisions

- **Q-PT-01** (decided below) Blank name: Modo stores an empty Part. Sane rule for
  Operandi: refuse a blank name (keep SP-13's provisional answer). An empty
  Part can't be told apart in lists and gains nothing.
- **Q-PT-02** (decided below) Vertex/Edge mode with a selection does nothing silently.
  Sane rule: copy Modo (matches Set Material); showing a hint
  is the alternative.
- Still open from set-part-2026-11-04: SP-Q03 (dialog's first value),
  SP-Q05–Q07 (preference default, list display and sorting). These are
  dialog behaviour, not scriptable.

## Decided (2026-11-05)

- **Q-PT-01 → decided:** Operandi refuses a blank name (PT06 is Modo
  reference only).
- **Q-PT-02 → decided:** with something selected in Vertex or Edge mode,
  Set Part does nothing, with no message (as Modo, and as Set Material).
