# Duplicate tools (v1, 2026-11-08)

Behaviour-only specs for the copy-making tools Operandi lacks. Written from
the reference's public user guide plus earlier Operandi research notes
(toolbox-panels-2026-11-06/DUPLICATE.md), then a scripted probe pass on
2026-11-08 (starting values for all six tools, Mirror seam weld and
facing, Mirror Angle, Radial Array count). REA-assisted: no.

Read in this order:

| File | What | Build order |
|---|---|---|
| `BEHAVIOR-DUPLICATE-SHARED.md` | The engine all five share: placer + copier, what gets copied, merge, flip, undo, panel | first (plumbing) |
| `BEHAVIOR-MIRROR.md` | Mirror | 1 |
| `BEHAVIOR-LATHE.md` | Lathe (the reference calls it Radial Sweep) | 2 |
| `BEHAVIOR-CLONE.md` | Clone (copies in a line) | 3 |
| `BEHAVIOR-ARRAY.md` | Array (copies in a 3D grid) | 4 |
| `BEHAVIOR-RADIAL-ARRAY.md` | Radial Array (copies round a circle or spiral) | 5 |
| `BEHAVIOR-DETRIANGULATE.md` | Detriangulate (triangle pairs back to quads) | 6, independent |
| `QUESTIONS.md` | Open questions, each with a provisional answer | |

Status tags, same as viewport-properties:
- **CONFIRMED** from the reference's public docs or Jason's own words.
- **LIKELY** strongly suggested, not checked.
- **OPEN** not known; a provisional answer is given so building can start.
- **[OP]** Operandi's own choice where the reference is silent.

## What a beginner tutorial needs first

Beginner modelling tutorials lean on these, in this order:
1. **Mirror** in polygon mode across X with the seam welded. Model half a
   head, a car, a chair; mirror; done. This is the one that must be solid.
2. **Lathe** of a profile curve or an edge strip round Y, full turn, seam
   welded. Wine glasses, bottles, vases, chess pieces.
3. **Radial Array** in polygon or item mode round Y: chair legs, bolts round
   a rim, petals.
4. **Clone** and **Array**: fence posts, shelves, tiles, stairs.
5. **Detriangulate**: cleaning imported or booleaned meshes. Rarer.

Not in this batch (later, need machinery Operandi doesn't have): the
Instance and Replica versions of every tool, Transform Array/Radial Array,
Source = background / random / preset, Mirror's Three Points mode,
Lathe's Square mode and preset profiles. Each is listed as "later" in its
file. Per UIC-117, rows for unbuilt features are left out of the panel
entirely, not greyed.
