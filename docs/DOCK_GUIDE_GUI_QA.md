# Dock guide and UI build mode: GUI checklist

Branch `claude/project-thread-1ngxi1`. Use the QA app; Model layout, two
views (Split Horizontal from a view's dot menu).

## Dock guide (UIC-72, redesigned 2026-11-20)

1. Drag Tool Properties by its corner grip. Four arrow buttons appear, one
   at the middle of each outer edge of the work area. No orange strips
   along the edges, dividers or tab rows.
2. Move over a viewport. A five-button cross pops up centred on that
   viewport only (left, right, top, bottom, and a centre "tab" button).
   Move to the other viewport: the cross moves with you.
3. Hover each cross button: it lights up and a translucent area shows the
   half (or, for the centre, the whole panel) the drop will take. Release
   on the bottom button: Tool Properties docks below that viewport only.
4. Drag the Items group over the left outer arrow: the preview covers the
   left half of the work area. Release: it docks against the whole left
   edge.
5. Drag a panel over a tab row: a thin insert line shows between tabs;
   release joins the row at that place.
6. Drag a panel and release somewhere that isn't a button or tab row: a
   translucent window-sized outline follows the pointer and the panel
   opens as a floating window there.
7. With only one viewport left in the main window, drag its dot off any
   target and release: nothing happens (no preview either).
8. Press Esc during any drag: nothing changes.

## UI build mode (the only time panels move)

1. Outside the mode, drag a viewport's dot, a panel's corner or a tab:
   nothing moves and the status line says to use UI build mode. Clicking
   tabs still switches them. No corner grip appears on hover.
2. Layout > UI Build Mode (Ctrl+Shift+L): every movable panel and the mode
   bar get an accent outline and a grip pill; the menu bar and layout tabs
   don't. A bar at the bottom says "UI build mode" with a Done button.
3. Press anywhere on a viewport or panel and drag: it moves (dock guide as
   above). Clicking a button or in a viewport does nothing; no selection,
   no navigation.
4. Tabs still switch and can be pulled out of their row.
5. Done, Ctrl+Shift+L or the menu entry leaves the mode.
6. Drag the mode bar from anywhere on it: only the top and bottom arrows
   show, the nearer one lit, with a strip where the bar lands. It never
   floats or goes to a side.
7. Layout menu has no Lock Thumb Dragging entry any more.
