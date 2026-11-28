<!-- help-ids: Solid Sketch, solid.sketch ; template: tool ; since: 0.1.0 -->
# Solid Sketch

**Solid Sketch** builds a smooth, solid shape from a chain of nodes you click in the **3D view**: each node is a box, and the boxes are joined into one mesh.

![A solid shape through four nodes clicked on the ground, with its Tool Properties](../images/solid-sketch.png)

## How to use

1. Make sure the mesh you want to work on is selected in the **Items list**.
2. Click **Solid Sketch** in the **Draw** section of the **Create** tool tab.
3. Click in the **3D view** to add a node. Each click adds the next node,
   joined to the one before.
4. Drag a node to move it. Drag its red, green or blue square to size it
   along X, Y or Z, or drag the dark green dot beyond the green square left
   and right to twist it.
5. Press **Q**, or click **Drop**, to keep it and close the tool.

The **Solid Sketch** button starts the tool with **Subdiv Mode** set to
**Level 1**, so the shape comes out smooth.

## Properties

| Name | What it does | Default |
|---|---|---|
| **Edit Mode** | **Create / Move / Scale**: click to add nodes, drag to move and size them. **Destroy**: click a node to delete it. | Create / Move / Scale |
| **Subdiv Mode** | **none**: plain polygons. **Level 1**: the shape as subdivision polygons. **Level 2**: smoothed once more (four times the polygons). Greyed until the first node exists. | Level 1 |
| **Rotate Handles** | Shows rings round the current node, to turn it and swing the nodes after it. | Off |
| **Current Node** | Which node the fields below edit. | the last one |
| **Center** | The current node's position (X, Y, Z). | where you clicked |
| **Size** | The current node's size (X, Y, Z). | 0.1 m |
| **Twist** | Turns the current node round the chain. | 0° |
| **Auto Align** | Turns each node to blend with the links in and out of it. | Off |
| **Auto Merge** | Nodes that overlap their neighbour merge into it instead of pinching. | Off |

**Negative** is greyed: it isn't available yet.

## Related

- [Interface](../interface/window.md): the **Toolbox** and **Tool Properties**
- [Topology Pen](topology-pen.md)
