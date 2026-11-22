# Rule exceptions (FLAGS)

Places that knowingly don't follow a UI conventions bible rule, with the
rule number. Each needs Jason's OK to stay.

- **UIC-220 / UIC-226 (Command History, stage H).** Viewport gestures
  still call editor commands directly, as before stage H: selection
  clicks and drags, handle drags with no tool, Pen point clicks and Solid
  Sketch node edits. Their undo steps show their name in the Undos tab
  with "(no command line yet)", and nothing is added to History. A tool's
  drags do record (`tool.attr` at mouse-up). Moving the gestures onto
  command lines (`select.element` by index, UIC-224) is its own job;
  nothing new was added this way.
- **UIC-220 (Command History, stage H).** Inside Pen, Solid Sketch and
  Polygon Reduction sessions the session's own undo steps keep no lines;
  the step the session drops keeps all of its lines (CH-Q06).

(Layouts stage 2 removed the earlier UIC-220 exception: the reset
question's Reset now runs `operandi.layout.revert`.)

Animation phases 1-2 (all approved by Jason, 2026-11-22):

- **UIC-165 (command names).** The animation commands are Operandi's own
  names, since the reference name map has none: `operandi.time.*`,
  `operandi.key.*`, `operandi.marker.*`, `operandi.channel.select`,
  `operandi.anim.*`, `operandi.render.frameRange`, `operandi.view.playblast`.
- **UIC-190..195 (panel kinds).** The timeline is a bottom strip, opened
  by the bottom bar's clock button and by the Animate layout; it isn't a
  dockable panel kind yet (TL-Q01).
- **UIC-201 (key map contexts).** `/`, `,`, `.`, Shift+R and Shift+Y are
  animation keys only in the Animate layout's keys, because Turn, Zoom
  In/Out, Quick Align and Flex hold them everywhere else (TK-Q09). The
  decision is made in one place, `anim_ui::animation_key`.
- **CP-A04 (item arrow walk).** The Items-mode arrow walk ignores
  Shift/Ctrl+arrows, which are the animation stepping and key-jump keys.
- **UIC-190/194 (shipped layouts).** Animate is a third shipped layout,
  beside Model and UV.

Animation phase 3 (provisional, needs Jason's OK):

- **UIC-190..195 / UIC-184 (GE-01).** The Graph Editor opens as a
  floating window (Animate > Graph Editor, the timeline's Graph button,
  `operandi.graph.open`), not yet a dockable panel kind or the Animation
  Editors preset.
