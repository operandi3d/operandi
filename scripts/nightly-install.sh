#!/bin/sh
# Schedules scripts/nightly.sh with launchd (macOS's built-in scheduler)
# for 02:30 every night. If the Mac is asleep then, launchd runs it when it
# wakes. Uses no Claude.
#
#   scripts/nightly-install.sh            install (or update) the schedule
#   scripts/nightly-install.sh remove     remove it
#
# The schedule runs the nightly.sh of the main checkout, so it always tests
# with main's own script.
set -eu
LABEL=com.operandi.nightly
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
REPO=$(cd "$(dirname "$0")/.." && git rev-parse --path-format=absolute --git-common-dir)
REPO=$(dirname "$REPO")
OUT="$HOME/Developer/Operandi-Test-Results"

launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
if [ "${1:-}" = remove ]; then
    rm -f "$PLIST"
    echo "Nightly run removed."
    exit 0
fi

mkdir -p "$HOME/Library/LaunchAgents" "$OUT"
cat >"$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>$LABEL</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/sh</string>
        <string>$REPO/scripts/nightly.sh</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict><key>Hour</key><integer>2</integer><key>Minute</key><integer>30</integer></dict>
    <!-- Full speed: with LowPriorityIO and Nice 10 macOS throttled the
         run so hard that clippy took 4.5 hours instead of minutes
         (2026-11-19). -->
    <key>ProcessType</key><string>Standard</string>
    <key>StandardOutPath</key><string>$OUT/launchd.log</string>
    <key>StandardErrorPath</key><string>$OUT/launchd.log</string>
</dict>
</plist>
EOF
launchctl bootstrap "gui/$(id -u)" "$PLIST"
echo "Nightly run scheduled for 02:30. Results: $OUT/latest.md"
echo "Run it now with: launchctl kickstart gui/$(id -u)/$LABEL"
