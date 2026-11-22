# Falloffs batch 1: GUI check

Build: `Operandi falloff-panel <commit>.app` in `~/Developer/Operandi-Builds`.

1. Select some polygons, press R (Scale), then Falloff ▸ Selection. Tool
   Properties shows Scale's rows, then a "Selection Falloff" section: Steps,
   Shape, In, Out, Mix Mode. In and Out are greyed; hover says "Only used
   when Shape is Custom".
2. Set Shape to Custom: In and Out light up and edit the ramp live.
3. Change Steps in the panel; the scale re-applies. Right-click in the view:
   Steps goes up by one. Right-drag right/left: Steps goes up/down.
4. Falloff ▸ Add ▸ Linear: a second "Linear Falloff" section appears with
   Start, End, Symmetric, Shape, In, Out, Mix Mode, Auto Size X/Y/Z,
   Reverse and Remove. Remove takes it off.
5. Change the first section's Mix Mode: nothing moves (no effect on the
   first layer).
6. Drop the tool (no edit in progress), press Esc over the view: the falloff
   is cleared and its sections go.
7. With no tool open, Falloff ▸ Radial: its section still shows in Tool
   Properties.
8. View ▸ Show Falloffs off: the coloured weight dots go; on brings them back.
9. Falloff menu: no option rows; (none), the five types, Add ▸, Clear Falloffs,
   Weight Map.
