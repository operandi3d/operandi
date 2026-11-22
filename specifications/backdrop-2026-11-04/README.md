# Backdrop item (reference image in the viewport)

**Status: v2, partial.** Observed in Modo 17.1v1 on 2026-11-04 through
scripted probes and viewport captures (pixel values sampled from the
captures). Names and intent come from the Modo help pages on modelling with
images, the Display preferences and the 3D viewport options. REA-assisted:
no.

v2 (same day) corrects BD-02 (a Backdrop with no image draws a 1 m dotted
square, not nothing) and adds BD-03 naming, BD-11a/b, BD-27, BD-31 for
Back/Bottom/Left and BD-33 (oracles 58-67).

Each rule in `BEHAVIOR-BACKDROP.md` is tagged **observed** (backed by an
oracle or a capture), **guide** (from the help, not measured) or
**predicted** (our best reading; build it, but expect it to change).

## What it is

A Backdrop is a scene item that shows one picture (a photo, sketch or
blueprint) flat in the 3D viewports as a modelling reference. It never
appears in a render. Each Backdrop faces one view direction (Front, Top,
Left and so on, or the Camera), has its own size, placement and colour
adjustments, can be moved, rotated and scaled like any item, and is saved
with the scene. To use front and side drawings together, the user makes one
Backdrop per view.

## Where Operandi deliberately differs (v2)

These rules describe Modo. For two of them the build side keeps its own
behaviour on purpose:

- **No-image placeholder (BD-02):** Operandi keeps its orange outline
  placeholder (Jason's request) instead of Modo's dotted 1 m square. The
  1 m size and centring still apply.
- **Perspective draw order (BD-11a):** Operandi keeps meshes drawn over the
  picture in Perspective, as in the orthographic views. Modo draws the
  picture over meshes there.

Match Modo on the rest of v2, in particular Auto Fit resetting the
out-of-plane scale to 1 (BD-33) and overlap order, first-created on top
(BD-11b).

## Files

- `BEHAVIOR-BACKDROP.md`: rules BD-01 onward.
- `ORACLES.json`: 67 cases: defaults, placement per projection, size rules,
  colour maths, Auto Fit, visibility, save round trip.
- `QUESTIONS.md`: what is still unobserved (BD-Q01 onward).

## Conventions

World axes are Y up, right-handed. The Front view looks toward -Z (screen
right is +X), Top looks down -Y (screen right +X, screen up -Z), Left looks
toward +X (screen right +Z). "Image left/top" means the picture as stored
in its file. Sizes are in metres.

## Suggested build order

1. Item with an image, Front/Top/Left projections, size from pixel size,
   draw behind meshes in matching ortho views (BD-01..BD-14).
2. Colour options and transparency (BD-20..BD-26).
3. Uniform Size, Keep Aspect, Scale rules (BD-15..BD-19), Auto Fit
   (BD-30..BD-33).
4. Perspective and Camera display, per-view Show Backdrop, selection
   outline, save/load (BD-40..BD-52).
