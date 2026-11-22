# Insets that collapse (M17-Q04)

**PB-C-01 No clamping.** Square, Inset 0.4, 0.5 and 0.6 all give 8 points
and 5 polygons. At 0.5 the inner polygon has zero size; at 0.6 it is
inverted: corners (0.6,0.6), (0.4,0.6), (0.4,0.4), (0.6,0.4), i.e. each
corner moved past the center. No message, no refusal. Selection after: the
inner polygon.

**PB-C-02 Auto Weld merges collapsed corners.** Square, Inset 0.6, Auto Weld
on: 5 points, 4 polygons: all inner corners weld into one point at the
center (0.5,0.5) and the inner polygon is removed, leaving four triangles
`[0,1,4]`, `[1,2,4]`, `[2,3,4]`, `[3,0,4]`. Selection after: nothing.

**PB-C-03** Rect (2×1), Inset 0.6, Auto Weld on: inner corners weld in
pairs to (0.5,0.5) and (1.5,0.5); 6 points, 4 polygons: two quads on the
long sides and two triangles on the short sides; inner polygon removed.
Without Auto Weld: 8 points, 5 polygons, inner polygon inverted across y.

**PB-C-04 Concave polygon.** L, Inset 0.2: 12 points, 7 polygons, a normal
inset. Inset 0.6: the same topology with a self-intersecting inner hexagon
(0.6,0.6), (1.4,0.6), (1.4,0.4), (0.4,0.4), (0.4,1.4), (0.6,1.4); no message.
