# Preferences: Undo and Auto-Save (behaviour spec v1, 2026-11-04)

Behaviour-only. "Not yet measured" means the reference's factory value
hasn't been read yet; it will follow in v2. Until then pick a provisional
value and log it as provisional.

## General

- Preferences take effect as soon as they are changed and are kept across
  launches, per user. (Whether there is any Apply/OK step or a reset-to-
  defaults control is still being checked: PREF-Q2.)

## Undo (Defaults > Application)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Max Undo Level | whole number | 100 | Most undo steps kept. When a new step would exceed it, the oldest step is dropped (expected; not yet observed). |
| Group Selection Commands | checkbox | not yet measured | When on, several selection changes in a row become a single undo step, so one Undo reverts the whole run of selection clicks. When off, each selection change is its own step. |
| Max Sub-Commands Recorded | whole number | not yet measured | Upper limit on how many consecutive selection changes are merged into one step while Group Selection Commands is on; the next change after the limit starts a new step (expected). |

Open:
- UND-Q1: what counts as "consecutive" (does any non-selection action
  end a group; does a pause end it)?
- UND-Q2: lowering Max Undo Level while the history is longer: trimmed at
  once or on the next step?

## Auto-Save (Defaults > Auto-Save)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Enable | checkbox | not yet measured | Turns periodic backups on or off. |
| Time Interval (minutes) | whole number | not yet measured | Minutes between backups. |
| Backup Directory | folder, with Browse button | not yet measured | Where backups are written. Separate from the user's own file, which is never overwritten by auto-save. |
| Number of Revisions | whole number | not yet measured | How many backups are kept per scene; the oldest is replaced once the count is reached. |

Behaviour:
- At each interval the app saves a backup of the scene, plus any images
  that changed (painting/sculpting; images are later work for Operandi).
- Before each backup, the 3D view shows a 10-second countdown with a
  Cancel button. Cancel skips that one backup only; the next comes after
  the normal interval.
- The app may pause briefly while a large scene is saved.
- After a crash, the user recovers by opening the latest backup from the
  Backup Directory by hand.
- When the app quits, backups older than 31 days are deleted from the
  Backup Directory.
- The same panel has a "Progressive render auto-save" group (image
  backups of the render preview). Not needed for Operandi.

Open:
- AS-Q1: factory values for the four settings.
- AS-Q2: countdown look and position; does an unchanged scene still get
  backed up?
- AS-Q3: backup file naming and how revisions are numbered.
