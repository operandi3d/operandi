# GitHub settings for inviting contributors (Jason)

Things only the repository owner can change on github.com. Checked
2026-11-11: the repository is private, on a free personal account, with no
branch protection, one collaborator (manfriday), and the self-hosted Mac
runner online.

## Before the first invite

1. **Push `main`.** The local `main` is many commits ahead of GitHub's, so a
   new contributor would clone an old Operandi. (`git push origin main`
   from the main checkout, once you've decided to.)
2. **Protect `main`.** On a private repository GitHub only allows branch
   protection with **GitHub Pro** (about $4 a month) or once the repository
   is public. Without it, any invited collaborator can push straight to
   `main`, because collaborators on a personal account always get write
   access. With Pro: Settings > Rules > Rulesets > New branch ruleset, target
   `main`, and turn on:
   - Restrict deletions; Block force pushes.
   - Require a pull request before merging: 1 approval, "Require review from
     Code Owners" (`.github/CODEOWNERS` names you for everything), dismiss
     stale approvals on new commits.
   - Require status checks to pass: `macOS`, `Linux` (CI) and
     `Signed-off-by check` (DCO).
   - Bypass list: add yourself (Repository admin), so your own merges and
     the coordinator's merge round keep working as now.
3. **Create the issue labels** the forms use (Issues > Labels):
   `bug`, `enhancement`, `spec-question`, `clean-room`.
4. **General settings:** tick "Automatically delete head branches" (Settings >
   General > Pull Requests). Leave the merge options as they are.
5. **Pick a conduct contact.** `CODE_OF_CONDUCT.md` and `SECURITY.md` say
   "the contact on Jason's GitHub profile". Put an email you're happy to
   share on your profile, or tell me one to name in the files.

## Inviting someone

Settings > Collaborators > Add people > their GitHub name. They get write
access (pushing branches, opening and reviewing pull requests). Before they
start, ask them to:

- read CONTRIBUTING.md and docs/CLEAN_ROOM.md;
- add themselves to AUTHORS.md;
- if they're ex-Foundry, read section 3 of docs/CLEAN_ROOM.md and tell you
  privately if they want to check where the line sits.

## CI and your Mac

`ci.yml` now runs the macOS job on your Mac only for pushes and pull
requests you started yourself. Anyone else's runs on GitHub's Mac, which
uses Actions minutes at 10x. If those minutes run short, the other choice is
to run their macOS checks only when you start a run by hand.

## Before going public

- Remove the self-hosted runner (Settings > Actions > Runners) and switch
  the macOS job to `macos-latest` (docs/CI.md).
- Turn on private vulnerability reporting (Settings > Code security), which
  `SECURITY.md` points to.
- Branch protection becomes free; set it up as above if you haven't.
