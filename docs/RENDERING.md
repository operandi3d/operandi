# Rendering: what we're aiming for

Operandi renders with [Cycles](https://www.cycles-renderer.org), the
open-source path tracer from the Blender project. If you're coming from
Modo, this page explains what "it renders like Modo" does and doesn't mean.

## The short version

**Your scene should feel like a Modo scene, and look like what Cycles
honestly makes of it.** Same ballpark, same mood. Never pixel-for-pixel.

## The four rules

1. **Cycles does the physics.** How light bounces, bends through glass,
   scatters in smoke and adds up is whatever Cycles calculates. We don't
   add hidden tweaks on top to imitate another renderer's look. If a
   render looks wrong, we fix the real cause (wrong units, a missing
   feature, a setting that's ignored, colour handled wrongly), not the
   symptom.

2. **Modo guides how you work.** Tools, panels, the shader tree, names,
   defaults and units follow the way Modo users already work, so you know
   where things are and what the numbers mean.

3. **Same settings, recognisable result.** Light a scene with the same
   lights at the same intensities, give it the same material settings and
   colour handling, and a Modo user should recognise the render. Expect
   differences in the fine detail, because Cycles is a different (and
   correct) renderer: noise, caustics, glossy falloff and so on may not
   look identical. That's fine.

4. **The shader tree is a friendly front end.** The layered shader tree
   you edit turns into ordinary Cycles / OpenPBR materials underneath.
   Nothing secret or Operandi-only, so your materials stay plain,
   portable and explainable. A node view of the same materials, for people
   who prefer Cycles-style nodes, is on the wish list.

## How we check it

We compare renders side by side against reference scenes using a "close
enough" bar. The point is to catch real bugs, such as:

- lights far too bright or too dark (wrong units),
- something missing (a feature Cycles supports that we haven't wired up),
- wrong colours (colour space or gamma mistakes),
- a setting that does nothing.

A difference that is just "Cycles draws it slightly differently" isn't a
bug, and we won't fake it away.

Found a render that looks clearly wrong? Use **Report Bug** in the app and
attach the scene if you can.

---

Operandi is not affiliated with or endorsed by Foundry. Modo is a trademark
of The Foundry Visionmongers Ltd.
