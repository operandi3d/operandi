# Making a kit

A kit is an optional add-on for Operandi: a folder with a `kit.toml`
manifest and the files it needs. People install it from a zip and turn it
on themselves. This page is the contract a kit is written against. It's
for kit authors, including contributors working from a fork with their own
AI assistant: everything you need is here and in `examples/kits/off_import`.

**What a kit can do today (kit API 1):** add file types to File > Import.
The kit ships a *helper program* that converts the file into a format
Operandi already reads. That's enough for importers such as STEP (with
OpenCascade in the helper). More kinds of parts come in later API
versions (see "What's next"); kits written for API 1 keep working.

The user guide's page for people installing kits is
[guide/src/scripting/kits.md](guide/src/scripting/kits.md). The longer
design notes behind this are in the kits plan (K1..K12) and the approved
rules KIT-1..KIT-4, summarised at the end.

## Start from the example

```bash
cp -R examples/kits/off_import ~/my_kit      # then rename it to your kit's id
scripts/package-kit.sh examples/kits/off_import
```

The example imports OFF files (a simple text mesh format). Its helper is
about 150 lines of Rust with no dependencies. `package-kit.sh` builds it
(through the kit's `build-kit.sh`) and writes
`development/kits/off_import-1.0.0-<os>-<arch>.zip`. Install that in
Operandi with `operandi.kit.install path:{<zip>}`, turn it on with
`operandi.kit.enable id:off_import`, and import `cube.off`.

## The kit folder

```
step_import/                 the folder is named after the kit's id
  kit.toml                   the manifest (required)
  bin/macos-aarch64/step-helper
  bin/windows-x86_64/step-helper.exe
  ...                        anything else the helper needs (libraries, data)
```

Installed kits live in the **Kits** folder inside Operandi's data folder
(`operandi.kit.openFolder` shows it; QA builds use `OPERANDI_DEV_DATA`).
Which kits are on is kept in `kits.toml` next to it. Nothing in a kit runs
until the user turns it on.

## kit.toml

```toml
[kit]
id = "step_import"            # required
name = "STEP Import"          # required
version = "1.0.0"             # required, your own version
api_version = 1               # required, the kit API you wrote for
licence = "GPL-3.0-or-later"  # required, SPDX expression
authors = ["Your Name"]
description = "Reads STEP and STP files with OpenCascade."
homepage = "https://example.org/step-kit"

[[importer]]                  # one per group of file types
label = "STEP files"          # shown in the Import dialog
extensions = ["step", "stp"]  # lower case, no dot
output = "glb"                # what the helper writes (see below)
timeout_seconds = 600         # default 600, at most a day

[importer.helper]             # the helper per platform, inside the kit
macos-aarch64 = "bin/macos-aarch64/step-helper"
macos-x86_64 = "bin/macos-x86_64/step-helper"
windows-x86_64 = "bin/windows-x86_64/step-helper.exe"
linux-x86_64 = "bin/linux-x86_64/step-helper"
```

Rules Operandi checks when it reads the manifest (each failure names the
field):

- **id**: 1 to 32 lower-case letters, digits or `_`, starting with a
  letter. It is also the kit's command namespace for later API versions
  (`step_import.remesh`), so it can't be one of Operandi's own namespaces
  (`poly`, `operandi`, `scene` ...). The kit's folder must have this name.
- **api_version**: a kit for a newer API than the running Operandi is
  refused with "This kit needs kit API N", never half-loaded.
- **Unknown fields are errors.** A typo shows up straight away instead of
  a part that silently does nothing.
- **extensions**: not one Operandi reads itself (obj, fbx, usd ... and
  lxo, opscene); a kit adds file types, it can't take one over. If two
  kits that are on claim the same extension, the first by folder name
  keeps it and the other is reported.
- **output**: one of `glb`, `gltf`, `ply`, `fbx`, `usd`, `usda`, `usdc`,
  `usdz`. These carry their own units and axes. OBJ and STL aren't
  allowed, because Operandi would rescale them by the user's File I/O
  preferences. glTF/GLB is the best choice for most importers: metres,
  Y up, materials, hierarchy.
- **helper**: platform keys are `<os>-<arch>` as Rust names them
  (`macos-aarch64`, `macos-x86_64`, `linux-x86_64`, `linux-aarch64`,
  `windows-x86_64`, `windows-aarch64`), or `any` for a helper that runs
  everywhere (a script with a `#!` line). Paths are relative, use `/`, and
  stay inside the kit folder. A zip needs only the helper for its own
  platform; a platform with no helper reports "has no helper for ...".

## Helper protocol 1

For each import Operandi runs the helper as a separate program:

```
<helper> import <input file> <output file>
```

- The working folder is the kit's folder. `OPERANDI_KIT_PROTOCOL=1` and
  `OPERANDI_KIT_DIR=<kit folder>` are set. Check the protocol and exit
  with an error if it isn't one you know.
- `<input file>` is an absolute path. `<output file>` is in a new, empty
  scratch folder, ends in the manifest's `output` extension, and is named
  after the input (Operandi names the imported items from it). You may
  write other files next to it (a glTF's `.bin` and textures); the
  folder is deleted afterwards.
- Write the scene to `<output file>` and exit 0. Operandi reads it with
  its own reader, so what comes in is exactly what that reader takes from
  the format (for glTF: meshes, hierarchy, UVs, basic materials).
- **Notes for the user**: print lines starting `note:` on stdout
  ("note: 3 faces could not be meshed"). They're shown with the import's
  other notes. Other stdout lines are ignored.
- **Errors**: print a plain message on stderr and exit non-zero. The user
  sees the last few stderr lines, so make the last one say what went
  wrong.
- A helper that crashes, exits non-zero, writes no file, or runs past
  `timeout_seconds` fails the import; Operandi and the open scene are
  untouched. Stdin is closed: never wait for input.
- Don't touch any file except the output folder, and don't reach the
  network.

Today the import runs while the user waits, with no progress bar or Cancel
(coming in a later slice). Keep the timeout realistic and print progress
notes only as `note:` lines at the end.

## Packaging

```bash
scripts/package-kit.sh <kit folder> [output folder]
```

It runs the kit's `build-kit.sh` if there is one, checks `kit.toml`, and
zips the folder (leaving out dot files and `target/` folders) as
`<id>-<version>-<os>-<arch>.zip`, one per platform. The kit can sit at
the top of the zip or inside one folder; both install. Users install with
`operandi.kit.install path:{<zip>}`; `replace:true` installs over an older
version. Operandi marks the declared helpers runnable when it installs.

Shipping kits:

- **macOS** blocks programs from the internet that aren't signed and
  notarized. Sign and notarize your helper (and any libraries next to it)
  before zipping, or users get a "can't be opened" message the first time
  they import. Signing is the kit author's job.
- **Windows**: an unsigned helper may get a SmartScreen warning.
- **Where to publish**: for now, a release on your own GitHub repository
  or website. There is no in-app kit list yet.

## Licences

Operandi is GPL-3.0-or-later. A helper program is a separate program that
Operandi only runs and exchanges files with, so it may use any licence
(decision D2 in the extensibility plan). Say which in `licence`, and ship
the licence texts of what's inside (OpenCascade is LGPL-2.1 with an
exception; check the exact text of the version you use). Later kit parts
that run *inside* Operandi (Python, WebAssembly) must be GPL-compatible.

Clean room: kits for Operandi follow the same Modo rule as the rest of the
project ([CLEAN_ROOM.md](CLEAN_ROOM.md)): nothing from reverse engineering
Modo, nothing copied from it.

## Testing your kit

- Unit-test your helper on its own (the example has `cargo test` tests in
  `helper/`).
- In a QA build, install the zip, turn it on, import a few real files,
  including a broken one and a big one.
- In Operandi's own tests, `crates/operandi-core/tests/kits.rs` shows how
  to drive a kit without the app (`kit_folder::install`,
  `kit_import::load`, `kit_import::run`). Its ignored test
  `the_example_kit_imports_its_cube` packs and installs the example kit
  end to end:
  `cargo test --locked --workspace -- kits::the_example_kit --ignored`.

## Commands

| Command | Does |
|---|---|
| `operandi.kit.list` | Each kit: id, version, on or off, file types, or why it can't load |
| `operandi.kit.install path:<zip or folder> [replace:true]` | Installs, left off |
| `operandi.kit.enable id:<id>` | Turns a kit on (the user's "I trust this kit" step) |
| `operandi.kit.disable id:<id>` | Turns it off |
| `operandi.kit.reload` | Reads the Kits folder again |
| `operandi.kit.openFolder` | Shows the Kits folder |

## Where it lives in the code

| File | What |
|---|---|
| `crates/operandi-core/src/kits/kit_manifest.rs` | `kit.toml`, `KIT_API_VERSION`, the checks |
| `crates/operandi-core/src/kits/kit_folder.rs` | Finding kits, `kits.toml`, installing |
| `crates/operandi-core/src/kits/kit_import.rs` | The importer table, helper protocol 1 |
| `crates/operandi-core/src/interchange/mod.rs` | `import` hands kit file types to the table |
| `crates/operandi-app/src/kits/kit_commands.rs` | `operandi.kit.*`, loading kits at start-up |
| `examples/kits/off_import/` | The example kit |
| `scripts/package-kit.sh` | Packaging |

## Rules that don't change (KIT-1..KIT-4, approved 2026-11-17)

- **KIT-1**: each new kind of thing (file formats, items, MeshOps, panels,
  render outputs) comes from one table that built-ins and kits share, and
  an unknown kind loads, shows as missing and saves back unchanged.
- **KIT-2**: renderers, the viewport and bridges read one evaluated scene.
- **KIT-3**: outside renderers connect only through a published protocol,
  in their own process.
- **KIT-4**: kits don't draw their own UI pixels; they describe forms.

Scene files never carry runnable code (D5).

## What's next

Later slices, each a small merge; API 1 kits keep working through all of
them:

- a progress bar and **Cancel** for imports, and the import off the main
  thread;
- a Kits window (list, turn on with a trust prompt, install, remove)
  following the UI bible;
- exporters through the same helper route;
- kit commands from the manifest, reaching the MCP bridge and scripts;
- Python kit parts, then kit forms and panels, kit MeshOps and item kinds.

If your kit needs one of these sooner, say so in an issue: the order
follows what kit authors need.
