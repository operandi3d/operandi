# Releasing Operandi for the Mac

A Mac app downloaded from the internet only opens without warnings if two
things are true: it is **signed** with your Apple Developer ID certificate
(proof it came from you and hasn't been changed), and Apple has
**notarized** it (scanned it and given it a pass). `scripts/release-macos.sh`
does both and packs the app into a disk image (`.dmg`) people can download.

Windows and Linux are built on their own machines (`docs/RELEASE_BUILDS.md`).
To put all three on GitHub in one go, use `scripts/release.sh` (next section).

## Releasing all three

```bash
scripts/release.sh v0.1.0 --dry-run   # checks only, changes nothing
scripts/release.sh v0.1.0             # the real thing
```

This is the only command that makes a release. The other one that uploads
is `scripts/test-build.sh`, which only ever replaces the files on the
"Test build" page ("Test builds", below). Your everyday builds
(`scripts/bundle-dev.sh` and friends) never upload.

What it does, in order:

1. **Checks** that you're on `main`, nothing is left uncommitted, your `main`
   matches GitHub's, the version you typed matches
   `crates/operandi-app/Cargo.toml`, and that version hasn't been released
   before. It also checks the Mac certificate and notary login, and asks
   GitHub which build machines are switched on. A platform whose machine is
   off is **skipped**, and the script says so loudly (the release then has
   fewer files; you can add the missing one later by hand on the release's
   GitHub page).
2. **Runs all the tests** (`scripts/test-all.sh`). Any failure stops here.
3. **Tags** the commit (a tag is a permanent name tag on one commit, like
   "v0.1.0") and pushes the tag to GitHub.
4. **Starts the Linux and Windows builds** on your Linux box and Windows
   machine (the "Release builds" workflow, `.github/workflows/release-builds.yml`),
   then builds, signs and notarizes the Mac version here while they run.
5. **Waits** for Linux and Windows, downloads their files, and **uploads**
   all of them, plus a checksum list, to the repo's Releases page.

If anything fails after step 3, the script removes the tag again and
publishes nothing, so you can fix it and rerun with the same version.

Options: `--draft` uploads it as a draft that only you see until you press
Publish on GitHub; `--platforms mac,linux` builds only those. A version with
a dash (`v0.2.0-beta.1`) is marked as a pre-release.

To release a new version: change `version = "..."` in
`crates/operandi-app/Cargo.toml`, commit, push, then run the script with
the same number.

**While the repo is private, only its collaborators can see the release.**
Everyone else gets "page not found" until the repo goes public.

The files are named `Operandi-<version>-macOS.dmg`,
`Operandi-<version>-Windows-x64.zip` and
`Operandi-<version>-Linux-x86_64.AppImage`, and a copy stays in
`~/Developer/Operandi-Builds/release-<version>/upload`.

All three include Cycles rendering and 3D painting. USD import/export is in
the Mac version only for now (it isn't built for Windows or Linux yet).
`OPERANDI_PAINTING=0` leaves painting out of a build.

### One-time setup

- **GitHub CLI** on the Mac: `brew install gh`, then `gh auth login`
  (pick GitHub.com, SSH, and log in with the browser).
- **Mac signing**: the certificate and notary login below.
- **Linux**: the "gaming" box already runs a GitHub runner (`docs/CI.md`).
  It also needs the build tools from `docs/RELEASE_BUILDS.md` ("Linux VM")
  and, for an AppImage that runs on older Linux too, podman
  (`sudo apt install -y podman`). Without podman it still builds, with a
  warning that the file needs a fairly new Linux.
- **Windows**: the Windows machine needs the tools from
  `docs/RELEASE_BUILDS.md` ("Windows VM") and a GitHub runner:
  1. On GitHub: the repo > Settings > Actions > Runners > New self-hosted
     runner > Windows, x64. Follow the commands it shows in PowerShell, in a
     folder like `C:\actions-runner`.
  2. When `config.cmd` asks, name it `operandi-windows` and keep the default
     labels (`self-hosted`, `Windows`, `X64`).
  3. Say **yes** to running it as a service, and when it asks which account,
     give your own Windows user (not the default "NETWORK SERVICE"), so it
     finds Rust, Git and Visual Studio the way you installed them.
  4. Restart the service after installing new tools (Services app >
     "GitHub Actions Runner" > Restart).

  The first release build on each machine takes about an hour (it builds
  Cycles); later ones take minutes, because each machine keeps that work in
  `~/operandi-release-build` (deleting that folder is safe, just slow next
  time).
- The Windows version isn't signed, so Windows shows a SmartScreen warning
  (More info > Run anyway). Signing it needs a Windows code-signing
  certificate, which is a separate purchase; not needed to start.

## Test builds

Testers get the newest Mac, Windows and Linux apps from one page that never
changes address:

    https://github.com/operandi3d/operandi/releases/tag/test-build

To put a new test build there, run on the Mac:

```bash
cd ~/Developer/Operandi && OPERANDI_WINDOWS_SSH=<user>@<windows machine> OPERANDI_LINUX_SSH=<user>@<linux machine> scripts/test-build.sh
```

(`OPERANDI_WINDOWS_SSH` and `OPERANDI_LINUX_SSH` are the ssh logins of the
Windows and Linux build machines; `docs/RELEASE_BUILDS.md` sets them up.
Linux builds in its checkout at `~/Development/Operandi`, or
`OPERANDI_LINUX_SSH_REPO`, inside the older-Linux container, so it needs
podman there.)

It builds GitHub's `main` (give a branch name at the end to build a pushed
branch instead): the Mac app here, and the Windows zip and Linux AppImage
on their machines, all at the same time. It checks none of them has files that mustn't be shared, and replaces the
files on the page. The page is a GitHub
*pre-release* called "Test build", so it never shows up as the latest
release, and real releases like v0.1.0 are left alone.

- `--platforms mac` (or `windows`, `linux`, or a list like `mac,linux`)
  builds and replaces just those files; the page still says which commit
  the others came from.
- `--no-upload` builds and checks without uploading; the files are in
  `~/Developer/Operandi-Builds/test-build/upload`.
- It builds in its own folder, `~/Developer/Operandi-test-build`, made the
  first time, so it never switches branches in your main checkout.
- Only commits that are on GitHub can be built, so anyone can get the
  source of each file (the GPL asks for that).
- The Mac app is a test build, not signed with your certificate or
  notarized, so the first time testers open it they click
  **Open Anyway** in System Settings > Privacy & Security. The page tells
  them how.
- Don't run it while `scripts/release.sh` is building Windows: both use the
  Windows machine's one checkout.

## Releasing only the Mac version

From the Operandi folder (or any of its worktrees), with everything
committed:

```bash
scripts/release-macos.sh
```

It takes a few minutes on top of the build (Apple usually answers within
5 minutes, sometimes longer). The result is

```
~/Developer/Operandi-Builds/Operandi <version> <commit>.dmg
```

for example `Operandi 0.1.0 bdf64ca3.dmg`. That file is ready to upload.
The version comes from `crates/operandi-app/Cargo.toml`; change it there
before a new release.

If you have uncommitted changes the script still runs, but the name gets a
`+` to warn you that the build doesn't match any commit. Don't publish those.

## What the script does, step by step

1. **Builds** Operandi in release mode with the Cycles renderer, set up as a
   packaged app (settings go in your user folders, the MCP bridge is off
   unless someone turns it on). It builds Cycles first if needed (about an
   hour the very first time on a machine).
2. **Makes the app**, `Operandi.app`, with its icons, the Cycles libraries and
   the licence texts. This part is shared with the QA builds
   (`scripts/macos-app-bundle.sh`).
3. **Signs** every library inside the app, then the app itself, with your
   certificate. It also turns on Apple's "hardened runtime" (a set of safety
   restrictions Apple requires for notarization) and adds a secure time stamp.
   Operandi needs no exceptions to those restrictions; rendering with Metal
   works under them.
4. **Notarizes the app**: sends it to Apple and waits for the answer, then
   **staples** Apple's pass (a "ticket") onto the app so it opens even when
   the Mac is offline.
5. **Packs the disk image** with the app and a shortcut to Applications (so
   people can drag it across), signs the disk image, notarizes it and
   staples it too.
6. **Checks** both with Gatekeeper (the part of macOS that decides whether
   downloaded apps may open). The last lines should say
   `source=Notarized Developer ID` for the disk image and for the app.

## Settings

The top of `scripts/release-macos.sh` has the names it uses. You can also
set them for one run, e.g. `OPERANDI_BUNDLE_ID=com.example.Operandi scripts/release-macos.sh`.

| Setting | Default | What it is |
|---|---|---|
| `OPERANDI_BUNDLE_ID` | `org.operandi3d.Operandi` | The app's permanent name to macOS, made from the project's domain, operandi3d.org. Don't change it: macOS would treat the app as a different program (settings and permissions start fresh). |
| `OPERANDI_SIGN_IDENTITY` | `Developer ID Application: Jason Morris (PNA7R84ZP6)` | The certificate's name in your keychain. |
| `OPERANDI_NOTARY_PROFILE` | `operandi` | The saved login the script uses to talk to Apple's notary service. |
| `OPERANDI_CYCLES_LIB_DIR` | (build Cycles here) | Use a finished Cycles build from another folder, e.g. `~/Developer/Operandi/development/cycles/install/lib`, to skip building it again in a new worktree. |
| `OPERANDI_RELEASE_OUTPUT` | `~/Developer/Operandi-Builds` | Where the `.dmg` goes. |
| `OPERANDI_SKIP_NOTARIZE=1` | off | Sign and pack only, without asking Apple. For testing the script; the result is not a real release. |

## The certificate

Your Developer ID certificate lives in the **login** keychain (open
Keychain Access, choose "login", then "My Certificates"). To check it's
there from Terminal:

```bash
security find-identity -v -p codesigning
```

It should list `Developer ID Application: Jason Morris (PNA7R84ZP6)`.

### Keep a backup

The certificate comes with a private key that exists only on this Mac.
If the Mac dies without a backup, you have to make a new certificate (which
works, but old builds can't be re-signed with the old one). To back it up:

1. In Keychain Access, under "My Certificates", find the Developer ID
   Application certificate and click the arrow so you see the key under it.
2. Select the certificate, choose File > Export Items, and save it as a
   `.p12` file. Keychain Access asks for a password to protect the file:
   pick a strong one and keep it in your password manager.
3. Store the `.p12` somewhere safe and off this Mac (not in the Operandi
   repository, and not anywhere public).

### Restore it on a new Mac

Double-click the `.p12` file and enter its password; it goes into the login
keychain. Then run the check above. You also need to save the notary login
again (next section).

### Renew it

Developer ID certificates last five years. Apple emails before it expires.
To make a new one: in Xcode, Settings > Accounts, pick your Apple ID and
team, click "Manage Certificates", then "+" and "Developer ID Application".
(Or create it at developer.apple.com under Certificates.) If the name
changes, update `OPERANDI_SIGN_IDENTITY` in the script. Apps signed with the
old certificate keep working after it expires, because of the time stamp.
Back up the new one as above.

## The notary login

The script logs in to Apple's notary service with a saved profile called
`operandi` (stored in the keychain, never in the repository). If it's missing
or you changed your Apple ID password, save it again:

1. Make an app-specific password at account.apple.com (Sign-In and Security >
   App-Specific Passwords). Your normal password won't work here.
2. Run this and type the app-specific password when asked:

```bash
xcrun notarytool store-credentials operandi --apple-id <your Apple ID email> --team-id PNA7R84ZP6
```

## When notarization fails

The script stops and prints Apple's log, which lists each problem with the
file it found it in. Common causes:

- **"The signature does not include a secure timestamp"** or **"The
  executable does not have the hardened runtime enabled"**: something in the
  app wasn't signed by the script, usually a new kind of file added to the
  bundle. Check that the signing step covers it.
- **"The binary is not signed with a valid Developer ID certificate"**: the
  certificate expired or the wrong one was used; see "Renew it".
- **Login errors** (HTTP 401, "invalid credentials"): save the notary login
  again (above).
- **Apple is slow or down**: the script waits; if it gives up, run it again
  later. Apple's status page is developer.apple.com/system-status.

To look up an earlier submission yourself:

```bash
xcrun notarytool history --keychain-profile operandi
```

```bash
xcrun notarytool log <submission id> --keychain-profile operandi
```

## Note for this Mac

The script uses Apple's tools from `/Applications/Xcode.app` when it's
there, whatever `xcode-select` points at (on this Mac, the Command Line
Tools). Nothing to do unless Xcode moves.
