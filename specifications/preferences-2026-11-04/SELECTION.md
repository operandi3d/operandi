# Preferences: Selection (behaviour spec v1, 2026-11-04)

Behaviour-only. "Not yet measured" = factory value still to be read (v2);
use a provisional value and log it. Settings apply at once and persist per
user.

| Setting | Panel | Type | Default | Behaviour |
|---|---|---|---|---|
| Initial Selection Type | Defaults > Application | dropdown (selection modes: points, edges, polygons, items, …) | not yet measured | Selection mode active when the app starts. |
| Selection Rollovers | Display > OpenGL | dropdown: None, Manual, Closest | not yet measured | Pre-highlight under the pointer. None: off. Manual: highlights elements of the current mode. Closest: highlights the nearest element of any kind (point, edge or polygon). |
| Mouse Regions Trigger Selections | Display > OpenGL | checkbox | not yet measured | When on, right-clicking an element switches to that element's mode (point/edge/polygon). Meant to pair with Closest rollovers. |
| OpenGL Point Size | Display > OpenGL | pixels | 3 | Drawn size of vertices; also sets the area where a click hits a vertex. |
| Selected Point Size | Display > OpenGL | number, multiple of point size | not yet measured | Selected vertices are drawn this much larger. |
| Selection transparency | Display > OpenGL | percent | not yet measured | How visible a selection is where a surface hides it (x-ray), so front and back selections can be told apart. |
| Polygon Selection | Input > Selection | dropdown: Frontface & Wireframe, Frontface & Backface, Frontface only, Always Raycast, No Raycast | not yet measured (Frontface & Wireframe likely) | Which polygons paint and lasso selection can pick. Frontface & Wireframe: front faces only, except in wireframe views where back faces count too. Frontface & Backface: always both. Frontface only: never back faces. Always Raycast: hidden elements are never picked, and wireframe is treated like solid. No Raycast: hiding is never tested, and solid is treated like wireframe. |
| Double-Sided Selection | Input > Selection | checkbox | not yet measured | Polygons whose material is double-sided can be paint-selected from behind. (Later for Operandi.) |
| Selection Hit Size (pixels) | Input > Selection | whole number | not yet measured | Radius around a pre-highlighted element in which a click selects it. |
| Point Selection Priority | Input > Selection | number (type not yet confirmed) | not yet measured | In tools that pick points and edges together (Tweak), near a polygon corner, the point is preferred over the edge by this amount. |
| Lasso: Ignore Shading Style | Input > Selection | checkbox | not yet measured | Normally the lasso picks only visible elements in solid views and everything in wireframe; when on, it ignores the view's shading style. |
| Lasso: Allow Partial Element Selection | Input > Selection | checkbox | not yet measured | Normally a polygon must be fully inside the lasso to be picked; when on, anything the lasso line touches is picked. |

Open: SEL-P-Q1 factory values; SEL-P-Q2 Point Selection Priority type
and range; SEL-P-Q3 Initial Selection Type choices.
