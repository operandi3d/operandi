@AGENTS.md

## Claude-specific notes

- `.claude/hooks/clean-room-guard.sh` blocks tool calls that touch Modo
  research material, and `.claude/settings.json` denies the same paths. Don't
  work around a block or edit the guard unless Jason asks; log a spec
  question instead.
- Claude's screen control treats Operandi as a browser (view only, no
  clicks), so GUI passes go to Jason or Codex. Write the checklist for them.
