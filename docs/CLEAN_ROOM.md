# Modo and Operandi: the ground rules

Operandi aims to feel like Modo while containing none of Modo's code. This
page explains the few rules every contribution follows, and the stricter
way the maintainer's own work is done. [CONTRIBUTING.md](../CONTRIBUTING.md)
has the short version.

If anything here is unclear for your situation, ask in an issue before you
start.

## 1. What we can't accept

From anyone, for any reason:

- Anything from reverse engineering Modo: decompiling, disassembling,
  debugging or tracing its binaries, reading its program files, plug-ins or
  internal data to see how it works, and tools made for that (REA, Ghidra,
  IDA, Hopper, `otool`, `class-dump`, `strings`, hex editors on its program
  files and the like). Studying a file format so Operandi can open or save
  that format (LXO, for example) is fine; that's the file, not the program.
- Modo source code of any kind: leaked or decompiled code, or code copied
  from the Modo SDK's source and headers, its sample plug-ins, or projects
  built on the SDK.
- Modo's own material: icons, images, interface artwork, presets, sample
  scenes or text copied from its documentation.
- Confidential Foundry material: code, internal documents, or anything an
  employment or contractor agreement treats as confidential.
- Code or specs that came from any of the above, whoever (or whatever AI)
  made them.

## 2. What's fine

- Using Modo as an ordinary user, including its public scripting
  interface, and recreating how its tools behave.
- Reading Modo's public documentation, and its human-readable plain-text
  settings files for default values. Describe what you learn in your own
  words; don't paste its text into code, comments or docs.
- Your own knowledge and experience of Modo, including from working at
  Foundry, as long as nothing confidential comes with it.
- Open-source code under a GPL-3.0-compatible licence, following
  "Licences" in CONTRIBUTING.md.

## 3. If you've worked at Foundry

You're welcome here. Keep to any agreement you have with Foundry; the
project can't advise you on it. One request: if you knew a feature from
Modo's source code, don't write Operandi's version from what you remember
of that code. Describing how the feature behaves, testing it and advising
on it are all fine, and so is writing it fresh. If you'd like to check
where the line sits for you, talk to the maintainer privately.

## 4. `.lxo` files

`.lxo` support is written from the format's public documentation and from
`.lxo` files you made yourself, for compatibility only. Don't take apart
Modo's own sample files to work out the format.

## 5. How the maintainer's own work is done

The maintainer's own builds keep a stricter clean room. Modo is studied in
a separate, private project, which writes behaviour specs into
[`specifications/`](../specifications); the maintainer's build agents work
only from those specs and never see Modo (the "Clean-room boundary" in
[AGENTS.md](../AGENTS.md), and the guard in `.claude/`). That is what the
"Built clean" section of [ABOUT.md](ABOUT.md) describes.

Contributors don't have to work that way. The specs are still the best
place to start on a tool, and when one doesn't answer something, log it in
`docs/SPECIFICATION_QUESTIONS.md` or open a spec-question issue.

## 6. If something slipped in

Mistakes happen; hiding them is the only real problem. If you (or your
agent) used something from section 1:

1. Stop working on the affected code.
2. Tell the maintainer in an issue or privately: what it was, where, and
   which work it could have touched.
3. Together we decide what needs redoing. The record goes in the pull
   request or issue.

## 7. Licences and outside code

Code from other programs is fine when its licence is compatible with GPL v3,
it wasn't built from reverse engineering Modo, its notice is kept, and it's
recorded in [DEPENDENCIES.md](DEPENDENCIES.md). See "Licences" in
CONTRIBUTING.md.
