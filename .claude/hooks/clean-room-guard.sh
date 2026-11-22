#!/bin/bash
# Clean-room guard for Operandi (PreToolUse hook).
#
# Operandi is implemented only from the specs in specifications/. This hook
# stops Claude from reading, searching or running anything that touches the
# Modo research side: the research project, the Modo application and its
# files, the Modo guide, Codex sessions and the Modo MCP. Exit code 2 blocks
# the tool call and shows the message to Claude.

input=$(cat)
tool=$(jq -r '.tool_name // ""' <<<"$input")

# Only the fields that name a path, a command or a URL. File contents being
# written and Grep's search text are not checked, so code and docs may still
# mention these names. If the input can't be parsed, check all of it.
fields='.file_path, .path, .notebook_path, .glob, .command, .url'
[[ "$tool" == Glob ]] && fields="$fields, .pattern"
targets=$(jq -r ".tool_input | [$fields] | map(select(. != null)) | join(\"\\n\")" <<<"$input" 2>/dev/null) \
  || targets=$input
case "$tool" in
  mcp__*) targets=$(jq -c '.tool_input' <<<"$input" 2>/dev/null) || targets=$input ;;
esac

blocked=(
  '(^|[/~ "'\''=])Research([/ "'\'']|$)'  # ~/Research, the research project
  'Modo.{0,2}Rebuild'                      # ~/Documents/Modo Rebuild
  'Modo1[0-9]'                             # the Modo app, guide and HTML docs
  'Applications/Modo'
  'UserGuide\.pdf'
  'Luxology'                               # Modo kits and preferences
  '\.modo_mcp'                             # the Modo knowledge index
  '\.codex'                                # Codex sessions and config
  'reference_projects'
  'foundry\.com'                           # Foundry's online docs and SDK
)

for re in "${blocked[@]}"; do
  if grep -qiE -- "$re" <<<"$targets"; then
    echo "Blocked by Operandi's clean-room guard: this touches Modo research material (matched /$re/). Work from specifications/ only; log a spec question instead." >&2
    exit 2
  fi
done

if [[ "$tool" == mcp__modo* ]]; then
  echo "Blocked by Operandi's clean-room guard: the Modo MCP belongs to the Research project." >&2
  exit 2
fi
exit 0
