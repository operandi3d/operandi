# Python scripting (preview)

Master plan F15 / A23, kits plan K3. Operandi's own design.

## Turning it on

Python scripting is off by default and hidden. Turn it on with either:

- Preferences > Defaults > Application > Experimental > **Python scripting
  (preview)**, or
- `OPERANDI_PYTHON=1` in the environment (it wins over the preference;
  `OPERANDI_PYTHON=0` forces it off).

Then System > **Script Console** opens the console, and System > **Run
Script…** runs a `.py` file (the panel starts in the Scripts folder, the
`Scripts` folder in Operandi's data folder).

The build must include Python (feature `python`; see "Building" below). A
build without it says "Python isn't in this build".

## The `operandi` module

```python
import operandi

operandi.run("operandi.time.set frame:12")          # any command line
operandi.run("transform.channel", name="pos.Y", value=2, item=1)
operandi.run("no.such.thing", check=False)          # False instead of raising
q = operandi.query("operandi.anim.query", frames="0,12")  # JSON -> Python

with operandi.undo_group("Bounce"):                  # one undo step
    ...

m = operandi.mesh_get()                              # {"item", "points", "polygons"}
operandi.mesh_set([(0, 0, 0), (1, 0, 0), (0, 1, 0)], [[0, 1, 2]])
flat = operandi.mesh_get(flat=True)["points"]        # memoryview of doubles
operandi.mesh_set(array.array("d", flat))            # points only, same count

operandi.set_time(24); operandi.set_channel(1, "pos.Y", 0); operandi.set_key(1, "pos.Y")
```

- `run` goes through the same path as a typed line, menus and the MCP
  bridge (UIC-160..165) and reports the command's own result: it returns
  `True` (or the answer text of a line that answers) and raises
  `operandi.CommandError` with the command's message when the command
  refuses or fails. Nothing is inferred from the undo list.
- `query` answers a query (UIC-163) and changes nothing; a line that acts
  is refused.
- A whole script is **one undo step** (UIC-227), named "Script <file>" or
  "Console: <line>"; a script that made just one step (one command or one
  group) keeps that step's name.
- `mesh_set` writes points alone (same count; polygons, UVs and selection
  stay) or points and polygons (the mesh is rebuilt; UV maps, vertex maps
  and component selection go). The same edit is the command
  `operandi.mesh.setData` (MCP `command_run`).

## How it runs

Python runs on one thread of its own, started the first time a script
runs. Each call into Operandi is answered by the app between frames, so
the window stays live while a script runs. **Stop** in the console (or
Ctrl+C in its input line, or `operandi.python.stop`) raises
`KeyboardInterrupt` in the script; a script inside a long C call (a sleep)
stops when that call returns. A Python error ends the script with its
traceback in the console (red) and the status line; it never takes the
app down. One script runs at a time.

The interpreter starts isolated: it ignores `PYTHONPATH`, `PYTHONHOME` and
the user's site-packages, and installs no signal handlers.

## Commands

| Command | Class | What it does |
|---|---|---|
| `operandi.python.console [open:toggle/true/false]` | UI | Shows or hides the Script Console |
| `operandi.python.run code:{...}` / `file:<path>` | side effect | Runs Python (one undo step for what it does) |
| `operandi.python.runScript` | UI | Run Script…: chooses a `.py` file and runs it |
| `operandi.python.stop` | UI | Stops the running script |
| `operandi.mesh.setData item? points polygons?` | undoable | Bulk mesh write (K9) |

MCP: `python_run {code}` runs a script on the bridge's editor (headless
commands) and returns what it printed; a Python exception is an error with
its traceback and leaves the scene as it was. In the running app it needs
the switch on.

## Building

- Development: any CPython 3.10+ works through PyO3:
  `PYO3_PYTHON=$(command -v python3) cargo test --workspace --features operandi-app/python`.
- The app bundle carries its own CPython (python-build-standalone,
  relocatable). `scripts/fetch-python.sh` downloads the pinned release
  into `development/python/python` and checks its SHA-256;
  `scripts/bundle-dev.sh` then builds Python in and copies libpython into
  `Contents/Frameworks` and the standard library (without tests, IDLE, Tk
  and pip) into `Contents/Resources/python`. `OPERANDI_PYTHON_BUILD=1`
  fetches first; `OPERANDI_PYTHON_BUILD=0` leaves Python out.
- `scripts/merge-check.sh` runs the Python tests with the bundled CPython
  or, failing that, the `python3` on PATH.

Not done yet:

- **Releases** (`scripts/release-macos.sh`): the extension modules
  (`lib-dynload/*.so`) and libpython need Developer ID signing and
  hardened-runtime entitlements for notarization; the QA bundle signs them
  ad hoc.
- **Windows**: python-build-standalone ships `x86_64-pc-windows-msvc`
  builds; Operandi's Windows builds are mingw (`x86_64-pc-windows-gnu`),
  so PyO3 needs a `python3.dll` import library made for mingw (or the
  abi3 route), and `python313.dll` plus the `Lib` folder go next to
  `Operandi.exe`.
- **Linux**: the `x86_64-unknown-linux-gnu` install_only build works the
  same way; `libpython3.13.so` goes in the AppImage's `lib` (rpath
  `$ORIGIN/../lib` is already set) and the standard library under
  `usr/share/operandi/python`, with `home()` taught that place.
