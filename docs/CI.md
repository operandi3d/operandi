# Automatic test runs (CI)

`.github/workflows/ci.yml` runs `cargo fmt --check` (Linux), clippy with
`-D warnings` and `cargo test` on every pull request, every push to `main`,
and by hand from the repo's Actions tab.

- **Linux** runs on Jason's Linux machine through a self-hosted runner, so
  it costs no GitHub minutes and keeps its build folder between runs. A
  manual run with "github_linux" ticked uses GitHub's Linux machine instead.
- **macOS** runs on Jason's Mac through a self-hosted runner, because GitHub
  bills its Mac minutes at 10x on a private repo. While the runner is off,
  the macOS job waits in the queue (up to 24 hours) and runs once it's on.
  A manual run with "github_mac" ticked uses GitHub's Mac instead.
- Only runs Jason starts himself (his pushes and pull requests) use his
  machines. A push or pull request by anyone else runs on GitHub's machines
  (Mac at 10x minutes), so contributors' code never runs on Jason's.
- Cycles isn't built; the `cycles` feature stays off.
- The Linux job also runs `scripts/area-deps.py --check --strict`, and
  so does `scripts/merge-check.sh`: either fails when a branch adds an area
  link against the flow inside a dependency cycle (see
  [AREA_DEPENDENCIES.md](AREA_DEPENDENCIES.md); what to do is in
  CONTRIBUTING.md, step 5).

## The Mac runner

Set up once:

1. On GitHub: the Operandi repo > Settings > Actions > Runners > New
   self-hosted runner > macOS, ARM64.
2. Run the commands that page shows, but make the folder
   `~/Developer/actions-runner`. When
   `./config.sh` asks, name the runner `operandi-mac` and accept the default
   labels (`self-hosted`, `macOS`, `ARM64`).
3. Don't run `./svc.sh install`; that would make it start with the Mac.

Turn it on when working on Operandi, and off when done:

```bash
cd ~/Developer/actions-runner && ./run.sh   # on; leave the window open
# Ctrl+C in that window                    # off
```

Start it from a Terminal where `cargo` works (rustup's `~/.cargo/bin` on the
PATH). Its builds live in `~/Developer/actions-runner/_work` (several GB);
deleting `_work` is safe and just makes the next run slower.

**Before the repo goes public**, remove the runner (Settings > Actions >
Runners) and switch the macOS job back to `macos-latest`: on a public repo
anyone's pull request could run code on the Mac. Public repos get GitHub's
machines free.

## The Linux runner

Set up once, on a 64-bit Ubuntu or Debian machine (8+ cores and about 60 GB
free is comfortable):

1. Build tools and Rust:
   ```bash
   sudo apt update && sudo apt install -y build-essential cmake pkg-config git curl libwayland-dev libxkbcommon-dev
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   source ~/.cargo/env
   ```
   `source ~/.cargo/env` matters: `./config.sh` saves the current PATH for
   the runner, and the runner needs `cargo` on it.
2. On GitHub: the Operandi repo > Settings > Actions > Runners > New
   self-hosted runner > Linux, x64. Run the commands that page shows, with
   the folder at `~/actions-runner`. Name the runner `operandi-linux` and
   accept the default labels (`self-hosted`, `Linux`, `X64`).
3. Start it with the machine:
   `cd ~/actions-runner && sudo ./svc.sh install $USER && sudo ./svc.sh start`
   (`sudo ./svc.sh stop` / `uninstall` to stop or remove it).

The tests there run with `OPERANDI_TEST_GPU=off`, so the GPU tests skip
as they do on GitHub's machine: they compare the card's pictures and shader
maths with tolerances tuned to Apple's GPUs, and run on the Mac job. (On
2026-11-18 the gaming machine's card failed two of them by small rounding
differences and once crashed the test program.)

Builds live in `~/actions-runner/_work`; deleting `_work` is safe and just
makes the next run slower. While the runner is offline, Linux jobs wait in
the queue.

**Before the repo goes public**, remove this runner too and switch the Linux
job back to `ubuntu-latest`.

## Nightly run on the Mac

`scripts/nightly.sh` tests the newest local `main` every night, in its own
detached worktree `~/Developer/Operandi-nightly`, so it never touches a
checkout someone is working in. It's plain shell and cargo: no Claude.
It runs fmt, clippy, every test, 2,000 random monkey-user sessions
(`monkey_tests.rs`), and the Cycles and USD tests when their
libraries are built in the main checkout's `development/` folder.

Results go to `~/Developer/Operandi-Test-Results/`: `latest.md` (one line per
step, failing tests, and failures that are new since the night before),
`runs/<date>.md`, and full logs in `runs/<date>/`.

- Schedule it (02:30; if the Mac is asleep, it runs on wake):
  `scripts/nightly-install.sh`. Remove: `scripts/nightly-install.sh remove`.
- Run it now: `launchctl kickstart gui/$(id -u)/com.operandi.nightly`, or
  `scripts/nightly.sh` in a Terminal.
- The first night builds from cold; later nights reuse the worktree's
  build folder.
