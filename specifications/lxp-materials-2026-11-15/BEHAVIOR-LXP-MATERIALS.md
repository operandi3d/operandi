# Modo material presets (.lxp) into Operandi materials
Stage 3 of `modo-content-folder-2026-11-12/BEHAVIOR-MODO-CONTENT-FOLDER.md`. Behaviour only. **v1.1 (2026-11-15)**: adds section 7, procedural layers.

Sources: public user documentation; Modo's plain-text config, read only, for internal channel names and the effect ids; one scripting-bridge session: 6 of the presets that ship with Modo were applied to a test cube and their items and channels read back.

Rule tags: `[D]` public user docs; `[C]` Modo's plain-text config (read only); `[B]` bridge observation; `[R]` our recommendation; `[O]` Operandi today; `[?]` still unknown.

Clean room: no preset file was opened byte by byte. The reader uses only the published container and chunk layout: FORM, DESC, PRVW, and ITEM with channels stored by name. Q-MC1 is decided black-box: unknown records are skipped by size, or the preset is refused. Builders get this file only, never the bridge logs.

---
## 1. What a material preset holds `[B]`
Applying a preset creates the following:
- **A mask (group)** named after the preset. Its polygon tag is empty, and its tag type is either empty or "Material". The mask is a generic container that gets bound to the drop target.
- **One material item**, of one of these types:
  - `advancedMaterial`: Modo's own material. Its `brdfType` is `gtr` (Physically Based), `ashikhmin` (Energy Conserving, legacy), or the Traditional or Principled models `[D]`.
  - `unrealShader` (Unreal Material).
  - `unityShader` (Unity Material).
  - A glTF type is mentioned in the docs `[D]`; its item type is `[?]`.
- **Texture layers** above the material:
  - `imageMap` layers, each with an image clip (`videoStill`, holding a file path and a colour space) and a texture locator (`txtrLocator`);
  - procedural layers (`noise`, `gradient`, …);
  - **third-party procedural types** (seen: `val.Noise_MarbleNoise.RJJ`, `val.Noise_Scruffed.RJJ`, `val.Skins_Crumpled.RJJ`). These come from a plug-in pack and can never be reproduced.
- Each layer carries `enable`, `opacity`, `blend` (normal, multiply, add, screen, …), `invert` and `effect`, which is the channel it drives.

Observed examples (values as stored; % shown as 0–1):
| Preset | Type | Key values |
|---|---|---|
| Polished Gold | advancedMaterial, gtr | diffAmt 0, diffCol (0.11, 0.06, 0.01), specAmt 1.0, specCol (1, 0.77, 0.34), rough 0.1, reflBlur on, plus a gradient on incidence angle that multiplies roughness |
| glass_crown | advancedMaterial, gtr | tranAmt 0.98, refIndex 1.52, useRefIdx on, specAmt 0.02 (ignored because useRefIdx is on), rough 0, diffAmt 0.8 white |
| White LED | advancedMaterial, **ashikhmin** | radiance 1.7, lumiCol 0.98 grey, diffAmt 0.15, specAmt 0.01, specFres 0.02 |
| Brown Leather | advancedMaterial, gtr | diffCol dark brown, specAmt 0.04, rough 0.1, refIndex 1.3, useRefIdx on. 9 procedural layers: Modo noise on diffColor, layerMask and textureOffset; third-party noises on bump and diffColor |
| Unreal Bowling Ball | unrealShader | 4 image maps: baseUE, normalUE, roughUE, metallicUE. Material base 0, metallic 0.04, specular 0.64, roughness 0.5 |
| Unity Bowling Ball | unityShader | image maps: albedoUT, normalUT, plus one packed map giving metallicUT from its **red** channel. Material smoothness 0.7, fresrel0 0.04 |

**Images** are referenced by absolute path once loaded in Modo, for example `/Library/Application Support/Luxology/Content/Assets/Images/PBR/...`. How the path is written inside the file (absolute or alias) is `[?]`; see section 4. The colour space is `(default)` for colour and scalar maps alike, and `(none)` for normal maps.

## 2. Pipeline
1. The browser shows a .lxp as a **Materials** entry when DESC names a material type. Others stay greyed (MC-12).
2. **Read.** Use the LXO item and channel reader on the preset FORM. Keep mask, material, texture layers, image clips and texture locators. Skip anything else with a note.
3. **Map** onto Operandi Material channels and texture layers (OPM-01..04). Section 3 gives the result in OpenPBR terms.
4. **Use.** Behaves the same as dropping any library material today: on a mesh, a polygon selection or a material group, in one undo step. The preset's mask is replaced by the drop target. `[O][R]`
5. **Report.** One import note lists everything dropped or approximated.

## 3. Channel mapping
### 3.1 advancedMaterial, Physically Based (`brdfType = gtr`)
| UI label `[D]` | Channel `[C]` | OpenPBR | Rule |
|---|---|---|---|
| Diffuse Color | diffCol | base_color | |
| Diffuse Amount | diffAmt | base_weight | Clamp to 1 and note it if higher. |
| Diffuse Roughness | diffRough | base_diffuse_roughness | |
| Use Refractive Index | useRefIdx | | When on, Refractive Index sets specular_ior and **Specular Amount is ignored** `[B]` (glass: specAmt 0.02 with IOR 1.52). |
| Specular Amount | specAmt | specular_ior | Only when useRefIdx is off. Read it as F0; IOR = (1+√F0)/(1−√F0), so 0.04 gives 1.5. `[R]` |
| Specular Color | specCol | specular_color | |
| Fresnel (specular) | specFres | | Seen at 1.0 on PB presets. Drop it, and note it only if it isn't 1. |
| Roughness | rough | specular_roughness | Same 0–1 scale assumed `[?]`. |
| Blurry Reflection | reflBlur | | Off means mirror-sharp reflections, so set specular_roughness to 0 **for the reflection only**? OpenPBR can't split them, so ignore it and keep Roughness. `[R]` |
| Match Specular | reflSpec | | When on, the reflection follows the specular settings (no extra mapping). When off, see 3.3. |
| Anisotropy | aniso | specular_roughness_anisotropy | |
| Clearcoat Amount / Roughness | coatAmt / coatRough | coat_weight / coat_roughness | |
| Clearcoat Bump | coatBump | coat normal follows bump × this | Approximate. |
| Transparency Amount / Color | tranAmt / tranCol | transmission_weight / transmission_color | |
| Absorption Distance | tranDist | transmission_depth | Stored in metres. 0 means thin. |
| Refractive Index | refIndex | specular_ior | |
| Dispersion | disperse | transmission_dispersion_* | 0 in every preset tested. When nonzero, use scale 1 and Abbe 40, with a note. `[R]` |
| Refraction Roughness | tranRough | specular_roughness | Only if it is the larger of the two, and note it. `[R]` |
| Dissolve Amount | dissAmt | geometry_opacity = 1 − dissAmt | |
| Subsurface Amount / Color | subsAmt / subsCol | subsurface_weight / subsurface_color | |
| Scattering Distance | subsDist | subsurface_radius | Metres (default 0.005). |
| Front Weighting | subsPhase | subsurface_scatter_anisotropy = 2·subsPhase − 1 | 0.5 gives 0. `[R]` |
| Luminous Intensity × Luminous Color | radiance × lumiCol | emission_luminance, emission_color | `radiance` is render-unit radiance, taken 1:1 (OPM-Q09, 1 nit = 1 render unit; answered 2026-11-15, LXP-B02b). |
| Metallic (Principled) | metallic | base_metalness | |
| Specular Tint | specTint | specular_color = mix(white, diffCol, t) | |
| Sheen / Sheen Tint | sheen / sheenTint | fuzz_weight / fuzz_color = mix(white, diffCol, t) | |
| Flatness | flatness | n/a | Drop, with a note. |
| Refractive Index from Specular Amount | specRefIdx | specular_ior = 1 + 0.8·specAmt | Documented line `[D]` |
| Bump Amplitude | bumpAmp | Operandi bump amplitude | Metres |
| Displacement Distance | displace | Operandi displacement | Metres |
| Smoothing, Smoothing Angle (radians), Double Sided, Rounded Edge Width and Angle | smooth, smAngle, dblSided, rndWidth, rndAngle | Operandi channels | 1:1 `[O]` |
| Reflection Type, Rays, Importance, Exit Color, Clipping, Volumetrics, LPE Label, Boolean | reflType, reflRays, tranRays, importance, exitCol, clip*, scatter*, absorb*, density, lpeLabel, useBoolMtl | n/a | Drop, with one note line. |

### 3.2 "Metal made by hand" (LXP-Q1)
Polished Gold is the typical case: diffAmt 0, specAmt 1, a coloured specCol, and metallic 0. **Rule:** if diffAmt ≤ 0.05 and specAmt ≥ 0.7, then base_metalness = 1, base_color = specCol, specular_color = white. Note it as "converted to metal". `[R]`

### 3.3 Legacy models (`ashikhmin`, Traditional)
These have a separate reflection, `reflAmt` and `reflCol`, which matters when reflSpec is off. Map with 3.1 and treat specular weight as max(specAmt, reflAmt). Note "legacy shading model approximated". White LED is the example: its look is almost all emission, so this approximation is harmless. `[R]`

### 3.4 Unreal / Unity
| Source channel `[B][C]` | OpenPBR |
|---|---|
| unreal base, unity albedo | base_color |
| metallic | base_metalness |
| unreal roughness | specular_roughness |
| unity smoothness | specular_roughness = 1 − smoothness |
| unreal specular (0.5 means F0 0.04) | F0 = 0.08 × specular, giving specular_ior |
| unity fresrel0 | F0, giving specular_ior |
| emissive (× unity emissivelevel) | emission_color. Luminance 1 nit per unit `[?]` |
| unity normalscale / heightscale | normal map strength / bump amplitude |
| unreal shadeMode other than standard | Note it and treat it as standard. |

## 4. Texture layers
### 4.1 Effects
The `effect` values come from the config and the bridge, and drive OpenPBR via section 3:
| Effect ids | Target |
|---|---|
| diffColor, baseUE, albedoUT, baseColorGLTF | base_color |
| diffAmount | base_weight |
| specAmount, specUE | specular F0 → ior (as 3.1 / 3.4) |
| rough, roughUE, roughnessGLTF | specular_roughness |
| smoothUT | specular_roughness = 1 − value |
| metallic, metallicUE, metallicUT, metallicGLTF | base_metalness |
| specularTint, sheen, sheenTint | as 3.1 |
| subsColor, sssUE | subsurface_color |
| emisUE, emisUT, emisGLTF | emission_color |
| opacUE | geometry_opacity |
| ccUE, ccrUE | coat_weight, coat_roughness |
| bump, bumpUE, bumpUT | bump |
| normal, normalUE, normalUT, normalGLTF, normal2UT | normal map |
| displace | displacement |
| stencil | geometry_opacity (hard) |
| aoUE, aoUT, aoGLTF, detailMaskUT, albedo2UT, flatness | dropped, with a note |
| layerMask, groupMask | Operandi layer mask if we have it; else drop the layer and note it |
| textureOffset | dropped, with a note |

### 4.2 Layer settings
- **Kept** on every layer: `enable`, `opacity`, `blend` (normal, multiply, add, screen, plus any others Operandi has; unknown ones fall back to normal with a note) and `invert`.
- **imageMap** keeps: `min`, `max`, `brightness`, `contrast` and `gamma`; `rgba` channel select: `use` means all channels, `red` means the red channel only, and so on. This is how packed maps work, e.g. Unity "mas" metallic from red; `swizzling`, `redInv` and friends.
- **Colour space:** `(none)` → linear; `(default)` → sRGB for colour effects, **linear for scalar effects** (roughness, metallic, bump, masks). Modo stores "(default)" on roughness and metallic maps too. `[R]`
- **Texture locator:** projection `uv` → UV map by name. If the name is missing on the target, use the first UV map and note it; `none` (procedural, object space) → our object-space procedural locator; other projections → our equivalent, else UV with a note; tiling: `tileU` and `tileV` (repeat, …).
- **Image path:** absolute path that exists → use it; otherwise try, in order: relative to the preset's folder, then the user's content root, with Modo alias prefixes (`asset:`, `content:`, `kit_Modo_Content:`) mapped onto the content folder the user added; still missing → keep the layer with a "missing image" tile, and note it.
- **Procedurals:** Modo noise and gradient → our noise and gradient where we have one; a gradient driven by `incidenceAngle` → our facing-ratio gradient if we have one, else drop it and note it; **third-party types (`*.RJJ` and any unknown type)** → drop the layer and note it. The browser tile says "some layers not supported".

## 5. Browser behaviour
- **LXP-01** A material .lxp is a normal (not greyed) Materials entry, with Modo's thumbnail (MC-30).
- **LXP-02** Tile badge: "Modo material". If the conversion drops anything, an info dot opens the note.
- **LXP-03** Drop, double-click and right-click behave like any library material. Nothing is written into the content folder.
- **LXP-04** A preset with no material item → greyed (MC-12), with tooltip "Modo preset with no material we can read."
- **LXP-05** A preset whose look depends mainly on dropped layers (more than half its layers dropped, or every bump/diffColor layer dropped) still imports. Its tile gets a "partial" badge so nobody is surprised. `[R]`

## 6. Open questions (all defaults APPROVED by Jason 2026-11-14 22:08 CT)
| # | Question | Recommended default |
|---|---|---|
| LXP-Q1 | Convert "metal made by hand" (3.2) to metalness? | **Yes**. It gives a truer look in Cycles. |
| LXP-Q2 | Diffuse Amount over 100 %? | Clamp to 1 and note it. |
| LXP-Q3 | Procedural layers we lack, including third-party plug-in noises? | Drop them with a note and a "partial" badge. Don't refuse the preset. |
| LXP-Q4 | Thumbnail: Modo's, or our own render? | Ours once it's rendered, because it shows what the user will actually get. Modo's until then. |
| LXP-Q5 | Luminous Intensity units? | Answered 2026-11-15 (LXP-B02b): render-unit radiance, 1:1, no × 683. |
| LXP-Q6 | Roughness scale the same as ours? | Assume yes. Check with one sphere render side by side before we call it done. |

---
## 7. Procedural layers: Modo built-ins → Operandi procedurals (v1.1)
### 7.1 What the presets actually use `[B]`
A survey of 60 presets, one from each of 60 of the 104 material folders that ship with Modo:
| What the preset's look depends on | Presets |
|---|---|
| Material values only (no layers) | 7 |
| Image maps only | 16 |
| Image maps + Modo built-in procedurals | 6 |
| Modo built-in procedurals only | 18 |
| Any third-party plug-in procedural (`*.RJJ`) | 13 (7 of them plug-in only) |
So **about 78 % can convert fully** once the built-in procedurals below map. About 22 % lose some layers to the third-party pack.

Layer types by number of presets using them:
| Type | Presets | Used for |
|---|---|---|
| imageMap | 23 | everything |
| **gradient** | 20 | mostly **Incidence Angle** (26 of 37 gradients), on Diffuse Amount, Diffuse Color, Roughness, Spec/Refl Color and Group Mask. Also Driver A (7) and Hair Length (4, fur only). |
| noise | 10 | Fractal (one Turbulence), on Bump (8), Diffuse Color (4) and a few others |
| wood | 7 | Diffuse Color / Bump |
| constant | 3 | |
| grid (Square), cellular (Angular), checker, process | 1–2 each | |
| occlusion | 1 | dirt in crevices |
| schematic nodes, channel modifiers, assemblies | 4 | node-driven looks |
| fur material | 3 | fur |
| thin film material | 1 | |

### 7.2 Mapping rules
Our procedural layers were built with the same setting names and two-zone model as Modo's (procedural-textures spec 2026-11-07). Most channels therefore map 1:1 by name.
| Modo type | Channels `[B]` | Operandi | Rule |
|---|---|---|---|
| All texture layers | enable, opacity, blend, invert, effect | same | Effect per section 4.1 |
| Two-zone procedurals | value1, color1, alpha1, value2, color2, alpha2, bias, gain | zone 1 / zone 2, Bias, Gain | 1:1 |
| noise | type (fractal, simple, turbulence), freqs, freqRatio, ampRatio, smoothTops, flowNoise, angle (Flow Phase) | Noise | 1:1 |
| cellular | type (angular, round), cellWidth, transWidth, freqs, freqRatio, ampRatio, smoothTops, variation, varyHue, varySat | Cellular | 1:1 |
| wood | ringScale, wave, waveScale, grain, grainScale | Wood | 1:1 (Waviness = wave, Graininess = grain) |
| grid | type, lineWidth, transWidth, variation, varyHue, varySat, varyAlpha | Grid | 1:1 |
| checker, dots | type, transWidth, dotWidth | Checker, Dots | 1:1 |
| constant | value, color | Constant | 1:1 |
| gradient | param (input), inVal, value / color curves | Gradient | The input maps by name (incidenceAngle → Incidence, slope, …, driverA–D → Driver A–D). The curve keys come from the published gradient-value record. Interpolation between keys follows **our** gradient's rules. Note "gradient curve approximated" only if the file asks for a key type we lack. |
| process | hue, sat, val, hueShift, bias, gain | Process (colour correct), if we have it; else drop and note | |
| occlusion | type, rays, dist, spread, sameSurf, … | Occlusion texture if we have it (Cycles AO); else drop and note | |
| Driver A–D layers (effect driverA…) | | Driver layers feeding our gradient | 1:1 if built; else drop the driver **and** its gradient together, with one note |
| Texture locator | projection, size, position, rotation | our locator | 1:1 (texture-locator spec) |
The scale is the same: a 1 m locator gives the same feature sizes in both.

### 7.3 Things we drop
| Source | Behaviour |
|---|---|
| Third-party `*.RJJ` procedurals | Drop the layer, note it, and add the "partial" badge. **Never approximate them by name** (no "Agate" or "Camo" look-alikes): that would be copying someone else's product. |
| Schematic node / channel-modifier / assembly-driven layers | Drop, with a note. Node materials come later. |
| Fur material, Hair Length gradients | Drop, with a note (fur is out of scope). |
| Thin film material | Map to OpenPBR thin_film if present; else drop with a note. |

### 7.4 Gaps worth closing (our own designs, not copies)
These would raise the "converts fully" share and help every Operandi user, not just Modo owners:
1. **Incidence gradient**: already specced; make sure Incidence + Diffuse Amount / Roughness / Group Mask all work. **This is the single biggest win** (1 in 3 presets).
2. **Driver A–D**: noise driving a gradient's colour, 7 of 37 gradients.
3. **Occlusion / dirt** and **Process** (colour correct), if not built yet.
4. Our own **look layers**, designed from real materials: scratches, edge wear (curvature), dirt. These are new Operandi procedurals with their own names. They are **not** substitutes for the dropped plug-in layers.

### 7.5 More open questions (defaults APPROVED by Jason 2026-11-14 22:08 CT)
| # | Question | Recommended default |
|---|---|---|
| LXP-Q7 | Should a dropped plug-in layer become a generic Operandi noise, so the material at least has *some* texture? | **No.** Drop it and show the badge. A wrong texture is worse than a clean one, and the user can add our noise in one click. |
| LXP-Q8 | Should we build the 7.4 gaps before or after the .lxp importer? | **Before, or alongside.** Incidence and Driver gradients make about a third of presets convert properly. |
