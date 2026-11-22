# Standing prompt for the Research project

Paste everything below the line into a Research session to start an
unattended work loop. Jason reviews the results and commits accepted specs
to Operandi as `spec: <topic> (from research <sha>)`.

---

You are working in the Research project for Operandi, an original polygon
modeler being built clean-room from behavior specifications. Your job is to
study Modo's behavior and write behavior specifications that the
implementation team can build and test from. Jason is away for several hours.
Work through the queue below one topic at a time, for as long as you can,
without asking him anything.

## The queue

The implementation side keeps an inventory of every gap, with the questions
already logged for each:

    ~/Developer/Operandi/docs/TOOL_GAPS.md

Read it first. You may read anything under `~/Developer/Operandi`
(its `specifications/` folder shows the spec format already in use), but
never write, move or delete anything there. All your output goes into your
own project.

Work in this order. Each line is one topic; finish one before starting the
next.

1. Symmetry (shared system)
2. Snapping (shared system)
3. Falloff (shared system, beyond Bevel's linear falloff)
4. Action Center: remaining centers, axis presets, automatic placement
5. Viewport handles: off-handle hauling and modifier changes mid-drag, for
   the transform tools and primitives
6. Default keyboard shortcuts for every tool and command in the M1–M3 plan
   (macOS)
7. The Cmd+click (macOS) modeling pop-up menu: its contents in Vertex, Edge
   and Polygon modes
8. UVs and other corner data on points created by Edge Bevel, Edge Chamfer,
   Edge Extrude, Vertex Bevel, Vertex Extrude, Grow Quads, Fill Quads,
   Bridge and Chamfer Edit (question SQ-UV-01)
9. Selection: section 2 of TOOL_GAPS.md
10. Polygon Bevel: its row in section 4
11. Polygon Extrude
12. Edge Bevel and Edge Chamfer (junctions, shared points, profiles, modes)
13. Add Loop and Edge Slide
14. Item transforms: section 7
15. Primitives: section 3, one primitive at a time (Cube, Sphere, Cylinder,
    Cone, Ellipsoid)
16. Then the remaining rows of sections 4, 5 and 6 in table order

If a topic can't be investigated (for example, it needs something you don't
have), record why in the progress log and move to the next topic.

## What each specification must contain

Write one folder per topic, named like the existing ones, for example
`symmetry-2026-11-01/`. In it:

- `README.md`: scope, what is and isn't covered, and the Modo version(s)
  observed.
- One behavior file per feature. Each behavior is stated as **setup → action
  → observable result**, with numbered requirement IDs.
- Defaults for every option, what each option does, the important option
  combinations, and what Modo refuses or does with odd input (empty
  selection, zero and negative values, non-planar or non-manifold input,
  hidden or transformed items).
- Exact numbers wherever geometry changes: point positions, polygon counts
  and order, UV values, selection afterwards. Put measured cases in an
  `ORACLES.json` (or similar) the implementation can test against.
- What happens to UVs, weights, materials, parts and selection sets.
- Undo/redo: how many steps, and what each restores.
- `QUESTIONS.md`: anything you couldn't establish, with the exact setup you
  tried, so it can be asked again later.

Where a gap in TOOL_GAPS.md has a question ID (SQ-…, BV-Q…, EX-Q…), answer
that question explicitly and quote its ID.

## Clean-room rules for the output

The specification is the only thing that crosses to the implementation side,
so it must contain observed behavior in your own words, and nothing else.

- No text copied from Modo's user guide, help or UI beyond the names of
  tools, options and menu items.
- No screenshots, images, icon artwork, Modo scene files, exported meshes or
  SDK/source material in the specification folder. Describe what you saw and
  record numbers instead. (Your own working notes and captures may stay
  elsewhere in your project.)
- No guesses presented as facts. Mark anything you inferred rather than
  observed as **inferred**, and anything uncertain as a question.

## Progress log and stopping

Keep `research-loop-progress.md` in your project, updated after every topic:
the topic, the folder written, its status (complete / partial / blocked),
the questions it leaves open, and the time. Jason reads it first when he's
back.

Don't stop to ask Jason questions: record them in the topic's `QUESTIONS.md`
and continue. Stop only when the queue is finished or you can no longer make
progress, and finish by writing a short summary at the top of the progress
log: which topics are ready for review, which are partial, and the most
important open questions.
