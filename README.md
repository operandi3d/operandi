# Operandi

Operandi is a free, open-source 3D polygon modeler written in Rust (egui and
wgpu), with rendering by [Cycles](https://www.cycles-renderer.org). If you've used
Modo, it should feel familiar, but Operandi is its own program, written
from scratch with no Modo code and no reverse engineering. [docs/ABOUT.md](docs/ABOUT.md) tells the story.

macOS on Apple silicon comes first; Windows and Linux builds exist but
aren't validated yet. Operandi is pre-release: [docs/STATUS.md](docs/STATUS.md)
says what's built and how close each area is, and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) maps the code.

Renders aim for a natural, physically based look: Cycles does the physics,
and the workflow stays artist-friendly. [docs/RENDERING.md](docs/RENDERING.md)
explains the goal.

## Build it

```bash
git clone https://github.com/operandi3d/Operandi.git
cd Operandi
cargo run --locked -p operandi-app
```

You need rustup, CMake and a C++ compiler (on macOS, the Xcode Command Line
Tools). The full guide for macOS, Windows and Linux, including the optional
Cycles renderer and the checks, is [docs/SETUP.md](docs/SETUP.md).

## Contribute

Read [CONTRIBUTING.md](CONTRIBUTING.md) first, including the short
[ground rules](docs/CLEAN_ROOM.md): please don't contribute anything copied
from another commercial app, or based on reverse engineering one. Be kind: see the [code of conduct](CODE_OF_CONDUCT.md). AI
agents follow [AGENTS.md](AGENTS.md).

Contributor docs:

- [docs/SETUP.md](docs/SETUP.md): setting up a machine and building.
- [docs/UI_BIBLE.md](docs/UI_BIBLE.md): the UI conventions every interface
  change follows (rules UIC-nn).
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and
  [docs/CODEMAP.md](docs/CODEMAP.md): where the code is.
- [docs/STATUS.md](docs/STATUS.md): what's built.

Found a bug or have an idea? Use **Help > Report Bug or Idea…** in the app. Security problems go through
[SECURITY.md](SECURITY.md).

## Licence

Operandi is free software under the GNU General Public License, version 3
or later ([LICENSE](LICENSE)). Contributions come in under the same
licence, except Blender-derived modules, which keep GPL-2.0-or-later; see
[CONTRIBUTING.md](CONTRIBUTING.md). Third-party credits and licences are in
[docs/DEPENDENCIES.md](docs/DEPENDENCIES.md) and [docs/licenses](docs/licenses).
The Matcap viewport style uses Blender matcaps (CC0); thank you to the
Blender community.

Operandi is not affiliated with or endorsed by Foundry. Modo is a trademark
of The Foundry Visionmongers Ltd.
