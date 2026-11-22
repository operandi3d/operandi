# Drop Action: build staging

## What Operandi has already (checked 2026-11-06, main)

- Item hierarchy, and Parent/Unparent with or without Keep Position (item
  right-click menu). That *is* Parent / Parent in Place.
- Item-mode picking under the pointer, and a surface raycast (Tweak's
  Always Raycast / picking) to reuse for Place.
- Drag-and-drop exists only in the Shading panel. No viewport item-onto-item
  drop, no Items-list row drag, no preset browser.

## Stages

1. **Core actions as commands (cheap now).** Pure scene operations taking
   (source(s), destination), each one undo step, with unit tests DA-T1..T9:
   Parent, Parent in Place (reuse existing), Match, Match Position/Rotation/
   Scale (world-space solve), Insert, Insert at Parent, Insert in Place
   (hierarchy splice + world solve). Expose through MCP so they're testable
   without a mouse. Roughly a day.
2. **The dropdown (cheap now).** Top-bar button between Work Plane and
   Render, DA-1..3. On its own it does nothing visible, so ship it with 3.
3. **Viewport drop gesture.** DA-4..9: in Item mode with no tool and a
   non-(none) action, press on an item, drag, release on another; target
   highlight; Esc cancels. This is the part that makes the button matter.
   Watch for clashes with box/lasso select and drag-anywhere move in Item
   mode; with (none) nothing changes.
4. **Place / Place and Align.** Needs the ray hit point and surface normal on
   the destination at release. Medium.
5. **Later:** pause-over popup + modifier mappings (DA-32); Items-list drops
   using the same actions (DA-Q3); presets and per-item drop overrides
   (DA-33), which wait for a preset browser.

Cheapest useful slice: stages 1–3 with Parent, Parent in Place and the Match
four. Insert's three come almost free with stage 1.
