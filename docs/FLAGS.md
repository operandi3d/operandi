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
