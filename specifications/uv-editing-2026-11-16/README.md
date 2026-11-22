# UV unwrapping and editing research (2026-11-16)

Behaviour-only. REA-assisted: no. No bridge pass yet.

| File | What it is |
|---|---|
| `UV-TOOL-FAMILIES.md` | Every UV button, the engine behind it, and its status in Operandi |
| `BEHAVIOR-UV-TOOLS.md` | Behaviour spec for the UV tools not covered by the earlier two specs |
| `ENGINES.md` | Open-source UV maths we could borrow, ranked, with licence notes |
| `BUILD-PLAN.md` | Six build stages. Stages 1 and 2 get the beginner tutorial's UV steps working |

Earlier specs this builds on: `uv-unwrap-pack-relax-2026-11-11` v1.1
(Unwrap, Pack, Relax, UV viewport) and
`tutorial-gaps-2026-11-13/SPEC-UV-EDITING.md` (map list, UV transforms,
Fit, Project from View, Mirror, Align).

Engine rule (Jason, 11-16): xatlas for Atlas auto-charting only, Blender's
maths for the rest (already ported as `operandi-uv`), and the reference
for the workflow.
