# Preferences: Viewport readouts and UI basics (behaviour spec v1, 2026-11-04)

Behaviour-only. "Not yet measured" = factory value still to be read (v2);
use a provisional value and log it. Settings apply at once and persist per
user.

## 3D Information Overlays (Display > 3D Information Overlays)

Each checkbox turns one on-screen readout in 3D views on or off. Defaults
not yet measured.

| Readout | Shows |
|---|---|
| Current Selection | What is selected, for the current mode |
| OpenGL Polygon Count | Polygons drawn, including subdivision |
| Grid Size | Real-world size of one grid square |
| Polygon Type | Polygon type of the active meshes: face, subdivision, Catmull-Clark or mixed |
| Current Tool | Active tool, plus falloff, action centre and snapping info |
| Snapping State | Which snapping options are on |

Later: Morph Map, Reference Item, Selected Channel Count, Deformers state,
Active Passes & Actions, UV Coverage (percent of UV space covered, in the
UV view), UV Real World Size.

## UI (Defaults > UI, Tool Properties)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Tidy Layouts | checkbox | on | Removes doubled borders where panels nest. |
| Popup Transparency | percent | not yet measured | Transparency of popups, popovers and pie menus; higher is see-through but harder to read. |
| Popover Dismissal Behavior | dropdown: Default, Force Click-Off, Force Roll-Off | Default | Default: each popover's own rule. Click-Off: stays until you click elsewhere. Roll-Off: closes when the pointer leaves. |
| Base Form Proficiency Level | dropdown: Basic, Standard, Advanced | not yet measured | How many controls tool property panels show: common only, all core, or all plus special ones. |
| Default Form Icon Size | dropdown: Small, Normal, Large | Normal | Icon size in panels. |
| Tool Properties layout | dropdown: Popover, Inline, Split | not yet measured | Popover: a button under each toolbar opens properties in a movable, pinnable popover. Inline: properties sit under the toolbar. Split: a properties panel appears beside the toolbars while a tool is active. |

## Other (Defaults > Application, Scene Save/Export, Display > OpenGL)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Use OS Color Picker | checkbox | not yet measured | Use the system colour dialog instead of the built-in picker. |
| Reset "In the future…" Messages | button | — | Brings back every dialog the user chose to stop seeing. |
| Author | text | empty | Written into saved files that support it. |
| Copyright | text | empty | Same. |
| Text Encoding | dropdown: UTF-8, system default | not yet measured | Encoding of saved text. |
| Sync Display Refresh | checkbox | not yet measured | V-sync for 3D views: less tearing, possible lag on heavy scenes. |
| Max GL Polygon Count | whole number | not yet measured | If an action would make the views draw more polygons than this, the user is warned and can cancel it (guards against runaway subdivision). |

Open: OUI-Q1 factory values; OUI-Q2 what the polygon-cap warning looks
like.
