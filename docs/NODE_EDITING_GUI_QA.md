# Node editing — GUI check (NE-01..NE-19)

Build: the QA app named in the thread (Cycles on). Fresh scene, one box.
Shading tab. Make a Material Group: Add Layer > Group, drag Base Material
into it. Right-click the group > Show as Nodes: the Node View panel docks
under the views (docs/NODE_VIEW_PANEL_GUI_QA.md).

1. **Layer boxes.** The window shows Base Material, a locked "Base
   Material Mix" (padlock) and Material Output, wired left to right. The
   side panel says "Right-click to add nodes".
2. **Add (NE-02).** Right-click empty canvas > Add > Texture > Noise: a
   Noise box appears where you clicked, dimmed (feeds nothing, NE-14). The
   Shader Tree shows "Nodes (1)" in italics at the bottom of the group
   (NE-11). Cmd+Z removes it in one step; Cmd+Shift+Z brings it back.
3. **Wire (NE-03).** Drag from Noise's output dot onto Base Material's
   Diffuse Color row: a wire stays, Noise is no longer dimmed, the row
   reads "(driven)" greyed. Viewport (Advanced or Default style) and a
   Preview render show the noise. Undo name: "Connect Noise > Diffuse Color".
4. **Refused wire (NE-08).** Add > Input > Texture Coordinates, drag its
   output over Diffuse Color: the wire turns red; release: no wire, the
   status line says the types don't fit.
5. **Driven in Properties (NE-04).** Select Base Material in the tree. In
   Properties, Diffuse Color is greyed with the hover reason, its status
   light has an orange core, and a "driven by nodes (Noise)" button sits
   under it. Click it: the node window comes forward with Noise selected.
6. **Unwire.** Drag the wire's end off Diffuse Color and drop it on empty
   canvas: the wire goes, Diffuse Color is editable again and the render
   looks exactly as before step 3.
7. **Mixing node locked (NE-05).** Try to drag the wire into the mixing
   node: refused with "Mixing nodes are locked". Select the mixing node:
   right-click > Delete Node is greyed with a reason; Delete key does
   nothing but the status line message. In the side panel set Opacity 0.5:
   the tree row's opacity shows 50 %, one undo step.
8. **Values on the boxes (NE-01).** On Base Material's box, drag the
   Roughness field: Properties follows live; release, then Cmd+Z undoes the
   whole drag in one step. Click Diffuse Color's swatch on the box and pick
   a colour: the same edit as Properties. On the mixing node only Opacity
   and Blend are fields (NE-05). Zoom the canvas out below 60 %: the
   fields turn into plain text; zoom in again to edit.
8b. **Texture settings take wires (NE-03).** Add a Constant texture layer
   (Diffuse Color) in the group: its box has a Color row. Wire Noise into
   it: the render and viewport show the noise, the row reads "(driven)",
   and in Properties the Constant's Color is greyed with "driven by nodes
   (Noise)". Unwire: the picture is exactly as before. A procedural
   (Checker...) has Color 1 and Color 2 rows that take wires the same way;
   Change Type to Image Map drops its wire and keeps Noise.
9. **Delete a layer box (NE-06, NE-19).** Add a Constant texture layer in
   the group; it appears as a box with its own mixing node. Select its box,
   press Delete: the layer row is gone from the tree. Cmd+Z: row, box and
   wires return in one step.
10. **Hide (NE-07).** Wire Noise into Diffuse Color again, select Noise,
    press Backspace: Noise disappears and Diffuse Color shows a short wire
    ending in a ringed dot. Double-click the dot: Noise is back.
    Right-click canvas > Show All shows every hidden box. Renders don't
    change.
11. **Move (NE-16).** Drag a box: Undo has one "Move Node" step; Preview
    doesn't restart.
12. **Delete a driven layer (NE-13).** Delete Base Material in the tree:
    Noise stays in the node view, dimmed and unwired.
13. **Save and reopen (NE-09/10).** Arrange the boxes, save, reopen: the
    boxes sit where you left them, wires and hidden state kept.
14. **Export (NE-10).** File > Export Material as MaterialX on the group;
    open the .mtlx in a text editor: the noise3d node is there with xpos
    and ypos. Import it back: a graph layer whose graph shows the Noise.
15. **One history (NE-15/17).** Edit the tree, then the node view, then the
    tree; three Cmd+Z reverse them in order whichever window has focus,
    and the node window stays on the material.
16. **As rendered.** Tick "As rendered": the read-only operandi_* form
    shows (slice 1); untick to edit again.
17. **More texture inputs (NE-03, NE-Q07).** In the group add an Image Map
    (pick a picture), a Gradient, an Occlusion and a weight Vertex Map
    Texture. Each box lists its wire inputs: Image Map Gamma, Brightness,
    Contrast (plus Low/High Value on a scalar effect); Gradient Input
    Parameter (no value of its own); Color/Value 1 and 2 on Occlusion and
    the Vertex Map. Wire a Noise into each: Preview and the viewport
    change the same way; Properties greys the row with "driven by nodes";
    Cmd+Z removes the wire in one step; save and reopen keeps it. Set an
    Occlusion's effect to Layer Mask under a Constant: its zone wires
    change the mask.
