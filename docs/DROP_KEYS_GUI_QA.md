# Q and Esc GUI checklist (drop_keys.rs)

1. Cube: drag a cube, type 2 in Size X (no Enter), press Q with the pointer on the panel: the size is applied and the tool drops. Repeat with the pointer moved onto the view first.
2. Bevel some polygons, type 1/0 in a field, press Q: the field shows its error, the tool stays.
3. Bevel, Loop Slice, Move: Esc over the view drops each, keeping the edit; the hint reads "Q / Escape: drop tool".
4. No tool on, polygons selected: Esc over the view clears them; Cmd+Z brings them back. Same in Vertices, Edges and Items.
5. Esc with the pointer over a panel keeps the selection.
6. With a tool on, type in a field, press Esc: only the typing is undone; the next Esc drops the tool, the one after that the selection.
