# Nav / Lock / Hide audit (2026-11-07, branch `nav-lock-hide`)

What Operandi had on `main` (716c76d6) before this branch, for each spec row.
**exists** = already does what the row says; **differs** = there but not as
specced; **missing** = not there. "Operandi preset" = Preferences > Input >
Navigation "Operandi (native)"; the Blender and Maya presets are left alone.

## Key clashes (NOT changed — Jason decides)

| Key | Spec wants | Operandi now |
| --- | --- | --- |
| **G** | NAV-12 Re-centre view on the point under the pointer | **Edge Slice** (default-shortcuts spec), in every mode |

That is the only clash. **Decided (Jason, 2026-11-07, via the coordinator):**
G stays Edge Slice. Re-centre is built as a command in the View menu, with
no default key; it can be bound in the Input Map. Checked and free: `.` `,` `/` `Shift+/` `Cmd+A`
`Cmd+Shift+A` `J` `Shift+J` `Cmd+J` `I` `H` `Shift+H` `Ctrl+H` `U`, and the
Alt + middle/right drags (they only show a "lasso unavailable" message today).
Not clashes, same meaning already: `A` = Frame All (NAV-15), `Shift+A` = Frame
Selection (NAV-16), `Ctrl+Space` = View pie (NAV-19). `Alt+Shift+A` is Select
All, which the spec doesn't mention.

Notes on near-clashes:
- **Cmd + Alt + left drag** orbits today (the Cmd key isn't read). NAV-06 makes
  it Zoom on Mac, as decided.
- **Maya preset** already uses Alt+middle = Pan and Alt+right = Zoom. The new
  Alt + middle/right gestures are added to the Operandi preset only.
- **Cmd+A** selects text inside a text field; shortcuts never fire while a
  field has focus, so no conflict.

## NAV — navigation

| Row | Status | Notes |
| --- | --- | --- |
| NAV-01 Alt+left orbit | exists | |
| NAV-02 Alt+middle roll | missing | |
| NAV-03 Alt+right flick orbit | missing | Now look around: the eye stays put and the view turns (changed 2026-11-22 from Jason's hands-on observation of the reference app; the older shortcut spec SC-034 said flick orbit, from the docs) |
| NAV-04 Shift+Alt+left pan | exists | |
| NAV-05 Shift+Alt+right vertical pan | missing | |
| NAV-06 Ctrl+Alt+left zoom | differs | zooms about the view centre, not where pressed; Cmd variant orbits |
| NAV-07 Ctrl+Alt+right box zoom | missing | |
| NAV-08 wheel zoom to pointer | exists | Preferences "Zoom to cursor", default on |
| NAV-10 `.` zoom in | missing | |
| NAV-11 `,` zoom out | missing | |
| NAV-12 `G` re-centre | missing | clash with Edge Slice: built unbound, View > Re-centre |
| NAV-13 `/` turntable | missing | |
| NAV-14 `Shift+/` upright | missing | |
| NAV-15 `A` fit | exists | "Frame All" |
| NAV-16 `Shift+A` fit selected | exists, small difference | Item mode frames only the primary selected item, not every selected item |
| NAV-17 `Cmd+A` align selected | missing | |
| NAV-18 `Cmd+Shift+A` fit and align | missing | |
| NAV-19 `Ctrl+Space` view pie | exists | pie_menu.rs, "Pie: View Direction" |

## LCK Part A — item lock

The lock already exists as **Assembly > Lock** (Default / Yes / No; Default
means unlocked).

| Row | Status | Notes |
| --- | --- | --- |
| LCK-01 Item List right-click Lock/Unlock | missing | |
| LCK-02 Lock setting in properties | exists, differs | three choices (Default/Yes/No), not Yes/No |
| LCK-03 padlock in Item List | missing | |
| LCK-04 no item transforms | mostly exists | tools, typed fields (greyed), Reset Transform, Drop Action, camera sync refuse; Quick Align / Ground Align not checked |
| LCK-05 links, visibility, pivot… | differs | Visible row in Display tab refused; **eye toggle and parenting not refused** |
| LCK-06 component editing still allowed | exists | |
| LCK-07 saved with scene | exists | native v16 |
| LCK-08 refusal says why | exists | "<name> is locked (Assembly > Lock)" in the status bar |

## LCK Part B — geometry lock

| Row | Status |
| --- | --- |
| LCK-20 … LCK-27, LCK-30 … LCK-33 | all missing |

There is no UV viewport in Operandi, so LCK-27 can't be built yet.

## HID — hide / unhide

| Row | Status |
| --- | --- |
| HID-01 … HID-06, HID-10 … HID-13 | all missing (only the whole-item eye exists) |

HID-06 (UV viewport): no UV viewport yet.

## Built on this branch

Everything marked missing or differs above is built, except LCK-27 and
HID-06 (no UV viewport). NAV-12 Re-centre has no default key (View >
Navigate > Re-centre; bindable). NAV-16 still frames only the primary item
in Item mode. Provisional readings: `docs/SPECIFICATION_QUESTIONS.md`
NLH-Q01..Q11. GUI checklist: `docs/NAV_LOCK_HIDE_GUI_QA.md`.
