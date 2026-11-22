#!/bin/sh
# Keeps "Operandi Test.app" (scripts/tester-build.sh) up to date: launchd
# runs tester-build.sh every 20 minutes and once at login. A run with
# nothing new costs well under a second; when main has moved it builds in
# the background and swaps the app in once you quit it. Uses no Claude.
#
#   scripts/tester-install.sh           install (or update) the schedule
#   scripts/tester-install.sh remove    remove it
set -eu
LABEL=com.operandi.tester
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
REPO=$(cd "$(dirname "$0")/.." && git rev-parse --path-format=absolute --git-common-dir)
REPO=$(dirname "$REPO")
STATE="$HOME/Developer/Operandi-Test-Results/tester"

launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
if [ "${1:-}" = remove ]; then
    rm -f "$PLIST"
    echo "Operandi Test auto-update removed."
    exit 0
fi

mkdir -p "$HOME/Library/LaunchAgents" "$STATE"
cat >"$PLIST" <<PLIST_EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>$LABEL</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/sh</string>
        <string>$REPO/scripts/tester-build.sh</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict><key>OPERANDI_REPO</key><string>$REPO</string></dict>
    <key>StartInterval</key><integer>1200</integer>
    <key>RunAtLoad</key><true/>
    <key>ProcessType</key><string>Standard</string>
    <key>StandardOutPath</key><string>$STATE/launchd.log</string>
    <key>StandardErrorPath</key><string>$STATE/launchd.log</string>
</dict>
</plist>
PLIST_EOF
launchctl bootstrap "gui/$(id -u)" "$PLIST"
echo "Operandi Test now updates itself within 20 minutes of main moving."
echo "Status: $STATE/status.md"
