# Texture Locator and projections (behaviour spec v0.1, 2026-11-07)

Tags: [measured] = seen in renders of the reference app, [docs] = from its
public help, [chosen] = our pick (revisable).

## 1. What a texture locator is

- Every texture layer gets its own Texture Locator item when added. Layer
  has a Locator setting; several layers may share one. [docs]
- New locators go into group item "Texture Group" (created on first need,
  reused). Named "<layer name> (Texture)". [measured]
- Normal item: Position, Rotation (+ order), Scale. Size X/Y/Z is the scale.
  Texture Locator sub-tab on the layer's Properties. Expander before the
  layer name in the Shader Tree reveals it. [docs]
- Deleting a texture layer deletes its locator (if unshared). Empty Texture
  Group stays. [measured]
- In viewports the locator draws as a small item gizmo; Planar shows its
  projection plane at Size so it can be dragged interactively. [docs]

## 2. Starting values [measured]

Projection Type Solid for procedurals and Constant; UV Map for Image Map
when mesh has UVs [docs]. Axis X. Position/Rotation 0. Size 1 m [chosen].
H/V Repeat: Repeat/Repeat. Repeat counts 1/1. UV Map (none) = first. UV
Rotation 0. World Coordinates off. World Transforms on. Front Culling
Project Through. Triplanar Blending Hardness 0%. Random Offsets (none),
Texture Offset Strength 0. Falloff Cubic, X/Y/Z 0 (off). Tangent dPdu dPdv.
Use Real World Size off. Particle Source (none).

## 3. Locator space

L = (P - position) rotated by inverse rotation, divided per axis by Size.
World Coordinates off: P in item's local space. On: world position. [docs;
frame chosen] World Transforms: store; implement when locator parenting
exists.

## 4. Projection types

Output is 2D (u,v) for images or a 3D point for procedurals. Tiling
(section 5) then applies to (u,v).

- Solid: image n/a (shows nothing) [chosen]; procedural gets L (3D).
  Default for procedurals.
- Planar: axis Z u = Lx+0.5, v = Ly+0.5 [measured]; axis X u = -Lz+0.5,
  v = Ly+0.5; axis Y u = Lx+0.5, v = -Lz+0.5 [chosen]. Procedural gets the
  two in-plane coords with depth 0 [measured]. Image centred on locator,
  one image per Size, +Y up on axis Z.
- Cylindrical: u = 0.5 + angle/360°, angle measured around the axis from +Z
  toward +X (axis Y) [measured]; v = L_axis + 0.5 [measured]. Procedural
  gets (u-0.5, v-0.5, 0) in locator units [chosen]. One wrap around; Size
  along the axis sets image height. (For axes X and Z, use the analogous
  cyclic choice of the other two axes, consistent with Planar's orientation
  [chosen].)
- Spherical: u as Cylindrical; v = 0.5 + latitude/180°, latitude from the
  plane normal to the axis, positive toward +axis [measured]. Procedural
  gets (u-0.5, v-0.5, 0) [chosen].
- Cubic: Planar from X, Y or Z, chosen per point by the largest component
  of the surface normal in locator space; follows locator rotation; faces on
  the negative side are NOT mirrored [measured for Z/-Z]. Procedural gets
  (two in-plane coords, 0) [chosen]. (Reference help says reverse faces
  flip; renders disagree; follow renders.)
- Box: as Cubic but six-sided and every face reads the right way round from
  outside; IGNORES locator rotation (world aligned), uses position and Size
  [measured rotation; position/Size chosen]. Procedural as Cubic.
- Triplanar: three planar projections blended by normal weights; Blending
  Hardness 0% = soft blend, 100% = hard switch (like Cubic) [docs].
  weight_i = abs(n_i)^k, k = 1 + 31*h^2, normalised [chosen].
- UV Map: mesh UV (named, else first) x repeat counts, then UV Rotation
  about (0,0) [docs]. Procedural (U*wrapU, V*wrapV, 0) [chosen]. Ignores
  position/rotation/Size.
- Front: projected from a chosen camera (adds Projection Camera setting)
  [docs]. Later stage (stored only).
- Light Probe: unwraps a mirror-ball image (environments) [docs]. Later
  (stored only).
- Implicit UV: per-fibre 0..1 UVs for fur [docs]. Stored only.

Projection Axis applies to Planar, Cylindrical, Spherical, Cubic, Box,
Light Probe [docs]; greyed for others.

## 5. Image tiling

After projection, u and v are multiplied by repeat counts, then per axis:
Reset = outside 0..1 the layer is fully transparent; Repeat = tiles
forever; Mirror = tiles flipping every other tile; Edge = clamp border.
[docs; Reset measured]

## 6. Other locator settings

- UV Rotation: rotates the image about UV (0,0), degrees. [docs]
- Front Culling: Project Through (default; texture shows on faces pointing
  away from the projection), Cull Backfacing (only faces toward the
  projection get it), Flip Backfacing (back faces get it mirrored in U and
  V), Flip U Backfacing, Flip V Backfacing. "Faces toward" = dot of normal
  and the projection axis direction > 0. [docs]
- Falloff: fades the layer's opacity with distance from the locator centre.
  Falloff value f per axis; 0 = off. Fully transparent at distance Size/f
  (f 0.5 on 1 m fades out over 2 m) [docs]. Types: Cubic (per-axis box,
  fade = max over axes), Spherical (radius), Linear X / Linear Y / Linear Z
  (one axis only). Linear fade from full at centre to 0 at that distance
  [chosen].
- Random Offsets: (none) / Particles and Replicas / Mesh Parts / Surfaces,
  with Texture Offset Strength (0..10; 10 = shift up to a whole texture),
  random shift per part. [docs] Store; Mesh Parts later.
- Use Real World Size + Size X/Y: store only.
- Tangent Vector Type: dPdu dPdv (default) / dPdu Cross Product. Belongs
  with normal-map work.
- Particle Source / Texture Replicator: store only.
- Auto Size: Position and Size from the bounding box of the surfaces the
  layer applies to; Size = bbox size (largest side for all three axes if
  uniform) [chosen].

## 7. Undo and file

Every locator edit = one undo step. Locators save with scene; old scenes
convert to UV Map, repeat 1.

## Open-question defaults (all approved)

Q-PT1 new locator at origin, 1 m, plus Auto Size. Q-PT2 World Coords off =
item local space. Q-PT3 procedurals on UV = (U*repU, V*repV, 0). Q-PT4
Planar X/Y: right way up looking down the axis from its positive side.
Q-PT5 Cubic follows rotation, no back-face mirroring; Box ignores rotation.
Q-PT6 noise/cellular/wood maths: match scale, contrast, default look, not
pixels. Q-PT7 image under Solid shows nothing. Q-PT8 triplanar
k = 1 + 31*h^2. Q-PT9 viewport fallback = flat zone average.
