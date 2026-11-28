<!-- help-ids: sculpt.inflate ; template: tool ; since: 0.1.2 -->
# Inflate brush

The **Inflate** brush swells the surface under the brush, moving every
point outward along its own normal, like air filling a balloon.

## How to use

1. Select the mesh you want to sculpt.
2. Click **Inflate** in the **Sculpt** tab (in the Brush tool set).
3. Press on the mesh and drag. Curved areas swell from the inside; on a
   flat area it lifts the surface like Push.
4. Hold **Ctrl** while you drag to deflate (move points inward), or
   **Shift** to smooth instead.
5. Let go to finish. Each stroke is one undo step.

Right-drag in the viewport to change the brush size, and middle-drag to
change **Offset Amount**.

## Properties

| Name | What it does | Default |
|---|---|---|
| **Sculpt Mode** | What the brush sculpts. Operandi sculpts mesh points, so **Image Map (Details)** is greyed. | Automatic |
| **Offset Mode** | **Adaptive**: strength is a share of the brush size. **Absolute**: a fixed distance. | Adaptive |
| **Offset Distance** | How far a dab moves the points with **Absolute**. Greyed with **Adaptive**. | 100 mm |
| **Offset Amount** | Strength as a percentage of the brush size, with **Adaptive**. | 30 % |
| **Smooth Amount** | How strongly Shift (or **Uniform**) smooths. | 50 % |
| **Smooth Mode** | **Shift Key**: Shift smooths. **Low Pressure**: light pen strokes smooth (needs a pen tablet). **Uniform**: every stroke smooths a little too. | Shift Key |
| **Density Mode** | What crossing the same spot twice does. **Additive** adds more, **Attenuated** stops at the full offset, **Maximum Stroke** never goes past the strongest dab. **Maximum Map** is greyed. | Attenuated |
| **Interpolation Step** | Gap between dabs, as a share of the brush size. Smaller is smoother. | 5 % |
| **Continuous Stroke** | Joins the dabs into a solid line even with a large step. | Off |
| **Curve Constraint** | Guides strokes along a curve. Only **None** works for now. | None |
| **Paint Mode**, **Set Color**, **Color**, **Alpha** | Greyed: they work on image maps, which Operandi doesn't sculpt yet. | |
| **Lock Borders** | Points on open edges never move. The same switch as in the Sculpt tab. | Off |
| **Lock Corners** | Border points used by only one polygon never move. Also in the Sculpt tab. | On |

The **Smooth Brush** section (**Hard Brush** with the hard tip) sets the
brush tip:

| Name | What it does | Default |
|---|---|---|
| **Size** | Brush radius in screen pixels. | 10 px |
| **Auto Scale** | Keeps the brush the same size on the model when you zoom. | Off |
| **Bias** | Bends the falloff: 50 % leaves it as it is, higher makes it fuller. | 50 % |
| **Shape Preset** | The falloff from the centre to the rim: Linear, Smooth, Bulge, Fat, Sharp or Very Sharp. | Fat |
| **Brush Settings** | Opens **In Weight**, **Out Weight**, **In Slope** and **Out Slope** to shape the falloff yourself. Changing one sets **Shape Preset** to Custom. | 50 %, 50 %, 0, 0 |

## Tips

- Inflate has no **Normal Mode**: it always uses each point's own normal.
  That is why it swells round shapes evenly instead of pushing them one way.
- Brush settings are kept per brush, so Inflate remembers its own size and
  strength when you switch brushes.

## Related

- [Move brush](move-brush.md)
- [Multiresolution](multiresolution.md)
