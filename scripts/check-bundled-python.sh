#!/bin/sh
# Checks the Python inside a built QA app (Python scripting): the app's
# binary binds to the libpython in its own Frameworks folder (ahead of the
# USD bridge, which carries another static Python), and, started for a few
# seconds with its MCP bridge on, the app's own interpreter imports the
# standard library (scripts/python-app-check.py), shows help(operandi) and
# runs the console's "try these" lines.
#
#   scripts/check-bundled-python.sh "<path>.app"
#
# The live part opens the app's window briefly, with its own data folder.
set -eu
cd "$(dirname "$0")/.."
APP="$1"
BIN="$APP/Contents/MacOS/operandi-app"
LINKS=$(otool -L "$BIN" | awk 'NR > 1 {print $1}')
PY_AT=$(printf '%s\n' "$LINKS" | grep -n 'libpython' | head -1 | cut -d: -f1)
USD_AT=$(printf '%s\n' "$LINKS" | grep -n 'liboperandi_usd' | head -1 | cut -d: -f1)
[ -n "$PY_AT" ] || { echo "check-bundled-python: the app has no libpython"; exit 1; }
if [ -n "$USD_AT" ] && [ "$USD_AT" -lt "$PY_AT" ]; then
    echo "check-bundled-python: liboperandi_usd loads before libpython; the app would run the bridge's Python"
    exit 1
fi
echo "Link order: libpython first."
[ "${OPERANDI_PYTHON_CHECK_LIVE:-1}" = 1 ] || exit 0
MCP="${OPERANDI_MCP_BIN:-target/qa/operandi-mcp}"
[ -x "$MCP" ] || "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" build --locked --profile qa -p operandi-mcp
T=$(mktemp -d /tmp/operandi-python-check.XXXXXX)
trap 'kill "$PID" 2>/dev/null; rm -rf "$T"' EXIT
mkdir -p "$T/data"
OPERANDI_MCP=1 OPERANDI_MCP_SOCKET="$T/s.sock" OPERANDI_PYTHON=1 OPERANDI_DEV_DATA="$T/data" \
    "$BIN" >"$T/app.log" 2>&1 &
PID=$!
i=0
while [ ! -S "$T/s.sock" ] && [ $i -lt 60 ]; do sleep 1; i=$((i + 1)); done
[ -S "$T/s.sock" ] || { echo "check-bundled-python: the app's bridge didn't start"; cat "$T/app.log"; exit 1; }
python3 - "$T/req.jsonl" scripts/python-app-check.py crates/operandi-python/src/lib.rs <<'PY'
import json, re, sys
out, probe, lib = sys.argv[1:4]
src = open(lib).read()
block = src.split("pub const TRY_THESE: &[&str] = &[", 1)[1].split("];", 1)[0]
lines = [json.loads(m) for m in re.findall(r'^\s*("(?:[^"\\]|\\.)*"),\s*$', block, re.M)]
try_these = "import operandi\n" + "\n".join(l for l in lines if not l.startswith("help("))
with open(out, "w") as f:
    f.write(json.dumps({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "check", "version": "1"}}}) + "\n")
    f.write(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "python_run", "arguments": {"code": open(probe).read()}}}) + "\n")
    f.write(json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "python_run", "arguments": {"code": try_these}}}) + "\n")
PY
(cat "$T/req.jsonl"; sleep 6) | "$MCP" --attach "$T/s.sock" > "$T/out.jsonl" 2>&1 || true
python3 - "$T/out.jsonl" <<'PY'
import json, sys
results = {}
for line in open(sys.argv[1]):
    try:
        d = json.loads(line)
    except ValueError:
        continue
    if "result" in d and d.get("id") in (1, 2):
        results[d["id"]] = d["result"]
ok = True
for i, what in ((1, "standard library + help"), (2, "try these")):
    r = results.get(i)
    text = r and r["content"][0]["text"]
    if not r or r.get("isError"):
        print(f"FAIL {what}: {text}")
        ok = False
        continue
    out = json.loads(text)["output"]
    if i == 1 and ("missing []" not in out or "help True" not in out):
        print(f"FAIL {what}: {out}")
        ok = False
    else:
        print(f"ok   {what}")
sys.exit(0 if ok else 1)
PY
