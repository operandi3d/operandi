# Set Part, open questions

Docs don't settle these. Each has a provisional answer the build can use.

- **SP-Q01** Nothing selected: does Set Part tag every polygon of the active
  mesh(es)? The guide states it for Set Material in the shared
  Material/Part/Smoothing section and doesn't repeat it for Part.
  Provisional: yes, same as Material.
- **SP-Q02** Does Set Part work from Vertex or Edge mode (e.g. polygons
  touching the selection)? Provisional: no, refuse with a message.
- **SP-Q03** Does the Name field open with the Default Part preference every
  time, or remember the last name used? The guide only says the preference
  name appears in the dialog. Provisional: preference value every time.
  (Today's build pre-fills the selection's shared Part. That is handy and
  could be kept as a deliberate Operandi choice; Jason's call.)
- **SP-Q04** What does applying a blank name, or the Default Part name, do:
  clear the Part, store that name literally, or refuse? Provisional: blank
  refused; any non-blank name, including `Default`, stored literally.
- **SP-Q05** Factory value of the Default Part preference. Provisional:
  `Default`.
- **SP-Q06** How are polygons with no Part shown in a Part list (blank,
  "Default", or hidden)? Provisional: hidden.
- **SP-Q07** Is the dropdown list sorted, and does it include parts from
  hidden or inactive meshes? Provisional: sorted A to Z, every mesh in the
  scene.

If Jason wants any of these settled by watching Modo, they fit in one short
paste-prompt; none needs it to start building.
