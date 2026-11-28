<!-- help-ids: Shader Tree ; template: panel ; since: 0.1.0 -->
# Shader Tree

The **Shader Tree** lists the layers that make up how the scene renders:
materials, shaders, textures, outputs, lights and environments. Layers
higher in the list sit on top of the ones below. It is the **Shader
Tree** tab of the panel at the top right.

![The Shader Tree of a new scene](../images/panel-shader-tree.png)

## Parts

| Part | What it does |
|---|---|
| **View** | Which tree to show. |
| **Assign Material...** | Gives the selected polygons a material, as **Set Material** does. |
| **Filter** | Shows only some kinds of layer. |
| **Add Layer ▾** | Adds a layer: a material, a texture, a shader, a gradient and more. |
| **F** | A filter field to find layers by name. |
| Eye column | Turns a layer on or off. |
| **B** | Blend mode. |
| **%** | How strongly the layer applies (its opacity). |
| **Effect** | What the layer sets: **Final Color**, **Alpha**, **Full Shading** and so on. |

A new scene has a **Render** item with an **Alpha Output**, a **Final
Color Output**, a **Base Shader** and a **Base Material**, plus the
**Library**, **Nodes**, **Lights**, **Environments**, **Bake Items** and
**FX** groups.

## Common tasks

- **Select a layer**: click it; its settings show in the
  [Properties panel](properties.md).
- **Reorder**: drag a layer up or down.
- **Right-click** a layer for its menu (instance, duplicate, delete and
  more).
- **Place a texture**: on its **Texture Locator** tab, set **Position**,
  **Rotation** and **Size**. The chain button on each group's first row
  links X, Y and Z, as on an item's transform: off, same value, scale
  together or add together.

## Related

- [Set Part and Set Material](../polygon/part-material.md)
