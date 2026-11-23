# Update check — GUI check

Build: the QA app named in the thread (`scripts/bundle-dev.sh`). Use its
own data folder (`OPERANDI_DEV_DATA`) so your real preferences are
untouched.

The check sends one plain GET to `api.github.com` (the public
`operandi3d/operandi` releases) with the system's `curl`, and nothing
about you. Nothing is ever downloaded or installed by the app: OK only
opens the release page in your browser.

## Faking a newer build

`OPERANDI_UPDATE_FEED=<file>` makes the app read that JSON file instead
of asking GitHub. Make these two files somewhere (e.g. `/tmp`):

`test-newer.json` (Test channel, for macOS, Windows and Linux; the time is
far in the future, so it is always newer):

```json
{"tag_name": "test-build", "prerelease": true,
 "body": "- **macOS**: `Operandi-Test-macOS.zip`, built from [`1a2b3c4d`](https://example.com)\n<!-- operandi-build mac=1a2b3c4d mac_time=4102444800 windows=1a2b3c4d windows_time=4102444800 linux=1a2b3c4d linux_time=4102444800 -->"}
```

A platform with an empty value in that line (`linux= linux_time=`) has no
test build yet: the check offers nothing for it.

`release-newer.json` (Release channel):

```json
[{"tag_name": "v9.0.0", "draft": false, "prerelease": true},
 {"tag_name": "v9.1.0", "draft": true},
 {"tag_name": "test-build", "draft": false, "prerelease": true}]
```

Start the QA app from Terminal with the variable set, for example:

```bash
OPERANDI_UPDATE_FEED=/tmp/test-newer.json OPERANDI_DEV_DATA=/tmp/operandi-qa "/path/to/Operandi update-check <commit>.app/Contents/MacOS/operandi-app"
```

## Steps

1. Preferences (Cmd+,) > Defaults > Application: an **Updates** section
   under Names and Files with **Update Channel** (a dropdown: Release,
   Test; default Release) and the checkbox **Check for Updates on Launch**
   (box then text, on). Set Update Channel to Test. Quit.
2. Start with `test-newer.json` as above. Once the window is up an
   **Update Available** dialog opens: "A newer test build is available,
   built Jan 1 from 1a2b3c4d. You have <commit>, built <date>. OK opens
   the download page." Buttons on the right: OK (Ret), Cancel (Esc), Skip
   This Build.
3. Press Return: the browser opens
   `https://github.com/operandi3d/operandi/releases/tag/test-build`.
4. Quit and start again: the prompt comes back. Press **Skip This Build**.
   Quit and start again: no prompt.
5. Help > **Check for Updates** (no "…"): the prompt shows even though the
   build was skipped. Esc closes it and nothing changes.
6. Untick Check for Updates on Launch, quit, start: no prompt. Help >
   Check for Updates still works. Tick it again.
7. Set Update Channel to Release, quit, start with
   `OPERANDI_UPDATE_FEED=/tmp/release-newer.json`: "Operandi 9.0.0 is
   available. You have 0.1.0. OK opens the download page." (The draft
   9.1.0 and test-build are ignored.) OK opens `.../releases/tag/v9.0.0`.
8. With Preferences open, choose Help > Check for Updates from the menu
   bar: the prompt waits until Preferences is closed, then opens.
9. Start with `OPERANDI_UPDATE_FEED=/tmp/missing.json` (no such file):
   nothing at launch. Help > Check for Updates: "Couldn't check for
   updates: Couldn't read OPERANDI_UPDATE_FEED (…)."
10. Without the variable, Wi-Fi off: nothing at launch; Help > Check for
    Updates says GitHub could not be reached. Wi-Fi on, Release channel:
    "You have the newest release." (or the offer, if a newer release is
    out).
11. On Linux (the AppImage), steps 2-7 work the same: the Test channel
    reads the `linux=` fields, or the "- **Linux**: ... built from" line
    when the hidden line has none. Change the feed to `linux= linux_time=`:
    Help > Check for Updates says there is no test build for this platform
    yet.
12. Type `operandi.update.check` on the command line: same as the menu.
    Type `pref.value updates.channel Test` and `pref.value
    updates.check_on_launch false`: the Preferences rows follow.

A development build with uncommitted changes (`+dirty` in About) can't
compare commits, so with a test-build feed that has no hidden
`operandi-build` line it says nothing at launch, and the menu check says
why.
