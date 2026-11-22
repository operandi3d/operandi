# Deform tab: layout correction (2026-11-06)

For the Deform builder. Words only. REA-assisted: no. Starting values were
read from the reference's tool definitions (read-only, nothing applied).

## What's wrong in build deform-presets 586550e0

DEFORM.md was right about the tools, but the build put them on the panel
in the wrong places:

1. **Soft Move and Shear are not top-level buttons.** They live inside a
   flyout whose face button is **Soft Drag**.
2. **Smooth is a flyout** (Smooth, Jitter, Quantize), not a plain button.
3. **Bulge is a flyout** (Bulge, Flare).
4. **Most of the reference's buttons are missing**, so every section is
   shorter and things shift around. Show the full panel now, with
   unbuilt tools **greyed out** (tooltip "Not built yet"), so the layout
   and muscle memory match from day one.
5. The **Align Tools** section is missing its first two buttons, Radial
   and Linear, and there's no **Commands** section (Relax Mesh...).
6. A **"Tool Properties / Select a tool... / Apply Last Cube"** block
   appears at the bottom of the Deform tab. That doesn't belong on this
   tab; the reference's tab ends with Commands. Remove it from here, or
   check whether it's leaking in from another panel.
7. Label: the first button reads **Tweak** in Operandi and **Element
   Move** in the reference (shortcut T on both). Keep Operandi's name if
   that was a deliberate choice; otherwise rename it. Jason's call.

## Correct panel, top to bottom

Large icon + label buttons; ▸ = flyout (small corner triangle, press and
hold; the face shows the last tool picked).

**Move Tools**
1. Element Move (T) — built (Tweak)
2. Element Push — *new*
3. Flex (Shift+Y) — *new*
4. Soft Drag ▸ — flyout: **Soft Drag** (new), **Soft Move** (built),
   **Shear** (built). Until Soft Drag exists, the face can show Soft Move.
5. Smooth ▸ — flyout: **Smooth** (built), **Jitter** (new), **Quantize**
   (new)
6. Soft Selection Move ▸ — flyout: Soft Selection Move, Rotate, Scale,
   Transform (all new)

**Rotate Tools**
7. Twist — built
8. Bend (Ctrl+E) — *new*
9. Vortex — *new*
10. Swirl — built

**Scale Tools**
11. Push — built
12. Sculpt — *new* (belongs with the later Sculpt work; greyed for now)
13. Taper — built
14. Bulge ▸ — flyout: **Bulge** (built), **Flare** (new)

**Align Tools**
15. Radial — *new* (greyed until something usable is selected)
16. Linear — *new* (same)
17. One half-width row: Quick Align | Ground Align — built

**Commands**
18. Relax Mesh... — *new*; small text button that opens a dialog

## The missing tools, in brief, with starting values

Full behaviour is in DEFORM.md §3; this is the short version.

- **Element Push**: Tweak, but the dragged element moves along its own
  normal. Distance 0; Selection Normals off. Element falloff: Range 0,
  Connected Elements first choice (Ignore), Always Raycast on, Shape
  Linear.
- **Flex**: full move/rotate/scale handle at the **border** of the
  selection; the selection moves fully and a band of **Steps** polygon
  rings around it fades out. Steps **2**, Shape **Smooth**. Off-handle
  drag = Translate.
- **Soft Drag**: move with a **screen circle**: everything inside the
  circle around the pointer moves with the drag, fading to the edge.
  Circle size **20 px** at start (right-drag resizes); Transparent **off**
  (only front-facing geometry).
- **Jitter**: random offset per point within Range X/Y/Z (0, 0, 0); seed
  **666** on this button; X, Y, Z enabled; Rigid Translate / Rotate /
  Scale off.
- **Quantize**: snap selected points to a grid; step X, Y, Z = 0 (0 =
  that axis not snapped).
- **Soft Selection family**: selection moves rigidly, effect fades over
  **Radius 0.1 m** outside it; Use Connectivity **on**; Shape Linear;
  Show Vertex **on**, Show Edge off.
- **Bend**: click the base on the work plane; spine handle + angle disc.
  Angle 0°, Spine (0, 1, 0) (straight up from the base, 1 m), Both Sides
  off.
- **Vortex**: rotate with a **cylinder** falloff (strongest on the axis,
  none at the radius, endless along the axis). Cylinder centre (0, 0, 0),
  size (1, 1, 1), axis **Y**, Shape Linear; Angle 0°.
- **Sculpt**: old single brush; defer.
- **Flare**: Push with a Linear falloff (auto-sized on first click).
  Distance 0.
- **Radial (Radial Align)**: put selected points on a circle or regular
  polygon. Mode **Circle**; Side **5** (for N-Sided); Rotate 0; Radius 0
  = measured from the selection at start; Start Angle 0°; Weight
  **100%**; Smoothing Iterations **50**; Flatten **on**; Exponent 0.
  (Central Angle reads 90° in the definition, which would make an arc;
  RA-Q2: check what a real first use shows before trusting it. Sane
  default for a "make it round" button is 360°.)
- **Linear (Linear Align)**: line points up between the ends of each
  run. Mode **Line**; Uniform **on**; Weight **100%**; Smoothing
  Iterations **50**; Flatten **on**; Align Length off; Align to First
  Selected off; Screen Align off; Preserve Original Curve off.
- **Relax Mesh...**: dialog with Iterations (**5**, per the guide) and
  Constrain (starting state not readable; use off). One undo step.

## Suggested next build order on this tab

Panel layout fix first (cheap, removes the mismatch Jason saw), then
Element Push, Flare, Jitter and Bend (each small), then Linear/Radial
Align and Relax Mesh, then the new falloffs (Cylinder → Vortex, Screen →
Soft Drag, Selection → Flex), Soft Selection last.
