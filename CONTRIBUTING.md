# Contributing to Operandi

Thanks for helping. Operandi is a community project led by Jason
(GitHub: manfriday), who has the final say on what gets merged.

Please read this page, the [Modo ground rules](docs/CLEAN_ROOM.md) and the
[code of conduct](CODE_OF_CONDUCT.md) before your first pull request.
[docs/SETUP.md](docs/SETUP.md) gets you from a fresh clone to a running
app on macOS, Windows or Linux.

## How to send a change

Only the maintainer can push to this repository, so `git push` to
`operandi3d/operandi` fails with a 403 error. That's expected. Send your
change from your own copy (a fork) instead:

1. Click **Fork** at the top of the
   [repository page](https://github.com/operandi3d/operandi). GitHub makes
   your own copy at `github.com/<you>/operandi`.
2. Clone your fork and start a branch from `main`:

   ```bash
   git clone https://github.com/<you>/operandi.git
   cd operandi
   git switch -c my-change
   ```

   Already cloned `operandi3d/operandi`? Point it at your fork instead:
   `git remote rename origin upstream`, then
   `git remote add origin https://github.com/<you>/operandi.git`.
3. Commit with a sign-off, `git commit -s` (see "Signing off your work"
   below). A check on the pull request fails without it.
4. Push the branch to your fork: `git push -u origin my-change`.
5. GitHub then shows a **Compare & pull request** button on your fork or on
   this repository. Open the pull request against `operandi3d/operandi`
   `main` and fill in its checklist.

The first time you contribute, the checks wait until the maintainer starts
them, so don't worry if they sit for a while. To change the pull request
later, push more commits to the same branch; it updates by itself. To catch
up with `main`, run `git pull upstream main` (add the `upstream` remote
with `git remote add upstream https://github.com/operandi3d/operandi.git`
if you cloned your fork).

## Modo: what we can't accept (read this first)

Operandi is an homage to Modo (see [docs/ABOUT.md](docs/ABOUT.md)). Knowing
Modo well, using it, reading its public documentation and recreating how
its tools behave are all fine.
What we can't take:

- **Anything from reverse engineering Modo.** No decompiling, disassembly,
  or digging through its program files (REA, Ghidra, IDA, Hopper and the
  like), and no code or specs that came from someone doing that. Studying
  a file format so Operandi can open or save that format is fine.
- **Modo's code.** Leaked or decompiled source, or code copied from the
  Modo SDK or from projects built on it.
- **Modo's own material.** Its icons, images, interface artwork, presets,
  sample content, or text copied from its documentation. Make your own,
  and describe things in your own words.
- **Confidential Foundry material.** If you've worked at Foundry, you're
  very welcome here, and your skills and 3D know-how are yours. Just don't
  bring code, internal documents or anything your agreement with Foundry
  treats as confidential.

If you're unsure whether something is OK, ask in an issue first. If
something slipped in, say so and we'll sort it out together. Being honest
is never the problem; hiding it is. [docs/CLEAN_ROOM.md](docs/CLEAN_ROOM.md)
has a little more detail.

## AI agents

You may use AI coding agents. They follow the same rules as you, and what
they do counts as your work. [AGENTS.md](AGENTS.md) explains how code is
written in this repository (Claude reads it through `CLAUDE.md`); point
yours at it. Don't let an agent reverse engineer Modo or bring in any of
the material listed above.

## Licences

Operandi as a whole is released under the GNU GPL, version 3 or later
([LICENSE](LICENSE)). Everything in it must be compatible with that: leave
out any code, library, asset or font whose licence isn't cleanly
GPL-3.0-compatible, and ask in an issue if you aren't sure.

### How your contribution is licensed

By submitting a contribution, you license it under the GNU GPL, version 3
or later: the same licence Operandi is released under. What comes in is
what goes out. Files that carry a licence of their own, such as the
Blender-derived files described below, keep it. In practice:

- You keep the copyright in your work.
- Your name stays in the project's history and in [AUTHORS.md](AUTHORS.md).
  Copyright notices you add are kept.
- Nobody, the maintainer included, can take your work closed-source.
  Everyone gets it on the same terms.

If you can't agree to this, please open an issue describing your idea
instead of sending code.

### Code adapted from Blender (and other GPL sources)

Blender's code is GPL-2.0-or-later. That fits inside Operandi's
GPL-3.0-or-later, but its files keep their own notices, so code adapted
from it stays apart:

- It goes in its own module (a crate, folder or file of its own), never
  mixed into Operandi's own files. Today that is
  `crates/operandi-uv/`, `crates/operandi-core/src/sculpt_blender/`,
  `crates/operandi-core/src/materials/blender_principled.rs`,
  `crates/operandi-usd/bridge/blender/` and `crates/operandi-remesh/`
  (Blender's QuadriFlow glue; its bundled QuadriFlow, LEMON, pcg32 and
  Eigen files keep their own licences, listed in that crate's README).
- Each file starts with Blender's copyright notice, an Operandi
  contributors line and `SPDX-License-Identifier: GPL-2.0-or-later`, then an
  "Adapted from Blender `<source path>` at commit `<sha>`" line.
- It's recorded in [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md) with the
  licence text in [LICENSES/](LICENSES) or `docs/licenses/`.
- Changes to those files come in under the file's own licence
  (GPL-2.0-or-later) and keep its notices.

The same applies to other open-source code you adapt: keep it in its own
module under its own licence and notice, and record it in DEPENDENCIES.md.
Never adapt code that was built from reverse engineering Modo or copied
from it.

## Signing off your work

Every commit must end with a sign-off line:

```
Signed-off-by: Your Name <you@example.com>
```

`git commit -s` adds it for you. A check on every pull request makes sure
each commit has one.

By signing off, you certify the
[Developer Certificate of Origin 1.1](https://developercertificate.org).
You also confirm two things:

1. Your contribution is licensed as described under "Licences" above.
2. Your contribution follows the Modo rules at the top of this page.

If your employer has rights to what you write, check with them before you
contribute.

## Making a change

1. Open or pick an issue first for anything bigger than a small fix, so we
   agree on scope.
2. Work on a branch named for the feature (for example `edge-slide`),
   started from `main`.
3. For a tool: read its spec, write 5–8 acceptance criteria (including empty
   or invalid selection, cancel, undo/redo and save/reopen), build the whole
   workflow (operation, live preview, panel, shortcut, command) and a test
   for each criterion. [AGENTS.md](AGENTS.md) has the details, including the
   fast-drag rules every interactive tool follows.
   To find your way around first, the
   [code map](https://operandi3d.org/architecture) shows every module and
   file with a one-line description and what connects to what, and
   [docs/DATA_MODEL.md](docs/DATA_MODEL.md) shows what a scene is made of
   (items, meshes, channels, materials, undo and the `.opscene` file).
4. Interface work follows the project's UI conventions,
   [docs/UI_BIBLE.md](docs/UI_BIBLE.md). Read the section for what you're
   changing before you start, and mention the rule numbers (UIC-nn) when
   your change makes or breaks one. Departures need the maintainer's OK.
   Anything a user can see or do needs its User Guide page in the same
   pull request (UIC-300; [docs/guide/RULES.md](docs/guide/RULES.md)).
5. Run the checks with the pinned toolchain:

   ```bash
   cargo fmt --check
   cargo clippy --locked --workspace --all-targets -- -D warnings
   cargo test --locked --workspace
   ```

6. New icons come only from the icon studio in
   [`tools/icon-studio`](tools/icon-studio): small 3D scenes rendered with
   Cycles, so the set keeps one look (same camera, lights and materials).
   Final icons are never hand-drawn, hand-edited or imported. The old flat generator in
   `tools/operandi-icons` is deprecated; don't add icons to it. The "Icons"
   section of [AGENTS.md](AGENTS.md) has the steps; Jason approves the
   contact sheet before an icon goes into the app.
   **Any icon you make yourself is a placeholder.** That's fine to include
   so your feature has a button, but say so in the pull request ("placeholder
   icon: <name>"). It will be replaced with a studio icon so the whole set
   keeps one consistent look.
7. Add new dependencies with their licences to
   [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md).
   Put each new tool or command in its own new file. Each shared list that
   has to know about it (module lists, `tests/all.rs`, the command name
   map, the MCP tool lists, the command table, the icon table) gets at
   most one added line, in its sorted place rather than at the end. Two
   people adding tools at once then rarely edit neighbouring lines, so
   their changes don't clash. Tests check the sorted lists and name any
   entry that's out of place; [AGENTS.md](AGENTS.md) ("Code") lists which
   lists are sorted and which keep a user-visible order.
8. If an AI coding tool helped, end each commit message with a
   `Co-Authored-By` line for it (for example
   `Co-Authored-By: Claude <noreply@anthropic.com>`) and name the tool in
   the pull request.
9. Open a pull request and fill in its checklist. Jason reviews and merges;
   `main` only changes through reviewed pull requests.

**Making a kit** (an optional add-on, such as an importer for a new file
type) doesn't need a change to Operandi at all: a kit lives in its own
repository and people install it from a zip.
[docs/KITS.md](docs/KITS.md) is the kit contract, and
[`examples/kits/off_import`](examples/kits/off_import) is a small kit to
start from.

Don't commit build output (`target/`, `development/`), `.docx` files, your
own scenes (`*.opscene` outside `fixtures/`), anything from Modo, or
secrets.

## Reporting bugs

Use **Help > Report Bug or Idea…** in the app. It opens a filled-in bug form in
this repository's [issues](https://github.com/operandi3d/operandi/issues)
for you to check and submit. Crash reports work the same way.

Security problems: see [SECURITY.md](SECURITY.md), not a public issue.
