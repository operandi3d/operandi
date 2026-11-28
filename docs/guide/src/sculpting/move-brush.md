<!-- help-ids: sculpt.move ; template: tool ; since: 0.1.2 -->
# Move brush

The **Move** brush grabs the surface under the brush and drags it with the
pointer, so you can push a whole area around in one stroke.

## How to use

1. Select the mesh you want to sculpt.
2. Click **Move** in the **Sculpt** tab (in the Brush tool set).
3. Press on the mesh and drag. Everything under the brush when you pressed
   follows the pointer, the centre most of all.
4. Let go to finish. Each stroke is one undo step.

## Properties

| Property | What it does | Default |
| --- | --- | --- |
| **Sculpt Mode** | What the brush sculpts. Operandi sculpts mesh points, so the image and curve choices are greyed. | Automatic |
| **Move Mode** | **Normal**: points follow the pointer. **Slide**: they slide along the surface instead of lifting off it. | Normal |
| **Move Back Faces** | Also grabs the points inside the brush that face away from you, so the far side of the model moves with the near side. | Off |
| **Offset Amount** | How closely the grabbed points follow the pointer. | 100 % |
| **Smooth Amount** | How strongly Shift smooths instead of moving. | 50 % |
| **Rigid UVs** | Greyed: Operandi's brushes don't move UVs yet. | Off |
| **Lock Borders**, **Lock Corners** | Keep points on open edges, or only border corners, from moving. The same switches as in the Sculpt tab. | Off, On |

## Tips

- With **Move Back Faces** off, Move only grabs the side of the model you
  can see. On a thin shape with a big brush, that can pull the front away
  from the back. Turn **Move Back Faces** on to move both sides together.
- Symmetry works with Move too: the mirrored side moves the same way.

## Related

- [Inflate brush](inflate-brush.md)
- [Multiresolution](multiresolution.md)
