# Python scripting: GUI checklist

Turn it on first: Preferences > Defaults > Application > Experimental >
Python scripting (preview), or start the QA app with `OPERANDI_PYTHON=1`.

1. With it off: the System menu has no Script Console or Run Script…; with it on, both show.
2. System > Script Console opens a floating window (title bar, can move to another screen, resize; reopens where it was).
3. Type `1 + 1`, Enter: `>>> 1 + 1` then `2`. Up arrow brings the line back.
4. `import operandi; operandi.run("operandi.time.set frame:12")` (with animation on): the time moves to 12. Cmd+Z doesn't undo it (time isn't an undo step).
5. `operandi.set_channel(<id>, "pos.Y", 2)` moves the item; Cmd+Z undoes it in one step.
6. `1/0`: the traceback shows in red; the status line says Script failed; the app carries on.
7. `while True: pass`: Stop (or Ctrl+C in the input line) ends it with KeyboardInterrupt; the window stayed responsive meanwhile.
8. System > Run Script…: the panel starts in the Scripts folder; a script that keys a bouncing ball runs; the whole script is one Cmd+Z.
9. The action shows in the Command History with its line (UIC-239): `operandi.python.run …` and the script's own lines (source script).
10. MCP: `python_run {"code": "print(42)"}` answers `42`; with the switch off it's refused.
