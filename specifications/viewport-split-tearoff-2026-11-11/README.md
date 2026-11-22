# Viewport splitting and tear-off spec v1 (2026-11-11)

Behaviour-only spec for splitting a panel by Ctrl-dragging its corner dot
(new panel below, right, left or above), the menu split commands, joining
and closing splits, minimum sizes, what a new view inherits, undo, and
tearing a view off into its own OS window. Builds on the panel corner dot
spec (2026-11-06), which is already built; it fills the gaps that spec left
and lists what is already done.

Files: BEHAVIOR-VIEWPORT-SPLIT-TEAROFF.md (the spec, VST-01..41 and test
cases T01..T17), QUESTIONS.md (VST-Q01..Q07).

Overlap: the UV viewport build running at the same time. Section 7 of the
spec says how the two meet (the UV view is one more panel kind in the same
layout tree).

Sources: the reference app's 17.1 user guide (layout controls, layout
customisation, viewport overview and layout tour pages, paraphrased); its
plain-text command help and floating-window definitions (split, quad,
detach, remove; floating window minimum sizes; divider positions stored as
fractions); the 2026-11-06 hands-on session recorded in the panel corner dot
spec; the Operandi UI conventions bible (UIC-70..73 and the key rule).
No new bridge probes for v1. No screenshots or assets included.
REA-assisted: no.
