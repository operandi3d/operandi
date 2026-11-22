# Layouts stage 5: per-layout key maps

Behaviour only.

Stage 5. Per-layout key maps. Each layout names a key map context; keys a
layout doesn't set fall back to global (LAY-20). Until a layout sets its own
keys, nothing changes. Rule landing: UIC-201.

- **LAY-20:** Each layout carries its own key map context, so the same key can
  do different things in different layouts. Keys not set for a layout fall
  back to the global key map.
- **LAY-71:** A layout file holds its key map context (among its panels,
  splits and so on).
- **UIC-201:** Each layout has its own key map context; keys it doesn't set
  fall back to the global map.
- **LAY-32 (keep as is):** On Mac the quick switcher is real Ctrl+Shift+Tab
  and the layout pie is real Ctrl+` (Cmd versions are taken by macOS).

## Build notes

- The existing input map / key binding editor gets a context choice: Global,
  or "This layout (<name>)". Binding a key in the layout context only affects
  that layout. Showing a key that's overridden by the current layout must be
  clear (e.g. a marker, and the tooltip says which layout).
- Lookup order: current layout's context, then the existing per-panel/tool
  contexts as today, then global. Work out where layout sits relative to
  today's panel/viewport-specific contexts so nothing that works today
  changes. Log the order chosen as provisional.
- Shipped layouts (Model, UV) ship with an empty layout context, so the app
  behaves identically. User overrides save with the user's copy of the layout
  (or alongside it) and survive save/reopen. Layout Revert should not
  silently drop key overrides; decide and log (provisional) whether Revert
  keeps them or asks, and give the key editor a "Clear this layout's keys".
- Key hints everywhere (tooltips, menus, toolbox `{binding}` placeholders,
  pie labels) must show the key that will really fire in the current layout,
  and update on a layout switch.
- Switching layouts mid-tool or mid-drag must not leave a stuck key or
  modifier.
