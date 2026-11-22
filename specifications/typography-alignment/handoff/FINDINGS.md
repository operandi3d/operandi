# Font findings and proposed correction

## Confirmed observations

| ID | Finding | Limit |
|---|---|---|
| FONT-R01 | Modo17.0v8, version17.0.8.207049, reports Lucida Grande Regular for its application font. QFontInfo resolves the requested family without fallback. Point size10, pixel size10, Qt weight400/Regular, no italic. All69 visible widgets queried reported the same font properties. | Public runtime metadata. A custom draw routine can select another font while painting; this query did not intercept every draw call. Not a claim about Modo16 or factory preferences on every installation. |
| FONT-R02 | The observed Modo runtime uses Qt6.3.1. Its screen query reports device-pixel ratio1.0, logical DPI72x72 and geometry5120x1440. Font strategy is PreferAntialias, hinting preference PreferDefaultHinting. | This is the observed logical desktop, not validation of every physical display/scaling mode. The preference does not prove a particular hinting algorithm was actually applied. |
| FONT-R03 | Current Operandi source loads the installed SFNS.ttf into egui as System sans when available and sets standard text styles to size11. Category-tab typography remains proportional size11, with a quarter-turn, ink centering and an origin snapped to physical pixels. | Source/report observation, not proof that every supplied screenshot comes from that exact binary. Some controls have explicit size overrides. Reading an OS font file into egui does not invoke the OS's native UI text renderer. |
| FONT-R04 | The installed SF font's metadata describes a Regular default face and an optical-size axis with minimum17, default28 and maximum96. Operandi's observed font setup does not request a variation-axis adjustment. The installed Lucida Grande collection contains an explicitly named Regular face. | Font metadata plus current configuration; no per-frame outline/atlas tracing established which optical variation the running app used. Treat the optical-size contribution as a hypothesis, not the proven sole cause of blur. |
| FONT-R05 | An original headless utility linked against the existing egui0.33.3 build measured both installed fonts at sizes9/10/11/12 and scales1x/2x. Lucida Grande Regular at size10 closely matches several Modo word advances. | Layout-only diagnostic. No full GPU composition, native pointer input, rotated-text clarity or corrected-app visual pass is credited. |

Modo's font metrics in the observed state are height11.765625,
ascent9.65625 and descent2.109375 logical units. The current runtime's 72 logical
DPI explains why its queried10-point font also reports pixel size10. Do not apply
a universal point-to-pixel conversion or blindly substitute 13.33 pixels.

## Measured advances

Measurements below are logical horizontal advances at1x. They are factual
summaries from the private read-only runtime query and original layout utility;
no font outlines or application drawing code accompany them.

| Text | Modo Qt Lucida Grande10 | egui current SF11 configuration | egui Lucida Grande10 candidate |
|---|---:|---:|---:|
| Create | 31.359375 | 30.625 | 30.5625 |
| Polygon | 38.34375 | 36.78125 | 38.21875 |
| Primitives | 47.15625 | 44.125 | 47.09375 |

For Primitives, the absolute advance difference falls from3.03125 to0.0625
logical units. For Polygon it falls from1.5625 to0.125. Create remains about
0.797 shorter in the candidate, so this is not exact text-layout parity.
The candidate egui layout height is12; the current SF11 diagnostic height is13.
Advance agreement does not measure stem clarity, anti-aliasing or perceived weight.

## What explains the visible difference

The family/size mismatch is confirmed and changes glyph proportions and spacing.
It is the first variable to correct in a controlled test. The observed SF optical
size is another plausible contributor: optical sizing can change stem weight,
proportions and fine detail independently of geometric scaling. The
[OpenType optical-size definition](https://learn.microsoft.com/en-us/typography/opentype/spec/dvaraxistag_opsz)
explains that distinction.

The earlier explanation emphasizing egui's rasterizer was incomplete. Egui's
[font-atlas documentation for the deployed version](https://docs.rs/epaint/0.33.3/epaint/text/struct.Fonts.html)
and its [text-antialiasing investigation](https://github.com/emilk/egui/issues/2639)
establish relevant rendering behavior and known small-font concerns, but do not
prove that rasterization alone causes these screenshots' difference. Modo's
hinting preference likewise does not prove it uses stronger hinting than Operandi.

Both supplied crop files carry the same ICC profile metadata. This removes an
obvious profile-metadata mismatch from the leading explanations, but does not
establish equal display scaling, screenshot resampling or all compositing behavior.

## Proposed correction

**FONT-001:** Test the installed Lucida Grande Regular face as the macOS UI
font, starting at size10 for the standard chrome and category labels. Confirm
actual resolved family/face and scale; calibrate using the metrics above plus
actual-size visual review. Do not blindly reduce every text style, numeric field,
heading or shortcut overlay. Preserve text roles, clarity and accessibility.

**FONT-002:** Apply typography consistently to standard menus/buttons, disclosures,
component labels and the category strip. Recalculate label metrics, baseline/ink
placement and clipping after a family change. Preserve current grid/button sizes,
compact category surfaces, top alignment, state colors and physical-pixel origin
handling. Do not resize the approved primitive/transform grid to hide a mismatch.

**FONT-003:** Use the already installed macOS font at runtime for this experiment;
no font file is supplied or admitted for bundling/publication. Handle unavailable
fonts explicitly and retain a documented fallback. Windows/Linux need their own
permitted font choices and metric/clarity checks; no platform pass is implied.
Keep font loading and scale integration behind the existing portable UI boundary.

**FONT-004:** If an appearance difference remains after matching family, face,
size, text color, background and scale, isolate atlas generation, coverage/blending,
filtering and physical placement one variable at a time. Compare horizontal and
rotated labels separately. A wholesale toolkit replacement, new rasterizer or
global gamma/filtering change is not justified by the current evidence alone.
Any such expansion requires its own bounded proposal and dependency review.

These are correction proposals, not user-approved implementation decisions.
No implementation source, approval records or build configuration was changed
by this investigation.
