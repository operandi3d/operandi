# Item-mode wire GUI check (VS-6, branch item-mode-wire)

Use a fresh data folder (new views get the new defaults; views saved
before keep their stored Show Wireframe in Item Mode and overlay).

1. **VS-T13** New scene, add a Cube, Item mode (nothing else changed): the
   cube shows a thin black wire. Viewport Properties > Drawing: Show
   Wireframe in Item Mode is ticked; Active Meshes overlay is Colored.
2. **VS-T14** Untick Show Wireframe in Item Mode, click empty space so
   nothing is selected: no wire on the cube.
3. **VS-T15** Same view, switch to Polygon mode: the black wire is back.
   Vertex and Edge modes too.
4. **VS-T16** Back to Item mode, setting still off, click the cube: it
   fills with the selection colour and its edges show only the orange
   selection highlight, no black wire.
5. Set Active Meshes > Wireframe Color to red, setting on: Item mode wire
   is red; setting off: gone.
