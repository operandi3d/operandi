# Set Material — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Set Material gives polygons a material tag and makes sure a material for
that tag exists. Options: **Name** (text), **Color** (RGB), **Diffuse**
(%), **Specular** (%), **Smoothing** (on/off), **Default** (on/off), plus a
library switch and a material type (Default, Unreal, Unity, glTF, AxF)
that were left at Default/off and not probed. In the UI the options come
from a dialog. Points and polygons never change.

## Rules

1. **Which polygons get the tag:**
   - Polygon mode: the selected polygons (SM01); nothing selected → every
     polygon of the mesh (SM03).
   - Item mode: every polygon of the selected mesh (SM06).
   - Vertex or Edge mode with something selected: **no polygon** is tagged
     (SM05, SM10, SM11), but the material is still created (rule 3);
     with nothing selected → every polygon (SM12, SM13). Decided: Operandi
     copies this (Q-SM-01).
2. **The tag** is Name exactly as typed, spaces and slashes included
   (SM07).
3. **New name:** a material group is added at the top level of the shader
   tree, named `<Name> (Material)`, matching polygons whose Material tag is
   Name. Inside it is a material named `M_<Name>` with diffuse colour =
   Color, diffuse amount = Diffuse, specular amount = Specular, and
   smoothing on or off as given, with a smoothing angle of 40°. Its other
   values are the ordinary new-material defaults (SM01, SM04). This
   happens even when no polygon was tagged (rule 1, SM05).
4. **Existing name:** no new group or material; the existing material's
   diffuse colour is changed to the new Color (SM02). Whether Diffuse,
   Specular and Smoothing are also overwritten wasn't tested (Q-SM-02).
5. **Name "Default"** is not special: a `Default (Material)` group is
   created like any other (SM08).
6. **Default on:** Name also becomes the material tag that every polygon
   created from then on gets (SM14: a new mesh built afterwards was all
   Kiwi; setting Default on again with "Default" restored it). The tagged
   polygons themselves follow rule 1 as usual (SM09).
7. **Selection after:** unchanged, mode unchanged (all cases).

## Open questions and decisions

- **Q-SM-01** (decided below) Vertex/Edge mode with a selection tags nothing but
  still creates the material. Sane rule for Operandi: match Modo (tag
  nothing, create the material); alternative is to tag the polygons
  touching the selection. Recommendation: match Modo.
- **Q-SM-02** Existing name: are Diffuse, Specular and Smoothing
  overwritten like Color?
- **Q-SM-03** How long Default lasts (scene, session or saved preference)
  and what "new polygons" covers (primitives, tools that add polygons).
- **Q-SM-04** Library switch and material types other than Default; the
  dialog's first-use values.

## Decided (2026-11-05)

- **Q-SM-01 → decided:** with something selected in Vertex or Edge mode,
  no polygon is tagged, but the material is still created (as Modo does).
