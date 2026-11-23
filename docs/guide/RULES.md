# Operandi User Guide: pipeline rules

Approved 2026-11-23.

## The short version
- The guide is written in Markdown inside the main Operandi repo (docs/guide/), so a feature and its page change in the same branch.
- A build turns it into a static HTML site published at operandi3d.org/guide/, one copy per release plus a "test" copy that follows main.
- Help > User Guide opens the guide for the exact version you are running. F1 / help mode (UI bible rule UIC-06) opens the page for whatever you click.
- Rule: no user-facing feature merges without its page. The builder writes the draft; a review pass checks it against the app.
- Screenshots come only from Operandi itself, made by a script, so they can be remade every release instead of going stale.

## 1. When docs get written
Definition of done: any branch that adds or changes something a user can see or do ships with its guide page, new or updated, in the same branch. Internal-only work says "no docs needed" in the merge note. A merge with neither gets bounced.
Who: the builder thread writes the first draft. A review pass checks the page against the running app (a Claude docs thread by default, Leonard for big areas, Codex for routine tool pages). Jason approves voice and templates once, and sees pages in merge pastes.
Release checklist: guide built for this version; coverage report has no new gaps.
Coverage report: the build lists every command, tool and panel with no guide page. That is the backfill to-do list.
Backfill order: Getting started, Interface (from the UI map), the 20 most used tools, then the rest in batches of about 10.

## 2. How docs get made
Source: Markdown in docs/guide/ in the main repo, one file per page, with a header comment naming help ids, template type and first version.
Why main repo: docs branch, merge and version with the code. The website repo only receives built HTML.
Build tool: mdBook (built-in search, offline, themeable).
Generated pages: keyboard shortcuts and commands reference are generated from the command registry and key map, never typed by hand.
Publishing: a GitHub Action in the main repo builds the guide and pushes HTML into the website repo using a scoped token (deploy keys are off on the org). operandi3d.org/guide/<version>/ frozen per release; /guide/latest/ = newest release; /guide/test/ rebuilt from main with a "test build" banner; test-only features tagged "Preview".
Screenshots: made by the offscreen QA harness from a shot list; default Steel theme, fixed window size, 2x, CC0 asset pack scenes only; remade every release; short loops/GIFs only where a still can't show it.
Clean room: pages are written only from Operandi's own app, code and behaviour-only specs. Other apps and their makers are never named, quoted, paraphrased or shown. No comparison pages. The build runs the repo's clean-room name check over docs/guide/.

## 3. Voice and style
Plain and friendly, leaning neutral. Humour in small doses: at most one light touch per page, in intros and tips, never in numbered steps or reference tables.
1. Talk to the reader as "you". Present tense. Short sentences.
2. Lead with what the thing does in one sentence.
3. One task per section. Numbered steps, one action per step.
4. UI names in bold, spelled exactly as on screen (Title Case labels per UIC-86/UIC-111), using the approved UI map names.
5. Keys written as Ctrl+D. Getting started says once that Cmd works wherever Ctrl is written on Mac.
6. Menu paths as Help > User Guide.
7. Say why, not just how, when a setting is surprising.
8. No marketing words ("powerful", "seamless", "simply").
9. No promises about future features.
10. Spelling: match the app's own UI labels; enforced by a spell-check in the build.

## 4. Look, feel and layout
Looks like part of operandi3d.org, dark by default to match the Steel theme, light switch available.
Top bar: logo, search, version switcher, light/dark. Left: nav tree (Getting started, Interface, Selection, Modelling, Sculpting, Curves, Materials, Lights and cameras, Rendering, Animation, Rigging, Scripting and the test bridge, Preferences, Reference, Glossary). Right: "On this page". Bottom: previous/next, Edit this page, "Something wrong? Report it" (GitHub issue with page name filled in).
Search: mdBook built-in, offline.
Templates (in docs/guide/_templates/):
- Tool: one-line purpose, picture, How to use (steps), Handles, Properties (name | what it does | default), Tips, Related.
- Panel: what it shows, picture with callouts, parts of the panel, common tasks, related.
- Concept: plain explanation, picture, how it affects tools, related.
- Tutorial: what you'll make, steps in stages, what you learned.

## 5. Help > User Guide and context help
Help > User Guide opens operandi3d.org/guide/<running version>/ (test builds: /guide/test/). F1 or Help > What's This? enters help mode (UIC-06); the next click opens that control's page.
Help id = the command-registry name of the tool/command/panel/preference. Pages list the help ids they cover; the build makes a redirect per id at /guide/<version>/go/<help-id>. The app only opens that URL. Missing id: lands on search with the name filled in, and the coverage report flags it. Tooltips (UIC-04) remain the first line of help.
Offline copy bundled in the app: later.

## 6. Licence
Guide text and pictures: CC BY-SA 4.0. Code snippets and example scripts: GPL v3. Contributor Terms get one line saying guide contributions are under this licence.

## 7. Rollout
1. Skeleton (mdBook, theme, templates, rules, clean-room check, help-id redirects, publish Action). 2. App side: Help > User Guide + F1 help ids. 3. First pages: Getting started, Interface, top 20 tools. 4. Rule switched on. 5. Backfill from the coverage report.

Bible: section 18 (UIC-300..305) points to this file.
