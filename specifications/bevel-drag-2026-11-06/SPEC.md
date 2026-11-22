# Bevel by dragging — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-06. REA-assisted: no.

## Rules

1. **A drag applies a bevel (G1):** dragging the blue handle up made one
   bevel ring with Shift 0.042 (8/6 → 12/10).
2. **Each new drag is a new bevel (G2):** dragging again without turning
   the tool off adds a second ring on top of the first (16/14; Shift
   0.0235 for the second; tops at y 0.542 then 0.566). Releasing a drag
   commits it, so the next drag starts from the new cap. This matches the
   numeric rule (each apply is a new bevel).
3. **Settings changed after a drag don't re-apply (G3):** changing Inset
   (0.05 by script, then 60 mm typed in the panel) updated the setting but
   not the mesh; after dropping, the mesh is G2's.

## Open questions and decisions

- **Q-BD-01 (decision pending)** G3: in Modo, editing a value after a drag doesn't
  re-apply. Sane rule for Operandi: editing a value right after a drag
  updates that last bevel live (what users expect from tool properties).
  Alternative: copy Modo.
