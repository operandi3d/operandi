# I2 remaining-check preparation

Read `PREPARATION_REPORT.md`, `CASES.json`, `NATIVE_PROTOCOL.md`, `PROVENANCE.md` and
full `states/` literals. This is a frozen proposal for independent review, not an
execution report. The existing runtime report remains Incomplete and unchanged.

Current progress/deadlines are in `PROGRESS.json` (intentionally mutable and excluded
from the freeze). The original runtime batch is over. The current entire follow-up
hard stop is **2026-10-26T01:36:00Z**, not a fresh clock for each phase. No test, normal
build, app launch/input or integration is released in this phase.

Pure independent authoring/structural commands actually run from the task checkout:

```sh
python3 docs/i2-gap-plan/author_literals.py
python3 docs/i2-gap-plan/verify_literals.py
```

They import only Python stdlib and approved literal JSON. They do not import or run
Operandi, its codec, tests, dependencies or applications. Regeneration must remain
byte-identical to frozen expectations. No candidate output is an authoring input.
The structural check is not independent reviewer approval or candidate correctness.

Read-only freeze verification:

```sh
python3 docs/i2-gap-plan/verify_frozen.py
```

`FROZEN.sha256` contains all new preparation files except itself and the progress
record. `INPUT_IDENTITIES.json`/`AUTHORING_INPUTS.json` bind external source/authority
and reviewed old literals. `BASELINE.json` records1,785 verified preserved entries,
normal executable hash and the unchanged branch/HEAD/index. Future review/release
must be a separate new handoff; never edit old frozen bytes to make an execution pass.

`PROPOSED_COMMANDS.json` gives future commands whose new guard and test adapter do
not exist yet. They require explicit review release; don't run them now or invoke
expired `development/i2/scripts` launchers. No normal executable recompilation is
needed while its87 runtime inputs remain unchanged. Outputs for any later release
belong only in `development/i2-gap-confirmation/` with new unique paths.

## Amendment — 2026-11-01, confirmed 2026-11-03 (Jason): position-only changes keep identities

A change that only moves points (same point count, same polygons and
corners in the same order) keeps every point, polygon, corner and edge
identity; it no longer allocates a new generation. Topology changes still
rebind. The `otherMove*` states were amended mechanically from the reviewed
values: the Move rebuild's IDs 83–132 map back to the originals (−82), IDs
allocated afterwards move down by 50, and later generations by one. History,
selection, geometry and publication counts are unchanged.
