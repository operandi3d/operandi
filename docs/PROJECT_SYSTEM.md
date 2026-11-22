# Project system — design note

Status: product direction from Jason, 2026-10-31. Phase 1 (Create/Set
Project) implemented 2026-11-01: `operandi-core/src/project.rs`,
`operandi-app/src/project_ui.rs`. Phase 3 (project presets) implemented
2026-11-01 as File > Project Presets (`operandi-app/src/project_presets.rs`):
layout, viewport display and input map can be saved to and loaded from the
project's `workspaces/` folder; nothing is applied automatically. Phase 2
(project-relative references, copy into the project) and the default-folder
part of phase 4 implemented 2026-11-16 on branch `project-file-paths`; see
"Phase 2 as built" below.
This is Jason's own idea, drawn from years of working in Maya, not from studying
Modo. It is Operandi's own design: we borrow the workflow idea (Create/Set
Project), not Maya's folder names, `workspace.mel` or any file format.

## The idea

An artist can create or set a **project**: a root folder that Operandi fills
with a predictable structure for scenes, source images, renders, caches,
references, assets and exports. A small manifest in the root says where each
kind of file belongs. Every part of Operandi that reads or writes files asks the
active project where things go, instead of each tool managing its own paths.

Opening a single scene without a project keeps working. Projects are an
organizing convenience, never a requirement.

## Why it matters

- **Organization from day one.** Files land in consistent places from the moment
  a project starts, which saves time and makes projects easy to hand over,
  move between machines, or revisit years later.
- **It compounds.** The benefit grows with the number of assets and people.
  Jason suspects this is an underrated reason studios stay with Maya even when
  other tools do individual tasks better.
- **One path system for everything.** Materials, importers and exporters,
  renderers, asset references, caches, simulations and eventually plugins can
  all resolve paths the same way.
- **Clear ownership.** Project paths, asset references, UI layouts and personal
  preferences each get a defined home.

## What a project contains

A hypothetical project:

```
MyProject/
  operandi.project.toml   project settings and path mappings
  scenes/                 .opscene files
  sourceimages/           textures and reference images
  renders/                render output
  cache/                  generated data that can be rebuilt
  references/             scenes or files referenced by other scenes
  assets/                 reusable models and parts
  exports/                files written for other applications
  workspaces/             shared layout presets (see below)
  materials/              material presets and MaterialX files
  scripts/                scripts the project shares
```

The folder names are defaults. The manifest can rename or relocate any of them,
for example pointing `sourceimages` at a studio's shared texture library.

A sketch of the manifest (illustrative, not final):

```toml
[project]
name = "MyProject"
format = 1                  # manifest version, for future migrations

[paths]                     # kind = folder, relative to the project root
scenes = "scenes"
sourceimages = "sourceimages"
renders = "renders"
cache = "cache"
references = "references"
assets = "assets"
exports = "exports"
workspaces = "workspaces"
materials = "materials"
scripts = "scripts"

[workspace]
default = "Modeling"        # layout preset to open with this project
```

## Layouts and workspaces

The project system also carries **task layouts**: arrangements of panels and
viewports for modeling, UV editing, texturing and, later, other disciplines.

- **Project presets** live in the project (`workspaces/`) and travel with it,
  so a team shares useful layouts and they appear on any machine.
- **Personal layouts** live with each artist's preferences, so one artist's
  arrangement never overwrites another's.
- The artist's own layout stays in charge: opening a project doesn't switch
  layouts; the project's presets are offered in a menu to pick from.

This connects directly to the workspace-layout work: layout
persistence should be designed with project presets in mind from the start.

## Where things are stored

Three levels, each with one owner:

| Level | Lives in | Examples |
|---|---|---|
| **App / user** | the user's application data folder | input map, personal layouts, recent projects, viewport display preferences, crash recovery |
| **Project** | the project root | manifest, folder mappings, shared layout presets, project-wide presets (e.g. Bevel recipes a team agrees on) |
| **Scene** | the `.opscene` file | geometry, materials, selection, and references to other files, stored relative to the project |

Today every settings file goes through `AppPaths` in
`crates/operandi-app/src/app_environment.rs` (recovery, input map, viewport
display, Bevel recipes, profile browser, layout, runtime viewport). That is the
natural place to add the project level beside the existing app level.

## How it fits the code

- **A portable `Project` type in `operandi-core`**: read and validate the
  manifest, create the default folders, and resolve "kind + relative path" to a
  real path. No GUI dependency, so importers, exporters and a future renderer can
  use it, and it can be tested without a window.
- **The app holds the active project** and uses it for default Open/Save
  locations (`scenes/`), exports and image pickers, falling back to today's
  behavior when no project is set.
- **Scene references are stored project-relative** (for example
  `sourceimages:wood/oak.png`), with a clear rule and a visible warning for
  files outside the project. Saving a scene never rewrites paths silently.
- **Versioned manifest** with explicit migrations, like `.opscene`.

## Suggested phases

1. **Create / Set Project:** manifest, default folders, active project in the
   app, Open/Save defaulting to `scenes/`, recent projects. Scenes without a
   project work exactly as today.
2. **Project-relative references:** when the first feature that references
   external files arrives (reference images, textures or imports).
3. **Project layout presets:** together with the M1.6 workspace-layout work.
4. **Shared path service for importers, exporters, renderers and plugins**, as
   those features are built.

## Decisions (Jason, 2026-11-01)

- **Folders:** the default list above (scenes, sourceimages, renders, cache,
  references, assets, exports, workspaces); renamable per project in the
  manifest.
- **Manifest:** `operandi.project.toml` (TOML).
- **Launch:** the last project you set becomes active again if its folder
  still exists; scenes don't auto-open.
- **Menu:** File > New Project…, Set Project…, Recent Projects ▸; the active
  project's name shows in the window title.
- **Crash recovery:** stays per user in the app data folder.
- **Project presets:** layout presets, viewport display and input map can be
  stored in the project, but your own settings win; the project's are offered
  as presets to load. Bevel recipes stay personal.
- **Set Project with unsaved changes:** the scene stays open; its next Save
  defaults to the new project's `scenes/`.
- **Scene outside the project:** just opens; the active project doesn't change.
- **New Project on a non-empty folder:** allowed; adds the manifest and any
  missing standard folders, never moves or deletes files (this also converts a
  loose folder of scenes into a project).
- **Moved or copied projects:** just work, since paths inside are relative to
  the project root; Recent Projects drops entries whose folder is gone.
- **Nested projects:** not allowed; New Project refuses inside an existing
  project folder.

## Phase 2 as built (2026-11-16)

**Saved paths.** In memory every external file stays an absolute path, so
no tool knows about projects. Only `persistence::save` / `load` rewrite
them (`operandi-core/src/project_paths.rs`):

- On save to P, files inside the project enclosing P are written as
  `project:<relative/path>` with forward slashes. Files outside it, and
  scenes outside any project (crash-recovery journal, a loose scene), keep
  absolute paths. A scene with any `project:` path is native version 48;
  others keep their older number.
- On load from P, `project:` paths resolve against the project enclosing P
  (even when the file is missing, so the message names where it belongs).
  With no project, P's folder and its ancestors are tried for a file that
  exists; otherwise the text is kept.
- Fields: `Scene.images[].path` (backdrops, image maps, environment and
  camera backgrounds), node-graph images (not packed ones), a light's IES
  file, a Render Output's file name. `ImportSource.path` is provenance and
  stays absolute. Presets keep their own folder-relative rule.

This replaces the earlier sketch's `sourceimages:wood/oak.png` (per-kind
prefixes): one `project:` prefix keeps working if the manifest renames a
folder, and the path still says which folder.

**Copy into the project.** With a project set and Preferences > File I/O >
Copy Added Files into the Project on (default; `pref.value
fileIo.copy_into_project`), a file chosen or dropped from outside the
project is copied in first (`operandi-app/src/project_files.rs`, `adopt_file`):
backdrop and camera background pictures to `references/`; image maps,
environment and light-layer pictures, IES files, imported Node Material
and MaterialX images and texture sets to `sourceimages/`. Same name and
bytes already there: reused. Same name, other bytes: `name (2).ext`. The
original is never moved, renamed or deleted; a failed copy uses the
original and says so on the status line. Undo takes back the scene change,
not the copy.

**Default folders.** With a project set: Render window and Preview Save
Image start in `renders/`, File > Export in `exports/`, Export/Import
Material as MaterialX and saving a material preset in `materials/`. Recent
Frames stay in app data. Set Project makes any standard folder an older
project lacks.

**Decided (Jason, 2026-11-16).**
- Presets applied from the Preset Browser copy their files too, by the
  same rules: library materials' texture maps, texture sets, environment
  presets and HDRIs, Modo material and mesh presets, item presets, and
  File > Load Preset… or a preset opened/dropped on the app. Pictures a
  Backdrop or camera Background Image shows go to `references/`, the rest
  to `sourceimages/`. Command-line and MCP loads (`preset.do`,
  `backdrop.image`, ...) are not copied.
- Saving a material preset starts in the project's `materials/` folder
  (instead of the Presets folder in app data, PB-Q2) while a project is
  set.

**Notes.**
- A picture dropped from the Finder (or a Preset Browser picture dropped
  on a view) is copied like a chosen one: HDR/EXR to `sourceimages/`
  (even when it lands on a mesh and becomes a Backdrop), other pictures
  to `references/`.
- Copies are matched by name and bytes only (a second copy of `oak.png`
  named `wood.png` is copied again).
