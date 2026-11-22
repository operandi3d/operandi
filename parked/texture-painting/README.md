# Parked: TextureCraft painting code (not built)

This folder holds three crates from
[TextureCraft](https://github.com/storytold/texturecraft) at commit
`9c1b9cbc`, copied unchanged for the coming 3D painting work. **Nothing
here is compiled into Operandi.** The folders are outside the Cargo
workspace and their `Cargo.toml` files are renamed `Cargo.toml.original`,
so no build, test or IDE picks them up by accident.

Licence: MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team and the
TextureCraft contributors (`LICENSE-MIT`, `LICENSE-APACHE` and `NOTICE`
here are theirs, unchanged). Both licences fit Operandi's GPL-3.0-or-later.
Keep the notices on anything taken from here. The ArtCraft name and logos
are not part of the licence and must not be used. TextureCraft isn't
derived from Modo, so this code is clean-room safe.

| Folder | What it is | Size |
|---|---|---|
| `paint/` | The brush engine: strokes from pointer samples (spacing along the surface, lazy mouse, jitter, symmetry), stamps rasterised into texture space over a UV G-buffer, back faces hidden, painting across UV seams, clone/smudge pickup, path and particle brushes, procedural brush alphas and presets, text alphas. | ~3.6k lines |
| `texel/` | Sparse 64×64 copy-on-write tiled images of `u16` channels: a snapshot is one reference-count bump, so per-stroke undo is cheap. Also colour-space helpers, noise and UV dilation. | ~1.2k lines |
| `compose/` | The CPU layer-stack compositor: layers, masks, fills and painted pixels into one image per channel, tile by tile in parallel, with a live preview of the stroke in progress, plus conversion to viewport texture formats. | ~6.8k lines |

## Already taken into Operandi

`texel/src/dilate.rs` (UV padding) lives on, adapted, in
`crates/operandi-core/src/bake/padding.rs`. The curvature bake and Atlas
Unwrap came from TextureCraft's `bake` and `geom` crates (not parked here).

## Starting the painting work from this

1. Make a new workspace crate (for example `crates/operandi-paint`) and
   move in what's needed, starting with `texel` (no dependencies) and then
   `paint`. Keep each file's licence credit at the top, like
   `bake/padding.rs` does, and add the crates to `docs/DEPENDENCIES.md`.
2. Replace their `texturecraft-core` / `texturecraft-geom` types (mesh,
   UV G-buffer, layer and brush settings) with Operandi's own: `Mesh` with
   per-corner UV maps, the bake rasteriser (`bake::rasterise`) and BVH
   (`bake::bvh`) already cover what a brush needs for picking and
   texture-space coverage.
3. Their `compose` layer model is TextureCraft's own; Operandi's layers
   live in the shader tree, so expect to keep the tile evaluation and
   blending maths and drop the layer types.
4. Paint tool *behaviour* (names, options, defaults) comes from the
   behaviour specs in `specifications/`, as always; this code is the
   engine, not the design.

The other dependencies the `Cargo.toml.original` files name (`rayon`,
`half`, `harfrust`, `skrifa`, `serde`) are permissive crates; record them
in `docs/DEPENDENCIES.md` when they come in.
