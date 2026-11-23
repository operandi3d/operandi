# Release builds for Windows and Linux

macOS builds come from `scripts/release-macos.sh` (signed and notarized), or
for testers from `scripts/bundle-dev.sh` on the Mac with
`OPERANDI_DEV_PROFILE=release` (its default `qa` profile is for QA only) and
`OPERANDI_CYCLES=1` for rendering. Windows and Linux builds with Cycles
rendering are built on a Windows and a Linux VM with one script,
`scripts/build-release.sh`. They can't be cross-compiled from the Mac:
Cycles' prebuilt Windows libraries only work with Microsoft's compiler.

The first build on a machine downloads Cycles' prebuilt libraries (several
GB) and compiles Cycles, about an hour. Later builds reuse both and take a
few minutes.

## Linux VM (Ubuntu 24.04, x86_64)

Set up once:

```bash
sudo apt update && sudo apt install -y build-essential git git-lfs cmake ninja-build python3 patchelf squashfs-tools curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
git lfs install
git clone <the Operandi repo URL> ~/Developer/Operandi
```

Each build:

```bash
cd ~/Developer/Operandi && git pull && scripts/build-release.sh
```

The result is `~/Developer/Operandi-Builds/Operandi <branch> <commit> Linux.AppImage`.
That AppImage only runs on distributions at least as new as the build
machine (its glibc). For one that runs almost everywhere (glibc 2.28: Ubuntu
20.04+, Fedora 29+, Bazzite, Debian 10+), build inside an older Linux with
podman instead (`sudo apt install -y podman` once):

```bash
cd ~/Developer/Operandi && git pull && scripts/build-release-container.sh
```

It builds in a Rocky Linux 8 container (`scripts/linux-build-container/`),
the same base Blender's own Linux builds use, and writes the same AppImage
name. Use this one for builds you hand to other people.

## Windows VM (Windows 11 or Server 2022, x86_64)

Install once:

1. **Visual Studio 2022 Build Tools** (free, from visualstudio.microsoft.com,
   "Tools for Visual Studio"). In the installer tick **Desktop development
   with C++**; its defaults include the compiler, the Windows SDK and CMake.
2. **Git for Windows** (git-scm.com), defaults. It includes Git LFS and
   Git Bash.
3. **Python 3** (python.org). Tick "Add python.exe to PATH".
4. **Rust** (rustup.rs, `rustup-init.exe`), default options (MSVC).

Then in **Git Bash**:

```bash
git config --global core.longpaths true
git lfs install
git clone <the Operandi repo URL> ~/Developer/Operandi
```

Each build, in Git Bash:

```bash
cd ~/Developer/Operandi && git pull && scripts/build-release.sh
```

The result is `~/Developer/Operandi-Builds/Operandi <branch> <commit> Windows.zip`
(in `C:\Users\<you>\Developer\Operandi-Builds`). Unzip it anywhere and run
`Operandi.exe`. The build isn't signed, so Windows SmartScreen asks first
(More info > Run anyway).

## Notes

- Both machines need about 40 GB free, 16 GB of RAM, and 4+ cores.
- `development/cycles` holds the Cycles download and build; deleting it is
  safe and makes the next build start from scratch.
- Rendering is CPU only on Windows and Linux for now (GPU back ends are
  switched off in `scripts/build-cycles.sh`).

## Animation in test builds

Animation shows by default. Testers can turn it off with Preferences >
Defaults > Application > Experimental > Animation, or start the app with
`OPERANDI_ANIMATION=0` (see SETUP.md, "Animation (on by default)"). Python
scripting stays opt-in (Experimental > Python scripting, or
`OPERANDI_PYTHON=1`).
