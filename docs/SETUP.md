# Setting up to work on Operandi

This guide takes you from a fresh machine to a running Operandi and passing
checks. macOS on Apple silicon is the main platform. Linux and Windows
build and pass the tests in CI or on the maintainer's build machines, but
the everyday contributor workflow there is newer. Steps marked **not yet
tested on Windows** have not been tried by anyone on Windows yet; if you
try one, please tell us what happened.

## 1. Install the tools (once)

Everyone needs:

- **Git.**
- **Rust through rustup** (<https://rustup.rs>). Don't install a Rust
  version yourself: the repository's `rust-toolchain.toml` pins the exact
  version, with rustfmt and clippy, and rustup downloads it the first time
  you build.
- **CMake.** The Boolean library (`manifold-csg-sys`) builds its C++ code
  with CMake during a normal build.
- **A C++17 compiler.** Several crates compile vendored C++ (remeshing,
  Alembic).

Git LFS, Ninja and Python are only needed for the optional Cycles renderer
(step 5). Python 3.10+ is also needed if you work on icons (see "Icons" in
[AGENTS.md](../AGENTS.md)).

### macOS

```bash
xcode-select --install
```

That gives you Git and the C++ compiler. Then install CMake (for example
with Homebrew) and rustup:

```bash
brew install cmake
```

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

You also need a Metal-capable GPU (any Apple silicon Mac).

### Linux (Ubuntu or Debian)

```bash
sudo apt update && sudo apt install -y build-essential cmake pkg-config git curl libwayland-dev libxkbcommon-dev
```

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Open a new terminal afterwards (or run `source ~/.cargo/env`) so `cargo` is
on your PATH. This is the same setup the Linux CI machine uses
([docs/CI.md](CI.md)). Other distributions need the same packages under
their own names.

### Windows

1. **Visual Studio 2022 Build Tools** (free, from visualstudio.microsoft.com,
   "Tools for Visual Studio"). In the installer tick **Desktop development
   with C++**; its defaults include the compiler, the Windows SDK and CMake.
2. **Git for Windows** (git-scm.com), default options. It includes Git Bash,
   which you'll use to run the repository's scripts.
3. **Rust** (rustup.rs, `rustup-init.exe`), default options (MSVC).

Then, once, in Git Bash:

```bash
git config --global core.longpaths true
```

Some paths in the repository are longer than Windows allows by default.

These are the same tools the maintainer's Windows release builds use
([RELEASE_BUILDS.md](RELEASE_BUILDS.md)), so they are known to build
Operandi. Running the app from `cargo run` and the day-to-day scripts are
**not yet tested on Windows**.

## 2. Get the code

```bash
git clone https://github.com/operandi3d/Operandi.git
```

```bash
cd Operandi
```

## 3. Build and run

The fastest path, with no optional features:

```bash
cargo run --locked -p operandi-app
```

The first build downloads crates from crates.io and compiles everything; it
takes several minutes. Later builds only recompile what changed.

To keep your test settings, recovery journal and crash files apart from
any installed Operandi, give the run its own data folder (`development/` is
ignored by Git):

```bash
OPERANDI_DEV_DATA="$PWD/development/my-data" cargo run --locked -p operandi-app
```

Add a scene path at the end of the command to open it. On Windows, run this
in Git Bash (**not yet tested on Windows**).

## 4. Run the checks

These are the checks CI runs on every pull request, and the ones to pass
before you ask for a merge. They work the same on every platform,
including plain Windows terminals:

```bash
cargo fmt --check
```

```bash
cargo clippy --locked --workspace --all-targets -- -D warnings
```

```bash
cargo test --locked --workspace
```

Two helper scripts make this quicker. They are POSIX shell scripts: on
macOS and Linux run them directly; on Windows run them in Git Bash
(**not yet tested on Windows**) or use the three commands above.

- `scripts/check-fast.sh` while you work: fmt, then clippy and tests for
  only what you changed since `main`. `scripts/check-fast.sh bevel` runs
  only tests whose name contains "bevel".
- `scripts/test-all.sh` runs the same tests as `cargo test --locked
  --workspace`, with every test program at once (about twice as fast).

Avoid `cargo test -p operandi-core` on its own: Cargo then picks different
dependency features and rebuilds core a second time. Use the scripts,
`--workspace` with a name filter after `--`, or `-p operandi-app`.

## 5. Optional features

None of these is needed for modelling work or for the checks above.

### Cycles renderer (final rendering)

You build Cycles yourself from its public source; nothing from the
maintainer's machine is needed. It needs Python 3, Git LFS (for Blender's
library download), optionally Ninja (faster), and about 10 GB of disk, and
takes about an hour the first time.

- macOS: `brew install git-lfs ninja python`, then `git lfs install`.
- Linux: `sudo apt install -y git-lfs ninja-build python3`, then
  `git lfs install`.
- Windows: Git for Windows already includes Git LFS; install Python 3 from
  python.org (tick "Add python.exe to PATH"), then `git lfs install`.

```bash
scripts/build-cycles.sh
```

It downloads Cycles at a pinned commit and Blender's precompiled libraries
(macOS, Windows, Linux x64), applies Operandi's patches and puts the result
in `development/cycles/install`. `scripts/build-cycles.sh --system` uses
libraries already installed on your machine instead. Then:

```bash
cargo run --locked -p operandi-app --features cycles
```

On Windows, run `scripts/build-cycles.sh` in Git Bash. The maintainer's
Windows release builds do this, but a contributor build with
`cargo run --features cycles` is **not yet tested on Windows**.

### USD export (macOS only for now)

Build Cycles first: USD comes from the same Blender library download.

```bash
scripts/build-usd.sh
```

```bash
cargo run --locked -p operandi-app --features usd
```

Windows and Linux support for USD is planned but not built yet.

### 3D painting

Painting is built in by default (since 2026-11-19), in dev, release and CI
builds. To build without it:

```bash
cargo run --locked -p operandi-app --no-default-features
```

`scripts/check-no-painting.sh` checks that such a build has none of it.

### App bundles (macOS)

```bash
scripts/bundle-dev.sh
```

builds an optimized, double-clickable app with the incremental `qa`
profile (a rebuild after a small change takes seconds) and prints where it
put it. Add features with environment variables: `OPERANDI_CYCLES=1`
(renderer), `OPERANDI_PAINTING=0` (leave painting out); USD is included on macOS when
its library is built (`OPERANDI_USD=0` leaves it out).

## 6. Branches and worktrees

Work on a branch named for the feature, started from `main` (see
[CONTRIBUTING.md](../CONTRIBUTING.md)). If you want a second checkout for
another branch without disturbing the first, use a Git worktree:

```bash
scripts/new-worktree.sh my-feature
```

It makes `../Operandi-my-feature` next to your checkout, on a new branch
from `main`, and copies your build cache there where it can so the first
build is faster. Keep one worktree per branch and reuse it.

If the first build in a new worktree fails with "cmake configure failed",
delete the copied `manifold-csg-sys` build output (`target/*/build/manifold-csg-sys-*`)
and build again.

## 7. Using an AI coding agent

AI agents are welcome, under the same rules as people:

- [AGENTS.md](../AGENTS.md) is the agents' rulebook: how a tool is built,
  the checks, the fast-drag rules and the Git rules. Codex and similar
  tools read it directly.
- [CLAUDE.md](../CLAUDE.md) points Claude Code at AGENTS.md and adds a few
  Claude-specific notes.
- `.claude/hooks/clean-room-guard.sh` and the deny rules in
  `.claude/settings.json` stop Claude Code from touching study-side
  material. They're part of the maintainer's own setup; leave them on
  unless you have a reason not to. Whatever agent you use, the
  [Modo ground rules](CLEAN_ROOM.md) apply to it.
- UI work follows [UI_BIBLE.md](UI_BIBLE.md); agents cite its rule numbers.
- End AI-assisted commit messages with a `Co-Authored-By` line for the
  tool, and name the tool in your pull request.

## Where to go next

- [CONTRIBUTING.md](../CONTRIBUTING.md) and the
  [Modo ground rules](CLEAN_ROOM.md) before you write code.
- [ARCHITECTURE.md](ARCHITECTURE.md) and [CODEMAP.md](CODEMAP.md) for a
  map of the code.
- [STATUS.md](STATUS.md) for what's built, and `specifications/` for
  what's next.
