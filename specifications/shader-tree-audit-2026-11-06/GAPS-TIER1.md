# Shader tree audit — Tier 1 gaps (behaviour-only)

From research audit v0.2. Approved for build by Jason, 2026-11-06. Updated the same day to v0.3
(G-06 replaced, G-19 added) and v0.4 (G-06 Soft Light and Divide, G-20).

## Tier 1: small work, big visible payoff

### G-01 Selection Set masks do nothing
Polygons now carry selection sets (merged 11-04), but a Material Group set to
Tag Type "Selection Set" never matches anything. A polygon can be in any
number of sets; the group applies when the polygon is in the named set (AND
with the item mask, as for Material and Part). Overlapping sets: each group is
tested on its own, so two set groups can both apply, in tree order.
[docs + hand-off §6.2]

### G-02 Reflection Fresnel effect is silently dropped
The Effect menu offers Reflection Fresnel, and the Material has no Reflection
Fresnel channel, so a texture on it does nothing. Add the channel (Material
default 0%) and route the effect. [measured default]

### G-03 Base Material starting values differ
| Setting | Reference | Operandi |
|---|---|---|
| Diffuse Color | 0.6 grey | 0.8 grey |
| Diffuse Amount | 80% | 80% |
| Specular Amount | 50% | 4% |
| Specular Fresnel | 100% | 0% |
| Reflection Fresnel | 0% | (missing) |
| Refractive Index | 1.0 | 1.5 |
| Subsurface Color | 0.8 grey | 1.0 |
| Subsurface Distance | 5 mm | 10 mm |
| Roughness | 40% | 40% |
| Bump Amplitude / Displacement Distance | 5 mm / 20 mm | same |
| Smoothing / Smoothing Angle | 100% / 40° | same |
| Double Sided | off | off |
| Clearcoat Bump | 100% | (missing) |

DECIDED (Jason, 2026-11-06 04:34Z): match the reference. All rows take the
reference value, including Diffuse Color 0.6 grey and Specular Amount 50%.
Set Material's starting values follow the same numbers so the material table
and the tree agree. Specular Amount 50% must then map onto Cycles so that it
still looks like a plain dielectric (check the ST-Q05 mapping; don't feed 50%
straight into Specular IOR Level). [measured]

### G-04 Constant / procedural colour slots
Operandi's Constant has one colour and one value. Every pattern texture in the
reference has two zones, each with Value, Color and Alpha, plus Bias (50%) and
Gain (50%). Zone 1 starts black / value 0, zone 2 white / value 1, alphas
100% (Wood, Ripples and Weave differ). Getting this shape right once makes
every procedural below cheap. [measured]

### G-05 Image Map has none of its adjustment settings
Operandi's Image Map is "pick an image". The reference layer has, with
starting values [measured]:

| Setting | Start | Behaviour [docs] |
|---|---|---|
| Low Value / High Value | 0% / 100% | black maps to Low, white to High (scalar effects) |
| Invert Red / Green / Blue | off | per channel |
| Gamma | 1.0 | |
| Brightness / Contrast | 100% / 100% | |
| Clamp | off | |
| Swizzling | off (RGBA) | RGB, Alpha, Red, Green, Blue variants |
| Alpha | Use | Use / Ignore / Alpha Only |
| Filtering | Bilinear | Nearest / Bilinear / Bicubic |
| Antialiasing | on, 100% | + Minimum Spot 1.0 |

Low/High, Alpha mode and Swizzling change render results; the rest are easy
per-pixel maths in the compiled shader.

### G-06 Blend-mode maths: three clamps differ
Measured with below B = (0.8, 0.4, 0.2), layer L = (0.25, 0.5, 0.75), 100%
opacity, per RGB channel:

| Mode | Result | Operandi today |
|---|---|---|
| Normal | L | same |
| Add | B + L, no clamp (can pass 1) | same |
| Subtract | max(B - L, 0): clamps at 0 | goes negative: fix |
| Difference | abs(B - L) | same |
| Multiply | B x L | same |
| Screen | 1 - (1 - B)(1 - L) | same |
| Overlay | B < 0.5: 2BL; else 1 - 2(1 - B)(1 - L) (keyed on the layer below) | check it keys on below |
| Hard Light | the same, keyed on the layer | same |
| Soft Light | L up to 0.5: B - (1 - 2L)*B*(1 - B); L above 0.5: B + (2L - 1)(sqrt(B) - B) (square root for every B, no low-B polynomial) | fix: Cycles' soft light differs above 0.5 |
| Darken / Lighten | min / max | same |
| Color Dodge | min(B / (1 - L), 1): clamps at 1 | check clamp |
| Color Burn | max(1 - (1 - B) / L, 0): clamps at 0 | check clamp |
| Divide | (1 - B + 2L) / (3(1 + B)). Not a plain B/L. Fits all 25 measured points (two bases, grey and coloured layers, L from 0 to 2). Example: B 0.8, L 0.25 gives 0.130 | fix: use this formula |
| Normal Multiply, Normal Map Blend | normal-map modes (wait for G-07) | Normal for now |

Opacity lerps between below and the blended result, as Operandi already
does. Menu order stays as the public guide lists it. Apply the same maths in
the viewport/eval fold and the Cycles compile (Cycles mix nodes may need
clamp/math nodes added). Soft Light and Divide differ from Cycles' built-ins,
so build them from math nodes in the Cycles compile, and match in the
viewport/eval fold.

### G-19 Layers above the Base Shader do nothing
A Constant placed above the Base Shader (at the top level, or inside a group
that sits above it) left the surface unchanged; moved below the Base Shader it
took effect. Shaders above it still win (hand-off section 14 item 3). So for
surfacing, only layers BELOW the topmost applicable shader count. Operandi's
fold applies them wherever they sit: fix by ignoring material and texture
layers above the shader that wins for that surface. Render outputs sit above
it by design and are unaffected. Masks and Invert need no change.

### G-20 Add Layer starting values and placement
With nothing selected, Add Layer gives:

| New layer | Lands | Starting values that differ from Operandi |
|---|---|---|
| Constant | top level, directly below the Base Shader | Color black, Value 100% (Operandi: 0.5 grey / 0.5). Effect Diffuse Color |
| Material Group | directly below the Base Shader | Scope All (Operandi: Surface) |
| Material | directly below the Base Shader | as the Base Material except Bump Amplitude 0, Displacement Distance 0 |
| Shader | top of the tree, above both Render Outputs (Operandi: below the Base Shader, ST-Q02) | |
| Procedural (Noise) | directly below the Base Shader | Effect Diffuse Color; zones per G-04 |
| Gradient | directly below the Base Shader | Effect Diffuse Color, Input Incidence |

Every texture gets a locator: projection axis X, Size 1 m, Repeat on both
directions (stored projection for procedurals reads as Solid). Image Map:
keep Operandi's UV Map start. Placement with a group or layer selected (Q-2)
is still unmeasured: keep current behaviour for that case. Only rows for
layer types Operandi already has apply (Procedural, Gradient and Texture
Locator are Tier 2). "Directly below the Base Shader" matches G-19.
G-07 bump/normal/displacement stays out of scope (Tier 2).
