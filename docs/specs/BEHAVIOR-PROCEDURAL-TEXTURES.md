# Procedural texture layers (behaviour spec v0.1, 2026-11-07)

Tags as in BEHAVIOR-TEXTURE-LOCATOR.md: [measured], [docs], [chosen].

## 1. Shared

t in 0..1 -> Bias/Gain -> two zones (Value, Color, Alpha each); number
effects mix values, colour effects mix colours with alpha into opacity;
then Blend/Opacity/Invert. Add Layer above Base Material, Diffuse Color,
own locator Solid/X/1 m. Features in metres at 1 m, scale with Size.
Common starts: zone 1 Value 0 / black, zone 2 Value 1 / white, Alphas 100%,
Bias/Gain 50%. Exceptions per type.

## 2. Pattern types

### Checker

Type Square (default) / Cube; Transition Width 10%; Value / Hue /
Saturation / Alpha Variation 0. Squares are half the locator size, edges on
multiples of half-size from the centre. Square: 2D checker in the plane
perpendicular to the Projection Axis; axis Z zone 2 where
floor(2Lx)+floor(2Ly) is even. Cube: 3D, zone 1 where
floor(2Lx)+floor(2Ly)+floor(2Lz) is odd. Transition Width: smoothstep ramp
across each edge, total width = Transition Width x square size, centred on
the edge. Over 100% allowed.

### Grid

Type Square (default) / Line / Triangle / Hexagon / Cube; Line Width 10%;
Transition Width 10%; variations as Checker. One cell = locator size, cells
centred on the locator; lines on half-size positions. Lines are zone 1
(Line), the space between zone 2 (Filler). Line = one set of parallel
lines; Square = two perpendicular sets; Triangle = Square plus one diagonal
set; Hexagon = hexagonal cells; Cube = three sets (3D).

### Dots

Type Square (default) / Hexagon / Cube; Dot Width 80%; Transition Width
10%; variations as Checker. Dots zone 1, filler zone 2; centres at cell
centres; diameter = Dot Width x cell (100% = touching). Hexagon: every
other row shifted half a cell. Cube: 3D lattice of spheres.

### Noise

Type Fractal (default) / Simple / Turbulence; Frequencies 4; Frequency
Ratio 2.0; Amplitude Ratio 0.5; Smooth Tops 0; Flow Noise off, Flow Phase
0. Smooth gradient noise; Fractal sums octaves, each Frequency Ratio x finer
and Amplitude Ratio x weaker; Simple = 1 octave; Turbulence = each octave
folded. Default look: soft, low-contrast blobs about the locator size,
centred on 0.5 (t about 0.41-0.73 over a 1.4 m patch, mean about 0.55).
Smooth Tops: the higher t, the smaller the fine octaves. Flow Noise
animates in place by rotating gradients with Flow Phase (one cycle per
1.0).

### Cellular

- Type Angular (default) / Round; Cell Width 60%; Transition Width 30%;
  Frequencies 1; Frequency Ratio 2; Amplitude Ratio 0.5; Smooth Tops 0;
  Value / Hue / Saturation Variation 0. [measured]
- Random feature points about one per locator-size cube. Cells are zone 1
  (Cell), gaps zone 2 (Filler). Angular: faceted cobblestones (distance to
  nearest point compared with second nearest). Round: soft round spots
  (distance to nearest only). Cell Width = how much of each cell is "cell"
  before the ramp; Transition = width of the ramp. [docs + measured look]
- Frequencies > 1 layer finer copies (as Noise). Variations randomise each
  cell's brightness / hue / saturation (colour effects only) by up to the
  percentage. [docs]

### Wood

- Zone 1 = Ring (starts colour 0.4 grey, Value 0), zone 2 = Filler (colour
  0.6 grey, Value 1). Ring Scale 25%, Waviness 50%, Wave Scale 100%,
  Graininess 50%, Grain Scale 10%. [measured]
- Concentric rings around the Projection Axis through the locator centre,
  spacing about Ring Scale x size (25% -> ~0.25 m). Waviness bends the rings
  with smooth noise of feature size Wave Scale x size. Graininess mixes in
  fine noise speckle of size Grain Scale x size. [measured spacing + docs]

### Ripples

- Zone 1 = Crest (starts white / Value 1), zone 2 = Trough (black / 0).
  Sources 8; Wavelength 20%; Phase 0. [measured]
- Sum of circular sine waves from Sources random centres within the locator
  volume; ring spacing = Wavelength x size; Phase shifts the waves
  (negative outward, positive inward), 100% = one wavelength [chosen].
  Result rescaled to 0..1. [docs]

### Weave

- Zone 1 = Yarn (starts white / Value 1), zone 2 = Gap (black / 0). Yarn
  Width 60%; Roundness 60%. [measured]
- Over-under basket weave of two perpendicular yarn sets in the plane
  perpendicular to the axis, one over-under repeat per locator size
  [chosen]. Yarn Width 100% closes gaps; Roundness 0% = flat strips, 100% =
  tube-shaped shading across each strand (feeds bump nicely). [docs]

### Variations (Checker / Grid / Dots)

Value / Hue / Saturation / Alpha Variation apply per square/cell, behaving
like Cellular's [chosen].

## 3. Extra types

### Gabor Noise (type val.noise.gabor)

- Zone 1 white / Value 1, zone 2 black / 0; Orientation 45°; Frequency
  0.0625; Bandwidth 0.05; Random Offset 1; Brightness 0%; Contrast 0%
  (these two replace Bias/Gain). [measured]
- Stripy directional noise; Orientation turns stripes about the axis;
  Bandwidth low = clean stripes, high = random. At starting values the
  default renders almost entirely zone 2 with sparse specks; that's the
  intended look, keep it. [measured + docs]

### Poisson Cells (type val.noise.poisson)

- Zones Cell white / 1, Filler black / 0 (separate Cell Alpha, Filler
  Alpha). Type Angular; Cell Width 90%; Transition 5%; Cell Density 100%;
  Level 3; Solid 3D Cells off; Brightness / Contrast 0%; Hue / Saturation /
  Value Variation 0; Frequencies 1, ratios 2 / 0.5; Use Profile Gradient
  off. [measured]
- Evenly spaced non-overlapping cells that tile within the locator square;
  each Level doubles the cell count; Density = how much of its slot each
  cell fills. Solid 3D Cells off = planar-style tiling; on = carved from 3D
  (loses tiling). Profile Gradient replaces the linear cell-to-filler ramp
  with a curve. [docs]

Large plug-in library: out of scope.

## 4. Projection interplay

Solid = full 3D L. Planar / Cubic / Box / Triplanar = two in-plane coords,
depth 0 [measured for Planar]; 3D patterns look odd there (Checker Cube on
Planar = flat 50% mix); keep that. UV Map = (U*repeatU, V*repeatV, 0).
Repeat modes and UV Rotation don't affect procedurals.
(Cylindrical/Spherical: (u-0.5, v-0.5, 0) as in the locator spec.)

## 5. Viewport

Same evaluation as render where cheap; if a GPU version isn't ready, show
zone mix at t = 0.5 and say so in the tooltip [chosen].

## 6. Undo, file, commands

Adding = one undo step (layer + locator + Texture Group if new); each edit
one step. Everything saved, including stored-only settings. Add Layer
Textures section: Checker, Grid, Dots, Noise, Cellular, Wood, Ripples,
Weave, Gabor Noise, Poisson Cells. Type names: checker, grid, dots, noise,
cellular, wood, ripples, weave, val.noise.gabor, val.noise.poisson. Locator
type txtrLocator, its group a group locator.
