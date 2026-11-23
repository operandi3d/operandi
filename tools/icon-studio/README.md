# Icon studio

Rendered UI icons: small 3D scenes modelled in Operandi and rendered with
Cycles, so every icon made this way shares one camera, one light rig and one
set of materials. It is a tool for **new** icons. The existing icons come
from the vector generator in `tools/operandi-icons` and are approved as they
are; nothing here redraws them.

## What's here

| Path | What it is |
| --- | --- |
| `studio.opscene` | The studio: camera, key light, sky and shared materials. No geometry. |
| `icons/<slug>.opscene` | One icon scene each: a copy of the studio with geometry added. |
| `../../scripts/render-icons.sh` | Renders every icon and the contact sheet. |

Code: `crates/operandi-core/src/icon_studio.rs` (the studio and how icon
scenes are rendered through it) and `crates/operandi-app/src/icon_studio_cli.rs`
(`operandi-app --icon-studio`: the Cycles renders, outline and contact sheet).

## The studio

- **Camera:** orthographic, looking down the (1, 1, 1) diagonal at the
  origin: the generator's 30° isometric view. Its frame is sized so a 1 m
  cube centred on the origin fills the picture as the generator's unit cube
  fills its 32 px canvas. Keep every icon's silhouette within that cube's
  (a round subject can be a little bigger, as `prim-sphere` is); the margin
  left is for the outline.
- **Light:** one strong directional key from above and a little to the left,
  with crisp shadows, plus a dim, slightly cool 4-colour sky for fill. A clay
  cube's top, left and right faces come out light, mid and dark.
- **Materials:** picked by polygon tag (Polygon ▸ Set Material, or the tag
  field), in the generator's palette:

  | Tag | Colour | Means |
  | --- | --- | --- |
  | `Clay` (and untagged) | light grey | the subject |
  | `Selected` | yellow-orange `#FFD470` | selected, always |
  | `New` | cyan `#56B4D3` | new geometry |
  | `Axis X`, `Axis Y`, `Axis Z` | red, green, blue | X/Y/Z |
  | `Glow` | warm white `#FFE4A0`, glowing | lit things: lights, lamps |
  | `Ink` | dark `#263442` | small dark details |

  Any other tag renders as Clay, and the renderer warns about it.
- **Render settings:** transparent background, 512 samples, no denoising.

## Making a new icon

1. Open `tools/icon-studio/studio.opscene` in Operandi and **File ▸ Save As**
   `tools/icon-studio/icons/<slug>.opscene` (the slug is the file name, in
   lowercase with hyphens, like the generator's).
2. Model the subject inside the 1 m box. Look through the camera (View ▸
   Camera) to see the framing.
3. Tag polygons with the studio's material names above.
4. Run the renderer and look at the contact sheet.

Only the icon scene's **geometry** is used. Its own copies of the camera,
light, sky and materials are ignored at render time; the renderer always uses
`studio.opscene`'s. So an icon can't drift from the shared look, and changing
the studio restyles every icon at once.

## Rendering

```bash
scripts/render-icons.sh
```

builds the app with Cycles (the `qa` profile, as QA builds use) and renders
every icon in `icons/` to `development/icon-studio/`:

- two sets with the same file names: `<slug>/dark-outline/` for light
  interface colours and `<slug>/light-outline/` for dark ones, each holding
  `<slug>-16.png`, `-20`, `-24`, `-32`, and `-16@2x.png` … `-32@2x.png` for
  high-density screens;
- `contact-sheet.png`: every icon at every size at its real pixel size, plus
  the 16 px one zoomed 4×: the dark-outline set on the generator's light
  preview background, the light-outline set on its dark one.

Each size is **rendered at that size**: the camera's frame is fitted to a 16,
20, 24 or 32 px picture (or twice that), and Cycles' own pixel filter and
samples anti-alias the edges. Nothing is shrunk from a big render.

After the render each picture gets an outline round its silhouette, the
generator's edge stroke scaled to the picture (0.96 px at 32 px, never under
0.7 px): ink `#263442` in the dark-outline set, `#DCE3E9` in the
light-outline set. Each size is rendered once and outlined twice.
`--no-outline` writes one `no-outline/` set instead.

The app chooses the set with **Preferences ▸ Display ▸ Colors ▸ Rendered
Icon Outline** (`rendered_icons.rs`): Automatic (the default) takes the dark
outline when the scheme's Panel colour is lighter than mid grey and the
light outline otherwise; Dark Outline and Light Outline force one.

Options: `scripts/render-icons.sh icons/prim-cube.opscene` renders just that
one; `--samples <n>` overrides the studio's samples; `--out <folder>` writes
elsewhere.

The command underneath:

```text
operandi-app --icon-studio template <studio.opscene>        writes the studio
operandi-app --icon-studio sample <slug> <icon.opscene>     writes a sample icon
operandi-app --icon-studio render <icon.opscene | folder>... --out <folder>
             [--studio <studio.opscene>] [--samples <n>] [--no-outline]
```

`studio.opscene` and the samples (`prim-cube`, `select-polygon`,
`prim-sphere`, `prim-cylinder`, `edge-bevel`, `light-point`) were written by
`template` and `sample`; rerun them only to start the studio over.

## Not done yet

- In the app: the icons listed in `RENDERED`
  (`crates/operandi-app/src/rendered_icons.rs`) are drawn from these renders
  instead of the generator's SVGs (79 so far: the primitives, the selection
  modes, curves and pen, the polygon, edge, vertex, duplicate, slice and
  deform tools listed there). To swap another icon, add its scene
  here, add a line to `RENDERED` and `FILES`, and run
  `scripts/render-icons.sh --install`. Sizes between the installed ones
  (16, 20, 24, 32, 40, 48, 64, 128 px) are scaled down from the next one
  up.
- No per-size pixel hinting: at 16 px a few edges fall between pixels and go
  soft. Moving the subject a fraction of a pixel per size would sharpen them.
- Only silhouette outlines: inner edges show by shading alone.
- **Curves** are modelled as thin tubes (`icon_studio::tube`, smoothed
  through their points with `through`), with small balls for their points:
  Cycles renders surfaces, not lines. Cyan for the new curve, Clay for
  points, Selected for handles.
- The light is the "more dramatic" rig Jason chose (2026-11-20): a strong
  key with crisp shadows and about half the sky fill of the first rig. Faces
  come out more neutral and contrasty than the generator's greys.