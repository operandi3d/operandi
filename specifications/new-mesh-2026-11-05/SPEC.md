# New Mesh — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05. REA-assisted: no.

## Summary

Two ways to add an empty mesh: **New Mesh Item** (Items menu/toolbar) and
the item list's **New** (new layer). Both add one empty mesh named "Mesh"
and make it the only selected item. They differ in where it goes.

## Rules

1. **Empty:** the new mesh has 0 points and 0 polygons. Its transform
   wasn't read (Q-NM-04).
2. **Name:** "Mesh"; if that's taken, "Mesh (2)", then presumably "Mesh (3)"
   (NM06).
3. **Selection:** the new mesh becomes the only selected item; everything
   selected before is deselected (NM02, NM04).
4. **Selection mode is kept:** it doesn't switch to Item mode. From Polygon
   mode it stays Polygon (NM03).
5. **New Mesh Item** always puts the mesh at the top level (no parent), even
   when the selected mesh has a parent (NM05). It's added after the existing
   items.
6. **Item list New** puts the mesh beside the selected item: same parent
   (NM08: under "Parent", next to the selected "Child"). With an unparented
   selection it's top level (NM07).
7. Works with nothing selected and in an empty scene (NM01).

## Open questions and decisions

- **Q-NM-01** Exact position in the item list (directly after the selected
  item, or at the end of its parent's children). Order was read only as
  creation order.
- **Q-NM-02** Several selected items with different parents, for the item
  list's New.
- **Q-NM-03** Whether a default mesh preset (if one is set in preferences)
  changes what's created.
- **Q-NM-04** The new mesh's position/rotation/scale (expected origin, like a
  new scene's mesh; not read).
